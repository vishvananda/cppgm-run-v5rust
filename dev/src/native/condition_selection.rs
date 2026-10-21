//! Integer conditions, fused flags and normalized scalar conversions. See NOTICE.
use super::{
    condition::Condition,
    condition_mir::FlagOp,
    scalar_mir::{Operand, ScalarOp, flip_sign},
    scalar_selection::Selection,
    value_facts::Alias,
    value_selection::unsupported,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{InstructionKind as I, TempId, Value},
    vocabulary::{Compare, Convert, Type, Unary},
};
struct Comparison {
    condition: Condition,
    op: FlagOp,
}
impl Selection<'_> {
    fn original_dying(&self, value: Value) -> Option<R> {
        if let Alias::Temporary(t) = self.facts.resolve(self.body, value) {
            self.placement
                .operand(t)
                .ok()
                .and_then(|op| self.dying(op, false))
        } else {
            None
        }
    }
    fn comparison(
        &mut self,
        pred: Compare,
        ty: Type,
        left: Value,
        right: Value,
        scratch: bool,
    ) -> Result<Comparison, Error> {
        super::memory::scalar(ty)?;
        let mut cc = Condition::from_predicate(pred);
        let mut a = self.operand(left, ty, R::R10)?;
        let mut b = self.operand(right, ty, R::R11)?;
        if matches!(b, Operand::Symbol(_)) {
            self.load(R::R11, b, Type::Ptr);
            b = Operand::Register(R::R11);
        }
        if matches!(a, Operand::Symbol(_)) {
            self.load(R::R10, a, Type::Ptr);
            a = Operand::Register(R::R10);
        }
        if matches!(a, Operand::Immediate(_)) && !matches!(b, Operand::Immediate(_)) {
            std::mem::swap(&mut a, &mut b);
            cc = cc.swapped();
        }
        if matches!(a, Operand::Immediate(_)) || a.is_memory() && b.is_memory() {
            self.load(R::R10, a, ty);
            a = Operand::Register(R::R10);
        }
        if scratch && matches!(b, Operand::Immediate(_)) {
            self.load(R::RDX, b, Type::I64);
            b = Operand::Register(R::RDX);
        }
        Ok(Comparison {
            condition: cc,
            op: FlagOp::Compare(ty, a, b),
        })
    }
    pub(super) fn compare(
        &mut self,
        temp: TempId,
        pred: Compare,
        ty: Type,
        left: Value,
        right: Value,
    ) -> Result<(), Error> {
        if ty.float_rank() != 0 {
            return self.float_compare(temp, pred, ty, left, right);
        }
        let comparison = self.comparison(
            pred,
            ty,
            left,
            right,
            self.facts.compare_scratch(self.body, self.pos),
        )?;
        let r = self.result(
            temp,
            [self.original_dying(left), self.original_dying(right)],
            0,
        );
        self.emit(ScalarOp::Condition(comparison.op));
        self.emit(ScalarOp::Condition(FlagOp::Set(comparison.condition, r)));
        self.emit(ScalarOp::Movzx(r));
        self.finish(temp, r)
    }
    pub(super) fn convert(
        &mut self,
        temp: TempId,
        op: Convert,
        to: Type,
        from: Type,
        value: Value,
    ) -> Result<(), Error> {
        if from.float_rank() != 0 || to.float_rank() != 0 {
            return self.float_convert(temp, op, to, from, value);
        }
        if from == Type::I128 || to == Type::I128 {
            return self.wide_convert(temp, op, to, from, value);
        }
        super::memory::scalar(to)?;
        super::memory::scalar(from)?;
        if !matches!(
            op,
            Convert::SignExtend | Convert::ZeroExtend | Convert::Truncate
        ) {
            return Err(unsupported());
        }
        let src = self.operand(value, from, R::R10)?;
        let r = self.result(temp, [self.original_dying(value), None], 0);
        if src.is_memory() && op == Convert::Truncate {
            self.load(r, src, to);
            if to == Type::I1 {
                self.normalize(r, to);
            }
            return self.finish(temp, r);
        }
        self.load(r, src, if src.is_memory() { from } else { Type::I64 });
        match op {
            Convert::SignExtend => {
                if from == Type::I1 {
                    self.emit(ScalarOp::Unary(Unary::Neg, Type::I64, r));
                } else if matches!(from, Type::U8 | Type::U16 | Type::U32) {
                    self.normalize(r, flip_sign(from));
                }
                if matches!(to, Type::U8 | Type::U16 | Type::U32 | Type::I1) {
                    self.normalize(r, to);
                }
            }
            Convert::ZeroExtend => {
                if matches!(from, Type::I8 | Type::I16 | Type::I32) {
                    self.normalize(r, flip_sign(from));
                }
                if bits(to) <= bits(from) {
                    self.normalize(r, to);
                }
            }
            _ => self.normalize(r, to),
        }
        self.finish(temp, r)
    }
    pub(super) fn fused_flags(&mut self, temp: TempId) -> Result<Condition, Error> {
        let def = self.body.temp(temp).definition.ok_or(Error::Limit)?;
        let cc = match self.body.instruction(def).kind {
            I::Compare(pred, ty, a, b) => {
                let comparison = self.comparison(pred, ty, a, b, false)?;
                self.emit(ScalarOp::Condition(comparison.op));
                comparison.condition
            }
            I::Unary(Unary::Not, ty, v) => {
                let src = self.operand(v, ty, R::R10)?;
                let mut src = src;
                if matches!(src, Operand::Immediate(_) | Operand::Symbol(_)) {
                    self.load(R::R10, src, ty);
                    src = Operand::Register(R::R10);
                }
                self.emit(ScalarOp::Condition(FlagOp::Compare(
                    ty,
                    src,
                    Operand::Immediate(0),
                )));
                Condition::Equal
            }
            _ => return Err(unsupported()),
        };
        Ok(cc)
    }
    fn select_flags(&mut self, value: Value) -> Result<(Condition, Option<bool>), Error> {
        let resolved = self.facts.resolve(self.body, value);
        if let Alias::Constant(Constant::Integer(bits)) = resolved {
            return Ok((Condition::NotEqual, Some(bits != 0)));
        }
        let ty = if let Alias::Temporary(t) = resolved {
            if self.facts.fused[t.index()] {
                let cc = self.fused_flags(t)?;
                return Ok((cc, None));
            }
            self.body.temp(t).ty.ok_or(Error::Limit)?
        } else {
            Type::Ptr
        };
        super::memory::scalar(ty)?;
        let source = self.operand(value, ty, R::R10)?;
        let width = if matches!(resolved,Alias::Temporary(t) if super::value_facts::Facts::known_boolean(self.body,t,0))
        {
            Type::U8
        } else {
            ty
        };
        let known = match source {
            Operand::Register(r) => {
                self.emit(ScalarOp::Condition(FlagOp::Test(width, r)));
                None
            }
            op if op.is_memory() => {
                self.emit(ScalarOp::Condition(FlagOp::Compare(
                    width,
                    op,
                    Operand::Immediate(0),
                )));
                None
            }
            Operand::Immediate(bits) => Some(bits != 0),
            _ => Some(matches!(source, Operand::Symbol(_))),
        };
        Ok((Condition::NotEqual, known))
    }
    fn select_source(&mut self, value: Value, ty: Type) -> Result<Operand, Error> {
        let op = self.operand(value, ty, R::R11)?;
        if op.register().is_some() {
            return Ok(op);
        }
        self.load(R::R11, op, if op.is_memory() { ty } else { Type::I64 });
        Ok(Operand::Register(R::R11))
    }
    pub(super) fn select_value(
        &mut self,
        temp: TempId,
        ty: Type,
        cond: Value,
        mut yes: Value,
        mut no: Value,
    ) -> Result<(), Error> {
        super::memory::scalar(ty)?;
        let (cc, known) = self.select_flags(cond)?;
        if let Some(taken) = known {
            let chosen = if taken { yes } else { no };
            yes = chosen;
            no = chosen;
        }
        let same = self.facts.same_value(yes, no);
        let ts = if same {
            Operand::Immediate(0)
        } else {
            self.select_source(yes, ty)?
        };
        let own = ts.register().filter(|&r| r != R::R11);
        let a = self.facts.resolve(self.body, yes);
        let b = self.facts.resolve(self.body, no);
        let operands = [a, b].map(|v| {
            if let Alias::Temporary(t) = v {
                self.placement.operand(t).unwrap_or(Operand::Immediate(0))
            } else {
                Operand::Immediate(0)
            }
        });
        let exclude = if ts.register() == Some(R::R11) {
            1 << R::R11.code()
        } else {
            0
        } | self.placement.parameter_mask(self.body, operands);
        let r = self.result(
            temp,
            [self.original_dying(no), self.original_dying(yes)],
            exclude,
        );
        if !same && Some(r) == own {
            let src = self.select_source(no, ty)?;
            self.emit(ScalarOp::Condition(FlagOp::Move(cc.invert(), r, src)));
            return self.finish(temp, r);
        }
        let source = self.operand(no, ty, r)?;
        self.load(r, source, if source.is_memory() { ty } else { Type::I64 });
        if !same {
            self.emit(ScalarOp::Condition(FlagOp::Move(cc, r, ts)));
        }
        self.finish(temp, r)
    }
}
fn bits(ty: Type) -> u32 {
    if ty == Type::Ptr { 64 } else { ty.bits() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{builder::FunctionBuilder, model::Program};
    #[test]
    fn equal_literals_retain_identity_for_conditional_move_selection() -> Result<(), Error> {
        for shared in [false, true] {
            let mut program = Program::default();
            let mut b =
                FunctionBuilder::new(&mut program, "f", &[("condition", Type::I64)], Type::I64)?;
            b.block("entry")?;
            let c = b.parameter(0)?;
            let yes = b.integer(7)?;
            let no = if shared { yes } else { b.integer(7)? };
            let value = b.value(I::Select(Type::I64, c, yes, no), "selected")?;
            b.effect(I::Return(Type::I64, Some(value)))?;
            b.finish()?;
            let f = &program.functions()[0];
            let symbol = program.find_symbol(f.name).ok_or(Error::Limit)?;
            let mir = super::super::value_selection::select(&program, symbol, f, 0)?;
            let count = mir
                .instructions
                .iter()
                .filter(|inst| {
                    matches!(
                        inst.op,
                        super::super::mir::Op::Scalar(ScalarOp::Condition(FlagOp::Move(..)))
                    )
                })
                .count();
            assert_eq!(count, usize::from(!shared));
        }
        Ok(())
    }
    #[test]
    fn compare_early_clobber_protects_a_live_rdx_parameter() -> Result<(), Error> {
        let mut program = Program::default();
        let mut b = FunctionBuilder::new(
            &mut program,
            "f",
            &[("a", Type::I64), ("b", Type::I64), ("d", Type::I8)],
            Type::I64,
        )?;
        b.block("entry")?;
        let d = b.parameter(2)?;
        let seven = b.integer(7)?;
        let value = b.value(I::Compare(Compare::Ne, Type::I8, d, seven), "condition")?;
        b.effect(I::Return(Type::I64, Some(value)))?;
        b.finish()?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let flow = super::super::control_flow::Flow::analyze(body)?;
        let slots = super::super::slot_analysis::analyze(body, &flow)?;
        let facts = super::super::value_facts::Facts::analyze(body, &slots, flow)?;
        assert_ne!(facts.intervals[2].forbid & (1 << R::RDX.code()), 0);
        Ok(())
    }
}
