// Translated from analyzer_initialization.cpp::queried_default_constructor and
// analyzer_construction.cpp::implicit_default_facts. See NOTICE.
//! Availability queries retain failed implicit plans as deleted facts. Uses keep
//! their existing diagnostic path; a query never emits a callable or demands a body.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorExceptionSource,
    graph::{EntityId, FunctionAvailability},
    special_members::SpecialDefinition,
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) struct DefaultProperties {
    pub deleted: bool,
    pub trivial: bool,
    pub nothrow: bool,
}
#[derive(Clone, Copy)]
enum Answer {
    Active,
    Ready(Option<DefaultProperties>),
}
#[derive(Default)]
pub(super) struct Queries {
    // Keys are complete canonical classes. Like queried_default_constructor,
    // successful selections survive later declarations. An absent selection
    // must be retried when that class's constructors acquire new defaults.
    defaults: HashMap<EntityId, Answer>,
}
impl Queries {
    pub(super) fn added_defaults(&mut self, class: EntityId) {
        if matches!(self.defaults.get(&class), Some(Answer::Ready(None))) {
            self.defaults.remove(&class);
        }
    }
}
impl Analyzer<'_> {
    pub(super) fn queried_default_properties(
        &mut self,
        class: EntityId,
    ) -> Result<Option<DefaultProperties>, Error> {
        self.check_stack()?;
        if let Some(answer) = self.graph.constructor_queries.defaults.get(&class) {
            return match *answer {
                Answer::Ready(value) => Ok(value),
                Answer::Active => Err(Error::Invariant("cyclic default constructor query")),
            };
        }
        self.graph
            .constructor_queries
            .defaults
            .insert(class, Answer::Active);
        self.unevaluated += 1;
        let result = self.default_property_query(class);
        self.unevaluated -= 1;
        if let Ok(value) = result {
            self.graph
                .constructor_queries
                .defaults
                .insert(class, Answer::Ready(value));
        } else {
            self.graph.constructor_queries.defaults.remove(&class);
        }
        result
    }

    fn default_property_query(
        &mut self,
        class: EntityId,
    ) -> Result<Option<DefaultProperties>, Error> {
        let selected = match self.declared_default_constructor(class) {
            Ok(selected) => selected,
            Err(Error::Diagnostic { .. }) => {
                self.graph.work.totals.substitution_failures += 1;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if selected.is_none() && self.graph.entity(class).user_constructor.is_some() {
            return Ok(None);
        }
        let function = selected.map(|binding| self.graph.binding(binding).entity);
        if let Some(function) = function {
            let entity = self.graph.entity(function);
            if !matches!(
                entity.special_definition,
                Some(SpecialDefinition::Defaulted { .. })
            ) && self.graph.inherited_constructors.source(function).is_none()
            {
                return Ok(Some(DefaultProperties {
                    deleted: entity.availability == FunctionAvailability::Deleted,
                    trivial: false,
                    nothrow: !self.function_may_throw(function)?,
                }));
            }
        }
        // Complete each dependency's query once before asking for a callable
        // plan. A known deleted child must not make every ancestor repeat the
        // same unsuccessful plan walk through a shared declaration graph.
        let subobjects = self.default_subobject_query(class)?;
        let plan = if subobjects.unavailable {
            None
        } else {
            match self.default_constructor(class, SourceOffset(0)) {
                Ok(id) => Some(id),
                Err(Error::Diagnostic { .. }) => {
                    self.graph.work.totals.substitution_failures += 1;
                    None
                }
                Err(error) => return Err(error),
            }
        };
        let mut facts = match plan {
            Some(id) => {
                let plan = self.graph.constructors.get(id);
                DefaultProperties {
                    deleted: plan.deleted,
                    trivial: !plan.nontrivial(),
                    nothrow: plan.nothrow,
                }
            }
            // SoftSubstitution does not discard a failed implicit constructor.
            // It keeps a deleted declaration, whose completed subobject calls
            // still contribute to its implicit exception specification.
            None => DefaultProperties {
                deleted: true,
                trivial: false,
                nothrow: subobjects.nothrow,
            },
        };
        if let Some(function) = function
            && self.graph.entity(function).constructor_exceptions
                == ConstructorExceptionSource::Declared
        {
            facts.nothrow = !self.function_may_throw(function)?;
        }
        Ok(Some(facts))
    }

    fn default_subobject_query(&mut self, class: EntityId) -> Result<SubobjectDefaults, Error> {
        let mut result = SubobjectDefaults {
            nothrow: true,
            unavailable: false,
        };
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            if let Some(facts) = self.queried_default_properties(base)? {
                result.nothrow &= facts.nothrow;
                result.unavailable |= facts.deleted;
            } else {
                result.unavailable = true;
            }
        }
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            self.request_member_initializer(member)?;
            if self.graph.entity(member).member_initializer.is_some() {
                result.nothrow &= !self.graph.member_initializers.throwing(member);
                if result.nothrow
                    && let Some(id) = self.graph.entity(member).constructor
                {
                    result.nothrow = self.subobject_constructor_nothrow(id)?;
                }
            } else if self.graph.entity(class).key != Keyword::Union
                && let Some((child, _, _)) = self
                    .graph
                    .copy_child(class, self.graph.entity(class).bases.len() + index)?
            {
                if let Some(facts) = self.queried_default_properties(child)? {
                    result.nothrow &= facts.nothrow;
                    result.unavailable |= facts.deleted;
                } else {
                    result.unavailable = true;
                }
            }
        }
        for base in self.graph.indirect_virtual_bases(class) {
            if let Some(facts) = self.queried_default_properties(base)? {
                result.nothrow &= facts.nothrow;
                result.unavailable |= facts.deleted;
            } else {
                result.unavailable = true;
            }
        }
        Ok(result)
    }
}
struct SubobjectDefaults {
    nothrow: bool,
    unavailable: bool,
}
