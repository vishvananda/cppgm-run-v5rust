// Translated from analyzer_initialization.cpp::queried_default_constructor. See NOTICE.
//! A real function-slot hit is pure. A miss owns one ordinary soft lifetime;
//! even a failed implicit plan retains the actual now-deleted function.
use super::{
    Error,
    analyzer::Analyzer,
    construction::DefaultFunction,
    graph::{EntityId, FunctionAvailability},
};

#[derive(Clone, Copy)]
pub(super) struct DefaultProperties {
    pub deleted: bool,
    pub trivial: bool,
    pub function: EntityId,
}
impl Analyzer<'_> {
    pub(super) fn queried_default_constructor_function(
        &mut self,
        class: EntityId,
    ) -> Result<Option<DefaultFunction>, Error> {
        if let Some(slot) = self.graph.constructors.default_function(class) {
            return Ok(Some(slot));
        }
        let owner = self.graph.analysis.substitution.current().owner;
        self.with_substitution_soft(owner, |analyzer| {
            analyzer.implicit_constructor_function(class)
        })
    }
    pub(super) fn queried_default_properties(
        &mut self,
        class: EntityId,
    ) -> Result<Option<DefaultProperties>, Error> {
        let Some(slot) = self.queried_default_constructor_function(class)? else {
            return Ok(None);
        };
        let id = self
            .graph
            .entity(slot.function)
            .initialization()
            .constructor
            .ok_or(Error::Invariant("default query without function facts"))?;
        Ok(Some(DefaultProperties {
            deleted: self.graph.entity(slot.function).availability == FunctionAvailability::Deleted,
            trivial: !self.graph.constructors.get(id).nontrivial(),
            function: slot.function,
        }))
    }
}
