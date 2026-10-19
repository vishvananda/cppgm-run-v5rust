// Translated from lower/exception_lowering.cpp::eh_unwind_entry/region_landing. See NOTICE.
//! Cleanup suffixes are interned under the complete active handler stack and marker state.
use super::{
    Error,
    function::Function,
    handlers::{Context, ContextId},
    lifetime::Cleanup,
};
use crate::lowir::model::{BlockId, InstructionKind as I, RuntimeInstruction as R};
use std::collections::HashMap;

#[derive(Clone, Copy, Default)]
pub(super) struct ObjectMemo {
    stack: ContextId,
    blocks: [Option<BlockId>; 3],
}
impl ObjectMemo {
    pub(super) fn clear_unwind(&mut self) {
        self.blocks[0] = None;
        self.blocks[1] = None;
    }
}
#[derive(Clone, Copy)]
pub(super) enum Action {
    Entry {
        pops: u8,
        tail: BlockId,
    },
    Handler {
        marked: bool,
        tail: BlockId,
    },
    LexicalHandler {
        tail: BlockId,
    },
    Dispatch {
        context: Option<Context>,
        tail: BlockId,
    },
    LexicalDispatch {
        context: Option<Context>,
        tail: BlockId,
        returned: Option<Cleanup>,
    },
    Destroy {
        cleanup: Cleanup,
        tail: BlockId,
    },
    ReturnExit {
        index: usize,
        tail: BlockId,
    },
    ReturnObject {
        index: usize,
        cleanup: Cleanup,
        tail: BlockId,
    },
    Resume,
    Subobjects {
        pops: u8,
    },
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Marker {
    Popped,
    Pushed,
    Lexical,
}
impl Marker {
    fn index(self) -> usize {
        match self {
            Self::Popped => 0,
            Self::Pushed => 1,
            Self::Lexical => 2,
        }
    }
}
#[derive(Clone, Copy)]
enum Element {
    Object(usize, Marker),
    Handler(ContextId, Marker),
}
struct Snapshot {
    exception: ObjectMemo,
    lexical: Option<BlockId>,
}
struct SnapshotMark {
    first: usize,
    resume: super::lexical_cleanup::ResumeMemo,
}
#[derive(Default)]
pub(super) struct Chains {
    contexts: HashMap<(ContextId, ContextId, Marker), BlockId>,
    bodies: HashMap<(ContextId, Marker, BlockId), BlockId>,
    resumes: HashMap<(ContextId, bool), BlockId>,
    landings: HashMap<BlockId, BlockId>,
    snapshots: Vec<Snapshot>,
    marks: Vec<SnapshotMark>,
    missing: Vec<Element>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn save_exception_memos(&mut self) {
        self.handlers.unwind.marks.push(SnapshotMark {
            first: self.handlers.unwind.snapshots.len(),
            resume: self.lexical.resume,
        });
        self.handlers
            .unwind
            .snapshots
            .extend(self.lifetime.cleanups.iter().map(|cleanup| Snapshot {
                exception: cleanup.exception,
                lexical: cleanup.lexical,
            }));
    }
    pub(super) fn restore_exception_memos(&mut self) {
        if let Some(mark) = self.handlers.unwind.marks.pop() {
            self.lexical.resume = mark.resume;
            for (cleanup, memo) in self
                .lifetime
                .cleanups
                .iter_mut()
                .zip(&self.handlers.unwind.snapshots[mark.first..])
            {
                cleanup.exception = memo.exception;
                cleanup.lexical = memo.lexical;
            }
            self.handlers.unwind.snapshots.truncate(mark.first);
        }
    }
    fn context_node(&mut self, element: Element, stack: ContextId) -> Option<BlockId> {
        match element {
            Element::Object(index, marked) => {
                let memo = &mut self.lifetime.cleanups[index].exception;
                if memo.stack != stack {
                    *memo = ObjectMemo {
                        stack,
                        ..ObjectMemo::default()
                    };
                }
                memo.blocks[marked.index()]
            }
            Element::Handler(owner, marked) => self
                .handlers
                .unwind
                .contexts
                .get(&(stack, owner, marked))
                .copied(),
        }
    }
    fn context_entry(&mut self, count: usize, lexical: bool) -> Result<BlockId, Error> {
        let attempt = self.innermost_try();
        let floor = attempt.map_or(0, |c| c.floor);
        let first_handler = self.handlers.innermost.map_or(0, |index| index + 1);
        let stack = self
            .handlers
            .contexts
            .last()
            .ok_or(crate::sema::Error::Invariant("context chain without stack"))?
            .stack;
        let mut missing = std::mem::take(&mut self.handlers.unwind.missing);
        missing.clear();
        let mut marked = if lexical {
            Marker::Lexical
        } else {
            Marker::Pushed
        };
        let mut handler = self.handlers.contexts.len();
        let mut tail = None;
        for index in (floor..=count).rev() {
            if tail.is_some() {
                break;
            }
            if index < count {
                let element = Element::Object(index, marked);
                tail = self.context_node(element, stack);
                if tail.is_none() {
                    missing.push(element);
                }
            }
            while tail.is_none()
                && handler > first_handler
                && self.handlers.contexts[handler - 1].floor == index
            {
                let element = Element::Handler(self.handlers.contexts[handler - 1].stack, marked);
                tail = self.context_node(element, stack);
                if tail.is_none() {
                    missing.push(element);
                }
                marked = if lexical {
                    Marker::Lexical
                } else {
                    Marker::Popped
                };
                handler -= 1;
            }
        }
        let body = self.constructor_unwind_target();
        let mut tail = if let Some(tail) = tail {
            tail
        } else if let Some(body) = body {
            if let Some(&block) = self.handlers.unwind.bodies.get(&(stack, marked, body)) {
                block
            } else {
                let block = self.new_block(if lexical {
                    "lexical_unwind"
                } else {
                    "cleanup_resume"
                })?;
                self.queue_exception_block(
                    block,
                    Action::Entry {
                        pops: if lexical {
                            1
                        } else {
                            u8::from(marked == Marker::Pushed) + 1
                        },
                        tail: body,
                    },
                    lexical,
                );
                self.handlers
                    .unwind
                    .bodies
                    .insert((stack, marked, body), block);
                block
            }
        } else if let Some(attempt) = attempt {
            let key = (stack, attempt.stack, marked);
            if let Some(&block) = self.handlers.unwind.contexts.get(&key) {
                block
            } else {
                let block = self.new_block(if lexical {
                    "lexical_unwind"
                } else {
                    "cleanup_resume"
                })?;
                self.queue_exception_block(
                    block,
                    if !lexical && self.function_try_subobjects(self.handlers.innermost) {
                        Action::Subobjects {
                            pops: u8::from(marked == Marker::Pushed),
                        }
                    } else {
                        Action::Entry {
                            pops: if lexical {
                                1
                            } else {
                                u8::from(marked == Marker::Pushed) + 1
                            },
                            tail: attempt.entry,
                        }
                    },
                    lexical,
                );
                self.handlers.unwind.contexts.insert(key, block);
                block
            }
        } else if let Some(&block) = self.handlers.unwind.resumes.get(&(stack, lexical)) {
            block
        } else {
            let block = self.new_block(if lexical {
                "lexical_unwind"
            } else {
                "cleanup_resume"
            })?;
            self.queue_exception_block(block, Action::Resume, lexical);
            self.handlers.unwind.resumes.insert((stack, lexical), block);
            block
        };
        for &element in missing.iter().rev() {
            let (prefix, action) = match element {
                Element::Object(index, _) => {
                    let cleanup = self.lifetime.cleanups[index];
                    (
                        if cleanup.guard.is_some() {
                            "conditional_cleanup_dispatch"
                        } else if cleanup.member() {
                            "construction_cleanup"
                        } else {
                            "cleanup_action"
                        },
                        Action::Destroy { cleanup, tail },
                    )
                }
                Element::Handler(_, marked) => (
                    "cleanup_action",
                    if lexical {
                        Action::LexicalHandler { tail }
                    } else {
                        Action::Handler {
                            marked: marked == Marker::Pushed,
                            tail,
                        }
                    },
                ),
            };
            let block = self.new_block(if lexical { "lexical_unwind" } else { prefix })?;
            self.queue_exception_block(block, action, lexical);
            match element {
                Element::Object(index, marked) => {
                    self.lifetime.cleanups[index].exception.blocks[marked.index()] = Some(block);
                }
                Element::Handler(owner, marked) => {
                    self.handlers
                        .unwind
                        .contexts
                        .insert((stack, owner, marked), block);
                }
            }
            tail = block;
        }
        missing.clear();
        self.handlers.unwind.missing = missing;
        Ok(tail)
    }
    fn queue_exception_block(&mut self, block: BlockId, action: Action, lexical: bool) {
        if lexical {
            self.queue_lexical_context(block, action);
        } else {
            self.queue_context_action(block, action);
        }
    }
    pub(super) fn context_lexical_handler(
        &mut self,
        count: usize,
    ) -> Result<Option<BlockId>, Error> {
        let floor = self.handler_floor();
        if count <= floor
            && self.handlers.contexts.last().is_none_or(|c| c.attempt)
            && self.lexical.returned.is_none()
            && self.constructor_unwind_target().is_none()
        {
            return Ok(None);
        }
        let tail = self.context_entry(count, true)?;
        let block = self.new_block("lexical_cleanup_dispatch")?;
        self.queue_lexical_context(
            block,
            Action::LexicalDispatch {
                context: self.innermost_try(),
                tail,
                returned: self.lexical.returned,
            },
        );
        Ok(Some(block))
    }
    pub(super) fn context_landing(&mut self, count: usize) -> Result<BlockId, Error> {
        let entry = self.context_entry(count, false)?;
        if let Some(&block) = self.handlers.unwind.landings.get(&entry) {
            return Ok(block);
        }
        let block = self.new_block("call_unwind_dispatch")?;
        self.queue_context_action(
            block,
            Action::Dispatch {
                context: self.innermost_try(),
                tail: entry,
            },
        );
        self.handlers.unwind.landings.insert(entry, block);
        Ok(block)
    }
    pub(super) fn place_context_action(&mut self, action: Action) -> Result<(), Error> {
        match action {
            Action::Entry { pops, tail } => {
                for _ in 0..pops {
                    self.effect(I::Runtime(R::EhEnd))?;
                }
                self.jump(tail)
            }
            Action::Handler { marked, tail } => {
                if marked {
                    self.effect(I::Runtime(R::EhEnd))?;
                }
                self.call_end_catch()?;
                self.effect(I::Runtime(R::EhEnd))?;
                self.jump(tail)
            }
            Action::LexicalHandler { tail } => {
                self.effect(I::Runtime(R::EhEnd))?;
                self.call_end_catch()?;
                self.jump(tail)
            }
            Action::LexicalDispatch {
                context,
                tail,
                returned,
            } => {
                if let Some(context) = context {
                    self.emit_try_clauses(context)?;
                }
                self.effect(I::Runtime(R::EhCleanup(None)))?;
                self.effect(I::Runtime(R::EhEnd))?;
                if let Some(returned) = returned {
                    self.destroy_cleanup_in(returned, None, true)?;
                }
                self.jump(tail)
            }
            Action::Dispatch { context, tail } => {
                if let Some(context) = context {
                    self.emit_try_clauses(context)?;
                    self.effect(I::Runtime(R::EhCleanup(None)))?;
                }
                self.jump(tail)
            }
            Action::Destroy { cleanup, tail } => {
                self.destroy_cleanup_in(cleanup, None, true)?;
                self.jump(tail)
            }
            Action::ReturnExit { index, tail } => {
                self.exit_handler_context(index)?;
                self.jump(tail)
            }
            Action::ReturnObject {
                index,
                cleanup,
                tail,
            } => {
                let contexts = self
                    .handlers
                    .contexts
                    .iter()
                    .take_while(|c| c.floor <= index)
                    .count();
                let below =
                    self.with_handler_contexts(contexts, |this| this.lexical_handler(index))?;
                self.destroy_cleanup_in(cleanup, below, false)?;
                self.jump(tail)
            }
            Action::Resume => self.handler_resume(),
            Action::Subobjects { pops } => {
                for _ in 0..pops {
                    self.effect(I::Runtime(R::EhEnd))?;
                }
                self.effect(I::Runtime(R::Resume))?;
                self.terminated = true;
                Ok(())
            }
        }
    }
}
