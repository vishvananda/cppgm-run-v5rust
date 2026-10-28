//! Redundant zero tests with exact flag-width and upper-word proofs. See NOTICE.
use super::{
    condition::Condition as C,
    condition_mir::FlagOp,
    mir::{Function, Op},
    mir_cleanup::{SimpleWrite, simple_write},
    scalar_mir::{Operand, ScalarOp, bytes},
    x86::Register as R,
};
use crate::lowir::vocabulary::{Binary, Type, Unary};
fn narrow(ty: Type) -> bool {
    matches!(ty, Type::I32 | Type::U32)
}
pub(super) fn zeroes(op: Op, known: u32) -> bool {
    match op {
        Op::Move(_, bits) | Op::Scalar(ScalarOp::Move(_, _, Operand::Immediate(bits))) => {
            super::mir::low_word(bits).is_ok_and(|word| u32::try_from(word).is_ok())
        }
        Op::Scalar(ScalarOp::Move(_, _, Operand::Register(r))) => {
            r.code() < 16 && known & (1 << r.code()) != 0
        }
        Op::Scalar(
            ScalarOp::Load(_, Type::U8 | Type::U16 | Type::U32, _)
            | ScalarOp::Lea32(..)
            | ScalarOp::Movzx(_),
        ) => true,
        Op::Scalar(ScalarOp::Binary(Binary::And, ty, _, source)) => {
            narrow(ty)
                || matches!(source,Operand::Immediate(bits) if super::mir::low_word(bits).is_ok_and(|word| u32::try_from(word).is_ok()))
        }
        Op::Scalar(
            ScalarOp::Binary(_, ty, _, _) | ScalarOp::Unary(Unary::Neg | Unary::BitNot, ty, _),
        ) => narrow(ty),
        Op::Scalar(ScalarOp::Shift(Binary::UShr, ty, _, source)) => {
            ty == Type::U32 || matches!(source,Operand::Immediate(bits) if bits&63>=32)
        }
        Op::Scalar(ScalarOp::Shift(_, ty, _, _) | ScalarOp::Rotate(_, ty, _, _)) => ty == Type::U32,
        Op::Scalar(ScalarOp::Extend(ty, _) | ScalarOp::ExtendMove(ty, _, _)) => {
            matches!(ty, Type::I1 | Type::U8 | Type::U16 | Type::U32)
        }
        _ => false,
    }
}
fn skip(op: Op, reg: R) -> bool {
    let Some(write) = simple_write(op) else {
        return false;
    };
    let candidate = matches!(
        op,
        Op::Nop
            | Op::Scalar(
                ScalarOp::Store(..) | ScalarOp::Lea(..) | ScalarOp::Lea32(..) | ScalarOp::Load(..)
            )
    ) || matches!(op,Op::Move(_,bits)|Op::Scalar(ScalarOp::Move(_, _,Operand::Immediate(bits))) if super::mir::low_word(bits).is_ok_and(|word| word != 0))
        || matches!(op,Op::Scalar(ScalarOp::Move(_, _,source)) if !matches!(source,Operand::Immediate(_)));
    candidate && !matches!(write, SimpleWrite::Register(r) if r == reg)
}
fn width(op: Op, reg: R) -> Option<(u8, bool)> {
    match op {
        Op::Scalar(ScalarOp::Binary(binary, ty, r, source)) if r == reg => {
            let mut width = if ty == Type::Void || ty == Type::Ptr {
                8
            } else {
                bytes(ty)
            };
            let full = matches!(binary, Binary::And | Binary::Or | Binary::Xor);
            if binary == Binary::And
                && width == 8
                && matches!(source,Operand::Immediate(bits) if super::mir::low_word(bits).is_ok_and(|word| u32::try_from(word).is_ok()))
            {
                width = 4;
            }
            matches!(
                binary,
                Binary::And | Binary::Or | Binary::Xor | Binary::Add | Binary::Sub
            )
            .then_some((width, full))
        }
        Op::Scalar(ScalarOp::Shift(_, ty, r, Operand::Immediate(bits))) if r == reg => {
            let width = if ty == Type::U32 { 4 } else { 8 };
            (bits & (u128::from(width) * 8 - 1) != 0).then_some((width, false))
        }
        _ => None,
    }
}
fn readers(f: &Function, start: usize, end: usize, full: bool, zero_only: bool) -> bool {
    for i in start..end {
        let op = f.instructions[i].op;
        let condition = match op {
            Op::Branch(cc, _)
            | Op::Scalar(ScalarOp::Condition(FlagOp::Set(cc, _) | FlagOp::Move(cc, _, _))) => {
                Some(cc)
            }
            Op::Scalar(ScalarOp::SetEqual(_)) => Some(C::Equal),
            _ => None,
        };
        if let Some(cc) = condition {
            if !((full && !zero_only) || matches!(cc, C::Equal | C::NotEqual)) {
                return false;
            }
            continue;
        }
        if matches!(
            op,
            Op::Nop
                | Op::Move(..)
                | Op::SymbolMove(..)
                | Op::Scalar(
                    ScalarOp::Move(..)
                        | ScalarOp::Load(..)
                        | ScalarOp::Store(..)
                        | ScalarOp::Lea(..)
                        | ScalarOp::Lea32(..)
                        | ScalarOp::Movzx(..)
                        | ScalarOp::Extend(..)
                        | ScalarOp::ExtendMove(..)
                )
        ) {
            continue;
        }
        return super::mir_cleanup::flags_dead_after(f, i - 1, i + 1);
    }
    true
}
pub(super) fn drop_tests(f: &mut Function) {
    for block in 0..f.blocks.len() {
        let range = f.blocks[block].range.clone();
        let mut known = 0;
        let mut known_at = range.start;
        for i in range.clone().skip(1) {
            let test = match f.instructions[i].op {
                Op::Scalar(ScalarOp::Condition(flag)) => flag.self_test().or_else(|| match flag {
                    FlagOp::Compare(ty, Operand::Register(r), Operand::Immediate(bits))
                        if super::mir::low_word(bits).is_ok_and(|word| word == 0) =>
                    {
                        Some((ty, r))
                    }
                    _ => None,
                }),
                Op::Scalar(ScalarOp::CompareZero(ty, Operand::Register(r))) => Some((ty, r)),
                _ => None,
            };
            let Some((ty, reg)) = test else {
                continue;
            };
            if reg.code() >= 16 {
                continue;
            }
            let mut at = i - 1;
            while at > range.start && skip(f.instructions[at].op, reg) {
                at -= 1;
            }
            let prior = f.instructions[at].op;
            let Some((mut width, full)) = width(prior, reg) else {
                continue;
            };
            let mut zero_only = false;
            if width == 8
                && bytes(ty) == 4
                && matches!(prior, Op::Scalar(ScalarOp::Binary(Binary::And, ..)))
            {
                while known_at < at {
                    let op = f.instructions[known_at].op;
                    known_at += 1;
                    known = update_known(op, known);
                }
                let Op::Scalar(ScalarOp::Binary(_, _, _, other)) = prior else {
                    continue;
                };
                if known & (1 << reg.code()) != 0
                    || other
                        .register()
                        .is_some_and(|r| r.code() < 16 && known & (1 << r.code()) != 0)
                {
                    width = 4;
                    zero_only = true;
                }
            }
            if width != bytes(ty) || !readers(f, i + 1, range.end, full, zero_only) {
                continue;
            }
            f.instructions[i].op = Op::Nop;
        }
    }
}

fn update_known(op: Op, known: u32) -> u32 {
    match simple_write(op) {
        None => 0,
        Some(SimpleWrite::None) => known,
        Some(SimpleWrite::Register(reg)) => {
            if zeroes(op, known) {
                known | (1 << reg.code())
            } else {
                known & !(1 << reg.code())
            }
        }
    }
}
