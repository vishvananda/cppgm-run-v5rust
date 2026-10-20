//! Per-function native facts shared by MIR serialization and encoding. See NOTICE.
use super::{
    abi::Layout,
    global_data,
    image::{Image, ImageSymbol, Section},
    x86::{Assembler, Register as R, Rm},
};
use crate::{
    lowir::{
        Error,
        literals::{self, FloatKind},
        model::{DebugLocation, NameId, Program, SignatureId, SymbolId},
        vocabulary::Type,
    },
    types::floating::FloatingScanner,
};
use std::io::Write;
pub(super) enum Op {
    Move(R, u128),
    SymbolMove(R, SymbolId),
    FloatMove(R, FloatKind, u128),
    FloatReturn(u128),
    Return(Option<R>),
    Trap,
    Scalar(super::scalar_mir::ScalarOp),
}
pub(super) struct Instruction {
    pub(super) op: Op,
    pub(super) location: Option<DebugLocation>,
}
pub(super) struct Binding {
    pub(super) name: NameId,
    pub(super) ty: Type,
    pub(super) offset: i32,
    pub(super) parameter: bool,
}
pub(super) struct Function {
    symbol: SymbolId,
    signature: SignatureId,
    block: NameId,
    pub(super) layout: Layout,
    pub(super) instructions: Vec<Instruction>,
    pub(super) scratch_bytes: u32,
    pub(super) stack_size: u32,
    pub(super) preserve: u32,
    pub(super) bindings: Vec<Binding>,
    pub(super) spills: usize,
    pub(super) homes: usize,
    pub(super) lowir_instructions: usize,
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
            block,
            layout,
            instructions: Vec::new(),
            scratch_bytes: 0,
            stack_size: 0,
            preserve: 0,
            bindings: Vec::new(),
            spills: 0,
            homes: 0,
            lowir_instructions: 0,
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
            _ => "rax",
        };
        writeln!(
            out,
            "    return {result} -> {reg}\n  frame\n    stack_size {}\n    scratch_bytes {}\n    frame_pointer keep\n    epilogues shared",
            self.stack_size, self.scratch_bytes
        )?;
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
                "    {} %{} -> [rbp{:+}] : {}",
                if binding.parameter {
                    "param-slot"
                } else {
                    "temp"
                },
                program.names.get(binding.name),
                binding.offset,
                binding.ty
            )?;
        }
        writeln!(out, "\n  block ^{}", program.names.get(self.block))?;
        let mut floating = FloatingScanner::default();
        for instruction in &self.instructions {
            match instruction.op {
                Op::Move(reg, bits) => {
                    let low = low_word(bits)
                        .map_err(|_| std::io::Error::other("invalid MIR immediate"))?;
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
                Op::FloatMove(reg, kind, bits) => write!(
                    out,
                    "    fmov.{} {}, {}",
                    kind.ty(),
                    reg.spelling(),
                    literals::mir_text(&mut floating, kind, bits)
                        .map_err(|_| std::io::Error::other("invalid floating MIR value"))?
                )?,
                Op::FloatReturn(bits) => write!(
                    out,
                    "    fret.f80 {}",
                    literals::mir_text(&mut floating, FloatKind::F80, bits)
                        .map_err(|_| std::io::Error::other("invalid floating MIR value"))?
                )?,
                Op::Return(reg) => {
                    write!(out, "    ret")?;
                    if let Some(reg) = reg {
                        write!(out, " {}", reg.spelling())?;
                    }
                }
                Op::Trap => write!(out, "    trap")?,
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
        }
        Ok(())
    }
    pub(super) fn emit(&self, image: &mut Image) -> Result<Emission, Error> {
        let base = image.align_text()?;
        image.define(ImageSymbol::Program(self.symbol), Section::Text, base);
        let mut a = Assembler::default();
        a.push(R::RBP);
        a.mov_rr(R::RBP, R::RSP)?;
        let saved = self.preserve.count_ones() * 8;
        if self.stack_size != saved {
            a.alu_ri(5, Rm::Reg(R::RSP), i64::from(self.stack_size - saved), 8)?;
        }
        for r in super::placement::SAVED.into_iter().rev() {
            if self.preserve & (1 << r.code()) != 0 {
                a.push(r);
            }
        }
        let mut returns = 0;
        let mut carries = super::frame_carries::Carries::new(&self.bindings);
        let mut carried_reloads = 0;
        for (index, instruction) in self.instructions.iter().enumerate() {
            let replacement = carries.replacement(&instruction.op);
            let pending = carries.track(&self.instructions, index);
            if let Some((dst, source)) = replacement {
                a.mov_rr(dst, source)?;
                carried_reloads += 1;
                continue;
            }
            match instruction.op {
                Op::Move(reg, bits) => a.mov_ri(reg, low_word(bits)?)?,
                Op::SymbolMove(reg, symbol) => {
                    a.lea(reg, Rm::rip(ImageSymbol::Program(symbol), 0))?;
                }
                Op::FloatMove(reg, kind, bits) => {
                    if bits & u128::from(u64::MAX) == 0 {
                        a.sse(0, 0x57, reg, Rm::Reg(reg))?;
                    } else {
                        let source = constant(image, kind, bits)?;
                        a.sse(
                            if kind == FloatKind::F32 { 0xf3 } else { 0xf2 },
                            0x10,
                            reg,
                            source,
                        )?;
                    }
                }
                Op::FloatReturn(bits) => {
                    let source = constant(image, FloatKind::F80, bits)?;
                    a.op(0, false, 0xdb, 1, 5, source, false, 0)?;
                    returns += 1;
                }
                Op::Return(reg) => {
                    if let Some(reg) = reg {
                        return_register(&mut a, reg)?;
                    }
                    returns += 1;
                }
                Op::Scalar(op) => op.emit(&mut a)?,
                Op::Trap => {
                    a.byte(0x0f);
                    a.byte(0x0b);
                }
            }
            if let Some(source) = pending {
                a.mov_rr(R::R11, source)?;
            }
        }
        for r in super::placement::SAVED {
            if self.preserve & (1 << r.code()) != 0 {
                a.pop(r);
            }
        }
        a.byte(0xc9);
        a.ret();
        let emission = Emission {
            carried_reloads,
            returns,
            fixups: a.fixups.len(),
            code_bytes: a.code.len(),
        };
        image.append(a)?;
        Ok(emission)
    }
}
pub(super) fn low_word(bits: u128) -> Result<i64, Error> {
    Ok(i64::from_le_bytes(
        u64::try_from(bits & u128::from(u64::MAX))
            .map_err(|_| Error::Limit)?
            .to_le_bytes(),
    ))
}
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
#[derive(Clone, Copy)]
pub(super) struct Emission {
    pub(super) returns: usize,
    pub(super) fixups: usize,
    pub(super) code_bytes: usize,
    pub(super) carried_reloads: usize,
}
fn constant(image: &mut Image, kind: FloatKind, bits: u128) -> Result<Rm, Error> {
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
