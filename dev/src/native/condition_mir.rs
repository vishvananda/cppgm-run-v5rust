//! Scalar flag operations and exact direct x86 encoding. See NOTICE.
use super::{
    condition::Condition,
    mir::{Instruction, Op, low_word},
    scalar_mir::{Operand, ScalarOp, bytes},
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{Error, model::Program, vocabulary::Type};
use std::io::Write;
#[derive(Clone, Copy)]
pub(super) enum FlagOp {
    Compare(Type, Operand, Operand),
    CompareFloatLiteral(crate::lowir::literals::FloatKind, Operand, u128),
    Test(Type, R),
    MaskTest(Type, Operand, Operand),
    BitTest(Type, R, Operand),
    Set(Condition, R),
    Move(Condition, R, Operand),
}
impl FlagOp {
    pub(super) fn self_test(self) -> Option<(Type, R)> {
        match self {
            Self::Test(ty, r) => Some((ty, r)),
            Self::MaskTest(ty, Operand::Register(a), Operand::Register(b)) if a == b => {
                Some((ty, a))
            }
            _ => None,
        }
    }
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::CompareFloatLiteral(kind, left, bits) => {
                write!(out, "cmp.{} ", kind.ty())?;
                left.write(program, out)?;
                write!(out, ", ")?;
                let mut floating = crate::types::floating::FloatingScanner::default();
                out.write_all(
                    crate::lowir::literals::mir_text(&mut floating, kind, bits)
                        .map_err(|_| std::io::Error::other("floating compare literal"))?
                        .as_bytes(),
                )
            }
            Self::Compare(ty, a, b) => {
                write!(out, "cmp.{} ", super::scalar_mir::Kind(ty))?;
                a.write(program, out)?;
                write!(out, ", ")?;
                b.write(program, out)
            }
            Self::Test(ty, r) => write!(out, "test.{ty} {}, {}", r.spelling(), r.spelling()),
            Self::MaskTest(ty, a, b) => {
                write!(out, "test.{ty} ")?;
                a.write(program, out)?;
                write!(out, ", ")?;
                b.write(program, out)
            }
            Self::BitTest(ty, r, b) => {
                write!(out, "bt.{ty} {}, ", r.spelling())?;
                b.write(program, out)
            }
            Self::Set(cc, r) => write!(out, "set{} {}", cc.spelling(), r.spelling()),
            Self::Move(cc, r, source) => {
                write!(out, "cmov{} {}, ", cc.spelling(), r.spelling())?;
                source.write(program, out)
            }
        }
    }
    pub(super) fn rebase(self, begin: i32, size: u32) -> Self {
        match self {
            Self::CompareFloatLiteral(kind, left, bits) => {
                Self::CompareFloatLiteral(kind, left.rebase(begin, size), bits)
            }
            Self::Compare(t, a, b) => {
                Self::Compare(t, a.rebase(begin, size), b.rebase(begin, size))
            }
            Self::Move(c, r, s) => Self::Move(c, r, s.rebase(begin, size)),
            Self::MaskTest(ty, a, b) => {
                Self::MaskTest(ty, a.rebase(begin, size), b.rebase(begin, size))
            }
            Self::BitTest(ty, r, b) => Self::BitTest(ty, r, b.rebase(begin, size)),
            other => other,
        }
    }
    pub(super) fn emit(self, a: &mut Assembler) -> Result<(), Error> {
        match self {
            // The frozen scalar CMP encoder requires a register, memory or
            // integer immediate. A floating switch case remains a typed FImm
            // in the MIR view and fails only when encoding is demanded.
            Self::CompareFloatLiteral(..) => Err(Error::Construction(
                "operand is not a register or memory location".into(),
            )),
            Self::Set(cc, r) => a.op(
                0,
                false,
                0x0f90 + u32::from(cc.code()),
                2,
                0,
                Rm::Reg(r),
                true,
                0,
            ),
            Self::Move(cc, r, source) => a.op(
                0,
                true,
                0x0f40 + u32::from(cc.code()),
                2,
                r.code(),
                source.rm()?,
                false,
                0,
            ),
            Self::Test(ty, r) => a.op(
                if bytes(ty) == 2 { 0x66 } else { 0 },
                bytes(ty) == 8,
                if bytes(ty) == 1 { 0x84 } else { 0x85 },
                1,
                r.code(),
                Rm::Reg(r),
                bytes(ty) == 1,
                0,
            ),
            Self::Compare(ty, left, right) => compare(a, ty, left, right),
            Self::MaskTest(ty, left, right) => mask_test(a, ty, left, right),
            Self::BitTest(ty, r, b) => {
                let width = bytes(ty);
                let prefix = if width == 2 { 0x66 } else { 0 };
                if let Operand::Immediate(bits) = b {
                    a.op(prefix, width == 8, 0x0fba, 2, 4, Rm::Reg(r), false, 1)?;
                    a.imm(
                        i64::try_from(bits & (u128::from(width) * 8 - 1))
                            .map_err(|_| Error::Limit)?,
                        1,
                    );
                    Ok(())
                } else {
                    a.op(
                        prefix,
                        width == 8,
                        0x0fa3,
                        2,
                        b.register().ok_or(Error::Limit)?.code(),
                        Rm::Reg(r),
                        false,
                        0,
                    )
                }
            }
        }
    }
    pub(super) fn operands(self) -> [Option<Operand>; 2] {
        match self {
            Self::CompareFloatLiteral(_, left, bits) => {
                [Some(left), Some(Operand::Immediate(bits))]
            }
            Self::Compare(_, a, b) | Self::MaskTest(_, a, b) => [Some(a), Some(b)],
            Self::BitTest(_, r, b) => [Some(Operand::Register(r)), Some(b)],
            Self::Test(_, r) => [Some(Operand::Register(r)), Some(Operand::Register(r))],
            Self::Set(_, r) => [Some(Operand::Register(r)), None],
            Self::Move(_, r, s) => [Some(Operand::Register(r)), Some(s)],
        }
    }
}
fn mask_test(a: &mut Assembler, ty: Type, left: Operand, right: Operand) -> Result<(), Error> {
    let width = bytes(ty);
    let prefix = if width == 2 { 0x66 } else { 0 };
    if let Some(reg) = right.register() {
        return a.op(
            prefix,
            width == 8,
            if width == 1 { 0x84 } else { 0x85 },
            1,
            reg.code(),
            left.rm()?,
            width == 1,
            0,
        );
    }
    let Operand::Immediate(bits) = right else {
        return Err(Error::Limit);
    };
    let word = low_word(bits)?;
    if width < 8 || i32::try_from(word).is_ok() {
        let count = if width == 1 {
            1
        } else if width == 2 {
            2
        } else {
            4
        };
        a.op(
            prefix,
            width == 8,
            if width == 1 { 0xf6 } else { 0xf7 },
            1,
            0,
            left.rm()?,
            width == 1,
            count,
        )?;
        a.imm(word, count);
        return Ok(());
    }
    let reg = if left.mask() & (1 << R::R11.code()) == 0 {
        R::R11
    } else {
        R::R10
    };
    a.mov_ri(reg, word)?;
    a.op(prefix, true, 0x85, 1, reg.code(), left.rm()?, false, 0)
}
fn compare(a: &mut Assembler, ty: Type, left: Operand, right: Operand) -> Result<(), Error> {
    let width = bytes(ty);
    let mut source = right;
    if let Operand::Immediate(bits) = right {
        let value = low_word(bits)?;
        if value == 0
            && let Some(r) = left.register()
        {
            return FlagOp::Test(ty, r).emit(a);
        }
        if width < 8 || i32::try_from(value).is_ok() {
            return a.alu_ri(7, left.rm()?, value, width);
        }
        let scratch = if (left.mask() | right.mask()) & (1 << R::R11.code()) == 0 {
            R::R11
        } else {
            R::R10
        };
        a.mov_ri(scratch, value)?;
        source = Operand::Register(scratch);
    }
    if let Some(r) = left.register() {
        return a.alu_rr(7, r, source.rm()?, width);
    }
    let reg = source
        .register()
        .ok_or_else(|| Error::Construction("memory compare requires a register source".into()))?;
    a.op(
        if width == 2 { 0x66 } else { 0 },
        width == 8,
        if width == 1 { 0x38 } else { 0x39 },
        1,
        reg.code(),
        left.rm()?,
        width == 1,
        0,
    )
}
pub(super) fn flags_after(instructions: &[Instruction]) -> Vec<bool> {
    let mut result = vec![false; instructions.len()];
    let mut next = false;
    for (i, inst) in instructions.iter().enumerate().rev() {
        result[i] = next;
        next = match inst.op {
            Op::Nop
            | Op::Move(..)
            | Op::SymbolMove(..)
            | Op::Scalar(ScalarOp::Move(..) | ScalarOp::Load(..) | ScalarOp::Lea(..)) => next,
            Op::Branch(..)
            | Op::Scalar(
                ScalarOp::SetEqual(..) | ScalarOp::Condition(FlagOp::Set(..) | FlagOp::Move(..)),
            ) => true,
            _ => false,
        };
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_moves_preserve_flags_only_until_the_next_flag_consumer() -> Result<(), Error> {
        let ops = [
            FlagOp::Compare(Type::I64, Operand::Register(R::RDI), Operand::Immediate(7)),
            FlagOp::Move(Condition::Equal, R::RAX, Operand::Register(R::R11)),
        ];
        let instructions = vec![
            Instruction {
                volatile: false,
                op: Op::Scalar(ScalarOp::Condition(ops[0])),
                location: None,
            },
            Instruction {
                volatile: false,
                op: Op::Move(R::RAX, 0),
                location: None,
            },
            Instruction {
                volatile: false,
                op: Op::Scalar(ScalarOp::Condition(ops[1])),
                location: None,
            },
            Instruction {
                volatile: false,
                op: Op::Move(R::R8, 0),
                location: None,
            },
            Instruction {
                volatile: false,
                op: Op::Return(Some(R::RAX)),
                location: None,
            },
        ];
        let flags = flags_after(&instructions);
        assert_eq!(flags, [true, true, false, false, false]);
        let mut a = Assembler::default();
        a.mov_ri_flags(R::RAX, 0, flags[1])?;
        a.mov_ri_flags(R::R8, 0, flags[3])?;
        assert_eq!(a.code, [0xb8, 0, 0, 0, 0, 0x45, 0x33, 0xc0]);
        Ok(())
    }
}
