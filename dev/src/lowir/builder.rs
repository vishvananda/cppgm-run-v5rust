//! Typed construction over the same private stores used by the reader. See NOTICE.
use super::{
    Error,
    literals::Constant,
    model::{
        BlockId, Body, Boundary, Function, Instruction, InstructionKind, Metadata, NameId,
        Parameter, Program, Signature, SymbolId, Value,
    },
    vocabulary::Type,
};

pub(crate) struct FunctionBuilder<'a> {
    program: &'a mut Program,
    symbol: SymbolId,
    function: Function,
    body: Body,
    block: Option<BlockId>,
    slot_names: Vec<super::construction_names::SlotName>,
    publish: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Position {
    block: BlockId,
    after: Option<super::model::InstId>,
}

/// A suspended body owns its construction state without borrowing the program.
/// This permits declaration-ordered startup work between ordinary definitions.
pub(crate) struct SuspendedFunction {
    symbol: SymbolId,
    function: Function,
    body: Body,
    block: Option<BlockId>,
    slot_names: Vec<super::construction_names::SlotName>,
    publish: bool,
}

impl<'a> FunctionBuilder<'a> {
    pub(crate) fn resume(program: &'a mut Program, saved: SuspendedFunction) -> Self {
        Self {
            program,
            symbol: saved.symbol,
            function: saved.function,
            body: saved.body,
            block: saved.block,
            slot_names: saved.slot_names,
            publish: saved.publish,
        }
    }
    pub(crate) fn suspend(self) -> SuspendedFunction {
        SuspendedFunction {
            symbol: self.symbol,
            function: self.function,
            body: self.body,
            block: self.block,
            slot_names: self.slot_names,
            publish: self.publish,
        }
    }
    /// Move construction out while a nested initializer borrows the program.
    /// The caller must restore it before issuing another builder operation.
    pub(crate) fn take_construction(&mut self) -> SuspendedFunction {
        let placeholder = Function {
            name: self.function.name,
            signature: self.function.signature,
            metadata: Metadata::default(),
            body: None,
            location: None,
        };
        SuspendedFunction {
            symbol: self.symbol,
            function: std::mem::replace(&mut self.function, placeholder),
            body: std::mem::replace(&mut self.body, Body::new(None, 0)),
            block: self.block.take(),
            slot_names: std::mem::take(&mut self.slot_names),
            publish: self.publish,
        }
    }
    pub(crate) fn restore_construction(&mut self, saved: SuspendedFunction) {
        self.symbol = saved.symbol;
        self.function = saved.function;
        self.body = saved.body;
        self.block = saved.block;
        self.slot_names = saved.slot_names;
        self.publish = saved.publish;
    }
    pub(crate) fn new(
        program: &'a mut Program,
        name: &str,
        parameters: &[(&str, Type)],
        result: Type,
    ) -> Result<Self, Error> {
        let name = program.names.intern(name)?;
        let source = program.add_source(std::path::Path::new(""))?;
        let symbol = program.reference(name, source, 0)?;
        if program.function_defined(symbol) {
            return Err(Error::Diagnostic("duplicate constructed function".into()));
        }
        let mut signature = Signature {
            parameters: Vec::new(),
            result,
            boundary: Boundary::default(),
        };
        let mut body = Body::new(None, parameters.len());
        for &(text, ty) in parameters {
            let name = program.names.intern(text)?;
            if body.find_temp(name).is_some() {
                return Err(Error::Diagnostic("duplicate constructed parameter".into()));
            }
            signature.parameters.push(Parameter {
                name,
                ty,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            });
            let id = body.use_temp(name, 0)?;
            body.temp_mut(id).ty = Some(ty);
        }
        let signature = program.add_signature(signature)?;
        let function = Function {
            name,
            signature,
            metadata: Metadata::default(),
            body: None,
            location: None,
        };
        Ok(Self {
            program,
            symbol,
            function,
            body,
            block: None,
            slot_names: Vec::new(),
            publish: true,
        })
    }
    /// Complete a generated callable whose boundary was published at demand.
    /// Its named parameters and ABI facts are shared with earlier callers.
    pub(crate) fn from_forward_boundary(
        program: &'a mut Program,
        symbol: SymbolId,
    ) -> Result<Self, Error> {
        let signature = program.forward_boundary(symbol).ok_or_else(|| {
            Error::Construction("generated function without forward boundary".into())
        })?;
        let metadata = program
            .callee_metadata(Value::Symbol(symbol))
            .copied()
            .ok_or(Error::Limit)?;
        let parameters = &program.signature(signature).parameters;
        let mut body = Body::new(None, parameters.len());
        for parameter in parameters {
            let temp = body.use_temp(parameter.name, 0)?;
            body.temp_mut(temp).ty = Some(parameter.ty);
        }
        let function = Function {
            name: program.symbol(symbol).name,
            signature,
            metadata,
            body: None,
            location: None,
        };
        Ok(Self {
            program,
            symbol,
            function,
            body,
            block: None,
            slot_names: Vec::new(),
            publish: true,
        })
    }
    pub(crate) fn complete(program: &'a mut Program, symbol: SymbolId) -> Result<Self, Error> {
        let source = program
            .function(program.symbol(symbol))
            .ok_or(Error::Limit)?;
        let function = Function {
            name: source.name,
            signature: source.signature,
            metadata: source.metadata,
            body: None,
            location: source.location,
        };
        let signature = program.signature(function.signature);
        let mut body = Body::new(None, signature.parameters.len());
        for parameter in &signature.parameters {
            body.append_parameter(parameter.name, parameter.ty)?;
        }
        Ok(Self {
            program,
            symbol,
            function,
            body,
            block: None,
            slot_names: Vec::new(),
            publish: false,
        })
    }
    pub(crate) fn anonymous_block(&mut self) -> Result<BlockId, Error> {
        self.body.generated_block()
    }
    pub(crate) fn anonymous_slot(&mut self, ty: Type) -> Result<Value, Error> {
        if self.publish {
            return Err(Error::Construction(
                "anonymous slot requires an internal definition".into(),
            ));
        }
        self.slot_names
            .push(super::construction_names::SlotName::Anonymous);
        self.body.constructed_slot(ty)
    }
    pub(crate) fn metadata(&mut self, metadata: Metadata) {
        self.function.metadata = metadata;
    }
    pub(crate) fn boundary(&mut self, boundary: Boundary) {
        self.program.signature_mut(self.function.signature).boundary = boundary;
    }
    pub(crate) fn parameter_boundary(
        &mut self,
        index: usize,
        pass: Option<super::vocabulary::Pass>,
        object_bytes: u64,
        no_alias: bool,
    ) -> Result<(), Error> {
        let parameter = self
            .program
            .signature_mut(self.function.signature)
            .parameters
            .get_mut(index)
            .ok_or(Error::Limit)?;
        parameter.pass = pass;
        parameter.object_bytes = object_bytes;
        parameter.no_alias = no_alias;
        Ok(())
    }
    pub(crate) fn parameter_classes(
        &mut self,
        index: usize,
        classes: Option<super::vocabulary::ObjectClasses>,
    ) -> Result<(), Error> {
        self.program
            .signature_mut(self.function.signature)
            .parameters
            .get_mut(index)
            .ok_or(Error::Limit)?
            .classes = classes;
        Ok(())
    }
    pub(crate) fn slot(&mut self, name: &str, ty: Type) -> Result<Value, Error> {
        let name = self.program.names.intern(name)?;
        self.named_slot(name, ty)
    }
    pub(crate) fn named_slot(&mut self, name: NameId, ty: Type) -> Result<Value, Error> {
        let range_for = matches!(
            self.program.names.get(name),
            "__range" | "__begin" | "__end" | "__idx"
        );
        self.slot_names.push(if range_for {
            super::construction_names::SlotName::RangeFor(name)
        } else {
            super::construction_names::SlotName::Source(name)
        });
        self.body.constructed_slot(ty)
    }
    pub(crate) fn generated_slot(&mut self, stem: &'static str, ty: Type) -> Result<Value, Error> {
        self.slot_names
            .push(super::construction_names::SlotName::Generated(stem));
        self.body.constructed_slot(ty)
    }
    pub(crate) fn parameter_slot(&mut self, index: usize, ty: Type) -> Result<Value, Error> {
        self.body.anonymous_parameter(index)?;
        self.slot_names.push(if u16::try_from(index).is_ok() {
            super::construction_names::SlotName::Parameter(index)
        } else {
            super::construction_names::SlotName::Generated("arg")
        });
        self.body.constructed_slot(ty)
    }
    pub(crate) fn program(&mut self) -> &mut Program {
        self.program
    }
    pub(crate) fn block_id(&mut self, name: &str) -> Result<BlockId, Error> {
        let name = self.program.names.intern(name)?;
        self.body.use_block(name, 0)
    }
    pub(crate) fn block_origin(&mut self, block: BlockId, stem: &str) -> Result<(), Error> {
        let stem = self.program.names.intern(stem)?;
        self.body.generated_block_origin(block, stem);
        Ok(())
    }
    pub(crate) fn signature(
        &mut self,
        name: &str,
        ty: Type,
        result: Type,
    ) -> Result<super::model::SignatureId, Error> {
        let name = self.program.names.intern(name)?;
        self.program.add_signature(Signature {
            parameters: vec![Parameter {
                name,
                ty,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            }],
            result,
            boundary: Boundary::default(),
        })
    }
    pub(crate) fn parameter(&self, index: usize) -> Result<Value, Error> {
        self.body.parameter_value(index)
    }
    pub(crate) fn integer(&mut self, value: u128) -> Result<Value, Error> {
        self.constant(Constant::Integer(value))
    }
    pub(crate) fn constant(&mut self, value: Constant) -> Result<Value, Error> {
        self.body.constant(value)
    }
    pub(crate) fn arguments(&mut self, values: &[Value]) -> Result<super::model::Arguments, Error> {
        self.body.add_arguments(values)
    }
    pub(crate) fn block(&mut self, name: &str) -> Result<(), Error> {
        let name = self.program.names.intern(name)?;
        let id = self.body.use_block(name, 0)?;
        self.place_block(id)
    }
    pub(crate) fn place_block(&mut self, id: BlockId) -> Result<(), Error> {
        if self.body.block(id).placed {
            return Err(Error::Diagnostic("duplicate constructed block".into()));
        }
        self.body.block_mut(id).placed = true;
        self.body.layout.push(id);
        self.block = Some(id);
        Ok(())
    }
    pub(crate) fn generated(&mut self, kind: InstructionKind) -> Result<Value, Error> {
        let id = self.body.generated_temp(kind.result_type())?;
        self.append(kind, Some(id))?;
        Ok(Value::Temp(id))
    }
    pub(crate) fn value(&mut self, kind: InstructionKind, name: &str) -> Result<Value, Error> {
        let name = self.program.names.intern(name)?;
        let id = self.body.use_temp(name, 0)?;
        if self.body.temp_is_parameter(id) || self.body.temp(id).definition.is_some() {
            return Err(Error::Diagnostic("duplicate constructed temporary".into()));
        }
        self.body.temp_mut(id).ty = Some(kind.result_type());
        self.append(kind, Some(id))?;
        Ok(Value::Temp(id))
    }
    pub(crate) fn effect(&mut self, kind: InstructionKind) -> Result<(), Error> {
        self.append(kind, None)
    }
    pub(crate) fn position(&self) -> Result<Position, Error> {
        let block = self.block.ok_or(Error::Limit)?;
        Ok(Position {
            block,
            after: self.body.block(block).instructions.last().copied(),
        })
    }
    pub(crate) fn same_block(&self, position: Position) -> bool {
        self.block == Some(position.block)
    }
    /// Unlinks an empty region without renumbering instructions or definitions.
    /// Callers rebase captured positions equal to the removed marker.
    pub(crate) fn drop_empty_region(
        &mut self,
        target: BlockId,
    ) -> Result<Option<(Position, Position)>, Error> {
        let removed = self.position()?;
        let Some(id) = removed.after else {
            return Ok(None);
        };
        if !matches!(self.body.instruction(id).kind,
            InstructionKind::Runtime(super::model::RuntimeInstruction::EhTry(entry)) if entry == target)
        {
            return Ok(None);
        }
        let _removed = self.body.block_mut(removed.block).instructions.pop();
        Ok(Some((removed, self.position()?)))
    }
    /// Whether a captured call boundary has already crossed an EH marker.
    pub(crate) fn region_since(&self, position: Position) -> bool {
        use super::model::RuntimeInstruction as R;
        self.body
            .block(position.block)
            .instructions
            .iter()
            .rev()
            .take_while(|&&id| Some(id) != position.after)
            .any(|&id| {
                matches!(
                    self.body.instruction(id).kind,
                    InstructionKind::Runtime(R::EhTry(_) | R::EhEnd | R::EhCleanup(_))
                )
            })
    }
    /// Inserts a region marker at a captured call boundary. Instruction IDs
    /// remain stable; only that block's execution order changes.
    pub(crate) fn insert_marker(
        &mut self,
        position: Position,
        kind: super::model::RuntimeInstruction,
    ) -> Result<(), Error> {
        self.insert_effect(position, InstructionKind::Runtime(kind))?;
        Ok(())
    }
    /// Inserts a result-free instruction without changing stable definition IDs.
    pub(crate) fn insert_effect(
        &mut self,
        position: Position,
        kind: InstructionKind,
    ) -> Result<Position, Error> {
        self.insert_effects(position, std::iter::once(kind))
    }
    /// Splices a group once, so resetting many lifetime flags does not shift
    /// the entire expression's instruction sequence once per temporary.
    pub(crate) fn insert_effects(
        &mut self,
        position: Position,
        kinds: impl Iterator<Item = InstructionKind>,
    ) -> Result<Position, Error> {
        let before = self.body.block(position.block).instructions.len();
        for kind in kinds {
            let _id = self
                .body
                .append(position.block, Instruction::new(None, kind, 0))?;
        }
        let instructions = &mut self.body.block_mut(position.block).instructions;
        let after = instructions
            .last()
            .copied()
            .filter(|_| instructions.len() > before)
            .or(position.after);
        let index = if let Some(previous) = position.after {
            instructions[..before]
                .iter()
                .rposition(|&id| id == previous)
                .ok_or(Error::Limit)?
                + 1
        } else {
            0
        };
        let added = instructions.len() - before;
        instructions[index..].rotate_right(added);
        Ok(Position {
            block: position.block,
            after,
        })
    }
    pub(crate) fn retarget_region(
        &mut self,
        marker: Position,
        target: BlockId,
    ) -> Result<(), Error> {
        let id = marker.after.ok_or(Error::Limit)?;
        self.body.retarget_region(id, target)
    }
    pub(crate) fn addressed_storage(&self, address: Value) -> Value {
        if let Value::Temp(temp) = address
            && let Some(id) = self.body.temp(temp).definition
            && let InstructionKind::Addr(storage @ Value::Slot(_)) = self.body.instruction(id).kind
        {
            return storage;
        }
        address
    }
    pub(crate) fn layout_len(&self) -> usize {
        self.body.layout.len()
    }
    /// Places newly emitted cleanup blocks after their full-expression's first
    /// block, then restores insertion without placing the saved block twice.
    pub(crate) fn restore_after_cleanup(
        &mut self,
        saved: Position,
        start: Position,
        before: usize,
    ) -> Result<(), Error> {
        let at = self.body.layout[..before]
            .iter()
            .position(|&block| block == start.block)
            .ok_or(Error::Limit)?;
        self.body.layout[at + 1..].rotate_left(before - at - 1);
        self.block = Some(saved.block);
        Ok(())
    }
    /// C++ lowering's incoming-edge policy for a syntactic non-void end.
    /// Branches, switches, folded branches and EH targets seed incoming edges
    /// even in dead blocks; ordinary jumps propagate only from entry or a
    /// targeted block. A worklist computes the same fixed point in linear time.
    pub(crate) fn insertion_targeted(&self, folded: &[BlockId]) -> Result<bool, Error> {
        let insertion = self.block.ok_or(Error::Limit)?;
        let entry = *self.body.layout.first().ok_or(Error::Limit)?;
        let mut incoming = vec![false; self.body.blocks().len()];
        let mut jumps = vec![None; incoming.len()];
        let mut work = Vec::new();
        let mut mark = |block: BlockId| {
            if !incoming[block.index()] {
                incoming[block.index()] = true;
                work.push(block);
            }
        };
        for &block in folded {
            mark(block);
        }
        for &block in &self.body.layout {
            for &id in &self.body.block(block).instructions {
                let kind = &self.body.instruction(id).kind;
                match kind {
                    InstructionKind::Jump(target) => jumps[block.index()] = Some(*target),
                    InstructionKind::Branch(..) | InstructionKind::Switch(..) => {
                        kind.visit_targets(&mut mark);
                    }
                    InstructionKind::Runtime(runtime) => {
                        if let Some(target) = runtime.handler_target() {
                            mark(target);
                        }
                    }
                    _ => {}
                }
            }
        }
        // Entry propagates its jump without itself being an incoming target.
        work.push(entry);
        while let Some(block) = work.pop() {
            if let Some(target) = jumps[block.index()]
                && !incoming[target.index()]
            {
                incoming[target.index()] = true;
                work.push(target);
            }
        }
        Ok(incoming[insertion.index()])
    }
    fn append(
        &mut self,
        kind: InstructionKind,
        result: Option<super::model::TempId>,
    ) -> Result<(), Error> {
        let block = self
            .block
            .ok_or_else(|| Error::Diagnostic("instruction outside constructed block".into()))?;
        let _ = self.body.append(block, Instruction::new(result, kind, 0))?;
        Ok(())
    }
    pub(crate) fn finish(mut self) -> Result<(), Error> {
        super::construction_names::record_shadows(&mut self.slot_names);
        let positional: std::collections::HashSet<_> = self
            .slot_names
            .iter()
            .filter_map(|label| {
                if let super::construction_names::SlotName::Parameter(index) = label {
                    Some(*index)
                } else {
                    None
                }
            })
            .collect();
        let parameters: Vec<_> = self
            .program
            .signature(self.function.signature)
            .parameters
            .iter()
            .enumerate()
            .filter(|(index, _)| !positional.contains(index))
            .map(|(_, p)| p.name)
            .collect();
        let names = super::construction_names::slots(
            &mut self.program.names,
            &self.slot_names,
            &parameters,
        )?;
        self.body.name_constructed_slots(&names)?;
        self.body.retain_slot_names(self.slot_names);
        self.body.name_blocks(&mut self.program.names)?;
        if !self.publish {
            return self.program.complete_body(self.symbol, self.body);
        }
        self.function.body = Some(self.body);
        self.program.add(self.symbol, self.function)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{exercises, model::InstructionKind as I, validator};

    #[test]
    fn completed_runtime_body_retains_parameter_names_and_anonymous_storage() -> Result<(), Error> {
        let mut program =
            crate::lowir::test_program(b"declare function @helper(%t1 : i64) -> i64\n")?;
        let symbol = program.publications()[0];
        let mut builder = FunctionBuilder::complete(&mut program, symbol)?;
        let slot = builder.anonymous_slot(Type::I64)?;
        builder.block("entry")?;
        let argument = builder.parameter(0)?;
        builder.effect(I::Store(
            super::super::model::Facts::default(),
            Type::I64,
            argument,
            slot,
        ))?;
        let result = builder.generated(I::Load(
            super::super::model::Facts::default(),
            Type::I64,
            slot,
        ))?;
        builder.effect(I::Return(Type::I64, Some(result)))?;
        builder.finish()?;
        validator::validate_constructed(&program)?;
        let mut bytes = Vec::new();
        crate::lowir::writer::write(&program, &mut bytes).map_err(|_| Error::Limit)?;
        let text = String::from_utf8(bytes).map_err(|_| Error::Limit)?;
        assert!(text.contains("function @helper(%t1 : i64) -> i64"));
        assert!(text.contains("slot $0 : i64"));
        assert!(text.contains("store i64 %t1, $0"));
        assert!(text.contains("%t2 = load i64 $0"));
        Ok(())
    }

    #[test]
    fn inserted_region_preserves_value_definitions_and_target_identity() -> Result<(), Error> {
        use crate::lowir::{
            model::{Facts, RuntimeInstruction as R},
            vocabulary::Binary,
        };
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "guarded", &[], Type::I32)?;
        builder.block("entry")?;
        let one = builder.integer(1)?;
        let first = builder.generated(I::Copy(Type::I32, one))?;
        let position = builder.position()?;
        let handler = builder.block_id("unwind")?;
        let result = builder.generated(I::Binary(
            Binary::Add,
            Facts::default(),
            Type::I32,
            first,
            one,
        ))?;
        let Value::Temp(temp) = result else {
            return Err(Error::Limit);
        };
        let definition = builder.body.temp(temp).definition.ok_or(Error::Limit)?;
        let after_result = builder.position()?;
        assert!(!builder.region_since(position));
        builder.insert_marker(position, R::EhTry(handler))?;
        assert!(builder.region_since(position));
        assert!(!builder.region_since(after_result));
        assert_eq!(builder.body.temp(temp).definition, Some(definition));
        let instructions = &builder.body.block(position.block).instructions;
        assert!(
            matches!(builder.body.instruction(instructions[1]).kind, I::Runtime(R::EhTry(target)) if target == handler)
        );
        assert_eq!(instructions[2], definition);
        builder.effect(I::Runtime(R::EhEnd))?;
        assert!(builder.region_since(after_result));
        builder.effect(I::Return(Type::I32, Some(result)))?;
        builder.place_block(handler)?;
        builder.effect(I::Runtime(R::Resume))?;
        builder.finish()?;
        validator::validate_constructed(&program)?;
        Ok(())
    }

    #[test]
    fn tracked_element_jump_retargets_to_completed_cleanup_chain() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "tracked_element", &[], Type::Void)?;
        builder.block("entry")?;
        let placeholder = builder.block_id("placeholder")?;
        let completed = builder.block_id("completed")?;
        builder.effect(I::Jump(placeholder))?;
        let marker = builder.position()?;
        builder.retarget_region(marker, completed)?;
        builder.place_block(placeholder)?;
        builder.effect(I::Unreachable)?;
        builder.place_block(completed)?;
        builder.effect(I::Return(Type::Void, None))?;
        builder.finish()?;
        validator::validate_constructed(&program)?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        assert!(
            body.instructions().iter().any(
                |instruction| matches!(instruction.kind, I::Jump(target) if target == completed)
            )
        );
        Ok(())
    }

    #[test]
    fn empty_region_removal_preserves_definitions_and_rebased_insertion() -> Result<(), Error> {
        use crate::lowir::model::RuntimeInstruction as R;
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "empty_region", &[], Type::I32)?;
        builder.block("entry")?;
        let one = builder.integer(1)?;
        let result = builder.generated(I::Copy(Type::I32, one))?;
        let before = builder.position()?;
        let handler = builder.block_id("unwind")?;
        let other = builder.block_id("other")?;
        builder.effect(I::Runtime(R::EhTry(handler)))?;
        let removed = builder.position()?;
        assert!(builder.drop_empty_region(other)?.is_none());
        assert!(builder.drop_empty_region(handler)? == Some((removed, before)));
        assert!(!builder.region_since(before));
        let marker = builder.insert_effect(before, I::Runtime(R::EhTry(handler)))?;
        let Value::Temp(temp) = result else {
            return Err(Error::Limit);
        };
        assert_eq!(builder.body.temp(temp).definition, before.after);
        builder.effect(I::Runtime(R::EhEnd))?;
        assert!(builder.drop_empty_region(handler)?.is_none());
        builder.retarget_region(marker, handler)?;
        builder.effect(I::Return(Type::I32, Some(result)))?;
        builder.place_block(handler)?;
        builder.effect(I::Runtime(R::Resume))?;
        builder.place_block(other)?;
        builder.effect(I::Unreachable)?;
        builder.finish()?;
        validator::validate_constructed(&program)?;
        assert_eq!(program.statistics().instructions, 6);
        Ok(())
    }

    #[test]
    fn tracked_regions_keep_definitions_and_restore_expression_layout() -> Result<(), Error> {
        use crate::lowir::model::{Facts, RuntimeInstruction as R};
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "tracked", &[], Type::Void)?;
        builder.block("entry")?;
        let start = builder.position()?;
        let slot = builder.generated_slot("object", Type::I32)?;
        let flag = builder.generated_slot("flag", Type::U8)?;
        let object = builder.generated(I::Addr(slot))?;
        assert_eq!(builder.addressed_storage(object), slot);
        let placeholder = builder.block_id("placeholder")?;
        let marker = builder.insert_effect(start, I::Runtime(R::EhTry(placeholder)))?;
        let zero = builder.integer(0)?;
        let _reset =
            builder.insert_effect(start, I::Store(Facts::default(), Type::U8, zero, flag))?;
        builder.effect(I::Runtime(R::EhEnd))?;
        let end = builder.block_id("end")?;
        builder.effect(I::Jump(end))?;
        builder.place_block(end)?;
        let saved = builder.position()?;
        let before = builder.layout_len();
        let cleanup = builder.block_id("cleanup")?;
        builder.place_block(cleanup)?;
        builder.effect(I::Runtime(R::Resume))?;
        builder.retarget_region(marker, cleanup)?;
        builder.restore_after_cleanup(saved, start, before)?;
        builder.effect(I::Return(Type::Void, None))?;
        let Value::Temp(object) = object else {
            return Err(Error::Limit);
        };
        assert!(builder.body.temp(object).definition.is_some());
        assert_eq!(builder.body.layout[1], cleanup);
        assert_eq!(builder.body.layout[2], end);
        builder.finish()?;
        validator::validate_constructed(&program)?;
        Ok(())
    }

    #[test]
    fn suspended_body_keeps_ids_and_slot_names_across_another_definition() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "startup", &[], Type::Void)?;
        builder.block("entry")?;
        let slot = builder.generated_slot("value", Type::I32)?;
        let one = builder.integer(1)?;
        builder.effect(I::Store(
            super::super::model::Facts::default(),
            Type::I32,
            one,
            slot,
        ))?;
        let saved = builder.suspend();
        let mut interposed = FunctionBuilder::new(&mut program, "ordinary", &[], Type::Void)?;
        interposed.block("entry")?;
        interposed.effect(I::Return(Type::Void, None))?;
        interposed.finish()?;
        let mut builder = FunctionBuilder::resume(&mut program, saved);
        let other = builder.slot("value__1", Type::I32)?;
        let value = builder.generated(I::Load(
            super::super::model::Facts::default(),
            Type::I32,
            slot,
        ))?;
        builder.effect(I::Store(
            super::super::model::Facts::default(),
            Type::I32,
            value,
            other,
        ))?;
        builder.effect(I::Return(Type::Void, None))?;
        builder.finish()?;
        validator::validate_constructed(&program)?;
        let mut text = Vec::new();
        super::super::writer::write(&program, &mut text).map_err(Error::Output)?;
        let text = String::from_utf8(text).map_err(|_| Error::Limit)?;
        assert!(text.contains("$value__1 : i32"));
        assert!(text.contains("$value__2 : i32"));
        Ok(())
    }

    #[test]
    fn incoming_policy_distinguishes_dead_jumps_and_branch_seeds() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "incoming", &[], Type::I32)?;
        builder.block("entry")?;
        let one = builder.integer(1)?;
        builder.effect(I::Return(Type::I32, Some(one)))?;
        // A disconnected jump chain is not an incoming edge, even when the
        // placed blocks occur in reverse propagation order.
        let first = builder.block_id("first")?;
        let middle = builder.block_id("middle")?;
        let end = builder.block_id("end")?;
        builder.place_block(middle)?;
        builder.effect(I::Jump(end))?;
        builder.place_block(first)?;
        builder.effect(I::Jump(middle))?;
        builder.place_block(end)?;
        assert!(!builder.insertion_targeted(&[])?);
        assert!(builder.insertion_targeted(&[first])?);
        // A branch seeds its targets even from a dead block. The resulting
        // cycle must converge and propagate through the ordinary jump chain.
        builder.effect(I::Branch(one, first, first))?;
        assert!(builder.insertion_targeted(&[])?);
        builder.finish()?;
        validator::validate_constructed(&program)?;
        Ok(())
    }

    #[test]
    fn sparse_locations_survive_reassignment_and_another_function() -> Result<(), Error> {
        use crate::lowir::{model::DebugLocation, writer};
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "located", &[], Type::I64)?;
        let file = builder.program.names.intern_bytes(b"source\xff.cpp")?;
        let location = DebugLocation {
            file,
            line: 7,
            column: 9,
        };
        builder.function.location = Some(location);
        builder.block("entry")?;
        let literal = builder.integer(1)?;
        let value = builder.value(I::Const(Type::I64, literal), "v")?;
        let Value::Temp(temp) = value else {
            return Err(Error::Limit);
        };
        let id = builder.body.temp(temp).definition.ok_or(Error::Limit)?;
        builder.body.set_location(id, location)?;
        // Replacing one instruction's location does not retarget any other instruction.
        builder.body.set_location(
            id,
            DebugLocation {
                line: 8,
                ..location
            },
        )?;
        builder.effect(I::Return(Type::I64, Some(value)))?;
        builder.finish()?;
        exercises::build(&mut program, "sum")?;
        validator::validate_constructed(&program)?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        assert_eq!(
            body.location(body.instruction(id)).map(|loc| loc.line),
            Some(8)
        );
        assert!(body.location(&body.instructions()[1]).is_none());
        let mut output = Vec::new();
        writer::write(&program, &mut output).map_err(Error::Output)?;
        let expected = b"function @located() -> i64 !dbg(source\xff.cpp, 7, 9) {\n  block ^entry:\n    %v = const i64 1 !dbg(source\xff.cpp, 8, 9)\n    return i64 %v\n}\n";
        assert!(output.starts_with(expected));
        Ok(())
    }

    #[test]
    fn runtime_operands_count_repeated_uses_and_survive_other_function_construction()
    -> Result<(), Error> {
        use crate::lowir::{model::RuntimeInstruction as R, vocabulary::MemoryOrder};
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(
            &mut program,
            "atomic",
            &[("p", Type::Ptr), ("n", Type::I64)],
            Type::I64,
        )?;
        builder.block("entry")?;
        let address = builder.parameter(0)?;
        let desired = builder.parameter(1)?;
        let result = builder.value(
            I::Runtime(R::AtomicCompareExchange {
                ty: Type::I64,
                address,
                expected: address,
                desired,
                success: MemoryOrder::SequentiallyConsistent,
                failure: MemoryOrder::Acquire,
            }),
            "ok",
        )?;
        builder.effect(I::Return(Type::I64, Some(result)))?;
        builder.finish()?;
        exercises::build(&mut program, "sum")?;
        validator::validate_constructed(&program)?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let Value::Temp(address) = address else {
            return Err(Error::Limit);
        };
        let Value::Temp(desired) = desired else {
            return Err(Error::Limit);
        };
        let Value::Temp(result) = result else {
            return Err(Error::Limit);
        };
        assert_eq!(body.temp(address).uses, 2);
        assert_eq!(body.temp(desired).uses, 1);
        assert_eq!(body.temp(result).uses, 1);
        Ok(())
    }

    #[test]
    fn calls_keep_body_owned_argument_slices_and_count_callee_uses() -> Result<(), Error> {
        let mut program = Program::default();
        exercises::build(&mut program, "call")?;
        validator::validate_constructed(&program)?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let calls: Vec<_> = body
            .instructions()
            .iter()
            .filter_map(|inst| match &inst.kind {
                I::Call(call) => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 2);
        let Value::Temp(callee) = calls[0].callee else {
            return Err(Error::Limit);
        };
        assert_eq!(body.temp(callee).uses, 2);
        assert!(
            matches!(body.arguments(calls[1].arguments), [Value::Temp(id)] if body.temp(*id).definition.is_some())
        );
        // Repeated construction owns another body, while the first function's IDs/facts survive.
        exercises::build(&mut program, "sum")?;
        validator::validate_constructed(&program)?;
        let first = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        assert_eq!(first.temp(callee).uses, 2);
        Ok(())
    }

    #[test]
    fn construction_audit_rejects_missing_value_results_and_parameter_collisions()
    -> Result<(), Error> {
        let mut program = Program::default();
        assert!(
            FunctionBuilder::new(
                &mut program,
                "bad_params",
                &[("x", Type::I64), ("x", Type::Ptr)],
                Type::Void
            )
            .is_err()
        );
        let mut program = Program::default();
        let mut b = FunctionBuilder::new(&mut program, "bad_result", &[], Type::I64)?;
        b.block("entry")?;
        let literal = b.integer(1)?;
        b.effect(I::Const(Type::I64, literal))?;
        b.effect(I::Return(Type::I64, Some(literal)))?;
        b.finish()?;
        let error = validator::validate_constructed(&program);
        assert!(
            matches!(error, Err(Error::Diagnostic(message)) if message == "value instruction without its result temporary in function @bad_result")
        );
        Ok(())
    }
    #[test]
    fn floating_constants_keep_bits_and_kind_in_their_body() -> Result<(), Error> {
        use super::super::{literals::FloatKind, writer};
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "floating", &[], Type::F80)?;
        builder.block("entry")?;
        let bits = (0x3fff_u128 << 64) | 0x8000_0000_0000_0001;
        let literal = builder.constant(Constant::Float(FloatKind::F80, bits))?;
        let result = builder.value(I::Const(Type::F80, literal), "x")?;
        builder.effect(I::Return(Type::F80, Some(result)))?;
        builder.finish()?;
        exercises::build(&mut program, "sum")?;
        validator::validate_constructed(&program)?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let Value::Float(id, kind) = literal else {
            return Err(Error::Limit);
        };
        assert_eq!(kind, FloatKind::F80);
        assert_eq!(body.literal(id), bits);
        let Value::Temp(result) = result else {
            return Err(Error::Limit);
        };
        assert_eq!(body.temp(result).uses, 1);
        let mut output = Vec::new();
        writer::write(&program, &mut output).map_err(Error::Output)?;
        let output = String::from_utf8(output).map_err(|_| Error::Limit)?;
        assert!(output.contains("const f80 1.0000000000000000001L"));
        Ok(())
    }
    #[test]
    fn deferred_header_observes_late_roots_after_releasing_the_body() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "destructor", &[], Type::Void)?;
        builder.block("entry")?;
        builder.effect(I::Return(Type::Void, None))?;
        builder.finish()?;
        let mut body = Vec::new();
        let symbol = program.stream_last_body(&mut body)?;
        assert!(program.function_defined(symbol));
        assert!(
            program
                .function(program.symbol(symbol))
                .unwrap()
                .body
                .is_none()
        );
        program.root_function(symbol);
        let function = program.function(program.symbol(symbol)).unwrap();
        let mut rendered = Vec::new();
        crate::lowir::writer::write_definition_header(&program, function, &mut rendered)
            .map_err(Error::Output)?;
        rendered.extend(body);
        assert_eq!(rendered, b"function @destructor() -> void [object_root=yes] {\n  block ^entry:\n    return void\n}\n");
        Ok(())
    }
    #[test]
    fn late_declarations_do_not_replace_the_completed_definition() -> Result<(), Error> {
        struct Sink;
        impl crate::lowir::model::DefinitionSink for Sink {
            fn append(
                &mut self,
                program: &Program,
                symbol: SymbolId,
                held: bool,
            ) -> Result<(), Error> {
                assert!(!held);
                assert_eq!(program.names.get(program.symbol(symbol).name), "body");
                assert!(
                    program
                        .function(program.symbol(symbol))
                        .unwrap()
                        .body
                        .is_some()
                );
                Ok(())
            }
            fn release_held(&mut self) {}
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }
        for native in [false, true] {
            let mut program = Program::default();
            if native {
                program.set_definition_sink(Box::new(Sink))?;
            }
            let mut builder = FunctionBuilder::new(&mut program, "body", &[], Type::Void)?;
            builder.block("entry")?;
            builder.effect(I::Return(Type::Void, None))?;
            builder.finish()?;
            let name = program.names.intern("body")?;
            let body = program.find_symbol(name).unwrap();
            let signature = program.function(program.symbol(body)).unwrap().signature;
            let name = program.names.intern("late_declaration")?;
            let source = program.add_source(std::path::Path::new(""))?;
            let declaration = program.reference(name, source, 0)?;
            program.add(
                declaration,
                Function {
                    name,
                    signature,
                    metadata: Metadata::default(),
                    body: None,
                    location: None,
                },
            )?;
            let mut output = Vec::new();
            program.stream_last_definition(&mut output)?;
            assert!(program.function_defined(body));
            assert!(!program.function_defined(declaration));
            assert!(
                program
                    .functions()
                    .iter()
                    .all(|function| function.body.is_none())
            );
            if !native {
                assert_eq!(
                    output,
                    b"function @body() -> void {\n  block ^entry:\n    return void\n}\n"
                );
            }
        }
        Ok(())
    }
    #[test]
    fn generated_names_avoid_later_slots_and_streamed_bodies_are_released() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "first", &[("t1", Type::I32)], Type::I32)?;
        builder.block("entry")?;
        let parameter = builder.parameter(0)?;
        let value = builder.generated(I::Copy(Type::I32, parameter))?;
        let _later = builder.slot("t2", Type::I32)?;
        builder.effect(I::Return(Type::I32, Some(value)))?;
        builder.finish()?;
        validator::validate_constructed(&program)?;
        let mut output = Vec::new();
        program.stream_last_definition(&mut output)?;
        assert!(program.functions()[0].body.is_none());
        let mut builder = FunctionBuilder::new(&mut program, "second", &[], Type::I32)?;
        builder.block("entry")?;
        let literal = builder.integer(3)?;
        let value = builder.generated(I::Copy(Type::I32, literal))?;
        builder.effect(I::Return(Type::I32, Some(value)))?;
        builder.finish()?;
        validator::validate_constructed(&program)?;
        program.stream_last_definition(&mut output)?;
        assert!(
            program
                .functions()
                .iter()
                .all(|function| function.body.is_none())
        );
        let output = String::from_utf8(output).map_err(|_| Error::Limit)?;
        assert!(output.contains("%t3 = copy i32 %t1"));
        assert!(output.contains("%t1 = copy i32 3"));
        program.finish_symbols()?;
        Ok(())
    }
}
