// Translated from sema/lookup.cpp and analyzer_types.cpp's name_prefix. See NOTICE.
//! Searches visit only lexical parents and explicit namespace edges.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::ValueKey,
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
#[derive(Default)]
pub(super) enum FunctionCandidates {
    #[default]
    Empty,
    One(BindingId),
    Many(Vec<BindingId>),
}
impl FunctionCandidates {
    pub(super) fn as_slice(&self) -> &[BindingId] {
        match self {
            Self::Empty => &[],
            Self::One(binding) => std::slice::from_ref(binding),
            Self::Many(bindings) => bindings,
        }
    }
    pub(super) fn iter(&self) -> std::slice::Iter<'_, BindingId> {
        self.as_slice().iter()
    }
    pub(super) fn push(&mut self, binding: BindingId) {
        match self {
            Self::Empty => *self = Self::One(binding),
            Self::One(first) => *self = Self::Many(vec![*first, binding]),
            Self::Many(bindings) => bindings.push(binding),
        }
    }
}
impl FromIterator<BindingId> for FunctionCandidates {
    fn from_iter<T: IntoIterator<Item = BindingId>>(values: T) -> Self {
        let mut result = Self::default();
        for binding in values {
            result.push(binding);
        }
        result
    }
}
#[derive(Default)]
pub(super) struct LookupResult {
    pub binding: Option<BindingId>,
    pub functions: FunctionCandidates,
}
impl Analyzer<'_> {
    fn merge(&self, first: &mut LookupResult, second: &LookupResult) -> Result<(), Error> {
        if let Some(second_id) = second.binding {
            if let Some(first_id) = first.binding {
                let a = self.graph.binding(first_id);
                let b = self.graph.binding(second_id);
                if a.entity != b.entity
                    && !(a.kind == BindingKind::Function && b.kind == BindingKind::Function)
                    && !(matches!(a.kind, BindingKind::Type | BindingKind::Alias)
                        && matches!(b.kind, BindingKind::Type | BindingKind::Alias)
                        && a.ty == b.ty)
                    && !(matches!(a.kind, BindingKind::Namespace | BindingKind::NamespaceAlias)
                        && matches!(b.kind, BindingKind::Namespace | BindingKind::NamespaceAlias)
                        && self.graph.entity(a.entity).scope == self.graph.entity(b.entity).scope)
                {
                    return Err(Error::at(self.lookup_location, "ambiguous name"));
                }
            } else {
                first.binding = Some(second_id);
            }
            for &binding in second.functions.as_slice() {
                let entity = self.graph.binding(binding).entity;
                if !first
                    .functions
                    .iter()
                    .any(|&id| self.graph.binding(id).entity == entity)
                {
                    first.functions.push(binding);
                }
            }
        }
        Ok(())
    }
    fn own_lookup(&self, scope: ScopeId, key: ValueKey, filter: Filter) -> LookupResult {
        let binding = self.graph.own_key(scope, key, filter);
        let functions =
            if binding.is_some_and(|id| self.graph.binding(id).kind == BindingKind::Function) {
                self.graph.functions_at(scope, key).collect()
            } else {
                FunctionCandidates::default()
            };
        LookupResult { binding, functions }
    }
    fn inline_set(
        &self,
        scope: ScopeId,
        name: ValueKey,
        filter: Filter,
        visited: &mut HashSet<ScopeId>,
        members: &mut Vec<ScopeId>,
    ) -> Result<LookupResult, Error> {
        let mut result = LookupResult::default();
        let mut frontier = vec![scope];
        while let Some(current) = frontier.pop() {
            if !visited.insert(current) {
                continue;
            }
            members.push(current);
            self.merge(&mut result, &self.own_lookup(current, name, filter))?;
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
        self.qualified_key(scope, name.into(), filter)
    }
    pub(super) fn qualified_key(
        &self,
        scope: ScopeId,
        name: ValueKey,
        filter: Filter,
    ) -> Result<Option<BindingId>, Error> {
        Ok(self.qualified_result(scope, name, filter)?.binding)
    }
    pub(super) fn qualified_result(
        &self,
        scope: ScopeId,
        name: ValueKey,
        filter: Filter,
    ) -> Result<LookupResult, Error> {
        if self.graph.scope(scope).kind != ScopeKind::Namespace {
            return Ok(self.own_lookup(scope, name, filter));
        }
        let mut visited = HashSet::new();
        let mut level = vec![scope];
        let mut result = LookupResult::default();
        while !level.is_empty() && result.binding.is_none() {
            let mut next = Vec::new();
            for current in level {
                let mut members = Vec::new();
                let found = self.inline_set(current, name, filter, &mut visited, &mut members)?;
                if found.binding.is_some() {
                    self.merge(&mut result, &found)?;
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
        Ok(self.unqualified_result(scope, name.into(), filter)?.binding)
    }
    pub(super) fn unqualified_result(
        &self,
        scope: ScopeId,
        name: ValueKey,
        filter: Filter,
    ) -> Result<LookupResult, Error> {
        let mut visited = HashSet::new();
        let mut pending = Vec::new();
        let mut current = Some(scope);
        while let Some(scope) = current {
            let mut result = self.own_lookup(scope, name, filter);
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
                    self.merge(&mut result, &self.own_lookup(nominated, name, filter))?;
                    pending.swap_remove(index);
                } else {
                    index += 1;
                }
            }
            if result.binding.is_some() {
                return Ok(result);
            }
            current = self.graph.scope(scope).parent;
        }
        Ok(LookupResult::default())
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
        let count = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .len()
            .saturating_sub(1);
        self.prefix_components(name, scope, count)
    }
    pub(super) fn prefix_components(
        &mut self,
        name: Name,
        scope: ScopeId,
        count: usize,
    ) -> Result<Option<ScopeId>, Error> {
        let name = self.tree.names.get(name);
        self.lookup_location = name.location;
        let components = self.tree.name_lists.get(name.components);
        let mut prefix = name.global.then_some(self.graph.global);
        for component in components
            .get(..count)
            .ok_or(Error::Invariant("name prefix outside components"))?
        {
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
                if binding.ty.is_some_and(|ty| {
                    matches!(
                        self.graph.types.get(ty).kind,
                        TypeKind::TemplateParameter { .. }
                    )
                }) {
                    return Err(Error::Missing("dependent qualified name lookup"));
                }
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
        self.lookup_result(name, scope, filter)?
            .binding
            .ok_or_else(|| Error::at(self.tree.names.get(name).location, "undeclared name"))
    }
    pub(super) fn lookup_result(
        &mut self,
        name: Name,
        scope: ScopeId,
        filter: Filter,
    ) -> Result<LookupResult, Error> {
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
        let NameKind::Identifier(key) = last.kind else {
            return Err(Error::at(
                node.location,
                "operator and special member names are not supported here",
            ));
        };
        let prefix = self.prefix(name, scope)?;
        let found = if let Some(prefix) = prefix {
            self.qualified_result(prefix, key.into(), filter)?
        } else {
            self.unqualified_result(scope, key.into(), filter)?
        };
        if let Some(binding) = found.binding {
            super::record(&mut self.graph.resolved_names, name.index(), binding);
        }
        Ok(found)
    }
}
