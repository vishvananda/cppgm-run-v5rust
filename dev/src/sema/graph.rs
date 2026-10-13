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
pub(super) enum FunctionAvailability {
    Callable,
    Deleted,
}
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
    pub member_access: Access,
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
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Mutability {
    #[default]
    Ordinary,
    Mutable,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Access {
    #[default]
    Public,
    Protected,
    Private,
}
#[derive(Clone, Copy)]
pub(super) struct FunctionProperties {
    pub internal_linkage: bool,
    pub is_static: bool,
    pub special: Option<super::function_names::SpecialName>,
    pub allocation: Option<super::function_names::AllocationOrigin>,
}
#[derive(Clone, Copy)]
pub(super) struct VariableProperties {
    // The C++ entity retains these facts from its first declaration.
    pub is_extern: bool,
    pub automatic: bool,
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
    pub home: Option<ScopeId>,
    pub name: Option<NameId>,
    pub key: Keyword,
    pub defined: bool,
    pub availability: FunctionAvailability,
    pub defaults: Vec<Option<super::default_arguments::DefaultArgument>>,
    pub complete: bool,
    pub enumeration: Option<super::enumerations::EnumInfo>,
    pub constant: super::constants::ConstantState,
    pub implicit_default: Option<ScopeId>,
    pub user_constructor: Option<BindingId>,
    pub declared_constructors: Vec<BindingId>,
    pub declared_conversions: Vec<BindingId>,
    pub special_definition: Option<super::special_members::SpecialDefinition>,
    pub destructor_specification: super::exceptions::DestructorSpecification,
    pub data_member: Option<DataMemberKind>,
    pub mutability: Mutability,
    pub exceptions: super::exceptions::ExceptionState,
    pub constexpr: bool,
    pub explicitness: super::specifiers::Explicitness,
    pub implicit_copies: Option<super::copy_initialization::ImplicitCopies>,
    pub copy_kind: Option<super::types::RefKind>,
    pub function: Option<FunctionProperties>,
    pub definition_emission: super::function_demands::DefinitionEmission,
    pub function_definition: Option<crate::syntax::ast::DeclId>,
    pub demands: super::function_demands::DemandList,
    pub member_function: Option<MemberKey>,
    pub special_name: Option<crate::syntax::ast::Name>,
    pub members: Vec<EntityId>,
    pub layout: super::layout::LayoutState,
    pub member_offset: Option<u64>,
    // Presentation provenance for an unnamed class, never a lookup key.
    pub anonymous_class: Option<super::anonymous_types::AnonymousClass>,
    pub local_type_ordinal: Option<u32>,
    pub anonymous_storage: Option<BindingId>,
    pub default_construction: super::construction::ConstructorState,
    pub constructor: Option<super::construction::ConstructorId>,
    pub member_initializer: Option<crate::syntax::ast::Initializer>,
    pub initialized_members: usize,
    pub variable: Option<VariableProperties>,
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
    binding_scopes: Vec<ScopeId>,
    binding_access: Vec<Access>,
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
    pub(super) expression_facts: super::expression_facts::ExpressionFacts,
    pub(super) constructors: super::construction::Constructors,
    pub(super) root_demands: super::function_demands::DemandList,
    pub(super) declaration_storage: Vec<Option<BindingId>>,
    pub(super) overloads: super::function_resolution::FunctionSets,
    pub(super) statement_readings: Vec<Option<crate::syntax::ast::StmtId>>,
    arithmetic_types: Vec<Option<(TypeId, TypeId)>>,
    call_bindings: Vec<Option<BindingId>>,
    member_functions: HashMap<MemberKey, BindingId>,
    prototype_parameters: HashMap<EntityId, usize>,
    special_values: HashMap<(ScopeId, super::function_names::SpecialName), BindingId>,
    function_sets: HashMap<(ScopeId, super::function_names::ValueKey), Vec<BindingId>>,
    function_origins: HashMap<EntityId, BindingId>,
}
impl Graph {
    pub(super) fn own_key(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        filter: super::lookup::Filter,
    ) -> Option<BindingId> {
        match key {
            super::function_names::ValueKey::Identifier(name) => self.own(scope, name, filter),
            super::function_names::ValueKey::Special(_) => self.value_binding(scope, key),
        }
    }
    pub(super) fn value_binding(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
    ) -> Option<BindingId> {
        match key {
            super::function_names::ValueKey::Identifier(name) => {
                self.own(scope, name, super::lookup::Filter::Ordinary)
            }
            super::function_names::ValueKey::Special(name) => self
                .special_values
                .get(&(scope, name))
                .copied()
                .filter(|id| {
                    self.scope(scope).kind != ScopeKind::Namespace
                        || self
                            .lookup_limit
                            .is_none_or(|limit| self.ordinals[id.0 as usize] < limit)
                }),
        }
    }
    pub(super) fn record_special_value(
        &mut self,
        scope: ScopeId,
        name: super::function_names::SpecialName,
        binding: BindingId,
    ) {
        if self.binding(binding).kind == BindingKind::Function {
            self.record_function_binding(
                scope,
                super::function_names::ValueKey::Special(name),
                binding,
            );
        }
        if let Some(old) = self.special_values.insert((scope, name), binding)
            && self.binding(old).entity == self.binding(binding).entity
        {
            self.ordinals[binding.0 as usize] = self.ordinals[old.0 as usize];
        }
    }
    pub(super) fn record_prototype_parameter(&mut self, entity: EntityId, index: usize) {
        self.prototype_parameters.insert(entity, index);
    }
    pub(super) fn prototype_parameter(&self, entity: EntityId) -> Option<usize> {
        self.prototype_parameters.get(&entity).copied()
    }
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
                member_access: Access::Public,
                written_name: None,
            }],
            entities: Vec::new(),
            bindings: Vec::new(),
            ordinals: Vec::new(),
            binding_scopes: Vec::new(),
            binding_access: Vec::new(),
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
            expression_facts: super::expression_facts::ExpressionFacts::default(),
            constructors: super::construction::Constructors::default(),
            root_demands: super::function_demands::DemandList::default(),
            declaration_storage: Vec::new(),
            overloads: super::function_resolution::FunctionSets::default(),
            statement_readings: Vec::new(),
            arithmetic_types: Vec::new(),
            call_bindings: Vec::new(),
            member_functions: HashMap::new(),
            prototype_parameters: HashMap::new(),
            special_values: HashMap::new(),
            function_sets: HashMap::new(),
            function_origins: HashMap::new(),
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
    pub(super) fn complete_variable_type(
        &mut self,
        id: BindingId,
        ty: TypeId,
    ) -> Result<(), Error> {
        let binding = self.binding(id);
        if binding.kind != BindingKind::Variable {
            return Err(Error::Invariant("completing a nonvariable type"));
        }
        self.bindings[id.0 as usize].ty = Some(ty);
        self.entity_mut(binding.entity).ty = Some(ty);
        Ok(())
    }
    pub(super) fn publish_variable_type(&mut self, id: BindingId) -> Result<TypeId, Error> {
        let ty = self
            .entity(self.binding(id).entity)
            .ty
            .ok_or(Error::Invariant("variable without type"))?;
        self.complete_variable_type(id, ty)?;
        Ok(ty)
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
            member_access: Access::Public,
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
            home: None,
            name,
            key,
            defined: false,
            availability: FunctionAvailability::Callable,
            defaults: Vec::new(),
            complete: false,
            enumeration: None,
            constant: super::constants::ConstantState::None,
            implicit_default: None,
            user_constructor: None,
            declared_constructors: Vec::new(),
            declared_conversions: Vec::new(),
            special_definition: None,
            destructor_specification: super::exceptions::DestructorSpecification::Explicit,
            data_member: None,
            mutability: Mutability::Ordinary,
            exceptions: super::exceptions::ExceptionState::Absent,
            constexpr: false,
            explicitness: super::specifiers::Explicitness::Converting,
            implicit_copies: None,
            copy_kind: None,
            function: None,
            definition_emission: super::function_demands::DefinitionEmission::Always,
            function_definition: None,
            demands: super::function_demands::DemandList::default(),
            member_function: None,
            special_name: None,
            members: Vec::new(),
            layout: super::layout::LayoutState::NotStarted,
            member_offset: None,
            anonymous_class: None,
            local_type_ordinal: None,
            anonymous_storage: None,
            default_construction: super::construction::ConstructorState::NotStarted,
            constructor: None,
            member_initializer: None,
            initialized_members: 0,
            variable: None,
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
            .filter(|&id| self.binding(id).entity == binding.entity)
            .or_else(|| {
                self.entity(binding.entity)
                    .member_function
                    .and_then(|key| self.member_function(key))
            })
            .or_else(|| {
                if binding.kind != BindingKind::Function {
                    return None;
                }
                let key = binding
                    .name
                    .map(super::function_names::ValueKey::Identifier)
                    .or_else(|| {
                        self.entity(binding.entity).function.and_then(|properties| {
                            properties
                                .special
                                .map(super::function_names::ValueKey::Special)
                        })
                    })?;
                self.functions_at(scope, key)
                    .find(|&id| self.binding(id).entity == binding.entity)
            });
        // A compatible redeclaration is visible since the entity's first
        // visible binding, just as C++ NameLookup::KeepOrdinal specifies.
        let ordinal = earlier.map_or(self.bindings.len(), |id| self.ordinals[id.0 as usize]);
        if self.entity(binding.entity).home.is_none() {
            self.entity_mut(binding.entity).home = Some(scope);
        }
        let access = earlier.map_or(self.scope(scope).member_access, |id| {
            self.binding_access(id)
        });
        self.binding_access.push(access);
        self.bindings.push(binding);
        self.binding_scopes.push(scope);
        self.ordinals.push(ordinal);
        if binding.kind == BindingKind::Function {
            self.function_origins.entry(binding.entity).or_insert(id);
            if let Some(name) = binding.name {
                self.record_function_binding(scope, name.into(), id);
            }
        }
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
    fn record_function_binding(
        &mut self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        binding: BindingId,
    ) {
        let entity = self.binding(binding).entity;
        let previous = self
            .function_sets
            .get(&(scope, key))
            .and_then(|set| set.iter().position(|&id| self.binding(id).entity == entity));
        let set = self.function_sets.entry((scope, key)).or_default();
        if let Some(index) = previous {
            set[index] = binding;
        } else {
            set.push(binding);
        }
    }
    pub(super) fn functions_at(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
    ) -> impl Iterator<Item = BindingId> + '_ {
        self.function_sets
            .get(&(scope, key))
            .into_iter()
            .flatten()
            .copied()
            .filter(move |id| {
                self.scope(scope).kind != ScopeKind::Namespace
                    || self
                        .lookup_limit
                        .is_none_or(|limit| self.ordinals[id.0 as usize] < limit)
            })
    }
    pub(super) fn function_default(
        &self,
        entity: EntityId,
        index: usize,
    ) -> Option<crate::syntax::ast::ExprId> {
        self.entity(entity)
            .defaults
            .get(index)
            .copied()
            .flatten()
            .filter(|default| {
                self.lookup_limit
                    .is_none_or(|limit| default.introduced < limit)
            })
            .map(|default| default.expression)
    }
    pub(super) fn function_origin(&self, entity: EntityId) -> Result<BindingId, Error> {
        self.function_origins
            .get(&entity)
            .copied()
            .ok_or(Error::Invariant("function without origin"))
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
            let function_set = entry
                .value
                .is_some_and(|id| self.binding(id).kind == BindingKind::Function);
            entry.value = entry.value.filter(|&id| visible(id)).or_else(|| {
                if function_set {
                    self.functions_at(scope, name.into()).next()
                } else {
                    None
                }
            });
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
    pub(super) fn binding_access(&self, id: BindingId) -> Access {
        self.binding_access[id.0 as usize]
    }
    pub(super) fn binding_scope(&self, id: BindingId) -> ScopeId {
        self.binding_scopes[id.0 as usize]
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
        if key.kind == MemberKind::Constructor {
            let entity = self.binding(binding).entity;
            let earlier = self
                .entity(key.class)
                .declared_constructors
                .iter()
                .position(|&id| self.binding(id).entity == entity);
            let constructors = &mut self.entity_mut(key.class).declared_constructors;
            if let Some(index) = earlier {
                constructors[index] = binding;
            } else {
                constructors.push(binding);
            }
        } else if key.kind == MemberKind::Conversion {
            let entity = self.binding(binding).entity;
            let earlier = self
                .entity(key.class)
                .declared_conversions
                .iter()
                .position(|&id| self.binding(id).entity == entity);
            let conversions = &mut self.entity_mut(key.class).declared_conversions;
            if let Some(index) = earlier {
                conversions[index] = binding;
            } else {
                conversions.push(binding);
            }
        }
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
