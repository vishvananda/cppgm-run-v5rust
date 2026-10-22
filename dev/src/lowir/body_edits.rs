//! Function-store rewrites and compaction from `opt/function_ir.cpp`. See NOTICE.
use super::{
    Arguments, Block, BlockId, BlockName, Body, Error, InstructionKind, LiteralId, NameId, Names,
    RuntimeInstruction, Slot, SlotId, TempId, Type, Value,
};
use crate::lowir::construction_names::SlotName;
use std::collections::HashSet;

trait Remap {
    fn value(&mut self, value: Value) -> Result<Value, Error>;
    fn block(&mut self, block: BlockId) -> Result<BlockId, Error>;
    fn arguments(&mut self, arguments: Arguments) -> Result<Arguments, Error>;
}
fn rewrite(kind: &mut InstructionKind, map: &mut impl Remap) -> Result<(), Error> {
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
        R::Fence(..)
        | R::Exception(_)
        | R::ExceptionSelector(_)
        | R::EhCleanup(None)
        | R::EhCatch(..)
        | R::EhCatchAll(_)
        | R::EhEnd
        | R::Resume => (),
    }
    Ok(())
}

impl Body {
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
                    BlockName::Generated(stem) => format!("{}_{index}", names.get(stem)),
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
}
