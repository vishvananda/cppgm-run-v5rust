// Translated from sema/entity_records.h and function_records.h. See NOTICE.
//! Compact entity headers and unit-owned, lazily populated role records.
use super::graph::{
    BindingId, ClassRepresentation, DataMemberKind, Entity, EntityId, FunctionProperties, Graph,
    MemberKey, ScopeId, VariableProperties,
};
use std::{
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Deref, DerefMut},
};

struct Slot<T> {
    value: NonZeroU32,
    marker: PhantomData<fn() -> T>,
}
impl<T> Copy for Slot<T> {}
impl<T> Clone for Slot<T> {
    fn clone(&self) -> Self {
        *self
    }
}
struct Pool<T> {
    entries: Vec<T>,
    absent: T,
    next: NonZeroU32,
}
impl<T: Default> Default for Pool<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            absent: T::default(),
            next: NonZeroU32::MIN,
        }
    }
}
impl<T: Default> Pool<T> {
    #[inline]
    fn get(&self, slot: Option<Slot<T>>) -> &T {
        slot.map_or(&self.absent, |slot| {
            &self.entries[(slot.value.get() - 1) as usize]
        })
    }
    #[inline]
    fn get_mut(&mut self, slot: &mut Option<Slot<T>>) -> &mut T {
        let id = if let Some(id) = *slot {
            id
        } else {
            let id = self.insert();
            *slot = Some(id);
            id
        };
        &mut self.entries[(id.value.get() - 1) as usize]
    }
    #[cold]
    #[inline(never)]
    fn insert(&mut self) -> Slot<T> {
        let value = self.next;
        // add_entity admits at most u32::MAX entities, each with at most
        // one slot in this pool. After the last legal slot saturates the
        // counter, no further insertion can be requested.
        self.next = self.next.saturating_add(1);
        self.entries.push(T::default());
        Slot {
            value,
            marker: PhantomData,
        }
    }
    #[cfg(feature = "telemetry")]
    fn storage_bytes(&self) -> usize {
        self.entries
            .capacity()
            .saturating_mul(std::mem::size_of::<T>())
    }
}

#[derive(Default)]
pub(super) struct ContextFacts {
    pub scope: Option<ScopeId>,
    pub template_parameter: Option<super::templates::ParameterInfo>,
    pub template_head: Option<super::templates::HeadId>,
}

#[derive(Default)]
pub(super) struct EnumerationFacts {
    pub enumeration: Option<super::enumerations::EnumInfo>,
}

pub(super) struct ClassFacts {
    pub representation: ClassRepresentation,
    pub implicit_default: Option<ScopeId>,
    pub user_constructor: Option<BindingId>,
    pub declared_constructors: Vec<BindingId>,
    pub declared_destructor: Option<BindingId>,
    pub implicit_destructor: Option<BindingId>,
    pub declared_conversions: Vec<BindingId>,
    pub members: Vec<EntityId>,
    pub polymorphism: super::virtual_members::Polymorphism,
    pub literal: super::literal_types::LiteralState,
    pub aggregation: super::aggregate_initialization::Aggregation,
    pub emptiness: Option<bool>,
    pub bases: Vec<super::inheritance::BaseClass>,
    pub base_dependency: crate::syntax::ast::BaseDependency,
    pub derivation: super::inheritance::Derivation,
    pub layout: super::layout::LayoutState,
    pub class_packing: u32,
    pub layout_category: super::layout::LayoutCategory,
    pub anonymous_class: Option<super::anonymous_types::AnonymousClass>,
    pub local_type_ordinal: Option<u32>,
    pub anonymous_storage: Option<BindingId>,
    pub initialized_members: usize,
}
impl Default for ClassFacts {
    fn default() -> Self {
        Self {
            representation: ClassRepresentation::Program,
            implicit_default: None,
            user_constructor: None,
            declared_constructors: Vec::new(),
            declared_destructor: None,
            implicit_destructor: None,
            declared_conversions: Vec::new(),
            members: Vec::new(),
            polymorphism: super::virtual_members::Polymorphism::Static,
            literal: super::literal_types::LiteralState::NotStarted,
            aggregation: super::aggregate_initialization::Aggregation::Eligible,
            emptiness: None,
            bases: Vec::new(),
            base_dependency: crate::syntax::ast::BaseDependency::Fixed,
            derivation: super::inheritance::Derivation::Allowed,
            layout: super::layout::LayoutState::NotStarted,
            class_packing: 0,
            layout_category: super::layout::LayoutCategory::Pod,
            anonymous_class: None,
            local_type_ordinal: None,
            anonymous_storage: None,
            initialized_members: 0,
        }
    }
}

pub(super) struct SpecialFacts {
    pub destruction: Option<super::destruction::Destruction>,
    pub destruction_preparation: super::destruction::Preparation,
    pub destructor_specification: super::exceptions::DestructorSpecification,
    pub implicit_copies: Option<super::copy_initialization::ImplicitCopies>,
    pub copy_preparation: super::copy_properties::Preparation,
    pub copy_properties: Option<super::copy_properties::CopyPropertiesId>,
    pub default_construction: super::construction::ConstructorState,
}
impl Default for SpecialFacts {
    fn default() -> Self {
        Self {
            destruction: None,
            destruction_preparation: super::destruction::Preparation::Pending,
            destructor_specification: super::exceptions::DestructorSpecification::Explicit,
            implicit_copies: None,
            copy_preparation: super::copy_properties::Preparation::default(),
            copy_properties: None,
            default_construction: super::construction::ConstructorState::NotStarted,
        }
    }
}

pub(super) struct CallableFacts {
    pub defaults: Vec<Option<super::default_arguments::DefaultArgument>>,
    pub special_definition: Option<super::special_members::SpecialDefinition>,
    pub exceptions: super::exceptions::ExceptionState,
    pub exception_origin: super::exceptions::SpecificationOrigin,
    pub exception_omission: super::exceptions::OmittedSpecification,
    pub target_unwind: super::exceptions::TargetUnwind,
    pub explicitness: super::specifiers::Explicitness,
    pub copy_origin: super::copy_construction::Origin,
    pub copy_kind: Option<super::types::RefKind>,
    pub assignment_kind: Option<super::types::RefKind>,
    pub assignment: Option<super::assignment::Properties>,
    pub assignment_origin: super::assignment::Origin,
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
    pub constructor_exceptions: super::construction::ConstructorExceptionSource,
}
impl Default for CallableFacts {
    fn default() -> Self {
        Self {
            defaults: Vec::new(),
            special_definition: None,
            exceptions: super::exceptions::ExceptionState::Absent,
            exception_origin: super::exceptions::SpecificationOrigin::Omitted,
            exception_omission: super::exceptions::OmittedSpecification::Compare,
            target_unwind: super::exceptions::TargetUnwind::MayThrow,
            explicitness: super::specifiers::Explicitness::Converting,
            copy_origin: super::copy_construction::Origin::Declared,
            copy_kind: None,
            assignment_kind: None,
            assignment: None,
            assignment_origin: super::assignment::Origin::Declared,
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
            constructor_exceptions: super::construction::ConstructorExceptionSource::Declared,
        }
    }
}

pub(super) struct ObjectFacts {
    pub constant: super::constants::ConstantState,
    pub data_member: Option<DataMemberKind>,
    pub array_member: super::aggregate_initialization::ArrayMemberKind,
    pub member_offset: Option<u64>,
    pub bit_field: Option<super::bit_fields::BitFieldId>,
    pub variable: Option<VariableProperties>,
    pub variable_definition: Option<crate::syntax::ast::DeclaratorId>,
}
impl Default for ObjectFacts {
    fn default() -> Self {
        Self {
            constant: super::constants::ConstantState::None,
            data_member: None,
            array_member: super::aggregate_initialization::ArrayMemberKind::Ordinary,
            member_offset: None,
            bit_field: None,
            variable: None,
            variable_definition: None,
        }
    }
}

#[derive(Default)]
pub(super) struct InitializationFacts {
    pub constructor: Option<super::construction::ConstructorId>,
    pub constructor_arguments: Option<super::constructor_initialization::ConstructorArguments>,
    pub member_initializer: Option<crate::syntax::ast::Initializer>,
    pub reference_temporary: Option<super::static_initialization::ReferenceTemporary>,
    pub static_initializer: Option<super::static_initialization::Value>,
}

#[derive(Default)]
pub(super) struct AttributesFacts {
    pub storage_attributes: super::layout_attributes::StorageAttributes,
}

#[derive(Default)]
pub(super) struct Roles {
    context: Option<Slot<ContextFacts>>,
    enumerated: Option<Slot<EnumerationFacts>>,
    class: Option<Slot<ClassFacts>>,
    special: Option<Slot<SpecialFacts>>,
    callable: Option<Slot<CallableFacts>>,
    object: Option<Slot<ObjectFacts>>,
    initialization: Option<Slot<InitializationFacts>>,
    attributes: Option<Slot<AttributesFacts>>,
}
#[derive(Default)]
pub(super) struct Records {
    context: Pool<ContextFacts>,
    enumerated: Pool<EnumerationFacts>,
    class: Pool<ClassFacts>,
    special: Pool<SpecialFacts>,
    callable: Pool<CallableFacts>,
    object: Pool<ObjectFacts>,
    initialization: Pool<InitializationFacts>,
    attributes: Pool<AttributesFacts>,
}
// A view holds identity, not precomputed references to every role. Header-only
// reads check only the header store; role reads check only their own stores.
#[derive(Clone, Copy)]
pub(super) struct EntityRef<'a> {
    graph: &'a Graph,
    id: EntityId,
}
impl<'a> EntityRef<'a> {
    #[inline]
    pub(super) fn new(graph: &'a Graph, id: EntityId) -> Self {
        Self { graph, id }
    }
    #[inline]
    pub(super) fn context(&self) -> &'a ContextFacts {
        self.graph.entity_context(self.id)
    }
    #[inline]
    pub(super) fn enumerated(&self) -> &'a EnumerationFacts {
        self.graph.entity_enumerated(self.id)
    }
    #[inline]
    pub(super) fn class(&self) -> &'a ClassFacts {
        self.graph.entity_class(self.id)
    }
    #[inline]
    pub(super) fn special(&self) -> &'a SpecialFacts {
        self.graph.entity_special(self.id)
    }
    #[inline]
    pub(super) fn callable(&self) -> &'a CallableFacts {
        self.graph.entity_callable(self.id)
    }
    #[inline]
    pub(super) fn object(&self) -> &'a ObjectFacts {
        self.graph.entity_object(self.id)
    }
    #[inline]
    pub(super) fn initialization(&self) -> &'a InitializationFacts {
        self.graph.entity_initialization(self.id)
    }
    #[inline]
    pub(super) fn attributes(&self) -> &'a AttributesFacts {
        self.graph.entity_attributes(self.id)
    }
}
pub(super) struct EntityMut<'a> {
    graph: &'a mut Graph,
    id: EntityId,
}
impl<'a> EntityMut<'a> {
    #[inline]
    pub(super) fn new(graph: &'a mut Graph, id: EntityId) -> Self {
        Self { graph, id }
    }
    #[inline]
    pub(super) fn context_mut(&mut self) -> &mut ContextFacts {
        self.graph.entity_context_mut(self.id)
    }
    #[inline]
    pub(super) fn enumerated_mut(&mut self) -> &mut EnumerationFacts {
        self.graph.entity_enumerated_mut(self.id)
    }
    #[inline]
    pub(super) fn class_mut(&mut self) -> &mut ClassFacts {
        self.graph.entity_class_mut(self.id)
    }
    #[inline]
    pub(super) fn special_mut(&mut self) -> &mut SpecialFacts {
        self.graph.entity_special_mut(self.id)
    }
    #[inline]
    pub(super) fn callable_mut(&mut self) -> &mut CallableFacts {
        self.graph.entity_callable_mut(self.id)
    }
    #[inline]
    pub(super) fn object_mut(&mut self) -> &mut ObjectFacts {
        self.graph.entity_object_mut(self.id)
    }
    #[inline]
    pub(super) fn initialization_mut(&mut self) -> &mut InitializationFacts {
        self.graph.entity_initialization_mut(self.id)
    }
    #[inline]
    pub(super) fn attributes_mut(&mut self) -> &mut AttributesFacts {
        self.graph.entity_attributes_mut(self.id)
    }
}
impl Deref for EntityRef<'_> {
    type Target = Entity;
    #[inline]
    fn deref(&self) -> &Entity {
        self.graph.entity_header(self.id)
    }
}
impl Deref for EntityMut<'_> {
    type Target = Entity;
    #[inline]
    fn deref(&self) -> &Entity {
        self.graph.entity_header(self.id)
    }
}
impl DerefMut for EntityMut<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Entity {
        self.graph.entity_header_mut(self.id)
    }
}
impl Records {
    #[inline]
    pub(super) fn context(&self, roles: &Roles) -> &ContextFacts {
        self.context.get(roles.context)
    }
    #[inline]
    pub(super) fn context_mut(&mut self, roles: &mut Roles) -> &mut ContextFacts {
        self.context.get_mut(&mut roles.context)
    }
    #[inline]
    pub(super) fn enumerated(&self, roles: &Roles) -> &EnumerationFacts {
        self.enumerated.get(roles.enumerated)
    }
    #[inline]
    pub(super) fn enumerated_mut(&mut self, roles: &mut Roles) -> &mut EnumerationFacts {
        self.enumerated.get_mut(&mut roles.enumerated)
    }
    #[inline]
    pub(super) fn class(&self, roles: &Roles) -> &ClassFacts {
        self.class.get(roles.class)
    }
    #[inline]
    pub(super) fn class_mut(&mut self, roles: &mut Roles) -> &mut ClassFacts {
        self.class.get_mut(&mut roles.class)
    }
    #[inline]
    pub(super) fn special(&self, roles: &Roles) -> &SpecialFacts {
        self.special.get(roles.special)
    }
    #[inline]
    pub(super) fn special_mut(&mut self, roles: &mut Roles) -> &mut SpecialFacts {
        self.special.get_mut(&mut roles.special)
    }
    #[inline]
    pub(super) fn callable(&self, roles: &Roles) -> &CallableFacts {
        self.callable.get(roles.callable)
    }
    #[inline]
    pub(super) fn callable_mut(&mut self, roles: &mut Roles) -> &mut CallableFacts {
        self.callable.get_mut(&mut roles.callable)
    }
    #[inline]
    pub(super) fn object(&self, roles: &Roles) -> &ObjectFacts {
        self.object.get(roles.object)
    }
    #[inline]
    pub(super) fn object_mut(&mut self, roles: &mut Roles) -> &mut ObjectFacts {
        self.object.get_mut(&mut roles.object)
    }
    #[inline]
    pub(super) fn initialization(&self, roles: &Roles) -> &InitializationFacts {
        self.initialization.get(roles.initialization)
    }
    #[inline]
    pub(super) fn initialization_mut(&mut self, roles: &mut Roles) -> &mut InitializationFacts {
        self.initialization.get_mut(&mut roles.initialization)
    }
    #[inline]
    pub(super) fn attributes(&self, roles: &Roles) -> &AttributesFacts {
        self.attributes.get(roles.attributes)
    }
    #[inline]
    pub(super) fn attributes_mut(&mut self, roles: &mut Roles) -> &mut AttributesFacts {
        self.attributes.get_mut(&mut roles.attributes)
    }
    #[cfg(feature = "telemetry")]
    pub(super) fn storage_bytes(&self) -> usize {
        self.context
            .storage_bytes()
            .saturating_add(self.enumerated.storage_bytes())
            .saturating_add(self.class.storage_bytes())
            .saturating_add(self.special.storage_bytes())
            .saturating_add(self.callable.storage_bytes())
            .saturating_add(self.object.storage_bytes())
            .saturating_add(self.initialization.storage_bytes())
            .saturating_add(self.attributes.storage_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::vocabulary::Keyword;

    #[test]
    fn headers_are_compact_and_reading_absent_roles_does_not_allocate() {
        assert_eq!(std::mem::size_of::<Entity>(), 32);
        assert_eq!(std::mem::size_of::<Roles>(), 32);
        let mut records = Records::default();
        let mut roles = Roles::default();
        assert!(records.context(&roles).scope.is_none());
        assert!(records.enumerated(&roles).enumeration.is_none());
        assert!(records.class(&roles).members.is_empty());
        assert!(records.callable(&roles).defaults.is_empty());
        assert!(matches!(
            records.object(&roles).constant,
            super::super::constants::ConstantState::None
        ));
        assert!(records.initialization(&roles).static_initializer.is_none());
        assert!(records.context.entries.is_empty());
        assert!(records.enumerated.entries.is_empty());
        assert!(records.class.entries.is_empty());
        assert!(records.special.entries.is_empty());
        assert!(records.callable.entries.is_empty());
        assert!(records.object.entries.is_empty());
        assert!(records.initialization.entries.is_empty());
        assert!(records.attributes.entries.is_empty());
        records.class_mut(&mut roles).class_packing = 16;
        assert_eq!(records.class(&roles).class_packing, 16);
        assert_eq!(records.class.entries.len(), 1);
        assert!(records.callable.entries.is_empty());
        assert!(records.object.entries.is_empty());
    }

    #[test]
    fn role_ids_survive_growth_and_default_records_are_never_mutated() {
        let mut records = Records::default();
        let mut slots = Vec::new();
        for index in 0..512_u32 {
            let mut roles = Roles::default();
            records.class_mut(&mut roles).class_packing = index;
            if index % 2 == 0 {
                records.callable_mut(&mut roles).defaults.push(None);
            }
            slots.push(roles);
        }
        for (index, roles) in (0_u32..).zip(&slots) {
            assert_eq!(records.class(roles).class_packing, index);
            assert_eq!(
                records.callable(roles).defaults.len(),
                usize::from(index % 2 == 0)
            );
            assert!(records.object(roles).variable.is_none());
        }
        assert_eq!(records.class.entries.len(), 512);
        assert_eq!(records.callable.entries.len(), 256);
        assert!(records.object.entries.is_empty());
        let roles = Roles::default();
        assert_eq!(records.class(&roles).class_packing, 0);
        assert!(records.callable(&roles).defaults.is_empty());
    }

    #[test]
    fn graph_views_read_current_header_and_role_facts() -> Result<(), super::super::Error> {
        let mut graph = Graph::new();
        let id = graph.add_entity(None, None, Keyword::Struct)?;
        assert!(graph.entity(id).class().members.is_empty());
        assert!(!graph.entity(id).defined);
        {
            let mut view = graph.entity_mut(id);
            view.defined = true;
            view.class_mut().class_packing = 32;
            view.callable_mut().defaults.push(None);
        }
        let view = graph.entity(id);
        assert!(view.defined);
        assert_eq!(view.class().class_packing, 32);
        assert_eq!(view.callable().defaults.len(), 1);
        assert!(view.context().scope.is_none());
        Ok(())
    }
}
