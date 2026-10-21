//! Memory-result i128 arithmetic and fused comparison selection. See NOTICE.
use super::{
    mir::{BlockId, Instruction, Op},
    scalar_mir::Operand,
    scalar_selection::Selection,
    value_facts::Alias,
    wide_compare::{Comparison, Destination},
    wide_mir::{Arithmetic, Operation},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{InstructionKind as I, TempId, Value},
    vocabulary::{Binary, Compare, Type, Unary},
};
impl Selection<'_> {
    pub(super) fn wide_operand(&mut self, value: Value, scratch: R) -> Result<Operand, Error> {
        if let Alias::Temporary(temp) = self.facts.resolve(self.body, value) {
            let own = self.body.temp(temp).ty.ok_or(Error::Limit)?;
            if (own.bits() != 0 && own != Type::I128) || own == Type::Ptr {
                return self.operand(value, Type::I128, scratch);
            }
        }
        self.aggregate_operand(value, Type::I128)
    }
    pub(super) fn wide_arithmetic(
        &mut self,
        temp: TempId,
        operation: Operation,
        left: Value,
        right: Option<Value>,
    ) -> Result<(), Error> {
        let left = self.wide_operand(left, R::R10)?;
        let right = if let Some(value) = right {
            if matches!(
                operation,
                Operation::Binary(Binary::Shl | Binary::Shr | Binary::UShr)
            ) {
                if let Alias::Constant(Constant::Integer(bits)) =
                    self.facts.resolve(self.body, value)
                {
                    Some(Operand::Immediate(bits & 127))
                } else {
                    let ty = self.value_type(value)?;
                    let count = if ty == Type::I128 {
                        self.aggregate_operand(value, ty)?
                    } else {
                        self.operand(value, ty, R::R11)?
                    };
                    self.load(R::RCX, count, if ty == Type::I128 { Type::I64 } else { ty });
                    Some(Operand::Register(R::RCX))
                }
            } else {
                Some(self.wide_operand(value, R::R11)?)
            }
        } else {
            None
        };
        let destination = Operand::Frame(self.placement.home(self.body, temp, &mut self.mir)?);
        self.mir.instructions.push(Instruction {
            op: Op::Wide(Arithmetic {
                operation,
                destination,
                left,
                right,
            }),
            location: self.location,
        });
        Ok(())
    }
    fn value_type(&self, value: Value) -> Result<Type, Error> {
        match value {
            Value::Temp(t) => self.body.temp(t).ty.ok_or(Error::Limit),
            _ => Ok(Type::Ptr),
        }
    }
    pub(super) fn wide_unary(
        &mut self,
        temp: TempId,
        op: Unary,
        value: Value,
    ) -> Result<(), Error> {
        let operation = match op {
            Unary::Neg => Operation::Negate,
            Unary::BitNot => Operation::Complement,
            _ => return Err(Error::Diagnostic("unsupported i128 unary operation".into())),
        };
        self.wide_arithmetic(temp, operation, value, None)
    }
    pub(super) fn wide_compare(
        &mut self,
        temp: TempId,
        predicate: Compare,
        left: Value,
        right: Value,
    ) -> Result<(), Error> {
        let left = self.wide_operand(left, R::R10)?;
        let right = self.wide_operand(right, R::R11)?;
        let dst = self.result(temp, [None, None], 0);
        self.mir.instructions.push(Instruction {
            op: Op::WideCompare(Comparison {
                predicate,
                destination: Destination::Register(dst),
                left,
                right,
            }),
            location: self.location,
        });
        self.finish(temp, dst)
    }
    pub(super) fn wide_branch_operands(
        &mut self,
        predicate: Compare,
        left: Operand,
        right: Operand,
        yes: BlockId,
        no: BlockId,
    ) {
        self.mir.instructions.push(Instruction {
            op: Op::WideCompare(Comparison {
                predicate,
                destination: Destination::Branch { yes, no },
                left,
                right,
            }),
            location: self.location,
        });
    }
    pub(super) fn wide_condition(
        &mut self,
        value: Value,
        yes: BlockId,
        no: BlockId,
    ) -> Result<(), Error> {
        let source = self.wide_operand(value, R::R10)?;
        if let Operand::Immediate(bits) = source {
            self.jump(if bits != 0 { yes } else { no });
        } else {
            self.wide_branch_operands(Compare::Ne, source, Operand::Immediate(0), yes, no);
        }
        Ok(())
    }
    pub(super) fn wide_branch(
        &mut self,
        temp: TempId,
        yes: BlockId,
        no: BlockId,
    ) -> Result<bool, Error> {
        let def = self.body.temp(temp).definition.ok_or(Error::Limit)?;
        let I::Compare(predicate, Type::I128, left, right) = self.body.instruction(def).kind else {
            return Ok(false);
        };
        let left = self.wide_operand(left, R::R10)?;
        let right = self.wide_operand(right, R::R11)?;
        self.wide_branch_operands(predicate, left, right, yes, no);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Facts, Program},
    };
    #[test]
    fn bare_slot_shift_counts_use_the_native_pointer_view() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "shift", &[], Type::I128)?;
        let count = builder.slot("count", Type::I8)?;
        builder.block("entry")?;
        let one = builder.integer(1)?;
        builder.effect(I::Store(Facts::default(), Type::I8, one, count))?;
        let value = builder.value(
            I::Binary(Binary::Shl, Facts::default(), Type::I128, one, count),
            "result",
        )?;
        builder.effect(I::Return(Type::I128, Some(value)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let mir = super::super::value_selection::select(&program, symbol, function, 0)?;
        assert!(mir.instructions.iter().any(|i| matches!(
            i.op,
            Op::Scalar(super::super::scalar_mir::ScalarOp::Load(
                R::RCX,
                Type::Ptr,
                _
            ))
        )));
        Ok(())
    }
    #[test]
    fn scalar_stack_sources_are_loaded_at_their_recorded_width() -> Result<(), Error> {
        let mut program = Program::default();
        let parameters = [
            ("a", Type::I8),
            ("b", Type::I8),
            ("c", Type::I8),
            ("d", Type::I8),
            ("e", Type::I8),
            ("f", Type::I8),
            ("g", Type::I8),
        ];
        let mut builder = FunctionBuilder::new(&mut program, "add", &parameters, Type::I128)?;
        builder.block("entry")?;
        let input = builder.parameter(6)?;
        let one = builder.integer(1)?;
        let value = builder.value(
            I::Binary(Binary::Add, Facts::default(), Type::I128, input, one),
            "result",
        )?;
        builder.effect(I::Return(Type::I128, Some(value)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let mir = super::super::value_selection::select(&program, symbol, function, 0)?;
        assert!(mir.instructions.iter().any(|i| matches!(
            i.op,
            Op::Scalar(super::super::scalar_mir::ScalarOp::Load(
                R::R10,
                Type::I8,
                _
            ))
        )));
        assert!(mir.instructions.iter().any(|i| matches!(
            i.op,
            Op::Wide(Arithmetic {
                left: Operand::Register(R::R10),
                ..
            })
        )));
        Ok(())
    }
}
