// Translated from lower/allocation_lowering.cpp::deletion. See NOTICE.
//! Delete[] recovers the allocation header and destroys the flattened tail first.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    lifetime::CleanupPlace,
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Binary, Compare, Type},
    },
    sema::lowering::{CallTarget, DeletionView, TypeId, TypeView},
    syntax::ast,
};
#[derive(Clone, Copy)]
pub(super) enum Destruction {
    Deleted,
    Unwinding,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn delete_array(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        use_: &DeletionView,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let mut element = use_.target;
        while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
            element = inner;
        }
        let class =
            !use_.incomplete && matches!(self.graph.lower_type(element)?, TypeView::Class(_));
        let object = self.operand(operand, Context::Operand)?;
        let cookie = if class {
            self.array_cookie_bytes(element, use_.sized_array_delete, false)?
        } else {
            0
        };
        let end = if cookie != 0 {
            let nonnull = self.new_block("array_delete_nonnull")?;
            let end = self.new_block("array_delete_end")?;
            let zero = self.builder.integer(0)?;
            let test = self
                .emit(
                    I::Compare(Compare::Ne, Type::Ptr, object.value, zero),
                    source,
                )?
                .value;
            self.effect(I::Branch(test, nonnull, end))?;
            self.place(nonnull)?;
            Some(end)
        } else {
            None
        };
        let size = if use_.incomplete {
            0
        } else {
            types::layout(self.graph, element)?.0
        };
        let ty = self.graph.lower_function(use_.function)?.ty;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("array delete signature").into());
        };
        let sized = parameters.len() == 2;
        let mut storage = object.value;
        let mut block = self.builder.integer(u128::from(size))?;
        let mut freed = None;
        if cookie != 0 {
            let offset = self
                .builder
                .integer(0_u128.wrapping_sub(u128::from(cookie)))?;
            storage = self
                .emit(I::Index(Type::I8, None, object.value, offset), source)?
                .value;
            let header = if cookie == 8 {
                storage
            } else {
                let offset = self.builder.integer(0_u128.wrapping_sub(8))?;
                self.emit(I::Index(Type::I8, None, object.value, offset), source)?
                    .value
            };
            let count = self
                .emit(
                    I::Load(Facts::memory(false, true), Type::I64, header),
                    source,
                )?
                .value;
            if sized {
                let size = self.builder.integer(u128::from(size))?;
                let bytes = self
                    .emit(
                        I::Binary(Binary::Mul, Facts::default(), Type::I64, count, size),
                        source,
                    )?
                    .value;
                let cookie = self.builder.integer(u128::from(cookie))?;
                block = self
                    .emit(
                        I::Binary(Binary::Add, Facts::default(), Type::I64, bytes, cookie),
                        source,
                    )?
                    .value;
            }
            if self.symbols.class_destructor(element)?.is_some() {
                let slot = self.generated_slot("array_delete_index", Type::I64)?;
                self.effect(I::Store(Facts::memory(false, true), Type::I64, count, slot))?;
                if self.destructor_throws(element)? {
                    let first = self.lifetime.cleanups.len();
                    let values = [storage, block];
                    let args = self
                        .builder
                        .arguments(&values[..if sized { 2 } else { 1 }])?;
                    self.register_storage_cleanup(element, storage, use_.function, Some(args))?;
                    self.register_aggregate_member(element, object.value);
                    let last = self
                        .lifetime
                        .cleanups
                        .last_mut()
                        .ok_or(crate::sema::Error::Invariant("deleted prefix cleanup"))?;
                    last.place = CleanupPlace::Counted {
                        root: object.value,
                        counter: slot,
                    };
                    self.close_lifetime_region()?;
                    freed = Some(first);
                }
                self.destroy_allocated_elements(element, object.value, slot, Destruction::Deleted)?;
            }
        }
        if let Some(first) = freed {
            self.close_lifetime_region()?;
            self.release_aggregate_members(first);
        }
        self.deallocate_storage(use_.function, storage, block, None, true)?;
        if let Some(end) = end {
            self.jump(end)?;
            self.place(end)?;
        }
        self.zero(source)
    }
    pub(super) fn destroy_allocated_elements(
        &mut self,
        element: TypeId,
        elements: Value,
        slot: Value,
        mode: Destruction,
    ) -> Result<(), Error> {
        let (condition, body, end) = match mode {
            Destruction::Deleted => (
                "array_delete_dtor_cond",
                "array_delete_dtor_body",
                "array_delete_dtor_end",
            ),
            Destruction::Unwinding => ("array_dtor_cond", "array_dtor_body", "array_dtor_end"),
        };
        let condition = self.new_block(condition)?;
        let body = self.new_block(body)?;
        let end = self.new_block(end)?;
        self.jump(condition)?;
        self.place(condition)?;
        let left = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let zero = self.builder.integer(0)?;
        let test = self
            .emit(I::Compare(Compare::Ne, Type::I64, left, zero), element)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.place(body)?;
        let one = self.builder.integer(1)?;
        let previous = self
            .emit(
                I::Binary(Binary::Sub, Facts::default(), Type::I64, left, one),
                element,
            )?
            .value;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::I64,
            previous,
            slot,
        ))?;
        let object = self.allocated_element_address(element, elements, previous)?;
        match mode {
            Destruction::Deleted => self.destroy_deleted_element(element, object)?,
            Destruction::Unwinding => {
                let throws = self.destructor_throws(element)?;
                if throws {
                    self.open_destructor_unwind_guard()?;
                }
                self.call_destructor(element, object)?;
                if throws {
                    self.effect(I::Runtime(R::EhEnd))?;
                }
            }
        }
        self.jump(condition)?;
        self.place(end)
    }
    fn destroy_deleted_element(&mut self, element: TypeId, object: Value) -> Result<(), Error> {
        let destructor = self
            .symbols
            .class_destructor(element)?
            .ok_or(crate::sema::Error::Invariant("array element destructor"))?;
        let throws = self.lifetime_call_throws(CallTarget::Direct(destructor))?;
        self.begin_lifetime_call(throws)?;
        self.before_lifetime_call(throws)?;
        self.call_destructor(element, object)?;
        let none = self.zero(element)?;
        self.finish_lifetime_call(none, throws)?;
        Ok(())
    }
}
