//! Explicit integer MIR operands used by frame-free region planning. See NOTICE.
use super::{
    condition_mir::FlagOp,
    memory::Base,
    mir::Op,
    scalar_mir::{Operand, ScalarOp},
    x86::Register as R,
};
use crate::lowir::vocabulary::{Binary, Unary};

// Includes destinations and address registers; frame_layout::operands visits
// memory/source operands only. Implicit scratch registers are not MIR operands.
pub(super) fn map(op: &mut Op, mut reg: impl FnMut(R) -> R) {
    super::frame_layout::operands(op, |operand, _| match operand {
        Operand::Register(r) => *r = reg(*r),
        Operand::Memory(m) => {
            if let Base::Register(r) = &mut m.base {
                *r = reg(*r);
            }
            m.index = m.index.map(&mut reg);
        }
        _ => {}
    });
    match op {
        Op::Move(r, _)
        | Op::SymbolMove(r, _)
        | Op::Return(Some(r))
        | Op::FramelessReturn(Some(r)) => *r = reg(*r),
        Op::Scalar(s) => match s {
            ScalarOp::Move(_, r, _)
            | ScalarOp::Lea(r, _)
            | ScalarOp::Lea32(r, _)
            | ScalarOp::Load(r, _, _)
            | ScalarOp::Binary(_, _, r, _)
            | ScalarOp::Unary(_, _, r)
            | ScalarOp::Extend(_, r)
            | ScalarOp::Scan(_, _, r, _)
            | ScalarOp::SetEqual(r)
            | ScalarOp::Movzx(r)
            | ScalarOp::Shift(_, _, r, _)
            | ScalarOp::Rotate(_, _, r, _)
            | ScalarOp::Condition(
                FlagOp::Test(_, r)
                | FlagOp::Set(_, r)
                | FlagOp::Move(_, r, _)
                | FlagOp::BitTest(_, r, _),
            ) => *r = reg(*r),
            ScalarOp::ExtendMove(_, to, from) => {
                *to = reg(*to);
                *from = reg(*from);
            }
            _ => {}
        },
        _ => {}
    }
}
pub(super) fn named(mut op: Op) -> u32 {
    let mut bits = 0;
    map(&mut op, |r| {
        bits |= 1 << r.code();
        r
    });
    bits
}
pub(super) fn written(op: Op) -> Option<R> {
    match op {
        Op::Move(r, _)
        | Op::SymbolMove(r, _)
        | Op::Scalar(
            ScalarOp::Move(_, r, _)
            | ScalarOp::Lea(r, _)
            | ScalarOp::Lea32(r, _)
            | ScalarOp::Load(r, _, _)
            | ScalarOp::Binary(_, _, r, _)
            | ScalarOp::Unary(_, _, r)
            | ScalarOp::Extend(_, r)
            | ScalarOp::ExtendMove(_, r, _)
            | ScalarOp::Scan(_, _, r, _)
            | ScalarOp::SetEqual(r)
            | ScalarOp::Movzx(r)
            | ScalarOp::Shift(_, _, r, _)
            | ScalarOp::Rotate(_, _, r, _)
            | ScalarOp::Condition(FlagOp::Set(_, r) | FlagOp::Move(_, r, _)),
        ) => Some(r),
        _ => None,
    }
}
pub(super) fn frameless(op: Op) -> bool {
    let allowed = match op {
        Op::Nop
        | Op::Jump(_)
        | Op::Branch(..)
        | Op::Move(..)
        | Op::SymbolMove(..)
        | Op::Return(_) => true,
        Op::Scalar(s) => match s {
            ScalarOp::Move(..)
            | ScalarOp::Load(..)
            | ScalarOp::Lea(..)
            | ScalarOp::Lea32(..)
            | ScalarOp::Store(..)
            | ScalarOp::Extend(..)
            | ScalarOp::ExtendMove(..)
            | ScalarOp::CompareZero(..)
            | ScalarOp::SetEqual(..)
            | ScalarOp::Movzx(..)
            | ScalarOp::Shift(..)
            | ScalarOp::Rotate(..)
            | ScalarOp::Scan(..) => true,
            ScalarOp::Binary(b, ..) | ScalarOp::MemoryBinary(b, ..) => matches!(
                b,
                Binary::Add | Binary::Sub | Binary::And | Binary::Or | Binary::Xor | Binary::Mul
            ),
            ScalarOp::Unary(u, ..) => {
                matches!(u, Unary::Neg | Unary::Not | Unary::BitNot | Unary::ByteSwap)
            }
            ScalarOp::Condition(flag) => !matches!(flag, FlagOp::CompareFloatLiteral(..)),
            _ => false,
        },
        Op::Bulk(bulk) => bulk.frameless(),
        _ => false,
    };
    if !allowed {
        return false;
    }
    let mut valid = true;
    let mut copy = op;
    super::frame_layout::operands(&mut copy, |operand, _| {
        valid &= !matches!(operand, Operand::Frame(_))
            && !operand.memory().is_some_and(|m| m.base == Base::Frame);
    });
    valid && named(op) & (0xffff_0000 | (1 << R::RSP.code())) == 0
}
