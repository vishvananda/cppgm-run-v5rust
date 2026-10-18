// Translated from lower/{function,expression,object}_lowering.cpp. See NOTICE.
//! Captures project through the current closure on each use, never a cached load.
use super::{Error, function::Function};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{EntityId, TypeId, TypeView},
};

impl Function<'_, '_, '_, '_> {
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
