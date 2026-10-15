// Translated from lower/conversion_lowering.cpp::call_conversion/converted_value.
// See NOTICE.
//! Selected scalar conversions borrow the original receiver and ordinary call lifetime state.
use super::{
    Error,
    function::{Function, RValue},
};
use crate::{
    lowir::model::{Call, InstructionKind as I},
    sema::lowering::{CallTarget, MemberCall},
    syntax::ast,
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn bind_function_conversion(
        &mut self,
        id: ast::ExprId,
        call: MemberCall,
        element: crate::sema::lowering::TypeId,
        persistent: bool,
    ) -> Result<crate::lowir::model::Value, Error> {
        if call.place {
            let address = self
                .call_conversion(
                    id,
                    MemberCall {
                        place: false,
                        ..call
                    },
                )?
                .value;
            return if let Some(offset) = self.graph.lower_base_conversion_offset(id)? {
                self.base_projection(address, element, offset)
            } else {
                Ok(address)
            };
        }
        if matches!(
            self.graph.lower_type(call.result)?,
            crate::sema::lowering::TypeView::Class(_)
        ) {
            return Err(Error::Missing("conversion-function reference temporary"));
        }
        let value = self.call_conversion(id, call)?;
        let value = self.convert(value, element, super::conversions::Context::Initialization)?;
        let value = self.fit(value, element)?;
        let storage = super::types::storage(self.graph, element)?;
        let slot = self.temporary_storage(element, persistent, "tmpref")?;
        self.effect(I::Store(
            crate::lowir::model::Facts::memory(
                false,
                !matches!(
                    storage,
                    crate::lowir::vocabulary::Type::I8 | crate::lowir::vocabulary::Type::U8
                ),
            ),
            storage,
            value.value,
            slot,
        ))?;
        Ok(self.emit(I::Addr(slot), element)?.value)
    }
    pub(super) fn condition_conversion_call(
        &mut self,
        place: crate::lowir::model::Value,
        source: crate::sema::lowering::TypeId,
        call: MemberCall,
    ) -> Result<RValue, Error> {
        use crate::lowir::{model::RuntimeInstruction as R, vocabulary::Type};
        let guarded = self.lifetime_call_throws(CallTarget::Direct(call.function))?
            && !self.lifetime.cleanups.is_empty()
            && !self.lifetime.full_expression;
        let dispatch = if guarded {
            let block = self.new_block("call_unwind_dispatch")?;
            self.effect(I::Runtime(R::EhTry(block)))?;
            Some(block)
        } else {
            None
        };
        let full = self.lifetime.full_expression;
        let region = self.lifetime.region;
        if guarded {
            self.lifetime.full_expression = true;
            self.lifetime.region = true;
        }
        let receiver = if matches!(
            self.graph.lower_type(source)?,
            crate::sema::lowering::TypeView::Reference { .. }
        ) {
            self.emit(
                I::Load(
                    crate::lowir::model::Facts::memory(false, true),
                    Type::Ptr,
                    place,
                ),
                source,
            )?
            .value
        } else {
            self.emit(I::Addr(place), source)?.value
        };
        // The frozen condition-variable path consumes the call's boundary
        // value directly, including a reference result, before its bool step.
        let value = self.call_conversion_at(
            receiver,
            MemberCall {
                place: false,
                ..call
            },
        )?;
        self.lifetime.full_expression = full;
        self.lifetime.region = region;
        if let Some(dispatch) = dispatch {
            self.effect(I::Runtime(R::EhEnd))?;
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            self.place(dispatch)?;
            let depth = self.lifetime.cleanups.len();
            let cleanup = self.lifetime.cleanups[depth - 1];
            self.destroy_cleanup_in(cleanup, None, true)?;
            if depth > 1 {
                let tail = self.cleanup_prefix(true, depth - 1)?;
                self.jump(tail)?;
            } else {
                self.effect(I::Runtime(R::Resume))?;
                self.terminated = true;
            }
            self.place_cleanup_chains(false)?;
            self.place(next)?;
        }
        Ok(value)
    }
    pub(super) fn call_conversion(
        &mut self,
        id: ast::ExprId,
        call: MemberCall,
    ) -> Result<RValue, Error> {
        self.call_conversion_value(id, call, self.graph.lower_builtin_operand(id))
    }
    pub(super) fn call_truth_conversion(
        &mut self,
        id: ast::ExprId,
        call: MemberCall,
    ) -> Result<RValue, Error> {
        // Contextual boolean conversion uses converted_operand's qualified
        // glvalue read, just like a converted builtin operand.
        self.call_conversion_value(id, call, true)
    }
    fn call_conversion_value(
        &mut self,
        id: ast::ExprId,
        call: MemberCall,
        qualified: bool,
    ) -> Result<RValue, Error> {
        let throws = self.lifetime_call_throws(CallTarget::Direct(call.function))?;
        self.begin_lifetime_call(throws)?;
        let source = self.graph.lower_expression(id)?.ty;
        let place = self.original_scalar_place(self.without_parentheses(id))?;
        let receiver = self.storage_address(place, source)?;
        self.emit_conversion_call(receiver, call, throws, qualified)
    }
    pub(super) fn call_conversion_at(
        &mut self,
        receiver: crate::lowir::model::Value,
        call: MemberCall,
    ) -> Result<RValue, Error> {
        let throws = self.lifetime_call_throws(CallTarget::Direct(call.function))?;
        self.begin_lifetime_call(throws)?;
        self.emit_conversion_call(receiver, call, throws, false)
    }
    fn emit_conversion_call(
        &mut self,
        mut receiver: crate::lowir::model::Value,
        call: MemberCall,
        throws: bool,
        qualified: bool,
    ) -> Result<RValue, Error> {
        if let Some(offset) = call.offset {
            receiver = self.base_projection(receiver, call.result, offset)?;
        }
        self.symbols.demand(call.function);
        let function = self.graph.lower_function(call.function)?;
        let crate::sema::lowering::TypeView::Function { result, .. } =
            self.graph.lower_type(function.ty)?
        else {
            return Err(crate::sema::Error::Invariant("conversion call signature").into());
        };
        let boundary = self.symbols.result_boundary(result)?;
        let callee = self.symbols.callee(call.function, &mut self.builder)?;
        let arguments = self.builder.arguments(&[receiver])?;
        let instruction = I::Call(Call {
            result: boundary.ty,
            callee,
            arguments,
            copy_elision: false,
            signature: None,
        });
        self.before_lifetime_call(throws)?;
        let value = self.finish_result_call(instruction, call.result, None, throws)?;
        if call.place {
            let ty = super::types::storage(self.graph, call.result)?;
            // A builtin conversion result is a normal glvalue. Ordinary scalar
            // initialization uses converted_value's unqualified Place instead.
            let memory = if qualified
                && matches!(
                    self.graph.lower_type(call.result)?,
                    crate::sema::lowering::TypeView::Array { .. }
                ) {
                crate::lowir::model::Facts::default()
            } else if qualified {
                super::types::memory(self.graph, call.result)?
            } else {
                crate::lowir::model::Facts::memory(
                    false,
                    !matches!(
                        ty,
                        crate::lowir::vocabulary::Type::I8 | crate::lowir::vocabulary::Type::U8
                    ),
                )
            };
            self.emit(I::Load(memory, ty, value.value), call.result)
        } else {
            Ok(value)
        }
    }
}
