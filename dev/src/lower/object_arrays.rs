// Translated from lower/object_lowering.cpp::construct_array and
// cleanup_lowering.cpp::construct_elements. See NOTICE.
//! Default array construction tracks the completed prefix at the exact eight-element boundary.
use super::{Error, construction::Variant, function::Function, types};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Binary, Compare, Projection, Type},
    },
    sema::lowering::{ConstructorId, EntityId, TypeId, TypeView},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn default_class_array(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        storage: Value,
    ) -> Result<(), Error> {
        let constructor = self
            .graph
            .lower_construction(entity)
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "array without constructor plan",
            )))?;
        let mut element = ty;
        let mut count = 1_u64;
        while let TypeView::Array {
            element: inner,
            bound,
        } = self.graph.lower_type(element)?
        {
            count = count.wrapping_mul(bound);
            element = inner;
        }
        // Default argument expressions use the same constructor call path.
        // Temporaries with nontrivial destruction retain their materialization boundary.
        let throws = self.default_constructor_throws(constructor)?;
        let destroyed = self.symbols.storage_destructor(element)?.is_some();
        if count <= 8 {
            self.construct_small_array(element, storage, count, constructor, throws && destroyed)
        } else {
            self.construct_array_loop(element, storage, count, constructor, throws, destroyed)
        }
    }
    fn construct_small_array(
        &mut self,
        element: TypeId,
        storage: Value,
        count: u64,
        constructor: ConstructorId,
        guarded: bool,
    ) -> Result<(), Error> {
        let size = types::layout(self.graph, element)?.0;
        for index in 0..count {
            let guard = if index != 0 && guarded {
                let dispatch = self.new_block("call_unwind_dispatch")?;
                let next = self.new_block("call_unwind_end")?;
                self.effect(I::Runtime(R::EhTry(dispatch)))?;
                Some((dispatch, next))
            } else {
                None
            };
            let base = self.emit(I::Addr(storage), element)?.value;
            let object = self.element_address(base, index, size, element)?;
            self.call_constructor(constructor, Variant::Complete, object, None)?;
            if let Some((dispatch, next)) = guard {
                self.effect(I::Runtime(R::EhEnd))?;
                self.jump(next)?;
                self.place(dispatch)?;
                for previous in (0..index).rev() {
                    let base = self.emit(I::Addr(storage), element)?.value;
                    let object = self.element_address(base, previous, size, element)?;
                    self.call_destructor(element, object)?;
                }
                self.effect(I::Runtime(R::Resume))?;
                self.terminated = true;
                self.place(next)?;
            }
        }
        Ok(())
    }
    fn construct_array_loop(
        &mut self,
        element: TypeId,
        storage: Value,
        count: u64,
        constructor: ConstructorId,
        throws: bool,
        destroyed: bool,
    ) -> Result<(), Error> {
        let size = types::layout(self.graph, element)?.0;
        let slot = self.generated_slot("constructor_array_index", Type::I64)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, zero, slot))?;
        self.before_lifetime_call(throws)?;
        let test = self.new_block("constructor_array_cond")?;
        let body = self.new_block("constructor_array_body")?;
        let end = self.new_block("constructor_array_end")?;
        self.jump(test)?;
        self.place(test)?;
        let current = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let bound = self.builder.integer(u128::from(count))?;
        let condition = self
            .emit(I::Compare(Compare::Ult, Type::I64, current, bound), element)?
            .value;
        self.effect(I::Branch(condition, body, end))?;
        self.place(body)?;
        let cleanup = if throws && destroyed {
            let cleanup = self.new_block("constructor_array_cleanup")?;
            self.effect(I::Runtime(R::EhTry(cleanup)))?;
            Some(cleanup)
        } else {
            None
        };
        let object = self.dynamic_element(storage, element, current, size)?;
        self.call_constructor(constructor, Variant::Complete, object, None)?;
        if cleanup.is_some() {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        let one = self.builder.integer(1)?;
        let next = self
            .emit(
                I::Binary(Binary::Add, Facts::default(), Type::I64, current, one),
                element,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, next, slot))?;
        self.jump(test)?;
        if let Some(cleanup) = cleanup {
            let run = self.new_block("constructor_array_cleanup_body")?;
            let resume = self.new_block("constructor_array_resume")?;
            self.place(cleanup)?;
            let made = self
                .emit(
                    I::Load(Facts::memory(false, true), Type::I64, slot),
                    element,
                )?
                .value;
            let condition = self
                .emit(I::Compare(Compare::Ne, Type::I64, made, zero), element)?
                .value;
            self.effect(I::Branch(condition, run, resume))?;
            self.place(run)?;
            let last = self
                .emit(
                    I::Binary(Binary::Sub, Facts::default(), Type::I64, made, one),
                    element,
                )?
                .value;
            self.effect(I::Store(Facts::memory(false, true), Type::I64, last, slot))?;
            let object = self.dynamic_element(storage, element, last, size)?;
            self.call_destructor(element, object)?;
            self.jump(cleanup)?;
            self.place(resume)?;
            self.effect(I::Runtime(R::Resume))?;
            self.terminated = true;
        }
        self.place(end)
    }
    fn dynamic_element(
        &mut self,
        storage: Value,
        element: TypeId,
        index: Value,
        size: u64,
    ) -> Result<Value, Error> {
        let base = self.emit(I::Addr(storage), element)?.value;
        let offset = if size == 1 {
            index
        } else {
            let size = self.builder.integer(u128::from(size))?;
            self.emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                element,
            )?
            .value
        };
        Ok(self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                element,
            )?
            .value)
    }
}
