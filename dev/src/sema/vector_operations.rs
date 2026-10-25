// Translated from sema/analyzer_expressions.cpp. See NOTICE.
//! Lane operators retain the vector identity and record scalar splat conversions once.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::ScopeId,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast, types::FundamentalType as F};
use std::cmp::Ordering;

fn integral_only(op: P) -> bool {
    matches!(
        op,
        P::Remainder
            | P::ShiftLeft
            | P::ShiftRight
            | P::BitAnd
            | P::BitOr
            | P::Xor
            | P::RemainderAssign
            | P::ShiftLeftAssign
            | P::ShiftRightAssign
            | P::AndAssign
            | P::OrAssign
            | P::XorAssign
    )
}
fn integer_rank(scalar: Scalar) -> u8 {
    match scalar {
        Scalar::Bool => 1,
        Scalar::SignedChar | Scalar::UnsignedChar | Scalar::Basic(F::Char) => 2,
        Scalar::Short | Scalar::UnsignedShort | Scalar::Basic(F::Char16T) => 3,
        Scalar::Basic(F::Int | F::UnsignedInt | F::Char32T | F::WcharT) => 4,
        Scalar::Basic(F::LongInt | F::UnsignedLongInt) => 5,
        Scalar::Basic(F::LongLongInt | F::UnsignedLongLongInt) => 6,
        _ => 7,
    }
}
impl Analyzer<'_> {
    pub(super) fn vector_shape(&self, ty: TypeId) -> Option<(TypeId, u64)> {
        match self.graph.types.get(ty).kind {
            TypeKind::Vector {
                element,
                bound,
                operand: None,
                ..
            } => Some((element, bound)),
            _ => None,
        }
    }
    pub(super) fn vector_unary(
        &mut self,
        id: ast::ExprId,
        op: P,
        operand: ast::ExprId,
        ty: TypeId,
    ) -> Result<ExprFacts, Error> {
        let (lane, _) = self
            .vector_shape(ty)
            .ok_or(Error::Invariant("vector unary type"))?;
        if op == P::Complement && !self.is_integral_operand(lane) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "~ requires an integral operand",
            ));
        }
        let result = self.bare_type(ty)?;
        self.convert_operand_scalar(operand, result, false)?;
        self.new_expression_facts(result, V::Prvalue)
    }
    pub(super) fn vector_binary(
        &mut self,
        id: ast::ExprId,
        op: P,
        operands: [ast::ExprId; 2],
        types: [TypeId; 2],
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let [left, right] = operands;
        let [a, b] = types;
        let left_vector = self.vector_shape(a).is_some();
        let vector = self.bare_type(if left_vector { a } else { b })?;
        let other = self.bare_type(if left_vector { b } else { a })?;
        let (lane, bound) = self
            .vector_shape(vector)
            .ok_or(Error::Invariant("vector binary type"))?;
        let comparison = matches!(
            op,
            P::Equal | P::NotEqual | P::Less | P::LessEqual | P::Greater | P::GreaterEqual
        );
        let location = self.tree.expressions.get(id).location;
        if other != vector
            && (matches!(self.graph.types.get(other).kind, TypeKind::Vector { .. })
                || !self.is_arithmetic_operand(other))
        {
            return Err(Error::at(
                location,
                if comparison {
                    "comparison of vectors of different types"
                } else {
                    "arithmetic on vectors of different types"
                },
            ));
        }
        if !comparison
            && (integral_only(op) && !self.is_integral_operand(lane)
                || self.graph.types.get(lane).kind == TypeKind::Scalar(Scalar::Bool))
        {
            return Err(Error::at(location, "invalid operands to a binary operator"));
        }
        if other != vector
            && !self.scalar_splats(if left_vector { right } else { left }, other, lane, scope)?
        {
            return Err(Error::at(
                location,
                "a scalar operand that does not convert to the vector's lanes without truncation",
            ));
        }
        self.convert_operand_scalar(
            left,
            if left_vector || other == vector {
                vector
            } else {
                lane
            },
            false,
        )?;
        self.convert_operand_scalar(
            right,
            if !left_vector || other == vector {
                vector
            } else {
                lane
            },
            false,
        )?;
        let result = if comparison {
            let scalar = if self.graph.types.get(lane).kind == TypeKind::Scalar(Scalar::Bool) {
                Scalar::Bool
            } else {
                match self
                    .type_size(lane, self.tree.expressions.get(id).location)?
                    .0
                {
                    1 => Scalar::SignedChar,
                    2 => Scalar::Short,
                    4 => Scalar::Basic(F::Int),
                    8 => Scalar::Basic(F::LongInt),
                    _ => Scalar::Basic(F::Int128),
                }
            };
            let element = self.graph.types.scalar(scalar)?;
            self.graph.types.intern(
                TypeKind::Vector {
                    element,
                    bound,
                    operand: None,
                    bytes: false,
                },
                Cv::default(),
            )?
        } else {
            vector
        };
        self.new_expression_facts(result, V::Prvalue)
    }
    pub(super) fn vector_compound(
        &mut self,
        id: ast::ExprId,
        op: P,
        right: ast::ExprId,
        vector: TypeId,
        other: TypeId,
    ) -> Result<(), Error> {
        let (lane, _) = self
            .vector_shape(vector)
            .ok_or(Error::Invariant("vector compound type"))?;
        let other = self.bare_type(other)?;
        let scope = self.graph.expression_scopes[self.graph.expression_facts.root(right).index()]
            .ok_or(Error::Invariant("vector compound scope"))?;
        if integral_only(op) && !self.is_integral_operand(lane)
            || self.graph.types.get(lane).kind == TypeKind::Scalar(Scalar::Bool)
            || other != vector
                && (matches!(self.graph.types.get(other).kind, TypeKind::Vector { .. })
                    || !self.is_arithmetic_operand(other)
                    || !self.scalar_splats(right, other, lane, scope)?)
        {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "invalid operands to a compound assignment",
            ));
        }
        self.convert_operand_scalar(right, if other == vector { vector } else { lane }, false)
    }
    pub(super) fn vector_subscript(
        &mut self,
        base: ExprFacts,
        index: ast::ExprId,
        index_type: TypeId,
    ) -> Result<ExprFacts, Error> {
        let (lane, _) = self
            .vector_shape(base.ty)
            .ok_or(Error::Invariant("vector subscript type"))?;
        let promoted = self.promoted_type(index_type)?;
        self.convert_operand_scalar(index, promoted, false)?;
        let bits = self.graph.types.get(lane).kind == TypeKind::Scalar(Scalar::Bool);
        let lane = if bits {
            lane
        } else {
            self.graph
                .types
                .qualify(lane, self.graph.types.get(base.ty).cv)?
        };
        self.new_expression_facts(
            lane,
            if base.category == V::Lvalue && !bits {
                V::Lvalue
            } else {
                V::Prvalue
            },
        )
    }
    fn scalar_splats(
        &mut self,
        expression: ast::ExprId,
        ty: TypeId,
        lane: TypeId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let (TypeKind::Scalar(scalar), TypeKind::Scalar(element)) = (
            self.graph.types.get(ty).kind.clone(),
            self.graph.types.get(lane).kind.clone(),
        ) else {
            return Ok(!matches!(self.graph.types.get(ty).kind, TypeKind::Enum(_)));
        };
        if !self.semantic() {
            return Ok(true);
        }
        let lk = self.integral_kind(lane);
        let sk = self.integral_kind(ty);
        if let (Some(lk), Some(sk)) = (lk, sk) {
            let ar = integer_rank(element);
            let br = integer_rank(scalar);
            let order = if lk.signed == sk.signed {
                ar.cmp(&br)
            } else if lk.signed {
                if br >= ar {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            } else if ar >= br {
                Ordering::Greater
            } else {
                Ordering::Less
            };
            if order == Ordering::Equal {
                return Ok(true);
            }
            let Some(value) = self.constant_value(expression, scope, false)? else {
                return Ok(order == Ordering::Greater);
            };
            let bits = if sk.signed && value.bits.cast_signed() < 0 {
                1 + 64 - (!value.bits).leading_zeros()
            } else {
                64 - value.bits.leading_zeros()
            };
            return Ok(lk.bits >= bits || order != Ordering::Less && sk.signed == lk.signed);
        }
        if lk.is_some() {
            return Ok(self
                .type_size(lane, self.tree.expressions.get(expression).location)?
                .0
                == self
                    .type_size(ty, self.tree.expressions.get(expression).location)?
                    .0);
        }
        let Scalar::Basic(element) = element else {
            return Err(Error::Invariant("floating vector lane"));
        };
        let precision = element
            .floating_precision()
            .ok_or(Error::Invariant("floating lane precision"))?;
        let exact = |value: crate::types::float_value::BinaryFloat| {
            use crate::types::float_value::Format;
            let format = match precision {
                24 => Format::Single,
                53 => Format::Double,
                _ => Format::Half,
            };
            precision >= 64
                || value.is_nan()
                || value
                    .rounded(format)
                    .is_some_and(|rounded| rounded.compare(value) == Some(Ordering::Equal))
        };
        if let Some(sk) = sk {
            return Ok(
                if let Some(value) = self.constant_value(expression, scope, false)? {
                    exact(crate::types::float_value::BinaryFloat::integer(
                        value.bits, sk.signed,
                    ))
                } else {
                    sk.bits <= precision
                },
            );
        }
        match self.evaluate_floating(expression, scope) {
            Ok(value) => Ok(exact(value.value)),
            Err(Error::NonConstant { .. }) => Ok(
                matches!(scalar, Scalar::Basic(f) if f.floating_rank() <= element.floating_rank()),
            ),
            Err(error) => Err(error),
        }
    }
}
