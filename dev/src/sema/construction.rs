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
use crate::{support::source::SourceOffset, syntax::ast};

#[derive(Clone, Copy)]
pub struct BaseInitialization {
    pub class: EntityId,
    pub constructor: Option<ConstructorId>,
    pub arguments: Option<ConstructorArguments>,
    pub initializer: Option<ast::Initializer>,
    pub forwards: bool,
}

enum InitializerLookup {
    Class,
    Unqualified,
    General,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ConstructorExceptionSource {
    #[default]
    Declared,
    ComputedDefault {
        constructor: ConstructorId,
        pending: bool,
    },
    InheritedPlan {
        constructor: ConstructorId,
        pending: bool,
    },
    SubobjectPlan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConstructorId(u32);
#[derive(Clone, Copy, Default)]
pub(super) enum ConstructorState {
    #[default]
    NotStarted,
    // Copy-body planning retains its independent in-progress state.
    InProgress,
    Published(ConstructorId),
    Ready(ConstructorId),
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Triviality {
    #[default]
    Trivial,
    Nontrivial,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConstructorFacts {
    Declaration,
    PlanPublished,
    Complete,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CopyTransfer {
    Call,
    Storage,
}
#[derive(Clone, Copy)]
pub struct MemberConstruction {
    pub member: EntityId,
    pub constructor: ConstructorId,
    pub arguments: Option<ConstructorArguments>,
}
#[derive(Clone, Copy)]
pub struct Delegation {
    pub constructor: ConstructorId,
    pub arguments: ConstructorArguments,
    pub initializer: ast::Initializer,
}
pub(super) struct Constructor {
    pub facts: ConstructorFacts,
    pub class: EntityId,
    pub binding: Option<BindingId>,
    pub declaration: Option<EntityId>,
    pub object_pointer: Option<TypeId>,
    pub callable_type: Option<TypeId>,
    pub nothrow: bool,
    pub(super) deleted: bool,
    pub members: Vec<(EntityId, Option<ast::Initializer>)>,
    pub written_members: Vec<bool>,
    pub bases: Vec<BaseInitialization>,
    pub indirect_bases: Vec<BaseInitialization>,
    pub subobjects: Vec<Option<MemberConstruction>>,
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
        graph.entity(self.class).class().polymorphism.is_dynamic()
            || graph
                .entity(self.class)
                .class()
                .bases
                .iter()
                .any(|edge| edge.is_virtual)
            || self.initialized_members() != 0
            || self.binding.is_some_and(|binding| {
                !matches!(
                    graph
                        .entity(graph.binding(binding).entity)
                        .callable()
                        .special_definition,
                    Some(SpecialDefinition::Defaulted { first: true, .. })
                )
            })
    }
}
#[derive(Default)]
pub(super) struct Constructors {
    records: Vec<Constructor>,
    // The class retains one selected function, declared or implicit, before
    // its structural/default facts. Absence is never a cached failure.
    default_functions: crate::support::hash::HashMap<EntityId, DefaultFunction>,
    const_default_classes: crate::support::hash::HashSet<EntityId>,
}
#[derive(Clone, Copy)]
pub(super) struct DefaultFunction {
    pub function: EntityId,
    pub declared: Option<BindingId>,
}
impl Constructors {
    pub(super) fn default_function(&self, class: EntityId) -> Option<DefaultFunction> {
        self.default_functions.get(&class).copied()
    }

    pub(super) fn const_default(&self, class: EntityId) -> bool {
        self.const_default_classes.contains(&class)
    }
    pub(super) fn record_const_default(&mut self, class: EntityId) {
        self.const_default_classes.insert(class);
    }

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
    fn replace(&mut self, id: ConstructorId, mut record: Constructor) {
        record.object_pointer = record
            .object_pointer
            .or(self.records[id.0 as usize].object_pointer);
        self.records[id.0 as usize] = record;
    }
    pub(super) fn record_object_pointer(&mut self, id: ConstructorId, pointer: TypeId) {
        self.records[id.0 as usize].object_pointer = Some(pointer);
    }
    pub(super) fn set_nothrow(&mut self, id: ConstructorId, nothrow: bool) {
        self.records[id.0 as usize].nothrow = nothrow;
    }
    pub(super) fn retain_default_function(&mut self, class: EntityId, slot: DefaultFunction) {
        self.default_functions.insert(class, slot);
    }
    pub(super) fn install_default_entries(
        &mut self,
        id: ConstructorId,
        members: Vec<(EntityId, Option<ast::Initializer>)>,
        bases: Vec<BaseInitialization>,
        indirect_bases: Vec<BaseInitialization>,
    ) {
        let record = &mut self.records[id.0 as usize];
        record.written_members = vec![false; members.len()];
        record.subobjects = vec![None; members.len()];
        record.members = members;
        record.bases = bases;
        record.indirect_bases = indirect_bases;
    }
    pub(super) fn publish_default_entries(&mut self, id: ConstructorId) {
        self.records[id.0 as usize].facts = ConstructorFacts::PlanPublished;
    }
    pub(super) fn set_forwarded_base(
        &mut self,
        id: ConstructorId,
        index: usize,
        indirect: bool,
        constructor: ConstructorId,
    ) {
        let record = &mut self.records[id.0 as usize];
        let entry = if indirect {
            &mut record.indirect_bases[index]
        } else {
            &mut record.bases[index]
        };
        entry.constructor = Some(constructor);
        entry.forwards = true;
    }
    pub(super) fn set_default_member(
        &mut self,
        id: ConstructorId,
        index: usize,
        initializer: Option<ast::Initializer>,
        action: Option<MemberConstruction>,
    ) -> Result<(), Error> {
        let record = &mut self.records[id.0 as usize];
        if action.is_some_and(|action| action.member != record.members[index].0) {
            return Err(Error::Invariant("default member entry identity"));
        }
        record.members[index].1 = initializer;
        record.subobjects[index] = action;
        Ok(())
    }
    pub(super) fn set_default_base(
        &mut self,
        id: ConstructorId,
        index: usize,
        indirect: bool,
        constructor: Option<ConstructorId>,
    ) {
        let record = &mut self.records[id.0 as usize];
        let entries = if indirect {
            &mut record.indirect_bases
        } else {
            &mut record.bases
        };
        entries[index].constructor = constructor;
    }
    pub(super) fn mark_default_deleted(&mut self, id: ConstructorId) {
        self.records[id.0 as usize].deleted = true;
    }
    pub(super) fn set_default_triviality(&mut self, id: ConstructorId, trivial: bool) {
        self.records[id.0 as usize].triviality = if trivial {
            Triviality::Trivial
        } else {
            Triviality::Nontrivial
        };
    }
    pub(super) fn set_default_transfer(&mut self, id: ConstructorId, storage: bool) {
        self.records[id.0 as usize].copy_transfer = if storage {
            CopyTransfer::Storage
        } else {
            CopyTransfer::Call
        };
    }
    pub(super) fn finish_default_facts(
        &mut self,
        id: ConstructorId,
        trivial: bool,
        nothrow: bool,
        deleted: bool,
    ) {
        let record = &mut self.records[id.0 as usize];
        record.triviality = if trivial {
            Triviality::Trivial
        } else {
            Triviality::Nontrivial
        };
        record.nothrow = nothrow;
        record.deleted |= deleted;
        record.facts = ConstructorFacts::Complete;
    }
    pub(super) fn patch_default_member(&mut self, id: ConstructorId, action: MemberConstruction) {
        let record = &mut self.records[id.0 as usize];
        let Some(index) = record
            .members
            .iter()
            .position(|&(member, source)| member == action.member && source.is_some())
        else {
            return;
        };
        if record.written_members.get(index) == Some(&true) {
            return;
        }
        record.subobjects[index] = Some(action);
    }
}
/// Translate completed source-ordered actions into their member entry slots.
/// Missing actions remain null entries; no member shifts when a late action is set.
pub(super) fn aligned_member_actions(
    members: &[(EntityId, Option<ast::Initializer>)],
    actions: Vec<MemberConstruction>,
) -> Result<Vec<Option<MemberConstruction>>, Error> {
    let mut actions = actions.into_iter().peekable();
    let mut slots = Vec::with_capacity(members.len());
    for &(member, _) in members {
        slots.push(
            if actions.peek().is_some_and(|action| action.member == member) {
                actions.next()
            } else {
                None
            },
        );
    }
    if actions.next().is_some() {
        return Err(Error::Invariant("member construction order"));
    }
    Ok(slots)
}

impl Analyzer<'_> {
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
            .object()
            .variable
            .is_some_and(|properties| properties.automatic)
            && variable
                .initialization()
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
        if self.graph.entity(variable).object().data_member
            == Some(super::graph::DataMemberKind::Static)
            && (self.graph.entity(variable).constexpr
                || matches!(
                    self.graph.entity(variable).object().constant,
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
            self.validate_constructor_use(
                constructor,
                class,
                self.graph.binding_scope(binding),
                location,
            )?;
            self.validate_const_default(object, constructor, location)?;
            self.graph
                .entity_mut(variable)
                .initialization_mut()
                .constructor = Some(constructor);
            if self.lowering()
                && self.unevaluated == 0
                && self.graph.entity(variable).object().data_member
                    != Some(super::graph::DataMemberKind::Instance)
            {
                self.graph.constructor_boundaries.push((
                    self.body_context.as_ref().and_then(|body| body.function),
                    constructor,
                ));
            }
            // An array shows one element action without demanding a definition.
            if !matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. })
                && self.unevaluated == 0
            {
                if let Some(selected) = self.graph.constructors.get(constructor).binding {
                    self.demand_specialization_body(selected);
                    self.demand_function(selected, true)?;
                } else {
                    self.record_demand(super::function_demands::Demand::Constructor(constructor));
                }
            }
        } else if self.graph.types.get(object).cv.0 & 1 != 0
            && !self
                .graph
                .entity(variable)
                .object()
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
        let Some(slot) = self.implicit_constructor_function(class)? else {
            return self.reject_substitution(
                location,
                "no default constructor for the class",
                false,
            );
        };
        let id = self
            .graph
            .entity(slot.function)
            .initialization()
            .constructor
            .ok_or(Error::Invariant(
                "default function without constructor record",
            ))?;
        // Omitted runtime arguments are requested by the actual initialization
        // owner. The function-only source operation never enters this hook.
        if let Some(binding) = slot.declared {
            self.request_constructor_defaults(binding, 0)?;
        }
        Ok(id)
    }
    pub(super) fn default_constructor_binding(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        self.declared_default_constructor(class)?
            .ok_or_else(|| Error::at(location, "no default constructor for the class"))
    }

    pub(super) fn declared_default_constructor(
        &mut self,
        class: EntityId,
    ) -> Result<Option<BindingId>, Error> {
        Ok(self
            .implicit_constructor_function(class)?
            .and_then(|slot| slot.declared))
    }

    pub(super) fn find_default_constructor(
        &mut self,
        class: EntityId,
    ) -> Result<super::default_construction::Lookup, Error> {
        let mut selected = None;
        for index in (0..self.graph.entity(class).class().declared_constructors.len()).rev() {
            let binding = self.graph.entity(class).class().declared_constructors[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            let count = self.graph.types.function_parameter_count(
                entity
                    .ty
                    .ok_or(Error::Invariant("constructor without type"))?,
            )?;
            if (0..count).any(|index| {
                entity
                    .callable()
                    .defaults
                    .get(index)
                    .is_none_or(Option::is_none)
            }) || entity.context().template_head.is_some()
            {
                continue;
            }
            if selected.replace(binding).is_some() {
                self.emit_substitution_failure(
                    SourceOffset(0),
                    "ambiguous default constructor",
                    false,
                )?;
                return Ok(super::default_construction::Lookup::Ambiguous);
            }
        }
        if let Some(binding) = selected {
            return Ok(super::default_construction::Lookup::Selected(binding));
        }
        // The frozen default-constructor query tries templates only after all
        // ordinary candidates. Empty calls do not partially order templates.
        for index in (0..self.graph.entity(class).class().declared_constructors.len()).rev() {
            let binding = self.graph.entity(class).class().declared_constructors[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            let Some(head) = entity.context().template_head else {
                continue;
            };
            let ty = entity
                .ty
                .ok_or(Error::Invariant("constructor template without type"))?;
            let count = self.graph.types.function_parameter_count(ty)?;
            let mut matches = true;
            for index in 0..count {
                let default = entity
                    .callable()
                    .defaults
                    .get(index)
                    .is_some_and(Option::is_some);
                let expansion = index + 1 == count
                    && matches!(
                        self.graph
                            .types
                            .get(self.graph.types.function_parameter(ty, index)?)
                            .kind,
                        TypeKind::Expansion(_)
                    );
                if !(default || expansion) {
                    matches = false;
                    break;
                }
            }
            if !matches {
                continue;
            }
            // Source deduced_specialization receives fresh undeduced slots;
            // this is signature completion, not deduction for a supplied call.
            let mut arguments = vec![None; self.graph.templates.parameters(head).len()];
            let Some(binding) = self.complete_signature(binding, &mut arguments)? else {
                continue;
            };
            if selected.replace(binding).is_some() {
                self.emit_substitution_failure(
                    SourceOffset(0),
                    "ambiguous default constructor",
                    false,
                )?;
                return Ok(super::default_construction::Lookup::Ambiguous);
            }
        }
        // analyzer_initialization.cpp::implicit_constructor stores the selected
        // base specialization itself. Only select_constructor creates an
        // inherited forwarding specialization for a call with supplied args.
        Ok(selected.map_or(
            super::default_construction::Lookup::Absent,
            super::default_construction::Lookup::Selected,
        ))
    }

    pub(super) fn indirect_constructor_bases(&self, class: EntityId) -> Vec<EntityId> {
        if !self.graph.lower_dynamic_class(class) {
            return Vec::new();
        }
        let direct: crate::support::hash::HashSet<_> = self
            .graph
            .entity(class)
            .class()
            .bases
            .iter()
            .filter(|edge| edge.is_virtual)
            .map(|edge| edge.class)
            .collect();
        self.graph
            .virtual_base_order(class)
            .into_iter()
            .filter(|base| !direct.contains(base))
            .collect()
    }

    pub(super) fn inherited_plan_nothrow(&mut self, id: ConstructorId) -> Result<bool, Error> {
        let mut nothrow = true;
        let binding = self.graph.constructors.get(id).binding;
        for index in 0..self.graph.constructors.get(id).bases.len() {
            let entry = self.graph.constructors.get(id).bases[index];
            nothrow = nothrow && self.planned_base_nothrow(entry, binding)?;
        }
        if nothrow {
            nothrow = self.inner_default_nothrow(id)?;
        }
        for index in 0..self.graph.constructors.get(id).indirect_bases.len() {
            let entry = self.graph.constructors.get(id).indirect_bases[index];
            nothrow = nothrow && self.planned_base_nothrow(entry, binding)?;
        }
        Ok(nothrow)
    }

    fn planned_base_nothrow(
        &mut self,
        base: BaseInitialization,
        binding: Option<BindingId>,
    ) -> Result<bool, Error> {
        let Some(id) = base.constructor else {
            return Ok(true);
        };
        let mut nothrow = self.subobject_constructor_nothrow(id)?;
        if !base.forwards || !nothrow {
            return Ok(nothrow);
        }
        let parent = binding.ok_or(Error::Invariant("forwarded constructor without binding"))?;
        let source = self
            .graph
            .constructors
            .get(id)
            .declaration
            .ok_or(Error::Invariant(
                "forwarded constructor without declaration",
            ))?;
        let count = self.graph.types.function_parameter_count(
            self.graph
                .binding(parent)
                .ty
                .ok_or(Error::Invariant("inheriting constructor without type"))?,
        )?;
        let total = self
            .graph
            .types
            .function_parameter_count(self.graph.lower_entity_type(source)?)?;
        for index in count..total {
            if !nothrow {
                break;
            }
            if let Some(default) = self
                .graph
                .entity(source)
                .callable()
                .defaults
                .get(index)
                .copied()
                .flatten()
            {
                nothrow = !self.expression_may_throw(default.expression)?;
            }
        }
        Ok(nothrow)
    }

    pub(super) fn subobject_constructor_nothrow(
        &mut self,
        id: ConstructorId,
    ) -> Result<bool, Error> {
        let child = self.graph.constructors.get(id);
        if let Some(function) = child.declaration {
            // A trivial subobject still contributes its declared specification.
            return self.function_may_throw(function).map(|throws| !throws);
        }
        self.inner_default_nothrow(id)
    }

    pub(super) fn implicit_default_declaration(
        &mut self,
        class: EntityId,
    ) -> Result<EntityId, Error> {
        let function = if let Some(slot) = self.graph.constructors.default_function(class) {
            slot.function
        } else {
            let signature = self.constructor_signature(None)?;
            let function = self.graph.add_implicit_constructor(class, signature)?;
            self.graph.entity_mut(function).callable_mut().exceptions =
                super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::All);
            self.graph.constructors.default_functions.insert(
                class,
                DefaultFunction {
                    function,
                    declared: None,
                },
            );
            function
        };
        Ok(function)
    }
    pub(super) fn constructor_signature(
        &mut self,
        binding: Option<BindingId>,
    ) -> Result<TypeId, Error> {
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
        Ok(callable_type)
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
                .chain(&record.indirect_bases)
                .filter_map(|base| base.constructor)
                .chain(
                    record
                        .subobjects
                        .iter()
                        .flatten()
                        .map(|child| child.constructor),
                )
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
        let function = record.declaration;
        if let Some(id) =
            function.and_then(|entity| self.graph.entity(entity).initialization().constructor)
        {
            self.graph.constructors.replace(id, record);
            return Ok(id);
        }
        let id = self.graph.constructors.add(record)?;
        if let Some(entity) = function {
            self.graph
                .entity_mut(entity)
                .initialization_mut()
                .constructor = Some(id);
        }
        Ok(id)
    }

    pub(super) fn declared_constructor_plan(
        &mut self,
        binding: BindingId,
    ) -> Result<ConstructorId, Error> {
        let entity = self.graph.binding(binding).entity;
        if let Some(id) = self.graph.entity(entity).initialization().constructor {
            return Ok(id);
        }
        let class = self
            .graph
            .entity(entity)
            .callable()
            .member_function
            .ok_or(Error::Invariant("constructor without owner"))?
            .class;
        if self.graph.inherited_constructors.source(entity).is_some() {
            return self.inherited_constructor_plan(binding);
        }
        if self.graph.entity(entity).callable().copy_origin
            == super::copy_construction::Origin::Implicit
        {
            let kind = self
                .graph
                .entity(entity)
                .callable()
                .copy_kind
                .ok_or(Error::Invariant("implicit constructor without copy kind"))?;
            return self.build_implicit_copy_plan(class, kind);
        }
        if matches!(
            self.graph.entity(entity).callable().special_definition,
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
            // Naming the declaration, including as an inherited source, does
            // not define its defaulted body. The default-constructor query and
            // an out-of-class defaulted definition own subobject planning.
            return self.constructor_declaration_plan(binding, class);
        }
        // A user-provided declaration alone has no subobject plan to demand.
        self.constructor_declaration_plan(binding, class)
    }

    pub(super) fn constructor_declaration_plan(
        &mut self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<ConstructorId, Error> {
        let function = self.graph.binding(binding).entity;
        if let Some(id) = self.graph.entity(function).initialization().constructor {
            return Ok(id);
        }
        // A declaration has no owned plan to fold. Retain the function's raw
        // flags, even for a first-defaulted function whose entries are null.
        self.default_header(
            class,
            Some(DefaultFunction {
                function,
                declared: Some(binding),
            }),
        )
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
                .special()
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
        let id = self.implicit_default_plan(function, class)?;
        if self.graph.constructors.get(id).deleted {
            return Err(Error::at(
                location,
                "an explicitly defaulted function is defined as deleted",
            ));
        }
        if matches!(
            self.graph.entity(function).callable().special_definition,
            Some(SpecialDefinition::Defaulted { first: false, .. })
        ) {
            self.graph.constructors.set_default_triviality(id, false);
        }
        Ok(())
    }

    pub(super) fn defaulted_constexpr_constructor_plan(
        &mut self,
        function: EntityId,
    ) -> Result<ConstructorId, Error> {
        let binding = self.graph.function_origin(function)?;
        let entity = self.graph.entity(function);
        let class = entity
            .callable()
            .member_function
            .ok_or(Error::Invariant("defaulted constructor without owner"))?
            .class;
        if self.graph.types.function_parameter_count(
            entity
                .ty
                .ok_or(Error::Invariant("constructor without type"))?,
        )? != 0
        {
            return self.declared_constructor_plan(binding);
        }
        if let Some(id) = entity.initialization().constructor
            && self.graph.constructors.get(id).facts != ConstructorFacts::Declaration
        {
            return Ok(id);
        }
        self.implicit_default_plan(function, class)
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
        self.check_class_destruction(ty, scope, location)?;
        if !self.graph.entity(class).class().bases.is_empty() {
            self.prepare_destruction(class)?;
        }
        let Some(destructor) = self.destructor_callable(class)? else {
            return Ok(());
        };
        self.prepare_destruction(class)?;
        let nontrivial = !self.graph.destructor_trivial(class)?;
        if nontrivial && self.unevaluated == 0 && self.template_depth == 0 {
            self.demand_instance_member(self.graph.binding(destructor).entity);
        }
        if nontrivial && self.lowering() {
            // Destruction is an output demand at the lifetime site, not an
            // analysis-time emission root. Its body's storage boundaries must
            // nevertheless be ready when that owner is reached by lowering.
            self.graph.storage_destructions.push((
                self.body_context.as_ref().and_then(|body| body.function),
                self.graph.binding(destructor).entity,
            ));
        }
        let object = self.graph.entity(self.graph.binding(binding).entity);
        if nontrivial
            && !initialized
            && object
                .object()
                .variable
                .is_some_and(|properties| properties.automatic)
            && !object
                .initialization()
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
        self.check_class_destruction(ty, scope, location)?;
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
            .context()
            .scope
            .ok_or(Error::Invariant("constructor owner without scope"))?;
        let mut written = crate::support::hash::HashMap::default();
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
        let mut deferred = false;
        let mut bases = Vec::with_capacity(self.graph.entity(class).class().bases.len());
        for index in 0..self.graph.entity(class).class().bases.len() {
            let base = self.graph.entity(class).class().bases[index].class;
            bases.push(self.reserved_base_plan(
                class,
                base,
                &written,
                scope,
                location,
                &mut deferred,
            )?);
        }
        let super::member_construction::MemberActions {
            members: plans,
            written_members,
            subobjects,
            deferred: pending,
        } = self.written_member_plans(class, &written, scope, location, deferred)?;
        deferred |= pending;
        let mut indirect_bases = Vec::new();
        for base in self.indirect_constructor_bases(class) {
            indirect_bases.push(self.reserved_base_plan(
                class,
                base,
                &written,
                scope,
                location,
                &mut deferred,
            )?);
        }
        self.graph.constructors.records[id.0 as usize].members = plans;
        self.graph.constructors.records[id.0 as usize].written_members = written_members;
        self.graph.constructors.records[id.0 as usize].bases = bases;
        self.graph.constructors.records[id.0 as usize].indirect_bases = indirect_bases;
        self.graph.constructors.records[id.0 as usize].subobjects = subobjects;
        if deferred {
            return Err(Error::DeferredBuiltin);
        }
        self.check_subobject_destruction(class, scope, location)?;
        self.prepare_destruction(class)?;
        Ok(())
    }

    fn reserved_base_plan(
        &mut self,
        class: EntityId,
        base: EntityId,
        written: &crate::support::hash::HashMap<EntityId, ast::MemInitializer>,
        scope: ScopeId,
        location: SourceOffset,
        deferred: &mut bool,
    ) -> Result<BaseInitialization, Error> {
        match self.written_base_plan(class, base, written, scope, location) {
            Err(Error::DeferredBuiltin) if self.reserved_context.deferring => *deferred = true,
            result => return result,
        }
        let initializer = written.get(&base).map(|written| ast::Initializer {
            location: written.location,
            ..written.initializer
        });
        if let Some(initializer) = initializer {
            self.note_deferred_initializer(initializer);
        }
        Ok(BaseInitialization {
            class: base,
            constructor: None,
            arguments: None,
            initializer,
            forwards: false,
        })
    }

    fn written_base_plan(
        &mut self,
        class: EntityId,
        base: EntityId,
        written: &crate::support::hash::HashMap<EntityId, ast::MemInitializer>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<BaseInitialization, Error> {
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
                self.emit_substitution_failure(location, "use of a deleted function", false)?;
            }
            if let Some(binding) = record.binding {
                self.check_special_use_access(binding, class, scope, location)?;
            }
            self.demand_constructor_use(id)?;
            Some((id, ConstructorArguments::Value))
        };
        Ok(BaseInitialization {
            class: base,
            constructor: selected.map(|(id, _)| id),
            arguments: selected.and_then(|(_, arguments)| initializer.map(|_| arguments)),
            initializer,
            forwards: false,
        })
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
        self.validate_constructor_use(constructor, child, scope, initializer.location)?;
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
        let simple = if !name.global && parts.len() == 1 {
            if let ast::NameKind::Identifier(key) = parts[0].kind
                && parts[0].arguments.is_none()
            {
                Some(key)
            } else {
                None
            }
        } else {
            None
        };
        let (found, lookup) = if let Some(key) = simple {
            let found = self.qualified_declaration_result(
                owner,
                key.into(),
                super::lookup::Filter::Ordinary,
            )?;
            if found.binding.is_some() || found.ambiguous {
                (found, InitializerLookup::Class)
            } else {
                (
                    self.unqualified_identifier_result(
                        scope,
                        key,
                        super::lookup::Filter::Ordinary,
                    )?,
                    InitializerLookup::Unqualified,
                )
            }
        } else {
            (
                self.lookup_result(initializer.name, scope, super::lookup::Filter::Ordinary)?,
                InitializerLookup::General,
            )
        };
        if found.ambiguous {
            return Err(Error::at(
                initializer.location,
                "the mem-initializer name is ambiguous",
            ));
        }
        let binding = found
            .binding
            .ok_or_else(|| Error::at(initializer.location, "the mem-initializer names nothing"))?;
        let node = self.graph.binding(binding);
        let member = self.graph.entity(node.entity);
        if node.kind == super::graph::BindingKind::Variable {
            if member.object().data_member == Some(super::graph::DataMemberKind::Instance)
                && member.home == Some(owner)
            {
                return Ok(node.entity);
            }
            if member.object().data_member == Some(super::graph::DataMemberKind::Instance)
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
        match lookup {
            InitializerLookup::Class => self.check_declaration_member_access(
                binding,
                class,
                None,
                scope,
                initializer.location,
            )?,
            InitializerLookup::Unqualified => {
                self.check_declaration_direct_access(binding, scope, initializer.location)?;
            }
            InitializerLookup::General => (),
        }
        if node.ty.is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Class(entity) if entity == class)) {
            return Ok(class);
        }
        if let Some(base) = node.ty.and_then(|ty| self.graph.class_of(ty)) {
            let direct = self
                .graph
                .entity(class)
                .class()
                .bases
                .iter()
                .find(|edge| edge.class == base);
            let is_virtual = self.graph.virtual_base_order(class).contains(&base);
            if direct.is_some_and(|edge| !edge.is_virtual) && is_virtual {
                return Err(Error::at(
                    initializer.location,
                    "the mem-initializer names both a direct base and a virtual base",
                ));
            }
            if direct.is_some() || is_virtual {
                return Ok(base);
            }
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
        support::{
            source::{SourceBuffer, SourceOffset},
            stack::StackBudget,
        },
        syntax::SyntaxUnit,
    };

    #[test]
    fn indirect_base_plans_keep_selection_without_replaying_syntax() {
        use super::ConstructorState;
        let source = SourceBuffer::new(b"struct V{int n;V(int x):n(x){}};struct B:virtual V{B():V(1){}};struct D:B{D():V(7){}};template<class T>struct U{U(){T::missing();}};void f(){D x;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"indirect-construction").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax = (tree.expressions.mark(), tree.declarations.mark());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut classes = Vec::new();
        for spelling in [b"V".as_slice(), b"B", b"D", b"U"] {
            let name = cursor.intern_name(spelling).unwrap();
            classes.push(
                graph
                    .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                    .entity,
            );
        }
        let ConstructorState::Ready(id) = graph.entity(classes[2]).special().default_construction
        else {
            panic!("default selection");
        };
        let plan = graph.constructors.get(id);
        assert_eq!(plan.bases.len(), 1);
        assert_eq!(plan.bases[0].class, classes[1]);
        assert_eq!(plan.indirect_bases.len(), 1);
        let base = plan.indirect_bases[0];
        assert_eq!(base.class, classes[0]);
        assert!(base.initializer.is_some());
        assert_eq!(
            graph.constructors.get(base.constructor.unwrap()).class,
            classes[0]
        );
        assert!(matches!(
            graph.entity(classes[3]).special().default_construction,
            ConstructorState::NotStarted
        ));
        assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
    }
    #[test]
    fn constructor_member_slots_keep_scalar_holes_and_original_member_ids() {
        let source = SourceBuffer::new(b"struct Leaf{Leaf();};struct Holder{int n=1;Leaf first;int other;Leaf initialized{};Leaf last;};void f(){Holder h;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constructor-member-slots").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Holder").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        let super::ConstructorState::Ready(id) = graph.entity(class).special().default_construction
        else {
            panic!("holder plan");
        };
        let plan = graph.constructors.get(id);
        assert_eq!(plan.members.len(), 5);
        assert_eq!(plan.subobjects.len(), plan.members.len());
        for (index, slot) in plan.subobjects.iter().enumerate() {
            assert_eq!(slot.is_some(), matches!(index, 1 | 3 | 4));
            if let Some(action) = slot {
                assert_eq!(action.member, plan.members[index].0);
                assert_eq!(action.member, graph.entity(class).class().members[index]);
            }
        }
    }

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
        let ConstructorState::Ready(outer_id) = graph.entity(outer).special().default_construction
        else {
            panic!("outer constructor was not prepared");
        };
        let ConstructorState::Ready(mid_id) = graph.entity(mid).special().default_construction
        else {
            panic!("child constructor was not prepared");
        };
        let outer_plan = graph.constructors.get(outer_id);
        assert!(outer_plan.nontrivial());
        assert_eq!(outer_plan.subobjects.len(), 1);
        let child = outer_plan.subobjects[0].unwrap();
        assert_eq!(child.member, graph.entity(outer).class().members[0]);
        assert_eq!(child.constructor, mid_id);
        assert!(child.arguments.is_none());
        let mid_plan = graph.constructors.get(mid_id);
        let child = mid_plan.subobjects[0].unwrap();
        assert_eq!(child.member, graph.entity(mid).class().members[0]);
        assert_eq!(
            Some(child.constructor),
            graph.entity(child.member).initialization().constructor
        );
        let InitializerKind::Braced(list) = graph
            .entity(child.member)
            .initialization()
            .member_initializer
            .unwrap()
            .kind
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
            graph
                .entity(class(b"Unused"))
                .special()
                .default_construction,
            ConstructorState::NotStarted
        ));
    }
    #[test]
    fn default_queries_share_base_identity_without_demanding_derived_plans() {
        use super::ConstructorState;
        let source = SourceBuffer::new(
            br"
            struct B { int n; template<class T=int> B(T x=T(8)):n(x){} };
            struct D:B { using B::B; D(double):B(9){} };
            struct E:B { using B::B; };
            struct F:E { using E::E; };
            struct Unused:B { using B::B; };
            void f(){D a;D b{};E c;F d;E supplied(3);}
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-query-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let mut classes = Vec::new();
        for spelling in [b"B".as_slice(), b"D", b"E", b"F", b"Unused"] {
            let name = analyzer.identifiers.intern(spelling).unwrap();
            classes.push(
                analyzer
                    .graph
                    .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                    .entity,
            );
        }
        let mark = analyzer.tree.expressions.mark();
        let first = analyzer
            .default_constructor(classes[1], crate::support::source::SourceOffset(0))
            .unwrap();
        let binding = analyzer.graph.constructors.get(first).binding.unwrap();
        assert_eq!(analyzer.graph.constructors.get(first).class, classes[0]);
        for &class in &classes[1..4] {
            assert_eq!(
                analyzer
                    .default_constructor(class, crate::support::source::SourceOffset(0))
                    .unwrap(),
                first
            );
            assert!(
                analyzer
                    .graph
                    .constructors
                    .default_function(class)
                    .is_some_and(|slot| slot.declared == Some(binding)
                        && slot.function == analyzer.graph.binding(binding).entity)
            );
            let ConstructorState::Ready(plan) =
                analyzer.graph.entity(class).special().default_construction
            else {
                panic!("selected plan");
            };
            assert_eq!(plan, first);
        }
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        let graph = &analyzer.graph;
        assert!(!graph.lower_positional_parameters(graph.binding(binding).entity));
        assert!(
            graph
                .entity(classes[2])
                .class()
                .declared_constructors
                .iter()
                .all(|&id| graph
                    .entity(graph.binding(id).entity)
                    .initialization()
                    .constructor
                    .is_none())
        );
        let unused = classes[4];
        assert!(graph.constructors.default_function(unused).is_none());
        assert!(matches!(
            graph.entity(unused).special().default_construction,
            ConstructorState::NotStarted
        ));
    }
    #[test]
    fn failed_default_queries_publish_no_binding_or_derived_plan() {
        let source = SourceBuffer::new(
            br"
            struct B {
                template<class T=int> B(T x=T()){}
                template<class T=long> B(T* x=0){}
            };
            struct D:B { using B::B; };
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-query-failure").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let name = analyzer.identifiers.intern(b"D").unwrap();
        let class = analyzer
            .graph
            .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
            .entity;
        for _ in 0..2 {
            assert!(matches!(
                analyzer.declared_default_constructor(class),
                Err(super::Error::UnconditionalDiagnostic {
                    message: "ambiguous default constructor",
                    ..
                })
            ));
            assert!(
                analyzer
                    .graph
                    .constructors
                    .default_function(class)
                    .is_none()
            );
            assert!(matches!(
                analyzer.graph.entity(class).special().default_construction,
                super::ConstructorState::NotStarted
            ));
            assert_eq!(analyzer.constexpr_frames.depth(), 0);
        }
    }
    #[test]
    fn constructor_views_leave_pointer_demand_to_reached_lowering_boundaries() {
        let source = SourceBuffer::new(
            b"struct Leaf{Leaf();};struct X{Leaf m;};void f(){X x;}struct Unused{};".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constructor-views").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"X".as_slice(), b"Unused"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes = names.map(|name| {
            let binding = analyzer.graph.own(global, name, Filter::Prefix).unwrap();
            analyzer.graph.binding(binding).entity
        });
        let plans = classes.map(|class| {
            analyzer
                .default_constructor(class, SourceOffset(0))
                .unwrap()
        });
        let default_functions =
            plans.map(|plan| analyzer.graph.lower_constructor_entity(plan).unwrap());
        let before = analyzer.graph.statistics();
        for ((class, plan), declaration) in classes.into_iter().zip(plans).zip(default_functions) {
            assert_eq!(
                analyzer
                    .default_constructor(class, SourceOffset(0))
                    .unwrap(),
                plan
            );
            assert_eq!(
                analyzer
                    .graph
                    .entity(declaration)
                    .initialization()
                    .constructor,
                Some(plan)
            );
            assert!(analyzer.graph.entity(declaration).context().scope.is_none());
            assert!(analyzer.graph.function_origin(declaration).is_err());
        }
        assert_eq!(analyzer.graph.statistics().entities, before.entities);
        assert_eq!(analyzer.graph.statistics().bindings, before.bindings);
        for plan in plans {
            assert!(
                analyzer
                    .graph
                    .constructors
                    .get(plan)
                    .object_pointer
                    .is_none()
            );
            assert!(
                analyzer
                    .graph
                    .lower_constructor(plan)
                    .object_pointer()
                    .is_err()
            );
        }
        let mut graph = analyzer.graph;
        let before = graph.statistics().types;
        let mut view = Vec::new();
        graph
            .print_semantics(tree, cursor, declarations, &mut view)
            .unwrap();
        assert_eq!(graph.statistics().types, before);
        assert!(
            view.windows(b"parameter this pointer to struct X".len())
                .any(|part| part == b"parameter this pointer to struct X")
        );
        let mut analyzer =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(&mut graph)).unwrap();
        analyzer.set_lowering();
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.finish_lowering_boundaries().unwrap();
        assert!(analyzer.graph.function_origin(default_functions[0]).is_ok());
        assert!(
            analyzer
                .graph
                .function_origin(default_functions[1])
                .is_err()
        );
        assert!(
            analyzer
                .graph
                .lower_constructor(plans[0])
                .object_pointer()
                .is_ok()
        );
        assert!(
            analyzer
                .graph
                .lower_constructor(plans[1])
                .object_pointer()
                .is_err()
        );
        let completed = analyzer.graph.statistics().types;
        analyzer.finish_lowering_boundaries().unwrap();
        assert_eq!(analyzer.graph.statistics().types, completed);
    }

    #[test]
    fn anonymous_member_expansion_and_direct_query_have_separate_owners() {
        let source =
            SourceBuffer::new(b"struct Holder{union{int n;};};void f(){Holder h;}".to_vec())
                .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"anonymous-default").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Holder").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let class = analyzer
            .graph
            .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
            .entity;
        let member = analyzer.graph.entity(class).class().members[0];
        let child = analyzer
            .graph
            .class_of(analyzer.graph.lower_entity_type(member).unwrap())
            .unwrap();
        let super::ConstructorState::Ready(outer) =
            analyzer.graph.entity(class).special().default_construction
        else {
            panic!("enclosing constructor was required");
        };
        let plan = analyzer.graph.constructors.get(outer).subobjects[0]
            .unwrap()
            .constructor;
        assert!(matches!(
            analyzer.graph.entity(child).special().default_construction,
            super::ConstructorState::NotStarted
        ));
        assert!(analyzer.graph.lower_constructor_entity(plan).is_none());
        let demands = analyzer.graph.root_demands.as_slice().len();
        let direct = analyzer
            .default_constructor(child, SourceOffset(0))
            .unwrap();
        assert_ne!(direct, plan);
        assert_eq!(
            analyzer
                .default_constructor(child, SourceOffset(0))
                .unwrap(),
            direct
        );
        assert!(analyzer.graph.lower_constructor_entity(plan).is_none());
        let declaration = analyzer.graph.lower_constructor_entity(direct).unwrap();
        assert_eq!(
            analyzer
                .graph
                .entity(declaration)
                .initialization()
                .constructor,
            Some(direct)
        );
        assert!(analyzer.graph.function_origin(declaration).is_err());
        assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
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
            graph.entity(u).class().layout,
            crate::sema::layout::LayoutState::NotStarted
        ));
    }
}
