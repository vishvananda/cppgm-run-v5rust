// Translated from lower/{function,expression,object}_lowering.cpp. See NOTICE.
//! Closure conversions preserve receiver effects; captures project on each use.
use super::{
    Error,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{EntityId, MemberCall, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn closure_function_address(
        &mut self,
        invoker: EntityId,
        source: TypeId,
    ) -> Result<RValue, Error> {
        self.symbols.demand(invoker);
        let symbol = self
            .symbols
            .function_symbol(self.builder.program(), invoker)?;
        self.emit(I::Addr(Value::Symbol(symbol)), source)
    }
    pub(super) fn convert_closure(
        &mut self,
        id: ast::ExprId,
        invoker: EntityId,
        call: MemberCall,
    ) -> Result<RValue, Error> {
        let id = self.without_parentheses(id);
        if !matches!(self.tree.expressions.get(id).kind, ExprKind::Lambda(_))
            && !self.side_effect_free(id, false, 0)?
        {
            let place = self.original_scalar_place(id)?;
            let object = self.storage_address(place, self.graph.lower_expression(id)?.ty)?;
            // conversion_object projects the receiver into the converter's
            // owning base, even when the address is unused and the offset is zero.
            if let Some(offset) = call.offset {
                self.base_projection(object, call.result, offset)?;
            }
        }
        self.closure_function_address(invoker, call.result)
    }
    pub(super) fn explicit_closure_conversion(
        &mut self,
        callee: ast::ExprId,
        invoker: EntityId,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let root = self.without_parentheses(callee);
        if let ExprKind::Member { base, op, .. } = self.tree.expressions.get(root).kind
            && !matches!(self.tree.expressions.get(base).kind, ExprKind::Lambda(_))
            && !self.side_effect_free(base, false, 0)?
        {
            if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Arrow) {
                self.arrow_operand(base)?;
            } else {
                self.address(base)?;
            }
        }
        self.closure_function_address(invoker, source)
    }
    pub(super) fn closure_field(
        &mut self,
        object: Value,
        member: EntityId,
        source: TypeId,
    ) -> Result<Value, Error> {
        let offset = self.graph.lower_member_offset(member)?;
        let offset = self.builder.integer(u128::from(offset))?;
        Ok(self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), object, offset),
                source,
            )?
            .value)
    }
    pub(super) fn captured_place(&mut self, entity: EntityId) -> Result<Option<Value>, Error> {
        let Some(&member) = self.captured_members.get(&entity) else {
            return Ok(None);
        };
        let ty = self.graph.lower_entity_type(member)?;
        let object = self.this_pointer(ty)?.value;
        let place = self.closure_field(object, member, ty)?;
        self.reference_holder(place, ty).map(Some)
    }
    fn capture_address(&mut self, entity: EntityId) -> Result<Value, Error> {
        if let Some(place) = self.captured_place(entity)? {
            return Ok(place);
        }
        let ty = self.local_object_type(entity)?;
        let place = self
            .locals
            .get(&entity)
            .copied()
            .ok_or(crate::sema::Error::Invariant(
                "automatic capture without local or enclosing member",
            ))?;
        let place = self.reference_holder(place, ty)?;
        if matches!(place, Value::Slot(_) | Value::Symbol(_)) {
            Ok(self.emit(I::Addr(place), ty)?.value)
        } else {
            Ok(place)
        }
    }
    pub(super) fn closure_object(&mut self, ty: TypeId, object: Value) -> Result<(), Error> {
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("closure object type").into());
        };
        let closure = self
            .graph
            .lower_closure(class)
            .ok_or(crate::sema::Error::Invariant("closure object facts"))?;
        if let Some(member) = closure.this_member {
            let place = if self.graph.lower_member_offset(member)? == 0 {
                object
            } else {
                self.closure_field(object, member, ty)?
            };
            let value = self.object_pointer(ty)?.value;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                value,
                place,
            ))?;
        }
        for capture in self.graph.lower_closure_captures(class) {
            let place = if self.graph.lower_member_offset(capture.member)? == 0 {
                object
            } else {
                self.closure_field(object, capture.member, ty)?
            };
            let value = self.capture_address(capture.entity)?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                value,
                place,
            ))?;
        }
        Ok(())
    }
}
