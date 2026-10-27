// Translated from sema/constant_evaluator.cpp's types-mode evaluator. See NOTICE.
//! Integral values keep type identity; optional evaluation never hides semantic errors.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::TemporaryValue as Value,
    graph::{BindingKind, EntityId, ScopeId},
    integral::{self, IntegralKind},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind, InitializerKind},
};
#[derive(Clone, Copy, Debug)]
pub(super) struct Constant {
    pub state: super::constant_addresses::ScalarState,
    pub bits: u64,
    pub high: u64,
    pub ty: TypeId,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ConstantState {
    None,
    Evaluating,
    Value(Constant),
    Floating(super::constant_floats::FloatingId),
    MemberPointer(super::symbolic_members::MemberPointer),
}
fn not_constant(location: SourceOffset, message: &'static str) -> Error {
    Error::NonConstant {
        location,
        message,
        failure: super::ConstantFailure::Reported,
    }
}
pub(super) enum ConstantRequirement {
    Optional,
    Expression,
    Initializer(SourceOffset),
}
impl From<bool> for ConstantRequirement {
    fn from(required: bool) -> Self {
        if required {
            Self::Expression
        } else {
            Self::Optional
        }
    }
}
impl Analyzer<'_> {
    pub(super) fn integral_kind(&self, ty: TypeId) -> Option<IntegralKind> {
        self.graph.integral_kind(ty)
    }
    pub(super) fn scoped_enum(&self, ty: TypeId) -> bool {
        if let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
            self.graph
                .entity(entity)
                .enumeration
                .is_some_and(|info| info.scoped)
        } else {
            false
        }
    }
    pub(super) fn check_integral(
        &self,
        value: Constant,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.scoped_enum(value.ty) {
            Err(Error::at(
                location,
                "a scoped enumeration value does not convert implicitly to an integer",
            ))
        } else {
            Ok(())
        }
    }
    pub(super) fn check_constant_bool(
        &self,
        value: Constant,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if value.indeterminate() {
            return Err(not_constant(
                location,
                if self
                    .integral_kind(value.ty)
                    .is_some_and(|kind| kind.bits == 1)
                {
                    "a truth value of an object that is no scalar"
                } else {
                    "a conversion of an indeterminate value"
                },
            ));
        }
        if self.evaluating_noexcept {
            self.exception_bool(value.ty, location)
        } else {
            self.check_integral(value, location)
        }
    }
    fn scalar_constant(&mut self, bits: u64, scalar: Scalar) -> Result<Constant, Error> {
        Ok(Constant {
            state: crate::sema::constant_addresses::ScalarState::Value,
            high: 0,
            bits,
            ty: self.graph.types.scalar(scalar)?,
        })
    }
    pub(super) fn boolean(&mut self, value: bool) -> Result<Constant, Error> {
        self.scalar_constant(u64::from(value), Scalar::Bool)
    }
    pub(super) fn constant_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        requirement: impl Into<ConstantRequirement>,
    ) -> Result<Option<Constant>, Error> {
        let requirement = requirement.into();
        let required = !matches!(requirement, ConstantRequirement::Optional);
        let result = if self.semantic() && self.is_address_expression(id) {
            self.evaluate_address(id, scope).and_then(|_| {
                Err(not_constant(
                    self.tree.expressions.get(id).location,
                    "not an integral constant expression",
                ))
            })
        } else {
            self.evaluate_constant(id, scope, false)
        };
        match result {
            Ok(value) if value.indeterminate() && !required => Ok(None),
            Ok(value) if value.indeterminate() => Err(not_constant(
                match requirement {
                    ConstantRequirement::Initializer(location) => location,
                    _ => self.tree.expressions.get(id).location,
                },
                if matches!(requirement, ConstantRequirement::Initializer(_)) {
                    "a constexpr object leaves a subobject uninitialized"
                } else {
                    "not a constant scalar value"
                },
            )),
            Ok(value) if self.held_integral_constant(value) => Ok(Some(value)),
            Ok(_) if required => Err(Error::at(
                self.tree.expressions.get(id).location,
                "a 128-bit constant beyond the 64-bit integral constants",
            )),
            Err(error @ Error::NonConstant { .. }) if required => Err(match requirement {
                ConstantRequirement::Initializer(location) => error.constant_initializer(location),
                _ => error.required_expression(self.tree.expressions.get(id).location),
            }),
            Ok(_) | Err(Error::NonConstant { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub(super) fn evaluate_constant(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        self.in_constant_evaluation(|analyzer| {
            analyzer.evaluate_constant_inner(id, scope, skipping)
        })
    }
    // ConstantEvaluator::type_of uses the unevaluated integral walk without
    // starting a value-evaluation demand. Its query scratch is still bounded
    // and restored even when a subexpression fails.
    pub(super) fn integral_expression_type(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        self.with_constant_storage(|analyzer| {
            analyzer
                .evaluate_constant_inner(id, scope, true)
                .map(|value| value.ty)
        })
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One source-expression dispatch preserves constant-evaluation order and diagnostics"
    )]
    fn evaluate_constant_inner(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        self.check_deferred_constant(id)?;
        self.check_stack()?;
        let id = if self.semantic() {
            self.graph.expression_facts.root(id)
        } else {
            id
        };
        if self.semantic() && !skipping && self.graph.expression_facts.get(id).is_ok() {
            self.constant_step(self.tree.expressions.get(id).location)?;
        }
        if let Some(value) = self.recorded_constant(id, scope, skipping)? {
            return Ok(value);
        }
        let expr = *self.tree.expressions.get(id);
        let location = expr.location;
        // Instantiating a constexpr callee can complete a class while an outer
        // call frame is live. Its untyped template operands still use the
        // integral evaluator; only recorded glvalues designate frame storage.
        if self.semantic()
            && !skipping
            && self.graph.expression_facts.known(id).is_some_and(|facts| {
                facts.category != super::expression_facts::ValueCategory::Prvalue
            })
            && !matches!(expr.kind, ExprKind::Conditional { .. })
        {
            let place = self.constant_place(id, scope)?;
            return match self.read_constant_scalar(place, location)? {
                Value::Integer(value) => Ok(value),
                _ => Err(not_constant(
                    location,
                    "not an integral constant expression",
                )),
            };
        }
        if self.semantic()
            && self.graph.expression_facts.get(id).is_ok_and(|facts| {
                facts.literal_call.is_some()
                    || facts.call_form == super::expression_facts::CallForm::Operator
            })
        {
            return self.constant_call(id, scope, skipping);
        }
        if self.semantic()
            && let ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } = expr.kind
            && matches!(op.token.as_punctuator(), Some(P::Equal | P::NotEqual))
            && self.graph.expression_facts.get(left).is_ok_and(|facts| {
                self.complex_part_type(facts.ty).is_some()
                    || facts
                        .conversion
                        .is_some_and(|c| self.complex_part_type(c.result).is_some())
            })
        {
            return if skipping {
                Ok(self.integral_value(self.graph.expression_facts.get(id)?.ty, 0))
            } else {
                self.constant_complex_equal(
                    id,
                    left,
                    right,
                    op.token.as_punctuator() == Some(P::Equal),
                    scope,
                )
            };
        }
        if self.semantic() && self.floating_binary(id) {
            return self.integral_floating_operation(id, scope, skipping);
        }
        let value = match expr.kind {
            ExprKind::Literal(literal) => {
                let (PostTokenKind::Literal { ty, data, .. }
                | PostTokenKind::Character {
                    ty,
                    data,
                    suffix_begin: None,
                    ..
                }) = literal.kind
                else {
                    return Err(not_constant(
                        location,
                        if matches!(literal.kind, PostTokenKind::String { .. }) {
                            "a string literal is not an integral constant"
                        } else {
                            "not an integral constant expression"
                        },
                    ));
                };
                let scalar = Scalar::Basic(ty);
                let Some(kind) = IntegralKind::of(scalar) else {
                    return Err(not_constant(
                        location,
                        if self.semantic() {
                            "not an integral constant expression"
                        } else {
                            "floating-point constant evaluation is not supported here"
                        },
                    ));
                };
                let mut bytes = [0u8; 8];
                let data = data.as_slice();
                let count = data.len().min(8);
                bytes[..count].copy_from_slice(&data[..count]);
                self.scalar_constant(kind.convert(u64::from_le_bytes(bytes)), scalar)?
            }
            ExprKind::Keyword(Keyword::True | Keyword::False) => {
                self.boolean(matches!(expr.kind, ExprKind::Keyword(Keyword::True)))?
            }
            ExprKind::Keyword(Keyword::Nullptr) if skipping => {
                self.scalar_constant(0, Scalar::Nullptr)?
            }
            ExprKind::Paren(inner) => self.evaluate_constant(inner, scope, skipping)?,
            ExprKind::Id(name) => self.constant_named_value(id, name, scope, skipping, location)?,
            ExprKind::Binary {
                assignment: true,
                left,
                right,
                op,
            } if self.semantic() => {
                if skipping {
                    Constant {
                        state: crate::sema::constant_addresses::ScalarState::Value,
                        high: 0,
                        bits: 0,
                        ty: self.graph.expression_facts.get(id)?.ty,
                    }
                } else {
                    let place = self.constant_assignment_place(
                        id,
                        left,
                        right,
                        op.token.as_punctuator(),
                        scope,
                    )?;
                    self.read_constant_integer(place, location)?
                }
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if self.semantic()
                && self.graph.expression_facts.get(id).is_ok()
                && op.token.as_punctuator() == Some(P::Comma) =>
            {
                // constant_value selects the interpreter only for typed syntax.
                // An untyped template pattern stays on the integral spine;
                // in particular, a void cast must remain nonconstant there.
                if !skipping {
                    self.discard_constant(left, scope)?;
                }
                self.evaluate_constant(right, scope, skipping)?
            }
            ExprKind::Binary {
                op,
                assignment: false,
                ..
            } if self.semantic()
                && !skipping
                && self.graph.expression_facts.get(id).is_ok()
                && matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr)) =>
            {
                self.constant_logical_chain(id, scope)?
            }
            ExprKind::Binary { op, .. }
                if self.semantic()
                    && !skipping
                    && self.graph.expression_facts.get(id).is_ok()
                    && !matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr)) =>
            {
                let Value::Integer(value) = self.constant_scalar_chain(id, scope)? else {
                    return Err(not_constant(
                        location,
                        "not an integral constant expression",
                    ));
                };
                value
            }
            ExprKind::Binary { .. } => self.constant_chain(id, scope, skipping)?,
            ExprKind::ComplexPart { imaginary, operand } => {
                let value = self.constant_component(id, operand, imaginary, scope)?;
                let Value::Integer(value) = value else {
                    return Err(Error::Invariant("integral complex component"));
                };
                value
            }
            ExprKind::Unary { op, operand } => {
                self.constant_unary(op.token.as_punctuator(), operand, scope, skipping, id)?
            }
            ExprKind::Postfix { op, operand } if self.semantic() => {
                self.constant_unary(op.token.as_punctuator(), operand, scope, skipping, id)?
            }
            ExprKind::Conditional { .. } => self.constant_conditional(id, scope, skipping)?,
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => {
                if keyword.is_some_and(|key| {
                    if self.semantic() {
                        matches!(key, Keyword::ReinterpretCast | Keyword::DynamicCast)
                    } else {
                        key != Keyword::StaticCast
                    }
                }) {
                    return Err(not_constant(
                        location,
                        if self.semantic() {
                            "a reinterpret_cast or dynamic_cast in a constant expression"
                        } else {
                            "cast is not allowed in a constant expression"
                        },
                    ));
                }
                let ty = self.type_syntax(ty, scope)?;
                self.constant_cast(ty, Some(operand), scope, skipping, location)?
            }
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                let ty = if (self.evaluating_noexcept || self.semantic())
                    && self.graph.expression_type(id).is_some()
                {
                    self.graph
                        .expression_type(id)
                        .ok_or(Error::Invariant("untyped exception cast"))?
                } else {
                    self.specifiers(specifiers, scope, false)?.ty
                };
                let args = if let Some(braced) = braced {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
                    else {
                        return Err(Error::Invariant("nonbraced cast list"));
                    };
                    elements
                } else {
                    arguments
                };
                let args = self.tree.expr_lists.get(args);
                if args.len() > 1 {
                    return Err(not_constant(
                        location,
                        "not an integral constant expression",
                    ));
                }
                self.constant_cast(ty, args.first().copied(), scope, skipping, location)?
            }
            ExprKind::Subscript { base, index } => {
                if self.semantic()
                    && self
                        .graph
                        .expression_facts
                        .known(base)
                        .is_some_and(|facts| self.vector_shape(facts.ty).is_some())
                {
                    if skipping {
                        self.integral_value(self.graph.expression_facts.get(id)?.ty, 0)
                    } else {
                        let Value::Integer(value) =
                            self.constant_vector_lane_value(id, base, index, scope)?
                        else {
                            return Err(not_constant(
                                location,
                                "not an integral constant expression",
                            ));
                        };
                        value
                    }
                } else {
                    self.constant_string_element(base, index, scope, skipping, location)?
                }
            }
            ExprKind::SizeofPack(name) => {
                let bits = self
                    .pack_size(name, scope)?
                    .ok_or_else(|| not_constant(location, "a value-dependent sizeof..."))?;
                self.scalar_constant(
                    bits,
                    Scalar::Basic(crate::types::FundamentalType::UnsignedLongInt),
                )?
            }
            ExprKind::Sizeof { ty, operand, .. } => {
                self.constant_query(ty, operand, scope, false, location)?
            }
            ExprKind::OffsetOf { ty, designator } => {
                self.offsetof_constant(ty, designator, scope, location)?
            }
            ExprKind::TypePredicate { predicate, types } => {
                self.predicate_constant(id, predicate, types, scope, location)?
            }
            ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ty,
                operand,
            } => self.constant_query(ty, operand, scope, true, location)?,
            ExprKind::Call { .. }
                if self.evaluating_noexcept
                    && !self.semantic()
                    && self.graph.expression_type(id).is_some() =>
            {
                return self.constant_exception_call(id, scope, skipping);
            }
            // A template pattern has no typed call facts. Like the frozen
            // ConstantEvaluator, leave that call unevaluated until substitution.
            ExprKind::Call { .. }
                if self.semantic() && self.graph.expression_facts.get(id).is_ok() =>
            {
                return self.constant_call(id, scope, skipping);
            }
            ExprKind::Statement(_) if self.semantic() => {
                return Err(not_constant(
                    location,
                    "an expression that is not a constant expression",
                ));
            }
            _ => {
                return Err(not_constant(
                    location,
                    "not an integral constant expression",
                ));
            }
        };
        if !skipping && !value.indeterminate() && !self.constexpr_frames.active() {
            super::record(&mut self.graph.constant_facts, id.index(), value);
        }
        Ok(value)
    }
    // Types-mode noexcept queries retain their historical type-only boundary.
    // They have no analyzed body facts to hand to the semantic interpreter.
    fn constant_exception_call(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        let ty = self.query_call_type(id, scope)?;
        if skipping {
            return Ok(Constant {
                state: crate::sema::constant_addresses::ScalarState::Value,
                high: 0,
                bits: 0,
                ty,
            });
        }
        if self.graph.entity(self.graph.call_entity(id)?).constexpr {
            return Err(Error::Missing("constexpr function interpretation"));
        }
        let message = match self.graph.binding(self.graph.call_binding(id)?).kind {
            BindingKind::Parameter => "a parameter is not usable in a constant expression",
            BindingKind::Variable => "a variable is not usable in a constant expression",
            _ => "a call of a function that is not constexpr",
        };
        Err(not_constant(
            self.tree.expressions.get(id).location,
            message,
        ))
    }
    fn recorded_constant(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Option<Constant>, Error> {
        if self.semantic()
            && !skipping
            && let Some(step) = self.graph.expression_facts.function_conversion(id)
        {
            let location = self.tree.expressions.get(id).location;
            let returned = self.constant_conversion_result(id, scope)?;
            let value = self.returned_scalar(returned, location)?;
            let value = self.convert_constant_scalar(value, step.after.result, location)?;
            return match value {
                Value::Integer(value) => Ok(Some(value)),
                _ => Err(not_constant(
                    location,
                    "not an integral constant expression",
                )),
            };
        }
        if self.semantic()
            && !skipping
            && let ExprKind::Member { .. } = self.tree.expressions.get(id).kind
            && self
                .graph
                .expression_facts
                .get(id)
                .is_ok_and(|facts| facts.folded.is_some())
        {
            self.constant_member_object(id, scope)?;
        }
        if self.semantic()
            && let Ok(facts) = self.graph.expression_facts.get(id)
            && let Some(bits) = facts.folded.or(facts.query_value)
        {
            let ty = self.bare_type(facts.ty)?;
            return Ok(Some(self.integral_value(ty, bits)));
        }
        if self.semantic()
            && self.graph.expression_facts.get(id).is_ok()
            && let Some(value) = self.member_integral_value(id, scope, skipping)?
        {
            return Ok(Some(value));
        }
        if self.semantic()
            && self.graph.expression_facts.get(id).is_ok()
            && let Some(value) = self.address_integral_value(id, scope, skipping)?
        {
            if !skipping && !value.indeterminate() && !self.constexpr_frames.active() {
                super::record(&mut self.graph.constant_facts, id.index(), value);
            }
            return Ok(Some(value));
        }
        if self.evaluating_noexcept
            && let Some(value) = self.graph.constant_fact(id)
        {
            return Ok(Some(value));
        }
        if self.semantic()
            && !skipping
            && self
                .graph
                .expression_facts
                .get(id)
                .is_ok_and(|facts| self.graph.class_of(facts.ty).is_some())
        {
            self.constant_object_place(id, scope)?;
            return Err(not_constant(
                self.tree.expressions.get(id).location,
                "not a constant scalar value",
            ));
        }
        Ok(None)
    }
    fn constant_conditional(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        let ExprKind::Conditional {
            condition,
            then_expr,
            else_expr,
        } = self.tree.expressions.get(id).kind
        else {
            return Err(Error::Invariant(
                "constant conditional without conditional syntax",
            ));
        };
        let truth = self.constant_truth(condition, scope, skipping)?;
        let (selected, other) = if truth {
            (then_expr, else_expr)
        } else {
            (else_expr, then_expr)
        };
        let chosen = self.evaluate_constant(selected, scope, skipping)?;
        if self.semantic()
            && let Ok(facts) = self.graph.expression_facts.get(id)
            && self.integral_kind(facts.ty).is_some()
        {
            // Both arms were checked by expression analysis. Evaluation
            // visits only the selected arm and consumes the recorded
            // result type, including its usual arithmetic conversion.
            let ty = self.bare_type(facts.ty)?;
            let chosen = self.constant_integer_operand(chosen, selected)?;
            return self.converted_constant_integer(
                chosen,
                ty,
                self.tree.expressions.get(id).location,
            );
        }
        let unchosen = self.evaluate_constant(other, scope, true)?;
        Ok(if chosen.ty == unchosen.ty {
            chosen
        } else {
            self.check_integral(chosen, self.tree.expressions.get(selected).location)?;
            self.check_integral(unchosen, self.tree.expressions.get(other).location)?;
            let ty = if self.evaluating_noexcept && self.graph.expression_type(id).is_some() {
                self.graph
                    .expression_type(id)
                    .ok_or(Error::Invariant("untyped exception conditional"))?
            } else {
                self.common_integral_type(chosen.ty, unchosen.ty)?
            };
            self.convert_integer_constant(chosen, ty)?
        })
    }
    fn constant_call(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        if facts.functional_call {
            let ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
                return Err(Error::Invariant("functional call without arguments"));
            };
            let ty = facts.ty;
            let values = self.tree.expr_lists.get(arguments);
            let values = if let [value] = values
                && let ExprKind::Braced { elements, .. } = self.tree.expressions.get(*value).kind
            {
                self.tree.expr_lists.get(elements)
            } else {
                values
            };
            return self.constant_cast(
                ty,
                values.first().copied(),
                scope,
                skipping,
                self.tree.expressions.get(id).location,
            );
        }
        if skipping {
            return Ok(Constant {
                state: crate::sema::constant_addresses::ScalarState::Value,
                high: 0,
                bits: 0,
                ty: facts.ty,
            });
        }
        if let Some(kind) = self.graph.expression_facts.source_location(id) {
            return match self.constant_source_location(id, kind)? {
                super::constexpr_calls::Returned::Value(Value::Integer(value)) => Ok(value),
                _ => Err(not_constant(
                    self.tree.expressions.get(id).location,
                    "not an integral constant expression",
                )),
            };
        }
        if self.graph.expression_facts.constant_evaluation_query(id) {
            return self.boolean(true);
        }
        if let Some(super::runtime_intrinsics::RuntimeIntrinsic::Overflow(operation)) =
            self.graph.expression_facts.runtime_intrinsic(id)
        {
            return self.constant_overflow_intrinsic(id, operation, scope);
        }
        if let Some(kind) = self.graph.expression_facts.integer_intrinsic(id) {
            return self.constant_integer_intrinsic(id, kind, scope);
        }
        if let Some(kind) = facts.intrinsic {
            if matches!(
                kind,
                super::floating_intrinsics::FloatingIntrinsic::Constant
                    | super::floating_intrinsics::FloatingIntrinsic::SignalingConstant
            ) {
                return Err(not_constant(
                    self.tree.expressions.get(id).location,
                    "not an integral constant expression",
                ));
            }
            return self.constant_floating_intrinsic(id, kind, scope);
        }
        if let Some(super::function_resolution::CallTarget::Direct(binding)) = facts.call
            && matches!(
                self.graph
                    .entity(self.graph.binding(binding).entity)
                    .builtin,
                Some(super::builtin_calls::BuiltinFunction::Expect)
            )
        {
            let ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
                return Err(Error::Invariant("expect builtin without call syntax"));
            };
            let [value, expected] = *self.tree.expr_lists.get(arguments) else {
                return Err(Error::Invariant("expect builtin without two arguments"));
            };
            let value = self.constant_operand_value(value, facts.ty, scope)?;
            self.constant_operand_value(expected, facts.ty, scope)?;
            let Value::Integer(value) = value else {
                return Err(Error::Invariant("expect builtin without integer result"));
            };
            return Ok(value);
        }
        if let Some(super::function_resolution::CallTarget::Direct(binding)) = facts.call
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .builtin
                .is_some()
        {
            return Err(not_constant(
                self.tree.expressions.get(id).location,
                "a builtin function that is not a constant expression",
            ));
        }
        if facts.literal_call.is_some()
            && let Some(super::function_resolution::CallTarget::Direct(binding)) = facts.call
            && !self
                .graph
                .entity(self.graph.binding(binding).entity)
                .constexpr
        {
            // A literal operator has no object operand. Its invocation fails
            // evaluable() before its literal arguments need interpretation.
            return Err(not_constant(
                self.tree.expressions.get(id).location,
                "a call of a function that is not constexpr",
            ));
        }
        self.interpret_constant_call(id, scope)
    }
    pub(super) fn constant_member_object(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let node = *self.tree.expressions.get(id);
        if self.semantic()
            && self.graph.expression_facts.get(id).is_ok_and(|facts| {
                self.graph.class_of(facts.ty).is_some()
                    && facts.category == super::expression_facts::ValueCategory::Prvalue
            })
        {
            self.constant_object_place(id, scope)?;
            return Ok(());
        }
        match node.kind {
            ExprKind::Id(_) | ExprKind::Literal(_) | ExprKind::Keyword(_) => Ok(()),
            ExprKind::Member { op, base, .. } if op.token.as_punctuator() == Some(P::Arrow) => {
                self.constant_arrow_operand(base, scope).map(|_| ())
            }
            ExprKind::Paren(inner) | ExprKind::Member { base: inner, .. } => {
                self.constant_member_object(inner, scope)
            }
            ExprKind::Subscript { base, index } => {
                self.constant_member_object(base, scope)?;
                self.evaluate_constant(index, scope, false).map(|_| ())
            }
            ExprKind::Binary {
                op, left, right, ..
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.constant_member_object(left, scope)?;
                self.constant_member_object(right, scope)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let value = self.evaluate_constant(condition, scope, false)?;
                self.constant_member_object(
                    if value.truth() { then_expr } else { else_expr },
                    scope,
                )
            }
            ExprKind::Cast {
                keyword: None | Some(Keyword::StaticCast | Keyword::ConstCast),
                operand,
                ..
            } if self
                .graph
                .expression_facts
                .get(id)?
                .reference_cast
                .is_some() =>
            {
                self.constant_member_object(operand, scope)
            }
            ExprKind::Call { .. } => {
                let facts = self.graph.expression_facts.get(id)?;
                if let Some(super::function_resolution::CallTarget::Direct(binding)) = facts.call
                    && !self
                        .graph
                        .entity(self.graph.binding(binding).entity)
                        .constexpr
                {
                    return Err(not_constant(
                        node.location,
                        "a call of a function that is not constexpr",
                    ));
                }
                Err(Error::Missing(
                    "constexpr member-object function evaluation",
                ))
            }
            _ => Err(Error::Missing("constant member-object place evaluation")),
        }
    }
    fn constant_named_value(
        &mut self,
        id: ast::ExprId,
        name: ast::Name,
        scope: ScopeId,
        skipping: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        // A specification can demand a class whose nested template operands
        // have not been typed. Only this occurrence's facts authorize a read
        // of the recorded binding; the enclosing noexcept query does not.
        let found = if self.semantic() && self.graph.expression_facts.get(id).is_ok()
            || self.evaluating_noexcept && self.graph.expression_type(id).is_some()
        {
            self.graph
                .resolved_name(name)
                .ok_or(Error::Invariant("unresolved constant operand name"))?
        } else {
            self.lookup(name, scope, Filter::Ordinary)?
        };
        let binding = self.graph.binding(found);
        if !skipping && let Some(place) = self.constexpr_frames.local(binding.entity) {
            return self.read_constant_integer(place, location);
        }
        if self.template_depth != 0 && self.graph.dependent_members.get(binding.entity).is_some() {
            return Err(not_constant(location, "a value-dependent constant"));
        }
        if self.semantic() && !skipping && binding.kind == BindingKind::Variable
            && (self.graph.lower_binding_alias(binding.entity).is_some()
                || binding.ty.is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Reference { element, .. } if self.integral_kind(element).is_some())))
        {
            let place = self.constant_place(id, scope)?;
            return self.read_constant_integer(place, location);
        }
        if matches!(
            binding.kind,
            BindingKind::Type
                | BindingKind::Alias
                | BindingKind::Namespace
                | BindingKind::NamespaceAlias
        ) {
            return Err(Error::at(location, "name does not denote a value"));
        }
        if binding.kind == BindingKind::TemplateValue
            && self
                .graph
                .entity(binding.entity)
                .template_parameter
                .is_some()
        {
            return Err(not_constant(location, "a value-dependent constant"));
        }
        if !skipping && binding.kind == BindingKind::Variable {
            self.use_instance_variable(binding.entity)?;
        }
        if let ConstantState::Value(value) = self.graph.entity(binding.entity).constant {
            return Ok(value);
        }
        let ty = self.binding_expression_type(found)?;
        if skipping && self.integral_kind(ty).is_some() {
            let ty = self.bare_type(ty)?;
            return Ok(Constant {
                state: crate::sema::constant_addresses::ScalarState::Value,
                high: 0,
                bits: 0,
                ty,
            });
        }
        let reference = self.semantic()
            && binding.ty.is_some_and(|ty| {
                matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            });
        let message = match binding.kind {
            BindingKind::Variable if reference => {
                "a reference is not usable in a constant expression"
            }
            BindingKind::Variable if self.semantic() && self.graph.types.get(ty).cv.0 & 2 != 0 => {
                "a read of a volatile glvalue"
            }
            BindingKind::Variable if self.semantic() || self.evaluating_noexcept => {
                "a variable is not usable in a constant expression"
            }
            BindingKind::Variable => "variable is not usable in a constant expression",
            BindingKind::Parameter => "a parameter is not usable in a constant expression",
            _ => "not an integral constant expression",
        };
        Err(not_constant(location, message))
    }
    fn constant_string_element(
        &mut self,
        mut base: ast::ExprId,
        index: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(base).kind {
            base = inner;
        }
        let ExprKind::Literal(literal) = self.tree.expressions.get(base).kind else {
            return Err(not_constant(
                location,
                "not an integral constant expression",
            ));
        };
        let PostTokenKind::String {
            ty,
            elements,
            suffix: None,
        } = literal.kind
        else {
            return Err(not_constant(
                location,
                "not an integral constant expression",
            ));
        };
        let index_value = self.evaluate_constant(index, scope, skipping)?;
        self.check_integral(index_value, self.tree.expressions.get(index).location)?;
        let index_kind = self
            .integral_kind(index_value.ty)
            .ok_or(Error::Invariant("string index kind"))?;
        if (index_kind.signed && integral::signed(index_value.bits) < 0)
            || index_value.bits >= u64::from(elements)
        {
            return Err(not_constant(
                location,
                "a string literal subscript outside its array",
            ));
        }
        let offset = usize::try_from(index_value.bits)
            .ok()
            .and_then(|index| index.checked_mul(ty.size()))
            .ok_or(Error::Limit("string subscript host size"))?;
        let end = offset
            .checked_add(ty.size())
            .ok_or(Error::Limit("string element host size"))?;
        let data = self
            .literal_bytes
            .get(literal.data)
            .get(offset..end)
            .ok_or(Error::Invariant("string element outside decoded literal"))?;
        let mut bytes = [0u8; 8];
        bytes[..data.len()].copy_from_slice(data);
        let scalar = Scalar::Basic(ty);
        let kind =
            IntegralKind::of(scalar).ok_or(Error::Invariant("nonintegral character type"))?;
        self.scalar_constant(kind.convert(u64::from_le_bytes(bytes)), scalar)
    }
    fn constant_logical_chain(
        &mut self,
        root: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Constant, Error> {
        let base = self.constant_spine.len();
        let result = (|| {
            let mut next = self.graph.expression_facts.root(root);
            while let ExprKind::Binary {
                left,
                op,
                assignment: false,
                ..
            } = self.tree.expressions.get(next).kind
            {
                if !matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr))
                    || self.graph.expression_facts.get(next)?.call_form
                        == super::expression_facts::CallForm::Operator
                    || self
                        .graph
                        .expression_facts
                        .function_conversion(next)
                        .is_some()
                {
                    break;
                }
                if next != root {
                    // Each logical operand is its own rvalue visit; unlike an
                    // arithmetic spine, a logical node makes a demand decision.
                    self.check_deferred_constant(next)?;
                    self.constant_step(self.tree.expressions.get(next).location)?;
                }
                self.constant_spine.push(next);
                next = self.graph.expression_facts.root(left);
            }
            let mut value = self.constant_truth(next, scope, false)?;
            for index in (base..self.constant_spine.len()).rev() {
                let ExprKind::Binary { right, op, .. } =
                    self.tree.expressions.get(self.constant_spine[index]).kind
                else {
                    return Err(Error::Invariant("logical constant spine node"));
                };
                let conjunction = op.token.as_punctuator() == Some(P::LogicalAnd);
                if value == conjunction {
                    value = self.constant_truth(right, scope, false)?;
                }
            }
            self.boolean(value)
        })();
        self.constant_spine.truncate(base);
        result
    }
    // ConstantInterpreter::binary visits the whole left spine once, including
    // through parentheses. Keep owned scalar results between its operations.
    pub(super) fn constant_scalar_chain(
        &mut self,
        root: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        let base = self.constant_spine.len();
        let result = (|| {
            let mut next = self.graph.expression_facts.root(root);
            while let ExprKind::Binary {
                left,
                op,
                assignment: false,
                ..
            } = self.tree.expressions.get(next).kind
            {
                if self.consumes_member_pointer(next)
                    || self.graph.expression_facts.get(next)?.call_form
                        == super::expression_facts::CallForm::Operator
                    || self
                        .graph
                        .expression_facts
                        .function_conversion(next)
                        .is_some()
                    || matches!(
                        op.token.as_punctuator(),
                        Some(P::Comma | P::LogicalAnd | P::LogicalOr)
                    )
                {
                    break;
                }
                self.constant_spine.push(next);
                next = self.graph.expression_facts.root(left);
            }
            let mut value = self.scalar_value(next, scope)?;
            for index in (base..self.constant_spine.len()).rev() {
                let id = self.constant_spine[index];
                let ExprKind::Binary {
                    left, right, op, ..
                } = self.tree.expressions.get(id).kind
                else {
                    return Err(Error::Invariant("scalar spine node"));
                };
                let left_type = self
                    .graph
                    .expression_facts
                    .get(left)?
                    .conversion
                    .map_or(self.graph.expression_facts.get(left)?.ty, |conversion| {
                        conversion.result
                    });
                let facts = self.graph.expression_facts.get(left)?;
                if let Value::Pointer(pointer) = value
                    && facts.conversion.is_some_and(|conversion| {
                        conversion.second == super::expression_facts::ValueConversion::DerivedToBase
                    })
                    && self
                        .graph
                        .expression_facts
                        .function_conversion(left)
                        .is_none()
                {
                    let TypeKind::Pointer(element) = self.graph.types.get(left_type).kind else {
                        return Err(Error::Invariant("base address conversion without pointer"));
                    };
                    let path = facts
                        .base_path
                        .ok_or(Error::Invariant("base address conversion without route"))?;
                    value = Value::Pointer(self.constant_base_pointer(
                        pointer,
                        element,
                        path,
                        self.tree.expressions.get(left).location,
                    )?);
                }
                value = self.convert_constant_scalar(
                    value,
                    left_type,
                    self.tree.expressions.get(left).location,
                )?;
                let right_type = self
                    .graph
                    .expression_facts
                    .get(right)?
                    .conversion
                    .map_or(self.graph.expression_facts.get(right)?.ty, |conversion| {
                        conversion.result
                    });
                let right = self.constant_operand_value(right, right_type, scope)?;
                value = if matches!(value, Value::Pointer(_)) || matches!(right, Value::Pointer(_))
                {
                    self.constant_pointer_operation(id, op.token.as_punctuator(), value, right)?
                } else {
                    self.constant_numeric_operation(id, op.token.as_punctuator(), value, right)?
                };
            }
            Ok(value)
        })();
        self.constant_spine.truncate(base);
        result
    }
    fn constant_chain(
        &mut self,
        root: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        // Parser-built binary left spines are unbounded; they consume no native recursion.
        let base = self.constant_spine.len();
        let mut id = root;
        while let ExprKind::Binary {
            left,
            assignment,
            op,
            ..
        } = self.tree.expressions.get(id).kind
        {
            // Template definitions can be untyped even in semantic mode.
            // Only recorded facts permit the interpreter's effectful paths.
            if self.semantic()
                && self.graph.expression_facts.get(id).is_ok()
                && (assignment
                    || self.graph.expression_facts.get(id).is_ok_and(|facts| {
                        facts.call_form == super::expression_facts::CallForm::Operator
                    })
                    || self.consumes_address(id)
                    || self.consumes_member_pointer(id)
                    || self.floating_binary(id)
                    || self
                        .graph
                        .expression_facts
                        .get(left)
                        .is_ok_and(|facts| self.complex_part_type(facts.ty).is_some())
                    || op.token.as_punctuator() == Some(P::Comma))
            {
                break;
            }
            // Like ConstantInterpreter::binary, the whole left spine is one
            // rvalue visit. Its operands retain their own budgeted visits.
            self.constant_spine.push(id);
            id = left;
        }
        let result = (|| {
            let mut value = self.evaluate_constant(id, scope, skipping)?;
            let end = self.constant_spine.len();
            for index in (base..end).rev() {
                let id = self.constant_spine[index];
                let expr = *self.tree.expressions.get(id);
                let ExprKind::Binary {
                    op, left, right, ..
                } = expr.kind
                else {
                    return Err(Error::Invariant("nonbinary constant spine"));
                };
                let Some(op) = op.token.as_punctuator() else {
                    return Err(not_constant(
                        expr.location,
                        "operator is not allowed in a constant expression",
                    ));
                };
                if matches!(op, P::LogicalAnd | P::LogicalOr) {
                    self.check_constant_bool(value, self.tree.expressions.get(left).location)?;
                    let truth = value.truth();
                    let decided = if op == P::LogicalOr { truth } else { !truth };
                    if decided {
                        match self.evaluate_constant(right, scope, true) {
                            Ok(right_value) => self.check_constant_bool(
                                right_value,
                                self.tree.expressions.get(right).location,
                            )?,
                            Err(Error::NonConstant { .. }) => (),
                            Err(error) => return Err(error),
                        }
                        value = self.boolean(truth)?;
                    } else {
                        let right_value = self.evaluate_constant(right, scope, skipping)?;
                        self.check_constant_bool(
                            right_value,
                            self.tree.expressions.get(right).location,
                        )?;
                        value = self.boolean(right_value.truth())?;
                    }
                } else {
                    if op != P::Comma {
                        value = self.constant_integer_operand(value, left)?;
                    }
                    let right_value = self.evaluate_constant(right, scope, skipping)?;
                    let right_value = if op == P::Comma {
                        right_value
                    } else {
                        self.constant_integer_operand(right_value, right)?
                    };
                    value = if op == P::Comma {
                        right_value
                    } else {
                        self.constant_arithmetic(op, value, right_value, skipping, id)?
                    };
                }
                if !skipping && !value.indeterminate() && !self.constexpr_frames.active() {
                    super::record(&mut self.graph.constant_facts, id.index(), value);
                }
            }
            Ok(value)
        })();
        self.constant_spine.truncate(base);
        result
    }
    pub(super) fn promoted_type(&mut self, ty: TypeId) -> Result<TypeId, Error> {
        let ty = if let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
            let info = self
                .graph
                .entity(entity)
                .enumeration
                .ok_or(Error::Invariant("enum without metadata"))?;
            if info.scoped {
                return Ok(ty);
            }
            info.underlying
                .ok_or(Error::Invariant("enum without underlying type"))?
        } else {
            ty
        };
        let TypeKind::Scalar(scalar) = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("promotion of nonintegral type"));
        };
        self.graph.types.scalar(integral::promoted(scalar))
    }
    pub(super) fn common_integral_type(&mut self, a: TypeId, b: TypeId) -> Result<TypeId, Error> {
        let a = self.promoted_type(a)?;
        let b = self.promoted_type(b)?;
        let (TypeKind::Scalar(a), TypeKind::Scalar(b)) =
            (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
        else {
            return Err(Error::Invariant("nonintegral common type"));
        };
        let scalar = integral::common(*a, *b).ok_or(Error::Invariant("nonintegral common kind"))?;
        self.graph.types.scalar(scalar)
    }
    pub(super) fn constant_arithmetic(
        &mut self,
        op: P,
        left: Constant,
        right: Constant,
        skipping: bool,
        id: ast::ExprId,
    ) -> Result<Constant, Error> {
        let location = self.tree.expressions.get(id).location;
        let comparison = matches!(
            op,
            P::Equal | P::NotEqual | P::Less | P::Greater | P::LessEqual | P::GreaterEqual
        );
        if !skipping && (left.indeterminate() || right.indeterminate()) {
            return Err(not_constant(
                location,
                if comparison {
                    "a comparison of values that are not scalars"
                } else {
                    "an arithmetic operand that is not an integer"
                },
            ));
        }
        if comparison && (self.scoped_enum(left.ty) || self.scoped_enum(right.ty)) {
            if left.ty != right.ty {
                return Err(Error::at(
                    location,
                    "comparison between a scoped enumeration and another type",
                ));
            }
            let kind = self
                .integral_kind(left.ty)
                .ok_or(Error::Invariant("enum without underlying type"))?;
            if kind.bits > 64 && self.semantic() {
                return self.wide_arithmetic(op, left, right, (left.ty, right.ty), skipping, id);
            }
            return self.boolean(
                integral::comparison(op, kind.signed, left.bits, right.bits)
                    .ok_or(Error::Invariant("noncomparison operator"))?,
            );
        }
        self.check_integral(left, location)?;
        self.check_integral(right, location)?;
        let (ty, right_ty) = if self.semantic() && self.graph.expression_facts.get(id).is_ok() {
            let ExprKind::Binary {
                left,
                right,
                assignment: false,
                ..
            } = self.tree.expressions.get(id).kind
            else {
                return Err(Error::Invariant(
                    "constant binary operation without binary syntax",
                ));
            };
            let left = self
                .graph
                .expression_facts
                .get(left)?
                .conversion
                .ok_or(Error::Invariant("constant left operand without conversion"))?
                .result;
            let right = self
                .graph
                .expression_facts
                .get(right)?
                .conversion
                .ok_or(Error::Invariant(
                    "constant right operand without conversion",
                ))?
                .result;
            (left, right)
        } else if self.evaluating_noexcept
            && !self.semantic()
            && self.graph.expression_type(id).is_some()
        {
            self.graph.arithmetic_types(id)?
        } else if matches!(op, P::ShiftLeft | P::ShiftRight) {
            (self.promoted_type(left.ty)?, self.promoted_type(right.ty)?)
        } else {
            let common = self.common_integral_type(left.ty, right.ty)?;
            (common, common)
        };
        if self.semantic()
            && self.graph.expression_facts.get(id).is_ok()
            && self.integral_kind(ty).is_some_and(|kind| kind.bits > 64)
        {
            return self.wide_arithmetic(op, left, right, (ty, right_ty), skipping, id);
        }
        if matches!(op, P::ShiftLeft | P::ShiftRight) {
            let kind = self
                .integral_kind(ty)
                .ok_or(Error::Invariant("nonintegral shift"))?;
            let count_kind = self
                .integral_kind(right_ty)
                .ok_or(Error::Invariant("nonintegral count"))?;
            let (bits, valid) = kind.shift(op, left.bits, right.bits, count_kind);
            if !valid && !skipping {
                return Err(not_constant(location, "a shift outside its defined range"));
            }
            return Ok(self.integral_value(ty, bits));
        }
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("nonintegral operation"))?;
        if let Some(result) = integral::comparison(
            op,
            kind.signed,
            kind.convert(left.bits),
            kind.convert(right.bits),
        ) {
            return self.boolean(result);
        }
        let (bits, valid) = kind.operation(op, left.bits, right.bits).ok_or_else(|| {
            not_constant(location, "operator is not allowed in a constant expression")
        })?;
        if !valid && !skipping {
            return Err(not_constant(
                location,
                "an arithmetic operation with an undefined result",
            ));
        }
        Ok(self.integral_value(ty, bits))
    }
    fn constant_unary(
        &mut self,
        op: Option<P>,
        operand: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
        id: ast::ExprId,
    ) -> Result<Constant, Error> {
        let location = self.tree.expressions.get(id).location;
        if self.semantic()
            && op == Some(P::LogicalNot)
            && (self.floating_expression(operand)
                || self
                    .graph
                    .expression_facts
                    .get(operand)
                    .is_ok_and(|facts| self.complex_part_type(facts.ty).is_some()))
        {
            let truth = self.constant_truth(operand, scope, skipping)?;
            return self.boolean(!truth);
        }

        if self.semantic() && matches!(op, Some(P::Increment | P::Decrement)) && !skipping {
            let (place, old) =
                self.constant_increment_values(id, operand, op == Some(P::Decrement), scope)?;
            return if matches!(self.tree.expressions.get(id).kind, ExprKind::Postfix { .. }) {
                match old {
                    Value::Integer(old) => Ok(old),
                    Value::Pointer(_) | Value::Floating(_) | Value::MemberPointer(_) => {
                        Err(Error::Invariant("integer increment with pointer value"))
                    }
                }
            } else {
                self.read_constant_integer(place, location)
            };
        }
        if op == Some(P::BitAnd) {
            return Err(not_constant(
                location,
                "operator is not allowed in a constant expression",
            ));
        }
        let value = self
            .evaluate_constant(operand, scope, skipping)
            .map_err(|error| {
                if self.semantic() && matches!(op, Some(P::Increment | P::Decrement)) {
                    match error {
                        Error::NonConstant { message, .. } => not_constant(location, message),
                        other => other,
                    }
                } else {
                    error
                }
            })?;
        if op == Some(P::LogicalNot) {
            self.check_constant_bool(value, self.tree.expressions.get(operand).location)?;
            return self.boolean(!value.truth());
        }
        if !matches!(op, Some(P::Add | P::Subtract | P::Complement)) {
            return Err(not_constant(
                location,
                "operator is not allowed in a constant expression",
            ));
        }
        self.check_integral(value, location)?;
        let ty = if (self.evaluating_noexcept || self.semantic())
            && self.graph.expression_type(id).is_some()
        {
            self.graph
                .expression_type(id)
                .ok_or(Error::Invariant("untyped constant unary operand"))?
        } else {
            self.promoted_type(value.ty)?
        };
        if value.indeterminate() && !skipping {
            let value = self.converted_constant_integer(
                value,
                ty,
                self.tree.expressions.get(operand).location,
            )?;
            return if op == Some(P::Add) {
                Ok(value)
            } else {
                Err(not_constant(
                    location,
                    "a unary operand that is not an integer",
                ))
            };
        }
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("nonintegral unary"))?;
        if kind.bits > 64 && self.semantic() && self.graph.expression_facts.get(id).is_ok() {
            return self.wide_unary(op, value, ty, skipping, location);
        }
        let bits = kind.convert(value.bits);
        let bits = match op {
            Some(P::Subtract) => {
                if kind.signed && integral::signed(bits) == kind.minimum() && !skipping {
                    return Err(not_constant(
                        location,
                        if self.semantic() {
                            "a negation that overflows"
                        } else {
                            "signed overflow in a constant expression"
                        },
                    ));
                }
                kind.convert(0u64.wrapping_sub(bits))
            }
            Some(P::Complement) => kind.convert(!bits),
            _ => bits,
        };
        Ok(self.integral_value(ty, bits))
    }
    fn constant_cast(
        &mut self,
        ty: TypeId,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
        skipping: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let ty = self
            .graph
            .types
            .intern(self.graph.types.get(ty).kind.clone(), Cv::default())?;
        if self.semantic()
            && let Some(operand) = operand
            && self
                .graph
                .expression_facts
                .known(operand)
                .is_some_and(|facts| self.vector_shape(facts.ty).is_some())
        {
            if skipping {
                return Ok(self.integral_value(ty, 0));
            }
            let Value::Integer(value) =
                self.constant_vector_scalar_cast(operand, ty, scope, location)?
            else {
                return Err(Error::Invariant("integral vector cast"));
            };
            return Ok(value);
        }
        if self.semantic()
            && matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Bool)
            )
            && let Some(operand) = operand
            && self
                .graph
                .expression_facts
                .get(operand)
                .is_ok_and(|facts| self.complex_part_type(facts.ty).is_some())
        {
            let truth = !skipping && self.constant_complex_truth_at(operand, scope, location)?;
            return Ok(self.integral_value(ty, u64::from(truth)));
        }
        if self.semantic()
            && operand.is_some_and(|id| {
                self.floating_expression(id) || self.member_pointer_expression(id)
            })
        {
            if skipping {
                return Ok(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    high: 0,
                    bits: 0,
                    ty,
                });
            }
            let value = self.scalar_value(
                operand.ok_or(Error::Invariant("floating cast operand"))?,
                scope,
            )?;
            let Value::Integer(value) = self.convert_constant_scalar(value, ty, location)? else {
                return Err(not_constant(
                    location,
                    "cast to a type that is not integral",
                ));
            };
            return Ok(value);
        }
        if self.integral_kind(ty).is_none() {
            return Err(not_constant(
                location,
                "cast to a type that is not integral",
            ));
        }
        let source = operand
            .map(|id| self.evaluate_constant(id, scope, skipping))
            .transpose()?
            .unwrap_or(Constant {
                state: crate::sema::constant_addresses::ScalarState::Value,
                high: 0,
                bits: 0,
                ty,
            });
        if self.integral_kind(source.ty).is_none() {
            return Err(not_constant(
                location,
                "not an integral constant expression",
            ));
        }
        if self.semantic()
            && operand.is_none_or(|operand| self.graph.expression_facts.get(operand).is_ok())
        {
            self.converted_constant_integer(source, ty, location)
        } else {
            let kind = self
                .integral_kind(ty)
                .ok_or(Error::Invariant("integral evaluator cast"))?;
            Ok(self.integral_value(ty, kind.convert(source.bits)))
        }
    }
    pub(super) fn single_initializer(&self, initializer: ast::Initializer) -> Option<ast::ExprId> {
        match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id) => {
                if self.graph.expression_facts.initializer_array(id).is_some() {
                    return Some(id);
                }
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                    let elements = self.tree.expr_lists.get(elements);
                    if elements.len() == 1 {
                        elements.first().copied()
                    } else {
                        None
                    }
                } else {
                    Some(id)
                }
            }
            InitializerKind::Parenthesized(args) => {
                let args = self.tree.expr_lists.get(args);
                if args.len() == 1 {
                    args.first().copied()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub(super) fn record_constant_initializer(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if self.semantic() {
            self.record_address_initializer(entity, initializer, scope)?;
        }
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("constant object without type"))?;
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        if self.semantic()
            && self.graph.entity(entity).constexpr
            && matches!(
                self.graph.types.get(ty).kind,
                TypeKind::MemberPointer { .. }
            )
        {
            return self.record_member_pointer_initializer(entity, initializer, scope);
        }
        if self.semantic() && reference {
            return Ok(());
        }
        if self.semantic()
            && self.graph.entity(entity).constexpr
            && matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Class(_)
                    | TypeKind::Complex(_)
                    | TypeKind::Vector { .. }
                    | TypeKind::Array { .. }
            )
        {
            return self.record_aggregate_constant(entity, Some(initializer), scope, true);
        }
        let object = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        if self.semantic()
            && !reference
            && self.constant_floating_type(object)
            && self.graph.entity(entity).constexpr
        {
            return self.record_floating_initializer(entity, initializer, scope);
        }
        let Some(kind) = self.integral_kind(object) else {
            return Ok(());
        };
        if self.graph.types.get(object).cv.0 != 1 {
            return Ok(());
        }
        let Some(expr) = self.single_initializer(initializer) else {
            if self.semantic()
                && !reference
                && matches!(initializer.kind, InitializerKind::Assign(id) | InitializerKind::Braced(id)
                    if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { elements, .. }
                        if self.tree.expr_lists.get(elements).is_empty()))
            {
                self.graph.entity_mut(entity).constant = ConstantState::Evaluating;
                let value = self.integral_initializer_value(entity, initializer, scope)?;
                self.graph.entity_mut(entity).constant =
                    value.map_or(ConstantState::None, ConstantState::Value);
            }
            return Ok(());
        };
        self.graph.entity_mut(entity).constant = ConstantState::Evaluating;
        let mut inner = expr;
        while let ExprKind::Paren(id) = self.tree.expressions.get(inner).kind {
            inner = id;
        }
        let value = if self.semantic() && !reference {
            self.integral_initializer_value(entity, initializer, scope)?
        } else if self.semantic()
            && (self.floating_expression(expr)
                || self
                    .graph
                    .expression_facts
                    .function_conversion(expr)
                    .is_some()
                || matches!(
                    self.graph.types.get(object).kind,
                    TypeKind::Scalar(Scalar::Bool)
                ) && self.is_address_expression(expr))
        {
            let result = self.constant_operand_value(expr, object, scope);
            self.graph.entity_mut(entity).constant = ConstantState::None;
            match result {
                Ok(Value::Integer(value)) => Some(value),
                Ok(_) => return Err(Error::Invariant("nonintegral converted initializer")),
                Err(error @ Error::NonConstant { .. }) if self.graph.entity(entity).constexpr => {
                    return Err(error.constant_initializer(initializer.location));
                }
                Err(Error::NonConstant { .. }) => None,
                Err(error) => return Err(error),
            }
        } else if reference && let ExprKind::Id(name) = self.tree.expressions.get(inner).kind {
            let found = if self.semantic() {
                self.graph
                    .resolved_name(name)
                    .ok_or(Error::Invariant("unresolved reference initializer"))?
            } else {
                self.lookup(name, scope, Filter::Ordinary)?
            };
            let binding = self.graph.binding(found);
            if binding.kind == BindingKind::Variable
                && let ConstantState::Value(value) = self.graph.entity(binding.entity).constant
            {
                Some(value)
            } else {
                None
            }
        } else if self.semantic() && kind.bits > 64 {
            match self.evaluate_constant(expr, scope, false) {
                Ok(value) => Some(value),
                Err(error @ Error::NonConstant { .. }) if self.graph.entity(entity).constexpr => {
                    return Err(error.constant_initializer(initializer.location));
                }
                Err(Error::NonConstant { .. }) => None,
                Err(error) => return Err(error),
            }
        } else {
            self.constant_value(
                expr,
                scope,
                if self.semantic() && self.graph.entity(entity).constexpr {
                    ConstantRequirement::Initializer(initializer.location)
                } else {
                    ConstantRequirement::Optional
                },
            )?
        };
        self.graph.entity_mut(entity).constant = ConstantState::None;
        if let Some(value) = value {
            if value.indeterminate() {
                return if self.graph.entity(entity).constexpr {
                    Err(not_constant(
                        initializer.location,
                        "a constexpr object leaves a subobject uninitialized",
                    ))
                } else {
                    Ok(())
                };
            }
            let target = self
                .graph
                .types
                .intern(self.graph.types.get(object).kind.clone(), Cv::default())?;
            if (self.scoped_enum(target) || self.scoped_enum(value.ty)) && value.ty != target {
                return Err(Error::at(
                    self.tree.expressions.get(expr).location,
                    "a scoped enumeration value does not convert implicitly",
                ));
            }
            if matches!(self.graph.types.get(target).kind, TypeKind::Enum(_)) && value.ty != target
            {
                return Err(Error::at(
                    self.tree.expressions.get(expr).location,
                    "an integer does not convert implicitly to an enumeration",
                ));
            }
            let value = self.convert_integer_constant(value, target)?;
            self.graph.entity_mut(entity).constant = ConstantState::Value(value);
        }
        Ok(())
    }
    // analyzer_constexpr.cpp::integral_constant_operand: type before evaluating
    // so queries consume their recorded operand types, calls and conversions.
    pub(super) fn integral_constant_operand(
        &mut self,
        expr: ast::ExprId,
        scope: ScopeId,
        target: Option<TypeId>,
    ) -> Result<(), Error> {
        let facts = self.typed_expression(expr, scope)?;
        self.check_resolved_function(expr)?;
        if self.graph.class_of(facts.ty).is_some() {
            if let Some(target) = target {
                // convert_to_scalar selects by the target, but the expression
                // stands for the converter's result before the target's final
                // standard conversion (including the case-label range check).
                if let Some(sequence) = self.user_conversion_sequence(facts, target, false)?
                    && let Some(super::procedural_calls::UserConversion::Function(binding)) =
                        sequence.user_conversion
                {
                    if self
                        .graph
                        .entity(self.graph.binding(binding).entity)
                        .availability
                        == super::graph::FunctionAvailability::Deleted
                    {
                        return Err(Error::at(
                            self.tree.expressions.get(expr).location,
                            "use of a deleted function",
                        ));
                    }
                    self.convert_class_operand(expr, binding, scope)?;
                }
            } else {
                self.convert_unique_class_operand(
                    expr,
                    super::conversion_functions::ConversionTarget::Integral,
                    scope,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn array_bound(&mut self, expr: ast::ExprId, scope: ScopeId) -> Result<u64, Error> {
        self.checked_array_bound(expr, scope, true)
    }
    pub(super) fn checked_array_bound(
        &mut self,
        expr: ast::ExprId,
        scope: ScopeId,
        hosted_zero: bool,
    ) -> Result<u64, Error> {
        if self.semantic() && self.template_depth == 0 {
            let target = self.graph.types.scalar(Scalar::Basic(
                crate::types::FundamentalType::UnsignedLongInt,
            ))?;
            self.integral_constant_operand(expr, scope, Some(target))?;
        } else if self.semantic() {
            self.validate_fixed_expression(expr, scope)?;
        }
        let value = self.constant_value(expr, scope, self.template_depth == 0)?;
        let Some(value) = value else {
            return Ok(0);
        };
        let location = self.tree.expressions.get(expr).location;
        let kind = self
            .integral_kind(value.ty)
            .ok_or(Error::Invariant("nonintegral array bound"))?;
        if self.template_depth != 0
            && (!value.truth() || kind.signed && integral::signed(value.bits) < 0)
        {
            return Ok(0);
        }
        // Hosted zero bounds share unknown-bound array identity. A signature
        // substitution still requires positive size, and enum zeros keep their
        // ordinary rejection (an integral conversion does not erase that fact).
        if self.hosted()
            && hosted_zero
            && !value.truth()
            && !matches!(self.graph.types.get(value.ty).kind, TypeKind::Enum(_))
        {
            return Ok(0);
        }
        let failure = if self.scoped_enum(value.ty) {
            Some("a scoped enumeration value is not an array bound")
        } else if kind.signed && integral::signed(value.bits) < 0 {
            Some("negative array bound")
        } else if !value.truth() {
            Some("array bound is zero")
        } else {
            None
        };
        if let Some(message) = failure {
            return Err(Error::at(location, message));
        }
        Ok(value.bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn floating_glvalues_remain_available_to_optional_scalar_queries() {
        let source = SourceBuffer::new(
            b"constexpr double narrow=1.5;constexpr __float128 wide{};\
              constexpr bool qn=__builtin_constant_p(narrow);\
              constexpr bool qw=__builtin_constant_p(wide);"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"scalar-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let qn = cursor.intern_name(b"qn").unwrap();
        let qw = cursor.intern_name(b"qw").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_hosted(true);
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        for name in [qn, qw] {
            let binding = graph.own(graph.global, name, Filter::Ordinary).unwrap();
            assert!(
                matches!(graph.entity(graph.binding(binding).entity).constant,
                ConstantState::Value(value) if value.truth())
            );
        }
    }

    #[test]
    fn enclosing_noexcept_does_not_publish_untyped_constant_operands() {
        for semantic in [false, true] {
            let source =
                SourceBuffer::new(b"constexpr long value = (true ? +1 : long(2)) + 3;".to_vec())
                    .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"constant-occurrence").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let declaration = tree.decl_lists.get(declarations)[0];
            let ast::DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind
            else {
                panic!("simple declaration");
            };
            let initializer = tree.init_declarators.get(declarators)[0]
                .initializer
                .unwrap();
            let mark = tree.expressions.mark();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            if semantic {
                analyzer.set_lowering();
            }
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let scope = analyzer.graph.global;
            let expression = analyzer.single_initializer(initializer).unwrap();
            analyzer.evaluating_noexcept = true;
            let value = analyzer
                .constant_value(expression, scope, true)
                .unwrap()
                .unwrap();
            assert_eq!(value.bits, 4);
            assert!(matches!(
                analyzer.graph.types.get(value.ty).kind,
                TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::LongInt))
            ));
            assert!(analyzer.graph.expression_type(expression).is_none());
            assert!(analyzer.graph.expression_facts.get(expression).is_err());
            assert_eq!(analyzer.tree.expressions.mark(), mark);
            assert!(analyzer.evaluating_noexcept);
            assert!(analyzer.constant_evaluation.is_none());
        }
    }

    #[test]
    fn active_call_frame_does_not_require_facts_for_untyped_template_constants() {
        let source =
            SourceBuffer::new(b"constexpr int value = (true ? +1 : int(2)) + 3;".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"frame-constant-occurrence").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let declaration = tree.decl_lists.get(declarations)[0];
        let ast::DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind
        else {
            panic!("simple declaration");
        };
        let initializer = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let mark = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        let expression = analyzer.single_initializer(initializer).unwrap();
        let ty = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::Int))
            .unwrap();
        analyzer
            .in_constant_evaluation(|analyzer| {
                let frame = super::super::constexpr_frames::Frame::new(ty, scope);
                analyzer.with_constant_frame(frame, |analyzer| {
                    let value = analyzer.constant_value(expression, scope, true)?.unwrap();
                    assert_eq!(value.bits, 4);
                    assert!(analyzer.constexpr_frames.active());
                    assert!(analyzer.graph.expression_facts.known(expression).is_none());
                    assert_eq!(analyzer.tree.expressions.mark(), mark);
                    Ok(())
                })
            })
            .unwrap();
        assert!(analyzer.constant_evaluation.is_none());
        assert!(!analyzer.constexpr_frames.active());
    }

    #[test]
    fn comma_pattern_acquires_discard_semantics_only_after_typing() {
        let source =
            SourceBuffer::new(b"constexpr bool value = ((void)true, true);".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constant-boundary").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let declaration = tree.decl_lists.get(declarations)[0];
        let ast::DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind
        else {
            panic!("simple declaration");
        };
        let initializer = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let mark = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        let expression = analyzer.single_initializer(initializer).unwrap();
        assert!(
            analyzer
                .constant_value(expression, scope, false)
                .unwrap()
                .is_none()
        );
        assert!(analyzer.graph.expression_facts.get(expression).is_err());
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        assert!(analyzer.constant_spine.is_empty());
        assert!(analyzer.constant_discard_work.is_empty());
        assert!(analyzer.constant_evaluation.is_none());
        analyzer.declarations(declarations, scope).unwrap();
        let value = analyzer
            .constant_value(expression, scope, true)
            .unwrap()
            .unwrap();
        assert_eq!(value.bits, 1);
        assert!(matches!(
            analyzer.graph.types.get(value.ty).kind,
            TypeKind::Scalar(Scalar::Bool)
        ));
        assert!(analyzer.graph.expression_facts.get(expression).is_ok());
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        assert!(analyzer.constant_spine.is_empty());
        assert!(analyzer.constant_discard_work.is_empty());
        assert!(analyzer.constant_evaluation.is_none());
    }
}
