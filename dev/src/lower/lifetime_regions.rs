// Translated from lower/cleanup_lowering.cpp::begin_call/open_region/emit_call/close_region.
// See NOTICE.
//! Exception regions cover completed objects and retain call-result spill order.
use super::{
    Error,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::Type,
    },
    sema::lowering::TypeId,
};
impl Function<'_, '_, '_, '_> {
    /// Captured-array elements branch inside an enclosing live-object region.
    /// Restore its ordinary control-flow policy on both success and failure.
    pub(super) fn with_enclosing_region<T>(
        &mut self,
        guarded: bool,
        operation: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let flow = self.lifetime.flow_region;
        if guarded && self.lifetime.region {
            self.lifetime.flow_region = super::lifetime::FlowRegion::Enclosing;
        }
        let result = operation(self);
        self.lifetime.flow_region = flow;
        result
    }

    pub(super) fn enclose_element_region(
        &mut self,
        covered: &mut Option<usize>,
    ) -> Result<(), Error> {
        if self.lifetime.cleanups.is_empty() {
            return self.close_lifetime_region();
        }
        if !self.lifetime.full_expression && !self.lifetime.region {
            return Ok(());
        }
        if covered.is_none() && self.lifetime.region {
            *covered = Some(self.lifetime.cleanups.len());
        }
        if self.lifetime.region && *covered == Some(self.lifetime.cleanups.len()) {
            return Ok(());
        }
        self.close_lifetime_region()?;
        // open_region(false, true) leaves objects outside the current try
        // to that try's continuation. Only an active handler's temporary
        // segment requires an otherwise empty enclosing region.
        let boundary = self.lifetime.temporary_segment.is_present()
            && self.lifetime.full_expression
            && self.handlers.contexts.last().is_some_and(|c| !c.attempt);
        if (self.lifetime.cleanups.len() > self.handler_floor() || boundary)
            && self.handlers.explicit == 0
            && !self.static_initializer
        {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        *covered = Some(self.lifetime.cleanups.len());
        Ok(())
    }

    // cleanup_lowering.cpp::temporary_done's ordinary object path. A completed
    // inert temporary ends its construction segment, even though it adds no
    // cleanup action. A local reference forces its own completion boundary.
    pub(super) fn finish_inert_temporary(&mut self, force: bool) -> Result<(), Error> {
        if force
            && self.lifetime.full_expression
            && self.has_tracked_region()
            && self.element_temporaries.depth == 0
            && self.handlers.explicit == 0
        {
            self.close_lifetime_region()?;
            let position = self.builder.position()?;
            return self.tracked_region(Some(position));
        }
        if !self.lifetime.full_expression
            || !self.lifetime.region
            || self.lifetime.guarded_operands != 0
            || self.lifetime.choice_region == super::lifetime::ChoiceRegion::Reopened
            || self.lifetime.tracking.active
            || self.element_temporaries.depth != 0
            || self.handlers.explicit != 0
            || !force
                && self
                    .lifetime
                    .cleanups
                    .iter()
                    .skip(self.lifetime.temporary_segment.floor())
                    .any(|cleanup| {
                        matches!(cleanup.place, super::lifetime::CleanupPlace::Temporary(_))
                    })
        {
            return Ok(());
        }
        self.close_lifetime_region()?;
        if self.lifetime.cleanups.len() > self.lifetime.temporary_segment.floor()
            && !self.static_initializer
        {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        if !self.lifetime.region {
            self.lifetime.temporary_split = Some(self.builder.position()?);
        }
        Ok(())
    }

    pub(super) fn has_tracked_region(&self) -> bool {
        self.lifetime.tracking.active
            && !self.lifetime.tracking.construction.active()
            && self.aggregation.floor.is_none()
    }
    pub(super) fn storage_region_pending(&self) -> bool {
        self.lifetime.full_expression
            && !self.lifetime.region
            && self
                .lifetime
                .storage_split
                .is_some_and(|position| self.builder.same_block(position))
    }
    pub(super) fn storage_region_event(&mut self) -> Result<(), Error> {
        if self.storage_region_pending() && !self.lifetime.tracking.active {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        Ok(())
    }
    pub(super) fn open_storage_completion_region(&mut self, first: usize) -> Result<(), Error> {
        let entry = self.cleanup_prefix(true, first)?;
        self.effect(I::Runtime(R::EhTry(entry)))?;
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        self.lifetime.region = true;
        self.lifetime.region_entry = Some(entry);
        self.lifetime.region_depth = first;
        self.lifetime.regions_opened += 1;
        Ok(())
    }
    pub(super) fn temporary_alive(
        &mut self,
        ty: TypeId,
        object: crate::lowir::model::Value,
    ) -> Result<(), Error> {
        if self.lifetime.guarded_operands != 0
            && !self.lifetime.tracking.active
            && self.lifetime.guard_start.is_none()
        {
            return Err(Error::Missing("guarded full-expression temporary cleanup"));
        }
        if self.element_temporaries.depth != 0 {
            return self.element_temporary_alive(ty, object);
        }
        let split = self.lifetime.region;
        if self.has_tracked_region() && self.handlers.explicit == 0 {
            self.register_temporary(ty, object)?;
            self.track_temporary()?;
            self.close_lifetime_region()?;
            return self.tracked_region(None);
        }
        self.register_temporary(ty, object)?;
        self.track_temporary()?;
        if self.handlers.explicit != 0 {
            return Ok(());
        }
        self.close_lifetime_region()?;
        if split || self.lifetime.tracking.active && self.handlers.explicit == 0 {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        Ok(())
    }
    pub(super) fn open_function_guard(&mut self) -> Result<(), Error> {
        let landing = self.new_block("function_exception_landing")?;
        self.lifetime.function_guard = Some(landing);
        self.effect(I::Runtime(R::EhTry(landing)))
    }
    pub(super) fn place_function_guard(
        &mut self,
        entry: super::construction::Entry,
    ) -> Result<(), Error> {
        // lower_definition returns after finish_body for a synthesized copy
        // or an inert destructor, before placing the function's guard. Keep
        // that ordering, including an entry marker with no landing block.
        let synthesized_copy = matches!(entry, super::construction::Entry::Constructor(id, _)
            if self.graph.lower_constructor(id).copy.is_some());
        let inert_destructor = if let Some(entity) = entry.entity(self.graph)
            && matches!(
                entry.facts(self.graph)?.name,
                crate::sema::lowering::FunctionName::Destructor(_)
            ) {
            self.symbols.destructor_inert(entity)?
        } else {
            false
        };
        if synthesized_copy || inert_destructor {
            return Ok(());
        }
        let Some(landing) = self.lifetime.function_guard.take() else {
            return Ok(());
        };
        if let Some(entity) = self.lifetime.unexpected_specification {
            return self.place_unexpected_landing(landing, entity);
        }
        let action = self.new_block("function_exception_action")?;
        self.place(landing)?;
        self.effect(I::Runtime(R::EhCatchAll(Some(1))))?;
        let slot = self.function_exception_slot()?;
        let source = self
            .result
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "synthetic function exception guard",
            )))?;
        let exception = self
            .emit(I::Runtime(R::Exception(Type::Ptr)), source)?
            .value;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            exception,
            slot,
        ))?;
        self.jump(action)?;
        self.place(action)?;
        let exception = self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)?
            .value;
        self.terminate_exception(exception)?;
        let ty = self.symbols.result_boundary(source)?.ty;
        self.termination_return(source, ty)
    }
    fn place_unexpected_landing(
        &mut self,
        landing: crate::lowir::model::BlockId,
        entity: crate::sema::lowering::EntityId,
    ) -> Result<(), Error> {
        use crate::lowir::{
            model::{Call, Value},
            vocabulary::Compare,
        };
        self.place(landing)?;
        let types =
            self.graph
                .lower_throw_specification(entity)
                .ok_or(crate::sema::Error::Invariant(
                    "missing dynamic exception specification",
                ))?;
        let mut information = Vec::with_capacity(types.len());
        for &ty in types {
            let ty = match self.graph.lower_type(ty)? {
                crate::sema::lowering::TypeView::Reference { element, .. } => element,
                _ => ty,
            };
            information.push(Value::Symbol(
                self.symbols
                    .exception_type_information(self.builder.program(), ty)?,
            ));
        }
        let arguments = self.builder.arguments(&information)?;
        self.effect(I::Runtime(R::EhFilter(arguments, Some(-1))))?;
        let source = self.result.ok_or(crate::sema::Error::Invariant(
            "specification without result",
        ))?;
        let selector = self
            .emit(I::Runtime(R::ExceptionSelector(Type::I32)), source)?
            .value;
        let violated = self.new_block("function_exception_action")?;
        let allowed = self.new_block("function_exception_resume")?;
        let minus_one = self.builder.integer(u128::MAX)?;
        let test = self
            .emit(
                I::Compare(Compare::Eq, Type::I32, selector, minus_one),
                source,
            )?
            .value;
        self.effect(I::Branch(test, violated, allowed))?;
        self.terminated = true;
        self.place(violated)?;
        let exception = self
            .emit(I::Runtime(R::Exception(Type::Ptr)), source)?
            .value;
        let callee = self.symbols.unexpected_function(self.builder.program())?;
        let arguments = self.builder.arguments(&[exception])?;
        self.effect(I::Call(Call {
            result: Type::Void,
            callee: Value::Symbol(callee),
            arguments,
            signature: None,
            copy_elision: false,
            argument_classes: Vec::new(),
        }))?;
        self.terminated = true;
        self.place(allowed)?;
        self.effect(I::Runtime(R::Resume))?;
        self.terminated = true;
        Ok(())
    }
    pub(super) fn terminate_exception(
        &mut self,
        exception: crate::lowir::model::Value,
    ) -> Result<(), Error> {
        let callee = self.symbols.terminate_helper(self.builder.program())?;
        let arguments = self.builder.arguments(&[exception])?;
        self.effect(I::Call(crate::lowir::model::Call {
            result: Type::Void,
            callee: crate::lowir::model::Value::Symbol(callee),
            arguments,
            signature: None,
            copy_elision: false,
            argument_classes: Vec::new(),
        }))
    }
    pub(super) fn terminate_current_exception(&mut self) -> Result<(), Error> {
        self.effect(I::Runtime(R::EhCatchAll(Some(1))))?;
        let slot = self.function_exception_slot()?;
        let source = self.result.ok_or(crate::sema::Error::Invariant(
            "termination without function",
        ))?;
        let exception = self
            .emit(I::Runtime(R::Exception(Type::Ptr)), source)?
            .value;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            exception,
            slot,
        ))?;
        let exception = self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)?
            .value;
        self.terminate_exception(exception)?;
        let ty = self.symbols.result_boundary(source)?.ty;
        self.termination_return(source, ty)
    }
    fn function_exception_slot(&mut self) -> Result<crate::lowir::model::Value, Error> {
        if let Some(slot) = self.lifetime.exception_slot {
            return Ok(slot);
        }
        let slot = self.generated_slot("function_exception", Type::Ptr)?;
        self.lifetime.exception_slot = Some(slot);
        Ok(slot)
    }
    pub(super) fn begin_lifetime_call(&mut self, throws: bool) -> Result<(), Error> {
        if throws && self.lifetime.full_expression {
            self.ensure_insertion()?;
            self.lifetime.call_marks.push(super::lifetime::CallMark {
                position: self
                    .lifetime
                    .call_start
                    .filter(|&position| self.builder.same_block(position))
                    .unwrap_or(self.builder.position()?),
                declared: false,
                depth: self.lifetime.cleanups.len(),
                regioned: false,
            });
        }
        Ok(())
    }
    pub(super) fn pause_call_region(&mut self) -> Result<(), Error> {
        if !self.lifetime.full_expression
            || self.lifetime.region
            || self.element_temporaries.depth != 0
            || self.handlers.explicit != 0
            || self.static_initializer
            || self.has_tracked_region()
            || self.terminated
            || self.lifetime.cleanups.len()
                <= self
                    .lifetime
                    .temporary_segment
                    .floor()
                    .max(self.handler_floor())
        {
            return Ok(());
        }
        let position = self.lifetime.call_marks.iter().find_map(|mark| {
            (self.builder.same_block(mark.position)
                && !mark.declared
                && !mark.regioned
                && mark.depth == self.lifetime.cleanups.len())
            .then_some(mark.position)
        });
        let Some(position) = position else {
            return Ok(());
        };
        if self.builder.region_since(position) {
            return Ok(());
        }
        self.open_temporary_region_at(Some(position))?;
        for mark in &mut self.lifetime.call_marks {
            mark.regioned |= self.builder.same_block(mark.position);
        }
        Ok(())
    }
    pub(super) fn begin_argument_call(
        &mut self,
        throws: bool,
        arguments: &[crate::syntax::ast::ExprId],
    ) -> Result<(), Error> {
        self.begin_lifetime_call(throws)?;
        // cleanup_lowering.cpp::note_call_arguments/begin_call: a return or
        // condition's hidden segment opens before a throwing call's operands
        // when they make temporaries, even if none has become alive yet.
        if self.lifetime.full_expression
            && self.lifetime.temporary_segment.allows_eager_arguments()
            && self.lifetime.region_start.is_pending()
            && !self.lifetime.region
        {
            for &argument in arguments {
                if self.expression_temporaries(argument)?.made {
                    self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Eager;
                    if throws {
                        self.open_temporary_region_at(self.lifetime.expression_start)?;
                    }
                    break;
                }
            }
        }
        Ok(())
    }
    pub(super) fn before_lifetime_call(&mut self, throws: bool) -> Result<(), Error> {
        if throws
            && !self.lifetime.region
            && self
                .lifetime
                .call_marks
                .last()
                .is_some_and(|mark| mark.regioned)
        {
            let boundary = self.lifetime.full_expression
                && self.lifetime.temporary_segment.is_present()
                && self.handlers.contexts.last().is_some_and(|c| !c.attempt);
            if !self.static_initializer
                && self.handlers.explicit == 0
                && (!self.lifetime_chain_empty() || boundary || self.lifetime.tracking.active)
            {
                let position = self.builder.position()?;
                return self.open_temporary_region_at(Some(position));
            }
            return Ok(());
        }
        if throws && self.lifetime.temporary_segment.floor() != 0 {
            self.drop_segment_floor();
            if self.lifetime.region && self.handlers.explicit == 0 {
                self.close_lifetime_region()?;
                // before_throwing_call reopens through open_region(false,
                // true): an enclosing try can own the unwind when the
                // widened segment still has no objects above its floor.
                let boundary = self.lifetime.full_expression
                    && self.lifetime.temporary_segment.is_present()
                    && self.handlers.contexts.last().is_some_and(|c| !c.attempt);
                if self.lifetime.cleanups.len() > self.handler_floor()
                    || boundary
                    || self.lifetime.tracking.active
                {
                    let position = self.builder.position()?;
                    return self.open_temporary_region_at(Some(position));
                }
                return Ok(());
            }
        }
        // A call closes only a region it opened, never an enclosing storage
        // initializer's region that was already active before its operands.
        self.lifetime.call_region = if throws
            && !self.lifetime.full_expression
            && !self.lifetime.construction
            && !self.lifetime.region
        {
            super::lifetime::CallRegion::Transient
        } else {
            super::lifetime::CallRegion::Enclosing
        };
        if self.element_temporaries.depth != 0 {
            return Ok(());
        }
        // emit_call invokes region_event only for a nonthrowing call.
        // A throwing call opens at its operand mark or segment start.
        let storage = !throws && self.storage_region_pending();
        if !throws {
            self.storage_region_event()?;
        }
        if storage && self.lifetime.region {
            self.lifetime.call_region = super::lifetime::CallRegion::Enclosing;
        }
        if throws && self.storage_region_pending() && self.lifetime.tracking.construction.active() {
            let position = self.builder.position()?;
            return self.open_temporary_region_at(Some(position));
        }
        if throws
            && self.lifetime.full_expression
            && self.lifetime.temporary_segment.is_eager()
            && self.lifetime.region_start.is_pending()
            && !self.lifetime.region
            && !self.lifetime.call_marks.is_empty()
        {
            return self.open_temporary_region_at(self.lifetime.expression_start);
        }
        if self.has_tracked_region() && !self.lifetime.region {
            return self.open_temporary_region();
        }
        // region_event: a temporary-making throwing segment also opens at
        // its first nonthrowing call while earlier objects are alive. A
        // pending operand call owns the start; otherwise it opens here,
        // after this call's operands (including a noexcept member copy).
        if !throws
            && self.lifetime.full_expression
            && !self.lifetime.region
            && !self.lifetime.tracking.active
            && self.lifetime.guarded_operands == 0
            && self.lifetime.choice_region != super::lifetime::ChoiceRegion::Reopened
            && self.handlers.explicit == 0
            && !self.static_initializer
            && self.lifetime.region_start.is_pending()
            && self.lifetime.temporary_segment.is_present()
            && !self.lifetime.temporary_segment.is_nothrow()
            && !self.lifetime.cleanups.is_empty()
        {
            let position = self
                .lifetime
                .call_marks
                .iter()
                .find(|mark| !mark.declared && self.builder.same_block(mark.position))
                .map_or(self.builder.position()?, |mark| mark.position);
            return self.open_temporary_region_at(Some(position));
        }
        if !throws
            && self.lifetime.full_expression
            && self.lifetime.guarded_operands == 0
            && self.lifetime.choice_region != super::lifetime::ChoiceRegion::Reopened
            && (self.lifetime.temporary_segment.is_nothrow()
                && self.lifetime.region_start.is_pending()
                || (self.lifetime.temporary_segment.is_nothrow()
                    || self
                        .lifetime
                        .temporary_split
                        .is_some_and(|position| self.builder.same_block(position)))
                    && self.lifetime.cleanups.len() > self.lifetime.temporary_segment.floor())
            && !self.lifetime.region
        {
            // region_event opens a nonthrowing segment at its start the
            // first time, then at the current call after a completion split.
            let position = if self.lifetime.temporary_segment.is_nothrow()
                && self.lifetime.region_start.is_pending()
            {
                self.lifetime.expression_start
            } else {
                Some(self.builder.position()?)
            };
            return self.open_temporary_region_at(position);
        }
        if self.handlers.explicit != 0 {
            return Ok(());
        }
        // Even without local objects, a temporary-making expression in an
        // active handler must leave that handler if one of its calls throws.
        let boundary = self.lifetime.full_expression
            && self.lifetime.temporary_segment.is_present()
            && self.handlers.contexts.last().is_some_and(|c| !c.attempt);
        if self.static_initializer
            || !throws
            || self.lifetime.region
            || self.lifetime.cleanups.len() <= self.handler_floor() && !boundary
        {
            return Ok(());
        }
        let entry = self.unwind_entry()?;
        if let Some(floor) = self.aggregation.floor {
            self.aggregation.landing |= self.lifetime.cleanups[floor..].iter().any(|c| c.member());
        }
        let call_start = if self.lifetime.region_start.is_pending() {
            self.lifetime
                .call_marks
                .iter()
                .find(|mark| self.builder.same_block(mark.position))
        } else {
            // open_region can reuse the innermost call's operand start only
            // while its live prefix is unchanged. A newly completed by-value
            // argument belongs in the call's handler, not its construction's.
            self.lifetime.call_marks.last().filter(|mark| {
                self.builder.same_block(mark.position) && mark.depth == self.lifetime.cleanups.len()
            })
        }
        .map(|mark| mark.position);
        if std::mem::take(&mut self.lifetime.result_region)
            == super::class_results::ResultRegion::Construction
            || self.lifetime.temporary_segment.is_nothrow()
            || self
                .lifetime
                .temporary_split
                .is_some_and(|position| self.builder.same_block(position))
            || !self.lifetime.region_start.is_pending()
                && self
                    .lifetime
                    .storage_split
                    .is_some_and(|position| self.builder.same_block(position))
        {
            self.effect(I::Runtime(R::EhTry(entry)))?;
        } else if let Some(position) = call_start {
            self.builder.insert_marker(position, R::EhTry(entry))?;
        } else if self.lifetime.full_expression
            && self.lifetime.region_start.is_pending()
            && let Some(position) = self
                .lifetime
                .expression_start
                .filter(|&position| self.builder.same_block(position))
        {
            self.builder.insert_marker(position, R::EhTry(entry))?;
        } else if self.lifetime.construction
            && self.lifetime.construction_flow
                != super::construction_flow::ConstructionFlow::Branched
            && !self.lifetime.full_expression
            && let Some((position, depth)) = self.lifetime.construction_start
            && !self.builder.same_block(position)
            && depth == self.lifetime.cleanups.len()
        {
            self.builder.insert_marker(position, R::EhTry(entry))?;
        } else {
            self.effect(I::Runtime(R::EhTry(entry)))?;
        }
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        self.lifetime.region = true;
        self.lifetime.region_entry = Some(entry);
        self.lifetime.region_depth = self.lifetime.cleanups.len();
        self.lifetime.regions_opened += 1;
        Ok(())
    }
    pub(super) fn open_temporary_region(&mut self) -> Result<(), Error> {
        self.open_temporary_region_at(None)
    }
    pub(super) fn open_temporary_region_at(
        &mut self,
        start: Option<crate::lowir::builder::Position>,
    ) -> Result<(), Error> {
        if self.element_temporaries.depth != 0 {
            return Ok(());
        }
        if self.has_tracked_region() {
            let position = self
                .lifetime
                .call_marks
                .iter()
                .filter(|mark| !mark.declared)
                .map(|mark| mark.position)
                .find(|&p| self.builder.same_block(p));
            return self.tracked_region(position);
        }
        let entry = self.unwind_entry()?;
        self.note_aggregate_landing();
        let start = if std::mem::take(&mut self.lifetime.result_region)
            == super::class_results::ResultRegion::Construction
        {
            Some(self.builder.position()?)
        } else {
            start
        };
        if let Some(position) = start
            .filter(|&position| self.builder.same_block(position))
            .or_else(|| {
                self.lifetime
                    .call_marks
                    .iter()
                    .map(|mark| mark.position)
                    .find(|&position| self.builder.same_block(position))
            })
        {
            self.builder.insert_marker(position, R::EhTry(entry))?;
        } else {
            self.effect(I::Runtime(R::EhTry(entry)))?;
        }
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        self.lifetime.region = true;
        self.lifetime.region_entry = Some(entry);
        self.lifetime.region_depth = self.lifetime.cleanups.len();
        self.lifetime.regions_opened += 1;
        Ok(())
    }
    pub(super) fn finish_lifetime_call(
        &mut self,
        value: RValue,
        throws: bool,
    ) -> Result<RValue, Error> {
        let result = self.finish_lifetime_value(value.value, value.ty, throws)?;
        Ok(RValue {
            value: result,
            ..value
        })
    }
    /// Preserve a call result using its `LowIR` type, including compiler-created
    /// calls that have no source-language result type (TLS wrappers).
    pub(super) fn finish_lifetime_value(
        &mut self,
        value: Value,
        ty: Type,
        throws: bool,
    ) -> Result<Value, Error> {
        if throws && self.lifetime.full_expression {
            self.lifetime.call_marks.pop();
        }
        let transient = !self.lifetime.full_expression && !self.lifetime.construction;
        if std::mem::take(&mut self.lifetime.call_region) == super::lifetime::CallRegion::Transient
        {
            self.close_lifetime_region()?;
        }
        // A pending outer call covers a nonthrowing receiver call's result
        // before its region marker is inserted. It is not an uncovered call
        // preceding all of the segment's calls and temporaries.
        if !transient
            && (throws || !self.has_tracked_region())
            && (self.lifetime.region
                || self.lifetime.full_expression
                    && (self.lifetime.temporary_segment.is_present()
                        || !throws && !self.lifetime.call_marks.is_empty()))
            && !matches!(ty, Type::Void | Type::Object { .. })
        {
            let slot = self.generated_slot("call", ty)?;
            self.effect(I::Store(
                Facts::memory(
                    false,
                    !matches!(ty, Type::I8 | Type::U8 | Type::Object { .. }),
                ),
                ty,
                value,
                slot,
            ))?;
            self.ensure_insertion()?;
            return Ok(self.builder.generated(I::Load(
                Facts::memory(
                    false,
                    !matches!(ty, Type::I8 | Type::U8 | Type::Object { .. }),
                ),
                ty,
                slot,
            ))?);
        }
        Ok(value)
    }
    pub(super) fn close_lifetime_region(&mut self) -> Result<(), Error> {
        self.close_lifetime_region_to("call_unwind_end")
    }
    pub(super) fn close_lifetime_region_to(&mut self, continuation: &str) -> Result<(), Error> {
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        if !self.lifetime.region {
            return Ok(());
        }
        self.lifetime.region = false;
        if self.drop_empty_lifetime_region()? {
            self.advance_construction_start()?;
            return Ok(());
        }
        self.effect(I::Runtime(R::EhEnd))?;
        let reused = self.reused_dropped_chains();
        if self.lifetime_has_pending_chains()
            && !reused
            && self.lifetime.landing_placement == super::class_results::LandingPlacement::Immediate
        {
            let next = self.new_block(continuation)?;
            self.jump(next)?;
            self.place_cleanup_chains(false)?;
            self.place(next)?;
        }
        if self.lifetime.full_expression && !self.terminated {
            let position = self.builder.position()?;
            for mark in &mut self.lifetime.call_marks {
                if self.builder.same_block(mark.position) {
                    mark.position = position;
                }
            }
        }
        self.advance_construction_start()
    }
    fn advance_construction_start(&mut self) -> Result<(), Error> {
        if self.lifetime.construction
            && !self.lifetime.full_expression
            && !self.terminated
            && let Some((position, _)) = &mut self.lifetime.construction_start
        {
            *position = self.builder.position()?;
        }
        Ok(())
    }
    pub(super) fn begin_lifetime_expression(&mut self) -> bool {
        if self.lifetime.full_expression || self.lifetime.cleanups.len() <= self.handler_floor() {
            return false;
        }
        self.lifetime.full_expression = true;
        self.lifetime.region_start = super::lifetime::RegionStart::Pending;
        true
    }
    pub(super) fn discard_full_expression(
        &mut self,
        id: crate::syntax::ast::ExprId,
    ) -> Result<(), Error> {
        let first = self.lifetime.cleanups.len();
        let began =
            self.begin_full_expression(Some(id), super::temporary_scan::ExpressionUse::Ordinary)?;
        self.with_full_root(Some(id), |this| this.discard(id))?;
        self.destroy_temporaries(first)?;
        self.end_lifetime_expression(began)
    }
    pub(super) fn end_lifetime_expression(&mut self, began: bool) -> Result<(), Error> {
        if began {
            self.close_lifetime_region()?;
            self.finish_tracking()?;
            self.end_tracked_cleanups();
            self.lifetime.tracking.active = false;
            self.lifetime.tracking.root = None;
            self.drop_segment_floor();
            self.lifetime.full_expression = false;
            self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Empty;
            self.lifetime.expression_start = None;
            self.lifetime.storage_split = None;
            self.lifetime.temporary_split = None;
        }
        Ok(())
    }
    pub(super) fn lifetime_call_throws(
        &self,
        target: crate::sema::lowering::CallTarget,
    ) -> Result<bool, Error> {
        // call_may_throw brackets ordinary functions in a hidden temporary
        // segment even when their declaration is noexcept. The declaration's
        // unwind fact remains unchanged; this decides the segment's regions
        // and scalar result staging, including converted class receivers.
        self.call_throws_in_context(
            target,
            self.lifetime.temporary_segment.allows_eager_arguments(),
        )
    }
    pub(super) fn call_throws_in_context(
        &self,
        target: crate::sema::lowering::CallTarget,
        hidden: bool,
    ) -> Result<bool, Error> {
        Ok(match target {
            crate::sema::lowering::CallTarget::Direct(entity) => {
                let facts = self.graph.lower_function(entity)?;
                facts.builtin.is_none()
                    && (!facts.nothrow
                        || hidden
                            && !matches!(
                                facts.name,
                                crate::sema::lowering::FunctionName::Destructor(_)
                            ))
            }
            crate::sema::lowering::CallTarget::Indirect(_, nothrow) => hidden || !nothrow,
        })
    }
    pub(super) fn lifetime_result(&mut self, call: I, source: TypeId) -> Result<RValue, Error> {
        if types::storage(self.graph, source)? == Type::Void {
            self.effect(call)?;
            self.zero(source)
        } else {
            self.emit(call, source)
        }
    }
}
