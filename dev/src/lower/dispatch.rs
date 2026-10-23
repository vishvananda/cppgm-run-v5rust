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
    pub(super) fn demand_virtual_call_table(&mut self) -> Result<(), Error> {
        if let Some(class) = self.call_object_class
            && !self.graph.lower_virtual_bases(class).is_empty()
            && !self.symbols.host_objects()
        {
            self.symbols
                .construction_table(self.builder.program(), class)?;
        }
        Ok(())
    }

    pub(super) fn store_vptr(&mut self, class: EntityId, source: TypeId) -> Result<(), Error> {
        if !self.graph.lower_dynamic_class(class) {
            return Ok(());
        }
        self.symbols.vtable_symbol(self.builder.program(), class)?;
        let offsets = self.graph.lower_segment_offsets(class)?;
        let order = if self.virtual_context.vtt.is_some() {
            self.symbols.vtt_pointer_order(class)?
        } else {
            Vec::new()
        };
        let mut from_vtt = vec![false; offsets.len()];
        for &index in &order {
            from_vtt[index] = true;
        }
        let object = self.table_subobject(class, source, 0, &offsets)?;
        if self.virtual_context.vtt.is_some() {
            let table = self.construction_table_value(source)?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                table,
                object,
            ))?;
        } else {
            self.store_table_at(class, source, 0, object)?;
        }
        for (index, &from_vtt) in from_vtt.iter().enumerate().skip(1) {
            if self
                .graph
                .lower_vtable_group(class)
                .is_some_and(|group| group.segments[index].shared)
                || from_vtt
            {
                continue;
            }
            let object = self.table_subobject(class, source, index, &offsets)?;
            self.store_table_at(class, source, index, object)?;
        }
        for index in order {
            let object = self.table_subobject(class, source, index, &offsets)?;
            let slot = self.symbols.vtt_segment_index(class, index)?;
            let offset = u64::try_from(slot)
                .map_err(|_| crate::lowir::Error::Limit)?
                .wrapping_mul(8);
            let offset = self.builder.integer(u128::from(offset))?;
            let vtt = self
                .virtual_context
                .vtt
                .ok_or(crate::sema::Error::Invariant(
                    "secondary pointer without VTT",
                ))?;
            let entry = self
                .emit(I::Index(Type::I8, None, vtt, offset), source)?
                .value;
            let table = self
                .emit(
                    I::Load(Facts::memory(false, true), Type::Ptr, entry),
                    source,
                )?
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
    fn store_table_at(
        &mut self,
        class: EntityId,
        source: TypeId,
        index: usize,
        object: Value,
    ) -> Result<(), Error> {
        let (symbol, point) = self
            .symbols
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
        ))
    }
    fn table_subobject(
        &mut self,
        class: EntityId,
        source: TypeId,
        index: usize,
        offsets: &[u64],
    ) -> Result<Value, Error> {
        let within = if index == 0 {
            None
        } else {
            self.graph
                .lower_vtable_group(class)
                .and_then(|g| g.segments[index].within)
        };
        if let Some(within) = within {
            let within = within.index();
            let holder = self
                .graph
                .lower_vtable_group(class)
                .ok_or(crate::sema::Error::Invariant(
                    "secondary table without group",
                ))?
                .segments[within]
                .class;
            let object = self.object_pointer(source)?.value;
            let address = self.virtual_base_address(object, source, class, holder)?;
            if self.virtual_context.vtt.is_some() && offsets[index] == offsets[within] {
                return Ok(address);
            }
            let offset = self
                .builder
                .integer(u128::from(offsets[index].wrapping_sub(offsets[within])))?;
            return Ok(self
                .emit(I::Index(Type::I8, None, address, offset), source)?
                .value);
        }
        let object = self.object_pointer(source)?.value;
        if index == 0 {
            return Ok(object);
        }
        let offset = self.builder.integer(u128::from(offsets[index]))?;
        Ok(self
            .emit(I::Index(Type::I8, None, object, offset), source)?
            .value)
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
            return Ok((
                self.symbols.direct_callee(self.builder.program(), entity)?,
                None,
            ));
        }
        if !self.graph.lower_virtual_pure(entity)
            && !self.symbols.host_objects()
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
