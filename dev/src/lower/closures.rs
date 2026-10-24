// Translated from lower/{function,expression,object}_lowering.cpp. See NOTICE.
//! Closure conversions preserve receiver effects; captures project on each use.
use super::{
    Error,
    function::{Function, RValue},
    types,
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
    fn capture_place(
        &mut self,
        entity: EntityId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Value, Error> {
        if let Some(place) = self.captured_place(entity)? {
            return Ok(place);
        }
        let ty = self.local_object_type(entity)?;
        let place = self.locals.get(&entity).copied().ok_or_else(|| {
            crate::sema::Error::at(
                location,
                if self.graph.lower_is_variable(entity) {
                    "a variable outside the procedural lowering slice"
                } else {
                    "a name outside the procedural lowering slice"
                },
            )
        })?;
        self.reference_holder(place, ty)
    }
    fn capture_address(
        &mut self,
        entity: EntityId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Value, Error> {
        let place = self.capture_place(entity, location)?;
        self.storage_address(place, self.local_object_type(entity)?)
    }
    pub(super) fn closure_object(
        &mut self,
        ty: TypeId,
        object: Value,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        // A declared closure's copy defaults can branch without making a
        // full-expression segment. Every path still passes this construction
        // entry; open_region may insert its first guard back at that entry.
        let saved = self.lifetime.construction_start;
        if self.lifetime.construction && !self.lifetime.full_expression {
            self.lifetime.construction_start =
                Some((self.builder.position()?, self.lifetime.cleanups.len()));
        }
        let result = self.initialize_closure(ty, object, location);
        self.lifetime.construction_start = saved;
        result
    }
    fn initialize_closure(
        &mut self,
        ty: TypeId,
        object: Value,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
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
        let captures = self.graph.lower_closure_captures(class);
        let mut last_throwing = None;
        for (index, capture) in captures.iter().enumerate() {
            if self.capture_copy_throws(
                capture.constructor,
                self.graph.lower_entity_type(capture.member)?,
            )? {
                last_throwing = Some(index);
            }
        }
        let first = self.lifetime.cleanups.len();
        let mut tracked = false;
        let mut pending = false;
        for (index, capture) in captures.iter().enumerate() {
            // Project this member before opening the region for its copy.
            let place = if self.graph.lower_member_offset(capture.member)? == 0 {
                object
            } else {
                self.closure_field(object, capture.member, ty)?
            };
            let member_type = self.graph.lower_entity_type(capture.member)?;
            if pending && self.capture_copy_throws(capture.constructor, member_type)? {
                self.closure_member_region()?;
                pending = false;
            }
            if matches!(
                self.graph.lower_type(member_type)?,
                TypeView::Reference { .. }
            ) {
                let value = self.capture_address(capture.entity, location)?;
                self.effect(I::Store(
                    Facts::memory(false, true),
                    Type::Ptr,
                    value,
                    place,
                ))?;
            } else if let Some(constructor) = capture.constructor {
                let source = self.capture_address(capture.entity, location)?;
                if matches!(self.graph.lower_type(member_type)?, TypeView::Array { .. }) {
                    if self.graph.lower_constructor_copies_storage(constructor) {
                        let (size, align) = types::layout(self.graph, member_type)?;
                        self.effect(I::CopyObject(size, align, source, place))?;
                    } else {
                        self.copy_captured_elements(
                            member_type,
                            constructor,
                            place,
                            source,
                            location,
                        )?;
                    }
                } else {
                    self.construct_from(constructor, place, source, location)?;
                }
            } else if matches!(self.graph.lower_type(member_type)?, TypeView::Array { .. }) {
                let source = self.capture_address(capture.entity, location)?;
                let (size, align) = types::layout(self.graph, member_type)?;
                self.effect(I::CopyObject(size, align, source, place))?;
            } else {
                let source = self.capture_place(capture.entity, location)?;
                let storage = types::storage(self.graph, member_type)?;
                let value = self
                    .emit(
                        I::Load(self.place_memory(source, member_type)?, storage, source),
                        member_type,
                    )?
                    .value;
                self.effect(I::Store(
                    Facts::memory(false, storage.bits() != 8),
                    storage,
                    value,
                    place,
                ))?;
            }
            if !self.static_initializer
                && last_throwing.is_some_and(|last| index < last)
                && self.symbols.class_destructor(member_type)?.is_some()
            {
                self.register_aggregate_member(member_type, place);
                tracked = true;
                pending = true;
            }
        }
        if tracked {
            self.close_lifetime_region()?;
            self.release_aggregate_members(first);
        }
        Ok(())
    }
    fn capture_copy_throws(
        &mut self,
        constructor: Option<crate::sema::lowering::ConstructorId>,
        ty: TypeId,
    ) -> Result<bool, Error> {
        let Some(id) = constructor else {
            return Ok(false);
        };
        let storage = self.graph.lower_constructor_copies_storage(id);
        let array = matches!(self.graph.lower_type(ty)?, TypeView::Array { .. });
        let called = !array
            && self.graph.lower_constructor_trivial(id)
            && self
                .graph
                .lower_class_empty(self.graph.lower_constructor(id).class)?
            && self.symbols.class_destructor(ty)?.is_some();
        Ok(!(storage && (array || !called)) && !self.graph.lower_constructor_function(id)?.nothrow)
    }
    fn closure_member_region(&mut self) -> Result<(), Error> {
        self.close_lifetime_region()?;
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        let construction = std::mem::replace(&mut self.lifetime.construction, false);
        let position = self.builder.position()?;
        let result = self.open_temporary_region_at(Some(position));
        self.lifetime.construction = construction;
        result
    }
}
