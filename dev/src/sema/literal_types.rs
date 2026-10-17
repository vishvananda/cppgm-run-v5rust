// Translated from sema/analyzer_constexpr.cpp. See NOTICE.
//! Literal-type queries belong to canonical classes; constructor queries are query-local.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    graph::{EntityId, FunctionAvailability},
    special_members::SpecialDefinition,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};
use std::collections::HashMap;

#[derive(Clone, Copy, Default)]
pub(super) enum LiteralState {
    #[default]
    NotStarted,
    InProgress,
    Complete(bool),
}
impl Analyzer<'_> {
    pub(super) fn check_constexpr_variable(
        &mut self,
        is_constexpr: bool,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.semantic()
            && self.template_depth == 0
            && is_constexpr
            && !self.constant_literal_type(ty)?
        {
            return Err(Error::at(
                location,
                "a constexpr variable of a type that is not a literal type",
            ));
        }
        Ok(())
    }
    pub(super) fn constant_literal_type(&mut self, mut ty: TypeId) -> Result<bool, Error> {
        self.check_stack()?;
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            return Ok(true);
        }
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(!matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            ));
        };
        match self.graph.entity(class).literal {
            LiteralState::Complete(value) => return Ok(value),
            LiteralState::InProgress => return Ok(true),
            LiteralState::NotStarted => (),
        }
        if !self.complete_class(class)? || self.graph.entity(class).scope.is_none() {
            return Ok(false);
        }
        self.graph.entity_mut(class).literal = LiteralState::InProgress;
        let result = self.constant_literal_class(class);
        self.graph.entity_mut(class).literal = match result {
            Ok(value) => LiteralState::Complete(value),
            Err(_) => LiteralState::NotStarted,
        };
        result
    }
    fn constant_literal_class(&mut self, class: EntityId) -> Result<bool, Error> {
        if self.graph.lower_nontrivial_destructor(class)? {
            return Ok(false);
        }
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            if !self.constant_literal_type(self.graph.lower_entity_type(base)?)? {
                return Ok(false);
            }
        }
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            let mut ty = self.graph.lower_entity_type(member)?;
            if !self.constant_literal_type(ty)? {
                return Ok(false);
            }
            if !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                if self.type_cv(ty).0 & 2 != 0 {
                    return Ok(false);
                }
            }
        }
        if self.graph.aggregate_class(class) {
            return Ok(true);
        }
        for &binding in &self.graph.entity(class).declared_constructors {
            let function = self.graph.entity(self.graph.binding(binding).entity);
            if function.copy_kind.is_none() && function.constexpr {
                return Ok(true);
            }
        }
        if self.graph.entity(class).user_constructor.is_some() {
            return Ok(false);
        }
        let id = self.default_constructor(class, SourceOffset(0))?;
        self.constant_constructor_eligible(id, &mut HashMap::new())
    }
    pub(super) fn constant_constructor_eligible(
        &mut self,
        id: ConstructorId,
        known: &mut HashMap<ConstructorId, bool>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(&value) = known.get(&id) {
            return Ok(value);
        }
        known.insert(id, false);
        let result = self.constant_constructor_eligible_plan(id, known)?;
        known.insert(id, result);
        Ok(result)
    }
    fn constant_constructor_eligible_plan(
        &mut self,
        id: ConstructorId,
        known: &mut HashMap<ConstructorId, bool>,
    ) -> Result<bool, Error> {
        let plan = self.graph.constructors.get(id);
        if plan.deleted {
            return Ok(false);
        }
        if let Some(binding) = plan.binding {
            let function = self.graph.entity(self.graph.binding(binding).entity);
            if !matches!(
                function.special_definition,
                Some(SpecialDefinition::Defaulted { .. })
            ) {
                return Ok(function.constexpr);
            }
        }
        let class = plan.class;
        if self.graph.entity(class).key == Keyword::Union {
            return Err(Error::Missing("constexpr union constructor eligibility"));
        }
        if let Some(copy) = plan.copy.as_ref() {
            let count = copy.len();
            for index in 0..count {
                if let Some(child) = self
                    .graph
                    .constructors
                    .get(id)
                    .copy
                    .as_ref()
                    .ok_or(Error::Invariant("lost constant copy plan"))?[index]
                    .constructor
                    && !self.constant_constructor_eligible(child, known)?
                {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        for index in 0..self.graph.constructors.get(id).bases.len() {
            if let Some(base) = self.graph.constructors.get(id).bases[index].constructor
                && !self.constant_constructor_eligible(base, known)?
            {
                return Ok(false);
            }
        }
        for index in 0..self.graph.constructors.get(id).members.len() {
            let (member, initializer) = self.graph.constructors.get(id).members[index];
            if initializer.is_some() || self.graph.unnamed_bit_field(member) {
                continue;
            }
            let mut ty = self.graph.lower_entity_type(member)?;
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            let Some(class) = self.graph.class_of(ty) else {
                return Ok(false);
            };
            let child = self.default_constructor(class, SourceOffset(0))?;
            if !self.constant_constructor_eligible(child, known)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn check_constant_constructor_plan(
        &mut self,
        function: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let entity = self.graph.entity(function);
        if !entity.constexpr
            || entity.availability == FunctionAvailability::Deleted
            || entity.template_head.is_some()
            || entity
                .special_definition
                .is_some_and(|special| matches!(special, SpecialDefinition::Defaulted { .. }))
            || entity
                .home
                .is_some_and(|home| self.graph.within_class_instance(home))
        {
            return Ok(());
        }
        let id = entity
            .constructor
            .ok_or(Error::Invariant("constexpr constructor without plan"))?;
        if let Some(delegation) = self.graph.constructors.get(id).delegation {
            if !self.constant_constructor_eligible(delegation.constructor, &mut HashMap::new())? {
                return Err(Error::at(
                    location,
                    "a constexpr constructor delegates to a constructor that is not constexpr",
                ));
            }
            return Ok(());
        }
        for index in 0..self.graph.constructors.get(id).subobjects.len() {
            let plan = self.graph.constructors.get(id).subobjects[index];
            if !self.constant_constructor_eligible(plan.constructor, &mut HashMap::new())? {
                return Err(Error::at(
                    location,
                    "a constexpr constructor initializes a subobject by a constructor that is not constexpr",
                ));
            }
        }
        for index in 0..self.graph.constructors.get(id).members.len() {
            let (member, initializer) = self.graph.constructors.get(id).members[index];
            if initializer.is_some() || self.graph.unnamed_bit_field(member) {
                continue;
            }
            let mut ty = self.graph.lower_entity_type(member)?;
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            if self.graph.class_of(ty).is_none() {
                return Err(Error::at(
                    location,
                    "a constexpr constructor leaves a non-static data member uninitialized",
                ));
            }
        }
        Ok(())
    }
}
