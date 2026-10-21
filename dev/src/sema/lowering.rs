// Translated from sema/semantic_unit.h's lowering facts. See NOTICE.
//! Read-only views of analyzed facts; the syntax and semantic stores remain their owners.
pub(crate) use super::anonymous_types::{ClassSpelling, UnnamedNumbering};
pub(crate) use super::assignment::Action as AssignmentAction;
pub(crate) use super::base_projections::{BaseProjection, BaseRoute, BaseRoutes, VcallSignature};
pub(crate) use super::builtin_calls::BuiltinFunction;
pub(crate) use super::constexpr_values::TemporaryId;
pub(crate) use super::constructor_initialization::ConstructorArguments;
pub(crate) use super::copy_construction::{CopyTarget, SubobjectCopy};
pub(crate) use super::copy_initialization::CopyAction;
pub(crate) use super::dynamic_casts::RuntimeCast;
pub(crate) use super::expression_facts::ClassInitialization;
pub(crate) use super::expression_facts::ExpressionOrigin;
pub(crate) use super::expression_keys::{KeyNode, KeyWord};
pub(crate) use super::floating_intrinsics::FloatingIntrinsic;
pub(crate) use super::literal_calls::LiteralCall;
pub(crate) use super::static_arrays::Item as StaticItem;
pub(crate) use super::static_initialization::Value as StaticValue;
pub(crate) use super::symbolic_members::MemberPointer as MemberPointerValue;
pub(crate) use super::virtual_members::{SlotId, SlotKind};
pub(crate) use super::vtable_groups::SegmentId;
use super::{
    Error,
    graph::{BindingKind, Graph},
    types::TypeKind,
};
pub(crate) use super::{
    graph::{EntityId, ScopeId},
    types::{Scalar, TypeId},
};
use crate::lex::vocabulary::Keyword;
use crate::{lex::identifiers::NameId, syntax::ast};
pub(crate) struct TemplateParameterView {
    pub index: u32,
    pub level: u32,
    pub value: bool,
    pub pack: bool,
    pub template: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct TemplateIdView<'a> {
    pub name: TypeId,
    pub arguments: &'a [TypeId],
    pub pack: Option<usize>,
}

pub(crate) enum TemplateDeclaration {
    Type,
    Value(TypeId),
    Template,
}

pub(crate) struct ConstructorView<'a> {
    pub object_pointer: TypeId,
    pub union: bool,
    pub zero_before_value: bool,
    pub members: &'a [(EntityId, Option<ast::Initializer>)],
    pub bases: &'a [super::construction::BaseInitialization],
    pub indirect_bases: &'a [super::construction::BaseInitialization],
    pub class: EntityId,
    pub subobjects: &'a [super::construction::MemberConstruction],
    pub delegation: Option<super::construction::Delegation>,
    pub copy: Option<&'a [SubobjectCopy]>,
}
pub(crate) use super::allocation::ArrayAllocation;
pub(crate) struct AllocationView {
    pub function: EntityId,
    pub target: TypeId,
    pub constructor: Option<(ConstructorId, ConstructorArguments)>,
    pub deallocator: Option<EntityId>,
    pub reserved: bool,
    pub array: Option<ArrayAllocation>,
    pub sized_array_delete: bool,
}
pub(crate) struct DeletionView {
    pub function: EntityId,
    pub target: TypeId,
    pub incomplete: bool,
    pub sized_array_delete: bool,
}
pub(crate) struct VariableFacts {
    pub name: Option<NameId>,
    pub home: ScopeId,
    pub ty: TypeId,
    pub internal: bool,
    pub locality: Locality,
    pub c_linkage: bool,
    pub thread_local: bool,
    pub initializer: Option<StaticValue>,
    pub in_class_initializer: Option<ast::Initializer>,
    pub reference_temporary: Option<super::static_initialization::ReferenceTemporary>,
    pub definition: Option<ast::DeclaratorId>,
    pub emission: Emission,
}
impl VariableFacts {
    pub(crate) fn named(&self) -> Result<NameId, Error> {
        self.name
            .ok_or(Error::Invariant("named variable without name"))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Locality {
    External,
    Unit,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeclaredLinkage {
    External,
    Internal,
}

pub(crate) enum TypeView<'a> {
    Address {
        ty: TypeId,
        entity: EntityId,
    },
    Value {
        ty: TypeId,
        bits: u64,
    },
    Scalar(Scalar),
    Pointer(TypeId),
    Array {
        element: TypeId,
        bound: u64,
    },
    Reference {
        element: TypeId,
        rvalue: bool,
    },
    Enum(TypeId),
    Class(EntityId),
    Function {
        result: TypeId,
        parameters: &'a [TypeId],
        variadic: bool,
    },
    Unsupported,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Emission {
    Always,
    Demanded,
}
#[derive(Clone, Copy)]
pub(crate) enum CallTarget {
    Direct(EntityId),
    Indirect(TypeId, bool),
}
#[derive(Clone, Copy)]
pub(crate) struct MemberCall {
    pub function: EntityId,
    pub result: TypeId,
    pub place: bool,
    pub offset: Option<BaseProjection>,
}
pub(crate) use super::construction::ConstructorId;
#[derive(Clone, Copy)]
pub(crate) enum FunctionName {
    Identifier(NameId),
    Constructor(ClassSpelling),
    Destructor(Option<ClassSpelling>),
    Operator {
        token: crate::lex::vocabulary::SimpleToken,
        array: bool,
    },
    Literal(NameId),
    Conversion(TypeId),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AllocationRole {
    Allocate,
    Free,
}
#[derive(Clone, Copy)]
pub(crate) struct Allocation {
    pub role: AllocationRole,
    pub array: bool,
}
pub(crate) struct FunctionFacts {
    pub template_instance: Option<EntityId>,
    pub name: FunctionName,
    pub label: Option<ast::Name>,
    pub home: ScopeId,
    pub ty: TypeId,
    pub scope: Option<ScopeId>,
    pub internal: bool,
    pub declared_linkage: DeclaredLinkage,
    pub c_linkage: bool,
    pub emission: Emission,
    pub nothrow: bool,
    pub allocation: Option<Allocation>,
    pub builtin: Option<BuiltinFunction>,
    pub definition: Option<ast::DeclId>,
    pub object: Option<EntityId>,
    pub qualifiers: crate::abi::graph::Qualifiers,
}
#[derive(Clone, Copy)]
pub(crate) struct CopyForm {
    pub class: EntityId,
    pub moving: bool,
    pub assignment: bool,
    pub locality: Locality,
}
#[derive(Clone, Copy)]
pub(crate) struct BitField {
    pub operation: TypeId,
    pub signed: bool,
    pub width: u64,
    pub bit: u64,
    pub offset: u64,
    pub bytes: u8,
    pub split: u8,
    pub shared: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct InitializerDemands {
    pub function: Option<EntityId>,
    pub constructor: Option<ConstructorId>,
}
#[derive(Clone, Copy)]
pub(crate) struct Expression {
    pub origin: super::expression_facts::ExpressionOrigin,
    pub ty: TypeId,
    pub target: Option<TypeId>,
    pub folded: Option<u64>,
    pub query: Option<u64>,
    pub query_kind: super::expression_facts::QueryKind,
    pub is_place: bool,
    pub glvalue: GlvalueKind,
    pub null_pointer: bool,
    pub commuted: bool,
    pub temporary: Option<TypeId>,
    pub reference_cast: Option<TypeId>,
}
impl Expression {
    pub(crate) fn is_bit_field(self) -> bool {
        matches!(self.glvalue, GlvalueKind::BitField)
    }
}
pub(crate) use super::expression_facts::{GlvalueKind, ObjectSource};
impl Graph {
    pub(crate) fn lower_condition_function(
        &self,
        id: ast::DeclaratorId,
    ) -> Result<Option<(MemberCall, TypeId)>, Error> {
        self.condition_function(id)
            .map(|step| {
                Ok((
                    MemberCall {
                        function: self.binding(step.binding).entity,
                        result: step.result,
                        place: step.category != super::expression_facts::ValueCategory::Prvalue,
                        offset: step
                            .path
                            .map(|path| self.lower_path_projection(path))
                            .transpose()?,
                    },
                    step.after.result,
                ))
            })
            .transpose()
    }
    pub(crate) fn lower_builtin_operand_type(&self, id: ast::ExprId) -> Result<TypeId, Error> {
        Ok(self
            .expression_facts
            .function_conversion(id)
            .filter(|step| step.usage == super::conversion_functions::ConversionUse::Builtin)
            .map_or(self.expression_facts.get(id)?.ty, |step| step.result))
    }
    pub(crate) fn lower_builtin_operand(&self, id: ast::ExprId) -> bool {
        self.expression_facts
            .function_conversion(id)
            .is_some_and(|step| step.usage == super::conversion_functions::ConversionUse::Builtin)
    }
    pub(crate) fn lower_function_conversion(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<(MemberCall, TypeId)>, Error> {
        self.expression_facts
            .function_conversion(id)
            .map(|step| {
                Ok((
                    MemberCall {
                        function: self.binding(step.binding).entity,
                        result: step.result,
                        place: step.category != super::expression_facts::ValueCategory::Prvalue,
                        offset: step
                            .path
                            .map(|path| self.lower_path_projection(path))
                            .transpose()?,
                    },
                    step.after.result,
                ))
            })
            .transpose()
    }
    pub(crate) fn lower_elides_conversion(
        &self,
        id: ast::ExprId,
        constructor: ConstructorId,
    ) -> bool {
        let Some(step) = self.expression_facts.function_conversion(id) else {
            return false;
        };
        let plan = self.constructors.get(constructor);
        plan.binding.is_some_and(|binding| {
            self.entity(self.binding(binding).entity)
                .copy_kind
                .is_some()
        }) && self.entity(self.binding(step.binding).entity).explicitness
            != super::specifiers::Explicitness::Explicit
            && step.category == super::expression_facts::ValueCategory::Prvalue
            && self.class_of(step.result) == Some(plan.class)
    }
    pub(crate) fn lower_conversion_copy(&self, id: ast::ExprId) -> Option<CopyAction> {
        self.expression_facts
            .function_conversion(id)
            .and_then(|step| step.copy.map(|copy| copy.action))
    }
    pub(crate) fn lower_arrow_count(&self, base: ast::ExprId) -> usize {
        self.expression_facts.arrow_calls(base).len()
    }
    pub(crate) fn lower_arrow_call(
        &self,
        base: ast::ExprId,
        index: usize,
    ) -> Result<Option<MemberCall>, Error> {
        self.expression_facts
            .arrow_calls(base)
            .get(index)
            .map(|step| {
                Ok(MemberCall {
                    function: self.binding(step.binding).entity,
                    result: step.result,
                    place: step.category != super::expression_facts::ValueCategory::Prvalue,
                    offset: step
                        .path
                        .map(|path| self.lower_path_projection(path))
                        .transpose()?,
                })
            })
            .transpose()
    }
    pub(crate) fn lower_allocation(&self, id: ast::ExprId) -> Result<AllocationView, Error> {
        let use_ = self.expression_facts.allocation(id)?;
        Ok(AllocationView {
            function: self.binding(use_.function).entity,
            target: use_.target,
            constructor: use_.constructor,
            deallocator: use_.deallocator.map(|binding| self.binding(binding).entity),
            reserved: use_.reserved,
            array: use_.array,
            sized_array_delete: use_.sized_array_delete,
        })
    }
    pub(crate) fn lower_deletion(&self, id: ast::ExprId) -> Result<DeletionView, Error> {
        let use_ = self.expression_facts.deletion(id)?;
        Ok(DeletionView {
            function: self.binding(use_.function).entity,
            target: use_.target,
            incomplete: use_.incomplete,
            sized_array_delete: use_.sized_array_delete,
        })
    }
    pub(crate) fn lower_base_initialization_span(
        &self,
        class: EntityId,
        index: usize,
        base_entry: bool,
    ) -> Result<(u64, bool, u64), Error> {
        let edge = &self.entity(class).bases[index];
        let offset = edge
            .offset
            .ok_or(Error::Invariant("base construction without layout"))?;
        let super::layout::LayoutState::Value {
            size,
            non_virtual_size,
            empty,
            ..
        } = self.entity(edge.class).layout
        else {
            return Err(Error::Invariant("base construction without size"));
        };
        let super::layout::LayoutState::Value {
            non_virtual_size: limit,
            ..
        } = self.entity(class).layout
        else {
            return Err(Error::Invariant("constructor owner without size"));
        };
        let begin = offset.wrapping_add(non_virtual_size);
        let end = offset.wrapping_add(size);
        let mut complete_limit = u64::MAX;
        for &(base, offset) in self.lower_virtual_bases(class) {
            if !self.lower_class_empty(base)? {
                complete_limit = complete_limit.min(offset);
            }
        }
        let mut occupied = !self.lower_virtual_bases(edge.class).is_empty()
            || !edge.is_virtual && end > if base_entry { limit } else { complete_limit };
        // Earlier initialized bases can occupy a later base's tail padding.
        // Source order and layout order need not agree (primary/empty bases).
        for prior in &self.entity(class).bases[..index] {
            let from = prior
                .offset
                .ok_or(Error::Invariant("prior base without layout"))?;
            let to = from.wrapping_add(self.lower_nonvirtual_bytes(prior.class)?);
            occupied |= from < end && to > begin;
        }
        Ok((
            offset,
            empty,
            if occupied { non_virtual_size } else { size },
        ))
    }
    pub(crate) fn lower_indirect_base_layout(
        &self,
        class: EntityId,
        base: EntityId,
    ) -> Result<(u64, bool, u64), Error> {
        let offset = self
            .lower_virtual_base_offset(class, base)
            .ok_or(Error::Invariant("indirect constructor base without layout"))?;
        let super::layout::LayoutState::Value { size, empty, .. } = self.entity(base).layout else {
            return Err(Error::Invariant("indirect constructor base without size"));
        };
        Ok((offset, empty, size))
    }
    pub(crate) fn lower_nonvirtual_bytes(&self, class: EntityId) -> Result<u64, Error> {
        if let super::layout::LayoutState::Value {
            non_virtual_size, ..
        } = self.entity(class).layout
        {
            Ok(non_virtual_size)
        } else {
            Err(Error::Invariant("subobject span without completed layout"))
        }
    }
    pub(crate) fn lower_implicit_copy(&self, entity: EntityId) -> bool {
        self.entity(entity).copy_origin == super::copy_construction::Origin::Implicit
    }
    pub(crate) fn lower_user_provided_constructor(&self, id: ConstructorId) -> bool {
        self.constructors.get(id).binding.is_some_and(|binding| {
            self.entity(self.binding(binding).entity)
                .special_definition
                .is_some_and(|definition| {
                    matches!(
                        definition,
                        super::special_members::SpecialDefinition::Body(_)
                    )
                })
        })
    }
    pub(crate) fn lower_virtual_destructor(&self, class: EntityId) -> bool {
        self.virtuals.destructor(class).is_some()
    }
    pub(crate) fn lower_constructor_trivial(&self, id: ConstructorId) -> bool {
        !self.constructors.get(id).nontrivial()
    }
    pub(crate) fn lower_constructor(&self, id: ConstructorId) -> ConstructorView<'_> {
        let plan = self.constructors.get(id);
        ConstructorView {
            object_pointer: plan.object_pointer,
            class: plan.class,
            zero_before_value: plan.binding.is_none_or(|binding| {
                let function = self.binding(binding).entity;
                self.inherited_constructors.source(function).is_some()
                    || matches!(
                        self.entity(function).special_definition,
                        Some(super::special_members::SpecialDefinition::Defaulted {
                            first: true,
                            ..
                        })
                    )
            }),
            union: self.entity(plan.class).key == crate::lex::vocabulary::Keyword::Union,
            members: &plan.members,
            bases: &plan.bases,
            indirect_bases: &plan.indirect_bases,
            subobjects: &plan.subobjects,
            delegation: plan.delegation,
            copy: plan.copy.as_deref(),
        }
    }
    pub(crate) fn lower_constructor_arguments(
        &self,
        entity: EntityId,
    ) -> Option<ConstructorArguments> {
        self.entity(entity).constructor_arguments
    }
    pub(crate) fn lower_construction(&self, entity: EntityId) -> Option<ConstructorId> {
        self.entity(entity).constructor
    }
    pub(crate) fn lower_constructor_entity(&self, id: ConstructorId) -> Option<EntityId> {
        self.constructors
            .get(id)
            .binding
            .map(|binding| self.binding(binding).entity)
    }
    // An argument's odr-use schedules definitions only. A declaration is emitted
    // when generated code actually refers to the function (C++ demand_entry).
    pub(crate) fn lower_argument_functions(
        &self,
        context: Option<EntityId>,
    ) -> impl DoubleEndedIterator<Item = EntityId> + '_ {
        self.constant_addresses
            .argument_functions(context.map_or(
                super::constant_addresses::ArgumentContext::Unit,
                super::constant_addresses::ArgumentContext::Function,
            ))
            .iter()
            .rev()
            .copied()
            .filter(|&entity| self.lower_function_defined(self.lower_function_entity(entity)))
    }
    pub(crate) fn lower_initializer_argument_functions(
        &self,
        entity: EntityId,
    ) -> impl Iterator<Item = EntityId> + '_ {
        self.constant_addresses
            .argument_functions(super::constant_addresses::ArgumentContext::Initializer(
                entity,
            ))
            .iter()
            .copied()
            .filter(|&entity| self.lower_function_defined(self.lower_function_entity(entity)))
    }
    pub(crate) fn lower_root_delegations(&self) -> impl Iterator<Item = ConstructorId> + '_ {
        self.root_demands.as_slice().iter().filter_map(|demand| {
            let id = match *demand {
                super::function_demands::Demand::Constructor(id) => id,
                super::function_demands::Demand::Function { binding, .. } => {
                    self.entity(self.binding(binding).entity).constructor?
                }
            };
            self.constructors.get(id).delegation.map(|_| id)
        })
    }
    pub(crate) fn lower_constructor_function(
        &self,
        id: ConstructorId,
    ) -> Result<FunctionFacts, Error> {
        let plan = self.constructors.get(id);
        if let Some(entity) = self.lower_constructor_entity(id) {
            return self.lower_function(entity);
        }
        let class = self.entity(plan.class);
        Ok(FunctionFacts {
            template_instance: None,
            name: FunctionName::Constructor(self.lower_class_spelling(plan.class)?),
            label: None,
            home: class
                .scope
                .ok_or(Error::Invariant("constructor without class scope"))?,
            ty: plan.callable_type,
            scope: None,
            internal: self.lower_entity_unit_local(plan.class)?
                && !self.lower_shared_local(plan.class)?,
            declared_linkage: DeclaredLinkage::External,
            c_linkage: false,
            emission: Emission::Demanded,
            nothrow: plan.nothrow,
            allocation: None,
            builtin: None,
            definition: None,
            object: Some(plan.class),
            qualifiers: crate::abi::graph::Qualifiers::default(),
        })
    }
    pub(crate) fn lower_conversion_overloads(
        &self,
        class: EntityId,
    ) -> impl Iterator<Item = EntityId> + '_ {
        self.entity(class)
            .declared_conversions
            .iter()
            .map(|&binding| self.binding(binding).entity)
            .filter(move |&entity| self.entity(entity).home == self.entity(class).scope)
    }
    pub(crate) fn lower_constructor_overloads(
        &self,
        class: EntityId,
    ) -> impl Iterator<Item = EntityId> + '_ {
        self.entity(class)
            .declared_constructors
            .iter()
            .map(|&binding| self.binding(binding).entity)
            .filter(move |&entity| self.entity(entity).home == self.entity(class).scope)
    }
    pub(crate) fn lower_function_defined(&self, entity: EntityId) -> bool {
        self.entity(entity).defined
            && (!self.function_templates.specializations.contains(entity)
                || self.entity(entity).function_definition.is_some())
    }
    pub(crate) fn lower_explicit_functions(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.function_templates
            .instantiations
            .definitions()
            .iter()
            .map(|definition| self.binding(definition.binding).entity)
            .filter(|&entity| {
                self.entity(entity).availability != super::graph::FunctionAvailability::Deleted
                    || matches!(
                        self.entity(entity).special_definition,
                        Some(super::special_members::SpecialDefinition::Defaulted {
                            first: true,
                            ..
                        })
                    )
            })
    }
    pub(crate) fn lower_explicit_classes(&self) -> &[EntityId] {
        self.function_templates.instantiations.classes()
    }
    pub(crate) fn lower_explicit_function_root(&self, entity: EntityId) -> bool {
        self.entity(entity).defined
            && self.function_templates.instantiations.defined(entity)
            && (self.entity(entity).inline != super::specifiers::InlinePreference::Inline
                || !self.function_templates.specializations.contains(entity)
                    && (self.function_templates.bodies.member(entity)
                        || self.function_templates.primary(entity).is_some()))
    }
    pub(crate) fn lower_explicit_variable_root(&self, entity: EntityId) -> bool {
        self.variable_instances.explicit_definition(entity)
    }
    pub(crate) fn lower_has_synthesized_body(&self, id: EntityId) -> bool {
        self.lower_implicit_destructor(id)
            || self.entity(id).assignment.is_some()
            || self.entity(id).member_function.is_some_and(|member| {
                member.kind == super::graph::MemberKind::Destructor
                    && matches!(
                        self.entity(id).special_definition,
                        Some(super::special_members::SpecialDefinition::Defaulted { .. })
                    )
            })
    }
    pub(crate) fn lower_template_operand(
        &self,
        ty: TypeId,
    ) -> Option<(ast::ExprId, &[super::expression_keys::KeyWord])> {
        let TypeKind::TemplateParameter { entity, .. } = self.types.get(ty).kind else {
            return None;
        };
        let pattern = self.template_operands.pattern(entity)?;
        // An unkeyed pending operand still is an expression placeholder.
        // Its empty key selects the frozen ABI_NONE fallback; it must not
        // masquerade as an ordinary named type.
        Some((
            pattern.expression,
            self.template_operands.words(entity).unwrap_or_default(),
        ))
    }
    pub(crate) fn lower_dependent_member(&self, ty: TypeId) -> Option<(TypeId, NameId)> {
        let TypeKind::TemplateParameter { entity, .. } = self.types.get(ty).kind else {
            return None;
        };
        if let Some(name) = self.class_templates.dependent_names.get(entity) {
            return Some((name.qualifier, name.name));
        }
        let member = self.dependent_members.get(entity)?;
        Some((member.qualifier, member.name))
    }
    pub(crate) fn lower_member_pointer_type(&self, ty: TypeId) -> Option<(TypeId, TypeId)> {
        if let TypeKind::MemberPointer { owner, element } = self.types.get(ty).kind {
            Some((owner, element))
        } else {
            None
        }
    }
    pub(crate) fn lower_function_type_qualifiers(
        &self,
        ty: TypeId,
    ) -> crate::abi::graph::Qualifiers {
        let mut qualifiers = crate::abi::graph::Qualifiers::default();
        if let TypeKind::Function { cv, reference, .. } = self.types.get(ty).kind {
            use crate::abi::graph::Qualifiers as Q;
            if cv.0 & 1 != 0 {
                qualifiers = qualifiers.union(Q::CONST);
            }
            if cv.0 & 2 != 0 {
                qualifiers = qualifiers.union(Q::VOLATILE);
            }
            if let Some(reference) = reference {
                qualifiers = qualifiers.union(if reference == super::types::RefKind::Lvalue {
                    Q::LVALUE
                } else {
                    Q::RVALUE
                });
            }
        }
        qualifiers
    }
    pub(crate) fn lower_type_parameter_name(&self, ty: TypeId) -> Option<NameId> {
        if let TypeKind::TemplateParameter { entity, .. } = self.types.get(ty).kind {
            self.entity(entity).name
        } else {
            None
        }
    }
    pub(crate) fn lower_operand_key_dependent(&self, ty: TypeId) -> Option<bool> {
        let TypeKind::TemplateParameter { entity, .. } = self.types.get(ty).kind else {
            return None;
        };
        let pattern = self.template_operands.pattern(entity)?;
        Some(pattern.key_dependent && self.template_operands.words(entity).is_some())
    }
    pub(crate) fn lower_array_operand(&self, ty: TypeId) -> Option<TypeId> {
        let TypeKind::Array { operand, .. } = self.types.get(ty).kind else {
            return None;
        };
        operand
    }
    pub(crate) fn lower_dependent_type(&self, ty: TypeId) -> bool {
        self.dependent_type(ty)
    }
    pub(crate) fn lower_aliased_type(&self, ty: TypeId) -> Option<TypeId> {
        let TypeKind::Class(entity) = self.types.get(ty).kind else {
            return None;
        };
        self.class_templates.aliases.aliased(entity)
    }
    pub(crate) fn lower_templated_class(&self, class: EntityId) -> bool {
        self.entity(class).template_head.is_some()
    }
    pub(crate) fn lower_operand_function(&self, entity: EntityId) -> bool {
        self.entity(entity).function.is_some()
    }
    pub(crate) fn lower_operand_type_name(&self, entity: EntityId) -> Option<TypeId> {
        let node = self.entity(entity);
        matches!(
            node.key,
            Keyword::Typedef | Keyword::Class | Keyword::Struct | Keyword::Union
        )
        .then_some(node.ty)
        .or_else(|| node.enumeration.is_some().then_some(node.ty))
        .flatten()
    }
    pub(crate) fn lower_operand_constant(&self, entity: EntityId) -> Option<(TypeId, u64)> {
        let node = self.entity(entity);
        let super::constants::ConstantState::Value(value) = node.constant else {
            return None;
        };
        Some((node.ty.unwrap_or(value.ty), value.bits))
    }
    pub(crate) fn lower_template_name(&self, ty: TypeId) -> Option<(ScopeId, NameId)> {
        let TypeKind::Template(entity) = self.types.get(ty).kind else {
            return None;
        };
        let entity = self.entity(entity);
        entity.home.zip(entity.name)
    }
    pub(crate) fn lower_template_id_name(&self, ty: TypeId) -> Option<TemplateIdView<'_>> {
        let TypeKind::Class(entity) = self.types.get(ty).kind else {
            return None;
        };
        let instance = self.class_templates.instance(entity)?;
        let name = self
            .class_templates
            .template_entities
            .name(instance.primary)?;
        Some(self.lower_named_template_id(name, &instance.arguments))
    }
    pub(crate) fn lower_named_template_id<'a>(
        &self,
        name: TypeId,
        arguments: &'a [TypeId],
    ) -> TemplateIdView<'a> {
        // C++ name_template_id: a positional template-name's own head
        // partitions its written arguments. An expansion before the pack
        // makes that partition unknown; an expansion in the pack does not.
        let pack = if let TypeKind::TemplateParameter {
            entity,
            template: true,
        } = self.types.get(name).kind
            && let Some(head) = self
                .entity(entity)
                .template_parameter
                .and_then(|info| info.inner)
        {
            self.templates
                .parameters(head)
                .iter()
                .position(|&parameter| {
                    self.entity(parameter)
                        .template_parameter
                        .is_some_and(super::templates::ParameterInfo::is_pack)
                })
                .filter(|&position| {
                    position <= arguments.len()
                        && arguments[..position].iter().all(|&argument| {
                            !matches!(self.types.get(argument).kind, TypeKind::Expansion(_))
                        })
                })
        } else {
            None
        };
        TemplateIdView {
            name,
            arguments,
            pack,
        }
    }
    pub(crate) fn lower_template_parameter(&self, ty: TypeId) -> Option<u32> {
        let TypeKind::TemplateParameter { entity, .. } = self.types.get(ty).kind else {
            return None;
        };
        let info = self.entity(entity).template_parameter?;
        Some(info.abi_index())
    }
    pub(crate) fn lower_value_argument(&self, ty: TypeId) -> bool {
        let TypeKind::TemplateParameter { entity, .. } = self.types.get(ty).kind else {
            return false;
        };
        self.entity(entity)
            .template_parameter
            .is_some_and(|info| info.value_type.is_some())
            || self
                .template_operands
                .pattern(entity)
                .is_some_and(|pattern| pattern.value)
    }
    pub(crate) fn lower_parameter_level(&self, ty: TypeId) -> Option<TemplateParameterView> {
        // The frozen analyzer represents auto by an artificial type parameter
        // with zero-initialized position and depth. Its ABI spelling is T_.
        if matches!(self.types.get(ty).kind, TypeKind::Auto) {
            return Some(TemplateParameterView {
                index: 0,
                level: 0,
                value: false,
                pack: false,
                template: false,
            });
        }
        let (entity, level) = match self.types.get(ty).kind {
            TypeKind::TemplateParameter { entity, .. } => (entity, None),
            TypeKind::BoundPack {
                parameter, depth, ..
            } => (parameter, Some(depth)),
            _ => return None,
        };
        let info = self.entity(entity).template_parameter?;
        Some(TemplateParameterView {
            index: info.abi_index(),
            level: level.unwrap_or_else(|| info.depth()),
            value: info.value_type.is_some(),
            pack: info.is_pack(),
            template: info.inner.is_some(),
        })
    }
    pub(crate) fn lower_bound_parameter(&self, ty: TypeId) -> bool {
        matches!(self.types.get(ty).kind, TypeKind::BoundPack { .. })
    }
    pub(crate) fn lower_names_bound_pack(&self, root: TypeId) -> bool {
        let mut work = vec![root];
        let mut seen = std::collections::HashSet::new();
        while let Some(ty) = work.pop() {
            if !seen.insert(ty) {
                continue;
            }
            match &self.types.get(ty).kind {
                TypeKind::BoundPack { .. } => return true,
                TypeKind::TemplateParameter { entity, .. } => {
                    if let Some(member) = self.dependent_members.get(*entity) {
                        work.push(member.qualifier);
                    }
                }
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.class_templates.instance(*entity)
                        && instance.dependent
                    {
                        work.extend(&instance.arguments);
                    }
                }
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Array { element, .. } => work.push(*element),
                TypeKind::MemberPointer { owner, element } => work.extend([*owner, *element]),
                TypeKind::Pack(elements) => work.extend(elements),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters);
                }
                _ => (),
            }
        }
        false
    }
    pub(crate) fn lower_template_parameter_entity(
        &self,
        instance: EntityId,
        index: usize,
    ) -> Option<EntityId> {
        let primary = self.function_templates.primary(instance)?;
        let head = self.entity(self.binding(primary).entity).template_head?;
        self.templates.parameters(head).get(index).copied()
    }
    pub(crate) fn lower_template_declaration(
        &self,
        parameter: EntityId,
    ) -> Result<(TemplateDeclaration, bool, u32), Error> {
        let info = self
            .entity(parameter)
            .template_parameter
            .ok_or(Error::Invariant(
                "ABI declaration without template parameter",
            ))?;
        let declaration = if info.inner.is_some() {
            TemplateDeclaration::Template
        } else if let Some(ty) = info.value_type {
            TemplateDeclaration::Value(ty)
        } else {
            TemplateDeclaration::Type
        };
        Ok((declaration, info.is_pack(), info.depth()))
    }
    pub(crate) fn lower_argument_template(&self, ty: TypeId) -> Option<EntityId> {
        match self.types.get(ty).kind {
            TypeKind::Template(entity) => Some(entity),
            TypeKind::TemplateParameter { entity, .. }
                if self.entity(entity).template_parameter?.inner.is_some() =>
            {
                Some(entity)
            }
            _ => None,
        }
    }
    pub(crate) fn lower_template_head_parameters(&self, entity: EntityId) -> Option<&[EntityId]> {
        let entity = self.entity(entity);
        let head = entity
            .template_parameter
            .and_then(|info| info.inner)
            .or(entity.template_head)?;
        Some(self.templates.parameters(head))
    }
    pub(crate) fn lower_instance_levels(&self, scope: ScopeId) -> Result<u32, Error> {
        self.instance_levels(Some(scope))
    }
    pub(crate) fn lower_template_signature(
        &self,
        entity: EntityId,
    ) -> Result<(TypeId, &[TypeId]), Error> {
        let primary = self
            .function_templates
            .primary(entity)
            .ok_or(Error::Invariant("template ABI without primary"))?;
        let ty = self
            .binding(primary)
            .ty
            .ok_or(Error::Invariant("template ABI without signature"))?;
        let arguments = self
            .function_templates
            .arguments
            .get(&entity)
            .ok_or(Error::Invariant("template ABI without arguments"))?;
        Ok((ty, arguments))
    }
    pub(crate) fn lower_inline_hint(&self, entity: EntityId) -> bool {
        // Ordinary demand-only entries include implicitly inline synthesized
        // members. A template specialization also has weak emission when its
        // primary has no inline specifier, so it retains that separate fact.
        if self.function_templates.primary(entity).is_some()
            || self.function_templates.bodies.member(entity)
        {
            self.entity(entity).inline == super::specifiers::InlinePreference::Inline
        } else {
            self.entity(entity).definition_emission
                == super::function_demands::DefinitionEmission::Demanded
                || self.entity(entity).inline == super::specifiers::InlinePreference::Inline
        }
    }
    pub(crate) fn lower_function_body(&self, id: EntityId) -> Option<ast::BodyId> {
        if let Some(body) = self.function_templates.bodies.body(id) {
            return Some(body);
        }
        match self.entity(id).special_definition {
            Some(super::special_members::SpecialDefinition::Body(body)) => Some(body),
            _ => self.entity(id).analyzed_body,
        }
    }

    pub(crate) fn lower_receiver_class(&self, id: ast::ExprId) -> Result<Option<EntityId>, Error> {
        Ok(self
            .expression_facts
            .get(id)?
            .object
            .and_then(|object| self.class_of(object.ty)))
    }
    pub(crate) fn lower_object_source(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<ObjectSource>, Error> {
        if let Some(receiver) = self.expression_facts.callable_receiver(id) {
            return Ok(Some(receiver.object.source));
        }
        Ok(self
            .expression_facts
            .get(id)?
            .object
            .map(|object| object.source))
    }
    pub(crate) fn lower_callable_object_call(&self, id: ast::ExprId) -> Result<bool, Error> {
        Ok(self.expression_facts.get(id)?.call_form
            == super::expression_facts::CallForm::CallableObject)
    }
    pub(super) fn lower_path_offset(
        &self,
        path: super::inheritance::BasePathId,
    ) -> Result<u64, Error> {
        self.base_paths
            .get(path)
            .iter()
            .try_fold(0u64, |sum, &(class, index)| {
                let edge = &self.entity(class).bases[index];
                // This API denotes a fixed adjustment. A virtual edge needs
                // either a proven complete-object projection or a vbase row;
                // its offset in this intermediate class is neither of those.
                if edge.is_virtual {
                    return Err(Error::Missing("virtual base projection"));
                }
                edge.offset
                    .map(|offset| sum.wrapping_add(offset))
                    .ok_or(Error::Invariant("base projection without completed layout"))
            })
    }
    pub(crate) fn lower_member_projections(
        &self,
        id: ast::ExprId,
    ) -> Result<[Option<BaseProjection>; 2], Error> {
        let facts = self.expression_facts.get(id)?;
        Ok([
            facts
                .member_naming_path
                .map(|path| self.lower_path_projection(path))
                .transpose()?,
            facts
                .member_path
                .map(|path| self.lower_path_projection(path))
                .transpose()?,
        ])
    }
    pub(crate) fn lower_base_conversion(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<BaseProjection>, Error> {
        if let Some(step) = self.expression_facts.function_conversion(id) {
            return step
                .after_path
                .map(|path| self.lower_path_projection(path))
                .transpose();
        }
        let facts = self.expression_facts.get(id)?;
        if !facts
            .conversion
            .into_iter()
            .chain(facts.class_copy.map(|copy| copy.binding))
            .any(|conversion| {
                conversion.second == super::expression_facts::ValueConversion::DerivedToBase
            })
        {
            return Ok(None);
        }
        let path = facts
            .base_path
            .ok_or(Error::Invariant("base conversion without retained path"))?;
        self.lower_path_projection(path).map(Some)
    }
    pub(crate) fn lower_conditional_projections(
        &self,
        id: ast::ExprId,
    ) -> Result<[Option<BaseProjection>; 2], Error> {
        let [left, right] = self.expression_facts.conditional_paths(id);
        Ok([
            left.map(|path| self.lower_path_projection(path))
                .transpose()?,
            right
                .map(|path| self.lower_path_projection(path))
                .transpose()?,
        ])
    }
    pub(crate) fn lower_receiver_projections(
        &self,
        id: ast::ExprId,
    ) -> Result<[Option<BaseProjection>; 2], Error> {
        if let Some(receiver) = self.expression_facts.callable_receiver(id) {
            return Ok([
                receiver
                    .naming_path
                    .or(receiver.path)
                    .map(|path| self.lower_path_projection(path))
                    .transpose()?,
                receiver
                    .member_path
                    .map(|path| self.lower_path_projection(path))
                    .transpose()?,
            ]);
        }
        let offsets = self.lower_member_projections(id)?;
        if offsets[0].is_some() {
            Ok(offsets)
        } else {
            Ok([self.lower_base_conversion(id)?, None])
        }
    }
    pub(crate) fn lower_cast_base_adjustment(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<(BaseProjection, bool)>, Error> {
        let facts = self.expression_facts.get(id)?;
        let Some(cast) = facts.base_cast else {
            return Ok(None);
        };
        let offset = self.lower_path_projection(cast.path)?;
        Ok(Some((
            if cast.downcast {
                offset.negated()?
            } else {
                offset
            },
            cast.project_zero,
        )))
    }
    pub(crate) fn lower_designated_function(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<EntityId>, Error> {
        Ok(self
            .expression_facts
            .get(id)?
            .function
            .map(|binding| self.binding(binding).entity))
    }
    pub(super) fn lower_entity_unit_local(&self, entity: EntityId) -> Result<bool, Error> {
        self.entity(entity).ty.map_or_else(
            || Ok(self.lower_entity_scope_unit_local(entity)),
            |ty| self.lower_type_unit_local(ty),
        )
    }
    pub(crate) fn lower_type_unit_local(&self, ty: TypeId) -> Result<bool, Error> {
        self.lower_type_entity_locality(
            ty,
            Self::lower_entity_scope_unit_local,
            Self::address_argument_unit_local,
        )
    }
    pub(super) fn lower_type_entity_locality(
        &self,
        mut ty: TypeId,
        is_local: fn(&Self, EntityId) -> bool,
        address_is_local: fn(&Self, EntityId) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        // Scalar/object and linear pointer shells need no scratch allocation.
        // Function components and class-template arguments share a DAG worklist.
        loop {
            ty = self.types.canonical_view(ty);
            match self.types.get(ty).kind {
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Array { element, .. } => ty = element,
                TypeKind::Class(entity) | TypeKind::Enum(entity)
                    if self.class_templates.instance(entity).is_some()
                        || self.entity(entity).home.is_some_and(|home| {
                            self.scope(home).kind == super::graph::ScopeKind::Class
                        }) =>
                {
                    if is_local(self, entity) {
                        return Ok(true);
                    }
                    break;
                }
                TypeKind::Class(entity) | TypeKind::Enum(entity) => {
                    return Ok(is_local(self, entity));
                }
                TypeKind::Address { entity, .. } => {
                    return address_is_local(self, entity);
                }
                TypeKind::Function { .. } | TypeKind::MemberPointer { .. } | TypeKind::Pack(_) => {
                    break;
                }
                _ => return Ok(false),
            }
        }
        let mut work = vec![ty];
        let mut seen = std::collections::HashSet::new();
        while let Some(ty) = work.pop() {
            let ty = self.types.canonical_view(ty);
            if !seen.insert(ty) {
                continue;
            }
            match &self.types.get(ty).kind {
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Array { element, .. } => work.push(*element),
                TypeKind::MemberPointer { owner, element } => work.extend([*owner, *element]),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters);
                }
                TypeKind::Pack(elements) => work.extend(elements),
                TypeKind::Address { entity, .. } if address_is_local(self, *entity)? => {
                    return Ok(true);
                }
                TypeKind::Class(entity) | TypeKind::Enum(entity) if is_local(self, *entity) => {
                    return Ok(true);
                }
                TypeKind::Class(entity) | TypeKind::Enum(entity) => {
                    if let Some(instance) = self.class_templates.instance(*entity) {
                        work.extend(&instance.arguments);
                    }
                    if let Some(owner) = self.entity(*entity).home
                        && self.scope(owner).kind == super::graph::ScopeKind::Class
                        && let Some(class) = self.scope(owner).owner
                        && let Some(ty) = self.entity(class).ty
                    {
                        work.push(ty);
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }
    pub(super) fn lower_entity_scope_unit_local(&self, mut entity: EntityId) -> bool {
        use super::graph::ScopeKind;
        loop {
            let node = self.entity(entity);
            let Some(owner) = node.home else {
                return true;
            };
            let scope = self.scope(owner);
            if scope.kind == ScopeKind::Class
                && self.lower_closure(entity).is_some()
                && !self.closure_member_context(entity)
            {
                return true;
            }
            if scope.kind != ScopeKind::Class {
                return matches!(scope.kind, ScopeKind::Block | ScopeKind::Function)
                    || node.name.is_none()
                    || self.lower_anonymous_namespace(owner);
            }
            let Some(outer) = scope.owner else {
                return true;
            };
            entity = outer;
        }
    }
    pub(crate) fn lower_presentation_scope(
        &self,
        id: ScopeId,
    ) -> (Option<ScopeId>, Option<NameId>) {
        let scope = self.scope(id);
        match scope.kind {
            super::graph::ScopeKind::Namespace => (scope.parent, scope.name),
            super::graph::ScopeKind::Class => (
                scope.owner.and_then(|class| {
                    self.lower_closure(class)
                        .and_then(|closure| closure.enclosing)
                        .and_then(|function| self.entity(function).home)
                        .or(self.entity(class).home)
                }),
                scope.name,
            ),
            _ => (None, None),
        }
    }
    pub(crate) fn lower_trivial_default_construction(
        &self,
        entity: EntityId,
    ) -> Result<bool, Error> {
        let id = self.entity(entity).constructor.ok_or(Error::Invariant(
            "default class object without construction facts",
        ))?;
        Ok(!self.constructors.get(id).nontrivial())
    }
    pub(crate) fn lower_empty_initialization_step(&self, entity: EntityId) -> Result<bool, Error> {
        let node = self.entity(entity);
        if node.constructor.is_none() {
            return Ok(false);
        }
        let mut ty = self.lower_entity_type(entity)?;
        while let TypeView::Array { element, .. } = self.lower_type(ty)? {
            ty = element;
        }
        let TypeView::Class(class) = self.lower_type(ty)? else {
            return Ok(false);
        };
        let owner = self
            .entity(class)
            .home
            .ok_or(Error::Invariant("class without owner"))?;
        Ok(!self.lower_anonymous_namespace(owner) && !self.entity(class).members.is_empty())
    }
    /// The semantic alias is recorded once; lowering never repeats name lookup.
    pub(crate) fn lower_function_entity(&self, id: EntityId) -> EntityId {
        self.entity(id)
            .function
            .and_then(|function| function.denotes)
            .unwrap_or(id)
    }
    fn lower_linkage_scope(&self, mut scope: ScopeId) -> Result<ScopeId, Error> {
        // A member keeps its class. A block declaration with linkage belongs
        // to the nearest namespace, including within a member function.
        if self.scope(scope).kind == super::graph::ScopeKind::Class {
            return Ok(scope);
        }
        while self.scope(scope).kind != super::graph::ScopeKind::Namespace {
            scope = self
                .scope(scope)
                .parent
                .ok_or(Error::Invariant("local scope without parent"))?;
        }
        Ok(scope)
    }
    pub(crate) fn lower_constexpr(&self, id: EntityId) -> bool {
        self.entity(id).constexpr
    }
    pub(crate) fn lower_requested_alignment(&self, id: EntityId) -> u32 {
        self.entity(id).storage_attributes.alignment
    }
    pub(crate) fn lower_initializer_constructors(&self, entity: EntityId) -> &[ConstructorId] {
        self.initializer_constructors.get(entity)
    }
    pub(crate) fn lower_absent_initializer_constructor(&self) -> Option<ConstructorId> {
        self.initializer_constructors.absent_initializer()
    }
    pub(crate) fn lower_array_image_prefix(
        &self,
        id: EntityId,
    ) -> Option<&[super::static_arrays::Item]> {
        self.array_images.prefix(id)
    }
    pub(crate) fn lower_default_constructor(
        &self,
        class: EntityId,
    ) -> Result<ConstructorId, Error> {
        let super::construction::ConstructorState::Ready(id) =
            self.entity(class).default_construction
        else {
            return Err(Error::Invariant(
                "lowering default construction without plan",
            ));
        };
        Ok(id)
    }
    pub(crate) fn lower_ready_default_constructor(&self, class: EntityId) -> Option<ConstructorId> {
        match self.entity(class).default_construction {
            super::construction::ConstructorState::Ready(id) => Some(id),
            _ => None,
        }
    }
    pub(crate) fn lower_list_constructor(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<ConstructorId>, Error> {
        Ok(self.expression_facts.get(id)?.constructor)
    }
    pub(crate) fn lower_reference_reads_storage(&self, entity: EntityId) -> bool {
        self.constant_addresses.read_through_storage(entity)
    }
    pub(crate) fn lower_initializer_demands(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<InitializerDemands>, Error> {
        let Ok(facts) = self.expression_facts.get(id) else {
            return Ok(None);
        };
        let mut function = self
            .expression_facts
            .function_conversion(id)
            .map(|step| step.binding)
            .or(match facts.call {
                Some(super::function_resolution::CallTarget::Direct(binding)) => Some(binding),
                _ => None,
            })
            .map(|binding| self.binding(binding).entity);
        if let Some(id) = function {
            let entity = self.entity(id);
            let trivial_destructor = if let Some(member) = entity.member_function
                && member.kind == super::graph::MemberKind::Destructor
            {
                self.destructor_trivial(member.class)?
            } else {
                false
            };
            if trivial_destructor
                || entity
                    .assignment
                    .as_ref()
                    .is_some_and(|properties| properties.trivial)
                || entity
                    .copy_properties
                    .is_some_and(|id| self.copy_properties.get(id).trivial)
                || entity
                    .constructor
                    .is_some_and(|id| self.lower_constructor_trivial(id))
            {
                function = None;
            }
        }
        Ok(Some(InitializerDemands {
            function,
            constructor: self
                .expression_facts
                .class_conversion(id)
                .map(|(constructor, _)| constructor),
        }))
    }
    pub(crate) fn lower_constant_constructors(
        &self,
        id: ast::ExprId,
    ) -> Option<[Option<ConstructorId>; 2]> {
        let facts = self.expression_facts.get(id).ok()?;
        Some([
            facts.constructor,
            self.expression_facts.class_conversion(id).map(|(id, _)| id),
        ])
    }
    pub(crate) fn lower_class_conversion(
        &self,
        id: ast::ExprId,
    ) -> Option<(ConstructorId, ConstructorArguments)> {
        self.expression_facts.class_conversion(id)
    }
    pub(crate) fn lower_prvalue_initialization(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<ClassInitialization>, Error> {
        Ok(self.expression_facts.get(id)?.class_initialization)
    }
    pub(crate) fn lower_aggregate_class(&self, class: EntityId) -> bool {
        self.aggregate_class(class)
    }
    pub(crate) fn lower_array_image(&self, id: EntityId) -> Option<&[super::static_arrays::Item]> {
        self.array_images.get(id)
    }
    pub(crate) fn lower_variable(&self, id: EntityId) -> Result<VariableFacts, Error> {
        let entity = self.entity(id);
        let variable = entity
            .variable
            .ok_or(Error::Invariant("lowering non-variable"))?;
        Ok(VariableFacts {
            name: entity.name,
            home: self.lower_linkage_scope(
                entity
                    .home
                    .ok_or(Error::Invariant("variable without home"))?,
            )?,
            ty: entity.ty.ok_or(Error::Invariant("variable without type"))?,
            internal: variable.internal_linkage,
            locality: if variable.internal_linkage
                || entity
                    .home
                    .filter(|&home| self.scope(home).kind == super::graph::ScopeKind::Class)
                    .and_then(|home| self.scope(home).owner)
                    .map(|class| self.lower_entity_unit_local(class))
                    .transpose()?
                    .unwrap_or(false)
                || variable.language != super::linkage::Language::C
                    && self.lower_type_unit_local(
                        entity.ty.ok_or(Error::Invariant("variable without type"))?,
                    )? {
                Locality::Unit
            } else {
                Locality::External
            },
            c_linkage: variable.language == super::linkage::Language::C,
            thread_local: variable.thread_storage == super::specifiers::ThreadStorage::ThreadLocal,
            initializer: entity.static_initializer,
            in_class_initializer: entity.member_initializer,
            reference_temporary: entity.reference_temporary,
            definition: if self.variable_instances.instance(id)
                && self.class_instantiation_suppressed(id)
            {
                None
            } else {
                entity.variable_definition
            },
            emission: if self.variable_instances.instance(id)
                || entity.inline == super::specifiers::InlinePreference::Inline
            {
                Emission::Demanded
            } else {
                Emission::Always
            },
        })
    }
    pub(crate) fn lower_demanded_variables(&self) -> &[EntityId] {
        self.variable_instances.demands()
    }
    pub(crate) fn lower_variable_arguments(&self, entity: EntityId) -> Option<&[TypeId]> {
        self.variable_instances.templates.arguments(entity)
    }
    pub(crate) fn lower_omitted_variable_initialization(&self, entity: EntityId) -> bool {
        self.variable_instances.omitted_initialization(entity)
            // declare_variable does not default-initialize an unqualified
            // in-class declaration without an initializer. An inline ordinary
            // member still defines zero storage on use; a template instance
            // instead performs default initialization in instantiate_variable.
            || self.ordinary_inline_member(entity)
                && self.entity(entity).member_initializer.is_none()
    }
    pub(super) fn ordinary_inline_member(&self, entity: EntityId) -> bool {
        !self.variable_instances.instance(entity)
            && self.entity(entity).defined
            && self.entity(entity).inline == super::specifiers::InlinePreference::Inline
            && self.entity(entity).data_member == Some(super::graph::DataMemberKind::Static)
    }
    pub(crate) fn lower_demanded_initializer(
        &self,
        entity: EntityId,
    ) -> Option<super::variable_instances::Definition> {
        if self.class_instantiation_suppressed(entity) {
            return None;
        }
        self.variable_instances.initializer(entity)
    }
    pub(crate) fn lower_arithmetic_types(
        &self,
        id: ast::ExprId,
    ) -> Result<(TypeId, TypeId), Error> {
        self.arithmetic_types(id)
    }
    pub(crate) fn lower_case(&self, id: ast::StmtId) -> Result<u64, Error> {
        self.case_values
            .get(id.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("case without converted value"))
    }
    pub(crate) fn lower_condition_target(&self, id: ast::DeclaratorId) -> Result<TypeId, Error> {
        Ok(self.condition_conversion(id)?.result)
    }
    pub(crate) fn lower_program_storage(&self, id: EntityId) -> bool {
        self.lower_static_member(id)
            || self
                .entity(id)
                .variable
                .is_some_and(|variable| variable.is_extern)
            || self
                .entity(id)
                .home
                .is_some_and(|scope| self.scope(scope).kind == super::graph::ScopeKind::Namespace)
    }
    pub(crate) fn lower_automatic(&self, id: EntityId) -> bool {
        self.entity(id)
            .variable
            .is_some_and(|properties| properties.automatic)
    }
    pub(crate) fn lower_block_static(&self, id: EntityId) -> bool {
        let entity = self.entity(id);
        entity.variable.is_some_and(|variable| {
            variable.static_specifier == super::specifiers::StaticSpecifier::Present
        }) && entity.home.is_some_and(|home| {
            matches!(
                self.scope(home).kind,
                super::graph::ScopeKind::Block | super::graph::ScopeKind::Function
            )
        })
    }
    pub(crate) fn lower_result_local(
        &self,
        id: EntityId,
        function: EntityId,
        result: TypeId,
    ) -> bool {
        use super::graph::ScopeKind;
        let entity = self.entity(id);
        let Some(properties) = entity.variable else {
            return false;
        };
        let Some(ty) = entity.ty else {
            return false;
        };
        let Some(mut home) = entity.home else {
            return false;
        };
        if !properties.automatic
            || self.scope(home).kind != ScopeKind::Block
            || self.types.get(ty).cv.0 & 2 != 0
            || !matches!((&self.types.get(ty).kind, &self.types.get(result).kind),
                (TypeKind::Class(from), TypeKind::Class(to)) if from == to)
        {
            return false;
        }
        loop {
            if Some(home) == self.entity(function).scope {
                return true;
            }
            let Some(parent) = self.scope(home).parent else {
                return false;
            };
            home = parent;
        }
    }
    pub(crate) fn lower_explicit_static(&self, id: EntityId) -> bool {
        self.entity(id).variable.is_some_and(|variable| {
            variable.static_specifier == super::specifiers::StaticSpecifier::Present
        })
    }
    pub(crate) fn lower_is_variable(&self, id: EntityId) -> bool {
        self.entity(id).variable.is_some()
    }
    pub(crate) fn lower_local_name(&self, id: EntityId) -> Option<NameId> {
        self.entity(id).name
    }

    pub(crate) fn lower_declared(&self, id: ast::DeclaratorId) -> Result<EntityId, Error> {
        self.declared_bindings
            .get(id.index())
            .copied()
            .flatten()
            .map(|binding| self.binding(binding).entity)
            .ok_or(Error::Invariant("lowering declarator without binding"))
    }
    pub(crate) fn lower_function(&self, id: EntityId) -> Result<FunctionFacts, Error> {
        let id = self.lower_function_entity(id);
        let entity = self.entity(id);
        if self.function_templates.primary(id).is_some()
            && matches!(
                entity.exceptions,
                super::exceptions::ExceptionState::Template(_)
            )
        {
            return Err(Error::Invariant(
                "unresolved specialization exception at lowering boundary",
            ));
        }
        let function = entity
            .function
            .ok_or(Error::Invariant("lowering non-function"))?;
        let object = entity
            .home
            .filter(|&home| self.scope(home).kind == super::graph::ScopeKind::Class)
            .and_then(|home| self.scope(home).owner)
            .filter(|_| !function.is_static);
        let qualifiers = entity
            .ty
            .map_or(crate::abi::graph::Qualifiers::default(), |ty| {
                self.lower_function_type_qualifiers(ty)
            });
        Ok(FunctionFacts {
            template_instance: self.function_templates.primary(id).map(|_| id),
            object,
            qualifiers,
            name: match function.special {
                Some(super::function_names::SpecialName::Operator { token, array }) => {
                    FunctionName::Operator { token, array }
                }
                Some(super::function_names::SpecialName::Literal(name)) => {
                    FunctionName::Literal(name)
                }
                Some(
                    super::function_names::SpecialName::Destructor
                    | super::function_names::SpecialName::Conversion,
                ) => {
                    return Err(Error::Missing("member function names"));
                }
                None if entity
                    .member_function
                    .is_some_and(|key| key.kind == super::graph::MemberKind::Destructor) =>
                {
                    FunctionName::Destructor(
                        self.entity(object.ok_or(Error::Invariant("destructor without class"))?)
                            .destruction
                            .ok_or(Error::Invariant("destructor without completed facts"))?
                            .declared_name
                            .map(ClassSpelling::Name)
                            .or_else(|| {
                                self.lower_closure(object?)
                                    .map(|closure| ClassSpelling::Closure(closure.ordinal))
                            }),
                    )
                }
                None if entity
                    .member_function
                    .is_some_and(|key| key.kind == super::graph::MemberKind::Constructor) =>
                {
                    FunctionName::Constructor(self.lower_class_spelling(
                        object.ok_or(Error::Invariant("constructor without class"))?,
                    )?)
                }
                None if entity
                    .member_function
                    .is_some_and(|key| key.kind == super::graph::MemberKind::Conversion) =>
                {
                    let ty = entity
                        .ty
                        .ok_or(Error::Invariant("conversion without signature"))?;
                    let TypeKind::Function { result, .. } = self.types.get(ty).kind else {
                        return Err(Error::Invariant("conversion signature"));
                    };
                    FunctionName::Conversion(result)
                }
                None => FunctionName::Identifier(
                    entity
                        .name
                        .ok_or(Error::Missing("lowering special member names"))?,
                ),
            },
            label: entity.special_name,
            home: self.lower_linkage_scope(
                entity
                    .home
                    .ok_or(Error::Invariant("function without home"))?,
            )?,
            ty: entity.ty.ok_or(Error::Invariant("function without type"))?,
            scope: entity.scope,
            internal: self.lower_function_unit_local(id)?,
            declared_linkage: if function.internal_linkage {
                DeclaredLinkage::Internal
            } else {
                DeclaredLinkage::External
            },
            c_linkage: function.language == super::linkage::Language::C,
            emission: if entity.definition_emission
                == super::function_demands::DefinitionEmission::Demanded
                || entity.inline == super::specifiers::InlinePreference::Inline
            {
                Emission::Demanded
            } else {
                Emission::Always
            },
            nothrow: if entity.constructor_exceptions
                == super::construction::ConstructorExceptionSource::SubobjectPlan
            {
                entity
                    .constructor
                    .is_some_and(|id| self.constructors.get(id).nothrow)
            } else {
                matches!(
                    &entity.exceptions,
                    super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None)
                )
            },
            allocation: if function.allocation.is_some() {
                let Some(super::function_names::SpecialName::Operator { token, array }) =
                    function.special
                else {
                    return Err(Error::Invariant(
                        "allocation function without operator name",
                    ));
                };
                Some(Allocation {
                    role: if token.as_keyword() == Some(crate::lex::vocabulary::Keyword::New) {
                        AllocationRole::Allocate
                    } else {
                        AllocationRole::Free
                    },
                    array,
                })
            } else {
                None
            },
            definition: entity.function_definition,
            builtin: entity.builtin,
        })
    }
    pub(crate) fn lower_scope_class(&self, scope: ScopeId) -> Option<EntityId> {
        (self.scope(scope).kind == super::graph::ScopeKind::Class)
            .then_some(self.scope(scope).owner)
            .flatten()
    }
    pub(crate) fn lower_class_specialization(&self, class: EntityId) -> bool {
        self.class_templates.instance(class).is_some()
    }
    pub(crate) fn lower_member_class_specialization(&self, class: EntityId) -> bool {
        self.class_templates
            .instance(class)
            .and_then(|instance| self.entity(instance.primary).home)
            .is_some_and(|home| self.scope(home).kind == super::graph::ScopeKind::Class)
    }
    pub(crate) fn lower_dependent_template(&self, class: EntityId) -> Option<(TypeId, NameId)> {
        let primary = self.class_templates.instance(class)?.primary;
        let name = self.class_templates.dependent_names.get(primary)?;
        Some((name.qualifier, name.name))
    }
    pub(crate) fn lower_specialization_assignment(&self, id: ast::ExprId) -> Result<bool, Error> {
        let Some(member) = self.expression_facts.get(id)?.member else {
            return Ok(false);
        };
        let member = self.entity(self.binding(member).entity);
        Ok(
            member.data_member == Some(super::graph::DataMemberKind::Instance)
                && member
                    .home
                    .and_then(|scope| self.scope(scope).owner)
                    .is_some_and(|owner| self.lower_class_specialization(owner)),
        )
    }
    pub(crate) fn lower_abi_scope(
        &self,
        id: ScopeId,
    ) -> Result<(Option<ScopeId>, Option<NameId>, bool), Error> {
        let namespace = self.scope(id).kind == super::graph::ScopeKind::Namespace;
        if !namespace && self.scope(id).kind != super::graph::ScopeKind::Class {
            return Err(Error::Missing("lowering class and local scopes"));
        }
        let (parent, name) = self.lower_presentation_scope(id);
        Ok((parent, name, namespace))
    }
    pub(crate) fn lower_overloads(
        &self,
        id: EntityId,
    ) -> Result<impl Iterator<Item = EntityId> + '_, Error> {
        let entity = self.entity(id);
        let home = entity
            .home
            .ok_or(Error::Invariant("function without home"))?;
        let name = if let Some(special) = entity.function.and_then(|function| function.special) {
            super::function_names::ValueKey::Special(special)
        } else {
            entity
                .name
                .ok_or(Error::Invariant("function without name"))?
                .into()
        };
        Ok(self
            .visible_functions_at(home, name)
            .map(|binding| self.binding(binding).entity)
            .filter(move |&id| self.entity(id).home == Some(home)))
    }
    pub(crate) fn lower_parameters(
        &self,
        scope: Option<ScopeId>,
    ) -> impl Iterator<Item = (EntityId, Option<NameId>, TypeId)> + '_ {
        scope
            .into_iter()
            .flat_map(|id| self.scope(id).bindings.iter())
            .filter_map(|&id| {
                let binding = self.binding(id);
                (binding.kind == BindingKind::Parameter).then_some(binding)
            })
            .filter_map(|binding| binding.ty.map(|ty| (binding.entity, binding.name, ty)))
    }
    pub(crate) fn lower_argument_pack(&self, id: TypeId) -> Option<&[TypeId]> {
        match &self.types.get(id).kind {
            TypeKind::Pack(elements) => Some(elements),
            _ => None,
        }
    }
    pub(crate) fn lower_pack_expansion(&self, id: TypeId) -> Option<TypeId> {
        match self.types.get(id).kind {
            TypeKind::Expansion(pattern) => Some(pattern),
            _ => None,
        }
    }
    pub(crate) fn lower_type(&self, id: TypeId) -> Result<TypeView<'_>, Error> {
        Ok(match &self.types.get(self.types.canonical_view(id)).kind {
            TypeKind::Address { ty, entity } => TypeView::Address {
                ty: *ty,
                entity: *entity,
            },
            TypeKind::Value { ty, bits } => TypeView::Value {
                ty: *ty,
                bits: *bits,
            },
            TypeKind::Scalar(scalar) => TypeView::Scalar(*scalar),
            TypeKind::Pointer(element) => TypeView::Pointer(*element),
            TypeKind::Array { element, bound, .. } => TypeView::Array {
                element: *element,
                bound: *bound,
            },
            TypeKind::Reference { element, kind } => TypeView::Reference {
                element: *element,
                rvalue: *kind == super::types::RefKind::Rvalue,
            },
            TypeKind::Enum(entity) => TypeView::Enum(
                self.entity(*entity)
                    .enumeration
                    .and_then(|info| info.underlying)
                    .ok_or(Error::Invariant("enum without underlying type"))?,
            ),
            TypeKind::Function {
                result,
                parameters,
                variadic,
                ..
            } => TypeView::Function {
                result: *result,
                parameters,
                variadic: *variadic,
            },
            TypeKind::Class(entity) => TypeView::Class(*entity),
            _ => TypeView::Unsupported,
        })
    }
    pub(crate) fn lower_signed_integral(&self, ty: TypeId) -> bool {
        self.integral_kind(ty).is_some_and(|kind| kind.signed)
    }
    pub(crate) fn lower_cv(&self, id: TypeId) -> u8 {
        let mut id = id;
        while let TypeKind::Array { element, .. } = self.types.get(id).kind {
            id = element;
        }
        self.types.get(id).cv.0
    }
    pub(crate) fn lower_lvalue(&self, id: ast::ExprId) -> Result<bool, Error> {
        Ok(self.expression_facts.get(id)?.category
            == super::expression_facts::ValueCategory::Lvalue)
    }
    pub(crate) fn lower_reference_binding(&self, id: ast::ExprId) -> Result<bool, Error> {
        Ok(self
            .expression_facts
            .user_conversion(id)
            .or(self.expression_facts.get(id)?.conversion)
            .is_some_and(|conversion| {
                matches!(
                    self.types.get(conversion.target).kind,
                    TypeKind::Reference { .. }
                )
            }))
    }
    /// A scalar functional cast's brace list is syntax only. Class/array
    /// initialization lists instead own typed value and construction facts.
    pub(crate) fn lower_list_is_typed(&self, id: ast::ExprId) -> bool {
        self.expression_facts.known(id).is_some()
    }
    pub(crate) fn lower_expression(&self, id: ast::ExprId) -> Result<Expression, Error> {
        let facts = self.expression_facts.get(id)?;
        Ok(Expression {
            origin: facts.origin,
            ty: facts.ty,
            glvalue: facts.glvalue,
            target: facts.conversion.map(|c| c.result),
            folded: facts.folded,
            query: facts.query_value,
            query_kind: facts.query_kind,
            is_place: facts.category != super::expression_facts::ValueCategory::Prvalue,
            null_pointer: facts
                .conversion
                .is_some_and(|c| c.second == super::expression_facts::ValueConversion::NullPointer),
            commuted: facts.commuted,
            temporary: facts.conversion.filter(|c| c.temporary).map(|c| c.result),
            reference_cast: facts.reference_cast,
        })
    }
    pub(crate) fn lower_name(&self, name: ast::Name) -> Result<EntityId, Error> {
        self.resolved_name(name)
            .map(|id| {
                let entity = self.binding(id).entity;
                self.reference_argument_entity(entity).unwrap_or(entity)
            })
            .ok_or(Error::Invariant("lowering name without entity"))
    }
    pub(crate) fn lower_named_object(&self, name: ast::Name) -> Result<bool, Error> {
        let binding = self
            .resolved_name(name)
            .ok_or(Error::Invariant("lowering name without binding"))?;
        Ok(self
            .reference_argument_entity(self.binding(binding).entity)
            .is_some()
            || matches!(
                self.binding(binding).kind,
                BindingKind::Variable | BindingKind::Parameter
            ))
    }
    pub(crate) fn lower_entity_type(&self, id: EntityId) -> Result<TypeId, Error> {
        self.entity(id)
            .ty
            .ok_or(Error::Invariant("lowering entity without type"))
    }
    pub(crate) fn lower_functional_call(&self, id: ast::ExprId) -> Result<bool, Error> {
        Ok(self.expression_facts.get(id)?.functional_call)
    }
    pub(crate) fn lower_no_inline(&self, entity: EntityId) -> bool {
        self.entity(self.lower_function_entity(entity))
            .function
            .is_some_and(|properties| properties.no_inline)
    }
    pub(crate) fn lower_floating_intrinsic(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<FloatingIntrinsic>, Error> {
        Ok(self.expression_facts.get(id)?.intrinsic)
    }
    pub(crate) fn lower_operator_call(&self, id: ast::ExprId) -> Result<bool, Error> {
        Ok(self.expression_facts.get(id)?.call_form == super::expression_facts::CallForm::Operator)
    }
    pub(crate) fn lower_literal_storage(&self, id: ast::ExprId) -> Result<TypeId, Error> {
        let facts = self.expression_facts.get(id)?;
        Ok(match facts.literal_call {
            Some(LiteralCall::String { object, .. } | LiteralCall::Raw { object, .. }) => object,
            _ => facts.ty,
        })
    }
    pub(crate) fn lower_literal_call(&self, id: ast::ExprId) -> Result<Option<LiteralCall>, Error> {
        Ok(self.expression_facts.get(id)?.literal_call)
    }
    pub(crate) fn lower_call(&self, id: ast::ExprId) -> Result<CallTarget, Error> {
        match self.expression_facts.get(id)?.call {
            Some(super::function_resolution::CallTarget::Direct(binding)) => {
                Ok(CallTarget::Direct(self.binding(binding).entity))
            }
            Some(super::function_resolution::CallTarget::Indirect(ty, entity)) => {
                let nothrow = entity.is_some_and(|entity| {
                    self.entity(entity).target_unwind == super::exceptions::TargetUnwind::NoThrow
                });
                Ok(CallTarget::Indirect(ty, nothrow))
            }
            None => Err(Error::Invariant("call without selected target")),
        }
    }
    pub(crate) fn lower_destructor_evaluation_only(&self, id: ast::ExprId) -> Result<bool, Error> {
        Ok(match self.expression_facts.get(id)?.destructor {
            Some(super::expression_facts::DestructorTarget::Scalar) => true,
            Some(super::expression_facts::DestructorTarget::Class(binding)) => {
                let key = self
                    .entity(self.binding(binding).entity)
                    .member_function
                    .ok_or(Error::Invariant("destructor without member key"))?;
                self.destructor_trivial(key.class)?
            }
            None => false,
        })
    }
    pub(crate) fn lower_destructor_body_inert(
        &self,
        entity: EntityId,
        tree: &ast::Tree,
    ) -> Result<bool, Error> {
        let Some(member) = self
            .entity(entity)
            .member_function
            .filter(|key| key.kind == super::graph::MemberKind::Destructor)
        else {
            return Ok(false);
        };
        // An implicit union destructor destroys no variant member. C++ tests
        // this before deletedness, including anonymous union storage.
        if self.lower_union_class(member.class) && self.lower_implicit_destructor(entity) {
            return Ok(true);
        }
        // A deleted destructor's synthesized body is not inert: an explicit
        // emission root still retains its subobject destruction sequence.
        if self.entity(entity).availability == super::graph::FunctionAvailability::Deleted {
            return Ok(false);
        }
        let Some(body) = self.lower_function_body(entity) else {
            return Ok(matches!(
                self.entity(entity).special_definition,
                Some(super::special_members::SpecialDefinition::Defaulted { .. })
            ));
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            return Err(Error::Invariant("destructor has unparsed body"));
        };
        Ok(body.handlers.is_none()
            && matches!(tree.statements.get(body.compound).kind, ast::StmtKind::Compound(items) if tree.stmt_lists.get(items).is_empty()))
    }
    pub(crate) fn lower_class_arguments(&self, entity: EntityId) -> Option<Vec<TypeId>> {
        if let Some(instance) = self.class_templates.instance(entity) {
            Some(instance.arguments.clone())
        } else {
            self.entity(entity).template_head.map(|head| {
                self.templates
                    .parameters(head)
                    .iter()
                    .filter_map(|&parameter| self.entity(parameter).ty)
                    .collect()
            })
        }
    }
    pub(crate) fn lower_qualifier_level(
        &self,
        ty: TypeId,
    ) -> Result<Option<(NameId, &[TypeId])>, Error> {
        let TypeKind::Class(entity) = self.types.get(ty).kind else {
            return Ok(None);
        };
        let Some(instance) = self.class_templates.instance(entity) else {
            return Ok(None);
        };
        if self
            .class_templates
            .template_entities
            .name(instance.primary)
            .is_some()
        {
            return Ok(None);
        }
        let Some(name) = self.entity(entity).name else {
            return Ok(None);
        };
        let count = self
            .class_templates
            .qualifiers
            .count(entity)?
            .ok_or(Error::Invariant("bound qualifier without argument facts"))?;
        Ok(Some((name, &instance.arguments[..count])))
    }
    pub(crate) fn lower_address_name(
        &self,
        id: EntityId,
    ) -> Result<(ScopeId, Option<NameId>), Error> {
        let entity = self.entity(id);
        Ok((
            self.lower_linkage_scope(
                entity
                    .home
                    .ok_or(Error::Invariant("address entity without home"))?,
            )?,
            entity.name,
        ))
    }
    pub(crate) fn lower_nominal_name(
        &self,
        ty: TypeId,
    ) -> Result<(EntityId, ScopeId, Option<NameId>), Error> {
        let (TypeKind::Enum(id) | TypeKind::Class(id)) = self.types.get(ty).kind else {
            return Err(Error::Invariant("non-nominal ABI type"));
        };
        let entity = self.entity(id);
        Ok((
            id,
            entity.home.ok_or(Error::Invariant("enum without home"))?,
            entity.name,
        ))
    }
    pub(crate) fn lower_default(&self, entity: EntityId, index: usize) -> Option<ast::ExprId> {
        self.function_default(entity, index)
    }
    pub(crate) fn lower_statement(&self, id: ast::StmtId) -> Result<ast::StmtId, Error> {
        self.statement_readings
            .get(id.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("ambiguous statement without reading"))
    }
    pub(crate) fn lower_anonymous_namespace(&self, id: ScopeId) -> bool {
        let mut scope = id;
        while let Some(parent) = self.scope(scope).parent {
            if self.scope(scope).kind == super::graph::ScopeKind::Namespace
                && self.scope(scope).name.is_none()
            {
                return true;
            }
            scope = parent;
        }
        false
    }
    pub(crate) fn lower_class_empty(&self, class: EntityId) -> Result<bool, Error> {
        if let super::layout::LayoutState::Value { empty, .. } = self.entity(class).layout {
            Ok(empty)
        } else {
            Err(Error::Invariant("class parameter without layout"))
        }
    }
    pub(crate) fn lower_class_layout(&self, entity: EntityId) -> Result<(u64, u8), Error> {
        let super::layout::LayoutState::Value {
            size, alignment, ..
        } = self.entity(entity).layout
        else {
            return Err(Error::Invariant("class lowering without completed layout"));
        };
        Ok((
            size,
            u8::try_from(alignment.trailing_zeros())
                .map_err(|_| Error::Limit("class alignment"))?,
        ))
    }
    pub(crate) fn lower_complete_object(&self, ty: TypeId) -> Result<bool, Error> {
        if self.lower_member_pointer_type(ty).is_some() {
            return Ok(true);
        }
        Ok(match self.lower_type(ty)? {
            TypeView::Array { bound, .. } => bound != 0,
            TypeView::Class(entity) => self.entity(entity).complete,
            TypeView::Function { .. } | TypeView::Unsupported | TypeView::Scalar(Scalar::Void) => {
                false
            }
            _ => true,
        })
    }
    pub(crate) fn lower_member(&self, expr: ast::ExprId) -> Result<(EntityId, Option<u64>), Error> {
        let binding = self
            .expression_facts
            .get(expr)?
            .member
            .ok_or(Error::Invariant("member without designation"))?;
        let entity = self.binding(binding).entity;
        let node = self.entity(entity);
        Ok((
            entity,
            if node.data_member == Some(super::graph::DataMemberKind::Instance) {
                Some(
                    self.lower_member_offset(entity)?
                        .wrapping_add(self.lower_anonymous_field_offset(expr)?),
                )
            } else {
                None
            },
        ))
    }
    pub(crate) fn lower_union_member(&self, expr: ast::ExprId) -> Result<bool, Error> {
        let facts = self.expression_facts.get(expr)?;
        let binding = facts
            .member
            .ok_or(Error::Invariant("member without designation"))?;
        if self.member_in_union(self.binding(binding).entity) {
            return Ok(true);
        }
        let holder = self.member_holder(self.binding(binding).entity, facts.member_naming)?;
        Ok(std::iter::once(holder)
            .chain(facts.member_naming)
            .any(|class| self.entity(class).key == crate::lex::vocabulary::Keyword::Union))
    }
    pub(crate) fn lower_member_union(&self, member: EntityId) -> bool {
        self.member_in_union(member)
    }
    pub(crate) fn lower_class_members(
        &self,
        entity: EntityId,
    ) -> Result<(&[EntityId], bool), Error> {
        if !self.aggregate_class(entity) {
            return Err(Error::Missing("nonaggregate class lowering"));
        }
        Ok((
            &self.entity(entity).members,
            self.entity(entity).key == crate::lex::vocabulary::Keyword::Union,
        ))
    }
    pub(crate) fn lower_member_offset(&self, entity: EntityId) -> Result<u64, Error> {
        if let Some(id) = self.entity(entity).bit_field {
            self.bit_fields.storage(id)?;
            return Err(Error::Missing("bit-field storage access lowering"));
        }
        self.entity(entity)
            .member_offset
            .ok_or(Error::Invariant("aggregate member without layout"))
    }
    pub(crate) fn lower_bit_field(&self, entity: EntityId) -> Result<Option<BitField>, Error> {
        let Some(id) = self.entity(entity).bit_field else {
            return Ok(None);
        };
        let (operation, signed) = self.bit_fields.semantics(id)?;
        let storage = self.bit_fields.storage(id)?;
        Ok(Some(BitField {
            operation,
            signed,
            width: self.bit_fields.width(id),
            bit: storage.bit,
            offset: storage.offset,
            bytes: storage.bytes,
            split: storage.split,
            shared: storage.shared,
        }))
    }
    pub(crate) fn lower_anonymous_field_offset(&self, expr: ast::ExprId) -> Result<u64, Error> {
        let facts = self.expression_facts.get(expr)?;
        let binding = facts
            .member
            .ok_or(Error::Invariant("member without designation"))?;
        self.lower_anonymous_member_offset(self.binding(binding).entity, facts.member_naming)
    }
    pub(crate) fn lower_anonymous_constructor(&self, id: ConstructorId) -> bool {
        self.anonymous_member_storage(self.constructors.get(id).class)
            .is_some()
    }
    pub(crate) fn lower_anonymous_member_offset(
        &self,
        entity: EntityId,
        within: Option<EntityId>,
    ) -> Result<u64, Error> {
        let mut owner = self
            .entity(entity)
            .home
            .and_then(|home| self.scope(home).owner);
        let mut offset = 0u64;
        while let Some(class) = owner {
            if Some(class) == within {
                break;
            }
            let Some(object) = self.anonymous_member_storage(class) else {
                break;
            };
            offset = offset.wrapping_add(self.lower_member_offset(object)?);
            owner = self
                .entity(object)
                .home
                .and_then(|home| self.scope(home).owner);
        }
        Ok(offset)
    }
    pub(crate) fn lower_designated_bit_field(&self, expr: ast::ExprId) -> Result<EntityId, Error> {
        let facts = self.expression_facts.get(expr)?;
        let binding = facts
            .member
            .ok_or(Error::Invariant("bit-field without designation"))?;
        Ok(self.binding(binding).entity)
    }
    pub(crate) fn lower_unnamed_bit_field(&self, entity: EntityId) -> bool {
        self.unnamed_bit_field(entity)
    }
    pub(crate) fn lower_class_member_entities(&self, entity: EntityId) -> &[EntityId] {
        &self.entity(entity).members
    }
    pub(crate) fn lower_class_bases(
        &self,
        entity: EntityId,
    ) -> impl Iterator<Item = EntityId> + '_ {
        self.entity(entity).bases.iter().map(|base| base.class)
    }
    pub(crate) fn lower_mutable_member(&self, entity: EntityId) -> bool {
        self.entity(entity).mutability == super::graph::Mutability::Mutable
    }
    pub(crate) fn lower_static_member(&self, entity: EntityId) -> bool {
        self.entity(entity).data_member == Some(super::graph::DataMemberKind::Static)
    }
    pub(crate) fn lower_instance_member(&self, entity: EntityId) -> bool {
        self.entity(entity).data_member == Some(super::graph::DataMemberKind::Instance)
    }
    pub(crate) fn lower_nontrivial_destructor(&self, entity: EntityId) -> Result<bool, Error> {
        Ok(!self.destructor_trivial(entity)?)
    }
    pub(crate) fn lower_implicit_union_destructor(&self, class: EntityId) -> bool {
        self.lower_union_class(class) && self.entity(class).declared_destructor.is_none()
    }
    pub(crate) fn lower_destructor_entity(&self, class: EntityId) -> Option<EntityId> {
        self.destructor_binding(class)
            .map(|binding| self.binding(binding).entity)
    }
    pub(crate) fn lower_implicit_destructor(&self, entity: EntityId) -> bool {
        self.entity(entity).member_function.is_some_and(|key| {
            key.kind == super::graph::MemberKind::Destructor
                && self
                    .entity(key.class)
                    .implicit_destructor
                    .is_some_and(|binding| self.binding(binding).entity == entity)
        })
    }
}

impl Graph {
    pub(crate) fn lower_destructor_bases(
        &self,
        class: EntityId,
    ) -> impl DoubleEndedIterator<Item = Result<(EntityId, u64), Error>> + ExactSizeIterator + '_
    {
        self.entity(class).bases.iter().map(|base| {
            Ok((
                base.class,
                base.offset
                    .ok_or(Error::Invariant("destructor base without layout"))?,
            ))
        })
    }
    pub(crate) fn lower_union_class(&self, class: EntityId) -> bool {
        self.entity(class).key == crate::lex::vocabulary::Keyword::Union
    }
}
