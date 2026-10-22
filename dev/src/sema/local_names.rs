// Translated from sema/analyzer_lambdas.cpp::local_name/odr_use_constant_operand.
// See NOTICE.
//! Enclosing automatic objects retain value reads separately from storage uses.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{Conversion, ExprFacts, FoldStyle, ValueCategory},
    graph::{BindingId, BindingKind, ScopeId, ScopeKind},
    types::TypeKind,
};
use crate::{
    lex::pp_lexer::Punctuator,
    syntax::ast::{self, ExprKind},
};

impl Analyzer<'_> {
    pub(super) fn local_name(
        &mut self,
        id: ast::ExprId,
        facts: &mut ExprFacts,
        binding: BindingId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if self.deferring_type_query() {
            self.type_queries
                .uses
                .push(super::type_queries::DeferredUse::Local(id, binding, scope));
        }
        let owner = self.graph.binding_scope(binding);
        let binding = self.graph.binding(binding);
        if !matches!(binding.kind, BindingKind::Variable | BindingKind::Parameter)
            || !matches!(
                self.graph.scope(owner).kind,
                ScopeKind::Block | ScopeKind::Function
            )
            || self.unevaluated != 0
        {
            return Ok(());
        }
        let location = self.tree.expressions.get(id).location;
        if self.default_argument_scope == Some(scope) {
            return Err(Error::at(
                location,
                "a local variable or parameter in a default argument (8.3.6p7)",
            ));
        }
        if self.template_depth != 0 || !self.in_nested_body(scope, owner) {
            return Ok(());
        }
        if self
            .graph
            .entity(binding.entity)
            .variable
            .is_some_and(|variable| !variable.automatic)
        {
            return Ok(());
        }
        if binding.kind == BindingKind::Variable
            && self.type_cv(facts.ty).0 & 1 != 0
            && self.is_integral_operand(facts.ty)
        {
            // new_facts publishes the designation before the C++ constant query.
            // Reference evaluation consumes this source type and resolved name.
            self.graph.expression_facts.set(id, *facts);
            if let Some(value) = self.constant_value(id, scope, false)? {
                facts.folded = self.held_integral_constant(value).then_some(value.bits);
                facts.fold_style = FoldStyle::EnclosingLocal;
                return Ok(());
            }
        }
        if self.capture_variable(binding.entity, location)? {
            facts.ty = self.captured_type(binding.entity, facts.ty, false)?;
            return Ok(());
        }
        Err(Error::at(
            location,
            "a local class odr-uses an automatic variable of an enclosing function (9.8p1)",
        ))
    }

    fn in_nested_body(&self, mut scope: ScopeId, owner: ScopeId) -> bool {
        let mut nested = false;
        loop {
            if scope == owner {
                return nested;
            }
            let node = self.graph.scope(scope);
            nested |= matches!(node.kind, ScopeKind::Class | ScopeKind::Function);
            let Some(parent) = node.parent else {
                return false;
            };
            scope = parent;
        }
    }

    pub(super) fn odr_use_constants(
        &mut self,
        id: ast::ExprId,
        conversion: Option<Conversion>,
    ) -> Result<(), Error> {
        if self.unevaluated != 0
            || conversion.is_some_and(|conversion| {
                conversion.temporary
                    || !matches!(
                        self.graph.types.get(conversion.target).kind,
                        TypeKind::Reference { .. }
                    )
            })
        {
            return Ok(());
        }
        self.odr_use_constant_operand(id)
    }

    fn odr_use_constant_operand(&mut self, id: ast::ExprId) -> Result<(), Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let facts = self.graph.expression_facts.get(id)?;
        if facts.category == ValueCategory::Prvalue || facts.call.is_some() {
            return Ok(());
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::Conditional {
                then_expr,
                else_expr,
                ..
            } => {
                self.odr_use_constant_operand(then_expr)?;
                self.odr_use_constant_operand(else_expr)
            }
            ExprKind::Binary { op, right, .. }
                if op.token.as_punctuator() == Some(Punctuator::Comma) =>
            {
                self.odr_use_constant_operand(right)
            }
            ExprKind::Id(name) if facts.fold_style == FoldStyle::EnclosingLocal => {
                let binding = self
                    .graph
                    .resolved_name(name)
                    .ok_or(Error::Invariant("constant local binding"))?;
                if self.capture_variable(
                    self.graph.binding(binding).entity,
                    self.tree.expressions.get(id).location,
                )? {
                    Ok(())
                } else {
                    Err(Error::at(
                        self.tree.expressions.get(id).location,
                        "a local class odr-uses an automatic variable of an enclosing function (9.8p1)",
                    ))
                }
            }
            _ => Ok(()),
        }
    }
}
