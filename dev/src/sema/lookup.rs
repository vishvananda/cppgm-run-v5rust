// Translated from sema/lookup.cpp and analyzer_types.cpp's name_prefix. See NOTICE.
//! Searches visit only lexical parents and explicit namespace edges.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, BindingKind, ScopeId, ScopeKind},
    types::TypeKind,
};
use crate::{
    lex::identifiers::NameId,
    syntax::ast::{Name, NameKind},
};
use std::collections::HashSet;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Filter {
    Ordinary,
    Prefix,
    Namespace,
    Elaborated,
}
impl Analyzer<'_> {
    fn merge(&self, first: &mut Option<BindingId>, second: Option<BindingId>) -> Result<(), Error> {
        if let Some(second) = second {
            if let Some(first) = *first {
                let a = self.graph.binding(first);
                let b = self.graph.binding(second);
                if a.entity != b.entity
                    && !(matches!(a.kind, BindingKind::Type | BindingKind::Alias)
                        && matches!(b.kind, BindingKind::Type | BindingKind::Alias)
                        && a.ty == b.ty)
                {
                    return Err(Error::at(self.lookup_location, "ambiguous name"));
                }
            } else {
                *first = Some(second);
            }
        }
        Ok(())
    }
    fn inline_set(
        &self,
        scope: ScopeId,
        name: NameId,
        filter: Filter,
        visited: &mut HashSet<ScopeId>,
        members: &mut Vec<ScopeId>,
    ) -> Result<Option<BindingId>, Error> {
        let mut result = None;
        let mut frontier = vec![scope];
        while let Some(current) = frontier.pop() {
            if !visited.insert(current) {
                continue;
            }
            members.push(current);
            self.merge(&mut result, self.graph.own(current, name, filter))?;
            frontier.extend(self.graph.visible_edges(current, true));
        }
        Ok(result)
    }
    pub(super) fn qualified(
        &self,
        scope: ScopeId,
        name: NameId,
        filter: Filter,
    ) -> Result<Option<BindingId>, Error> {
        if self.graph.scope(scope).kind != ScopeKind::Namespace {
            return Ok(self.graph.own(scope, name, filter));
        }
        let mut visited = HashSet::new();
        let mut level = vec![scope];
        let mut result = None;
        while !level.is_empty() && result.is_none() {
            let mut next = Vec::new();
            for current in level {
                let mut members = Vec::new();
                let found = self.inline_set(current, name, filter, &mut visited, &mut members)?;
                if found.is_some() {
                    self.merge(&mut result, found)?;
                } else {
                    for member in members {
                        next.extend(self.graph.visible_edges(member, false).rev());
                    }
                }
            }
            level = next;
        }
        Ok(result)
    }
    fn namespace(&self, mut scope: ScopeId) -> ScopeId {
        while self.graph.scope(scope).kind != ScopeKind::Namespace {
            let Some(parent) = self.graph.scope(scope).parent else {
                break;
            };
            scope = parent;
        }
        scope
    }
    fn common_namespace(&self, a: ScopeId, b: ScopeId) -> ScopeId {
        let mut a = self.namespace(a);
        let b = self.namespace(b);
        while !self.graph.encloses(a, b) {
            let Some(parent) = self.graph.scope(a).parent else {
                break;
            };
            a = parent;
        }
        a
    }
    pub(super) fn unqualified(
        &self,
        scope: ScopeId,
        name: NameId,
        filter: Filter,
    ) -> Result<Option<BindingId>, Error> {
        let mut visited = HashSet::new();
        let mut pending = Vec::new();
        let mut current = Some(scope);
        while let Some(scope) = current {
            let mut result = self.graph.own(scope, name, filter);
            let mut frontier: Vec<_> = self.graph.visible_edges(scope, false).collect();
            while let Some(nominated) = frontier.pop() {
                if !visited.insert(nominated) {
                    continue;
                }
                pending.push((nominated, self.common_namespace(scope, nominated)));
                frontier.extend(self.graph.visible_edges(nominated, false));
            }
            let mut index = 0;
            while index < pending.len() {
                let (nominated, home) = pending[index];
                if home == scope {
                    self.merge(&mut result, self.graph.own(nominated, name, filter))?;
                    pending.swap_remove(index);
                } else {
                    index += 1;
                }
            }
            if result.is_some() {
                return Ok(result);
            }
            current = self.graph.scope(scope).parent;
        }
        Ok(None)
    }
    pub(super) fn component_name(kind: NameKind) -> Result<NameId, Error> {
        match kind {
            NameKind::Identifier(name) => Ok(name),
            _ => Err(Error::Missing(
                "operator and special member semantic identities",
            )),
        }
    }
    pub(super) fn prefix(&mut self, name: Name, scope: ScopeId) -> Result<Option<ScopeId>, Error> {
        let name = self.tree.names.get(name);
        self.lookup_location = name.location;
        let components = self.tree.name_lists.get(name.components);
        let mut prefix = name.global.then_some(self.graph.global);
        for component in &components[..components.len().saturating_sub(1)] {
            if component.arguments.is_some() {
                return Err(Error::Missing("template specialization lookup"));
            }
            let key = Self::component_name(component.kind)?;
            let found = if let Some(prefix) = prefix {
                self.qualified(prefix, key, Filter::Prefix)?
            } else {
                self.unqualified(scope, key, Filter::Prefix)?
            };
            let binding = self.graph.binding(found.ok_or_else(|| {
                Error::at(
                    component.location,
                    "undeclared name in a nested-name-specifier",
                )
            })?);
            let entity = self.graph.entity(binding.entity);
            prefix = entity.scope;
            if prefix.is_none()
                && let Some(ty) = binding.ty
                && let TypeKind::Class(entity) | TypeKind::Enum(entity) =
                    self.graph.types.get(ty).kind
            {
                prefix = self.graph.entity(entity).scope;
            }
            if prefix.is_none() {
                return Err(Error::at(
                    component.location,
                    "name before :: is not a class, namespace or enumeration",
                ));
            }
        }
        Ok(prefix)
    }
    pub(super) fn lookup(
        &mut self,
        name: Name,
        scope: ScopeId,
        filter: Filter,
    ) -> Result<BindingId, Error> {
        let node = *self.tree.names.get(name);
        self.lookup_location = node.location;
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty semantic name"))?;
        if last.arguments.is_some() {
            return Err(Error::Missing("template specialization lookup"));
        }
        let key = Self::component_name(last.kind)?;
        let prefix = self.prefix(name, scope)?;
        let found = if let Some(prefix) = prefix {
            self.qualified(prefix, key, filter)?
        } else {
            self.unqualified(scope, key, filter)?
        };
        let binding = found.ok_or_else(|| Error::at(node.location, "undeclared name"))?;
        super::record(&mut self.graph.resolved_names, name.index(), binding);
        Ok(binding)
    }
}
