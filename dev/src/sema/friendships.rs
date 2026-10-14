// Translated from sema/analyzer_access.cpp and name_lookup.cpp. See NOTICE.
//! Unit-owned friendship edges and hidden-friend indexes use canonical identities.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::ValueKey,
    graph::{BindingId, EntityId, ScopeId, ScopeKind},
    types::TypeKind,
};
use crate::support::source::SourceOffset;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Friendships {
    edges: HashSet<(EntityId, EntityId)>,
    granting: HashMap<EntityId, Vec<EntityId>>,
    hidden: HashSet<BindingId>,
    functions: HashMap<(EntityId, ValueKey), Vec<BindingId>>,
}
impl Friendships {
    pub(super) fn record(&mut self, class: EntityId, entity: EntityId) {
        if self.edges.insert((class, entity)) {
            self.granting.entry(entity).or_default().push(class);
        }
    }
    pub(super) fn grants(&self, class: EntityId, entity: EntityId) -> bool {
        self.edges.contains(&(class, entity))
    }
    pub(super) fn granting(&self, entity: EntityId) -> &[EntityId] {
        self.granting.get(&entity).map_or(&[], Vec::as_slice)
    }
    pub(super) fn hide(&mut self, binding: BindingId) {
        self.hidden.insert(binding);
    }
    pub(super) fn hidden(&self, binding: BindingId) -> bool {
        self.hidden.contains(&binding)
    }
    pub(super) fn index(&mut self, class: EntityId, key: ValueKey, binding: BindingId) {
        self.functions
            .entry((class, key))
            .or_default()
            .push(binding);
    }
    pub(super) fn functions(&self, class: EntityId, key: ValueKey) -> &[BindingId] {
        self.functions.get(&(class, key)).map_or(&[], Vec::as_slice)
    }
}
impl Analyzer<'_> {
    pub(super) fn friend_environment(
        &self,
        specs: super::specifiers::Specifiers,
        scope: ScopeId,
        environment: &mut super::declarators::DeclaratorEnvironment,
    ) {
        environment.friendship = specs.friendship;
        if specs.friendship == super::specifiers::Friendship::Friend
            && environment
                .home
                .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Namespace)
        {
            environment.lookup = scope;
        }
    }
    pub(super) fn elaborated_friend_type(
        &mut self,
        key: crate::lex::vocabulary::Keyword,
        name: crate::syntax::ast::Name,
        scope: ScopeId,
    ) -> Result<super::types::TypeId, Error> {
        let class = self.friend_owner(scope, self.tree.names.get(name).location)?;
        let mut home = self.declaration_scope(scope);
        while !matches!(
            self.graph.scope(home).kind,
            ScopeKind::Namespace | ScopeKind::Block
        ) {
            home = self
                .graph
                .scope(home)
                .parent
                .ok_or(Error::Invariant("friend class without enclosing scope"))?;
        }
        let prefix = self.prefix(name, scope)?;
        let last = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()
            .ok_or(Error::Invariant("empty friend class name"))?;
        let named = Self::component_name(last.kind)?;
        let previous = if let Some(prefix) = prefix {
            self.qualified(prefix, named, super::lookup::Filter::Elaborated)?
        } else {
            let mut current = Some(scope);
            let mut found = None;
            while let Some(owner) = current {
                let node = self.graph.scope(owner);
                if matches!(
                    node.kind,
                    ScopeKind::Class | ScopeKind::Namespace | ScopeKind::Block
                ) {
                    found = self
                        .graph
                        .declared_name(owner, named, super::lookup::Filter::Elaborated)
                        .filter(|&binding| {
                            node.kind != ScopeKind::Class
                                || Some(self.graph.binding(binding).entity) != node.owner
                        });
                    if found.is_some()
                        || matches!(node.kind, ScopeKind::Namespace | ScopeKind::Block)
                    {
                        break;
                    }
                }
                current = node.parent;
            }
            found
        };
        let ty = if let Some(binding) = previous {
            let entity = self.elaborated_class_entity(
                binding,
                key,
                self.tree.names.get(name).location,
                false,
            )?;
            self.graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("friend class without type"))?
        } else {
            let ty = self.elaborated_type(key, name, home, true)?;
            let binding = self
                .graph
                .declared_name(home, named, super::lookup::Filter::Elaborated)
                .ok_or(Error::Invariant("friend class declaration without binding"))?;
            self.graph.friendships.hide(binding);
            ty
        };
        if let TypeKind::Class(entity) = self.graph.types.get(ty).kind {
            self.graph.friendships.record(class, entity);
        }
        Ok(ty)
    }
    pub(super) fn friend_owner(
        &self,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<EntityId, Error> {
        let scope = self.declaration_scope(scope);
        if self.graph.scope(scope).kind != ScopeKind::Class {
            return Err(Error::at(location, "friend declaration outside a class"));
        }
        self.graph
            .scope(scope)
            .owner
            .ok_or(Error::Invariant("friend without class"))
    }
    pub(super) fn friend_type(
        &mut self,
        ty: super::types::TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let class = self.friend_owner(scope, location)?;
        if let TypeKind::Class(entity) = self.graph.types.get(ty).kind {
            self.graph.friendships.record(class, entity);
        } else if matches!(self.graph.types.get(ty).kind, TypeKind::Auto) {
            return Err(Error::at(location, "auto in a friend declaration"));
        }
        Ok(())
    }
    pub(super) fn member_or_friend(&self, mut scope: ScopeId, class: EntityId) -> bool {
        if self.declaration_access.function.is_some_and(|function| {
            self.graph.friendships.grants(class, function)
                || self
                    .graph
                    .entity(function)
                    .home
                    .is_some_and(|home| self.graph.scope(home).owner == Some(class))
        }) {
            return true;
        }
        loop {
            let current = self.graph.scope(scope);
            if matches!(current.kind, ScopeKind::Class | ScopeKind::Function)
                && current.owner.is_some_and(|entity| {
                    current.kind == ScopeKind::Class && entity == class
                        || self.graph.friendships.grants(class, entity)
                })
            {
                return true;
            }
            let Some(parent) = current.parent else {
                return false;
            };
            scope = parent;
        }
    }
    pub(super) fn access_contexts(&self, scope: ScopeId) -> impl Iterator<Item = EntityId> + '_ {
        self.declaration_access
            .function
            .into_iter()
            .flat_map(|function| {
                self.graph
                    .entity(function)
                    .home
                    .and_then(|home| self.graph.scope(home).owner)
                    .into_iter()
                    .chain(self.graph.friendships.granting(function).iter().copied())
            })
            .chain(
                std::iter::successors(Some(scope), |&scope| self.graph.scope(scope).parent)
                    .filter_map(|scope| {
                        let current = self.graph.scope(scope);
                        matches!(current.kind, ScopeKind::Class | ScopeKind::Function)
                            .then_some((current.kind, current.owner?))
                    })
                    .flat_map(|(kind, entity)| {
                        (kind == ScopeKind::Class)
                            .then_some(entity)
                            .into_iter()
                            .chain(self.graph.friendships.granting(entity).iter().copied())
                    }),
            )
    }
}
