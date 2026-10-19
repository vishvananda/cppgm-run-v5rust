// Translated from lower/initializer_list_lowering.cpp::counted_elements/chained_elements.
// See NOTICE.
//! Large backing arrays share one handler; temporary-producing elements join the live prefix chain.
use super::{
    Error,
    function::Function,
    list_elements::{Backing, ElementAddresses},
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, RuntimeInstruction as R},
        vocabulary::{Binary, Compare, Projection, Type},
    },
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn counted_list_elements(
        &mut self,
        list: ast::ExprId,
        backing: Backing,
    ) -> Result<(), Error> {
        let Backing {
            element,
            base,
            persistent,
            ..
        } = backing;
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Err(crate::sema::Error::Invariant("counted list syntax").into());
        };
        let values = self.tree.expr_lists.get(elements);
        let stride = super::types::layout(self.graph, element)?.0;
        let counter = self.generated_slot("initlist_constructor_index", Type::I64)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::I64,
            zero,
            counter,
        ))?;
        let cleanup = self.new_block("initlist_constructor_cleanup")?;
        let mut covered = None;
        for (index, &item) in values.iter().enumerate() {
            let mut target = base;
            if index != 0 {
                self.enclose_list_element(&mut covered)?;
                let made = self
                    .builder
                    .integer(u128::try_from(index).map_err(|_| crate::lowir::Error::Limit)?)?;
                self.effect(I::Store(
                    Facts::memory(false, true),
                    Type::I64,
                    made,
                    counter,
                ))?;
                target = self.list_element_address(base, index, stride, element)?;
                self.effect(I::Runtime(R::EhTry(cleanup)))?;
                self.handlers.explicit += 1;
            }
            self.materialize_list_element(item, element, target, persistent, backing.owner)?;
            if index != 0 {
                self.handlers.explicit -= 1;
                self.effect(I::Runtime(R::EhEnd))?;
            }
        }
        let count = self
            .builder
            .integer(u128::try_from(values.len()).map_err(|_| crate::lowir::Error::Limit)?)?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::I64,
            count,
            counter,
        ))?;
        let end = self.new_block("initlist_constructor_end")?;
        let enclosing = self.lifetime.region_entry.filter(|_| self.lifetime.region);
        self.with_enclosing_region(enclosing.is_some(), |this| this.jump(end))?;
        self.with_enclosing_region(true, |this| {
            this.place(cleanup)?;
            if enclosing.is_some()
                && !this.handlers.contexts.is_empty()
                && let Some(attempt) = this.innermost_try()
            {
                this.emit_try_clauses(attempt)?;
                this.effect(I::Runtime(R::EhCleanup(None)))?;
            }
            let test = this.new_block("initlist_constructor_cleanup_condition")?;
            this.jump(test)?;
            this.place(test)?;
            let made = this
                .emit(
                    I::Load(Facts::memory(false, true), Type::I64, counter),
                    element,
                )?
                .value;
            let body = this.new_block("initlist_constructor_cleanup_body")?;
            let resume = this.new_block("initlist_constructor_resume")?;
            let nonzero = this
                .emit(I::Compare(Compare::Ne, Type::I64, made, zero), element)?
                .value;
            this.effect(I::Branch(nonzero, body, resume))?;
            this.terminated = true;
            this.place(body)?;
            let one = this.builder.integer(1)?;
            let previous = this
                .emit(
                    I::Binary(Binary::Sub, Facts::default(), Type::I64, made, one),
                    element,
                )?
                .value;
            this.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                previous,
                counter,
            ))?;
            let offset = if stride == 1 {
                previous
            } else {
                let size = this.builder.integer(u128::from(stride))?;
                this.emit(
                    I::Binary(Binary::Mul, Facts::default(), Type::I64, previous, size),
                    element,
                )?
                .value
            };
            let address = this
                .emit(
                    I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                    element,
                )?
                .value;
            this.call_destructor(element, address)?;
            this.jump(test)?;
            this.place(resume)?;
            this.resume_list_element(enclosing)
        })?;
        self.place(end)
    }
    pub(super) fn chained_list_elements(
        &mut self,
        list: ast::ExprId,
        backing: Backing,
    ) -> Result<ElementAddresses, Error> {
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Err(crate::sema::Error::Invariant("chained list syntax").into());
        };
        let saved_count = self.aggregation.count.take();
        let saved_landing = self.aggregation.landing;
        let saved_regions = self.aggregation.regions;
        let scope = self.begin_counted_elements(backing.array, backing.storage)?;
        // The backing prefix is independent of any aggregate member array
        // initialized inside one element. Keep its counter on this owner.
        let mut count = self
            .aggregation
            .count
            .take()
            .ok_or(crate::sema::Error::Invariant("list prefix counter"))?;
        let stride = super::types::layout(self.graph, backing.element)?.0;
        let mut made = ElementAddresses::default();
        let result = (|| {
            for (index, &item) in self.tree.expr_lists.get(elements).iter().enumerate() {
                let first = self.lifetime.cleanups.len();
                let target = if index == 0 {
                    backing.base
                } else {
                    self.list_element_address(backing.base, index, stride, backing.element)?
                };
                self.materialize_list_element(
                    item,
                    backing.element,
                    target,
                    backing.persistent,
                    backing.owner,
                )?;
                count = self.aggregate_element_made(count, first)?;
                if self.tree.expr_lists.get(elements).len() <= 8 {
                    made.push(target)?;
                }
            }
            self.end_counted_backing(scope)
        })();
        self.aggregation.count = saved_count;
        self.aggregation.landing = saved_landing;
        self.aggregation.regions = saved_regions;
        result?;
        Ok(made)
    }
}
