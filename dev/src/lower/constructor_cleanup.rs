// Translated from lower/cleanup_lowering.cpp::subobject_constructed and
// lexical_cleanup_lowering.cpp::flush_deferred. See NOTICE.
//! Constructor regions retain completed subobjects until the body exits.
use super::{
    Error,
    construction::{Entry, Variant},
    destruction::{BaseSubobject, Subobject},
    function::Function,
};
use crate::{
    lowir::model::{BlockId, InstructionKind as I, RuntimeInstruction as R},
    sema::lowering::TypeId,
};

struct Deferred {
    landing: BlockId,
    object: CompletedObject,
    previous: Option<BlockId>,
    entry: Option<BlockId>,
    contexts: usize,
}
enum CompletedObject {
    Subobject(Subobject),
    Whole { ty: TypeId, variant: Variant },
}
#[derive(Default)]
pub(super) struct Construction {
    pending: Vec<Deferred>,
    pub active: Option<BlockId>,
    pub detached: bool,
    pub depth: usize,
    pub placing: bool,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn subobject_constructed(
        &mut self,
        ty: TypeId,
        offset: u64,
        base: Option<BaseSubobject>,
    ) -> Result<(), Error> {
        if self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .class_destructor(ty)?
            .is_none()
        {
            return Ok(());
        }
        self.completed_construction(CompletedObject::Subobject(Subobject {
            ty,
            offset,
            base,
            element: None,
        }))
    }
    pub(super) fn delegated_object_constructed(
        &mut self,
        ty: TypeId,
        variant: Variant,
    ) -> Result<(), Error> {
        if self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .class_destructor(ty)?
            .is_none()
        {
            return Ok(());
        }
        self.completed_construction(CompletedObject::Whole { ty, variant })
    }
    fn completed_construction(&mut self, object: CompletedObject) -> Result<(), Error> {
        let previous = self.construction.active;
        if previous.is_some() {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        let block = self.new_block("constructor_cleanup")?;
        let entry = if self.construction.detached {
            Some(self.new_block("constructor_cleanup_entry")?)
        } else {
            None
        };
        self.construction.pending.push(Deferred {
            landing: block,
            object,
            previous,
            entry,
            contexts: self.handlers.contexts.len(),
        });
        self.effect(I::Runtime(R::EhCleanup(Some(block))))?;
        self.construction.active = entry.or(Some(block));
        self.construction.depth = self.handlers.contexts.len();
        self.constructor_unwind_changed();
        Ok(())
    }
    pub(super) fn constructor_unwind_target(&self) -> Option<BlockId> {
        let depth = if self.construction.detached {
            self.construction.depth
        } else {
            self.destruction.body_depth
        };
        if self
            .handlers
            .innermost
            .is_some_and(|index| depth == 0 || index >= depth)
        {
            return None;
        }
        self.detached_body_target()
    }
    pub(super) fn detached_body_target(&self) -> Option<BlockId> {
        self.construction
            .detached
            .then_some(self.construction.active)
            .flatten()
            .or(self.destruction.body_target)
    }
    pub(super) fn finish_array_unwind(&mut self) -> Result<(), Error> {
        if let Some(target) = self.constructor_unwind_target() {
            self.effect(I::Runtime(R::EhEnd))?;
            self.effect(I::Runtime(R::EhEnd))?;
            self.jump(target)
        } else {
            self.handler_resume()
        }
    }
    pub(super) fn finish_constructor_cleanups(&mut self) -> Result<(), Error> {
        self.place_constructor_cleanups()?;
        self.construction.active = None;
        Ok(())
    }
    pub(super) fn end_constructor_region(&mut self) -> Result<(), Error> {
        if self.construction.active.is_some() {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        Ok(())
    }
    pub(super) fn place_constructor_cleanups(&mut self) -> Result<(), Error> {
        let pending = std::mem::take(&mut self.construction.pending);
        if pending.is_empty() {
            return Ok(());
        }
        self.construction.placing = true;
        for deferred in pending {
            // A body's inner try/handler is not part of the context in which
            // this subobject completed. Restore that context for its calls
            // and any pending-segment event, then restore the body on errors too.
            let contexts = if deferred.entry.is_some() {
                deferred.contexts.min(self.handlers.contexts.len())
            } else {
                self.handlers.contexts.len()
            };
            self.with_handler_contexts(contexts, |this| this.place_constructor_cleanup(deferred))?;
        }
        self.construction.placing = false;
        Ok(())
    }
    fn place_constructor_cleanup(
        &mut self,
        Deferred {
            landing,
            object,
            previous,
            entry,
            contexts: _,
        }: Deferred,
    ) -> Result<(), Error> {
        // A deferred batch places its landings directly. Nested entry
        // placement, rather than the next landing, flushes terminate guards.
        self.builder.place_block(landing)?;
        self.terminated = false;
        let attempt = self.innermost_try();
        if let Some(entry) = entry {
            if let Some(attempt) = attempt {
                self.emit_try_clauses(attempt)?;
            }
            self.effect(I::Runtime(R::EhEnd))?;
            self.jump(entry)?;
            self.place(entry)?;
        }
        match object {
            CompletedObject::Subobject(subobject) => self.destroy_subobject(subobject, true)?,
            CompletedObject::Whole { ty, variant } => {
                self.destroy_delegated_object(ty, variant)?;
            }
        }
        if let Some(previous) = previous {
            self.jump(previous)?;
        } else {
            if entry.is_none() {
                self.effect(I::Runtime(R::EhEnd))?;
            }
            if entry.is_some()
                && let Some(attempt) = attempt
            {
                self.effect(I::Runtime(R::EhEnd))?;
                self.jump(attempt.entry)?;
            } else if entry.is_none() && self.handlers.function_try.is_some() {
                // flush_deferred: a constructor's function-try still encloses
                // subobject initialization after its source context was popped.
                self.effect(I::Runtime(R::Resume))?;
                self.terminated = true;
            } else {
                self.handler_resume()?;
            }
        }
        Ok(())
    }
    fn destroy_delegated_object(&mut self, ty: TypeId, variant: Variant) -> Result<(), Error> {
        let destructor = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .class_destructor(ty)?
            .ok_or(crate::sema::Error::Invariant(
                "delegated cleanup without destructor",
            ))?;
        let guarded =
            self.lifetime_call_throws(crate::sema::lowering::CallTarget::Direct(destructor))?;
        if guarded {
            self.open_destructor_unwind_guard()?;
        }
        let facts = self.graph.lower_function(destructor)?;
        let hidden = variant == Variant::Base
            && !self
                .graph
                .lower_virtual_bases(
                    facts
                        .object
                        .ok_or(crate::sema::Error::Invariant("delegated cleanup class"))?,
                )
                .is_empty();
        let entry = if variant == Variant::Base
            && (facts.emission == crate::sema::lowering::Emission::Always || hidden)
        {
            Entry::DestructorBase(destructor)
        } else {
            Entry::Function(destructor)
        };
        let object = self.object_pointer(ty)?.value;
        let mut values = vec![object];
        if hidden {
            let saved = self.virtual_context.hidden;
            self.virtual_context.hidden = Some(0);
            let result = self.hidden_vtt_value();
            self.virtual_context.hidden = saved;
            values.push(result?);
        }
        let symbol = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .entry_symbol(self.builder.program(), entry)?;
        self.symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .demand_entry(entry);
        let arguments = self.builder.arguments(&values)?;
        self.effect(I::Call(crate::lowir::model::Call {
            result: crate::lowir::vocabulary::Type::Void,
            callee: crate::lowir::model::Value::Symbol(symbol),
            arguments,
            signature: None,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
        }))?;
        if guarded {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        Ok(())
    }
}
