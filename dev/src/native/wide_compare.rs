//! Signed high-word and unsigned low-word i128 predicates. See NOTICE.
use super::{
    condition::Condition as C,
    condition_mir::FlagOp,
    mir::BlockId,
    scalar_mir::{Operand, ScalarOp},
    wide_mir::{alu, half},
    x86::{Assembler, Label, Register as R},
};
use crate::lowir::{Error, model::Program, vocabulary::Compare};
use std::io::Write;
#[derive(Clone, Copy)]
pub(super) enum Destination {
    Register(R),
    Branch { yes: BlockId, no: BlockId },
}
#[derive(Clone, Copy)]
pub(super) struct Comparison {
    pub(super) predicate: Compare,
    pub(super) destination: Destination,
    pub(super) left: Operand,
    pub(super) right: Operand,
}
impl Comparison {
    pub(super) fn write<W: Write>(
        self,
        program: &Program,
        out: &mut W,
        mut label: impl FnMut(BlockId, &mut W) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        write!(
            out,
            "    {}.{}.i128 ",
            if matches!(self.destination, Destination::Register(_)) {
                "set"
            } else {
                "cmp_branch"
            },
            self.predicate.spelling()
        )?;
        if let Destination::Register(r) = self.destination {
            write!(out, "{}, ", r.spelling())?;
        }
        self.left.write(program, out)?;
        write!(out, ", ")?;
        self.right.write(program, out)?;
        if let Destination::Branch { yes, no } = self.destination {
            write!(out, ", ")?;
            label(yes, out)?;
            write!(out, ", ")?;
            label(no, out)?;
        }
        Ok(())
    }
    pub(super) fn rebase(self, begin: i32, bytes: u32) -> Self {
        Self {
            left: self.left.rebase(begin, bytes),
            right: self.right.rebase(begin, bytes),
            ..self
        }
    }
    pub(super) fn emit(
        self,
        a: &mut Assembler,
        labels: &[Label],
        mut branch: impl FnMut(&mut Assembler, Option<u8>, Label),
    ) -> Result<(), Error> {
        let compare = |a: &mut Assembler, high| {
            half(a, R::R11, self.left, high)?;
            alu(a, 7, R::R11, self.right, high)
        };
        let pred = self.predicate;
        let equal = matches!(pred, Compare::Eq | Compare::Ne);
        let high_cc = strict_high(pred);
        let low_cc = low_condition(pred);
        if let Destination::Branch { yes, no } = self.destination {
            let yes = labels[yes.index()];
            let no = labels[no.index()];
            compare(a, !equal)?;
            if equal {
                branch(
                    a,
                    Some(C::NotEqual.code()),
                    if pred == Compare::Eq { no } else { yes },
                );
                compare(a, true)?;
                branch(a, Some(C::from_predicate(pred).code()), yes);
            } else {
                branch(a, Some(high_cc.code()), yes);
                branch(a, Some(C::NotEqual.code()), no);
                compare(a, false)?;
                branch(a, Some(low_cc.code()), yes);
            }
            branch(a, None, no);
            return Ok(());
        }
        let Destination::Register(dst) = self.destination else {
            return Err(Error::Limit);
        };
        if equal {
            compare(a, false)?;
            let done = a.label()?;
            a.branch(Some(C::NotEqual.code()), done, true);
            compare(a, true)?;
            a.bind(done);
            FlagOp::Set(C::from_predicate(pred), dst).emit(a)?;
        } else {
            compare(a, true)?;
            let high = a.label()?;
            let done = a.label()?;
            a.branch(Some(C::NotEqual.code()), high, true);
            compare(a, false)?;
            FlagOp::Set(low_cc, dst).emit(a)?;
            a.branch(None, done, true);
            a.bind(high);
            FlagOp::Set(high_cc, dst).emit(a)?;
            a.bind(done);
        }
        ScalarOp::Movzx(dst).emit(a, false)
    }
}
fn strict_high(pred: Compare) -> C {
    match pred {
        Compare::Lt | Compare::Le => C::Less,
        Compare::Gt | Compare::Ge => C::Greater,
        Compare::Ult | Compare::Ule => C::Below,
        _ => C::Above,
    }
}
fn low_condition(pred: Compare) -> C {
    match pred {
        Compare::Lt | Compare::Ult => C::Below,
        Compare::Le | Compare::Ule => C::BelowEqual,
        Compare::Gt | Compare::Ugt => C::Above,
        _ => C::AboveEqual,
    }
}
