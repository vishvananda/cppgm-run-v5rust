// Translated from analyzer_classes.cpp::finish_class_nesting and
// analyzer_construction.cpp::instantiate_member_initializer/member_default_initializer.
// See NOTICE.
//! Complete-class initializer queries, owned by the concrete member identity.
use super::{
    Error,
    analyzer::{Analyzer, DeferredMember},
    graph::{BindingId, EntityId, ScopeId},
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Pending {
    binding: BindingId,
    scope: ScopeId,
    limit: usize,
    template_depth: usize,
}
#[derive(Clone, Copy)]
enum State {
    Queued { pending: Pending, used: bool },
    Pending(Pending),
    Started,
    NamesBound,
    Complete { throwing: bool },
}
#[derive(Default)]
pub(super) struct Initializers(HashMap<EntityId, State>);
impl Initializers {
    pub(super) fn typed(&self, member: EntityId) -> bool {
        matches!(self.0.get(&member), Some(State::Complete { .. }))
    }
    pub(super) fn throwing(&self, member: EntityId) -> bool {
        // The source member's throwing_initializer starts false before typing.
        matches!(
            self.0.get(&member),
            Some(State::Complete { throwing: true })
        )
    }
}

impl Analyzer<'_> {
    pub(super) fn finish_class_nesting(&mut self, first: usize) -> Result<(), Error> {
        let context = std::mem::take(&mut self.reserved_context);
        let result = (|| {
            let mut index = first;
            while index < self.deferred.len() {
                if let DeferredMember::ConstantValue(entity, initialization) = self.deferred[index]
                {
                    self.finish_deferred_constant(entity, initialization)?;
                }
                index += 1;
            }
            index = first;
            while index < self.deferred.len() {
                if let DeferredMember::Initializer(member) = self.deferred[index] {
                    self.finish_member_initializer(member)?;
                }
                index += 1;
            }
            index = first;
            while index < self.deferred.len() {
                match self.deferred[index] {
                    DeferredMember::ConstantValue(..) | DeferredMember::Initializer(_) => (),
                    DeferredMember::Body(body) => {
                        if self.graph.entity(body.entity).scope.is_none() {
                            self.function_body(body)?;
                        }
                    }
                    DeferredMember::Defaults(defaults) => self.type_function_defaults(defaults)?,
                }
                index += 1;
            }
            self.deferred.truncate(first);
            Ok(())
        })();
        self.reserved_context = context;
        result
    }

    pub(super) fn defer_member_initializer(&mut self, binding: BindingId, scope: ScopeId) {
        let entity = self.graph.binding(binding).entity;
        let pending = Pending {
            binding,
            scope,
            limit: self
                .graph
                .lookup_limit
                .unwrap_or_else(|| self.graph.binding_limit()),
            template_depth: self.template_depth,
        };
        self.graph.member_initializers.0.insert(
            entity,
            State::Queued {
                pending,
                used: false,
            },
        );
        self.deferred.push(DeferredMember::Initializer(entity));
    }

    pub(super) fn finish_member_initializer(&mut self, entity: EntityId) -> Result<(), Error> {
        let Some(State::Queued { mut pending, used }) =
            self.graph.member_initializers.0.get(&entity).copied()
        else {
            return Ok(());
        };
        let lazy = pending.template_depth == 0
            && self
                .graph
                .entity(entity)
                .home
                .is_some_and(|home| self.graph.instance_class(home));
        if lazy && !used {
            // Source publishes initializer_pending only at the class drain,
            // with its completed definition lookup limit.
            pending.limit = self
                .graph
                .lookup_limit
                .unwrap_or_else(|| self.graph.binding_limit());
            self.graph
                .member_initializers
                .0
                .insert(entity, State::Pending(pending));
            return Ok(());
        }
        self.enter_member_initializer(entity, pending, false)
    }

    pub(super) fn request_member_initializer(&mut self, entity: EntityId) -> Result<(), Error> {
        match self.graph.member_initializers.0.get(&entity).copied() {
            Some(State::Queued { pending, .. }) => {
                // A plan formed before the class drain retains the still
                // untyped leaf, and makes that drain type it rather than defer.
                self.graph.member_initializers.0.insert(
                    entity,
                    State::Queued {
                        pending,
                        used: true,
                    },
                );
                Ok(())
            }
            Some(State::Pending(pending)) => self.enter_member_initializer(entity, pending, true),
            _ => Ok(()),
        }
    }

    fn enter_member_initializer(
        &mut self,
        entity: EntityId,
        pending: Pending,
        lazy: bool,
    ) -> Result<(), Error> {
        // initializer_pending is cleared before lookup/context entry. Both a
        // recursive read and a later read after hard Err observe it consumed.
        self.graph
            .member_initializers
            .0
            .insert(entity, State::Started);
        let initializer = self
            .graph
            .entity(entity)
            .member_initializer
            .ok_or(Error::Invariant("member initializer query without syntax"))?;
        if lazy {
            self.instantiation_context(
                self.instantiation_depth,
                initializer.location,
                pending.limit,
                |analyzer| analyzer.type_member_initializer(pending, initializer),
            )
        } else {
            let reserved = std::mem::take(&mut self.reserved_context);
            let result = self.type_member_initializer(pending, initializer);
            self.reserved_context = reserved;
            result
        }
    }

    fn type_member_initializer(
        &mut self,
        pending: Pending,
        initializer: crate::syntax::ast::Initializer,
    ) -> Result<(), Error> {
        if pending.template_depth != 0 {
            let target = self.graph.publish_variable_type(pending.binding)?;
            let saved_depth = std::mem::replace(&mut self.template_depth, pending.template_depth);
            let member = self.graph.binding(pending.binding).entity;
            let saved_member = self.member_initializer_class;
            self.member_initializer_class = self
                .graph
                .entity(member)
                .home
                .and_then(|home| self.graph.scope(home).owner);
            let result = self.initializer_names(initializer, pending.scope, target);
            self.member_initializer_class = saved_member;
            self.template_depth = saved_depth;
            if result.is_ok() {
                self.graph
                    .member_initializers
                    .0
                    .insert(member, State::NamesBound);
            }
            return result;
        }
        // A concrete member initializer has the constructor's object context
        // even when its expression never spells `this`.
        let member = self.graph.binding(pending.binding).entity;
        let class = self
            .graph
            .entity(member)
            .home
            .and_then(|home| self.graph.scope(home).owner)
            .ok_or(Error::Invariant("member initializer without class"))?;
        let object = self.graph.lower_entity_type(class)?;
        let pointer = self.graph.types.pointer(object)?;
        let void = self.graph.types.scalar(super::types::Scalar::Void)?;
        let mut context = super::procedural_statements::BodyContext::new(void, None);
        context.object_pointer = Some(pointer);
        let saved_body = self.body_context.replace(context);
        let saved_member = self.member_initializer_class.replace(class);
        let saved_initializing = self.initializing.replace(member);
        let saved_default = self.closure_default_argument.take();
        let throwing = self.graph.analysis.potentially_throwing;
        let result = (|| {
            let ty = self.initialize_variable(
                pending.binding,
                initializer,
                pending.scope,
                initializer.location,
            )?;
            self.check_object_type(ty, true, initializer.location)?;
            let changed = self.graph.analysis.potentially_throwing != throwing;
            let throwing = changed
                || match initializer.kind {
                    crate::syntax::ast::InitializerKind::Assign(id)
                    | crate::syntax::ast::InitializerKind::Braced(id) => {
                        self.expression_may_throw(id)?
                    }
                    _ => false,
                };
            self.graph
                .member_initializers
                .0
                .insert(member, State::Complete { throwing });
            Ok(())
        })();
        self.body_context = saved_body;
        self.member_initializer_class = saved_member;
        self.initializing = saved_initializing;
        self.closure_default_argument = saved_default;
        result?;
        self.typed_inheriting_member(member);
        Ok(())
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
    fn concrete_initializers_retain_counted_and_expression_throwing_results() {
        let source = SourceBuffer::new(
            b"struct Throwing{Throwing()noexcept(false);};\
              int may_throw();int quiet()noexcept;\
              struct Owner{Throwing constructed{};int called=may_throw();int calm=quiet();};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-throwing").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let owner = cursor.intern_name(b"Owner").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(
                graph
                    .own(graph.global, owner, super::super::lookup::Filter::Prefix)
                    .unwrap(),
            )
            .entity;
        let members = &graph.entity(class).members;
        assert_eq!(members.len(), 3);
        assert!(
            members
                .iter()
                .all(|&member| graph.member_initializers.typed(member))
        );
        let answers: Vec<_> = members
            .iter()
            .map(|&member| graph.member_initializers.throwing(member))
            .collect();
        assert_eq!(answers, [true, true, false]);
        // Ordinary calls are covered by the expression fallback; only the
        // selected throwing constructor contributes to the persistent counter.
        assert_eq!(graph.analysis.potentially_throwing, 1);
    }

    #[test]
    fn complete_class_queries_keep_unused_instances_pending_and_reuse_typed_syntax() {
        let source = SourceBuffer::new(
            b"template<class T>struct Bad{int x=T::missing;};
              struct Complete{int x=later;static const int later=7;};
              template<class T>struct Good{int x=sizeof(T);};
              int size=sizeof(Bad<int>);int main(){Good<char>a;Good<char>b;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-queries").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let complete_name = cursor.intern_name(b"Complete").unwrap();
        let main_name = cursor.intern_name(b"main").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let mut graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(
                graph
                    .own(
                        graph.global,
                        complete_name,
                        super::super::lookup::Filter::Prefix,
                    )
                    .unwrap(),
            )
            .entity;
        let object = graph.lower_entity_type(class).unwrap();
        let caller = graph
            .binding(
                graph
                    .own(
                        graph.global,
                        main_name,
                        super::super::lookup::Filter::Ordinary,
                    )
                    .unwrap(),
            )
            .entity;
        let before = graph.statistics().types;
        graph.types.pointer(object).unwrap();
        assert_eq!(graph.statistics().types, before);
        let pending: Vec<_> = graph
            .member_initializers
            .0
            .iter()
            .filter_map(|(&entity, state)| matches!(state, State::Pending(_)).then_some(entity))
            .collect();
        assert_eq!(pending.len(), 1);
        let complete: Vec<_> = graph
            .member_initializers
            .0
            .iter()
            .filter_map(|(&entity, state)| {
                matches!(state, State::Complete { .. }).then_some(entity)
            })
            .collect();
        assert_eq!(complete.len(), 2);
        assert_eq!(
            graph
                .member_initializers
                .0
                .values()
                .filter(|state| matches!(state, State::NamesBound))
                .count(),
            2
        );
        let expression_count = tree.expressions.mark();
        let binding_count = graph.binding_limit();
        let mut analyzer =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(&mut graph)).unwrap();
        analyzer.set_lowering();
        for entity in complete {
            analyzer.request_member_initializer(entity).unwrap();
        }
        assert_eq!(binding_count, analyzer.graph.binding_limit());
        assert_eq!(expression_count, analyzer.tree.expressions.mark());
        analyzer.unevaluated = 3;
        analyzer.dependent_uses.set(17);
        analyzer.default_argument_scope = Some(analyzer.graph.global);
        let void = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Void)
            .unwrap();
        analyzer.body_context = Some(super::super::procedural_statements::BodyContext::new(
            void,
            Some(caller),
        ));
        analyzer.initializing = Some(pending[0]);
        analyzer.member_initializer_class = Some(class);
        assert!(analyzer.request_member_initializer(pending[0]).is_err());
        assert_eq!(analyzer.unevaluated, 3);
        assert_eq!(analyzer.dependent_uses.get(), 17);
        assert_eq!(analyzer.default_argument_scope, Some(analyzer.graph.global));
        assert_eq!(
            analyzer
                .body_context
                .as_ref()
                .and_then(|body| body.function),
            Some(caller)
        );
        assert_eq!(analyzer.initializing, Some(pending[0]));
        assert_eq!(analyzer.member_initializer_class, Some(class));
        assert!(matches!(
            analyzer.graph.member_initializers.0[&pending[0]],
            State::Started
        ));
        assert!(!analyzer.graph.member_initializers.typed(pending[0]));
        let count = analyzer.graph.analysis.potentially_throwing;
        analyzer.request_member_initializer(pending[0]).unwrap();
        assert_eq!(analyzer.graph.analysis.potentially_throwing, count);
    }
}
