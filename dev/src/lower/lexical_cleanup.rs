// Translated from lower/lexical_cleanup_lowering.cpp and
// object_lowering.cpp::destroy_local. See NOTICE.
//! Lexical destruction has its own interned exceptional suffixes per body context.
use super::{
    Error,
    function::Function,
    lifetime::{Cleanup, CleanupPlace},
};
use crate::{
    lowir::model::{BlockId, InstructionKind as I, RuntimeInstruction as R, Value},
    sema::lowering::TypeId,
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ReturnedKey {
    Storage(TypeId, Value),
    Temporary(TypeId, Value),
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct DispatchKey {
    tail: BlockId,
    returned: Option<ReturnedKey>,
}

enum Block {
    Context {
        block: BlockId,
        action: super::exception_chains::Action,
    },
    Resume {
        block: BlockId,
        target: Option<BlockId>,
    },
    Destroy {
        block: BlockId,
        cleanup: Cleanup,
        tail: BlockId,
    },
    Dispatch {
        block: BlockId,
        tail: BlockId,
        returned: Option<Cleanup>,
    },
}
pub(super) type ResumeMemo = Option<(Option<BlockId>, BlockId)>;
#[derive(Default)]
pub(super) struct Lexical {
    pub returned: Option<Cleanup>,
    pending: Vec<Block>,
    pub resume: ResumeMemo,
    placing: bool,
    terminate: Option<BlockId>,
    dispatches: HashMap<DispatchKey, BlockId>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn destructor_throws(&mut self, ty: TypeId) -> Result<bool, Error> {
        self.symbols
            .class_destructor(ty)?
            .map_or(Ok(false), |entity| {
                Ok(!self.graph.lower_function(entity)?.nothrow)
            })
    }
    pub(super) fn cleanup_unwind_guard(&mut self) -> Result<BlockId, Error> {
        if self.lexical.placing {
            return self.shared_cleanup_unwind_guard();
        }
        self.new_cleanup_terminate()
    }
    pub(super) fn shared_cleanup_unwind_guard(&mut self) -> Result<BlockId, Error> {
        // Object landings have one enclosing region state. Serialized course
        // LowIR alone shares this landing across distinct destructions.
        if self.symbols.object_mode() {
            return self.new_cleanup_terminate();
        }
        if let Some(block) = self.lexical.terminate {
            return Ok(block);
        }
        let block = self.new_cleanup_terminate()?;
        // Serialized LowIR shares catch retirement and lexical destruction
        // landings for this function, independently of their placement time.
        self.lexical.terminate = Some(block);
        Ok(block)
    }
    pub(super) fn lexical_handler(&mut self, depth: usize) -> Result<Option<BlockId>, Error> {
        if !self.destructor_throws(self.lifetime.cleanups[depth].ty)? {
            return Ok(None);
        }
        if !self.handlers.contexts.is_empty() {
            return self.context_lexical_handler(depth);
        }
        let target = self.constructor_unwind_target();
        if depth == 0 && target.is_none() && self.lexical.returned.is_none() {
            return Ok(None);
        }
        let mut first = depth;
        while first > 0 && self.lifetime.cleanups[first - 1].lexical.is_none() {
            first -= 1;
        }
        let mut tail = if first == 0 {
            if let Some((_, block)) = self.lexical.resume.filter(|(saved, _)| *saved == target) {
                block
            } else {
                let block = self.new_block("lexical_unwind")?;
                self.lexical.pending.push(Block::Resume { block, target });
                self.lexical.resume = Some((target, block));
                block
            }
        } else {
            self.lifetime.cleanups[first - 1]
                .lexical
                .ok_or(crate::sema::Error::Invariant(
                    "lexical cleanup without tail",
                ))?
        };
        for index in first..depth {
            let block = self.new_block("lexical_unwind")?;
            self.lexical.pending.push(Block::Destroy {
                block,
                cleanup: self.lifetime.cleanups[index],
                tail,
            });
            self.lifetime.cleanups[index].lexical = Some(block);
            tail = block;
        }
        self.lexical_dispatch(tail).map(Some)
    }
    pub(super) fn lexical_dispatch(&mut self, tail: BlockId) -> Result<BlockId, Error> {
        let returned = self
            .lexical
            .returned
            .map(|cleanup| match cleanup.place {
                CleanupPlace::Storage(value) => Ok(ReturnedKey::Storage(cleanup.ty, value)),
                CleanupPlace::Temporary(value) => Ok(ReturnedKey::Temporary(cleanup.ty, value)),
                _ => Err(crate::sema::Error::Invariant(
                    "return cleanup without result storage",
                )),
            })
            .transpose()?;
        let key = DispatchKey { tail, returned };
        if let Some(&block) = self.lexical.dispatches.get(&key) {
            return Ok(block);
        }
        let block = self.new_block("lexical_cleanup_dispatch")?;
        if self.handlers.contexts.is_empty() {
            self.lexical.pending.push(Block::Dispatch {
                block,
                tail,
                returned: self.lexical.returned,
            });
        } else {
            self.queue_lexical_context(
                block,
                super::exception_chains::Action::LexicalDispatch {
                    context: self.innermost_try(),
                    tail,
                    returned: self.lexical.returned,
                },
            );
        }
        self.lexical.dispatches.insert(key, block);
        Ok(block)
    }
    pub(super) fn queue_lexical_context(
        &mut self,
        block: BlockId,
        action: super::exception_chains::Action,
    ) {
        self.lexical.pending.push(Block::Context { block, action });
    }
    pub(super) fn place_lexical_cleanups(&mut self) -> Result<(), Error> {
        let mut pending = std::mem::take(&mut self.lexical.pending);
        let saved = std::mem::replace(&mut self.lexical.placing, true);
        for item in &pending {
            match *item {
                Block::Context { block, action } => {
                    self.place(block)?;
                    self.place_context_action(action)?;
                }
                Block::Resume { block, target } => {
                    self.place(block)?;
                    if let Some(target) = target {
                        self.effect(I::Runtime(R::EhEnd))?;
                        self.jump(target)?;
                    } else {
                        self.resume_outside_tries()?;
                    }
                }
                Block::Destroy {
                    block,
                    cleanup,
                    tail,
                } => {
                    self.place(block)?;
                    self.destroy_cleanup_in(cleanup, None, true)?;
                    self.jump(tail)?;
                }
                Block::Dispatch {
                    block,
                    tail,
                    returned,
                } => {
                    self.place(block)?;
                    self.effect(I::Runtime(R::EhCleanup(None)))?;
                    self.effect(I::Runtime(R::EhEnd))?;
                    if let Some(returned) = returned {
                        self.destroy_cleanup_in(returned, None, true)?;
                    }
                    self.jump(tail)?;
                }
            }
        }
        self.lexical.placing = saved;
        pending.clear();
        self.lexical.pending = pending;
        Ok(())
    }
    pub(super) fn place_result_cleanup_chains(&mut self) -> Result<(), Error> {
        if !self.lifetime_has_pending_chains() {
            return Ok(());
        }
        let mut lexical = std::mem::take(&mut self.lexical.pending);
        let result = self.place_cleanup_chains(false);
        lexical.append(&mut self.lexical.pending);
        self.lexical.pending = lexical;
        result
    }
}
