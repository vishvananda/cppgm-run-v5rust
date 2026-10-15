// Translated from lower/object_lowering.cpp and cleanup_lowering.cpp. See NOTICE.
//! Completed automatic objects and interned cleanup continuations belong to one body.
use super::{Error, function::Function, types};
use crate::{
    lowir::{
        model::{BlockId, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::Type,
    },
    sema::lowering::{EntityId, TypeId, TypeView},
    syntax::ast,
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) struct Cleanup {
    pub place: CleanupPlace,
    pub ty: TypeId,
    pub variable: Option<EntityId>,
    unwind: Option<BlockId>,
    conditional: Option<BlockId>,
    shared_return: Option<BlockId>,
    pub lexical: Option<BlockId>,
    pub guard: Option<Value>,
    pub branched: bool,
    pub made_at: Option<Value>,
}
#[derive(Clone, Copy)]
pub(super) enum CleanupPlace {
    Storage(Value),
    Subobject(Value),
    Extended(Value),
    Temporary(Value),
    Counted { root: Value, counter: Value },
}
impl Cleanup {
    pub(super) fn member(self) -> bool {
        matches!(
            self.place,
            CleanupPlace::Subobject(_) | CleanupPlace::Counted { .. }
        )
    }
}
#[derive(Clone, Copy)]
struct ChainBlock {
    block: BlockId,
    action: Option<Cleanup>,
    tail: Option<BlockId>,
    unwinding: bool,
    constructor_tail: Option<BlockId>,
    below: Option<BlockId>,
}
#[derive(Clone, Copy)]
pub(super) struct LiveId(u32);
#[derive(Default)]
pub(super) enum ScanState {
    #[default]
    Pending,
    Complete,
}
pub(super) struct LiveNode {
    pub variable: EntityId,
    pub parent: Option<LiveId>,
}
#[derive(Clone, Copy, Default)]
pub(super) enum TemporarySegment {
    #[default]
    Empty,
    Delayed,
    Hidden,
    Eager,
    Nothrow,
}
impl TemporarySegment {
    pub(super) fn new(made: bool, eager: bool) -> Self {
        if !made {
            Self::Empty
        } else if eager {
            Self::Hidden
        } else {
            Self::Delayed
        }
    }
    pub(super) fn is_eager(self) -> bool {
        matches!(self, Self::Eager)
    }
    pub(super) fn is_nothrow(self) -> bool {
        matches!(self, Self::Nothrow)
    }
    pub(super) fn allows_eager_arguments(self) -> bool {
        matches!(self, Self::Hidden | Self::Eager)
    }
    pub(super) fn is_present(self) -> bool {
        !matches!(self, Self::Empty)
    }
}
#[derive(Clone, Copy, Default)]
pub(super) enum RegionStart {
    Pending,
    #[default]
    Consumed,
}
impl RegionStart {
    pub(super) fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }
}
#[derive(Default)]
pub(super) struct Lifetime {
    pub cleanups: Vec<Cleanup>,
    pending: Vec<ChainBlock>,
    late: Vec<ChainBlock>,
    resume: Option<BlockId>,
    conditional_resume: Option<BlockId>,
    return_terminal: Option<BlockId>,
    return_slot: Option<Value>,
    pub body: Option<ast::StmtId>,
    pub scan: ScanState,
    pub live: Vec<LiveNode>,
    pub returns: HashMap<EntityId, u32>,
    pub labels: HashMap<crate::lex::identifiers::NameId, Option<LiveId>>,
    pub depths: HashMap<EntityId, usize>,
    pub full_expression: bool,
    pub temporary_segment: TemporarySegment,
    pub expression_start: Option<crate::lowir::builder::Position>,
    pub construction: bool,
    pub region: bool,
    pub region_start: RegionStart,
    pub region_depth: usize,
    pub call_marks: Vec<CallMark>,
    pub declared_object: Option<Value>,
    pub guard_start: Option<crate::lowir::builder::Position>,
    pub function_guard: Option<BlockId>,
    pub exception_slot: Option<Value>,
    pub throw_work: Vec<ast::ExprId>,
    pub temporary_work: Vec<ast::ExprId>,
    pub temporary_queries: HashMap<ast::ExprId, super::temporary_scan::TemporarySummary>,
    pub guarded_operands: u32,
    pub tracking: super::tracked_lifetimes::Tracking,
}
#[derive(Clone, Copy)]
pub(super) struct CallMark {
    pub position: crate::lowir::builder::Position,
    pub declared: bool,
}
impl Lifetime {
    pub(super) fn add_live(
        &mut self,
        variable: EntityId,
        parent: Option<LiveId>,
    ) -> Result<LiveId, Error> {
        let id = LiveId(u32::try_from(self.live.len()).map_err(|_| crate::lowir::Error::Limit)?);
        self.live.push(LiveNode { variable, parent });
        Ok(id)
    }
    pub(super) fn live_node(&self, id: LiveId) -> &LiveNode {
        &self.live[id.0 as usize]
    }
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn lifetime_has_pending_chains(&self) -> bool {
        !self.lifetime.pending.is_empty()
    }
    pub(super) fn automatic_destructor(
        &mut self,
        entity: EntityId,
    ) -> Result<Option<EntityId>, Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        let Some(destructor) = self.symbols.storage_destructor(ty)? else {
            return Ok(None);
        };
        if !matches!(self.graph.lower_type(ty)?, TypeView::Array { .. })
            && self.symbols.destructor_inert(destructor)?
        {
            return Ok(None);
        }
        Ok(Some(destructor))
    }
    pub(super) fn register_local_destruction(&mut self, entity: EntityId) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        let Some(destructor) = self.symbols.storage_destructor(ty)? else {
            return Ok(());
        };
        if self.automatic_destructor(entity)?.is_none() {
            if self.returned_local != Some(entity) {
                self.symbols
                    .root_destructor(self.builder.program(), destructor)?;
            }
            return Ok(());
        }
        let storage =
            *self
                .locals
                .get(&entity)
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "cleanup without local storage",
                )))?;
        self.lifetime.cleanups.push(Cleanup {
            place: CleanupPlace::Storage(storage),
            ty,
            variable: Some(entity),
            unwind: None,
            conditional: None,
            shared_return: None,
            lexical: None,
            guard: None,
            branched: false,
            made_at: None,
        });
        self.lifetime
            .depths
            .insert(entity, self.lifetime.cleanups.len());
        Ok(())
    }
    pub(super) fn register_aggregate_member(&mut self, ty: TypeId, storage: Value) {
        self.lifetime.cleanups.push(Cleanup {
            place: CleanupPlace::Subobject(storage),
            ty,
            variable: None,
            unwind: None,
            conditional: None,
            shared_return: None,
            lexical: None,
            guard: None,
            branched: false,
            made_at: None,
        });
    }
    pub(super) fn register_extended_temporary(
        &mut self,
        ty: TypeId,
        object: Value,
        owner: EntityId,
    ) {
        self.lifetime.cleanups.push(Cleanup {
            place: CleanupPlace::Extended(object),
            ty,
            variable: Some(owner),
            unwind: None,
            conditional: None,
            shared_return: None,
            lexical: None,
            guard: None,
            branched: false,
            made_at: None,
        });
        self.lifetime
            .depths
            .insert(owner, self.lifetime.cleanups.len());
    }
    pub(super) fn register_temporary(&mut self, ty: TypeId, object: Value) {
        self.lifetime.cleanups.push(Cleanup {
            place: CleanupPlace::Temporary(object),
            ty,
            variable: None,
            unwind: None,
            conditional: None,
            shared_return: None,
            lexical: None,
            guard: None,
            branched: false,
            made_at: None,
        });
    }
    pub(super) fn extend_last_temporary(&mut self, owner: EntityId) -> Result<(), Error> {
        let cleanup = self
            .lifetime
            .cleanups
            .last_mut()
            .ok_or(crate::sema::Error::Invariant(
                "reference conversion without temporary",
            ))?;
        let CleanupPlace::Temporary(object) = cleanup.place else {
            return Err(crate::sema::Error::Invariant(
                "reference conversion extends non-temporary",
            )
            .into());
        };
        cleanup.place = CleanupPlace::Extended(object);
        cleanup.variable = Some(owner);
        self.lifetime
            .depths
            .insert(owner, self.lifetime.cleanups.len());
        Ok(())
    }
    pub(super) fn destroy_temporaries(&mut self, first: usize) -> Result<(), Error> {
        self.destroy_temporaries_from(first, true)
    }
    pub(super) fn destroy_temporaries_from(
        &mut self,
        first: usize,
        pop: bool,
    ) -> Result<(), Error> {
        self.destroy_temporaries_with_completion(first, pop, None)
    }
    pub(super) fn destroy_local_temporaries(
        &mut self,
        first: usize,
        entity: EntityId,
    ) -> Result<(), Error> {
        let completed =
            if self.graph.lower_automatic(entity) && self.automatic_destructor(entity)?.is_some() {
                let storage = *self
                    .locals
                    .get(&entity)
                    .ok_or(crate::sema::Error::Invariant(
                        "completed object without storage",
                    ))?;
                Some(Cleanup {
                    place: CleanupPlace::Storage(storage),
                    ty: self.graph.lower_entity_type(entity)?,
                    variable: Some(entity),
                    unwind: None,
                    conditional: None,
                    shared_return: None,
                    lexical: None,
                    guard: None,
                    branched: false,
                    made_at: None,
                })
            } else {
                None
            };
        self.destroy_temporaries_with_completion(first, true, completed)
    }
    fn destroy_temporaries_with_completion(
        &mut self,
        first: usize,
        pop: bool,
        completed: Option<Cleanup>,
    ) -> Result<(), Error> {
        if first <= self.lifetime.tracking.floor {
            self.finish_tracking()?;
        }
        let mut throwing = false;
        let mut guarded = false;
        let guarded_region = self.lifetime.region;
        for index in first..self.lifetime.cleanups.len() {
            let cleanup = self.lifetime.cleanups[index];
            if matches!(cleanup.place, CleanupPlace::Temporary(_)) {
                throwing |= self.destructor_throws(cleanup.ty)?;
                guarded |= cleanup.guard.is_some();
            }
        }
        if throwing {
            self.close_lifetime_region()?;
        }
        let full_expression = self.lifetime.full_expression;
        if throwing {
            self.lifetime.full_expression = false;
        }
        let pending = self.lifetime.pending.len();
        for index in (first..self.lifetime.cleanups.len()).rev() {
            let cleanup = self.lifetime.cleanups[index];
            if matches!(cleanup.place, CleanupPlace::Temporary(_)) {
                let destroys_completed = self.destructor_throws(cleanup.ty)?;
                let below = if throwing && destroys_completed && (index != 0 || completed.is_some())
                {
                    let tail = self.cleanup_prefix(true, index)?;
                    if let Some(completed) = completed {
                        let block = self.new_block("cleanup_action")?;
                        self.lifetime.pending.push(ChainBlock {
                            block,
                            action: Some(completed),
                            tail: Some(tail),
                            unwinding: true,
                            constructor_tail: self.constructor_unwind_target(),
                            below: None,
                        });
                        Some(block)
                    } else {
                        Some(tail)
                    }
                } else {
                    None
                };
                self.destroy_cleanup_in(cleanup, below, false)?;
            }
        }
        self.lifetime.full_expression = full_expression;
        if self.lifetime.pending.len() > pending
            || guarded_region && guarded && !throwing && !self.lifetime.tracking.active
        {
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            self.place_cleanup_chains(false)?;
            self.place(next)?;
        }
        if !pop {
            return Ok(());
        }
        let mut kept = first;
        for index in first..self.lifetime.cleanups.len() {
            let mut cleanup = self.lifetime.cleanups[index];
            if matches!(cleanup.place, CleanupPlace::Temporary(_)) {
                continue;
            }
            if kept != index {
                cleanup.unwind = None;
                cleanup.conditional = None;
                cleanup.shared_return = None;
            }
            self.lifetime.cleanups[kept] = cleanup;
            if let Some(variable) = cleanup.variable {
                self.lifetime.depths.insert(variable, kept + 1);
            }
            kept += 1;
        }
        self.lifetime.cleanups.truncate(kept);
        Ok(())
    }
    pub(super) fn release_aggregate_members(&mut self, first: usize) {
        let mut kept = first;
        let mut removed = false;
        for index in first..self.lifetime.cleanups.len() {
            let mut cleanup = self.lifetime.cleanups[index];
            if cleanup.member() {
                removed = true;
                continue;
            }
            // A retained temporary's old tail may contain a member now owned
            // by the completed aggregate. Keep that handler for earlier calls;
            // later calls need a chain from the new lexical ownership alone.
            if removed {
                cleanup.unwind = None;
                cleanup.conditional = None;
                cleanup.shared_return = None;
            }
            self.lifetime.cleanups[kept] = cleanup;
            kept += 1;
        }
        self.lifetime.cleanups.truncate(kept);
        for (index, cleanup) in self.lifetime.cleanups.iter().enumerate().skip(first) {
            if let Some(variable) = cleanup.variable {
                self.lifetime.depths.insert(variable, index + 1);
            }
        }
    }
    pub(super) fn leave_scope(&mut self, depth: usize) -> Result<(), Error> {
        if !self.terminated {
            self.destroy_cleanups(depth)?;
        }
        self.lifetime.cleanups.truncate(depth);
        Ok(())
    }
    pub(super) fn scoped_statement(&mut self, body: ast::StmtId) -> Result<(), Error> {
        let depth = self.lifetime.cleanups.len();
        self.statement(body)?;
        self.leave_scope(depth)
    }
    pub(super) fn destroy_cleanups(&mut self, depth: usize) -> Result<(), Error> {
        for index in (depth..self.lifetime.cleanups.len()).rev() {
            let cleanup = self.lifetime.cleanups[index];
            let below = self.lexical_handler(index)?;
            self.destroy_cleanup_in(cleanup, below, false)?;
        }
        Ok(())
    }
    pub(super) fn destroy_cleanup_in(
        &mut self,
        cleanup: Cleanup,
        below: Option<BlockId>,
        unwinding: bool,
    ) -> Result<(), Error> {
        let terminate = unwinding && self.destructor_throws(cleanup.ty)?;
        let region = if terminate {
            Some(self.cleanup_unwind_guard()?)
        } else {
            below
        };
        if let Some(region) = region {
            self.effect(I::Runtime(R::EhTry(region)))?;
        }
        let done = if let Some(flag) = cleanup.guard {
            let destroy = self.new_block("conditional_temporary_destroy")?;
            let done = self.new_block("conditional_temporary_done")?;
            let value = self
                .emit(I::Load(Facts::default(), Type::U8, flag), cleanup.ty)?
                .value;
            // A tracked segment covers both the test and destruction. An
            // ordinary guarded cleanup ends its segment before branching.
            if self.lifetime.tracking.active {
                self.builder.effect(I::Branch(value, destroy, done))?;
            } else {
                self.effect(I::Branch(value, destroy, done))?;
            }
            self.terminated = true;
            self.place(destroy)?;
            let zero = self.builder.integer(0)?;
            self.effect(I::Store(Facts::default(), Type::U8, zero, flag))?;
            Some(done)
        } else {
            None
        };
        let result = match cleanup.place {
            CleanupPlace::Storage(storage) => {
                self.destroy_storage_guarded(cleanup.ty, storage, !unwinding)
            }
            CleanupPlace::Subobject(object)
            | CleanupPlace::Extended(object)
            | CleanupPlace::Temporary(object) => {
                let object = self.storage_address(object, cleanup.ty)?;
                self.destroy_completed_object(cleanup.ty, object)
            }
            CleanupPlace::Counted { root, counter } => {
                self.destroy_counted_elements(cleanup.ty, root, counter)
            }
        };
        result?;
        if let Some(done) = done {
            if self.lifetime.tracking.active {
                self.builder.effect(I::Jump(done))?;
                self.terminated = true;
            } else {
                self.jump(done)?;
            }
            self.place(done)?;
        }
        if region.is_some() {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        Ok(())
    }
    fn destroy_completed_object(&mut self, ty: TypeId, object: Value) -> Result<(), Error> {
        if let TypeView::Array { element, bound } = self.graph.lower_type(ty)? {
            if bound > 8 && !matches!(self.graph.lower_type(element)?, TypeView::Array { .. }) {
                return self.destroy_array_loop(element, object, bound);
            }
            let size = types::layout(self.graph, element)?.0;
            for index in (0..bound).rev() {
                let address = self.element_address(object, index, size, element)?;
                self.destroy_object(element, address)?;
            }
            Ok(())
        } else {
            self.call_destructor(ty, object)
        }
    }
    pub(super) fn call_destructor(&mut self, ty: TypeId, object: Value) -> Result<(), Error> {
        let Some(destructor) = self.symbols.class_destructor(ty)? else {
            return Ok(());
        };
        self.symbols.demand(destructor);
        let callee = self.symbols.callee(destructor, &mut self.builder)?;
        let arguments = self.builder.arguments(&[object])?;
        self.effect(I::Call(crate::lowir::model::Call {
            result: Type::Void,
            callee,
            arguments,
            signature: None,
            copy_elision: false,
        }))
    }
    pub(super) fn destroy_object(&mut self, ty: TypeId, object: Value) -> Result<(), Error> {
        self.check_stack()?;
        if let TypeView::Array { element, bound } = self.graph.lower_type(ty)? {
            let size = types::layout(self.graph, element)?.0;
            for index in (0..bound).rev() {
                let offset = self.builder.integer(u128::from(index.wrapping_mul(size)))?;
                let address = self
                    .emit(
                        I::Index(
                            Type::I8,
                            Some(crate::lowir::vocabulary::Projection::ArrayElement),
                            object,
                            offset,
                        ),
                        element,
                    )?
                    .value;
                self.destroy_object(element, address)?;
            }
            Ok(())
        } else {
            self.call_destructor(ty, object)
        }
    }
    fn destroy_storage_guarded(
        &mut self,
        ty: TypeId,
        storage: Value,
        guarded: bool,
    ) -> Result<(), Error> {
        if let TypeView::Array { element, bound } = self.graph.lower_type(ty)? {
            if guarded && self.destructor_throws(ty)? {
                let mut element = ty;
                let mut count = 1_u64;
                while let TypeView::Array {
                    element: child,
                    bound,
                } = self.graph.lower_type(element)?
                {
                    count = count.wrapping_mul(bound);
                    element = child;
                }
                let base = self.emit(I::Addr(storage), ty)?.value;
                return self.destroy_member_array_guarded(element, base, count);
            }
            if bound > 8 && !matches!(self.graph.lower_type(element)?, TypeView::Array { .. }) {
                let base = self.emit(I::Addr(storage), ty)?.value;
                return self.destroy_array_loop(element, base, bound);
            }
            let size = types::layout(self.graph, element)?.0;
            for index in (0..bound).rev() {
                let base = self.emit(I::Addr(storage), ty)?.value;
                let address = self.element_address(base, index, size, element)?;
                self.destroy_object(element, address)?;
            }
            Ok(())
        } else {
            let object = self.storage_address(storage, ty)?;
            self.call_destructor(ty, object)
        }
    }
    pub(super) fn element_address(
        &mut self,
        base: Value,
        index: u64,
        size: u64,
        ty: TypeId,
    ) -> Result<Value, Error> {
        use crate::lowir::vocabulary::{Binary, Projection};
        let index = self.builder.integer(u128::from(index))?;
        let offset = if size == 1 {
            index
        } else {
            let size = self.builder.integer(u128::from(size))?;
            self.emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                ty,
            )?
            .value
        };
        Ok(self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                ty,
            )?
            .value)
    }
    pub(super) fn destroy_array_loop(
        &mut self,
        element: TypeId,
        base: Value,
        count: u64,
    ) -> Result<(), Error> {
        use crate::lowir::vocabulary::{Binary, Compare, Projection};
        let size = types::layout(self.graph, element)?.0;
        let slot = self.generated_slot("destructor_array_index", Type::I64)?;
        let count = self.builder.integer(u128::from(count))?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, count, slot))?;
        let test = self.new_block("destructor_array_cond")?;
        let body = self.new_block("destructor_array_body")?;
        let end = self.new_block("destructor_array_end")?;
        self.jump(test)?;
        self.place(test)?;
        let current = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let zero = self.builder.integer(0)?;
        let condition = self
            .emit(I::Compare(Compare::Ne, Type::I64, current, zero), element)?
            .value;
        self.effect(I::Branch(condition, body, end))?;
        self.place(body)?;
        let one = self.builder.integer(1)?;
        let last = self
            .emit(
                I::Binary(Binary::Sub, Facts::default(), Type::I64, current, one),
                element,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, last, slot))?;
        let offset = if size == 1 {
            last
        } else {
            let size = self.builder.integer(u128::from(size))?;
            self.emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, last, size),
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
        self.call_destructor(element, object)?;
        self.jump(test)?;
        self.place(end)
    }
    fn cleanup_entry(&mut self, unwinding: bool) -> Result<BlockId, Error> {
        self.cleanup_prefix(unwinding, self.lifetime.cleanups.len())
    }
    pub(super) fn cleanup_prefix(
        &mut self,
        unwinding: bool,
        count: usize,
    ) -> Result<BlockId, Error> {
        let late = unwinding && (count > 8 || self.lifetime.tracking.active);
        let terminal = if !unwinding {
            self.lifetime.return_terminal
        } else if late {
            self.lifetime.conditional_resume
        } else {
            self.lifetime.resume
        };
        let terminal = if let Some(terminal) = terminal {
            terminal
        } else {
            let block = self.new_block(if !unwinding {
                "return_cleanup_terminal"
            } else if late {
                "conditional_cleanup_resume"
            } else {
                "cleanup_resume"
            })?;
            let record = ChainBlock {
                block,
                action: None,
                tail: None,
                unwinding,
                constructor_tail: self.constructor_unwind_target(),
                below: None,
            };
            if !unwinding {
                self.lifetime.return_terminal = Some(block);
            } else if late {
                self.lifetime.conditional_resume = Some(block);
            } else {
                self.lifetime.resume = Some(block);
            }
            if late {
                self.lifetime.late.push(record);
            } else {
                self.lifetime.pending.push(record);
            }
            block
        };
        let mut first = count;
        while first > 0
            && if late {
                self.lifetime.cleanups[first - 1].conditional.is_none()
            } else if unwinding {
                self.lifetime.cleanups[first - 1].unwind.is_none()
            } else {
                self.lifetime.cleanups[first - 1].shared_return.is_none()
            }
        {
            first -= 1;
        }
        let mut tail = if first == 0 {
            Some(terminal)
        } else if late {
            self.lifetime.cleanups[first - 1].conditional
        } else if unwinding {
            self.lifetime.cleanups[first - 1].unwind
        } else {
            self.lifetime.cleanups[first - 1].shared_return
        }
        .ok_or(Error::Semantic(crate::sema::Error::Invariant(
            "cleanup chain tail",
        )))?;
        for index in first..count {
            let block = self.new_block(if !unwinding {
                "return_cleanup_action"
            } else if late || self.lifetime.cleanups[index].guard.is_some() {
                "conditional_cleanup_dispatch"
            } else if self.lifetime.cleanups[index].member() {
                "construction_cleanup"
            } else {
                "cleanup_action"
            })?;
            let below = if !unwinding
                && index != 0
                && self.destructor_throws(self.lifetime.cleanups[index].ty)?
            {
                Some(self.cleanup_prefix(true, index)?)
            } else {
                None
            };
            let record = ChainBlock {
                block,
                action: Some(self.lifetime.cleanups[index]),
                tail: Some(tail),
                unwinding,
                constructor_tail: self.constructor_unwind_target(),
                below,
            };
            if late {
                self.lifetime.late.push(record);
            } else {
                self.lifetime.pending.push(record);
            }
            if late {
                self.lifetime.cleanups[index].conditional = Some(block);
            } else if unwinding {
                self.lifetime.cleanups[index].unwind = Some(block);
            } else {
                self.lifetime.cleanups[index].shared_return = Some(block);
            }
            tail = block;
        }
        Ok(tail)
    }
    pub(super) fn constructor_unwind_changed(&mut self) {
        self.lifetime.resume = None;
        self.lifetime.conditional_resume = None;
        for cleanup in &mut self.lifetime.cleanups {
            cleanup.unwind = None;
            cleanup.conditional = None;
            cleanup.lexical = None;
        }
    }
    pub(super) fn unwind_entry(&mut self) -> Result<BlockId, Error> {
        self.cleanup_entry(true)
    }
    pub(super) fn cleanup_return(
        &mut self,
        ty: Type,
        value: Option<Value>,
        expression: bool,
    ) -> Result<(), Error> {
        self.destroy_temporaries(0)?;
        if self.destruction.cleanup.is_some() {
            self.end_lifetime_expression(expression)?;
            self.destroy_cleanups(0)?;
            self.effect(I::Runtime(R::EhEnd))?;
            let target = if let Some(target) = self.destruction.epilogue {
                target
            } else {
                let target = self.new_block("destructor_return_epilogue")?;
                self.destruction.epilogue = Some(target);
                target
            };
            return self.jump(target);
        }
        self.scan_lifetimes()?;
        let shared = self.result_destination.is_none()
            && self.lifetime.cleanups.last().is_some_and(|cleanup| {
                cleanup
                    .variable
                    .and_then(|variable| self.lifetime.returns.get(&variable))
                    .is_some_and(|&count| count > 1)
            });
        if !shared {
            self.end_lifetime_expression(expression)?;
            for index in (0..self.lifetime.cleanups.len()).rev() {
                let cleanup = self.lifetime.cleanups[index];
                // The elided result transfers to the caller only on return.
                // Its unwind and other scope-exit actions remain intact.
                if cleanup.variable.is_none()
                    || cleanup.variable != self.returned_local
                    || !matches!(cleanup.place, CleanupPlace::Storage(_))
                {
                    let below = self.lexical_handler(index)?;
                    self.destroy_cleanup_in(cleanup, below, false)?;
                }
            }
            self.effect(I::Return(ty, value))?;
            self.terminated = true;
            return Ok(());
        }
        if let Some(value) = value.filter(|value| Some(*value) != self.result_storage) {
            let slot = if let Some(slot) = self.lifetime.return_slot {
                slot
            } else {
                let slot = self.generated_slot("cleanup_return", ty)?;
                self.lifetime.return_slot = Some(slot);
                slot
            };
            self.effect(I::Store(
                Facts::memory(false, !matches!(ty, Type::I8 | Type::U8)),
                ty,
                value,
                slot,
            ))?;
        } else if value.is_some() {
            // A class return already preserved its bytes in the shared retobj.
            self.lifetime.return_slot = value;
        }
        self.end_lifetime_expression(expression)?;
        let entry = self.cleanup_entry(false)?;
        self.jump(entry)?;
        self.place_cleanup_chains(false)
    }
    pub(super) fn place_cleanup_chains(&mut self, late: bool) -> Result<(), Error> {
        let mut blocks = if late {
            std::mem::take(&mut self.lifetime.late)
        } else {
            std::mem::take(&mut self.lifetime.pending)
        };
        self.place_chain_blocks(&blocks)?;
        blocks.clear();
        if late {
            self.lifetime.late = blocks;
        } else {
            self.lifetime.pending = blocks;
        }
        Ok(())
    }
    fn place_chain_blocks(&mut self, blocks: &[ChainBlock]) -> Result<(), Error> {
        for &block in blocks {
            self.place(block.block)?;
            if let Some(action) = block.action {
                let tail = block
                    .tail
                    .ok_or(crate::sema::Error::Invariant("cleanup without tail"))?;
                self.destroy_cleanup_in(action, block.below, block.unwinding)?;
                self.jump(tail)?;
            } else if block.unwinding {
                if let Some(tail) = block.constructor_tail {
                    self.effect(I::Runtime(R::EhEnd))?;
                    self.effect(I::Runtime(R::EhEnd))?;
                    self.jump(tail)?;
                } else if self.lifetime.function_guard.is_some() {
                    self.terminate_unwinding()?;
                } else {
                    self.effect(I::Runtime(R::Resume))?;
                    self.terminated = true;
                }
            } else {
                let ty = types::storage(
                    self.graph,
                    self.result
                        .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                            "shared synthetic return",
                        )))?,
                )?;
                self.effect(I::Return(ty, self.lifetime.return_slot))?;
                self.terminated = true;
            }
        }
        Ok(())
    }
    pub(super) fn end_tracked_cleanups(&mut self) {
        for cleanup in self
            .lifetime
            .cleanups
            .iter_mut()
            .skip(self.lifetime.tracking.floor)
        {
            if cleanup.guard.is_some() && !matches!(cleanup.place, CleanupPlace::Temporary(_)) {
                if !cleanup.branched {
                    cleanup.guard = None;
                    if let (CleanupPlace::Extended(_), Some(object)) =
                        (cleanup.place, cleanup.made_at)
                    {
                        cleanup.place = CleanupPlace::Extended(object);
                    }
                }
                cleanup.unwind = None;
                cleanup.conditional = None;
                cleanup.shared_return = None;
            }
        }
    }
    pub(super) fn place_tracked_chain(&mut self) -> Result<(), Error> {
        let before = self.builder.layout_len();
        let saved = self.builder.position()?;
        let terminated = self.terminated;
        let region = std::mem::replace(&mut self.lifetime.region, false);
        let full = std::mem::replace(&mut self.lifetime.full_expression, false);
        let pending = std::mem::take(&mut self.lifetime.pending);
        let mark = self.lifetime.late.len();
        let head = self.unwind_entry()?;
        let blocks = self.lifetime.late.split_off(mark);
        self.place_chain_blocks(&blocks)?;
        self.lifetime.pending = pending;
        self.lifetime.full_expression = full;
        self.lifetime.region = region;
        self.terminated = terminated;
        let start = self
            .lifetime
            .tracking
            .start
            .ok_or(crate::sema::Error::Invariant("tracking without start"))?;
        self.builder.restore_after_cleanup(saved, start, before)?;
        for marker in self.lifetime.tracking.markers.drain(..) {
            self.builder.retarget_region(marker, head)?;
        }
        self.lifetime.tracking.head = Some(head);
        Ok(())
    }
}
