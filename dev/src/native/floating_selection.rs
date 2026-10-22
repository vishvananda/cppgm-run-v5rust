//! Primitive floating values, memory results and ABI transport. See NOTICE.
use super::{
    float_transport::FloatCopy,
    floating_mir::{Arithmetic, FloatingOp},
    mir::{Instruction, Op},
    scalar_mir::Operand,
    scalar_selection::Selection,
    value_facts::Alias,
    x86::Register as R,
};
use crate::{
    lowir::{
        Error,
        literals::{self, FloatKind},
        model::{InstructionKind as I, TempId, Value},
        vocabulary::{Binary, Compare, Type, Unary},
    },
    types::floating::FloatingScanner,
};
impl Selection<'_> {
    pub(super) fn float_compare(
        &mut self,
        result: TempId,
        predicate: Compare,
        ty: Type,
        left: Value,
        right: Value,
    ) -> Result<(), Error> {
        let left = self.float_operand(left, ty)?;
        let right = self.float_operand(right, ty)?;
        let destination = self.result(result, [None, None], 0);
        self.floating(FloatingOp::Set {
            predicate,
            ty,
            destination,
            left,
            right,
        });
        self.finish(result, destination)
    }
    pub(super) fn float_branch(
        &mut self,
        result: TempId,
        yes: super::mir::BlockId,
        no: super::mir::BlockId,
    ) -> Result<bool, Error> {
        use super::condition::Condition as C;
        let pos = self.facts.definitions[result.index()].ok_or(Error::Limit)?;
        let I::Compare(predicate, ty, left, right) = self
            .body
            .instruction(self.facts.flow.instructions[pos])
            .kind
        else {
            return Ok(false);
        };
        if ty.float_rank() == 0 {
            return Ok(false);
        }
        let mut left = self.float_operand(left, ty)?;
        let mut right = self.float_operand(right, ty)?;
        if matches!(
            predicate,
            Compare::Lt | Compare::Le | Compare::Ugt | Compare::Uge
        ) {
            std::mem::swap(&mut left, &mut right);
        }
        self.floating(FloatingOp::Compare { ty, left, right });
        match predicate {
            Compare::Eq => {
                self.branch(C::Parity, no);
                self.branch(C::Equal, yes);
            }
            Compare::Ne => {
                self.branch(C::Parity, yes);
                self.branch(C::NotEqual, yes);
            }
            Compare::Gt | Compare::Lt => self.branch(C::Above, yes),
            Compare::Ge | Compare::Le => self.branch(C::AboveEqual, yes),
            Compare::Ule | Compare::Uge => {
                self.branch(C::Above, no);
                self.jump(yes);
                return Ok(true);
            }
            _ => {
                self.branch(C::AboveEqual, no);
                self.jump(yes);
                return Ok(true);
            }
        }
        self.jump(no);
        Ok(true)
    }
    pub(super) fn float_operand(&mut self, value: Value, ty: Type) -> Result<Operand, Error> {
        match self.facts.resolve(self.body, value) {
            Alias::Constant(c) => {
                let mut scanner = FloatingScanner::default();
                Ok(Operand::Immediate(literals::native_float(
                    &mut scanner,
                    c,
                    FloatKind::from_type(ty).ok_or(Error::Limit)?,
                )?))
            }
            Alias::Temporary(temp) => self.temp_operand(temp),
            Alias::Slot(slot) => Ok(Operand::Frame(self.placement.slot(slot)?)),
            Alias::Symbol(_) => Err(Error::Diagnostic(
                "floating value requires a numeric value or storage operand".into(),
            )),
        }
    }
    pub(super) fn float_copy(&mut self, ty: Type, destination: Operand, source: Operand) {
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::FloatCopy(FloatCopy {
                ty,
                destination,
                source,
            }),
            location: self.location,
        });
    }
    pub(super) fn floating(&mut self, operation: FloatingOp) {
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::Floating(operation),
            location: self.location,
        });
    }
    pub(super) fn float_destination(
        &mut self,
        result: TempId,
        ty: Type,
        prefs: [Option<R>; 2],
    ) -> Result<Operand, Error> {
        if ty == Type::F80 {
            Ok(Operand::Frame(self.placement.home(
                self.body,
                result,
                &mut self.mir,
            )?))
        } else {
            Ok(Operand::Register(self.result(result, prefs, 0)))
        }
    }
    pub(super) fn float_read(
        &mut self,
        result: TempId,
        ty: Type,
        value: Value,
    ) -> Result<(), Error> {
        let source = self.address(value, R::R10)?;
        let destination = self.float_destination(result, ty, [None, None])?;
        self.float_copy(ty, destination, source);
        if ty != Type::F80 {
            self.mark_volatile();
        }
        if let Some(r) = destination.register() {
            self.finish(result, r)?;
        }
        Ok(())
    }
    pub(super) fn float_write(
        &mut self,
        ty: Type,
        value: Value,
        address: Value,
    ) -> Result<(), Error> {
        if let Alias::Temporary(t) = self.facts.resolve(self.body, value)
            && self
                .body
                .temp(t)
                .ty
                .is_some_and(|own| own.float_rank() != 0 && own != ty)
        {
            let from = self.body.temp(t).ty.ok_or(Error::Limit)?;
            let source = self.temp_operand(t)?;
            let destination = self.address(address, R::R10)?;
            if ty == Type::F80 {
                self.width_conversion(from, ty, destination, source);
            } else {
                self.width_conversion(from, ty, Operand::Register(R::XMM14), source);
                self.float_copy(ty, destination, Operand::Register(R::XMM14));
                self.mark_volatile();
            }
            return Ok(());
        }
        let source = if ty == Type::F80 {
            self.aggregate_operand(value, ty)?
        } else if matches!(self.facts.resolve(self.body, value), Alias::Temporary(_)) {
            self.operand(value, ty, R::R11)?
        } else {
            self.float_operand(value, ty)?
        };
        let destination = self.address(address, R::R10)?;
        self.float_copy(ty, destination, source);
        self.mark_volatile();
        Ok(())
    }
    pub(super) fn float_value_copy(
        &mut self,
        result: TempId,
        ty: Type,
        value: Value,
    ) -> Result<(), Error> {
        let source = if ty == Type::F80 {
            self.aggregate_operand(value, ty)?
        } else {
            let own = if let Value::Temp(temp) = value {
                self.body.temp(temp).ty.ok_or(Error::Limit)?
            } else {
                Type::Ptr
            };
            self.operand(value, own, R::R10)?
        };
        if source.is_symbol() {
            return Err(Error::Diagnostic(
                "floating copy requires a numeric value or storage operand".into(),
            ));
        }
        let destination = self.float_destination(result, ty, [self.dying(source, false), None])?;
        if ty != Type::F80 && matches!(source, Operand::Immediate(_)) {
            self.load(R::R10, source, Type::I64);
            self.float_copy(ty, destination, Operand::Register(R::R10));
        } else {
            self.float_copy(ty, destination, source);
        }
        if let Some(r) = destination.register() {
            self.finish(result, r)?;
        }
        Ok(())
    }
    pub(super) fn float_binary(
        &mut self,
        result: TempId,
        ty: Type,
        operation: Binary,
        left: Value,
        right: Value,
    ) -> Result<(), Error> {
        if !matches!(
            operation,
            Binary::Add | Binary::Sub | Binary::Mul | Binary::Div
        ) {
            return Err(Error::Diagnostic(
                "unsupported floating binary operation".into(),
            ));
        }
        let left = self.float_operand(left, ty)?;
        let right = self.float_operand(right, ty)?;
        let prefs = [
            self.dying(left, false),
            if matches!(operation, Binary::Add | Binary::Mul) {
                self.dying(right, false)
            } else {
                None
            },
        ];
        let destination = self.float_destination(result, ty, prefs)?;
        self.floating(FloatingOp::Arithmetic {
            operation: Arithmetic::Binary(operation),
            ty,
            destination,
            left,
            right: Some(right),
        });
        if let Some(r) = destination.register() {
            self.finish(result, r)?;
        }
        Ok(())
    }
    pub(super) fn float_unary(
        &mut self,
        result: TempId,
        ty: Type,
        operation: Unary,
        value: Value,
    ) -> Result<(), Error> {
        if operation != Unary::Neg {
            return Err(Error::Diagnostic(
                "unsupported floating unary operation".into(),
            ));
        }
        let left = self.float_operand(value, ty)?;
        let destination = self.float_destination(result, ty, [self.dying(left, false), None])?;
        self.floating(FloatingOp::Arithmetic {
            operation: Arithmetic::Negate,
            ty,
            destination,
            left,
            right: None,
        });
        if let Some(r) = destination.register() {
            self.finish(result, r)?;
        }
        Ok(())
    }
    pub(super) fn float_return(&mut self, ty: Type, value: Value) -> Result<(), Error> {
        if let Alias::Temporary(t) = self.facts.resolve(self.body, value)
            && self
                .body
                .temp(t)
                .ty
                .is_some_and(|own| own.float_rank() != 0 && own != ty)
        {
            let from = self.body.temp(t).ty.ok_or(Error::Limit)?;
            if ty == Type::F80 {
                let home = Operand::Frame(self.placement.frame_alloc(16, 16)?);
                let source = self.temp_operand(t)?;
                self.width_conversion(from, ty, home, source);
                self.floating(FloatingOp::Return(home));
            } else {
                let source = self.temp_operand(t)?;
                self.width_conversion(from, ty, Operand::Register(R::XMM0), source);
                self.mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::Return(Some(R::XMM0)),
                    location: self.location,
                });
            }
            return Ok(());
        }
        let source = self.operand(value, ty, R::R10)?;
        if ty == Type::F80 {
            self.floating(FloatingOp::Return(source));
            return Ok(());
        }
        let reg = source.register().unwrap_or(R::XMM0);
        if source.register().is_none() {
            self.load(reg, source, ty);
        }
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::Return(Some(reg)),
            location: self.location,
        });
        Ok(())
    }
}
