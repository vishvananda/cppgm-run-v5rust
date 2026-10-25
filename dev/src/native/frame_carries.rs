//! Bounded encoder tracking of the frozen frame-home carried reloads. See NOTICE.
use super::{
    mir::{Binding, BindingKind, Instruction, Op},
    scalar_mir::{Operand, ScalarOp},
    x86::Register as R,
};
use crate::lowir::vocabulary::{Type, Unary};
#[derive(Default)]
pub(super) struct Carries {
    values: Vec<(i32, R)>,
    homes: Vec<i32>,
}
fn operands(op: &Op) -> Option<[Option<Operand>; 2]> {
    Some(match *op {
        Op::Move(r, v) => [Some(Operand::Register(r)), Some(Operand::Immediate(v))],
        Op::Scalar(op) => match op {
            ScalarOp::Condition(
                super::condition_mir::FlagOp::Move(..) | super::condition_mir::FlagOp::Test(..),
            ) => return None,
            ScalarOp::Condition(op) => op.operands(),
            ScalarOp::Move(r, s) | ScalarOp::Lea(r, s) | ScalarOp::Binary(_, _, r, s) => {
                [Some(Operand::Register(r)), Some(s)]
            }
            ScalarOp::Load(r, _, d) => [Some(Operand::Register(r)), Some(d)],
            ScalarOp::Store(d, _, r) => [Some(d), Some(r)],
            ScalarOp::Extend(_, r) | ScalarOp::SetEqual(r) | ScalarOp::Movzx(r) => {
                [Some(Operand::Register(r)), None]
            }
            ScalarOp::Unary(Unary::Neg | Unary::BitNot, _, r) => [Some(Operand::Register(r)), None],
            ScalarOp::CompareZero(_, s) => [Some(s), None],
            ScalarOp::Shift(_, _, r, s) => [Some(Operand::Register(r)), Some(s)],
            _ => return None,
        },
        _ => return None,
    })
}
fn plain(op: &Op) -> bool {
    operands(op).is_some_and(|ops| {
        ops.into_iter().flatten().all(|operand| match operand {
            Operand::Symbol(_) | Operand::SymbolOffset(..) | Operand::AccessedSymbol(..) => false,
            Operand::Memory(m) if matches!(m.base, super::memory::Base::Symbol(_)) => false,
            Operand::Immediate(v) => {
                super::mir::low_word(v).is_ok_and(|v| i32::try_from(v).is_ok())
            }
            _ => true,
        })
    })
}
fn written(op: &Op) -> Option<R> {
    match op {
        Op::Scalar(
            ScalarOp::Store(..)
            | ScalarOp::CompareZero(..)
            | ScalarOp::Condition(
                super::condition_mir::FlagOp::Compare(..) | super::condition_mir::FlagOp::Test(..),
            ),
        ) => None,
        _ => operands(op)
            .and_then(|ops| ops[0])
            .and_then(Operand::register),
    }
}
fn reload(op: &Op, offset: i32) -> Option<R> {
    if let Op::Scalar(ScalarOp::Load(r, Type::I64 | Type::Ptr, d)) = *op
        && d.frame_offset() == Some(offset)
    {
        return Some(r);
    }
    None
}
fn store(op: &Op) -> Option<i32> {
    if let Op::Scalar(ScalarOp::Store(offset, _, _)) = *op {
        offset.frame_offset()
    } else {
        None
    }
}
impl Carries {
    pub(super) fn clear(&mut self) {
        self.values.clear();
    }
    pub(super) fn new(bindings: &[Binding]) -> Self {
        let mut homes: Vec<_> = bindings
            .iter()
            .filter(|b| {
                b.kind != BindingKind::Slot
                    && !matches!(b.ty, Type::Object { .. } | Type::I128 | Type::F80)
            })
            .map(|b| b.offset)
            .collect();
        homes.sort_unstable();
        Self {
            values: Vec::new(),
            homes,
        }
    }
    pub(super) fn replacement(&self, op: &Op) -> Option<(R, R)> {
        if let Op::Scalar(ScalarOp::Load(r, Type::I64 | Type::Ptr, d)) = *op {
            self.values
                .iter()
                .find(|(offset, _)| Some(*offset) == d.frame_offset())
                .map(|(_, source)| (r, *source))
        } else {
            None
        }
    }
    pub(super) fn track(&mut self, instructions: &[Instruction], index: usize) -> Option<R> {
        let op = &instructions[index].op;
        if !plain(op) {
            self.values.clear();
            return None;
        }
        let home = if let Op::Scalar(ScalarOp::Store(
            d,
            Type::I64 | Type::Ptr,
            Operand::Register(r),
        )) = *op
        {
            d.frame_offset()
                .filter(|d| self.homes.binary_search(d).is_ok())
                .map(|d| (d, r))
        } else {
            None
        };
        let Some((offset, source)) = home else {
            self.values
                .retain(|(offset, r)| Some(*r) != written(op) && Some(*offset) != store(op));
            return None;
        };
        self.values.retain(|(d, _)| *d != offset);
        let mut source_written = false;
        let mut scratch_used = source == R::R11 || self.values.iter().any(|(_, r)| *r == R::R11);
        for inst in &instructions[index + 1..instructions.len().min(index + 64)] {
            if reload(&inst.op, offset).is_some() {
                if source_written && scratch_used {
                    return None;
                }
                self.values
                    .push((offset, if source_written { R::R11 } else { source }));
                return source_written.then_some(source);
            }
            if !plain(&inst.op) || store(&inst.op) == Some(offset) {
                return None;
            }
            source_written |= written(&inst.op) == Some(source);
            scratch_used |= operands(&inst.op).is_some_and(|ops| {
                ops.into_iter()
                    .flatten()
                    .any(|o| o.mask() & (1 << R::R11.code()) != 0)
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn materialized_condition_test_ends_home_carry_search() {
        let instructions = [
            ScalarOp::Store(Operand::Frame(-8), Type::I64, Operand::Register(R::RAX)),
            ScalarOp::Condition(super::super::condition_mir::FlagOp::Test(Type::U8, R::R8)),
            ScalarOp::Load(R::R11, Type::I64, Operand::Frame(-8)),
        ]
        .map(|op| Instruction {
            volatile: false,
            op: Op::Scalar(op),
            location: None,
        });
        let mut carries = Carries {
            values: Vec::new(),
            homes: vec![-8],
        };
        assert_eq!(carries.track(&instructions, 0), None);
        assert_eq!(carries.replacement(&instructions[2].op), None);
    }
}
