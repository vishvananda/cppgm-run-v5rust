// Translated from sema/semantic_unit.h and analyzer.cpp's graph construction. See NOTICE.
//! Unit-local identities; lookup indexes and source-facing order are separate.
use super::{
    Error,
    types::{TypeId, Types},
};
use crate::lex::{identifiers::NameId, vocabulary::Keyword};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ScopeId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct EntityId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct BindingId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScopeKind {
    Namespace,
    Enum,
    Class,
    Function,
    Block,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BindingKind {
    Namespace,
    NamespaceAlias,
    Type,
    Alias,
    Variable,
    Function,
    Parameter,
    Enumerator,
}
#[derive(Clone, Copy)]
pub(super) struct Entry {
    pub ty: Option<BindingId>,
    pub value: Option<BindingId>,
    pub tag: Option<BindingId>,
}
pub(super) struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub name: Option<NameId>,
    pub children: Vec<ScopeId>,
    pub bindings: Vec<BindingId>,
    pub index: HashMap<NameId, Entry>,
    pub directives: Vec<ScopeId>,
    pub inline_members: Vec<ScopeId>,
    pub is_inline: bool,
    pub written_name: Option<crate::syntax::ast::Name>,
}
pub(super) struct Entity {
    pub ty: Option<TypeId>,
    pub scope: Option<ScopeId>,
    pub name: Option<NameId>,
    pub key: Keyword,
    pub defined: bool,
    pub complete: bool,
    pub enumeration: Option<super::enumerations::EnumInfo>,
    pub constant: super::constants::ConstantState,
    pub implicit_default: Option<ScopeId>,
}
#[derive(Clone, Copy)]
pub(super) struct Binding {
    pub kind: BindingKind,
    pub name: Option<NameId>,
    pub entity: EntityId,
    pub ty: Option<TypeId>,
    pub key: Keyword,
}
pub(crate) struct Graph {
    pub(super) types: Types,
    scopes: Vec<Scope>,
    entities: Vec<Entity>,
    bindings: Vec<Binding>,
    pub(super) global: ScopeId,
    // Dense facts refer to the original AST. No duplicate semantic syntax tree.
    pub(super) declarator_types: Vec<Option<TypeId>>,
    pub(super) declared_bindings: Vec<Option<BindingId>>,
    pub(super) type_facts: Vec<Option<TypeId>>,
    enum_declarations: Vec<Option<BindingId>>,
    pub(super) binding_labels: HashMap<BindingId, crate::syntax::ast::Name>,
    pub(super) constant_facts: Vec<Option<super::constants::Constant>>,
    pub(super) resolved_names: Vec<Option<BindingId>>,
}
impl Graph {
    pub(super) fn integral_kind(&self, ty: TypeId) -> Option<super::integral::IntegralKind> {
        let ty = if let super::types::TypeKind::Enum(entity) = self.types.get(ty).kind {
            self.entity(entity).enumeration?.underlying?
        } else {
            ty
        };
        let super::types::TypeKind::Scalar(scalar) = self.types.get(ty).kind else {
            return None;
        };
        super::integral::IntegralKind::of(scalar)
    }
    pub(super) fn new() -> Self {
        Self {
            types: Types::default(),
            scopes: vec![Scope {
                kind: ScopeKind::Namespace,
                parent: None,
                name: None,
                children: Vec::new(),
                bindings: Vec::new(),
                index: HashMap::new(),
                directives: Vec::new(),
                inline_members: Vec::new(),
                is_inline: false,
                written_name: None,
            }],
            entities: Vec::new(),
            bindings: Vec::new(),
            global: ScopeId(0),
            declarator_types: Vec::new(),
            declared_bindings: Vec::new(),
            type_facts: Vec::new(),
            resolved_names: Vec::new(),
            enum_declarations: Vec::new(),
            binding_labels: HashMap::new(),
            constant_facts: Vec::new(),
        }
    }
    pub(super) fn scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0 as usize]
    }
    pub(super) fn scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id.0 as usize]
    }
    pub(super) fn entity(&self, id: EntityId) -> &Entity {
        &self.entities[id.0 as usize]
    }
    pub(super) fn entity_mut(&mut self, id: EntityId) -> &mut Entity {
        &mut self.entities[id.0 as usize]
    }
    pub(super) fn enum_declaration(&self, id: BindingId) -> Option<BindingId> {
        self.enum_declarations.get(id.0 as usize).copied().flatten()
    }
    pub(super) fn record_enum_declaration(&mut self, id: BindingId, declaration: BindingId) {
        super::record(&mut self.enum_declarations, id.0 as usize, declaration);
    }
    pub(super) fn binding(&self, id: BindingId) -> Binding {
        self.bindings[id.0 as usize]
    }
    pub(super) fn add_scope(
        &mut self,
        kind: ScopeKind,
        parent: ScopeId,
        print: Option<ScopeId>,
        name: Option<NameId>,
    ) -> Result<ScopeId, Error> {
        let id = ScopeId(
            u32::try_from(self.scopes.len()).map_err(|_| Error::Limit("semantic scope limit"))?,
        );
        self.scopes.push(Scope {
            kind,
            parent: Some(parent),
            name,
            children: Vec::new(),
            bindings: Vec::new(),
            index: HashMap::new(),
            directives: Vec::new(),
            inline_members: Vec::new(),
            is_inline: false,
            written_name: None,
        });
        if let Some(print) = print {
            self.scope_mut(print).children.push(id);
        }
        Ok(id)
    }
    pub(super) fn add_entity(
        &mut self,
        ty: Option<TypeId>,
        name: Option<NameId>,
        key: Keyword,
    ) -> Result<EntityId, Error> {
        let id = EntityId(
            u32::try_from(self.entities.len())
                .map_err(|_| Error::Limit("semantic entity limit"))?,
        );
        self.entities.push(Entity {
            ty,
            scope: None,
            name,
            key,
            defined: false,
            complete: false,
            enumeration: None,
            constant: super::constants::ConstantState::None,
            implicit_default: None,
        });
        Ok(id)
    }
    pub(super) fn add_binding(
        &mut self,
        scope: ScopeId,
        print: Option<ScopeId>,
        binding: Binding,
    ) -> Result<BindingId, Error> {
        let id = BindingId(
            u32::try_from(self.bindings.len())
                .map_err(|_| Error::Limit("semantic binding limit"))?,
        );
        self.bindings.push(binding);
        if let Some(print) = print {
            self.scope_mut(print).bindings.push(id);
        }
        if let Some(name) = binding.name {
            let entry = self.scope_mut(scope).index.entry(name).or_insert(Entry {
                ty: None,
                value: None,
                tag: None,
            });
            if binding.kind == BindingKind::Type {
                entry.tag = Some(id);
            }
            match binding.kind {
                BindingKind::Type
                | BindingKind::Alias
                | BindingKind::Namespace
                | BindingKind::NamespaceAlias => entry.ty = Some(id),
                _ => entry.value = Some(id),
            }
        }
        Ok(id)
    }
    pub(super) fn encloses(&self, outer: ScopeId, mut inner: ScopeId) -> bool {
        loop {
            if inner == outer {
                return true;
            }
            let Some(parent) = self.scope(inner).parent else {
                return false;
            };
            inner = parent;
        }
    }
    pub(super) fn own(
        &self,
        scope: ScopeId,
        name: NameId,
        filter: super::lookup::Filter,
    ) -> Option<BindingId> {
        let entry = self.scope(scope).index.get(&name)?;
        let id = match filter {
            super::lookup::Filter::Ordinary => entry.value.or(entry.ty),
            super::lookup::Filter::Elaborated => entry.tag.or(entry.ty).filter(|&id| {
                !matches!(
                    self.binding(id).kind,
                    BindingKind::Namespace | BindingKind::NamespaceAlias
                )
            }),
            _ => entry.ty,
        }?;
        if filter == super::lookup::Filter::Namespace
            && !matches!(
                self.binding(id).kind,
                BindingKind::Namespace | BindingKind::NamespaceAlias
            )
        {
            return None;
        }
        Some(id)
    }
}
