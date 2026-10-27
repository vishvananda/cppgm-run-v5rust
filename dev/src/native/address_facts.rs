//! Function-owned address composition, lazy uses and adjacent-load folds. See NOTICE.
use super::{
    abi,
    mir::low_word,
    value_facts::{Alias, Facts},
    value_selection::unsupported,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, Program, SlotId, SymbolId, TempId, Value},
    vocabulary::{Binary, Type},
};
#[derive(Clone, Copy)]
pub(super) enum AddressBase {
    Slot(SlotId),
    Symbol(SymbolId),
    Temporary(TempId),
    Home(TempId),
    Absolute,
}
#[derive(Clone, Copy)]
pub(super) struct Expression {
    pub(super) base: AddressBase,
    pub(super) displacement: i64,
    pub(super) index: Option<TempId>,
    pub(super) scale: u8,
}
impl Expression {
    fn new(base: AddressBase) -> Self {
        Self {
            base,
            displacement: 0,
            index: None,
            scale: 1,
        }
    }
}
pub(super) struct Addresses {
    pub(super) expressions: Vec<Option<Expression>>,
    pub(super) lazy: Vec<bool>,
    pub(super) folded: Vec<Option<Value>>,
    pub(super) lazy_count: crate::support::telemetry::Counter<usize>,
    pub(super) folded_count: crate::support::telemetry::Counter<usize>,
}
fn record_value_uses(
    body: &Body,
    facts: &Facts,
    kind: &I,
    value_use: &mut [bool],
    call_arguments: bool,
) {
    use crate::lowir::model::RuntimeInstruction as Runtime;
    match *kind {
        I::Call(ref call) if !call_arguments => {
            if let Alias::Temporary(t) = facts.resolve(body, call.callee) {
                value_use[t.index()] = true;
            }
        }
        I::Load(..)
        | I::CopyObject(..)
        | I::ZeroInit(..)
        | I::Runtime(Runtime::VaStart(_) | Runtime::VaArg(..) | Runtime::AtomicLoad(..)) => {}
        I::Index(_, _, _, v)
        | I::Store(_, _, v, _)
        | I::Runtime(
            Runtime::AtomicStore(_, v, ..)
            | Runtime::AtomicUpdate(_, _, _, v, _)
            | Runtime::AtomicCompareExchange { desired: v, .. },
        ) => {
            if let Alias::Temporary(t) = facts.resolve(body, v) {
                value_use[t.index()] = true;
            }
        }
        _ => body.visit_values(kind, |v| {
            if let Alias::Temporary(t) = facts.resolve(body, v) {
                value_use[t.index()] = true;
            }
        }),
    }
}
impl Addresses {
    fn expression(
        &self,
        body: &Body,
        facts: &Facts,
        kind: &I,
    ) -> Result<Option<Expression>, Error> {
        let I::Index(ty, _, base, offset) = *kind else {
            return if let I::Addr(storage) = *kind {
                Ok(Some(Expression::new(match facts.resolve(body, storage) {
                    Alias::Slot(s) => AddressBase::Slot(s),
                    Alias::Symbol(s) => AddressBase::Symbol(s),
                    _ => return Err(unsupported()),
                })))
            } else {
                Ok(None)
            };
        };
        let size = abi::value_bytes(ty)?;
        let mut expr = match facts.resolve(body, base) {
            Alias::Slot(s) => Expression::new(AddressBase::Slot(s)),
            Alias::Symbol(s) => Expression::new(AddressBase::Symbol(s)),
            Alias::Temporary(t) => {
                let own = body.temp(t).ty.ok_or(Error::Limit)?;
                if matches!(own, Type::Object { .. } | Type::I128 | Type::F80) {
                    Expression::new(AddressBase::Home(t))
                } else if self.lazy[t.index()] {
                    self.expressions[t.index()].ok_or(Error::Limit)?
                } else {
                    Expression::new(AddressBase::Temporary(t))
                }
            }
            Alias::Constant(Constant::Integer(bits)) => Expression {
                displacement: low_word(bits)?,
                ..Expression::new(AddressBase::Absolute)
            },
            Alias::Constant(_) => return Ok(None),
        };
        match facts.resolve(body, offset) {
            Alias::Constant(Constant::Integer(bits)) => {
                let k = low_word(bits)?;
                let limit = 1_i64 << 30;
                if !(-limit..=limit).contains(&k)
                    || !(-limit..=limit).contains(&(i64::from(size) * k))
                {
                    return Ok(None);
                }
                expr.displacement = expr.displacement.wrapping_add(k * i64::from(size));
                if !(-limit..=limit).contains(&expr.displacement) {
                    return Ok(None);
                }
            }
            Alias::Temporary(t) => {
                if expr.index.is_some()
                    || matches!(expr.base, AddressBase::Symbol(_))
                    || !matches!(size, 1 | 2 | 4 | 8)
                {
                    return Ok(None);
                }
                if let AddressBase::Temporary(base) = expr.base
                    && abi::value_bytes(body.temp(base).ty.ok_or(Error::Limit)?)? != 8
                {
                    return Ok(None);
                }
                expr.index = Some(t);
                expr.scale = u8::try_from(size).map_err(|_| Error::Limit)?;
                if size == 1
                    && let Value::Temp(raw) = offset
                    && facts.scales[raw.index()] != 0
                {
                    expr.scale = facts.scales[raw.index()];
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(expr))
    }
    fn compose(
        &mut self,
        program: &Program,
        body: &Body,
        facts: &Facts,
        model: super::object_compile::ObjectModel,
        value_use: &[bool],
        other_use: &[bool],
    ) -> Result<(), Error> {
        // Each repeat permanently makes at least one base eager. Later
        // expressions then refer to that base, never to abandoned lazy parts.
        let mut eager = vec![false; body.temps().len()];
        loop {
            self.expressions.fill(None);
            self.lazy.fill(false);
            self.lazy_count = crate::support::telemetry::counter(0);
            let mut again = false;
            for (pos, &id) in facts.flow.instructions.iter().enumerate() {
                let inst = body.instruction(id);
                if !facts.active[pos] || !matches!(inst.kind, I::Addr(..) | I::Index(..)) {
                    continue;
                }
                let temp = inst.result.ok_or(Error::Limit)?;
                let mut expr = self.expression(body, facts, &inst.kind)?;
                if expr.is_none()
                    && let I::Index(_, _, base, _) = inst.kind
                    && let Alias::Temporary(base) = facts.resolve(body, base)
                    && self.lazy[base.index()]
                {
                    self.lazy[base.index()] = false;
                    self.lazy_count -= 1;
                    eager[base.index()] = true;
                    again = facts.level >= 1;
                    expr = self.expression(body, facts, &inst.kind)?;
                }
                let Some(expr) = expr else { continue };
                self.expressions[temp.index()] = Some(expr);
                let direct = match expr.base {
                    AddressBase::Slot(_) | AddressBase::Temporary(_) => true,
                    AddressBase::Symbol(symbol) => {
                        model == super::object_compile::ObjectModel::Course
                            || super::symbol_access::host_symbol_access(program, symbol)
                                == super::symbol_access::SymbolAccess::Direct
                    }
                    _ => false,
                };
                let remat =
                    facts.level >= 1 && direct && expr.index.is_none() && !other_use[temp.index()];
                let global_part = |part: TempId| {
                    facts.global[part.index()]
                        && body.temp(part).ty.is_some_and(|ty| {
                            ty.float_rank() == 0 && !matches!(ty, Type::Object { .. } | Type::I128)
                        })
                };
                let base = if let AddressBase::Temporary(base) = expr.base {
                    Some(base)
                } else {
                    None
                };
                let registers = base.is_some() || expr.index.is_some();
                if (value_use[temp.index()] && !remat)
                    || facts.intervals[temp.index()].uses == 0
                    || eager[temp.index()]
                    || registers
                        && facts.global[temp.index()]
                        && !(facts.level >= 1
                            && base.is_none_or(global_part)
                            && expr.index.is_none_or(global_part))
                {
                    continue;
                }
                self.lazy[temp.index()] = true;
                self.lazy_count += 1;
            }
            if !again {
                return Ok(());
            }
        }
    }
    pub(super) fn analyze(
        program: &Program,
        body: &Body,
        facts: &mut Facts,
        model: super::object_compile::ObjectModel,
    ) -> Result<Self, Error> {
        let count = body.temps().len();
        let ids = facts.flow.instructions.clone();
        let mut out = Self {
            expressions: vec![None; count],
            lazy: vec![false; count],
            folded: vec![None; count],
            lazy_count: crate::support::telemetry::counter(0),
            folded_count: crate::support::telemetry::counter(0),
        };
        let mut uses = vec![0_u32; count];
        let mut consumers = vec![None; count];
        let mut value_use = vec![false; count];
        let mut other_use = vec![false; count];
        for (pos, &id) in ids.iter().enumerate() {
            if !facts.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            body.visit_values(&inst.kind, |v| {
                if let Alias::Temporary(t) = facts.resolve(body, v) {
                    uses[t.index()] += 1;
                    consumers[t.index()] = Some(pos);
                }
            });
            record_value_uses(body, facts, &inst.kind, &mut value_use, true);
            if facts.level >= 1 {
                record_value_uses(body, facts, &inst.kind, &mut other_use, false);
            }
        }
        out.compose(program, body, facts, model, &value_use, &other_use)?;
        for (pos, &id) in ids.iter().enumerate() {
            if !facts.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            if let Some(temp) = inst.result
                && matches!(
                    inst.kind,
                    I::Compare(..) | I::Unary(crate::lowir::vocabulary::Unary::Not, ..)
                )
                && !facts.global[temp.index()]
                && uses[temp.index()] == 1
                && let Some(next) = consumers[temp.index()]
                    .filter(|&p| p == pos + 1 && facts.flow.blocks[p] == facts.flow.blocks[pos])
            {
                let (I::Compare(_, ty, ..) | I::Unary(_, ty, _)) = inst.kind else {
                    return Err(Error::Limit);
                };
                if ty.float_rank() == 0
                    && ty != Type::I128
                    && let I::Select(_, cond, yes, no) = body.instruction(ids[next]).kind
                {
                    let refers = |v| matches!(facts.resolve(body,v),Alias::Temporary(t) if t==temp);
                    if refers(cond) && !refers(yes) && !refers(no) {
                        facts.fused[temp.index()] = true;
                        facts.active[pos] = false;
                        facts.reads[pos] = next;
                    }
                }
            }
            if let Some(temp) = inst.result
                && !facts.global[temp.index()]
                && matches!(
                    inst.kind,
                    I::Compare(..) | I::Unary(crate::lowir::vocabulary::Unary::Not, ..)
                )
                && uses[temp.index()] == 1
                && let Some(next) = consumers[temp.index()]
                    .filter(|&p| p > pos && facts.flow.blocks[p] == facts.flow.blocks[pos])
                && let I::Branch(value, ..) = body.instruction(ids[next]).kind
                && matches!(facts.resolve(body,value),Alias::Temporary(t) if t==temp)
            {
                let (I::Compare(_, ty, ..) | I::Unary(_, ty, _)) = inst.kind else {
                    return Err(Error::Limit);
                };
                if ty.float_rank() == 0
                    && (ty != Type::I128 || matches!(inst.kind, I::Compare(..)) && next == pos + 1)
                    || matches!(ty, Type::F32 | Type::F64) && next == pos + 1
                {
                    facts.fused[temp.index()] = true;
                    facts.active[pos] = false;
                    facts.reads[pos] = next;
                }
            }
            let I::Load(flags, ty, address) = inst.kind else {
                continue;
            };
            let temp = inst.result.ok_or(Error::Limit)?;
            if facts.global[temp.index()]
                || flags.volatile()
                || ty.float_rank() != 0
                || matches!(ty, Type::Object { .. } | Type::I128)
                || uses[temp.index()] != 1
            {
                continue;
            }
            let Some(next) = consumers[temp.index()].filter(|&p| {
                facts.flow.blocks[p] == facts.flow.blocks[pos]
                    && (p == pos + 1
                        || facts.level >= 1
                            && p > pos
                            && p - pos <= 8
                            && ids[pos + 1..p].iter().enumerate().all(|(offset, &id)| {
                                !facts.active[pos + 1 + offset]
                                    || quiet_load_motion(&body.instruction(id).kind)
                            }))
            }) else {
                continue;
            };
            let consumer = body.instruction(ids[next]);
            let (user_ty, a, b, allowed) = match consumer.kind {
                I::Binary(op, _, user_ty, a, b) => {
                    let right = matches!(facts.resolve(body,b),Alias::Temporary(t) if t==temp);
                    (
                        user_ty,
                        a,
                        b,
                        matches!(ty, Type::I64 | Type::Ptr | Type::I32 | Type::U32)
                            && (matches!(
                                op,
                                Binary::Add | Binary::And | Binary::Or | Binary::Xor | Binary::Mul
                            ) || op == Binary::Sub && right),
                    )
                }
                I::Branch(value, ..) if facts.level >= 1 => (ty, value, value, ty != Type::I128),
                I::Compare(_, user_ty, a, b) => (
                    user_ty,
                    a,
                    b,
                    (ty != Type::I1 || facts.level >= 1) && !facts.sunk_compare(body, next),
                ),
                _ => continue,
            };
            let left = matches!(facts.resolve(body,a),Alias::Temporary(t) if t==temp);
            let right = matches!(facts.resolve(body,b),Alias::Temporary(t) if t==temp);
            if user_ty != ty || !matches!(consumer.kind, I::Branch(..)) && left == right || !allowed
            {
                continue;
            }
            facts.reads[pos] = next;
            out.folded[temp.index()] = Some(address);
            out.folded_count += 1;
            facts.active[pos] = false;
            if let Alias::Temporary(t) = facts.resolve(body, address)
                && facts.definitions[t.index()].map_or(body.layout[0], |p| facts.flow.blocks[p])
                    == facts.flow.blocks[pos]
            {
                let last = &mut facts.intervals[t.index()].last;
                *last = Some(last.map_or(next, |old| old.max(next)));
            }
        }
        for t in (0..count).rev() {
            if !out.lazy[t] {
                continue;
            }
            let expr = out.expressions[t].ok_or(Error::Limit)?;
            let parts = [
                if let AddressBase::Temporary(t) = expr.base {
                    Some(t)
                } else {
                    None
                },
                expr.index,
            ];
            for part in parts.into_iter().flatten() {
                if facts.global[t] {
                    facts.global[part.index()] = true;
                }
                if let Some(last) = facts.intervals[t].last
                    && facts.definitions[part.index()]
                        .map_or(body.layout[0], |p| facts.flow.blocks[p])
                        == facts.definitions[t].map_or(body.layout[0], |p| facts.flow.blocks[p])
                {
                    let old = &mut facts.intervals[part.index()].last;
                    *old = Some(old.map_or(last, |p| p.max(last)));
                }
            }
        }
        for (pos, &id) in ids.iter().enumerate() {
            if body
                .instruction(id)
                .result
                .is_some_and(|t| out.lazy[t.index()])
            {
                facts.active[pos] = false;
            }
        }
        for pos in (0..facts.reads.len()).rev() {
            let reader = facts.reads[pos];
            facts.reads[pos] = facts.reads[reader];
        }
        for (pos, &id) in ids.iter().enumerate() {
            if !facts.sunk_compare(body, pos) {
                continue;
            }
            let read = facts.reads[pos];
            body.visit_values(&body.instruction(id).kind, |value| {
                if let Alias::Temporary(temp) = facts.resolve(body, value)
                    && !facts.global[temp.index()]
                    && facts.definitions[temp.index()]
                        .map_or(body.layout[0], |p| facts.flow.blocks[p])
                        == facts.flow.blocks[pos]
                {
                    let last = &mut facts.intervals[temp.index()].last;
                    *last = Some(last.map_or(read, |old| old.max(read)));
                }
            });
        }
        facts.refresh_clobbers(body)?;
        Ok(out)
    }
}

// The frozen fold window permits reads but never a volatile read or division.
fn quiet_load_motion(kind: &I) -> bool {
    match kind {
        I::Load(flags, ..) => !flags.volatile(),
        I::Binary(Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod, ..) => false,
        I::Binary(..)
        | I::Unary(..)
        | I::Convert(..)
        | I::Compare(..)
        | I::Copy(..)
        | I::Const(..)
        | I::Index(..)
        | I::Addr(..)
        | I::Select(..) => true,
        _ => false,
    }
}
