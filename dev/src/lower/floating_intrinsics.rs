// Translated from lower/intrinsic_lowering.cpp. See NOTICE.
//! Floating classification expands from one evaluated operand, preserving instruction order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        literals::{Constant, FloatKind},
        model::{Facts, InstructionKind as I},
        vocabulary::{Binary, Compare, Convert, Type, Unary},
    },
    sema::lowering::{FloatingIntrinsic as Intrinsic, TypeId},
    syntax::{ast, storage::List},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn floating_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        kind: Intrinsic,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let arguments = self.tree.expr_lists.get(arguments);
        if kind.comparison() {
            let x = self.operand(arguments[0], Context::Operand)?;
            let y = self.operand(arguments[1], Context::Operand)?;
            return match kind {
                Intrinsic::IsLessGreater => {
                    let greater = self.floating_truth(Compare::Gt, x, y, source)?;
                    let less = self.floating_truth(Compare::Lt, x, y, source)?;
                    self.intrinsic_binary(Binary::Or, less, greater, source)
                }
                Intrinsic::IsUnordered => {
                    let yn = self.classify_nan(y, source)?;
                    let xn = self.classify_nan(x, source)?;
                    self.intrinsic_binary(Binary::Or, xn, yn, source)
                }
                _ => self.floating_truth(
                    match kind {
                        Intrinsic::IsGreater => Compare::Gt,
                        Intrinsic::IsGreaterEqual => Compare::Ge,
                        Intrinsic::IsLess => Compare::Lt,
                        Intrinsic::IsLessEqual => Compare::Le,
                        _ => {
                            return Err(Error::Semantic(crate::sema::Error::Invariant(
                                "floating comparison invariant",
                            )));
                        }
                    },
                    x,
                    y,
                    source,
                ),
            };
        }
        let operand = *arguments
            .last()
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "floating operand invariant",
            )))?;
        let x = self.operand(operand, Context::Operand)?;
        match kind {
            Intrinsic::IsFinite => self.classify_finite(x, source),
            Intrinsic::IsInf => self.classify_infinite(x, source),
            Intrinsic::IsNan => self.classify_nan(x, source),
            Intrinsic::IsNormal => self.classify_normal(x, source),
            Intrinsic::SignBit => self.floating_representation_bit(x, source, false),
            Intrinsic::IsInfSign => {
                let infinite = self.classify_infinite(x, source)?;
                let sign = self.floating_representation_bit(x, source, false)?;
                let negative = self.intrinsic_binary(Binary::And, infinite, sign, source)?;
                let twice = self.intrinsic_binary(Binary::Add, negative, negative, source)?;
                self.intrinsic_binary(Binary::Sub, infinite, twice, source)
            }
            Intrinsic::IsSignaling => {
                let quiet = self.floating_representation_bit(x, source, true)?;
                let one = self.intrinsic_integer(1, source)?;
                let signaling = self.intrinsic_binary(Binary::Xor, quiet, one, source)?;
                let nan = self.classify_nan(x, source)?;
                self.intrinsic_binary(Binary::And, nan, signaling, source)
            }
            Intrinsic::Classify => self.classify_codes(x, &arguments[..5], source),
            _ => Err(Error::Semantic(crate::sema::Error::Invariant(
                "floating classification invariant",
            ))),
        }
    }
    fn intrinsic_integer(&mut self, bits: u128, source: TypeId) -> Result<RValue, Error> {
        Ok(RValue {
            value: self.builder.integer(bits)?,
            ty: Type::I32,
            source,
            literal: false,
            bits: None,
        })
    }
    fn intrinsic_binary(
        &mut self,
        op: Binary,
        a: RValue,
        b: RValue,
        source: TypeId,
    ) -> Result<RValue, Error> {
        self.emit(
            I::Binary(op, Facts::default(), Type::I32, a.value, b.value),
            source,
        )
    }
    fn floating_truth(
        &mut self,
        op: Compare,
        x: RValue,
        y: RValue,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let value = self.emit(I::Compare(op, x.ty, x.value, y.value), source)?;
        self.emit(
            I::Convert(Convert::Truncate, Type::I32, Type::I64, value.value),
            source,
        )
    }
    fn classify_nan(&mut self, x: RValue, source: TypeId) -> Result<RValue, Error> {
        self.floating_truth(Compare::Ne, x, x, source)
    }
    fn classify_infinite(&mut self, x: RValue, source: TypeId) -> Result<RValue, Error> {
        let positive = self.floating_constant(x, false)?;
        let negative = self.emit(I::Unary(Unary::Neg, x.ty, positive.value), x.source)?;
        let above = self.floating_truth(Compare::Eq, x, positive, source)?;
        let below = self.floating_truth(Compare::Eq, x, negative, source)?;
        self.intrinsic_binary(Binary::Or, above, below, source)
    }
    fn classify_finite(&mut self, x: RValue, source: TypeId) -> Result<RValue, Error> {
        let one = self.intrinsic_integer(1, source)?;
        let nan = self.classify_nan(x, source)?;
        let not_nan = self.intrinsic_binary(Binary::Sub, one, nan, source)?;
        let infinite = self.classify_infinite(x, source)?;
        let not_infinite = self.intrinsic_binary(Binary::Sub, one, infinite, source)?;
        self.intrinsic_binary(Binary::And, not_nan, not_infinite, source)
    }
    fn classify_normal(&mut self, x: RValue, source: TypeId) -> Result<RValue, Error> {
        let positive = self.floating_constant(x, true)?;
        let negative = self.emit(I::Unary(Unary::Neg, x.ty, positive.value), x.source)?;
        let above = self.floating_truth(Compare::Ge, x, positive, source)?;
        let below = self.floating_truth(Compare::Le, x, negative, source)?;
        let large = self.intrinsic_binary(Binary::Or, above, below, source)?;
        let finite = self.classify_finite(x, source)?;
        self.intrinsic_binary(Binary::And, finite, large, source)
    }
    fn floating_constant(&mut self, x: RValue, least: bool) -> Result<RValue, Error> {
        let kind = FloatKind::from_type(x.ty).ok_or(Error::Semantic(
            crate::sema::Error::Invariant("floating storage invariant"),
        ))?;
        // Exact target representations: exponent one and its implicit (x87 explicit)
        // leading bit for the least normal value, all exponent bits for infinity.
        let bits = match (kind, least) {
            (FloatKind::F32, true) => 1 << 23,
            (FloatKind::F64, true) => 1 << 52,
            (FloatKind::F80, true) => (1 << 64) | (1 << 63),
            (FloatKind::F32, false) => 0x7f80_0000,
            (FloatKind::F64, false) => 0x7ff0_0000_0000_0000,
            (FloatKind::F80, false) => (0x7fff << 64) | (1 << 63),
        };
        Ok(RValue {
            value: self.builder.constant(Constant::Float(kind, bits))?,
            ty: x.ty,
            source: x.source,
            literal: false,
            bits: None,
        })
    }
    fn floating_representation_bit(
        &mut self,
        x: RValue,
        source: TypeId,
        quiet: bool,
    ) -> Result<RValue, Error> {
        let slot = self.generated_slot("floating_signbit", x.ty)?;
        self.effect(I::Store(Facts::memory(false, true), x.ty, x.value, slot))?;
        let mut address = self.emit(I::Addr(slot), source)?.value;
        let bits = if x.ty == Type::F32 {
            Type::I32
        } else if x.ty == Type::F80 && !quiet {
            Type::I16
        } else {
            Type::I64
        };
        if x.ty == Type::F80 && !quiet {
            let index = self.builder.integer(4)?;
            let size = self.builder.integer(2)?;
            let offset = self
                .emit(
                    I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                    source,
                )?
                .value;
            address = self
                .emit(
                    I::Index(
                        Type::I8,
                        Some(crate::lowir::vocabulary::Projection::ArrayElement),
                        address,
                        offset,
                    ),
                    source,
                )?
                .value;
        }
        let word = self.emit(I::Load(Facts::memory(false, true), bits, address), source)?;
        if !quiet {
            let zero = self.builder.integer(0)?;
            let truth = self.emit(I::Compare(Compare::Lt, bits, word.value, zero), source)?;
            return self.emit(
                I::Convert(Convert::Truncate, Type::I32, Type::I64, truth.value),
                source,
            );
        }
        let position = self.builder.integer(match x.ty {
            Type::F32 => 22,
            Type::F64 => 51,
            _ => 62,
        })?;
        let shifted = self.emit(
            I::Binary(Binary::UShr, Facts::default(), bits, word.value, position),
            source,
        )?;
        let one = self.builder.integer(1)?;
        let bit = self.emit(
            I::Binary(Binary::And, Facts::default(), bits, shifted.value, one),
            source,
        )?;
        if bits == Type::I32 {
            Ok(bit)
        } else {
            self.emit(
                I::Convert(Convert::Truncate, Type::I32, bits, bit.value),
                source,
            )
        }
    }
    fn classify_codes(
        &mut self,
        x: RValue,
        arguments: &[ast::ExprId],
        source: TypeId,
    ) -> Result<RValue, Error> {
        let [
            nan_code,
            infinite_code,
            normal_code,
            subnormal_code,
            zero_code,
        ] = arguments
        else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "classification code count",
            )));
        };
        let codes = [
            self.operand(*nan_code, Context::Argument)?,
            self.operand(*infinite_code, Context::Argument)?,
            self.operand(*normal_code, Context::Argument)?,
            self.operand(*subnormal_code, Context::Argument)?,
            self.operand(*zero_code, Context::Argument)?,
        ];
        let nan = self.classify_nan(x, source)?;
        let infinite = self.classify_infinite(x, source)?;
        let one = self.intrinsic_integer(1, source)?;
        let not_nan = self.intrinsic_binary(Binary::Sub, one, nan, source)?;
        let not_infinite = self.intrinsic_binary(Binary::Sub, one, infinite, source)?;
        let finite = self.intrinsic_binary(Binary::And, not_nan, not_infinite, source)?;
        let kind = FloatKind::from_type(x.ty).ok_or(Error::Semantic(
            crate::sema::Error::Invariant("floating zero invariant"),
        ))?;
        let zero = RValue {
            value: self.builder.constant(Constant::Float(kind, 0))?,
            ..x
        };
        let zero = self.floating_truth(Compare::Eq, x, zero, source)?;
        let normal = self.classify_normal(x, source)?;
        let not_zero = self.intrinsic_binary(Binary::Sub, one, zero, source)?;
        let not_normal = self.intrinsic_binary(Binary::Sub, one, normal, source)?;
        let small = self.intrinsic_binary(Binary::And, not_zero, not_normal, source)?;
        let subnormal = self.intrinsic_binary(Binary::And, finite, small, source)?;
        let mut sum = self.intrinsic_integer(0, source)?;
        for (code, truth) in codes
            .into_iter()
            .zip([nan, infinite, normal, subnormal, zero])
        {
            let weighted = self.intrinsic_binary(Binary::Mul, code, truth, source)?;
            sum = self.intrinsic_binary(Binary::Add, sum, weighted, source)?;
        }
        Ok(sum)
    }
}
