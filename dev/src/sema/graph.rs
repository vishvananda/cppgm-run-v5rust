// Translated from sema/semantic_unit.h and analyzer.cpp's graph construction. See NOTICE.
//! Unit-local identities; lookup indexes and source-facing order are separate.
use super::{
    Error,
    types::{TypeId, Types},
};
use crate::lex::{identifiers::NameId, vocabulary::Keyword};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ScopeId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum FunctionAvailability {
    Callable,
    Deleted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct EntityId(u32);
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
    TemplateValue,
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
    pub template_bridge: bool,
    /// Syntactic template lists introduced here; argument frames introduce none.
    pub template_lists: u32,
    pub initialized: u32,
    pub initialized_at_entry: u32,
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
    pub no_inline: bool,
    pub internal_linkage: bool,
    pub language: super::linkage::Language,
    pub denotes: Option<EntityId>,
    pub is_static: bool,
    pub special: Option<super::function_names::SpecialName>,
    pub allocation: Option<super::function_names::AllocationOrigin>,
}
#[derive(Clone, Copy)]
pub(super) struct VariableProperties {
    // The C++ entity retains these facts from its first declaration.
    pub is_extern: bool,
    pub internal_linkage: bool,
    pub language: super::linkage::Language,
    pub automatic: bool,
    pub thread_storage: super::specifiers::ThreadStorage,
    pub static_specifier: super::specifiers::StaticSpecifier,
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
    pub inline: super::specifiers::InlinePreference,
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
    pub declared_destructor: Option<BindingId>,
    pub implicit_destructor: Option<BindingId>,
    pub destruction: Option<super::destruction::Destruction>,
    pub destruction_preparation: super::destruction::Preparation,
    pub declared_conversions: Vec<BindingId>,
    pub special_definition: Option<super::special_members::SpecialDefinition>,
    pub destructor_specification: super::exceptions::DestructorSpecification,
    pub data_member: Option<DataMemberKind>,
    pub mutability: Mutability,
    pub exceptions: super::exceptions::ExceptionState,
    pub target_unwind: super::exceptions::TargetUnwind,
    pub constexpr: bool,
    pub explicitness: super::specifiers::Explicitness,
    pub implicit_copies: Option<super::copy_initialization::ImplicitCopies>,
    pub copy_preparation: super::copy_properties::Preparation,
    pub copy_properties: Option<super::copy_properties::CopyPropertiesId>,
    pub copy_origin: super::copy_construction::Origin,
    pub copy_kind: Option<super::types::RefKind>,
    pub assignment_kind: Option<super::types::RefKind>,
    pub assignment: Option<super::assignment::Properties>,
    pub assignment_origin: super::assignment::Origin,
    pub implicit_assignment_use: super::assignment::ImplicitUse,
    pub assignment_exceptions: super::assignment::ExceptionSource,
    pub function: Option<FunctionProperties>,
    pub builtin: Option<super::builtin_calls::BuiltinFunction>,
    pub definition_emission: super::function_demands::DefinitionEmission,
    pub function_definition: Option<crate::syntax::ast::DeclId>,
    pub analyzed_body: Option<crate::syntax::ast::BodyId>,
    pub parameter_entities: Vec<EntityId>,
    pub function_parameters: Option<crate::syntax::ast::ParameterClause>,
    pub demands: super::function_demands::DemandList,
    pub member_function: Option<MemberKey>,
    pub special_name: Option<crate::syntax::ast::Name>,
    pub members: Vec<EntityId>,
    pub polymorphism: super::virtual_members::Polymorphism,
    pub literal: super::literal_types::LiteralState,
    pub aggregation: super::aggregate_initialization::Aggregation,
    pub bases: Vec<super::inheritance::BaseClass>,
    pub base_dependency: crate::syntax::ast::BaseDependency,
    pub derivation: super::inheritance::Derivation,
    pub layout: super::layout::LayoutState,
    pub storage_attributes: super::layout_attributes::StorageAttributes,
    pub class_packing: u32,
    pub layout_category: super::layout::LayoutCategory,
    pub member_offset: Option<u64>,
    pub bit_field: Option<super::bit_fields::BitFieldId>,
    // Presentation provenance for an unnamed class, never a lookup key.
    pub anonymous_class: Option<super::anonymous_types::AnonymousClass>,
    pub local_type_ordinal: Option<u32>,
    pub anonymous_storage: Option<BindingId>,
    pub default_construction: super::construction::ConstructorState,
    pub constructor: Option<super::construction::ConstructorId>,
    pub constructor_arguments: Option<super::constructor_initialization::ConstructorArguments>,
    pub constructor_exceptions: super::construction::ConstructorExceptionSource,
    pub member_initializer: Option<crate::syntax::ast::Initializer>,
    pub initialized_members: usize,
    pub variable: Option<VariableProperties>,
    pub variable_definition: Option<crate::syntax::ast::DeclaratorId>,
    pub reference_temporary: Option<super::static_initialization::ReferenceTemporary>,
    pub static_initializer: Option<super::static_initialization::Value>,
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
#[derive(Clone, Copy)]
struct FunctionPosition {
    declared: usize,
    visible: Option<usize>,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FunctionSignature {
    scope: ScopeId,
    name: super::function_names::ValueKey,
    ty: TypeId,
    shape: Option<super::templates::ShapeId>,
}

pub(crate) struct Graph {
    pub(super) hosted: bool,
    pub(super) closures: Box<super::lambdas::Closures>,
    pub(super) virtuals: super::virtual_members::Virtuals,
    pub(super) local_classes: super::local_classes::LocalClasses,
    pub(super) bit_fields: super::bit_fields::BitFields,
    pub(super) types: Types,
    pub(super) friendships: super::friendships::Friendships,
    pub(super) base_paths: super::inheritance::BasePaths,
    pub(super) dependent_members: super::dependent_members::Members,
    // Unit-owned query tables stay off the recursive analyzer's stack value.
    pub(super) template_operands: Box<super::template_operands::Operands>,
    pub(super) class_templates: super::class_templates::ClassTemplates,
    pub(super) templates: super::templates::Templates,
    pub(super) function_templates: super::function_templates::FunctionTemplates,
    scopes: Vec<Scope>,
    entities: Vec<Entity>,
    bindings: Vec<Binding>,
    ordinals: Vec<usize>,
    binding_scopes: Vec<ScopeId>,
    binding_access: Vec<Access>,
    pub(super) anonymous_enums: u32,
    pub(super) local_types: u32,
    pub(super) block_linkages: super::linkage::BlockLinkages,
    pub(super) conversion_sets: HashMap<EntityId, Vec<BindingId>>,
    pub(super) boundary_roots: Vec<super::graph::EntityId>,
    pub(super) boundaries: super::layout::Boundaries,
    pub(super) instantiation_levels: super::template_context::Levels,
    pub(super) storage_layouts: Vec<(
        Option<super::graph::EntityId>,
        super::layout::StorageDemand,
        crate::support::source::SourceOffset,
    )>,
    pub(super) constructor_boundaries: Vec<(
        Option<super::graph::EntityId>,
        super::construction::ConstructorId,
    )>,
    pub(super) storage_destructions: Vec<(Option<super::graph::EntityId>, super::graph::EntityId)>,
    pub(super) lookup_limit: Option<usize>,
    pub(super) global: ScopeId,
    // Dense facts refer to the original AST. No duplicate semantic syntax tree.
    pub(super) declarator_types: Vec<Option<TypeId>>,
    pub(super) declared_bindings: Vec<Option<BindingId>>,
    pub(super) type_facts: Vec<Option<TypeId>>,
    enum_declarations: Vec<Option<BindingId>>,
    dependent_bindings: Vec<bool>,
    pub(super) binding_labels: HashMap<BindingId, crate::syntax::ast::Name>,
    pub(super) special_labels: HashMap<BindingId, crate::syntax::ast::Name>,
    pub(super) constant_facts: Vec<Option<super::constants::Constant>>,
    pub(super) floating_constants: super::constant_floats::Floats,
    pub(super) aggregate_constants: Box<super::constexpr_values::Values>,
    pub(super) constant_addresses: Box<super::constant_addresses::Addresses>,
    pub(super) initializer_constructors: super::constructor_initialization::InitializerConstructors,
    pub(super) array_images: super::static_arrays::Images,
    pub(super) static_constructions: super::static_construction::Images,
    pub(super) resolved_names: Vec<Option<BindingId>>,
    pub(super) expression_types: Vec<Option<TypeId>>,
    pub(super) expression_facts: super::expression_facts::ExpressionFacts,
    pub(super) expression_scopes: Vec<Option<ScopeId>>,
    pub(super) copy_properties: super::copy_properties::CopyPropertiesStore,
    pub(super) constructors: super::construction::Constructors,
    pub(super) inherited_constructors: super::inherited_constructors::InheritedConstructors,
    pub(super) member_initializers: super::member_initializers::Initializers,
    pub(super) variable_instances: Box<super::variable_instances::Variables>,
    pub(super) root_demands: super::function_demands::DemandList,
    pub(super) declaration_storage: Vec<Option<BindingId>>,
    pub(super) overloads: super::function_resolution::FunctionSets,
    pub(super) statement_readings: Vec<Option<crate::syntax::ast::StmtId>>,
    arithmetic_types: Vec<Option<(TypeId, TypeId)>>,
    condition_functions: std::collections::HashMap<
        crate::syntax::ast::DeclaratorId,
        super::conversion_functions::FunctionConversion,
    >,
    condition_conversions: Vec<Option<super::expression_facts::Conversion>>,
    pub(super) case_values: Vec<Option<u64>>,
    call_bindings: Vec<Option<BindingId>>,
    member_functions: HashMap<MemberKey, BindingId>,
    prototype_parameters: HashMap<EntityId, usize>,
    special_values: HashMap<(ScopeId, super::function_names::SpecialName), BindingId>,
    function_sets: HashMap<(ScopeId, super::function_names::ValueKey), Vec<BindingId>>,
    function_positions:
        HashMap<(ScopeId, super::function_names::ValueKey, EntityId), FunctionPosition>,
    visible_function_sets: HashMap<(ScopeId, super::function_names::ValueKey), Vec<BindingId>>,
    namespace_signatures: HashMap<FunctionSignature, BindingId>,
    function_origins: HashMap<EntityId, BindingId>,
}
impl Graph {
    pub(super) fn set_closure_binding_type(&mut self, binding: BindingId, ty: TypeId) {
        self.bindings[binding.0 as usize].ty = Some(ty);
    }
    pub(super) fn note_no_inline(
        &mut self,
        entity: EntityId,
        requested: bool,
    ) -> Result<(), Error> {
        self.entity_mut(entity)
            .function
            .as_mut()
            .ok_or(Error::Invariant("function policy owner without function"))?
            .no_inline |= requested;
        Ok(())
    }
    pub(super) fn own_key(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        filter: super::lookup::Filter,
    ) -> Option<BindingId> {
        match key {
            super::function_names::ValueKey::Identifier(name) => self.own(scope, name, filter),
            super::function_names::ValueKey::Special(_) => self
                .value_binding(scope, key)
                .filter(|&id| !self.friendships.hidden(id))
                .or_else(|| self.visible_functions_at(scope, key).next()),
        }
    }
    pub(super) fn value_binding(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
    ) -> Option<BindingId> {
        match key {
            super::function_names::ValueKey::Identifier(name) => {
                self.declared_name(scope, name, super::lookup::Filter::Ordinary)
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
    /// Declaration agreement sees every declaration already introduced,
    /// independently of the namespace lookup limit of an instantiation.
    pub(super) fn redeclared_value(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
    ) -> Option<BindingId> {
        match key {
            super::function_names::ValueKey::Identifier(name) => {
                let entry = self.scope(scope).index.get(&name)?;
                entry.value.or(entry.ty)
            }
            super::function_names::ValueKey::Special(name) => {
                self.special_values.get(&(scope, name)).copied()
            }
        }
    }
    pub(super) fn record_special_value(
        &mut self,
        scope: ScopeId,
        name: super::function_names::SpecialName,
        binding: BindingId,
    ) -> Result<(), Error> {
        if self.binding(binding).kind == BindingKind::Function {
            self.record_function_binding(
                scope,
                super::function_names::ValueKey::Special(name),
                binding,
            )?;
        }
        if let Some(old) = self.special_values.insert((scope, name), binding)
            && !self.friendships.hidden(old)
            && self.binding(old).entity == self.binding(binding).entity
        {
            self.ordinals[binding.0 as usize] = self.ordinals[old.0 as usize];
        }
        Ok(())
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
            hosted: false,
            closures: Box::default(),
            virtuals: super::virtual_members::Virtuals::default(),
            member_initializers: super::member_initializers::Initializers::default(),
            variable_instances: Box::default(),
            types: Types::default(),
            base_paths: super::inheritance::BasePaths::default(),
            dependent_members: super::dependent_members::Members::default(),
            template_operands: Box::default(),
            class_templates: super::class_templates::ClassTemplates::default(),
            templates: super::templates::Templates::default(),
            function_templates: super::function_templates::FunctionTemplates::default(),
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
                template_bridge: false,
                template_lists: 0,
                initialized: 0,
                initialized_at_entry: 0,
                member_access: Access::Public,
                written_name: None,
            }],
            entities: Vec::new(),
            local_classes: super::local_classes::LocalClasses::default(),
            bit_fields: super::bit_fields::BitFields::default(),
            bindings: Vec::new(),
            ordinals: Vec::new(),
            binding_scopes: Vec::new(),
            binding_access: Vec::new(),
            anonymous_enums: 0,
            local_types: 0,
            block_linkages: super::linkage::BlockLinkages::default(),
            conversion_sets: HashMap::new(),
            boundary_roots: Vec::new(),
            boundaries: super::layout::Boundaries::default(),
            instantiation_levels: super::template_context::Levels::default(),
            storage_layouts: Vec::new(),
            constructor_boundaries: Vec::new(),
            storage_destructions: Vec::new(),
            lookup_limit: None,
            global: ScopeId(0),
            declarator_types: Vec::new(),
            declared_bindings: Vec::new(),
            type_facts: Vec::new(),
            resolved_names: Vec::new(),
            enum_declarations: Vec::new(),
            dependent_bindings: Vec::new(),
            binding_labels: HashMap::new(),
            special_labels: HashMap::new(),
            constant_facts: Vec::new(),
            floating_constants: super::constant_floats::Floats::default(),
            aggregate_constants: Box::default(),
            constant_addresses: Box::default(),
            initializer_constructors:
                super::constructor_initialization::InitializerConstructors::default(),
            array_images: super::static_arrays::Images::default(),
            static_constructions: super::static_construction::Images::default(),
            expression_types: Vec::new(),
            expression_facts: super::expression_facts::ExpressionFacts::default(),
            expression_scopes: Vec::new(),
            copy_properties: super::copy_properties::CopyPropertiesStore::default(),
            constructors: super::construction::Constructors::default(),
            inherited_constructors: super::inherited_constructors::InheritedConstructors::default(),
            root_demands: super::function_demands::DemandList::default(),
            declaration_storage: Vec::new(),
            overloads: super::function_resolution::FunctionSets::default(),
            statement_readings: Vec::new(),
            arithmetic_types: Vec::new(),
            condition_functions: std::collections::HashMap::new(),
            condition_conversions: Vec::new(),
            case_values: Vec::new(),
            call_bindings: Vec::new(),
            member_functions: HashMap::new(),
            prototype_parameters: HashMap::new(),
            special_values: HashMap::new(),
            function_sets: HashMap::new(),
            function_positions: HashMap::new(),
            visible_function_sets: HashMap::new(),
            namespace_signatures: HashMap::new(),
            function_origins: HashMap::new(),
            friendships: super::friendships::Friendships::default(),
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
    pub(super) fn dependent_binding(&self, id: BindingId) -> bool {
        self.dependent_bindings[id.0 as usize]
    }
    pub(super) fn set_binding_dependence(&mut self, id: BindingId, dependent: bool) {
        self.dependent_bindings[id.0 as usize] = dependent;
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
            template_bridge: false,
            template_lists: 0,
            initialized: 0,
            initialized_at_entry: self.scope(parent).initialized,
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
            inline: super::specifiers::InlinePreference::Unspecified,
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
            declared_destructor: None,
            implicit_destructor: None,
            destruction: None,
            destruction_preparation: super::destruction::Preparation::Pending,
            declared_conversions: Vec::new(),
            special_definition: None,
            destructor_specification: super::exceptions::DestructorSpecification::Explicit,
            data_member: None,
            mutability: Mutability::Ordinary,
            exceptions: super::exceptions::ExceptionState::Absent,
            target_unwind: super::exceptions::TargetUnwind::MayThrow,
            constexpr: false,
            explicitness: super::specifiers::Explicitness::Converting,
            implicit_copies: None,
            copy_preparation: super::copy_properties::Preparation::default(),
            copy_properties: None,
            copy_origin: super::copy_construction::Origin::Declared,
            copy_kind: None,
            assignment_kind: None,
            assignment: None,
            assignment_origin: super::assignment::Origin::Declared,
            implicit_assignment_use: super::assignment::ImplicitUse::Unused,
            assignment_exceptions: super::assignment::ExceptionSource::Explicit,
            function: None,
            builtin: None,
            definition_emission: super::function_demands::DefinitionEmission::Always,
            function_definition: None,
            analyzed_body: None,
            parameter_entities: Vec::new(),
            function_parameters: None,
            demands: super::function_demands::DemandList::default(),
            member_function: None,
            special_name: None,
            members: Vec::new(),
            polymorphism: super::virtual_members::Polymorphism::Static,
            literal: super::literal_types::LiteralState::NotStarted,
            aggregation: super::aggregate_initialization::Aggregation::Eligible,
            bases: Vec::new(),
            base_dependency: crate::syntax::ast::BaseDependency::Fixed,
            derivation: super::inheritance::Derivation::Allowed,
            layout: super::layout::LayoutState::NotStarted,
            storage_attributes: super::layout_attributes::StorageAttributes::default(),
            class_packing: 0,
            layout_category: super::layout::LayoutCategory::Pod,
            member_offset: None,
            bit_field: None,
            anonymous_class: None,
            local_type_ordinal: None,
            anonymous_storage: None,
            default_construction: super::construction::ConstructorState::NotStarted,
            constructor: None,
            constructor_arguments: None,
            constructor_exceptions: super::construction::ConstructorExceptionSource::Declared,
            member_initializer: None,
            initialized_members: 0,
            variable: None,
            variable_definition: None,
            static_initializer: None,
            reference_temporary: None,
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
        self.add_binding_inner(scope, print, binding, true, false)
    }
    pub(super) fn add_value_binding(
        &mut self,
        scope: ScopeId,
        print: Option<ScopeId>,
        binding: Binding,
        invisible: bool,
    ) -> Result<BindingId, Error> {
        self.add_binding_inner(scope, print, binding, true, invisible)
    }
    pub(super) fn add_hidden_function(
        &mut self,
        origin: BindingId,
        binding: Binding,
    ) -> Result<BindingId, Error> {
        let id = self.add_hidden_binding_in(self.binding_scope(origin), binding)?;
        self.binding_access[id.0 as usize] = self.binding_access(origin);
        Ok(id)
    }
    // A class template's friend is source-facing syntax, not a namespace
    // declaration until that class is instantiated (14.5.4p4).
    pub(super) fn add_pattern_function(
        &mut self,
        home: ScopeId,
        print: ScopeId,
        binding: Binding,
    ) -> Result<BindingId, Error> {
        self.add_binding_inner(home, Some(print), binding, false, true)
    }
    pub(super) fn add_hidden_binding_in(
        &mut self,
        home: ScopeId,
        binding: Binding,
    ) -> Result<BindingId, Error> {
        self.add_binding_inner(home, None, binding, false, false)
    }
    fn add_binding_inner(
        &mut self,
        scope: ScopeId,
        print: Option<ScopeId>,
        binding: Binding,
        visible: bool,
        invisible: bool,
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
                self.function_positions
                    .get(&(scope, key, binding.entity))
                    .and_then(|position| {
                        self.function_sets
                            .get(&(scope, key))?
                            .get(position.declared)
                            .copied()
                    })
            });
        // Imported templates join the overload list anew on each using,
        // unlike redeclarations indexed in their own namespace.
        let imported_template = binding.kind == BindingKind::Function
            && self.entity(binding.entity).template_head.is_some()
            && self
                .entity(binding.entity)
                .home
                .is_some_and(|home| home != scope);
        let earlier = earlier.filter(|&id| !imported_template && !self.friendships.hidden(id));
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
        if invisible {
            self.friendships.hide(id);
        }
        self.bindings.push(binding);
        // Analyzer::declare_in conservatively marks block declarations;
        // variables and typedef declarators refine this after their type forms.
        self.dependent_bindings
            .push(visible && self.scope(scope).kind == ScopeKind::Block);
        self.binding_scopes.push(scope);
        self.ordinals.push(ordinal);
        if binding.kind == BindingKind::Function {
            self.function_origins.entry(binding.entity).or_insert(id);
            if visible && let Some(name) = binding.name {
                self.record_function_binding(scope, name.into(), id)?;
            }
        }
        if let Some(print) = print {
            self.scope_mut(print).bindings.push(id);
        }
        if visible && let Some(name) = binding.name {
            let entry = self.scope_mut(scope).index.entry(name).or_insert(Entry {
                ty: None,
                value: None,
                tag: None,
            });
            if binding.kind == BindingKind::Type && (!invisible || entry.tag.is_none()) {
                entry.tag = Some(id);
            }
            match binding.kind {
                BindingKind::Type
                | BindingKind::Alias
                | BindingKind::Namespace
                | BindingKind::NamespaceAlias => {
                    if !invisible || entry.ty.is_none() {
                        entry.ty = Some(id);
                    }
                }
                _ => {
                    if !invisible || entry.value.is_none() {
                        entry.value = Some(id);
                    }
                }
            }
        }
        Ok(id)
    }
    fn record_function_binding(
        &mut self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        binding: BindingId,
    ) -> Result<(), Error> {
        let entity = self.binding(binding).entity;
        let position_key = (scope, key, entity);
        let imported_template =
            self.entity(entity).template_head.is_some() && self.entity(entity).home != Some(scope);
        let previous = self
            .function_positions
            .get(&position_key)
            .copied()
            .filter(|_| !imported_template);
        let set = self.function_sets.entry((scope, key)).or_default();
        let declared = if let Some(position) = previous {
            set[position.declared] = binding;
            position.declared
        } else {
            let index = set.len();
            set.push(binding);
            index
        };
        let mut visible = previous.and_then(|position| position.visible);
        if !self.friendships.hidden(binding) {
            let set = self.visible_function_sets.entry((scope, key)).or_default();
            if let Some(index) = visible {
                set[index] = binding;
            } else {
                visible = Some(set.len());
                set.push(binding);
            }
        }
        self.function_positions
            .insert(position_key, FunctionPosition { declared, visible });
        if self.scope(scope).kind == ScopeKind::Namespace
            && (self.entity(entity).template_head.is_none()
                || self.entity(entity).home == Some(scope))
        {
            let ty = self
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("function index without type"))?;
            let head = self.entity(entity).template_head;
            let signature_key = self.function_signature(scope, key, ty, head)?;
            if self
                .namespace_signatures
                .get(&signature_key)
                .is_none_or(|&old| self.binding(old).entity == entity)
            {
                self.namespace_signatures.insert(signature_key, binding);
            }
        }
        Ok(())
    }
    pub(super) fn namespace_function(
        &mut self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        ty: TypeId,
        head: Option<super::templates::HeadId>,
    ) -> Result<Option<BindingId>, Error> {
        let signature = self.function_signature(scope, key, ty, head)?;
        Ok(self.namespace_signatures.get(&signature).copied())
    }
    fn function_signature(
        &mut self,
        scope: ScopeId,
        name: super::function_names::ValueKey,
        ty: TypeId,
        head: Option<super::templates::HeadId>,
    ) -> Result<FunctionSignature, Error> {
        let ty = if head.is_some() {
            self.types.declared_type(ty)?
        } else {
            self.types.declaration_signature(ty)?
        };
        Ok(FunctionSignature {
            scope,
            name,
            ty,
            shape: head.map(|head| self.templates.shape(head)),
        })
    }
    // A derived declaration hides an imported base function without changing
    // its entity, original binding or printed declaration. Only this class's
    // name indexes lose the import; the other overloads keep their order.
    pub(super) fn remove_class_function(
        &mut self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        entity: EntityId,
    ) -> Result<(), Error> {
        if self.scope(scope).kind != ScopeKind::Class {
            return Err(Error::Invariant("removing an import outside a class"));
        }
        let position = self
            .function_positions
            .remove(&(scope, key, entity))
            .ok_or(Error::Invariant("removing an unindexed function"))?;
        let declared = self
            .function_sets
            .get_mut(&(scope, key))
            .ok_or(Error::Invariant("missing declared function set"))?;
        declared.remove(position.declared);
        for (index, &binding) in declared.iter().enumerate().skip(position.declared) {
            let function = self.bindings[binding.0 as usize].entity;
            self.function_positions
                .get_mut(&(scope, key, function))
                .ok_or(Error::Invariant("missing declared function position"))?
                .declared = index;
        }
        let replacement = declared.last().copied();
        if let Some(index) = position.visible {
            let visible = self
                .visible_function_sets
                .get_mut(&(scope, key))
                .ok_or(Error::Invariant("missing visible function set"))?;
            visible.remove(index);
            for (index, &binding) in visible.iter().enumerate().skip(index) {
                let function = self.bindings[binding.0 as usize].entity;
                self.function_positions
                    .get_mut(&(scope, key, function))
                    .ok_or(Error::Invariant("missing visible function position"))?
                    .visible = Some(index);
            }
        }
        let replace_value = self
            .value_binding(scope, key)
            .is_some_and(|id| self.binding(id).entity == entity);
        match key {
            super::function_names::ValueKey::Identifier(name) => {
                if let Some(entry) = self.scope_mut(scope).index.get_mut(&name)
                    && replace_value
                {
                    entry.value = replacement;
                }
            }
            super::function_names::ValueKey::Special(name) => {
                if replace_value {
                    self.special_values.remove(&(scope, name));
                    if let Some(binding) = replacement {
                        self.special_values.insert((scope, name), binding);
                    }
                }
            }
        }
        Ok(())
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
    pub(super) fn visible_functions_at(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
    ) -> impl DoubleEndedIterator<Item = BindingId> + '_ {
        self.associated_functions_at(scope, key, self.lookup_limit)
    }
    pub(super) fn associated_functions_at(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        limit: Option<usize>,
    ) -> impl DoubleEndedIterator<Item = BindingId> + '_ {
        self.visible_function_sets
            .get(&(scope, key))
            .into_iter()
            .flatten()
            .copied()
            .filter(move |&id| {
                self.scope(scope).kind != ScopeKind::Namespace
                    || self.binding_visible_before(id, limit)
            })
    }
    pub(super) fn binding_visible_before(&self, id: BindingId, limit: Option<usize>) -> bool {
        limit.is_none_or(|limit| self.ordinals[id.0 as usize] < limit)
    }
    // An ordinal cursor lets declaration matching intern canonical types without
    // borrowing the index or allocating a copy of its candidate set.
    pub(super) fn next_function(
        &self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        next: &mut usize,
    ) -> Option<BindingId> {
        let set = self.function_sets.get(&(scope, key))?;
        while let Some(&id) = set.get(*next) {
            *next += 1;
            if self.scope(scope).kind != ScopeKind::Namespace
                || self
                    .lookup_limit
                    .is_none_or(|limit| self.ordinals[id.0 as usize] < limit)
            {
                return Some(id);
            }
        }
        None
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
                default
                    .added_at
                    .is_none_or(|ordinal| self.lookup_limit.is_none_or(|limit| ordinal < limit))
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
        let binding = self.declared_name(scope, name, filter)?;
        if !self.friendships.hidden(binding) {
            return Some(binding);
        }
        if self.binding(binding).kind == BindingKind::Function {
            return self
                .visible_functions_at(scope, name.into())
                .next()
                .or_else(|| {
                    self.declared_name(scope, name, super::lookup::Filter::Prefix)
                        .filter(|&id| !self.friendships.hidden(id))
                });
        }
        None
    }
    pub(super) fn declared_name(
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

    // analyzer_classes.cpp::earlier_class: declarations ignore the lookup
    // point's ordinal, including a class introduced only by hidden friends.
    // Ordinary name lookup continues to use own/declared_name's limit.
    pub(super) fn class_declaration(&self, scope: ScopeId, name: NameId) -> Option<BindingId> {
        let entry = self.scope(scope).index.get(&name)?;
        entry.tag.or(entry.ty).filter(|&id| {
            !matches!(
                self.binding(id).kind,
                BindingKind::Namespace | BindingKind::NamespaceAlias
            )
        })
    }
    pub(super) fn add_implicit_copy_binding(
        &mut self,
        home: ScopeId,
        function: EntityId,
        signature: TypeId,
    ) -> Result<BindingId, Error> {
        let binding = self.add_binding(
            home,
            None,
            Binding {
                kind: BindingKind::Function,
                name: None,
                entity: function,
                ty: Some(signature),
                key: Keyword::Int,
            },
        )?;
        self.binding_access[binding.0 as usize] = Access::Public;
        Ok(binding)
    }
    pub(super) fn add_implicit_assignment(
        &mut self,
        class: EntityId,
        signature: TypeId,
        kind: super::types::RefKind,
        properties: super::assignment::Properties,
    ) -> Result<BindingId, Error> {
        let home = self
            .entity(class)
            .scope
            .ok_or(Error::Invariant("assignment class scope"))?;
        let function = self.add_entity(Some(signature), None, Keyword::Int)?;
        let scope = self.add_scope(ScopeKind::Function, home, None, None)?;
        self.scope_mut(scope).owner = Some(function);
        let special = super::assignment::key()?
            .special()
            .ok_or(Error::Invariant("assignment key"))?;
        let entity = self.entity_mut(function);
        entity.scope = Some(scope);
        entity.defined = true;
        entity.inline = super::specifiers::InlinePreference::Inline;
        entity.definition_emission = super::function_demands::DefinitionEmission::Demanded;
        entity.assignment_kind = Some(kind);
        entity.assignment_origin = super::assignment::Origin::Implicit;
        entity.availability = if properties.deleted {
            FunctionAvailability::Deleted
        } else {
            FunctionAvailability::Callable
        };
        entity.exceptions = super::exceptions::ExceptionState::Value(if properties.nothrow {
            super::exceptions::ExceptionSet::None
        } else {
            super::exceptions::ExceptionSet::All
        });
        entity.assignment = Some(properties);
        entity.function = Some(FunctionProperties {
            no_inline: false,
            internal_linkage: false,
            language: super::linkage::Language::Cpp,
            denotes: None,
            is_static: false,
            special: Some(special),
            allocation: None,
        });
        let parameter = self.types.function_parameter(signature, 0)?;
        let other = self.add_entity(Some(parameter), None, Keyword::Int)?;
        self.add_binding(
            scope,
            Some(scope),
            Binding {
                kind: BindingKind::Parameter,
                name: None,
                entity: other,
                ty: Some(parameter),
                key: Keyword::Int,
            },
        )?;
        let binding = self.add_binding(
            home,
            None,
            Binding {
                kind: BindingKind::Function,
                name: None,
                entity: function,
                ty: Some(signature),
                key: Keyword::Int,
            },
        )?;
        self.binding_access[binding.0 as usize] = Access::Public;
        self.record_special_value(home, special, binding)?;
        Ok(binding)
    }
    pub(super) fn add_implicit_destructor(
        &mut self,
        class: EntityId,
        signature: TypeId,
    ) -> Result<BindingId, Error> {
        let scope = self
            .entity(class)
            .scope
            .ok_or(Error::Invariant("implicit destructor without class scope"))?;
        let function = self.add_entity(Some(signature), None, Keyword::Int)?;
        let entity = self.entity_mut(function);
        entity.defined = true;
        entity.inline = super::specifiers::InlinePreference::Inline;
        entity.special_definition = Some(super::special_members::SpecialDefinition::Defaulted {
            first: true,
            body: None,
        });
        entity.definition_emission = super::function_demands::DefinitionEmission::Demanded;
        entity.exceptions =
            super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
        entity.member_function = Some(MemberKey {
            class,
            kind: MemberKind::Destructor,
            signature,
            shape: None,
        });
        entity.function = Some(FunctionProperties {
            no_inline: false,
            language: super::linkage::Language::Cpp,
            denotes: None,
            internal_linkage: false,
            is_static: false,
            special: None,
            allocation: None,
        });
        let binding = self.add_binding(
            scope,
            None,
            Binding {
                kind: BindingKind::Function,
                name: None,
                entity: function,
                ty: Some(signature),
                key: Keyword::Int,
            },
        )?;
        self.binding_access[binding.0 as usize] = Access::Public;
        self.entity_mut(class).implicit_destructor = Some(binding);
        Ok(binding)
    }
    pub(super) fn binding_access(&self, id: BindingId) -> Access {
        self.binding_access[id.0 as usize]
    }
    pub(super) fn inherit_binding_access(&mut self, binding: BindingId, origin: BindingId) {
        self.binding_access[binding.0 as usize] = self.binding_access(origin);
    }
    pub(super) fn import_conversion(
        &mut self,
        scope: ScopeId,
        target: BindingId,
    ) -> Result<(), Error> {
        let class = (self.scope(scope).kind == ScopeKind::Class)
            .then_some(self.scope(scope).owner)
            .flatten();
        let binding = self.add_binding(scope, Some(scope), self.binding(target))?;
        // A using-declaration has the access of its declaration, while its
        // entity, function origin and receiver remain those of the base member.
        self.binding_access[binding.0 as usize] = self.scope(scope).member_access;
        self.record_special_value(
            scope,
            super::function_names::SpecialName::Conversion,
            binding,
        )?;
        if let Some(&label) = self.special_labels.get(&target) {
            self.special_labels.insert(binding, label);
        }
        let Some(class) = class else {
            return Ok(());
        };
        let entity = self.binding(target).entity;
        let earlier = self
            .entity(class)
            .declared_conversions
            .iter()
            .position(|&id| self.binding(id).entity == entity);
        let conversions = &mut self.entity_mut(class).declared_conversions;
        if let Some(index) = earlier {
            conversions[index] = binding;
        } else {
            conversions.push(binding);
        }
        Ok(())
    }
    pub(super) fn hide_imported_conversions(&mut self, key: MemberKey) -> Result<(), Error> {
        let scope = self
            .entity(key.class)
            .scope
            .ok_or(Error::Invariant("conversion class without scope"))?;
        let mut index = 0;
        while let Some(&binding) = self.entity(key.class).declared_conversions.get(index) {
            let entity = self.binding(binding).entity;
            let old = self.entity(entity);
            if old.home != Some(scope)
                && old
                    .member_function
                    .is_some_and(|old| old.signature == key.signature && old.shape == key.shape)
            {
                self.remove_class_function(
                    scope,
                    super::function_names::ValueKey::Special(
                        super::function_names::SpecialName::Conversion,
                    ),
                    entity,
                )?;
                self.entity_mut(key.class)
                    .declared_conversions
                    .remove(index);
            } else {
                index += 1;
            }
        }
        Ok(())
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
    pub(super) fn record_condition_function(
        &mut self,
        id: crate::syntax::ast::DeclaratorId,
        selected: super::conversion_functions::FunctionConversion,
    ) {
        self.condition_functions.insert(id, selected);
    }
    pub(super) fn condition_function(
        &self,
        id: crate::syntax::ast::DeclaratorId,
    ) -> Option<super::conversion_functions::FunctionConversion> {
        self.condition_functions.get(&id).copied()
    }
    pub(super) fn record_condition_conversion(
        &mut self,
        declarator: crate::syntax::ast::DeclaratorId,
        mut conversion: super::expression_facts::Conversion,
    ) {
        conversion.initialization = false;
        super::record(
            &mut self.condition_conversions,
            declarator.index(),
            conversion,
        );
    }
    pub(super) fn condition_conversion(
        &self,
        declarator: crate::syntax::ast::DeclaratorId,
    ) -> Result<super::expression_facts::Conversion, Error> {
        self.condition_conversions
            .get(declarator.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("condition declaration without conversion"))
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
