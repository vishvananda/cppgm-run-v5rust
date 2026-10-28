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
    lazy: bool,
    template_depth: usize,
}
#[derive(Clone, Copy)]
enum State {
    Pending(Pending),
    InProgress,
    Complete,
}
#[derive(Default)]
pub(super) struct Initializers(HashMap<EntityId, State>);
impl Initializers {
    pub(super) fn typed(&self, member: EntityId) -> bool {
        matches!(self.0.get(&member), Some(State::Complete))
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
        let home = self.graph.entity(entity).home;
        let pending = Pending {
            binding,
            scope,
            limit: self
                .graph
                .lookup_limit
                .unwrap_or_else(|| self.graph.binding_limit()),
            lazy: self.template_depth == 0
                && home.is_some_and(|home| self.graph.within_class_instance(home)),
            template_depth: self.template_depth,
        };
        self.graph
            .member_initializers
            .0
            .insert(entity, State::Pending(pending));
        self.deferred.push(DeferredMember::Initializer(entity));
    }

    pub(super) fn finish_member_initializer(&mut self, entity: EntityId) -> Result<(), Error> {
        if let Some(State::Pending(pending)) = self.graph.member_initializers.0.get(&entity)
            && !pending.lazy
        {
            self.request_member_initializer(entity)?;
        }
        Ok(())
    }

    pub(super) fn request_member_initializer(&mut self, entity: EntityId) -> Result<(), Error> {
        let Some(state) = self.graph.member_initializers.0.get(&entity).copied() else {
            return Ok(());
        };
        let initializer = self
            .graph
            .entity(entity)
            .member_initializer
            .ok_or(Error::Invariant("member initializer query without syntax"))?;
        let pending = match state {
            // C++ clears initializer_pending before entering the query. A
            // recursive constructor plan observes the initializer being typed.
            State::Complete | State::InProgress => return Ok(()),
            State::Pending(pending) => pending,
        };
        self.graph
            .member_initializers
            .0
            .insert(entity, State::InProgress);
        let result = if pending.lazy {
            self.instantiation_context(
                self.instantiation_depth,
                initializer.location,
                pending.limit,
                |analyzer| analyzer.type_member_initializer(pending, initializer),
            )
        } else {
            let body = self.body_context.take();
            let reserved = std::mem::take(&mut self.reserved_context);
            let context = self.closure_default_argument.take();
            let result = self.type_member_initializer(pending, initializer);
            self.body_context = body;
            self.reserved_context = reserved;
            self.closure_default_argument = context;
            result
        };
        self.graph.member_initializers.0.insert(
            entity,
            if result.is_ok() {
                State::Complete
            } else {
                State::Pending(pending)
            },
        );
        result
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
            return result;
        }
        // A concrete member initializer has the constructor's object context
        // even when its expression never spells `this`.
        let member = self
            .graph
            .entity(self.graph.binding(pending.binding).entity);
        let class = member
            .home
            .and_then(|home| self.graph.scope(home).owner)
            .ok_or(Error::Invariant("member initializer without class"))?;
        let object = self.graph.lower_entity_type(class)?;
        self.graph.types.pointer(object)?;
        let ty = self.initialize_variable(
            pending.binding,
            initializer,
            pending.scope,
            initializer.location,
        )?;
        self.check_object_type(ty, true, initializer.location)
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
            .filter_map(|(&entity, state)| matches!(state, State::Complete).then_some(entity))
            .collect();
        assert_eq!(complete.len(), 4);
        let expression_count = tree.expressions.mark();
        let binding_count = graph.binding_limit();
        let mut analyzer =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(graph)).unwrap();
        analyzer.set_lowering();
        for entity in complete {
            analyzer.request_member_initializer(entity).unwrap();
        }
        assert_eq!(binding_count, analyzer.graph.binding_limit());
        assert_eq!(expression_count, analyzer.tree.expressions.mark());
        analyzer.unevaluated = 3;
        analyzer.dependent_uses.set(17);
        analyzer.default_argument_scope = Some(analyzer.graph.global);
        assert!(analyzer.request_member_initializer(pending[0]).is_err());
        assert_eq!(analyzer.unevaluated, 3);
        assert_eq!(analyzer.dependent_uses.get(), 17);
        assert_eq!(analyzer.default_argument_scope, Some(analyzer.graph.global));
        assert!(matches!(
            analyzer.graph.member_initializers.0[&pending[0]],
            State::Pending(_)
        ));
    }
}
