//! Dominator-scoped memory forwarding from `opt/memory_forwarding.cpp`. See NOTICE.
use super::{
    alias_facts::{self as facts, Address, Addresses, Contents, Key, Saved},
    definition_order,
    memory_contents::bytes,
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Facts, InstId, Instruction, InstructionKind as I, RuntimeInstruction as R, Value,
    },
    vocabulary::{Projection, Type},
};
use std::collections::VecDeque;

pub(super) struct MemoryPass<'a, 'p> {
    pub(super) context: &'a mut Context<'p>,
    pub(super) addresses: Addresses,
    pub(super) contents: Contents,
    pub(super) replacements: Vec<Option<Value>>,
    pub(super) changed: bool,
    replaced: bool,
    arm_marks: Vec<u32>,
    arm_epoch: u32,
}
impl<'a, 'p> MemoryPass<'a, 'p> {
    fn new(context: &'a mut Context<'p>) -> Self {
        let addresses = Addresses::analyze(context.body);
        let count = context.body.temps().len();
        Self {
            context,
            addresses,
            contents: Contents::new(),
            replacements: vec![None; count],
            changed: false,
            replaced: false,
            arm_marks: Vec::new(),
            arm_epoch: 0,
        }
    }
    pub(super) fn at(&self, value: Value) -> Address {
        self.addresses.facts(self.context.body, Some(value))
    }
    pub(super) fn key(&self, value: Value, ty: Type) -> Key {
        Key::new(self.at(value), Some(value), ty)
    }
    pub(super) fn reused_load(&mut self) -> Result<(), Error> {
        self.context.loads_reused = self
            .context
            .loads_reused
            .checked_add(1)
            .ok_or(Error::Limit)?;
        self.changed = true;
        Ok(())
    }
    pub(super) fn removed_store(&mut self) -> Result<(), Error> {
        self.context.stores_removed = self
            .context
            .stores_removed
            .checked_add(1)
            .ok_or(Error::Limit)?;
        self.changed = true;
        Ok(())
    }
    fn remember(
        &mut self,
        address: Value,
        ty: Type,
        value: Value,
        typed: bool,
    ) -> Result<(), Error> {
        self.contents
            .remember(self.at(address), address, ty, value, typed)
    }
    fn invalidate(&mut self, address: Option<Value>, size: u64, writer: Option<(bool, Type)>) {
        let at = self.addresses.facts(self.context.body, address);
        self.contents.invalidate(&self.addresses, at, size, writer);
    }
    fn resolve(replacements: &[Option<Value>], mut value: Value) -> Result<Value, Error> {
        let mut walked = 0;
        while let Value::Temp(temp) = value {
            let Some(next) = replacements.get(temp.index()).copied().flatten() else {
                break;
            };
            value = next;
            walked += 1;
            if walked > replacements.len() {
                return Err(Error::Construction("cyclic memory replacement".into()));
            }
        }
        Ok(value)
    }
    fn substitute(&mut self, id: InstId) -> Result<(), Error> {
        if !self.replaced {
            return Ok(());
        }
        let replacements = &self.replacements;
        self.context
            .body
            .rewrite_values(id, |value| Self::resolve(replacements, value))
    }

    fn load(&mut self, id: InstId) -> Result<bool, Error> {
        let mut instruction = self.context.body.instruction(id).clone();
        let I::Load(flags, ty, address) = instruction.kind else {
            return Err(Error::Limit);
        };
        let result = instruction.result.ok_or(Error::Limit)?;
        let integral = facts::integral(ty);
        let found = self
            .contents
            .find(self.key(address, ty))
            .and_then(|f| f.value);
        let known = if found.is_some() {
            found
        } else if integral && self.contents.known_zero(self.at(address), bytes(ty)) {
            Some(self.context.body.integer(0)?)
        } else {
            None
        };
        if let Some(value) = known {
            self.replacements[result.index()] = Some(value);
            self.replaced = true;
            self.reused_load()?;
            return Ok(false);
        }
        if integral
            && let Some(other) = self.contents.find(self.key(address, facts::twin(ty)))
            && let Some(value) = other.value
            && !(matches!(value, Value::Integer(_) | Value::Float(..))
                && (ty == Type::Ptr || other.ty == Type::Ptr))
        {
            instruction.reuse_load(value)?;
            self.context.body.update_instruction(id, instruction);
            self.remember(address, ty, Value::Temp(result), flags.typed())?;
            self.reused_load()?;
            return Ok(true);
        }
        self.remember(address, ty, Value::Temp(result), flags.typed())?;
        Ok(true)
    }
    fn store(&mut self, id: InstId) -> Result<bool, Error> {
        let I::Store(flags, ty, value, address) = self.context.body.instruction(id).kind else {
            return Err(Error::Limit);
        };
        let size = bytes(ty);
        let zero = facts::integral(ty)
            && matches!(value,Value::Integer(id) if self.context.body.literal(id)==0);
        if !flags.volatile()
            && self.context.level >= 2
            && (zero && self.contents.known_zero(self.at(address), size)
                || self
                    .contents
                    .find(self.key(address, ty))
                    .is_some_and(|f| f.value == Some(value)))
        {
            self.removed_store()?;
            return Ok(false);
        }
        self.invalidate(Some(address), size, Some((flags.typed(), ty)));
        if facts::exact(self.context.body, value, ty) && !flags.volatile() {
            self.remember(address, ty, value, flags.typed())?;
        }
        if zero && !flags.volatile() {
            self.contents.note_zero(self.at(address), size);
        }
        Ok(true)
    }
    fn process(&mut self, block: BlockId) -> Result<(), Error> {
        let mut pending: VecDeque<_> = self.context.body.block(block).instructions.clone().into();
        let mut kept = Vec::with_capacity(pending.len());
        while let Some(id) = pending.pop_front() {
            self.substitute(id)?;
            let kind = self.context.body.instruction(id).kind.clone();
            let keep = match kind {
                I::Load(flags, ..) if !flags.volatile() => self.load(id)?,
                I::Store(..) => self.store(id)?,
                I::Load(..) | I::Runtime(R::Exception(_) | R::ExceptionSelector(_)) => true,
                I::CopyObject(size, _, _, destination) => {
                    if let Some(sequence) = self.expand_copy(block, id)? {
                        for id in sequence.into_iter().rev() {
                            pending.push_front(id);
                        }
                        continue;
                    }
                    self.invalidate(Some(destination), size, None);
                    true
                }
                I::ZeroInit(size, _, address) => {
                    self.invalidate(Some(address), size, None);
                    self.contents.note_zero(self.at(address), size);
                    true
                }
                _ => {
                    if facts::writes(self.context.program, &kind) && !facts::marker(&kind) {
                        self.invalidate(None, i64::MAX.unsigned_abs(), None);
                    }
                    true
                }
            };
            if keep {
                kept.push(id);
            }
        }
        self.context.body.replace_links(block, kept)
    }
    pub(super) fn diamond_arms(&mut self, block: BlockId) -> Result<Option<Vec<BlockId>>, Error> {
        let cfg = self.context.cfg()?;
        let dom = cfg.dominator(block);
        if cfg.predecessors(block).is_empty() {
            return Ok(None);
        }
        self.arm_marks.resize(self.context.body.blocks().len(), 0);
        self.arm_epoch = self.arm_epoch.wrapping_add(1);
        let mut arms = Vec::new();
        let mut from = block;
        let mut walked = 0;
        loop {
            if cfg.predecessors(from).is_empty() {
                return Ok(None);
            }
            for &pred in cfg.predecessors(from) {
                if Some(pred) == dom || self.arm_marks[pred.index()] == self.arm_epoch {
                    continue;
                }
                if arms.len() == 32 || cfg.handler(pred) {
                    return Ok(None);
                }
                self.arm_marks[pred.index()] = self.arm_epoch;
                arms.push(pred);
            }
            if walked == arms.len() {
                return Ok(Some(arms));
            }
            from = arms[walked];
            walked += 1;
        }
    }
    fn replay(&mut self, arms: &[BlockId]) {
        for &block in arms {
            let ids = self.context.body.block(block).instructions.clone();
            for id in ids {
                if self.contents.is_empty() {
                    return;
                }
                let kind = self.context.body.instruction(id).kind.clone();
                match kind {
                    I::Store(flags, ty, _, address) => {
                        self.invalidate(Some(address), bytes(ty), Some((flags.typed(), ty)));
                    }
                    I::CopyObject(size, _, _, address) | I::ZeroInit(size, _, address) => {
                        self.invalidate(Some(address), size, None);
                    }
                    I::Load(..) | I::Runtime(R::Exception(_) | R::ExceptionSelector(_)) => (),
                    _ => {
                        if !facts::marker(&kind) && facts::writes(self.context.program, &kind) {
                            self.invalidate(None, i64::MAX.unsigned_abs(), None);
                        }
                    }
                }
            }
        }
    }
    fn enter(&mut self, block: BlockId) -> Result<Saved, Error> {
        let saved = self.contents.save();
        let arms = if !self.contents.is_empty()
            && self.context.cfg()?.dominator(block).is_some()
            && !self.context.cfg()?.handler(block)
        {
            self.diamond_arms(block)?
        } else {
            None
        };
        if let Some(arms) = arms {
            self.replay(&arms);
        } else {
            self.contents.clear();
        }
        self.process(block)?;
        Ok(saved)
    }
    fn run(&mut self) -> Result<bool, Error> {
        let Some(&entry) = self.context.cfg()?.reverse_postorder().first() else {
            return Ok(false);
        };
        let saved = self.enter(entry)?;
        let mut stack = vec![(entry, 0_usize, saved)];
        while let Some((block, next, _)) = stack.last_mut() {
            if let Some(&child) = self.context.cfg()?.children(*block).get(*next) {
                *next += 1;
                let saved = self.enter(child)?;
                stack.push((child, 0, saved));
            } else {
                let (_, _, saved) = stack.pop().ok_or(Error::Limit)?;
                self.contents.restore(saved);
            }
        }
        if self.replaced {
            for block in self.context.body.layout.clone() {
                for id in self.context.body.block(block).instructions.clone() {
                    self.substitute(id)?;
                }
            }
        }
        self.drop_unread_writes()?;
        self.sink_stores()?;
        self.place_loads()?;
        Ok(self.changed)
    }
}
pub(super) fn forward(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    let changed = MemoryPass::new(context).run()?;
    if changed {
        definition_order::restore(context)?;
    }
    context.body.recount_uses()?;
    Ok(changed)
}

#[derive(Clone, Copy)]
enum CopiedValue {
    Existing(Value),
    Image(u64),
}
#[derive(Clone, Copy)]
struct Part {
    offset: u64,
    ty: Type,
    value: CopiedValue,
}
impl MemoryPass<'_, '_> {
    fn constant_image(&self, source: Address, size: usize) -> Result<Option<Vec<u8>>, Error> {
        let mut covered = vec![false; size];
        let mut image = vec![0_u8; size];
        let size_signed = i64::try_from(size).map_err(|_| Error::Limit)?;
        for range in self.contents.zeros() {
            if range.base.same_base(source) {
                let mut at = range.start.max(source.offset);
                let end = source.offset.wrapping_add(size_signed);
                while at < range.end && at < end {
                    let index = usize::try_from(at.wrapping_sub(source.offset))
                        .map_err(|_| Error::Limit)?;
                    let entry = covered.get_mut(index).ok_or_else(|| {
                        Error::Construction("constant-copy zero index outside image".into())
                    })?;
                    *entry = true;
                    at = at.wrapping_add(1);
                }
            }
        }
        for found in self.contents.live() {
            let Some(Value::Integer(literal)) = found.value else {
                continue;
            };
            if !facts::integral(found.ty) {
                continue;
            }
            let offset = found.at.offset.wrapping_sub(source.offset);
            let width = facts::signed_size(found.bytes);
            if !found.at.same_base(source) || offset < 0 || offset.wrapping_add(width) > size_signed
            {
                continue;
            }
            let low = facts::signed_word(self.context.body.literal(literal)).to_le_bytes();
            let offset = usize::try_from(offset).map_err(|_| Error::Limit)?;
            let width = usize::try_from(width).map_err(|_| Error::Limit)?;
            for (index, &byte) in low.iter().take(width).enumerate() {
                let position = offset.checked_add(index).ok_or(Error::Limit)?;
                let covered = covered.get_mut(position).ok_or_else(|| {
                    Error::Construction("constant-copy byte index outside image".into())
                })?;
                let output = image.get_mut(position).ok_or(Error::Limit)?;
                *covered = true;
                *output = byte;
            }
        }
        Ok(covered.iter().all(|&b| b).then_some(image))
    }
    fn image_parts(image: &[u8]) -> Result<Option<(bool, Vec<Part>)>, Error> {
        let mut narrow = Vec::new();
        let mut whole = Vec::new();
        for chunk in (0..image.len()).step_by(8) {
            let end = (chunk + 8).min(image.len());
            let mut low = 8;
            let mut high = 0;
            for (offset, &byte) in image[chunk..end].iter().enumerate() {
                if byte != 0 {
                    low = low.min(offset);
                    high = offset + 1;
                }
            }
            let mut at = chunk;
            let mut width = 8;
            while at < chunk + 8 && at < image.len() {
                if at + width <= image.len() {
                    whole.push((at, width));
                    at += width;
                }
                width /= 2;
            }
            if high == 0 {
                continue;
            }
            let mut width = 1;
            while low / width != (high - 1) / width {
                width *= 2;
            }
            let at = chunk + low / width * width;
            if at + width > image.len() {
                return Ok(None);
            }
            narrow.push((at, width));
        }
        let zeroing =
            !(whole.len() <= 4 && whole.len() < narrow.len() + image.len().div_ceil(16) + 1);
        if zeroing && narrow.len() > 8 {
            return Ok(None);
        }
        let mut parts = Vec::new();
        for (offset, width) in if zeroing { narrow } else { whole } {
            let mut raw = [0_u8; 8];
            raw[..width].copy_from_slice(&image[offset..offset + width]);
            let bits = u64::from_le_bytes(raw);
            let value = CopiedValue::Image(bits);
            let ty = match width {
                1 => Type::U8,
                2 => Type::U16,
                4 => Type::U32,
                _ => Type::I64,
            };
            parts.push(Part {
                offset: u64::try_from(offset).map_err(|_| Error::Limit)?,
                ty,
                value,
            });
        }
        Ok(Some((zeroing, parts)))
    }
    fn field_parts(&self, source: Address, size: u64) -> Option<Vec<Part>> {
        let mut parts = Vec::new();
        for found in self.contents.live() {
            let Some(value) = found.value else {
                continue;
            };
            if !matches!(value, Value::Integer(_) | Value::Temp(_))
                || !facts::integral(found.ty)
                || matches!(value,Value::Temp(t) if self.context.body.temp(t).ty!=Some(found.ty))
            {
                continue;
            }
            let offset = found.at.offset.wrapping_sub(source.offset);
            if !found.at.same_base(source)
                || offset < 0
                || offset.wrapping_add(facts::signed_size(found.bytes)) > facts::signed_size(size)
            {
                continue;
            }
            parts.push(Part {
                offset: u64::from_le_bytes(offset.to_le_bytes()),
                ty: found.ty,
                value: CopiedValue::Existing(value),
            });
        }
        parts.sort_unstable_by_key(|p| p.offset);
        let mut next = 0_u64;
        for part in &parts {
            if part.offset != next {
                return None;
            }
            next = next.wrapping_add(bytes(part.ty));
        }
        (next == size && parts.len() <= 4).then_some(parts)
    }
    fn copy_stores(
        &mut self,
        block: BlockId,
        id: InstId,
        destination: Value,
        parts: &[Part],
    ) -> Result<Vec<InstId>, Error> {
        let destination_facts = self.at(destination);
        let mut sequence = Vec::new();
        for part in parts {
            let mut address = destination;
            if part.offset != 0 {
                let offset = self
                    .context
                    .body
                    .integer(facts::literal_integer(facts::signed_size(part.offset)))?;
                let result = self.context.body.generated_temp(Type::Ptr)?;
                let at = self.context.body.append(
                    block,
                    Instruction::new(
                        Some(result),
                        I::Index(Type::I8, Some(Projection::Field), destination, offset),
                        0,
                    ),
                )?;
                self.context.body.inherit_location(at, id);
                sequence.push(at);
                self.addresses
                    .values
                    .resize(self.context.body.temps().len(), Address::default());
                self.replacements
                    .resize(self.context.body.temps().len(), None);
                if destination_facts.base.is_some() {
                    self.addresses.values[result.index()] =
                        destination_facts.shifted(facts::signed_size(part.offset));
                }
                address = Value::Temp(result);
            }
            let value = match part.value {
                CopiedValue::Existing(value) => value,
                CopiedValue::Image(bits) => self
                    .context
                    .body
                    .integer(facts::literal_integer(facts::signed_size(bits)))?,
            };
            let at = self.context.body.append(
                block,
                Instruction::new(None, I::Store(Facts::default(), part.ty, value, address), 0),
            )?;
            self.context.body.inherit_location(at, id);
            sequence.push(at);
        }
        Ok(sequence)
    }
    pub(super) fn expand_copy(
        &mut self,
        block: BlockId,
        id: InstId,
    ) -> Result<Option<Vec<InstId>>, Error> {
        let I::CopyObject(size, align, source, destination) =
            self.context.body.instruction(id).kind
        else {
            return Err(Error::Limit);
        };
        let source = self.at(source);
        if size == 0 || size > 256 || source.base.is_none() {
            return Ok(None);
        }
        if let Some(image) =
            self.constant_image(source, usize::try_from(size).map_err(|_| Error::Limit)?)?
            && let Some((zeroing, parts)) = Self::image_parts(&image)?
        {
            self.context
                .body
                .replace_kind(id, I::ZeroInit(size, align, destination));
            let mut sequence = if zeroing { vec![id] } else { Vec::new() };
            sequence.extend(self.copy_stores(block, id, destination, &parts)?);
            self.reused_load()?;
            return Ok(Some(sequence));
        }
        if size > 32 {
            return Ok(None);
        }
        let Some(parts) = self.field_parts(source, size) else {
            return Ok(None);
        };
        let sequence = self.copy_stores(block, id, destination, &parts)?;
        self.reused_load()?;
        Ok(Some(sequence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn transform(source: &str, level: u8) -> (String, bool, u64, u64) {
        let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
        let symbol = program
            .publications()
            .iter()
            .copied()
            .find(|&s| {
                program
                    .function(program.symbol(s))
                    .is_some_and(|f| f.body.is_some())
            })
            .unwrap();
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let (changed, loads, stores) = {
            let mut context = Context::new(&mut program, symbol, &mut body, level, &[], &[]);
            let changed = context
                .run(super::super::pass_context::Pass::ForwardMemory)
                .unwrap();
            (changed, context.loads_reused, context.stores_removed)
        };
        body.compact(&mut program.names).unwrap();
        program.restore_body(symbol, body).unwrap();
        let mut text = Vec::new();
        crate::lowir::writer::write(&program, &mut text).unwrap();
        (String::from_utf8(text).unwrap(), changed, loads, stores)
    }
    #[test]
    fn a_wrapped_constant_copy_index_returns_a_structured_error() {
        let mut program=crate::lowir::test_program(b"function @f(%out : ptr)->void {slot $a : obj<1x1> block ^entry:%base = addr $a %at = index i8 %base, 9223372036854775807 store u8 7, %at zeroinit 1x1 %base copyobj 1x1 %base, %out return void}").unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&mut program, symbol, &mut body, 2, &[], &[]);
        assert!(
            matches!(forward(&mut context),Err(Error::Construction(message))if message=="constant-copy byte index outside image")
        );
    }
    #[test]
    fn forwarding_crosses_region_markers_but_never_reuses_volatile_loads() {
        let (text, _, loads, _) = transform(
            "function @f(%p : ptr, %v : i64) -> i64 {block ^entry: store i64 %v, %p eh_cleanup eh_end %r = load i64 %p return i64 %r}",
            1,
        );
        assert_eq!(loads, 1);
        assert!(text.contains("return i64 %v"));
        let (_, _, loads, _) = transform(
            "function @f(%p : ptr) -> i64 {block ^entry: %a = load volatile i64 %p %b = load volatile i64 %p return i64 %b}",
            1,
        );
        assert_eq!(loads, 0);
    }
    #[test]
    fn calls_preserve_private_slots_and_clobber_escaped_slots() {
        for escaped in [false, true] {
            let call = if escaped {
                "call void @g(%p) as (%a : ptr) -> void"
            } else {
                "call void @g()"
            };
            let source = format!(
                "declare function @g() -> void\nfunction @f(%v : i64) -> i64 {{slot $x : i64 block ^entry: %p = addr $x store i64 %v, $x {call} %r = load i64 $x return i64 %r}}"
            );
            let (text, _, loads, stores) = transform(&source, 1);
            assert_eq!(loads, u64::from(!escaped));
            assert_eq!(stores, u64::from(!escaped));
            assert_eq!(text.contains("load i64"), escaped);
        }
    }
    #[test]
    fn repeated_stores_keep_the_o1_policy() {
        let source = "function @f(%p : ptr, %v : i64) -> void {block ^entry: store i64 %v, %p store i64 %v, %p return void}";
        let (o1, _, _, removed) = transform(source, 1);
        assert_eq!(removed, 0);
        assert_eq!(o1.matches("store i64").count(), 2);
        let (o2, _, _, removed) = transform(source, 2);
        assert_eq!(removed, 1);
        assert_eq!(o2.matches("store i64").count(), 1);
    }
    #[test]
    fn typed_alias_separation_does_not_separate_a_known_union_base() {
        let source = "function @f(%p : ptr, %q : ptr, %v : ptr, %n : i64) -> ptr {block ^entry: store typed ptr %v, %p store typed i64 %n, %q %r = load typed ptr %p return ptr %r}";
        let (_, _, loads, _) = transform(source, 1);
        assert_eq!(loads, 1);
        let (_, _, loads, _) = transform(&source.replace("i64 %n, %q", "i64 %n, %p"), 1);
        assert_eq!(loads, 1); // The twin becomes a retyping copy, not the old pointer value.
        let (text, _, _, _) = transform(&source.replace("i64 %n, %q", "i64 %n, %p"), 1);
        assert!(text.contains("copy ptr %n"));
    }
    #[test]
    fn known_and_variable_copies_expand_in_frozen_field_order() {
        let source = "function @f(%out : ptr, %v : i64) -> void {slot $x : obj<16x8> block ^entry: %p = index i8 $x, 8 store i64 %v, $x store i64 42, %p copyobj 16x8 $x, %out return void}";
        let (text, _, loads, stores) = transform(source, 1);
        assert_eq!(loads, 1);
        assert_eq!(stores, 2);
        assert!(!text.contains("copyobj"));
        assert!(text.contains("store i64 %v, %out"));
        assert!(text.contains("[projection=field]"));
        let (text, _, loads, _) = transform(
            "function @f(%out : ptr) -> void {slot $x : obj<24x8> block ^entry: zeroinit 24x8 $x copyobj 24x8 $x, %out return void}",
            1,
        );
        assert_eq!(loads, 1);
        assert!(text.contains("zeroinit 24x8 %out"));
    }
    #[test]
    fn zero_ranges_preserve_both_sides_of_a_write() {
        let (text, _, loads, _) = transform(
            "function @f(%p : ptr, %v : i64) -> i64 {block ^entry: zeroinit 24x8 %p %middle = index i8 %p, 8 store i64 %v, %middle %left = load i64 %p %rightp = index i8 %p, 16 %right = load i64 %rightp %sum = binary add i64 %left, %right return i64 %sum}",
            1,
        );
        assert_eq!(loads, 2);
        assert!(!text.contains("load i64"));
    }
    #[test]
    fn phi_address_loads_keep_predecessor_order() {
        let (text, _, loads, _) = transform(
            "function @f(%c : i64, %p : ptr, %q : ptr, %a : i64, %b : i64) -> i64 {block ^entry: branch %c, ^left, ^right block ^left: store i64 %a, %p jump ^join block ^right: store i64 %b, %q jump ^join block ^join: %address = phi ptr [^right: %q, ^left: %p] %r = load i64 %address return i64 %r}",
            1,
        );
        assert_eq!(loads, 1);
        assert!(text.contains("%r = phi i64 [^right: %b, ^left: %a]"));
        assert!(!text.contains("load i64"));
    }
    #[test]
    fn partial_redundancy_inserts_only_the_missing_load() {
        let (text, _, loads, _) = transform(
            "function @f(%c : i64, %p : ptr, %v : i64) -> i64 {block ^entry: branch %c, ^left, ^right block ^left: store i64 %v, %p jump ^join block ^right: jump ^join block ^join: %r = load typed i64 %p return i64 %r}",
            1,
        );
        assert_eq!(loads, 1);
        assert_eq!(text.matches("load typed i64").count(), 1);
        assert!(text.contains("%r = phi i64"));
    }
    #[test]
    fn sinking_places_the_store_only_on_the_call_path() {
        let source = "declare function @g(%p : ptr) -> void\nfunction @f(%c : i64, %v : i64) -> void {slot $x : i64 block ^entry: store i64 %v, $x branch %c, ^use, ^exit block ^use: %p = addr $x call void @g(%p) return void block ^exit: return void}";
        let (text, _, _, stores) = transform(source, 2);
        assert_eq!(stores, 1);
        assert!(text.find("block ^use").unwrap() < text.find("store i64").unwrap());
    }
    #[test]
    fn a_retyped_load_retains_its_alias_proof_through_compaction() {
        let mut program=crate::lowir::test_program(b"function @f(%p : ptr, %v : i64) -> ptr {block ^entry: store typed i64 %v, %p %r = load typed ptr %p return ptr %r}").unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&mut program, symbol, &mut body, 1, &[], &[]);
        assert!(
            context
                .run(super::super::pass_context::Pass::ForwardMemory)
                .unwrap()
        );
        let flags = context
            .body
            .instructions()
            .iter()
            .find(|i| matches!(i.kind, I::Copy(..)))
            .unwrap()
            .copy_memory_facts();
        assert!(flags.typed());
        body.compact(&mut program.names).unwrap();
        assert!(
            body.instructions()
                .iter()
                .find(|i| matches!(i.kind, I::Copy(..)))
                .unwrap()
                .copy_memory_facts()
                .typed()
        );
    }
}
