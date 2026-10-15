// Translated from analyzer_copy.cpp and analyzer_construction.cpp. See NOTICE.
//! Same-class construction retains selection separately from operand conversions.
//! Elision, storage transfer and constructor calls have distinct demand boundaries.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{Conversion, ValueCategory as V},
    graph::{BindingId, EntityId, FunctionAvailability, ScopeId},
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
    Storage { reference: TypeId, skip_empty: bool },
    Constructor(super::construction::ConstructorId),
}
#[derive(Clone, Copy)]
pub(super) enum CopyUse {
    Object { direct: bool },
    Value { in_place: bool },
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
    deleted: bool,
    elidable: bool,
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
        if !self.graph.entity(class).complete || self.graph.entity(class).scope.is_none() {
            return Err(Error::at(
                location,
                "initialization of an incomplete class object",
            ));
        }
        let facts = self.graph.expression_facts.get(id)?;
        let direct = matches!(usage, CopyUse::Object { direct: true });
        let in_place = matches!(usage, CopyUse::Value { in_place: true });
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
        let elided = facts.category == V::Prvalue
            && (in_place
                || !matches!(
                    self.tree
                        .expressions
                        .get(self.graph.expression_facts.root(id))
                        .kind,
                    ast::ExprKind::Conditional { .. }
                ));
        if !elided && facts.category == V::Prvalue {
            return Err(Error::Missing(
                "conditional class temporary copy construction",
            ));
        }
        let selected = self.select_copy_sequence(class, facts, Some(scope), direct, location)?;
        let mut facts = facts;
        let action = if elided {
            None
        } else {
            Some(self.copy_action(class, selected, usage, location)?)
        };
        self.graph.expression_facts.set_copy_action(id, action);
        facts.class_copy = Some(selected);
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }

    fn copy_action(
        &mut self,
        class: EntityId,
        selected: CopySelection,
        usage: CopyUse,
        location: SourceOffset,
    ) -> Result<CopyAction, Error> {
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
            return Ok(CopyAction::Constructor(constructor));
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
        self.select_copy_sequence(class, facts, Some(scope), true, location)
    }
    pub(super) fn select_subobject_copy(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
    ) -> Result<CopySelection, Error> {
        // Preserve the selected identity even when access deletes the enclosing
        // defaulted member. Its owner checks base/member protected access next.
        self.select_copy_sequence(class, facts, None, true, SourceOffset(0))
    }
    fn select_copy_sequence(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        access_scope: Option<ScopeId>,
        direct: bool,
        location: SourceOffset,
    ) -> Result<CopySelection, Error> {
        let mut scratch = std::mem::take(&mut self.copy_scratch);
        let selected = self.select_class_copy(
            class,
            facts,
            access_scope,
            direct,
            location,
            &mut scratch.candidates,
        );
        scratch.candidates.clear();
        self.copy_scratch = scratch;
        selected
    }
    fn select_class_copy(
        &mut self,
        class: EntityId,
        facts: super::expression_facts::ExprFacts,
        access_scope: Option<ScopeId>,
        direct: bool,
        location: SourceOffset,
        candidates: &mut Vec<Candidate>,
    ) -> Result<CopySelection, Error> {
        // A standard reference binding from an unrelated source cannot select
        // an implicit copy or move member (13.3.3.1p4). Reject that sequence
        // before demanding the destination's subobject copy properties.
        let implicit_binding = self
            .graph
            .class_of(facts.ty)
            .is_some_and(|source| source == class || self.graph.is_base_of(class, source));
        let copies = if implicit_binding {
            Some(self.implicit_copy_candidates(class)?)
        } else {
            None
        };
        candidates.clear();
        let mut offered =
            !implicit_binding || copies.is_some_and(|c| c.copy.is_some() || c.moving.is_some());
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
            let copy_properties = entity.copy_properties;
            let parameter = self.graph.types.function_parameter(ty, 0)?;
            let Some(conversion) = self.scalar_sequence(facts, parameter, false)? else {
                continue;
            };
            let kind = self.declared_copy_kind(binding, class)?;
            let deleted = explicitly_deleted
                || copy_properties.is_some_and(|id| self.graph.copy_properties.get(id).deleted);
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
                .and_then(|c| c.copy)
                .map(|(parameter, deleted)| (RefKind::Lvalue, parameter, deleted)),
            copies
                .and_then(|c| c.moving)
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
        if let CopyOrigin::Declared(binding) = chosen.selection.origin
            && let Some(scope) = access_scope
        {
            self.check_direct_member_access(binding, scope, location)?;
        }
        Ok(chosen.selection)
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
}
