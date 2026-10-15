// Translated from lower/aggregate_lowering.cpp::aggregate_object/member_made.
// See NOTICE.
//! Completed aggregate subobjects join the active function's unwind chain until ownership transfers.
use super::{Error, function::Function};
use crate::{
    lowir::model::Value,
    sema::lowering::{EntityId, TypeId, TypeView},
    syntax::ast,
};

#[derive(Default)]
pub(super) struct State {
    pub floor: Option<usize>,
    pub landing: bool,
    pub count: Option<super::aggregate_arrays::Count>,
    pub call_scope: CallScope,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum CallScope {
    #[default]
    FullExpression,
    OmittedValue,
}
#[derive(Clone, Copy)]
pub(super) struct Scope {
    floor: Option<usize>,
    landing: bool,
    first: usize,
    tracking: bool,
    expression: bool,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn begin_aggregate(
        &mut self,
        ty: TypeId,
        values: &[ast::ExprId],
    ) -> Result<Scope, Error> {
        let mut throws = false;
        if !self.persistent_references {
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
        let scope = Scope {
            floor: self.aggregation.floor,
            landing: self.aggregation.landing,
            first: self.lifetime.cleanups.len(),
            tracking: throws,
            expression: self.lifetime.full_expression,
        };
        if throws {
            self.aggregation.floor = Some(scope.first);
            self.aggregation.landing = false;
            self.lifetime.full_expression |= !values.is_empty();
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
                self.emit(crate::lowir::model::InstructionKind::Addr(storage), ty)?;
            }
            self.close_lifetime_region()?;
            self.release_aggregate_members(scope.first);
        }
        self.aggregation.floor = scope.floor;
        self.aggregation.landing = scope.landing;
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
        if throws && self.lifetime.region_depth != self.lifetime.cleanups.len() {
            self.close_lifetime_region()?;
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
                .map(|id| self.default_constructor_throws(id))
                .transpose()
                .map(Option::unwrap_or_default);
        }
        if depth > 64 {
            return Ok(true);
        }
        for &member in self.graph.lower_class_member_entities(class) {
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
