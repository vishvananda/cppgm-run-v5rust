// Translated from analyzer_builtins.cpp::floating_builtin and lex/gnu_builtins.cpp. See NOTICE.
//! Type-generic floating operations retain their selected operation on the source `ExprId`.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    types::{Scalar, TypeKind},
};
use crate::{
    lex::identifiers::NameId,
    syntax::{ast, cursor::Cursor, storage::List},
    types::FundamentalType as F,
};
use std::cmp::Ordering::{Equal, Greater, Less};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FloatingIntrinsic {
    IsFinite,
    IsInf,
    IsNan,
    IsNormal,
    Classify,
    SignBit,
    IsInfSign,
    IsSignaling,
    IsGreater,
    IsGreaterEqual,
    IsLess,
    IsLessEqual,
    IsLessGreater,
    IsUnordered,
}
impl FloatingIntrinsic {
    pub(crate) fn comparison(self) -> bool {
        matches!(
            self,
            Self::IsGreater
                | Self::IsGreaterEqual
                | Self::IsLess
                | Self::IsLessEqual
                | Self::IsLessGreater
                | Self::IsUnordered
        )
    }
}
pub(super) fn names(cursor: &mut Cursor<'_>) -> Result<Vec<(NameId, FloatingIntrinsic)>, Error> {
    use FloatingIntrinsic as I;
    [
        ("__builtin_isfinite", I::IsFinite),
        ("__builtin_isinf", I::IsInf),
        ("__builtin_isnan", I::IsNan),
        ("__builtin_isnormal", I::IsNormal),
        ("__builtin_fpclassify", I::Classify),
        ("__builtin_signbit", I::SignBit),
        ("__builtin_isinf_sign", I::IsInfSign),
        ("__builtin_issignaling", I::IsSignaling),
        ("__builtin_isgreater", I::IsGreater),
        ("__builtin_isgreaterequal", I::IsGreaterEqual),
        ("__builtin_isless", I::IsLess),
        ("__builtin_islessequal", I::IsLessEqual),
        ("__builtin_islessgreater", I::IsLessGreater),
        ("__builtin_isunordered", I::IsUnordered),
    ]
    .into_iter()
    .map(|(name, kind)| {
        Ok((
            cursor
                .intern_name(name.as_bytes())
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            kind,
        ))
    })
    .collect()
}
impl Analyzer<'_> {
    pub(super) fn constant_floating_intrinsic(
        &mut self,
        id: ast::ExprId,
        kind: FloatingIntrinsic,
        scope: ScopeId,
    ) -> Result<super::constants::Constant, Error> {
        let ast::ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("constant intrinsic without call"));
        };
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let arguments = argument_buffer.as_slice();
        let result = if kind.comparison() {
            let left = self.constant_float_operand(arguments[0], scope)?;
            let right = self.constant_float_operand(arguments[1], scope)?;
            let order = left.compare(right);
            u64::from(match kind {
                FloatingIntrinsic::IsGreater => order == Some(Greater),
                FloatingIntrinsic::IsGreaterEqual => matches!(order, Some(Greater | Equal)),
                FloatingIntrinsic::IsLess => order == Some(Less),
                FloatingIntrinsic::IsLessEqual => matches!(order, Some(Less | Equal)),
                FloatingIntrinsic::IsLessGreater => matches!(order, Some(Less | Greater)),
                _ => order.is_none(),
            })
        } else {
            let argument = *arguments
                .last()
                .ok_or(Error::Invariant("constant intrinsic without operand"))?;
            let value = self.constant_float_operand(argument, scope)?;
            let category = value.category();
            match kind {
                FloatingIntrinsic::IsFinite => u64::from(!matches!(
                    category,
                    FloatCategory::Nan | FloatCategory::Infinite
                )),
                FloatingIntrinsic::IsInf => u64::from(category == FloatCategory::Infinite),
                FloatingIntrinsic::IsNan => u64::from(category == FloatCategory::Nan),
                FloatingIntrinsic::IsNormal => u64::from(category == FloatCategory::Normal),
                FloatingIntrinsic::SignBit => u64::from(value.negative()),
                FloatingIntrinsic::IsInfSign if category != FloatCategory::Infinite => 0,
                FloatingIntrinsic::IsInfSign => {
                    if value.negative() {
                        u64::MAX
                    } else {
                        1
                    }
                }
                FloatingIntrinsic::IsSignaling => {
                    u64::from(category == FloatCategory::Nan && !value.quiet())
                }
                FloatingIntrinsic::Classify => {
                    let index = match category {
                        FloatCategory::Nan => 0,
                        FloatCategory::Infinite => 1,
                        FloatCategory::Normal => 2,
                        FloatCategory::Subnormal => 3,
                        FloatCategory::Zero => 4,
                    };
                    let code = self.evaluate_constant(arguments[index], scope, false)?;
                    let ty = self.graph.expression_facts.get(id)?.ty;
                    let integer = self
                        .graph
                        .integral_kind(ty)
                        .ok_or(Error::Invariant("classification result is not integral"))?;
                    return Ok(super::constants::Constant {
                        bits: integer.convert(code.bits),
                        ty,
                    });
                }
                _ => return Err(Error::Invariant("constant classification kind")),
            }
        };
        Ok(super::constants::Constant {
            bits: result,
            ty: self.graph.expression_facts.get(id)?.ty,
        })
    }

    fn constant_float_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<FloatingBits, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        let target = facts
            .conversion
            .map_or(facts.ty, |conversion| conversion.target);
        let Some(super::static_initialization::Value::Floating(kind, bits)) =
            self.static_scalar(id, target, scope)?
        else {
            let id = self.graph.expression_facts.root(id);
            if let ast::ExprKind::Id(name) = self.tree.expressions.get(id).kind
                && let Some(binding) = self.graph.resolved_name(name)
            {
                let binding = self.graph.binding(binding);
                let message = match binding.kind {
                    super::graph::BindingKind::Parameter => {
                        Some("a parameter is not usable in a constant expression")
                    }
                    super::graph::BindingKind::Variable
                        if !self.graph.entity(binding.entity).constexpr =>
                    {
                        Some("a variable is not usable in a constant expression")
                    }
                    _ => None,
                };
                if let Some(message) = message {
                    return Err(Error::NonConstant {
                        location: self.tree.expressions.get(id).location,
                        message,
                    });
                }
            }
            return Err(Error::Missing(
                "floating intrinsic constant operand interpretation",
            ));
        };
        Ok(FloatingBits { kind, bits })
    }

    pub(super) fn floating_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        kind: FloatingIntrinsic,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let values = argument_buffer.as_slice();
        let count = if kind.comparison() {
            2
        } else if kind == FloatingIntrinsic::Classify {
            6
        } else {
            1
        };
        let location = self.tree.expressions.get(id).location;
        if values.len() != count {
            return Err(Error::at(
                location,
                if kind.comparison() {
                    "wrong number of arguments to a floating comparison builtin"
                } else {
                    "wrong number of arguments to a floating classification builtin"
                },
            ));
        }
        let integer = self.graph.types.scalar(Scalar::Basic(F::Int))?;
        if kind.comparison() {
            let left = self.typed_expression(values[0], scope)?;
            self.check_resolved_function(values[0])?;
            let right = self.typed_expression(values[1], scope)?;
            self.check_resolved_function(values[1])?;
            if !self.is_arithmetic_type(left.ty) || !self.is_arithmetic_type(right.ty) {
                return Err(Error::at(
                    location,
                    "a floating comparison of operands of no floating common type",
                ));
            }
            let common = self.arithmetic_type(left.ty, right.ty)?;
            if !matches!(
                self.graph.types.get(common).kind,
                TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
            ) {
                return Err(Error::at(
                    location,
                    "a floating comparison of operands of no floating common type",
                ));
            }
            self.convert_operand_scalar(values[0], common, false)?;
            self.convert_operand_scalar(values[1], common, false)?;
        } else {
            for (index, &value) in values.iter().enumerate() {
                let facts = self.typed_expression(value, scope)?;
                self.check_resolved_function(value)?;
                if index + 1 < count {
                    self.convert_scalar(value, integer, false)?;
                } else {
                    if !matches!(
                        self.graph.types.get(facts.ty).kind,
                        TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
                    ) {
                        return Err(Error::at(
                            self.tree.expressions.get(value).location,
                            "a floating classification of a non-floating operand",
                        ));
                    }
                    let target = self.bare_type(facts.ty)?;
                    self.convert_operand_scalar(value, target, false)?;
                }
            }
        }
        let mut facts = self.new_expression_facts(integer, ValueCategory::Prvalue)?;
        facts.intrinsic = Some(kind);
        Ok(facts)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FloatCategory {
    Nan,
    Infinite,
    Normal,
    Subnormal,
    Zero,
}
#[derive(Clone, Copy)]
struct FloatingBits {
    kind: F,
    bits: u128,
}
impl FloatingBits {
    fn fraction(&self) -> u32 {
        match self.kind {
            F::Float => 23,
            F::Double => 52,
            _ => 63,
        }
    }
    fn exponent_width(&self) -> u32 {
        match self.kind {
            F::Float => 8,
            F::Double => 11,
            _ => 15,
        }
    }
    fn sign_shift(&self) -> u32 {
        self.fraction() + self.exponent_width() + u32::from(self.kind == F::LongDouble)
    }
    fn negative(&self) -> bool {
        self.bits & (1 << self.sign_shift()) != 0
    }
    fn quiet(&self) -> bool {
        self.bits & (1 << (self.fraction() - 1)) != 0
    }
    fn category(&self) -> FloatCategory {
        let fraction = self.bits & ((1 << self.fraction()) - 1);
        let exponent = self.bits >> (self.sign_shift() - self.exponent_width())
            & ((1 << self.exponent_width()) - 1);
        if exponent == (1 << self.exponent_width()) - 1 {
            if fraction == 0 {
                FloatCategory::Infinite
            } else {
                FloatCategory::Nan
            }
        } else if exponent != 0 {
            FloatCategory::Normal
        } else if fraction != 0 {
            FloatCategory::Subnormal
        } else {
            FloatCategory::Zero
        }
    }
    fn compare(&self, other: Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        if self.category() == FloatCategory::Nan || other.category() == FloatCategory::Nan {
            return None;
        }
        if self.category() == FloatCategory::Zero && other.category() == FloatCategory::Zero {
            return Some(Ordering::Equal);
        }
        if self.negative() != other.negative() {
            return Some(if self.negative() {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        let order = self.bits.cmp(&other.bits);
        Some(if self.negative() {
            order.reverse()
        } else {
            order
        })
    }
}
