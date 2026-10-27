// Translated from lower/cleanup_lowering.cpp::begin/end_element_temporaries. See NOTICE.
//! Default arguments inside a construction chain have explicit temporary regions.
use super::{Error, function::Function, lifetime::TemporarySegment};
use crate::{
    lowir::{
        builder::Position,
        model::{BlockId, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::Type,
    },
    sema::lowering::{ConstructorId, TypeId, TypeView},
};
#[derive(Clone, Copy)]
struct Handler {
    block: BlockId,
    ty: TypeId,
    address: Value,
    shared: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    markers: u64,
    regions: usize,
    open: bool,
    cleanups: usize,
    contexts: usize,
    explicit: usize,
}
#[derive(Default)]
pub(super) struct State {
    pub depth: usize,
    handlers: Vec<Handler>,
    scratch: Vec<Handler>,
    markers: u64,
    sharing: bool,
    key: Option<Key>,
    shared: Vec<Handler>,
    slots: Vec<(Type, Value)>,
    made: Vec<(Type, Value)>,
}
pub(super) struct Scope {
    first: usize,
    handlers: usize,
    evaluation: Evaluation,
    segment: TemporarySegment,
    region: bool,
    flow_region: super::lifetime::FlowRegion,
    region_start: super::lifetime::RegionStart,
    start: Option<Position>,
    storage_split: Option<Position>,
    temporary_split: Option<Position>,
    tracking: bool,
    share: bool,
}
struct Evaluation {
    full_expression: bool,
    construction: bool,
    construction_flow: super::construction_flow::ConstructionFlow,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn note_element_marker(&mut self) -> Result<(), Error> {
        self.element_temporaries.markers = self
            .element_temporaries
            .markers
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        Ok(())
    }
    fn element_key(&self) -> Key {
        Key {
            markers: self.element_temporaries.markers,
            regions: self.lifetime.regions_opened,
            open: self.lifetime.region,
            cleanups: self.lifetime.cleanups.len(),
            contexts: self.handlers.contexts.len(),
            explicit: self.handlers.explicit,
        }
    }
    pub(super) fn sharing_element_arguments(&self) -> bool {
        self.element_temporaries.sharing && self.element_temporaries.depth == 1
    }
    pub(super) fn element_argument_slot(
        &mut self,
        ty: Type,
        stem: &'static str,
    ) -> Result<Value, Error> {
        if !self.sharing_element_arguments() {
            return self.generated_slot(stem, ty);
        }
        let index = self.element_temporaries.made.len();
        let slot = match self.element_temporaries.slots.get(index) {
            Some(&(previous, slot)) if previous == ty => slot,
            _ => self.generated_slot(stem, ty)?,
        };
        self.element_temporaries.made.push((ty, slot));
        Ok(slot)
    }
    pub(super) fn copy_defaults_temporaries(&mut self, id: ConstructorId) -> Result<bool, Error> {
        let facts = self.graph.lower_constructor_function(id)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(crate::sema::Error::Invariant("copy default signature").into());
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let function = self
            .graph
            .lower_constructor_entity(id)
            .ok_or(crate::sema::Error::Invariant("copy defaults function"))?;
        let mut made = false;
        for (index, &parameter) in parameters.iter().enumerate().skip(1) {
            made |= matches!(self.graph.lower_type(parameter)?, TypeView::Class(_))
                && self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .storage_destructor(parameter)?
                    .is_some();
            if let Some(expression) = self.graph.lower_default(function, index) {
                made |= self.expression_temporaries(expression)?.made;
            }
        }
        Ok(made)
    }
    pub(super) fn begin_copy_temporaries(&mut self, active: bool) -> Option<Scope> {
        self.begin_element_temporaries(active, false)
    }
    pub(super) fn begin_element_temporaries(&mut self, active: bool, share: bool) -> Option<Scope> {
        if !active {
            return None;
        }
        let share = share && self.element_temporaries.depth == 0;
        if share && self.element_temporaries.key != Some(self.element_key()) {
            self.element_temporaries.shared.clear();
            self.element_temporaries.slots.clear();
        }
        self.element_temporaries.sharing = share;
        self.element_temporaries.made.clear();
        let scope = Scope {
            first: self.lifetime.cleanups.len(),
            handlers: self.element_temporaries.handlers.len(),
            evaluation: Evaluation {
                full_expression: self.lifetime.full_expression,
                construction: self.lifetime.construction,
                construction_flow: self.lifetime.construction_flow,
            },
            segment: self.lifetime.temporary_segment,
            region: self.lifetime.region,
            flow_region: self.lifetime.flow_region,
            region_start: self.lifetime.region_start,
            start: self.lifetime.expression_start,
            storage_split: self.lifetime.storage_split.take(),
            temporary_split: self.lifetime.temporary_split.take(),
            tracking: self.lifetime.tracking.active,
            share,
        };
        self.lifetime.full_expression = false;
        self.lifetime.construction = false;
        self.lifetime.construction_flow = super::construction_flow::ConstructionFlow::Linear;
        self.lifetime.temporary_segment = TemporarySegment::Empty;
        self.lifetime.region = false;
        self.lifetime.flow_region = super::lifetime::FlowRegion::Local;
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        self.lifetime.expression_start = None;
        self.lifetime.tracking.active = false;
        self.element_temporaries.depth += 1;
        self.handlers.explicit += 1;
        Some(scope)
    }
    pub(super) fn element_temporary_alive(
        &mut self,
        ty: TypeId,
        address: Value,
    ) -> Result<(), Error> {
        self.register_temporary(ty, address)?;
        let previous = self
            .element_temporaries
            .shared
            .get(self.element_temporaries.handlers.len());
        let shared = self.sharing_element_arguments()
            && self
                .element_temporaries
                .handlers
                .last()
                .is_none_or(|h| h.shared)
            && previous.is_some_and(|h| h.ty == ty && h.address == address);
        let block = if let Some(handler) = previous.filter(|_| shared) {
            handler.block
        } else {
            self.new_block("cleanup_action")?
        };
        self.effect(I::Runtime(R::EhTry(block)))?;
        self.element_temporaries.handlers.push(Handler {
            block,
            ty,
            address,
            shared,
        });
        Ok(())
    }
    pub(super) fn end_copy_temporaries(&mut self, scope: Option<Scope>) -> Result<(), Error> {
        let Some(scope) = scope else {
            return Ok(());
        };
        // Retire this scope's live handlers before emitting destruction,
        // retaining function-owned scratch capacity across later elements.
        let mut handlers = std::mem::take(&mut self.element_temporaries.scratch);
        handlers.clear();
        handlers.extend_from_slice(&self.element_temporaries.handlers[scope.handlers..]);
        self.element_temporaries.handlers.truncate(scope.handlers);
        for _ in &handlers {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        self.destroy_temporaries(scope.first)?;
        if handlers.iter().any(|h| !h.shared) {
            let next = self.new_block("call_unwind_end")?;
            self.jump(next)?;
            for handler in handlers.iter().rev().filter(|h| !h.shared) {
                self.place(handler.block)?;
                let throws = self.destructor_throws(handler.ty)?;
                if throws {
                    self.open_destructor_unwind_guard()?;
                }
                let address = self.storage_address(handler.address, handler.ty)?;
                self.call_destructor(handler.ty, address)?;
                if throws {
                    self.effect(I::Runtime(R::EhEnd))?;
                }
                self.handler_resume()?;
            }
            self.place(next)?;
        }
        self.element_temporaries.depth -= 1;
        self.handlers.explicit -= 1;
        self.lifetime.full_expression = scope.evaluation.full_expression;
        self.lifetime.construction = scope.evaluation.construction;
        self.lifetime.construction_flow = scope.evaluation.construction_flow;
        self.lifetime.temporary_segment = scope.segment;
        self.lifetime.region = scope.region;
        self.lifetime.flow_region = scope.flow_region;
        self.lifetime.region_start = scope.region_start;
        self.lifetime.expression_start = scope.start;
        self.lifetime.storage_split = scope.storage_split;
        self.lifetime.temporary_split = scope.temporary_split;
        self.lifetime.tracking.active = scope.tracking;
        if scope.share {
            self.element_temporaries.sharing = false;
            std::mem::swap(&mut self.element_temporaries.shared, &mut handlers);
            std::mem::swap(
                &mut self.element_temporaries.slots,
                &mut self.element_temporaries.made,
            );
            self.element_temporaries.key = Some(self.element_key());
        }
        handlers.clear();
        self.element_temporaries.scratch = handlers;
        Ok(())
    }
}
