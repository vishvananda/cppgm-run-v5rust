// Translated from sema/analyzer_construction.cpp's destructor declaration and checks.
// See NOTICE.
//! Completed classes retain destructor properties; implicit callable identities
//! are created only when a use needs them. References never own their referents.
use super::{
    Error,
    analyzer::Analyzer,
    exceptions::{DestructorSpecification, ExceptionSet, ExceptionState},
    graph::{BindingId, EntityId, FunctionAvailability, Graph},
    special_members::SpecialDefinition,
    types::{Cv, Scalar, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Preparation {
    #[default]
    Pending,
    Ready,
}
#[derive(Clone, Copy)]
pub(super) struct Destruction {
    pub declared_name: Option<crate::lex::identifiers::NameId>,
    pub trivial: bool,
    pub deleted: bool,
    pub unwind: DestructorUnwind,
}
#[derive(Clone, Copy)]
pub(super) enum DestructorUnwind {
    Implicit { nothrow: bool },
    Declared(EntityId),
}
struct SubobjectDestruction {
    trivial: bool,
    deleted: bool,
}
impl Graph {
    fn destruction_child(
        &self,
        class: EntityId,
        index: usize,
        indirect: &[EntityId],
    ) -> Result<Option<(EntityId, bool)>, Error> {
        let entity = self.entity(class);
        if let Some(base) = entity.bases.get(index) {
            return Ok(Some((base.class, false)));
        }
        let index = index - entity.bases.len();
        if let Some(&base) = indirect.get(index) {
            return Ok(Some((base, false)));
        }
        let member = entity.members[index - indirect.len()];
        let mut ty = self
            .entity(member)
            .ty
            .ok_or(Error::Invariant("member without type"))?;
        while let TypeKind::Array { element, .. } = self.types.get(ty).kind {
            ty = element;
        }
        Ok(if let TypeKind::Class(child) = self.types.get(ty).kind {
            Some((child, true))
        } else {
            None
        })
    }
    /// Output destroys existing subobjects only. Property/access queries above
    /// still inspect variant members and the element type of an empty array.
    fn destruction_output_child(
        &self,
        class: EntityId,
        index: usize,
    ) -> Result<Option<EntityId>, Error> {
        let entity = self.entity(class);
        if index >= entity.bases.len() {
            if entity.key == Keyword::Union {
                return Ok(None);
            }
            let member = entity.members[index - entity.bases.len()];
            let mut ty = self
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            while let TypeKind::Array { element, bound, .. } = self.types.get(ty).kind {
                if bound == 0 {
                    return Ok(None);
                }
                ty = element;
            }
        }
        Ok(self
            .destruction_child(class, index, &[])?
            .map(|(child, _)| child))
    }
    pub(crate) fn lower_destructor_subobjects_throw(&self, class: EntityId) -> Result<bool, Error> {
        let count = self.entity(class).bases.len()
            + if self.entity(class).key == Keyword::Union {
                0
            } else {
                self.entity(class).members.len()
            };
        for index in 0..count {
            if let Some((child, _)) = self.destruction_child(class, index, &[])? {
                let facts = self.entity(child).destruction.ok_or(Error::Invariant(
                    "subobject destructor properties not complete",
                ))?;
                let nothrow = match facts.unwind {
                    DestructorUnwind::Implicit { nothrow } => nothrow,
                    DestructorUnwind::Declared(function) => matches!(
                        self.entity(function).exceptions,
                        ExceptionState::Value(ExceptionSet::None)
                    ),
                };
                if !facts.trivial && !nothrow {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub(super) fn destructor_binding(&self, class: EntityId) -> Option<BindingId> {
        let entity = self.entity(class);
        entity.declared_destructor.or(entity.implicit_destructor)
    }
    pub(super) fn destructor_trivial(&self, class: EntityId) -> Result<bool, Error> {
        self.entity(class)
            .destruction
            .map(|facts| facts.trivial)
            .ok_or(Error::Invariant("destructor properties not complete"))
    }
}
impl Analyzer<'_> {
    pub(super) fn implicit_destructor_nothrow(&mut self, class: EntityId) -> Result<bool, Error> {
        let indirect = self.graph.indirect_virtual_bases(class);
        let count = self.graph.entity(class).bases.len()
            + indirect.len()
            + self.graph.entity(class).members.len();
        for index in 0..count {
            let Some((child, _)) = self.graph.destruction_child(class, index, &indirect)? else {
                continue;
            };
            let facts = self
                .graph
                .entity(child)
                .destruction
                .ok_or(Error::Invariant(
                    "subobject destructor properties not complete",
                ))?;
            let nothrow = match facts.unwind {
                DestructorUnwind::Implicit { nothrow } => nothrow,
                DestructorUnwind::Declared(function) => !self.function_may_throw(function)?,
            };
            if !nothrow {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn subobject_destructor_facts(
        &mut self,
        class: EntityId,
    ) -> Result<SubobjectDestruction, Error> {
        let scope = self.graph.entity(class).scope.ok_or(Error::Invariant(
            "destructor properties without class scope",
        ))?;
        let mut facts = SubobjectDestruction {
            trivial: true,
            deleted: false,
        };
        let indirect = self.graph.indirect_virtual_bases(class);
        let count = self.graph.entity(class).bases.len()
            + indirect.len()
            + self.graph.entity(class).members.len();
        for index in 0..count {
            let Some((child, member)) = self.graph.destruction_child(class, index, &indirect)?
            else {
                continue;
            };
            let child_facts = self
                .graph
                .entity(child)
                .destruction
                .ok_or(Error::Invariant(
                    "subobject destructor properties not complete",
                ))?;
            facts.trivial &= child_facts.trivial;
            facts.deleted |= child_facts.deleted
                || self.graph.entity(class).key == Keyword::Union && !child_facts.trivial;
            if let Some(binding) = self.graph.destructor_binding(child) {
                facts.deleted |= self
                    .check_special_member_access(
                        binding,
                        if member { child } else { class },
                        scope,
                        SourceOffset(0),
                    )
                    .is_err();
            }
        }
        Ok(facts)
    }
    pub(super) fn defaulted_destructor_deleted(&mut self, class: EntityId) -> Result<bool, Error> {
        // An out-of-class defaulted definition is user-provided. Check the
        // implicit definition's deletedness without demanding storage or
        // changing the completed class's triviality/exception specification.
        Ok(self.subobject_destructor_facts(class)?.deleted)
    }
    pub(super) fn complete_destructor(&mut self, class: EntityId) -> Result<(), Error> {
        let declared = self.graph.entity(class).declared_destructor;
        let explicit = declared
            .map(|binding| self.graph.binding(binding).entity)
            .filter(|&function| {
                self.graph.entity(function).destructor_specification
                    == DestructorSpecification::Explicit
            });
        // An explicit specification remains declaration-owned and lazy. In
        // particular a nested destructor's operand may name the incomplete
        // enclosing class; declare_destructor does not evaluate it here.
        let unwind = if let Some(function) = explicit {
            DestructorUnwind::Declared(function)
        } else {
            DestructorUnwind::Implicit {
                nothrow: self.implicit_destructor_nothrow(class)?,
            }
        };
        let subobjects = self.subobject_destructor_facts(class)?;
        let mut facts = Destruction {
            declared_name: self.graph.entity(class).name,
            trivial: subobjects.trivial,
            deleted: subobjects.deleted,
            unwind,
        };
        if let Some(binding) = declared {
            let id = self.graph.binding(binding).entity;
            let entity = self.graph.entity(id);
            let defaulted = matches!(
                entity.special_definition,
                Some(SpecialDefinition::Defaulted { first: true, .. } | SpecialDefinition::Deleted)
            );
            facts.trivial &= defaulted;
            facts.deleted =
                entity.availability == FunctionAvailability::Deleted || defaulted && facts.deleted;
            if entity.destructor_specification == DestructorSpecification::Implicit {
                let DestructorUnwind::Implicit { nothrow } = facts.unwind else {
                    return Err(Error::Invariant("implicit destructor with explicit unwind"));
                };
                self.graph.entity_mut(id).exceptions = ExceptionState::Value(if nothrow {
                    ExceptionSet::None
                } else {
                    ExceptionSet::All
                });
            }
            if facts.deleted {
                self.graph.entity_mut(id).availability = FunctionAvailability::Deleted;
            }
        }
        self.graph.entity_mut(class).destruction = Some(facts);
        Ok(())
    }
    pub(super) fn destructor_callable(
        &mut self,
        class: EntityId,
    ) -> Result<Option<BindingId>, Error> {
        if let Some(binding) = self.graph.destructor_binding(class) {
            return Ok(Some(binding));
        }
        let Some(facts) = self
            .graph
            .entity(class)
            .destruction
            .filter(|facts| !facts.trivial || facts.deleted)
        else {
            return Ok(None);
        };
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
        let binding = self.graph.add_implicit_destructor(class, signature)?;
        let entity = self.graph.binding(binding).entity;
        let DestructorUnwind::Implicit { nothrow } = facts.unwind else {
            return Err(Error::Invariant("implicit destructor with declared unwind"));
        };
        self.graph.entity_mut(entity).exceptions = ExceptionState::Value(if nothrow {
            ExceptionSet::None
        } else {
            ExceptionSet::All
        });
        if facts.deleted {
            self.graph.entity_mut(entity).availability = FunctionAvailability::Deleted;
        }
        Ok(Some(binding))
    }
    /// Check an existing object's lifetime end without requiring that its
    /// static class can be instantiated. Abstract base subobjects can be deleted.
    pub(super) fn check_class_destruction(
        &mut self,
        mut ty: super::types::TypeId,
        scope: super::graph::ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(());
        };
        if !self.graph.entity(class).bases.is_empty() {
            self.prepare_destruction(class)?;
        }
        let Some(binding) = self.destructor_callable(class)? else {
            return Ok(());
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
        Ok(())
    }

    /// Lowering destroys a materialized list object even where the frozen
    /// `list_argument` path performs no destructor access check. Keep that output
    /// demand separate from validation, and attach it to the containing body.
    pub(super) fn request_temporary_destruction(
        &mut self,
        mut ty: super::types::TypeId,
    ) -> Result<(), Error> {
        if !self.lowering() || self.unevaluated != 0 || self.template_depth != 0 {
            return Ok(());
        }
        while let TypeKind::Array { element, bound, .. } = self.graph.types.get(ty).kind {
            if bound == 0 {
                return Ok(());
            }
            ty = element;
        }
        // object_lowering.cpp::needs_destruction is an optional query: a
        // nested-class signature need not have a published destructor yet.
        // Recording storage alone must not instantiate subobject bodies.
        if let Some(class) = self.graph.class_of(ty)
            && self
                .graph
                .entity(class)
                .destruction
                .is_some_and(|facts| !facts.trivial)
            && (self.graph.entity(class).key != Keyword::Union
                || self.graph.entity(class).declared_destructor.is_some())
            && let Some(binding) = self.destructor_callable(class)?
        {
            self.graph.storage_destructions.push((
                self.body_context.as_ref().map(|body| body.entity),
                self.graph.binding(binding).entity,
            ));
        }
        Ok(())
    }

    pub(super) fn check_temporary_destruction(
        &mut self,
        facts: super::expression_facts::ExprFacts,
        scope: super::graph::ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if facts.category == super::expression_facts::ValueCategory::Prvalue
            && facts.functions.is_none()
        {
            self.class_parameter_destruction(facts.ty, scope, location)?;
            if self.lowering() && self.unevaluated == 0 {
                self.request_object_layout(facts.ty, location);
                self.request_temporary_destruction(facts.ty)?;
            }
        }
        Ok(())
    }
    pub(super) fn check_binding_temporary(
        &mut self,
        facts: super::expression_facts::ExprFacts,
        conversion: super::expression_facts::Conversion,
        scope: super::graph::ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if matches!(
            self.graph.types.get(conversion.target).kind,
            TypeKind::Reference { .. }
        ) {
            if conversion.temporary {
                self.class_parameter_destruction(conversion.result, scope, location)?;
                if self.lowering() && self.unevaluated == 0 {
                    self.request_object_layout(conversion.result, location);
                    self.request_temporary_destruction(conversion.result)?;
                }
            } else {
                self.check_temporary_destruction(facts, scope, location)?;
            }
        }
        Ok(())
    }
    pub(super) fn check_subobject_destruction(
        &mut self,
        class: EntityId,
        scope: super::graph::ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let indirect = self.graph.indirect_virtual_bases(class);
        let count = self.graph.entity(class).bases.len()
            + indirect.len()
            + if self.graph.entity(class).key == Keyword::Union {
                0
            } else {
                self.graph.entity(class).members.len()
            };
        for index in 0..count {
            let Some((child, member)) = self.graph.destruction_child(class, index, &indirect)?
            else {
                continue;
            };
            if member && self.graph.entity(child).anonymous_class.is_some() {
                continue;
            }
            if let Some(binding) = self.destructor_callable(child)? {
                if self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .availability
                    == FunctionAvailability::Deleted
                {
                    return Err(Error::at(location, "use of a deleted function"));
                }
                self.check_special_member_access(
                    binding,
                    if member { child } else { class },
                    scope,
                    location,
                )?;
            }
        }
        Ok(())
    }
    /// A reached constructor may destroy completed subobjects on failure even
    /// when no use reaches the complete object's destructor.
    pub(super) fn constructor_cleanup_boundaries(
        &mut self,
        class: EntityId,
        work: &mut Vec<EntityId>,
    ) -> Result<(), Error> {
        self.prepare_destruction(class)?;
        let count = self.graph.entity(class).bases.len() + self.graph.entity(class).members.len();
        for index in 0..count {
            if let Some(child) = self.graph.destruction_output_child(class, index)?
                && !self.graph.destructor_trivial(child)?
                && let Some(binding) = self.destructor_callable(child)?
            {
                work.push(self.graph.binding(binding).entity);
            }
        }
        Ok(())
    }

    pub(super) fn prepare_destruction(&mut self, class: EntityId) -> Result<(), Error> {
        // Follow only this object's containment graph, recording output-boundary
        // edges on each destructor instead of demanding unrelated definitions.
        let mut work = vec![class];
        while let Some(current) = work.pop() {
            if self.graph.entity(current).destruction_preparation == Preparation::Ready {
                continue;
            }
            self.graph.entity_mut(current).destruction_preparation = Preparation::Ready;
            let parent = self
                .destructor_callable(current)?
                .map(|binding| self.graph.binding(binding).entity);
            let count =
                self.graph.entity(current).bases.len() + self.graph.entity(current).members.len();
            for index in 0..count {
                if let Some(child) = self.graph.destruction_output_child(current, index)? {
                    if let (Some(parent), Some(binding)) =
                        (parent, self.destructor_callable(child)?)
                        && !self.graph.destructor_trivial(child)?
                        && self.lowering()
                    {
                        // The parent's reached output boundary owns this body
                        // demand; preparing containment alone is not a use.
                        self.graph
                            .storage_destructions
                            .push((Some(parent), self.graph.binding(binding).entity));
                    }
                    work.push(child);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn nested_explicit_unwind_stays_lazy_and_reuses_its_declaration() {
        let source = SourceBuffer::new(b"struct Outer{~Outer()noexcept(sizeof(Outer)>0);struct Inner{~Inner()noexcept(sizeof(Outer)>0);};Inner member;};struct Unused{~Unused()noexcept(sizeof(Unused)>0);};struct InlineOnly{~InlineOnly();};inline InlineOnly::~InlineOnly()=default;".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"lazy-unwind").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let outer_name = cursor.intern_name(b"Outer").unwrap();
        let inner_name = cursor.intern_name(b"Inner").unwrap();
        let unused_name = cursor.intern_name(b"Unused").unwrap();
        let inline_name = cursor.intern_name(b"InlineOnly").unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let stack_marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&stack_marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let class = |scope, name| {
            let binding = analyzer
                .graph
                .own(scope, name, super::super::lookup::Filter::Elaborated)
                .unwrap();
            analyzer.graph.binding(binding).entity
        };
        let outer = class(global, outer_name);
        let inner = class(analyzer.graph.entity(outer).scope.unwrap(), inner_name);
        let unused = class(global, unused_name);
        let inline_only = class(global, inline_name);
        let destructor = |class| {
            analyzer
                .graph
                .binding(analyzer.graph.entity(class).declared_destructor.unwrap())
                .entity
        };
        let inner_destructor = destructor(inner);
        let unused_destructor = destructor(unused);
        assert!(matches!(
            analyzer.graph.entity(inner_destructor).exceptions,
            ExceptionState::Pending(_)
        ));
        assert!(matches!(
            analyzer.graph.entity(outer).layout,
            super::super::layout::LayoutState::NotStarted
        ));
        assert!(!analyzer.function_may_throw(inner_destructor).unwrap());
        assert!(matches!(
            analyzer.graph.entity(inner_destructor).exceptions,
            ExceptionState::Value(ExceptionSet::None)
        ));
        assert!(!analyzer.function_may_throw(inner_destructor).unwrap());
        assert!(matches!(
            analyzer.graph.entity(unused_destructor).exceptions,
            ExceptionState::Pending(_)
        ));
        assert!(matches!(
            analyzer.graph.entity(unused).layout,
            super::super::layout::LayoutState::NotStarted
        ));
        assert!(matches!(
            analyzer.graph.entity(inline_only).layout,
            super::super::layout::LayoutState::NotStarted
        ));
        assert!(analyzer.graph.storage_layouts.is_empty());
        assert!(analyzer.graph.boundary_roots.is_empty());
        assert!(super::super::function_demands::trailing(&analyzer.graph).is_empty());
        assert_eq!(tree.expressions.mark(), expressions);
    }
}
