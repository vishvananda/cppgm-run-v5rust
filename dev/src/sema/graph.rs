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
    TemplateParameters,
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
    pub owner: Option<EntityId>,
    pub children: Vec<ScopeId>,
    pub anonymous_namespace: Option<ScopeId>,
    pub bindings: Vec<BindingId>,
    pub index: HashMap<NameId, Entry>,
    pub directives: Vec<ScopeEdge>,
    pub inline_members: Vec<ScopeEdge>,
    pub is_inline: bool,
    pub guarded: bool,
    pub written_name: Option<crate::syntax::ast::Name>,
}
#[derive(Clone, Copy)]
pub(super) struct ScopeEdge {
    pub target: ScopeId,
    pub introduced: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DataMemberKind {
    Instance,
    Static,
}
#[derive(Clone, Copy)]
pub(super) struct FunctionProperties {
    pub internal_linkage: bool,
    pub is_static: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum MemberKind {
    Constructor,
    Destructor,
    Conversion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct MemberKey {
    pub class: EntityId,
    pub kind: MemberKind,
    pub signature: TypeId,
    pub shape: Option<super::templates::ShapeId>,
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
    pub user_constructor: Option<BindingId>,
    pub data_member: Option<DataMemberKind>,
    pub exceptions: super::exceptions::ExceptionState,
    pub constexpr: bool,
    pub function: Option<FunctionProperties>,
    pub member_function: Option<MemberKey>,
    pub special_name: Option<crate::syntax::ast::Name>,
    pub members: Vec<EntityId>,
    pub layout: super::layout::LayoutState,
    pub member_offset: Option<u64>,
    // Presentation provenance for an unnamed class, never a lookup key.
    pub anonymous_class: Option<super::anonymous_types::AnonymousClass>,
    pub template_parameter: Option<super::templates::ParameterInfo>,
    pub template_head: Option<super::templates::HeadId>,
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
    pub(super) templates: super::templates::Templates,
    scopes: Vec<Scope>,
    entities: Vec<Entity>,
    bindings: Vec<Binding>,
    ordinals: Vec<usize>,
    pub(super) lookup_limit: Option<usize>,
    pub(super) global: ScopeId,
    // Dense facts refer to the original AST. No duplicate semantic syntax tree.
    pub(super) declarator_types: Vec<Option<TypeId>>,
    pub(super) declared_bindings: Vec<Option<BindingId>>,
    pub(super) type_facts: Vec<Option<TypeId>>,
    enum_declarations: Vec<Option<BindingId>>,
    pub(super) binding_labels: HashMap<BindingId, crate::syntax::ast::Name>,
    pub(super) special_labels: HashMap<BindingId, crate::syntax::ast::Name>,
    pub(super) constant_facts: Vec<Option<super::constants::Constant>>,
    pub(super) resolved_names: Vec<Option<BindingId>>,
    pub(super) expression_types: Vec<Option<TypeId>>,
    pub(super) statement_readings: Vec<Option<crate::syntax::ast::StmtId>>,
    arithmetic_types: Vec<Option<(TypeId, TypeId)>>,
    call_bindings: Vec<Option<BindingId>>,
    member_functions: HashMap<MemberKey, BindingId>,
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
            templates: super::templates::Templates::default(),
            scopes: vec![Scope {
                kind: ScopeKind::Namespace,
                parent: None,
                name: None,
                owner: None,
                children: Vec::new(),
                anonymous_namespace: None,
                bindings: Vec::new(),
                index: HashMap::new(),
                directives: Vec::new(),
                inline_members: Vec::new(),
                is_inline: false,
                guarded: false,
                written_name: None,
            }],
            entities: Vec::new(),
            bindings: Vec::new(),
            ordinals: Vec::new(),
            lookup_limit: None,
            global: ScopeId(0),
            declarator_types: Vec::new(),
            declared_bindings: Vec::new(),
            type_facts: Vec::new(),
            resolved_names: Vec::new(),
            enum_declarations: Vec::new(),
            binding_labels: HashMap::new(),
            special_labels: HashMap::new(),
            constant_facts: Vec::new(),
            expression_types: Vec::new(),
            statement_readings: Vec::new(),
            arithmetic_types: Vec::new(),
            call_bindings: Vec::new(),
            member_functions: HashMap::new(),
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
            owner: None,
            children: Vec::new(),
            anonymous_namespace: None,
            bindings: Vec::new(),
            index: HashMap::new(),
            directives: Vec::new(),
            inline_members: Vec::new(),
            is_inline: false,
            guarded: false,
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
            user_constructor: None,
            data_member: None,
            exceptions: super::exceptions::ExceptionState::Absent,
            constexpr: false,
            function: None,
            member_function: None,
            special_name: None,
            members: Vec::new(),
            layout: super::layout::LayoutState::NotStarted,
            member_offset: None,
            anonymous_class: None,
            template_parameter: None,
            template_head: None,
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
        let earlier = binding
            .name
            .and_then(|name| self.scope(scope).index.get(&name))
            .and_then(|entry| match binding.kind {
                BindingKind::Type
                | BindingKind::Alias
                | BindingKind::Namespace
                | BindingKind::NamespaceAlias => entry.ty,
                _ => entry.value,
            })
            .filter(|&id| self.binding(id).entity == binding.entity);
        // A compatible redeclaration is visible since the entity's first
        // visible binding, just as C++ NameLookup::KeepOrdinal specifies.
        let ordinal = earlier.map_or(self.bindings.len(), |id| self.ordinals[id.0 as usize]);
        self.bindings.push(binding);
        self.ordinals.push(ordinal);
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
        let mut entry = *self.scope(scope).index.get(&name)?;
        if self.scope(scope).kind == ScopeKind::Namespace
            && let Some(limit) = self.lookup_limit
        {
            let visible = |id: BindingId| self.ordinals[id.0 as usize] < limit;
            entry.ty = entry.ty.filter(|&id| visible(id));
            entry.value = entry.value.filter(|&id| visible(id));
            entry.tag = entry.tag.filter(|&id| visible(id));
        }
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
    pub(super) fn binding_limit(&self) -> usize {
        self.bindings.len()
    }
    // Constructors have their own lookup category. Their printed class name
    // must never replace the injected type name in ordinary lookup.
    pub(super) fn constructor(&self, class: EntityId, signature: TypeId) -> Option<BindingId> {
        self.member_function(MemberKey {
            class,
            kind: MemberKind::Constructor,
            signature,
            shape: None,
        })
    }
    pub(super) fn member_function(&self, key: MemberKey) -> Option<BindingId> {
        self.member_functions.get(&key).copied()
    }
    pub(super) fn record_member_function(&mut self, key: MemberKey, binding: BindingId) {
        self.member_functions.insert(key, binding);
        self.entity_mut(self.binding(binding).entity)
            .member_function = Some(key);
    }
    pub(super) fn resolved_name(&self, name: crate::syntax::ast::Name) -> Option<BindingId> {
        self.resolved_names.get(name.index()).copied().flatten()
    }
    pub(super) fn expression_type(&self, id: crate::syntax::ast::ExprId) -> Option<TypeId> {
        self.expression_types.get(id.index()).copied().flatten()
    }
    pub(super) fn type_fact(&self, id: crate::syntax::ast::TypeId) -> Option<TypeId> {
        self.type_facts.get(id.index()).copied().flatten()
    }
    pub(super) fn record_arithmetic_types(
        &mut self,
        id: crate::syntax::ast::ExprId,
        left: TypeId,
        right: TypeId,
    ) {
        super::record(&mut self.arithmetic_types, id.index(), (left, right));
    }
    pub(super) fn arithmetic_types(
        &self,
        id: crate::syntax::ast::ExprId,
    ) -> Result<(TypeId, TypeId), Error> {
        self.arithmetic_types
            .get(id.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("arithmetic without conversion types"))
    }
    pub(super) fn constant_fact(
        &self,
        id: crate::syntax::ast::ExprId,
    ) -> Option<super::constants::Constant> {
        self.constant_facts.get(id.index()).copied().flatten()
    }
    pub(super) fn record_call_binding(
        &mut self,
        expr: crate::syntax::ast::ExprId,
        binding: BindingId,
    ) {
        super::record(&mut self.call_bindings, expr.index(), binding);
    }
    pub(super) fn call_entity(&self, expr: crate::syntax::ast::ExprId) -> Result<EntityId, Error> {
        Ok(self.binding(self.call_binding(expr)?).entity)
    }
    pub(super) fn call_binding(
        &self,
        expr: crate::syntax::ast::ExprId,
    ) -> Result<BindingId, Error> {
        self.call_bindings
            .get(expr.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("call without selected binding"))
    }
    pub(super) fn visible_edges(
        &self,
        scope: ScopeId,
        inline: bool,
    ) -> impl DoubleEndedIterator<Item = ScopeId> + '_ {
        let owner = self.scope(scope);
        let edges = if inline {
            &owner.inline_members
        } else {
            &owner.directives
        };
        edges
            .iter()
            .filter(move |edge| {
                owner.kind != ScopeKind::Namespace
                    || self
                        .lookup_limit
                        .is_none_or(|limit| edge.introduced < limit)
            })
            .map(|edge| edge.target)
    }
    pub(super) fn add_edge(&mut self, scope: ScopeId, target: ScopeId, inline: bool) {
        let edge = ScopeEdge {
            target,
            introduced: self.bindings.len(),
        };
        let scope = self.scope_mut(scope);
        if inline {
            scope.inline_members.push(edge);
        } else {
            scope.directives.push(edge);
        }
    }
}
