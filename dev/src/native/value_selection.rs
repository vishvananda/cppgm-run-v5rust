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
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Arity, DebugLocation, Function, InstructionKind as I, Program, SymbolId, Value},
    vocabulary::Type,
};
pub(super) fn unsupported() -> Error {
    Error::NotImplemented("native instruction selection and placement")
}
fn emit_runtime(
    selection: &mut Selection<'_>,
    result: Option<crate::lowir::model::TempId>,
    runtime: &crate::lowir::model::RuntimeInstruction,
    targets: &[Option<super::mir::BlockId>],
) -> Result<(), Error> {
    use crate::lowir::model::RuntimeInstruction as Runtime;
    match *runtime {
        Runtime::VaStart(list) => selection.va_start(list),
        Runtime::VaArg(ty, list, classes) => {
            selection.va_arg(result.ok_or(Error::Limit)?, ty, list, classes)
        }
        Runtime::StackAlloc(size) => selection.stack_alloc(result.ok_or(Error::Limit)?, size),
        Runtime::AtomicLoad(..)
        | Runtime::AtomicStore(..)
        | Runtime::AtomicUpdate(..)
        | Runtime::AtomicCompareExchange { .. }
        | Runtime::Fence(..) => selection.atomic(result, runtime),
        _ => selection.exception(result, runtime, targets),
    }
}
fn floating_scratch(
    program: &Program,
    function: &Function,
    body: &crate::lowir::model::Body,
) -> bool {
    let signature = program.signature(function.signature);
    let mut floating = signature.boundary.arity == Arity::Variadic
        || signature.result.float_rank() != 0
        || signature.parameters.iter().any(|p| p.ty.float_rank() != 0);
    for inst in body.instructions() {
        if floating {
            break;
        }
        floating |= matches!(inst.kind, I::Call(_)) && function.metadata.flags & 1 != 0;
        floating |= match inst.kind {
            I::Store(_, ty, _, _) | I::Compare(_, ty, _, _) => ty,
            _ => inst.kind.result_type(),
        }
        .float_rank()
            != 0;
        body.visit_values(&inst.kind, |v| floating |= matches!(v, Value::Float(..)) || matches!(v, Value::Temp(t) if body.temp(t).ty.is_some_and(|ty| ty.float_rank() != 0)));
        if let I::Call(call) = &inst.kind
            && let Some(signature) = program.call_boundary(call)
        {
            floating |= signature.result.float_rank() != 0
                || signature.parameters.iter().any(|p| p.ty.float_rank() != 0);
        }
    }
    floating
}
fn emit_entry(selection: &mut Selection<'_>) -> Result<(), Error> {
    let first = selection.mir.instructions.len();
    let mut moves =
        selection
            .placement
            .entry(selection.body, &selection.facts, &mut selection.mir)?;
    super::parallel_transfer::emit(selection, &mut moves)?;
    if selection.facts.level >= 1 {
        super::host_parameters::fold_entry_extensions(&mut selection.mir.instructions[first..]);
    }
    Ok(())
}
// Signature register classes and aligned local storage jointly reserve the
// physical entry pool before either allocator or selector consumes it.
struct EntryRegisters {
    vector_boundary: bool,
    excluded: u32,
    reserved: u32,
}
impl EntryRegisters {
    fn analyze(
        body: &crate::lowir::model::Body,
        slots: &[Slot],
        mir: &MirFunction,
        program: &Program,
        function: &Function,
    ) -> Self {
        let signature = program.signature(function.signature);
        let vector_boundary =
            signature
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
        let gprs = mir.layout.arguments.iter().filter(|arg| matches!(mir.layout.parts(arg), [part] if part.register.is_some_and(|r| r.code() < 16))).count();
        let reserved = if slots.iter().zip(body.slots()).any(|(state, slot)| {
            matches!(state, Slot::Memory)
                && matches!(slot.ty, Type::Object { .. })
                && abi::value_align(slot.ty).is_ok_and(|align| align > 16)
        }) {
            1 << R::R12.code()
        } else {
            0
        };
        Self {
            vector_boundary,
            reserved,
            excluded: if vector_boundary && gprs >= 2 {
                1 << R::R8.code()
            } else {
                0
            },
        }
    }
}
pub(super) fn select(
    program: &Program,
    symbol: SymbolId,
    function: &Function,
    level: u8,
) -> Result<MirFunction, Error> {
    select_for_object(
        program,
        symbol,
        function,
        level,
        super::object_compile::ObjectModel::Course,
    )
}
#[expect(
    clippy::too_many_lines,
    reason = "cohesive per-function selection driver and typed instruction dispatch; audited below 360 lines"
)]
pub(super) fn select_for_object(
    program: &Program,
    symbol: SymbolId,
    function: &Function,
    level: u8,
    model: super::object_compile::ObjectModel,
) -> Result<MirFunction, Error> {
    let body = function.body.as_ref().ok_or(Error::Limit)?;
    let signature = program.signature(function.signature);
    let layout = abi::Layout::classify(signature)?;
    let mut flow = super::control_flow::Flow::analyze(body)?;
    let mut region_flow = if model == super::object_compile::ObjectModel::Host
        && super::region_flow::needed(body, &flow)
    {
        Some(super::region_flow::analyze(
            program, symbol, body, &mut flow,
        )?)
    } else {
        None
    };
    if level >= 1
        && model == super::object_compile::ObjectModel::Host
        && (flow.reachable.iter().filter(|&&r| r).count() > 1
            || !flow.predecessors(body.layout[0]).is_empty())
    {
        return Err(Error::NotImplemented(
            "native optimized host allocation and shrink wrapping",
        ));
    }
    let slots = slot_analysis::analyze(body, &flow)?;
    let mut facts = if model == super::object_compile::ObjectModel::Course {
        Facts::analyze(body, &slots, flow, level)?
    } else {
        Facts::analyze_for_object(body, &slots, flow, level, model)?
    };
    // Address folding suppresses emission, not the typed reads that decide
    // host parameter normalization (notably a lazy array index).
    let host_parameters = if model == super::object_compile::ObjectModel::Host {
        super::host_parameters::observe(body, &facts)
    } else {
        Vec::new()
    };
    let mut addresses = super::address_facts::Addresses::analyze(program, body, &mut facts, model)?;
    super::branch_tests::fold(body, &mut facts, &addresses);
    let updates = super::memory_updates::Updates::analyze(body, &mut facts, &mut addresses);
    super::branch_tests::bytes_only(
        program,
        body,
        &mut facts,
        model == super::object_compile::ObjectModel::Host,
    );
    let calls = super::call_facts::Calls::analyze(program, body, &facts)?;
    facts.apply_calls(body, &calls)?;
    let mut mir = MirFunction::new(
        symbol,
        function.signature,
        body.block(body.layout[0]).name,
        layout,
    );
    mir.optimized = level >= 1;
    mir.frame_policy.records_pushed = level >= 1
        && model == super::object_compile::ObjectModel::Course
        && facts.flow.instructions.iter().enumerate().any(|(p, &id)| {
            facts.active[p]
                && matches!(
                    body.instruction(id).kind,
                    I::Runtime(
                        crate::lowir::model::RuntimeInstruction::EhTry(_)
                            | crate::lowir::model::RuntimeInstruction::EhCleanup(_)
                    )
                )
        })
        && !facts.flow.instructions.iter().enumerate().any(|(p, &id)| {
            facts.active[p]
                && matches!(
                    body.instruction(id).kind,
                    I::Runtime(crate::lowir::model::RuntimeInstruction::StackAlloc(_))
                )
        });
    mir.object_model = model;
    mir.variadic = signature.boundary.arity == Arity::Variadic;
    mir.lowir_instructions = body.instructions().len();
    mir.dead_slots = slots.iter().filter(|s| matches!(s, Slot::Dead)).count();
    mir.promoted_slots = slots
        .iter()
        .filter(|s| matches!(s, Slot::Promoted(_)))
        .count();
    let registers = EntryRegisters::analyze(body, &slots, &mir, program, function);
    let rbp_free = super::register_allocation::rbp_available(
        body,
        &facts,
        &calls,
        signature.boundary.arity,
        registers.reserved,
        model,
    );
    let globals = if level >= 1 {
        super::register_allocation::analyze(
            program,
            body,
            &mut facts,
            &addresses,
            &mir.layout,
            registers.reserved,
            rbp_free,
            model == super::object_compile::ObjectModel::Host,
            region_flow.as_ref(),
            &calls,
        )?
    } else {
        super::global_placement::Globals::analyze(
            body,
            &facts,
            &addresses,
            &mir.layout,
            registers.reserved,
        )?
    };
    mir.global_registers = globals.count;
    mir.liveness_steps = globals.steps;
    mir.hull_fallbacks = globals.fallbacks;
    let mut control = if let Some(plan) = region_flow.as_mut() {
        super::control_selection::Control::with_eh(body, &facts, Some(plan))?
    } else {
        super::control_selection::Control::new(body, &facts)?
    };
    mir.edge_blocks = control.edges.len();
    let mut placement = Placement::new(
        body,
        &mir.layout,
        globals,
        registers.vector_boundary,
        registers.excluded,
    );
    placement.host_parameters = host_parameters;
    if let Some(plan) = &mut region_flow {
        if !plan.tables.landings.is_empty() {
            plan.tables.slot = placement.frame_alloc(if plan.tables.raw { 24 } else { 16 }, 8)?;
        }
        mir.eh = std::mem::take(&mut plan.tables);
    }
    let mut selection = Selection {
        program,
        updates,
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
    selection.placement.layout_slots(
        body,
        &slots,
        &selection.facts,
        &selection.addresses,
        &mut selection.mir,
    )?;
    selection.mir.lazy_addresses = selection.addresses.lazy_count;
    selection.mir.folded_loads = selection.addresses.folded_count;
    selection.mir.fused_compares = selection.facts.fused.iter().filter(|&&v| v).count();
    let floating = floating_scratch(program, function, body);
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
        emit_entry(&mut selection)?;
        selection.jump(control.targets[entry.index()].ok_or(Error::Limit)?);
        selection.mir.end_block();
    }
    for &block in &body.layout {
        if !selection.facts.flow.reachable[block.index()] {
            continue;
        }
        control.prepare(block);
        selection.location = None;
        if let Some(plan) = &region_flow {
            selection.host_pad(
                plan,
                block,
                control.targets[block.index()].ok_or(Error::Limit)?,
            )?;
        }
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
            emit_entry(&mut selection)?;
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
                I::Runtime(ref runtime) => {
                    if let Some(plan) = &region_flow
                        && super::landing_pad::runtime(runtime)
                    {
                        selection.host_exception(
                            plan,
                            id,
                            inst.result,
                            runtime,
                            &control.targets,
                        )?;
                    } else {
                        emit_runtime(&mut selection, inst.result, runtime, &control.targets)?;
                    }
                }
                I::Load(_, Type::I128, address) => {
                    selection.wide_load(inst.result.ok_or(Error::Limit)?, address)?;
                }
                I::Store(_, Type::I128, value, address) => selection.wide_store(value, address)?,
                I::Copy(Type::I128, value) => {
                    selection.wide_copy(inst.result.ok_or(Error::Limit)?, value)?;
                }
                I::CopyObject(bytes, align, source, destination) => {
                    selection.bulk(bytes, align, Some(source), destination)?;
                }
                I::ZeroInit(bytes, align, destination) => {
                    selection.bulk(bytes, align, None, destination)?;
                }
                I::Jump(..) | I::Branch(..) | I::Switch(..) => {
                    if let Some(plan) = &region_flow
                        && let I::Jump(to) = inst.kind
                    {
                        if plan.sites[id.index()] == super::region_flow::Site::None {
                            selection.host_entry_selector(plan.entry_selectors[to.index()]);
                            selection.terminator(&control, block, &inst.kind)?;
                        } else {
                            selection.host_resume(
                                plan,
                                plan.sites[id.index()],
                                &control.targets,
                            )?;
                        }
                    } else {
                        selection.terminator(&control, block, &inst.kind)?;
                    }
                }
                I::Call(ref call) => {
                    let first = selection.mir.instructions.len();
                    selection.call(
                        program,
                        inst.result,
                        call,
                        calls.at(pos).ok_or(Error::Limit)?,
                    )?;
                    if let Some(plan) = &region_flow {
                        super::landing_pad::protect_call(&mut selection.mir, first, plan, id)?;
                    }
                }
                I::Load(_, ty @ Type::Object { .. }, address) => {
                    selection.object_load(inst.result.ok_or(Error::Limit)?, ty, address)?;
                }
                I::Store(_, ty @ Type::Object { .. }, value, address) => {
                    selection.object_store(ty, value, address)?;
                }
                I::Copy(ty @ Type::Object { .. }, value) => {
                    selection.object_copy(inst.result.ok_or(Error::Limit)?, ty, value)?;
                }
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
                I::Binary(op, flags, ty, a, b) => {
                    selection.binary(inst.result.ok_or(Error::Limit)?, op, flags, ty, a, b)?;
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
                    if super::aggregate::memory_value(ty) {
                        selection.aggregate_return(ty, value.ok_or(Error::Limit)?)?;
                    } else {
                        emit_return(&mut selection, ty, value, body.location(inst))?;
                    }
                }
                I::Unreachable => selection.mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::Trap,
                    location: selection.location,
                }),
                I::Const(..) => return Err(unsupported()),
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
    selection.mir.scratch_bytes = if floating || selection.mir.scratch_bytes != 0 {
        48
    } else {
        0
    };
    selection.placement.finish_frame(&mut selection.mir)?;
    selection.mir.frame_policy.omit_pointer = super::frame_encoding::can_omit(&selection.mir);
    if level >= 1 {
        super::return_slots::forward(&mut selection.mir);
        super::home_forwarding::forward(&mut selection.mir);
        super::mir_cleanup::extensions(&mut selection.mir);
        selection.placement.optimize_frame(&mut selection.mir)?;
    }
    if level >= 1 {
        super::branch_layout::layout(&mut selection.mir, program)?;
    }
    if level >= 2 && model == super::object_compile::ObjectModel::Host {
        super::tail_calls::form(&mut selection.mir);
    }
    Ok(selection.mir)
}
fn emit_return(
    selection: &mut Selection<'_>,
    ty: Type,
    value: Option<Value>,
    location: Option<DebugLocation>,
) -> Result<(), Error> {
    if ty.float_rank() != 0
        && let Some(value) = value
    {
        return selection.float_return(ty, value);
    }
    let Some(value) = value else {
        let mir = &mut selection.mir;
        let mut emit = |op| {
            mir.instructions.push(Instruction {
                volatile: false,
                op,
                location,
            });
        };
        emit(Op::Return(None));
        return Ok(());
    };
    let source = selection.facts.resolve(selection.body, value);
    match source {
        Alias::Constant(value) => {
            let mut emit = |op| {
                selection.mir.instructions.push(Instruction {
                    volatile: false,
                    op,
                    location,
                });
            };
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
        Alias::Temporary(temp) => {
            let own = selection.body.temp(temp).ty.ok_or(Error::Limit)?;
            let source = selection.operand(value, ty, R::R10)?;
            let reg = if selection.facts.level >= 1
                && source.register().is_some()
                && ty != own
                && ty.bits() > 0
                && ty.bits() < 64
                && selection.facts.fits[temp.index()] > super::integer_facts::capacity(ty)
            {
                selection.load(R::RAX, source, Type::I64);
                selection.normalize(R::RAX, ty);
                R::RAX
            } else if let Some(reg) = source.register() {
                reg
            } else {
                selection.load(R::RAX, source, ty);
                R::RAX
            };
            selection.mir.instructions.push(Instruction {
                volatile: false,
                op: Op::Return(Some(reg)),
                location,
            });
        }
        Alias::Slot(_) => {
            super::memory::scalar(ty)?;
            let source = selection.operand(value, ty, R::R10)?;
            selection.load(R::RAX, source, ty);
            selection.mir.instructions.push(Instruction {
                volatile: false,
                op: Op::Return(Some(R::RAX)),
                location,
            });
        }
        Alias::Symbol(symbol) => {
            let source = selection.symbol_operand(symbol, 0);
            let mut emit = |op| {
                selection.mir.instructions.push(Instruction {
                    volatile: false,
                    op,
                    location,
                });
            };
            if ty == Type::I128 || ty.float_rank() != 0 {
                return Err(unsupported());
            }
            if matches!(source, super::scalar_mir::Operand::AccessedSymbol(..)) {
                emit(Op::Scalar(super::scalar_mir::ScalarOp::Move(
                    R::RAX,
                    source,
                )));
            } else {
                emit(Op::SymbolMove(R::RAX, symbol));
            }
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
