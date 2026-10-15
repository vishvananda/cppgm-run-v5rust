// Translated from lower/cleanup_lowering.cpp::subobject_constructed and
// lexical_cleanup_lowering.cpp::flush_deferred. See NOTICE.
//! Constructor regions retain completed subobjects until the body exits.
use super::{
    Error,
    construction::{Entry, Variant},
    destruction::Subobject,
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
}
enum CompletedObject {
    Subobject(Subobject),
    Whole { ty: TypeId, variant: Variant },
}
#[derive(Default)]
pub(super) struct Construction {
    pending: Vec<Deferred>,
    active: Option<BlockId>,
    pub detached: bool,
    pub placing: bool,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn subobject_constructed(
        &mut self,
        ty: TypeId,
        offset: u64,
        member: bool,
    ) -> Result<(), Error> {
        if self.symbols.class_destructor(ty)?.is_none() {
            return Ok(());
        }
        self.completed_construction(CompletedObject::Subobject(Subobject {
            ty,
            offset,
            member,
            element: None,
        }))
    }
    pub(super) fn delegated_object_constructed(
        &mut self,
        ty: TypeId,
        variant: Variant,
    ) -> Result<(), Error> {
        if self.symbols.class_destructor(ty)?.is_none() {
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
        });
        self.effect(I::Runtime(R::EhCleanup(Some(block))))?;
        self.construction.active = entry.or(Some(block));
        self.constructor_unwind_changed();
        Ok(())
    }
    pub(super) fn constructor_unwind_target(&self) -> Option<BlockId> {
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
        } else if self.lifetime.function_guard.is_some() {
            self.terminate_unwinding()
        } else {
            self.effect(I::Runtime(R::Resume))?;
            self.terminated = true;
            Ok(())
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
        for Deferred {
            landing,
            object,
            previous,
            entry,
        } in pending
        {
            // A deferred batch places its landings directly. Nested entry
            // placement, rather than the next landing, flushes terminate guards.
            self.builder.place_block(landing)?;
            self.terminated = false;
            if let Some(entry) = entry {
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
                if self.lifetime.function_guard.is_some() {
                    self.terminate_unwinding()?;
                } else {
                    self.effect(I::Runtime(R::Resume))?;
                    self.terminated = true;
                }
            }
        }
        self.construction.placing = false;
        Ok(())
    }
    fn destroy_delegated_object(&mut self, ty: TypeId, variant: Variant) -> Result<(), Error> {
        let destructor =
            self.symbols
                .class_destructor(ty)?
                .ok_or(crate::sema::Error::Invariant(
                    "delegated cleanup without destructor",
                ))?;
        let facts = self.graph.lower_function(destructor)?;
        let guarded = !facts.nothrow;
        if guarded {
            self.open_destructor_unwind_guard()?;
        }
        let entry = if variant == Variant::Base
            && facts.emission == crate::sema::lowering::Emission::Always
        {
            Entry::DestructorBase(destructor)
        } else {
            Entry::Function(destructor)
        };
        let object = self.object_pointer(ty)?.value;
        let symbol = self.symbols.entry_symbol(self.builder.program(), entry)?;
        self.symbols.demand_entry(entry);
        let arguments = self.builder.arguments(&[object])?;
        self.effect(I::Call(crate::lowir::model::Call {
            result: crate::lowir::vocabulary::Type::Void,
            callee: crate::lowir::model::Value::Symbol(symbol),
            arguments,
            signature: None,
            copy_elision: false,
        }))?;
        if guarded {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        Ok(())
    }
}
