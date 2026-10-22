// Translated from sema/constant_evaluator.cpp's types-mode evaluator. See NOTICE.
//! Integral values keep type identity; optional evaluation never hides semantic errors.
use super::{
    Error,
    analyzer::Analyzer,
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
    pub bits: u64,
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
    Error::NonConstant { location, message }
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
        if self.evaluating_noexcept {
            self.exception_bool(value.ty, location)
        } else {
            self.check_integral(value, location)
        }
    }
    fn scalar_constant(&mut self, bits: u64, scalar: Scalar) -> Result<Constant, Error> {
        Ok(Constant {
            bits,
            ty: self.graph.types.scalar(scalar)?,
        })
    }
    fn boolean(&mut self, value: bool) -> Result<Constant, Error> {
        self.scalar_constant(u64::from(value), Scalar::Bool)
    }
    pub(super) fn constant_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        required: bool,
    ) -> Result<Option<Constant>, Error> {
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
            Ok(value) => Ok(Some(value)),
            Err(Error::NonConstant { location, message }) if required => {
                Err(Error::at(location, message))
            }
            Err(Error::NonConstant { .. }) => Ok(None),
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
    fn evaluate_constant_inner(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        self.check_stack()?;
        let id = if self.semantic() {
            self.graph.expression_facts.root(id)
        } else {
            id
        };
        if self.semantic() && !skipping {
            self.constant_step(self.tree.expressions.get(id).location)?;
        }
        if let Some(value) = self.recorded_constant(id, scope, skipping)? {
            return Ok(value);
        }
        let expr = *self.tree.expressions.get(id);
        let location = expr.location;
        if self.semantic()
            && (self.constexpr_frames.active() || matches!(expr.kind, ExprKind::Member { .. }))
            && !skipping
            && self.graph.expression_facts.get(id)?.category
                != super::expression_facts::ValueCategory::Prvalue
            && !matches!(expr.kind, ExprKind::Conditional { .. })
        {
            let place = self.constant_place(id, scope)?;
            return self.read_constant_integer(place, location);
        }
        if self.semantic()
            && self.graph.expression_facts.get(id).is_ok_and(|facts| {
                facts.literal_call.is_some()
                    || facts.call_form == super::expression_facts::CallForm::Operator
            })
        {
            return self.constant_call(id, scope, skipping);
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
            ExprKind::Binary { .. } => self.constant_chain(id, scope, skipping)?,
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
                let ty = if self.evaluating_noexcept
                    || self.semantic() && self.graph.expression_type(id).is_some()
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
                self.constant_string_element(base, index, scope, skipping, location)?
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
            ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ty,
                operand,
            } => self.constant_query(ty, operand, scope, true, location)?,
            ExprKind::Call { .. } if self.evaluating_noexcept && !self.semantic() => {
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
        if !skipping && !self.constexpr_frames.active() {
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
            return Ok(Constant { bits: 0, ty });
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
                super::constant_addresses::TemporaryValue::Integer(value) => Ok(Some(value)),
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
            return Ok(Some(Constant {
                bits,
                ty: self.bare_type(facts.ty)?,
            }));
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
            if !skipping && !self.constexpr_frames.active() {
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
            && let Some(kind) = self.integral_kind(facts.ty)
        {
            // Both arms were checked by expression analysis. Evaluation
            // visits only the selected arm and consumes the recorded
            // result type, including its usual arithmetic conversion.
            return Ok(Constant {
                bits: kind.convert(chosen.bits),
                ty: self.bare_type(facts.ty)?,
            });
        }
        let unchosen = self.evaluate_constant(other, scope, true)?;
        Ok(if chosen.ty == unchosen.ty {
            chosen
        } else {
            self.check_integral(chosen, self.tree.expressions.get(selected).location)?;
            self.check_integral(unchosen, self.tree.expressions.get(other).location)?;
            let ty = if self.evaluating_noexcept {
                self.graph
                    .expression_type(id)
                    .ok_or(Error::Invariant("untyped exception conditional"))?
            } else {
                self.common_integral_type(chosen.ty, unchosen.ty)?
            };
            let kind = self
                .integral_kind(ty)
                .ok_or(Error::Invariant("common nonintegral type"))?;
            Constant {
                bits: kind.convert(chosen.bits),
                ty,
            }
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
                bits: 0,
                ty: facts.ty,
            });
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
            let super::constant_addresses::TemporaryValue::Integer(value) = value else {
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
                    if value.bits == 0 {
                        else_expr
                    } else {
                        then_expr
                    },
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
        let found = if self.evaluating_noexcept
            || self.semantic() && self.graph.expression_facts.get(id).is_ok()
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
            && binding.ty.is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Reference { element, .. } if self.integral_kind(element).is_some()))
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
            return Ok(Constant { bits: 0, ty });
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
                    || op.token.as_punctuator() == Some(P::Comma))
            {
                break;
            }
            if id != root && self.semantic() && !skipping {
                self.constant_step(self.tree.expressions.get(id).location)?;
            }
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
                    let truth = value.bits != 0;
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
                        value = self.boolean(right_value.bits != 0)?;
                    }
                } else {
                    let right_value = self.evaluate_constant(right, scope, skipping)?;
                    value = if op == P::Comma {
                        right_value
                    } else {
                        self.constant_arithmetic(op, value, right_value, skipping, id)?
                    };
                }
                if !skipping && !self.constexpr_frames.active() {
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
            return self.boolean(
                integral::comparison(op, kind.signed, left.bits, right.bits)
                    .ok_or(Error::Invariant("noncomparison operator"))?,
            );
        }
        self.check_integral(left, location)?;
        self.check_integral(right, location)?;
        let (ty, right_ty) = if self.evaluating_noexcept && !self.semantic() {
            self.graph.arithmetic_types(id)?
        } else if matches!(op, P::ShiftLeft | P::ShiftRight) {
            (self.promoted_type(left.ty)?, self.promoted_type(right.ty)?)
        } else {
            let common = self.common_integral_type(left.ty, right.ty)?;
            (common, common)
        };
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
            return Ok(Constant { bits, ty });
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
        Ok(Constant { bits, ty })
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
        if self.semantic() && op == Some(P::LogicalNot) && self.floating_expression(operand) {
            let truth = self.constant_truth(operand, scope, skipping)?;
            return self.boolean(!truth);
        }

        if self.semantic() && matches!(op, Some(P::Increment | P::Decrement)) && !skipping {
            let (place, old) =
                self.constant_increment_values(id, operand, op == Some(P::Decrement), scope)?;
            return if matches!(self.tree.expressions.get(id).kind, ExprKind::Postfix { .. }) {
                match old {
                    super::constant_addresses::TemporaryValue::Integer(old) => Ok(old),
                    super::constant_addresses::TemporaryValue::Pointer(_)
                    | super::constant_addresses::TemporaryValue::Floating(_)
                    | super::constant_addresses::TemporaryValue::MemberPointer(_) => {
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
            return self.boolean(value.bits == 0);
        }
        if !matches!(op, Some(P::Add | P::Subtract | P::Complement)) {
            return Err(not_constant(
                location,
                "operator is not allowed in a constant expression",
            ));
        }
        self.check_integral(value, location)?;
        let ty = if self.evaluating_noexcept {
            self.graph
                .expression_type(id)
                .ok_or(Error::Invariant("untyped exception unary operand"))?
        } else {
            self.promoted_type(value.ty)?
        };
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("nonintegral unary"))?;
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
        Ok(Constant { bits, ty })
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
            && operand.is_some_and(|id| {
                self.floating_expression(id) || self.member_pointer_expression(id)
            })
        {
            if skipping {
                return Ok(Constant { bits: 0, ty });
            }
            let value = self.scalar_value(
                operand.ok_or(Error::Invariant("floating cast operand"))?,
                scope,
            )?;
            let super::constant_addresses::TemporaryValue::Integer(value) =
                self.convert_constant_scalar(value, ty, location)?
            else {
                return Err(not_constant(
                    location,
                    "cast to a type that is not integral",
                ));
            };
            return Ok(value);
        }
        let kind = self
            .integral_kind(ty)
            .ok_or_else(|| not_constant(location, "cast to a type that is not integral"))?;
        let source = operand
            .map(|id| self.evaluate_constant(id, scope, skipping))
            .transpose()?
            .unwrap_or(Constant { bits: 0, ty });
        if self.integral_kind(source.ty).is_none() {
            return Err(not_constant(
                location,
                "not an integral constant expression",
            ));
        }
        Ok(Constant {
            bits: kind.convert(source.bits),
            ty,
        })
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
                TypeKind::Class(_) | TypeKind::Array { .. }
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
            && self.float_format(object).is_some()
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
                let target = self.bare_type(object)?;
                self.graph.entity_mut(entity).constant = ConstantState::Value(Constant {
                    bits: 0,
                    ty: target,
                });
            }
            return Ok(());
        };
        self.graph.entity_mut(entity).constant = ConstantState::Evaluating;
        let mut inner = expr;
        while let ExprKind::Paren(id) = self.tree.expressions.get(inner).kind {
            inner = id;
        }
        let value = if self.semantic()
            && (self.floating_expression(expr)
                || matches!(
                    self.graph.types.get(object).kind,
                    TypeKind::Scalar(Scalar::Bool)
                ) && self.is_address_expression(expr))
        {
            let result = self.constant_operand_value(expr, object, scope);
            self.graph.entity_mut(entity).constant = ConstantState::None;
            match result {
                Ok(super::constant_addresses::TemporaryValue::Integer(value)) => Some(value),
                Ok(_) => return Err(Error::Invariant("nonintegral converted initializer")),
                Err(Error::NonConstant { location, message })
                    if self.graph.entity(entity).constexpr =>
                {
                    return Err(Error::at(location, message));
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
        } else {
            self.constant_value(
                expr,
                scope,
                self.semantic() && self.graph.entity(entity).constexpr,
            )?
        };
        self.graph.entity_mut(entity).constant = ConstantState::None;
        if let Some(value) = value {
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
            self.graph.entity_mut(entity).constant = ConstantState::Value(Constant {
                bits: kind.convert(value.bits),
                ty: target,
            });
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
            self.expression(expr, scope)?;
            return Ok(0);
        };
        let location = self.tree.expressions.get(expr).location;
        let kind = self
            .integral_kind(value.ty)
            .ok_or(Error::Invariant("nonintegral array bound"))?;
        if self.template_depth != 0
            && (value.bits == 0 || kind.signed && integral::signed(value.bits) < 0)
        {
            return Ok(0);
        }
        let failure = if self.scoped_enum(value.ty) {
            Some("a scoped enumeration value is not an array bound")
        } else if kind.signed && integral::signed(value.bits) < 0 {
            Some("negative array bound")
        } else if value.bits == 0 {
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
