// Translated from sema/semantic_unit.h and analyzer.cpp's graph construction. See NOTICE.
//! Unit-local identities; lookup indexes and source-facing order are separate.
use super::{
    Error,
    types::{TypeId, Types},
};
use crate::lex::{identifiers::NameId, vocabulary::Keyword};
use crate::support::hash::HashMap;
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
    // Prefix carrier for an unscoped enumeration; not a declared scope.
    EnumMembers,
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeEntry {
    Ordinary,
    Exception,
    SelectionInit,
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
    pub entry: ScopeEntry,
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
    pub effects: FunctionEffect,
    pub no_inline: bool,
    pub returns: FunctionReturn,
    pub internal_linkage: bool,
    pub language: super::linkage::Language,
    pub denotes: Option<EntityId>,
    pub is_static: bool,
    pub special: Option<super::function_names::SpecialName>,
    pub allocation: Option<super::function_names::AllocationOrigin>,
}
/// Declaration property, deliberately absent from canonical function types.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FunctionReturn {
    MayReturn,
    NoReturn,
}
/// Ordered as the frozen callable's strongest declared memory guarantee.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FunctionEffect {
    #[default]
    Default,
    ReadOnly,
    ReadNone,
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
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ClassRepresentation {
    #[default]
    Program,
    BuiltinInitializerList,
}
pub(super) struct Entity {
    pub inline: super::specifiers::InlinePreference,
    pub ty: Option<TypeId>,
    pub home: Option<ScopeId>,
    pub name: Option<NameId>,
    pub key: Keyword,
    pub defined: bool,
    pub availability: FunctionAvailability,
    pub complete: bool,
    pub member_access: Access,
    pub mutability: Mutability,
    pub constexpr: bool,
}
impl Entity {
    pub(super) fn new(ty: Option<TypeId>, name: Option<NameId>, key: Keyword) -> Self {
        Self {
            inline: super::specifiers::InlinePreference::Unspecified,
            ty,
            home: None,
            name,
            key,
            defined: false,
            availability: FunctionAvailability::Callable,
            complete: false,
            member_access: Access::Public,
            mutability: Mutability::Ordinary,
            constexpr: false,
        }
    }
}

const _: () = assert!(std::mem::size_of::<Entity>() <= 32);
#[derive(Clone, Copy)]
enum BindingOrigin {
    Declaration(std::num::NonZeroUsize),
    Callable,
}
impl BindingOrigin {
    fn declaration(ordinal: usize) -> Result<Self, Error> {
        ordinal
            .checked_add(1)
            .and_then(std::num::NonZeroUsize::new)
            .map(Self::Declaration)
            .ok_or(Error::Limit("semantic declaration limit"))
    }
    fn before(self, limit: usize) -> bool {
        match self {
            Self::Declaration(ordinal) => ordinal.get() <= limit,
            Self::Callable => true,
        }
    }
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
    shape: Option<TypeId>,
}

pub(crate) struct Graph {
    pub(super) localities: Box<super::locality::Queries>,
    // Immutable name identities survive every temporary analyzer facade.
    pub(super) builtin_names: Option<std::rc::Rc<super::builtin_calls::BuiltinNames>>,
    pub(super) analysis: Box<super::analyzer::UnitState>,
    pub(super) work: Box<super::work::Observations>,
    pub(super) names: Box<super::lookup::Names>,
    pub(super) hosted: bool,
    pub(super) cxx14: bool,
    pub(super) exceptions_enabled: bool,
    pub(super) reserved_deferrals: Box<super::reserved_wrappers::Deferrals>,
    pub(super) predefined_variables: super::predefined_variables::Variables,
    pub(super) property_predicates: super::property_predicates::Properties,
    pub(super) expression_predicates: Box<super::expression_predicates::Queries>,
    pub(super) triviality: super::triviality::Triviality,
    pub(super) alias_alignments: super::aligned_aliases::AliasAlignments,
    pub(super) symbol_attributes: super::symbol_attributes::SymbolAttributes,
    pub(super) abi_tags: super::abi_tags::AbiTags,
    pub(super) closures: Box<super::lambdas::Closures>,
    pub(super) decompositions: super::bindings::Bindings,
    pub(super) range_loops: Box<super::range_for::RangeLoops>,
    pub(super) virtuals: Box<super::virtual_members::Virtuals>,
    pub(super) local_classes: super::local_classes::LocalClasses,
    pub(super) bit_fields: super::bit_fields::BitFields,
    // The unit owns the canonical table through analysis and lowering. Moving
    // a graph between demand epochs moves its handle, not the table's indexes.
    pub(super) types: Box<Types>,
    auto_placeholder: Option<TypeId>,
    pub(super) friendships: super::friendships::Friendships,
    pub(super) base_paths: Box<super::inheritance::BasePaths>,
    pub(super) dependent_members: super::dependent_members::Members,
    // Unit-owned query tables stay off the recursive analyzer's stack value.
    pub(super) template_operands: Box<super::template_operands::Operands>,
    pub(super) class_templates: super::class_templates::ClassTemplates,
    pub(super) templates: super::templates::Templates,
    pub(super) function_templates: super::function_templates::FunctionTemplates,
    scopes: Vec<Scope>,
    entities: Vec<Entity>,
    entity_roles: Vec<super::entity_records::Roles>,
    entity_records: Box<super::entity_records::Records>,
    bindings: Vec<Binding>,
    binding_origins: Vec<BindingOrigin>,
    redeclarations: Vec<bool>,
    declaration_count: usize,
    binding_scopes: Vec<ScopeId>,
    binding_access: Vec<Access>,
    member_imports: crate::support::hash::HashSet<EntityId>,
    pub(super) anonymous_enums: u32,
    pub(super) local_types: u32,
    pub(super) block_linkages: super::linkage::BlockLinkages,
    pub(super) conversion_sets: HashMap<EntityId, Vec<BindingId>>,
    pub(super) boundary_roots: Vec<super::graph::EntityId>,
    // One unit-owned scheduler; keep its growing indexes off native call frames.
    pub(super) boundaries: Box<super::layout::Boundaries>,
    pub(super) rtti_types: crate::support::hash::HashMap<TypeId, TypeId>,
    pub(super) class_information_uses: Vec<(Option<EntityId>, EntityId)>,
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
    pub(super) constant_member_spans: super::constant_pointer_operations::MemberSpans,
    pub(super) aggregate_constants: Box<super::constexpr_values::Values>,
    pub(super) constant_addresses: Box<super::constant_addresses::Addresses>,
    pub(super) initializer_constructors: super::constructor_initialization::InitializerConstructors,
    pub(super) array_images: super::static_arrays::Images,
    pub(super) static_constructions: super::static_construction::Images,
    pub(super) resolved_names: Vec<Option<BindingId>>,
    pub(super) expression_types: Vec<Option<TypeId>>,
    pub(super) expression_facts: Box<super::expression_facts::ExpressionFacts>,
    pub(super) literal_numbers: Box<super::literal_numbers::Numbers>,
    pub(super) source_exceptions: Box<super::source_exceptions::SourceExceptions>,
    pub(super) specifications: Box<super::exceptions::Specifications>,
    pub(super) expression_scopes: Vec<Option<ScopeId>>,
    pub(super) copy_properties: super::copy_properties::CopyPropertiesStore,
    pub(super) constructors: super::construction::Constructors,
    pub(super) inherited_constructors: super::inherited_constructors::InheritedConstructors,
    pub(super) member_initializers: super::member_initializers::Initializers,
    pub(super) variable_instances: Box<super::variable_instances::Variables>,
    pub(super) root_demands: super::function_demands::DemandList,
    pub(super) demand_marks: super::function_demands::DemandMarks,
    pub(super) declaration_storage: Vec<Option<BindingId>>,
    pub(super) overloads: super::function_resolution::FunctionSets,
    pub(super) assembly: super::assembly::Programs,
    pub(super) constexpr_selections: Vec<Option<bool>>,
    pub(super) statement_readings: Vec<Option<crate::syntax::ast::StmtId>>,
    arithmetic_types: Vec<Option<(TypeId, TypeId)>>,
    condition_functions: crate::support::hash::HashMap<
        crate::syntax::ast::DeclaratorId,
        super::conversion_functions::FunctionConversion,
    >,
    condition_conversions: Vec<Option<super::expression_facts::Conversion>>,
    pub(super) case_values: Vec<Option<u64>>,
    call_bindings: Vec<Option<BindingId>>,
    member_functions: HashMap<MemberKey, BindingId>,
    prototype_parameters: HashMap<EntityId, usize>,
    parameter_objects: HashMap<EntityId, TypeId>,
    special_values: HashMap<(ScopeId, super::function_names::SpecialName), Option<BindingId>>,
    function_sets: HashMap<(ScopeId, super::function_names::ValueKey), Vec<BindingId>>,
    function_positions:
        HashMap<(ScopeId, super::function_names::ValueKey, EntityId), FunctionPosition>,
    visible_function_sets: HashMap<(ScopeId, super::function_names::ValueKey), Vec<BindingId>>,
    function_signatures: HashMap<FunctionSignature, BindingId>,
    function_origins: HashMap<EntityId, BindingId>,
}
impl Graph {
    pub(super) fn initialize_intrinsics(&mut self, name: NameId) -> Result<(), Error> {
        self.types.initialize_fundamentals()?;
        let placeholder = self.add_entity(None, Some(name), Keyword::Auto)?;
        let auto = self.types.intern(
            super::types::TypeKind::Auto(placeholder),
            super::types::Cv::default(),
        )?;
        self.entity_mut(placeholder).ty = Some(auto);
        self.auto_placeholder = Some(auto);
        let global = self.add_entity(None, None, Keyword::Namespace)?;
        self.entity_mut(global).context_mut().scope = Some(self.global);
        self.scope_mut(self.global).owner = Some(global);
        Ok(())
    }
    pub(super) fn auto_type(&self) -> Result<TypeId, Error> {
        self.auto_placeholder
            .ok_or(Error::Invariant("missing auto placeholder"))
    }
    #[cfg(feature = "telemetry")]
    pub(crate) fn statistics(&self) -> super::Statistics {
        let (names, lookups, member_cache) = self.names.observations();
        let (types, type_bytes) = self.types.observations();
        super::Statistics {
            scopes: self
                .scopes
                .iter()
                .filter(|scope| scope.kind != ScopeKind::EnumMembers)
                .count(),
            bindings: self.declaration_count,
            entities: self.entities.len(),
            types,
            names,
            lookups,
            member_cache,
            arena_bytes: self
                .entity_records
                .storage_bytes()
                .saturating_add(
                    self.entity_roles
                        .capacity()
                        .saturating_mul(std::mem::size_of::<super::entity_records::Roles>()),
                )
                .saturating_add(type_bytes)
                .saturating_add(self.specifications.storage_bytes())
                .saturating_add(
                    self.scopes
                        .capacity()
                        .saturating_mul(std::mem::size_of::<Scope>()),
                )
                .saturating_add(
                    self.entities
                        .capacity()
                        .saturating_mul(std::mem::size_of::<Entity>()),
                )
                .saturating_add(
                    self.bindings
                        .capacity()
                        .saturating_mul(std::mem::size_of::<Binding>()),
                ),
            work: self.work(),
        }
    }
    #[cfg(not(feature = "telemetry"))]
    pub(crate) fn statistics(&self) -> super::Statistics {
        let _ = self;
        super::observations::Statistics::default()
    }
    // Unscoped enumerators are canonical declarations in their enclosing
    // scope. Their qualifier checks that same current entry (not an alias
    // binding or a second index), independently of namespace lookup limits.
    pub(super) fn enum_member(
        &self,
        scope: ScopeId,
        name: super::function_names::ValueKey,
        filter: super::lookup::Filter,
    ) -> Option<BindingId> {
        if filter != super::lookup::Filter::Ordinary {
            return None;
        }
        let enumeration = self.scope(scope).owner?;
        let owner = self.entity(enumeration).enumerated().enumeration?.owner;
        let binding = self.scope(owner).index.get(&name.identifier()?)?.value?;
        if self.binding(binding).kind != BindingKind::Enumerator {
            return None;
        }
        let declaration = self.enum_declaration(binding)?;
        (self.binding(declaration).entity == enumeration).then_some(binding)
    }
    pub(crate) fn work(&self) -> super::Work {
        self.work.snapshot()
    }
    pub(super) fn note_effects(
        &mut self,
        entity: EntityId,
        requested: FunctionEffect,
    ) -> Result<(), Error> {
        let mut record = self.entity_mut(entity);
        let properties = record
            .callable_mut()
            .function
            .as_mut()
            .ok_or(Error::Invariant("function effect owner without function"))?;
        properties.effects = properties.effects.max(requested);
        Ok(())
    }
    pub(super) fn set_function_binding_type(&mut self, binding: BindingId, ty: TypeId) {
        self.bindings[binding.0 as usize].ty = Some(ty);
    }
    pub(super) fn note_no_inline(
        &mut self,
        entity: EntityId,
        requested: bool,
    ) -> Result<(), Error> {
        self.entity_mut(entity)
            .callable_mut()
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
            super::function_names::ValueKey::Special(name) => {
                let binding = self.special_values.get(&(scope, name)).copied().flatten()?;
                if self.scope(scope).kind == ScopeKind::Namespace && self.lookup_limit.is_some() {
                    self.work.limited_lookup();
                }
                Some(binding).filter(|id| {
                    self.scope(scope).kind != ScopeKind::Namespace
                        || self
                            .lookup_limit
                            .is_none_or(|limit| self.binding_origins[id.0 as usize].before(limit))
                })
            }
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
                self.special_values.get(&(scope, name)).copied().flatten()
            }
        }
    }
    pub(super) fn binding_redeclared(&self, binding: BindingId) -> bool {
        self.redeclarations[binding.0 as usize]
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
        let new = !self.special_values.contains_key(&(scope, name));
        self.note_lookup_declaration(scope, super::function_names::ValueKey::Special(name), new);
        if let Some(old) = self
            .special_values
            .insert((scope, name), Some(binding))
            .flatten()
            && !self.friendships.hidden(old)
            && self.binding(old).entity == self.binding(binding).entity
        {
            self.binding_origins[binding.0 as usize] = self.binding_origins[old.0 as usize];
            self.redeclarations[binding.0 as usize] = true;
        }
        Ok(())
    }
    pub(super) fn record_prototype_parameter(
        &mut self,
        entity: EntityId,
        index: usize,
        object: TypeId,
    ) {
        self.prototype_parameters.insert(entity, index);
        self.record_parameter_object(entity, object);
    }
    pub(super) fn record_parameter_object(&mut self, entity: EntityId, object: TypeId) {
        if self.entity(entity).ty != Some(object) {
            self.parameter_objects.insert(entity, object);
        }
    }
    pub(super) fn parameter_object(&self, entity: EntityId) -> Option<TypeId> {
        self.parameter_objects.get(&entity).copied()
    }
    pub(super) fn prototype_parameter(&self, entity: EntityId) -> Option<usize> {
        self.prototype_parameters.get(&entity).copied()
    }
    pub(super) fn integral_kind(&self, ty: TypeId) -> Option<super::integral::IntegralKind> {
        let ty = if let super::types::TypeKind::Enum(entity) = self.types.get(ty).kind {
            self.entity(entity).enumerated().enumeration?.underlying?
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
            localities: Box::default(),
            builtin_names: None,
            analysis: Box::default(),
            work: Box::default(),
            names: Box::default(),
            hosted: false,
            cxx14: false,
            exceptions_enabled: true,
            reserved_deferrals: Box::default(),
            predefined_variables: super::predefined_variables::Variables::default(),
            property_predicates: super::property_predicates::Properties::default(),
            expression_predicates: Box::default(),
            triviality: super::triviality::Triviality::default(),
            alias_alignments: super::aligned_aliases::AliasAlignments::default(),
            symbol_attributes: super::symbol_attributes::SymbolAttributes::default(),
            abi_tags: super::abi_tags::AbiTags::default(),
            closures: Box::default(),
            decompositions: super::bindings::Bindings::default(),
            range_loops: Box::default(),
            virtuals: Box::default(),
            member_initializers: super::member_initializers::Initializers::default(),
            variable_instances: Box::default(),
            types: Box::default(),
            auto_placeholder: None,
            base_paths: Box::default(),
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
                index: HashMap::default(),
                directives: Vec::new(),
                inline_members: Vec::new(),
                is_inline: false,
                guarded: false,
                entry: ScopeEntry::Ordinary,
                template_bridge: false,
                template_lists: 0,
                initialized: 0,
                initialized_at_entry: 0,
                member_access: Access::Public,
                written_name: None,
            }],
            entities: Vec::new(),
            entity_roles: Vec::new(),
            entity_records: Box::default(),
            local_classes: super::local_classes::LocalClasses::default(),
            bit_fields: super::bit_fields::BitFields::default(),
            bindings: Vec::new(),
            binding_origins: Vec::new(),
            redeclarations: Vec::new(),
            declaration_count: 0,
            binding_scopes: Vec::new(),
            binding_access: Vec::new(),
            member_imports: crate::support::hash::HashSet::default(),
            anonymous_enums: 0,
            local_types: 0,
            block_linkages: super::linkage::BlockLinkages::default(),
            conversion_sets: HashMap::default(),
            boundary_roots: Vec::new(),
            boundaries: Box::default(),
            class_information_uses: Vec::new(),
            rtti_types: crate::support::hash::HashMap::default(),
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
            binding_labels: HashMap::default(),
            special_labels: HashMap::default(),
            constant_facts: Vec::new(),
            floating_constants: super::constant_floats::Floats::default(),
            constant_member_spans: super::constant_pointer_operations::MemberSpans::default(),
            aggregate_constants: Box::default(),
            constant_addresses: Box::default(),
            initializer_constructors:
                super::constructor_initialization::InitializerConstructors::default(),
            array_images: super::static_arrays::Images::default(),
            static_constructions: super::static_construction::Images::default(),
            expression_types: Vec::new(),
            expression_facts: Box::default(),
            literal_numbers: Box::default(),
            source_exceptions: Box::default(),
            specifications: Box::default(),
            expression_scopes: Vec::new(),
            copy_properties: super::copy_properties::CopyPropertiesStore::default(),
            constructors: super::construction::Constructors::default(),
            inherited_constructors: super::inherited_constructors::InheritedConstructors::default(),
            root_demands: super::function_demands::DemandList::default(),
            demand_marks: super::function_demands::DemandMarks::default(),
            declaration_storage: Vec::new(),
            overloads: super::function_resolution::FunctionSets::default(),
            assembly: super::assembly::Programs::default(),
            statement_readings: Vec::new(),
            constexpr_selections: Vec::new(),
            arithmetic_types: Vec::new(),
            condition_functions: crate::support::hash::HashMap::default(),
            condition_conversions: Vec::new(),
            case_values: Vec::new(),
            call_bindings: Vec::new(),
            member_functions: HashMap::default(),
            prototype_parameters: HashMap::default(),
            parameter_objects: HashMap::default(),
            special_values: HashMap::default(),
            function_sets: HashMap::default(),
            function_positions: HashMap::default(),
            visible_function_sets: HashMap::default(),
            function_signatures: HashMap::default(),
            function_origins: HashMap::default(),
            friendships: super::friendships::Friendships::default(),
        }
    }
    pub(super) fn scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0 as usize]
    }
    pub(super) fn scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id.0 as usize]
    }
    #[inline]
    pub(super) fn entity(&self, id: EntityId) -> super::entity_records::EntityRef<'_> {
        super::entity_records::EntityRef::new(self, id)
    }
    #[inline]
    pub(super) fn entity_mut(&mut self, id: EntityId) -> super::entity_records::EntityMut<'_> {
        super::entity_records::EntityMut::new(self, id)
    }
    #[inline]
    pub(super) fn entity_header(&self, id: EntityId) -> &Entity {
        &self.entities[id.0 as usize]
    }
    #[inline]
    pub(super) fn entity_header_mut(&mut self, id: EntityId) -> &mut Entity {
        &mut self.entities[id.0 as usize]
    }
    #[inline]
    pub(super) fn entity_context(&self, id: EntityId) -> &super::entity_records::ContextFacts {
        self.entity_records
            .context(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_context_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::ContextFacts {
        self.entity_records
            .context_mut(&mut self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_enumerated(
        &self,
        id: EntityId,
    ) -> &super::entity_records::EnumerationFacts {
        self.entity_records
            .enumerated(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_enumerated_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::EnumerationFacts {
        self.entity_records
            .enumerated_mut(&mut self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_class(&self, id: EntityId) -> &super::entity_records::ClassFacts {
        self.entity_records.class(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_class_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::ClassFacts {
        self.entity_records
            .class_mut(&mut self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_special(&self, id: EntityId) -> &super::entity_records::SpecialFacts {
        self.entity_records
            .special(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_special_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::SpecialFacts {
        self.entity_records
            .special_mut(&mut self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_callable(&self, id: EntityId) -> &super::entity_records::CallableFacts {
        self.entity_records
            .callable(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_callable_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::CallableFacts {
        self.entity_records
            .callable_mut(&mut self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_object(&self, id: EntityId) -> &super::entity_records::ObjectFacts {
        self.entity_records
            .object(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_object_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::ObjectFacts {
        self.entity_records
            .object_mut(&mut self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_initialization(
        &self,
        id: EntityId,
    ) -> &super::entity_records::InitializationFacts {
        self.entity_records
            .initialization(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_initialization_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::InitializationFacts {
        self.entity_records
            .initialization_mut(&mut self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_attributes(
        &self,
        id: EntityId,
    ) -> &super::entity_records::AttributesFacts {
        self.entity_records
            .attributes(&self.entity_roles[id.0 as usize])
    }
    #[inline]
    pub(super) fn entity_attributes_mut(
        &mut self,
        id: EntityId,
    ) -> &mut super::entity_records::AttributesFacts {
        self.entity_records
            .attributes_mut(&mut self.entity_roles[id.0 as usize])
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
            index: HashMap::default(),
            directives: Vec::new(),
            inline_members: Vec::new(),
            is_inline: false,
            guarded: false,
            entry: ScopeEntry::Ordinary,
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
                .ok()
                .and_then(|n| n.checked_add(1))
                .ok_or(Error::Limit("semantic entity limit"))?
                - 1,
        );
        self.entity_roles
            .push(super::entity_records::Roles::default());
        self.entities.push(Entity::new(ty, name, key));
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
    pub(super) fn function_handle(
        &mut self,
        origin: BindingId,
        function: EntityId,
        signature: TypeId,
    ) -> Result<BindingId, Error> {
        let source = self.binding(origin);
        self.store_binding(
            self.binding_scope(origin),
            Binding {
                kind: BindingKind::Function,
                name: source.name,
                entity: function,
                ty: Some(signature),
                key: source.key,
            },
            self.binding_access(origin),
            BindingOrigin::Callable,
            false,
            false,
        )
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
                    .callable()
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
                        self.entity(binding.entity)
                            .callable()
                            .function
                            .and_then(|properties| {
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
            && self
                .entity(binding.entity)
                .context()
                .template_head
                .is_some()
            && self
                .entity(binding.entity)
                .home
                .is_some_and(|home| home != scope);
        let earlier = earlier.filter(|&id| !imported_template && !self.friendships.hidden(id));
        // A compatible redeclaration is visible since the entity's first
        // visible binding, just as C++ NameLookup::KeepOrdinal specifies.
        let introduced = self.next_declaration_ordinal()?;
        let ordinal = earlier.map_or(BindingOrigin::declaration(introduced)?, |id| {
            self.binding_origins[id.0 as usize]
        });
        if self.entity(binding.entity).home.is_none() {
            self.entity_mut(binding.entity).home = Some(scope);
        }
        if self.scope(scope).kind == ScopeKind::Class
            && self.scope(scope).owner != Some(binding.entity)
            && self
                .entity(binding.entity)
                .home
                .is_some_and(|home| home != scope && self.scope(home).kind == ScopeKind::Class)
        {
            self.member_imports.insert(binding.entity);
        }
        let access = earlier.map_or(self.scope(scope).member_access, |id| {
            self.binding_access(id)
        });
        let id = self.store_binding(
            scope,
            binding,
            access,
            ordinal,
            visible && self.scope(scope).kind == ScopeKind::Block,
            earlier.is_some(),
        )?;
        if invisible {
            self.friendships.hide(id);
        }
        if binding.kind == BindingKind::Function
            && visible
            && let Some(name) = binding.name
        {
            self.record_function_binding(scope, name.into(), id)?;
        }
        if let Some(print) = print {
            self.scope_mut(print).bindings.push(id);
        }
        if visible {
            self.publish_binding_name(scope, id);
        }
        Ok(id)
    }
    fn store_binding(
        &mut self,
        scope: ScopeId,
        binding: Binding,
        access: Access,
        origin: BindingOrigin,
        dependent: bool,
        redeclared: bool,
    ) -> Result<BindingId, Error> {
        let id = BindingId(
            u32::try_from(self.bindings.len())
                .map_err(|_| Error::Limit("semantic binding limit"))?,
        );
        self.bindings.push(binding);
        self.binding_access.push(access);
        self.dependent_bindings.push(dependent);
        self.binding_scopes.push(scope);
        self.binding_origins.push(origin);
        self.redeclarations.push(redeclared);
        if binding.kind == BindingKind::Function {
            self.function_origins.entry(binding.entity).or_insert(id);
        }
        Ok(id)
    }
    // An implicit callable needs a Rust handle, but C++ has no source Binding
    // for it. It neither advances declaration ordinals nor enters lookup/print.
    fn callable_binding(
        &mut self,
        home: ScopeId,
        function: EntityId,
        signature: TypeId,
    ) -> Result<BindingId, Error> {
        self.store_binding(
            home,
            Binding {
                kind: BindingKind::Function,
                name: None,
                entity: function,
                ty: Some(signature),
                key: Keyword::Int,
            },
            Access::Public,
            BindingOrigin::Callable,
            false,
            false,
        )
    }
    // A bridge republishes a declaration; it does not introduce another
    // binding, change its original owner/ordinal or attach another print node.
    pub(super) fn bind_template_parameter(
        &mut self,
        scope: ScopeId,
        id: BindingId,
    ) -> Result<(), Error> {
        if self.scope(scope).kind != ScopeKind::TemplateParameters
            || self
                .entity(self.binding(id).entity)
                .context()
                .template_parameter
                .is_none()
        {
            return Err(Error::Invariant("invalid template bridge declaration"));
        }
        self.publish_binding_name(scope, id);
        Ok(())
    }
    fn publish_binding_name(&mut self, scope: ScopeId, id: BindingId) {
        let binding = self.binding(id);
        let invisible = self.friendships.hidden(id);
        if let Some(name) = binding.name {
            let (entry, new) = match self.scope_mut(scope).index.entry(name) {
                std::collections::hash_map::Entry::Occupied(entry) => (entry.into_mut(), false),
                std::collections::hash_map::Entry::Vacant(entry) => (
                    entry.insert(Entry {
                        ty: None,
                        value: None,
                        tag: None,
                    }),
                    true,
                ),
            };
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
            self.note_lookup_declaration(scope, name.into(), new);
        }
    }
    fn note_lookup_declaration(
        &mut self,
        scope: ScopeId,
        name: super::function_names::ValueKey,
        new: bool,
    ) {
        let class = self.scope(scope).kind == ScopeKind::Class;
        let complete = self.complete_lookup_class(scope);
        self.names.declared(name, new, class, complete);
    }
    pub(super) fn complete_lookup_class(&self, scope: ScopeId) -> bool {
        self.scope(scope).kind == ScopeKind::Class
            && self
                .scope(scope)
                .owner
                .is_some_and(|owner| self.entity(owner).complete)
    }
    fn record_function_binding(
        &mut self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        binding: BindingId,
    ) -> Result<(), Error> {
        let entity = self.binding(binding).entity;
        let position_key = (scope, key, entity);
        let imported_template = self.entity(entity).context().template_head.is_some()
            && self.entity(entity).home != Some(scope);
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
        if self.entity(entity).context().template_head.is_none()
            || self.entity(entity).home == Some(scope)
        {
            let ty = self
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("function index without type"))?;
            let head = self.entity(entity).context().template_head;
            let signature_key = self.function_signature(scope, key, ty, head)?;
            if self
                .function_signatures
                .get(&signature_key)
                .is_none_or(|&old| self.binding(old).entity == entity)
            {
                self.function_signatures.insert(signature_key, binding);
            }
        }
        Ok(())
    }
    pub(super) fn indexed_function(
        &mut self,
        scope: ScopeId,
        key: super::function_names::ValueKey,
        ty: TypeId,
        head: Option<super::templates::HeadId>,
    ) -> Result<Option<BindingId>, Error> {
        let signature = self.function_signature(scope, key, ty, head)?;
        Ok(self.function_signatures.get(&signature).copied())
    }
    fn function_signature(
        &mut self,
        scope: ScopeId,
        name: super::function_names::ValueKey,
        ty: TypeId,
        head: Option<super::templates::HeadId>,
    ) -> Result<FunctionSignature, Error> {
        let ty = if head.is_some()
            || name
                == super::function_names::ValueKey::Special(
                    super::function_names::SpecialName::Conversion,
                ) {
            self.types.declared_type(ty)?
        } else {
            self.types.declaration_signature(ty)?
        };
        Ok(FunctionSignature {
            scope,
            name,
            ty,
            shape: head.map(|head| self.template_shape(head)).transpose()?,
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
        self.names
            .invalidate(key, self.complete_lookup_class(scope));
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
                    self.special_values.insert((scope, name), replacement);
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
                        .is_none_or(|limit| self.binding_origins[id.0 as usize].before(limit))
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
        limit.is_none_or(|limit| self.binding_origins[id.0 as usize].before(limit))
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
                    .is_none_or(|limit| self.binding_origins[id.0 as usize].before(limit))
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
            .callable()
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
            self.work.limited_lookup();
            let visible = |id: BindingId| self.binding_origins[id.0 as usize].before(limit);
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
    pub(super) fn add_implicit_constructor(
        &mut self,
        class: EntityId,
        signature: TypeId,
    ) -> Result<EntityId, Error> {
        let home = self
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("implicit constructor class scope"))?;
        let function = self.add_entity(Some(signature), None, Keyword::Int)?;
        let mut entity = self.entity_mut(function);
        entity.home = Some(home);
        entity.defined = true;
        entity.inline = super::specifiers::InlinePreference::Inline;
        entity.callable_mut().definition_emission =
            super::function_demands::DefinitionEmission::Demanded;
        entity.callable_mut().member_function = Some(MemberKey {
            class,
            kind: MemberKind::Constructor,
            signature,
            shape: None,
        });
        entity.callable_mut().function = Some(FunctionProperties {
            effects: FunctionEffect::Default,
            no_inline: false,
            returns: FunctionReturn::MayReturn,
            internal_linkage: false,
            language: super::linkage::Language::Cpp,
            denotes: None,
            is_static: false,
            special: None,
            allocation: None,
        });
        Ok(function)
    }
    pub(super) fn bind_implicit_constructor(
        &mut self,
        function: EntityId,
    ) -> Result<BindingId, Error> {
        if let Some(&binding) = self.function_origins.get(&function) {
            return Ok(binding);
        }
        let entity = self.entity(function);
        let home = entity
            .home
            .ok_or(Error::Invariant("implicit constructor home"))?;
        let signature = entity
            .ty
            .ok_or(Error::Invariant("implicit constructor signature"))?;
        self.callable_binding(home, function, signature)
    }
    pub(super) fn add_implicit_copy_binding(
        &mut self,
        home: ScopeId,
        function: EntityId,
        signature: TypeId,
    ) -> Result<BindingId, Error> {
        self.entity_mut(function).home = Some(home);
        self.callable_binding(home, function, signature)
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
            .context()
            .scope
            .ok_or(Error::Invariant("assignment class scope"))?;
        let function = self.add_entity(Some(signature), None, Keyword::Int)?;
        let special = super::assignment::key()?
            .special()
            .ok_or(Error::Invariant("assignment key"))?;
        let mut entity = self.entity_mut(function);
        entity.defined = true;
        entity.inline = super::specifiers::InlinePreference::Inline;
        entity.callable_mut().definition_emission =
            super::function_demands::DefinitionEmission::Demanded;
        entity.callable_mut().assignment_kind = Some(kind);
        entity.callable_mut().assignment_origin = super::assignment::Origin::Implicit;
        entity.availability = if properties.deleted {
            FunctionAvailability::Deleted
        } else {
            FunctionAvailability::Callable
        };
        entity.callable_mut().exceptions =
            super::exceptions::ExceptionState::Value(if properties.nothrow {
                super::exceptions::ExceptionSet::None
            } else {
                super::exceptions::ExceptionSet::All
            });
        entity.callable_mut().assignment = Some(properties);
        entity.callable_mut().function = Some(FunctionProperties {
            effects: super::graph::FunctionEffect::Default,
            no_inline: false,
            returns: super::graph::FunctionReturn::MayReturn,
            internal_linkage: false,
            language: super::linkage::Language::Cpp,
            denotes: None,
            is_static: false,
            special: Some(special),
            allocation: None,
        });
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
    ) -> Result<EntityId, Error> {
        let scope = self
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("implicit destructor without class scope"))?;
        let function = self.add_entity(Some(signature), None, Keyword::Int)?;
        let mut entity = self.entity_mut(function);
        entity.home = Some(scope);
        entity.special_mut().destructor_specification =
            super::exceptions::DestructorSpecification::Implicit;
        entity.defined = true;
        entity.inline = super::specifiers::InlinePreference::Inline;
        entity.callable_mut().special_definition =
            Some(super::special_members::SpecialDefinition::Defaulted {
                first: true,
                body: None,
            });
        entity.callable_mut().definition_emission =
            super::function_demands::DefinitionEmission::Demanded;
        entity.callable_mut().exceptions =
            super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
        entity.callable_mut().member_function = Some(MemberKey {
            class,
            kind: MemberKind::Destructor,
            signature,
            shape: None,
        });
        entity.callable_mut().function = Some(FunctionProperties {
            effects: super::graph::FunctionEffect::Default,
            no_inline: false,
            returns: super::graph::FunctionReturn::MayReturn,
            language: super::linkage::Language::Cpp,
            denotes: None,
            internal_linkage: false,
            is_static: false,
            special: None,
            allocation: None,
        });
        Ok(function)
    }
    pub(super) fn bind_implicit_destructor(
        &mut self,
        class: EntityId,
        function: EntityId,
    ) -> Result<BindingId, Error> {
        if let Some(binding) = self.entity(class).class().implicit_destructor {
            return if self.binding(binding).entity == function {
                Ok(binding)
            } else {
                Err(Error::Invariant("implicit destructor declaration changed"))
            };
        }
        let scope = self
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("implicit destructor without class scope"))?;
        let signature = self
            .entity(function)
            .ty
            .ok_or(Error::Invariant("implicit destructor without type"))?;
        let binding = self.callable_binding(scope, function, signature)?;
        self.entity_mut(class).class_mut().implicit_destructor = Some(binding);
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
            .class()
            .declared_conversions
            .iter()
            .position(|&id| self.binding(id).entity == entity);
        let mut record = self.entity_mut(class);
        let conversions = &mut record.class_mut().declared_conversions;
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
            .context()
            .scope
            .ok_or(Error::Invariant("conversion class without scope"))?;
        let mut index = 0;
        while let Some(&binding) = self
            .entity(key.class)
            .class()
            .declared_conversions
            .get(index)
        {
            let entity = self.binding(binding).entity;
            let old = self.entity(entity);
            if old.home != Some(scope)
                && old
                    .callable()
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
                    .class_mut()
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
    pub(super) fn has_member_import(&self, binding: BindingId) -> bool {
        self.member_imports.contains(&self.binding(binding).entity)
    }
    // Analyzer::member_declaration, using the existing declaration and entity
    // indexes instead of scanning a class or copying its overload set.
    pub(super) fn own_member_declaration(
        &self,
        scope: ScopeId,
        binding: BindingId,
    ) -> Option<BindingId> {
        let binding = self.binding(binding);
        if let Some(entry) = binding
            .name
            .and_then(|name| self.scope(scope).index.get(&name))
        {
            for id in [entry.value, entry.ty, entry.tag].into_iter().flatten() {
                if self.binding(id).entity == binding.entity {
                    return Some(id);
                }
            }
        }
        let key = binding
            .name
            .map(super::function_names::ValueKey::Identifier)
            .or_else(|| {
                self.entity(binding.entity)
                    .callable()
                    .function?
                    .special
                    .map(super::function_names::ValueKey::Special)
            })?;
        let position = self
            .function_positions
            .get(&(scope, key, binding.entity))?
            .visible?;
        self.visible_function_sets
            .get(&(scope, key))?
            .get(position)
            .copied()
    }
    pub(super) fn binding_limit(&self) -> usize {
        self.declaration_count
    }
    fn next_declaration_ordinal(&mut self) -> Result<usize, Error> {
        let ordinal = self.declaration_count;
        self.declaration_count = ordinal
            .checked_add(1)
            .ok_or(Error::Limit("semantic declaration limit"))?;
        Ok(ordinal)
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
    pub(super) fn record_member_function(
        &mut self,
        key: MemberKey,
        binding: BindingId,
    ) -> Result<(), Error> {
        self.member_functions.insert(key, binding);
        self.entity_mut(self.binding(binding).entity)
            .callable_mut()
            .member_function = Some(key);
        if key.kind == MemberKind::Constructor {
            let entity = self.binding(binding).entity;
            let earlier = self
                .entity(key.class)
                .class()
                .declared_constructors
                .iter()
                .position(|&id| self.binding(id).entity == entity);
            let mut record = self.entity_mut(key.class);
            let constructors = &mut record.class_mut().declared_constructors;
            if let Some(index) = earlier {
                constructors[index] = binding;
            } else {
                constructors.push(binding);
            }
        } else if key.kind == MemberKind::Conversion {
            let entity = self.binding(binding).entity;
            let earlier = self
                .entity(key.class)
                .class()
                .declared_conversions
                .iter()
                .position(|&id| self.binding(id).entity == entity);
            let mut record = self.entity_mut(key.class);
            let conversions = &mut record.class_mut().declared_conversions;
            if let Some(index) = earlier {
                conversions[index] = binding;
            } else {
                conversions.push(binding);
            }
        }
        let name = match key.kind {
            MemberKind::Constructor => super::function_names::SpecialName::Constructor,
            MemberKind::Destructor => super::function_names::SpecialName::Destructor,
            MemberKind::Conversion => super::function_names::SpecialName::Conversion,
        };
        let home = self
            .entity(key.class)
            .context()
            .scope
            .ok_or(Error::Invariant("member function class without scope"))?;
        self.record_special_value(home, name, binding)
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
    pub(super) fn add_edge(
        &mut self,
        scope: ScopeId,
        target: ScopeId,
        inline: bool,
    ) -> Result<(), Error> {
        let edge = ScopeEdge {
            target,
            introduced: if inline {
                self.declaration_count
                    .checked_sub(1)
                    .ok_or(Error::Invariant("inline membership before its directive"))?
            } else {
                self.next_declaration_ordinal()?
            },
        };
        let scope = self.scope_mut(scope);
        if inline {
            scope.inline_members.push(edge);
        } else {
            scope.directives.push(edge);
        }
        Ok(())
    }
}
