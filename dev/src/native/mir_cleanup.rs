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
    let mut removed = vec![false; function.instructions.len()];
    for block in 0..function.blocks.len() {
        let range = function.blocks[block].range.clone();
        // This is narrow_to_u32's ALU case: an in-place u32 extension
        // immediately follows a full-width result whose flags are dead.
        for i in range.clone().skip(1) {
            let Op::Scalar(ScalarOp::Extend(Type::U32, r)) = function.instructions[i].op else {
                continue;
            };
            if narrow_to_u32(function, i - 1, r, range.end) {
                removed[i] = true;
            }
        }
    }
    compact(function, &removed);
    redundant_constants(function);
}

// simple_inst and written_reg from the frozen cleanup: only explicit GPR
// writes may preserve other holders. Calls, runtime operations, symbolic
// accesses, wide immediates and non-scalar operations forget every holder.
enum SimpleWrite {
    None,
    Register(R),
}
fn simple_write(op: Op) -> Option<SimpleWrite> {
    use super::condition_mir::FlagOp;
    use crate::lowir::vocabulary::Unary;
    let (writer, operands) = match op {
        Op::Move(r, bits) => (
            Some(r),
            [Some(Operand::Register(r)), Some(Operand::Immediate(bits))],
        ),
        Op::Scalar(op) => match op {
            ScalarOp::Move(r, source)
            | ScalarOp::Load(r, _, source)
            | ScalarOp::Lea(r, source)
            | ScalarOp::Lea32(r, source)
            | ScalarOp::Binary(_, _, r, source)
            | ScalarOp::Shift(_, r, source) => {
                (Some(r), [Some(Operand::Register(r)), Some(source)])
            }
            ScalarOp::Store(destination, _, source) => (None, [Some(destination), Some(source)]),
            ScalarOp::Extend(_, r)
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

fn redundant_constants(function: &mut Function) {
    let mut removed = vec![false; function.instructions.len()];
    for block in &function.blocks {
        // Both scalar immediate-move variants have the frozen Void width.
        // The high word is not part of this scalar register-holding key.
        let mut held = [None; 16];
        for i in block.range.clone() {
            let op = function.instructions[i].op;
            if matches!(op, Op::Branch(..)) {
                continue;
            }
            if let Op::Move(r, bits) | Op::Scalar(ScalarOp::Move(r, Operand::Immediate(bits))) = op
                && r.code() < 16
            {
                let Ok(value) = super::mir::low_word(bits) else {
                    continue;
                };
                let holder = &mut held[usize::from(r.code())];
                if *holder == Some(value) {
                    removed[i] = true;
                } else {
                    *holder = Some(value);
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
    compact(function, &removed);
}

fn flags_dead_after(function: &Function, at: usize, end: usize) -> bool {
    use super::condition_mir::FlagOp;
    use crate::lowir::vocabulary::Unary;
    for inst in &function.instructions[at + 1..end] {
        match inst.op {
            Op::Scalar(ScalarOp::Shift(_, _, Operand::Immediate(bits))) => return bits & 63 != 0,
            Op::Move(..)
            | Op::SymbolMove(..)
            | Op::Scalar(
                ScalarOp::Move(..)
                | ScalarOp::Load(..)
                | ScalarOp::Store(..)
                | ScalarOp::Lea(..)
                | ScalarOp::Movzx(..)
                | ScalarOp::Extend(..)
                | ScalarOp::ExtendMove(..)
                | ScalarOp::Shift(..),
            ) => {}
            Op::Return(..)
            | Op::Call(..)
            | Op::Jump(..)
            | Op::Scalar(
                ScalarOp::Binary(..)
                | ScalarOp::Unary(Unary::Neg, ..)
                | ScalarOp::Scan(..)
                | ScalarOp::Divide { .. }
                | ScalarOp::MultiplyWide { .. }
                | ScalarOp::Condition(FlagOp::Compare(..) | FlagOp::Test(..)),
            ) => return true,
            _ => return false,
        }
    }
    false
}

fn narrow_to_u32(function: &mut Function, at: usize, r: R, end: usize) -> bool {
    use crate::lowir::vocabulary::Unary;
    let op = function.instructions[at].op;
    let narrowed = match op {
        Op::Move(writer, bits) | Op::Scalar(ScalarOp::Move(writer, Operand::Immediate(bits)))
            if writer == r =>
        {
            let bytes = bits.to_le_bytes();
            let low = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            // A wide immediate move may use implicit scratch; the source
            // must meet simple_inst before this rewrite.
            if !super::mir::low_word(bits).is_ok_and(|v| i32::try_from(v).is_ok()) {
                return false;
            }
            Op::Scalar(ScalarOp::Move(r, Operand::Immediate(u128::from(low))))
        }
        Op::Scalar(ScalarOp::Move(writer, Operand::Register(source)))
            if writer == r && source.code() < 16 =>
        {
            Op::Scalar(ScalarOp::ExtendMove(Type::U32, r, source))
        }
        Op::Scalar(previous) if flags_dead_after(function, at, end) => {
            if let Some((binary, Type::I64 | Type::Ptr, writer, source)) = simple_binary(previous)
                && writer == r
            {
                Op::Scalar(ScalarOp::Binary(binary, Type::U32, r, source))
            } else if let ScalarOp::Unary(
                unary @ (Unary::Neg | Unary::BitNot),
                Type::I64 | Type::Ptr,
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
            let Op::Scalar(ScalarOp::Move(b, Operand::Register(a))) = function.instructions[i].op
            else {
                continue;
            };
            if a == b || a.code() >= 16 || b.code() >= 16 {
                continue;
            }
            let next = function.instructions[i + 1].op;
            let (op, ty, writer, source) = match next {
                Op::Scalar(ScalarOp::Binary(op, ty, r, source)) => (op, ty, r, source),
                Op::Scalar(ScalarOp::Shift(Binary::Shl, r, source)) => {
                    (Binary::Shl, Type::I64, r, source)
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
    compact(function, &removed);
}
fn compact(function: &mut Function, removed: &[bool]) {
    // The frozen branch-layout counter retains Nop instruction positions.
    // Compact storage keeps that deterministic count as an explicit fact.
    function.removed_instructions += removed.iter().filter(|&&drop| drop).count();
    let mut old = 0;
    let mut kept = 0;
    for block in &mut function.blocks {
        let count = removed[block.range.clone()]
            .iter()
            .filter(|&&drop| !drop)
            .count();
        block.range = kept..kept + count;
        kept += count;
    }
    function.instructions.retain(|_| {
        let keep = !removed[old];
        old += 1;
        keep
    });
}
