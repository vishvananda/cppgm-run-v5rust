//! Per-function native facts shared by MIR serialization and encoding. See NOTICE.
use super::{
    abi::Layout,
    image::{Image, ImageSymbol},
    scalar_mir::Operand,
    x86::{Assembler, Register as R},
};
use crate::lowir::{
    Error,
    model::{DebugLocation, NameId, Program, SignatureId, SymbolId},
    vocabulary::Type,
};
use std::io::Write;
#[derive(Clone, Copy)]
pub(super) struct CallSite {
    pub(super) target: Operand,
    pub(super) arguments: u32,
    pub(super) stack: u32,
    pub(super) variadic: bool,
    pub(super) no_unwind: bool,
    pub(super) no_return: bool,
}
impl CallSite {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(
            out,
            "    call {}",
            if matches!(self.target, Operand::Symbol(_)) {
                ""
            } else {
                "*"
            }
        )?;
        self.target.write(program, out)?;
        write!(out, " [args=(")?;
        let mut first = true;
        for r in [
            R::RAX,
            R::RCX,
            R::RDX,
            R::RBX,
            R::RSP,
            R::RBP,
            R::RSI,
            R::RDI,
            R::R8,
            R::R9,
            R::R10,
            R::R11,
            R::R12,
            R::R13,
            R::R14,
            R::R15,
        ] {
            if self.arguments & (1 << r.code()) != 0 {
                if !first {
                    write!(out, ",")?;
                }
                write!(out, "{}", r.spelling())?;
                first = false;
            }
        }
        for index in 0..16 {
            if self.arguments & (1 << (16 + index)) != 0 {
                if !first {
                    write!(out, ",")?;
                }
                write!(out, "xmm{index}")?;
                first = false;
            }
        }
        write!(out, ")")?;
        if self.stack != 0 {
            write!(out, ", stack={}", self.stack.wrapping_add(15) & !15)?;
        }
        for (set, name) in [
            (self.variadic, "variadic"),
            (self.no_unwind, "unwind=no"),
            (self.no_return, "returns=noreturn"),
        ] {
            if set {
                write!(out, ", {name}")?;
            }
        }
        write!(out, "]")
    }
    pub(super) fn emit(self, a: &mut Assembler, begin: i32, size: u32) -> Result<(), Error> {
        if let Operand::Symbol(symbol) = self.target {
            a.call_symbol(ImageSymbol::Program(symbol))
        } else {
            a.call_rm(self.target.rebase(begin, size).rm()?)
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Op {
    Jump(BlockId),
    Branch(super::condition::Condition, BlockId),
    Move(R, u128),
    SymbolMove(R, SymbolId),
    Return(Option<R>),
    Trap,
    Call(CallSite),
    DynamicCopy,
    Atomic(super::atomic::Atomic),
    VaStart(super::variadic::Start),
    Bulk(super::bulk::Bulk),
    FloatCopy(super::float_transport::FloatCopy),
    Floating(super::floating_mir::FloatingOp),
    Convert(super::conversion::Conversion),
    Exception(super::exception::ExceptionOp),
    Wide(super::wide_mir::Arithmetic),
    WideCompare(super::wide_compare::Comparison),
    WideMove {
        destination: Operand,
        source: Operand,
    },
    WideExtend {
        signed: bool,
        destination: Operand,
        source: R,
    },
    Scalar(super::scalar_mir::ScalarOp),
}
pub(super) struct Instruction {
    pub(super) op: Op,
    pub(super) location: Option<DebugLocation>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BindingKind {
    Slot,
    Parameter,
    Temp,
}
pub(super) struct Binding {
    pub(super) name: Option<NameId>,
    pub(super) ty: Type,
    pub(super) offset: i32,
    pub(super) kind: BindingKind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct BlockId(pub(super) u32);
impl BlockId {
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy)]
pub(super) enum BlockName {
    Named(NameId),
    Edge(NameId, NameId),
}
pub(super) struct Block {
    pub(super) name: BlockName,
    pub(super) range: std::ops::Range<usize>,
}
pub(super) struct Function {
    pub(super) symbol: SymbolId,
    signature: SignatureId,
    pub(super) optimized: bool,
    pub(super) removed_instructions: usize,
    pub(super) variadic: bool,
    pub(super) blocks: Vec<Block>,
    pub(super) layout: Layout,
    pub(super) instructions: Vec<Instruction>,
    pub(super) scratch_bytes: u32,
    pub(super) scratch_offset: i32,
    pub(super) stack_size: u32,
    pub(super) aligned_base: u32,
    pub(super) aligned_disp: i32,
    pub(super) aligned_bytes: u32,
    pub(super) preserve: u32,
    pub(super) bindings: Vec<Binding>,
    pub(super) spills: usize,
    pub(super) homes: usize,
    pub(super) lazy_addresses: usize,
    pub(super) folded_loads: usize,
    pub(super) fused_compares: usize,
    pub(super) dynamic_copies: usize,
    pub(super) lowir_instructions: usize,
    pub(super) global_registers: usize,
    pub(super) liveness_steps: usize,
    pub(super) hull_fallbacks: usize,
    pub(super) edge_blocks: usize,
    pub(super) promoted_slots: usize,
    pub(super) dead_slots: usize,
}
impl Function {
    pub(super) fn new(
        symbol: SymbolId,
        signature: SignatureId,
        block: NameId,
        layout: Layout,
    ) -> Self {
        Self {
            symbol,
            signature,
            optimized: false,
            removed_instructions: 0,
            variadic: false,
            blocks: vec![Block {
                name: BlockName::Named(block),
                range: 0..0,
            }],
            layout,
            instructions: Vec::new(),
            scratch_bytes: 0,
            scratch_offset: 0,
            stack_size: 0,
            aligned_base: 0,
            aligned_disp: 0,
            aligned_bytes: 0,
            preserve: 0,
            bindings: Vec::new(),
            spills: 0,
            homes: 0,
            lazy_addresses: 0,
            folded_loads: 0,
            fused_compares: 0,
            dynamic_copies: 0,
            lowir_instructions: 0,
            global_registers: 0,
            liveness_steps: 0,
            hull_fallbacks: 0,
            edge_blocks: 0,
            promoted_slots: 0,
            dead_slots: 0,
        }
    }
    pub(super) fn write(&self, program: &Program, out: &mut impl Write) -> Result<(), Error> {
        self.write_inner(program, out).map_err(Error::Output)
    }
    fn write_inner(&self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        let signature = program.signature(self.signature);
        writeln!(
            out,
            "\nfunction @{}\n  abi",
            program.names.get(program.symbol(self.symbol).name)
        )?;
        for (parameter, argument) in signature.parameters.iter().zip(&self.layout.arguments) {
            let parts = self.layout.parts(argument);
            for part in parts {
                write!(out, "    param %{} -> ", program.names.get(parameter.name))?;
                if let Some(reg) = part.register {
                    write!(out, "{}", reg.spelling())?;
                } else {
                    write!(out, "[rbp+{}]", 16_i32.wrapping_add(part.stack))?;
                }
                if part.register.is_none() || parts.len() == 1 {
                    writeln!(out, " : {}", parameter.ty)?;
                } else {
                    writeln!(out, " : i64")?;
                }
            }
        }
        let result = signature.result;
        let reg = match result {
            Type::Void => "void",
            Type::F32 | Type::F64 => "xmm0",
            Type::F80 => "st0",
            Type::Object { bytes, .. } if bytes > 16 => "[rax]",
            _ => "rax",
        };
        writeln!(
            out,
            "    return {result} -> {reg}\n  frame\n    stack_size {}\n    scratch_bytes {}\n    frame_pointer keep\n    epilogues {}",
            self.stack_size,
            self.scratch_bytes,
            if self.optimized {
                "duplicated"
            } else {
                "shared"
            }
        )?;
        if self.aligned_base != 0 {
            writeln!(
                out,
                "    aligned_base {} r12 = rbp{}",
                self.aligned_base, self.aligned_disp
            )?;
        }
        if self.preserve != 0 {
            write!(out, "    preserve")?;
            for r in super::placement::SAVED {
                if self.preserve & (1 << r.code()) != 0 {
                    write!(out, " {}", r.spelling())?;
                }
            }
            writeln!(out)?;
        }
        for binding in &self.bindings {
            writeln!(
                out,
                "    {} {}{} -> [rbp{:+}] : {}",
                match binding.kind {
                    BindingKind::Slot => "slot",
                    BindingKind::Parameter => "param-slot",
                    BindingKind::Temp => "temp",
                },
                if binding.kind == BindingKind::Slot {
                    '$'
                } else {
                    '%'
                },
                binding
                    .name
                    .map_or("va-register-save", |name| program.names.get(name)),
                binding.offset,
                binding.ty
            )?;
        }
        for block in &self.blocks {
            write!(out, "\n  block ")?;
            Self::block_name(program, block.name, out)?;
            writeln!(out)?;
            let end = if self.blocks.len() == 1 {
                self.instructions.len()
            } else {
                block.range.end
            };
            for instruction in &self.instructions[block.range.start..end] {
                self.write_instruction(program, out, instruction)?;
            }
        }
        Ok(())
    }
    pub(super) fn begin_block(&mut self, name: BlockName) -> Result<BlockId, Error> {
        self.end_block();
        let id = BlockId(u32::try_from(self.blocks.len()).map_err(|_| Error::Limit)?);
        let at = self.instructions.len();
        self.blocks.push(Block {
            name,
            range: at..at,
        });
        Ok(id)
    }
    pub(super) fn end_block(&mut self) {
        if let Some(block) = self.blocks.last_mut() {
            block.range.end = self.instructions.len();
        }
    }
    fn block_name(program: &Program, name: BlockName, out: &mut impl Write) -> std::io::Result<()> {
        match name {
            BlockName::Named(n) => write!(out, "^{}", program.names.get(n)),
            BlockName::Edge(from, to) => write!(
                out,
                "^{}~{}",
                program.names.get(from),
                program.names.get(to)
            ),
        }
    }
    fn label(&self, program: &Program, id: BlockId, out: &mut impl Write) -> std::io::Result<()> {
        Self::block_name(program, self.blocks[id.index()].name, out)
    }
    fn write_instruction(
        &self,
        program: &Program,
        out: &mut impl Write,
        instruction: &Instruction,
    ) -> std::io::Result<()> {
        match instruction.op {
            Op::Jump(target) => {
                write!(out, "    jmp ")?;
                self.label(program, target, out)?;
            }
            Op::Branch(cond, target) => {
                write!(out, "    j{} ", cond.spelling())?;
                self.label(program, target, out)?;
            }
            Op::Move(reg, bits) => {
                let low =
                    low_word(bits).map_err(|_| std::io::Error::other("invalid MIR immediate"))?;
                write!(out, "    mov {}, ", reg.spelling())?;
                if bits >> 64 == if low < 0 { u128::from(u64::MAX) } else { 0 } {
                    write!(out, "{low}")?;
                } else {
                    write!(out, "0x{bits:032x}")?;
                }
            }
            Op::SymbolMove(reg, symbol) => write!(
                out,
                "    mov {}, @{}",
                reg.spelling(),
                program.names.get(program.symbol(symbol).name)
            )?,
            Op::Return(reg) => {
                write!(out, "    ret")?;
                if let Some(reg) = reg {
                    write!(out, " {}", reg.spelling())?;
                }
            }
            Op::Trap => write!(out, "    trap")?,
            Op::Call(site) => site.write(program, out)?,
            Op::VaStart(op) => op.write(program, out)?,
            Op::DynamicCopy => write!(out, "    copy_bytes_dynamic")?,
            Op::Bulk(op) => op.write(program, out)?,
            Op::FloatCopy(op) => op.write(program, out)?,
            Op::Floating(op) => op.write(program, out)?,
            Op::Convert(op) => op.write(program, out)?,
            Op::Exception(op) => {
                op.write(program, out, |target, out| self.label(program, target, out))?;
            }
            Op::Atomic(op) => op.write(program, out)?,
            Op::Wide(op) => op.write(program, out)?,
            Op::WideCompare(op) => {
                op.write(program, out, |target, out| self.label(program, target, out))?;
            }
            Op::WideMove {
                destination,
                source,
            } => {
                write!(out, "    mov.i128 ")?;
                destination.write(program, out)?;
                write!(out, ", ")?;
                source.write(program, out)?;
            }
            Op::WideExtend {
                signed,
                destination,
                source,
            } => {
                write!(out, "    {}.i128 ", if signed { "sext" } else { "zext" })?;
                destination.write(program, out)?;
                write!(out, ", {}", source.spelling())?;
            }
            Op::Scalar(op) => {
                write!(out, "    ")?;
                op.write(program, out)?;
            }
        }
        if let Some(location) = instruction.location {
            out.write_all(b" !dbg(")?;
            let file = program.names.bytes(location.file);
            let end = file
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(file.len());
            out.write_all(&file[..end])?;
            write!(out, ", {}, {})", location.line, location.column)?;
        }
        writeln!(out)?;
        Ok(())
    }
    pub(super) fn emit(
        &self,
        image: &mut Image,
        runtime: Option<super::runtime::Symbols>,
    ) -> Result<Emission, Error> {
        super::emission::emit(self, image, runtime)
    }
}

pub(super) fn low_word(bits: u128) -> Result<i64, Error> {
    Ok(i64::from_le_bytes(
        u64::try_from(bits & u128::from(u64::MAX))
            .map_err(|_| Error::Limit)?
            .to_le_bytes(),
    ))
}
#[derive(Clone, Copy, Default)]
pub(super) struct Emission {
    pub(super) returns: usize,
    pub(super) fixups: usize,
    pub(super) code_bytes: usize,
    pub(super) carried_reloads: usize,
    pub(super) short_branches: usize,
    pub(super) long_branches: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn call_view_keeps_raw_stack_demand_and_wraps_target_alignment() -> Result<(), Error> {
        let program = Program::default();
        for (raw, suffix) in [
            (0, ""),
            (8, ", stack=16"),
            (16, ", stack=16"),
            (u32::MAX - 7, ", stack=0"),
        ] {
            let site = CallSite {
                target: Operand::Register(R::RAX),
                arguments: 0,
                stack: raw,
                variadic: false,
                no_unwind: false,
                no_return: false,
            };
            let mut text = Vec::new();
            site.write(&program, &mut text).map_err(Error::Output)?;
            assert_eq!(text, format!("    call *rax [args=(){suffix}]").as_bytes());
        }
        Ok(())
    }
}
