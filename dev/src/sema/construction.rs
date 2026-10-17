// Translated from sema/analyzer_initialization.cpp and analyzer_construction.cpp. See NOTICE.
//! Demand-only default construction and source-ordered scalar subobject plans.
//! Records are unit-owned; declarations and expressions retain compact identities.
use super::constructor_initialization::ConstructorArguments;
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, EntityId, MemberKind, ScopeId},
    special_members::SpecialDefinition,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

#[derive(Clone, Copy)]
pub(crate) struct BaseInitialization {
    pub class: EntityId,
    pub constructor: Option<ConstructorId>,
    pub arguments: Option<ConstructorArguments>,
    pub initializer: Option<ast::Initializer>,
    pub forwards: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ConstructorExceptionSource {
    #[default]
    Declared,
    SubobjectPlan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ConstructorId(u32);
#[derive(Clone, Copy, Default)]
pub(super) enum ConstructorState {
    #[default]
    NotStarted,
    InProgress,
    Ready(ConstructorId),
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Triviality {
    #[default]
    Trivial,
    Nontrivial,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CopyTransfer {
    Call,
    Storage,
}
#[derive(Clone, Copy)]
pub(crate) struct MemberConstruction {
    pub member: EntityId,
    pub constructor: ConstructorId,
    pub arguments: Option<ConstructorArguments>,
}
#[derive(Clone, Copy)]
pub(crate) struct Delegation {
    pub constructor: ConstructorId,
    pub arguments: ConstructorArguments,
    pub initializer: ast::Initializer,
}
pub(super) struct Constructor {
    pub class: EntityId,
    pub binding: Option<BindingId>,
    pub object_pointer: TypeId,
    // The dump's function type includes its implicit object parameter.
    pub presentation_type: TypeId,
    pub callable_type: TypeId,
    pub nothrow: bool,
    pub(super) deleted: bool,
    pub(super) const_default: bool,
    pub members: Vec<(EntityId, Option<ast::Initializer>)>,
    pub written_members: Vec<bool>,
    pub bases: Vec<BaseInitialization>,
    pub subobjects: Vec<MemberConstruction>,
    pub delegation: Option<Delegation>,
    pub copy: Option<Vec<super::copy_construction::SubobjectCopy>>,
    pub(super) copy_transfer: CopyTransfer,
    pub(super) triviality: Triviality,
}
impl Constructor {
    fn initialized_members(&self) -> usize {
        self.members
            .iter()
            .filter(|(_, source)| source.is_some())
            .count()
    }
    pub(super) fn nontrivial(&self) -> bool {
        self.triviality == Triviality::Nontrivial
    }
    fn own_action(&self, graph: &super::graph::Graph) -> bool {
        graph.entity(self.class).polymorphism.is_dynamic()
            || self.initialized_members() != 0
            || self.binding.is_some_and(|binding| {
                !matches!(
                    graph
                        .entity(graph.binding(binding).entity)
                        .special_definition,
                    Some(SpecialDefinition::Defaulted { first: true, .. })
                )
            })
    }
}
#[derive(Default)]
pub(super) struct Constructors {
    records: Vec<Constructor>,
}
impl Constructors {
    pub(super) fn add(&mut self, constructor: Constructor) -> Result<ConstructorId, Error> {
        let id = ConstructorId(
            u32::try_from(self.records.len())
                .map_err(|_| Error::Limit("implicit constructor limit"))?,
        );
        self.records.push(constructor);
        Ok(id)
    }
    pub(super) fn get(&self, id: ConstructorId) -> &Constructor {
        &self.records[id.0 as usize]
    }
    fn replace(&mut self, id: ConstructorId, record: Constructor) {
        self.records[id.0 as usize] = record;
    }
}
impl Analyzer<'_> {
    pub(super) fn validate_const_default(
        &self,
        target: TypeId,
        constructor: ConstructorId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let plan = self.graph.constructors.get(constructor);
        if self.graph.types.get(target).cv.0 & 1 != 0 && !plan.const_default {
            return Err(Error::at(
                location,
                if self.graph.entity(plan.class).key == Keyword::Union {
                    "a const union object initializes no member"
                } else {
                    "a const object leaves a member uninitialized"
                },
            ));
        }
        Ok(())
    }
    pub(super) fn default_initialize_declaration(
        &mut self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.default_initialize(binding, location)?;
        let entity = self.graph.binding(binding).entity;
        if self.graph.entity(entity).constexpr {
            let scope = self.graph.binding_scope(binding);
            self.record_aggregate_constant(entity, None, scope, true)?;
        }
        if self.lowering() {
            self.request_in_class_initializer(self.graph.binding(binding).entity)?;
        }
        let variable = self.graph.entity(self.graph.binding(binding).entity);
        // declare_variable counts nontrivial automatic construction after
        // selecting it. anonymous_union's hidden-object path does not count it.
        if variable
            .variable
            .is_some_and(|properties| properties.automatic)
            && variable
                .constructor
                .is_some_and(|id| self.graph.constructors.get(id).nontrivial())
        {
            self.note_local_initializer(
                variable
                    .home
                    .ok_or(Error::Invariant("local without home"))?,
            )?;
        }
        Ok(())
    }
    pub(super) fn default_initialize(
        &mut self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.template_depth != 0 {
            return Ok(());
        }
        let variable = self.graph.binding(binding).entity;
        if self.graph.entity(variable).data_member == Some(super::graph::DataMemberKind::Static)
            && (self.graph.entity(variable).constexpr
                || matches!(
                    self.graph.entity(variable).constant,
                    super::constants::ConstantState::Value(_)
                ))
        {
            return Ok(());
        }
        let ty = self
            .graph
            .entity(variable)
            .ty
            .ok_or(Error::Invariant("default initialization without type"))?;
        let mut object = ty;
        while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
            object = element;
        }
        if let TypeKind::Class(class) = self.graph.types.get(object).kind {
            let constructor = self.default_constructor(class, location)?;
            let record = self.graph.constructors.get(constructor);
            if record.deleted {
                return Err(Error::at(location, "use of a deleted function"));
            }
            if let Some(selected) = record.binding {
                self.check_special_member_access(
                    selected,
                    self.base_object.unwrap_or(class),
                    self.graph.binding_scope(binding),
                    location,
                )?;
            }
            if self.graph.types.get(object).cv.0 & 1 != 0 && !record.const_default {
                return Err(Error::at(
                    location,
                    if self.graph.entity(class).key == Keyword::Union {
                        "a const union object initializes no member"
                    } else {
                        "a const object leaves a member uninitialized"
                    },
                ));
            }
            self.graph.entity_mut(variable).constructor = Some(constructor);
            if self.lowering()
                && self.unevaluated == 0
                && self.graph.entity(variable).data_member
                    != Some(super::graph::DataMemberKind::Instance)
            {
                self.graph.constructor_boundaries.push((
                    self.body_context.as_ref().map(|body| body.entity),
                    constructor,
                ));
            }
            // An array shows one element action without demanding a definition.
            if !matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. })
                && self.unevaluated == 0
            {
                if let Some(selected) = self.graph.constructors.get(constructor).binding {
                    self.demand_function(selected, true);
                } else {
                    self.record_demand(super::function_demands::Demand::Constructor(constructor));
                }
            }
        } else if self.graph.types.get(object).cv.0 & 1 != 0
            && !self
                .graph
                .entity(variable)
                .variable
                .is_some_and(|properties| properties.is_extern)
        {
            return Err(Error::at(location, "const object without an initializer"));
        } else if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            return Err(Error::at(location, "reference without an initializer"));
        }
        Ok(())
    }

    pub(super) fn default_constructor(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        // Ordinary nonvirtual bases are completed bottom-up on demand. A deep
        // hierarchy must not consume one native Rust frame per base.
        let mut work = vec![(class, 0usize)];
        let mut active = std::collections::HashSet::from([class]);
        while let Some((current, next)) = work.last_mut() {
            let plan_location = if *current == class {
                location
            } else {
                SourceOffset(0)
            };
            if matches!(
                self.graph.entity(*current).default_construction,
                ConstructorState::Ready(_)
            ) {
                active.remove(current);
                work.pop();
            } else if *next == 0
                && self.graph.entity(*current).user_constructor.is_some()
                && !matches!(
                    self.graph
                        .entity(
                            self.graph
                                .binding(self.default_constructor_binding(*current, plan_location)?)
                                .entity
                        )
                        .special_definition,
                    Some(SpecialDefinition::Defaulted { .. })
                )
            {
                // A user-provided declaration does not ask for its bases'
                // default constructors. Written/defaulted plans own that demand.
                self.select_default_constructor(*current, plan_location)?;
                active.remove(current);
                work.pop();
            } else if let Some(edge) = self.graph.entity(*current).bases.get(*next) {
                *next += 1;
                active.insert(edge.class);
                work.push((edge.class, 0));
            } else if *next
                < self.graph.entity(*current).bases.len()
                    + self.graph.entity(*current).members.len()
            {
                let index = *next - self.graph.entity(*current).bases.len();
                *next += 1;
                let member = self.graph.entity(*current).members[index];
                if let Some(child) = self.default_member_dependency(member)? {
                    if !active.insert(child) {
                        return Err(Error::at(location, "class contains itself"));
                    }
                    work.push((child, 0));
                }
            } else {
                self.select_default_constructor(*current, plan_location)?;
                active.remove(current);
                work.pop();
            }
        }
        let ConstructorState::Ready(id) = self.graph.entity(class).default_construction else {
            return Err(Error::Invariant("uncompleted default constructor"));
        };
        Ok(id)
    }

    pub(super) fn default_member_dependency(
        &mut self,
        member: EntityId,
    ) -> Result<Option<EntityId>, Error> {
        self.request_member_initializer(member)?;
        let member = self.graph.entity(member);
        if member.member_initializer.is_some() {
            return Ok(None);
        }
        let mut ty = member
            .ty
            .ok_or(Error::Invariant("constructor member without type"))?;
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        Ok(
            if let TypeKind::Class(child) = self.graph.types.get(ty).kind {
                Some(child)
            } else {
                None
            },
        )
    }

    fn select_default_constructor(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        match self.graph.entity(class).default_construction {
            ConstructorState::Ready(id) => return Ok(id),
            ConstructorState::InProgress => {
                return Err(Error::Invariant("recursive scalar constructor demand"));
            }
            ConstructorState::NotStarted => (),
        }
        if self.graph.entity(class).user_constructor.is_some() {
            let binding = self.default_constructor_binding(class, location)?;
            // The semantic view records this constructor action without
            // lowering its omitted arguments; emission needs their facts.
            if self.lowering() {
                self.instantiate_call_defaults(binding, 0)?;
            }
            let id = self.selected_constructor_plan(binding, location)?;
            self.graph.entity_mut(class).default_construction = ConstructorState::Ready(id);
            return Ok(id);
        }
        if !self.complete_class(class)? {
            return Err(Error::Invariant("constructor of an incomplete class"));
        }
        if self.graph.entity(class).template_head.is_some() {
            return Err(Error::Missing("class template default construction"));
        }
        self.graph.entity_mut(class).default_construction = ConstructorState::InProgress;
        let result = self.scalar_constructor_facts(class, None);
        self.graph.entity_mut(class).default_construction =
            result.as_ref().map_or(ConstructorState::NotStarted, |&id| {
                ConstructorState::Ready(id)
            });
        result
    }

    pub(super) fn default_constructor_binding(
        &self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        let mut selected = None;
        let mut templates = false;
        for &binding in &self.graph.entity(class).declared_constructors {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            let count = self.graph.types.function_parameter_count(
                entity
                    .ty
                    .ok_or(Error::Invariant("constructor without type"))?,
            )?;
            if (0..count).any(|index| entity.defaults.get(index).is_none_or(Option::is_none)) {
                continue;
            }
            if entity.template_head.is_some() {
                templates = true;
                continue;
            }
            if selected.replace(binding).is_some() {
                return Err(Error::at(SourceOffset(0), "ambiguous default constructor"));
            }
        }
        if selected.is_none() && templates {
            return Err(Error::Missing("default constructor template deduction"));
        }
        selected.ok_or_else(|| Error::at(location, "no default constructor for the class"))
    }

    fn prepare_defaulted_subobjects(&mut self, class: EntityId) -> Result<(), Error> {
        for index in 0..self.graph.entity(class).bases.len() {
            let child = self.graph.entity(class).bases[index].class;
            self.default_constructor(child, SourceOffset(0))?;
        }
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if let Some(child) = self.default_member_dependency(member)? {
                self.default_constructor(child, SourceOffset(0))?;
            }
        }
        Ok(())
    }

    pub(super) fn scalar_constructor_facts(
        &mut self,
        class: EntityId,
        binding: Option<BindingId>,
    ) -> Result<ConstructorId, Error> {
        let union = self.graph.entity(class).key == Keyword::Union;
        let mut deleted = false;
        let mut bases_const_default = true;
        let mut bases = Vec::with_capacity(self.graph.entity(class).bases.len());
        let inherited = binding.and_then(|binding| {
            self.graph
                .inherited_constructors
                .source(self.graph.binding(binding).entity)
        });
        for index in 0..self.graph.entity(class).bases.len() {
            let edge = self.graph.entity(class).bases[index];
            if matches!(
                self.graph.entity(edge.class).layout,
                super::layout::LayoutState::Unsupported
            ) {
                return Err(Error::Missing("extended base subobject constructor plans"));
            }
            let forwarded = inherited.filter(|&source| {
                self.graph
                    .entity(self.graph.binding(source).entity)
                    .member_function
                    .is_some_and(|key| key.class == edge.class)
            });
            let base = if let Some(source) = forwarded {
                self.declared_constructor_plan(source)?
            } else if let ConstructorState::Ready(base) =
                self.graph.entity(edge.class).default_construction
            {
                base
            } else {
                return Err(Error::Missing("defaulted base subobject constructor plans"));
            };
            bases.push(BaseInitialization {
                class: edge.class,
                constructor: Some(base),
                arguments: None,
                initializer: None,
                forwards: forwarded.is_some(),
            });
            let base = self.graph.constructors.get(base);
            // implicit_default_facts plans its bases with location zero;
            // default_constructor checks deletion at that plan's location.
            if base.deleted {
                return Err(Error::at(SourceOffset(0), "use of a deleted function"));
            }
            bases_const_default &= base.const_default;
        }
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("constructor class without scope"))?;
        for base in &bases {
            if !base.forwards
                && let Some(binding) = base
                    .constructor
                    .and_then(|id| self.graph.constructors.get(id).binding)
            {
                self.check_special_member_access(binding, class, scope, SourceOffset(0))?;
            }
            if let Some(binding) = self.graph.entity(base.class).declared_destructor {
                deleted |= !self.direct_member_accessible(binding, scope)
                    || self
                        .graph
                        .entity(self.graph.binding(binding).entity)
                        .availability
                        == super::graph::FunctionAvailability::Deleted;
            }
        }
        let mut plans = Vec::with_capacity(self.graph.entity(class).members.len());
        let mut subobjects = Vec::new();
        let mut const_members = 0;
        for index in 0..self.graph.entity(class).members.len() {
            let member_id = self.graph.entity(class).members[index];
            self.request_member_initializer(member_id)?;
            let member = self.graph.entity(member_id);
            let mut ty = member
                .ty
                .ok_or(Error::Invariant("constructor member without type"))?;
            if member.member_initializer.is_some() {
                if let Some(constructor) = member.constructor {
                    subobjects.push(MemberConstruction {
                        member: member_id,
                        constructor,
                        arguments: member.constructor_arguments,
                    });
                }
                plans.push((member_id, member.member_initializer));
                continue;
            }
            plans.push((member_id, None));
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            if let TypeKind::Class(child_class) = self.graph.types.get(ty).kind {
                let ConstructorState::Ready(id) =
                    self.graph.entity(child_class).default_construction
                else {
                    return Err(Error::Missing(
                        "defaulted class subobject constructor plans",
                    ));
                };
                let child = self.graph.constructors.get(id);
                if union {
                    deleted |=
                        self.graph.entity(class).initialized_members == 0 && child.nontrivial();
                    if self.graph.anonymous_member_storage(child_class).is_none()
                        || self.graph.entity(child_class).initialized_members == 0
                    {
                        continue;
                    }
                }
                if child.deleted && self.graph.anonymous_member_storage(child_class).is_none() {
                    return Err(Error::at(SourceOffset(0), "use of a deleted function"));
                }
                deleted |= child.deleted;
                let scope = self
                    .graph
                    .entity(class)
                    .scope
                    .ok_or(Error::Invariant("constructor class without scope"))?;
                if let Some(binding) = child.binding {
                    self.check_direct_member_access(binding, scope, SourceOffset(0))?;
                }
                deleted |= self
                    .graph
                    .entity(child_class)
                    .declared_destructor
                    .is_some_and(|binding| {
                        !self.direct_member_accessible(binding, scope)
                            || self
                                .graph
                                .entity(self.graph.binding(binding).entity)
                                .availability
                                == super::graph::FunctionAvailability::Deleted
                    });
                if !union && child.const_default {
                    const_members += 1;
                }
                if self.graph.types.get(ty).cv.0 & 1 != 0 && !child.const_default {
                    return Err(Error::at(
                        SourceOffset(0),
                        "a const object leaves a member uninitialized",
                    ));
                }
                subobjects.push(MemberConstruction {
                    member: member_id,
                    constructor: id,
                    arguments: None,
                });
                continue;
            }
            if !union {
                deleted |= matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                    || self.graph.types.get(ty).cv.0 & 1 != 0;
            }
        }
        let members = self.graph.entity(class).members.len();
        let initialized = self.graph.entity(class).initialized_members;
        let const_default = bases_const_default
            && (members == 0
                || if union {
                    initialized == 1
                } else {
                    initialized + const_members == members
                });
        let (object_pointer, presentation_type, callable_type) =
            self.constructor_types(class, binding)?;
        if let Some(binding) = binding {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            deleted |= entity.availability == super::graph::FunctionAvailability::Deleted;
        }
        let mut nothrow = true;
        for id in bases
            .iter()
            .filter_map(|base| base.constructor)
            .chain(subobjects.iter().map(|child| child.constructor))
        {
            nothrow &= self.subobject_constructor_nothrow(id)?;
        }
        if let (Some(source), Some(binding)) = (inherited, binding) {
            let function = self.graph.binding(source).entity;
            let count = self.graph.types.function_parameter_count(
                self.graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("inheriting constructor without type"))?,
            )?;
            let total = self.graph.types.function_parameter_count(
                self.graph
                    .binding(source)
                    .ty
                    .ok_or(Error::Invariant("forwarded constructor without type"))?,
            )?;
            for index in count..total {
                if let Some(default) = self.graph.function_default(function, index) {
                    nothrow &= !self.expression_may_throw(default)?;
                }
            }
        }
        for &(_, source) in &plans {
            if let Some(source) = source {
                nothrow &= !self.initializer_may_throw(source)?;
            }
        }
        let record = Constructor {
            class,
            binding,
            object_pointer,
            presentation_type,
            callable_type,
            nothrow,
            deleted,
            const_default: const_default
                || binding.is_some_and(|binding| {
                    matches!(
                        self.graph
                            .entity(self.graph.binding(binding).entity)
                            .special_definition,
                        Some(SpecialDefinition::Defaulted { first: false, .. })
                    )
                }),
            written_members: vec![false; plans.len()],
            members: plans,
            bases,
            subobjects,
            delegation: None,
            copy: None,
            copy_transfer: CopyTransfer::Call,
            triviality: Triviality::Trivial,
        };
        self.publish_constructor(record)
    }

    fn subobject_constructor_nothrow(&mut self, id: ConstructorId) -> Result<bool, Error> {
        let child = self.graph.constructors.get(id);
        if let Some(binding) = child.binding
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .constructor_exceptions
                == ConstructorExceptionSource::Declared
        {
            // A trivial subobject still contributes its declared specification.
            return self
                .function_may_throw(self.graph.binding(binding).entity)
                .map(|throws| !throws);
        }
        Ok(child.nothrow)
    }

    pub(super) fn constructor_types(
        &mut self,
        class: EntityId,
        binding: Option<BindingId>,
    ) -> Result<(TypeId, TypeId, TypeId), Error> {
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("constructor class without type"))?;
        let object_pointer = self
            .graph
            .types
            .intern(TypeKind::Pointer(ty), Cv::default())?;
        let void = self.graph.types.scalar(Scalar::Void)?;
        let callable_type = if let Some(binding) = binding {
            self.graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("constructor without signature"))?
        } else {
            self.graph.types.intern(
                TypeKind::Function {
                    result: void,
                    parameters: Vec::new(),
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )?
        };
        let TypeKind::Function {
            result,
            ref parameters,
            variadic,
            ..
        } = self.graph.types.get(callable_type).kind
        else {
            return Err(Error::Invariant("constructor without function type"));
        };
        let mut parameters = parameters.clone();
        parameters.insert(0, object_pointer);
        let presentation_type = self.graph.types.intern(
            TypeKind::Function {
                result,
                parameters,
                variadic,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        Ok((object_pointer, presentation_type, callable_type))
    }
    pub(super) fn publish_constructor(
        &mut self,
        mut record: Constructor,
    ) -> Result<ConstructorId, Error> {
        // Child default facts are ready bottom-up. Memoize transitive triviality
        // so use sites never walk the hierarchy again.
        record.triviality = if record.own_action(&self.graph)
            || record
                .bases
                .iter()
                .filter_map(|base| base.constructor)
                .chain(record.subobjects.iter().map(|child| child.constructor))
                .any(|id| self.graph.constructors.get(id).nontrivial())
        {
            Triviality::Nontrivial
        } else {
            Triviality::Trivial
        };
        self.store_constructor(record)
    }

    pub(super) fn store_constructor(
        &mut self,
        mut record: Constructor,
    ) -> Result<ConstructorId, Error> {
        record.copy_transfer = if self.graph.constructor_copies_storage(&record) {
            CopyTransfer::Storage
        } else {
            CopyTransfer::Call
        };
        let function = record
            .binding
            .map(|binding| self.graph.binding(binding).entity);
        if let Some(id) = function.and_then(|entity| self.graph.entity(entity).constructor) {
            self.graph.constructors.replace(id, record);
            return Ok(id);
        }
        let id = self.graph.constructors.add(record)?;
        if let Some(entity) = function {
            self.graph.entity_mut(entity).constructor = Some(id);
        }
        Ok(id)
    }

    pub(super) fn declared_constructor_plan(
        &mut self,
        binding: BindingId,
    ) -> Result<ConstructorId, Error> {
        let entity = self.graph.binding(binding).entity;
        if let Some(id) = self.graph.entity(entity).constructor {
            return Ok(id);
        }
        let class = self
            .graph
            .entity(entity)
            .member_function
            .ok_or(Error::Invariant("constructor without owner"))?
            .class;
        if self.graph.inherited_constructors.source(entity).is_some() {
            return self.inherited_constructor_plan(binding);
        }
        if self.graph.entity(entity).copy_origin == super::copy_construction::Origin::Implicit {
            let kind = self
                .graph
                .entity(entity)
                .copy_kind
                .ok_or(Error::Invariant("implicit constructor without copy kind"))?;
            return self.build_implicit_copy_plan(class, kind);
        }
        if matches!(
            self.graph.entity(entity).special_definition,
            Some(SpecialDefinition::Defaulted { .. })
        ) {
            if self.graph.types.function_parameter_count(
                self.graph
                    .entity(entity)
                    .ty
                    .ok_or(Error::Invariant("constructor without type"))?,
            )? != 0
            {
                return self.declared_copy_plan(class, binding);
            }
            return self.scalar_constructor_facts(class, Some(binding));
        }
        // A user-provided declaration alone has no subobject plan to demand.
        self.constructor_declaration_plan(binding, class)
    }

    pub(super) fn constructor_declaration_plan(
        &mut self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<ConstructorId, Error> {
        let (object_pointer, presentation_type, signature) =
            self.constructor_types(class, Some(binding))?;
        let entity = self.graph.binding(binding).entity;
        self.publish_constructor(Constructor {
            class,
            binding: Some(binding),
            object_pointer,
            presentation_type,
            callable_type: signature,
            nothrow: false,
            deleted: self.graph.entity(entity).availability
                == super::graph::FunctionAvailability::Deleted,
            const_default: true,
            members: Vec::new(),
            written_members: Vec::new(),
            bases: Vec::new(),
            subobjects: Vec::new(),
            delegation: None,
            copy: None,
            copy_transfer: CopyTransfer::Call,
            triviality: Triviality::Trivial,
        })
    }

    pub(super) fn defaulted_constructor_definition(
        &mut self,
        function: EntityId,
        class: EntityId,
        kind: MemberKind,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if kind == MemberKind::Destructor {
            if self.defaulted_destructor_deleted(class)? {
                self.graph.entity_mut(function).availability =
                    super::graph::FunctionAvailability::Deleted;
                return Err(Error::at(
                    location,
                    "an explicitly defaulted function is defined as deleted",
                ));
            }
            return self.prepare_destruction(class);
        }
        if kind != MemberKind::Constructor {
            return Err(Error::Invariant("defaulted conversion function"));
        }
        if self.graph.types.function_parameter_count(
            self.graph
                .entity(function)
                .ty
                .ok_or(Error::Invariant("constructor without type"))?,
        )? != 0
        {
            let binding = self.graph.function_origin(function)?;
            self.declared_copy_kind(binding, class)?
                .ok_or(Error::Invariant("defaulted non-copy constructor"))?;
            self.prepare_copy_properties(class)?;
            self.prepare_declared_copy(binding, class)?;
            let properties = self
                .graph
                .entity(function)
                .copy_properties
                .ok_or(Error::Invariant("defaulted copy properties"))?;
            if self.graph.copy_properties.get(properties).deleted {
                return Err(Error::at(
                    location,
                    "an explicitly defaulted function is defined as deleted",
                ));
            }
            self.declared_copy_plan(class, binding)?;
            return Ok(());
        }
        let binding = self.graph.function_origin(function)?;
        self.prepare_defaulted_subobjects(class)?;
        let id = self.scalar_constructor_facts(class, Some(binding))?;
        if self.graph.constructors.get(id).deleted {
            return Err(Error::at(
                location,
                "an explicitly defaulted function is defined as deleted",
            ));
        }
        Ok(())
    }

    pub(super) fn variable_destruction(
        &mut self,
        binding: BindingId,
        scope: ScopeId,
        location: SourceOffset,
        initialized: bool,
    ) -> Result<(), Error> {
        let mut ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("object without type"))?;
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(());
        };
        if !self.graph.entity(class).bases.is_empty() {
            self.prepare_destruction(class)?;
        }
        let Some(destructor) = self.destructor_callable(class)? else {
            return Ok(());
        };
        self.prepare_destruction(class)?;
        let entity = self.graph.entity(self.graph.binding(destructor).entity);
        if entity.availability == super::graph::FunctionAvailability::Deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.check_direct_member_access(destructor, scope, location)?;
        let nontrivial = !self.graph.destructor_trivial(class)?;
        if nontrivial && self.unevaluated == 0 && self.template_depth == 0 {
            self.demand_instance_member(self.graph.binding(destructor).entity);
        }
        if nontrivial && self.lowering() {
            // Destruction is an output demand at the lifetime site, not an
            // analysis-time emission root. Its body's storage boundaries must
            // nevertheless be ready when that owner is reached by lowering.
            self.graph.storage_destructions.push((
                self.body_context.as_ref().map(|body| body.entity),
                self.graph.binding(destructor).entity,
            ));
        }
        let object = self.graph.entity(self.graph.binding(binding).entity);
        if nontrivial
            && !initialized
            && object
                .variable
                .is_some_and(|properties| properties.automatic)
            && !object
                .constructor
                .is_some_and(|id| self.graph.constructors.get(id).nontrivial())
        {
            self.note_local_initializer(
                object.home.ok_or(Error::Invariant("local without home"))?,
            )?;
        }
        Ok(())
    }

    pub(super) fn class_parameter_destruction(
        &mut self,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(());
        };
        if !self.graph.entity(class).bases.is_empty() {
            self.prepare_destruction(class)?;
        }
        let Some(binding) = self.destructor_callable(class)? else {
            return self.check_abstract_object(ty, location);
        };
        self.prepare_destruction(class)?;
        if self
            .graph
            .entity(self.graph.binding(binding).entity)
            .availability
            == super::graph::FunctionAvailability::Deleted
        {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.check_special_use_access(binding, class, scope, location)?;
        if self.unevaluated == 0
            && self.template_depth == 0
            && !self.graph.destructor_trivial(class)?
        {
            self.demand_instance_member(self.graph.binding(binding).entity);
        }
        self.check_abstract_object(ty, location)
    }

    pub(super) fn written_constructor_plan(
        &mut self,
        function: EntityId,
        body: &ast::FunctionBody,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let binding = self.graph.function_origin(function)?;
        let id = self.declared_constructor_plan(binding)?;
        let class = self.graph.constructors.get(id).class;
        let location = self.tree.statements.get(body.compound).location;
        let owner = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("constructor owner without scope"))?;
        let mut written = std::collections::HashMap::new();
        for syntax_index in 0..self.tree.mem_initializers.get(body.initializers).len() {
            let syntax_item = self.tree.mem_initializers.get(body.initializers)[syntax_index];
            let initializer = &syntax_item;
            let member = self.constructor_member_target(initializer, owner, scope, class)?;
            if member == class {
                if self.tree.mem_initializers.get(body.initializers).len() != 1 {
                    return Err(Error::at(
                        initializer.location,
                        "a delegating mem-initializer is not the only mem-initializer",
                    ));
                }
                return self.delegate_construction(id, initializer, scope);
            }
            if written.insert(member, *initializer).is_some() {
                return Err(Error::at(
                    initializer.location,
                    "a member or base initialized twice",
                ));
            }
        }
        let mut bases = Vec::with_capacity(self.graph.entity(class).bases.len());
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            let initializer = written.get(&base).map(|written| ast::Initializer {
                location: written.location,
                ..written.initializer
            });
            let selected = if let Some(initializer) = initializer {
                let ty = self
                    .graph
                    .entity(base)
                    .ty
                    .ok_or(Error::Invariant("base without type"))?;
                let saved = self.base_object.replace(class);
                let selected = self.initialize_class_object_context(
                    ty,
                    initializer,
                    scope,
                    initializer.location,
                    super::constructor_initialization::ClassContext::Base,
                );
                self.base_object = saved;
                selected?
            } else {
                let id = self.default_constructor(base, location)?;
                let record = self.graph.constructors.get(id);
                if record.deleted {
                    return Err(Error::at(location, "use of a deleted function"));
                }
                if let Some(binding) = record.binding {
                    self.check_special_member_access(binding, class, scope, location)?;
                }
                self.demand_constructor_use(id);
                Some((id, ConstructorArguments::Value))
            };
            bases.push(BaseInitialization {
                class: base,
                constructor: selected.map(|(id, _)| id),
                arguments: selected.and_then(|(_, arguments)| initializer.map(|_| arguments)),
                initializer,
                forwards: false,
            });
        }
        let super::member_construction::MemberActions {
            members: plans,
            written_members,
            subobjects,
        } = self.written_member_plans(class, &written, scope, location)?;
        self.graph.constructors.records[id.0 as usize].members = plans;
        self.graph.constructors.records[id.0 as usize].written_members = written_members;
        self.graph.constructors.records[id.0 as usize].bases = bases;
        self.graph.constructors.records[id.0 as usize].subobjects = subobjects;
        self.check_subobject_destruction(class, scope, location)?;
        self.prepare_destruction(class)?;
        Ok(())
    }

    fn delegate_construction(
        &mut self,
        id: ConstructorId,
        written: &ast::MemInitializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let class = self.graph.constructors.get(id).class;
        let ty = self.graph.entity(class).ty.ok_or(Error::Invariant(
            "delegating constructor without class type",
        ))?;
        let initializer = ast::Initializer {
            location: written.location,
            ..written.initializer
        };
        let (constructor, arguments) = self
            .initialize_class_object_context(
                ty,
                initializer,
                scope,
                written.location,
                super::constructor_initialization::ClassContext::Delegation,
            )?
            .ok_or(Error::Invariant("delegation without selected constructor"))?;
        let mut next = Some(constructor);
        while let Some(target) = next {
            if target == id {
                return Err(Error::at(
                    written.location,
                    "a constructor delegates to itself",
                ));
            }
            next = self
                .graph
                .constructors
                .get(target)
                .delegation
                .map(|edge| edge.constructor);
        }
        self.graph.constructors.records[id.0 as usize].delegation = Some(Delegation {
            constructor,
            arguments,
            initializer,
        });
        self.prepare_destruction(class)
    }

    pub(super) fn written_member_construction(
        &mut self,
        ty: TypeId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<Option<(ConstructorId, ConstructorArguments)>, Error> {
        if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
            return self.initialize_class_object(ty, initializer, scope, initializer.location);
        }
        self.object_initializer(initializer, scope, ty, initializer.location)?;
        if !matches!(initializer.kind, ast::InitializerKind::Parenthesized(values) if self.tree.expr_lists.get(values).is_empty())
        {
            return Ok(None);
        }
        let mut element = ty;
        while let TypeKind::Array { element: inner, .. } = self.graph.types.get(element).kind {
            element = inner;
        }
        let TypeKind::Class(child) = self.graph.types.get(element).kind else {
            return Ok(None);
        };
        let constructor = self.default_constructor(child, initializer.location)?;
        self.validate_constructor_use(constructor, scope, initializer.location)?;
        Ok(Some((constructor, ConstructorArguments::Value)))
    }

    fn constructor_member_target(
        &mut self,
        initializer: &ast::MemInitializer,
        owner: ScopeId,
        scope: ScopeId,
        class: EntityId,
    ) -> Result<EntityId, Error> {
        let name = self.tree.names.get(initializer.name);
        let parts = self.tree.name_lists.get(name.components);
        let own = if !name.global && parts.len() == 1 {
            if let ast::NameKind::Identifier(key) = parts[0].kind {
                self.graph.own(owner, key, super::lookup::Filter::Ordinary)
            } else {
                None
            }
        } else {
            None
        };
        let binding = if let Some(binding) = own {
            Some(binding)
        } else {
            self.lookup_result(initializer.name, scope, super::lookup::Filter::Ordinary)?
                .binding
        }
        .ok_or_else(|| Error::at(initializer.location, "the mem-initializer names nothing"))?;
        let node = self.graph.binding(binding);
        let member = self.graph.entity(node.entity);
        if node.kind == super::graph::BindingKind::Variable {
            if member.data_member == Some(super::graph::DataMemberKind::Instance)
                && member.home == Some(owner)
            {
                return Ok(node.entity);
            }
            if member.data_member == Some(super::graph::DataMemberKind::Instance)
                && self.graph.member_holder(node.entity, Some(class))? == class
            {
                return Ok(node.entity);
            }
            return Err(Error::at(
                initializer.location,
                "the mem-initializer does not name a non-static data member of the class",
            ));
        }
        if !matches!(
            node.kind,
            super::graph::BindingKind::Type | super::graph::BindingKind::Alias
        ) {
            return Err(Error::at(
                initializer.location,
                "the mem-initializer does not name a member or base",
            ));
        }
        self.check_direct_member_access(binding, scope, initializer.location)?;
        if node.ty.is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Class(entity) if entity == class)) {
            return Ok(class);
        }
        if let Some(base) = node.ty.and_then(|ty| self.graph.class_of(ty))
            && self
                .graph
                .entity(class)
                .bases
                .iter()
                .any(|edge| edge.class == base)
        {
            return Ok(base);
        }
        Err(Error::at(
            initializer.location,
            "the mem-initializer does not name a direct or virtual base",
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn recursive_construction_keeps_child_selection_and_original_arguments() {
        use crate::sema::{
            construction::ConstructorState, constructor_initialization::ConstructorArguments,
        };
        use crate::syntax::ast::{ExprKind, InitializerKind};
        let source = SourceBuffer::new(b"struct Leaf{int n;Leaf(int x):n(x){}~Leaf(){}};struct Mid{Leaf a{7};};struct Outer{Mid m;};void f(){Outer x;}struct Unused{Leaf a;};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"recursive-construction").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(expressions, tree.expressions.mark());
        let mut class = |spelling: &[u8]| {
            let name = cursor.intern_name(spelling).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity
        };
        let mid = class(b"Mid");
        let outer = class(b"Outer");
        let ConstructorState::Ready(outer_id) = graph.entity(outer).default_construction else {
            panic!("outer constructor was not prepared");
        };
        let ConstructorState::Ready(mid_id) = graph.entity(mid).default_construction else {
            panic!("child constructor was not prepared");
        };
        let outer_plan = graph.constructors.get(outer_id);
        assert!(outer_plan.nontrivial());
        assert_eq!(outer_plan.subobjects.len(), 1);
        let child = outer_plan.subobjects[0];
        assert_eq!(child.member, graph.entity(outer).members[0]);
        assert_eq!(child.constructor, mid_id);
        assert!(child.arguments.is_none());
        let mid_plan = graph.constructors.get(mid_id);
        let child = mid_plan.subobjects[0];
        assert_eq!(child.member, graph.entity(mid).members[0]);
        assert_eq!(
            Some(child.constructor),
            graph.entity(child.member).constructor
        );
        let InitializerKind::Braced(list) =
            graph.entity(child.member).member_initializer.unwrap().kind
        else {
            panic!("member initializer shape changed");
        };
        let ExprKind::Braced { elements, .. } = tree.expressions.get(list).kind else {
            panic!("member initializer lost original list");
        };
        assert!(
            matches!(child.arguments, Some(ConstructorArguments::Values(values)) if std::ptr::eq(tree.expr_lists.get(values), tree.expr_lists.get(elements)))
        );
        assert!(matches!(
            graph.entity(class(b"Unused")).default_construction,
            ConstructorState::NotStarted
        ));
    }
    #[test]
    fn array_constructor_boundaries_prepare_only_demanded_parameter_facts() {
        let source = SourceBuffer::new(b"struct P{int n;P(int);};struct Q{Q(P=P(7));};struct R{Q a[2];};void f(){R r;}struct U{};struct Unused{Unused(U=U());};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constructor-boundaries").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let p = cursor.intern_name(b"P").unwrap();
        let p = graph
            .binding(graph.own(graph.global, p, Filter::Prefix).unwrap())
            .entity;
        assert!(graph.lower_direct_class_parameter(p).unwrap());
        assert_eq!(graph.lower_class_layout(p).unwrap(), (4, 2));
        let u = cursor.intern_name(b"U").unwrap();
        let u = graph
            .binding(graph.own(graph.global, u, Filter::Prefix).unwrap())
            .entity;
        assert!(graph.lower_direct_class_parameter(u).is_err());
        assert!(matches!(
            graph.entity(u).layout,
            crate::sema::layout::LayoutState::NotStarted
        ));
    }
}
