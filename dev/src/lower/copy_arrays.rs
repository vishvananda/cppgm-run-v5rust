// Translated from lower/copy_lowering.cpp::copy_elements/loop. See NOTICE.
//! Small element runs join the construction chain; large runs retain one prefix counter.
use super::{Error, construction::Variant, function::Function, types};
use crate::{
    lowir::{
        model::{BlockId, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Binary, Compare, Projection, Type},
    },
    sema::lowering::{ConstructorId, EntityId, SubobjectCopy, TypeId, TypeView},
};
#[derive(Clone, Copy)]
struct Elements {
    element: TypeId,
    count: u64,
    destination: Value,
    source: Value,
    constructor: ConstructorId,
    guarded: bool,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn copy_elements(
        &mut self,
        function: EntityId,
        action: SubobjectCopy,
        transfer: super::bit_fields::Transfer,
        destination: Value,
    ) -> Result<(), Error> {
        let mut element = action.ty;
        let mut count = 1u64;
        while let TypeView::Array {
            element: child,
            bound,
        } = self.graph.lower_type(element)?
        {
            count = count.wrapping_mul(bound);
            element = child;
        }
        let constructor = action
            .constructor
            .ok_or(crate::sema::Error::Invariant("array copy constructor"))?;
        let guarded = self.symbols.class_destructor(element)?.is_some()
            && self.default_constructor_throws(constructor)?;
        let size = types::layout(self.graph, element)?.0;
        if count > 8 {
            let source = self.assignment_source(function)?;
            let source = self.copy_address(source, action.ty, transfer)?;
            return self.copy_elements_loop(Elements {
                element,
                count,
                destination,
                source,
                constructor,
                guarded,
            });
        }
        let mark = self.copy_mark();
        let mut source = None;
        for index in 0..count {
            let to = self.element_address(destination, index, size, element)?;
            let base = if let Some(source) = source {
                source
            } else {
                let base = self.assignment_source(function)?;
                let base = self.copy_address(base, action.ty, transfer)?;
                source = Some(base);
                base
            };
            let from = self.element_address(base, index, size, element)?;
            self.call_copy_constructor(constructor, Variant::Complete, to, from)?;
            if guarded && index + 1 < count {
                self.copied_subobject(element, to);
            }
        }
        self.copy_truncate(mark);
        Ok(())
    }
    fn copy_elements_loop(&mut self, elements: Elements) -> Result<(), Error> {
        let Elements {
            element,
            count,
            destination,
            source,
            constructor,
            guarded,
        } = elements;
        let slot = self.generated_slot("copy_array_index", Type::I64)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, zero, slot))?;
        let condition = self.new_block("copy_array_cond")?;
        let body = self.new_block("copy_array_body")?;
        let end = self.new_block("copy_array_end")?;
        let cleanup = if guarded {
            Some(self.new_block("copy_array_cleanup")?)
        } else {
            None
        };
        let chained = self.copy_mark() != 0;
        let outer = if chained {
            Some(self.copy_handler()?)
        } else {
            None
        };
        if let Some(outer) = outer {
            self.effect(I::Runtime(R::EhTry(outer)))?;
        }
        self.jump(condition)?;
        self.place(condition)?;
        let current = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let count = self.builder.integer(u128::from(count))?;
        let test = self
            .emit(I::Compare(Compare::Ult, Type::I64, current, count), element)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.terminated = true;
        self.place(body)?;
        let to = self.copy_dynamic_element(element, destination, current)?;
        let from = self.copy_dynamic_element(element, source, current)?;
        if let Some(cleanup) = cleanup {
            self.effect(I::Runtime(R::EhTry(cleanup)))?;
        }
        self.copying.active = false;
        self.call_copy_constructor(constructor, Variant::Complete, to, from)?;
        self.copying.active = true;
        if guarded {
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
        self.jump(condition)?;
        if let Some(cleanup) = cleanup {
            self.copy_array_cleanup(elements, slot, cleanup, outer)?;
        }
        self.place(end)?;
        if chained {
            self.effect(I::Runtime(R::EhEnd))?;
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            self.place_copy_handlers()?;
            self.place(next)?;
        }
        Ok(())
    }
    fn copy_dynamic_element(
        &mut self,
        element: TypeId,
        base: Value,
        index: Value,
    ) -> Result<Value, Error> {
        let size = self
            .builder
            .integer(u128::from(types::layout(self.graph, element)?.0))?;
        let offset = self
            .emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                element,
            )?
            .value;
        Ok(self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                element,
            )?
            .value)
    }
    fn copy_array_cleanup(
        &mut self,
        elements: Elements,
        slot: Value,
        cleanup: BlockId,
        outer: Option<BlockId>,
    ) -> Result<(), Error> {
        let test = self.new_block("construction_array_cleanup")?;
        let body = self.new_block("construction_array_cleanup_body")?;
        let done = self.new_block("construction_array_cleanup_end")?;
        self.place(cleanup)?;
        self.jump(test)?;
        self.place(test)?;
        let element = elements.element;
        let made = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let zero = self.builder.integer(0)?;
        let condition = self
            .emit(I::Compare(Compare::Ne, Type::I64, made, zero), element)?
            .value;
        self.effect(I::Branch(condition, body, done))?;
        self.terminated = true;
        self.place(body)?;
        let one = self.builder.integer(1)?;
        let last = self
            .emit(
                I::Binary(Binary::Sub, Facts::default(), Type::I64, made, one),
                element,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, last, slot))?;
        let throws = self.destructor_throws(element)?;
        if throws {
            self.open_destructor_unwind_guard()?;
        }
        let object = self.copy_dynamic_element(element, elements.destination, last)?;
        self.call_destructor(element, object)?;
        if throws {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        self.jump(test)?;
        self.place_destructor_terminates()?;
        self.place(done)?;
        if let Some(outer) = outer {
            self.effect(I::Runtime(R::EhEnd))?;
            self.effect(I::Runtime(R::EhEnd))?;
            self.jump(outer)?;
        } else if self.lifetime.function_guard.is_some() {
            self.terminate_unwinding()?;
        } else {
            self.effect(I::Runtime(R::Resume))?;
            self.terminated = true;
        }
        Ok(())
    }
}
