//! Copy grouping and rooted storage facts from `opt/copy_coalescing.cpp`. See NOTICE.
use super::{
    alias_facts::{signed_size, signed_word},
    memory_contents::bytes,
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, Facts, InstId, Instruction, InstructionKind as I, LinkEditor, Value},
    vocabulary::Type,
};
#[derive(Clone, Copy)]
pub(super) struct Rooted {
    pub(super) root: Value,
    pub(super) offset: i64,
}
/// Eight visits, including the visit that recognizes the root. This is not
/// the more general alias analysis used by memory forwarding.
pub(super) fn rooted(body: &Body, mut value: Value) -> Option<Rooted> {
    let mut offset = 0_i64;
    for _ in 0..8 {
        if matches!(value, Value::Slot(_) | Value::Symbol(_))
            || matches!(value,Value::Temp(t)if body.temp_is_parameter(t))
        {
            return Some(Rooted {
                root: value,
                offset,
            });
        }
        let Value::Temp(temp) = value else {
            return None;
        };
        let instruction = body.instruction(body.temp(temp).definition?);
        match instruction.kind {
            I::Addr(base) | I::Copy(Type::Ptr, base) => value = base,
            I::Index(ty, _, base, Value::Integer(literal)) => {
                offset = offset.wrapping_add(
                    signed_word(body.literal(literal)).wrapping_mul(signed_size(bytes(ty))),
                );
                value = base;
            }
            _ => return None,
        }
    }
    None
}
#[derive(Clone, Copy)]
struct Pair {
    load: InstId,
    store: InstId,
    from: Rooted,
    to: Rooted,
    source: Value,
    destination: Value,
    bytes: u64,
}
pub(super) struct Copies<'a, 'p> {
    pub(super) context: &'a mut Context<'p>,
    pub(super) links: LinkEditor,
    pub(super) uses: Vec<u32>,
    pub(super) slot_uses: Vec<u32>,
    pub(super) escaped: Vec<bool>,
    dead: Vec<bool>,
    merges: usize,
    pub(super) changed: bool,
}
impl Copies<'_, '_> {
    pub(super) fn disjoint(&self, a: Value, b: Value) -> bool {
        if a == b {
            return false;
        }
        let named = |v| matches!(v, Value::Slot(_) | Value::Symbol(_));
        if named(a) && named(b) {
            return true;
        }
        if matches!(a,Value::Slot(s)if self.escaped.get(s.index())==Some(&false))
            || matches!(b,Value::Slot(s)if self.escaped.get(s.index())==Some(&false))
        {
            return true;
        }
        let (Value::Temp(a), Value::Temp(b)) = (a, b) else {
            return false;
        };
        let Some(function) = self
            .context
            .program
            .function(self.context.program.symbol(self.context.symbol))
        else {
            return false;
        };
        let parameters = &self
            .context
            .program
            .signature(function.signature)
            .parameters;
        parameters.get(a.index()).is_some_and(|p| p.no_alias)
            && parameters.get(b.index()).is_some_and(|p| p.no_alias)
    }
    pub(super) fn removed_store(&mut self) -> Result<(), Error> {
        self.context.stores_removed = self
            .context
            .stores_removed
            .checked_add(1)
            .ok_or(Error::Limit)?;
        Ok(())
    }
    fn find_escapes(&mut self) {
        for &block in &self.context.body.layout {
            for &id in &self.context.body.block(block).instructions {
                let kind = &self.context.body.instruction(id).kind;
                if matches!(
                    kind,
                    I::Addr(_) | I::Copy(Type::Ptr, _) | I::Index(_, _, _, Value::Integer(_))
                ) {
                    continue;
                }
                let mut position = 0;
                self.context.body.visit_values(kind, |value| {
                    let at = position;
                    position += 1;
                    let address = matches!(kind,I::Load(..)if at==0)
                        || matches!(kind,I::Store(..)if at==1)
                        || matches!(kind, I::CopyObject(..) | I::ZeroInit(..));
                    if address {
                        return;
                    }
                    if let Some(Rooted {
                        root: Value::Slot(slot),
                        ..
                    }) = rooted(self.context.body, value)
                        && (matches!(value, Value::Temp(_)) || !matches!(kind, I::Load(..)))
                    {
                        self.escaped[slot.index()] = true;
                    }
                });
            }
        }
    }
    fn may_reread(&self, pairs: &[Pair]) -> bool {
        let Some(first) = pairs.first() else {
            return true;
        };
        let Some(last) = pairs.last() else {
            return true;
        };
        let mut pending = crate::support::hash::HashMap::default();
        for pair in pairs {
            let Some(temp) = self.context.body.instruction(pair.load).result else {
                return false;
            };
            if self.uses[temp.index()] != 1 {
                pending.insert(temp, (pair.load, pair.store, false));
            }
        }
        if pending.is_empty() {
            return true;
        }
        // The C++ test scans the interval once per reread. Track which
        // loads have been visited to test those same intervals in one scan.
        let mut cursor = Some(first.load);
        while let Some(id) = cursor {
            if id == last.store {
                break;
            }
            let inst = self.context.body.instruction(id);
            let mut forbidden = false;
            self.context.body.visit_values(&inst.kind, |value| {
                if let Value::Temp(temp) = value {
                    forbidden |= pending
                        .get(&temp)
                        .is_some_and(|&(_, store, active)| active && id != store);
                }
            });
            if forbidden {
                return false;
            }
            if let Some(result) = inst.result
                && let Some((load, _, active)) = pending.get_mut(&result)
                && *load == id
            {
                *active = true;
            }
            cursor = self.links.next(id);
        }
        true
    }
    fn flush(&mut self, block: BlockId, pairs: &mut Vec<Pair>) -> Result<(), Error> {
        if pairs.len() < 2 || !self.may_reread(pairs) {
            pairs.clear();
            return Ok(());
        }
        let first = pairs[0];
        let last = pairs.last().ok_or(Error::Limit)?.store;
        let size = pairs
            .iter()
            .fold(0_u64, |size, p| size.wrapping_add(p.bytes));
        let align = u8::try_from(first.bytes.trailing_zeros()).map_err(|_| Error::Limit)?;
        let id = self.context.body.append(
            block,
            Instruction::new(
                None,
                I::CopyObject(size, align, first.source, first.destination),
                0,
            ),
        )?;
        self.context.body.inherit_location(id, last);
        self.links.insert_after(block, Some(last), id)?;
        let mut after = id;
        self.dead
            .resize(self.context.body.instructions().len(), false);
        for pair in pairs.iter() {
            self.dead[pair.store.index()] = true;
            let result = self
                .context
                .body
                .instruction(pair.load)
                .result
                .ok_or(Error::Limit)?;
            self.dead[pair.load.index()] = true;
            if self.uses[result.index()] == 1 {
                continue;
            }
            let mut reread = self.context.body.instruction(pair.load).clone();
            let I::Load(flags, ty, _) = reread.kind else {
                return Err(Error::Limit);
            };
            let I::Store(store, _, _, _) = self.context.body.instruction(pair.store).kind else {
                return Err(Error::Limit);
            };
            reread.kind = I::Load(
                Facts::memory(flags.volatile(), store.typed()),
                ty,
                pair.destination,
            );
            let at = self.context.body.append(block, reread)?;
            self.links.insert_after(block, Some(after), at)?;
            after = at;
            self.dead
                .resize(self.context.body.instructions().len(), false);
        }
        self.merges = self.merges.checked_add(1).ok_or(Error::Limit)?;
        self.changed = true;
        self.removed_store()?;
        pairs.clear();
        Ok(())
    }
    fn skip_pure(&self, mut at: Option<InstId>) -> Option<InstId> {
        while let Some(id) = at {
            let kind = &self.context.body.instruction(id).kind;
            if !super::pure(kind) || matches!(kind, I::Phi(..)) {
                break;
            }
            at = self.links.next(id);
        }
        at
    }
    fn pair(&self, load: InstId, store: InstId) -> Option<Pair> {
        let instruction = self.context.body.instruction(load);
        let I::Load(flags, ty, source) = instruction.kind else {
            return None;
        };
        let I::Store(stored, store_ty, Value::Temp(value), destination) =
            self.context.body.instruction(store).kind
        else {
            return None;
        };
        if flags.volatile()
            || stored.volatile()
            || store_ty != ty
            || ty.bits() == 0 && ty != Type::Ptr
            || instruction.result != Some(value)
            || self.uses[value.index()] < 1
        {
            return None;
        }
        let from = rooted(self.context.body, source)?;
        let to = rooted(self.context.body, destination)?;
        self.disjoint(from.root, to.root).then_some(Pair {
            load,
            store,
            from,
            to,
            source,
            destination,
            bytes: bytes(ty),
        })
    }
    fn load_group(&self, first: InstId) -> Option<Vec<Pair>> {
        let mut loads = vec![first];
        let mut next = self.links.next(first);
        loop {
            next = self.skip_pure(next);
            let Some(id) = next else {
                break;
            };
            if loads.len() == 4 || !matches!(self.context.body.instruction(id).kind, I::Load(..)) {
                break;
            }
            loads.push(id);
            next = self.links.next(id);
        }
        let mut pairs = Vec::with_capacity(loads.len());
        for (index, load) in loads.into_iter().enumerate() {
            if index != 0 {
                next = self.skip_pure(next);
            }
            let store = next?;
            pairs.push(self.pair(load, store)?);
            next = self.links.next(store);
        }
        Some(pairs)
    }
    fn extends(first: Pair, last: Pair, pair: Pair) -> bool {
        last.bytes == pair.bytes
            && last.from.root == pair.from.root
            && last.to.root == pair.to.root
            && pair.from.offset == last.from.offset.wrapping_add(signed_size(last.bytes))
            && pair.to.offset == last.to.offset.wrapping_add(signed_size(last.bytes))
            && pair.from.offset.wrapping_sub(first.from.offset)
                == pair.to.offset.wrapping_sub(first.to.offset)
    }
    fn merge_block(&mut self, block: BlockId) -> Result<(), Error> {
        let before = self.merges;
        let mut run = Vec::new();
        let mut cursor = self.links.first(block);
        while let Some(id) = cursor {
            let kind = &self.context.body.instruction(id).kind;
            if super::pure(kind) && !matches!(kind, I::Phi(..)) {
                cursor = self.links.next(id);
                continue;
            }
            let group = if matches!(kind, I::Load(..)) {
                self.load_group(id)
            } else {
                None
            };
            let Some(group) = group else {
                self.flush(block, &mut run)?;
                cursor = self.links.next(id);
                continue;
            };
            for &pair in &group {
                if let Some(&last) = run.last()
                    && !Self::extends(run[0], last, pair)
                {
                    self.flush(block, &mut run)?;
                }
                run.push(pair);
            }
            cursor = self.links.next(group.last().ok_or(Error::Limit)?.store);
        }
        self.flush(block, &mut run)?;
        if self.merges == before {
            return Ok(());
        }
        let mut cursor = self.links.first(block);
        while let Some(id) = cursor {
            cursor = self.links.next(id);
            if self.dead.get(id.index()) == Some(&true) {
                self.links.unlink(block, id)?;
            }
        }
        Ok(())
    }
    pub(super) fn recount(&mut self) -> Result<(), Error> {
        self.links.write_links(self.context.body)?;
        self.context.body.recount_uses()?;
        self.uses = self.context.body.temps().iter().map(|t| t.uses).collect();
        Ok(())
    }
    fn count_slot_uses(&mut self) -> Result<(), Error> {
        self.slot_uses = vec![0; self.context.body.slots().len()];
        for &block in &self.context.body.layout {
            for id in self.links.ids(block) {
                let inst = self.context.body.instruction(id);
                if matches!(inst.kind, I::Addr(Value::Slot(_)))
                    || super::pure(&inst.kind)
                        && inst.result.is_some_and(|t| self.uses[t.index()] == 0)
                {
                    continue;
                }
                let mut overflow = false;
                self.context.body.visit_values(&inst.kind, |v| {
                    if let Some(Rooted {
                        root: Value::Slot(slot),
                        ..
                    }) = rooted(self.context.body, v)
                    {
                        let count = &mut self.slot_uses[slot.index()];
                        overflow |= count.checked_add(1).is_none();
                        *count = count.saturating_add(1);
                    }
                });
                if overflow {
                    return Err(Error::Limit);
                }
            }
        }
        Ok(())
    }
    fn run(&mut self) -> Result<bool, Error> {
        self.find_escapes();
        for block in self.context.body.layout.clone() {
            self.merge_block(block)?;
        }
        if self.merges != 0 {
            self.recount()?;
        }
        self.count_slot_uses()?;
        let mut forwarded = false;
        for block in self.context.body.layout.clone() {
            forwarded = self.forward(block)? || forwarded;
        }
        self.changed |= forwarded;
        if self.context.level >= 2 {
            if forwarded {
                self.recount()?;
            }
            self.changed = self.zero_slots()? || self.changed;
            self.changed = self.constant_slots()? || self.changed;
        }
        Ok(self.changed)
    }
}
pub(super) fn coalesce(context: &mut Context<'_>) -> Result<bool, Error> {
    context.prepare_uses(Pass::CoalesceCopies, true)?;
    let uses = context.entry_uses().to_vec();
    let slots = context.body.slots().len();
    let count = context.body.instructions().len();
    let links = LinkEditor::new(context.body)?;
    let mut pass = Copies {
        context,
        links,
        uses,
        slot_uses: Vec::new(),
        escaped: vec![false; slots],
        dead: vec![false; count],
        merges: 0,
        changed: false,
    };
    let changed = pass.run()?;
    pass.links.commit(pass.context.body)?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::super::pass_context::Pass;
    use std::fmt::Write as _;

    #[test]
    fn surviving_loads_reread_the_destination_with_store_alias_facts() {
        let source = "function @f()->i64 {slot $a : obj<16x8> slot $b : obj<16x8> block ^entry:%a = addr $a %b = addr $b %a1 = index i8 %a, 8 %b1 = index i8 %b, 8 %x = load i64 %a store typed i64 %x, %b %y = load i64 %a1 store typed i64 %y, %b1 return i64 %x}";
        let (output, facts) = super::super::pass_view(source, 1, Pass::CoalesceCopies);
        assert!(facts.0);
        assert!(output.contains("copyobj 16x8 %a, %b"));
        assert!(output.contains("%x = load typed i64 %b"));
        assert!(!output.contains("store typed"));
        let early = source.replace(
            "store typed i64 %x",
            "%early = copy i64 %x store typed i64 %x",
        );
        assert!(!super::super::pass_view(&early, 1, Pass::CoalesceCopies).1.0);
    }
    #[test]
    fn seven_pointer_steps_reach_a_root_but_eight_do_not() {
        for depth in [7, 8] {
            let mut source = String::from(
                "function @f()->void {slot $a : obj<16x8> slot $b : obj<16x8> block ^entry:",
            );
            let mut a = String::from("$a");
            let mut b = String::from("$b");
            for i in 0..depth {
                write!(source, "%a{i} = copy ptr {a} %b{i} = copy ptr {b} ").unwrap();
                a = format!("%a{i}");
                b = format!("%b{i}");
            }
            write!(source, "%x = load i64 {a} store i64 %x, {b} %aend = index i8 $a, 8 %bend = index i8 $b, 8 %y = load i64 %aend store i64 %y, %bend return void}}").unwrap();
            assert_eq!(
                super::super::pass_view(&source, 1, Pass::CoalesceCopies)
                    .1
                    .0,
                depth == 7
            );
        }
    }
    #[test]
    fn writes_block_storage_forwarding_but_not_an_object_value() {
        let source = "function @f(%v : obj<16x8>,%out : ptr)->void {slot $a : obj<16x8> slot $b : obj<16x8> block ^entry:copyobj 16x8 $a, $b store i64 1, %out copyobj 16x8 $b, %out return void}";
        assert!(!super::super::pass_view(source, 1, Pass::CoalesceCopies).1.0);
        let object = source.replace("16x8 $a, $b", "16x8 %v, $b");
        let (output, facts) = super::super::pass_view(&object, 1, Pass::CoalesceCopies);
        assert!(facts.0);
        assert!(output.contains("copyobj 16x8 %v, %out"));
    }
    #[test]
    fn zero_fill_preserves_the_level_and_sixty_four_byte_limits() {
        for size in [64, 65] {
            for level in [1, 2] {
                let source = format!(
                    "function @f()->void {{slot $a : obj<{size}x8> block ^entry:store i64 0, $a store i64 0, $a return void}}"
                );
                let (output, facts) = super::super::pass_view(&source, level, Pass::CoalesceCopies);
                assert_eq!(facts.0, level == 2 && size == 64);
                assert_eq!(output.contains("zeroinit"), level == 2 && size == 64);
            }
        }
    }
    #[test]
    fn constant_slot_replacement_keeps_variable_address_steps() {
        let source = "global @g [storage=readonly] = {zero 16}\nfunction @f(%n : i64)->i64 {slot $a : obj<16x8> block ^entry:%a = addr $a %p = index i8 %a, %n copyobj 16x8 @g, $a %x = load i64 %p return i64 %x}";
        let (output, facts) = super::super::pass_view(source, 2, Pass::CoalesceCopies);
        assert!(facts.0);
        assert!(!output.contains("copyobj"));
        assert!(output.contains("%a = addr @g"));
        assert!(output.contains("index i8 %a, %n"));
    }
}
