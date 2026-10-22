//! Four-interval edge proofs from `opt/jump_threading.cpp`. See NOTICE.
use super::{algebra::producer, alias_facts::signed_word, numeric_fold};
use crate::lowir::{
    literals::Constant,
    model::{BlockId, Body, InstructionKind as I, Value},
    vocabulary::{Compare as C, Convert as X, Type},
};

#[derive(Clone, Copy, Default)]
struct Interval {
    low: u64,
    high: u64,
}
#[derive(Clone, Copy, Default)]
pub(super) struct ValueSet {
    parts: [Interval; 4],
    length: usize,
    lost: bool,
}
pub(super) fn ones(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1_u64 << bits) - 1
    }
}
pub(super) fn width(ty: Type) -> u32 {
    match ty {
        Type::I8 | Type::U8 => 8,
        Type::I16 | Type::U16 => 16,
        Type::I32 | Type::U32 => 32,
        Type::I64 | Type::Ptr => 64,
        _ => 0,
    }
}
impl ValueSet {
    fn add(&mut self, low: u64, high: u64) {
        if low > high {
            return;
        }
        if self.length == 4 {
            self.lost = true;
            return;
        }
        self.parts[self.length] = Interval { low, high };
        self.length += 1;
    }
    fn signed_range(&mut self, low: i64, high: i64, mask: u64) {
        if low > high {
            return;
        }
        if high >= 0 {
            self.add(low.max(0).unsigned_abs(), high.unsigned_abs());
        }
        if low < 0 {
            self.add(
                u64::from_le_bytes(low.to_le_bytes()) & mask,
                u64::from_le_bytes(high.min(-1).to_le_bytes()) & mask,
            );
        }
    }
    pub(super) fn admitted(pred: C, mut value: u64, bits: u32) -> Self {
        let mask = ones(bits);
        let sign = 1_u64 << (bits - 1);
        value &= mask;
        let signed = i64::from_le_bytes(
            (if bits == 64 || value & sign == 0 {
                value
            } else {
                value | !mask
            })
            .to_le_bytes(),
        );
        let low = if bits == 64 {
            i64::MIN
        } else {
            -i64::from_le_bytes(sign.to_le_bytes())
        };
        let high = if bits == 64 {
            i64::MAX
        } else {
            i64::from_le_bytes((sign - 1).to_le_bytes())
        };
        let mut out = Self::default();
        match pred {
            C::Eq => out.add(value, value),
            C::Ne => {
                if value > 0 {
                    out.add(0, value - 1);
                }
                if value < mask {
                    out.add(value + 1, mask);
                }
            }
            C::Ult => {
                if value > 0 {
                    out.add(0, value - 1);
                }
            }
            C::Ule => out.add(0, value),
            C::Ugt => {
                if value < mask {
                    out.add(value + 1, mask);
                }
            }
            C::Uge => out.add(value, mask),
            C::Lt => {
                if signed > low {
                    out.signed_range(low, signed - 1, mask);
                }
            }
            C::Le => out.signed_range(low, signed, mask),
            C::Gt => {
                if signed < high {
                    out.signed_range(signed + 1, high, mask);
                }
            }
            C::Ge => out.signed_range(signed, high, mask),
        }
        out
    }
    fn extended(self, from: u32, to: u32, signed: bool) -> Self {
        if !signed {
            return self;
        }
        let sign = 1_u64 << (from - 1);
        let high = ones(to) & !ones(from);
        let mut out = Self {
            lost: self.lost,
            ..Self::default()
        };
        for part in &self.parts[..self.length] {
            if part.low < sign {
                out.add(part.low, part.high.min(sign - 1));
            }
            if part.high >= sign {
                out.add(part.low.max(sign) | high, part.high | high);
            }
        }
        out
    }
    fn unextended(self, from: u32, to: u32, signed: bool) -> Self {
        let mask = ones(from);
        let sign = 1_u64 << (from - 1);
        let high = ones(to) & !mask;
        let mut out = Self {
            lost: self.lost,
            ..Self::default()
        };
        for part in &self.parts[..self.length] {
            if !signed {
                if part.low <= mask {
                    out.add(part.low, part.high.min(mask));
                }
                continue;
            }
            if part.low < sign {
                out.add(part.low, part.high.min(sign - 1));
            }
            let low = part.low.max(high | sign);
            let end = part.high.min(high | mask);
            if low <= end && low >= high | sign {
                out.add(low & mask, end & mask);
            }
        }
        out
    }
    pub(super) fn decides(self, query: Self) -> Option<bool> {
        if self.lost || query.lost || self.length == 0 {
            return None;
        }
        let mut all = true;
        let mut none = true;
        for part in &self.parts[..self.length] {
            let mut inside = false;
            let mut overlaps = false;
            for q in &query.parts[..query.length] {
                inside |= q.low <= part.low && part.high <= q.high;
                overlaps |= q.low <= part.high && part.low <= q.high;
            }
            all &= inside;
            none &= !overlaps;
        }
        if all {
            Some(true)
        } else if none {
            Some(false)
        } else {
            None
        }
    }
    pub(super) fn switched(body: &Body, kind: &I, target: BlockId, bits: u32) -> Self {
        let I::Switch(_, default, pairs) = kind else {
            return Self {
                lost: true,
                ..Self::default()
            };
        };
        let by_default = *default == target;
        let mask = ones(bits);
        let mut keys = Vec::new();
        let mut out = Self::default();
        for &(value, to) in pairs {
            let Value::Integer(value) = value else {
                return Self {
                    lost: true,
                    ..Self::default()
                };
            };
            if (to == target) != by_default {
                keys.push(
                    u64::from_le_bytes(signed_word(body.literal(value)).to_le_bytes()) & mask,
                );
            }
        }
        keys.sort_unstable();
        keys.dedup();
        if !by_default {
            let mut at = 0;
            while at < keys.len() {
                let mut end = at;
                while end + 1 < keys.len() && keys[end + 1] == keys[end].wrapping_add(1) {
                    end += 1;
                }
                out.add(keys[at], keys[end]);
                at = end + 1;
            }
            return out;
        }
        let mut from = 0;
        for &key in &keys {
            if key > from {
                out.add(from, key - 1);
            }
            from = key.wrapping_add(1);
        }
        if keys.last().is_none_or(|&last| last < mask) {
            out.add(from, mask);
        }
        out
    }
}
#[derive(Clone, Copy)]
struct Extension {
    from: u32,
    to: u32,
    signed: bool,
}
fn stripped(body: &Body, mut value: Value) -> (Value, [Option<Extension>; 4]) {
    let mut chain = [None; 4];
    for slot in &mut chain {
        let Some((_, inst)) = producer(body, value) else {
            break;
        };
        let I::Convert(op, to, from, base) = inst.kind else {
            break;
        };
        let source = width(from);
        let target = width(to);
        if !matches!(op, X::ZeroExtend | X::SignExtend)
            || source == 0
            || target == 0
            || source >= target
            || from.bits() == 0
            || to.bits() == 0
        {
            break;
        }
        *slot = Some(Extension {
            from: source,
            to: target,
            signed: op == X::SignExtend,
        });
        value = base;
    }
    (value, chain)
}
pub(super) fn related(
    body: &Body,
    known: Value,
    mut facts: ValueSet,
    bits: u32,
    value: Value,
    value_bits: u32,
) -> ValueSet {
    let unknown = ValueSet {
        lost: true,
        ..ValueSet::default()
    };
    if known == value {
        return if bits == value_bits { facts } else { unknown };
    }
    let (a, down) = stripped(body, known);
    let (b, up) = stripped(body, value);
    if a != b {
        return unknown;
    }
    let mut width = bits;
    for e in down.into_iter().flatten() {
        if e.to != width {
            return unknown;
        }
        facts = facts.unextended(e.from, e.to, e.signed);
        width = e.from;
    }
    for e in up.into_iter().rev().flatten() {
        if e.from != width {
            return unknown;
        }
        facts = facts.extended(e.from, e.to, e.signed);
        width = e.to;
    }
    if width == value_bits { facts } else { unknown }
}
pub(super) fn edge_fact(body: &Body, from: BlockId, to: BlockId) -> Option<(Value, ValueSet, u32)> {
    let kind = &body
        .instruction(*body.block(from).instructions.last()?)
        .kind;
    if let I::Switch(value @ Value::Temp(temp), default, pairs) = kind {
        let bits = width(body.temp(*temp).ty?);
        return (bits != 0 && (*default == to || pairs.iter().any(|&(_, b)| b == to)))
            .then(|| (*value, ValueSet::switched(body, kind, to, bits), bits));
    }
    let I::Branch(value @ Value::Temp(temp), yes, no) = *kind else {
        return None;
    };
    if yes == no || yes != to && no != to {
        return None;
    }
    let taken = yes == to;
    if let Some((_, inst)) = producer(body, value)
        && let I::Compare(mut pred, ty, mut base, mut constant) = inst.kind
        && ty.float_rank() == 0
    {
        if !matches!(base, Value::Temp(_)) {
            std::mem::swap(&mut base, &mut constant);
            pred = pred.swapped();
        }
        let bits = width(ty);
        if bits == 0 || !matches!(base, Value::Temp(_)) {
            return None;
        }
        let Constant::Integer(raw) = numeric_fold::literal(body, constant)? else {
            return None;
        };
        return Some((
            base,
            ValueSet::admitted(
                if taken { pred } else { pred.inverted() },
                u64::from_le_bytes(signed_word(raw).to_le_bytes()),
                bits,
            ),
            bits,
        ));
    }
    let bits = width(body.temp(temp).ty?);
    (bits != 0).then(|| {
        (
            value,
            ValueSet::admitted(if taken { C::Ne } else { C::Eq }, 0, bits),
            bits,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_and_unsigned_byte_intervals_agree_with_target_comparisons() {
        for pred in [
            C::Eq,
            C::Ne,
            C::Lt,
            C::Le,
            C::Gt,
            C::Ge,
            C::Ult,
            C::Ule,
            C::Ugt,
            C::Uge,
        ] {
            for constant in [0, 1, 127, 128, 255] {
                let query = ValueSet::admitted(pred, constant, 8);
                for value in 0..=255 {
                    let known = ValueSet::admitted(C::Eq, value, 8);
                    let result = numeric_fold::compare(
                        pred,
                        Type::I8,
                        Constant::Integer(u128::from(value)),
                        Constant::Integer(u128::from(constant)),
                    );
                    assert_eq!(
                        known.decides(query),
                        result.map(|v| matches!(v, Constant::Integer(1)))
                    );
                }
            }
        }
    }
    #[test]
    fn a_fifth_interval_loses_proof_and_adjacent_query_parts_do_not_strengthen_it() {
        let mut known = ValueSet::default();
        known.add(2, 3);
        let mut query = ValueSet::default();
        query.add(2, 2);
        query.add(3, 3);
        assert_eq!(known.decides(query), None);
        for value in 0..5 {
            query.add(value * 3, value * 3);
        }
        assert!(query.lost);
        assert_eq!(ValueSet::admitted(C::Eq, 2, 8).decides(query), None);
    }
    #[test]
    fn sign_extension_proofs_project_through_negative_and_positive_halves() {
        let narrow = ValueSet::admitted(C::Lt, 0, 8);
        let wide = narrow.extended(8, 64, true);
        assert_eq!(wide.decides(ValueSet::admitted(C::Lt, 0, 64)), Some(true));
        let restored = wide.unextended(8, 64, true);
        assert_eq!(restored.decides(narrow), Some(true));
        assert_eq!(
            narrow
                .extended(8, 64, false)
                .decides(ValueSet::admitted(C::Lt, 0, 64)),
            Some(false)
        );
    }
}
