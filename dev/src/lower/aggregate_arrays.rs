// Translated from lower/aggregate_lowering.cpp::push_counted/element_made
// and cleanup_lowering.cpp's counted construction cleanup. See NOTICE.
//! Aggregate member arrays retain one completed-prefix counter per active construction.
use super::{Error, function::Function, lifetime::CleanupPlace, types};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Compare, Projection, Type},
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast,
};

pub(super) struct Count {
    slot: Value,
    made: u64,
    total: u64,
    pub rows: bool,
    row_slot: Option<Value>,
}
#[derive(Clone, Copy)]
pub(super) struct Scope {
    first: usize,
    floor: Option<usize>,
    ty: TypeId,
    root: Value,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn counts_aggregate_elements(
        &mut self,
        ty: TypeId,
        values: &[ast::ExprId],
    ) -> Result<bool, Error> {
        if self.static_initializer || self.persistent_references || self.aggregation.count.is_some()
        {
            return Ok(false);
        }
        let mut element = ty;
        while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
            element = inner;
        }
        if self.symbols.class_destructor(element)?.is_none() {
            return Ok(false);
        }
        for &id in values {
            if self.expression_throws(id)? {
                return Ok(true);
            }
        }
        let TypeView::Class(class) = self.graph.lower_type(element)? else {
            return Ok(false);
        };
        self.omitted_aggregate_throws(class, 0)
    }
    pub(super) fn begin_counted_elements(
        &mut self,
        ty: TypeId,
        root: Value,
    ) -> Result<Scope, Error> {
        self.begin_counted_using(ty, root, None)
    }
    fn begin_counted_using(
        &mut self,
        ty: TypeId,
        root: Value,
        reusable: Option<Value>,
    ) -> Result<Scope, Error> {
        let scope = Scope {
            first: self.lifetime.cleanups.len(),
            floor: self.aggregation.floor,
            ty,
            root,
        };
        let mut element = ty;
        let mut total = 1_u64;
        while let TypeView::Array {
            element: inner,
            bound,
        } = self.graph.lower_type(element)?
        {
            total = total.wrapping_mul(bound);
            element = inner;
        }
        let slot = if let Some(slot) = reusable {
            slot
        } else {
            self.generated_slot("constructor_array_index", Type::I64)?
        };
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, zero, slot))?;
        self.register_aggregate_member(ty, root);
        let cleanup = self
            .lifetime
            .cleanups
            .last_mut()
            .ok_or(crate::sema::Error::Invariant(
                "counted array without cleanup",
            ))?;
        cleanup.place = CleanupPlace::Counted {
            root,
            counter: slot,
        };
        self.aggregation.floor = None;
        self.aggregation.count = Some(Count {
            slot,
            made: 0,
            total,
            rows: false,
            row_slot: None,
        });
        // The counted prefix now owns construction cleanup. Its first call
        // opens after the array address and zero prefix have been established.
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        if self.lifetime.region {
            self.close_lifetime_region()?;
            self.before_lifetime_call(true)?;
        }
        Ok(scope)
    }
    pub(super) fn begin_counted_row(
        &mut self,
        ty: TypeId,
        root: Value,
    ) -> Result<(Count, Scope), Error> {
        let mut parent = self
            .aggregation
            .count
            .take()
            .ok_or(crate::sema::Error::Invariant("counted row without parent"))?;
        let scope = self.begin_counted_using(ty, root, parent.row_slot)?;
        let row = self
            .aggregation
            .count
            .as_mut()
            .ok_or(crate::sema::Error::Invariant("counted row without counter"))?;
        parent.row_slot = Some(row.slot);
        row.rows = matches!(self.graph.lower_type(ty)?, TypeView::Array { element, .. }
            if matches!(self.graph.lower_type(element)?, TypeView::Array { .. }));
        Ok((parent, scope))
    }
    pub(super) fn end_counted_row(&mut self, mut parent: Count, scope: Scope) -> Result<(), Error> {
        let row = self
            .aggregation
            .count
            .take()
            .ok_or(crate::sema::Error::Invariant("counted row without counter"))?;
        self.end_counted_elements(scope)?;
        parent.made = parent.made.wrapping_add(row.total);
        if parent.made < parent.total {
            let made = self.builder.integer(u128::from(parent.made))?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                made,
                parent.slot,
            ))?;
        }
        self.aggregation.count = Some(parent);
        Ok(())
    }
    pub(super) fn aggregate_element_made(
        &mut self,
        mut count: Count,
        first: usize,
    ) -> Result<Count, Error> {
        if self.lifetime.cleanups[first..]
            .iter()
            .any(|cleanup| cleanup.member())
        {
            self.close_lifetime_region()?;
            self.release_aggregate_members(first);
        }
        count.made = count.made.wrapping_add(1);
        if count.made < count.total {
            let made = self.builder.integer(u128::from(count.made))?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                made,
                count.slot,
            ))?;
        }
        Ok(count)
    }
    pub(super) fn end_counted_elements(&mut self, scope: Scope) -> Result<(), Error> {
        self.close_lifetime_region()?;
        self.aggregation.count = None;
        self.release_aggregate_members(scope.first);
        self.aggregation.floor = scope.floor;
        if scope.floor.is_some() {
            self.register_aggregate_member(scope.ty, scope.root);
        }
        Ok(())
    }
    pub(super) fn destroy_counted_elements(
        &mut self,
        ty: TypeId,
        root: Value,
        counter: Value,
    ) -> Result<(), Error> {
        let mut element = ty;
        while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
            element = inner;
        }
        let condition = self.new_block("construction_array_cleanup")?;
        let body = self.new_block("construction_array_cleanup_body")?;
        let end = self.new_block("construction_array_cleanup_end")?;
        self.jump(condition)?;
        self.place(condition)?;
        let count = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, counter),
                element,
            )?
            .value;
        let zero = self.builder.integer(0)?;
        let test = self
            .emit(I::Compare(Compare::Ne, Type::I64, count, zero), element)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.place(body)?;
        let one = self.builder.integer(1)?;
        let last = self
            .emit(
                I::Binary(Binary::Sub, Facts::default(), Type::I64, count, one),
                element,
            )?
            .value;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::I64,
            last,
            counter,
        ))?;
        let size = types::layout(self.graph, element)?.0;
        let offset = if size == 1 {
            last
        } else {
            let size = self.builder.integer(u128::from(size))?;
            self.emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, last, size),
                element,
            )?
            .value
        };
        let base = self.storage_address(root, ty)?;
        let object = self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                element,
            )?
            .value;
        self.call_destructor(element, object)?;
        self.jump(condition)?;
        self.place(end)
    }
}
