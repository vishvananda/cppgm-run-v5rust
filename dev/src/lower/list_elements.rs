// Translated from lower/initializer_list_lowering.cpp::backing_array. See NOTICE.
//! Each element's handler owns the completed prefix inside the enclosing expression.
use super::{Error, function::Function};
use crate::{
    lowir::{
        model::{BlockId, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{EntityId, TypeId},
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy)]
pub(super) struct Backing {
    pub array: TypeId,
    pub element: TypeId,
    pub base: Value,
    pub persistent: bool,
    pub owner: Option<EntityId>,
    pub storage: Value,
}

/// A construction keeps at most eight addresses inline; larger arrays use a counter.
#[derive(Default)]
pub(super) struct ElementAddresses {
    values: [Option<Value>; 8],
    len: usize,
}
impl ElementAddresses {
    pub(super) fn push(&mut self, address: Value) -> Result<(), Error> {
        let slot = self
            .values
            .get_mut(self.len)
            .ok_or(crate::lowir::Error::Limit)?;
        *slot = Some(address);
        self.len += 1;
        Ok(())
    }
    fn iter(&self) -> impl DoubleEndedIterator<Item = Value> + '_ {
        self.values[..self.len].iter().flatten().copied()
    }
}
#[derive(Clone, Copy)]
struct ElementRun {
    first: u32,
    count: u32,
}
/// Sparse cleanup facts and their contiguous addresses end with this function.
#[derive(Default)]
pub(super) struct SavedElements {
    runs: std::collections::HashMap<Value, ElementRun>,
    values: Vec<Value>,
}
impl SavedElements {
    pub(super) fn insert(&mut self, object: Value, made: &ElementAddresses) -> Result<(), Error> {
        if made.len == 0 {
            return Ok(());
        }
        let first = u32::try_from(self.values.len()).map_err(|_| crate::lowir::Error::Limit)?;
        let count = u32::try_from(made.len).map_err(|_| crate::lowir::Error::Limit)?;
        first.checked_add(count).ok_or(crate::lowir::Error::Limit)?;
        self.values.extend(made.iter());
        self.runs.insert(object, ElementRun { first, count });
        Ok(())
    }
    pub(super) fn len(&self, object: Value) -> Option<u32> {
        self.runs.get(&object).map(|run| run.count)
    }
    pub(super) fn address(&self, object: Value, index: u32) -> Result<Value, Error> {
        let run = self
            .runs
            .get(&object)
            .filter(|run| index < run.count)
            .ok_or(crate::sema::Error::Invariant("saved list address"))?;
        let offset = usize::try_from(run.first + index).map_err(|_| crate::lowir::Error::Limit)?;
        self.values
            .get(offset)
            .copied()
            .ok_or_else(|| crate::sema::Error::Invariant("saved list address range").into())
    }
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn initialize_list_elements(
        &mut self,
        list: ast::ExprId,
        backing: Backing,
    ) -> Result<ElementAddresses, Error> {
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Err(crate::sema::Error::Invariant("list elements syntax").into());
        };
        let Backing {
            array,
            element,
            base,
            persistent,
            owner,
            ..
        } = backing;
        let guarded = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .storage_destructor(element)?
            .is_some();
        let values = self.owned_expression_ids(elements);
        let values = values.as_slice();
        if guarded
            && (!persistent || self.static_temporaries.is_some())
            && self.handlers.explicit == 0
            && self.counts_aggregate_elements(array, values)?
        {
            let aggregate = matches!(self.graph.lower_type(element)?, crate::sema::lowering::TypeView::Class(class) if self.graph.lower_aggregate_class(class));
            let mut temporaries = aggregate;
            for &item in values {
                temporaries |= self
                    .scan_temporaries(item, false, false, Some(element))?
                    .made;
            }
            if temporaries {
                return self.chained_list_elements(list, backing);
            }
        }
        if guarded && values.len() > 8 {
            self.counted_list_elements(list, backing)?;
            return Ok(ElementAddresses::default());
        }
        let stride = super::types::layout(&*self.graph, element)?.0;
        let mut made = ElementAddresses::default();
        let mut inside = false;
        let mut covered = None;
        for (index, &item) in values.iter().enumerate() {
            let region = if guarded && index != 0 {
                Some((
                    self.new_block("call_unwind_dispatch")?,
                    self.new_block("call_unwind_end")?,
                ))
            } else {
                None
            };
            let first = self.lifetime.cleanups.len();
            if region.is_some() {
                inside |= self.lifetime.region;
                self.enclose_element_region(&mut covered)?;
            }
            let split =
                region.is_some() && (inside || !persistent && self.lifetime.full_expression);
            let enclosing = if split {
                self.lifetime.region_entry.filter(|_| self.lifetime.region)
            } else {
                None
            };
            let mut target = base;
            if index != 0 && !split {
                target = self.list_element_address(base, index, stride, element)?;
            }
            if let Some((dispatch, _)) = region {
                self.effect(I::Runtime(R::EhTry(dispatch)))?;
                self.handlers.explicit += 1;
            }
            if index != 0 && split {
                target = self.list_element_address(base, index, stride, element)?;
            }
            self.materialize_list_element(item, element, target, persistent, owner)?;
            if let Some((dispatch, next)) = region {
                self.handlers.explicit -= 1;
                self.effect(I::Runtime(R::EhEnd))?;
                self.with_enclosing_region(enclosing.is_some(), |this| this.jump(next))?;
                self.place(dispatch)?;
                self.with_enclosing_region(true, |this| {
                    if enclosing.is_some()
                        && !this.handlers.contexts.is_empty()
                        && !this
                            .symbols
                            .view(&*this.graph, &*this.tree, this.cursor)
                            .host_objects()
                        && let Some(attempt) = this.innermost_try()
                    {
                        this.emit_try_clauses(attempt)?;
                        this.effect(I::Runtime(R::EhCleanup(None)))?;
                    }
                    for c in (first..this.lifetime.cleanups.len()).rev() {
                        this.destroy_cleanup_in(this.lifetime.cleanups[c], None, true)?;
                    }
                    for address in made.iter().rev() {
                        this.call_destructor(element, address)?;
                    }
                    this.resume_list_element(enclosing)
                })?;
                self.place(next)?;
            }
            if guarded {
                made.push(target)?;
            }
        }
        Ok(made)
    }
    pub(super) fn materialize_list_element(
        &mut self,
        item: ast::ExprId,
        element: TypeId,
        target: Value,
        persistent: bool,
        owner: Option<EntityId>,
    ) -> Result<(), Error> {
        if let Some(nested) = self.list_object(item)? {
            self.initialize_list_object(nested, element, target, persistent, owner)
        } else {
            let saved_owner = self.reference_lifetimes.aggregate;
            let saved_persistent = self.persistent_references;
            let saved_static = self.reference_lifetimes.static_object;
            let saved_list_owner = self.reference_lifetimes.list_owner;
            self.reference_lifetimes.list_owner = owner;
            self.reference_lifetimes.aggregate = owner.filter(|_| !persistent);
            self.reference_lifetimes.static_object = persistent;
            self.persistent_references = false;
            let result = self.materialize_class(item, element, target);
            self.reference_lifetimes.aggregate = saved_owner;
            self.reference_lifetimes.static_object = saved_static;
            self.reference_lifetimes.list_owner = saved_list_owner;
            self.persistent_references = saved_persistent;
            result
        }
    }
    pub(super) fn list_element_address(
        &mut self,
        base: Value,
        index: usize,
        stride: u64,
        ty: TypeId,
    ) -> Result<Value, Error> {
        let offset = u64::try_from(index)
            .map_err(|_| crate::lowir::Error::Limit)?
            .wrapping_mul(stride);
        let offset = self.builder.integer(u128::from(offset))?;
        Ok(self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                ty,
            )?
            .value)
    }
    pub(super) fn resume_list_element(&mut self, enclosing: Option<BlockId>) -> Result<(), Error> {
        if enclosing.is_some()
            && !self.handlers.contexts.is_empty()
            && self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .host_objects()
        {
            return self.handler_resume();
        }
        if let Some(enclosing) = enclosing {
            self.effect(I::Runtime(R::EhEnd))?;
            if self.handlers.contexts.is_empty() {
                self.effect(I::Runtime(R::EhEnd))?;
            }
            self.jump(enclosing)?;
            if self.lifetime.tracking.active
                && !self.lifetime.tracking.made
                && self.lifetime.tracking.head == Some(enclosing)
            {
                self.lifetime
                    .tracking
                    .markers
                    .push(self.builder.position()?);
            }
            Ok(())
        } else {
            self.handler_resume()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{builder::FunctionBuilder, model::Program};

    #[test]
    fn saved_runs_keep_nested_arrays_separate_and_bound_small_prefixes() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "saved_elements", &[], Type::Void)?;
        let outer = builder.slot("outer", Type::I8)?;
        let inner = builder.slot("inner", Type::I8)?;
        let first = builder.integer(1)?;
        let second = builder.integer(2)?;
        let mut saved = SavedElements::default();
        saved.insert(outer, &ElementAddresses::default())?;
        assert_eq!(saved.len(outer), None);
        let mut nested = ElementAddresses::default();
        nested.push(second)?;
        saved.insert(inner, &nested)?;
        let mut prefix = ElementAddresses::default();
        for _ in 0..8 {
            prefix.push(first)?;
        }
        assert!(prefix.push(second).is_err());
        saved.insert(outer, &prefix)?;
        assert_eq!(saved.len(inner), Some(1));
        assert_eq!(saved.address(inner, 0)?, second);
        assert_eq!(saved.len(outer), Some(8));
        assert_eq!(saved.address(outer, 7)?, first);
        assert!(saved.address(inner, 1).is_err());
        assert_eq!(SavedElements::default().len(outer), None);
        Ok(())
    }
}
