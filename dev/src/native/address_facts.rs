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
    model::{Body, InstructionKind as I, SlotId, SymbolId, TempId, Value},
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
    pub(super) lazy_count: usize,
    pub(super) folded_count: usize,
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
            }
            _ => return Ok(None),
        }
        Ok(Some(expr))
    }
    pub(super) fn analyze(body: &Body, facts: &mut Facts) -> Result<Self, Error> {
        let count = body.temps().len();
        let ids = &body.block(body.layout[0]).instructions;
        let mut out = Self {
            expressions: vec![None; count],
            lazy: vec![false; count],
            folded: vec![None; count],
            lazy_count: 0,
            folded_count: 0,
        };
        let mut uses = vec![0_u32; count];
        let mut consumers = vec![None; count];
        let mut value_use = vec![false; count];
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
            match inst.kind {
                I::Load(..) => {}
                I::Index(_, _, _, v) | I::Store(_, _, v, _) => {
                    if let Alias::Temporary(t) = facts.resolve(body, v) {
                        value_use[t.index()] = true;
                    }
                }
                _ => body.visit_values(&inst.kind, |v| {
                    if let Alias::Temporary(t) = facts.resolve(body, v) {
                        value_use[t.index()] = true;
                    }
                }),
            }
        }
        for (pos, &id) in ids.iter().enumerate() {
            if !facts.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            if !matches!(inst.kind, I::Addr(..) | I::Index(..)) {
                continue;
            }
            let temp = inst.result.ok_or(Error::Limit)?;
            let mut expr = out.expression(body, facts, &inst.kind)?;
            if expr.is_none()
                && let I::Index(_, _, base, _) = inst.kind
                && let Alias::Temporary(base) = facts.resolve(body, base)
                && out.lazy[base.index()]
            {
                out.lazy[base.index()] = false;
                out.lazy_count -= 1;
                expr = out.expression(body, facts, &inst.kind)?;
            }
            if let Some(expr) = expr {
                out.expressions[temp.index()] = Some(expr);
                if uses[temp.index()] != 0 && !value_use[temp.index()] {
                    out.lazy[temp.index()] = true;
                    out.lazy_count += 1;
                }
            }
        }
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
                && uses[temp.index()] == 1
                && let Some(next) = consumers[temp.index()].filter(|&p| p == pos + 1)
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
                    }
                }
            }
            let I::Load(flags, ty, address) = inst.kind else {
                continue;
            };
            let temp = inst.result.ok_or(Error::Limit)?;
            if flags.volatile()
                || ty.float_rank() != 0
                || matches!(ty, Type::Object { .. } | Type::I128)
                || uses[temp.index()] != 1
            {
                continue;
            }
            let Some(next) = consumers[temp.index()].filter(|&p| p == pos + 1) else {
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
                I::Compare(_, user_ty, a, b) => (user_ty, a, b, ty != Type::I1),
                _ => continue,
            };
            let left = matches!(facts.resolve(body,a),Alias::Temporary(t) if t==temp);
            let right = matches!(facts.resolve(body,b),Alias::Temporary(t) if t==temp);
            if user_ty != ty || left == right || !allowed {
                continue;
            }
            out.folded[temp.index()] = Some(address);
            out.folded_count += 1;
            facts.active[pos] = false;
            if let Alias::Temporary(t) = facts.resolve(body, address) {
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
                if let Some(last) = facts.intervals[t].last {
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
        facts.refresh_clobbers(body);
        Ok(out)
    }
}
