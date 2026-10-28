//! Frozen block-local extension cleanup over typed scalar MIR. See NOTICE.
use super::{
    mir::{Function, Op},
    scalar_mir::{Operand, ScalarOp},
    x86::Register as R,
};
use crate::lowir::vocabulary::{Binary, Type};

// The simple ALU form has no implicit scratch effects when every operand is
// a GPR or a small immediate. A symbolic/memory source is a separate boundary.
fn simple_binary(op: ScalarOp) -> Option<(Binary, Type, R, Operand)> {
    let ScalarOp::Binary(binary, ty, r, source) = op else {
        return None;
    };
    let simple = match source {
        Operand::Register(s) => s.code() < 16,
        Operand::Immediate(bits) => {
            super::mir::low_word(bits).is_ok_and(|v| i32::try_from(v).is_ok())
        }
        _ => false,
    };
    (r.code() < 16 && simple).then_some((binary, ty, r, source))
}
pub(super) fn extensions(function: &mut Function) {
    copies_into_lea(function);
    super::address_fusion::run(function);
    super::flag_cleanup::drop_tests(function);
    redundant_copies(function);
    redundant_constants(function);
}

// simple_inst and written_reg from the frozen cleanup: only explicit GPR
// writes may preserve other holders. Calls, runtime operations, symbolic
// accesses, wide immediates and non-scalar operations forget every holder.
pub(super) enum SimpleWrite {
    None,
    Register(R),
}
pub(super) fn simple_write(op: Op) -> Option<SimpleWrite> {
    use super::condition_mir::FlagOp;
    use crate::lowir::vocabulary::Unary;
    let (writer, operands) = match op {
        Op::Nop => (None, [None, None]),
        Op::Move(r, bits) => (
            Some(r),
            [Some(Operand::Register(r)), Some(Operand::Immediate(bits))],
        ),
        Op::Scalar(op) => match op {
            ScalarOp::Move(_, r, source)
            | ScalarOp::Load(r, _, source)
            | ScalarOp::Lea(r, source)
            | ScalarOp::Lea32(r, source)
            | ScalarOp::Binary(_, _, r, source)
            | ScalarOp::Shift(_, _, r, source) => {
                (Some(r), [Some(Operand::Register(r)), Some(source)])
            }
            ScalarOp::Store(destination, _, source)
            | ScalarOp::MemoryBinary(_, _, destination, source) => {
                (None, [Some(destination), Some(source)])
            }
            ScalarOp::Rotate(_, _, r, _)
            | ScalarOp::Extend(_, r)
            | ScalarOp::Movzx(r)
            | ScalarOp::SetEqual(r)
            | ScalarOp::Unary(Unary::Neg | Unary::BitNot, _, r) => {
                (Some(r), [Some(Operand::Register(r)), None])
            }
            ScalarOp::ExtendMove(_, r, source) => (
                Some(r),
                [Some(Operand::Register(r)), Some(Operand::Register(source))],
            ),
            ScalarOp::CompareZero(_, source) => (None, [Some(source), Some(Operand::Immediate(0))]),
            // Frozen FImm operands bypass the signed integer-immediate limit.
            ScalarOp::Condition(FlagOp::CompareFloatLiteral(_, left, _)) => {
                (None, [Some(left), None])
            }
            ScalarOp::Condition(flag) => (
                match flag {
                    FlagOp::Set(_, r) | FlagOp::Move(_, r, _) => Some(r),
                    _ => None,
                },
                flag.operands(),
            ),
            _ => return None,
        },
        _ => return None,
    };
    operands
        .into_iter()
        .flatten()
        .all(|operand| match operand {
            Operand::Register(r) => r.code() < 16,
            Operand::Immediate(bits) => {
                super::mir::low_word(bits).is_ok_and(|v| i32::try_from(v).is_ok())
            }
            Operand::Memory(memory) => {
                !matches!(memory.base, super::memory::Base::Symbol(_))
                    && memory.index.is_none_or(|r| r.code() < 16)
            }
            Operand::Frame(_) => true,
            Operand::Symbol(_) | Operand::SymbolOffset(..) | Operand::AccessedSymbol(..) => false,
        })
        .then_some(writer.map_or(SimpleWrite::None, SimpleWrite::Register))
}

fn redundant_constants(function: &mut Function) -> usize {
    let mut removed = 0;
    for block in &function.blocks {
        // The frozen key includes Mov's type, even when two types encode
        // identically. Literal.high is not part of the scalar holding key.
        let mut held = [None; 16];
        for i in block.range.clone() {
            let op = function.instructions[i].op;
            if matches!(op, Op::Nop | Op::Branch(..)) {
                continue;
            }
            let immediate = match op {
                Op::Move(r, bits) => Some((Type::Void, r, bits)),
                Op::Scalar(ScalarOp::Move(ty, r, Operand::Immediate(bits))) => Some((ty, r, bits)),
                _ => None,
            };
            if let Some((ty, r, bits)) = immediate
                && r.code() < 16
            {
                let Ok(value) = super::mir::low_word(bits) else {
                    continue;
                };
                let holder = &mut held[usize::from(r.code())];
                if *holder == Some((ty, value)) {
                    function.instructions[i].op = Op::Nop;
                    removed += 1;
                } else {
                    *holder = Some((ty, value));
                }
                continue;
            }
            match simple_write(op) {
                Some(SimpleWrite::Register(writer)) => held[usize::from(writer.code())] = None,
                Some(SimpleWrite::None) => {}
                None => held.fill(None),
            }
        }
    }
    removed
}

pub(super) fn flags_dead_after(function: &Function, at: usize, end: usize) -> bool {
    use super::condition_mir::FlagOp;
    use crate::lowir::vocabulary::Unary;
    for inst in &function.instructions[at + 1..end] {
        match inst.op {
            Op::Scalar(ScalarOp::Shift(_, ty, _, Operand::Immediate(bits))) => {
                return bits & (u128::from(super::scalar_mir::bytes(ty)) * 8 - 1) != 0;
            }
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
                | ScalarOp::Shift(..)
                | ScalarOp::Rotate(..),
            ) => {}
            Op::Return(..)
            | Op::Call(..)
            | Op::Jump(..)
            | Op::JumpTable(_)
            | Op::Scalar(
                ScalarOp::Binary(..)
                | ScalarOp::MemoryBinary(..)
                | ScalarOp::Unary(Unary::Neg, ..)
                | ScalarOp::Scan(..)
                | ScalarOp::Divide { .. }
                | ScalarOp::MultiplyWide { .. }
                | ScalarOp::Condition(
                    FlagOp::Compare(..)
                    | FlagOp::Test(..)
                    | FlagOp::MaskTest(..)
                    | FlagOp::BitTest(..),
                ),
            ) => return true,
            _ => return false,
        }
    }
    false
}

pub(super) fn narrow_to_u32(function: &mut Function, at: usize, r: R, end: usize) -> bool {
    use crate::lowir::vocabulary::Unary;
    let op = function.instructions[at].op;
    if function.instructions[at].volatile
        || !matches!(simple_write(op), Some(SimpleWrite::Register(writer)) if writer == r)
    {
        return false;
    }
    let narrowed = match op {
        Op::Move(writer, bits)
        | Op::Scalar(ScalarOp::Move(_, writer, Operand::Immediate(bits)))
            if writer == r =>
        {
            let bytes = bits.to_le_bytes();
            let low = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            // A wide immediate move may use implicit scratch; the source
            // must meet simple_inst before this rewrite.
            if !super::mir::low_word(bits).is_ok_and(|v| i32::try_from(v).is_ok()) {
                return false;
            }
            // The frozen rewrite changes only Literal.low. Literal.high is
            // still observable in MIR even though scalar encoding ignores it.
            let narrowed = (bits & (u128::MAX << 64)) | u128::from(low);
            let ty = if let Op::Scalar(ScalarOp::Move(ty, ..)) = op {
                ty
            } else {
                Type::Void
            };
            Op::Scalar(ScalarOp::Move(ty, r, Operand::Immediate(narrowed)))
        }
        Op::Scalar(ScalarOp::Move(_, writer, Operand::Register(source)))
            if writer == r && source.code() < 16 =>
        {
            Op::Scalar(ScalarOp::ExtendMove(Type::U32, r, source))
        }
        Op::Scalar(ScalarOp::Load(writer, ty, source))
            if writer == r
                && super::scalar_mir::bytes(ty) >= 4
                && !matches!(ty, Type::F32 | Type::F64) =>
        {
            Op::Scalar(ScalarOp::Load(r, Type::U32, source))
        }
        Op::Scalar(ScalarOp::Lea(writer, source)) if writer == r => {
            Op::Scalar(ScalarOp::Lea32(r, source))
        }
        Op::Scalar(ScalarOp::Shift(
            Binary::Shl,
            Type::Void | Type::I64 | Type::Ptr,
            writer,
            Operand::Immediate(bits),
        )) if writer == r && bits < 32 && flags_dead_after(function, at, end) => Op::Scalar(
            ScalarOp::Shift(Binary::Shl, Type::U32, r, Operand::Immediate(bits)),
        ),
        Op::Scalar(previous) if flags_dead_after(function, at, end) => {
            if let Some((binary, Type::Void | Type::I64 | Type::Ptr, writer, source)) =
                simple_binary(previous)
                && writer == r
            {
                Op::Scalar(ScalarOp::Binary(binary, Type::U32, r, source))
            } else if let ScalarOp::Unary(
                unary @ (Unary::Neg | Unary::BitNot),
                Type::Void | Type::I64 | Type::Ptr,
                writer,
            ) = previous
                && writer == r
            {
                Op::Scalar(ScalarOp::Unary(unary, Type::U32, r))
            } else {
                return false;
            }
        }
        _ => return false,
    };
    function.instructions[at].op = narrowed;
    true
}

fn copies_into_lea(function: &mut Function) {
    use super::memory::{Base, Memory};
    let mut removed = vec![false; function.instructions.len()];
    for block in 0..function.blocks.len() {
        let range = function.blocks[block].range.clone();
        for i in range.start..range.end.saturating_sub(1) {
            let Op::Scalar(ScalarOp::Move(_, b, Operand::Register(a))) =
                function.instructions[i].op
            else {
                continue;
            };
            if a == b || a.code() >= 16 || b.code() >= 16 {
                continue;
            }
            let next = function.instructions[i + 1].op;
            let (op, ty, writer, source) = match next {
                Op::Scalar(ScalarOp::Binary(op, ty, r, source)) => (op, ty, r, source),
                Op::Scalar(ScalarOp::Shift(Binary::Shl, ty, r, source)) => {
                    (Binary::Shl, ty, r, source)
                }
                Op::Scalar(
                    ScalarOp::Lea(r, Operand::Memory(mut mem))
                    | ScalarOp::Lea32(r, Operand::Memory(mut mem)),
                ) if r == b => {
                    let Base::Register(base) = mem.base else {
                        continue;
                    };
                    if base != b && mem.index != Some(b)
                        || base.code() >= 16
                        || mem.index.is_some_and(|r| r.code() >= 16)
                        || mem.index == Some(b) && a == R::RSP
                    {
                        continue;
                    }
                    if base == b {
                        mem.base = Base::Register(a);
                    }
                    if mem.index == Some(b) {
                        mem.index = Some(a);
                    }
                    function.instructions[i + 1].op =
                        Op::Scalar(if matches!(next, Op::Scalar(ScalarOp::Lea32(..))) {
                            ScalarOp::Lea32(r, Operand::Memory(mem))
                        } else {
                            ScalarOp::Lea(r, Operand::Memory(mem))
                        });
                    removed[i] = true;
                    continue;
                }
                _ => continue,
            };
            let narrow = matches!(ty, Type::I32 | Type::U32);
            if writer != b
                || !(narrow || matches!(ty, Type::I64 | Type::Ptr))
                || !flags_dead_after(function, i + 1, range.end)
            {
                continue;
            }
            let mut mem = Memory::new(Base::Register(a), 0);
            match (op, source) {
                (Binary::Add | Binary::Sub, Operand::Immediate(bits)) => {
                    let Ok(value) = super::mir::low_word(bits) else {
                        continue;
                    };
                    if !narrow
                        && (i32::try_from(value).is_err()
                            || op == Binary::Sub && value == i64::from(i32::MIN))
                    {
                        continue;
                    }
                    let bytes = bits.to_le_bytes();
                    let low = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    mem.displacement = if op == Binary::Sub {
                        low.wrapping_neg()
                    } else {
                        low
                    }
                    .cast_signed();
                }
                (Binary::Add, Operand::Register(r))
                    if r != b && r.code() < 16 && (a != R::RSP || r != R::RSP) =>
                {
                    if r == R::RSP {
                        mem.base = Base::Register(R::RSP);
                        mem.index = Some(a);
                    } else {
                        mem.index = Some(r);
                    }
                }
                (Binary::Shl | Binary::Mul, Operand::Immediate(bits)) if a != R::RSP => {
                    let Ok(value) = super::mir::low_word(bits) else {
                        continue;
                    };
                    let factor = if op == Binary::Shl && (1..=3).contains(&value) {
                        1_i64 << value
                    } else if op == Binary::Mul {
                        value
                    } else {
                        continue;
                    };
                    if matches!(factor, 2 | 4 | 8) {
                        mem.index = Some(a);
                        if factor == 2 {
                            mem.scale = 1;
                        } else {
                            mem.base = Base::Absolute;
                            mem.scale = u8::try_from(factor).unwrap_or(1);
                        }
                    } else if !narrow && op == Binary::Mul && factor > 0 {
                        let shift = factor.trailing_zeros();
                        let odd = factor >> shift;
                        if !matches!(odd, 3 | 5 | 9) {
                            continue;
                        }
                        mem.index = Some(a);
                        mem.scale = u8::try_from(odd - 1).unwrap_or(1);
                        function.instructions[i].op =
                            Op::Scalar(ScalarOp::Lea(b, Operand::Memory(mem)));
                        if shift == 0 {
                            removed[i + 1] = true;
                        } else {
                            function.instructions[i + 1].op = Op::Scalar(ScalarOp::Shift(
                                Binary::Shl,
                                Type::I64,
                                b,
                                Operand::Immediate(u128::from(shift)),
                            ));
                        }
                        continue;
                    } else {
                        continue;
                    }
                }
                _ => continue,
            }
            function.instructions[i + 1].op = Op::Scalar(if narrow {
                ScalarOp::Lea32(b, Operand::Memory(mem))
            } else {
                ScalarOp::Lea(b, Operand::Memory(mem))
            });
            removed[i] = true;
        }
    }
    erase(function, &removed);
}
pub(super) fn erase(function: &mut Function, removed: &[bool]) {
    // Later cleanups observe immediate neighbours and the original ranges.
    // Keep the frozen no-op positions until branch layout releases them.
    for (instruction, &drop) in function.instructions.iter_mut().zip(removed) {
        if drop {
            instruction.op = Op::Nop;
        }
    }
}

pub(super) fn touches(op: Op, r: R) -> bool {
    use super::condition_mir::FlagOp;
    use crate::lowir::vocabulary::Unary;
    if matches!(op, Op::Scalar(ScalarOp::Binary(Binary::Shl | Binary::Shr | Binary::UShr, _, _, source)) if !matches!(source, Operand::Immediate(_)))
        && r == R::RCX
    {
        return true;
    }
    let operands = match op {
        Op::Move(to, _) | Op::SymbolMove(to, _) => [Some(Operand::Register(to)), None],
        Op::Scalar(op) => match op {
            ScalarOp::Move(_, to, source)
            | ScalarOp::Load(to, _, source)
            | ScalarOp::Lea(to, source)
            | ScalarOp::Lea32(to, source)
            | ScalarOp::Binary(_, _, to, source) => [Some(Operand::Register(to)), Some(source)],
            ScalarOp::Store(to, _, source) | ScalarOp::MemoryBinary(_, _, to, source) => {
                [Some(to), Some(source)]
            }
            ScalarOp::Condition(FlagOp::Compare(_, a, b) | FlagOp::MaskTest(_, a, b)) => {
                [Some(a), Some(b)]
            }
            ScalarOp::Condition(FlagOp::BitTest(_, r, b)) => [Some(Operand::Register(r)), Some(b)],
            ScalarOp::Condition(FlagOp::CompareFloatLiteral(_, a, _))
            | ScalarOp::CompareZero(_, a) => [Some(a), None],
            ScalarOp::Rotate(_, _, to, _)
            | ScalarOp::Extend(_, to)
            | ScalarOp::Movzx(to)
            | ScalarOp::Unary(Unary::Neg | Unary::BitNot, _, to) => {
                [Some(Operand::Register(to)), None]
            }
            ScalarOp::ExtendMove(_, to, source) => {
                [Some(Operand::Register(to)), Some(Operand::Register(source))]
            }
            ScalarOp::Shift(_, _, to, count) => {
                if !matches!(count, Operand::Immediate(_)) && r == R::RCX {
                    return true;
                }
                [Some(Operand::Register(to)), Some(count)]
            }
            ScalarOp::Condition(FlagOp::Test(_, to)) => [Some(Operand::Register(to)), None],
            _ => return true,
        },
        Op::Nop => return false,
        _ => return true,
    };
    operands
        .into_iter()
        .flatten()
        .any(|value| value.mask() & (1 << r.code()) != 0)
}
pub(super) fn redundant_copies(function: &mut Function) -> usize {
    let mut removed = 0;
    let mut same = Vec::new();
    for block in &function.blocks {
        same.clear();
        for i in block.range.clone() {
            let op = function.instructions[i].op;
            if matches!(op, Op::Nop) {
                continue;
            }
            if let Op::Scalar(ScalarOp::Move(ty, to, Operand::Register(source))) = op
                && to.code() < 16
                && source.code() < 16
                && matches!(ty, Type::Void | Type::Ptr | Type::I64)
            {
                let mut known = to == source;
                for &(a, b) in &same {
                    known = known || a == to && b == source || a == source && b == to;
                }
                if known {
                    function.instructions[i].op = Op::Nop;
                    removed += 1;
                    continue;
                }
                same.retain(|&(a, b)| a != to && b != to);
                same.push((to, source));
            } else {
                same.retain(|&(a, b)| !touches(op, a) && !touches(op, b));
            }
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(ops: &[Op]) -> Function {
        let p = crate::lowir::test_program(
            b"function @f() -> void {block ^entry: jump ^exit block ^exit: return void}",
        )
        .unwrap();
        let symbol = p.publications()[0];
        let mut f = super::super::value_selection::select(
            &p,
            symbol,
            p.function(p.symbol(symbol)).unwrap(),
            0,
        )
        .unwrap();
        f.instructions = ops
            .iter()
            .map(|&op| super::super::mir::Instruction {
                op,
                location: None,
                volatile: false,
            })
            .collect();
        f.blocks[0].range = 0..ops.len();
        f.blocks[1].range = ops.len()..ops.len();
        f
    }

    #[test]
    fn constant_holders_retain_types_and_reset_only_at_the_frozen_boundaries() {
        let held = Op::Scalar(ScalarOp::Move(Type::I64, R::R11, Operand::Immediate(1)));
        let mut f = function(&[
            Op::Move(R::R11, 1),
            Op::Branch(
                super::super::condition::Condition::Equal,
                super::super::mir::BlockId(1),
            ),
            held,
            Op::Scalar(ScalarOp::Move(
                Type::I64,
                R::R11,
                Operand::Immediate(1 | (7 << 64)),
            )),
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I64,
                Operand::Immediate(0),
            )),
            held,
            // Immediate moves are handled before the implicit-scratch guard.
            Op::Move(R::R9, u128::from(u64::MAX)),
            held,
            Op::Scalar(ScalarOp::Load(R::R11, Type::I64, Operand::Frame(-8))),
            held,
            Op::Trap,
            held,
            held,
        ]);
        f.blocks[0].range = 0..12;
        f.blocks[1].range = 12..13;
        f.instructions[3].volatile = true;
        assert_eq!(redundant_constants(&mut f), 3);
        for at in [3, 5, 7] {
            assert!(matches!(f.instructions[at].op, Op::Nop));
        }
        for at in [2, 9, 11, 12] {
            assert!(matches!(
                f.instructions[at].op,
                Op::Scalar(ScalarOp::Move(..))
            ));
        }
        assert!(f.instructions[3].volatile);
    }

    #[test]
    fn copy_holders_are_pairwise_and_narrow_writes_and_new_blocks_forget_them() {
        let copy =
            |to, source| Op::Scalar(ScalarOp::Move(Type::Void, to, Operand::Register(source)));
        let mut f = function(&[
            copy(R::R8, R::RDI),
            copy(R::R9, R::R8),
            copy(R::RDI, R::R9),
            copy(R::R8, R::R9),
            Op::Scalar(ScalarOp::ExtendMove(Type::U32, R::R8, R::R8)),
            copy(R::R8, R::R9),
            Op::Scalar(ScalarOp::SetEqual(R::RAX)),
            copy(R::R9, R::R8),
            copy(R::R8, R::R9),
        ]);
        f.blocks[0].range = 0..8;
        f.blocks[1].range = 8..9;
        assert_eq!(redundant_copies(&mut f), 1);
        assert!(matches!(f.instructions[3].op, Op::Nop));
        for at in [2, 5, 7, 8] {
            assert!(matches!(
                f.instructions[at].op,
                Op::Scalar(ScalarOp::Move(..))
            ));
        }
    }

    #[test]
    fn copy_pairs_survive_nops_but_not_named_or_implicit_register_uses() {
        let p = crate::lowir::test_program(b"function @f() -> void {block ^entry: return void}")
            .unwrap();
        let symbol = p.publications()[0];
        let mut f = super::super::value_selection::select(
            &p,
            symbol,
            p.function(p.symbol(symbol)).unwrap(),
            0,
        )
        .unwrap();
        let copy = |a, b| Op::Scalar(ScalarOp::Move(Type::Void, a, Operand::Register(b)));
        let ops = [
            copy(R::R8, R::RDI),
            Op::Nop,
            copy(R::RDI, R::R8),
            copy(R::RCX, R::R9),
            Op::Scalar(ScalarOp::Shift(
                Binary::Shl,
                Type::I64,
                R::RAX,
                Operand::Register(R::RDX),
            )),
            copy(R::R9, R::RCX),
            Op::Return(None),
        ];
        f.instructions = ops
            .into_iter()
            .map(|op| super::super::mir::Instruction {
                op,
                location: None,
                volatile: false,
            })
            .collect();
        f.blocks[0].range = 0..f.instructions.len();
        assert_eq!(redundant_copies(&mut f), 1);
        assert!(matches!(f.instructions[2].op, Op::Nop));
        assert!(matches!(
            f.instructions[5].op,
            Op::Scalar(ScalarOp::Move(..))
        ));
    }
}
