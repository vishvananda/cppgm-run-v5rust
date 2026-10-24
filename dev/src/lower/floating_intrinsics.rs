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
        if matches!(kind, Intrinsic::Constant | Intrinsic::SignalingConstant) {
            let bits = self.graph.lower_builtin_floating(id)?;
            if kind == Intrinsic::SignalingConstant {
                self.discard(arguments[0])?;
            }
            let ty = super::types::storage(self.graph, source)?;
            let kind = FloatKind::from_type(ty)
                .ok_or(crate::sema::Error::Invariant("floating builtin storage"))?;
            return Ok(RValue {
                value: self.builder.constant(Constant::Float(kind, bits))?,
                ty,
                source,
                literal: false,
                bits: None,
            });
        }
        if kind.comparison() {
            let x = self.floating_operand(arguments[0])?;
            let y = self.floating_operand(arguments[1])?;
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
        let x = self.floating_operand(operand)?;
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
    fn floating_operand(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let value = self.operand(id, Context::Operand)?;
        if super::soft_float::kind(self.graph, value.source)?
            == Some(super::soft_float::Kind::BFloat)
        {
            self.bfloat_widened(value)
        } else {
            Ok(value)
        }
    }
    fn soft_operand_kind(value: RValue) -> Option<super::soft_float::Kind> {
        match value.ty {
            Type::Object { bytes: 2, .. } => Some(super::soft_float::Kind::Half),
            Type::Object { bytes: 16, .. } => Some(super::soft_float::Kind::Quad),
            _ => None,
        }
    }
    fn soft_magnitude_at_least(
        &mut self,
        x: RValue,
        least: bool,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let kind = Self::soft_operand_kind(x)
            .ok_or(crate::sema::Error::Invariant("soft magnitude kind"))?;
        let quad = kind == super::soft_float::Kind::Quad;
        let bits = match (quad, least) {
            (false, false) => 0x7c00,
            (false, true) => 0x0400,
            (true, false) => 0x7fff_u128 << 112,
            (true, true) => 1 << 112,
        };
        let positive = self.soft_constant(kind, bits, x.source)?;
        let negative =
            self.soft_constant(kind, bits | if quad { 1 << 127 } else { 1 << 15 }, x.source)?;
        let below = self.floating_truth(Compare::Le, x, negative, source)?;
        let above = self.floating_truth(Compare::Ge, x, positive, source)?;
        self.intrinsic_binary(Binary::Or, above, below, source)
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
        let value = if let Some(kind) = Self::soft_operand_kind(x) {
            self.soft_compare(op, kind, x, y, source)?
        } else {
            self.emit(I::Compare(op, x.ty, x.value, y.value), source)?
        };
        self.emit(
            I::Convert(Convert::Truncate, Type::I32, Type::I64, value.value),
            source,
        )
    }
    fn classify_nan(&mut self, x: RValue, source: TypeId) -> Result<RValue, Error> {
        self.floating_truth(Compare::Ne, x, x, source)
    }
    fn classify_infinite(&mut self, x: RValue, source: TypeId) -> Result<RValue, Error> {
        if Self::soft_operand_kind(x).is_some() {
            return self.soft_magnitude_at_least(x, false, source);
        }
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
        if Self::soft_operand_kind(x).is_some() {
            let large = self.soft_magnitude_at_least(x, true, source)?;
            let finite = self.classify_finite(x, source)?;
            return self.intrinsic_binary(Binary::And, finite, large, source);
        }
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
        self.effect(I::Store(
            Facts::memory(false, !matches!(x.ty, Type::Object { .. })),
            x.ty,
            x.value,
            slot,
        ))?;
        let mut address = self.emit(I::Addr(slot), source)?.value;
        let soft = Self::soft_operand_kind(x);
        let bits = if x.ty == Type::F32 {
            Type::I32
        } else if x.ty == Type::F80 && !quiet || soft == Some(super::soft_float::Kind::Half) {
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
        if soft == Some(super::soft_float::Kind::Quad) {
            let index = self.builder.integer(1)?;
            let size = self.builder.integer(8)?;
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
            Type::Object { bytes: 2, .. } => 9,
            Type::Object { .. } => 47,
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
                I::Convert(
                    if bits == Type::I16 {
                        Convert::ZeroExtend
                    } else {
                        Convert::Truncate
                    },
                    Type::I32,
                    bits,
                    bit.value,
                ),
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
        let zero = if let Some(kind) = Self::soft_operand_kind(x) {
            self.soft_constant(kind, 0, x.source)?
        } else {
            let kind = FloatKind::from_type(x.ty)
                .ok_or(crate::sema::Error::Invariant("floating zero invariant"))?;
            RValue {
                value: self.builder.constant(Constant::Float(kind, 0))?,
                ..x
            }
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
