// Translated from analyzer_copy.cpp and analyzer_construction.cpp. See NOTICE.
//! Same-class construction retains selection separately from operand conversions.
//! Elision, storage transfer and constructor calls have distinct demand boundaries.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{Conversion, ValueCategory as V},
    function_templates::TemplateOrdering,
    graph::{BindingId, EntityId, FunctionAvailability, ScopeId},
    procedural_calls::ArgumentSequence,
    special_members::SpecialDefinition,
    types::{RefKind, TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CopyOrigin {
    Declared(BindingId),
    Implicit { class: EntityId, kind: RefKind },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct CopySelection {
    pub origin: CopyOrigin,
    pub binding: Conversion,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum CopyAction {
    Storage {
        reference: TypeId,
        skip_empty: bool,
    },
    Constructor {
        constructor: super::construction::ConstructorId,
        elision: bool,
        written_move: bool,
    },
}
#[derive(Clone, Copy)]
pub(super) enum CopyUse {
    Object { direct: bool },
    Value { in_place: bool },
    Return { implicit_move: bool },
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConstructorContext {
    Copy,
    Direct,
    CopyList,
    Standard { explicit: bool },
}
#[derive(Clone, Copy)]
pub(super) enum ClassPassing {
    Direct,
    Indirect,
}
#[derive(Clone, Copy)]
pub(super) struct ImplicitCopies {
    pub(super) copy: Option<(TypeId, bool)>,
    pub(super) moving: Option<TypeId>,
    pub(super) copy_properties: Option<super::copy_properties::CopyPropertiesId>,
    pub(super) move_properties: Option<super::copy_properties::CopyPropertiesId>,
    pub(super) passing: Option<ClassPassing>,
}
impl super::graph::Graph {
    pub(crate) fn lower_copy_form(
        &self,
        function: EntityId,
    ) -> Result<Option<super::lowering::CopyForm>, Error> {
        let entity = self.entity(function);
        let Some(kind) = entity.copy_kind.or(entity.assignment_kind) else {
            return Ok(None);
        };
        let class = entity
            .home
            .and_then(|home| self.scope(home).owner)
            .ok_or(Error::Invariant("copy form without class owner"))?;
        Ok(Some(super::lowering::CopyForm {
            class,
            moving: kind == RefKind::Rvalue,
            assignment: entity.assignment_kind.is_some(),
            locality: if self.lower_entity_unit_local(class)? {
                super::lowering::Locality::Unit
            } else {
                super::lowering::Locality::External
            },
        }))
    }
    pub(crate) fn lower_copy_constructor(&self, function: EntityId) -> bool {
        self.entity(function).copy_kind.is_some()
    }
    pub(crate) fn lower_class_copy(&self, id: ast::ExprId) -> Result<Option<CopyAction>, Error> {
        self.expression_facts.get(id)?;
        Ok(self.expression_facts.copy_action(id))
    }
    pub(crate) fn lower_direct_class_parameter(&self, class: EntityId) -> Result<bool, Error> {
        self.entity(class)
            .implicit_copies
            .and_then(|copies| copies.passing)
            .map(|passing| matches!(passing, ClassPassing::Direct))
            .ok_or(Error::Missing("class parameter call boundary"))
    }
    fn class_parameter_direct(&self, class: EntityId) -> Result<bool, Error> {
        let entity = self.entity(class);
        let copies = entity
            .implicit_copies
            .ok_or(Error::Missing("class parameter copy properties"))?;
        let mut usable =
            copies.copy.is_some_and(|(_, deleted)| !deleted) || copies.moving.is_some();
        let mut trivial = self.destructor_trivial(class)?;
        for id in [copies.copy_properties, copies.move_properties]
            .into_iter()
            .flatten()
        {
            let facts = self.copy_properties.get(id);
            if !facts.deleted {
                trivial &= facts.trivial;
            }
        }
        for &binding in &entity.declared_constructors {
            let constructor = self.entity(self.binding(binding).entity);
            if constructor.copy_kind.is_none()
                || constructor.availability == FunctionAvailability::Deleted
            {
                continue;
            }
            if let Some(id) = constructor.copy_properties {
                let facts = self.copy_properties.get(id);
                if facts.deleted {
                    continue;
                }
                trivial &= facts.trivial;
            } else {
                trivial = false;
            }
            usable = true;
        }
        Ok(trivial && usable)
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    selection: CopySelection,
    argument: Option<ArgumentSequence>,
    deleted: bool,
}

#[derive(Clone, Copy)]
enum CopyResolution {
    Selected(Candidate),
    NoViable { offered: bool },
    Ambiguous,
}

#[derive(Default)]
pub(super) struct CopyScratch {
    candidates: Vec<Candidate>,
}

impl Analyzer<'_> {
    pub(super) fn prepare_class_parameter(&mut self, class: EntityId) -> Result<(), Error> {
        let mut copies = self.implicit_copy_candidates(class)?;
        if copies.passing.is_none() {
            copies.passing = Some(if self.graph.class_parameter_direct(class)? {
                ClassPassing::Direct
            } else {
                ClassPassing::Indirect
            });
            self.graph.entity_mut(class).implicit_copies = Some(copies);
        }
        Ok(())
    }
    fn classify_copy_kind(
        &self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<Option<RefKind>, Error> {
        let function = self.graph.entity(self.graph.binding(binding).entity);
        if function.template_head.is_some()
            || self
                .graph
                .inherited_constructors
                .source(self.graph.binding(binding).entity)
                .is_some()
        {
            return Ok(None);
        }
        let ty = function
            .ty
            .ok_or(Error::Invariant("copy constructor without type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        if count == 0 || (1..count).any(|i| function.defaults.get(i).is_none_or(Option::is_none)) {
            return Ok(None);
        }
        let parameter = self.graph.types.function_parameter(ty, 0)?;
        Ok(match self.graph.types.get(parameter).kind {
            TypeKind::Reference { element, kind } if matches!(self.graph.types.get(element).kind, TypeKind::Class(owner) if owner == class) => {
                Some(kind)
            }
            _ => None,
        })
    }

    pub(super) fn classify_class_copies(&mut self, class: EntityId) -> Result<(), Error> {
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let kind = self.classify_copy_kind(binding, class)?;
            let function = self.graph.binding(binding).entity;
            self.graph.entity_mut(function).copy_kind = kind;
        }
        self.classify_assignments(class)?;
        // A written defaulted member publishes its own facts at class completion;
        // the class's implicit constructors still wait for a use to demand them.
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.copy_kind.is_some()
                && matches!(
                    entity.special_definition,
                    Some(SpecialDefinition::Defaulted { .. })
                )
            {
                self.prepare_copy_children(class)?;
                self.prepare_declared_copy(binding, class)?;
            }
        }
        Ok(())
    }

    pub(super) fn declared_copy_kind(
        &self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<Option<RefKind>, Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        if entity.template_head.is_some() {
            return Ok(None);
        }
        if !entity.member_function.is_some_and(|key| key.class == class) {
            return Err(Error::Invariant("copy constructor of another class"));
        }
        Ok(entity.copy_kind)
    }

    fn implicit_copy_candidates(&mut self, class: EntityId) -> Result<ImplicitCopies, Error> {
        self.prepare_copy_properties(class)?;
        self.graph
            .entity(class)
            .implicit_copies
            .ok_or(Error::Invariant("copy properties not published"))
    }

    pub(super) fn record_class_copy(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
        usage: CopyUse,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Class(class) = self.graph.types.get(target).kind else {
            return Ok(());
        };
        if self
            .graph
            .expression_facts
            .class_conversion(id)
            .is_some_and(|(id, _)| self.graph.constructors.get(id).class == class)
        {
            return Ok(());
        }
        if self
            .graph
            .expression_facts
            .function_conversion(id)
            .is_some()
        {
            return Ok(());
        }
        if !self.graph.entity(class).complete || self.graph.entity(class).scope.is_none() {
            return Err(Error::at(
                location,
                "initialization of an incomplete class object",
            ));
        }
        let facts = self.graph.expression_facts.get(id)?;
        let direct = matches!(usage, CopyUse::Object { direct: true });
        let in_place = matches!(
            usage,
            CopyUse::Value { in_place: true } | CopyUse::Return { .. }
        );
        if facts.functions.is_some()
            || !matches!(self.graph.types.get(facts.ty).kind, TypeKind::Class(from)
                if from == class || self.graph.is_base_of(class, from))
        {
            return Err(Error::Missing(
                "converting and derived class initialization",
            ));
        }
        let own_class = self.graph.class_of(facts.ty) == Some(class);
        let elided = own_class
            && facts.category == V::Prvalue
            && (in_place
                || !matches!(
                    self.tree
                        .expressions
                        .get(self.graph.expression_facts.root(id))
                        .kind,
                    ast::ExprKind::Conditional { .. }
                ));
        let candidate = if matches!(
            usage,
            CopyUse::Return {
                implicit_move: true
            }
        ) {
            self.select_return_copy(class, facts, scope, location)?
        } else {
            self.select_copy_sequence(class, facts, Some(scope), direct, location)?
        };
        let selected = candidate.selection;
        let special = match selected.origin {
            CopyOrigin::Implicit { .. } => true,
            CopyOrigin::Declared(binding) => self
                .graph
                .entity(self.graph.binding(binding).entity)
                .copy_kind
                .is_some(),
        };
        if let Some(sequence) = candidate.argument {
            self.apply_argument_sequence(id, sequence, scope)?;
            if let CopyOrigin::Declared(binding) = selected.origin {
                self.check_constructor_parameters(
                    self.graph.binding(binding).entity,
                    &[id],
                    scope,
                    false,
                )?;
            }
        }
        let mut facts = self.graph.expression_facts.get(id)?;
        facts.base_path =
            self.conversion_base_path(facts, selected.binding, scope, location, true)?;
        // A base value's selected constructor binds the original derived
        // expression. Its prvalue is a distinct complete temporary, whose
        // destruction and layout must be available before lowering that bind.
        if !own_class {
            self.check_binding_temporary(facts, selected.binding, scope, location)?;
        }
        let written_move = direct
            && matches!(selected.origin, CopyOrigin::Declared(binding)
            if self.graph.entity(self.graph.binding(binding).entity).copy_kind == Some(RefKind::Rvalue));
        let action = if special
            && candidate.argument.is_none()
            && !selected.binding.temporary
            && (elided
                || own_class
                    && facts.category == V::Prvalue
                    && !written_move
                    && self.selected_copy_properties(class, selected)?.trivial)
        {
            None
        } else {
            let mut action = self.copy_action(class, selected, usage, location)?;
            if let CopyAction::Constructor { elision, .. } = &mut action {
                *elision = special && own_class && facts.category == V::Prvalue && !direct;
            }
            Some(action)
        };
        self.graph.expression_facts.set_copy_action(id, action);
        facts.class_copy = Some(selected);
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }

    pub(super) fn copy_action(
        &mut self,
        class: EntityId,
        selected: CopySelection,
        usage: CopyUse,
        location: SourceOffset,
    ) -> Result<CopyAction, Error> {
        if let CopyOrigin::Declared(binding) = selected.origin
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .copy_kind
                .is_none()
        {
            let constructor = self.selected_constructor_plan(binding, location)?;
            self.demand_constructor_use(constructor);
            return Ok(CopyAction::Constructor {
                constructor,
                elision: false,
                written_move: false,
            });
        }
        let properties = self.selected_copy_properties(class, selected)?;
        let written_move = matches!(usage, CopyUse::Object { direct: true })
            && matches!(selected.origin, CopyOrigin::Declared(binding)
                if self.graph.entity(self.graph.binding(binding).entity).copy_kind == Some(RefKind::Rvalue));
        let empty_lifetime = !self.graph.destructor_trivial(class)? && {
            self.class_layout(class, location)?;
            self.graph.lower_class_empty(class)?
        };
        let provided = matches!(selected.origin, CopyOrigin::Declared(binding)
            if !matches!(self.graph.entity(self.graph.binding(binding).entity).special_definition,
                Some(SpecialDefinition::Defaulted { .. })));
        if provided || !properties.trivial || written_move || empty_lifetime {
            let constructor = self.copy_constructor_plan(class, selected, location)?;
            self.demand_constructor_use(constructor);
            return Ok(CopyAction::Constructor {
                constructor,
                elision: false,
                written_move,
            });
        }
        // A written defaulted declaration is odr-used even for a byte transfer.
        if let CopyOrigin::Declared(binding) = selected.origin {
            self.demand_function(binding, true);
        }
        Ok(CopyAction::Storage {
            reference: selected.binding.target,
            skip_empty: matches!(usage, CopyUse::Object { .. }),
        })
    }

    pub(super) fn check_final_copy_selection(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let resolution = self.resolve_class_constructor(
            class,
            facts,
            ConstructorContext::Standard { explicit: true },
        )?;
        self.check_copy_selection(resolution, Some(scope), location)
            .map(|_| ())
            .map_err(|error| match error {
                Error::Diagnostic {
                    message:
                        "no viable function for the call"
                        | "no constructor for the initialization"
                        | "ambiguous call to overloaded function",
                    ..
                } => Error::at(
                    location,
                    "no unambiguous constructor initializes the object from the converted temporary",
                ),
                other => other,
            })
    }
    pub(super) fn select_result_constructor(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<CopySelection, Error> {
        let resolution = self.resolve_class_constructor(
            class,
            facts,
            ConstructorContext::Standard { explicit: true },
        )?;
        if let CopyResolution::Selected(candidate) = resolution
            && !matches!(
                self.graph
                    .types
                    .get(candidate.selection.binding.target)
                    .kind,
                TypeKind::Reference { .. }
            )
        {
            return Err(Error::at(
                location,
                "a class object initialized through a constructor's class parameter is not supported",
            ));
        }
        self.check_copy_selection(resolution, Some(scope), location)
            .map(|candidate| candidate.selection)
            .map_err(|error| match error {
                Error::Diagnostic {
                    message:
                        "no viable function for the call"
                        | "no constructor for the initialization"
                        | "ambiguous call to overloaded function",
                    ..
                } => Error::at(
                    location,
                    "no unambiguous constructor initializes the class object from the value",
                ),
                other => other,
            })
    }
    pub(super) fn select_subobject_copy(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        direct: bool,
    ) -> Result<CopySelection, Error> {
        // Preserve the selected identity even when access deletes the enclosing
        // defaulted member. Its owner checks base/member protected access next.
        let resolution = self.resolve_class_constructor(
            class,
            facts,
            ConstructorContext::Standard { explicit: direct },
        )?;
        self.check_copy_selection(resolution, None, SourceOffset(0))
            .map(|candidate| candidate.selection)
    }
    fn select_copy_sequence(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        access_scope: Option<ScopeId>,
        direct: bool,
        location: SourceOffset,
    ) -> Result<Candidate, Error> {
        let resolution = self.resolve_class_copy(class, facts, direct)?;
        self.check_copy_selection(resolution, access_scope, location)
    }

    fn select_return_copy(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Candidate, Error> {
        // 12.8p32: probe without changing the named operand's source category.
        // Deletion/access are checked only after choosing which pass to keep.
        let mut view = facts;
        view.category = V::Xvalue;
        let resolution = self.resolve_class_copy(class, view, false)?;
        if let CopyResolution::Selected(candidate) = resolution
            && let TypeKind::Reference {
                element,
                kind: RefKind::Rvalue,
            } = self
                .graph
                .types
                .get(candidate.selection.binding.target)
                .kind
            && self.bare_type(element)? == self.bare_type(facts.ty)?
        {
            return self.check_copy_selection(resolution, Some(scope), location);
        }
        self.select_copy_sequence(class, facts, Some(scope), false, location)
    }

    fn resolve_class_copy(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        direct: bool,
    ) -> Result<CopyResolution, Error> {
        self.resolve_class_constructor(
            class,
            facts,
            if direct {
                ConstructorContext::Direct
            } else {
                ConstructorContext::Copy
            },
        )
    }
    fn resolve_class_constructor(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        context: ConstructorContext,
    ) -> Result<CopyResolution, Error> {
        let mut scratch = std::mem::take(&mut self.copy_scratch);
        let selected = self.rank_class_copy(class, facts, context, &mut scratch.candidates);
        scratch.candidates.clear();
        self.copy_scratch = scratch;
        let mut selected = selected?;
        if let CopyResolution::Selected(candidate) = &mut selected
            && let CopyOrigin::Declared(binding) = candidate.selection.origin
        {
            let binding = self.inherited_constructor_specialization(class, binding, 1)?;
            candidate.selection.origin = CopyOrigin::Declared(binding);
            candidate.deleted |= self
                .graph
                .entity(self.graph.binding(binding).entity)
                .availability
                == FunctionAvailability::Deleted;
        }
        Ok(selected)
    }
    pub(super) fn single_constructor_not_viable(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
    ) -> Result<bool, Error> {
        Ok(matches!(
            self.resolve_class_copy(class, facts, true)?,
            CopyResolution::NoViable { .. }
        ))
    }
    fn rank_class_copy(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        context: ConstructorContext,
        candidates: &mut Vec<Candidate>,
    ) -> Result<CopyResolution, Error> {
        let copies = self.implicit_copy_candidates(class)?;
        candidates.clear();
        let mut offered = copies.copy.is_some() || copies.moving.is_some();
        offered |= self.declared_copy_candidates(class, facts, context, false, candidates)?;
        for (kind, parameter, deleted) in [
            copies
                .copy
                .map(|(parameter, deleted)| (RefKind::Lvalue, parameter, deleted)),
            copies
                .moving
                .map(|parameter| (RefKind::Rvalue, parameter, false)),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(sequence) = self.copy_argument_sequence(facts, parameter, class, context)? {
                candidates.push(Candidate {
                    selection: CopySelection {
                        origin: CopyOrigin::Implicit { class, kind },
                        binding: sequence.conversion,
                    },
                    argument: sequence.user_conversion.map(|_| sequence),
                    deleted,
                });
            }
        }
        // Preserve resolve's last viable non-template's perfect-sequence flag;
        // a later converting copy candidate can require template deduction.
        if !candidates.last().is_some_and(|&candidate| {
            self.perfect_sequence(
                Some(candidate.argument.unwrap_or(ArgumentSequence {
                    conversion: candidate.selection.binding,
                    application: Some(candidate.selection.binding),
                    function: None,
                    user_conversion: None,
                })),
                facts.ty,
                facts.category,
                facts.functions.is_some(),
            )
        }) {
            offered |= self.declared_copy_candidates(class, facts, context, true, candidates)?;
        }
        let mut best = 0;
        for index in 1..candidates.len() {
            if self.compare_copy_candidates(candidates[index], candidates[best])?
                == Ordering::Greater
            {
                best = index;
            }
        }
        let Some(chosen) = candidates.get(best).copied() else {
            return Ok(CopyResolution::NoViable { offered });
        };
        for (index, candidate) in candidates.iter().enumerate() {
            if index != best
                && self.compare_copy_candidates(chosen, *candidate)? != Ordering::Greater
            {
                return Ok(CopyResolution::Ambiguous);
            }
        }
        Ok(CopyResolution::Selected(chosen))
    }

    fn declared_copy_candidates(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        context: ConstructorContext,
        templated: bool,
        candidates: &mut Vec<Candidate>,
    ) -> Result<bool, Error> {
        let mut offered = false;
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() != templated {
                continue;
            }
            if matches!(
                context,
                ConstructorContext::Copy | ConstructorContext::Standard { explicit: false }
            ) && entity.explicitness == super::specifiers::Explicitness::Explicit
            {
                continue;
            }
            offered = true;
            let Some(binding) = self.deduce_function_call(binding, &[], false, Some(facts))? else {
                continue;
            };
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if matches!(
                context,
                ConstructorContext::Copy | ConstructorContext::Standard { explicit: false }
            ) && entity.explicitness == super::specifiers::Explicitness::Explicit
            {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("constructor without type"))?;
            let count = self.graph.types.function_parameter_count(ty)?;
            let variadic = matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Function { variadic: true, .. }
            );
            if count == 0 && !variadic
                || (1..count)
                    .any(|i| !self.function_has_default(self.graph.binding(binding).entity, i))
            {
                continue;
            }
            let defaulted = matches!(
                entity.special_definition,
                Some(SpecialDefinition::Defaulted { .. })
            );
            let explicitly_deleted = entity.availability == FunctionAvailability::Deleted;
            let copy_properties = entity.copy_properties;
            let kind = entity.copy_kind;
            let sequence = if count == 0 {
                let target = self.decayed_type(facts.ty)?;
                let Some(mut conversion) = self.scalar_sequence(facts, target, false)? else {
                    continue;
                };
                conversion.rank = super::expression_facts::ConversionRank::Ellipsis;
                ArgumentSequence {
                    conversion,
                    application: Some(conversion),
                    function: None,
                    user_conversion: None,
                }
            } else {
                let parameter = self.graph.types.function_parameter(ty, 0)?;
                let Some(sequence) =
                    self.copy_argument_sequence(facts, parameter, class, context)?
                else {
                    continue;
                };
                sequence
            };
            let deleted = explicitly_deleted
                || copy_properties.is_some_and(|id| self.graph.copy_properties.get(id).deleted);
            if defaulted && deleted && kind == Some(RefKind::Rvalue) {
                continue;
            }
            candidates.push(Candidate {
                selection: CopySelection {
                    origin: CopyOrigin::Declared(binding),
                    binding: sequence.conversion,
                },
                argument: sequence.user_conversion.map(|_| sequence),
                deleted,
            });
        }
        Ok(offered)
    }

    fn copy_argument_sequence(
        &mut self,
        source: super::expression_facts::ExprFacts,
        parameter: TypeId,
        class: EntityId,
        context: ConstructorContext,
    ) -> Result<Option<ArgumentSequence>, Error> {
        if let Some(conversion) = self.scalar_sequence(source, parameter, false)? {
            return Ok(Some(ArgumentSequence {
                conversion,
                application: Some(conversion),
                function: None,
                user_conversion: None,
            }));
        }
        if matches!(context, ConstructorContext::Standard { .. }) {
            return Ok(None);
        }
        let direct = context == ConstructorContext::Direct
            && matches!(self.graph.types.get(parameter).kind,
            TypeKind::Reference { element, .. } if self.graph.class_of(element) == Some(class));
        self.user_conversion_sequence(source, parameter, direct)
    }
    fn compare_copy_candidates(&mut self, a: Candidate, b: Candidate) -> Result<Ordering, Error> {
        let sequence = |candidate: Candidate| {
            candidate.argument.unwrap_or(ArgumentSequence {
                conversion: candidate.selection.binding,
                application: Some(candidate.selection.binding),
                function: None,
                user_conversion: None,
            })
        };
        let order = self.compare_argument_sequences(sequence(a), sequence(b))?;
        if order != Ordering::Equal {
            return Ok(order);
        }
        match (a.selection.origin, b.selection.origin) {
            (CopyOrigin::Declared(a), CopyOrigin::Declared(b)) => {
                self.compare_function_templates(a, b, TemplateOrdering::Call(1))
            }
            (CopyOrigin::Declared(binding), CopyOrigin::Implicit { .. }) => Ok(
                if self
                    .graph
                    .function_templates
                    .primary(self.graph.binding(binding).entity)
                    .is_some()
                {
                    Ordering::Less
                } else {
                    Ordering::Equal
                },
            ),
            (CopyOrigin::Implicit { .. }, CopyOrigin::Declared(binding)) => Ok(
                if self
                    .graph
                    .function_templates
                    .primary(self.graph.binding(binding).entity)
                    .is_some()
                {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                },
            ),
            _ => Ok(Ordering::Equal),
        }
    }
    pub(super) fn initialize_single_constructor(
        &mut self,
        class: EntityId,
        argument: ast::ExprId,
        input: super::constructor_initialization::ClassInput,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<super::construction::ConstructorId, Error> {
        let facts = self.graph.expression_facts.get(argument)?;
        let context = if input.copy_list {
            ConstructorContext::CopyList
        } else {
            ConstructorContext::Direct
        };
        let resolution = self.resolve_class_constructor(class, facts, context)?;
        if let CopyResolution::Selected(candidate) = resolution
            && let CopyOrigin::Declared(binding) = candidate.selection.origin
        {
            self.check_inherited_constructor_access(binding, location)?;
            if input.copy_list
                && self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .explicitness
                    == super::specifiers::Explicitness::Explicit
            {
                return Err(Error::at(
                    location,
                    "copy-list-initialization selects an explicit constructor",
                ));
            }
        }
        let candidate = self.check_copy_selection(resolution, Some(scope), location)?;
        let selected = candidate.selection;
        let constructor = match selected.origin {
            CopyOrigin::Declared(binding) => self.selected_constructor_plan(binding, location)?,
            CopyOrigin::Implicit { .. } => self.copy_constructor_plan(class, selected, location)?,
        };
        self.validate_constructor_use(constructor, scope, location)?;
        let sequence = candidate.argument.unwrap_or(ArgumentSequence {
            conversion: selected.binding,
            application: Some(selected.binding),
            function: None,
            user_conversion: None,
        });
        if selected.binding.rank == super::expression_facts::ConversionRank::Ellipsis {
            self.ellipsis_argument(argument)?;
        } else {
            self.apply_argument_sequence(argument, sequence, scope)?;
        }
        let function = self
            .graph
            .lower_constructor_entity(constructor)
            .ok_or(Error::Invariant("single constructor without entity"))?;
        self.check_constructor_parameters(function, &[argument], scope, input.list.is_some())?;
        self.demand_constructor_use(constructor);
        Ok(constructor)
    }
    fn check_copy_selection(
        &mut self,
        resolution: CopyResolution,
        access_scope: Option<ScopeId>,
        location: SourceOffset,
    ) -> Result<Candidate, Error> {
        let chosen = match resolution {
            CopyResolution::Selected(chosen) => chosen,
            CopyResolution::NoViable { offered } => {
                return Err(Error::at(
                    location,
                    if offered {
                        "no viable function for the call"
                    } else {
                        "no constructor for the initialization"
                    },
                ));
            }
            CopyResolution::Ambiguous => {
                return Err(Error::at(location, "ambiguous call to overloaded function"));
            }
        };
        if chosen.deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        if let CopyOrigin::Declared(binding) = chosen.selection.origin
            && let Some(scope) = access_scope
        {
            self.check_declaration_direct_access(binding, scope, location)?;
        }
        Ok(chosen)
    }
}

#[cfg(test)]
mod tests {
    use super::CopyAction;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };
    #[test]
    fn direct_constructor_queries_keep_sequences_in_scratch_until_application() {
        use super::{ConstructorContext, CopyResolution};
        use crate::sema::{
            expression_facts::{ExprFacts, ValueCategory},
            procedural_calls::UserConversion,
        };
        let source = SourceBuffer::new(
            b"struct A{int n;A();A(const A&);};struct S{explicit operator A() const;};struct Unused{A a;};".to_vec(),
        ).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"direct-constructor-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let target_name = cursor.intern_name(b"A").unwrap();
        let source_name = cursor.intern_name(b"S").unwrap();
        let unused_name = cursor.intern_name(b"Unused").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let class = |name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity
        };
        let target = class(target_name);
        let source = class(source_name);
        let unused = class(unused_name);
        let facts = ExprFacts::new(
            analyzer.graph.entity(source).ty.unwrap(),
            ValueCategory::Lvalue,
        );
        let converter = analyzer.graph.entity(source).declared_conversions[0];
        let demands = analyzer.graph.root_demands.as_slice().len();
        for _ in 0..3 {
            let CopyResolution::Selected(candidate) = analyzer
                .resolve_class_constructor(target, facts, ConstructorContext::Direct)
                .unwrap()
            else {
                panic!("direct constructor selection");
            };
            assert_eq!(
                candidate.argument.unwrap().user_conversion,
                Some(UserConversion::Function(converter))
            );
            for context in [
                ConstructorContext::CopyList,
                ConstructorContext::Standard { explicit: true },
            ] {
                assert!(matches!(
                    analyzer
                        .resolve_class_constructor(target, facts, context)
                        .unwrap(),
                    CopyResolution::NoViable { .. }
                ));
            }
            assert!(analyzer.copy_scratch.candidates.is_empty());
        }
        assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
        assert!(analyzer.graph.entity(unused).implicit_copies.is_none());
        for &binding in &analyzer.graph.entity(target).declared_constructors {
            assert!(
                analyzer
                    .graph
                    .entity(analyzer.graph.binding(binding).entity)
                    .constructor
                    .is_none()
            );
        }
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
        assert!(std::mem::size_of::<ExprFacts>() <= 256);
    }
    #[test]
    fn base_value_arguments_keep_source_identity_and_selected_reference_binding() {
        use crate::{
            sema::{
                expression_facts::ValueCategory,
                types::{RefKind, TypeKind},
            },
            syntax::ast,
        };
        let source = SourceBuffer::new(
            br"
            struct B{int n;B();B(const B&);B(B&&);~B();};
            struct D:B{int m;D();};
            void take(B);
            void f(D&d){take(((d)));take(D());}
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"base-value-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        let mut class = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity
        };
        let base = class(b"B");
        let derived = class(b"D");
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        for (&statement, (category, reference)) in tree.stmt_lists.get(statements).iter().zip([
            (ValueCategory::Lvalue, RefKind::Lvalue),
            (ValueCategory::Prvalue, RefKind::Rvalue),
        ]) {
            let ast::StmtKind::Expression(Some(call)) = tree.statements.get(statement).kind else {
                panic!("call statement");
            };
            let ast::ExprKind::Call { arguments, .. } = tree.expressions.get(call).kind else {
                panic!("call");
            };
            let argument = tree.expr_lists.get(arguments)[0];
            let facts = graph.expression_facts.get(argument).unwrap();
            assert_eq!(graph.class_of(facts.ty), Some(derived));
            assert_eq!(facts.category, category);
            assert_eq!(
                graph.expression_type(graph.expression_facts.root(argument)),
                Some(facts.ty)
            );
            assert_eq!(
                facts.conversion.unwrap().result,
                graph.entity(base).ty.unwrap()
            );
            let binding = facts.class_copy.unwrap().binding;
            assert!(
                matches!(graph.types.get(binding.target).kind, TypeKind::Reference { element, kind } if graph.class_of(element) == Some(base) && kind == reference)
            );
            assert!(facts.base_path.is_some());
            let Some(CopyAction::Constructor {
                constructor,
                elision: false,
                ..
            }) = graph.lower_class_copy(argument).unwrap()
            else {
                panic!("base copy action");
            };
            assert_eq!(graph.constructors.get(constructor).class, base);
        }
        assert!(graph.entity(derived).implicit_destructor.is_some());
    }

    #[test]
    fn losing_base_value_overload_does_not_demand_its_copy_callable() {
        let source = SourceBuffer::new(
            b"struct B{B(const B&);};struct D:B{};void take(B);void take(D&);void f(D&d){take(d);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"base-value-ranking").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let name = cursor.intern_name(b"B").unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        let binding = graph.entity(class).declared_constructors[0];
        assert!(
            graph
                .entity(graph.binding(binding).entity)
                .constructor
                .is_none()
        );
        assert!(!graph.root_demands.as_slice().iter().any(|demand| matches!(demand, super::super::function_demands::Demand::Function { binding: selected, .. } if *selected == binding)));
    }

    #[test]
    fn nonelided_copy_actions_share_source_identity_without_replacing_operand_facts() {
        use crate::{sema::expression_facts::ValueCategory, syntax::ast};
        let source =
            SourceBuffer::new(b"struct A{int n;};A a={3};A b=((a));A c=A(a);".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"copy-use-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let parsed_expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), parsed_expressions);
        let ast::DeclKind::Simple { declarators, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("copy object declaration");
        };
        let ast::InitializerKind::Assign(outer) = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap()
            .kind
        else {
            panic!("copy source");
        };
        let root = graph.expression_facts.root(outer);
        assert_ne!(root, outer);
        let facts = graph.expression_facts.get(outer).unwrap();
        assert_eq!(facts.category, ValueCategory::Lvalue);
        assert!(facts.conversion.is_none());
        assert_eq!(graph.expression_type(root), Some(facts.ty));
        let Some(CopyAction::Storage {
            reference,
            skip_empty: true,
        }) = graph.lower_class_copy(outer).unwrap()
        else {
            panic!("named storage copy action");
        };
        assert_eq!(facts.class_copy.unwrap().binding.target, reference);
        assert!(matches!(graph.lower_class_copy(root).unwrap(),
            Some(CopyAction::Storage { reference: inner, skip_empty: true }) if inner == reference));
    }
    #[test]
    fn class_parameter_boundary_consumes_completed_copy_properties() {
        let source = SourceBuffer::new(b"struct W{int w;W(int);};int f(W v){return v.w;}int g(){return f(W(1));}struct N{N();N(const N&);};void h(N){}void i(){h(N());}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"class-parameter-boundary").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        for (spelling, direct) in [(b"W".as_slice(), true), (b"N".as_slice(), false)] {
            let name = cursor.intern_name(spelling).unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            assert_eq!(graph.lower_direct_class_parameter(class).unwrap(), direct);
        }
    }

    #[test]
    fn return_move_selection_preserves_source_facts_and_demands_only_the_kept_pass() {
        use crate::{
            sema::{
                expression_facts::ValueCategory,
                types::{RefKind, TypeKind},
            },
            syntax::ast,
        };
        let source = SourceBuffer::new(
            br"
            struct A{int n;A();A(const A&);A(A&&);};
            struct B{int n;B();B(B&);B(const B&&);B(volatile B&&);};
            struct Unused{A a;};
            A moved(A a){return ((a));}
            A repeated(A a){return a;}
            A referenced(A&& a){return a;}
            B fallback(B b){return b;}
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"return-move-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let declarations_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), declarations_count);
        let mut kept_move = None;
        for (name, expected) in [
            (b"moved".as_slice(), RefKind::Rvalue),
            (b"repeated", RefKind::Rvalue),
            (b"referenced", RefKind::Lvalue),
            (b"fallback", RefKind::Lvalue),
        ] {
            let name = cursor.intern_name(name).unwrap();
            let function = graph
                .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
                .entity;
            let definition = graph.entity(function).function_definition.unwrap();
            let ast::DeclKind::Function { body, .. } = tree.declarations.get(definition).kind
            else {
                panic!("function definition");
            };
            let ast::Body::Parsed(body) = tree.bodies.get(body) else {
                panic!("parsed function body");
            };
            let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
                panic!("compound body");
            };
            let ast::StmtKind::Return(Some(operand)) =
                tree.statements.get(tree.stmt_lists.get(items)[0]).kind
            else {
                panic!("return operand");
            };
            let facts = graph.expression_facts.get(operand).unwrap();
            assert_eq!(facts.category, ValueCategory::Lvalue);
            let conversion = facts.conversion.unwrap();
            assert_eq!(conversion.target, facts.ty);
            assert!(!conversion.temporary);
            let selected = facts.class_copy.unwrap();
            assert!(
                matches!(graph.types.get(selected.binding.target).kind, TypeKind::Reference { kind, .. } if kind == expected)
            );
            let Some(CopyAction::Constructor {
                constructor,
                elision: false,
                ..
            }) = graph.lower_class_copy(operand).unwrap()
            else {
                panic!("selected return transfer");
            };
            if expected == RefKind::Rvalue {
                assert_eq!(*kept_move.get_or_insert(constructor), constructor);
            }
        }
        let unused = cursor.intern_name(b"Unused").unwrap();
        let unused = graph
            .binding(graph.own(graph.global, unused, Filter::Prefix).unwrap())
            .entity;
        assert!(graph.entity(unused).implicit_copies.is_none());
        let name = cursor.intern_name(b"B").unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        for &binding in &graph.entity(class).declared_constructors {
            let entity = graph.entity(graph.binding(binding).entity);
            if entity.copy_kind == Some(RefKind::Rvalue) {
                assert!(entity.constructor.is_none());
            }
        }
    }
}
