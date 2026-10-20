//! Function-owned aliases, dead definitions and scalar use intervals. See NOTICE.
use super::{slot_analysis::Slot, value_selection::unsupported, x86::Register as R};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, InstructionKind as I, TempId, Value},
    vocabulary::{Binary, Type},
};
#[derive(Clone, Copy)]
pub(super) enum Alias {
    Constant(Constant),
    Temporary(TempId),
    Symbol(crate::lowir::model::SymbolId),
}
#[derive(Clone, Copy, Default)]
pub(super) struct Interval {
    pub(super) last: Option<usize>,
    pub(super) hint: Option<R>,
    pub(super) forbid: u32,
}
pub(super) struct Facts {
    aliases: Vec<Option<Alias>>,
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
    pub(super) fn resolve(&self, body: &Body, value: Value) -> Result<Alias, Error> {
        Ok(match value {
            Value::Temp(id) => self.aliases[id.index()].unwrap_or(Alias::Temporary(id)),
            Value::Integer(id) => Alias::Constant(Constant::Integer(body.literal(id))),
            Value::Float(id, kind) => Alias::Constant(Constant::Float(kind, body.literal(id))),
            Value::Symbol(id) => Alias::Symbol(id),
            Value::Slot(_) => return Err(unsupported()),
        })
    }
    pub(super) fn analyze(body: &Body, slots: &[Slot]) -> Result<Self, Error> {
        let ids = &body.block(body.layout[0]).instructions;
        let mut facts = Self {
            aliases: vec![None; body.temps().len()],
            active: vec![true; ids.len()],
            intervals: vec![Interval::default(); body.temps().len()],
        };
        for (pos, &id) in ids.iter().enumerate() {
            let inst = body.instruction(id);
            let alias = match inst.kind {
                I::Const(ty, value) => Some(match facts.resolve(body, value)? {
                    Alias::Constant(Constant::Integer(bits))
                        if ty.bits() != 0 && ty != Type::I128 =>
                    {
                        Alias::Constant(Constant::Integer(normalize(bits, ty)))
                    }
                    other => other,
                }),
                I::Copy(ty, value) => {
                    let source = facts.resolve(body, value)?;
                    match source {
                        Alias::Constant(Constant::Integer(bits))
                            if (ty.bits() != 0 || ty == Type::Ptr) && ty != Type::I128 =>
                        {
                            Some(Alias::Constant(Constant::Integer(normalize(bits, ty))))
                        }
                        Alias::Constant(Constant::Float(..)) if ty.float_rank() != 0 => {
                            Some(source)
                        }
                        Alias::Temporary(temp)
                            if body
                                .temp(temp)
                                .ty
                                .is_some_and(|from| alias_compatible(ty, from)) =>
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
                    Some(match facts.resolve(body, value)? {
                        Alias::Constant(Constant::Integer(bits))
                            if ty.bits() != 0 && ty != Type::I128 =>
                        {
                            Alias::Constant(Constant::Integer(normalize(bits, ty)))
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
                _ => None,
            };
            if let Some(result) = inst.result {
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
                    if let Ok(Alias::Temporary(temp)) = facts.resolve(body, v) {
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
            let pure = matches!(inst.kind, I::Copy(..) | I::Unary(..) | I::Binary(..));
            if pure && inst.result.is_some_and(|r| uses[r.index()] == 0) {
                facts.active[pos] = false;
                body.visit_values(&inst.kind, |v| {
                    if let Ok(Alias::Temporary(temp)) = facts.resolve(body, v) {
                        uses[temp.index()] = uses[temp.index()].saturating_sub(1);
                    }
                });
            }
        }
        for (pos, &id) in ids.iter().enumerate() {
            if facts.active[pos] {
                body.visit_values(&body.instruction(id).kind, |v| {
                    if let Ok(Alias::Temporary(t)) = facts.resolve(body, v) {
                        facts.intervals[t.index()].last = Some(pos);
                    }
                });
            }
        }
        let mut next_shift = None;
        for (pos, &id) in ids.iter().enumerate().rev() {
            if !facts.active[pos] {
                continue;
            }
            let inst = body.instruction(id);
            if let Some(result) = inst.result {
                let interval = &mut facts.intervals[result.index()];
                if next_shift.is_some_and(|p| interval.last.is_some_and(|last| p <= last)) {
                    interval.forbid = 1 << R::RCX.code();
                }
            }
            if matches!(inst.kind,I::Binary(Binary::Shl|Binary::Shr|Binary::UShr,_,_,_,count) if !matches!(facts.resolve(body,count)?,Alias::Constant(Constant::Integer(_))))
            {
                next_shift = Some(pos);
            }
            if let I::Return(_, Some(value)) = inst.kind
                && let Alias::Temporary(temp) = facts.resolve(body, value)?
            {
                let two_address = body
                    .temp(temp)
                    .definition
                    .is_some_and(|def| matches!(body.instruction(def).kind, I::Binary(..)));
                if !two_address && facts.intervals[temp.index()].last == Some(pos) {
                    facts.intervals[temp.index()].hint = Some(R::RAX);
                }
            }
        }
        for interval in facts.intervals.iter_mut().take(body.parameter_count) {
            if next_shift.is_some_and(|p| interval.last.is_some_and(|last| p <= last)) {
                interval.forbid = 1 << R::RCX.code();
            }
        }
        Ok(facts)
    }
}
