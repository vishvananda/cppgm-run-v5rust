// Translated from lower/cleanup_lowering.cpp::destruction_order/destructor_epilogue
// and function_lowering.cpp::destructor_body. See NOTICE.
//! Reverse subobject actions and exception continuations live with one function.
use super::{Error, construction::Entry, function::Function};
use crate::{
    lowir::{
        model::{BlockId, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Binary, Compare, Projection, Type},
    },
    sema::lowering::{EntityId, TypeId, TypeView},
};

#[derive(Clone, Copy)]
pub(super) struct Subobject {
    pub ty: TypeId,
    pub offset: u64,
    pub member: bool,
    pub element: Option<u64>,
}
#[derive(Default)]
pub(super) struct Destruction {
    actions: Vec<Subobject>,
    pub class: Option<EntityId>,
    pub cleanup: Option<BlockId>,
    pub epilogue: Option<BlockId>,
    pub body_target: Option<BlockId>,
    progress: Option<Value>,
    terminates: Vec<(BlockId, i64)>,
    pub selector: i64,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn begin_destructor(&mut self, entity: EntityId) -> Result<(), Error> {
        if self.symbols.destructor_inert(entity)? {
            return Ok(());
        }
        let class = self
            .graph
            .lower_function(entity)?
            .object
            .ok_or(crate::sema::Error::Invariant("destructor without class"))?;
        self.destruction.class = Some(class);
        if !self.graph.lower_union_class(class) {
            for &member in self.graph.lower_class_member_entities(class).iter().rev() {
                let ty = self.graph.lower_entity_type(member)?;
                if self.symbols.class_destructor(ty)?.is_none() {
                    continue;
                }
                let offset = self.graph.lower_member_offset(member)?;
                let mut count = 1u64;
                let mut element = ty;
                while let TypeView::Array {
                    element: child,
                    bound,
                } = self.graph.lower_type(element)?
                {
                    count = count.wrapping_mul(bound);
                    element = child;
                }
                if count <= 8
                    && let TypeView::Array { bound, .. } = self.graph.lower_type(ty)?
                {
                    for index in (0..bound).rev() {
                        self.destruction.actions.push(Subobject {
                            ty,
                            offset,
                            member: true,
                            element: Some(index),
                        });
                    }
                } else {
                    self.destruction.actions.push(Subobject {
                        ty,
                        offset,
                        member: true,
                        element: None,
                    });
                }
            }
        }
        for base in self.graph.lower_destructor_bases(class).rev() {
            let (base, offset) = base?;
            let ty = self.graph.lower_entity_type(base)?;
            if self.symbols.class_destructor(ty)?.is_some() {
                self.destruction.actions.push(Subobject {
                    ty,
                    offset,
                    member: false,
                    element: None,
                });
            }
        }
        if self.destruction.actions.is_empty() {
            return Ok(());
        }
        let cleanup = self.new_block("destructor_cleanup")?;
        self.destruction.cleanup = Some(cleanup);
        if self.destruction.actions.len() > 8 {
            let progress = self.generated_slot("destructor_progress", Type::I64)?;
            let zero = self.builder.integer(0)?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                zero,
                progress,
            ))?;
            self.destruction.progress = Some(progress);
        }
        self.effect(I::Runtime(R::EhCleanup(Some(cleanup))))?;
        if self
            .lifetime
            .body
            .map(|body| self.body_unwinds(body, &mut false))
            .transpose()?
            .unwrap_or(false)
        {
            self.destruction.body_target = Some(self.new_block("destructor_cleanup_entry")?);
        }
        Ok(())
    }
    pub(super) fn destroy_subobject(
        &mut self,
        action: Subobject,
        unwinding: bool,
    ) -> Result<(), Error> {
        let destructor =
            self.symbols
                .class_destructor(action.ty)?
                .ok_or(crate::sema::Error::Invariant(
                    "destruction action without destructor",
                ))?;
        let guard = if unwinding && !self.graph.lower_function(destructor)?.nothrow {
            let guard = self.open_destructor_unwind_guard()?;
            Some(guard)
        } else {
            None
        };
        if action.element.is_none()
            && let TypeView::Array { bound, .. } = self.graph.lower_type(action.ty)?
        {
            let mut element = action.ty;
            let mut count = 1u64;
            while let TypeView::Array {
                element: inner,
                bound,
            } = self.graph.lower_type(element)?
            {
                count = count.wrapping_mul(bound);
                element = inner;
            }
            if count <= 8 {
                for index in (0..bound).rev() {
                    self.destroy_subobject(
                        Subobject {
                            element: Some(index),
                            ..action
                        },
                        false,
                    )?;
                }
                if guard.is_some() {
                    self.effect(I::Runtime(R::EhEnd))?;
                }
                return Ok(());
            }
        }
        let class = self.destruction.class.ok_or(crate::sema::Error::Invariant(
            "destruction actions without class",
        ))?;
        let source = self.graph.lower_entity_type(class)?;
        let object = self.object_pointer(source)?.value;
        let offset = self.builder.integer(u128::from(action.offset))?;
        let object = self
            .emit(
                I::Index(
                    Type::I8,
                    action.member.then_some(Projection::Field),
                    object,
                    offset,
                ),
                action.ty,
            )?
            .value;
        if let TypeView::Array { element, .. } = self.graph.lower_type(action.ty)? {
            if let Some(index) = action.element {
                let size = super::types::layout(self.graph, element)?.0;
                let object = self.element_address(object, index, size, element)?;
                self.destroy_object(element, object)?;
            } else {
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
                if !unwinding && !self.graph.lower_function(destructor)?.nothrow {
                    self.destroy_member_array_guarded(element, object, count)?;
                } else {
                    self.destroy_array_loop(element, object, count)?;
                }
            }
        } else {
            let entry = if !action.member
                && self.graph.lower_function(destructor)?.emission
                    == crate::sema::lowering::Emission::Always
            {
                Entry::DestructorBase(destructor)
            } else {
                Entry::Function(destructor)
            };
            self.symbols.demand_entry(entry);
            let symbol = self.symbols.entry_symbol(self.builder.program(), entry)?;
            let arguments = self.builder.arguments(&[object])?;
            self.effect(I::Call(crate::lowir::model::Call {
                result: Type::Void,
                callee: Value::Symbol(symbol),
                arguments,
                signature: None,
                copy_elision: false,
            }))?;
        }
        if guard.is_some() {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        Ok(())
    }
    pub(super) fn open_destructor_unwind_guard(&mut self) -> Result<BlockId, Error> {
        let guard = self.new_cleanup_terminate()?;
        self.effect(I::Runtime(R::EhTry(guard)))?;
        Ok(guard)
    }
    pub(super) fn new_cleanup_terminate(&mut self) -> Result<BlockId, Error> {
        let guard = self.new_block("lexical_cleanup_terminate")?;
        self.destruction.selector = self
            .destruction
            .selector
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        self.destruction
            .terminates
            .push((guard, self.destruction.selector));
        self.symbols.terminate_helper(self.builder.program())?;
        Ok(guard)
    }
    pub(super) fn destroy_member_array_guarded(
        &mut self,
        element: TypeId,
        base: Value,
        count: u64,
    ) -> Result<(), Error> {
        let size = super::types::layout(self.graph, element)?.0;
        let slot = self.generated_slot("destructor_array_index", Type::I64)?;
        let count = self.builder.integer(u128::from(count))?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, count, slot))?;
        let condition = self.new_block("destructor_array_cond")?;
        let body = self.new_block("destructor_array_body")?;
        let end = self.new_block("destructor_array_end")?;
        let handler = self.new_block("destructor_array_cleanup")?;
        self.jump(condition)?;
        self.place(condition)?;
        let remaining = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let zero = self.builder.integer(0)?;
        let test = self
            .emit(I::Compare(Compare::Ne, Type::I64, remaining, zero), element)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.terminated = true;
        self.place(body)?;
        let one = self.builder.integer(1)?;
        let last = self
            .emit(
                I::Binary(Binary::Sub, Facts::default(), Type::I64, remaining, one),
                element,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, last, slot))?;
        self.effect(I::Runtime(R::EhCleanup(Some(handler))))?;
        self.destroy_member_array_element(element, base, last, size)?;
        self.effect(I::Runtime(R::EhEnd))?;
        self.jump(condition)?;
        let handler_body = self.new_block("destructor_array_cleanup_body")?;
        let resume = self.new_block("destructor_array_resume")?;
        self.place(handler)?;
        let left = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let test = self
            .emit(I::Compare(Compare::Ne, Type::I64, left, zero), element)?
            .value;
        self.effect(I::Branch(test, handler_body, resume))?;
        self.terminated = true;
        self.place(handler_body)?;
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
        self.open_destructor_unwind_guard()?;
        self.destroy_member_array_element(element, base, previous, size)?;
        self.effect(I::Runtime(R::EhEnd))?;
        self.jump(handler)?;
        self.place_destructor_terminates()?;
        self.place(resume)?;
        self.continue_destructor_unwind()?;
        self.place(end)
    }
    fn destroy_member_array_element(
        &mut self,
        element: TypeId,
        base: Value,
        index: Value,
        size: u64,
    ) -> Result<(), Error> {
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
        let object = self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                element,
            )?
            .value;
        self.destroy_object(element, object)
    }
    fn destruction_resume(&mut self) -> Result<(), Error> {
        self.effect(I::Runtime(R::EhEnd))?;
        self.continue_destructor_unwind()
    }
    fn continue_destructor_unwind(&mut self) -> Result<(), Error> {
        if self.lifetime.function_guard.is_some() {
            self.terminate_unwinding()?;
        } else {
            self.effect(I::Runtime(R::Resume))?;
            self.terminated = true;
        }
        self.place_destructor_terminates()
    }
    pub(super) fn finish_destructor(&mut self) -> Result<(), Error> {
        let Some(cleanup) = self.destruction.cleanup else {
            return Ok(());
        };
        let body_entry = self.destruction.body_target.take();
        if !self.terminated {
            self.effect(I::Runtime(R::EhEnd))?;
            if let Some(epilogue) = self.destruction.epilogue {
                self.jump(epilogue)?;
            }
        }
        if let Some(epilogue) = self.destruction.epilogue {
            self.place(epilogue)?;
        }
        let end = self.new_block("destructor_end")?;
        let count = self.destruction.actions.len();
        for index in 0..count {
            if index + 1 == count {
                self.destroy_subobject(self.destruction.actions[index], false)?;
                break;
            }
            if let Some(progress) = self.destruction.progress {
                let value = self
                    .builder
                    .integer(u128::try_from(index + 1).map_err(|_| crate::lowir::Error::Limit)?)?;
                self.effect(I::Store(
                    Facts::memory(false, true),
                    Type::I64,
                    value,
                    progress,
                ))?;
                self.effect(I::Runtime(R::EhCleanup(Some(cleanup))))?;
                self.destroy_subobject(self.destruction.actions[index], false)?;
                self.effect(I::Runtime(R::EhEnd))?;
            } else {
                let suffix = self.new_block("destructor_suffix_cleanup")?;
                let next = self.new_block("destructor_suffix_next")?;
                self.effect(I::Runtime(R::EhCleanup(Some(suffix))))?;
                self.destroy_subobject(self.destruction.actions[index], false)?;
                self.effect(I::Runtime(R::EhEnd))?;
                self.jump(next)?;
                self.place(suffix)?;
                for k in index + 1..count {
                    self.destroy_subobject(self.destruction.actions[k], true)?;
                }
                self.destruction_resume()?;
                self.place(next)?;
            }
        }
        self.jump(end)?;
        self.place(cleanup)?;
        if let Some(entry) = body_entry {
            self.effect(I::Runtime(R::EhEnd))?;
            self.jump(entry)?;
            self.place(entry)?;
        }
        if let Some(progress) = self.destruction.progress {
            let mut chain = Vec::with_capacity(count);
            let mut cases = Vec::with_capacity(count);
            for index in 0..count {
                let block = self.new_block("destructor_suffix_cleanup")?;
                let value = self
                    .builder
                    .integer(u128::try_from(index).map_err(|_| crate::lowir::Error::Limit)?)?;
                chain.push(block);
                cases.push((value, block));
            }
            let source = self.graph.lower_entity_type(self.destruction.class.ok_or(
                crate::sema::Error::Invariant("destructor progress without class"),
            )?)?;
            let value = self
                .emit(
                    I::Load(Facts::memory(false, true), Type::I64, progress),
                    source,
                )?
                .value;
            self.effect(I::Switch(value, chain[0], cases))?;
            self.terminated = true;
            for index in 0..count {
                self.place(chain[index])?;
                self.destroy_subobject(self.destruction.actions[index], true)?;
                if let Some(&next) = chain.get(index + 1) {
                    self.jump(next)?;
                } else if body_entry.is_some() {
                    self.continue_destructor_unwind()?;
                } else {
                    self.destruction_resume()?;
                }
            }
        } else {
            for index in 0..count {
                self.destroy_subobject(self.destruction.actions[index], true)?;
            }
            if body_entry.is_some() {
                self.continue_destructor_unwind()?;
            } else {
                self.destruction_resume()?;
            }
        }
        self.place(end)?;
        self.destruction.cleanup = None;
        Ok(())
    }
    pub(super) fn place_destructor_terminates(&mut self) -> Result<(), Error> {
        for (block, selector) in std::mem::take(&mut self.destruction.terminates) {
            self.place(block)?;
            self.effect(I::Runtime(R::EhCatchAll(Some(selector))))?;
            let exception = self
                .builder
                .generated(I::Runtime(R::Exception(Type::Ptr)))?;
            self.terminate_exception(exception)?;
            self.effect(I::Unreachable)?;
            self.terminated = true;
        }
        Ok(())
    }
}
