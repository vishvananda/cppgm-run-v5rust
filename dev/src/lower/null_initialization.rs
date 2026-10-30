// Translated from constant_lowering.cpp::null_member_pointers/null_fill and
// object_lowering.cpp::null_data_member_pointers. See NOTICE.
//! Null images retain ordered subobject ranges and complete/base context.
use super::{Error, function::Function, types};
use crate::support::hash::HashMap;
use crate::{
    lowir::{
        literals::Constant,
        model::{DataItem, Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Compare, Type},
    },
    sema::{
        Graph,
        lowering::{EntityId, ObjectExtent, TypeId, TypeView},
    },
    support::stack::StackBudget,
};
use std::ops::Range;

#[derive(Clone, Copy)]
struct Piece {
    ty: TypeId,
    offset: u64,
    end: u64,
    reach: u64,
    extent: ObjectExtent,
}
#[derive(Default)]
pub(super) struct Layouts {
    spans: HashMap<(EntityId, ObjectExtent), Range<usize>>,
    pieces: Vec<Piece>,
}
impl Layouts {
    fn span(
        &mut self,
        graph: &Graph,
        class: EntityId,
        extent: ObjectExtent,
    ) -> Result<Range<usize>, Error> {
        if let Some(span) = self.spans.get(&(class, extent)) {
            return Ok(span.clone());
        }
        let first = self.pieces.len();
        for child in graph.lower_null_subobjects(class, extent) {
            let child = child?;
            self.pieces.push(Piece {
                ty: child.ty,
                offset: child.offset,
                end: child.offset.wrapping_add(types::layout(graph, child.ty)?.0),
                reach: 0,
                extent: child.extent,
            });
        }
        self.pieces[first..].sort_by_key(|piece| piece.offset);
        let mut reach = 0;
        for piece in &mut self.pieces[first..] {
            reach = reach.max(piece.end);
            piece.reach = reach;
        }
        let span = first..self.pieces.len();
        self.spans.insert((class, extent), span.clone());
        Ok(span)
    }
}
#[derive(Clone, Copy)]
struct FillRange {
    ty: TypeId,
    base: u64,
    lo: u64,
    hi: u64,
    extent: ObjectExtent,
}
struct Fill<'a> {
    graph: &'a Graph,
    layouts: &'a mut Layouts,
    budget: StackBudget,
    stack_base: usize,
}
impl Fill<'_> {
    fn range(&mut self, range: FillRange, out: &mut Vec<DataItem>) -> Result<(), Error> {
        let marker = 0_u8;
        if self
            .budget
            .exceeded(self.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(crate::sema::Error::Limit("constructs nested too deeply").into());
        }
        let FillRange {
            ty,
            base,
            lo,
            hi,
            extent,
        } = range;
        if lo >= hi {
            return Ok(());
        }
        if !self.graph.lower_holds_data_member_pointer(ty)? {
            return append_zero(out, hi - lo);
        }
        if self.graph.lower_member_pointer_type(ty).is_some() {
            if lo != base || hi != base.wrapping_add(8) {
                return append_zero(out, hi - lo);
            }
            out.push(DataItem::Constant(Type::I64, Constant::Integer(u128::MAX)));
            return Ok(());
        }
        if let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            return self.array(range, element, out);
        }
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("null image type").into());
        };
        let span = self.layouts.span(self.graph, class, extent)?;
        let first = self.layouts.pieces[span.clone()]
            .partition_point(|piece| piece.reach <= lo - base)
            + span.start;
        let mut at = lo;
        for index in first..span.end {
            let piece = self.layouts.pieces[index];
            let offset = base.wrapping_add(piece.offset);
            if offset >= hi {
                break;
            }
            let from = offset.max(at);
            let to = base.wrapping_add(piece.end).min(hi);
            if from >= to {
                continue;
            }
            append_zero(out, from - at)?;
            self.range(
                FillRange {
                    ty: piece.ty,
                    base: offset,
                    lo: from,
                    hi: to,
                    extent: piece.extent,
                },
                out,
            )?;
            at = to;
        }
        append_zero(out, hi - at)
    }
    fn array(
        &mut self,
        range: FillRange,
        element: TypeId,
        out: &mut Vec<DataItem>,
    ) -> Result<(), Error> {
        let FillRange { base, lo, hi, .. } = range;
        let size = types::layout(self.graph, element)?.0;
        if size == 0 {
            return Err(crate::sema::Error::Invariant("zero-sized null array element").into());
        }
        let mut index = (lo - base) / size;
        let whole = (hi - base) / size;
        if !(lo - base).is_multiple_of(size) {
            let at = base.wrapping_add(index.wrapping_mul(size));
            self.range(
                FillRange {
                    ty: element,
                    base: at,
                    lo,
                    hi: at.wrapping_add(size).min(hi),
                    extent: ObjectExtent::Complete,
                },
                out,
            )?;
            index += 1;
        }
        if index < whole {
            let at = base.wrapping_add(index.wrapping_mul(size));
            let first = out.len();
            self.range(
                FillRange {
                    ty: element,
                    base: at,
                    lo: at,
                    hi: at.wrapping_add(size),
                    extent: ObjectExtent::Complete,
                },
                out,
            )?;
            let last = out.len();
            // C++ copies precisely the items appended for the first element.
            // Its leading zero may already have merged into the preceding item.
            let copies =
                usize::try_from(whole - index - 1).map_err(|_| crate::lowir::Error::Limit)?;
            let additional = (last - first)
                .checked_mul(copies)
                .ok_or(crate::lowir::Error::Limit)?;
            out.try_reserve(additional)
                .map_err(|_| crate::lowir::Error::Limit)?;
            for _ in 0..copies {
                for item in first..last {
                    out.push(out[item]);
                }
            }
            index = whole;
        }
        let at = base.wrapping_add(index.wrapping_mul(size));
        if at < hi {
            self.range(
                FillRange {
                    ty: element,
                    base: at,
                    lo: at,
                    hi,
                    extent: ObjectExtent::Complete,
                },
                out,
            )?;
        }
        Ok(())
    }
}
fn append_zero(out: &mut Vec<DataItem>, bytes: u64) -> Result<(), Error> {
    if bytes == 0 {
        return Ok(());
    }
    let bytes = i64::try_from(bytes).map_err(|_| crate::lowir::Error::Limit)?;
    if let Some(DataItem::Zero(last)) = out.last_mut() {
        *last = last.checked_add(bytes).ok_or(crate::lowir::Error::Limit)?;
    } else {
        out.push(DataItem::Zero(bytes));
    }
    Ok(())
}
fn item_layout(item: DataItem) -> (u64, u64) {
    match item {
        DataItem::Zero(bytes) => (bytes.cast_unsigned(), 1),
        DataItem::Address(..) | DataItem::Constant(Type::Ptr, _) => (8, 8),
        DataItem::Constant(Type::Object { bytes, align }, _) => {
            let bytes = bytes.get();
            (bytes, 1_u64 << align)
        }
        DataItem::Constant(ty, _) => {
            let size = if ty.bits() != 0 {
                u64::from(ty.bits().max(8) / 8)
            } else {
                match ty.float_rank() {
                    1 => 4,
                    2 => 8,
                    _ => 16,
                }
            };
            (size, size)
        }
    }
}
pub(super) fn static_nulls(
    layouts: &mut Layouts,
    graph: &Graph,
    ty: TypeId,
    items: &mut Vec<DataItem>,
    budget: StackBudget,
) -> Result<(), Error> {
    if !graph.lower_holds_data_member_pointer(ty)?
        || !items.iter().any(|item| matches!(item, DataItem::Zero(_)))
    {
        return Ok(());
    }
    let marker = 0_u8;
    let mut fill = Fill {
        graph,
        layouts,
        budget,
        stack_base: std::ptr::from_ref(&marker).addr(),
    };
    let mut result = Vec::with_capacity(items.len());
    let mut at = 0_u64;
    for item in items.drain(..) {
        let (size, align) = item_layout(item);
        if matches!(item, DataItem::Zero(_)) {
            fill.range(
                FillRange {
                    ty,
                    base: 0,
                    lo: at,
                    hi: at.wrapping_add(size),
                    extent: ObjectExtent::Complete,
                },
                &mut result,
            )?;
        } else {
            at = at.wrapping_add(align - 1) & !(align - 1);
            result.push(item);
        }
        at = at.wrapping_add(size);
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
        extent: ObjectExtent,
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
                let stride = types::layout(&*self.graph, element)?.0;
                if bound <= 4 {
                    for index in 0..bound {
                        self.null_data_members(
                            element,
                            object,
                            at.wrapping_add(index.wrapping_mul(stride)),
                            ObjectExtent::Complete,
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
                let graph = &*self.graph;
                for child in graph
                    .lower_null_subobjects(class, extent)
                    .collect::<Vec<_>>()
                {
                    let child = child?;
                    self.null_data_members(
                        child.ty,
                        object,
                        at.wrapping_add(child.offset),
                        child.extent,
                    )?;
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
        let stride = types::layout(&*self.graph, ty)?.0;
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
        self.null_data_members(ty, address, 0, ObjectExtent::Complete)?;
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
