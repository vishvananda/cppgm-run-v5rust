// Translated from analyzer_copy.cpp and analyzer_construction.cpp. See NOTICE.
//! Checked same-class prvalue copy elision. Selection is retained independently
//! of operand conversions; an elided constructor never contributes body demand.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{Conversion, ValueCategory as V},
    function_names::{SpecialName, ValueKey},
    graph::{BindingId, EntityId, FunctionAvailability, ScopeId},
    special_members::SpecialDefinition,
    types::{Cv, RefKind, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::SimpleToken},
    support::source::SourceOffset,
    syntax::ast,
};
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
#[derive(Clone, Copy)]
pub(super) struct ImplicitCopies {
    copy: Option<(TypeId, bool)>,
    moving: Option<TypeId>,
    copy_deleted: bool,
}
#[derive(Clone, Copy)]
struct Candidate {
    selection: CopySelection,
    deleted: bool,
    elidable: bool,
}

#[derive(Default)]
pub(super) struct CopyScratch {
    candidates: Vec<Candidate>,
}

impl Analyzer<'_> {
    fn classify_copy_kind(
        &self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<Option<RefKind>, Error> {
        let function = self.graph.entity(self.graph.binding(binding).entity);
        if function.template_head.is_some() {
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
        Ok(())
    }

    pub(super) fn declared_copy_kind(
        &self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<Option<RefKind>, Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        if !entity.member_function.is_some_and(|key| key.class == class) {
            return Err(Error::Invariant("copy constructor of another class"));
        }
        Ok(entity.copy_kind)
    }

    pub(super) fn scalar_copy_deleted(
        &self,
        class: EntityId,
        kind: RefKind,
    ) -> Result<bool, Error> {
        if !self.graph.entity(class).bases.is_empty() {
            return Err(Error::Missing("base subobject copy and move plans"));
        }
        let mut deleted = false;
        for &member in &self.graph.entity(class).members {
            let mut ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("copy member without type"))?;
            if matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Reference {
                    kind: RefKind::Rvalue,
                    ..
                }
            ) {
                deleted |= kind == RefKind::Lvalue;
            }
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                return Err(Error::Missing("class subobject copy and move plans"));
            }
        }
        Ok(deleted)
    }

    fn implicit_copy_candidates(&mut self, class: EntityId) -> Result<ImplicitCopies, Error> {
        if let Some(copies) = self.graph.entity(class).implicit_copies {
            return Ok(copies);
        }
        // Immutable scalar subobjects are checked once at this class demand.
        let copy_deleted = self.scalar_copy_deleted(class, RefKind::Lvalue)?;
        let mut has_copy = false;
        let mut has_move = false;
        for &binding in &self.graph.entity(class).declared_constructors {
            match self.declared_copy_kind(binding, class)? {
                Some(RefKind::Lvalue) => has_copy = true,
                Some(RefKind::Rvalue) => has_move = true,
                None => (),
            }
        }
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("copy class without scope"))?;
        let assignment = ValueKey::Special(SpecialName::Operator {
            token: SimpleToken::punctuator(P::Assign)
                .ok_or(Error::Invariant("assignment token"))?,
            array: false,
        });
        let mut copy_assignment = false;
        let mut move_assignment = false;
        for binding in self.graph.functions_at(home, assignment) {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() || entity.function.is_some_and(|p| p.is_static) {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("assignment without type"))?;
            if self.graph.types.function_parameter_count(ty)? != 1
                || matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Function { variadic: true, .. }
                )
            {
                continue;
            }
            let parameter = self.graph.types.function_parameter(ty, 0)?;
            let (element, moving) = match self.graph.types.get(parameter).kind {
                TypeKind::Reference { element, kind } => (element, kind == RefKind::Rvalue),
                _ => (parameter, false),
            };
            if matches!(self.graph.types.get(element).kind, TypeKind::Class(owner) if owner == class)
            {
                move_assignment |= moving;
                copy_assignment |= !moving;
            }
        }
        let destructor = self.destructor_callable(class)?.is_some();
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("copy class without type"))?;
        let copy = if has_copy {
            None
        } else {
            let deleted = has_move || move_assignment || copy_deleted;
            let element = self.graph.types.qualify(ty, Cv::CONST)?;
            Some((
                self.graph.types.reference(element, RefKind::Lvalue)?,
                deleted,
            ))
        };
        let moving = if has_copy || has_move || copy_assignment || move_assignment || destructor {
            None
        } else {
            Some(self.graph.types.reference(ty, RefKind::Rvalue)?)
        };
        let copies = ImplicitCopies {
            copy,
            moving,
            copy_deleted,
        };
        self.graph.entity_mut(class).implicit_copies = Some(copies);
        Ok(copies)
    }

    pub(super) fn elide_class_copy(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
        direct: bool,
        in_place: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Class(class) = self.graph.types.get(target).kind else {
            return Ok(());
        };
        if !self.graph.entity(class).complete || self.graph.entity(class).scope.is_none() {
            return Err(Error::at(
                location,
                "initialization of an incomplete class object",
            ));
        }
        let facts = self.graph.expression_facts.get(id)?;
        if facts.functions.is_some()
            || !matches!(self.graph.types.get(facts.ty).kind, TypeKind::Class(from) if from == class)
        {
            return Err(Error::Missing(
                "converting and derived class initialization",
            ));
        }
        if !self.graph.entity(class).declared_conversions.is_empty() {
            return Err(Error::Missing(
                "copy construction with user conversion candidates",
            ));
        }
        if facts.category != V::Prvalue
            || !in_place
                && matches!(
                    self.tree
                        .expressions
                        .get(self.graph.expression_facts.root(id))
                        .kind,
                    ast::ExprKind::Conditional { .. }
                )
        {
            return Err(Error::Missing("nonelided class copy and move construction"));
        }
        let mut scratch = std::mem::take(&mut self.copy_scratch);
        let selected = self.select_elided_copy(
            class,
            facts,
            scope,
            direct,
            location,
            &mut scratch.candidates,
        );
        scratch.candidates.clear();
        self.copy_scratch = scratch;
        let mut facts = facts;
        facts.elided_copy = Some(selected?);
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }

    pub(super) fn check_final_copy_selection(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.select_copy_initialization(class, facts, scope, location)
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
    pub(super) fn select_copy_initialization(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<CopySelection, Error> {
        let mut scratch = std::mem::take(&mut self.copy_scratch);
        let selected =
            self.select_elided_copy(class, facts, scope, true, location, &mut scratch.candidates);
        scratch.candidates.clear();
        self.copy_scratch = scratch;
        selected
    }
    fn select_elided_copy(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        scope: ScopeId,
        direct: bool,
        location: SourceOffset,
        candidates: &mut Vec<Candidate>,
    ) -> Result<CopySelection, Error> {
        let copies = self.implicit_copy_candidates(class)?;
        candidates.clear();
        let mut offered = copies.copy.is_some() || copies.moving.is_some();
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if !direct && entity.explicitness == super::specifiers::Explicitness::Explicit {
                continue;
            }
            offered = true;
            let ty = entity
                .ty
                .ok_or(Error::Invariant("constructor without type"))?;
            let count = self.graph.types.function_parameter_count(ty)?;
            if count == 0
                || (1..count).any(|i| {
                    self.graph
                        .function_default(self.graph.binding(binding).entity, i)
                        .is_none()
                })
            {
                continue;
            }
            if entity.template_head.is_some() {
                return Err(Error::Missing("copy constructor template deduction"));
            }
            let defaulted = matches!(
                entity.special_definition,
                Some(SpecialDefinition::Defaulted { .. })
            );
            let explicitly_deleted = entity.availability == FunctionAvailability::Deleted;
            let parameter = self.graph.types.function_parameter(ty, 0)?;
            let Some(conversion) = self.scalar_sequence(facts, parameter, false)? else {
                continue;
            };
            let kind = self.declared_copy_kind(binding, class)?;
            let deleted = explicitly_deleted
                || defaulted && kind == Some(RefKind::Lvalue) && copies.copy_deleted;
            if defaulted && deleted && kind == Some(RefKind::Rvalue) {
                continue;
            }
            candidates.push(Candidate {
                selection: CopySelection {
                    origin: CopyOrigin::Declared(binding),
                    binding: conversion,
                },
                deleted,
                elidable: kind.is_some(),
            });
        }
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
            if let Some(conversion) = self.scalar_sequence(facts, parameter, false)? {
                candidates.push(Candidate {
                    selection: CopySelection {
                        origin: CopyOrigin::Implicit { class, kind },
                        binding: conversion,
                    },
                    deleted,
                    elidable: true,
                });
            }
        }
        let mut best = 0;
        for index in 1..candidates.len() {
            if self.compare_sequences(
                candidates[index].selection.binding,
                candidates[best].selection.binding,
            )? == Ordering::Greater
            {
                best = index;
            }
        }
        let chosen = candidates.get(best).copied().ok_or_else(|| {
            Error::at(
                location,
                if offered {
                    "no viable function for the call"
                } else {
                    "no constructor for the initialization"
                },
            )
        })?;
        for (index, candidate) in candidates.iter().enumerate() {
            if index != best
                && self.compare_sequences(chosen.selection.binding, candidate.selection.binding)?
                    != Ordering::Greater
            {
                return Err(Error::at(location, "ambiguous call to overloaded function"));
            }
        }
        if chosen.deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        if !chosen.elidable
            || !matches!(
                self.graph.types.get(chosen.selection.binding.target).kind,
                TypeKind::Reference { .. }
            )
            || chosen.selection.binding.temporary
        {
            return Err(Error::Missing("nonelided constructor argument conversion"));
        }
        if let CopyOrigin::Declared(binding) = chosen.selection.origin {
            self.check_direct_member_access(binding, scope, location)?;
        }
        Ok(chosen.selection)
    }
}
