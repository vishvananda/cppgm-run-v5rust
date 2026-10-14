// Translated from sema/semantic_unit.h's lowering facts. See NOTICE.
//! Read-only views of analyzed facts; the syntax and semantic stores remain their owners.
pub(crate) use super::builtin_calls::BuiltinFunction;
pub(crate) use super::static_arrays::Item as StaticItem;
pub(crate) use super::static_initialization::Value as StaticValue;
use super::{
    Error,
    graph::{BindingKind, Graph},
    types::TypeKind,
};
pub(crate) use super::{
    graph::{EntityId, ScopeId},
    types::{Scalar, TypeId},
};
use crate::{lex::identifiers::NameId, syntax::ast};
pub(crate) struct ConstructorView<'a> {
    pub object_pointer: TypeId,
    pub union: bool,
    pub members: &'a [(EntityId, Option<ast::Initializer>)],
    pub bases: &'a [ConstructorId],
    pub subobjects: &'a [(EntityId, ConstructorId)],
}
pub(crate) struct VariableFacts {
    pub name: NameId,
    pub home: ScopeId,
    pub ty: TypeId,
    pub internal: bool,
    pub locality: Locality,
    pub c_linkage: bool,
    pub thread_local: bool,
    pub initializer: Option<StaticValue>,
    pub reference_temporary: Option<super::static_initialization::ReferenceTemporary>,
    pub definition: Option<ast::DeclaratorId>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Locality {
    External,
    Unit,
}

pub(crate) enum TypeView<'a> {
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
pub(crate) enum CallTarget {
    Direct(EntityId),
    Indirect(TypeId, bool),
}
pub(crate) use super::construction::ConstructorId;
#[derive(Clone, Copy)]
pub(crate) enum FunctionName {
    Identifier(NameId),
    Constructor(NameId),
    Operator {
        token: crate::lex::vocabulary::SimpleToken,
        array: bool,
    },
    Literal(NameId),
}
#[derive(Clone, Copy)]
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
    pub name: FunctionName,
    pub label: Option<ast::Name>,
    pub home: ScopeId,
    pub ty: TypeId,
    pub scope: Option<ScopeId>,
    pub internal: bool,
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
pub(crate) struct Expression {
    pub ty: TypeId,
    pub target: Option<TypeId>,
    pub folded: Option<u64>,
    pub query: Option<u64>,
    pub is_place: bool,
    pub null_pointer: bool,
    pub commuted: bool,
    pub temporary: Option<TypeId>,
    pub reference_cast: Option<TypeId>,
}
pub(crate) use super::expression_facts::ObjectSource;
impl Graph {
    pub(crate) fn lower_constructor_trivial(&self, id: ConstructorId) -> bool {
        !self.constructors.get(id).nontrivial(self)
    }
    pub(crate) fn lower_constructor(&self, id: ConstructorId) -> ConstructorView<'_> {
        let plan = self.constructors.get(id);
        ConstructorView {
            object_pointer: plan.object_pointer,
            union: self.entity(plan.class).key == crate::lex::vocabulary::Keyword::Union,
            members: &plan.members,
            bases: &plan.bases,
            subobjects: &plan.subobjects,
        }
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
            name: FunctionName::Constructor(
                class
                    .name
                    .ok_or(Error::Missing("anonymous constructor name"))?,
            ),
            label: None,
            home: class
                .scope
                .ok_or(Error::Invariant("constructor without class scope"))?,
            ty: plan.callable_type,
            scope: None,
            internal: self.lower_entity_unit_local(plan.class),
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
    pub(crate) fn lower_constructor_overloads(
        &self,
        class: EntityId,
    ) -> impl Iterator<Item = EntityId> + '_ {
        self.entity(class)
            .declared_constructors
            .iter()
            .map(|&binding| self.binding(binding).entity)
    }
    pub(crate) fn lower_function_defined(&self, entity: EntityId) -> bool {
        self.entity(entity).defined
    }
    pub(crate) fn lower_function_body(&self, id: EntityId) -> Option<ast::BodyId> {
        match self.entity(id).special_definition {
            Some(super::special_members::SpecialDefinition::Body(body)) => Some(body),
            _ => None,
        }
    }

    pub(crate) fn lower_object_source(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<ObjectSource>, Error> {
        Ok(self
            .expression_facts
            .get(id)?
            .object
            .map(|object| object.source))
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
    fn lower_type_unit_local(&self, mut ty: TypeId) -> bool {
        // Scalar/object and linear pointer shells need no scratch allocation.
        // Only a function type introduces the shared component DAG worklist.
        loop {
            ty = self.types.canonical_view(ty);
            match self.types.get(ty).kind {
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Array { element, .. } => ty = element,
                TypeKind::Class(entity) | TypeKind::Enum(entity) => {
                    return self.lower_entity_unit_local(entity);
                }
                TypeKind::Function { .. } => break,
                _ => return false,
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
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters);
                }
                TypeKind::Class(entity) | TypeKind::Enum(entity)
                    if self.lower_entity_unit_local(*entity) =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    }
    fn lower_entity_unit_local(&self, mut entity: EntityId) -> bool {
        use super::graph::ScopeKind;
        loop {
            let node = self.entity(entity);
            let Some(owner) = node.home else {
                return true;
            };
            let scope = self.scope(owner);
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
    ) -> Result<(Option<ScopeId>, Option<NameId>), Error> {
        let scope = self.scope(id);
        match scope.kind {
            super::graph::ScopeKind::Namespace => Ok((scope.parent, scope.name)),
            super::graph::ScopeKind::Class => Ok((
                scope.parent,
                Some(
                    scope
                        .name
                        .ok_or(Error::Missing("anonymous class helper label"))?,
                ),
            )),
            _ => Ok((None, None)),
        }
    }
    pub(crate) fn lower_trivial_default_construction(
        &self,
        entity: EntityId,
    ) -> Result<bool, Error> {
        let id = self.entity(entity).constructor.ok_or(Error::Invariant(
            "default class object without construction facts",
        ))?;
        Ok(!self.constructors.get(id).nontrivial(self))
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
    pub(crate) fn lower_array_image(&self, id: EntityId) -> Option<&[super::static_arrays::Item]> {
        self.array_images.get(id)
    }
    pub(crate) fn lower_variable(&self, id: EntityId) -> Result<VariableFacts, Error> {
        let entity = self.entity(id);
        let variable = entity
            .variable
            .ok_or(Error::Invariant("lowering non-variable"))?;
        Ok(VariableFacts {
            name: entity
                .name
                .ok_or(Error::Missing("anonymous variable lowering"))?,
            home: self.lower_linkage_scope(
                entity
                    .home
                    .ok_or(Error::Invariant("variable without home"))?,
            )?,
            ty: entity.ty.ok_or(Error::Invariant("variable without type"))?,
            internal: variable.internal_linkage,
            locality: if variable.internal_linkage
                || variable.language != super::linkage::Language::C
                    && self.lower_type_unit_local(
                        entity.ty.ok_or(Error::Invariant("variable without type"))?,
                    ) {
                Locality::Unit
            } else {
                Locality::External
            },
            c_linkage: variable.language == super::linkage::Language::C,
            thread_local: variable.thread_storage == super::specifiers::ThreadStorage::ThreadLocal,
            initializer: entity.static_initializer,
            reference_temporary: entity.reference_temporary,
            definition: entity.variable_definition,
        })
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
    pub(crate) fn lower_namespace_storage(&self, id: EntityId) -> bool {
        self.entity(id)
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
        let function = entity
            .function
            .ok_or(Error::Invariant("lowering non-function"))?;
        let object = entity
            .home
            .filter(|&home| self.scope(home).kind == super::graph::ScopeKind::Class)
            .and_then(|home| self.scope(home).owner)
            .filter(|_| !function.is_static);
        let mut qualifiers = crate::abi::graph::Qualifiers::default();
        if let Some(ty) = entity.ty
            && let TypeKind::Function { cv, reference, .. } = self.types.get(ty).kind
        {
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
        Ok(FunctionFacts {
            object,
            qualifiers,
            name: match function.special {
                Some(super::function_names::SpecialName::Operator { token, array }) => {
                    FunctionName::Operator { token, array }
                }
                Some(super::function_names::SpecialName::Literal(name)) => {
                    FunctionName::Literal(name)
                }
                Some(super::function_names::SpecialName::Destructor) => {
                    return Err(Error::Missing("member function names"));
                }
                None if entity
                    .member_function
                    .is_some_and(|key| key.kind == super::graph::MemberKind::Constructor) =>
                {
                    FunctionName::Constructor(
                        self.entity(object.ok_or(Error::Invariant("constructor without class"))?)
                            .name
                            .ok_or(Error::Missing("anonymous constructor name"))?,
                    )
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
            internal: function.internal_linkage
                || entity
                    .home
                    .filter(|&home| self.scope(home).kind == super::graph::ScopeKind::Class)
                    .and_then(|home| self.scope(home).owner)
                    .is_some_and(|class| self.lower_entity_unit_local(class)),
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
    pub(crate) fn lower_abi_scope(
        &self,
        id: ScopeId,
    ) -> Result<(Option<ScopeId>, Option<NameId>, bool), Error> {
        let namespace = self.scope(id).kind == super::graph::ScopeKind::Namespace;
        if !namespace && self.scope(id).kind != super::graph::ScopeKind::Class {
            return Err(Error::Missing("lowering class and local scopes"));
        }
        let (parent, name) = self.lower_presentation_scope(id)?;
        Ok((parent, name, namespace))
    }
    pub(crate) fn lower_namespace(
        &self,
        id: ScopeId,
    ) -> Result<(Option<ScopeId>, Option<NameId>), Error> {
        let scope = self.scope(id);
        if scope.kind != super::graph::ScopeKind::Namespace {
            return Err(Error::Missing("lowering class and local scopes"));
        }
        Ok((scope.parent, scope.name))
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
            .functions_at(home, name)
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
    pub(crate) fn lower_type(&self, id: TypeId) -> Result<TypeView<'_>, Error> {
        Ok(match &self.types.get(self.types.canonical_view(id)).kind {
            TypeKind::Scalar(scalar) => TypeView::Scalar(*scalar),
            TypeKind::Pointer(element) => TypeView::Pointer(*element),
            TypeKind::Array { element, bound } => TypeView::Array {
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
    pub(crate) fn lower_expression(&self, id: ast::ExprId) -> Result<Expression, Error> {
        let facts = self.expression_facts.get(id)?;
        Ok(Expression {
            ty: facts.ty,
            target: facts.conversion.map(|c| c.result),
            folded: facts.folded,
            query: facts.query_value,
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
            .map(|id| self.binding(id).entity)
            .ok_or(Error::Invariant("lowering name without entity"))
    }
    pub(crate) fn lower_entity_type(&self, id: EntityId) -> Result<TypeId, Error> {
        self.entity(id)
            .ty
            .ok_or(Error::Invariant("lowering entity without type"))
    }
    pub(crate) fn lower_functional_call(&self, id: ast::ExprId) -> Result<bool, Error> {
        Ok(self.expression_facts.get(id)?.functional_call)
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
    pub(crate) fn lower_nominal_name(&self, ty: TypeId) -> Result<(ScopeId, NameId), Error> {
        let (TypeKind::Enum(id) | TypeKind::Class(id)) = self.types.get(ty).kind else {
            return Err(Error::Invariant("non-nominal ABI type"));
        };
        let entity = self.entity(id);
        Ok((
            entity.home.ok_or(Error::Invariant("enum without home"))?,
            entity
                .name
                .ok_or(Error::Missing("anonymous enum ABI name"))?,
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
    pub(crate) fn lower_class_layout(&self, entity: EntityId) -> Result<(u64, u8), Error> {
        let super::layout::LayoutState::Value { size, alignment } = self.entity(entity).layout
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
                    node.member_offset
                        .ok_or(Error::Invariant("member without offset"))?,
                )
            } else {
                None
            },
        ))
    }
    pub(crate) fn lower_union_member(&self, entity: EntityId) -> bool {
        self.entity(entity)
            .home
            .and_then(|home| self.scope(home).owner)
            .is_some_and(|class| self.entity(class).key == crate::lex::vocabulary::Keyword::Union)
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
        self.entity(entity)
            .member_offset
            .ok_or(Error::Invariant("aggregate member without layout"))
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
    pub(crate) fn lower_nontrivial_destructor(&self, entity: EntityId) -> bool {
        self.entity(entity)
            .declared_destructor
            .is_some_and(|binding| {
                !matches!(
                    self.entity(self.binding(binding).entity).special_definition,
                    Some(super::special_members::SpecialDefinition::Defaulted { first: true, .. })
                )
            })
    }
}
