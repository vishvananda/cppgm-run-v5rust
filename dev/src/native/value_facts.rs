//! Function-owned aliases, dead definitions and scalar use intervals. See NOTICE.
use super::{slot_analysis::Slot, x86::Register as R};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, RuntimeInstruction as Runtime, TempId, Value},
    vocabulary::{Binary, Convert, Type},
};
#[derive(Clone, Copy)]
pub(super) enum Alias {
    Constant(Constant),
    Temporary(TempId),
    Symbol(crate::lowir::model::SymbolId),
    Slot(crate::lowir::model::SlotId),
}
#[derive(Clone, Copy, Default)]
pub(super) struct Interval {
    pub(super) last: Option<usize>,
    pub(super) hint: Option<R>,
    pub(super) forbid: u32,
    pub(super) uses: u32,
    pub(super) consumer: Option<usize>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Identity {
    Original(Value),
    Normalized(TempId),
}
pub(super) struct Facts {
    aliases: Vec<Option<Alias>>,
    identities: Vec<Option<Identity>>,
    pub(super) fused: Vec<bool>,
    pub(super) active: Vec<bool>,
    pub(super) intervals: Vec<Interval>,
}
pub(super) fn alias_compatible(to: Type, from: Type) -> bool {
    if to == from {
        return true;
    }
    if !matches!(to, Type::Ptr) && to.bits() == 0
        || !matches!(from, Type::Ptr) && from.bits() == 0
        || matches!(to, Type::I128)
        || matches!(from, Type::I128)
    {
        return false;
    }
    let tb = if to == Type::Ptr { 64 } else { to.bits() };
    let fb = if from == Type::Ptr { 64 } else { from.bits() };
    from == Type::I1
        || tb == 64
        || tb > fb
            && (!matches!(from, Type::I8 | Type::I16 | Type::I32)
                || matches!(to, Type::I8 | Type::I16 | Type::I32))
}
pub(super) fn normalize(bits: u128, ty: Type) -> u128 {
    if ty == Type::I128 {
        return bits;
    }
    let width = if ty == Type::Ptr { 64 } else { ty.bits() };
    if width == 0 {
        return bits;
    }
    let mask = (1_u128 << width) - 1;
    let low = bits & mask;
    if matches!(ty, Type::I8 | Type::I16 | Type::I32 | Type::I64) && low & (1 << (width - 1)) != 0 {
        low | !mask
    } else {
        low
    }
}

impl Facts {
    pub(super) fn resolve(&self, body: &Body, value: Value) -> Alias {
        match value {
            Value::Temp(id) => self.aliases[id.index()].unwrap_or(Alias::Temporary(id)),
            Value::Integer(id) => Alias::Constant(Constant::Integer(body.literal(id))),
            Value::Float(id, kind) => Alias::Constant(Constant::Float(kind, body.literal(id))),
            Value::Symbol(id) => Alias::Symbol(id),
            Value::Slot(id) => Alias::Slot(id),
        }
    }
    pub(super) fn analyze(body: &Body, slots: &[Slot]) -> Result<Self, Error> {
        let ids = &body.block(body.layout[0]).instructions;
        let mut facts = Self {
            aliases: vec![None; body.temps().len()],
            identities: vec![None; body.temps().len()],
            fused: vec![false; body.temps().len()],
            active: vec![true; ids.len()],
            intervals: vec![Interval::default(); body.temps().len()],
        };
        for (pos, &id) in ids.iter().enumerate() {
            let inst = body.instruction(id);
            let alias = match inst.kind {
                I::Const(ty, value) => Some(match facts.resolve(body, value) {
                    Alias::Constant(Constant::Integer(bits))
                        if ty.bits() != 0 && ty != Type::I128 =>
                    {
                        Alias::Constant(Constant::Integer(alias_bits(bits, ty)))
                    }
                    other => other,
                }),
                I::Copy(ty, value) => {
                    let source = facts.resolve(body, value);
                    match source {
                        Alias::Constant(Constant::Integer(bits))
                            if (ty.bits() != 0 || ty == Type::Ptr) && ty != Type::I128 =>
                        {
                            Some(Alias::Constant(Constant::Integer(alias_bits(bits, ty))))
                        }
                        Alias::Constant(Constant::Float(..)) if ty.float_rank() != 0 => {
                            Some(source)
                        }
                        Alias::Temporary(temp)
                            if body.temp(temp).ty.is_some_and(|from| {
                                alias_compatible(ty, from)
                                    || (ty.bits() != 0
                                        && ty != Type::I128
                                        && Self::known_boolean(body, temp, 0))
                            }) =>
                        {
                            Some(source)
                        }
                        _ => None,
                    }
                }
                I::Load(_, ty, Value::Slot(slot))
                    if matches!(slots[slot.index()], Slot::Promoted(_)) =>
                {
                    let Slot::Promoted(value) = slots[slot.index()] else {
                        return Err(Error::Limit);
                    };
                    Some(match facts.resolve(body, value) {
                        Alias::Constant(Constant::Integer(bits))
                            if ty.bits() != 0 && ty != Type::I128 =>
                        {
                            Alias::Constant(Constant::Integer(alias_bits(bits, ty)))
                        }
                        other => other,
                    })
                }
                I::Store(_, _, _, Value::Slot(slot))
                    if matches!(slots[slot.index()], Slot::Dead | Slot::Promoted(_)) =>
                {
                    facts.active[pos] = false;
                    None
                }
                I::Convert(op, to, from, value) => {
                    let source = facts.resolve(body, value);
                    if let Alias::Temporary(temp) = source {
                        let ints = (to.bits() != 0 || to == Type::Ptr)
                            && (from.bits() != 0 || from == Type::Ptr)
                            && to != Type::I128
                            && from != Type::I128;
                        let boolean = Self::known_boolean(body, temp, 0)
                            && !(op == Convert::SignExtend && from == Type::I1);
                        let extension = (op == Convert::ZeroExtend
                            && matches!(from, Type::U8 | Type::U16 | Type::U32 | Type::I1)
                            || op == Convert::SignExtend
                                && matches!(from, Type::I8 | Type::I16 | Type::I32))
                            && alias_compatible(to, from);
                        if ints && (boolean || extension) {
                            Some(source)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                I::Index(_, _, base, offset) => {
                    let off = facts.resolve(body, offset);
                    let base = facts.resolve(body, base);
                    if matches!(off,Alias::Constant(Constant::Integer(bits)) if bits & u128::from(u64::MAX)==0)
                        && matches!(base,Alias::Temporary(t) if body.temp(t).ty==Some(Type::Ptr))
                    {
                        Some(base)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(result) = inst.result {
                if let Some(alias) = alias {
                    facts.identities[result.index()] =
                        facts.alias_identity(body, slots, &inst.kind, result, alias);
                }
                facts.aliases[result.index()] = alias;
                if alias.is_some() {
                    facts.active[pos] = false;
                }
            }
        }
        let mut uses = vec![0_u32; body.temps().len()];
        for (pos, &id) in ids.iter().enumerate() {
            if facts.active[pos] {
                body.visit_values(&body.instruction(id).kind, |v| {
                    if let Alias::Temporary(temp) = facts.resolve(body, v) {
                        uses[temp.index()] += 1;
                    }
                });
            }
        }
        for (pos, &id) in ids.iter().enumerate().rev() {
            if !facts.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            let pure = matches!(
                inst.kind,
                I::Copy(..)
                    | I::Unary(..)
                    | I::Binary(..)
                    | I::Addr(..)
                    | I::Index(..)
                    | I::Compare(..)
                    | I::Convert(..)
                    | I::Select(..)
            ) || matches!(inst.kind,I::Load(flags,_,_) if !flags.volatile());
            if pure && inst.result.is_some_and(|r| uses[r.index()] == 0) {
                facts.active[pos] = false;
                body.visit_values(&inst.kind, |v| {
                    if let Alias::Temporary(temp) = facts.resolve(body, v) {
                        uses[temp.index()] = uses[temp.index()].saturating_sub(1);
                    }
                });
            }
        }
        for (pos, &id) in ids.iter().enumerate() {
            if facts.active[pos] {
                body.visit_values(&body.instruction(id).kind, |v| {
                    if let Alias::Temporary(t) = facts.resolve(body, v) {
                        let interval = &mut facts.intervals[t.index()];
                        interval.last = Some(pos);
                        interval.uses += 1;
                        interval.consumer = Some(pos);
                    }
                });
            }
        }
        facts.refresh_clobbers(body);
        Ok(facts)
    }
    fn alias_identity(
        &self,
        body: &Body,
        slots: &[Slot],
        kind: &I,
        result: TempId,
        alias: Alias,
    ) -> Option<Identity> {
        let source = match *kind {
            I::Const(_, v) | I::Copy(_, v) | I::Convert(_, _, _, v) | I::Index(_, _, v, _) => v,
            I::Load(_, _, Value::Slot(slot)) => {
                let Slot::Promoted(v) = slots[slot.index()] else {
                    return None;
                };
                v
            }
            _ => return None,
        };
        Some(
            if matches!((self.resolve(body,source),alias),(Alias::Constant(Constant::Integer(before)),Alias::Constant(Constant::Integer(after))) if before!=after)
            {
                Identity::Normalized(result)
            } else {
                self.identity(source)
            },
        )
    }
    fn identity(&self, value: Value) -> Identity {
        if let Value::Temp(t) = value {
            self.identities[t.index()].unwrap_or(Identity::Original(value))
        } else {
            Identity::Original(value)
        }
    }
    pub(super) fn same_value(&self, left: Value, right: Value) -> bool {
        self.identity(left) == self.identity(right)
    }
    pub(super) fn known_boolean(body: &Body, temp: TempId, depth: u8) -> bool {
        if temp.index() < body.parameter_count {
            return false;
        }
        let Some(def) = body.temp(temp).definition else {
            return false;
        };
        let inst = body.instruction(def);
        if let I::Select(_, _, left, right) = inst.kind
            && depth < 4
        {
            return [left, right].into_iter().all(|v| match v {
                Value::Integer(id) => body.literal(id) <= 1,
                Value::Temp(t) => Self::known_boolean(body, t, depth + 1),
                _ => false,
            });
        }
        matches!(
            inst.kind,
            I::Compare(..) | I::Runtime(Runtime::AtomicCompareExchange { .. })
        ) || body.temp(temp).ty == Some(Type::I1)
            && !matches!(
                inst.kind,
                I::Call(..) | I::Load(..) | I::Runtime(Runtime::AtomicLoad(..))
            )
    }
    pub(super) fn compare_scratch(&self, body: &Body, pos: usize) -> bool {
        let inst = body.instruction(body.block(body.layout[0]).instructions[pos]);
        let I::Compare(_, ty, a, b) = inst.kind else {
            return false;
        };
        let Some(result) = inst.result else {
            return false;
        };
        if ty.float_rank() != 0
            || ty == Type::I128
            || !self.active[pos]
            || self.fused[result.index()]
        {
            return false;
        }
        if !matches!(self.resolve(body, a), Alias::Constant(Constant::Integer(_)))
            && !matches!(self.resolve(body, b), Alias::Constant(Constant::Integer(_)))
        {
            return false;
        }
        let interval = &self.intervals[result.index()];
        if interval.uses == 1
            && matches!(ty, Type::I32 | Type::U32 | Type::I64 | Type::Ptr)
            && let Some(consumer) = interval.consumer
            && matches!(body.instruction(body.block(body.layout[0]).instructions[consumer]).kind,I::Return(_,Some(Value::Temp(t))) if t==result)
        {
            return false;
        }
        true
    }
    pub(super) fn refresh_clobbers(&mut self, body: &Body) {
        self.rebuild_clobbers(body, None);
    }
    pub(super) fn apply_calls(&mut self, body: &Body, calls: &super::call_facts::Calls) {
        self.rebuild_clobbers(body, Some(calls));
    }
    fn call_hints(
        &mut self,
        body: &Body,
        call: &crate::lowir::model::Call,
        plan: &super::call_facts::CallPlan,
        pos: usize,
    ) {
        for (arg, layout) in body
            .arguments(call.arguments)
            .iter()
            .zip(&plan.layout.arguments)
        {
            let Alias::Temporary(temp) = self.resolve(body, *arg) else {
                continue;
            };
            if temp.index() < body.parameter_count {
                continue;
            }
            let interval = &mut self.intervals[temp.index()];
            let address = body
                .temp(temp)
                .definition
                .is_some_and(|def| matches!(body.instruction(def).kind, I::Addr(_)));
            if (interval.uses == 1 || address)
                && interval.last == Some(pos)
                && interval.hint.is_none()
                && let [part] = plan.layout.parts(layout)
            {
                interval.hint = part.register;
            }
        }
    }
    fn rebuild_clobbers(&mut self, body: &Body, calls: Option<&super::call_facts::Calls>) {
        let ids = &body.block(body.layout[0]).instructions;
        for interval in &mut self.intervals {
            interval.forbid = 0;
        }
        let mut late = [None; 16];
        let mut early = [None; 16];
        let mask = |interval: &Interval,
                    late: &[Option<usize>; 16],
                    early: &[Option<usize>; 16]| {
            let mut out = 0_u32;
            for reg in 0..16 {
                if interval.last.is_some_and(|last| {
                    late[reg].is_some_and(|at| at < last) || early[reg].is_some_and(|at| at <= last)
                }) {
                    out |= 1 << reg;
                }
            }
            out
        };
        for (pos, &id) in ids.iter().enumerate().rev() {
            if !self.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            if let Some(result) = inst.result {
                self.intervals[result.index()].forbid =
                    mask(&self.intervals[result.index()], &late, &early);
            }
            let fixed = if let I::Binary(
                op @ (Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod),
                _,
                ty,
                _,
                right,
            ) = inst.kind
                && ty.float_rank() == 0
                && ty != Type::I128
            {
                super::division::Form::at(self, body, op, right).clobbers()
            } else if self.compare_scratch(body, pos) {
                1 << R::RDX.code()
            } else if matches!(inst.kind,I::Binary(Binary::Shl|Binary::Shr|Binary::UShr,_,_,_,count) if !matches!(self.resolve(body,count),Alias::Constant(Constant::Integer(_))))
            {
                1 << R::RCX.code()
            } else {
                0
            };
            let call = calls.and_then(|c| c.at(pos));
            let clobber = fixed | call.map_or(0, |_| super::abi::CALLER_SAVED);
            let before = fixed | call.map_or(0, |c| c.early);
            for reg in 0..16 {
                if clobber & (1 << reg) != 0 {
                    late[reg] = Some(pos);
                }
                if before & (1 << reg) != 0 {
                    early[reg] = Some(pos);
                }
            }
            if let I::Return(_, Some(value)) = inst.kind
                && let Alias::Temporary(temp) = self.resolve(body, value)
            {
                let two = body
                    .temp(temp)
                    .definition
                    .is_some_and(|def| matches!(body.instruction(def).kind, I::Binary(..)));
                if !two && self.intervals[temp.index()].last == Some(pos) {
                    self.intervals[temp.index()].hint = Some(R::RAX);
                }
            }
            if let Some(plan) = call
                && let I::Call(call) = &inst.kind
            {
                self.call_hints(body, call, plan, pos);
            }
        }
        for interval in self.intervals.iter_mut().take(body.parameter_count) {
            interval.forbid = mask(interval, &late, &early);
        }
    }
}
fn alias_bits(bits: u128, ty: Type) -> u128 {
    if matches!(
        ty,
        Type::I1 | Type::I8 | Type::U8 | Type::I16 | Type::U16 | Type::I32 | Type::U32
    ) {
        normalize(bits, ty)
    } else {
        bits
    }
}
