// Translated from lower/conversion_lowering.cpp::call_conversion/converted_value.
// See NOTICE.
//! Selected conversions borrow original receivers and own only their result destinations and lifetimes.
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
            return if let Some(offset) = self.graph.lower_base_conversion(id)? {
                self.base_projection(address, element, offset)
            } else {
                Ok(address)
            };
        }
        if matches!(
            self.graph.lower_type(call.result)?,
            crate::sema::lowering::TypeView::Class(_)
        ) {
            let address = self.conversion_temporary(id, call, persistent)?;
            return if let Some(offset) = self.graph.lower_base_conversion(id)? {
                self.base_projection(address, element, offset)
            } else {
                Ok(address)
            };
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
    fn conversion_receiver(
        &mut self,
        id: ast::ExprId,
    ) -> Result<crate::lowir::model::Value, Error> {
        let facts = self.graph.lower_expression(id)?;
        let place = self.original_scalar_place(self.without_parentheses(id))?;
        if !facts.is_place && self.symbols.class_destructor(facts.ty)?.is_none() {
            self.finish_inert_temporary(false)?;
        }
        self.storage_address(place, facts.ty)
    }
    pub(super) fn stage_local_conversion(
        &mut self,
        ty: crate::sema::lowering::TypeId,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        use crate::sema::lowering::TypeView;
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Ok(());
        };
        let Some(ast::Initializer {
            kind: ast::InitializerKind::Assign(source),
            ..
        }) = initializer
        else {
            return Ok(());
        };
        let Some((call, _)) = self.graph.lower_function_conversion(source)? else {
            return Ok(());
        };
        if self.graph.lower_aggregate_class(class) {
            let signature = self.graph.lower_function(call.function)?.ty;
            let TypeView::Function { result, .. } = self.graph.lower_type(signature)? else {
                return Err(crate::sema::Error::Invariant("staged conversion signature").into());
            };
            if !self.symbols.result_boundary(result)?.indirect {
                // local_object plans this staging slot even when the converter
                // initializes the variable itself; its identity is observable.
                self.generated_slot("argobj", super::types::storage(self.graph, ty)?)?;
            }
        }
        Ok(())
    }
    pub(super) fn conversion_temporary(
        &mut self,
        id: ast::ExprId,
        call: MemberCall,
        persistent: bool,
    ) -> Result<crate::lowir::model::Value, Error> {
        let destroyed = !persistent && self.symbols.class_destructor(call.result)?.is_some();
        let opens = destroyed
            && self.lifetime.full_expression
            && !self.lifetime.region
            && (self.lifetime.region_start.is_pending()
                || self.lifetime.cleanups.is_empty()
                || self.aggregation.floor.is_some());
        let deferred = opens
            && !self.lifetime.cleanups.is_empty()
            && !self.lifetime.tracking.active
            && !self
                .lifetime
                .call_marks
                .iter()
                .any(|mark| !mark.declared && self.builder.same_block(mark.position));
        if opens && !deferred {
            let start = self
                .lifetime
                .region_start
                .is_pending()
                .then_some(self.lifetime.expression_start)
                .flatten();
            self.open_temporary_region_at(start)?;
        }
        let slot = self.temporary_storage(call.result, persistent, "arg")?;
        let address = self.emit(I::Addr(slot), call.result)?.value;
        if deferred && !self.lifetime.region {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        self.conversion_call_into(id, call, Some(address))?;
        if destroyed {
            self.temporary_alive(call.result, address)?;
        } else if persistent {
            self.persistent_alive(call.result, slot)?;
        }
        Ok(address)
    }
    fn conversion_call_into(
        &mut self,
        id: ast::ExprId,
        call: MemberCall,
        destination: Option<crate::lowir::model::Value>,
    ) -> Result<RValue, Error> {
        let throws = self.lifetime_call_throws(CallTarget::Direct(call.function))?;
        // pending_mark_region only uses marks in the segment's first block.
        // After a split, the conversion's region begins at its call, after
        // evaluating the receiver, rather than enclosing that evaluation.
        let split = !self.lifetime.region_start.is_pending()
            && self
                .lifetime
                .expression_start
                .is_some_and(|start| !self.builder.same_block(start));
        if !split {
            self.begin_lifetime_call(throws)?;
        }
        let receiver = self.conversion_receiver(id)?;
        if split {
            self.begin_lifetime_call(throws)?;
        }
        self.emit_conversion_call(receiver, call, throws, false, destination)
    }
    pub(super) fn elide_conversion_copy(
        &mut self,
        id: ast::ExprId,
        constructor: crate::sema::lowering::ConstructorId,
        object: crate::lowir::model::Value,
    ) -> Result<bool, Error> {
        if self.graph.lower_elides_conversion(id, constructor) {
            let (call, _) =
                self.graph
                    .lower_function_conversion(id)?
                    .ok_or(crate::sema::Error::Invariant(
                        "elided conversion without call",
                    ))?;
            self.conversion_call_into(id, call, Some(object))?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn initialize_function_conversion(
        &mut self,
        id: ast::ExprId,
        ty: crate::sema::lowering::TypeId,
        object: crate::lowir::model::Value,
    ) -> Result<bool, Error> {
        use crate::sema::lowering::CopyAction;
        let Some((call, _)) = self.graph.lower_function_conversion(id)? else {
            return Ok(false);
        };
        let Some(copy) = self.graph.lower_conversion_copy(id) else {
            self.conversion_call_into(id, call, Some(object))?;
            return Ok(true);
        };
        let mut source = if call.place {
            self.conversion_call_into(
                id,
                MemberCall {
                    place: false,
                    ..call
                },
                None,
            )?
            .value
        } else {
            self.conversion_temporary(id, call, false)?
        };
        if let Some(offset) = self.graph.lower_base_conversion(id)? {
            if !call.place {
                let slot = self.builder.addressed_storage(source);
                source = self.emit(I::Addr(slot), call.result)?.value;
            }
            source = self.base_projection(source, ty, offset)?;
        }
        match copy {
            CopyAction::Storage { .. } => {
                let crate::sema::lowering::TypeView::Class(class) = self.graph.lower_type(ty)?
                else {
                    return Err(crate::sema::Error::Invariant("conversion copy class").into());
                };
                if !self.graph.lower_class_empty(class)? {
                    let (bytes, align) = super::types::layout(self.graph, ty)?;
                    let bytes = bytes.min(self.graph.lower_nonvirtual_bytes(class)?);
                    self.effect(I::CopyObject(bytes, align, source, object))?;
                }
            }
            CopyAction::Constructor { constructor, .. } => self.call_copy_constructor(
                constructor,
                super::construction::Variant::Complete,
                object,
                source,
            )?,
        }
        Ok(true)
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
        let contexts = !self.handlers.contexts.is_empty();
        let dispatch = if guarded {
            let block = if contexts {
                self.context_landing(self.lifetime.cleanups.len())?
            } else {
                self.new_block("call_unwind_dispatch")?
            };
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
            if !contexts {
                self.place(dispatch)?;
                let depth = self.lifetime.cleanups.len();
                let cleanup = self.lifetime.cleanups[depth - 1];
                self.destroy_cleanup_in(cleanup, None, true)?;
                if depth > 1 {
                    let tail = self.cleanup_prefix(true, depth - 1)?;
                    self.jump(tail)?;
                } else {
                    self.handler_resume()?;
                }
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
        if let Some(invoker) = self.graph.lower_closure_converter(call.function) {
            return self.convert_closure(id, invoker, call);
        }
        let throws = self.lifetime_call_throws(CallTarget::Direct(call.function))?;
        self.begin_lifetime_call(throws)?;
        let receiver = self.conversion_receiver(id)?;
        self.emit_conversion_call(receiver, call, throws, qualified, None)
    }
    pub(super) fn call_conversion_at(
        &mut self,
        receiver: crate::lowir::model::Value,
        call: MemberCall,
    ) -> Result<RValue, Error> {
        let throws = self.lifetime_call_throws(CallTarget::Direct(call.function))?;
        self.begin_lifetime_call(throws)?;
        self.emit_conversion_call(receiver, call, throws, false, None)
    }
    fn emit_conversion_call(
        &mut self,
        mut receiver: crate::lowir::model::Value,
        call: MemberCall,
        throws: bool,
        qualified: bool,
        destination: Option<crate::lowir::model::Value>,
    ) -> Result<RValue, Error> {
        if let Some(offset) = call.offset {
            receiver = self.base_projection(receiver, call.result, offset)?;
        }
        let function = self.graph.lower_function(call.function)?;
        let crate::sema::lowering::TypeView::Function { result, .. } =
            self.graph.lower_type(function.ty)?
        else {
            return Err(crate::sema::Error::Invariant("conversion call signature").into());
        };
        let boundary = self.symbols.result_boundary(result)?;
        let (callee, signature) = self.member_callee(
            call.function,
            receiver,
            self.graph.lower_virtual_slot(call.function).is_some(),
            call.result,
        )?;
        let arguments = if boundary.indirect {
            self.builder.arguments(&[
                destination.ok_or(crate::sema::Error::Invariant(
                    "conversion result without destination",
                ))?,
                receiver,
            ])?
        } else {
            self.builder.arguments(&[receiver])?
        };
        let instruction = I::Call(Call {
            result: boundary.ty,
            callee,
            arguments,
            copy_elision: false,
            signature,
        });
        self.before_lifetime_call(throws)?;
        let value = self.finish_result_call(instruction, call.result, destination, throws)?;
        if call.place
            && !matches!(
                self.graph.lower_type(call.result)?,
                crate::sema::lowering::TypeView::Function { .. }
            )
        {
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
