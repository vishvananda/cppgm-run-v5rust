//! Native function driver over typed analysis, placement and instruction selection. See NOTICE.
pub(super) use super::value_facts::alias_compatible;
use super::{
    abi,
    mir::{Function as MirFunction, Instruction, Op},
    placement::Placement,
    scalar_selection::Selection,
    slot_analysis::{self, Slot},
    value_facts::{Alias, Facts, normalize},
    x86::Register as R,
};
use crate::{
    lowir::{
        Error,
        literals::{self, Constant, FloatKind},
        model::{Arity, DebugLocation, Function, InstructionKind as I, Program, SymbolId, Value},
        vocabulary::Type,
    },
    types::floating::FloatingScanner,
};
pub(super) fn unsupported() -> Error {
    Error::NotImplemented("native instruction selection and placement")
}
pub(super) fn select(
    program: &Program,
    symbol: SymbolId,
    function: &Function,
    level: u8,
) -> Result<MirFunction, Error> {
    let body = function.body.as_ref().ok_or(Error::Limit)?;
    if level != 0 {
        return Err(unsupported());
    }
    let signature = program.signature(function.signature);
    if signature.boundary.arity == Arity::Variadic
        || matches!(signature.result, Type::Object { .. })
    {
        return Err(unsupported());
    }
    let layout = abi::Layout::classify(signature)?;
    if layout.indirect_result {
        return Err(unsupported());
    }
    let flow = super::control_flow::Flow::analyze(body)?;
    let slots = slot_analysis::analyze(body, &flow)?;
    let mut facts = Facts::analyze(body, &slots, flow)?;
    let addresses = super::address_facts::Addresses::analyze(body, &mut facts)?;
    let calls = super::call_facts::Calls::analyze(program, body, &facts)?;
    facts.apply_calls(body, &calls);
    let mut mir = MirFunction::new(
        symbol,
        function.signature,
        body.block(body.layout[0]).name,
        layout,
    );
    mir.lowir_instructions = body.instructions().len();
    mir.dead_slots = slots.iter().filter(|s| matches!(s, Slot::Dead)).count();
    mir.promoted_slots = slots
        .iter()
        .filter(|s| matches!(s, Slot::Promoted(_)))
        .count();
    let vector_boundary = signature
        .parameters
        .iter()
        .zip(&mir.layout.arguments)
        .any(|(p, arg)| {
            matches!(p.ty, Type::F32 | Type::F64)
                && mir
                    .layout
                    .parts(arg)
                    .iter()
                    .any(|part| part.register.is_some())
        });
    let gprs=signature.parameters.iter().zip(&mir.layout.arguments).filter(|(p,_)|!matches!(p.ty,Type::Object{..}|Type::I128)||p.classes.is_none()).filter(|(_,arg)|matches!(mir.layout.parts(arg),[part] if part.register.is_some_and(|r|r.code()<16))).count();
    let reserved = if slots.iter().zip(body.slots()).any(|(state, slot)| {
        matches!(state, Slot::Memory)
            && matches!(slot.ty, Type::Object { .. })
            && abi::value_align(slot.ty).is_ok_and(|align| align > 16)
    }) {
        1 << R::R12.code()
    } else {
        0
    };
    let globals =
        super::global_placement::Globals::analyze(body, &facts, &addresses, &mir.layout, reserved)?;
    mir.global_registers = globals.count;
    mir.liveness_steps = globals.steps;
    mir.hull_fallbacks = globals.fallbacks;
    let mut control = super::control_selection::Control::new(body, &facts)?;
    mir.edge_blocks = control.edges.len();
    let placement = Placement::new(
        body,
        &mir.layout,
        globals,
        vector_boundary,
        if vector_boundary && gprs >= 2 {
            1 << R::R8.code()
        } else {
            0
        },
    );
    let mut selection = Selection {
        phis: super::control_selection::PhiInputs::default(),
        edge_moves: Vec::new(),
        parallel: super::parallel_transfer::Workspace::default(),
        outgoing: super::call_selection::Staging::default(),
        addresses,
        body,
        facts,
        placement,
        mir,
        location: None,
        pos: 0,
    };
    selection
        .placement
        .layout_slots(body, &slots, &mut selection.mir)?;
    selection.mir.lazy_addresses = selection.addresses.lazy_count;
    selection.mir.folded_loads = selection.addresses.folded_count;
    selection.mir.fused_compares = selection.facts.fused.iter().filter(|&&v| v).count();
    let mut floating = signature.result.float_rank() != 0
        || signature.parameters.iter().any(|p| p.ty.float_rank() != 0);
    let mut scanner = FloatingScanner::default();
    for inst in body.instructions() {
        floating |= matches!(inst.kind, I::Call(_)) && function.metadata.flags & 1 != 0;
        floating |= match inst.kind {
            I::Store(_, ty, _, _) => ty,
            _ => inst.kind.result_type(),
        }
        .float_rank()
            != 0;
        body.visit_values(&inst.kind, |v| floating |= matches!(v, Value::Float(..)));
    }
    selection.mir.blocks.clear();
    if control.entry_loop {
        let entry = body.layout[0];
        selection
            .placement
            .enter_block(body, &selection.facts, &selection.addresses, entry);
        selection.mir.begin_block(super::mir::BlockName::Edge(
            body.block(entry).name,
            body.block(entry).name,
        ))?;
        let mut moves = selection
            .placement
            .entry(body, &selection.facts, &mut selection.mir)?;
        super::parallel_transfer::emit(&mut selection, &mut moves)?;
        selection.jump(control.targets[entry.index()].ok_or(Error::Limit)?);
        selection.mir.end_block();
    }
    for &block in &body.layout {
        if !selection.facts.flow.reachable[block.index()] {
            continue;
        }
        control.prepare(block);
        selection.location = None;
        selection
            .placement
            .enter_block(body, &selection.facts, &selection.addresses, block);
        let created = selection
            .mir
            .begin_block(super::mir::BlockName::Named(body.block(block).name))?;
        if Some(created) != control.targets[block.index()] {
            return Err(Error::Construction("native block out of order".into()));
        }
        if block == body.layout[0] && !control.entry_loop {
            let mut moves =
                selection
                    .placement
                    .entry(body, &selection.facts, &mut selection.mir)?;
            super::parallel_transfer::emit(&mut selection, &mut moves)?;
        }
        for pos in selection.facts.flow.ranges[block.index()].clone() {
            let id = selection.facts.flow.instructions[pos];
            let inst = body.instruction(id);
            if !selection.facts.active[pos] {
                continue;
            }
            selection.pos = pos;
            selection.location = body.location(inst);
            match inst.kind {
                I::Phi(..) => {}
                I::Jump(..) | I::Branch(..) | I::Switch(..) => {
                    selection.terminator(&control, block, &inst.kind)?;
                }
                I::Call(ref call) => selection.call(
                    program,
                    inst.result,
                    call,
                    calls.at(pos).ok_or(Error::Limit)?,
                )?,
                I::Load(_, ty, address) => {
                    selection.read_memory(inst.result.ok_or(Error::Limit)?, ty, address)?;
                }
                I::Store(_, ty, value, address) => selection.write_memory(ty, value, address)?,
                I::Addr(_) => selection.select_address(inst.result.ok_or(Error::Limit)?)?,
                I::Index(ty, _, base, offset) => {
                    selection.select_index(inst.result.ok_or(Error::Limit)?, ty, base, offset)?;
                }
                I::Copy(ty, value) => {
                    selection.copy(inst.result.ok_or(Error::Limit)?, ty, value)?;
                }
                I::Binary(op, _, ty, a, b) => {
                    selection.binary(inst.result.ok_or(Error::Limit)?, op, ty, a, b)?;
                }
                I::Unary(op, ty, value) => {
                    selection.unary(inst.result.ok_or(Error::Limit)?, op, ty, value)?;
                }
                I::Compare(pred, ty, a, b) => {
                    selection.compare(inst.result.ok_or(Error::Limit)?, pred, ty, a, b)?;
                }
                I::Convert(op, to, from, value) => {
                    selection.convert(inst.result.ok_or(Error::Limit)?, op, to, from, value)?;
                }
                I::Select(ty, cond, yes, no) => {
                    selection.select_value(inst.result.ok_or(Error::Limit)?, ty, cond, yes, no)?;
                }
                I::Return(ty, value) => {
                    emit_return(&mut selection, ty, value, body.location(inst), &mut scanner)?;
                }
                I::Unreachable => selection.mir.instructions.push(Instruction {
                    op: Op::Trap,
                    location: selection.location,
                }),
                _ => return Err(unsupported()),
            }
            let (late, early) = selection.facts.clobbers[pos];
            selection.placement.note_clobbers(late | early);
            if inst.kind.terminator() {
                selection.mir.end_block();
                selection.edge_blocks(&control, block)?;
            }
            selection.placement.release(&selection.facts, pos);
            selection.placement.retire();
        }
        selection.mir.end_block();
        selection.placement.finish_block(block);
    }
    selection.mir.scratch_bytes = if floating { 48 } else { 0 };
    selection.placement.finish_frame(&mut selection.mir)?;
    Ok(selection.mir)
}
fn emit_return(
    selection: &mut Selection<'_>,
    ty: Type,
    value: Option<Value>,
    location: Option<DebugLocation>,
    scanner: &mut FloatingScanner,
) -> Result<(), Error> {
    let Some(value) = value else {
        let mir = &mut selection.mir;
        let mut emit = |op| mir.instructions.push(Instruction { op, location });
        emit(Op::Return(None));
        return Ok(());
    };
    let source = selection.facts.resolve(selection.body, value);
    match source {
        Alias::Constant(value) => {
            let mut emit = |op| {
                selection
                    .mir
                    .instructions
                    .push(Instruction { op, location });
            };
            if let Some(kind) = FloatKind::from_type(ty) {
                let bits = literals::native_float(scanner, value, kind)?;
                if kind == FloatKind::F80 {
                    emit(Op::FloatReturn(bits));
                } else {
                    let reg = R::xmm(0)?;
                    emit(Op::FloatMove(reg, kind, bits));
                    emit(Op::Return(Some(reg)));
                }
            } else {
                let bits = match value {
                    Constant::Integer(bits) => normalize(bits, ty),
                    Constant::Float(_, bits) => bits,
                };
                let bytes = bits.to_le_bytes();
                let mut word = [0; 8];
                word.copy_from_slice(&bytes[..8]);
                if ty == Type::I128 {
                    emit(Op::Move(R::RAX, u128::from(u64::from_le_bytes(word))));
                    word.copy_from_slice(&bytes[8..]);
                    emit(Op::Move(R::RDX, u128::from(u64::from_le_bytes(word))));
                    emit(Op::Return(None));
                } else {
                    let immediate = if matches!(value, Constant::Float(..)) {
                        bits
                    } else {
                        i128::from(i64::from_le_bytes(word)).cast_unsigned()
                    };
                    emit(Op::Move(R::RAX, immediate));
                    emit(Op::Return(Some(R::RAX)));
                }
            }
        }
        Alias::Temporary(temp) => {
            let own = selection.body.temp(temp).ty.ok_or(Error::Limit)?;
            if own.float_rank() != 0
                || matches!(own, Type::Object { .. } | Type::I128)
                || matches!(ty, Type::I128 | Type::F80)
            {
                return Err(unsupported());
            }
            let source = selection.operand(value, ty, R::R10)?;
            let reg = if let Some(reg) = source.register() {
                reg
            } else {
                selection.load(R::RAX, source, ty);
                R::RAX
            };
            selection.mir.instructions.push(Instruction {
                op: Op::Return(Some(reg)),
                location,
            });
        }
        Alias::Slot(_) => {
            super::memory::scalar(ty)?;
            let source = selection.operand(value, ty, R::R10)?;
            selection.load(R::RAX, source, ty);
            selection.mir.instructions.push(Instruction {
                op: Op::Return(Some(R::RAX)),
                location,
            });
        }
        Alias::Symbol(symbol) => {
            let mut emit = |op| {
                selection
                    .mir
                    .instructions
                    .push(Instruction { op, location });
            };
            if ty == Type::I128 || ty.float_rank() != 0 {
                return Err(unsupported());
            }
            emit(Op::SymbolMove(R::RAX, symbol));
            emit(Op::Return(Some(R::RAX)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{builder::FunctionBuilder, vocabulary::Binary};

    fn chosen(program: &Program, index: usize) -> Result<MirFunction, Error> {
        let function = &program.functions()[index];
        select(
            program,
            program.find_symbol(function.name).ok_or(Error::Limit)?,
            function,
            0,
        )
    }
    #[test]
    fn backward_dead_elision_does_not_keep_parameter_registers_live() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "dead", &[("p", Type::I64)], Type::I64)?;
        builder.block("entry")?;
        let p = builder.parameter(0)?;
        let one = builder.integer(1)?;
        let a = builder.value(
            I::Binary(
                Binary::Add,
                crate::lowir::model::Facts::default(),
                Type::I64,
                p,
                one,
            ),
            "a",
        )?;
        builder.value(
            I::Unary(crate::lowir::vocabulary::Unary::Neg, Type::I64, a),
            "unused",
        )?;
        builder.effect(I::Return(Type::I64, Some(one)))?;
        builder.finish()?;
        let selected = chosen(&program, 0)?;
        assert_eq!(selected.instructions.len(), 2);
        assert_eq!(selected.homes, 0);
        assert_eq!(selected.preserve, 0);
        Ok(())
    }
    #[test]
    fn a_parameter_survives_until_its_actual_last_use() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "reuse", &[("p", Type::I64)], Type::I64)?;
        builder.block("entry")?;
        let p = builder.parameter(0)?;
        let one = builder.integer(1)?;
        let a = builder.value(
            I::Binary(
                Binary::Add,
                crate::lowir::model::Facts::default(),
                Type::I64,
                p,
                one,
            ),
            "a",
        )?;
        let b = builder.value(
            I::Binary(
                Binary::Add,
                crate::lowir::model::Facts::default(),
                Type::I64,
                p,
                a,
            ),
            "b",
        )?;
        builder.effect(I::Return(Type::I64, Some(b)))?;
        builder.finish()?;
        let selected = chosen(&program, 0)?;
        let regs: Vec<_> = selected
            .instructions
            .iter()
            .filter_map(|inst| {
                if let Op::Scalar(super::super::scalar_mir::ScalarOp::Binary(_, _, r, _)) = inst.op
                {
                    Some(r)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(regs, [R::R8, R::RDI]);
        Ok(())
    }
    #[test]
    fn spill_and_preservation_state_ends_with_its_function() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "pressure", &[("p", Type::I64)], Type::I64)?;
        builder.block("entry")?;
        let p = builder.parameter(0)?;
        let one = builder.integer(1)?;
        let mut values = Vec::new();
        for i in 0..20 {
            values.push(builder.value(
                I::Binary(
                    Binary::Add,
                    crate::lowir::model::Facts::default(),
                    Type::I64,
                    p,
                    one,
                ),
                &format!("v{i}"),
            )?);
        }
        let mut sum = values[0];
        for (i, &v) in values.iter().enumerate().skip(1) {
            sum = builder.value(
                I::Binary(
                    Binary::Add,
                    crate::lowir::model::Facts::default(),
                    Type::I64,
                    sum,
                    v,
                ),
                &format!("s{i}"),
            )?;
        }
        builder.effect(I::Return(Type::I64, Some(sum)))?;
        builder.finish()?;
        let mut builder = FunctionBuilder::new(&mut program, "plain", &[], Type::I64)?;
        builder.block("entry")?;
        let zero = builder.integer(0)?;
        builder.effect(I::Return(Type::I64, Some(zero)))?;
        builder.finish()?;
        let large = chosen(&program, 0)?;
        let small = chosen(&program, 1)?;
        assert!(large.spills > 0);
        assert_eq!(large.preserve.count_ones(), 5);
        assert!(large.stack_size >= u32::try_from(large.homes).map_err(|_| Error::Limit)? * 8);
        assert_eq!(small.stack_size, 0);
        assert_eq!(small.preserve, 0);
        assert_eq!(small.homes, 0);
        Ok(())
    }
    #[test]
    fn wide_return_words_keep_their_zero_extended_immediate_facts() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "wide", &[], Type::I128)?;
        builder.block("entry")?;
        let literal = builder.integer(u128::MAX)?;
        let value = builder.value(I::Const(Type::I128, literal), "value")?;
        builder.effect(I::Return(Type::I128, Some(value)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let selected = select(&program, symbol, function, 0)?;
        let mut output = Vec::new();
        selected.write(&program, &mut output)?;
        let output = std::str::from_utf8(&output).map_err(|_| Error::Limit)?;
        assert!(output.contains("mov rax, 0x0000000000000000ffffffffffffffff\n"));
        assert!(output.contains("mov rdx, 0x0000000000000000ffffffffffffffff\n"));
        Ok(())
    }
}
