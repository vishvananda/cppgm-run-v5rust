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
        let mut chains = vec![Chain::Unknown; count];
        let mut walk = Vec::new();
        let jump = |block: usize| {
            let range = &function.blocks[block].range;
            if block != 0
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
}
struct Pass<'a> {
    function: &'a Function,
    assembler: Assembler,
    labels: Vec<Label>,
    epilogue: Label,
    branches: Vec<Branch>,
    short: &'a [bool],
    flags: Vec<bool>,
    carries: Carries,
    stats: Emission,
}
impl<'a> Pass<'a> {
    fn new(function: &'a Function, order: &Order, short: &'a [bool]) -> Result<Self, Error> {
        let mut assembler = Assembler::default();
        let mut labels = Vec::with_capacity(function.blocks.len());
        for _ in &function.blocks {
            labels.push(assembler.label()?);
        }
        let epilogue = assembler.label()?;
        for (block, &target) in order.targets.iter().enumerate() {
            labels[block] = labels[target];
        }
        Ok(Self {
            function,
            assembler,
            labels,
            epilogue,
            branches: Vec::new(),
            short,
            flags: super::condition_mir::flags_after(&function.instructions),
            carries: Carries::new(&function.bindings),
            stats: Emission::default(),
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
        });
        self.assembler.branch(condition, label, short);
    }
    fn run(&mut self, image: &mut Image, order: &Order) -> Result<(), Error> {
        let f = self.function;
        let a = &mut self.assembler;
        a.push(R::RBP);
        a.mov_rr(R::RBP, R::RSP)?;
        let saved = f.preserve.count_ones() * 8;
        if f.stack_size != saved {
            a.alu_ri(5, Rm::Reg(R::RSP), i64::from(f.stack_size - saved), 8)?;
        }
        for reg in super::placement::SAVED.into_iter().rev() {
            if f.preserve & (1 << reg.code()) != 0 {
                a.push(reg);
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
        for block in 0..f.blocks.len() {
            if order.targets[block] != block {
                continue;
            }
            self.assembler.bind(self.labels[block]);
            self.carries.clear();
            let next = order.next[block + 1];
            let range = if f.blocks.len() == 1 {
                0..f.instructions.len()
            } else {
                f.blocks[block].range.clone()
            };
            for index in range.clone() {
                let op = f.instructions[index].op;
                let replacement = self.carries.replacement(&op);
                let pending = self.carries.track(&f.instructions[..range.end], index);
                if let Some((to, from)) = replacement {
                    self.assembler.mov_rr(to, from)?;
                    self.stats.carried_reloads += 1;
                    continue;
                }
                if let Op::Branch(cond, target) = op
                    && index + 2 == range.end
                    && next < f.blocks.len()
                    && let Op::Jump(other) = f.instructions[index + 1].op
                    && self.labels[target.index()] == self.labels[next]
                {
                    self.branch(Some(cond.invert().code()), self.labels[other.index()]);
                    break;
                }
                if matches!(op,Op::Jump(target) if next<f.blocks.len()&&self.labels[target.index()]==self.labels[next])
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
        for reg in super::placement::SAVED {
            if f.preserve & (1 << reg.code()) != 0 {
                self.assembler.pop(reg);
            }
        }
        self.assembler.byte(0xc9);
        self.assembler.ret();
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
            Op::Jump(target) => self.branch(None, self.labels[target.index()]),
            Op::Branch(cond, target) => self.branch(Some(cond.code()), self.labels[target.index()]),
            Op::Move(reg, bits) => {
                self.assembler
                    .mov_ri_flags(reg, low_word(bits)?, self.flags[index])?;
            }
            Op::SymbolMove(reg, symbol) => self
                .assembler
                .lea(reg, Rm::rip(ImageSymbol::Program(symbol), 0))?,
            Op::FloatMove(reg, kind, bits) => {
                if bits & u128::from(u64::MAX) == 0 {
                    self.assembler.sse(0, 0x57, reg, Rm::Reg(reg))?;
                } else {
                    let source = constant(image, kind, bits)?;
                    self.assembler.sse(
                        if kind == FloatKind::F32 { 0xf3 } else { 0xf2 },
                        0x10,
                        reg,
                        source,
                    )?;
                }
            }
            Op::FloatReturn(bits) => {
                let source = constant(image, FloatKind::F80, bits)?;
                self.assembler.op(0, false, 0xdb, 1, 5, source, false, 0)?;
                self.stats.returns += 1;
                if !last {
                    self.branch(None, self.epilogue);
                }
            }
            Op::Return(reg) => {
                if let Some(reg) = reg {
                    return_register(&mut self.assembler, reg)?;
                }
                self.stats.returns += 1;
                if !last {
                    self.branch(None, self.epilogue);
                }
            }
            Op::Call(site) => site.emit(&mut self.assembler, f.aligned_disp, f.aligned_bytes)?,
            Op::DynamicCopy => {
                self.assembler.mov_rr(R::RCX, R::RDX)?;
                self.assembler.byte(0xf3);
                self.assembler.byte(0xa4);
            }
            Op::Bulk(op) => op
                .rebase(f.aligned_disp, f.aligned_bytes)
                .emit(&mut self.assembler)?,
            Op::FloatCopy(op) => op
                .rebase(f.aligned_disp, f.aligned_bytes)
                .emit(&mut self.assembler, image)?,
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
            Op::Scalar(op) => op
                .rebase(f.aligned_disp, f.aligned_bytes)
                .emit(&mut self.assembler, self.flags[index])?,
            Op::Trap => {
                self.assembler.byte(0x0f);
                self.assembler.byte(0x0b);
            }
        }
        Ok(())
    }
}
pub(super) fn emit(function: &Function, image: &mut Image) -> Result<Emission, Error> {
    let base = image.align_text()?;
    image.define(ImageSymbol::Program(function.symbol), Section::Text, base);
    let order = Order::new(function);
    let mut pass = Pass::new(function, &order, &[])?;
    pass.run(image, &order)?;
    let mut short = Vec::with_capacity(pass.branches.len());
    for branch in &pass.branches {
        let target = pass
            .assembler
            .label_position(branch.label)
            .ok_or(Error::Limit)?;
        let displacement = i64::try_from(target).map_err(|_| Error::Limit)?
            - i64::try_from(branch.offset + 2).map_err(|_| Error::Limit)?;
        short.push(i8::try_from(displacement).is_ok());
    }
    if short.iter().any(|&short| short) {
        pass = Pass::new(function, &order, &short)?;
        pass.run(image, &order)?;
    }
    pass.assembler.resolve()?;
    pass.stats.short_branches = short.iter().filter(|&&value| value).count();
    pass.stats.long_branches = short.len() - pass.stats.short_branches;
    pass.stats.code_bytes = pass.assembler.code.len();
    pass.stats.fixups = pass.assembler.fixups.len();
    let stats = pass.stats;
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
        emit(&mir, &mut image)?;
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
        let stats = emit(&mir, &mut image)?;
        assert_eq!(stats.returns, 2);
        assert_eq!(stats.short_branches, 2);
        assert_eq!(stats.long_branches, 0);
        Ok(())
    }
}
