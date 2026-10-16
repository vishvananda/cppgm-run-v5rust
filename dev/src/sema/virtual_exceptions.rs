// Translated from sema/analyzer_exception_sets.cpp. See NOTICE.
//! Override exception subsets wait for complete classes and pending noexcept operands.
use super::{
    Error,
    analyzer::Analyzer,
    exceptions::{DestructorSpecification, ExceptionSet, ExceptionState},
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
pub(super) struct PendingOverride {
    function: Owner,
    overridden: Owner,
    location: SourceOffset,
    implicit_destructor: bool,
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
        if nonthrowing
            && !self.pending_virtual_noexcept(function)
            && !self.virtual_nothrow(function)?
        {
            return Err(Error::at(
                location,
                "an overrider allows exceptions its overridden function does not",
            ));
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
    ) {
        self.exception_overrides.push(PendingOverride {
            function: Owner::Function(function),
            overridden: destructor.map_or(Owner::Function(found), Owner::Destructor),
            location,
            implicit_destructor: false,
        });
    }
    pub(super) fn note_implicit_destructor_override(
        &mut self,
        class: EntityId,
        base: EntityId,
        location: SourceOffset,
    ) {
        self.exception_overrides.push(PendingOverride {
            function: Owner::Destructor(class),
            overridden: Owner::Destructor(base),
            location,
            implicit_destructor: true,
        });
    }
    pub(super) fn check_exception_overrides(&mut self, final_check: bool) -> Result<(), Error> {
        let mut pending = std::mem::take(&mut self.exception_overrides);
        if final_check {
            pending.append(&mut self.deferred_exception_overrides);
        }
        for edge in pending {
            let Some(allowed) = self.override_exception_set(edge.overridden, final_check)? else {
                self.defer_exception_override(edge, final_check)?;
                continue;
            };
            if allowed == ExceptionSet::All {
                continue;
            }
            let Some(thrown) = self.override_exception_set(edge.function, final_check)? else {
                self.defer_exception_override(edge, final_check)?;
                continue;
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
        }
        Ok(())
    }

    fn defer_exception_override(
        &mut self,
        edge: PendingOverride,
        final_check: bool,
    ) -> Result<(), Error> {
        if final_check {
            return Err(Error::Invariant(
                "unresolved override exception specification",
            ));
        }
        // Outermost-class completion consumes only that class's ready work.
        // A pending noexcept waits on the unit event, never on unrelated classes.
        self.deferred_exception_overrides.push(edge);
        Ok(())
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
        while let Some(owner) = work.pop() {
            match owner {
                Owner::Function(function) => {
                    let entity = self.graph.entity(function);
                    if entity
                        .member_function
                        .is_some_and(|key| key.kind == super::graph::MemberKind::Destructor)
                        && entity.destructor_specification == DestructorSpecification::Implicit
                    {
                        let class = entity
                            .member_function
                            .ok_or(Error::Invariant("destructor without class"))?
                            .class;
                        work.push(Owner::Destructor(class));
                        continue;
                    }
                    if matches!(entity.exceptions, ExceptionState::Pending(_)) {
                        if !final_check {
                            return Ok(None);
                        }
                        self.function_may_throw(function)?;
                    }
                    match &self.graph.entity(function).exceptions {
                        ExceptionState::Value(ExceptionSet::None) => (),
                        ExceptionState::Value(ExceptionSet::Dynamic(thrown)) => {
                            types.extend_from_slice(thrown);
                        }
                        ExceptionState::Value(ExceptionSet::All) | ExceptionState::Absent => {
                            return Ok(Some(ExceptionSet::All));
                        }
                        _ => return Ok(None),
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
                    work.extend(
                        entity
                            .bases
                            .iter()
                            .map(|edge| Owner::Destructor(edge.class)),
                    );
                    for &member in &entity.members {
                        let mut ty = self.graph.lower_entity_type(member)?;
                        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                            ty = element;
                        }
                        if let TypeKind::Class(child) = self.graph.types.get(ty).kind {
                            work.push(Owner::Destructor(child));
                        }
                    }
                }
            }
        }
        types.sort_unstable();
        types.dedup();
        Ok(Some(if types.is_empty() {
            ExceptionSet::None
        } else {
            ExceptionSet::Dynamic(types)
        }))
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
        let declarations = tree.decl_lists.get(declarations);
        let global = analyzer.graph.global;
        for &declaration in &declarations[..2] {
            analyzer.declaration(declaration, global).unwrap();
        }
        assert!(analyzer.exception_overrides.is_empty());
        assert_eq!(analyzer.deferred_exception_overrides.len(), 1);
        let binding = analyzer.graph.own(global, b, Filter::Elaborated).unwrap();
        let class = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity)
            .scope
            .unwrap();
        let binding = analyzer.graph.own(class, f, Filter::Ordinary).unwrap();
        let function = analyzer.graph.binding(binding).entity;
        analyzer.declaration(declarations[2], global).unwrap();
        assert!(analyzer.exception_overrides.is_empty());
        assert_eq!(analyzer.deferred_exception_overrides.len(), 1);
        assert!(matches!(
            analyzer.graph.entity(function).exceptions,
            ExceptionState::Pending(_)
        ));
        analyzer.finish_unit_exceptions().unwrap();
        assert!(analyzer.deferred_exception_overrides.is_empty());
        assert!(analyzer.virtual_nothrow(function).unwrap());
    }
}
