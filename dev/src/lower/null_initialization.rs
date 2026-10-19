// Translated from constant_lowering.cpp::null_member_pointers/null_fill and
// object_lowering.cpp::null_data_member_pointers. See NOTICE.
//! Zero bytes retain their spelling except for complete data-member null slots.
use super::{Error, function::Function, types};
use crate::{
    lowir::{
        literals::Constant,
        model::{DataItem, Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Compare, Type},
    },
    sema::{
        Graph,
        lowering::{TypeId, TypeView},
    },
};

pub(super) fn static_nulls(
    graph: &Graph,
    ty: TypeId,
    items: &mut Vec<DataItem>,
) -> Result<(), Error> {
    if !graph.lower_holds_data_member_pointer(ty)?
        || !items.iter().any(|item| matches!(item, DataItem::Zero(_)))
    {
        return Ok(());
    }
    let mut offsets = Vec::new();
    let mut work = vec![(ty, 0_u64)];
    while let Some((ty, at)) = work.pop() {
        if !graph.lower_holds_data_member_pointer(ty)? {
            continue;
        }
        if graph.lower_member_pointer_type(ty).is_some() {
            offsets.push(at);
        } else {
            match graph.lower_type(ty)? {
                TypeView::Array { element, bound } => {
                    let stride = types::layout(graph, element)?.0;
                    for index in (0..bound).rev() {
                        work.push((element, at.wrapping_add(index.wrapping_mul(stride))));
                    }
                }
                TypeView::Class(class) => {
                    for child in graph.lower_null_subobjects(class) {
                        let (ty, offset) = child?;
                        work.push((ty, at.wrapping_add(offset)));
                    }
                }
                _ => return Err(crate::sema::Error::Invariant("null image type").into()),
            }
        }
    }
    offsets.sort_unstable();
    let mut result = Vec::with_capacity(items.len());
    let mut at = 0_u64;
    let mut next = 0;
    for item in items.drain(..) {
        if let DataItem::Zero(bytes) = item {
            let end = at
                .checked_add(u64::try_from(bytes).map_err(|_| crate::lowir::Error::Limit)?)
                .ok_or(crate::lowir::Error::Limit)?;
            while next < offsets.len() && offsets[next] < at {
                next += 1;
            }
            while next < offsets.len()
                && offsets[next]
                    .checked_add(8)
                    .is_some_and(|end_slot| end_slot <= end)
            {
                let offset = offsets[next];
                if offset > at {
                    result.push(DataItem::Zero(
                        i64::try_from(offset - at).map_err(|_| crate::lowir::Error::Limit)?,
                    ));
                }
                result.push(DataItem::Constant(Type::I64, Constant::Integer(u128::MAX)));
                at = offset + 8;
                next += 1;
            }
            if end > at {
                result.push(DataItem::Zero(
                    i64::try_from(end - at).map_err(|_| crate::lowir::Error::Limit)?,
                ));
            }
            at = end;
        } else {
            let (size, align) = match &item {
                DataItem::Constant(ty, _) | DataItem::Address(ty, ..) => {
                    let size = match ty {
                        Type::Object { bytes, .. } => *bytes,
                        Type::F80 | Type::I128 => 16,
                        Type::I64 | Type::Ptr | Type::F64 => 8,
                        Type::I32 | Type::U32 | Type::F32 => 4,
                        Type::I16 | Type::U16 => 2,
                        Type::I8 | Type::U8 => 1,
                        _ => {
                            return Err(
                                crate::sema::Error::Invariant("null image item type").into()
                            );
                        }
                    };
                    (size, 1_u64 << types::storage_alignment(*ty))
                }
                DataItem::Zero(_) => {
                    return Err(crate::sema::Error::Invariant("null image zero branch").into());
                }
            };
            at = at.wrapping_add(align - 1) / align * align;
            at = at.wrapping_add(size);
            result.push(item);
        }
    }
    *items = result;
    Ok(())
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn null_data_members(
        &mut self,
        ty: TypeId,
        object: Value,
        at: u64,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if !self.graph.lower_holds_data_member_pointer(ty)? {
            return Ok(());
        }
        if self.graph.lower_member_pointer_type(ty).is_some() {
            let place = self.null_member_address(ty, object, at)?;
            let null = self.builder.integer(u128::MAX)?;
            return self.effect(I::Store(Facts::memory(false, true), Type::I64, null, place));
        }
        match self.graph.lower_type(ty)? {
            TypeView::Array { element, bound } => {
                let stride = types::layout(self.graph, element)?.0;
                if bound <= 4 {
                    for index in 0..bound {
                        self.null_data_members(
                            element,
                            object,
                            at.wrapping_add(index.wrapping_mul(stride)),
                        )?;
                    }
                    Ok(())
                } else {
                    let address = self.null_member_address(ty, object, at)?;
                    let count = self.builder.integer(u128::from(bound))?;
                    self.null_member_elements(element, address, count)
                }
            }
            TypeView::Class(class) => {
                let graph = self.graph;
                for child in graph.lower_null_subobjects(class) {
                    let (child, offset) = child?;
                    self.null_data_members(child, object, at.wrapping_add(offset))?;
                }
                Ok(())
            }
            _ => Err(crate::sema::Error::Invariant("null initialization type").into()),
        }
    }
    fn null_member_address(&mut self, ty: TypeId, object: Value, at: u64) -> Result<Value, Error> {
        if at == 0 {
            return Ok(object);
        }
        let at = self.builder.integer(u128::from(at))?;
        Ok(self.emit(I::Index(Type::I8, None, object, at), ty)?.value)
    }
    pub(super) fn null_member_elements(
        &mut self,
        ty: TypeId,
        elements: Value,
        count: Value,
    ) -> Result<(), Error> {
        let stride = types::layout(self.graph, ty)?.0;
        let slot = self.generated_slot("null_member_index", Type::I64)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, zero, slot))?;
        let condition = self.new_block("null_member_cond")?;
        let body = self.new_block("null_member_body")?;
        let end = self.new_block("null_member_end")?;
        self.jump(condition)?;
        self.place(condition)?;
        let index = self
            .emit(I::Load(Facts::memory(false, true), Type::I64, slot), ty)?
            .value;
        let test = self
            .emit(I::Compare(Compare::Ult, Type::I64, index, count), ty)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.place(body)?;
        let size = self.builder.integer(u128::from(stride))?;
        let offset = self
            .emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                ty,
            )?
            .value;
        let address = self
            .emit(I::Index(Type::I8, None, elements, offset), ty)?
            .value;
        self.null_data_members(ty, address, 0)?;
        let one = self.builder.integer(1)?;
        let next = self
            .emit(
                I::Binary(Binary::Add, Facts::default(), Type::I64, index, one),
                ty,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, next, slot))?;
        self.jump(condition)?;
        self.place(end)
    }
}
