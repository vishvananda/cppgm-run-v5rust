//! Split-word i128 machine operations and exact integer encoding. See NOTICE.
use super::{
    mir::low_word,
    scalar_mir::Operand,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{Error, model::Program, vocabulary::Binary};
use std::io::Write;

#[derive(Clone, Copy)]
pub(super) enum Operation {
    Binary(Binary),
    Negate,
    Complement,
}
impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::Binary(Binary::Shr) => "sar",
            Self::Binary(Binary::UShr) => "shr",
            Self::Binary(op) => op.spelling(),
            Self::Negate => "neg",
            Self::Complement => "not",
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Arithmetic {
    pub(super) operation: Operation,
    pub(super) destination: Operand,
    pub(super) left: Operand,
    pub(super) right: Option<Operand>,
}
impl Arithmetic {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(out, "    {}.i128 ", self.operation.name())?;
        self.destination.write(program, out)?;
        write!(out, ", ")?;
        self.left.write(program, out)?;
        if let Some(right) = self.right {
            write!(out, ", ")?;
            right.write(program, out)?;
        }
        Ok(())
    }
    pub(super) fn rebase(self, begin: i32, bytes: u32) -> Self {
        Self {
            destination: self.destination.rebase(begin, bytes),
            left: self.left.rebase(begin, bytes),
            right: self.right.map(|r| r.rebase(begin, bytes)),
            ..self
        }
    }
    pub(super) fn emit(
        self,
        a: &mut Assembler,
        runtime: Option<super::runtime::Symbols>,
    ) -> Result<(), Error> {
        let to = self.destination.rm()?;
        let left = self.left;
        match self.operation {
            Operation::Binary(
                op @ (Binary::Add | Binary::Sub | Binary::And | Binary::Or | Binary::Xor),
            ) => {
                let (low, high) = match op {
                    Binary::Add => (0, 2),
                    Binary::Sub => (5, 3),
                    Binary::And => (4, 4),
                    Binary::Or => (1, 1),
                    _ => (6, 6),
                };
                let right = self.right.ok_or(Error::Limit)?;
                half(a, R::R11, left, false)?;
                alu(a, low, R::R11, right, false)?;
                a.store(to, R::R11, 8)?;
                half(a, R::R11, left, true)?;
                alu(a, high, R::R11, right, true)?;
                a.store(to.displaced(8), R::R11, 8)
            }
            Operation::Binary(Binary::Mul) => {
                multiply(a, to, left, self.right.ok_or(Error::Limit)?)
            }
            Operation::Binary(op @ (Binary::Shl | Binary::Shr | Binary::UShr)) => {
                shift(a, to, left, self.right.ok_or(Error::Limit)?, op)
            }
            Operation::Binary(op @ (Binary::Div | Binary::UDiv | Binary::Mod | Binary::UMod)) => {
                let right = self.right.ok_or(Error::Limit)?;
                half(a, R::RDI, left, false)?;
                half(a, R::RSI, left, true)?;
                half(a, R::RDX, right, false)?;
                half(a, R::RCX, right, true)?;
                let routine = match op {
                    Binary::Div => 0,
                    Binary::UDiv => 1,
                    Binary::Mod => 2,
                    _ => 3,
                };
                a.call_symbol(runtime.ok_or(Error::Limit)?.wide_division[routine])?;
                a.store(to, R::RAX, 8)?;
                a.store(to.displaced(8), R::RDX, 8)
            }
            Operation::Negate | Operation::Complement => {
                half(a, R::R10, left, false)?;
                half(a, R::R11, left, true)?;
                if matches!(self.operation, Operation::Negate) {
                    a.op(0, true, 0xf7, 1, 3, Rm::Reg(R::R10), false, 0)?;
                    a.alu_ri(2, Rm::Reg(R::R11), 0, 8)?;
                    a.op(0, true, 0xf7, 1, 3, Rm::Reg(R::R11), false, 0)?;
                } else {
                    for r in [R::R10, R::R11] {
                        a.op(0, true, 0xf7, 1, 2, Rm::Reg(r), false, 0)?;
                    }
                }
                a.store(to, R::R10, 8)?;
                a.store(to.displaced(8), R::R11, 8)
            }
        }
    }
}
pub(super) fn half(a: &mut Assembler, dst: R, source: Operand, high: bool) -> Result<(), Error> {
    match source {
        Operand::Immediate(bits) => {
            a.mov_ri_flags(dst, low_word(if high { bits >> 64 } else { bits })?, true)
        }
        Operand::Register(r) => a.mov_rr(dst, r),
        _ => a.load(
            dst,
            source.displaced(if high { 8 } else { 0 }).rm()?,
            8,
            false,
        ),
    }
}
pub(super) fn alu(
    a: &mut Assembler,
    digit: u8,
    dst: R,
    source: Operand,
    high: bool,
) -> Result<(), Error> {
    if let Operand::Immediate(bits) = source {
        let word = low_word(if high { bits >> 64 } else { bits })?;
        if i32::try_from(word).is_ok() {
            return a.alu_ri(digit, Rm::Reg(dst), word, 8);
        }
        let other = if dst == R::R10 { R::R11 } else { R::R10 };
        a.mov_ri_flags(other, word, true)?;
        a.alu_rr(digit, dst, Rm::Reg(other), 8)
    } else {
        a.alu_rr(
            digit,
            dst,
            source.rm()?.displaced(if high { 8 } else { 0 }),
            8,
        )
    }
}
fn multiply(a: &mut Assembler, to: Rm, left: Operand, right: Operand) -> Result<(), Error> {
    half(a, R::RAX, left, false)?;
    let low = if matches!(right, Operand::Immediate(_)) {
        half(a, R::R11, right, false)?;
        Rm::Reg(R::R11)
    } else {
        right.rm()?
    };
    a.op(0, true, 0xf7, 1, 4, low, false, 0)?;
    for (left_high, right_high) in [(false, true), (true, false)] {
        half(a, R::R11, left, left_high)?;
        let from = if matches!(right, Operand::Immediate(_)) {
            half(a, R::R10, right, right_high)?;
            Rm::Reg(R::R10)
        } else {
            right.rm()?.displaced(if right_high { 8 } else { 0 })
        };
        a.op(0, true, 0x0faf, 2, R::R11.code(), from, false, 0)?;
        a.alu_rr(0, R::RDX, Rm::Reg(R::R11), 8)?;
    }
    a.store(to, R::RAX, 8)?;
    a.store(to.displaced(8), R::RDX, 8)
}
fn zero(a: &mut Assembler, r: R) -> Result<(), Error> {
    a.op(0, false, 0x33, 1, r.code(), Rm::Reg(r), false, 0)
}
fn shift_imm(a: &mut Assembler, digit: u8, r: R, count: i64) -> Result<(), Error> {
    a.op(
        0,
        true,
        if count == 1 { 0xd1 } else { 0xc1 },
        1,
        digit,
        Rm::Reg(r),
        false,
        u8::from(count != 1),
    )?;
    if count != 1 {
        a.imm(count, 1);
    }
    Ok(())
}
fn shift(
    a: &mut Assembler,
    to: Rm,
    source: Operand,
    count: Operand,
    op: Binary,
) -> Result<(), Error> {
    half(a, R::R10, source, false)?;
    half(a, R::R11, source, true)?;
    let left = op == Binary::Shl;
    let digit = if left {
        4
    } else if op == Binary::Shr {
        7
    } else {
        5
    };
    let upper = |a: &mut Assembler| {
        if left {
            a.mov_rr(R::R11, R::R10)?;
            zero(a, R::R10)
        } else {
            a.mov_rr(R::R10, R::R11)?;
            if op == Binary::Shr {
                shift_imm(a, 7, R::R11, 63)
            } else {
                zero(a, R::R11)
            }
        }
    };
    if let Operand::Immediate(bits) = count {
        let k = i64::try_from(bits & 127).map_err(|_| Error::Limit)?;
        if k >= 64 {
            if left {
                a.mov_rr(R::R11, R::R10)?;
                if k > 64 {
                    shift_imm(a, 4, R::R11, k - 64)?;
                }
                zero(a, R::R10)?;
            } else {
                a.mov_rr(R::R10, R::R11)?;
                if k > 64 {
                    shift_imm(a, digit, R::R10, k - 64)?;
                }
                if op == Binary::Shr {
                    shift_imm(a, 7, R::R11, 63)?;
                } else {
                    zero(a, R::R11)?;
                }
            }
        } else if k != 0 {
            let (dst, src, code) = if left {
                (R::R11, R::R10, 0x0fa4)
            } else {
                (R::R10, R::R11, 0x0fac)
            };
            a.op(0, true, code, 2, src.code(), Rm::Reg(dst), false, 1)?;
            a.imm(k, 1);
            shift_imm(a, digit, if left { R::R10 } else { R::R11 }, k)?;
        }
    } else {
        let done = a.label()?;
        let (dst, src, code) = if left {
            (R::R11, R::R10, 0x0fa5)
        } else {
            (R::R10, R::R11, 0x0fad)
        };
        a.op(0, true, code, 2, src.code(), Rm::Reg(dst), false, 0)?;
        a.shift_cl(digit, Rm::Reg(if left { R::R10 } else { R::R11 }), 8)?;
        a.op(0, false, 0xf6, 1, 0, Rm::Reg(R::RCX), true, 1)?;
        a.imm(64, 1);
        a.branch(Some(4), done, true);
        upper(a)?;
        a.bind(done);
    }
    a.store(to, R::R10, 8)?;
    a.store(to.displaced(8), R::R11, 8)
}
