// Translated from lower/aggregate_lowering.cpp::copy_object. See NOTICE.
//! Storage copies preserve reusable class tail padding; object values transfer whole.
use super::{Error, function::Function, types};
use crate::{
    lowir::model::{InstructionKind as I, Value},
    sema::lowering::{TypeId, TypeView},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn copy_object(
        &mut self,
        ty: TypeId,
        source: Value,
        destination: Value,
    ) -> Result<(), Error> {
        let (mut bytes, align) = types::layout(self.graph, ty)?;
        if let TypeView::Class(class) = self.graph.lower_type(ty)? {
            if self.empty_storage == super::member_initialization::EmptyStorage::Overlapping
                && self.graph.lower_class_empty(class)?
            {
                return Ok(());
            }
            let data = self.graph.lower_nonvirtual_bytes(class)?;
            if data != 0 && !self.builder.is_object_value(source) {
                bytes = bytes.min(data);
            }
        }
        self.effect(I::CopyObject(bytes, align, source, destination))
    }
}
