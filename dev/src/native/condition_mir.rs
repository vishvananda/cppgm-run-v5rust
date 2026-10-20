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
    Test(Type, R),
    Set(Condition, R),
    Move(Condition, R, Operand),
}
impl FlagOp {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::Compare(ty, a, b) => {
                write!(out, "cmp.{ty} ")?;
                a.write(program, out)?;
                write!(out, ", ")?;
                b.write(program, out)
            }
            Self::Test(ty, r) => write!(out, "test.{ty} {}, {}", r.spelling(), r.spelling()),
            Self::Set(cc, r) => write!(out, "set{} {}", cc.spelling(), r.spelling()),
            Self::Move(cc, r, source) => {
                write!(out, "cmov{} {}, ", cc.spelling(), r.spelling())?;
                source.write(program, out)
            }
        }
    }
    pub(super) fn rebase(self, begin: i32, size: u32) -> Self {
        match self {
            Self::Compare(t, a, b) => {
                Self::Compare(t, a.rebase(begin, size), b.rebase(begin, size))
            }
            Self::Move(c, r, s) => Self::Move(c, r, s.rebase(begin, size)),
            other => other,
        }
    }
    pub(super) fn emit(self, a: &mut Assembler) -> Result<(), Error> {
        match self {
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
        }
    }
    pub(super) fn operands(self) -> [Option<Operand>; 2] {
        match self {
            Self::Compare(_, a, b) => [Some(a), Some(b)],
            Self::Test(_, r) => [Some(Operand::Register(r)), Some(Operand::Register(r))],
            Self::Set(_, r) => [Some(Operand::Register(r)), None],
            Self::Move(_, r, s) => [Some(Operand::Register(r)), Some(s)],
        }
    }
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
            Op::Move(..)
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
                op: Op::Scalar(ScalarOp::Condition(ops[0])),
                location: None,
            },
            Instruction {
                op: Op::Move(R::RAX, 0),
                location: None,
            },
            Instruction {
                op: Op::Scalar(ScalarOp::Condition(ops[1])),
                location: None,
            },
            Instruction {
                op: Op::Move(R::R8, 0),
                location: None,
            },
            Instruction {
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
