//! Bounded field tiling and packing policy from `opt/aggregate_slots.cpp`. See NOTICE.
use super::{
    aggregate_slots::{Address, Aggregate, Field, SlotFacts, filler, unsigned},
    alias_facts::signed_size,
    memory_contents::bytes,
};
use crate::lowir::{model::SlotId, vocabulary::Type};
const MAX_FIELDS: usize = 16;
fn sort(fields: &mut [Field]) {
    crate::support::ordering::sort_by(fields, |a, b| a.offset < b.offset);
}
fn bit(offset: u64) -> u64 {
    if offset / 8 < 64 {
        1_u64 << (offset / 8)
    } else {
        0
    }
}
fn forced_wholes(facts: &mut SlotFacts, size: u64) {
    // Every eligible whole lies on one of the frozen 64 eightbyte starts.
    // Repeated loads/stores do not change the shared-field predicate.
    let mut examined = 0_u64;
    for &(offset, _) in &facts.wholes {
        let mark = bit(offset);
        if mark == 0 || offset.wrapping_add(8) > size || examined & mark != 0 {
            continue;
        }
        examined |= mark;
        let shared = facts.fields.iter().any(|f| {
            f.offset < offset.wrapping_add(8)
                && f.offset.wrapping_add(bytes(f.ty)) > offset
                && !(f.offset == offset && f.ty == Type::I64)
        });
        if shared {
            facts.forced |= mark;
        }
    }
    if facts.forced == 0 {
        return;
    }
    let forced = |offset| facts.forced & bit(offset) != 0;
    facts
        .fields
        .retain(|f| !(f.ty == Type::I64 && f.offset % 8 == 0 && forced(f.offset)));
    facts
        .stores
        .retain(|&(from, to)| !(to.wrapping_sub(from) == 8 && from % 8 == 0 && forced(from)));
    for &(offset, load) in &facts.wholes {
        if forced(offset) {
            let span = (offset, offset.wrapping_add(8));
            facts.spans.push(span);
            if load {
                facts.reads.push(span);
            }
        }
    }
}
impl Aggregate<'_, '_> {
    fn hint(&self, slot: SlotId, gap: u64, end: u64) -> Option<Type> {
        for &index in &self.partners[slot.index()] {
            let copy = &self.copies[index];
            let (own, other) = if copy.from.slot == slot {
                (copy.from, copy.to)
            } else if copy.to.slot == slot {
                (copy.to, copy.from)
            } else {
                continue;
            };
            let signed = signed_size(gap);
            if signed < own.offset || signed >= own.offset.wrapping_add(signed_size(copy.bytes)) {
                continue;
            }
            let at = other.offset.wrapping_add(signed).wrapping_sub(own.offset);
            for f in &self.facts[other.slot.index()].fields {
                let size = bytes(f.ty);
                if signed_size(f.offset) == at
                    && (4..=8).contains(&size)
                    && size.is_power_of_two()
                    && gap.wrapping_add(size) <= end
                    && gap.is_multiple_of(size)
                    && f.ty != Type::F80
                    && !matches!(f.ty, Type::Object { .. })
                {
                    return Some(f.ty);
                }
            }
        }
        None
    }
    fn fill(
        &self,
        slot: SlotId,
        mut gap: u64,
        end: u64,
        base: usize,
        fields: &mut Vec<Field>,
    ) -> bool {
        while gap < end {
            let hint = self.hint(slot, gap, end);
            let mut size = hint.map_or(8, bytes);
            while size > 1 && (!gap.is_multiple_of(size) || gap.wrapping_add(size) > end) {
                size /= 2;
            }
            fields.push(Field {
                offset: gap,
                ty: hint.unwrap_or_else(|| filler(size)),
                packed: false,
            });
            gap = gap.wrapping_add(size);
            if base + fields.len() > MAX_FIELDS {
                return false;
            }
        }
        true
    }
    pub(super) fn settle(&self, facts: &mut SlotFacts, size: u64, slot: SlotId) -> bool {
        forced_wholes(facts, size);
        sort(&mut facts.fields);
        let mut unique = Vec::<Field>::new();
        for &field in &facts.fields {
            let end = field.offset.wrapping_add(bytes(field.ty));
            if end > size || bytes(field.ty) == 0 {
                return false;
            }
            if let Some(previous) = unique.last() {
                if previous.offset == field.offset {
                    if previous.ty != field.ty {
                        return false;
                    }
                    continue;
                }
                if previous.offset.wrapping_add(bytes(previous.ty)) > field.offset {
                    return false;
                }
            }
            unique.push(field);
        }
        for &(start, end) in &facts.spans {
            if end > size {
                return false;
            }
            if unique
                .iter()
                .any(|f| f.ty == Type::F80 && f.offset < end && f.offset.wrapping_add(16) > start)
            {
                return false;
            }
            let mut at = start;
            let mut filled = Vec::new();
            for field in &unique {
                let after = field.offset.wrapping_add(bytes(field.ty));
                if after <= at || field.offset >= end {
                    continue;
                }
                if field.offset < at || after > end {
                    return false;
                }
                if !self.fill(slot, at, field.offset, unique.len(), &mut filled) {
                    return false;
                }
                at = after;
            }
            if !self.fill(slot, at, end, unique.len(), &mut filled) {
                return false;
            }
            unique.extend(filled);
            sort(&mut unique);
        }
        if unique.is_empty() || unique.len() > MAX_FIELDS {
            return false;
        }
        facts.fields = unique;
        true
    }
    fn agree_copy(&mut self, from: Address, to: Address, size: u64, settled: &[bool]) -> bool {
        if !settled[from.slot.index()] && settled[to.slot.index()] {
            let before = self.facts[to.slot.index()].packed;
            let mut mask = before;
            for &(start, end) in &self.facts[from.slot.index()].stores {
                let mut at = start.max(unsigned(from.offset));
                let limit = unsigned(from.offset).wrapping_add(size);
                while at < end && at < limit {
                    mask &= !bit(at
                        .wrapping_sub(unsigned(from.offset))
                        .wrapping_add(unsigned(to.offset)));
                    at = at.wrapping_add(1);
                }
            }
            self.facts[to.slot.index()].packed = mask;
            return mask != before;
        }
        if !settled[from.slot.index()] || !settled[to.slot.index()] {
            return false;
        }
        let mut changed = false;
        for relative in 0..size {
            let a = unsigned(from.offset).wrapping_add(relative);
            let b = unsigned(to.offset).wrapping_add(relative);
            let left = self.facts[from.slot.index()].packed & bit(a) != 0;
            let right = self.facts[to.slot.index()].packed & bit(b) != 0;
            if (left || right) && (left != right || a % 8 != b % 8) {
                self.facts[from.slot.index()].packed &= !bit(a);
                self.facts[to.slot.index()].packed &= !bit(b);
                changed = true;
            }
        }
        changed
    }
    pub(super) fn agree_packing(&mut self, settled: &[bool]) {
        loop {
            let mut changed = false;
            for index in 0..self.copies.len() {
                let copy = self.copies[index];
                changed = self.agree_copy(copy.from, copy.to, copy.bytes, settled) || changed;
            }
            if !changed {
                break;
            }
        }
    }
}
pub(super) fn packable(facts: &SlotFacts, size: u64) -> u64 {
    let mut mask = 0;
    let mut start = 0_u64;
    while start < size && start / 8 < 64 {
        let end = start.wrapping_add(8).min(size);
        let width = end - start;
        if !matches!(width, 2 | 4 | 8) {
            start += 8;
            continue;
        }
        let mut inside = 0;
        let mut okay = true;
        for f in &facts.fields {
            let after = f.offset.wrapping_add(bytes(f.ty));
            if after <= start || f.offset >= end {
                continue;
            }
            okay &= f.offset >= start && after <= end && f.ty.bits() != 0 && f.ty != Type::I128;
            inside += 1;
        }
        let stored = facts.stores.iter().any(|&(a, b)| b > start && a < end);
        let read = facts.reads.iter().any(|&(a, b)| a <= start && b >= end);
        let forced = facts.forced & bit(start) != 0;
        okay &= forced || !stored || read && inside >= 3;
        okay &= facts
            .spans
            .iter()
            .all(|&(a, b)| b <= start || a >= end || a <= start && b >= end);
        let whole = facts
            .fields
            .iter()
            .any(|f| f.offset == start && f.ty == filler(width));
        if inside == 1 && whole || okay && (inside >= 2 || forced && inside >= 1) {
            mask |= bit(start);
        }
        start += 8;
    }
    mask
}
pub(super) fn pack(facts: &mut SlotFacts) {
    if facts.packed == 0 {
        return;
    }
    let mut fields = Vec::<Field>::new();
    for &field in &facts.fields {
        let start = field.offset / 8 * 8;
        if facts.packed & bit(start) == 0 {
            fields.push(field);
            continue;
        }
        if fields.last().is_some_and(|f| f.packed && f.offset == start) {
            continue;
        }
        let mut end = start;
        for other in &facts.fields {
            if other.offset / 8 * 8 == start {
                end = end.max(other.offset.wrapping_add(bytes(other.ty)));
            }
        }
        let mut width = 2;
        while width < end.wrapping_sub(start) {
            width *= 2;
        }
        fields.push(Field {
            offset: start,
            ty: filler(width),
            packed: true,
        });
    }
    facts.fields = fields;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_whole_accesses_keep_every_required_span_and_read() {
        let mut facts = SlotFacts {
            fields: vec![
                Field {
                    offset: 0,
                    ty: Type::I64,
                    packed: false
                };
                4096
            ],
            wholes: vec![(0, true); 4096],
            ..SlotFacts::default()
        };
        facts.fields.push(Field {
            offset: 1,
            ty: Type::U8,
            packed: false,
        });
        forced_wholes(&mut facts, 8);
        assert_eq!(facts.forced, 1);
        assert_eq!(facts.fields.len(), 1);
        assert_eq!(facts.spans.len(), 4096);
        assert_eq!(facts.reads.len(), 4096);
        assert_eq!(facts.fields[0].ty, Type::U8);
    }
}
