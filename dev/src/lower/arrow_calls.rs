// Translated from lower/expression_lowering.cpp's member-call path. See NOTICE.
//! Arrow uses consume their bounded semantic call chain and ordinary result/lifetime boundaries.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{Call, InstructionKind as I},
        vocabulary::Type,
    },
    sema::lowering::{CallTarget, TypeView},
    syntax::ast,
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn arrow_operand(&mut self, base: ast::ExprId) -> Result<RValue, Error> {
        let Some(last) = self.graph.lower_arrow_count(base).checked_sub(1) else {
            return self.operand(base, Context::Operand);
        };
        self.arrow_call(base, last)
    }
    fn arrow_call(&mut self, base: ast::ExprId, index: usize) -> Result<RValue, Error> {
        self.check_stack()?;
        let step = self
            .graph
            .lower_arrow_call(base, index)?
            .ok_or(crate::sema::Error::Invariant("missing arrow call"))?;
        let target = CallTarget::Direct(step.function);
        let throws = self.lifetime_call_throws(target)?;
        self.begin_lifetime_call(throws)?;
        let source = step.result;
        let class_result =
            !step.place && matches!(self.graph.lower_type(source)?, TypeView::Class(_));
        let destroyed = class_result
            && self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .storage_destructor(source)?
                .is_some();
        if destroyed
            && self.lifetime.guarded_operands != 0
            && !self.lifetime.tracking.active
            && self.lifetime.guard_start.is_none()
        {
            return Err(Error::Missing("guarded full-expression temporary cleanup"));
        }
        if destroyed && self.lifetime.full_expression && !self.lifetime.region {
            self.open_temporary_region()?;
        }
        let destination = if class_result {
            let slot = self.temporary_storage(source, false, "tmpobj")?;
            Some(self.emit(I::Addr(slot), source)?.value)
        } else {
            None
        };
        let mut receiver = if let Some(previous) = index.checked_sub(1) {
            self.arrow_call(base, previous)?.value
        } else {
            self.address(base)?
        };
        if let Some(offset) = step.offset {
            receiver = self.base_projection(receiver, source, offset)?;
        }
        let function = self.graph.lower_function(step.function)?;
        let TypeView::Function { result, .. } = self.graph.lower_type(function.ty)? else {
            return Err(crate::sema::Error::Invariant("arrow call without signature").into());
        };
        let boundary = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .result_boundary(result)?;
        let mut values = [receiver; 2];
        if boundary.indirect {
            values[0] = destination.ok_or(crate::sema::Error::Invariant(
                "arrow result without destination",
            ))?;
            values[1] = receiver;
        }
        let (callee, signature) = self.member_callee(
            step.function,
            receiver,
            self.graph.lower_virtual_slot(step.function).is_some(),
            source,
        )?;
        let arguments = self
            .builder
            .arguments(&values[..=usize::from(boundary.indirect)])?;
        let call = I::Call(Call {
            result: boundary.ty,
            callee,
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature,
        });
        self.before_lifetime_call(throws)?;
        let mut value = self.finish_result_call(call, source, destination, throws)?;
        if let Some(object) = destination {
            if destroyed {
                self.temporary_alive(source, object)?;
            }
            value = RValue {
                value: object,
                ty: Type::Ptr,
                source,
                literal: false,
                bits: None,
            };
        } else if step.place && !matches!(self.graph.lower_type(source)?, TypeView::Class(_)) {
            value = self.read_place(value.value, source)?;
        }
        Ok(value)
    }
}
