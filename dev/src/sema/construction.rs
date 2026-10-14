// Translated from sema/analyzer_initialization.cpp and analyzer_construction.cpp. See NOTICE.
//! Demand-only default construction and source-ordered scalar subobject plans.
//! Records are unit-owned; declarations and expressions retain compact identities.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, EntityId, MemberKind, ScopeId},
    special_members::SpecialDefinition,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

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
pub(super) struct Constructor {
    pub class: EntityId,
    pub binding: Option<BindingId>,
    pub object_pointer: TypeId,
    // The dump's function type includes its implicit object parameter.
    pub presentation_type: TypeId,
    pub callable_type: TypeId,
    pub nothrow: bool,
    pub(super) deleted: bool,
    const_default: bool,
    pub members: Vec<(EntityId, Option<ast::Initializer>)>,
    pub bases: Vec<ConstructorId>,
    pub subobjects: Vec<(EntityId, ConstructorId)>,
}
impl Constructor {
    fn initialized_members(&self) -> usize {
        self.members
            .iter()
            .filter(|(_, source)| source.is_some())
            .count()
    }
    pub(super) fn nontrivial(&self, graph: &super::graph::Graph) -> bool {
        self.own_action(graph)
            || self
                .bases
                .iter()
                .any(|&base| graph.constructors.get(base).own_action(graph))
            || self
                .subobjects
                .iter()
                .any(|&(_, id)| graph.constructors.get(id).own_action(graph))
    }
    fn own_action(&self, graph: &super::graph::Graph) -> bool {
        self.initialized_members() != 0
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
    fn add(&mut self, constructor: Constructor) -> Result<ConstructorId, Error> {
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
    pub(super) fn default_initialize_declaration(
        &mut self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.default_initialize(binding, location)?;
        let variable = self.graph.entity(self.graph.binding(binding).entity);
        // declare_variable counts nontrivial automatic construction after
        // selecting it. anonymous_union's hidden-object path does not count it.
        if variable
            .variable
            .is_some_and(|properties| properties.automatic)
            && variable
                .constructor
                .is_some_and(|id| self.graph.constructors.get(id).nontrivial(&self.graph))
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
                self.check_direct_member_access(
                    selected,
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

    fn default_member_dependency(&self, member: EntityId) -> Result<Option<EntityId>, Error> {
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
            let id = self.declared_constructor_plan(binding)?;
            self.graph.entity_mut(class).default_construction = ConstructorState::Ready(id);
            return Ok(id);
        }
        if !self.graph.entity(class).complete {
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

    fn default_constructor_binding(
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

    fn scalar_constructor_facts(
        &mut self,
        class: EntityId,
        binding: Option<BindingId>,
    ) -> Result<ConstructorId, Error> {
        let union = self.graph.entity(class).key == Keyword::Union;
        let mut deleted = false;
        let mut bases_const_default = true;
        let mut bases = Vec::with_capacity(self.graph.entity(class).bases.len());
        for edge in &self.graph.entity(class).bases {
            if matches!(
                self.graph.entity(edge.class).layout,
                super::layout::LayoutState::Unsupported
            ) {
                return Err(Error::Missing("extended base subobject constructor plans"));
            }
            let ConstructorState::Ready(base) = self.graph.entity(edge.class).default_construction
            else {
                return Err(Error::Missing("defaulted base subobject constructor plans"));
            };
            bases.push(base);
            let base = self.graph.constructors.get(base);
            if base.nontrivial(&self.graph) || base.binding.is_some() {
                return Err(Error::Missing(
                    "nontrivial base subobject constructor actions",
                ));
            }
            // implicit_default_facts plans its bases with location zero;
            // default_constructor checks deletion at that plan's location.
            if base.deleted {
                return Err(Error::at(SourceOffset(0), "use of a deleted function"));
            }
            bases_const_default &= base.const_default;
        }
        let mut plans = Vec::with_capacity(self.graph.entity(class).members.len());
        let mut subobjects = Vec::new();
        let mut const_members = 0;
        for index in 0..self.graph.entity(class).members.len() {
            let member_id = self.graph.entity(class).members[index];
            let member = self.graph.entity(member_id);
            let mut ty = member
                .ty
                .ok_or(Error::Invariant("constructor member without type"))?;
            if member.member_initializer.is_some() {
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
                if child.nontrivial(&self.graph) {
                    return Err(Error::Missing("class subobject default construction"));
                }
                if child.deleted {
                    return Err(Error::at(SourceOffset(0), "use of a deleted function"));
                }
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
                    return Err(Error::Missing("const class subobject default construction"));
                }
                subobjects.push((member_id, id));
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
        let presentation_type = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters: vec![object_pointer],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        if let Some(binding) = binding {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            deleted |= entity.availability == super::graph::FunctionAvailability::Deleted;
        }
        let callable_type = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters: Vec::new(),
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let mut nothrow = true;
        for &id in bases.iter().chain(subobjects.iter().map(|(_, id)| id)) {
            let child = self.graph.constructors.get(id);
            if let Some(binding) = child.binding
                && self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .constructor_exceptions
                    == ConstructorExceptionSource::Declared
            {
                // Even a trivial subobject constructor contributes its declared
                // specification to the enclosing implicit/defaulted plan.
                nothrow &= !self.function_may_throw(self.graph.binding(binding).entity)?;
            } else {
                nothrow &= child.nothrow;
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
            members: plans,
            bases,
            subobjects,
        };
        self.publish_constructor(record)
    }

    fn publish_constructor(&mut self, record: Constructor) -> Result<ConstructorId, Error> {
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
                return Err(Error::Missing("defaulted copy or move constructor plans"));
            }
            return self.scalar_constructor_facts(class, Some(binding));
        }
        // A user-provided declaration alone has no subobject plan to demand.
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("constructor owner without type"))?;
        let object_pointer = self.graph.types.pointer(ty)?;
        let signature = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("constructor without signature"))?;
        let TypeKind::Function {
            result,
            ref parameters,
            variadic,
            ..
        } = self.graph.types.get(signature).kind
        else {
            return Err(Error::Invariant("constructor signature is not a function"));
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
            bases: Vec::new(),
            subobjects: Vec::new(),
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
            return self.require_scalar_destruction(class);
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
            let kind = self
                .declared_copy_kind(binding, class)?
                .ok_or(Error::Invariant("defaulted non-copy constructor"))?;
            if self.scalar_copy_deleted(class, kind)? {
                return Err(Error::at(
                    location,
                    "an explicitly defaulted function is defined as deleted",
                ));
            }
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

    pub(super) fn require_scalar_destruction(&mut self, class: EntityId) -> Result<(), Error> {
        let mut work = vec![class];
        let mut seen = std::collections::HashSet::new();
        while let Some(current) = work.pop() {
            if !seen.insert(current) {
                continue;
            }
            if current != class && self.scalar_destructor_binding(current)?.is_some() {
                return Err(Error::Missing("base subobject destructor plans"));
            }
            work.extend(
                self.graph
                    .entity(current)
                    .bases
                    .iter()
                    .map(|edge| edge.class),
            );
            self.require_scalar_members_destruction(current)?;
        }
        Ok(())
    }

    fn require_scalar_members_destruction(&self, class: EntityId) -> Result<(), Error> {
        for &member in &self.graph.entity(class).members {
            let mut ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                return Err(Error::Missing("class subobject destruction plans"));
            }
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
            self.require_scalar_destruction(class)?;
        }
        let Some(destructor) = self.scalar_destructor_binding(class)? else {
            return Ok(());
        };
        self.require_scalar_destruction(class)?;
        let entity = self.graph.entity(self.graph.binding(destructor).entity);
        if entity.availability == super::graph::FunctionAvailability::Deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.check_direct_member_access(destructor, scope, location)?;
        let nontrivial = !matches!(
            entity.special_definition,
            Some(SpecialDefinition::Defaulted { first: true, .. })
        );
        let object = self.graph.entity(self.graph.binding(binding).entity);
        if nontrivial
            && !initialized
            && object
                .variable
                .is_some_and(|properties| properties.automatic)
            && !object
                .constructor
                .is_some_and(|id| self.graph.constructors.get(id).nontrivial(&self.graph))
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
            self.require_scalar_destruction(class)?;
        }
        let Some(binding) = self.scalar_destructor_binding(class)? else {
            return Ok(());
        };
        self.require_scalar_destruction(class)?;
        if self
            .graph
            .entity(self.graph.binding(binding).entity)
            .availability
            == super::graph::FunctionAvailability::Deleted
        {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.check_direct_member_access(binding, scope, location)
    }

    pub(super) fn scalar_destructor_binding(
        &mut self,
        class: EntityId,
    ) -> Result<Option<BindingId>, Error> {
        let void = self.graph.types.scalar(Scalar::Void)?;
        let signature = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters: Vec::new(),
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        Ok(self.graph.member_function(super::graph::MemberKey {
            class,
            kind: MemberKind::Destructor,
            signature,
            shape: None,
        }))
    }

    pub(super) fn complete_scalar_destructor_exceptions(
        &mut self,
        class: EntityId,
    ) -> Result<(), Error> {
        let Some(binding) = self.scalar_destructor_binding(class)? else {
            return Ok(());
        };
        let entity = self.graph.binding(binding).entity;
        if self.graph.entity(entity).destructor_specification
            == super::exceptions::DestructorSpecification::Implicit
        {
            self.require_scalar_destruction(class)?;
            self.graph.entity_mut(entity).exceptions =
                super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
        }
        Ok(())
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
        if !self.graph.entity(class).bases.is_empty() {
            return Err(Error::Missing("written base subobject constructor plans"));
        }
        let location = self.tree.statements.get(body.compound).location;
        let owner = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("constructor owner without scope"))?;
        let mut written = std::collections::HashMap::new();
        for initializer in self.tree.mem_initializers.get(body.initializers) {
            if initializer.pack {
                return Err(Error::Missing("constructor initializer pack expansion"));
            }
            let member = self.constructor_member_target(initializer, owner, scope, class)?;
            if written.insert(member, initializer).is_some() {
                return Err(Error::at(
                    initializer.location,
                    "a member or base initialized twice",
                ));
            }
        }
        if self.graph.entity(class).key == Keyword::Union && written.len() > 1 {
            let second = self
                .graph
                .entity(class)
                .members
                .iter()
                .filter_map(|member| written.get(member))
                .nth(1)
                .ok_or(Error::Invariant("union initializer without member"))?
                .location;
            return Err(Error::at(
                second,
                "a union's constructor initializes several members",
            ));
        }
        let mut plans = Vec::with_capacity(self.graph.entity(class).members.len());
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            let ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            let variant = self.graph.entity(class).key == Keyword::Union;
            let source = written
                .get(&member)
                .map(|written| written.initializer)
                .or_else(|| {
                    if variant && !written.is_empty() {
                        None
                    } else {
                        self.graph.entity(member).member_initializer
                    }
                });
            if let Some(written) = written.get(&member) {
                let initializer = written.initializer;
                self.object_initializer(initializer, scope, ty, initializer.location)?;
            } else if source.is_none() && !variant {
                let mut object = ty;
                while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
                    object = element;
                }
                if matches!(self.graph.types.get(object).kind, TypeKind::Class(_)) {
                    return Err(Error::Missing("class subobject constructor plans"));
                }
                if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                    return Err(Error::at(location, "a reference member is not initialized"));
                }
                if self.graph.types.get(object).cv.0 & 1 != 0 {
                    return Err(Error::at(location, "a const member is not initialized"));
                }
            }
            plans.push((member, source));
        }
        self.graph.constructors.records[id.0 as usize].members = plans;
        Ok(())
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
            if member
                .home
                .and_then(|home| self.graph.scope(home).owner)
                .is_some_and(|owner| self.graph.entity(owner).anonymous_storage.is_some())
            {
                return Err(Error::Missing("anonymous subobject constructor plans"));
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
            return Err(Error::Missing("delegating constructor plans"));
        }
        Err(Error::at(
            initializer.location,
            "the mem-initializer does not name a direct or virtual base",
        ))
    }
}
