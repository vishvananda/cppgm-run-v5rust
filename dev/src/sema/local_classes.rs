// Translated from sema/analyzer_classes.cpp::new_class/local_number. See NOTICE.
//! Declaration ordinals and indexed ownership of function-local classes.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, Graph, ScopeId, ScopeKind},
};
use crate::lex::identifiers::NameId;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct LocalClasses {
    counts: HashMap<(EntityId, NameId), u32>,
    declared: HashMap<EntityId, u16>,
    by_outer_function: HashMap<EntityId, Vec<EntityId>>,
}
impl Analyzer<'_> {
    pub(super) fn record_local_class(
        &mut self,
        class: EntityId,
        home: ScopeId,
    ) -> Result<(), Error> {
        if !matches!(
            self.graph.scope(home).kind,
            ScopeKind::Block | ScopeKind::Function
        ) {
            return Ok(());
        }
        let Some(function) = self.body_context.as_ref().map(|body| body.entity) else {
            return Ok(());
        };
        if let Some(name) = self.graph.entity(class).name {
            let count = self
                .graph
                .local_classes
                .counts
                .entry((function, name))
                .or_default();
            let earlier = u16::try_from(*count).unwrap_or(u16::MAX);
            *count = count
                .checked_add(1)
                .ok_or(Error::Limit("local class count limit"))?;
            self.graph.local_classes.declared.insert(class, earlier);
        }
        let outer = self.graph.outermost_function(function);
        self.graph
            .local_classes
            .by_outer_function
            .entry(outer)
            .or_default()
            .push(class);
        Ok(())
    }
}
impl Graph {
    fn lower_inline_function(&self, function: EntityId) -> bool {
        let node = self.entity(function);
        node.inline == super::specifiers::InlinePreference::Inline
            || node.constexpr
            || node.definition_emission == super::function_demands::DefinitionEmission::Demanded
                && node
                    .home
                    .is_some_and(|home| self.scope(home).kind == ScopeKind::Class)
    }

    pub(crate) fn lower_visible_type(&self, class: EntityId) -> bool {
        if let Some(function) = self
            .entity(class)
            .home
            .and_then(|home| self.lower_scope_function(home))
        {
            return self.lower_inline_function(self.outermost_function(function));
        }
        !self.lower_entity_unit_local(class)
    }
    pub(super) fn lower_shared_local(&self, mut class: EntityId) -> bool {
        loop {
            let node = self.entity(class);
            if node.name.is_none() || node.template_head.is_some() {
                return false;
            }
            let Some(home) = node.home else {
                return false;
            };
            if self.scope(home).kind == ScopeKind::Class {
                let Some(owner) = self.scope(home).owner else {
                    return false;
                };
                class = owner;
                continue;
            }
            let Some(function) = self.lower_local_scope(home) else {
                return false;
            };
            let node = self.entity(function);
            if !self.lower_inline_function(function)
                || node.function.is_none_or(|facts| facts.internal_linkage)
            {
                return false;
            }
            if node
                .function
                .is_some_and(|facts| facts.language != super::linkage::Language::C)
                && let Some(ty) = node.ty
                && let super::types::TypeKind::Function { ref parameters, .. } =
                    self.types.get(ty).kind
                && parameters.iter().any(|&ty| self.lower_type_unit_local(ty))
            {
                return false;
            }
            let Some(owner) = node
                .home
                .filter(|&home| self.scope(home).kind == ScopeKind::Class)
                .and_then(|home| self.scope(home).owner)
            else {
                return true;
            };
            if !self.lower_entity_unit_local(owner) {
                return true;
            }
            class = owner;
        }
    }

    pub(super) fn lower_function_unit_local(&self, function: EntityId) -> Result<bool, Error> {
        let node = self.entity(function);
        let facts = node
            .function
            .ok_or(Error::Invariant("function locality without function"))?;
        if facts.internal_linkage
            || node
                .home
                .filter(|&home| self.scope(home).kind == ScopeKind::Class)
                .and_then(|home| self.scope(home).owner)
                .is_some_and(|class| {
                    self.lower_entity_unit_local(class) && !self.lower_shared_local(class)
                })
        {
            return Ok(true);
        }
        if facts.language == super::linkage::Language::C {
            return Ok(false);
        }
        let ty = node
            .ty
            .ok_or(Error::Invariant("function locality without type"))?;
        let super::types::TypeKind::Function {
            result,
            ref parameters,
            ..
        } = self.types.get(ty).kind
        else {
            return Err(Error::Invariant("function locality without signature"));
        };
        if parameters.iter().any(|&ty| self.lower_type_unit_local(ty)) {
            return Ok(true);
        }
        let mut bare = result;
        while let super::types::TypeKind::Pointer(element)
        | super::types::TypeKind::Reference { element, .. }
        | super::types::TypeKind::Array { element, .. } = self.types.get(bare).kind
        {
            bare = element;
        }
        if let super::types::TypeKind::Class(class) | super::types::TypeKind::Enum(class) =
            self.types.get(bare).kind
            && (self.lower_shared_local(class)
                || self
                    .entity(class)
                    .home
                    .and_then(|home| self.lower_scope_function(home))
                    == Some(function))
        {
            return Ok(false);
        }
        Ok(self.lower_type_unit_local(result))
    }

    pub(crate) fn lower_function_local_class(&self, function: EntityId) -> bool {
        self.entity(function).home.is_some_and(|home| {
            self.scope(home).kind == ScopeKind::Class && self.lower_scope_function(home).is_some()
        })
    }
    pub(crate) fn lower_scope_function(&self, mut scope: ScopeId) -> Option<EntityId> {
        loop {
            let node = self.scope(scope);
            if node.kind == ScopeKind::Function {
                return node.owner;
            }
            scope = node.parent?;
        }
    }
    fn outermost_function(&self, mut function: EntityId) -> EntityId {
        while let Some(outer) = self
            .entity(function)
            .home
            .and_then(|home| self.lower_scope_function(home))
        {
            function = outer;
        }
        function
    }
    pub(crate) fn lower_local_scope(&self, scope: ScopeId) -> Option<EntityId> {
        matches!(
            self.scope(scope).kind,
            ScopeKind::Block | ScopeKind::Function
        )
        .then(|| self.lower_scope_function(scope))
        .flatten()
    }
    pub(crate) fn lower_local_class_scope(
        &self,
        scope: ScopeId,
    ) -> Option<(EntityId, EntityId, u32)> {
        let node = self.scope(scope);
        if node.kind != ScopeKind::Class {
            return None;
        }
        let parent = node.parent?;
        let function = self.lower_local_scope(parent)?;
        let class = node.owner?;
        Some((
            class,
            function,
            u32::from(
                self.local_classes
                    .declared
                    .get(&class)
                    .copied()
                    .unwrap_or(0),
            ),
        ))
    }
    pub(crate) fn lower_local_type(&self, class: EntityId) -> Option<(EntityId, u32)> {
        let function = self.lower_local_scope(self.entity(class).home?)?;
        Some((
            function,
            u32::from(
                self.local_classes
                    .declared
                    .get(&class)
                    .copied()
                    .unwrap_or(0),
            ),
        ))
    }
    pub(crate) fn lower_function_visible_locals(&self, function: EntityId) -> bool {
        let outer = self.outermost_function(function);
        self.lower_inline_function(outer)
    }
    pub(crate) fn lower_deferred_local_classes(&self, function: EntityId) -> &[EntityId] {
        self.local_classes
            .by_outer_function
            .get(&function)
            .map_or(&[], Vec::as_slice)
    }
    pub(crate) fn lower_local_class_definition_scope(&self, class: EntityId) -> Option<ScopeId> {
        let node = self.entity(class);
        let scope = node.scope?;
        self.scope(scope)
            .bindings
            .iter()
            .any(|&binding| {
                let binding = self.binding(binding);
                let entity = self.entity(binding.entity);
                binding.kind == super::graph::BindingKind::Function
                    && entity.home == Some(scope)
                    && entity.function_definition.is_some()
            })
            .then_some(scope)
    }
}
