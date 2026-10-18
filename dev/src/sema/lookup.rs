// Translated from sema/lookup.cpp and analyzer_types.cpp's name_prefix. See NOTICE.
//! Searches visit only lexical parents and explicit namespace edges.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::ValueKey,
    graph::{BindingId, BindingKind, ScopeId, ScopeKind},
    types::{TypeId, TypeKind},
};
use crate::{
    lex::identifiers::NameId,
    syntax::ast::{Name, NameKind},
};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Filter {
    Ordinary,
    Prefix,
    Namespace,
    Elaborated,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Specialization {
    None,
    NonFunction,
    All,
}

// A dependent qualifier has no scope until substitution. Never manufacture a
// class scope for its placeholder or search its members speculatively.
#[derive(Clone, Copy)]
pub(super) enum Prefix {
    Scope(ScopeId),
    Dependent(TypeId),
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
    // The qualifier belongs to this query, so nested decltype lookups cannot
    // overwrite the named-type caller's class context.
    pub qualifier: Option<ScopeId>,
    pub ambiguous: bool,
    pub injected_template: bool,
}

// Query-local base results retain only a representative scope, never copies
// of the overload set. Ambiguity remains a fact until the use checks it.
#[derive(Clone, Copy, Default)]
struct BaseLookup {
    scope: Option<ScopeId>,
    ambiguous: bool,
    injected_template: bool,
}
impl LookupResult {
    fn checked_template_id(
        self,
        template_id: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<Self, Error> {
        if template_id && self.injected_template {
            Ok(self)
        } else {
            self.checked(location)
        }
    }
    fn checked(self, location: crate::support::source::SourceOffset) -> Result<Self, Error> {
        if self.ambiguous {
            Err(Error::at(location, "ambiguous name"))
        } else {
            Ok(self)
        }
    }
}
impl Analyzer<'_> {
    fn merge(&self, first: &mut LookupResult, second: &LookupResult) {
        if let Some(second_id) = second.binding {
            first.ambiguous |= second.ambiguous;
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
                    first.ambiguous = true;
                    first.injected_template = false;
                }
            } else {
                first.binding = Some(second_id);
            }
            if !second.functions.as_slice().is_empty()
                && first.functions.as_slice() != second.functions.as_slice()
            {
                first.functions = self.function_union(&first.functions, &second.functions);
            }
        }
    }
    // NameLookup::function_union: prepend the smaller list's missing
    // entities in reverse visitation order; an equal length favors second.
    #[inline(never)]
    fn function_union(
        &self,
        first: &FunctionCandidates,
        second: &FunctionCandidates,
    ) -> FunctionCandidates {
        let a = first.as_slice();
        let b = second.as_slice();
        if a.is_empty() {
            return b.iter().copied().collect();
        }
        let (larger, smaller) = if a.len() > b.len() { (a, b) } else { (b, a) };
        let mut pending: HashSet<_> = smaller
            .iter()
            .map(|&id| self.graph.binding(id).entity)
            .collect();
        for &id in larger {
            pending.remove(&self.graph.binding(id).entity);
        }
        let mut prefix = Vec::with_capacity(pending.len());
        for &id in smaller {
            if pending.remove(&self.graph.binding(id).entity) {
                prefix.push(id);
            }
        }
        prefix
            .into_iter()
            .rev()
            .chain(larger.iter().copied())
            .collect()
    }
    fn own_lookup(&self, scope: ScopeId, key: ValueKey, filter: Filter) -> LookupResult {
        let binding = self.graph.own_key(scope, key, filter);
        let functions =
            if binding.is_some_and(|id| self.graph.binding(id).kind == BindingKind::Function) {
                // NameLookup stores newest overloads at the list head.
                self.graph.visible_functions_at(scope, key).rev().collect()
            } else {
                FunctionCandidates::default()
            };
        LookupResult {
            binding,
            functions,
            qualifier: None,
            ambiguous: false,
            injected_template: false,
        }
    }
    pub(super) fn class_lookup(
        &self,
        scope: ScopeId,
        key: ValueKey,
        filter: Filter,
    ) -> Result<LookupResult, Error> {
        self.class_lookup_in(scope, key, filter, false)?
            .checked(self.lookup_location)
    }
    fn class_lookup_in(
        &self,
        scope: ScopeId,
        key: ValueKey,
        filter: Filter,
        unqualified: bool,
    ) -> Result<LookupResult, Error> {
        let own = self.own_lookup(scope, key, filter);
        if own.binding.is_some() || self.graph.scope(scope).kind != ScopeKind::Class {
            return Ok(own);
        }
        // A result names its representative declaring scope. Its overload set
        // is collected only once, after the graph walk, rather than copied at
        // every inherited level. Query memos store absence or this compact fact.
        let mut memo = HashMap::<ScopeId, BaseLookup>::new();
        let mut work = vec![(scope, 0usize, BaseLookup::default())];
        while let Some((current, next, result)) = work.last_mut() {
            let owner = self
                .graph
                .scope(*current)
                .owner
                .ok_or(Error::Invariant("class lookup without owner"))?;
            if let Some(edge) = self.graph.entity(owner).bases.get(*next) {
                // Only direct dependent bases of the queried class are skipped;
                // a fixed base's members use ordinary qualified base lookup.
                if unqualified
                    && *current == scope
                    && edge.dependency == crate::syntax::ast::BaseDependency::Dependent
                {
                    *next += 1;
                    continue;
                }
                let base = self
                    .graph
                    .entity(edge.class)
                    .scope
                    .ok_or(Error::Invariant("base without scope"))?;
                let found = if let Some(&cached) = memo.get(&base) {
                    cached
                } else {
                    let own = BaseLookup {
                        scope: self.graph.own_key(base, key, filter).map(|_| base),
                        ..BaseLookup::default()
                    };
                    if own.scope.is_none() && !self.graph.entity(edge.class).bases.is_empty() {
                        work.push((base, 0, BaseLookup::default()));
                        continue;
                    }
                    memo.insert(base, own);
                    own
                };
                self.merge_base_lookup(result, found, key, filter)?;
                *next += 1;
            } else {
                let (current, _, found) = work
                    .pop()
                    .ok_or(Error::Invariant("missing class lookup frame"))?;
                if current == scope {
                    let mut result = found.scope.map_or_else(LookupResult::default, |found| {
                        self.own_lookup(found, key, filter)
                    });
                    result.ambiguous = found.ambiguous;
                    result.injected_template = found.injected_template;
                    return Ok(result);
                }
                memo.insert(current, found);
            }
        }
        Err(Error::Invariant("class lookup without result"))
    }
    fn merge_base_lookup(
        &self,
        result: &mut BaseLookup,
        found: BaseLookup,
        key: ValueKey,
        filter: Filter,
    ) -> Result<(), Error> {
        // Match merge_base's order: identical entities propagate a subtree's
        // ambiguity; equal designated types converge only before ambiguity.
        let (Some(a), Some(b)) = (result.scope, found.scope) else {
            if result.scope.is_none() {
                *result = found;
            }
            return Ok(());
        };
        let a_id = self
            .graph
            .own_key(a, key, filter)
            .ok_or(Error::Invariant("base representative without name"))?;
        let b_id = self
            .graph
            .own_key(b, key, filter)
            .ok_or(Error::Invariant("base representative without name"))?;
        let a = self.graph.binding(a_id);
        let b = self.graph.binding(b_id);
        if a.entity == b.entity {
            if found.ambiguous {
                result.injected_template =
                    (!result.ambiguous || result.injected_template) && found.injected_template;
            }
            result.ambiguous |= found.ambiguous;
        } else if !(matches!(a.kind, BindingKind::Type | BindingKind::Alias)
            && matches!(b.kind, BindingKind::Type | BindingKind::Alias)
            && a.ty == b.ty
            && !result.ambiguous
            && !found.ambiguous)
        {
            result.injected_template = (!result.ambiguous || result.injected_template)
                && self.graph.same_template_injected_names(a_id, b_id);
            result.ambiguous = true;
        }
        Ok(())
    }
    fn inline_set(
        &self,
        scope: ScopeId,
        name: ValueKey,
        filter: Filter,
        visited: &mut HashSet<ScopeId>,
        members: &mut Vec<ScopeId>,
    ) -> LookupResult {
        let mut result = LookupResult::default();
        let mut frontier = vec![scope];
        while let Some(current) = frontier.pop() {
            if !visited.insert(current) {
                continue;
            }
            members.push(current);
            self.merge(&mut result, &self.own_lookup(current, name, filter));
            // C++ prepends inline members, then pushes that list onto a LIFO
            // frontier: the oldest member supplies the representative first.
            frontier.extend(self.graph.visible_edges(current, true).rev());
        }
        result
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
        self.qualified_declaration_result(scope, name, filter)?
            .checked(self.lookup_location)
    }
    // Source-name lookup, unlike raw declaration-set lookup, can find an
    // unknown specialization through the current instantiation (14.6.2.1p5).
    fn qualified_name_result(
        &mut self,
        scope: ScopeId,
        key: ValueKey,
        filter: Filter,
        template_id: bool,
    ) -> Result<LookupResult, Error> {
        let mut found = self
            .qualified_declaration_result(scope, key, filter)?
            .checked_template_id(template_id, self.lookup_location)?;
        if found.binding.is_none()
            && filter != Filter::Namespace
            && let Some(class) = self.graph.scope(scope).owner
            && self.graph.scope(scope).kind == ScopeKind::Class
            && self.graph.entity(class).base_dependency
                == crate::syntax::ast::BaseDependency::Dependent
            && let Some(name) = key.identifier()
        {
            let ty = self
                .graph
                .entity(class)
                .ty
                .ok_or(Error::Invariant("class without type"))?;
            found.binding = Some(self.dependent_member_binding(ty, name)?);
        }
        Ok(found)
    }
    // C++ class_specifier consumes the representative even when namespace
    // lookup marks it ambiguous. Ordinary name uses diagnose that fact.
    pub(super) fn qualified_declaration_result(
        &self,
        scope: ScopeId,
        name: ValueKey,
        filter: Filter,
    ) -> Result<LookupResult, Error> {
        if self.graph.scope(scope).kind != ScopeKind::Namespace {
            return self.class_lookup_in(scope, name, filter, false);
        }
        let mut visited = HashSet::new();
        let mut level = vec![scope];
        let mut result = LookupResult::default();
        while !level.is_empty() && result.binding.is_none() {
            let mut next = Vec::new();
            for current in level {
                let mut members = Vec::new();
                let found = self.inline_set(current, name, filter, &mut visited, &mut members);
                if found.binding.is_some() {
                    self.merge(&mut result, &found);
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
        self.unqualified_declaration_result(scope, name, filter)?
            .checked(self.lookup_location)
    }
    pub(super) fn unqualified_declaration_result(
        &self,
        scope: ScopeId,
        name: ValueKey,
        filter: Filter,
    ) -> Result<LookupResult, Error> {
        let mut visited = HashSet::new();
        let mut pending = Vec::new();
        let mut current = Some(scope);
        let mut bridged = false;
        let mut outside = false;
        while let Some(scope) = current {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::TemplateParameters {
                if outside {
                    current = node.parent;
                    continue;
                }
                bridged |= node.template_bridge;
            } else {
                outside = bridged;
            }
            let mut result = self.class_lookup_in(scope, name, filter, true)?;
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
                    self.merge(&mut result, &self.own_lookup(nominated, name, filter));
                    pending.swap_remove(index);
                } else {
                    index += 1;
                }
            }
            if result.binding.is_some() {
                if let Some(binding) = result.binding
                    && ((node.kind == ScopeKind::TemplateParameters
                        && self.graph.binding(binding).kind != BindingKind::Enumerator)
                        || self.graph.dependent_binding(binding)
                        || (node.kind == ScopeKind::Class && self.dependent_lookup_class(scope)))
                {
                    self.note_dependent_use();
                } else if node.kind == ScopeKind::TemplateParameters {
                    self.value_parameter_uses
                        .set(self.value_parameter_uses.get().wrapping_add(1));
                }
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
        self.prefix_components_in(name, scope, count, false)
    }
    pub(super) fn expression_prefix(
        &mut self,
        name: Name,
        scope: ScopeId,
    ) -> Result<Option<ScopeId>, Error> {
        let count = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .len()
            .saturating_sub(1);
        self.prefix_components_in(name, scope, count, true)
    }
    fn prefix_components_in(
        &mut self,
        name: Name,
        scope: ScopeId,
        count: usize,
        access: bool,
    ) -> Result<Option<ScopeId>, Error> {
        match self.name_prefix(name, scope, count, access)? {
            Some(Prefix::Scope(scope)) => Ok(Some(scope)),
            Some(Prefix::Dependent(_)) => Err(Error::Missing("dependent qualified name lookup")),
            None => Ok(None),
        }
    }
    pub(super) fn name_prefix(
        &mut self,
        name: Name,
        scope: ScopeId,
        count: usize,
        access: bool,
    ) -> Result<Option<Prefix>, Error> {
        let name = *self.tree.names.get(name);
        self.lookup_location = name.location;
        let component_count = self.tree.name_lists.get(name.components).len();
        let mut prefix = name.global.then_some(Prefix::Scope(self.graph.global));
        if count > component_count {
            return Err(Error::Invariant("name prefix outside components"));
        }
        for index in 0..count {
            let component = self.tree.name_lists.get(name.components)[index];
            if let NameKind::Decltype(operand) = component.kind {
                let ty = self.decltype_type(operand, scope)?;
                if self.graph.dependent_type(ty)
                    && !matches!(self.graph.types.get(ty).kind, TypeKind::Class(class)
                        if self.graph.entity(class).scope.is_some())
                {
                    prefix = Some(Prefix::Dependent(ty));
                    continue;
                }
                if !matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Class(_) | TypeKind::Enum(_)
                ) {
                    return Err(Error::at(
                        component.location,
                        "decltype in a nested-name-specifier must name a class or enumeration",
                    ));
                }
                prefix = Some(Prefix::Scope(self.type_qualifier(ty, component.location)?));
                continue;
            }
            let NameKind::Identifier(key) = component.kind else {
                return Err(Error::at(
                    component.location,
                    "invalid nested-name-specifier",
                ));
            };
            self.lookup_location = component.location;
            if let Some(Prefix::Dependent(qualifier)) = prefix
                && component.arguments.is_some()
            {
                let binding = self.dependent_template_id(qualifier, component, scope)?;
                let ty = self
                    .graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("dependent specialization without type"))?;
                prefix = Some(Prefix::Dependent(ty));
                continue;
            }
            let found = match prefix {
                Some(Prefix::Scope(prefix)) => self
                    .qualified_name_result(
                        prefix,
                        key.into(),
                        Filter::Prefix,
                        component.arguments.is_some(),
                    )
                    .map(|found| found.binding),
                Some(Prefix::Dependent(ty)) => self.dependent_member_binding(ty, key).map(Some),
                None => self
                    .unqualified_declaration_result(scope, key.into(), Filter::Prefix)
                    .and_then(|found| {
                        found.checked_template_id(
                            component.arguments.is_some(),
                            self.lookup_location,
                        )
                    })
                    .map(|found| found.binding),
            }
            .map_err(|error| match error {
                Error::Diagnostic {
                    message: "ambiguous name",
                    ..
                } => Error::at(
                    component.location,
                    "ambiguous name in a nested-name-specifier",
                ),
                other => other,
            })?;
            let mut binding_id = found.ok_or_else(|| {
                Error::at(
                    component.location,
                    "undeclared name in a nested-name-specifier",
                )
            })?;
            if access && self.semantic() && self.template_depth == 0 && prefix.is_none() {
                self.check_declaration_direct_access(binding_id, scope, component.location)?;
            }
            if let Some(Prefix::Scope(search)) = prefix
                && let Some(naming) = self.graph.scope(search).owner
                && self.graph.scope(search).kind == ScopeKind::Class
                && if access {
                    self.semantic() && self.template_depth == 0
                } else {
                    self.graph.binding_scope(binding_id) != search
                }
            {
                self.check_declaration_member_access(
                    binding_id,
                    naming,
                    None,
                    scope,
                    component.location,
                )?;
            }
            if component.arguments.is_some() {
                binding_id = self
                    .class_template_id(binding_id, component, scope)?
                    .ok_or_else(|| {
                        Error::at(
                            component.location,
                            "a template-id in a nested-name-specifier that names no class template",
                        )
                    })?;
            }
            let binding = self.graph.binding(binding_id);
            prefix = Some(
                if matches!(binding.kind, BindingKind::Type | BindingKind::Alias)
                    && let Some(ty) = binding.ty
                    && self.graph.dependent_type(ty)
                    && match self.graph.types.get(ty).kind {
                        TypeKind::Class(class) => self.graph.entity(class).scope.is_none(),
                        TypeKind::Enum(_) => false,
                        _ => true,
                    }
                {
                    Prefix::Dependent(ty)
                } else {
                    Prefix::Scope(self.binding_qualifier(binding_id, component.location)?)
                },
            );
        }
        Ok(prefix)
    }
    pub(super) fn binding_qualifier(
        &mut self,
        binding: BindingId,
        location: crate::support::source::SourceOffset,
    ) -> Result<ScopeId, Error> {
        let binding = self.graph.binding(binding);
        match binding.kind {
            BindingKind::Namespace | BindingKind::NamespaceAlias => self
                .graph
                .entity(binding.entity)
                .scope
                .ok_or(Error::Invariant("namespace qualifier without scope")),
            BindingKind::Type | BindingKind::Alias => self.type_qualifier(
                binding
                    .ty
                    .ok_or(Error::Invariant("type qualifier without type"))?,
                location,
            ),
            _ => Err(Error::at(
                location,
                "name before :: is not a class, namespace or enumeration",
            )),
        }
    }
    fn type_qualifier(
        &mut self,
        ty: super::types::TypeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<ScopeId, Error> {
        match self.graph.types.get(ty).kind {
            TypeKind::Class(class) => {
                self.complete_class(class)?;
                self.graph.entity(class).scope.ok_or_else(|| {
                    Error::at(location, "incomplete class in a nested-name-specifier")
                })
            }
            TypeKind::Enum(enumeration) => self
                .graph
                .entity(enumeration)
                .scope
                .ok_or(Error::Invariant("enumeration qualifier without scope")),
            TypeKind::TemplateParameter { .. } => {
                Err(Error::Missing("dependent qualified name lookup"))
            }
            _ => Err(Error::at(
                location,
                "name before :: is not a class, namespace or enumeration",
            )),
        }
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
        self.lookup_result_in(name, scope, filter, Specialization::All)
    }
    pub(super) fn lookup_call_result(
        &mut self,
        name: Name,
        scope: ScopeId,
    ) -> Result<LookupResult, Error> {
        // Function template candidates wait for argument-dependent lookup and
        // deduction. A variable or class template-id already denotes its
        // specialization before the call checks whether that value is callable.
        self.lookup_result_in(name, scope, Filter::Ordinary, Specialization::NonFunction)
    }
    pub(super) fn lookup_result_in(
        &mut self,
        name: Name,
        scope: ScopeId,
        filter: Filter,
        specialize: Specialization,
    ) -> Result<LookupResult, Error> {
        let node = *self.tree.names.get(name);
        self.lookup_location = node.location;
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .copied()
            .ok_or(Error::Invariant("empty semantic name"))?;
        // Types-mode lookup_name accepts identifiers only; semantics has the
        // separate operator_function_lookup path in the frozen owner.
        if !self.semantic() && !matches!(last.kind, NameKind::Identifier(_)) {
            return Err(Error::at(
                node.location,
                "operator and special member names are not supported here",
            ));
        }
        if self.semantic()
            && last.arguments.is_none()
            && let NameKind::Conversion(target) = last.kind
        {
            return self.conversion_name_lookup(name, target, scope);
        }
        let key = Self::expression_name(last.kind)?;
        let count = self
            .tree
            .name_lists
            .get(node.components)
            .len()
            .saturating_sub(1);
        let prefix = match self.name_prefix(name, scope, count, true)? {
            Some(Prefix::Dependent(qualifier)) => {
                let binding = if filter == Filter::Namespace {
                    None
                } else if last.arguments.is_some() && node.typename_keyword {
                    Some(self.dependent_template_id(qualifier, last, scope)?)
                } else if let Some(name) = key.identifier() {
                    Some(self.dependent_member_binding(qualifier, name)?)
                } else {
                    return Err(Error::Missing("dependent special member name"));
                };
                return Ok(LookupResult {
                    binding,
                    ..LookupResult::default()
                });
            }
            Some(Prefix::Scope(scope)) => Some(scope),
            None => None,
        };
        self.prepare_assignment_lookup(prefix.unwrap_or(scope), key, prefix.is_none())?;
        self.lookup_location = node.location;
        let mut found = if let Some(prefix) = prefix {
            self.qualified_name_result(prefix, key, filter, last.arguments.is_some())?
        } else {
            self.unqualified_declaration_result(scope, key, filter)?
                .checked_template_id(last.arguments.is_some(), self.lookup_location)?
        };
        if filter == Filter::Ordinary
            && found.binding.is_none()
            && prefix.is_none_or(|prefix| prefix == self.graph.global)
            && let Some(identifier) = key.identifier()
            && let Some(builtin) = self.builtin_lookup(identifier)?
        {
            found = builtin;
        }
        if let Some(binding) = found.binding {
            if let Some(search) = prefix
                && let Some(naming) = self.graph.scope(search).owner
                && self.graph.scope(search).kind == ScopeKind::Class
                && self.graph.binding_scope(binding) != search
            {
                self.check_declaration_member_access(binding, naming, None, scope, node.location)?;
            }
            if self.semantic() && self.graph.binding(binding).kind != BindingKind::Function {
                if let Some(search) = prefix
                    && self.graph.scope(search).kind == ScopeKind::Class
                    && (self.template_depth == 0 || self.instance_member_template_scope(scope))
                    && let Some(class) = self.graph.scope(search).owner
                {
                    self.check_declaration_member_access(
                        binding,
                        class,
                        None,
                        scope,
                        node.location,
                    )?;
                } else if self.template_depth == 0 {
                    self.check_declaration_direct_access(binding, scope, node.location)?;
                }
            }
        }
        if specialize != Specialization::None && last.arguments.is_some() {
            if found.functions.as_slice().is_empty()
                && let Some(binding) = found.binding
            {
                found.binding = Some(
                    if let Some(variable) = self.variable_template_binding(binding, last, scope)? {
                        variable
                    } else {
                        self.class_template_id(binding, last, scope)?
                            .ok_or_else(|| {
                                Error::at(
                                    node.location,
                                    "a template-id that names no class or variable template",
                                )
                            })?
                    },
                );
            } else if self.semantic() && specialize == Specialization::All {
                self.explicit_function_specializations(last, &mut found, scope, false)?;
            } else if !self.semantic() && found.functions.as_slice().is_empty() {
                return Err(Error::Missing("template specialization lookup"));
            }
        }
        if let Some(binding) = found.binding {
            super::record(&mut self.graph.resolved_names, name.index(), binding);
        }
        found.qualifier = prefix;
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::{Analyzer, Filter};
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{function_demands::Demand, layout::LayoutState, types::TypeKind},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn injected_template_ambiguity_keeps_a_canonical_representative_and_lazy_bodies() {
        let source = SourceBuffer::new(b"template<class T>struct B{void unused(){typename T::missing x;}};struct D:B<int>,B<char>{};struct E:D,B<long>{};struct X{using B=::B<int>;};struct Y{using B=::B<char>;};struct Z:X,Y{};struct M:D,X{};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"injected-lookup").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let key = cursor.intern_name(b"B").unwrap();
        let names = [b"D", b"E", b"Z", b"M"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.set_lowering();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let primary = analyzer
            .graph
            .binding(analyzer.graph.own(global, key, Filter::Prefix).unwrap())
            .entity;
        let mut representative = None;
        for (index, name) in names.into_iter().enumerate() {
            let class = analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity;
            let scope = analyzer.graph.entity(class).scope.unwrap();
            let result = analyzer
                .qualified_declaration_result(scope, key.into(), Filter::Prefix)
                .unwrap();
            assert!(result.ambiguous);
            assert_eq!(result.injected_template, index < 2);
            assert!(analyzer.qualified(scope, key, Filter::Prefix).is_err());
            let binding = result.binding.unwrap();
            if index < 2 {
                let entity = analyzer.graph.binding(binding).entity;
                assert_eq!(
                    analyzer.graph.binding_scope(binding),
                    analyzer.graph.entity(entity).scope.unwrap()
                );
                assert_eq!(
                    analyzer
                        .graph
                        .class_templates
                        .instance(entity)
                        .unwrap()
                        .primary,
                    primary
                );
                if let Some(previous) = representative {
                    assert_eq!(binding, previous);
                }
                representative = Some(binding);
            }
        }
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn decltype_qualifier_keeps_original_call_facts_without_emission_or_layout_demand() {
        let source = SourceBuffer::new(b"struct S{struct T{static int g(){return 7;}};};S make();int f(){return decltype(make())::T::g();}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"qualifier-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(count, tree.expressions.mark());
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Call { callee, .. } = tree.expressions.get(call).kind else {
            panic!("call");
        };
        let ast::ExprKind::Id(name) = tree.expressions.get(callee).kind else {
            panic!("name");
        };
        let components = tree.name_lists.get(tree.names.get(name).components);
        let ast::NameKind::Decltype(operand) = components[0].kind else {
            panic!("decltype");
        };
        let class_name = cursor.intern_name(b"S").unwrap();
        let class = graph
            .binding(graph.own(graph.global, class_name, Filter::Prefix).unwrap())
            .entity;
        let facts = graph.expression_facts.get(operand).unwrap();
        assert!(matches!(graph.types.get(facts.ty).kind, TypeKind::Class(owner) if owner == class));
        assert!(facts.conversion.is_none());
        assert!(matches!(
            graph.entity(class).layout,
            LayoutState::NotStarted
        ));
        let make = graph.call_binding(operand).unwrap();
        assert!(!graph.root_demands.as_slice().iter().any(|demand| {
            matches!(demand, Demand::Function{binding,..} if graph.binding(*binding).entity == graph.binding(make).entity)
        }));
        let selected = graph.call_binding(call).unwrap();
        assert_eq!(graph.resolved_name(name), Some(selected));
        assert!(graph.root_demands.as_slice().iter().any(|demand| {
            matches!(demand, Demand::Function{binding,..} if graph.binding(*binding).entity == graph.binding(selected).entity)
        }));
    }

    #[test]
    fn qualified_class_completion_preserves_inline_namespace_entity_and_home() {
        let source = SourceBuffer::new(
            b"namespace N{inline namespace I{struct X;}}struct N::X{int n;};".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inline-completion").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        let namespace = cursor.intern_name(b"N").unwrap();
        let namespace = graph
            .binding(
                graph
                    .own(graph.global, namespace, Filter::Namespace)
                    .unwrap(),
            )
            .entity;
        let outer = graph.entity(namespace).scope.unwrap();
        let inline = cursor.intern_name(b"I").unwrap();
        let inline = graph
            .binding(graph.own(outer, inline, Filter::Namespace).unwrap())
            .entity;
        let inner = graph.entity(inline).scope.unwrap();
        let class = cursor.intern_name(b"X").unwrap();
        let forward = graph.own(inner, class, Filter::Elaborated).unwrap();
        let complete = graph.own(outer, class, Filter::Elaborated).unwrap();
        let entity = graph.binding(forward).entity;
        assert_eq!(entity, graph.binding(complete).entity);
        assert_eq!(graph.entity(entity).home, Some(inner));
        assert_eq!(
            graph.scope(graph.entity(entity).scope.unwrap()).parent,
            Some(outer)
        );
        assert!(graph.entity(entity).complete);
        assert!(matches!(
            graph.entity(entity).layout,
            LayoutState::NotStarted
        ));
    }

    #[test]
    fn class_head_keeps_the_first_inline_representative_of_an_ambiguous_set() {
        let source = SourceBuffer::new(b"namespace N{inline namespace I{struct X;}inline namespace J{struct X;}}struct N::X{};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"class-representative").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        let namespace = cursor.intern_name(b"N").unwrap();
        let namespace = graph
            .binding(
                graph
                    .own(graph.global, namespace, Filter::Namespace)
                    .unwrap(),
            )
            .entity;
        let outer = graph.entity(namespace).scope.unwrap();
        let class_name = cursor.intern_name(b"X").unwrap();
        for (name, completed) in [(b"I".as_slice(), true), (b"J".as_slice(), false)] {
            let name = cursor.intern_name(name).unwrap();
            let namespace = graph
                .binding(graph.own(outer, name, Filter::Namespace).unwrap())
                .entity;
            let home = graph.entity(namespace).scope.unwrap();
            let class = graph
                .binding(graph.own(home, class_name, Filter::Elaborated).unwrap())
                .entity;
            assert_eq!(graph.entity(class).complete, completed);
        }
    }
}
