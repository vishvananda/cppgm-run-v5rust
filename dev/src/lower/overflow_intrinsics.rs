// Translated from lower/intrinsic_lowering.cpp. See NOTICE.
//! Exact overflow expansions, including frozen right-before-left instruction order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Binary as B, Compare as C, Convert, Type},
    },
    sema::lowering::{OverflowOperation as Op, TypeId, TypeView},
    syntax::{ast, storage::List},
};

#[derive(Clone, Copy)]
struct Operand {
    value: RValue,
    bits: u32,
    signed: bool,
}
#[derive(Clone, Copy)]
struct Target {
    pointer: Value,
    stored: Type,
    bits: u32,
    signed: bool,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn overflow_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        operation: Op,
    ) -> Result<RValue, Error> {
        let [left, right, pointer] = *self.tree.expr_lists.get(arguments) else {
            return Err(crate::sema::Error::Invariant("overflow argument count").into());
        };
        let source = self.graph.lower_expression(id)?.ty;
        let a = self.overflow_operand(left)?;
        let b = self.overflow_operand(right)?;
        let address = self.operand(pointer, Context::Argument)?;
        let pointer_type = self.graph.lower_expression(pointer)?.ty;
        let (TypeView::Pointer(result)
        | TypeView::Array {
            element: result, ..
        }) = self.graph.lower_type(pointer_type)?
        else {
            return Err(crate::sema::Error::Invariant("overflow result pointer").into());
        };
        let stored = types::storage(&*self.graph, result)?;
        let target = Target {
            pointer: address.value,
            stored,
            bits: stored.bits(),
            signed: self.graph.lower_signed_integral(result),
        };
        let same = matches!(
            (self.graph.lower_type(a.value.source)?, self.graph.lower_type(b.value.source)?, self.graph.lower_type(result)?),
            (TypeView::Scalar(a), TypeView::Scalar(b), TypeView::Scalar(r)) if a == r && b == r
        );
        let value = if same && (operation != Op::Multiply || target.bits == 32) && target.bits >= 32
        {
            self.overflow_same(operation, a.value, b.value, target)?
        } else if a.bits > 64 || b.bits > 64 || target.bits > 64 {
            self.overflow_wide(operation, a, b, target)?
        } else {
            self.overflow_mixed(operation, a, b, target)?
        };
        Ok(RValue { source, ..value })
    }

    fn overflow_operand(&mut self, id: ast::ExprId) -> Result<Operand, Error> {
        let value = self.operand(id, Context::Argument)?;
        Ok(Operand {
            bits: value.ty.bits(),
            signed: self.graph.lower_signed_integral(value.source),
            value,
        })
    }

    fn overflow_same(
        &mut self,
        operation: Op,
        a: RValue,
        b: RValue,
        target: Target,
    ) -> Result<RValue, Error> {
        let ty = a.ty;
        if operation == Op::Multiply {
            let widen = if target.signed {
                Convert::SignExtend
            } else {
                Convert::ZeroExtend
            };
            let wide_b = self.overflow_convert(widen, Type::I64, b)?;
            let wide_a = self.overflow_convert(widen, Type::I64, a)?;
            let wide = self.overflow_binary(B::Mul, Type::I64, wide_a, wide_b)?;
            let narrow = self.overflow_convert(Convert::Truncate, ty, wide)?;
            self.overflow_store(target, narrow)?;
            let extended = self.overflow_convert(widen, Type::I64, narrow)?;
            return self.overflow_compare(C::Ne, Type::I64, wide, extended);
        }
        let add = operation == Op::Add;
        let r = self.overflow_binary(if add { B::Add } else { B::Sub }, ty, a, b)?;
        self.overflow_store(target, r)?;
        if !target.signed {
            return if add {
                self.overflow_compare(C::Ult, ty, r, a)
            } else {
                self.overflow_compare(C::Ult, ty, a, b)
            };
        }
        let result_sign = self.overflow_binary(B::Xor, ty, if add { b } else { a }, r)?;
        let different = self.overflow_binary(B::Xor, ty, a, if add { r } else { b })?;
        let signs = self.overflow_binary(B::And, ty, different, result_sign)?;
        let zero = self.overflow_integer(ty, 0, a.source)?;
        self.overflow_compare(C::Lt, ty, signs, zero)
    }

    fn overflow_mixed(
        &mut self,
        operation: Op,
        a: Operand,
        b: Operand,
        target: Target,
    ) -> Result<RValue, Error> {
        let wide_b = self.overflow_widen(b)?;
        let wide_a = self.overflow_widen(a)?;
        let r = self.overflow_binary(overflow_binary(operation), Type::I128, wide_a, wide_b)?;
        let narrow = self.overflow_convert(Convert::Truncate, target.stored, r)?;
        self.overflow_store(target, narrow)?;
        let max = if target.signed {
            (1u64 << (target.bits - 1)) - 1
        } else {
            u64::MAX >> (64 - target.bits)
        };
        let max_value = self.overflow_integer(Type::I64, u128::from(max), a.value.source)?;
        let high = self.overflow_convert(Convert::ZeroExtend, Type::I128, max_value)?;
        if operation == Op::Multiply && !a.signed && !b.signed {
            return self.overflow_compare(C::Ugt, Type::I128, r, high);
        }
        let low = if target.signed {
            let min = self.overflow_integer(Type::I64, u128::from(!max), a.value.source)?;
            self.overflow_convert(Convert::SignExtend, Type::I128, min)?
        } else {
            self.overflow_integer(Type::I128, 0, a.value.source)?
        };
        let below = self.overflow_compare(C::Lt, Type::I128, r, low)?;
        let above = self.overflow_compare(C::Gt, Type::I128, r, high)?;
        self.overflow_binary(B::Or, Type::I64, above, below)
    }

    fn overflow_wide(
        &mut self,
        operation: Op,
        a: Operand,
        b: Operand,
        target: Target,
    ) -> Result<RValue, Error> {
        let ty = Type::I128;
        let zero = self.overflow_integer(ty, 0, a.value.source)?;
        let one = self.overflow_integer(ty, 1, a.value.source)?;
        let (magnitude_a, negative_a) = self.overflow_magnitude(a, zero)?;
        let (magnitude_b, negative_b) = self.overflow_magnitude(b, zero)?;
        let (negative, result, carry) = if operation == Op::Multiply {
            let negative = self.overflow_binary(B::Xor, ty, negative_a, negative_b)?;
            let shift = self.overflow_integer(ty, 64, a.value.source)?;
            let mask = self.overflow_integer(ty, u128::from(u64::MAX), a.value.source)?;
            let a1 = self.overflow_binary(B::UShr, ty, magnitude_a, shift)?;
            let a0 = self.overflow_binary(B::And, ty, magnitude_a, mask)?;
            let b1 = self.overflow_binary(B::UShr, ty, magnitude_b, shift)?;
            let b0 = self.overflow_binary(B::And, ty, magnitude_b, mask)?;
            let b_high = self.overflow_truth(C::Ne, b1, zero)?;
            let a_high = self.overflow_truth(C::Ne, a1, zero)?;
            let both_high = self.overflow_binary(B::And, ty, a_high, b_high)?;
            let a0_b1 = self.overflow_binary(B::Mul, ty, a0, b1)?;
            let a1_b0 = self.overflow_binary(B::Mul, ty, a1, b0)?;
            let cross = self.overflow_binary(B::Add, ty, a1_b0, a0_b1)?;
            let high = self.overflow_binary(B::UShr, ty, cross, shift)?;
            let cross_high = self.overflow_truth(C::Ne, high, zero)?;
            let low = self.overflow_binary(B::Mul, ty, a0, b0)?;
            let shifted = self.overflow_binary(B::Shl, ty, cross, shift)?;
            let result = self.overflow_binary(B::Add, ty, low, shifted)?;
            let low_carry = self.overflow_truth(C::Ult, result, low)?;
            let high_carry = self.overflow_binary(B::Or, ty, both_high, cross_high)?;
            let carry = self.overflow_binary(B::Or, ty, high_carry, low_carry)?;
            (negative, result, carry)
        } else {
            let sign_b = if operation == Op::Subtract {
                self.overflow_binary(B::Xor, ty, negative_b, one)?
            } else {
                negative_b
            };
            let alike = self.overflow_truth(C::Eq, negative_a, sign_b)?;
            let sum = self.overflow_binary(B::Add, ty, magnitude_a, magnitude_b)?;
            let a_larger = self.overflow_truth(C::Uge, magnitude_a, magnitude_b)?;
            let b_minus_a = self.overflow_binary(B::Sub, ty, magnitude_b, magnitude_a)?;
            let a_minus_b = self.overflow_binary(B::Sub, ty, magnitude_a, magnitude_b)?;
            let difference = self.overflow_select(a_larger, a_minus_b, b_minus_a, zero)?;
            let result = self.overflow_select(alike, sum, difference, zero)?;
            let sign = self.overflow_select(a_larger, negative_a, sign_b, zero)?;
            let negative = self.overflow_select(alike, negative_a, sign, zero)?;
            let sum_carry = self.overflow_truth(C::Ult, sum, magnitude_a)?;
            let carry = self.overflow_binary(B::And, ty, alike, sum_carry)?;
            (negative, result, carry)
        };
        let mask = self.overflow_binary(B::Sub, ty, zero, negative)?;
        let flipped = self.overflow_binary(B::Xor, ty, result, mask)?;
        let value = self.overflow_binary(B::Sub, ty, flipped, mask)?;
        let stored = if target.bits == 128 {
            value
        } else {
            self.overflow_convert(Convert::Truncate, target.stored, value)?
        };
        self.overflow_store(target, stored)?;
        let bits = target.bits - u32::from(target.signed);
        let limit = self.overflow_integer(ty, u128::MAX >> (128 - bits), a.value.source)?;
        let mut fits = self.overflow_truth(C::Ule, result, limit)?;
        if target.signed {
            let minimum = self.overflow_binary(B::Add, ty, limit, one)?;
            let minimum = self.overflow_truth(C::Eq, result, minimum)?;
            let negative_minimum = self.overflow_binary(B::And, ty, negative, minimum)?;
            fits = self.overflow_binary(B::Or, ty, fits, negative_minimum)?;
        } else {
            let is_zero = self.overflow_truth(C::Eq, result, zero)?;
            let positive = self.overflow_binary(B::Xor, ty, negative, one)?;
            let valid_sign = self.overflow_binary(B::Or, ty, positive, is_zero)?;
            fits = self.overflow_binary(B::And, ty, fits, valid_sign)?;
        }
        let no_carry = self.overflow_binary(B::Xor, ty, carry, one)?;
        fits = self.overflow_binary(B::And, ty, fits, no_carry)?;
        self.overflow_compare(C::Eq, ty, fits, zero)
    }

    fn overflow_magnitude(
        &mut self,
        operand: Operand,
        zero: RValue,
    ) -> Result<(RValue, RValue), Error> {
        let wide = if operand.bits == 128 {
            operand.value
        } else {
            self.overflow_widen(operand)?
        };
        if !operand.signed {
            return Ok((wide, zero));
        }
        let negative = self.overflow_truth(C::Lt, wide, zero)?;
        let mask = self.overflow_binary(B::Sub, Type::I128, zero, negative)?;
        let flipped = self.overflow_binary(B::Xor, Type::I128, wide, mask)?;
        let magnitude = self.overflow_binary(B::Sub, Type::I128, flipped, mask)?;
        Ok((magnitude, negative))
    }
    fn overflow_select(
        &mut self,
        condition: RValue,
        x: RValue,
        y: RValue,
        zero: RValue,
    ) -> Result<RValue, Error> {
        let mask = self.overflow_binary(B::Sub, Type::I128, zero, condition)?;
        let delta = self.overflow_binary(B::Xor, Type::I128, x, y)?;
        let selected = self.overflow_binary(B::And, Type::I128, delta, mask)?;
        self.overflow_binary(B::Xor, Type::I128, y, selected)
    }
    fn overflow_widen(&mut self, operand: Operand) -> Result<RValue, Error> {
        self.overflow_convert(
            if operand.signed {
                Convert::SignExtend
            } else {
                Convert::ZeroExtend
            },
            Type::I128,
            operand.value,
        )
    }
    fn overflow_convert(&mut self, op: Convert, ty: Type, value: RValue) -> Result<RValue, Error> {
        self.emit(I::Convert(op, ty, value.ty, value.value), value.source)
    }
    fn overflow_truth(&mut self, predicate: C, a: RValue, b: RValue) -> Result<RValue, Error> {
        let truth = self.overflow_compare(predicate, Type::I128, a, b)?;
        self.overflow_convert(Convert::ZeroExtend, Type::I128, truth)
    }
    fn overflow_compare(&mut self, op: C, ty: Type, a: RValue, b: RValue) -> Result<RValue, Error> {
        self.emit(I::Compare(op, ty, a.value, b.value), a.source)
    }
    fn overflow_binary(&mut self, op: B, ty: Type, a: RValue, b: RValue) -> Result<RValue, Error> {
        self.emit(
            I::Binary(op, Facts::default(), ty, a.value, b.value),
            a.source,
        )
    }
    fn overflow_integer(&mut self, ty: Type, bits: u128, source: TypeId) -> Result<RValue, Error> {
        Ok(RValue {
            value: self.builder.integer(bits)?,
            ty,
            source,
            literal: false,
            bits: None,
        })
    }
    fn overflow_store(&mut self, target: Target, value: RValue) -> Result<(), Error> {
        self.effect(I::Store(
            Facts::memory(
                false,
                !matches!(target.stored, Type::I8 | Type::U8)
                    && !self.untyped_places.contains(&target.pointer),
            ),
            target.stored,
            value.value,
            target.pointer,
        ))
    }
}
fn overflow_binary(operation: Op) -> B {
    match operation {
        Op::Add => B::Add,
        Op::Subtract => B::Sub,
        Op::Multiply => B::Mul,
    }
}
