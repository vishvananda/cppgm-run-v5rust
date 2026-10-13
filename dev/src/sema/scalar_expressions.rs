// Translated from sema/analyzer_expressions.cpp and analyzer_literals.cpp. See NOTICE.
//! Procedural expression typing on original syntax IDs, with bounded native recursion.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{BindingKind, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, vocabulary::Keyword},
    syntax::ast::{self, ExprKind},
    types::FundamentalType as F,
};
impl Analyzer<'_> {
    pub(super) fn typed_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let node = *self.tree.expressions.get(id);
        self.check_stack().map_err(|error| match error {
            Error::Limit("constructs nested too deeply") => {
                Error::at(node.location, "constructs nested too deeply")
            }
            other => other,
        })?;
        let mut facts = match node.kind {
            ExprKind::Paren(inner) => {
                let facts = self.typed_expression(inner, scope)?;
                self.graph.expression_facts.share(id, inner);
                return Ok(facts);
            }
            ExprKind::Id(name) => {
                let binding = self.lookup(name, scope, Filter::Ordinary)?;
                let binding = self.graph.binding(binding);
                if matches!(
                    binding.kind,
                    BindingKind::Namespace | BindingKind::NamespaceAlias
                ) {
                    return Err(Error::at(node.location, "namespace name used as a value"));
                }
                if matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
                    return Err(Error::at(node.location, "type name used as a value"));
                }
                if binding.kind == BindingKind::Function {
                    return Err(Error::Missing("function expression resolution"));
                }
                let mut ty = binding
                    .ty
                    .ok_or(Error::Invariant("value binding without type"))?;
                if binding.kind == BindingKind::Variable {
                    ty = self
                        .graph
                        .entity(binding.entity)
                        .ty
                        .ok_or(Error::Invariant("variable without type"))?;
                }
                if binding.kind == BindingKind::Parameter {
                    ty = self.graph.types.parameter_object(ty)?;
                }
                if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                let category = if binding.kind == BindingKind::Enumerator {
                    V::Prvalue
                } else {
                    V::Lvalue
                };
                let mut facts = self.new_expression_facts(ty, category)?;
                if binding.kind == BindingKind::Enumerator {
                    let super::constants::ConstantState::Value(value) =
                        self.graph.entity(binding.entity).constant
                    else {
                        return Err(Error::Invariant("enumerator without value"));
                    };
                    facts.ty = value.ty;
                    facts.folded = Some(value.bits);
                }
                facts
            }
            ExprKind::Literal(literal) => {
                let (ty, category) = match literal.kind {
                    PostTokenKind::Literal { ty, .. }
                    | PostTokenKind::Character {
                        ty,
                        suffix_begin: None,
                        ..
                    } => (self.graph.types.scalar(Scalar::Basic(ty))?, V::Prvalue),
                    PostTokenKind::String { ty, elements, .. } => {
                        let element = self.graph.types.scalar(Scalar::Basic(ty))?;
                        let element = self.graph.types.qualify(element, Cv::CONST)?;
                        (
                            self.graph.types.intern(
                                TypeKind::Array {
                                    element,
                                    bound: u64::from(elements),
                                },
                                Cv::default(),
                            )?,
                            V::Lvalue,
                        )
                    }
                    _ => return Err(Error::Missing("user-defined semantic literals")),
                };
                let mut facts = self.new_expression_facts(ty, category)?;
                if let PostTokenKind::Literal { ty, data, .. } = literal.kind {
                    facts.null_literal = super::integral::IntegralKind::of(Scalar::Basic(ty))
                        .is_some()
                        && data.as_slice().iter().all(|&b| b == 0);
                }
                facts
            }
            ExprKind::Keyword(keyword @ (Keyword::True | Keyword::False | Keyword::Nullptr)) => {
                let scalar = if keyword == Keyword::Nullptr {
                    Scalar::Nullptr
                } else {
                    Scalar::Bool
                };
                let ty = self.graph.types.scalar(scalar)?;
                self.new_expression_facts(ty, V::Prvalue)?
            }
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                self.typed_expression(operand, scope)?;
                let Some(op) = op.token.as_punctuator() else {
                    return Err(Error::Invariant("nonpunctuator unary"));
                };
                self.scalar_unary(
                    id,
                    op,
                    operand,
                    matches!(node.kind, ExprKind::Postfix { .. }),
                )?
            }
            ExprKind::Binary {
                assignment: false, ..
            } => return self.scalar_binary_chain(id, scope),
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: true,
            } => {
                self.typed_expression(left, scope)?;
                self.typed_expression(right, scope)?;
                let Some(op) = op.token.as_punctuator() else {
                    return Err(Error::Invariant("nonpunctuator assignment"));
                };
                self.scalar_assignment(id, op, left, right)?
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.typed_expression(condition, scope)?;
                self.convert_bool(condition)?;
                let a = self.typed_expression(then_expr, scope)?;
                let b = self.typed_expression(else_expr, scope)?;
                self.scalar_conditional(id, then_expr, else_expr, a, b)?
            }
            ExprKind::Subscript { base, index } => {
                self.typed_expression(base, scope)?;
                self.typed_expression(index, scope)?;
                self.scalar_subscript(id, base, index)?
            }
            ExprKind::Sizeof { ty, operand, .. } => {
                let ty = if let Some(ty) = ty {
                    self.type_syntax(ty, scope)?
                } else if let Some(operand) = operand {
                    self.typed_expression(operand, scope)?.ty
                } else {
                    return Err(Error::Invariant("sizeof without operand"));
                };
                let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind
                {
                    element
                } else {
                    ty
                };
                if matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Function { .. }
                        | TypeKind::Scalar(Scalar::Void)
                        | TypeKind::Array { bound: 0, .. }
                ) {
                    return Err(Error::at(
                        node.location,
                        "sizeof of an incomplete or function type",
                    ));
                }
                self.type_size(ty, node.location)?;
                let ty = self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?;
                self.new_expression_facts(ty, V::Prvalue)?
            }
            ExprKind::Call { callee, arguments } => {
                if let ExprKind::Id(name) = self.tree.expressions.get(callee).kind {
                    let lookup = self.lookup(name, scope, Filter::Ordinary);
                    if let Ok(binding) = lookup {
                        let binding = self.graph.binding(binding);
                        if matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
                            let target = binding
                                .ty
                                .ok_or(Error::Invariant("cast name without type"))?;
                            let mut facts =
                                self.scalar_functional_cast(id, target, arguments, None, scope)?;
                            facts.functional_call = true;
                            self.graph.expression_facts.set(id, facts);
                            super::record(&mut self.graph.expression_types, id.index(), facts.ty);
                            return Ok(facts);
                        }
                    } else if let Err(
                        error @ (Error::Missing(_) | Error::Invariant(_) | Error::Limit(_)),
                    ) = lookup
                    {
                        return Err(error);
                    }
                }
                return Err(Error::Missing("semantic calls and overload resolution"));
            }
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => {
                let target = self.type_syntax(ty, scope)?;
                self.explicit_scalar_conversion(id, target, operand, keyword, scope)?
            }
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                let target = self.specifiers(specifiers, scope, false)?.ty;
                self.scalar_functional_cast(id, target, arguments, braced, scope)?
            }
            _ => return Err(Error::Missing("extended semantic expressions")),
        };
        // A constexpr initializer's printed cv type is applied by its declaration view,
        // never by mutating the operand's canonical semantic type.
        facts.ty = self.graph.types.canonical_function(facts.ty)?;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), facts.ty);
        Ok(facts)
    }
    pub(super) fn new_expression_facts(
        &mut self,
        ty: TypeId,
        category: V,
    ) -> Result<ExprFacts, Error> {
        let ty = self.graph.types.canonical_function(ty)?;
        Ok(ExprFacts::new(ty, category))
    }
    pub(super) fn scalar_binary_chain(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let base = self.binary_spine.len();
        let result = (|| {
            let mut current = id;
            while let ExprKind::Binary {
                left,
                assignment: false,
                ..
            } = self.tree.expressions.get(current).kind
            {
                self.binary_spine.push(current);
                current = left;
            }
            self.typed_expression(current, scope)?;
            for index in (base..self.binary_spine.len()).rev() {
                let current = self.binary_spine[index];
                let ExprKind::Binary {
                    op, left, right, ..
                } = self.tree.expressions.get(current).kind
                else {
                    return Err(Error::Invariant("binary spine changed"));
                };
                self.typed_expression(right, scope)?;
                let Some(op) = op.token.as_punctuator() else {
                    return Err(Error::Invariant("nonpunctuator binary"));
                };
                let facts = self.scalar_binary(current, op, left, right)?;
                self.graph.expression_facts.set(current, facts);
                super::record(&mut self.graph.expression_types, current.index(), facts.ty);
            }
            self.graph.expression_facts.get(id)
        })();
        self.binary_spine.truncate(base);
        result
    }
}
