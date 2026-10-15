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
    pub nothrow: bool,
}
impl Graph {
    fn destruction_child(
        &self,
        class: EntityId,
        index: usize,
    ) -> Result<Option<(EntityId, bool)>, Error> {
        let entity = self.entity(class);
        if let Some(base) = entity.bases.get(index) {
            return Ok(Some((base.class, false)));
        }
        let member = entity.members[index - entity.bases.len()];
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
    pub(crate) fn lower_destructor_subobjects_throw(&self, class: EntityId) -> Result<bool, Error> {
        let count = self.entity(class).bases.len()
            + if self.entity(class).key == Keyword::Union {
                0
            } else {
                self.entity(class).members.len()
            };
        for index in 0..count {
            if let Some((child, _)) = self.destruction_child(class, index)?
                && self
                    .entity(child)
                    .destruction
                    .is_some_and(|facts| !facts.trivial && !facts.nothrow)
            {
                return Ok(true);
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
    pub(super) fn complete_destructor(&mut self, class: EntityId) -> Result<(), Error> {
        let scope = self.graph.entity(class).scope.ok_or(Error::Invariant(
            "destructor properties without class scope",
        ))?;
        let mut facts = Destruction {
            declared_name: self.graph.entity(class).name,
            trivial: true,
            deleted: false,
            nothrow: true,
        };
        let count = self.graph.entity(class).bases.len() + self.graph.entity(class).members.len();
        for index in 0..count {
            let Some((child, member)) = self.graph.destruction_child(class, index)? else {
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
            facts.nothrow &= child_facts.nothrow;
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
        if let Some(binding) = self.graph.entity(class).declared_destructor {
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
                self.graph.entity_mut(id).exceptions = ExceptionState::Value(if facts.nothrow {
                    ExceptionSet::None
                } else {
                    ExceptionSet::All
                });
            } else {
                facts.nothrow = !self.function_may_throw(id)?;
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
        self.graph.entity_mut(entity).exceptions = ExceptionState::Value(if facts.nothrow {
            ExceptionSet::None
        } else {
            ExceptionSet::All
        });
        if facts.deleted {
            self.graph.entity_mut(entity).availability = FunctionAvailability::Deleted;
        }
        Ok(Some(binding))
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
        let count = self.graph.entity(class).bases.len()
            + if self.graph.entity(class).key == Keyword::Union {
                0
            } else {
                self.graph.entity(class).members.len()
            };
        for index in 0..count {
            let Some((child, member)) = self.graph.destruction_child(class, index)? else {
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
                if let Some((child, _)) = self.graph.destruction_child(current, index)? {
                    if let (Some(parent), Some(binding)) =
                        (parent, self.destructor_callable(child)?)
                        && !self.graph.destructor_trivial(child)?
                        && self.lowering()
                    {
                        self.storage_destructions
                            .push((Some(parent), self.graph.binding(binding).entity));
                    }
                    work.push(child);
                }
            }
        }
        Ok(())
    }
}
