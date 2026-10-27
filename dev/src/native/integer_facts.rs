//! Nonnegative bit capacities for the frozen optimized alias pass. See NOTICE.
use super::value_facts::Facts;
use crate::lowir::{
    model::{Body, InstructionKind as I, Value},
    vocabulary::{Binary, Convert, Type, Unary},
};

pub(super) fn width(ty: Type) -> u32 {
    if ty == Type::Ptr { 64 } else { ty.bits() }
}
pub(super) fn capacity(ty: Type) -> u32 {
    match ty {
        Type::I1 => 1,
        Type::I8 => 7,
        Type::U8 => 8,
        Type::I16 => 15,
        Type::U16 => 16,
        Type::I32 => 31,
        Type::U32 => 32,
        Type::I64 => 63,
        // Pointers and wide/memory values have no canonical integer fact.
        _ => 0,
    }
}
fn unsigned_narrow(ty: Type) -> bool {
    matches!(ty, Type::U8 | Type::U16 | Type::U32)
}
// Unsigned results may report their width after wrapping; only a result below
// that width is already canonical when computed by a 64-bit operation.
pub(super) fn unnormalized_fits(bits: u32, ty: Type) -> bool {
    bits <= capacity(ty).saturating_sub(u32::from(unsigned_narrow(ty)))
}
fn value_bits(body: &Body, fits: &[u32], value: Value) -> u32 {
    match value {
        Value::Integer(id) => {
            let bits = body.literal(id);
            if bits >= 1_u128 << 63 {
                64
            } else {
                128 - bits.leading_zeros()
            }
        }
        Value::Temp(t) => fits[t.index()],
        _ => 64,
    }
}
pub(super) fn analyze(body: &Body, facts: &Facts) -> Vec<u32> {
    let mut fits = vec![64; body.temps().len()];
    if facts.level == 0 {
        return fits;
    }
    for (pos, &id) in facts.flow.instructions.iter().enumerate() {
        let inst = body.instruction(id);
        let Some(result) = inst.result else {
            continue;
        };
        // find_aliases uses the instruction's stated type here: an index
        // carries its element type, and a compare carries its operand type.
        // These frozen numeric facts do not change either result's actual type.
        let ty = match inst.kind {
            I::Index(ty, ..) | I::Compare(_, ty, ..) => ty,
            _ => inst.kind.result_type(),
        };
        if !facts.active[pos] || ty.bits() == 0 || ty == Type::I128 {
            continue;
        }
        let bits = |v| value_bits(body, &fits, v);
        let w = width(ty);
        let mut r = match inst.kind {
            I::Load(..) if unsigned_narrow(ty) || ty == Type::I1 => w,
            I::Compare(..) => 1,
            I::Unary(Unary::Ctz | Unary::Clz, ..) => 6,
            I::Copy(_, v) => bits(v),
            I::Convert(op, _, from, v) if width(from) != 0 => {
                let b = bits(v);
                match op {
                    Convert::ZeroExtend => b.min(width(from)),
                    Convert::SignExtend if b < width(from) => b,
                    Convert::Truncate if b <= w => b,
                    _ => 64,
                }
            }
            I::Binary(op, _, _, a, b) => {
                let (a, c) = (bits(a), bits(b));
                let shift = if let Value::Integer(id) = b {
                    u32::try_from(body.literal(id)).ok().filter(|&v| v < 64)
                } else {
                    None
                };
                match op {
                    Binary::And => a.min(c),
                    Binary::Or | Binary::Xor => a.max(c),
                    Binary::Add if a.max(c) < 63 => a.max(c) + 1,
                    Binary::Mul if a + c <= 63 => a + c,
                    Binary::Shl if shift.is_some_and(|k| a + k <= 63) => a + shift.unwrap_or(0),
                    Binary::UShr if shift.is_some() => a.min(w).saturating_sub(shift.unwrap_or(0)),
                    Binary::Shr if a < w && shift.is_some() => a.saturating_sub(shift.unwrap_or(0)),
                    _ => 64,
                }
            }
            I::Phi(_, ref pairs) => pairs
                .iter()
                .map(|&(_, value)| bits(value))
                .max()
                .unwrap_or(0),
            I::Select(_, _, yes, no) => bits(yes).max(bits(no)),
            _ => 64,
        };
        if unsigned_narrow(ty) {
            r = r.min(w);
        } else if r > capacity(ty) {
            r = 64;
        }
        fits[result.index()] = r;
    }
    fits
}

fn narrows(kind: &I) -> bool {
    matches!(kind, I::Convert(Convert::Truncate, to, from, _)
        if to.bits() != 0 && !matches!(to, Type::I1 | Type::I128)
        && from.bits() != 0 && *from != Type::I128)
}

// analysis.cpp::find_aliases: a truncation can retain its source's upper
// bits only when every raw reader ignores them. Promotion makes a slot's
// stored value stand for its loads, so its later readers are not bounded by
// the store's width and must keep the normalized value.
pub(super) fn low_only(
    body: &Body,
    slots: &[super::slot_analysis::Slot],
    facts: &Facts,
) -> Vec<bool> {
    if facts.level == 0 {
        return Vec::new();
    }
    let mut low = vec![false; body.temps().len()];
    for (pos, &id) in facts.flow.instructions.iter().enumerate() {
        let inst = body.instruction(id);
        if facts.active[pos]
            && narrows(&inst.kind)
            && matches!(inst.kind, I::Convert(_, _, _, Value::Temp(_)))
            && let Some(result) = inst.result
        {
            low[result.index()] = true;
        }
    }
    for slot in slots {
        if let super::slot_analysis::Slot::Promoted(Value::Temp(temp)) = slot {
            low[temp.index()] = false;
        }
    }
    for (pos, &id) in facts.flow.instructions.iter().enumerate() {
        if !facts.active[pos] {
            continue;
        }
        let kind = &body.instruction(id).kind;
        if matches!(kind, I::Store(_, _, _, Value::Slot(slot))
            if matches!(slots[slot.index()], super::slot_analysis::Slot::Dead
                | super::slot_analysis::Slot::Promoted(_)))
        {
            continue;
        }
        let mut position = 0;
        body.visit_values(kind, |value| {
            let operand = position;
            position += 1;
            let Value::Temp(temp) = value else { return };
            if !low[temp.index()] {
                return;
            }
            let Some(ty) = body.temp(temp).ty else {
                low[temp.index()] = false;
                return;
            };
            let allowed = match kind {
                I::Binary(Binary::Add | Binary::Sub | Binary::Mul, _, to, ..) => {
                    operand < 2 && *to == ty
                }
                I::Store(flags, to, ..) => {
                    operand == 0 && to.bits() != 0 && to.bits() <= ty.bits() && !flags.volatile()
                }
                I::Compare(_, to, ..) => operand < 2 && *to == ty,
                I::Convert(_, to, ..) => operand == 0 && narrows(kind) && to.bits() <= ty.bits(),
                _ => false,
            };
            low[temp.index()] = allowed;
        });
    }
    low
}
