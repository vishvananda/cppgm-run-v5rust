// Translated from lower/copy_lowering.cpp::construction_made/handler. See NOTICE.
//! Synthesized construction uses address-owned plain unwind chains, local to one body.
use super::{Error, function::Function, types};
use crate::{
    lowir::model::{BlockId, InstructionKind as I, RuntimeInstruction as R, Value},
    sema::lowering::{TypeId, TypeView},
};
#[derive(Clone, Copy)]
struct Made {
    address: Value,
    ty: TypeId,
    handler: Option<BlockId>,
}
#[derive(Default)]
pub(super) struct State {
    pub active: bool,
    actions: Vec<Made>,
    resume: Option<BlockId>,
    pending: Vec<Option<usize>>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn copy_mark(&self) -> usize {
        self.copying.actions.len()
    }
    pub(super) fn copy_truncate(&mut self, mark: usize) {
        self.copying.actions.truncate(mark);
    }
    pub(super) fn copied_subobject(&mut self, ty: TypeId, address: Value) {
        self.copying.actions.push(Made {
            address,
            ty,
            handler: None,
        });
    }
    pub(super) fn copy_handler(&mut self) -> Result<BlockId, Error> {
        let resume = if let Some(resume) = self.copying.resume {
            resume
        } else {
            let resume = self.new_block("cleanup_resume")?;
            self.copying.resume = Some(resume);
            self.copying.pending.push(None);
            resume
        };
        let first = self
            .copying
            .actions
            .iter()
            .rposition(|action| action.handler.is_some())
            .map_or(0, |i| i + 1);
        for index in first..self.copying.actions.len() {
            let handler = self.new_block("construction_cleanup")?;
            self.copying.actions[index].handler = Some(handler);
            self.copying.pending.push(Some(index));
        }
        Ok(self
            .copying
            .actions
            .last()
            .and_then(|action| action.handler)
            .unwrap_or(resume))
    }
    pub(super) fn begin_copy_call(&mut self, throws: bool) -> Result<bool, Error> {
        let chained = self.copying.active && throws && !self.copying.actions.is_empty();
        if chained {
            let handler = self.copy_handler()?;
            self.effect(I::Runtime(R::EhTry(handler)))?;
        }
        Ok(chained)
    }
    pub(super) fn end_copy_call(&mut self, chained: bool) -> Result<(), Error> {
        if chained {
            self.effect(I::Runtime(R::EhEnd))?;
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            self.place_copy_handlers()?;
            self.place(next)?;
        }
        Ok(())
    }
    pub(super) fn place_copy_handlers(&mut self) -> Result<(), Error> {
        let pending = std::mem::take(&mut self.copying.pending);
        for index in pending {
            let resume = self
                .copying
                .resume
                .ok_or(crate::sema::Error::Invariant("copy cleanup resume"))?;
            let Some(index) = index else {
                self.place(resume)?;
                self.effect(I::Runtime(R::Resume))?;
                self.terminated = true;
                continue;
            };
            let action = self.copying.actions[index];
            let handler = action
                .handler
                .ok_or(crate::sema::Error::Invariant("copy cleanup handler"))?;
            self.place(handler)?;
            let throws = self.destructor_throws(action.ty)?;
            if throws {
                self.open_destructor_unwind_guard()?;
            }
            self.destroy_copied_object(action.ty, action.address)?;
            if throws {
                self.effect(I::Runtime(R::EhEnd))?;
            }
            let tail = if index == 0 {
                resume
            } else {
                self.copying.actions[index - 1]
                    .handler
                    .ok_or(crate::sema::Error::Invariant("copy cleanup tail"))?
            };
            self.jump(tail)?;
            self.place_destructor_terminates()?;
        }
        Ok(())
    }
    fn destroy_copied_object(&mut self, ty: TypeId, address: Value) -> Result<(), Error> {
        let mut element = ty;
        let mut count = 1u64;
        while let TypeView::Array {
            element: child,
            bound,
        } = self.graph.lower_type(element)?
        {
            count = count.wrapping_mul(bound);
            element = child;
        }
        if element == ty {
            return self.call_destructor(ty, address);
        }
        if count > 8 {
            return self.destroy_array_loop(element, address, count);
        }
        let size = types::layout(self.graph, element)?.0;
        for index in (0..count).rev() {
            // The chain's constant element offsets are already target byte offsets.
            let offset = self.builder.integer(u128::from(index.wrapping_mul(size)))?;
            let object = self
                .emit(
                    I::Index(
                        crate::lowir::vocabulary::Type::I8,
                        Some(crate::lowir::vocabulary::Projection::ArrayElement),
                        address,
                        offset,
                    ),
                    element,
                )?
                .value;
            self.call_destructor(element, object)?;
        }
        Ok(())
    }
}
