// Translated from sema/analyzer_exception_sets.cpp. See NOTICE.
//! Override exception subsets wait for complete classes and pending noexcept operands.
use super::{
    Error,
    analyzer::Analyzer,
    exceptions::{DestructorSpecification, ExceptionSet, ExceptionState, SpecificationOrigin},
    graph::{Access, EntityId},
    types::{Scalar, TypeId, TypeKind},
};
use crate::support::source::SourceOffset;
use std::collections::HashSet;

#[derive(Clone, Copy)]
enum Owner {
    Function(EntityId),
    Destructor(EntityId),
}

#[derive(Clone, Copy)]
pub(super) struct PendingNonthrowingOverride {
    function: EntityId,
    location: SourceOffset,
}

#[derive(Clone, Copy)]
pub(super) struct PendingOverride {
    function: Owner,
    overridden: Owner,
    location: SourceOffset,
    implicit_destructor: bool,
}

enum OverrideCheck {
    Complete,
    Deferred,
}

enum FunctionExceptions<'a> {
    All,
    Nonthrowing,
    Dynamic(&'a [TypeId]),
    Destructor(EntityId),
    Deferred,
}

impl Analyzer<'_> {
    pub(super) fn check_override_nonthrowing(
        &mut self,
        function: EntityId,
        found: EntityId,
        destructor: Option<EntityId>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if destructor.is_some()
            && self.graph.entity(function).destructor_specification
                == DestructorSpecification::Implicit
        {
            return Ok(());
        }
        let nonthrowing = if let Some(class) = destructor {
            match self
                .graph
                .entity(class)
                .destruction
                .ok_or(Error::Invariant("overridden destructor without properties"))?
                .unwind
            {
                super::destruction::DestructorUnwind::Implicit { nothrow } => nothrow,
                super::destruction::DestructorUnwind::Declared(function) => {
                    self.virtual_nothrow(function)?
                }
            }
        } else {
            self.virtual_nothrow(found)?
        };
        if nonthrowing {
            if self.pending_virtual_noexcept(function) {
                self.graph
                    .analysis
                    .nonthrowing_overrides
                    .push(PendingNonthrowingOverride { function, location });
            } else if !self.virtual_nothrow(function)? {
                return Err(Error::at(
                    location,
                    "an overrider allows exceptions its overridden function does not",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn check_pending_nonthrowing_overrides(&mut self) -> Result<(), Error> {
        // This unit queue is independent of class-local finite-set events.
        let pending = std::mem::take(&mut self.graph.analysis.nonthrowing_overrides);
        for event in pending {
            if self.function_may_throw(event.function)? {
                return Err(Error::at(
                    event.location,
                    "an overrider allows exceptions its overridden function does not",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn check_completed_destructor_override(
        &mut self,
        class: EntityId,
        base: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let implicit = self
            .graph
            .entity(class)
            .declared_destructor
            .is_none_or(|binding| {
                self.graph
                    .entity(self.graph.binding(binding).entity)
                    .destructor_specification
                    == DestructorSpecification::Implicit
            });
        if implicit
            && self.override_exception_set(Owner::Destructor(base), false)?
                == Some(ExceptionSet::None)
            && let Some(thrown) = self.override_exception_set(Owner::Destructor(class), false)?
            && thrown != ExceptionSet::None
        {
            return Err(Error::at(
                location,
                "a destructor allows exceptions the destructor it overrides does not",
            ));
        }
        let deleted = |class| {
            self.graph
                .entity(class)
                .destruction
                .map(|facts| facts.deleted)
        };
        if deleted(class) != deleted(base) {
            return Err(Error::at(
                location,
                "a deleted destructor and a destructor not deleted override one another",
            ));
        }
        Ok(())
    }
    pub(super) fn note_exception_override(
        &mut self,
        function: EntityId,
        found: EntityId,
        destructor: Option<EntityId>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if destructor.is_none()
            && self.graph.entity(found).exception_origin == SpecificationOrigin::Omitted
        {
            return Ok(());
        }
        self.note_pending_exception_override(PendingOverride {
            function: Owner::Function(function),
            overridden: destructor.map_or(Owner::Function(found), Owner::Destructor),
            location,
            implicit_destructor: false,
        })
    }
    pub(super) fn note_implicit_destructor_override(
        &mut self,
        class: EntityId,
        base: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.note_pending_exception_override(PendingOverride {
            function: Owner::Destructor(class),
            overridden: Owner::Destructor(base),
            location,
            implicit_destructor: true,
        })
    }
    fn note_pending_exception_override(&mut self, edge: PendingOverride) -> Result<(), Error> {
        let first = self.graph.analysis.exception_overrides.len();
        self.graph.analysis.exception_overrides.push(edge);
        if self.class_depth == 0 && self.graph.analysis.pending_throws.is_empty() {
            self.check_exception_overrides(first, false)?;
        }
        Ok(())
    }
    pub(super) fn check_exception_overrides(
        &mut self,
        first: usize,
        final_check: bool,
    ) -> Result<(), Error> {
        let mut index = first;
        let mut kept = first;
        // Copy each event before querying: a nested class may append and
        // complete its own suffix. Visit the live tail after earlier events.
        while index < self.graph.analysis.exception_overrides.len() {
            let edge = self.graph.analysis.exception_overrides[index];
            if matches!(
                self.check_exception_override(edge, final_check)?,
                OverrideCheck::Deferred
            ) {
                self.graph.analysis.exception_overrides[kept] = edge;
                kept += 1;
            }
            index += 1;
        }
        self.graph.analysis.exception_overrides.truncate(kept);
        Ok(())
    }

    fn check_exception_override(
        &mut self,
        edge: PendingOverride,
        final_check: bool,
    ) -> Result<OverrideCheck, Error> {
        if !final_check
            && [edge.function, edge.overridden].into_iter().any(|owner| {
                let function = match owner {
                    Owner::Function(function) => Some(function),
                    Owner::Destructor(class) => self
                        .graph
                        .entity(class)
                        .declared_destructor
                        .map(|binding| self.graph.binding(binding).entity),
                };
                function.is_some_and(|function| self.pending_virtual_noexcept(function))
            })
        {
            return Ok(OverrideCheck::Deferred);
        }
        let Some(allowed) = self.override_exception_set(edge.overridden, final_check)? else {
            return Self::unresolved_override(final_check);
        };
        if allowed == ExceptionSet::All
            || matches!(&allowed, ExceptionSet::Dynamic(types) if types.is_empty())
            || allowed == ExceptionSet::None && !edge.implicit_destructor
        {
            return Ok(OverrideCheck::Complete);
        }
        let Some(thrown) = self.override_exception_set(edge.function, final_check)? else {
            return Self::unresolved_override(final_check);
        };
        let nonthrowing_destructor = edge.implicit_destructor && allowed == ExceptionSet::None;
        let allows = match (allowed, thrown) {
            (ExceptionSet::All, _) | (_, ExceptionSet::None) => true,
            (ExceptionSet::None, _) | (_, ExceptionSet::All) => false,
            (ExceptionSet::Dynamic(allowed), ExceptionSet::Dynamic(thrown)) => {
                self.exception_subset(&allowed, &thrown)?
            }
        };
        if !allows {
            return Err(Error::at(
                edge.location,
                if nonthrowing_destructor {
                    "a destructor allows exceptions the destructor it overrides does not"
                } else {
                    "an overrider allows exceptions its overridden function does not"
                },
            ));
        }
        Ok(OverrideCheck::Complete)
    }

    fn unresolved_override(final_check: bool) -> Result<OverrideCheck, Error> {
        if final_check {
            Err(Error::Invariant(
                "unresolved override exception specification",
            ))
        } else {
            Ok(OverrideCheck::Deferred)
        }
    }

    fn exception_subset(&mut self, allowed: &[TypeId], thrown: &[TypeId]) -> Result<bool, Error> {
        for &thrown in thrown {
            let mut matched = false;
            for &handler in allowed {
                if self.override_handler_allows(handler, thrown)? {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn override_exception_set(
        &mut self,
        root: Owner,
        final_check: bool,
    ) -> Result<Option<ExceptionSet>, Error> {
        let mut work = vec![root];
        let mut seen_classes = HashSet::new();
        let mut types = Vec::new();
        let mut finite = false;
        while let Some(owner) = work.pop() {
            match owner {
                Owner::Function(function) => {
                    match self.function_exception_set(function, final_check)? {
                        FunctionExceptions::Nonthrowing => (),
                        FunctionExceptions::Dynamic(thrown) => {
                            finite = true;
                            types.extend_from_slice(thrown);
                        }
                        FunctionExceptions::All => return Ok(Some(ExceptionSet::All)),
                        FunctionExceptions::Destructor(class) => {
                            work.push(Owner::Destructor(class));
                        }
                        FunctionExceptions::Deferred => return Ok(None),
                    }
                }
                Owner::Destructor(class) => {
                    if !seen_classes.insert(class) {
                        continue;
                    }
                    let entity = self.graph.entity(class);
                    if let Some(binding) = entity.declared_destructor {
                        let function = self.graph.binding(binding).entity;
                        if self.graph.entity(function).destructor_specification
                            == DestructorSpecification::Explicit
                        {
                            work.push(Owner::Function(function));
                            continue;
                        }
                    }
                    // Push in reverse execution order: direct bases, indirect
                    // virtual bases, then members, each in source order.
                    for &member in entity.members.iter().rev() {
                        let mut ty = self.graph.lower_entity_type(member)?;
                        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                            ty = element;
                        }
                        if let TypeKind::Class(child) = self.graph.types.get(ty).kind {
                            work.push(Owner::Destructor(child));
                        }
                    }
                    work.extend(
                        self.graph
                            .indirect_virtual_bases(class)
                            .into_iter()
                            .rev()
                            .map(Owner::Destructor),
                    );
                    work.extend(
                        entity
                            .bases
                            .iter()
                            .rev()
                            .map(|edge| Owner::Destructor(edge.class)),
                    );
                }
            }
        }
        Ok(Some(if types.is_empty() && !finite {
            ExceptionSet::None
        } else {
            ExceptionSet::Dynamic(types)
        }))
    }

    fn function_exception_set(
        &mut self,
        function: EntityId,
        final_check: bool,
    ) -> Result<FunctionExceptions<'_>, Error> {
        let entity = self.graph.entity(function);
        if let Some(member) = entity.member_function
            && member.kind == super::graph::MemberKind::Destructor
            && entity.destructor_specification == DestructorSpecification::Implicit
        {
            return Ok(FunctionExceptions::Destructor(member.class));
        }
        let dynamic = self.pending_dynamic_throw(function);
        if matches!(entity.exceptions, ExceptionState::Pending(_)) && !dynamic {
            if !final_check {
                return Ok(FunctionExceptions::Deferred);
            }
            self.function_may_throw(function)?;
        }
        let written = self.graph.source_exceptions.specification(function);
        Ok(match &self.graph.entity(function).exceptions {
            ExceptionState::Value(ExceptionSet::None) => FunctionExceptions::Nonthrowing,
            ExceptionState::Value(ExceptionSet::Dynamic(types)) => {
                FunctionExceptions::Dynamic(written.unwrap_or(types))
            }
            ExceptionState::Pending(_) if dynamic => {
                written.map_or(FunctionExceptions::All, FunctionExceptions::Dynamic)
            }
            ExceptionState::Value(ExceptionSet::All)
            | ExceptionState::Absent
            | ExceptionState::Entered => {
                written.map_or(FunctionExceptions::All, FunctionExceptions::Dynamic)
            }
            _ => FunctionExceptions::Deferred,
        })
    }

    fn public_exception_base(&self, base: EntityId, derived: EntityId) -> bool {
        if self.graph.base_count(derived, base) != 1 {
            return false;
        }
        let mut work = vec![derived];
        let mut seen = HashSet::new();
        while let Some(class) = work.pop() {
            if !seen.insert(class) {
                continue;
            }
            for edge in &self.graph.entity(class).bases {
                if edge.access == Access::Public {
                    if edge.class == base {
                        return true;
                    }
                    work.push(edge.class);
                }
            }
        }
        false
    }

    fn override_handler_allows(&mut self, handler: TypeId, thrown: TypeId) -> Result<bool, Error> {
        let reference = matches!(
            self.graph.types.get(handler).kind,
            TypeKind::Reference { .. }
        );
        let referred =
            if let TypeKind::Reference { element, .. } = self.graph.types.get(handler).kind {
                element
            } else {
                handler
            };
        let target = self.graph.types.parameter(referred)?;
        let exception =
            if let TypeKind::Reference { element, .. } = self.graph.types.get(thrown).kind {
                element
            } else {
                thrown
            };
        let exception = self.graph.types.parameter(exception)?;
        if target == exception {
            return Ok(true);
        }
        let tk = &self.graph.types.get(target).kind;
        let ek = &self.graph.types.get(exception).kind;
        if let (TypeKind::Class(to), TypeKind::Class(from)) = (tk, ek) {
            return Ok(self.public_exception_base(*to, *from));
        }
        if !matches!(tk, TypeKind::Pointer(_) | TypeKind::MemberPointer { .. })
            || reference && self.graph.types.get(referred).cv.0 != 1
        {
            return Ok(false);
        }
        if matches!(ek, TypeKind::Scalar(Scalar::Nullptr)) {
            return Ok(true);
        }
        if self.pointer_qualification(exception, target)? {
            return Ok(true);
        }
        let (TypeKind::Pointer(to), TypeKind::Pointer(from)) = (tk, ek) else {
            return Ok(false);
        };
        if self.graph.types.get(*from).cv.0 & !self.graph.types.get(*to).cv.0 != 0 {
            return Ok(false);
        }
        if matches!(
            self.graph.types.get(*to).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return Ok(!matches!(
                self.graph.types.get(*from).kind,
                TypeKind::Function { .. }
            ));
        }
        Ok(
            match (self.graph.class_of(*to), self.graph.class_of(*from)) {
                (Some(to), Some(from)) => self.public_exception_base(to, from),
                _ => false,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Analyzer;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{exceptions::ExceptionState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn override_handler_candidates_keep_written_order_and_repetitions() {
        let source = SourceBuffer::new(
            b"struct A{};A* earlier();struct B{virtual void f()throw(void*,A*,A*);};".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"exception-handler-order").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let b = cursor.intern_name(b"B").unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let binding = analyzer
            .graph
            .own(analyzer.graph.global, b, Filter::Elaborated)
            .unwrap();
        let class = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity)
            .scope
            .unwrap();
        let binding = analyzer.graph.own(class, f, Filter::Ordinary).unwrap();
        let function = analyzer.graph.binding(binding).entity;
        let written = analyzer
            .graph
            .source_exceptions
            .specification(function)
            .unwrap()
            .to_vec();
        let allowed = analyzer
            .override_exception_set(super::Owner::Function(function), true)
            .unwrap();
        assert!(matches!(allowed, Some(super::ExceptionSet::Dynamic(types)) if types == written));
    }

    #[test]
    fn pending_noexcept_waits_on_the_unit_instead_of_unrelated_class_events() {
        let source = SourceBuffer::new(b"struct A{virtual void f()noexcept;};struct B:A{void f()noexcept(1==1)override;};struct Unrelated{};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"override-events").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let b = cursor.intern_name(b"B").unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let declarations = analyzer.tree.decl_lists.get(declarations).to_vec();
        let global = analyzer.graph.global;
        for &declaration in &declarations[..2] {
            analyzer.declaration(declaration, global).unwrap();
        }
        assert_eq!(analyzer.graph.analysis.exception_overrides.len(), 1);
        let binding = analyzer.graph.own(global, b, Filter::Elaborated).unwrap();
        let class = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity)
            .scope
            .unwrap();
        let binding = analyzer.graph.own(class, f, Filter::Ordinary).unwrap();
        let function = analyzer.graph.binding(binding).entity;
        analyzer.declaration(declarations[2], global).unwrap();
        assert_eq!(analyzer.graph.analysis.exception_overrides.len(), 1);
        assert!(matches!(
            analyzer.graph.entity(function).exceptions,
            ExceptionState::Pending(_)
        ));
        analyzer.finish_unit_exceptions().unwrap();
        assert!(analyzer.graph.analysis.exception_overrides.is_empty());
        assert!(analyzer.virtual_nothrow(function).unwrap());
    }
}
