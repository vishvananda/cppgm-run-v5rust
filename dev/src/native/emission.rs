//! Per-function O0 native block threading, branch relaxation and encoding. See NOTICE.
use super::global_data;
use super::mir::low_word;
use super::{
    frame_carries::Carries,
    image::{Image, ImageSymbol, Section},
    mir::{Emission, Function, Op},
    x86::{Assembler, Label, Register as R, Rm},
};
use crate::{
    lowir::{
        Error,
        literals::{self, FloatKind},
    },
    types::floating::FloatingScanner,
};
fn return_register(a: &mut Assembler, reg: R) -> Result<(), Error> {
    if reg.code() < 16 {
        return a.mov_rr(R::RAX, reg);
    }
    let zero = R::xmm(0)?;
    if reg != zero {
        a.sse(0, 0x28, zero, Rm::Reg(reg))?;
    }
    Ok(())
}
pub(super) fn constant(image: &mut Image, kind: FloatKind, bits: u128) -> Result<Rm, Error> {
    let mut scanner = FloatingScanner::default();
    let bytes = global_data::constant_bytes(
        &mut scanner,
        kind.ty(),
        literals::Constant::Float(kind, bits),
    )?;
    let size = if kind == FloatKind::F32 {
        4
    } else if kind == FloatKind::F64 {
        8
    } else {
        16
    };
    Ok(Rm::rip(
        image.constant(
            &bytes[..size],
            u8::try_from(size).map_err(|_| Error::Limit)?,
        )?,
        0,
    ))
}

#[derive(Clone, Copy)]
enum Chain {
    Unknown,
    Walking,
    Cycle,
    End(usize),
}
struct Order {
    targets: Vec<usize>,
    next: Vec<usize>,
}
impl Order {
    fn new(function: &Function) -> Self {
        let count = function.blocks.len();
        if function.branches_laid_out == super::mir::BranchLayout::Final {
            return Self {
                targets: (0..count).collect(),
                next: (0..=count).collect(),
            };
        }
        let mut chains = vec![Chain::Unknown; count];
        let mut walk = Vec::new();
        let jump = |block: usize| {
            let range = &function.blocks[block].range;
            if block != 0
                && !function.blocks[block].landing
                && range.len() == 1
                && let Op::Jump(target) = function.instructions[range.start].op
            {
                Some(target.index())
            } else {
                None
            }
        };
        for block in 0..count {
            walk.clear();
            let mut target = block;
            while matches!(chains[target], Chain::Unknown)
                && let Some(next) = jump(target)
            {
                chains[target] = Chain::Walking;
                walk.push(target);
                target = next;
            }
            let end = match chains[target] {
                Chain::Unknown => Chain::End(target),
                Chain::Walking => Chain::Cycle,
                other => other,
            };
            if matches!(chains[target], Chain::Unknown) {
                chains[target] = Chain::End(target);
            }
            for &at in &walk {
                chains[at] = end;
            }
        }
        let targets: Vec<_> = chains
            .into_iter()
            .enumerate()
            .map(|(index, c)| if let Chain::End(end) = c { end } else { index })
            .collect();
        let mut next = vec![count; count + 1];
        for block in (0..count).rev() {
            next[block] = if targets[block] == block {
                block
            } else {
                next[block + 1]
            };
        }
        Self { targets, next }
    }
}
struct Branch {
    offset: usize,
    label: Label,
    conditional: bool,
}
struct Pass<'a> {
    runtime: Option<super::runtime::Symbols>,
    function: &'a Function,
    assembler: Assembler,
    labels: Vec<Label>,
    epilogue: Label,
    terminal: Option<Label>,
    branches: Vec<Branch>,
    tables: Vec<(Label, super::switch_plan::Table)>,
    short: &'a [bool],
    flags: Vec<bool>,
    carries: Carries,
    stats: Emission,
    frame: super::host_unwind::Frame,
    plan: super::frame_encoding::Plan,
    block_first: usize,
    record_frame: bool,
}
impl<'a> Pass<'a> {
    fn new(
        function: &'a Function,
        order: &Order,
        short: &'a [bool],
        runtime: Option<super::runtime::Symbols>,
        record_frame: bool,
    ) -> Result<Self, Error> {
        let mut assembler = Assembler::default();
        let mut labels = Vec::with_capacity(function.blocks.len());
        for _ in &function.blocks {
            labels.push(assembler.label()?);
        }
        let epilogue = assembler.label()?;
        let terminal = if function
            .instructions
            .iter()
            .any(|i| matches!(i.op, Op::HostEh(super::landing_pad::Effect::Resume)))
        {
            Some(assembler.label()?)
        } else {
            None
        };
        for (block, &target) in order.targets.iter().enumerate() {
            labels[block] = labels[target];
        }
        Ok(Self {
            runtime,
            function,
            assembler,
            labels,
            epilogue,
            terminal,
            branches: Vec::new(),
            tables: Vec::new(),
            short,
            flags: super::condition_mir::flags_after(&function.instructions),
            block_first: 0,
            carries: Carries::new(&function.bindings),
            stats: Emission::default(),
            frame: super::host_unwind::Frame::default(),
            plan: super::frame_encoding::Plan::new(function),
            record_frame: record_frame
                && function.object_model == super::object_compile::ObjectModel::Host,
        })
    }
    fn branch(&mut self, condition: Option<u8>, label: Label) {
        let short = self
            .short
            .get(self.branches.len())
            .copied()
            .unwrap_or(false);
        self.branches.push(Branch {
            offset: self.assembler.code.len(),
            label,
            conditional: condition.is_some(),
        });
        self.assembler.branch(condition, label, short);
    }
    fn run(&mut self, image: &mut Image, order: &Order) -> Result<(), Error> {
        let f = self.function;
        self.frame.frameless = self.plan.omit;
        self.frame.reserve = self.plan.reserve;
        if !f.frame_policy.shrink_wrapped {
            self.prologue()?;
        }
        self.assembler.frame_addressing(self.plan.address);
        for block in 0..f.blocks.len() {
            if order.targets[block] != block {
                continue;
            }
            self.assembler.bind(self.labels[block]);
            if self.plan.omit && self.plan.address.reset_area() {
                self.assembler.frame_addressing(self.plan.address);
                self.frame.adjusts.push(super::host_unwind::Adjust {
                    end: self.assembler.position()?,
                    args: 0,
                });
            }
            self.carries.clear();
            let next = order.next[block + 1];
            let range = if f.blocks.len() == 1 && f.blocks[block].range.end == 0 {
                0..f.instructions.len()
            } else {
                f.blocks[block].range.clone()
            };
            self.block_first = range.start;
            for index in range.clone() {
                let op = f.instructions[index].op;
                if matches!(op, Op::Nop) {
                    continue;
                }
                let replacement = self.carries.replacement(&op);
                let pending = self.carries.track(&f.instructions[..range.end], index);
                if let Some((to, from)) = replacement {
                    self.assembler.mov_rr(to, from)?;
                    self.stats.carried_reloads += 1;
                    continue;
                }
                if f.branches_laid_out == super::mir::BranchLayout::Pending
                    && let Op::Branch(cond, target) = op
                    && index + 2 == range.end
                    && next < f.blocks.len()
                    && let Op::Jump(other) = f.instructions[index + 1].op
                    && self.labels[target.index()] == self.labels[next]
                {
                    self.branch(Some(cond.invert().code()), self.labels[other.index()]);
                    break;
                }
                if f.branches_laid_out == super::mir::BranchLayout::Pending
                    && matches!(op,Op::Jump(target) if next<f.blocks.len()&&self.labels[target.index()]==self.labels[next])
                {
                    continue;
                }
                self.instruction(
                    image,
                    op,
                    index,
                    next == f.blocks.len() && index + 1 == range.end,
                )?;
                if let Some(source) = pending {
                    self.assembler.mov_rr(R::R11, source)?;
                }
            }
        }
        self.assembler.bind(self.epilogue);
        let shared = self.epilogue_body()?;
        self.frame.leave_end = shared.released;
        self.frame.ret_end = shared.returned;
        self.frame.pop_end = shared.pops;
        if let Some(terminal) = self.terminal {
            self.assembler.bind(terminal);
            self.assembler
                .load(R::RDI, Rm::frame(f.eh.slot), 8, false)?;
            let begin = self.assembler.position()?;
            self.assembler
                .call_symbol(self.runtime.ok_or(Error::Limit)?.unwind_resume)?;
            self.frame.calls.push(super::host_unwind::Call {
                begin,
                end: self.assembler.position()?,
                landing: None,
                args: 0,
            });
        }
        if self.record_frame {
            self.frame.size = self.assembler.position()?;
            for landing in &f.eh.landings {
                self.frame.pads.push(if let Some(pad) = landing.pad {
                    u32::try_from(
                        self.assembler
                            .label_position(self.labels[pad.index()])
                            .ok_or(Error::Limit)?,
                    )
                    .map_err(|_| Error::Limit)?
                } else {
                    0
                });
            }
        }
        if !self.tables.is_empty() {
            while !self.assembler.code.len().is_multiple_of(4) {
                self.assembler.byte(0xcc);
            }
            for &(label, table) in &self.tables {
                self.assembler.bind(label);
                for &target in &f.jump_targets[table.range()] {
                    self.assembler
                        .label_offset32(self.labels[target.index()], label);
                }
            }
        }
        Ok(())
    }
    fn prologue(&mut self) -> Result<(), Error> {
        let f = self.function;
        let a = &mut self.assembler;
        let first = self.frame.prologues.is_empty();
        let mut prologue = super::host_unwind::Prologue {
            start: a.position()?,
            ..Default::default()
        };
        self.frame.frameless = self.plan.omit;
        self.frame.reserve = self.plan.reserve;
        if self.plan.omit {
            if self.plan.reserve != 0 {
                a.alu_ri(5, Rm::Reg(R::RSP), self.plan.reserve.into(), 8)?;
            }
            prologue.reserve_end = a.position()?;
            if first {
                self.frame.reserve_end = prologue.reserve_end;
            }
        } else {
            a.push(R::RBP);
            self.frame.push_end = a.position()?;
            a.mov_rr(R::RBP, R::RSP)?;
            self.frame.established_end = a.position()?;
        }
        let saved = f.preserve.count_ones() * 8;
        if !self.plan.omit && f.stack_size != saved {
            a.alu_ri(5, Rm::Reg(R::RSP), i64::from(f.stack_size - saved), 8)?;
        }
        let mut offset = if self.plan.omit {
            8_i32.wrapping_sub(self.plan.reserve.cast_signed())
        } else {
            f.stack_size
                .cast_signed()
                .wrapping_neg()
                .wrapping_add(saved.cast_signed())
        };
        let mut pushed = 0;
        for reg in super::frame_encoding::SAVES.into_iter().rev() {
            if f.preserve & (1 << reg.code()) != 0 {
                a.push(reg);
                offset = offset.wrapping_sub(8);
                prologue.save_end[pushed] = a.position()?;
                pushed += 1;
                if self.record_frame && first {
                    self.frame.saves.push((a.position()?, reg, offset));
                }
            }
        }
        if f.aligned_base != 0 {
            a.lea(
                R::R12,
                Rm::mem(
                    R::RBP,
                    f.aligned_disp
                        .wrapping_add(f.aligned_base.cast_signed())
                        .wrapping_sub(1),
                ),
            )?;
            a.alu_ri(4, Rm::Reg(R::R12), -i64::from(f.aligned_base), 8)?;
        }
        if f.frame_policy.shrink_wrapped {
            self.frame.prologues.push(prologue);
        }
        Ok(())
    }
    fn teardown(&mut self) -> Result<super::host_unwind::Epilogue, Error> {
        let f = self.function;
        if f.preserve != 0 && self.plan.dynamic {
            self.assembler.lea(
                R::RSP,
                Rm::mem(R::RBP, f.stack_size.cast_signed().wrapping_neg()),
            )?;
        }
        let mut record = super::host_unwind::Epilogue::default();
        let mut popped = 0;
        for reg in super::frame_encoding::SAVES {
            if f.preserve & (1 << reg.code()) != 0 {
                self.assembler.pop(reg);
                record.pops[popped] = self.assembler.position()?;
                popped += 1;
            }
        }
        if !self.plan.omit {
            self.assembler.byte(0xc9);
        } else if self.plan.reserve != 0 {
            self.assembler
                .alu_ri(0, Rm::Reg(R::RSP), self.plan.reserve.into(), 8)?;
        }
        record.released = self.assembler.position()?;
        Ok(record)
    }
    fn epilogue_body(&mut self) -> Result<super::host_unwind::Epilogue, Error> {
        let mut record = self.teardown()?;
        self.assembler.ret();
        record.returned = self.assembler.position()?;
        Ok(record)
    }
    fn return_jump(&mut self, last: bool) -> Result<(), Error> {
        if !last {
            if self.function.optimized {
                let record = self.epilogue_body()?;
                self.frame.returns.push(record);
            } else {
                self.branch(None, self.epilogue);
            }
        }
        Ok(())
    }
    fn argument_area(&mut self, op: super::scalar_mir::ScalarOp) -> Result<(), Error> {
        use super::{scalar_mir::Operand, scalar_mir::ScalarOp};
        use crate::lowir::vocabulary::Binary;
        let ScalarOp::Binary(
            kind @ (Binary::Add | Binary::Sub),
            _,
            R::RSP,
            Operand::Immediate(bits),
        ) = op
        else {
            return Ok(());
        };
        if self.plan.omit {
            let bytes = low_word(bits)?;
            let value = if kind == Binary::Sub {
                bytes
            } else {
                bytes.wrapping_neg()
            };
            let delta = i32::from_le_bytes(
                value.to_le_bytes()[..4]
                    .try_into()
                    .map_err(|_| Error::Limit)?,
            );
            self.plan.address.adjust(delta);
            self.assembler.frame_addressing(self.plan.address);
            self.frame.adjusts.push(super::host_unwind::Adjust {
                end: self.assembler.position()?,
                args: self.plan.address.area(),
            });
        }
        Ok(())
    }
    fn instruction(
        &mut self,
        image: &mut Image,
        op: Op,
        index: usize,
        last: bool,
    ) -> Result<(), Error> {
        let f = self.function;
        match op {
            Op::Nop => {}
            Op::Prologue => self.prologue()?,
            Op::FramelessReturn(reg) => {
                if let Some(reg) = reg {
                    return_register(&mut self.assembler, reg)?;
                }
                self.stats.returns += 1;
                self.assembler.ret();
            }
            Op::HostEh(super::landing_pad::Effect::Resume) => {
                self.branch(None, self.terminal.ok_or(Error::Limit)?);
            }
            Op::HostEh(effect) => effect.emit(&mut self.assembler, &f.eh)?,
            Op::Exception(op) => op.rebase(f.aligned_disp, f.aligned_bytes).emit(
                &mut self.assembler,
                self.runtime.ok_or(Error::Limit)?,
                &self.labels,
                f.frame_policy.records_pushed,
            )?,
            Op::JumpTable(table) => {
                let label = self.assembler.label()?;
                self.tables.push((label, table));
                self.assembler.lea_label(R::R11, label);
                self.assembler.load(
                    R::R10,
                    Rm::Memory {
                        base: Some(R::R11),
                        index: Some(R::R10),
                        scale: 4,
                        disp: 0,
                        target: None,
                    },
                    4,
                    true,
                )?;
                self.assembler.alu_rr(0, R::R10, Rm::Reg(R::R11), 8)?;
                self.assembler.jump_rm(Rm::Reg(R::R10))?;
            }
            Op::Jump(target) => self.branch(None, self.labels[target.index()]),
            Op::Branch(cond, target) => self.branch(Some(cond.code()), self.labels[target.index()]),
            Op::Move(reg, bits) => {
                self.assembler
                    .mov_ri_flags(reg, low_word(bits)?, self.flags[index])?;
            }
            Op::SymbolMove(reg, symbol) => self
                .assembler
                .lea(reg, Rm::rip(ImageSymbol::Program(symbol), 0))?,
            Op::Return(reg) => {
                if let Some(reg) = reg {
                    return_register(&mut self.assembler, reg)?;
                }
                self.stats.returns += 1;
                self.return_jump(last)?;
            }
            Op::Call(site) => {
                let done = site.emit_prefix(&mut self.assembler)?;
                let begin = self.assembler.position()?;
                site.emit(&mut self.assembler, f.aligned_disp, f.aligned_bytes)?;
                if let Some(done) = done {
                    self.assembler.bind(done);
                }
                if self.record_frame && (site.landing.is_some() || !site.no_unwind) {
                    self.frame.calls.push(super::host_unwind::Call {
                        begin,
                        end: self.assembler.position()?,
                        landing: site.landing,
                        args: site.stack.wrapping_add(15) & !15,
                    });
                }
            }
            Op::TailJump(site) => {
                use super::scalar_mir::Operand;
                let target = site.target.rebase(f.aligned_disp, f.aligned_bytes);
                match target {
                    Operand::Symbol(_) => {}
                    Operand::Register(reg) => self.assembler.mov_rr(R::R11, reg)?,
                    _ => self.assembler.load(R::R11, target.rm()?, 8, false)?,
                }
                let mut record = self.teardown()?;
                if let Operand::Symbol(symbol) = target {
                    self.assembler.jump_symbol(ImageSymbol::Program(symbol))?;
                } else {
                    self.assembler.jump_rm(Rm::Reg(R::R11))?;
                }
                record.returned = self.assembler.position()?;
                self.frame.returns.push(record);
            }
            Op::VaStart(op) => op.emit(&mut self.assembler, f.aligned_disp, f.aligned_bytes)?,
            Op::VaArg(op) => op.emit(&mut self.assembler, f.aligned_disp, f.aligned_bytes)?,
            Op::StackAlloc(op) => op.emit(&mut self.assembler)?,
            Op::DynamicCopy => {
                self.assembler.mov_rr(R::RCX, R::RDX)?;
                self.assembler.byte(0xf3);
                self.assembler.byte(0xa4);
            }
            Op::Bulk(op) => {
                let limit = if f.optimized {
                    super::bulk::chunk_limit(f, index, self.block_first, op)
                } else {
                    16
                };
                op.rebase(f.aligned_disp, f.aligned_bytes).emit(
                    &mut self.assembler,
                    f.optimized,
                    limit,
                )?;
            }
            Op::FloatCopy(op) => op
                .rebase(f.aligned_disp, f.aligned_bytes)
                .emit(&mut self.assembler, image)?,
            Op::Floating(op) => {
                op.rebase(f.aligned_disp, f.aligned_bytes)
                    .emit(&mut self.assembler, image)?;
                if matches!(
                    op,
                    super::floating_mir::FloatingOp::Return(_)
                        | super::floating_mir::FloatingOp::ReturnPair(_, _)
                ) {
                    self.stats.returns += 1;
                    self.return_jump(last)?;
                }
            }
            Op::Convert(op) => op.rebase(f.aligned_disp, f.aligned_bytes).emit(
                &mut self.assembler,
                image,
                f.scratch_offset,
                self.runtime,
            )?,
            Op::Atomic(op) => op
                .rebase(f.aligned_disp, f.aligned_bytes)
                .emit(&mut self.assembler, f.scratch_offset)?,
            Op::Wide(op) => op
                .rebase(f.aligned_disp, f.aligned_bytes)
                .emit(&mut self.assembler, self.runtime)?,
            Op::WideCompare(op) => {
                let branches = &mut self.branches;
                let short = self.short;
                op.rebase(f.aligned_disp, f.aligned_bytes).emit(
                    &mut self.assembler,
                    &self.labels,
                    |a, condition, label| {
                        let is_short = short.get(branches.len()).copied().unwrap_or(false);
                        branches.push(Branch {
                            offset: a.code.len(),
                            label,
                            conditional: condition.is_some(),
                        });
                        a.branch(condition, label, is_short);
                    },
                )?;
            }
            Op::WideMove {
                destination,
                source,
            } => {
                let destination = destination.rebase(f.aligned_disp, f.aligned_bytes);
                let source = source.rebase(f.aligned_disp, f.aligned_bytes);
                for offset in [0, 8] {
                    if let super::scalar_mir::Operand::Immediate(bits) = source {
                        let word = if offset == 0 { bits } else { bits >> 64 };
                        self.assembler.mov_ri_flags(R::R11, low_word(word)?, true)?;
                    } else {
                        self.assembler
                            .load(R::R11, source.displaced(offset).rm()?, 8, false)?;
                    }
                    self.assembler
                        .store(destination.displaced(offset).rm()?, R::R11, 8)?;
                }
            }
            Op::WideExtend {
                signed,
                destination,
                source,
            } => {
                let to = destination.rebase(f.aligned_disp, f.aligned_bytes).rm()?;
                if signed {
                    self.assembler.mov_rr(R::R11, source)?;
                    self.assembler.store(to, R::R11, 8)?;
                    self.assembler
                        .op(0, true, 0xc1, 1, 7, Rm::Reg(R::R11), false, 1)?;
                    self.assembler.imm(63, 1);
                    self.assembler.store(to.displaced(8), R::R11, 8)?;
                } else {
                    self.assembler.store(to, source, 8)?;
                    self.assembler.store_imm(to.displaced(8), 0, 8)?;
                }
            }
            Op::Scalar(op) => {
                op.rebase(f.aligned_disp, f.aligned_bytes)
                    .emit(&mut self.assembler, self.flags[index])?;
                self.argument_area(op)?;
            }
            Op::Trap => {
                self.assembler.byte(0x0f);
                self.assembler.byte(0x0b);
            }
        }
        Ok(())
    }
}
pub(super) fn emit(
    function: &Function,
    image: &mut Image,
    runtime: Option<super::runtime::Symbols>,
) -> Result<Emission, Error> {
    emit_with_frame(function, image, runtime, None)
}
pub(super) fn emit_with_frame(
    function: &Function,
    image: &mut Image,
    runtime: Option<super::runtime::Symbols>,
    frame: Option<&mut super::host_unwind::Frame>,
) -> Result<Emission, Error> {
    let base = image.align_text()?;
    image.define(ImageSymbol::Program(function.symbol), Section::Text, base);
    let order = Order::new(function);
    let mut pass = Pass::new(function, &order, &[], runtime, frame.is_some())?;
    pass.run(image, &order)?;
    let mut short = vec![false; pass.branches.len()];
    let targets = pass
        .branches
        .iter()
        .map(|branch| {
            pass.assembler
                .label_position(branch.label)
                .ok_or(Error::Limit)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut saved = vec![0_usize; short.len() + 1];
    let rounds = if function.branches_laid_out == super::mir::BranchLayout::Final {
        3
    } else {
        1
    };
    for _ in 0..rounds {
        for (i, branch) in pass.branches.iter().enumerate() {
            saved[i + 1] = saved[i]
                + if short[i] {
                    if branch.conditional { 4 } else { 3 }
                } else {
                    0
                };
        }
        let mut changed = false;
        for (i, branch) in pass.branches.iter().enumerate() {
            if short[i] {
                continue;
            }
            let before = pass
                .branches
                .partition_point(|branch| branch.offset < targets[i]);
            let target = i64::try_from(targets[i] - saved[before]).map_err(|_| Error::Limit)?;
            let offset = i64::try_from(branch.offset - saved[i] + 2).map_err(|_| Error::Limit)?;
            if i8::try_from(target - offset).is_ok() {
                short[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if short.iter().any(|&short| short) {
        pass = Pass::new(function, &order, &short, runtime, frame.is_some())?;
        pass.run(image, &order)?;
    }
    pass.assembler.resolve()?;
    #[cfg(feature = "telemetry")]
    {
        let short_count = short.iter().filter(|&&value| value).count();
        pass.stats.short_branches = short_count;
        pass.stats.long_branches = short.len() - short_count;
    }
    pass.stats.code_bytes = crate::support::telemetry::counter(pass.assembler.code.len());
    pass.stats.fixups = crate::support::telemetry::counter(pass.assembler.fixups.len());
    let stats = pass.stats;
    if let Some(frame) = frame {
        *frame = pass.frame;
    }
    image.append(pass.assembler)?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{InstructionKind as I, Program},
        vocabulary::Type,
    };
    #[test]
    fn wide_zero_words_use_flag_preserving_immediate_moves() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Ptr)?;
        let slot = builder.slot("storage", Type::I128)?;
        builder.block("entry")?;
        let zero = builder.integer(0)?;
        builder.effect(I::Store(
            crate::lowir::model::Facts::default(),
            Type::I128,
            zero,
            slot,
        ))?;
        let address = builder.value(I::Addr(slot), "address")?;
        builder.effect(I::Return(Type::Ptr, Some(address)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let mir = super::super::value_selection::select(&program, symbol, function, 0)?;
        let mut image = Image::new(program.symbols().len());
        emit(&mir, &mut image, None)?;
        assert_eq!(
            image
                .text
                .windows(6)
                .filter(|bytes| *bytes == [0x41, 0xbb, 0, 0, 0, 0])
                .count(),
            2
        );
        Ok(())
    }
    #[test]
    fn conditional_fallthrough_inversion_and_shared_epilogue_are_relaxed() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "f", &[("condition", Type::I64)], Type::I64)?;
        builder.block("entry")?;
        let condition = builder.parameter(0)?;
        let yes = builder.block_id("yes")?;
        let no = builder.block_id("no")?;
        builder.effect(I::Branch(condition, yes, no))?;
        builder.block("yes")?;
        let one = builder.integer(1)?;
        builder.effect(I::Return(Type::I64, Some(one)))?;
        builder.block("no")?;
        let two = builder.integer(2)?;
        builder.effect(I::Return(Type::I64, Some(two)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let mir = super::super::value_selection::select(&program, symbol, function, 0)?;
        assert_eq!(mir.blocks.len(), 3);
        let mut image = Image::new(program.symbols().len());
        let stats = emit(&mir, &mut image, None)?;
        assert_eq!(stats.returns, 2);
        assert_eq!(stats.short_branches, 2);
        assert_eq!(stats.long_branches, 0);
        Ok(())
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    #[test]
    fn optimized_returns_emit_their_own_epilogue_and_record_each_teardown() {
        let program=crate::lowir::test_program(b"function @f(%c : i64) -> i64 { block ^entry: branch %c, ^yes, ^no block ^yes: return i64 1 block ^no: return i64 2 }").unwrap();
        let symbol = program.publications()[0];
        let mut function = super::super::value_selection::select_for_object(
            &program,
            symbol,
            program.function(program.symbol(symbol)).unwrap(),
            0,
            super::super::object_compile::ObjectModel::Host,
        )
        .unwrap();
        function.optimized = true;
        function.frame_policy.omit_pointer = true;
        let mut image = Image::new(program.symbols().len());
        let mut frame = super::super::host_unwind::Frame::default();
        emit_with_frame(&function, &mut image, None, Some(&mut frame)).unwrap();
        assert_eq!(frame.returns.len(), 1);
        assert!(frame.frameless);
        assert_eq!(
            image.text[usize::try_from(frame.ret_end - 1).unwrap()],
            0xc3
        );
        assert_eq!(
            image.text[usize::try_from(frame.returns[0].returned - 1).unwrap()],
            0xc3
        );
        assert!(frame.encode(false).unwrap().is_empty());
    }
}
