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
pub(super) enum BaseSubobject {
    Direct(u32),
    Virtual(EntityId),
}
#[derive(Clone, Copy)]
pub(super) struct Subobject {
    pub ty: TypeId,
    pub offset: u64,
    pub base: Option<BaseSubobject>,
    pub element: Option<u64>,
}
#[derive(Default)]
pub(super) struct Destruction {
    actions: Vec<Subobject>,
    deallocate: bool,
    pub class: Option<EntityId>,
    pub cleanup: Option<BlockId>,
    pub epilogue: Option<BlockId>,
    pub body_target: Option<BlockId>,
    pub body_depth: usize,
    progress: Option<Value>,
    terminates: Vec<(BlockId, i64)>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn destructor_prologue(
        &mut self,
        entity: EntityId,
        deleting: bool,
        calls_complete: bool,
    ) -> Result<(), Error> {
        self.begin_destructor(entity, deleting, calls_complete)?;
        let class = self
            .graph
            .lower_function(entity)?
            .object
            .ok_or(crate::sema::Error::Invariant("destructor class"))?;
        let source = self.graph.lower_entity_type(class)?;
        if calls_complete {
            let object = self.object_pointer(source)?.value;
            self.symbols.demand_entry(Entry::DestructorDeleting(entity));
            self.symbols.demand_entry(Entry::Function(entity));
            let callee = self
                .symbols
                .function_symbol(self.builder.program(), entity)?;
            let arguments = self.builder.arguments(&[object])?;
            self.effect(I::Call(crate::lowir::model::Call {
                result: Type::Void,
                callee: Value::Symbol(callee),
                arguments,
                signature: None,
                copy_elision: false,
            }))
        } else if self.graph.lower_polymorphic(class) {
            self.store_vptr(class, source)
        } else {
            // destructor_stores_tables: a vptr required only by virtual
            // inheritance is retained during destruction in the course form.
            Ok(())
        }
    }
    pub(super) fn begin_destructor(
        &mut self,
        entity: EntityId,
        deleting: bool,
        calls_complete: bool,
    ) -> Result<(), Error> {
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
                            base: None,
                            element: Some(index),
                        });
                    }
                } else {
                    self.destruction.actions.push(Subobject {
                        ty,
                        offset,
                        base: None,
                        element: None,
                    });
                }
            }
        }
        for (index, base) in self.graph.lower_destructor_bases(class).enumerate().rev() {
            let (base, offset) = base?;
            if self
                .graph
                .lower_inheritance_edge(class, index)
                .is_some_and(|(_, shared, _)| shared)
            {
                continue;
            }
            let ty = self.graph.lower_entity_type(base)?;
            if self.symbols.class_destructor(ty)?.is_some() {
                self.destruction.actions.push(Subobject {
                    ty,
                    offset,
                    base: Some(BaseSubobject::Direct(
                        u32::try_from(index).map_err(|_| crate::lowir::Error::Limit)?,
                    )),
                    element: None,
                });
            }
        }
        if self.virtual_context.vtt.is_none() {
            for base in self
                .symbols
                .virtual_construction_order(class)
                .into_iter()
                .rev()
            {
                let ty = self.graph.lower_entity_type(base)?;
                if self.symbols.class_destructor(ty)?.is_some() {
                    let offset = self.graph.lower_virtual_base_offset(class, base).ok_or(
                        crate::sema::Error::Invariant("destructor virtual base layout"),
                    )?;
                    self.destruction.actions.push(Subobject {
                        ty,
                        offset,
                        base: Some(BaseSubobject::Virtual(base)),
                        element: None,
                    });
                }
            }
        }
        if calls_complete {
            self.destruction.actions.clear();
        }
        self.destruction.deallocate = deleting;
        if self.destruction.actions.is_empty() && !deleting {
            return Ok(());
        }
        let cleanup = self.new_block("destructor_cleanup")?;
        self.destruction.cleanup = Some(cleanup);
        if self.destruction.actions.len() + usize::from(deleting) > 8 {
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
        if !calls_complete
            && self
                .lifetime
                .body
                .map(|body| self.body_unwinds(body, &mut false))
                .transpose()?
                .unwrap_or(false)
        {
            self.destruction.body_target = Some(self.new_block("destructor_cleanup_entry")?);
            self.destruction.body_depth = self.handlers.contexts.len();
        }
        Ok(())
    }
    fn destructor_action(&mut self, index: usize, unwinding: bool) -> Result<(), Error> {
        if let Some(&action) = self.destruction.actions.get(index) {
            return self.destroy_subobject(action, unwinding);
        }
        if !self.destruction.deallocate || index != self.destruction.actions.len() {
            return Err(
                crate::sema::Error::Invariant("destructor action outside its recipe").into(),
            );
        }
        let class = self
            .destruction
            .class
            .ok_or(crate::sema::Error::Invariant("deleting class"))?;
        let source = self.graph.lower_entity_type(class)?;
        let function = self.graph.lower_virtual_deallocator(class)?;
        let size = self
            .builder
            .integer(u128::from(super::types::layout(self.graph, source)?.0))?;
        let object = self.object_pointer(source)?.value;
        self.deallocate_storage(function, object, size, None, false)
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
        let class = self
            .virtual_context
            .class
            .ok_or(crate::sema::Error::Invariant(
                "destruction actions without class",
            ))?;
        let source = self.graph.lower_entity_type(class)?;
        let object = self.object_pointer(source)?.value;
        let offset = self.builder.integer(u128::from(action.offset))?;
        let object = self
            .emit(
                I::Index(
                    Type::I8,
                    action.base.is_none().then_some(Projection::Field),
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
            let hidden = action.base.is_some()
                && self
                    .graph
                    .lower_function(destructor)?
                    .object
                    .is_some_and(|base| !self.graph.lower_virtual_bases(base).is_empty());
            let own_entry = matches!(action.base, Some(BaseSubobject::Virtual(_)))
                && self
                    .entity
                    .map(|entity| self.graph.lower_function(entity))
                    .transpose()?
                    .is_some_and(|facts| {
                        matches!(
                            facts.name,
                            crate::sema::lowering::FunctionName::Destructor(_)
                        )
                    });
            let entry = if action.base.is_some()
                && (self.graph.lower_function(destructor)?.emission
                    == crate::sema::lowering::Emission::Always
                    || self.graph.lower_virtual_slot(destructor).is_some()
                    || hidden
                    || own_entry)
            {
                Entry::DestructorBase(destructor)
            } else {
                Entry::Function(destructor)
            };
            let mut values = vec![object];
            if hidden {
                let saved = self.virtual_context.hidden;
                self.virtual_context.hidden = Some(match action.base {
                    Some(BaseSubobject::Direct(index)) => {
                        self.symbols.vtt_base_index(class, index as usize)?
                    }
                    Some(BaseSubobject::Virtual(base)) => {
                        self.symbols.vtt_virtual_index(class, base)?
                    }
                    None => {
                        return Err(
                            crate::sema::Error::Invariant("hidden member destructor").into()
                        );
                    }
                });
                let result = self.hidden_vtt_value();
                self.virtual_context.hidden = saved;
                values.push(result?);
            }
            self.symbols.demand_entry(entry);
            let symbol = self.symbols.entry_symbol(self.builder.program(), entry)?;
            let arguments = self.builder.arguments(&values)?;
            // A deferred subobject cleanup can be emitted while a source
            // initializer's segment is pending. Its nonthrowing destructor
            // is still that segment's next region event (emit_call).
            // A tracked source segment also covers a subobject call here;
            // only an explicit terminate guard already owns this call.
            if guard.is_none() {
                self.before_lifetime_call(false)?;
            }
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
        let selector = i64::from(self.next_handler_selector()?);
        self.destruction.terminates.push((guard, selector));
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
        if self.lifetime.function_guard.is_some() && self.innermost_try().is_none() {
            self.terminate_unwinding()?;
        } else {
            self.effect(I::Runtime(R::Resume))?;
            self.terminated = true;
        }
        self.place_destructor_terminates()
    }
    fn close_destructor_body_cleanup(&mut self) -> Result<(), Error> {
        if let Some(attempt) = self.innermost_try() {
            self.effect(I::Runtime(R::EhEnd))?;
            self.jump(attempt.entry)?;
            self.place_destructor_terminates()
        } else {
            self.continue_destructor_unwind()
        }
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
        let count = self.destruction.actions.len() + usize::from(self.destruction.deallocate);
        for index in 0..count {
            if index + 1 == count {
                self.destructor_action(index, false)?;
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
                self.destructor_action(index, false)?;
                self.effect(I::Runtime(R::EhEnd))?;
            } else {
                let suffix = self.new_block("destructor_suffix_cleanup")?;
                let next = self.new_block("destructor_suffix_next")?;
                self.effect(I::Runtime(R::EhCleanup(Some(suffix))))?;
                self.destructor_action(index, false)?;
                self.effect(I::Runtime(R::EhEnd))?;
                self.jump(next)?;
                self.place(suffix)?;
                for k in index + 1..count {
                    self.destructor_action(k, true)?;
                }
                self.destruction_resume()?;
                self.place(next)?;
            }
        }
        self.jump(end)?;
        self.place(cleanup)?;
        if let Some(entry) = body_entry {
            if let Some(attempt) = self.innermost_try() {
                self.emit_try_clauses(attempt)?;
            }
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
                self.destructor_action(index, true)?;
                if let Some(&next) = chain.get(index + 1) {
                    self.jump(next)?;
                } else if body_entry.is_some() {
                    self.close_destructor_body_cleanup()?;
                } else {
                    self.destruction_resume()?;
                }
            }
        } else {
            for index in 0..count {
                self.destructor_action(index, true)?;
            }
            if body_entry.is_some() {
                self.close_destructor_body_cleanup()?;
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
