// Translated from sema/analyzer_declarations.cpp. See NOTICE.
//! Indexed linkage obligations and block declarations, separate from source lookup.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::SpecialName,
    graph::{BindingKind, EntityId, ScopeId, ScopeKind},
    types::{TypeId, TypeKind},
};
use crate::{lex::identifiers::NameId, support::source::SourceOffset};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Language {
    C,
    Cpp,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Specification {
    Block,
    Single,
}
#[derive(Clone, Copy)]
pub(super) struct Declaration {
    pub home: ScopeId,
    pub name: super::function_names::ValueKey,
    pub ty: TypeId,
    pub kind: BindingKind,
    pub templated: bool,
    pub location: SourceOffset,
}
impl Language {
    pub(super) fn from_c(c: bool) -> Self {
        if c { Self::C } else { Self::Cpp }
    }
}
#[derive(Clone, Copy)]
struct BlockDeclaration {
    entity: EntityId,
    kind: BindingKind,
    language: Language,
}
#[derive(Default)]
pub(super) struct BlockLinkages {
    records: HashMap<(ScopeId, NameId), Vec<BlockDeclaration>>,
}

impl Analyzer<'_> {
    pub(super) fn complete_linked_variable(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let old = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("linked variable without type"))?;
        let completed = self
            .graph
            .types
            .complete_array(old, ty)
            .ok_or_else(|| Error::at(location, "conflicting declaration of a variable"))?;
        self.graph.entity_mut(entity).ty = Some(completed);
        Ok(())
    }
    pub(super) fn initialize_linkage(
        &mut self,
        entity: EntityId,
        specs: super::specifiers::Specifiers,
        declaration: Declaration,
    ) -> Result<(), Error> {
        let Declaration {
            home,
            name,
            ty,
            kind,
            templated,
            location,
        } = declaration;
        if kind == BindingKind::Variable {
            let namespace = self.graph.scope(home).kind == ScopeKind::Namespace;
            let internal = namespace
                && (specs.is_static
                    || !self.c_language && self.internal_namespace(home)
                    || self.type_cv(ty).0 & 1 != 0
                        && !specs.is_extern
                        && !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }));
            let language = super::linkage::Language::from_c(
                self.c_language && self.graph.scope(home).kind != ScopeKind::Class,
            );
            self.graph.entity_mut(entity).variable = Some(super::graph::VariableProperties {
                is_extern: specs.is_extern,
                internal_linkage: internal,
                language,
                thread_storage: specs.thread_storage,
                static_specifier: if specs.is_static {
                    super::specifiers::StaticSpecifier::Present
                } else {
                    super::specifiers::StaticSpecifier::Absent
                },
                automatic: self.graph.scope(home).kind == ScopeKind::Block
                    && !specs.is_static
                    && !specs.is_extern
                    && specs.thread_storage == super::specifiers::ThreadStorage::Ordinary,
            });
            if let Some(name) = name.identifier() {
                if namespace {
                    self.agree_block_linkage(home, name, kind, ty, internal, location)?;
                } else if self.graph.scope(home).kind == ScopeKind::Block && specs.is_extern {
                    self.note_block_linkage(home, name, entity, kind, language);
                }
            }
        } else if kind == BindingKind::Function {
            let member = self.graph.scope(home).kind == ScopeKind::Class;
            let special = name.special();
            let allocation = special.is_some_and(SpecialName::allocation);
            if member
                && specs.is_static
                && matches!(
                    special,
                    Some(super::function_names::SpecialName::Operator { .. })
                )
                && !allocation
            {
                return Err(Error::at(location, "a static member operator function"));
            }
            let mut internal = self.graph.scope(home).kind == ScopeKind::Namespace
                && (specs.is_static || !self.c_language && self.internal_namespace(home));
            let mut language = super::linkage::Language::from_c(self.c_language && !member);
            let mut denotes = None;
            if self.graph.scope(home).kind == ScopeKind::Block
                && !templated
                && let Some(name) = name.identifier()
                && let Some(outer) = self.outer_linkage_entity(home, name, kind, ty)?
            {
                let old = self
                    .graph
                    .entity(outer)
                    .ty
                    .ok_or(Error::Invariant("linked function without type"))?;
                if self.graph.types.canonical_function(old)?
                    != self.graph.types.canonical_function(ty)?
                {
                    return Err(Error::at(
                        location,
                        "a block-scope declaration of a function with another type",
                    ));
                }
                if let Some(properties) = self.graph.entity(outer).function {
                    internal = properties.internal_linkage;
                    language = properties.language;
                    denotes = Some(properties.denotes.unwrap_or(outer));
                }
            }
            self.graph.entity_mut(entity).function = Some(super::graph::FunctionProperties {
                internal_linkage: internal,
                language,
                denotes,
                is_static: member && (specs.is_static || allocation),
                special,
                allocation: None,
            });
            if !templated && let Some(name) = name.identifier() {
                if self.graph.scope(home).kind == ScopeKind::Namespace {
                    self.agree_block_linkage(home, name, kind, ty, internal, location)?;
                } else if self.graph.scope(home).kind == ScopeKind::Block && denotes.is_none() {
                    self.note_block_linkage(home, name, entity, kind, language);
                }
            }
        } else {
            return Err(Error::Invariant("linkage facts for nonvalue declaration"));
        }
        Ok(())
    }

    pub(super) fn check_variable_storage(
        &self,
        specs: super::specifiers::Specifiers,
        initialized: bool,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if specs.is_extern && initialized && self.graph.scope(home).kind == ScopeKind::Block {
            return Err(Error::at(
                location,
                "an initializer on a block-scope extern declaration",
            ));
        }
        if specs.constexpr
            && specs.is_extern
            && !initialized
            && self.graph.scope(home).kind != ScopeKind::Class
        {
            return Err(Error::at(
                location,
                "a constexpr variable declaration that is not a definition",
            ));
        }
        Ok(())
    }
    pub(super) fn agree_variable_linkage(
        &self,
        entity: EntityId,
        specs: super::specifiers::Specifiers,
        home: ScopeId,
        qualified: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.scope(home).kind != ScopeKind::Namespace {
            return Ok(());
        }
        let properties = self
            .graph
            .entity(entity)
            .variable
            .ok_or(Error::Invariant("variable without storage facts"))?;
        if specs.is_static && !properties.internal_linkage {
            return Err(Error::at(
                location,
                "a static declaration of a variable declared with external linkage",
            ));
        }
        if qualified {
            Ok(())
        } else {
            self.agree_language(properties.language, location)
        }
    }
    pub(super) fn enclosing_namespace(&self, mut scope: ScopeId) -> ScopeId {
        while self.graph.scope(scope).kind != ScopeKind::Namespace {
            let Some(parent) = self.graph.scope(scope).parent else {
                return self.graph.global;
            };
            scope = parent;
        }
        scope
    }
    fn hidden_below_namespace(&self, mut scope: ScopeId, name: NameId) -> bool {
        let Some(parent) = self.graph.scope(scope).parent else {
            return false;
        };
        scope = parent;
        while self.graph.scope(scope).kind != ScopeKind::Namespace {
            if self
                .graph
                .own(scope, name, super::lookup::Filter::Ordinary)
                .is_some()
            {
                return true;
            }
            let Some(parent) = self.graph.scope(scope).parent else {
                break;
            };
            scope = parent;
        }
        false
    }
    pub(super) fn outer_linkage_entity(
        &mut self,
        scope: ScopeId,
        name: NameId,
        kind: BindingKind,
        ty: TypeId,
    ) -> Result<Option<EntityId>, Error> {
        let space = self.enclosing_namespace(scope);
        let binding = if kind == BindingKind::Function {
            let mut found = None;
            let mut next = 0;
            while let Some(binding) = self.graph.next_function(space, name.into(), &mut next) {
                let entity = self.graph.entity(self.graph.binding(binding).entity);
                if entity.template_head.is_none()
                    && self.same_function_parameters(
                        entity
                            .ty
                            .ok_or(Error::Invariant("linked function without type"))?,
                        ty,
                    )?
                {
                    found = Some(binding);
                    break;
                }
            }
            found
        } else {
            self.graph.value_binding(space, name.into())
        };
        let Some(binding) = binding else {
            return Ok(None);
        };
        let binding = self.graph.binding(binding);
        if binding.kind != kind {
            return Ok(None);
        }
        let entity = self.graph.entity(binding.entity);
        let internal = if kind == BindingKind::Function {
            entity.function.is_some_and(|p| p.internal_linkage)
        } else {
            entity.variable.is_some_and(|p| p.internal_linkage)
        };
        Ok((!internal || !self.hidden_below_namespace(scope, name)).then_some(binding.entity))
    }
    pub(super) fn agree_language(
        &self,
        previous: Language,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if previous != Language::from_c(self.c_language)
            && (self.c_language || self.linkage_specification.is_some())
        {
            Err(Error::at(
                location,
                "declarations of an entity with different language linkages",
            ))
        } else {
            Ok(())
        }
    }
    pub(super) fn note_block_linkage(
        &mut self,
        scope: ScopeId,
        name: NameId,
        entity: EntityId,
        kind: BindingKind,
        language: Language,
    ) {
        let space = self.enclosing_namespace(scope);
        self.block_linkages
            .records
            .entry((space, name))
            .or_default()
            .push(BlockDeclaration {
                entity,
                kind,
                language,
            });
    }
    pub(super) fn agree_block_linkage(
        &mut self,
        space: ScopeId,
        name: NameId,
        kind: BindingKind,
        ty: TypeId,
        internal: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let count = self
            .block_linkages
            .records
            .get(&(space, name))
            .map_or(0, Vec::len);
        for index in (0..count).rev() {
            let record = self.block_linkages.records[&(space, name)][index];
            if record.kind != kind {
                return Err(Error::at(
                    location,
                    "a declaration of another kind of entity than a block-scope declaration of its name",
                ));
            }
            let old = self
                .graph
                .entity(record.entity)
                .ty
                .ok_or(Error::Invariant("block linkage without type"))?;
            if kind == BindingKind::Function {
                if !self.same_function_parameters(old, ty)? {
                    continue;
                }
                if self.graph.types.canonical_function(old)?
                    != self.graph.types.canonical_function(ty)?
                {
                    return Err(Error::at(
                        location,
                        "a declaration of a function with another type than a block-scope declaration's",
                    ));
                }
            } else if self.graph.types.complete_array(old, ty).is_none() {
                return Err(Error::at(
                    location,
                    "a declaration of a variable with another type than a block-scope declaration's",
                ));
            }
            if internal {
                return Err(Error::at(
                    location,
                    "an internal declaration of an entity a block-scope declaration gave external linkage",
                ));
            }
            self.agree_language(record.language, location)?;
        }
        Ok(())
    }
}
