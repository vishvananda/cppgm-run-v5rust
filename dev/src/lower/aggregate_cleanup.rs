// Translated from lower/aggregate_lowering.cpp::aggregate_object/member_made.
// See NOTICE.
//! Completed aggregate subobjects join the active function's unwind chain until ownership transfers.
use super::{Error, function::Function};
use crate::{
    lowir::model::Value,
    sema::lowering::{EntityId, TypeId, TypeView},
    syntax::ast,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum MemberRegions {
    #[default]
    Absent,
    Opened,
}

#[derive(Default)]
pub(super) struct State {
    pub floor: Option<usize>,
    pub landing: bool,
    pub regions: MemberRegions,
    pub count: Option<super::aggregate_arrays::Count>,
    pub counted_item: bool,
    pub call_scope: CallScope,
    serial: usize,
    pub reopened: bool,
    pub completion: Option<Value>,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum CallScope {
    #[default]
    FullExpression,
    OmittedValue,
}
pub(super) struct Scope {
    floor: Option<usize>,
    landing: bool,
    regions: MemberRegions,
    first: usize,
    tracking: bool,
    expression: bool,
    serial: usize,
    aside: Vec<super::lifetime::Cleanup>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn begin_aggregate(
        &mut self,
        ty: TypeId,
        values: &[ast::ExprId],
        computed: bool,
    ) -> Result<Scope, Error> {
        let mut throws = false;
        if !self.static_initializer {
            for &id in values {
                throws |= self.expression_throws(id)?;
            }
            let mut element = ty;
            while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
                element = inner;
            }
            if let TypeView::Class(class) = self.graph.lower_type(element)? {
                throws |= self.omitted_aggregate_throws(class, 0)?;
            }
        }
        let aside_at = self.lifetime.tracking.floor;
        let aside = if throws
            && self.lifetime.tracking.active
            && self.aggregation.floor.is_none()
            && self.lifetime.cleanups.len() > aside_at
            && self.lifetime.cleanups[aside_at..].iter().all(|cleanup| {
                matches!(cleanup.place, super::lifetime::CleanupPlace::Temporary(_))
                    && cleanup.guard.is_some()
            }) {
            self.lifetime.cleanups.split_off(aside_at)
        } else {
            Vec::new()
        };
        let scope = Scope {
            floor: self.aggregation.floor,
            landing: self.aggregation.landing,
            regions: self.aggregation.regions,
            first: self.lifetime.cleanups.len(),
            tracking: throws,
            expression: self.lifetime.full_expression,
            serial: self.aggregation.serial,
            aside,
        };
        if throws {
            // A computed object's construction consumes an empty initial
            // segment before deriving any member address. Its pending landing
            // is reused by the first member (aggregate_object's Root::Value).
            if computed
                && self.lifetime.full_expression
                && !self.lifetime.region
                && self.handlers.explicit == 0
            {
                let position = if self.lifetime.region_start.is_pending() {
                    self.lifetime.expression_start
                } else {
                    Some(self.builder.position()?)
                };
                self.open_temporary_region_at(position)?;
                self.close_lifetime_region()?;
            }
            if self.lifetime.tracking.active && self.lifetime.full_expression {
                self.lifetime.tracking.construction =
                    super::tracked_lifetimes::ConstructionTracking::Storage;
            }
            self.aggregation.floor = Some(scope.first);
            self.aggregation.landing = false;
            self.aggregation.regions = MemberRegions::Absent;
            self.aggregation.serial = self.lifetime.regions_opened;
        }
        Ok(scope)
    }
    pub(super) fn end_aggregate(
        &mut self,
        scope: Scope,
        completion: Option<(Value, TypeId)>,
    ) -> Result<(), Error> {
        if scope.tracking {
            if self.aggregation.landing
                && let Some((storage, ty)) = completion
            {
                self.aggregation.completion = Some(self.storage_address(storage, ty)?);
            }
            self.close_lifetime_region()?;
            self.release_aggregate_members(scope.first);
        }
        if !scope.aside.is_empty() {
            self.lifetime
                .cleanups
                .splice(scope.first..scope.first, scope.aside);
            for cleanup in &mut self.lifetime.cleanups[scope.first..] {
                cleanup.clear_unwind_memos();
            }
        }
        self.aggregation.floor = scope.floor;
        self.aggregation.landing = scope.landing;
        self.aggregation.regions = scope.regions;
        self.aggregation.serial = scope.serial;
        self.lifetime.full_expression = scope.expression;
        Ok(())
    }
    pub(super) fn aggregate_has_members(&self) -> bool {
        self.aggregation.floor.is_some_and(|floor| {
            self.lifetime.cleanups[floor..]
                .iter()
                .any(|cleanup| cleanup.member())
        })
    }
    pub(super) fn begin_aggregate_member(
        &mut self,
        ty: TypeId,
        expr: Option<ast::ExprId>,
    ) -> Result<(), Error> {
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Ok(());
        };
        if self.aggregation.floor.is_none() || self.graph.lower_aggregate_class(class) {
            return Ok(());
        }
        let throws = if let Some(id) = expr {
            self.expression_throws(id)?
        } else {
            self.omitted_aggregate_throws(class, 0)?
        };
        let reusable = self.lifetime.region
            && self.lifetime.region_depth == self.lifetime.cleanups.len()
            && (self.lifetime.regions_opened > self.aggregation.serial
                || self.aggregation.reopened);
        if throws && !reusable {
            self.close_lifetime_region()?;
        }
        if throws {
            // aggregate_item opens this member's region after its address,
            // rather than at the enclosing initializer segment's start.
            self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        }
        Ok(())
    }
    pub(super) fn note_aggregate_landing(&mut self) {
        if let Some(floor) = self.aggregation.floor {
            self.aggregation.landing |= self.lifetime.cleanups[floor..].iter().any(|c| c.member());
        }
    }
    pub(super) fn open_aggregate_member(
        &mut self,
        ty: TypeId,
        expr: Option<ast::ExprId>,
    ) -> Result<(), Error> {
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Ok(());
        };
        if self.aggregation.floor.is_some()
            && !self.graph.lower_aggregate_class(class)
            && !self.lifetime.region
            && (if let Some(id) = expr {
                self.expression_throws(id)?
            } else {
                self.omitted_aggregate_throws(class, 0)?
            })
            && (self.lifetime.cleanups.len() > self.handler_floor()
                || self.lifetime.tracking.active
                || self.lifetime.full_expression
                    && self.lifetime.temporary_segment.is_present()
                    && self
                        .handlers
                        .contexts
                        .last()
                        .is_some_and(|context| !context.attempt))
            && !self.static_initializer
            && self.handlers.explicit == 0
        {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
            if self.lifetime.region {
                self.aggregation.regions = MemberRegions::Opened;
            }
        }
        Ok(())
    }
    pub(super) fn aggregate_member_made(
        &mut self,
        ty: TypeId,
        object: Value,
        first: usize,
    ) -> Result<(), Error> {
        self.close_lifetime_region()?;
        self.release_aggregate_members(first);
        self.register_aggregate_member(ty, object);
        Ok(())
    }
    pub(super) fn omitted_aggregate_throws(
        &mut self,
        class: EntityId,
        depth: u8,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if !self.graph.lower_aggregate_class(class) {
            return self
                .graph
                .lower_ready_default_constructor(class)
                .map(|id| {
                    Ok(!self.graph.lower_constructor_trivial(id)
                        && !self.graph.lower_constructor_function(id)?.nothrow)
                })
                .transpose()
                .map(Option::unwrap_or_default);
        }
        if depth > 64 {
            return Ok(true);
        }
        for member in self.graph.lower_class_member_entities(class).to_vec() {
            let mut ty = self.graph.lower_entity_type(member)?;
            while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
                ty = element;
            }
            if let TypeView::Class(child) = self.graph.lower_type(ty)?
                && self.omitted_aggregate_throws(child, depth + 1)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
