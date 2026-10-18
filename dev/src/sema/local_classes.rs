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
    unnamed_counts: HashMap<EntityId, u32>,
    unnamed: HashMap<EntityId, u16>,
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
        self.record_unnamed_local_type(class, home)?;
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
    pub(super) fn record_unnamed_local_type(
        &mut self,
        entity: EntityId,
        home: ScopeId,
    ) -> Result<(), Error> {
        if self.graph.entity(entity).name.is_none()
            && matches!(
                self.graph.scope(home).kind,
                ScopeKind::Block | ScopeKind::Function
            )
            && let Some(function) = self.body_context.as_ref().map(|body| body.entity)
        {
            let count = self
                .graph
                .local_classes
                .unnamed_counts
                .entry(function)
                .or_default();
            let earlier = u16::try_from(*count).unwrap_or(u16::MAX);
            self.graph.local_classes.unnamed.insert(entity, earlier);
            *count = count
                .checked_add(1)
                .ok_or(Error::Limit("unnamed local type count"))?;
        }
        Ok(())
    }
}
impl Graph {
    pub(super) fn unnamed_local_ordinal(&self, entity: EntityId) -> Option<u32> {
        self.local_classes
            .unnamed
            .get(&entity)
            .copied()
            .map(u32::from)
    }
    fn lower_inline_function(&self, function: EntityId) -> bool {
        let node = self.entity(function);
        node.inline == super::specifiers::InlinePreference::Inline
            || node.constexpr
            || node.definition_emission == super::function_demands::DefinitionEmission::Demanded
                && node
                    .home
                    .is_some_and(|home| self.scope(home).kind == ScopeKind::Class)
    }

    pub(crate) fn lower_visible_type(&self, class: EntityId) -> Result<bool, Error> {
        if let Some(function) = self
            .entity(class)
            .home
            .and_then(|home| self.lower_scope_function(home))
        {
            return Ok(self.lower_inline_function(self.outermost_function(function)));
        }
        Ok(!self.lower_entity_unit_local(class)?)
    }
    pub(super) fn lower_shared_local(&self, mut class: EntityId) -> Result<bool, Error> {
        loop {
            let node = self.entity(class);
            if node.name.is_none() && self.closure_call(class).is_none()
                || node.template_head.is_some()
            {
                return Ok(false);
            }
            let Some(home) = node.home else {
                return Ok(false);
            };
            if self.scope(home).kind == ScopeKind::Class {
                let Some(owner) = self.scope(home).owner else {
                    return Ok(false);
                };
                class = owner;
                continue;
            }
            let Some(function) = self.lower_local_scope(home) else {
                return Ok(false);
            };
            let node = self.entity(function);
            if !self.lower_inline_function(function)
                || node.function.is_none_or(|facts| facts.internal_linkage)
            {
                return Ok(false);
            }
            if node
                .function
                .is_some_and(|facts| facts.language != super::linkage::Language::C)
                && let Some(ty) = node.ty
                && let super::types::TypeKind::Function { ref parameters, .. } =
                    self.types.get(ty).kind
                && self.lower_types_unit_local(parameters)?
            {
                return Ok(false);
            }
            let Some(owner) = node
                .home
                .filter(|&home| self.scope(home).kind == ScopeKind::Class)
                .and_then(|home| self.scope(home).owner)
            else {
                return Ok(true);
            };
            if !self.lower_entity_unit_local(owner)? {
                return Ok(true);
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
            || self.lower_function_arguments_unit_local(function)?
            || node
                .home
                .filter(|&home| self.scope(home).kind == ScopeKind::Class)
                .and_then(|home| self.scope(home).owner)
                .map(|class| {
                    Ok(self.lower_entity_unit_local(class)? && !self.lower_shared_local(class)?)
                })
                .transpose()?
                .unwrap_or(false)
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
        if self.lower_types_unit_local(parameters)? {
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
            && (self.lower_shared_local(class)?
                || self
                    .entity(class)
                    .home
                    .and_then(|home| self.lower_scope_function(home))
                    == Some(function))
        {
            return Ok(false);
        }
        self.lower_type_unit_local(result)
    }

    fn lower_types_unit_local(&self, types: &[super::types::TypeId]) -> Result<bool, Error> {
        for &ty in types {
            if self.lower_type_unit_local(ty)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn lower_function_arguments_unit_local(
        &self,
        function: EntityId,
    ) -> Result<bool, Error> {
        // symbol_lowering.cpp::argument_locality: only this specialization's
        // canonical arguments determine its argument locality.
        self.function_templates
            .arguments
            .get(&function)
            .map(|arguments| self.lower_types_unit_local(arguments))
            .transpose()
            .map(|local| local.unwrap_or(false))
    }

    pub(crate) fn lower_class_no_linkage(&self, class: EntityId) -> Result<bool, Error> {
        self.entity(class)
            .ty
            .map(|ty| {
                self.lower_type_entity_locality(
                    ty,
                    Self::lower_entity_scope_no_linkage,
                    |graph, entity| Ok(graph.lower_entity_scope_no_linkage(entity)),
                )
            })
            .transpose()
            .map(|local| local.unwrap_or(false))
    }
    pub(super) fn lower_entity_scope_no_linkage(&self, mut class: EntityId) -> bool {
        loop {
            let entity = self.entity(class);
            let Some(home) = entity.home else {
                return true;
            };
            let scope = self.scope(home);
            if scope.kind != ScopeKind::Class {
                return entity.name.is_none()
                    || matches!(scope.kind, ScopeKind::Block | ScopeKind::Function);
            }
            let Some(owner) = scope.owner else {
                return true;
            };
            class = owner;
        }
    }
    pub(crate) fn lower_function_local_class(&self, function: EntityId) -> Result<bool, Error> {
        self.entity(function)
            .home
            .filter(|&home| self.scope(home).kind == ScopeKind::Class)
            .and_then(|home| self.scope(home).owner)
            .map(|owner| self.lower_class_no_linkage(owner))
            .transpose()
            .map(|local| local.unwrap_or(false))
    }
    pub(crate) fn lower_unnamed_member_owner(&self, function: EntityId) -> Option<ScopeId> {
        let home = self.entity(function).home?;
        if self.scope(home).kind != ScopeKind::Class {
            return None;
        }
        let mut scope = home;
        loop {
            let node = self.scope(scope);
            let owner = self.entity(node.owner?);
            if owner.name.is_none() {
                return Some(home);
            }
            scope = owner.home?;
            if self.scope(scope).kind != ScopeKind::Class {
                return None;
            }
        }
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
    pub(crate) fn lower_numbered_closure(&self, class: EntityId) -> bool {
        let mut function = self
            .lower_closure(class)
            .and_then(|closure| closure.enclosing);
        while let Some(current) = function {
            if self.lower_inline_function(current)
                || self.function_templates.primary(current).is_some()
            {
                return true;
            }
            function = self
                .entity(current)
                .home
                .and_then(|home| self.lower_scope_function(home));
        }
        false
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
