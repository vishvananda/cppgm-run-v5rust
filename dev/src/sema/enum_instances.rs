// Translated from class_templates.cpp::declare_member_enum_instance /
// instantiate_member_enum and analyzer_classes.cpp::qualified_enum_definition.
// See NOTICE.
//! Opaque member enums wait on their pattern definition and complete owner.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, ScopeId},
    lookup::Filter,
    templates::HeadId,
};
use crate::support::hash::HashMap;
use crate::syntax::ast;

#[derive(Clone, Copy)]
struct Pattern {
    syntax: ast::DeclId,
    head: Option<HeadId>,
    parameters: ScopeId,
    limit: usize,
}

#[derive(Clone, Copy)]
struct Instance {
    pattern: EntityId,
    owner: EntityId,
    state: State,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Pending,
    InProgress,
    Complete,
}
#[derive(Default)]
pub(super) struct Enumerations {
    patterns: HashMap<EntityId, Pattern>,
    instances: HashMap<EntityId, Instance>,
    waiting: HashMap<EntityId, Vec<EntityId>>,
    pending: HashMap<EntityId, Vec<EntityId>>,
}
impl Analyzer<'_> {
    pub(super) fn record_member_enum(
        &mut self,
        entity: EntityId,
        syntax: ast::DeclId,
        scope: ScopeId,
    ) {
        let head = self.active_template_head(scope);
        let limit = self.graph.binding_limit();
        self.graph.class_templates.enumerations.patterns.insert(
            entity,
            Pattern {
                syntax,
                head,
                parameters: scope,
                limit,
            },
        );
    }

    pub(super) fn declare_member_enum_instance(&mut self, entity: EntityId) -> Result<(), Error> {
        if self
            .graph
            .class_templates
            .enumerations
            .instances
            .contains_key(&entity)
        {
            return Ok(());
        }
        let enumeration = self.graph.entity(entity);
        let info = enumeration
            .enumerated()
            .enumeration
            .ok_or(Error::Invariant("member enum metadata"))?;
        let Some(name) = enumeration.name else {
            return Ok(());
        };
        let Some(owner) = self.graph.scope(info.owner).owner else {
            return Ok(());
        };
        let source = self
            .graph
            .class_templates
            .instance(owner)
            .map(|instance| instance.primary)
            .or_else(|| self.graph.class_templates.definitions.source(owner));
        let Some(scope) = source.and_then(|source| self.graph.entity(source).context().scope)
        else {
            return Ok(());
        };
        let Some(binding) = self.graph.own(scope, name, Filter::Elaborated) else {
            return Ok(());
        };
        let pattern = self.graph.binding(binding).entity;
        if self
            .graph
            .entity(pattern)
            .enumerated()
            .enumeration
            .is_none()
        {
            return Ok(());
        }
        let defined = self
            .graph
            .class_templates
            .enumerations
            .patterns
            .contains_key(&pattern);
        if self.graph.entity(pattern).defined && !defined {
            return Ok(());
        }
        self.graph.class_templates.enumerations.instances.insert(
            entity,
            Instance {
                pattern,
                owner,
                state: State::Pending,
            },
        );
        if defined {
            if self.graph.entity(owner).complete {
                self.instantiate_member_enum(entity)?;
            } else {
                self.graph
                    .class_templates
                    .enumerations
                    .pending
                    .entry(owner)
                    .or_default()
                    .push(entity);
            }
        } else {
            self.graph
                .class_templates
                .enumerations
                .waiting
                .entry(pattern)
                .or_default()
                .push(entity);
        }
        Ok(())
    }

    pub(super) fn finish_enum_waiters(&mut self, pattern: EntityId) -> Result<(), Error> {
        if let Some(waiting) = self
            .graph
            .class_templates
            .enumerations
            .waiting
            .remove(&pattern)
        {
            // C++ links each newly declared instance at the head of its list.
            for entity in waiting.into_iter().rev() {
                self.instantiate_member_enum(entity)?;
            }
        }
        Ok(())
    }

    pub(super) fn finish_member_enums(&mut self, owner: EntityId) -> Result<(), Error> {
        if let Some(pending) = self
            .graph
            .class_templates
            .enumerations
            .pending
            .remove(&owner)
        {
            for entity in pending {
                self.instantiate_member_enum(entity)?;
            }
        }
        Ok(())
    }

    fn instantiate_member_enum(&mut self, entity: EntityId) -> Result<(), Error> {
        // A qualified own definition may have completed this same enum while
        // its primary's definition was still unavailable. The definition event
        // drains the waiter, but must not overwrite the completed enumerators.
        if self.graph.entity(entity).defined {
            return Ok(());
        }
        let instance = *self
            .graph
            .class_templates
            .enumerations
            .instances
            .get(&entity)
            .ok_or(Error::Invariant("member enum without instance"))?;
        if instance.state != State::Pending {
            return Ok(());
        }
        let pattern = *self
            .graph
            .class_templates
            .enumerations
            .patterns
            .get(&instance.pattern)
            .ok_or(Error::Invariant("member enum without pattern definition"))?;
        let info = self
            .graph
            .entity(entity)
            .enumerated()
            .enumeration
            .ok_or(Error::Invariant("member enum metadata"))?;
        let lookup = if info.scoped {
            self.graph
                .entity(entity)
                .context()
                .scope
                .ok_or(Error::Invariant("member enum scope"))?
        } else {
            info.owner
        };
        let declaration = self
            .graph
            .entity(entity)
            .name
            .and_then(|name| self.graph.own(info.owner, name, Filter::Elaborated))
            .ok_or(Error::Invariant("member enum declaration"))?;
        // The oracle binds the definition's original parameter scope here,
        // independently of its definition-time name-checking bridge.
        let values =
            self.outside_lookup_frame(info.owner, lookup, pattern.head, pattern.parameters)?;
        let ast::DeclKind::Enum { enumerators, .. } =
            self.tree.declarations.get(pattern.syntax).kind
        else {
            return Err(Error::Invariant("member enum pattern syntax"));
        };
        let mut packs = super::pack_frames::FramePacks::new(&self.graph, values, self.identifiers);
        let copied = packs
            .cloner(self.tree, self.stack_budget, self.stack_base)
            .enum_values(enumerators);
        let copy = packs.finish(copied)?;
        let location = self.tree.declarations.get(pattern.syntax).location;
        self.graph
            .class_templates
            .enumerations
            .instances
            .get_mut(&entity)
            .ok_or(Error::Invariant("member enum instance disappeared"))?
            .state = State::InProgress;
        let result = self.required_instantiation(
            self.graph.instantiation_levels.get(instance.owner)?,
            location,
            pattern.limit,
            |analyzer| analyzer.enumerators(entity, copy, lookup, declaration, Some(values)),
        );
        self.graph
            .class_templates
            .enumerations
            .instances
            .get_mut(&entity)
            .ok_or(Error::Invariant("member enum instance disappeared"))?
            .state = if result.is_ok() {
            State::Complete
        } else {
            State::Pending
        };
        result
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
    fn definition_events_drain_waiters_and_completed_queries_reuse_facts() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{enum E:int;enum class F:int;};\
              int early=sizeof(A<int>);\
              template<class U>enum A<U>::E:int{x=sizeof(U),y};\
              template<class V>enum class A<V>::F:int{x=sizeof(V),y=x+1};\
              int late=sizeof(A<char>);"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"enum-events").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let store = &analyzer.graph.class_templates.enumerations;
        assert_eq!(store.patterns.len(), 2);
        assert_eq!(store.instances.len(), 4);
        assert!(store.waiting.is_empty());
        assert!(store.pending.is_empty());
        let instances: Vec<_> = store.instances.keys().copied().collect();
        assert!(
            store
                .instances
                .values()
                .all(|instance| instance.state == State::Complete)
        );
        let syntax_count = analyzer.tree.expressions.mark();
        let binding_count = analyzer.graph.binding_limit();
        analyzer.template_depth = 3;
        analyzer.dependent_uses.set(7);
        for instance in instances {
            analyzer.instantiate_member_enum(instance).unwrap();
            assert!(analyzer.graph.entity(instance).defined);
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax_count);
        assert_eq!(analyzer.graph.binding_limit(), binding_count);
        assert_eq!(analyzer.template_depth, 3);
        assert_eq!(analyzer.dependent_uses.get(), 7);
    }
}
