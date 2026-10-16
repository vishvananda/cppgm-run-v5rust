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
    pub template: Option<super::template_defaults::Pattern>,
}
#[derive(Clone, Copy)]
pub(super) struct PendingDefaults {
    pub clause: ast::ParameterClause,
    pub scope: ScopeId,
    pub ty: TypeId,
    pub template_depth: usize,
}
impl Analyzer<'_> {
    pub(super) fn record_template_function_defaults(
        &mut self,
        entity: EntityId,
        clause: ast::ParameterClause,
        scope: ScopeId,
        ty: TypeId,
    ) -> Result<(), Error> {
        let count = self.graph.types.function_parameter_count(ty)?;
        let parameters = self.tree.parameters.get(clause.parameters);
        if parameters.len() != count {
            return Ok(());
        }
        let mut any = false;
        let templated = self.graph.entity(entity).template_head.is_some();
        for (index, parameter) in parameters.iter().enumerate() {
            let Some(initializer) = parameter.default else {
                continue;
            };
            let (ast::InitializerKind::Assign(expression)
            | ast::InitializerKind::Braced(expression)) = initializer.kind
            else {
                continue;
            };
            any = true;
            if !templated {
                continue;
            }
            if self.graph.entity(entity).defaults.is_empty() {
                self.graph.entity_mut(entity).defaults.resize(count, None);
            } else if self.graph.entity(entity).defaults[index].is_some() {
                return Err(Error::at(parameter.location, "default argument redefined"));
            }
            self.graph.entity_mut(entity).defaults[index] = Some(DefaultArgument {
                expression,
                template: Some(super::template_defaults::Pattern {
                    head: self
                        .template_head
                        .ok_or(Error::Invariant("template default without active head"))?,
                    scope,
                    limit: self.graph.templates.definition_limit(
                        self.graph
                            .entity(entity)
                            .template_head
                            .ok_or(Error::Invariant("template default without primary head"))?,
                    ),
                }),
                introduced: self
                    .graph
                    .binding_limit()
                    .checked_sub(1)
                    .ok_or(Error::Invariant("template default without declaration"))?,
            });
        }
        if any {
            let defaults = PendingDefaults {
                clause,
                scope,
                ty,
                template_depth: self.template_depth,
            };
            if self.class_depth != 0 {
                // finish_members skips default-only entries for member/friend
                // templates. Their patterns remain for specialization demand.
                if !templated {
                    self.deferred
                        .push(super::analyzer::DeferredMember::Defaults(defaults));
                }
            } else {
                self.type_function_defaults(defaults)?;
            }
        }
        Ok(())
    }
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
            // The visibility limit is the first excluded binding. A default
            // belongs to the declaration just published, not the next one.
            let introduced = self
                .graph
                .binding_limit()
                .checked_sub(1)
                .ok_or(Error::Invariant("default argument without declaration"))?;
            self.graph.entity_mut(entity).defaults[index] = Some(DefaultArgument {
                expression,
                introduced,
                template: None,
            });
        }
        let defaults = PendingDefaults {
            clause,
            scope,
            ty,
            template_depth: self.template_depth,
        };
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
            let saved = self.default_argument_scope;
            self.default_argument_scope = Some(scope);
            let result = self.type_function_defaults(defaults);
            self.default_argument_scope = saved;
            result?;
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
        let PendingDefaults {
            clause,
            scope,
            ty,
            template_depth,
        } = defaults;
        if template_depth != 0 {
            let saved_depth = self.template_depth;
            self.template_depth = template_depth;
            let result = (|| {
                for syntax_index in 0..self.tree.parameters.get(clause.parameters).len() {
                    let syntax_item = self.tree.parameters.get(clause.parameters)[syntax_index];
                    let parameter = &syntax_item;
                    if let Some(initializer) = parameter.default {
                        self.initializer_names(initializer, scope, ty)?;
                    }
                }
                Ok(())
            })();
            self.template_depth = saved_depth;
            return result;
        }
        // A complete-class context is checked at class completion, in the
        // same source-ordered queue as member bodies. No enclosing body supplies
        // an implicit object to a default argument.
        let saved_body = self.body_context.take();
        let result = (|| {
            for index in 0..self.tree.parameters.get(clause.parameters).len() {
                let syntax_item = self.tree.parameters.get(clause.parameters)[index];
                let parameter = &syntax_item;
                let Some(initializer) = parameter.default else {
                    continue;
                };
                let (ast::InitializerKind::Assign(expression)
                | ast::InitializerKind::Braced(expression)) = initializer.kind
                else {
                    return Err(Error::Invariant("recorded default without expression"));
                };
                let target = self.graph.types.function_parameter(ty, index)?;
                let result = (|| {
                    if matches!(
                        self.tree.expressions.get(expression).kind,
                        ast::ExprKind::Braced { .. }
                    ) {
                        self.initialize_list_argument(expression, target, scope, None)
                    } else {
                        self.typed_expression(expression, scope)?;
                        self.convert_scalar(expression, target, false)?;
                        self.record_class_copy(
                            expression,
                            target,
                            scope,
                            super::copy_initialization::CopyUse::Value { in_place: false },
                            self.tree.expressions.get(expression).location,
                        )
                    }
                })();
                result?;
            }
            Ok(())
        })();
        self.body_context = saved_body;
        result
    }
}
