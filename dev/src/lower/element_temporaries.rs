// Translated from lower/cleanup_lowering.cpp::begin/end_element_temporaries. See NOTICE.
//! Default arguments inside a construction chain have explicit temporary regions.
use super::{Error, function::Function, lifetime::TemporarySegment};
use crate::{
    lowir::{
        builder::Position,
        model::{BlockId, InstructionKind as I, RuntimeInstruction as R, Value},
    },
    sema::lowering::{ConstructorId, TypeId, TypeView},
};
#[derive(Clone, Copy)]
struct Handler {
    block: BlockId,
    ty: TypeId,
    address: Value,
}
#[derive(Default)]
pub(super) struct State {
    pub depth: usize,
    handlers: Vec<Handler>,
}
pub(super) struct Scope {
    first: usize,
    handlers: usize,
    full_expression: bool,
    segment: TemporarySegment,
    region: bool,
    region_start: super::lifetime::RegionStart,
    start: Option<Position>,
    storage_split: Option<Position>,
    tracking: bool,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn copy_defaults_temporaries(&mut self, id: ConstructorId) -> Result<bool, Error> {
        let facts = self.graph.lower_constructor_function(id)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(crate::sema::Error::Invariant("copy default signature").into());
        };
        let function = self
            .graph
            .lower_constructor_entity(id)
            .ok_or(crate::sema::Error::Invariant("copy defaults function"))?;
        let mut made = false;
        for (index, &parameter) in parameters.iter().enumerate().skip(1) {
            made |= matches!(self.graph.lower_type(parameter)?, TypeView::Class(_))
                && self.symbols.storage_destructor(parameter)?.is_some();
            if let Some(expression) = self.graph.lower_default(function, index) {
                made |= self.expression_temporaries(expression)?.made;
            }
        }
        Ok(made)
    }
    pub(super) fn begin_copy_temporaries(&mut self, active: bool) -> Option<Scope> {
        if !active {
            return None;
        }
        let scope = Scope {
            first: self.lifetime.cleanups.len(),
            handlers: self.element_temporaries.handlers.len(),
            full_expression: self.lifetime.full_expression,
            segment: self.lifetime.temporary_segment,
            region: self.lifetime.region,
            region_start: self.lifetime.region_start,
            start: self.lifetime.expression_start,
            storage_split: self.lifetime.storage_split.take(),
            tracking: self.lifetime.tracking.active,
        };
        self.lifetime.full_expression = false;
        self.lifetime.temporary_segment = TemporarySegment::Empty;
        self.lifetime.region = false;
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        self.lifetime.expression_start = None;
        self.lifetime.tracking.active = false;
        self.element_temporaries.depth += 1;
        Some(scope)
    }
    pub(super) fn element_temporary_alive(
        &mut self,
        ty: TypeId,
        address: Value,
    ) -> Result<(), Error> {
        self.register_temporary(ty, address);
        let block = self.new_block("cleanup_action")?;
        self.effect(I::Runtime(R::EhTry(block)))?;
        self.element_temporaries
            .handlers
            .push(Handler { block, ty, address });
        Ok(())
    }
    pub(super) fn end_copy_temporaries(&mut self, scope: Option<Scope>) -> Result<(), Error> {
        let Some(scope) = scope else {
            return Ok(());
        };
        let handlers = self.element_temporaries.handlers.split_off(scope.handlers);
        for _ in &handlers {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        self.destroy_temporaries(scope.first)?;
        if !handlers.is_empty() {
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            for handler in handlers.into_iter().rev() {
                self.place(handler.block)?;
                let throws = self.destructor_throws(handler.ty)?;
                if throws {
                    self.open_destructor_unwind_guard()?;
                }
                self.call_destructor(handler.ty, handler.address)?;
                if throws {
                    self.effect(I::Runtime(R::EhEnd))?;
                }
                self.effect(I::Runtime(R::Resume))?;
                self.terminated = true;
            }
            self.place(next)?;
        }
        self.element_temporaries.depth -= 1;
        self.lifetime.full_expression = scope.full_expression;
        self.lifetime.temporary_segment = scope.segment;
        self.lifetime.region = scope.region;
        self.lifetime.region_start = scope.region_start;
        self.lifetime.expression_start = scope.start;
        self.lifetime.storage_split = scope.storage_split;
        self.lifetime.tracking.active = scope.tracking;
        Ok(())
    }
}
