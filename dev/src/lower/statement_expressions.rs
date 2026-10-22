// Translated from lower/statement_lowering.cpp::statement_expression/ExpressionScope.
// See NOTICE.
//! Block statements own their full expressions while retaining the function's live objects.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::model::Value,
    syntax::ast::{self, StmtKind},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn statement_expression(
        &mut self,
        id: ast::ExprId,
        body: ast::StmtId,
        object: Option<Value>,
    ) -> Result<RValue, Error> {
        self.close_lifetime_region()?;
        self.drop_segment_floor();
        let mut scope = ExpressionScope::take(self);
        let result = self.statement_expression_block(id, body, object);
        scope.restore(self)?;
        result
    }

    fn statement_expression_block(
        &mut self,
        id: ast::ExprId,
        body: ast::StmtId,
        object: Option<Value>,
    ) -> Result<RValue, Error> {
        let StmtKind::Compound(items) = self.tree.statements.get(body).kind else {
            return Err(
                crate::sema::Error::Invariant("statement expression without compound").into(),
            );
        };
        let source = self.graph.lower_expression(id)?.ty;
        let value =
            if super::types::storage(self.graph, source)? == crate::lowir::vocabulary::Type::Void {
                None
            } else {
                self.graph.statement_expression_value(self.tree, body)?
            };
        let depth = self.lifetime.cleanups.len();
        self.scan_statement_expression(body)?;
        let statements = self.tree.stmt_lists.get(items);
        let count = statements.len() - usize::from(value.is_some());
        for &statement in &statements[..count] {
            self.statement(statement)?;
        }
        if value.is_some() {
            let mut last = statements[count];
            loop {
                match self.tree.statements.get(last).kind {
                    StmtKind::Ambiguous { .. } => last = self.graph.lower_statement(last)?,
                    StmtKind::Label { name, body } => {
                        let target = self.goto_label(name)?;
                        if !self.terminated {
                            self.jump(target)?;
                        }
                        self.place(target)?;
                        last = body;
                    }
                    _ => break,
                }
            }
        }
        let mut result = self.zero(source)?;
        if let Some(value) = value.filter(|_| !self.terminated) {
            let first = self.lifetime.cleanups.len();
            let began = self.begin_full_expression(
                Some(value),
                super::temporary_scan::ExpressionUse::Ordinary,
            )?;
            self.with_full_root(Some(value), |this| {
                if let Some(object) = object {
                    this.materialize_class(value, source, object)?;
                } else {
                    result = this.operand(value, Context::Initialization)?;
                }
                Ok(())
            })?;
            self.destroy_temporaries(first)?;
            self.end_lifetime_expression(began)?;
        }
        self.leave_scope(depth)?;
        self.close_lifetime_region()?;
        if self.terminated {
            let next = self.new_block("unreachable")?;
            self.place(next)?;
            if object.is_none() {
                result = self.zero(source)?;
            }
        }
        Ok(result)
    }
}

struct ExpressionScope {
    full_expression: bool,
    temporary_segment: super::lifetime::TemporarySegment,
    expression_start: Option<crate::lowir::builder::Position>,
    construction: bool,
    construction_flow: super::construction_flow::ConstructionFlow,
    construction_start: Option<(crate::lowir::builder::Position, usize)>,
    flow_region: super::lifetime::FlowRegion,
    region_start: super::lifetime::RegionStart,
    regions_opened: usize,
    result_region: super::class_results::ResultRegion,
    landing_placement: super::class_results::LandingPlacement,
    storage_split: Option<crate::lowir::builder::Position>,
    temporary_split: Option<crate::lowir::builder::Position>,
    call_marks: Vec<super::lifetime::CallMark>,
    call_start: Option<crate::lowir::builder::Position>,
    declared_object: Option<Value>,
    guard_start: Option<crate::lowir::builder::Position>,
    guarded_operands: u32,
    choice_region: super::lifetime::ChoiceRegion,
    tracking: super::tracked_lifetimes::Tracking,
    aggregation: super::aggregate_cleanup::State,
    reference_lifetimes: super::reference_lifetimes::Extensions,
    bit_initialization: super::bit_fields::Initialization,
    call_object_class: Option<crate::sema::lowering::EntityId>,
    conditional_slot: Option<Value>,
    persistent_references: bool,
}
impl ExpressionScope {
    fn take(f: &mut Function<'_, '_, '_, '_>) -> Self {
        Self {
            full_expression: std::mem::take(&mut f.lifetime.full_expression),
            temporary_segment: std::mem::take(&mut f.lifetime.temporary_segment),
            expression_start: std::mem::take(&mut f.lifetime.expression_start),
            construction: std::mem::take(&mut f.lifetime.construction),
            construction_flow: std::mem::take(&mut f.lifetime.construction_flow),
            construction_start: std::mem::take(&mut f.lifetime.construction_start),
            flow_region: std::mem::take(&mut f.lifetime.flow_region),
            region_start: std::mem::take(&mut f.lifetime.region_start),
            regions_opened: std::mem::take(&mut f.lifetime.regions_opened),
            result_region: std::mem::take(&mut f.lifetime.result_region),
            landing_placement: std::mem::take(&mut f.lifetime.landing_placement),
            storage_split: std::mem::take(&mut f.lifetime.storage_split),
            temporary_split: std::mem::take(&mut f.lifetime.temporary_split),
            call_marks: std::mem::take(&mut f.lifetime.call_marks),
            call_start: std::mem::take(&mut f.lifetime.call_start),
            declared_object: std::mem::take(&mut f.lifetime.declared_object),
            guard_start: std::mem::take(&mut f.lifetime.guard_start),
            guarded_operands: std::mem::take(&mut f.lifetime.guarded_operands),
            choice_region: std::mem::take(&mut f.lifetime.choice_region),
            tracking: std::mem::take(&mut f.lifetime.tracking),
            aggregation: std::mem::take(&mut f.aggregation),
            reference_lifetimes: std::mem::take(&mut f.reference_lifetimes),
            bit_initialization: std::mem::take(&mut f.bit_initialization),
            call_object_class: std::mem::take(&mut f.call_object_class),
            conditional_slot: std::mem::take(&mut f.conditional_slot),
            persistent_references: std::mem::take(&mut f.persistent_references),
        }
    }
    fn restore(&mut self, f: &mut Function<'_, '_, '_, '_>) -> Result<(), Error> {
        // Calls and a pending construction resume at the block's end, never
        // at an instruction position belonging to the enclosing expression.
        let position = f.builder.position()?;
        for mark in &mut self.call_marks {
            mark.position = position;
        }
        self.expression_start = Some(position);
        if let Some((start, _)) = &mut self.construction_start {
            *start = position;
        }
        std::mem::swap(&mut f.lifetime.full_expression, &mut self.full_expression);
        std::mem::swap(
            &mut f.lifetime.temporary_segment,
            &mut self.temporary_segment,
        );
        std::mem::swap(&mut f.lifetime.expression_start, &mut self.expression_start);
        std::mem::swap(&mut f.lifetime.construction, &mut self.construction);
        std::mem::swap(
            &mut f.lifetime.construction_flow,
            &mut self.construction_flow,
        );
        std::mem::swap(
            &mut f.lifetime.construction_start,
            &mut self.construction_start,
        );
        std::mem::swap(&mut f.lifetime.flow_region, &mut self.flow_region);
        std::mem::swap(&mut f.lifetime.region_start, &mut self.region_start);
        std::mem::swap(&mut f.lifetime.regions_opened, &mut self.regions_opened);
        std::mem::swap(&mut f.lifetime.result_region, &mut self.result_region);
        std::mem::swap(
            &mut f.lifetime.landing_placement,
            &mut self.landing_placement,
        );
        std::mem::swap(&mut f.lifetime.storage_split, &mut self.storage_split);
        std::mem::swap(&mut f.lifetime.temporary_split, &mut self.temporary_split);
        std::mem::swap(&mut f.lifetime.call_marks, &mut self.call_marks);
        std::mem::swap(&mut f.lifetime.call_start, &mut self.call_start);
        std::mem::swap(&mut f.lifetime.declared_object, &mut self.declared_object);
        std::mem::swap(&mut f.lifetime.guard_start, &mut self.guard_start);
        std::mem::swap(&mut f.lifetime.guarded_operands, &mut self.guarded_operands);
        std::mem::swap(&mut f.lifetime.choice_region, &mut self.choice_region);
        std::mem::swap(&mut f.lifetime.tracking, &mut self.tracking);
        std::mem::swap(&mut f.aggregation, &mut self.aggregation);
        std::mem::swap(&mut f.reference_lifetimes, &mut self.reference_lifetimes);
        std::mem::swap(&mut f.bit_initialization, &mut self.bit_initialization);
        std::mem::swap(&mut f.call_object_class, &mut self.call_object_class);
        std::mem::swap(&mut f.conditional_slot, &mut self.conditional_slot);
        std::mem::swap(
            &mut f.persistent_references,
            &mut self.persistent_references,
        );
        Ok(())
    }
}
