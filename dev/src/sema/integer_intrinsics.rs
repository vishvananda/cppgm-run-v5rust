// Translated from analyzer_builtins.cpp and constant_expressions.cpp. See NOTICE.
//! Bit operations publish their selected kind and operand conversions once.
use super::{
    Error,
    analyzer::Analyzer,
    constants::Constant,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    types::{Scalar, TypeKind},
};
use crate::{
    lex::identifiers::NameId,
    syntax::{ast, cursor::Cursor, storage::List},
    types::FundamentalType as F,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BitOperation {
    ByteSwap,
    LeadingZeros,
    TrailingZeros,
    Population,
}
#[derive(Clone, Copy)]
pub(crate) struct IntegerBuiltin {
    pub operation: BitOperation,
    // None selects the generic form, preserving the operand's unsigned type.
    pub operand: Option<Scalar>,
}

pub(super) fn names(cursor: &mut Cursor<'_>) -> Result<Vec<(NameId, IntegerBuiltin)>, Error> {
    use BitOperation as B;
    let mut names = Vec::with_capacity(15);
    for (stem, operation, variants) in [
        (
            "__builtin_bswap",
            B::ByteSwap,
            [
                ("16", Scalar::UnsignedShort),
                ("32", Scalar::Basic(F::UnsignedInt)),
                ("64", Scalar::Basic(F::UnsignedLongInt)),
            ],
        ),
        ("__builtin_clz", B::LeadingZeros, counted_types()),
        ("__builtin_ctz", B::TrailingZeros, counted_types()),
        ("__builtin_popcount", B::Population, counted_types()),
    ] {
        for (suffix, operand) in variants
            .into_iter()
            .map(|(suffix, ty)| (suffix, Some(ty)))
            .chain((operation != B::ByteSwap).then_some(("g", None)))
        {
            let name = cursor
                .intern_name(format!("{stem}{suffix}").as_bytes())
                .map_err(|_| Error::Limit("semantic identifier limit"))?;
            names.push((name, IntegerBuiltin { operation, operand }));
        }
    }
    Ok(names)
}
fn counted_types() -> [(&'static str, Scalar); 3] {
    [
        ("", Scalar::Basic(F::UnsignedInt)),
        ("l", Scalar::Basic(F::UnsignedLongInt)),
        ("ll", Scalar::Basic(F::UnsignedLongLongInt)),
    ]
}

impl Analyzer<'_> {
    pub(super) fn integer_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        builtin: IntegerBuiltin,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let buffer = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let arguments = buffer.as_slice();
        let generic = builtin.operand.is_none();
        let fallback =
            generic && builtin.operation != BitOperation::Population && arguments.len() == 2;
        let location = self.tree.expressions.get(id).location;
        if arguments.len() != 1 && !fallback {
            return Err(Error::UnconditionalDiagnostic {
                location,
                message: if generic {
                    "wrong number of arguments to a generic bit builtin"
                } else {
                    "wrong number of arguments to a bit builtin"
                },
            });
        }
        let facts = self.typed_expression(arguments[0], scope)?;
        self.check_resolved_function(arguments[0])?;
        let integer = self.graph.types.scalar(Scalar::Basic(F::Int))?;
        let operand = if let Some(scalar) = builtin.operand {
            let target = self.graph.types.scalar(scalar)?;
            self.convert_scalar(arguments[0], target, false)?;
            target
        } else {
            let target = self.bare_type(facts.ty)?;
            if !matches!(
                self.graph.types.get(target).kind,
                TypeKind::Scalar(
                    Scalar::UnsignedChar
                        | Scalar::UnsignedShort
                        | Scalar::Basic(
                            F::UnsignedInt
                                | F::UnsignedLongInt
                                | F::UnsignedLongLongInt
                                | F::UnsignedInt128
                        )
                )
            ) {
                return Err(Error::at(
                    location,
                    "a generic bit builtin of a type that is no unsigned integer type",
                ));
            }
            self.convert_operand_scalar(arguments[0], target, false)?;
            target
        };
        if fallback {
            let zero = self.typed_expression(arguments[1], scope)?;
            self.check_resolved_function(arguments[1])?;
            if !self.is_integral_operand(zero.ty) || self.promoted_type(zero.ty)? != integer {
                return Err(Error::UnconditionalDiagnostic {
                    location: self.tree.expressions.get(arguments[1]).location,
                    message: "a generic bit builtin's second operand is no int",
                });
            }
            self.convert_scalar(arguments[1], integer, false)?;
        }
        self.graph
            .expression_facts
            .set_integer_intrinsic(id, builtin.operation);
        self.new_expression_facts(
            if builtin.operation == BitOperation::ByteSwap {
                operand
            } else {
                integer
            },
            ValueCategory::Prvalue,
        )
    }

    pub(super) fn constant_integer_intrinsic(
        &mut self,
        id: ast::ExprId,
        operation: BitOperation,
        scope: ScopeId,
    ) -> Result<Constant, Error> {
        let ast::ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("integer intrinsic without call"));
        };
        let buffer = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let arguments = buffer.as_slice();
        let facts = self.graph.expression_facts.get(arguments[0])?;
        let target = facts
            .conversion
            .map_or(facts.ty, |conversion| conversion.target);
        let super::constant_addresses::TemporaryValue::Integer(value) =
            self.constant_operand_value(arguments[0], target, scope)?
        else {
            return Err(Error::Invariant("integer builtin without integer operand"));
        };
        let width = self
            .integral_kind(target)
            .ok_or(Error::Invariant("bit operand width"))?
            .bits;
        if value.indeterminate() {
            return Err(super::constant_addresses::unreported_constant(
                self.tree.expressions.get(id).location,
            ));
        }
        let bits = value.wide() & (u128::MAX >> (128 - width));
        let result = self.graph.expression_facts.get(id)?.ty;
        if bits == 0
            && matches!(
                operation,
                BitOperation::LeadingZeros | BitOperation::TrailingZeros
            )
        {
            if let Some(&zero) = arguments.get(1) {
                let super::constant_addresses::TemporaryValue::Integer(value) =
                    self.constant_operand_value(zero, result, scope)?
                else {
                    return Err(Error::Invariant("bit fallback without integer value"));
                };
                if value.indeterminate() {
                    return Err(super::constant_addresses::unreported_constant(
                        self.tree.expressions.get(id).location,
                    ));
                }
                return Ok(value);
            }
            return Err(Error::NonConstant {
                location: self.tree.expressions.get(id).location,
                message: if width == 128 {
                    "__builtin_clzg or __builtin_ctzg of zero"
                } else {
                    "__builtin_clz or __builtin_ctz of zero"
                },
                failure: super::ConstantFailure::Reported,
            });
        }
        let bits = match operation {
            BitOperation::ByteSwap => bits.swap_bytes() >> (128 - width),
            BitOperation::LeadingZeros => u128::from(bits.leading_zeros() - (128 - width)),
            BitOperation::TrailingZeros => u128::from(bits.trailing_zeros()),
            BitOperation::Population => u128::from(bits.count_ones()),
        };
        Ok(Constant::from_wide(result, bits))
    }
}
