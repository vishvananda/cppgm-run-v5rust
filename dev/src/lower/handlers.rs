// Translated from lower/exception_lowering.cpp. See NOTICE.
//! Source handlers retain syntax identity; contexts and selectors live for one function.
use super::{Error, function::Function, types};
use crate::{
    lowir::{
        model::{BlockId, Call, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Compare, Role, Type},
    },
    sema::lowering::{TypeId, TypeView},
    syntax::{ast, storage::List},
};
use std::collections::HashMap;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(super) struct ContextId(u32);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CatchRetirement {
    Scalar,
    MayDestroy,
}
#[derive(Clone, Copy)]
pub(super) struct Context {
    pub clauses: List<ast::Handler>,
    pub key: ast::StmtId,
    pub floor: usize,
    pub dispatch: BlockId,
    pub entry: BlockId,
    pub end: BlockId,
    pub attempt: bool,
    pub stack: ContextId,
    pub parent_try: Option<usize>,
    pub chains: (usize, usize),
    pub unwinds: bool,
    pub end_throws: bool,
    pub retirement: CatchRetirement,
}
#[derive(Default)]
pub(super) struct Handlers {
    pub function_try: Option<super::definition_regions::FunctionTry>,
    pub contexts: Vec<Context>,
    pub innermost: Option<usize>,
    selectors: HashMap<ast::StmtId, u32>,
    pub labels: HashMap<crate::lex::identifiers::NameId, usize>,
    pub returns: HashMap<(crate::sema::lowering::EntityId, ast::StmtId), u32>,
    pub return_exits: HashMap<ContextId, BlockId>,
    pub next_selector: u32,
    next_stack: u32,
    chains: Vec<List<ast::Handler>>,
    pub explicit: usize,
    pub unwind: super::exception_chains::Chains,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn with_explicit_region<T>(
        &mut self,
        active: bool,
        operation: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let explicit = self.handlers.explicit;
        self.handlers.explicit += usize::from(active);
        let result = operation(self);
        self.handlers.explicit = explicit;
        result
    }
    pub(super) fn next_context_stack(&mut self) -> Result<ContextId, Error> {
        self.handlers.next_stack = self
            .handlers
            .next_stack
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        Ok(ContextId(self.handlers.next_stack))
    }
    fn handler_selector(&mut self, body: ast::StmtId) -> Result<u32, Error> {
        if let Some(&selector) = self.handlers.selectors.get(&body) {
            return Ok(selector);
        }
        let selector = self.next_handler_selector()?;
        self.handlers.selectors.insert(body, selector);
        Ok(selector)
    }
    pub(super) fn next_handler_selector(&mut self) -> Result<u32, Error> {
        self.handlers.next_selector = self
            .handlers
            .next_selector
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        Ok(self.handlers.next_selector)
    }
    pub(super) fn innermost_try(&self) -> Option<Context> {
        self.handlers
            .innermost
            .map(|index| self.handlers.contexts[index])
    }
    /// An explicit landing forwards clauses only when the source try is its
    /// immediate enclosing region. Otherwise it resumes into the live chain.
    pub(super) fn landing_try(&self) -> Option<Context> {
        if self.lifetime.region {
            return None;
        }
        self.handlers.contexts.last().copied().filter(|c| c.attempt)
    }
    pub(super) fn landing_dispatch(&mut self, attempt: Context) -> Result<(), Error> {
        self.emit_try_clauses(attempt)?;
        self.effect(I::Runtime(R::EhCleanup(None)))
    }
    pub(super) fn landing_exit(&mut self, attempt: Context) -> Result<(), Error> {
        self.effect(I::Runtime(R::EhEnd))?;
        if self.function_try_subobjects(self.handlers.innermost) {
            self.effect(I::Runtime(R::Resume))?;
            self.terminated = true;
            return Ok(());
        }
        self.effect(I::Runtime(R::EhEnd))?;
        self.jump(attempt.entry)
    }
    pub(super) fn handler_floor(&self) -> usize {
        self.innermost_try().map_or(0, |c| c.floor)
    }
    fn catches_all(&self, clauses: List<ast::Handler>) -> bool {
        self.tree
            .handlers
            .get(clauses)
            .iter()
            .any(|h| matches!(h.exception, ast::ExceptionDeclaration::Ellipsis))
    }
    fn emit_clauses(&mut self, clauses: List<ast::Handler>) -> Result<bool, Error> {
        for handler in self.tree.handlers.get(clauses) {
            let selector = Some(i64::from(self.handler_selector(handler.body)?));
            if matches!(handler.exception, ast::ExceptionDeclaration::Ellipsis) {
                self.effect(I::Runtime(R::EhCatchAll(selector)))?;
                return Ok(true);
            }
            let (ty, _) = self.graph.lower_handler(handler.body)?;
            let ty = match self.graph.lower_type(ty)? {
                TypeView::Reference { element, .. } => element,
                _ => ty,
            };
            let symbol = self
                .symbols
                .exception_type_information(self.builder.program(), ty)?;
            self.effect(I::Runtime(R::EhCatch(symbol, selector)))?;
        }
        Ok(false)
    }
    pub(super) fn emit_try_clauses(&mut self, context: Context) -> Result<(), Error> {
        if self.emit_clauses(context.clauses)? {
            return Ok(());
        }
        for index in context.chains.0..context.chains.0 + context.chains.1 {
            if self.emit_clauses(self.handlers.chains[index])? {
                break;
            }
        }
        Ok(())
    }
    pub(super) fn begin_try(
        &mut self,
        key: ast::StmtId,
        clauses: List<ast::Handler>,
    ) -> Result<Context, Error> {
        let floor = self.lifetime.cleanups.len();
        let dispatch = self.new_block("catch_dispatch")?;
        let entry = self.new_block("catch_entry")?;
        let end = self.new_block("try_end")?;
        let stack = self.next_context_stack()?;
        let first = self.handlers.chains.len();
        if !self.catches_all(clauses) {
            for index in (0..self.handlers.contexts.len()).rev() {
                let outer = self.handlers.contexts[index];
                if !outer.attempt {
                    continue;
                }
                self.handlers.chains.push(outer.clauses);
                if self.catches_all(outer.clauses) {
                    break;
                }
            }
        }
        let context = Context {
            clauses,
            key,
            floor,
            dispatch,
            entry,
            end,
            attempt: true,
            stack,
            chains: (first, self.handlers.chains.len() - first),
            parent_try: self.handlers.innermost,
            end_throws: false,
            retirement: CatchRetirement::Scalar,
            unwinds: floor > self.handler_floor()
                || self.handlers.contexts.last().is_some_and(|c| !c.attempt)
                || self.detached_body_target().is_some(),
        };
        self.effect(I::Runtime(R::EhTry(dispatch)))?;
        self.save_exception_memos();
        self.handlers.contexts.push(context);
        self.handlers.innermost = Some(self.handlers.contexts.len() - 1);
        Ok(context)
    }
    pub(super) fn try_statement(
        &mut self,
        body: ast::StmtId,
        clauses: List<ast::Handler>,
    ) -> Result<(), Error> {
        let context = self.begin_try(body, clauses)?;
        self.statement(body)?;
        self.end_try(context)
    }
    pub(super) fn end_try(&mut self, context: Context) -> Result<(), Error> {
        if !self.terminated {
            self.effect(I::Runtime(R::EhEnd))?;
            self.jump(context.end)?;
        }
        self.handlers.contexts.pop();
        self.handlers.innermost = context.parent_try;
        self.restore_exception_memos();
        self.place(context.dispatch)?;
        self.emit_try_clauses(context)?;
        if context.unwinds && !self.catches_all(context.clauses) {
            self.effect(I::Runtime(R::EhCleanup(None)))?;
            self.effect(I::Runtime(R::EhEnd))?;
        }
        self.jump(context.entry)?;
        self.place(context.entry)?;
        for &handler in self.tree.handlers.get(context.clauses) {
            let source = self
                .result
                .ok_or(crate::sema::Error::Invariant("handler without function"))?;
            let exception = self
                .emit(I::Runtime(R::Exception(Type::Ptr)), source)?
                .value;
            let selector = self
                .emit(I::Runtime(R::ExceptionSelector(Type::I32)), source)?
                .value;
            let body = self.new_block("catch_body")?;
            let next = self.new_block("catch_next")?;
            let landing = self.new_block("catch_cleanup")?;
            let number = self.handler_selector(handler.body)?;
            let number = self.builder.integer(u128::from(number))?;
            let test = self
                .emit(I::Compare(Compare::Eq, Type::I32, selector, number), source)?
                .value;
            self.effect(I::Branch(test, body, next))?;
            self.terminated = true;
            self.place(body)?;
            self.handler(handler, exception, landing, context)?;
            self.place(landing)?;
            if let Some(outer) = self.innermost_try() {
                self.emit_try_clauses(outer)?;
            }
            if self.detached_body_target().is_some() {
                self.effect(I::Runtime(R::EhCleanup(None)))?;
            }
            self.call_end_catch(context.retirement == CatchRetirement::MayDestroy)?;
            self.effect(I::Runtime(R::EhEnd))?;
            self.leave_handler_contexts()?;
            self.place(next)?;
        }
        if self.catches_all(context.clauses) {
            for _ in 0..self.handlers.contexts.len() {
                self.effect(I::Runtime(R::EhEnd))?;
            }
            self.handler_resume()?;
        } else {
            self.leave_handler_contexts()?;
        }
        self.place(context.end)
    }
    fn handler(
        &mut self,
        written: ast::Handler,
        exception: Value,
        landing: BlockId,
        attempt: Context,
    ) -> Result<(), Error> {
        let source = self
            .result
            .ok_or(crate::sema::Error::Invariant("handler without result"))?;
        let caught = self.catch_runtime(Role::EhBeginCatch, Type::Ptr, &[exception], source)?;
        let declaration = if matches!(written.exception, ast::ExceptionDeclaration::Ellipsis) {
            None
        } else {
            Some(self.graph.lower_handler(written.body)?)
        };
        let record = if declaration.is_some_and(|(_, entity)| entity.is_some()) {
            let record = self.generated_slot("catch", Type::Ptr)?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                caught,
                record,
            ))?;
            Some(record)
        } else {
            None
        };
        self.effect(I::Runtime(R::EhCleanup(Some(landing))))?;
        let stack = self.next_context_stack()?;
        let caught_type = if let Some((ty, _)) = declaration {
            Some(match self.graph.lower_type(ty)? {
                TypeView::Reference { element, .. } => element,
                _ => ty,
            })
        } else {
            None
        };
        let end_throws = caught_type.map_or(Ok(false), |ty| self.destructor_throws(ty))?;
        let retirement = if caught_type
            .map(|ty| self.graph.lower_type(ty))
            .transpose()?
            .is_none_or(|ty| matches!(ty, TypeView::Class(_)))
        {
            CatchRetirement::MayDestroy
        } else {
            CatchRetirement::Scalar
        };
        let context = Context {
            end_throws,
            retirement,
            key: written.body,
            attempt: false,
            floor: self.lifetime.cleanups.len(),
            stack,
            ..attempt
        };
        self.save_exception_memos();
        self.handlers.contexts.push(context);
        if let Some((ty, entity)) = declaration {
            let object = match self.graph.lower_type(ty)? {
                TypeView::Reference { element, .. } => element,
                _ => ty,
            };
            self.symbols.caught_type(self.builder.program(), object)?;
            if let Some(entity) = entity {
                self.catch_variable(entity, ty, caught, record)?;
            }
        }
        self.statement(written.body)?;
        self.leave_scope(context.floor)?;
        if !self.terminated
            && self
                .handlers
                .function_try
                .is_some_and(|function| function.rethrow && function.body == attempt.key)
        {
            self.throw_object(None, true)?;
        }
        if !self.terminated {
            self.exit_handler_context(self.handlers.contexts.len() - 1)?;
            self.jump(attempt.end)?;
            if context.end_throws && self.lifetime_has_pending_chains() {
                self.place_cleanup_chains(false)?;
            }
        }
        self.handlers.contexts.pop();
        self.restore_exception_memos();
        Ok(())
    }
    fn catch_variable(
        &mut self,
        entity: crate::sema::lowering::EntityId,
        ty: TypeId,
        caught: Value,
        record: Option<Value>,
    ) -> Result<(), Error> {
        let reference = matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. });
        let storage = types::storage(self.graph, ty)?;
        let name = self
            .graph
            .lower_local_name(entity)
            .map(|name| self.symbols.parameter_name(name))
            .transpose()?;
        let slot = if let Some(name) = name {
            self.builder.slot(&name, storage)?
        } else {
            self.builder.slot("__slot", storage)?
        };
        self.locals.insert(entity, slot);
        let value = match self.graph.lower_type(ty)? {
            TypeView::Reference { element, .. }
                if matches!(self.graph.lower_type(element)?, TypeView::Pointer(_)) =>
            {
                let record = record.ok_or(crate::sema::Error::Invariant(
                    "pointer catch without record",
                ))?;
                self.effect(I::Store(
                    Facts::memory(false, true),
                    Type::Ptr,
                    caught,
                    record,
                ))?;
                self.emit(I::Addr(record), ty)?.value
            }
            TypeView::Reference { .. } | TypeView::Pointer(_) => caught,
            TypeView::Class(_) => {
                let address = self.emit(I::Addr(slot), ty)?.value;
                if let Some(constructor) = self.graph.lower_construction(entity) {
                    self.handlers.explicit += 1;
                    self.construct_from(constructor, address, caught)?;
                    self.handlers.explicit -= 1;
                }
                self.register_local_destruction(entity)?;
                return Ok(());
            }
            _ => {
                self.emit(
                    I::Load(
                        Facts::memory(false, !matches!(storage, Type::I8 | Type::U8)),
                        storage,
                        caught,
                    ),
                    ty,
                )?
                .value
            }
        };
        self.effect(I::Store(
            Facts::memory(false, !matches!(storage, Type::I8 | Type::U8)),
            storage,
            value,
            slot,
        ))?;
        if reference {
            self.lifetime
                .depths
                .insert(entity, self.lifetime.cleanups.len());
        }
        Ok(())
    }
    pub(super) fn catch_runtime(
        &mut self,
        role: Role,
        result: Type,
        operands: &[Value],
        source: TypeId,
    ) -> Result<Value, Error> {
        let symbol = self
            .symbols
            .runtime_function(self.builder.program(), role)?;
        let arguments = self.builder.arguments(operands)?;
        let call = I::Call(Call {
            result,
            callee: Value::Symbol(symbol),
            arguments,
            signature: None,
            copy_elision: false,
        });
        if result == Type::Void {
            self.effect(call)?;
            Ok(self.builder.integer(0)?)
        } else {
            Ok(self.emit(call, source)?.value)
        }
    }
    pub(super) fn call_end_catch(&mut self, unwinding: bool) -> Result<(), Error> {
        if unwinding {
            let guard = self.shared_cleanup_unwind_guard()?;
            self.effect(I::Runtime(R::EhTry(guard)))?;
        }
        let source = self
            .result
            .ok_or(crate::sema::Error::Invariant("end catch without function"))?;
        self.catch_runtime(Role::EhEndCatch, Type::Void, &[], source)?;
        if unwinding {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        Ok(())
    }
    pub(super) fn handler_resume(&mut self) -> Result<(), Error> {
        // Serialized LowIR resumes into the enclosing function guard.
        self.effect(I::Runtime(R::Resume))?;
        self.terminated = true;
        Ok(())
    }
    fn leave_handler_contexts(&mut self) -> Result<(), Error> {
        let mut top = self.lifetime.cleanups.len();
        for index in (0..self.handlers.contexts.len()).rev() {
            let context = self.handlers.contexts[index];
            for object in (context.floor..top).rev() {
                self.destroy_cleanup_in(self.lifetime.cleanups[object], None, true)?;
            }
            top = context.floor;
            if context.attempt {
                if let Some(target) = self.constructor_unwind_target() {
                    self.effect(I::Runtime(R::EhEnd))?;
                    return self.jump(target);
                }
                if self.function_try_subobjects(Some(index)) {
                    self.effect(I::Runtime(R::Resume))?;
                    self.terminated = true;
                    return Ok(());
                }
                self.effect(I::Runtime(R::EhEnd))?;
                return self.jump(context.entry);
            }
            self.call_end_catch(context.retirement == CatchRetirement::MayDestroy)?;
            self.effect(I::Runtime(R::EhEnd))?;
        }
        for object in (0..top).rev() {
            self.destroy_cleanup_in(self.lifetime.cleanups[object], None, true)?;
        }
        if let Some(target) = self.constructor_unwind_target() {
            self.effect(I::Runtime(R::EhEnd))?;
            self.jump(target)
        } else {
            self.handler_resume()
        }
    }
    pub(super) fn with_handler_contexts<T>(
        &mut self,
        count: usize,
        operation: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let inner = self.handlers.contexts.split_off(count);
        let innermost = self.handlers.innermost;
        self.handlers.innermost = self.handlers.contexts.iter().rposition(|c| c.attempt);
        let result = operation(self);
        self.handlers.contexts.extend(inner);
        self.handlers.innermost = innermost;
        result
    }
    pub(super) fn exit_handler_context(&mut self, index: usize) -> Result<(), Error> {
        self.effect(I::Runtime(R::EhEnd))?;
        let context = self.handlers.contexts[index];
        if context.attempt {
            return Ok(());
        }
        if !context.end_throws {
            return self.call_end_catch(false);
        }
        let entry = self.with_handler_contexts(index, |this| {
            if context.floor > this.handler_floor() {
                Ok(Some(this.cleanup_prefix(true, context.floor)?))
            } else {
                Ok(None)
            }
        })?;
        if let Some(entry) = entry {
            self.effect(I::Runtime(R::EhTry(entry)))?;
        }
        self.call_end_catch(false)?;
        if entry.is_some() {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        Ok(())
    }
    pub(super) fn destroy_context_return(&mut self) -> Result<(), Error> {
        let floor =
            usize::from(self.destruction.cleanup.is_some() && self.handlers.function_try.is_some());
        self.destroy_context_exit(0, floor, true)
    }
    pub(super) fn destroy_context_exit(
        &mut self,
        depth: usize,
        context_floor: usize,
        returning: bool,
    ) -> Result<(), Error> {
        if !returning && context_floor >= self.handlers.contexts.len() {
            return self.destroy_cleanups(depth);
        }
        let pending = self.lifetime_pending_count();
        let mut context = self.handlers.contexts.len();
        for index in (depth..=self.lifetime.cleanups.len()).rev() {
            while context > context_floor && self.handlers.contexts[context - 1].floor >= index {
                context -= 1;
                self.exit_handler_context(context)?;
            }
            if index == depth {
                break;
            }
            let cleanup = self.lifetime.cleanups[index - 1];
            if returning && cleanup.variable.is_some() && cleanup.variable == self.returned_local {
                continue;
            }
            let below =
                self.with_handler_contexts(context, |this| this.lexical_handler(index - 1))?;
            self.destroy_cleanup_in(cleanup, below, false)?;
        }
        if self.lifetime_pending_count() != pending {
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            self.place_cleanup_chains(false)?;
            self.place(next)?;
        }
        Ok(())
    }
}
