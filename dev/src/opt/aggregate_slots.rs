//! Slot access/escape facts and splitting from `opt/aggregate_slots.cpp`. See NOTICE.
use super::{
    alias_facts::{signed_size, signed_word},
    memory_contents::bytes,
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{Call, InstId, InstructionKind as I, OptimizerSlot, SlotId, Value},
    vocabulary::{Pass, Type},
};

#[derive(Clone, Copy)]
pub(super) struct Address {
    pub(super) slot: SlotId,
    pub(super) offset: i64,
}
#[derive(Clone, Copy)]
pub(super) struct Field {
    pub(super) offset: u64,
    pub(super) ty: Type,
    pub(super) packed: bool,
}
#[derive(Default)]
pub(super) struct SlotFacts {
    pub(super) escaped: bool,
    pub(super) call_written: bool,
    pub(super) copied_out: bool,
    pub(super) fields: Vec<Field>,
    pub(super) spans: Vec<(u64, u64)>,
    pub(super) stores: Vec<(u64, u64)>,
    pub(super) reads: Vec<(u64, u64)>,
    pub(super) writes: Vec<(u64, u64)>,
    pub(super) wholes: Vec<(u64, bool)>,
    pub(super) packed: u64,
    pub(super) forced: u64,
}
#[derive(Clone, Copy)]
pub(super) struct SlotCopy {
    pub(super) from: Address,
    pub(super) to: Address,
    pub(super) bytes: u64,
}
pub(super) struct Aggregate<'a, 'p> {
    pub(super) context: &'a mut Context<'p>,
    pub(super) addresses: Vec<Option<Address>>,
    pub(super) facts: Vec<SlotFacts>,
    pub(super) copies: Vec<SlotCopy>,
    /// Copy encounters indexed by each original slot, in discovery order.
    pub(super) partners: Vec<Vec<usize>>,
    /// Original slot -> allocated scalar field identities, or retained storage.
    pub(super) fields: Vec<Option<Vec<SlotId>>>,
}
pub(super) fn unsigned(value: i64) -> u64 {
    u64::from_le_bytes(value.to_le_bytes())
}
pub(super) fn filler(width: u64) -> Type {
    match width {
        1 => Type::U8,
        2 => Type::U16,
        4 => Type::U32,
        _ => Type::I64,
    }
}
impl Aggregate<'_, '_> {
    pub(super) fn address(&self, value: Value) -> Option<Address> {
        let Value::Temp(temp) = value else {
            return None;
        };
        self.addresses.get(temp.index()).copied().flatten()
    }
    pub(super) fn storage(&self, value: Value) -> Option<Address> {
        if let Value::Slot(slot) = value {
            Some(Address { slot, offset: 0 })
        } else {
            self.address(value)
        }
    }
    fn escape(&mut self, value: Value) {
        if let Some(at) = self.storage(value) {
            self.facts[at.slot.index()].escaped = true;
        }
    }
    fn access(&mut self, value: Value, ty: Type, store: bool) {
        let Some(at) = self.storage(value) else {
            return;
        };
        let facts = &mut self.facts[at.slot.index()];
        if matches!(ty, Type::Object { .. }) || at.offset < 0 {
            facts.escaped = true;
            return;
        }
        let offset = unsigned(at.offset);
        facts.fields.push(Field {
            offset,
            ty,
            packed: false,
        });
        if ty == Type::I64 && at.offset % 8 == 0 {
            facts.wholes.push((offset, !store));
        }
        if store {
            let span = (offset, offset.wrapping_add(bytes(ty)));
            facts.stores.push(span);
            facts.writes.push(span);
        }
    }
    fn span(&mut self, value: Value, size: u64, read: bool) {
        let Some(at) = self.storage(value) else {
            return;
        };
        let facts = &mut self.facts[at.slot.index()];
        if at.offset < 0 {
            facts.escaped = true;
            return;
        }
        let span = (unsigned(at.offset), unsigned(at.offset).wrapping_add(size));
        facts.spans.push(span);
        if read {
            facts.reads.push(span);
        } else {
            facts.writes.push(span);
        }
    }
    fn copy(&mut self, size: u64, source: Value, dest: Value) {
        if let Value::Temp(temp) = source
            && self
                .context
                .body
                .temp(temp)
                .ty
                .is_some_and(|ty| matches!(ty, Type::Object { .. }))
        {
            if self.context.body.temp_is_parameter(temp) {
                self.escape(dest);
            } else {
                self.span(dest, size, false);
            }
            return;
        }
        self.span(source, size, true);
        self.span(dest, size, false);
        if let Some(at) = self.storage(source) {
            self.facts[at.slot.index()].copied_out = true;
        }
        if let (Some(from), Some(to)) = (self.storage(source), self.storage(dest))
            && from.slot != to.slot
        {
            let index = self.copies.len();
            self.partners[from.slot.index()].push(index);
            self.partners[to.slot.index()].push(index);
            self.copies.push(SlotCopy {
                from,
                to,
                bytes: size,
            });
        }
    }
    fn call(&mut self, call: &Call) {
        self.escape(call.callee);
        let passes = self
            .context
            .program
            .call_boundary(call)
            .map(|s| s.parameters.iter().map(|p| p.pass).collect::<Vec<_>>())
            .unwrap_or_default();
        for (index, value) in self
            .context
            .body
            .arguments(call.arguments)
            .to_vec()
            .into_iter()
            .enumerate()
        {
            let result = self.address(value).filter(|at| {
                passes.get(index) == Some(&Some(Pass::IndirectResult))
                    && at.offset == 0
                    && matches!(self.context.body.slot(at.slot).ty, Type::Object { .. })
            });
            if let Some(at) = result {
                self.span(value, bytes(self.context.body.slot(at.slot).ty), false);
                self.facts[at.slot.index()].call_written = true;
            } else {
                self.escape(value);
            }
        }
    }
    fn instruction(&mut self, id: InstId) -> Result<(), Error> {
        let instruction = self.context.body.instruction(id).clone();
        let mut modeled = true;
        match instruction.kind {
            I::Addr(value) => {
                if let Value::Slot(slot) = value {
                    self.addresses[instruction.result.ok_or(Error::Limit)?.index()] =
                        Some(Address { slot, offset: 0 });
                }
            }
            I::Copy(Type::Ptr, value) if self.address(value).is_some() => {
                self.addresses[instruction.result.ok_or(Error::Limit)?.index()] =
                    self.address(value);
            }
            I::Index(ty, _, base, Value::Integer(literal)) if self.address(base).is_some() => {
                let mut at = self.address(base).ok_or(Error::Limit)?;
                at.offset = at.offset.wrapping_add(
                    signed_word(self.context.body.literal(literal))
                        .wrapping_mul(signed_size(bytes(ty))),
                );
                self.addresses[instruction.result.ok_or(Error::Limit)?.index()] = Some(at);
            }
            I::Load(flags, ty, storage) if !flags.volatile() => self.access(storage, ty, false),
            I::Store(flags, ty, value, storage) if !flags.volatile() => {
                self.escape(value);
                self.access(storage, ty, true);
            }
            I::CopyObject(size, _, source, dest) => self.copy(size, source, dest),
            I::ZeroInit(size, _, dest) => self.span(dest, size, false),
            I::Phi(..) => (), // Address producers may occur later in layout.
            I::Call(ref call) => self.call(call),
            _ => modeled = false,
        }
        if !modeled {
            let mut values = Vec::new();
            self.context
                .body
                .visit_values(&instruction.kind, |v| values.push(v));
            for value in values {
                self.escape(value);
            }
        }
        Ok(())
    }
    fn analyze(&mut self) -> Result<(), Error> {
        for block in self.context.body.layout.clone() {
            for id in self.context.body.block(block).instructions.clone() {
                self.instruction(id)?;
            }
        }
        for block in self.context.body.layout.clone() {
            for id in self.context.body.block(block).instructions.clone() {
                let I::Phi(_, pairs) = &self.context.body.instruction(id).kind else {
                    break;
                };
                let values = pairs.iter().map(|&(_, v)| v).collect::<Vec<_>>();
                for value in values {
                    self.escape(value);
                }
            }
        }
        Ok(())
    }
    fn choose(&mut self) -> Result<bool, Error> {
        let mut settled = vec![false; self.facts.len()];
        for (index, settled) in settled.iter_mut().enumerate() {
            if self.facts[index].escaped
                || self.facts[index].call_written && self.facts[index].copied_out
            {
                continue;
            }
            let slot = self.context.body.slot_id(index).ok_or(Error::Limit)?;
            let ty = self.context.body.slot(slot).ty;
            // Hints read only copy partners, never the slot being settled.
            // Restore even a refused candidate's partially settled facts.
            let mut facts = std::mem::take(&mut self.facts[index]);
            let accepted = self.settle(&mut facts, bytes(ty), slot);
            self.facts[index] = facts;
            if !accepted || !matches!(ty, Type::Object { .. }) && self.context.level < 2 {
                continue;
            }
            *settled = true;
            if self.context.level >= 2 && matches!(ty, Type::Object { .. }) {
                self.facts[index].packed =
                    super::field_layout::packable(&self.facts[index], bytes(ty));
            }
        }
        self.agree_packing(&settled);
        let mut any = false;
        for (index, &accepted) in settled.iter().enumerate() {
            if !accepted || self.facts[index].forced & !self.facts[index].packed != 0 {
                continue;
            }
            super::field_layout::pack(&mut self.facts[index]);
            let mut fields = Vec::new();
            for field in &self.facts[index].fields {
                let Value::Slot(slot) = self
                    .context
                    .body
                    .optimizer_slot(field.ty, OptimizerSlot::Field)?
                else {
                    return Err(Error::Limit);
                };
                fields.push(slot);
            }
            self.fields[index] = Some(fields);
            any = true;
        }
        Ok(any)
    }
    pub(super) fn split_storage(&self, value: Value) -> Option<Address> {
        self.storage(value).filter(|at| {
            self.fields
                .get(at.slot.index())
                .is_some_and(Option::is_some)
        })
    }
    pub(super) fn field(&self, slot: SlotId, offset: u64) -> Option<Value> {
        let index = self.facts[slot.index()]
            .fields
            .iter()
            .position(|f| f.offset == offset)?;
        self.fields[slot.index()]
            .as_ref()?
            .get(index)
            .copied()
            .map(Value::Slot)
    }
    fn same_fields(&self, from: Address, to: Address, size: u64) -> bool {
        let inside = |at: Address, field: &Field| {
            field.offset >= unsigned(at.offset)
                && field.offset.wrapping_add(bytes(field.ty))
                    <= unsigned(at.offset).wrapping_add(size)
        };
        let first = &self.facts[from.slot.index()].fields;
        let second = &self.facts[to.slot.index()].fields;
        let count = first.iter().filter(|f| inside(from, f)).count();
        let mut found = 0;
        for field in second {
            if !inside(to, field) {
                continue;
            }
            found += 1;
            let offset = field
                .offset
                .wrapping_sub(unsigned(to.offset))
                .wrapping_add(unsigned(from.offset));
            if !first.iter().any(|f| f.offset == offset && f.ty == field.ty) {
                return false;
            }
        }
        count == found
    }
    fn compatible(&mut self) {
        // Includes self copies, which the filler-hint copy list omits.
        loop {
            let mut changed = false;
            for block in self.context.body.layout.clone() {
                for &id in &self.context.body.block(block).instructions {
                    let I::CopyObject(size, _, source, dest) =
                        self.context.body.instruction(id).kind
                    else {
                        continue;
                    };
                    let (Some(from), Some(to)) =
                        (self.split_storage(source), self.split_storage(dest))
                    else {
                        continue;
                    };
                    if from.slot == to.slot || !self.same_fields(from, to, size) {
                        self.fields[from.slot.index()] = None;
                        self.fields[to.slot.index()] = None;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }
}
pub(super) fn split(context: &mut Context<'_>) -> Result<bool, Error> {
    if context.body.slots().is_empty() {
        return Ok(false);
    }
    let slots = context.body.slots().len();
    let temps = context.body.temps().len();
    let mut pass = Aggregate {
        context,
        addresses: vec![None; temps],
        facts: (0..slots).map(|_| SlotFacts::default()).collect(),
        copies: Vec::new(),
        partners: vec![Vec::new(); slots],
        fields: vec![None; slots],
    };
    pass.analyze()?;
    if !pass.choose()? {
        return Ok(false);
    }
    pass.compatible();
    if !pass.fields.iter().any(Option::is_some) {
        return Ok(false);
    }
    pass.rewrite()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::pass_context::Pass as FunctionPass;
    #[test]
    fn object_fields_split_at_o1_but_addressed_scalars_wait_until_o2() {
        let object = "function @f(%v : i64)->i64 {slot $s : obj<8x8> block ^entry:%p = addr $s store i64 %v, %p %x = load i64 %p return i64 %x}";
        let (output, facts) = super::super::pass_view(object, 1, FunctionPass::SplitSlots);
        assert!(facts.0);
        assert!(output.contains("store i64 %v, $field__1"));
        let scalar = object.replace("obj<8x8>", "i64");
        assert!(
            !super::super::pass_view(&scalar, 1, FunctionPass::SplitSlots)
                .1
                .0
        );
        assert!(
            super::super::pass_view(&scalar, 2, FunctionPass::SplitSlots)
                .1
                .0
        );
    }
    #[test]
    fn copied_byte_fields_pack_only_at_the_original_optimization_level() {
        let source = "function @f(%in : ptr)->u8 {slot $s : obj<8x8> block ^entry:%p = addr $s %q = index i8 %p, 1 copyobj 8x8 %in, %p %x = load u8 %q return u8 %x}";
        let (plain, _) = super::super::pass_view(source, 1, FunctionPass::SplitSlots);
        let (packed, _) = super::super::pass_view(source, 2, FunctionPass::SplitSlots);
        assert!(!plain.contains("binary ushr"));
        assert!(packed.contains("binary ushr i64"));
        assert!(packed.contains("convert trunc u8 i64"));
    }
    #[test]
    fn whole_f80_spans_keep_padding_in_memory() {
        let source = "function @f(%out : ptr)->f80 {slot $s : obj<16x16> block ^entry:%p = addr $s zeroinit 16x16 %p %x = load f80 %p copyobj 16x16 %p, %out return f80 %x}";
        let (_, facts) = super::super::pass_view(source, 3, FunctionPass::SplitSlots);
        assert!(!facts.0);
    }
    #[test]
    fn filler_budget_rejects_the_seventeenth_eightbyte() {
        for size in [128, 129] {
            let source = format!(
                "function @f()->void {{slot $s : obj<{size}x1> block ^entry:zeroinit {size}x1 $s return void}}"
            );
            assert_eq!(
                super::super::pass_view(&source, 1, FunctionPass::SplitSlots)
                    .1
                    .0,
                size == 128
            );
        }
    }
    #[test]
    fn returned_objects_use_their_own_storage_before_field_extraction() {
        let source = "declare function @g()->obj<8x8>\nfunction @f()->i64 {slot $s : obj<8x8> block ^entry:%p = addr $s %v = call obj<8x8> @g() copyobj 8x8 %v, %p %x = load i64 %p return i64 %x}";
        let (output, facts) = super::super::pass_view(source, 1, FunctionPass::SplitSlots);
        assert!(facts.0);
        assert!(output.contains("$aggregate__"));
        assert!(output.contains("copyobj 8x8 %v"));
    }
    #[test]
    fn indirect_result_fields_are_copied_after_the_call() {
        let source = "declare function @g(%out : ptr [pass=indirect_result])->void\nfunction @f()->i64 {slot $s : obj<8x8> block ^entry:%p = addr $s call void @g(%p) %x = load i64 %p return i64 %x}";
        let (output, facts) = super::super::pass_view(source, 2, FunctionPass::SplitSlots);
        assert!(facts.0);
        assert!(output.contains("$aggregate__"));
        assert!(output.find("call void").unwrap() < output.find("store i64").unwrap());
    }
}
