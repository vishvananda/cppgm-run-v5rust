// Translated from template_specializations.cpp::explicit_function_instantiation,
// class_templates.cpp::explicit_instantiation and analyzer.cpp. See NOTICE.
//! Explicit declarations suppress implicit bodies; definitions demand unit-tail
//! analysis and emission independently of uses. Canonical entities own state.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, EntityId, ScopeId, ScopeKind},
    specifiers::InlinePreference,
    types::TypeKind,
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

#[derive(Clone, Copy, Default)]
pub(super) struct MemberPolicy {
    pub excluded: bool,
    pub always_inline: bool,
}

#[derive(Clone, Copy)]
enum State {
    Declaration,
    Definition,
}

#[derive(Clone, Copy)]
pub(super) struct Definition {
    pub binding: BindingId,
    pub location: SourceOffset,
}
#[derive(Default)]
pub(super) struct Instantiations {
    states: HashMap<EntityId, State>,
    definitions: Vec<Definition>,
    classes: Vec<EntityId>,
    policies: HashMap<EntityId, MemberPolicy>,
}
impl Instantiations {
    pub(super) fn note_policy(&mut self, entity: EntityId, policy: MemberPolicy) {
        if policy.excluded || policy.always_inline {
            let old = self.policies.entry(entity).or_default();
            old.excluded |= policy.excluded;
            old.always_inline |= policy.always_inline;
        }
    }
    pub(super) fn policy(&self, entity: EntityId) -> MemberPolicy {
        self.policies.get(&entity).copied().unwrap_or_default()
    }
    pub(super) fn declared(&self, entity: EntityId) -> bool {
        matches!(self.states.get(&entity), Some(State::Declaration))
    }
    pub(super) fn classes(&self) -> &[EntityId] {
        &self.classes
    }
    pub(super) fn record_class(
        &mut self,
        entity: EntityId,
        is_extern: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.record_state(entity, is_extern, location)
    }
    pub(super) fn demand_class(&mut self, entity: EntityId) {
        self.classes.push(entity);
    }
    pub(super) fn definitions(&self) -> &[Definition] {
        &self.definitions
    }
    pub(super) fn defined(&self, entity: EntityId) -> bool {
        matches!(self.states.get(&entity), Some(State::Definition))
    }
    pub(super) fn record(
        &mut self,
        binding: BindingId,
        entity: EntityId,
        is_extern: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.record_state(entity, is_extern, location)?;
        if !is_extern {
            self.definitions.push(Definition { binding, location });
        }
        Ok(())
    }
    fn record_state(
        &mut self,
        entity: EntityId,
        is_extern: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.defined(entity) {
            return Err(Error::at(
                location,
                if is_extern {
                    "an explicit instantiation declaration after the definition"
                } else {
                    "a specialization explicitly instantiated twice"
                },
            ));
        }
        self.states.insert(
            entity,
            if is_extern {
                State::Declaration
            } else {
                State::Definition
            },
        );
        Ok(())
    }
}
impl Analyzer<'_> {
    pub(super) fn explicit_instantiation(
        &mut self,
        declaration: ast::DeclId,
        is_extern: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let inner = *self.tree.declarations.get(declaration);
        if let ast::DeclKind::Simple { declarators, .. } = inner.kind
            && self.tree.init_declarators.get(declarators).len() > 1
        {
            return Err(Error::at(
                location,
                "an explicit instantiation must declare a single entity",
            ));
        }
        if let ast::DeclKind::Class(specifier) = inner.kind {
            return self.explicit_class_instantiation(specifier, is_extern, scope, inner.location);
        }
        if matches!(inner.kind, ast::DeclKind::SpecialMember { .. }) {
            return Err(Error::Missing("explicit member function instantiation"));
        }
        let ast::DeclKind::Simple {
            specifiers,
            declarators,
        } = inner.kind
        else {
            return Err(Error::at(
                inner.location,
                "an explicit instantiation of no class, function or variable",
            ));
        };
        let declarators = self.tree.init_declarators.get(declarators);
        if declarators.len() != 1 || declarators[0].initializer.is_some() {
            return Err(Error::at(
                inner.location,
                "an explicit instantiation declares one function or variable",
            ));
        }
        let declarator = declarators[0].declarator;
        // 14.7.2p12: names in the declaration, including explicit arguments,
        // do not impose access checks. Drop the deferred checks on every path.
        let access = self.begin_declaration_access(scope, true);
        let result = (|| {
            let specs = self.specifiers(specifiers, scope, false)?;
            let mut environment = self.declarator_environment(Some(declarator), scope)?;
            environment.placement = super::declarators::Placement::ExplicitInstantiation;
            let info = self.declarator_in(Some(declarator), specs.ty, environment)?;
            if info.syntax_name.is_none() {
                return Err(Error::at(
                    inner.location,
                    "an explicit instantiation of no function or variable",
                ));
            }
            if !matches!(
                self.graph.types.get(info.ty).kind,
                TypeKind::Function { .. }
            ) {
                self.explicit_variable_instantiation(
                    &info,
                    specs,
                    is_extern,
                    scope,
                    inner.location,
                )?;
                return Ok((specs, None));
            }
            let binding = self
                .declared_function_specialization(&info, scope, inner.location)?
                .ok_or(Error::at(
                    inner.location,
                    "an explicit instantiation matches no function template specialization",
                ))?;
            Ok((specs, Some(binding)))
        })();
        self.finish_declaration_access(access);
        let (specs, binding) = result?;
        let Some(binding) = binding else {
            return Ok(());
        };
        if specs.inline == InlinePreference::Inline
            || specs.constexpr
            || specs.is_static
            || specs.is_extern
        {
            return Err(Error::at(
                inner.location,
                "a specifier an explicit instantiation may not have",
            ));
        }
        let entity = self.graph.binding(binding).entity;
        let home = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("instance without home"))?;
        let here = self.declaration_scope(scope);
        if self.graph.scope(here).kind != ScopeKind::Namespace
            || !self.graph.encloses(here, self.enclosing_namespace(home))
        {
            return Err(Error::at(
                inner.location,
                "an explicit instantiation outside its template's namespace",
            ));
        }
        if self.semantic() {
            self.check_operator_declaration(binding, inner.location)?;
        }
        self.graph.function_templates.instantiations.record(
            binding,
            entity,
            is_extern,
            inner.location,
        )
    }
}
