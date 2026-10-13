// Translated from sema/analyzer_initialization.cpp's default-argument owner. See NOTICE.
//! Declaration-time checking and visibility of defaults on canonical functions.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, ScopeId},
    types::TypeId,
};
use crate::syntax::ast;
#[derive(Clone, Copy)]
pub(super) struct DefaultArgument {
    pub expression: ast::ExprId,
    pub introduced: usize,
}
#[derive(Clone, Copy)]
pub(super) struct PendingDefaults {
    pub clause: ast::ParameterClause,
    pub scope: ScopeId,
    pub ty: TypeId,
}
impl Analyzer<'_> {
    pub(super) fn record_function_defaults(
        &mut self,
        entity: EntityId,
        clause: ast::ParameterClause,
        scope: ScopeId,
        ty: TypeId,
    ) -> Result<(), Error> {
        let ty = self.graph.types.canonical_function(ty)?;
        let count = self.graph.types.function_parameter_count(ty)?;
        let parameters = self.tree.parameters.get(clause.parameters);
        if parameters.len() != count {
            return Ok(());
        }
        let any = parameters
            .iter()
            .any(|parameter| parameter.default.is_some());
        if !any && self.graph.entity(entity).defaults.is_empty() {
            return Ok(());
        }
        if self.graph.entity(entity).defaults.is_empty() {
            self.graph.entity_mut(entity).defaults.resize(count, None);
        }
        for (index, parameter) in parameters.iter().enumerate() {
            let Some(initializer) = parameter.default else {
                continue;
            };
            if self.graph.entity(entity).defaults[index].is_some() {
                return Err(Error::at(parameter.location, "default argument redefined"));
            }
            let (ast::InitializerKind::Assign(expression)
            | ast::InitializerKind::Braced(expression)) = initializer.kind
            else {
                return Err(Error::at(
                    parameter.location,
                    "a default argument without an expression",
                ));
            };
            let introduced = self.graph.binding_limit();
            self.graph.entity_mut(entity).defaults[index] = Some(DefaultArgument {
                expression,
                introduced,
            });
        }
        let defaults = PendingDefaults { clause, scope, ty };
        if self.class_depth != 0
            && self
                .graph
                .entity(entity)
                .home
                .is_some_and(|home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
        {
            self.deferred
                .push(super::analyzer::DeferredMember::Defaults(defaults));
        } else {
            self.type_function_defaults(defaults)?;
        }
        let mut seen = false;
        for default in &self.graph.entity(entity).defaults {
            if default.is_some() {
                seen = true;
            } else if seen {
                return Err(Error::at(
                    clause.location,
                    "missing default argument after a parameter with one",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn type_function_defaults(
        &mut self,
        defaults: PendingDefaults,
    ) -> Result<(), Error> {
        let PendingDefaults { clause, scope, ty } = defaults;
        // A complete-class context is checked at class completion, in the
        // same source-ordered queue as member bodies. No enclosing body supplies
        // an implicit object to a default argument.
        let saved_body = self.body_context.take();
        let result = (|| {
            for (index, parameter) in self
                .tree
                .parameters
                .get(clause.parameters)
                .iter()
                .enumerate()
            {
                let Some(initializer) = parameter.default else {
                    continue;
                };
                let (ast::InitializerKind::Assign(expression)
                | ast::InitializerKind::Braced(expression)) = initializer.kind
                else {
                    return Err(Error::Invariant("recorded default without expression"));
                };
                let saved = self.default_argument_scope;
                self.default_argument_scope = Some(scope);
                let target = self.graph.types.function_parameter(ty, index)?;
                let result = (|| {
                    if matches!(
                        self.tree.expressions.get(expression).kind,
                        ast::ExprKind::Braced { .. }
                    ) {
                        self.list_initialize(expression, target, scope)?;
                        let mut facts = self.graph.expression_facts.get(expression)?;
                        facts.category = super::expression_facts::ValueCategory::Prvalue;
                        self.graph.expression_facts.set(expression, facts);
                        Ok(())
                    } else {
                        self.typed_expression(expression, scope)?;
                        self.convert_scalar(expression, target, false)
                    }
                })();
                self.default_argument_scope = saved;
                result?;
            }
            Ok(())
        })();
        self.body_context = saved_body;
        result
    }
}
