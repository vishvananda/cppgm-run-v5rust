//! Frozen address fusion and upper-half tracking from `select/mir_cleanup.cpp`.
//! See NOTICE. All state and scans are confined to one MIR block.
use super::{
    condition_mir::FlagOp,
    memory::{Base, Memory},
    mir::{Function, Op},
    mir_cleanup::{flags_dead_after, narrow_to_u32, simple_write, touches},
    mir_registers::written,
    scalar_mir::{Operand, ScalarOp},
    x86::Register as R,
};
use crate::lowir::vocabulary::{Binary, Type};

fn narrow(ty: Type) -> bool {
    matches!(ty, Type::I32 | Type::U32)
}
fn wide(ty: Type) -> bool {
    matches!(ty, Type::Void | Type::I64 | Type::Ptr)
}
fn add(op: Op) -> Option<(Type, R, Operand)> {
    match op {
        Op::Scalar(ScalarOp::Binary(Binary::Add, ty, r, source)) if r.code() < 16 => {
            Some((ty, r, source))
        }
        _ => None,
    }
}
fn lea(narrow: bool, r: R, memory: Memory) -> Op {
    Op::Scalar(if narrow {
        ScalarOp::Lea32(r, Operand::Memory(memory))
    } else {
        ScalarOp::Lea(r, Operand::Memory(memory))
    })
}
fn fuse_add(function: &mut Function, at: usize, end: usize) -> bool {
    let Some((ty, destination, first)) = add(function.instructions[at].op) else {
        return false;
    };
    let mut next = at + 1;
    while next < end && !touches(function.instructions[next].op, destination) {
        next += 1;
    }
    let Some((other_ty, writer, second)) = function
        .instructions
        .get(next)
        .filter(|_| next < end)
        .and_then(|inst| add(inst.op))
    else {
        return false;
    };
    if writer != destination
        || !(narrow(ty) && narrow(other_ty) || wide(ty) && wide(other_ty))
        || !flags_dead_after(function, next, end)
    {
        return false;
    }
    let (imm, reg) = if matches!(first, Operand::Immediate(_)) {
        (first, second)
    } else {
        (second, first)
    };
    let (Operand::Immediate(bits), Operand::Register(index)) = (imm, reg) else {
        return false;
    };
    if index.code() >= 16
        || index == destination
        || index == R::RSP
        || next != at + 1 && !matches!(first, Operand::Immediate(_))
    {
        return false;
    }
    let Ok(value) = super::mir::low_word(bits) else {
        return false;
    };
    if !narrow(ty) && i32::try_from(value).is_err() {
        return false;
    }
    let bytes = bits.to_le_bytes();
    let disp = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let mut memory = Memory::new(Base::Register(destination), disp);
    memory.index = Some(index);
    function.instructions[next].op = lea(narrow(ty), destination, memory);
    function.instructions[at].op = Op::Nop;
    true
}
fn fuse_lea(function: &mut Function, at: usize, end: usize) -> bool {
    let (destination, memory, narrow_lea) = match function.instructions[at].op {
        Op::Scalar(ScalarOp::Lea(destination, Operand::Memory(m))) => (destination, m, false),
        Op::Scalar(ScalarOp::Lea32(destination, Operand::Memory(m))) => (destination, m, true),
        _ => return false,
    };
    let Base::Register(base) = memory.base else {
        return false;
    };
    if destination.code() >= 16
        || base.code() >= 16
        || base == destination
        || memory.index.is_some()
    {
        return false;
    }
    let mut next = at + 1;
    while next < end
        && !touches(function.instructions[next].op, destination)
        && written(function.instructions[next].op) != Some(base)
        && simple_write(function.instructions[next].op).is_some()
    {
        next += 1;
    }
    let Some((ty, writer, Operand::Register(index))) = function
        .instructions
        .get(next)
        .filter(|_| next < end)
        .and_then(|inst| add(inst.op))
    else {
        return false;
    };
    if writer != destination
        || index.code() >= 16
        || index == destination
        || index == R::RSP
        || !(if narrow_lea { narrow(ty) } else { wide(ty) })
        || !flags_dead_after(function, next, end)
    {
        return false;
    }
    let mut combined = Memory::new(Base::Register(base), memory.displacement);
    combined.index = Some(index);
    function.instructions[next].op = lea(narrow_lea, destination, combined);
    function.instructions[at].op = Op::Nop;
    true
}
fn in_place_extension(op: Op) -> Option<R> {
    match op {
        Op::Scalar(ScalarOp::Extend(Type::U32, r)) if r.code() < 16 => Some(r),
        Op::Scalar(ScalarOp::ExtendMove(Type::U32, r, source)) if r.code() < 16 && r == source => {
            Some(r)
        }
        _ => None,
    }
}
fn track(op: Op, known: u32) -> u32 {
    use crate::lowir::vocabulary::Unary;
    match op {
        Op::Nop
        | Op::Jump(_)
        | Op::Branch(..)
        | Op::Scalar(
            ScalarOp::Store(..)
            | ScalarOp::CompareZero(..)
            | ScalarOp::Condition(
                FlagOp::Compare(..) | FlagOp::Test(..) | FlagOp::MaskTest(..) | FlagOp::BitTest(..),
            ),
        ) => known,
        Op::Move(..)
        | Op::SymbolMove(..)
        | Op::Scalar(
            ScalarOp::Move(..)
            | ScalarOp::Load(..)
            | ScalarOp::Lea(..)
            | ScalarOp::Lea32(..)
            | ScalarOp::Binary(..)
            | ScalarOp::Unary(Unary::Neg | Unary::BitNot, ..)
            | ScalarOp::Shift(..)
            | ScalarOp::Rotate(..)
            | ScalarOp::Movzx(..)
            | ScalarOp::Extend(..)
            | ScalarOp::ExtendMove(..)
            | ScalarOp::Condition(FlagOp::Set(..)),
        ) => {
            let Some(r) = written(op) else { return known };
            if r.code() >= 16 {
                return 0;
            }
            let zero = if matches!(op, Op::Scalar(ScalarOp::Condition(FlagOp::Set(..)))) {
                known & (1 << r.code()) != 0
            } else {
                super::flag_cleanup::zeroes(op, known)
            };
            if zero && r != R::R10 && r != R::R11 {
                known | (1 << r.code())
            } else {
                known & !(1 << r.code())
            }
        }
        _ => 0,
    }
}
pub(super) fn run(function: &mut Function) {
    for block in 0..function.blocks.len() {
        let range = function.blocks[block].range.clone();
        let mut known = 0;
        for i in range.clone() {
            if fuse_add(function, i, range.end) || fuse_lea(function, i, range.end) {
                continue;
            }
            let op = function.instructions[i].op;
            if let Some(r) = in_place_extension(op) {
                let mut w = i;
                if known & (1 << r.code()) == 0 {
                    while w > range.start
                        && i - w < 16
                        && !touches(function.instructions[w - 1].op, r)
                    {
                        w -= 1;
                    }
                }
                if known & (1 << r.code()) != 0
                    || w > range.start && narrow_to_u32(function, w - 1, r, range.end)
                {
                    if r != R::R10 && r != R::R11 {
                        known |= 1 << r.code();
                    }
                    function.instructions[i].op = Op::Nop;
                    continue;
                }
            }
            known = track(op, known);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn function(ops: &[Op]) -> Function {
        let program =
            crate::lowir::test_program(b"function @f()->void {block ^entry:return void}").unwrap();
        let symbol = program.publications()[0];
        let mut f = super::super::value_selection::select(
            &program,
            symbol,
            program.function(program.symbol(symbol)).unwrap(),
            0,
        )
        .unwrap();
        f.instructions = ops
            .iter()
            .copied()
            .map(|op| super::super::mir::Instruction {
                op,
                location: None,
                volatile: false,
            })
            .collect();
        f.blocks[0].range = 0..ops.len();
        f
    }
    fn add_to(source: Operand) -> Op {
        Op::Scalar(ScalarOp::Binary(Binary::Add, Type::I64, R::RAX, source))
    }
    #[test]
    fn add_pair_respects_intervening_reads_and_flag_consumers() {
        let immediate = add_to(Operand::Immediate(7));
        let register = add_to(Operand::Register(R::RCX));
        let independent = Op::Move(R::RDX, 3);
        for (first, second, fuses) in [(immediate, register, true), (register, immediate, false)] {
            let mut f = function(&[first, independent, second, Op::Return(None)]);
            run(&mut f);
            assert_eq!(matches!(f.instructions[0].op, Op::Nop), fuses);
            if fuses {
                assert!(
                    matches!(f.instructions[2].op, Op::Scalar(ScalarOp::Lea(R::RAX,Operand::Memory(m))) if m.base == Base::Register(R::RAX) && m.index == Some(R::RCX) && m.displacement == 7)
                );
            }
        }
        let mut f = function(&[
            immediate,
            register,
            Op::Branch(
                super::super::condition::Condition::Equal,
                super::super::mir::BlockId(0),
            ),
            Op::Return(None),
        ]);
        run(&mut f);
        assert!(!matches!(f.instructions[0].op, Op::Nop));
    }
    #[test]
    fn displaced_lea_keeps_its_source_until_the_add() {
        for (middle, fuses) in [(Op::Move(R::RDX, 3), true), (Op::Move(R::RDI, 3), false)] {
            let mut f = function(&[
                lea(false, R::RAX, Memory::new(Base::Register(R::RDI), -7)),
                middle,
                add_to(Operand::Register(R::RCX)),
                Op::Return(None),
            ]);
            run(&mut f);
            assert_eq!(matches!(f.instructions[0].op, Op::Nop), fuses);
            if fuses {
                assert!(
                    matches!(f.instructions[2].op,Op::Scalar(ScalarOp::Lea(R::RAX,Operand::Memory(m))) if m.base == Base::Register(R::RDI) && m.index == Some(R::RCX) && m.displacement == -7)
                );
            }
        }
    }
    #[test]
    fn immediate_narrowing_preserves_the_literal_high_word_in_the_view() {
        let bits = (-3_i128).cast_unsigned();
        let mut f = function(&[
            Op::Move(R::RAX, bits),
            Op::Scalar(ScalarOp::Extend(Type::U32, R::RAX)),
            Op::Return(Some(R::RAX)),
        ]);
        run(&mut f);
        let expected = (bits & (u128::MAX << 64)) | u128::from(u32::MAX - 2);
        assert!(matches!(f.instructions[0].op,
            Op::Scalar(ScalarOp::Move(_, R::RAX, Operand::Immediate(value))) if value == expected));
        assert!(matches!(f.instructions[1].op, Op::Nop));
    }
    #[test]
    fn narrowing_crosses_sixteen_unrelated_instructions_and_respects_volatile_loads() {
        for (between, removed) in [(16, true), (17, false)] {
            let mut ops = vec![Op::Scalar(ScalarOp::Load(
                R::RAX,
                Type::I64,
                Operand::Frame(-8),
            ))];
            ops.extend(std::iter::repeat_n(Op::Nop, between));
            ops.push(Op::Scalar(ScalarOp::Extend(Type::U32, R::RAX)));
            ops.push(Op::Return(None));
            let mut f = function(&ops);
            run(&mut f);
            assert_eq!(matches!(f.instructions[between + 1].op, Op::Nop), removed);
            assert_eq!(
                matches!(
                    f.instructions[0].op,
                    Op::Scalar(ScalarOp::Load(_, Type::U32, _))
                ),
                removed
            );
        }
        let mut f = function(&[
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
            Op::Scalar(ScalarOp::Extend(Type::U32, R::RAX)),
            Op::Return(None),
        ]);
        f.instructions[0].volatile = true;
        run(&mut f);
        assert!(matches!(
            f.instructions[1].op,
            Op::Scalar(ScalarOp::Extend(..))
        ));
    }
}
