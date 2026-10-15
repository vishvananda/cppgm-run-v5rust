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
        model::{Facts, InstructionKind as I, RuntimeInstruction as R},
        vocabulary::Type,
    },
    sema::lowering::TypeId,
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn temporary_alive(
        &mut self,
        ty: TypeId,
        object: crate::lowir::model::Value,
    ) -> Result<(), Error> {
        if self.lifetime.guarded_operands != 0 {
            return Err(Error::Missing("guarded full-expression temporary cleanup"));
        }
        let split = self.lifetime.region;
        self.close_lifetime_region()?;
        self.register_temporary(ty, object);
        if split {
            self.open_temporary_region()?;
        }
        Ok(())
    }
    pub(super) fn open_function_guard(&mut self) -> Result<(), Error> {
        let landing = self.new_block("function_exception_landing")?;
        self.lifetime.function_guard = Some(landing);
        self.effect(I::Runtime(R::EhTry(landing)))
    }
    pub(super) fn place_function_guard(&mut self) -> Result<(), Error> {
        let Some(landing) = self.lifetime.function_guard.take() else {
            return Ok(());
        };
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
        let value = self.zero_return(source)?;
        self.effect(I::Return(ty, value))?;
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
        }))
    }
    fn function_exception_slot(&mut self) -> Result<crate::lowir::model::Value, Error> {
        if let Some(slot) = self.lifetime.exception_slot {
            return Ok(slot);
        }
        let slot = self.generated_slot("function_exception", Type::Ptr)?;
        self.lifetime.exception_slot = Some(slot);
        Ok(slot)
    }
    pub(super) fn terminate_unwinding(&mut self) -> Result<(), Error> {
        self.effect(I::Runtime(R::EhCatchAll(Some(1))))?;
        let slot = self.function_exception_slot()?;
        let source = self
            .result
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "synthetic terminate result",
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
        let exception = self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)?
            .value;
        self.terminate_exception(exception)?;
        let ty = self.symbols.result_boundary(source)?.ty;
        let value = self.zero_return(source)?;
        let guard = self.lifetime.function_guard.take();
        self.effect(I::Return(ty, value))?;
        self.lifetime.function_guard = guard;
        self.terminated = true;
        Ok(())
    }
    pub(super) fn begin_lifetime_call(&mut self, throws: bool) -> Result<(), Error> {
        if throws && self.lifetime.full_expression {
            self.ensure_insertion()?;
            self.lifetime.call_marks.push(self.builder.position()?);
        }
        Ok(())
    }
    pub(super) fn before_lifetime_call(&mut self, throws: bool) -> Result<(), Error> {
        if self.static_initializer
            || !throws
            || self.lifetime.region
            || self.lifetime.cleanups.is_empty()
        {
            return Ok(());
        }
        let entry = self.unwind_entry()?;
        if let Some(floor) = self.aggregation.floor {
            self.aggregation.landing |= self.lifetime.cleanups[floor..].iter().any(|c| c.member());
        }
        if let Some(position) = self
            .lifetime
            .call_marks
            .iter()
            .copied()
            .find(|&position| self.builder.same_block(position))
        {
            self.builder.insert_marker(position, R::EhTry(entry))?;
        } else {
            self.effect(I::Runtime(R::EhTry(entry)))?;
        }
        self.lifetime.region = true;
        self.lifetime.region_depth = self.lifetime.cleanups.len();
        Ok(())
    }
    pub(super) fn open_temporary_region(&mut self) -> Result<(), Error> {
        self.open_temporary_region_at(None)
    }
    pub(super) fn open_temporary_region_at(
        &mut self,
        start: Option<crate::lowir::builder::Position>,
    ) -> Result<(), Error> {
        let entry = self.unwind_entry()?;
        if let Some(position) = start
            .filter(|&position| self.builder.same_block(position))
            .or_else(|| {
                self.lifetime
                    .call_marks
                    .iter()
                    .copied()
                    .find(|&position| self.builder.same_block(position))
            })
        {
            self.builder.insert_marker(position, R::EhTry(entry))?;
        } else {
            self.effect(I::Runtime(R::EhTry(entry)))?;
        }
        self.lifetime.region = true;
        self.lifetime.region_depth = self.lifetime.cleanups.len();
        Ok(())
    }
    pub(super) fn finish_lifetime_call(
        &mut self,
        value: RValue,
        throws: bool,
    ) -> Result<RValue, Error> {
        if throws && self.lifetime.full_expression {
            self.lifetime.call_marks.pop();
        }
        let transient = !self.lifetime.full_expression && !self.lifetime.construction;
        if transient {
            self.close_lifetime_region()?;
        }
        if !transient
            && self.lifetime.region
            && !matches!(value.ty, Type::Void | Type::Object { .. })
        {
            let slot = self.generated_slot("call", value.ty)?;
            self.effect(I::Store(
                Facts::memory(false, !matches!(value.ty, Type::I8 | Type::U8)),
                value.ty,
                value.value,
                slot,
            ))?;
            return self.emit(
                I::Load(
                    Facts::memory(false, !matches!(value.ty, Type::I8 | Type::U8)),
                    value.ty,
                    slot,
                ),
                value.source,
            );
        }
        Ok(value)
    }
    pub(super) fn close_lifetime_region(&mut self) -> Result<(), Error> {
        if !self.lifetime.region {
            return Ok(());
        }
        self.lifetime.region = false;
        self.effect(I::Runtime(R::EhEnd))?;
        if self.lifetime_has_pending_chains() {
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            self.place_cleanup_chains(false)?;
            self.place(next)?;
        }
        Ok(())
    }
    pub(super) fn begin_lifetime_expression(&mut self) -> bool {
        if self.lifetime.full_expression || self.lifetime.cleanups.is_empty() {
            return false;
        }
        self.lifetime.full_expression = true;
        true
    }
    pub(super) fn discard_full_expression(
        &mut self,
        id: crate::syntax::ast::ExprId,
    ) -> Result<(), Error> {
        let first = self.lifetime.cleanups.len();
        let began = self.begin_full_expression(Some(id))?;
        self.discard(id)?;
        self.destroy_temporaries(first)?;
        self.end_lifetime_expression(began)
    }
    pub(super) fn end_lifetime_expression(&mut self, began: bool) -> Result<(), Error> {
        if began {
            self.close_lifetime_region()?;
            self.lifetime.full_expression = false;
            self.lifetime.expression_start = None;
        }
        Ok(())
    }
    pub(super) fn lifetime_call_throws(
        &self,
        target: crate::sema::lowering::CallTarget,
    ) -> Result<bool, Error> {
        Ok(match target {
            crate::sema::lowering::CallTarget::Direct(entity) => {
                let facts = self.graph.lower_function(entity)?;
                facts.builtin.is_none() && !facts.nothrow
            }
            crate::sema::lowering::CallTarget::Indirect(_, nothrow) => !nothrow,
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
