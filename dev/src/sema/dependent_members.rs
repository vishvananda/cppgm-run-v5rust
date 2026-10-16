// Translated from class_templates.cpp::dependent_member_binding and
// function_templates.cpp::substitute_member. See NOTICE.
//! Dependent member identity is a typed qualifier/name pair, never a scope.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::Filter,
    types::{Cv, TypeId, TypeKind},
};
use crate::lex::{identifiers::NameId, vocabulary::Keyword};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) struct Member {
    pub qualifier: TypeId,
    pub name: NameId,
}
#[derive(Default)]
pub(super) struct Members {
    bindings: HashMap<(TypeId, NameId), BindingId>,
    facts: HashMap<EntityId, Member>,
    contexts: HashMap<EntityId, ScopeId>,
}
impl Members {
    pub(super) fn get(&self, entity: EntityId) -> Option<Member> {
        self.facts.get(&entity).copied()
    }
}
impl Analyzer<'_> {
    pub(super) fn dependent_member_binding(
        &mut self,
        qualifier: TypeId,
        name: NameId,
    ) -> Result<BindingId, Error> {
        self.check_stack()?;
        let qualifier = self.bare_type(qualifier)?;
        if let Some(&binding) = self
            .graph
            .dependent_members
            .bindings
            .get(&(qualifier, name))
        {
            return Ok(binding);
        }
        let canonical = self.graph.types.canonical_function(qualifier)?;
        let representative = if canonical == qualifier {
            None
        } else {
            Some(self.dependent_member_binding(canonical, name)?)
        };
        let entity = self.graph.add_entity(None, Some(name), Keyword::Typename)?;
        self.graph
            .dependent_members
            .facts
            .insert(entity, Member { qualifier, name });
        if let Some(binding) = representative {
            self.graph
                .types
                .record_parameter_entity(entity, self.graph.binding(binding).entity);
        }
        let ty = self.graph.types.intern(
            TypeKind::TemplateParameter {
                entity,
                template: false,
            },
            Cv::default(),
        )?;
        self.graph.entity_mut(entity).ty = Some(ty);
        let binding = self.graph.add_hidden_binding_in(
            self.graph.global,
            Binding {
                kind: BindingKind::Alias,
                name: Some(name),
                entity,
                ty: Some(ty),
                key: Keyword::Typename,
            },
        )?;
        self.graph
            .dependent_members
            .bindings
            .insert((qualifier, name), binding);
        Ok(binding)
    }

    pub(super) fn substitution_scope(&mut self, entity: EntityId) -> Result<ScopeId, Error> {
        if let Some(&scope) = self.graph.dependent_members.contexts.get(&entity) {
            return Ok(scope);
        }
        let home = self.graph.entity(entity).home.unwrap_or(self.graph.global);
        let scope = self
            .graph
            .add_scope(ScopeKind::Function, home, None, None)?;
        self.graph.scope_mut(scope).owner = Some(entity);
        self.graph.dependent_members.contexts.insert(entity, scope);
        Ok(scope)
    }

    pub(super) fn substitute_member_type(
        &mut self,
        ty: TypeId,
        member: Member,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let Some(qualifier) = self.substitute_signature_type(member.qualifier, environment)? else {
            return Ok(None);
        };
        if qualifier == member.qualifier {
            return Ok(Some(ty));
        }
        let cv = self.graph.types.get(ty).cv;
        if self.exception_dependent_type(qualifier) {
            let binding = self.dependent_member_binding(qualifier, member.name)?;
            let ty = self
                .graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            return self.graph.types.qualify(ty, cv).map(Some);
        }
        let TypeKind::Class(class) = self.graph.types.get(qualifier).kind else {
            return Ok(None);
        };
        // A class being defined exposes its members so far. Never memoize a
        // failed lookup: completing that class can make a later trial succeed.
        let Some(scope) = self.graph.entity(class).scope else {
            return Ok(None);
        };
        let found =
            match self.qualified_declaration_result(scope, member.name.into(), Filter::Ordinary) {
                Ok(found) => found,
                Err(Error::Diagnostic {
                    message: "ambiguous name",
                    ..
                }) => return Ok(None),
                Err(error) => return Err(error),
            };
        let Some(binding) = found.binding.filter(|_| !found.ambiguous) else {
            return Ok(None);
        };
        let value = self.graph.binding(binding);
        if !matches!(value.kind, BindingKind::Type | BindingKind::Alias) {
            return Ok(None);
        }
        if let Some(entity) = environment.context {
            let context = self.substitution_scope(entity)?;
            // Substitution's access context is the template, never the calling
            // declaration whose signature is currently being checked.
            let saved_function = self.declaration_access.function.take();
            let saved_class = self.declaration_access.class.take();
            let inaccessible = self
                .member_access_failure(binding, class, None, context)
                .is_some();
            self.declaration_access.function = saved_function;
            self.declaration_access.class = saved_class;
            if inaccessible {
                return Ok(None);
            }
        }
        let resolved = value
            .ty
            .ok_or(Error::Invariant("member type binding without type"))?;
        self.graph.types.qualify(resolved, cv).map(Some)
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
    fn renamed_qualifiers_share_canonical_identity_without_member_demand() {
        let source = SourceBuffer::new(
            b"template<class T>typename T::inner::type f(T);\
              template<class U>typename U::inner::type f(U);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let mut graph = analyzer.analyze(declarations).unwrap();
        let inner = cursor.intern_name(b"inner").unwrap();
        let terminal = cursor.intern_name(b"type").unwrap();
        let mut canonical_inner = None;
        let mut canonical_terminal = None;
        for &entity in graph.dependent_members.facts.keys() {
            let ty = graph.entity(entity).ty.unwrap();
            assert!(graph.dependent_type(ty));
            assert!(graph.entity(entity).scope.is_none());
            assert!(graph.entity(entity).template_parameter.is_none());
            let canonical = graph.types.canonical_function(ty).unwrap();
            let previous = if graph.entity(entity).name == Some(inner) {
                &mut canonical_inner
            } else {
                assert_eq!(graph.entity(entity).name, Some(terminal));
                &mut canonical_terminal
            };
            if let Some(previous) = *previous {
                assert_eq!(previous, canonical);
            } else {
                *previous = Some(canonical);
            }
        }
        assert!(canonical_inner.is_some());
        assert!(canonical_terminal.is_some());
        assert_ne!(canonical_inner, canonical_terminal);
        assert!(graph.dependent_members.contexts.is_empty());
        for &binding in graph.dependent_members.bindings.values() {
            assert!(!graph.scope(graph.global).bindings.contains(&binding));
            assert_ne!(
                graph.own(graph.global, inner, Filter::Ordinary),
                Some(binding)
            );
            assert_ne!(
                graph.own(graph.global, terminal, Filter::Ordinary),
                Some(binding)
            );
        }
    }
}
