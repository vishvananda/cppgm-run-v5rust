// Translated from lower/dispatch_lowering.cpp's primary-chain path. See NOTICE.
//! Constructor vpointer writes and dispatch consume completed semantic slots.
use super::{Error, function::Function};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, SignatureId, Value},
        vocabulary::Type,
    },
    sema::lowering::{EntityId, SlotId, TypeId},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn store_vptr(&mut self, class: EntityId, source: TypeId) -> Result<(), Error> {
        if !self.graph.lower_polymorphic(class) {
            return Ok(());
        }
        self.symbols.vtable_symbol(self.builder.program(), class)?;
        let offsets = self.graph.lower_segment_offsets(class)?;
        for (index, offset) in offsets.into_iter().enumerate() {
            if self
                .graph
                .lower_vtable_group(class)
                .is_some_and(|group| group.segments[index].shared)
            {
                continue;
            }
            let mut object = self.object_pointer(source)?.value;
            if index != 0 {
                let offset = self.builder.integer(u128::from(offset))?;
                object = self
                    .emit(I::Index(Type::I8, None, object, offset), source)?
                    .value;
            }
            let (symbol, point) =
                self.symbols
                    .table_address(self.builder.program(), class, index)?;
            let table = self.emit(I::Addr(Value::Symbol(symbol)), source)?.value;
            let offset = self.builder.integer(u128::from(point))?;
            let table = self
                .emit(I::Index(Type::I8, None, table, offset), source)?
                .value;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                table,
                object,
            ))?;
        }
        Ok(())
    }
    pub(super) fn virtual_callee(
        &mut self,
        object: Value,
        slot: SlotId,
        source: TypeId,
    ) -> Result<Value, Error> {
        let mut entry = self
            .emit(
                I::Load(Facts::memory(false, true), Type::Ptr, object),
                source,
            )?
            .value;
        if slot.byte_offset() != 0 {
            let offset = self.builder.integer(u128::from(slot.byte_offset()))?;
            entry = self
                .emit(I::Index(Type::I8, None, entry, offset), source)?
                .value;
        }
        Ok(self
            .emit(
                I::Load(Facts::memory(false, true), Type::Ptr, entry),
                source,
            )?
            .value)
    }
    pub(super) fn member_callee(
        &mut self,
        entity: EntityId,
        object: Value,
        dispatch: bool,
        source: TypeId,
    ) -> Result<(Value, Option<SignatureId>), Error> {
        if !dispatch {
            self.symbols.demand(entity);
            return Ok((self.symbols.callee(entity, &mut self.builder)?, None));
        }
        if !self.graph.lower_virtual_pure(entity)
            && (self.graph.lower_function_defined(entity)
                || self.graph.lower_has_synthesized_body(entity))
        {
            self.symbols.demand(entity);
        }
        let slot = self
            .graph
            .lower_virtual_slot(entity)
            .ok_or(crate::sema::Error::Invariant("virtual call without slot"))?;
        let signature = self
            .symbols
            .virtual_signature(self.builder.program(), entity)?;
        Ok((self.virtual_callee(object, slot, source)?, Some(signature)))
    }
}
