//! Frozen integer divide forms, fixed-register demands and selection. See NOTICE.
use super::{
    scalar_mir::{Operand, ScalarOp, flip_sign, signed},
    scalar_selection::Selection,
    value_facts::{Alias, Facts},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, Facts as Flags, TempId, Value},
    vocabulary::{Binary, Type},
};

#[derive(Clone, Copy)]
pub(super) enum Form {
    Hardware,
    UnsignedShift(u64),
    SignedShift(i64),
    Exact(i64),
    UnsignedMagic(i64),
    SignedMagic(i64),
}
impl Form {
    pub(super) fn at(
        facts: &Facts,
        body: &Body,
        op: Binary,
        flags: Flags,
        ty: Type,
        right: Value,
    ) -> Self {
        let Alias::Constant(Constant::Integer(bits)) = facts.resolve(body, right) else {
            return Self::Hardware;
        };
        let bytes = bits.to_le_bytes();
        let mut word = [0; 8];
        word.copy_from_slice(&bytes[..8]);
        let low = u64::from_le_bytes(word);
        let unsigned = matches!(op, Binary::UDiv | Binary::UMod);
        if unsigned && low.is_power_of_two() && low.cast_signed() > 0 {
            return Self::UnsignedShift(low);
        }
        if facts.level == 0 || ty == Type::I1 {
            return Self::Hardware;
        }
        let width = ty.bits();
        let mut bits = low;
        if width != 0 && width < 64 {
            let mask = (1_u64 << width) - 1;
            bits &= mask;
            if !unsigned && bits >> (width - 1) != 0 {
                bits |= !mask;
            }
        }
        let divisor = bits.cast_signed();
        if unsigned {
            return if bits <= 1 {
                Self::Hardware
            } else if bits.is_power_of_two() {
                Self::UnsignedShift(bits)
            } else {
                Self::UnsignedMagic(divisor)
            };
        }
        if matches!(divisor, 0 | 1 | -1 | i64::MIN) {
            return Self::Hardware;
        }
        if flags.exact() && op == Binary::Div {
            return Self::Exact(divisor);
        }
        if divisor.unsigned_abs().is_power_of_two() {
            Self::SignedShift(divisor)
        } else {
            Self::SignedMagic(divisor)
        }
    }
    pub(super) fn clobbers(self) -> u32 {
        match self {
            Self::Hardware | Self::UnsignedMagic(_) | Self::SignedMagic(_) => {
                (1 << R::RAX.code()) | (1 << R::RDX.code())
            }
            Self::UnsignedShift(_) | Self::SignedShift(_) | Self::Exact(_) => 0,
        }
    }
}
fn narrow(ty: Type) -> bool {
    matches!(
        ty,
        Type::I1 | Type::I8 | Type::U8 | Type::I16 | Type::U16 | Type::I32 | Type::U32
    )
}
impl Selection<'_> {
    pub(super) fn divide(
        &mut self,
        temp: TempId,
        op: Binary,
        flags: Flags,
        ty: Type,
        left: Value,
        right: Value,
    ) -> Result<(), Error> {
        let remainder = matches!(op, Binary::Mod | Binary::UMod);
        let form = Form::at(&self.facts, self.body, op, flags, ty, right);
        if matches!(
            form,
            Form::SignedShift(_) | Form::Exact(_) | Form::UnsignedMagic(_) | Form::SignedMagic(_)
        ) {
            return self.constant_divide(temp, op, ty, left, form);
        }
        let a = self.operand(left, ty, R::R10)?;
        if let Form::UnsignedShift(divisor) = form {
            let r = self.result(temp, [self.dying(a, false), None], 0);
            self.load(r, a, ty);
            if signed(ty) && ty != Type::I64 {
                self.normalize(r, flip_sign(ty));
            }
            if remainder {
                self.emit(ScalarOp::Binary(
                    Binary::And,
                    Type::I64,
                    r,
                    Operand::Immediate(u128::from(divisor - 1)),
                ));
            } else if divisor != 1 {
                self.emit(ScalarOp::Shift(
                    Binary::UShr,
                    r,
                    Operand::Immediate(u128::from(divisor.trailing_zeros())),
                ));
            }
            if signed(ty) && ty != Type::I64 {
                self.normalize(r, ty);
            }
            return self.finish(temp, r);
        }
        let unsigned = matches!(op, Binary::UDiv | Binary::UMod);
        let convert = narrow(ty) && unsigned == signed(ty);
        let mut b = self.operand(right, ty, R::R11)?;
        if matches!(b, Operand::Immediate(_))
            || b.is_symbol()
            || b.is_memory() && narrow(ty)
            || convert && b.register().is_some()
        {
            self.load(R::R11, b, ty);
            b = Operand::Register(R::R11);
            if convert {
                self.normalize(R::R11, flip_sign(ty));
            }
        }
        self.load(R::RAX, a, if a.is_memory() { ty } else { Type::I64 });
        if convert {
            self.normalize(R::RAX, flip_sign(ty));
        }
        if unsigned {
            self.load(R::RDX, Operand::Immediate(0), Type::I64);
        } else {
            self.emit(ScalarOp::SignDividend);
        }
        self.emit(ScalarOp::Divide {
            unsigned,
            source: b,
        });
        let from = if remainder { R::RDX } else { R::RAX };
        let r = self.result(temp, [Some(from), None], 0);
        self.load(r, Operand::Register(from), Type::I64);
        if narrow(ty) && (convert || !remainder) {
            self.normalize(r, ty);
        }
        self.finish(temp, r)
    }
}

#[cfg(test)]
mod tests {
    use super::super::mir::Op;
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Facts as Flags, InstructionKind as I, Program},
    };
    #[test]
    fn live_third_parameter_is_relocated_before_hardware_division() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(
            &mut program,
            "function",
            &[
                ("dividend", Type::I64),
                ("divisor", Type::I64),
                ("scale", Type::I64),
            ],
            Type::I64,
        )?;
        builder.block("entry")?;
        let dividend = builder.parameter(0)?;
        let divisor = builder.parameter(1)?;
        let scale = builder.parameter(2)?;
        let quotient = builder.value(
            I::Binary(Binary::Div, Flags::default(), Type::I64, dividend, divisor),
            "quotient",
        )?;
        let result = builder.value(
            I::Binary(Binary::Mul, Flags::default(), Type::I64, scale, quotient),
            "result",
        )?;
        builder.effect(I::Return(Type::I64, Some(result)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let mir = super::super::value_selection::select(
            &program,
            program.find_symbol(function.name).ok_or(Error::Limit)?,
            function,
            0,
        )?;
        let (index, relocated) = mir
            .instructions
            .iter()
            .enumerate()
            .find_map(|(i, inst)| {
                if let Op::Scalar(ScalarOp::Move(to, Operand::Register(R::RDX))) = inst.op {
                    Some((i, to))
                } else {
                    None
                }
            })
            .ok_or(Error::Limit)?;
        assert_ne!(relocated, R::RAX);
        assert_ne!(relocated, R::RDX);
        assert!(mir.instructions[index + 1..].iter().any(|inst| matches!(
            inst.op,
            Op::Scalar(ScalarOp::Divide {
                unsigned: false,
                ..
            })
        )));
        assert!(mir.instructions.iter().any(|inst| matches!(inst.op, Op::Scalar(ScalarOp::Binary(Binary::Mul, _, r, _)) if r == relocated)));
        Ok(())
    }
    #[test]
    fn hardware_dividend_from_stack_has_a_typed_load() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(
            &mut program,
            "function",
            &[
                ("a", Type::I64),
                ("builder", Type::I64),
                ("c", Type::I64),
                ("d", Type::I64),
                ("e", Type::I64),
                ("function", Type::I64),
                ("dividend", Type::I16),
            ],
            Type::I16,
        )?;
        builder.block("entry")?;
        let dividend = builder.parameter(6)?;
        let divisor = builder.integer(3)?;
        let quotient = builder.value(
            I::Binary(Binary::Div, Flags::default(), Type::I16, dividend, divisor),
            "quotient",
        )?;
        builder.effect(I::Return(Type::I16, Some(quotient)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let mir = super::super::value_selection::select(
            &program,
            program.find_symbol(function.name).ok_or(Error::Limit)?,
            function,
            0,
        )?;
        assert!(mir.instructions.iter().any(|inst| matches!(inst.op, Op::Scalar(ScalarOp::Load(R::RAX, Type::I16, Operand::Frame(d))) if d < 0)));
        assert!(!mir.instructions.iter().any(|inst| matches!(
            inst.op,
            Op::Scalar(ScalarOp::Move(R::RAX, Operand::Frame(_)))
        )));
        Ok(())
    }
}
