//! Function-store rewrites and compaction from `opt/function_ir.cpp`. See NOTICE.
use super::{
    Arguments, Block, BlockId, BlockName, Body, Error, InstructionKind, LiteralId, NameId, Names,
    RuntimeInstruction, Slot, SlotId, TempId, Type, Value,
};
use crate::lowir::construction_names::SlotName;
use crate::lowir::literals::Constant;
use std::collections::HashSet;

pub(super) trait Remap {
    fn value(&mut self, value: Value) -> Result<Value, Error>;
    fn block(&mut self, block: BlockId) -> Result<BlockId, Error>;
    fn arguments(&mut self, arguments: Arguments) -> Result<Arguments, Error>;
}
pub(super) fn rewrite(kind: &mut InstructionKind, map: &mut impl Remap) -> Result<(), Error> {
    use InstructionKind as I;
    match kind {
        I::Const(_, v)
        | I::Copy(_, v)
        | I::Unary(_, _, v)
        | I::Convert(_, _, _, v)
        | I::Addr(v)
        | I::Load(_, _, v)
        | I::ZeroInit(_, _, v)
        | I::Return(_, Some(v)) => {
            *v = map.value(*v)?;
        }
        I::Binary(_, _, _, a, b)
        | I::Compare(_, _, a, b)
        | I::Store(_, _, a, b)
        | I::Index(_, _, a, b)
        | I::CopyObject(_, _, a, b) => {
            *a = map.value(*a)?;
            *b = map.value(*b)?;
        }
        I::Select(_, a, b, c) => {
            *a = map.value(*a)?;
            *b = map.value(*b)?;
            *c = map.value(*c)?;
        }
        I::Phi(_, pairs) => {
            for (block, value) in pairs {
                *block = map.block(*block)?;
                *value = map.value(*value)?;
            }
        }
        I::Call(call) => {
            call.callee = map.value(call.callee)?;
            call.arguments = map.arguments(call.arguments)?;
        }
        I::Runtime(runtime) => rewrite_runtime(runtime, map)?,
        I::Jump(block) => *block = map.block(*block)?,
        I::Branch(value, yes, no) => {
            *value = map.value(*value)?;
            *yes = map.block(*yes)?;
            *no = map.block(*no)?;
        }
        I::Switch(value, fallback, pairs) => {
            *value = map.value(*value)?;
            *fallback = map.block(*fallback)?;
            for (value, block) in pairs {
                *value = map.value(*value)?;
                *block = map.block(*block)?;
            }
        }
        I::Return(_, None) | I::Unreachable => (),
    }
    Ok(())
}
fn rewrite_runtime(runtime: &mut RuntimeInstruction, map: &mut impl Remap) -> Result<(), Error> {
    use RuntimeInstruction as R;
    match runtime {
        R::AtomicLoad(_, value, _)
        | R::VaStart(value)
        | R::VaArg(_, value)
        | R::StackAlloc(value)
        | R::Throw(_, value) => *value = map.value(*value)?,
        R::AtomicStore(_, a, b, _) | R::AtomicUpdate(_, _, a, b, _) => {
            *a = map.value(*a)?;
            *b = map.value(*b)?;
        }
        R::AtomicCompareExchange {
            address,
            expected,
            desired,
            ..
        } => {
            *address = map.value(*address)?;
            *expected = map.value(*expected)?;
            *desired = map.value(*desired)?;
        }
        R::EhTry(block) | R::EhCleanup(Some(block)) => *block = map.block(*block)?,
        R::EhFilter(arguments, _) => *arguments = map.arguments(*arguments)?,
        R::EhCatch(symbol, _) => {
            let Value::Symbol(replacement) = map.value(Value::Symbol(*symbol))? else {
                return Err(Error::Construction(
                    "catch type information must name a symbol".into(),
                ));
            };
            *symbol = replacement;
        }
        R::Fence(..)
        | R::Exception(_)
        | R::ExceptionSelector(_)
        | R::EhCleanup(None)
        | R::EhCatchAll(_)
        | R::EhEnd
        | R::Resume => (),
    }
    Ok(())
}

impl Body {
    pub(crate) fn update_instruction(
        &mut self,
        id: super::InstId,
        instruction: super::Instruction,
    ) {
        self.instructions[id.index()] = instruction;
    }
    pub(crate) fn retype_load(&mut self, id: super::InstId, ty: Type) -> Result<(), Error> {
        let instruction = &mut self.instructions[id.index()];
        let InstructionKind::Load(_, from, _) = &mut instruction.kind else {
            return Err(Error::Limit);
        };
        *from = ty;
        let temp = instruction.result.ok_or(Error::Limit)?;
        self.temps[temp.index()].ty = Some(ty);
        Ok(())
    }
    pub(crate) fn rewrite_values(
        &mut self,
        id: super::InstId,
        rewrite_value: impl FnMut(Value) -> Result<Value, Error>,
    ) -> Result<(), Error> {
        let mut map = ValueRewrite {
            operands: &mut self.operands,
            rewrite_value,
        };
        rewrite(&mut self.instructions[id.index()].kind, &mut map)
    }
    pub(crate) fn replace_kind(&mut self, id: super::InstId, kind: InstructionKind) {
        let instruction = &mut self.instructions[id.index()];
        instruction.kind = kind;
        instruction.switch_type = None;
    }
    pub(crate) fn replace_links(
        &mut self,
        block: BlockId,
        instructions: Vec<super::InstId>,
    ) -> Result<(), Error> {
        if instructions
            .iter()
            .any(|id| id.index() >= self.instructions.len())
        {
            return Err(Error::Limit);
        }
        self.blocks[block.index()].instructions = instructions;
        Ok(())
    }
    pub(crate) fn retain_layout(&mut self, dead: &[bool]) -> Result<(), Error> {
        if dead.len() != self.blocks.len() {
            return Err(Error::Limit);
        }
        self.layout.retain(|id| !dead[id.index()]);
        for (block, &dead) in self.blocks.iter_mut().zip(dead) {
            if dead {
                block.placed = false;
            }
        }
        Ok(())
    }
    /// Absorb a successor after omitting its leading phis and our final jump.
    pub(crate) fn join_blocks(&mut self, a: BlockId, b: BlockId, phis: usize) -> Result<(), Error> {
        if a == b {
            return Err(Error::Limit);
        }
        let (a, b) = if a.index() < b.index() {
            let (left, right) = self.blocks.split_at_mut(b.index());
            (&mut left[a.index()], &mut right[0])
        } else {
            let (left, right) = self.blocks.split_at_mut(a.index());
            (&mut right[0], &mut left[b.index()])
        };
        if a.instructions.is_empty() || phis > b.instructions.len() {
            return Err(Error::Limit);
        }
        a.instructions.pop();
        b.instructions.drain(..phis);
        a.instructions.append(&mut b.instructions);
        Ok(())
    }
    pub(crate) fn remove_phi_predecessor(&mut self, block: BlockId, pred: BlockId) {
        for &id in &self.blocks[block.index()].instructions {
            let InstructionKind::Phi(_, pairs) = &mut self.instructions[id.index()].kind else {
                break;
            };
            if let Some(index) = pairs.iter().position(|&(b, _)| b == pred) {
                pairs.remove(index);
            }
        }
    }
    pub(crate) fn rename_phi_predecessor(&mut self, block: BlockId, from: BlockId, to: BlockId) {
        for &id in &self.blocks[block.index()].instructions {
            let InstructionKind::Phi(_, pairs) = &mut self.instructions[id.index()].kind else {
                break;
            };
            for (block, _) in pairs {
                if *block == from {
                    *block = to;
                }
            }
        }
    }
    pub(crate) fn append_phi_predecessor(
        &mut self,
        id: super::InstId,
        block: BlockId,
        value: Value,
    ) -> Result<(), Error> {
        let InstructionKind::Phi(_, pairs) = &mut self.instructions[id.index()].kind else {
            return Err(Error::Limit);
        };
        if pairs.len() >= (u32::MAX / 2) as usize {
            return Err(Error::Limit);
        }
        pairs.push((block, value));
        Ok(())
    }
    pub(crate) fn redirect_successors(&mut self, id: super::InstId, from: BlockId, to: BlockId) {
        let redirect = |block: &mut BlockId| {
            if *block == from {
                *block = to;
            }
        };
        match &mut self.instructions[id.index()].kind {
            InstructionKind::Jump(block) => redirect(block),
            InstructionKind::Branch(_, yes, no) => {
                redirect(yes);
                redirect(no);
            }
            InstructionKind::Switch(_, default, cases) => {
                redirect(default);
                for (_, block) in cases {
                    redirect(block);
                }
            }
            _ => (),
        }
    }
    pub(crate) fn clear_speculation_facts(&mut self, id: super::InstId) {
        if let InstructionKind::Binary(_, facts, ..) = &mut self.instructions[id.index()].kind {
            facts.0 &= !0x30;
        }
    }
    pub(crate) fn record_switch_types(&mut self) {
        for &block in &self.layout {
            let Some(&last) = self.blocks[block.index()].instructions.last() else {
                continue;
            };
            let instruction = &mut self.instructions[last.index()];
            if instruction.switch_type.is_none()
                && let InstructionKind::Switch(Value::Temp(temp), ..) = instruction.kind
            {
                instruction.switch_type = self.temps[temp.index()].ty;
            }
        }
    }
    /// Substitute one canonical literal per known temporary, in `TempId` order.
    /// A call's callee must keep its callable identity even when known constant.
    pub(crate) fn substitute_constants(
        &mut self,
        constants: &[Option<Constant>],
    ) -> Result<u64, Error> {
        if constants.len() != self.temps.len() {
            return Err(Error::Limit);
        }
        let mut values = vec![None; constants.len()];
        for (index, &constant) in constants.iter().enumerate().skip(self.parameter_count) {
            if let Some(constant) = constant {
                values[index] = Some(self.constant(constant)?);
            }
        }
        let mut count = 0_u64;
        for &block in &self.layout {
            for &id in &self.blocks[block.index()].instructions {
                let kind = &mut self.instructions[id.index()].kind;
                let mut map = Substitution {
                    values: &values,
                    operands: &mut self.operands,
                    count: &mut count,
                    callee: matches!(kind, InstructionKind::Call(_)),
                };
                rewrite(kind, &mut map)?;
            }
        }
        self.recount_uses()?;
        Ok(count)
    }
    pub(in crate::lowir) fn append_parameter(
        &mut self,
        name: NameId,
        ty: Type,
    ) -> Result<(), Error> {
        let id = self.generated_temp(ty)?;
        if name != NameId(0) {
            self.temps[id.index()].name = name;
            self.temps[id.index()].generated = false;
            self.temp_index.entry(name).or_insert(id);
        }
        Ok(())
    }
    pub(crate) fn anonymous_parameter(&mut self, index: usize) -> Result<(), Error> {
        if index >= self.parameter_count {
            return Err(Error::Limit);
        }
        let temp = self.temps.get_mut(index).ok_or(Error::Limit)?;
        self.temp_index.remove(&temp.name);
        temp.name = NameId(0);
        temp.generated = true;
        Ok(())
    }
    pub(crate) fn generated_block_origin(&mut self, block: BlockId, stem: NameId) {
        self.blocks[block.index()].origin = BlockName::Generated(stem);
    }
    pub(crate) fn explicit_value_names(&self) -> impl Iterator<Item = NameId> + '_ {
        self.temps
            .iter()
            .filter(|temp| !temp.generated && temp.name != NameId(0))
            .map(|temp| temp.name)
            .chain(self.slots.iter().enumerate().filter_map(|(index, slot)| {
                match self.slot_names.get(index) {
                    Some(SlotName::Source(name)) => Some(*name),
                    None if slot.name != NameId(0) => Some(slot.name),
                    _ => None,
                }
            }))
    }
    pub(crate) fn recount_uses(&mut self) -> Result<(), Error> {
        let mut counts = vec![0_u32; self.temps.len()];
        let mut overflow = false;
        for &block in &self.layout {
            for &id in &self.blocks[block.index()].instructions {
                self.visit_values(&self.instructions[id.index()].kind, |value| {
                    let Value::Temp(temp) = value else {
                        return;
                    };
                    if let Some(next) = counts[temp.index()].checked_add(1) {
                        counts[temp.index()] = next;
                    } else {
                        overflow = true;
                    }
                });
            }
        }
        if overflow {
            return Err(Error::Limit);
        }
        for (temp, count) in self.temps.iter_mut().zip(counts) {
            temp.uses = count;
        }
        Ok(())
    }
    pub(crate) fn retain_instructions(&mut self, live: &[bool]) -> Result<bool, Error> {
        if live.len() != self.instructions.len() {
            return Err(Error::Limit);
        }
        let mut changed = false;
        for &block in &self.layout {
            self.blocks[block.index()].instructions.retain(|id| {
                changed |= !live[id.index()];
                live[id.index()]
            });
        }
        self.recount_uses()?;
        Ok(changed)
    }
    /// Dense storage is built separately; a bad linked reference leaves this body intact.
    pub(crate) fn compact(&mut self, names: &mut Names) -> Result<(), Error> {
        let out = Compactor::run(self, names)?;
        *self = out;
        Ok(())
    }
}

struct ValueRewrite<'a, F> {
    operands: &'a mut [Value],
    rewrite_value: F,
}
impl<F: FnMut(Value) -> Result<Value, Error>> Remap for ValueRewrite<'_, F> {
    fn value(&mut self, value: Value) -> Result<Value, Error> {
        (self.rewrite_value)(value)
    }
    fn block(&mut self, block: BlockId) -> Result<BlockId, Error> {
        Ok(block)
    }
    fn arguments(&mut self, arguments: Arguments) -> Result<Arguments, Error> {
        for index in arguments.first..arguments.first + arguments.count {
            self.operands[index as usize] = (self.rewrite_value)(self.operands[index as usize])?;
        }
        Ok(arguments)
    }
}

struct Substitution<'a> {
    values: &'a [Option<Value>],
    operands: &'a mut [Value],
    count: &'a mut u64,
    callee: bool,
}
impl Remap for Substitution<'_> {
    fn value(&mut self, value: Value) -> Result<Value, Error> {
        if std::mem::take(&mut self.callee) {
            return Ok(value);
        }
        if let Value::Temp(temp) = value
            && let Some(replacement) = self.values[temp.index()]
        {
            *self.count = self.count.checked_add(1).ok_or(Error::Limit)?;
            return Ok(replacement);
        }
        Ok(value)
    }
    fn block(&mut self, block: BlockId) -> Result<BlockId, Error> {
        Ok(block)
    }
    fn arguments(&mut self, arguments: Arguments) -> Result<Arguments, Error> {
        for index in arguments.first..arguments.first + arguments.count {
            let value = self.operands[index as usize];
            self.operands[index as usize] = self.value(value)?;
        }
        Ok(arguments)
    }
}

struct Compactor<'a> {
    input: &'a Body,
    out: Body,
    temps: Vec<Option<TempId>>,
    blocks: Vec<Option<BlockId>>,
    slots: Vec<Option<SlotId>>,
    literals: Vec<Option<LiteralId>>,
}
impl<'a> Compactor<'a> {
    fn run(input: &'a Body, names: &mut Names) -> Result<Body, Error> {
        let mut map = Self {
            input,
            out: Body::new(input.source, input.parameter_count),
            temps: vec![None; input.temps.len()],
            blocks: vec![None; input.blocks.len()],
            slots: vec![None; input.slots.len()],
            literals: vec![None; input.literals.len()],
        };
        map.definitions()?;
        map.instructions()?;
        // The frozen implementation retains the full location pool, including
        // locations of unlinked instructions; individual LocationIds stay valid.
        map.out.locations.clone_from(&input.locations);
        map.out.unique_names(names)?;
        Ok(map.out)
    }
    fn definitions(&mut self) -> Result<(), Error> {
        for index in 0..self.input.parameter_count {
            let temp = self.input.temps.get(index).ok_or(Error::Limit)?.clone();
            self.temps[index] = Some(TempId(u32::try_from(index).map_err(|_| Error::Limit)?));
            self.out.temps.push(temp);
        }
        for &block in &self.input.layout {
            let old = self.input.block(block);
            let new = BlockId(u32::try_from(self.out.blocks.len()).map_err(|_| Error::Limit)?);
            self.blocks[block.index()] = Some(new);
            self.out.blocks.push(Block {
                name: old.name,
                origin: old.origin,
                placed: true,
                first_use: old.first_use,
                instructions: Vec::with_capacity(old.instructions.len()),
            });
            self.out.layout.push(new);
        }
        // Every result is mapped before operands, including later phi definitions.
        for &block in &self.input.layout {
            for &id in &self.input.block(block).instructions {
                if let Some(result) = self.input.instruction(id).result {
                    let new =
                        TempId(u32::try_from(self.out.temps.len()).map_err(|_| Error::Limit)?);
                    self.temps[result.index()] = Some(new);
                    let mut temp = self.input.temp(result).clone();
                    temp.definition = None;
                    temp.uses = 0;
                    self.out.temps.push(temp);
                }
            }
        }
        for temp in &mut self.out.temps {
            temp.uses = 0;
        }
        Ok(())
    }
    fn instructions(&mut self) -> Result<(), Error> {
        for &block in &self.input.layout {
            let new = self.block(block)?;
            for &id in &self.input.block(block).instructions {
                // An intentional copy for physical compaction, not a second IR
                // transport. Program/global identities and signature IDs stay shared.
                let mut instruction = self.input.instruction(id).clone();
                instruction.result = instruction
                    .result
                    .map(|id| self.temps[id.index()].ok_or(Error::Limit))
                    .transpose()?;
                rewrite(&mut instruction.kind, self)?;
                self.out.append(new, instruction)?;
            }
        }
        Ok(())
    }
}
impl Remap for Compactor<'_> {
    fn value(&mut self, value: Value) -> Result<Value, Error> {
        Ok(match value {
            Value::Temp(id) => {
                Value::Temp(self.temps[id.index()].ok_or_else(|| {
                    Error::Construction("unlinked temporary at compaction".into())
                })?)
            }
            Value::Slot(id) => {
                if self.slots[id.index()].is_none() {
                    let new =
                        SlotId(u32::try_from(self.out.slots.len()).map_err(|_| Error::Limit)?);
                    let slot = self.input.slot(id);
                    self.out.slots.push(Slot {
                        name: slot.name,
                        ty: slot.ty,
                    });
                    self.out.slot_names.push(
                        self.input.slot_names.get(id.index()).copied().unwrap_or(
                            if slot.name == NameId(0) {
                                SlotName::Anonymous
                            } else {
                                SlotName::Source(slot.name)
                            },
                        ),
                    );
                    self.slots[id.index()] = Some(new);
                }
                Value::Slot(self.slots[id.index()].ok_or(Error::Limit)?)
            }
            Value::Integer(id) | Value::Float(id, _) => {
                if self.literals[id.0 as usize].is_none() {
                    let new = LiteralId(
                        u32::try_from(self.out.literals.len()).map_err(|_| Error::Limit)?,
                    );
                    self.out.literals.push(self.input.literal(id));
                    self.literals[id.0 as usize] = Some(new);
                }
                let new = self.literals[id.0 as usize].ok_or(Error::Limit)?;
                if let Value::Float(_, kind) = value {
                    Value::Float(new, kind)
                } else {
                    Value::Integer(new)
                }
            }
            Value::Symbol(_) => value,
        })
    }
    fn block(&mut self, block: BlockId) -> Result<BlockId, Error> {
        self.blocks[block.index()]
            .ok_or_else(|| Error::Construction("unplaced block at compaction".into()))
    }
    fn arguments(&mut self, arguments: Arguments) -> Result<Arguments, Error> {
        if arguments.count == 0 {
            return Ok(Arguments { first: 0, count: 0 });
        }
        let mut values = Vec::with_capacity(arguments.count as usize);
        for &value in self.input.arguments(arguments) {
            values.push(self.value(value)?);
        }
        self.out.add_arguments(&values)
    }
}
impl Body {
    fn unique_names(&mut self, names: &mut Names) -> Result<(), Error> {
        let mut seen = HashSet::new();
        self.temp_index.clear();
        for (index, temp) in self.temps.iter_mut().enumerate() {
            if temp.generated || temp.name == NameId(0) {
                continue;
            }
            if !seen.insert(temp.name) && index >= self.parameter_count {
                temp.name = NameId(0);
                temp.generated = true;
            } else {
                self.temp_index
                    .entry(temp.name)
                    .or_insert(TempId(u32::try_from(index).map_err(|_| Error::Limit)?));
            }
        }
        seen.clear();
        for label in &mut self.slot_names {
            if let SlotName::Source(name) = *label
                && !seen.insert(name)
            {
                *label = SlotName::Anonymous;
            }
        }
        self.name_optimizer_slots(names, &[])?;
        seen.clear();
        for block in &mut self.blocks {
            if let BlockName::Source(name) = block.origin
                && !seen.insert(name)
            {
                block.origin = BlockName::Anonymous;
            }
        }
        self.name_blocks(names)
    }
    pub(in crate::lowir) fn name_blocks(&mut self, names: &mut Names) -> Result<(), Error> {
        let taken: HashSet<_> = self
            .blocks
            .iter()
            .filter_map(|block| {
                if let BlockName::Source(name) = block.origin {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        self.block_index.clear();
        for (index, block) in self.blocks.iter_mut().enumerate() {
            if let BlockName::Source(name) = block.origin {
                block.name = name;
            } else {
                let mut text = match block.origin {
                    BlockName::Generated(stem) | BlockName::CopiedSource(stem) => {
                        format!("{}_{index}", names.get(stem))
                    }
                    BlockName::Optimizer(kind) => format!("{}_{index}", kind.spelling()),
                    _ => index.to_string(),
                };
                while taken.contains(&names.intern(&text)?) {
                    text.push('_');
                }
                block.name = names.intern(&text)?;
            }
            self.block_index
                .entry(block.name)
                .or_insert(BlockId(u32::try_from(index).map_err(|_| Error::Limit)?));
        }
        Ok(())
    }
}

/// A function-local edit view over instruction IDs. Records and operands stay
/// in Body; commit materializes block order once after constant-time link edits.
pub(crate) struct LinkEditor {
    next: Vec<Option<super::InstId>>,
    previous: Vec<Option<super::InstId>>,
    block_of: Vec<Option<BlockId>>,
    first: Vec<Option<super::InstId>>,
    last: Vec<Option<super::InstId>>,
}
impl LinkEditor {
    pub(crate) fn new(body: &Body) -> Result<Self, Error> {
        let mut out = Self {
            next: vec![None; body.instructions.len()],
            previous: vec![None; body.instructions.len()],
            block_of: vec![None; body.instructions.len()],
            first: vec![None; body.blocks.len()],
            last: vec![None; body.blocks.len()],
        };
        for &block in &body.layout {
            let mut prev = None;
            for &id in &body.block(block).instructions {
                out.insert_after(block, prev, id)?;
                prev = Some(id);
            }
        }
        Ok(out)
    }
    pub(crate) fn first(&self, block: BlockId) -> Option<super::InstId> {
        self.first[block.index()]
    }
    pub(crate) fn last(&self, block: BlockId) -> Option<super::InstId> {
        self.last[block.index()]
    }
    pub(crate) fn next(&self, id: super::InstId) -> Option<super::InstId> {
        self.next[id.index()]
    }
    pub(crate) fn previous(&self, id: super::InstId) -> Option<super::InstId> {
        self.previous[id.index()]
    }
    pub(crate) fn unlink(&mut self, block: BlockId, id: super::InstId) -> Result<(), Error> {
        if self.block_of.get(id.index()) != Some(&Some(block)) {
            return Err(Error::Limit);
        }
        let prev = self.previous[id.index()];
        let next = self.next[id.index()];
        if let Some(prev) = prev {
            self.next[prev.index()] = next;
        } else {
            self.first[block.index()] = next;
        }
        if let Some(next) = next {
            self.previous[next.index()] = prev;
        } else {
            self.last[block.index()] = prev;
        }
        self.previous[id.index()] = None;
        self.next[id.index()] = None;
        self.block_of[id.index()] = None;
        Ok(())
    }
    pub(crate) fn insert_after(
        &mut self,
        block: BlockId,
        prev: Option<super::InstId>,
        id: super::InstId,
    ) -> Result<(), Error> {
        let blocks = block.index().checked_add(1).ok_or(Error::Limit)?;
        if self.first.len() < blocks {
            self.first.resize(blocks, None);
            self.last.resize(blocks, None);
        }
        let size = id.index().checked_add(1).ok_or(Error::Limit)?;
        if self.next.len() < size {
            self.next.resize(size, None);
            self.previous.resize(size, None);
            self.block_of.resize(size, None);
        }
        if self.block_of[id.index()].is_some()
            || prev.is_some_and(|p| self.block_of.get(p.index()) != Some(&Some(block)))
        {
            return Err(Error::Limit);
        }
        let next = prev.map_or(self.first[block.index()], |p| self.next[p.index()]);
        self.previous[id.index()] = prev;
        self.next[id.index()] = next;
        self.block_of[id.index()] = Some(block);
        if let Some(prev) = prev {
            self.next[prev.index()] = Some(id);
        } else {
            self.first[block.index()] = Some(id);
        }
        if let Some(next) = next {
            self.previous[next.index()] = Some(id);
        } else {
            self.last[block.index()] = Some(id);
        }
        Ok(())
    }
    pub(crate) fn ids(&self, block: BlockId) -> Vec<super::InstId> {
        let mut ids = Vec::new();
        let mut next = self.first(block);
        while let Some(id) = next {
            ids.push(id);
            next = self.next(id);
        }
        ids
    }
    pub(crate) fn write_links(&self, body: &mut Body) -> Result<(), Error> {
        for block in body.layout.clone() {
            body.replace_links(block, self.ids(block))?;
        }
        Ok(())
    }
    pub(crate) fn commit(self, body: &mut Body) -> Result<(), Error> {
        self.write_links(body)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Facts, Program, SymbolId, Unwind};
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        vocabulary::{Binary, Pass},
        writer,
    };
    fn output(program: &Program) -> String {
        let mut bytes = Vec::new();
        writer::write(program, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }
    fn symbol(program: &Program) -> SymbolId {
        program
            .publications()
            .iter()
            .copied()
            .find(|&id| {
                program
                    .function(program.symbol(id))
                    .is_some_and(|f| f.body.is_some())
            })
            .unwrap()
    }
    fn compact(program: &mut Program) {
        let id = symbol(program);
        let mut body = program.take_body(id).unwrap().unwrap();
        body.compact(&mut program.names).unwrap();
        program.restore_body(id, body).unwrap();
        crate::lowir::validate_test_program(program).unwrap();
    }
    #[test]
    fn compaction_maps_forward_phis_first_used_storage_literals_and_handler_targets() {
        let mut program = crate::lowir::test_program(
            br"
function @f(%p : i64) -> i64 {
 slot $unused : i64
 slot $second : i64
 slot $first : i64
 block ^entry:
  %unused = const i64 123 !dbg(test, 9, 9)
  %a = addr $first
  store i64 %p, %a
  jump ^loop
 block ^dead: return i64 42
 block ^loop:
  %v = phi i64 [^entry: %p, ^loop: %next]
  %next = binary add i64 %v, 1 !dbg(test, 4, 2)
  %q = addr $second
  store i64 %next, %q
  %condition = cmp lt i64 %next, 3
  branch %condition, ^loop, ^exit
 block ^exit: return i64 %next
}",
        )
        .unwrap();
        let id = symbol(&program);
        let mut body = program.take_body(id).unwrap().unwrap();
        let dead = body.layout.remove(1);
        let unused = body.block(body.layout[0]).instructions[0];
        let mut live = vec![true; body.instructions.len()];
        live[unused.index()] = false;
        assert!(body.retain_instructions(&live).unwrap());
        let locations = body.locations.len();
        assert_eq!(locations, 2);
        assert!(!body.layout.contains(&dead));
        body.compact(&mut program.names).unwrap();
        assert_eq!(
            body.layout.iter().map(|id| id.index()).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(body.slots.len(), 2);
        assert_eq!(body.literals, [1, 3]);
        assert_eq!(body.locations.len(), locations);
        assert_eq!(program.names.get(body.slots[0].name), "first");
        assert_eq!(program.names.get(body.slots[1].name), "second");
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                if let Some(temp) = body.instruction(id).result {
                    assert_eq!(body.temp(temp).definition, Some(id));
                }
            }
        }
        program.restore_body(id, body).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        let first = output(&program);
        assert!(!first.contains("unused") && !first.contains("^dead"));
        compact(&mut program);
        assert_eq!(output(&program), first);
    }
    #[test]
    fn explicit_name_collisions_preserve_parameters_and_reserve_all_temporary_ordinals() {
        let mut program = crate::lowir::test_program(
            br"
function @f(%keep : i64) -> i64 {
 slot $first : i64
 slot $second : i64
 block ^entry:
  %a = copy i64 %keep
  %b = copy i64 %a
  store i64 %b, $second
  store i64 %b, $first
  jump ^left
 block ^left:
  %c = load i64 $second
  return i64 %c
}",
        )
        .unwrap();
        let id = symbol(&program);
        let mut body = program.take_body(id).unwrap().unwrap();
        body.temps[1].name = body.temps[0].name;
        body.temps[2].name = program.names.intern("t1").unwrap();
        body.slots[1].name = body.slots[0].name;
        let duplicate = body.blocks[0].name;
        body.blocks[1].name = duplicate;
        body.blocks[1].origin = BlockName::Source(duplicate);
        program.restore_body(id, body).unwrap();
        compact(&mut program);
        let text = output(&program);
        assert!(text.contains("%t2 = copy i64 %keep"));
        assert!(text.contains("%t1 = copy i64 %t2"));
        assert!(text.contains("slot $1 : i64") && text.contains("jump ^1"));
        compact(&mut program);
        assert_eq!(output(&program), text);
    }
    #[test]
    fn shadow_and_generated_block_kinds_survive_dropped_predecessors_and_repeated_compaction() {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "f", &[("__param0", Type::I64)], Type::I64).unwrap();
        let parameter = builder.parameter_slot(0, Type::I64).unwrap();
        let _unused = builder.slot("x", Type::I64).unwrap();
        let shadow = builder.slot("x", Type::I64).unwrap();
        builder.block("entry").unwrap();
        let entry = builder.block_id("entry").unwrap();
        let _dead = builder.block_id("discard_1").unwrap();
        let finish = builder.block_id("if_end_2").unwrap();
        builder.block_origin(finish, "if_end").unwrap();
        let value = builder.parameter(0).unwrap();
        builder
            .effect(InstructionKind::Store(
                Facts::default(),
                Type::I64,
                value,
                parameter,
            ))
            .unwrap();
        builder
            .effect(InstructionKind::Store(
                Facts::default(),
                Type::I64,
                value,
                shadow,
            ))
            .unwrap();
        builder.effect(InstructionKind::Jump(finish)).unwrap();
        builder.place_block(finish).unwrap();
        builder
            .effect(InstructionKind::Return(Type::I64, Some(value)))
            .unwrap();
        builder.finish().unwrap();
        compact(&mut program);
        let text = output(&program);
        assert!(text.contains("slot $__param0 : i64"), "{text}");
        assert!(text.contains("slot $x__shadow1 : i64"), "{text}");
        assert!(text.contains("block ^if_end_1:"), "{text}");
        let body = program
            .function(program.symbol(symbol(&program)))
            .unwrap()
            .body
            .as_ref()
            .unwrap();
        assert_eq!(body.layout[0], entry);
        compact(&mut program);
        assert_eq!(output(&program), text);
    }
    #[test]
    fn arguments_map_callee_before_arguments_and_preserve_explicit_call_boundaries() {
        let mut program = crate::lowir::test_program(
            br"
function @f() -> void {
 slot $argument : ptr
 slot $callee : ptr
 block ^entry:
  %target = load ptr $callee
  call void %target($argument) as (%arg : ptr [pass=by_address]) -> void [unwind=no]
  return void
}",
        )
        .unwrap();
        compact(&mut program);
        let body = program
            .function(program.symbol(symbol(&program)))
            .unwrap()
            .body
            .as_ref()
            .unwrap();
        assert_eq!(program.names.get(body.slots[0].name), "callee");
        assert_eq!(program.names.get(body.slots[1].name), "argument");
        assert_eq!(body.operands.len(), 1);
        let InstructionKind::Call(call) = &body.instructions[1].kind else {
            panic!("call");
        };
        let signature = program.signature(call.signature.unwrap());
        assert!(signature.boundary.unwind == Unwind::No);
        assert_eq!(signature.parameters[0].pass, Some(Pass::ByAddress));
    }
    #[test]
    fn bad_live_reference_reports_a_structured_error_without_replacing_the_body() {
        let mut program = crate::lowir::test_program(
            b"function @f() -> i64 {block ^entry: %a = const i64 7 return i64 %a}\n",
        )
        .unwrap();
        let id = symbol(&program);
        let mut body = program.take_body(id).unwrap().unwrap();
        let before = body.instructions.len();
        let mut live = vec![true; before];
        live[0] = false;
        body.retain_instructions(&live).unwrap();
        assert!(matches!(
            body.compact(&mut program.names),
            Err(Error::Construction(_))
        ));
        assert_eq!(body.instructions.len(), before);
        assert_eq!(body.blocks[0].instructions.len(), 1);
    }
    #[test]
    fn compaction_keeps_used_result_types_flags_and_literal_bits() {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::I128).unwrap();
        builder.block("entry").unwrap();
        let literal = builder.integer(u128::MAX).unwrap();
        let value = builder
            .generated(InstructionKind::Binary(
                Binary::Add,
                Facts::arithmetic(true, false),
                Type::I128,
                literal,
                literal,
            ))
            .unwrap();
        builder
            .effect(InstructionKind::Return(Type::I128, Some(value)))
            .unwrap();
        builder.finish().unwrap();
        compact(&mut program);
        let body = program
            .function(program.symbol(symbol(&program)))
            .unwrap()
            .body
            .as_ref()
            .unwrap();
        assert_eq!(body.literals, [u128::MAX]);
        assert!(
            matches!(body.instructions[0].kind,InstructionKind::Binary(_,facts,Type::I128,_,_) if facts.nowrap() && !facts.exact())
        );
    }
    #[test]
    fn linked_edits_reject_wrong_owners_and_commit_ordered_movement() {
        let mut program=crate::lowir::test_program(b"function @f(%v : i64, %p : ptr) -> void {block ^entry: store i64 %v, %p jump ^exit block ^exit: return void}").unwrap();
        let symbol = symbol(&program);
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let entry = body.layout[0];
        let exit = body.layout[1];
        let store = body.block(entry).instructions[0];
        let original = body.block(entry).instructions.clone();
        let mut links = LinkEditor::new(&body).unwrap();
        assert!(links.unlink(exit, store).is_err());
        assert_eq!(links.ids(entry), original);
        links.unlink(entry, store).unwrap();
        links.insert_after(exit, None, store).unwrap();
        assert!(links.insert_after(exit, None, store).is_err());
        assert_eq!(links.first(exit), Some(store));
        links.commit(&mut body).unwrap();
        assert_eq!(body.block(exit).instructions[0], store);
        assert!(matches!(
            body.instruction(store).kind,
            InstructionKind::Store(..)
        ));
    }
}

#[cfg(test)]
mod optimizer_name_tests {
    use super::*;
    use crate::lowir::model::{Instruction, OptimizerBlock};
    #[test]
    fn generated_block_stems_skip_explicit_names_after_compaction() {
        let mut program = crate::lowir::test_program(
            b"function @f()->void {block ^entry:jump ^thread_1 block ^thread_1:return void}",
        )
        .unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let entry = body.layout[0];
        let exit = body.layout[1];
        let block = body.optimizer_block(OptimizerBlock::Thread).unwrap();
        body.append(
            block,
            Instruction::new(None, InstructionKind::Jump(exit), 0),
        )
        .unwrap();
        let tail = *body.block(entry).instructions.last().unwrap();
        body.redirect_successors(tail, exit, block);
        body.layout.insert(1, block);
        body.compact(&mut program.names).unwrap();
        assert_eq!(
            program.names.get(body.block(body.layout[1]).name),
            "thread_1_"
        );
        assert_eq!(
            program.names.get(body.block(body.layout[2]).name),
            "thread_1"
        );
    }
}
