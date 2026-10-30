// Translated from lower/allocation_lowering.cpp::allocation/allocation_initializer.
// See NOTICE.
//! Allocation results own explicit storage, never an automatic object's cleanup.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Arguments, Call, InstructionKind as I, Value},
        vocabulary::{Compare, Convert, Type},
    },
    sema::lowering::{AllocationView, CallTarget, ConstructorArguments, EntityId, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn deletion(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let ExprKind::Delete { operand, array, .. } = self.tree.expressions.get(id).kind else {
            return Err(crate::sema::Error::Invariant("deletion without syntax").into());
        };
        let use_ = self.graph.lower_deletion(id)?;
        if array {
            return self.delete_array(id, operand, &use_);
        }
        let source = self.graph.lower_expression(id)?.ty;
        let object = self.operand(operand, Context::Operand)?;
        let class =
            !use_.incomplete && matches!(self.graph.lower_type(use_.target)?, TypeView::Class(_));
        let end = if class {
            let nonnull = self.new_block("delete_nonnull")?;
            let end = self.new_block("delete_end")?;
            let zero = self.builder.integer(0)?;
            let test = self.emit(
                I::Compare(Compare::Ne, Type::Ptr, object.value, zero),
                source,
            )?;
            self.effect(I::Branch(test.value, nonnull, end))?;
            self.place(nonnull)?;
            Some(end)
        } else {
            None
        };
        if let TypeView::Class(class) = self.graph.lower_type(use_.target)?
            && !use_.incomplete
            && self.graph.lower_virtual_destructor(class)
        {
            let destructor = self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .class_destructor(use_.target)?
                .ok_or(crate::sema::Error::Invariant("virtual deletion destructor"))?;
            let throws = self.lifetime_call_throws(CallTarget::Direct(destructor))?;
            crate::lower::unit_frontend::Context {
                graph: self.graph,
                tree: self.tree,
                cursor: self.cursor,
                symbols: self.symbols,
            }
            .vtable_symbol(self.builder.program(), class)?;
            self.begin_lifetime_call(throws)?;
            self.before_lifetime_call(throws)?;
            let slot = self
                .graph
                .lower_virtual_slot(destructor)
                .ok_or(crate::sema::Error::Invariant("virtual deletion slot"))?
                .deleting()?;
            let callee = self.virtual_callee(object.value, slot, source)?;
            let signature = crate::lower::unit_frontend::Context {
                graph: self.graph,
                tree: self.tree,
                cursor: self.cursor,
                symbols: self.symbols,
            }
            .virtual_signature(self.builder.program(), destructor)?;
            let arguments = self.builder.arguments(&[object.value])?;
            self.effect(I::Call(Call {
                result: Type::Void,
                callee,
                arguments,
                signature: Some(signature),
                copy_elision: false,
                argument_classes: (Vec::new()).into(),
            }))?;
            let none = self.zero(source)?;
            self.finish_lifetime_call(none, throws)?;
            if let Some(end) = end {
                self.jump(end)?;
                self.place(end)?;
            }
            return self.zero(source);
        }
        let bytes = if use_.incomplete {
            0
        } else {
            types::layout(&*self.graph, use_.target)?.0
        };
        if class
            && let Some(destructor) = self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .class_destructor(use_.target)?
        {
            let throws = self.lifetime_call_throws(CallTarget::Direct(destructor))?;
            let first = self.lifetime.cleanups.len();
            if throws {
                self.register_storage_cleanup(use_.target, object.value, use_.function, None)?;
            }
            self.begin_lifetime_call(throws)?;
            self.before_lifetime_call(throws)?;
            self.call_destructor(use_.target, object.value)?;
            let none = self.zero(source)?;
            self.finish_lifetime_call(none, throws)?;
            if throws {
                self.close_lifetime_region()?;
                self.release_aggregate_members(first);
            }
        }
        let size = self.builder.integer(u128::from(bytes))?;
        self.deallocate_storage(use_.function, object.value, size, None, true)?;
        if let Some(end) = end {
            self.jump(end)?;
            self.place(end)?;
        }
        self.zero(source)
    }

    pub(super) fn deallocate_storage(
        &mut self,
        function: EntityId,
        storage: Value,
        size: Value,
        placement: Option<Arguments>,
        may_throw: bool,
    ) -> Result<(), Error> {
        let facts = self.graph.lower_function(function)?;
        let TypeView::Function {
            parameters, result, ..
        } = self.graph.lower_type(facts.ty)?
        else {
            return Err(crate::sema::Error::Invariant("deallocation signature").into());
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let throws = may_throw && self.lifetime_call_throws(CallTarget::Direct(function))?;
        let arguments = if let Some(arguments) = placement {
            arguments
        } else if parameters.len() == 2 {
            self.builder.arguments(&[storage, size])?
        } else {
            self.builder.arguments(&[storage])?
        };
        self.symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .demand(function);
        let callee = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .callee(function, &mut self.builder)?;
        // The frozen delete expression emits this call directly: unlike its
        // destructor it does not begin/end a lifetime call around deallocation.
        let transient = throws
            && !self.lifetime.full_expression
            && !self.lifetime.construction
            && !self.lifetime.region;
        self.before_lifetime_call(throws)?;
        let call = I::Call(Call {
            result: Type::Void,
            callee,
            arguments,
            signature: None,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
        });
        self.lifetime_result(call, result)?;
        if transient {
            self.close_lifetime_region()?;
        }
        Ok(())
    }

    pub(super) fn allocation(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let ExprKind::New {
            placement,
            initializer,
            ..
        } = self.tree.expressions.get(id).kind
        else {
            return Err(crate::sema::Error::Invariant("allocation without syntax").into());
        };
        let use_ = self.graph.lower_allocation(id)?;
        if let Some(array) = use_.array {
            return self.array_allocation(id, &use_, array);
        }
        let source = self.graph.lower_expression(id)?.ty;
        let bytes = types::layout(&*self.graph, use_.target)?.0;
        let size = self.builder.integer(u128::from(bytes))?;
        let size = if bytes <= 0x7fff_ffff {
            self.emit(
                I::Convert(Convert::SignExtend, Type::I64, Type::I32, size),
                source,
            )?
            .value
        } else {
            size
        };
        let function = self.graph.lower_function(use_.function)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(function.ty)? else {
            return Err(crate::sema::Error::Invariant("allocation signature").into());
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let mut values = vec![size];
        let mut argument_classes = Vec::new();
        let written = self
            .tree
            .expr_lists
            .get(placement.unwrap_or_default())
            .to_vec();
        let written = written.as_slice();
        // The frozen allocation loop emits only written placement operands,
        // including when overload viability accepted an unwritten default.
        for (index, &argument) in written.iter().enumerate() {
            let index = index + 1;
            let value = if let Some(&parameter) = parameters.get(index) {
                self.operator_argument(argument, parameter)?
            } else {
                self.push_ellipsis_argument(argument, &mut values, &mut argument_classes)?;
                continue;
            };
            values.push(value);
        }
        let mut deallocator = use_.deallocator.filter(|_| !use_.reserved);
        if deallocator.is_some() && !self.allocation_initializer_throws(id)? {
            deallocator = None;
        }
        self.symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .demand(use_.function);
        let callee = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .callee(use_.function, &mut self.builder)?;
        let arguments = self.builder.arguments(&values)?;
        let throws = self.lifetime_call_throws(CallTarget::Direct(use_.function))?;
        self.before_lifetime_call(throws)?;
        let object = self.lifetime_result(
            I::Call(Call {
                result: Type::Ptr,
                callee,
                arguments,
                signature: None,
                copy_elision: false,
                argument_classes: argument_classes.into_iter().rev().collect(),
            }),
            source,
        )?;
        let object = self.finish_allocation_call(object, throws)?;
        let work = initializer.is_some()
            || use_
                .constructor
                .is_some_and(|(id, _)| !self.graph.lower_constructor_trivial(id));
        let end = if self.function_nothrow(use_.function)? && !use_.reserved && work {
            if self.lifetime.tracking.active && self.lifetime.region {
                self.close_lifetime_region()?;
            }
            let initialize = self.new_block("new_init")?;
            let end = self.new_block("new_end")?;
            let zero = self.builder.integer(0)?;
            let nonnull = self.emit(
                I::Compare(Compare::Ne, Type::Ptr, object.value, zero),
                source,
            )?;
            self.effect(I::Branch(nonnull.value, initialize, end))?;
            self.place(initialize)?;
            Some(end)
        } else {
            None
        };
        let opened = deallocator.is_some()
            && self.lifetime.full_expression
            && self.lifetime.region_start.is_pending()
            && self.lifetime.temporary_segment.is_present()
            && !self.lifetime.region
            && !self.lifetime.tracking.active
            && end.is_none();
        if opened {
            self.open_temporary_region_at(self.lifetime.expression_start)?;
            self.close_lifetime_region()?;
        }
        let first = self.lifetime.cleanups.len();
        if let Some(deallocator) = deallocator {
            if self.lifetime.tracking.active {
                self.lifetime.tracking.construction =
                    super::tracked_lifetimes::ConstructionTracking::Storage;
            }
            let placement = if written.is_empty() {
                None
            } else {
                values[0] = object.value;
                Some(self.builder.arguments(&values)?)
            };
            self.register_storage_cleanup(use_.target, object.value, deallocator, placement)?;
        }
        let saved_floor = self.aggregation.floor;
        let saved_regions = self.aggregation.regions;
        let saved_count = if deallocator.is_some() {
            let count = self.aggregation.count.take();
            self.aggregation.floor = if use_.constructor.is_none()
                && matches!(self.graph.lower_type(use_.target)?, TypeView::Class(class) if self.graph.lower_aggregate_class(class))
            {
                Some(self.lifetime.cleanups.len())
            } else {
                None
            };
            self.aggregation.regions = super::aggregate_cleanup::MemberRegions::Absent;
            count
        } else {
            None
        };
        let saved = self.reference_lifetimes.aggregate.take();
        let initialized = self.initialize_allocation(
            &use_,
            object.value,
            initializer,
            self.tree.expressions.get(id).location,
        );
        self.reference_lifetimes.aggregate = saved;
        self.aggregation.floor = saved_floor;
        self.aggregation.regions = saved_regions;
        if deallocator.is_some() {
            self.aggregation.count = saved_count;
        }
        initialized?;
        if deallocator.is_some() {
            self.close_lifetime_region()?;
            self.release_aggregate_members(first);
        }
        if let Some(end) = end {
            self.jump(end)?;
            self.place(end)?;
        }
        // The allocation split resumes the full-expression's region even
        // when its initializer only binds an existing object and leaves no
        // temporaries. Its landing owns the chain below the freed storage.
        if opened && !self.lifetime.region {
            self.open_storage_completion_region(first)?;
        }
        Ok(object)
    }

    fn initialize_allocation(
        &mut self,
        use_: &AllocationView,
        object: Value,
        initializer: Option<ast::Initializer>,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        if let Some((constructor, arguments)) = use_.constructor {
            if initializer.is_none() && self.graph.lower_constructor_trivial(constructor) {
                return Ok(());
            }
            return self.construct_at(
                constructor,
                use_.target,
                object,
                super::construction::ConstructorInput {
                    arguments: Some(arguments),
                    location,
                },
            );
        }
        if let TypeView::Class(class) = self.graph.lower_type(use_.target)? {
            let initializer =
                initializer.ok_or(Error::Missing("allocated class initialization shape"))?;
            if let ast::InitializerKind::Braced(list) = initializer.kind {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
                    return Err(crate::sema::Error::Invariant("allocated aggregate list").into());
                };
                if elements.is_empty() {
                    return if self
                        .symbols
                        .view(&*self.graph, &*self.tree, self.cursor)
                        .empty_list_constructs(class)?
                    {
                        self.aggregate_array(use_.target, object, &[], None)
                    } else {
                        self.zero_class_object(use_.target, object, None)
                    };
                }
                return self.allocated_aggregate(
                    use_.target,
                    object,
                    &self.owned_expression_ids(elements),
                    self.graph.lower_list_union_member(list),
                );
            }
            let source = self.initializer_expression(initializer)?;
            if let Some(crate::sema::lowering::CopyAction::Constructor {
                constructor,
                elision,
                ..
            }) = self.graph.lower_class_copy(source)?
                && self.graph.lower_expression(source)?.is_place
            {
                // allocation_copy_ in the frozen call_special: an allocated
                // glvalue copy calls its selected constructor, even when that
                // constructor can otherwise transfer storage directly.
                return self.call_constructor_from_expression(
                    constructor,
                    object,
                    source,
                    elision,
                    super::construction::CopyCall::Expression,
                    location,
                );
            }
            return self.materialize_class(source, use_.target, object);
        }
        // The frozen allocation_initializer starts with an ordinary Place;
        // scalar initialization stores ignore the target's volatile qualifier;
        // byte-sized stores still follow typed_access's character exclusion.
        self.initialize_scalar_memory(
            use_.target,
            object,
            initializer,
            crate::lowir::model::Facts::memory(
                false,
                !matches!(
                    types::storage(&*self.graph, use_.target)?,
                    Type::I8 | Type::U8
                ),
            ),
        )
    }

    pub(super) fn allocation_initializer_throws(&mut self, id: ast::ExprId) -> Result<bool, Error> {
        let use_ = self.graph.lower_allocation(id)?;
        if let Some((constructor, arguments)) = use_.constructor {
            let mut defaults = Vec::new();
            if self.allocated_constructor_work(constructor, arguments, &mut defaults, false)? {
                return Ok(true);
            }
            for default in defaults {
                if self.expression_throws(default)? {
                    return Ok(true);
                }
            }
        }
        let ExprKind::New { initializer, .. } = self.tree.expressions.get(id).kind else {
            return Err(crate::sema::Error::Invariant("allocation throw query").into());
        };
        if self.initializer_throws(initializer)? {
            return Ok(true);
        }
        if use_.constructor.is_none()
            && let TypeView::Class(class) = self.graph.lower_type(use_.target)?
        {
            return self.omitted_aggregate_throws_in_context(class, 0, false);
        }
        Ok(false)
    }

    pub(super) fn allocated_constructor_work(
        &mut self,
        constructor: crate::sema::lowering::ConstructorId,
        arguments: ConstructorArguments,
        work: &mut Vec<ast::ExprId>,
        hidden: bool,
    ) -> Result<bool, Error> {
        if !self.graph.lower_constructor_trivial(constructor)
            && self.entry_throws_in_context(
                super::construction::Entry::Constructor(
                    constructor,
                    super::construction::Variant::Complete,
                ),
                hidden,
            )?
        {
            return Ok(true);
        }
        let written = match arguments {
            ConstructorArguments::Values(values) => self.tree.expr_lists.get(values).len(),
            ConstructorArguments::Single(_) => 1,
            ConstructorArguments::Value => 0,
        };
        if let Some(entity) = self.graph.lower_constructor_entity(constructor) {
            let ty = self.graph.lower_constructor_function(constructor)?.ty;
            let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
                return Err(
                    crate::sema::Error::Invariant("allocated constructor signature").into(),
                );
            };
            let parameters = parameters.to_vec();
            let parameters = parameters.as_slice();
            for index in written..parameters.len() {
                work.extend(self.graph.lower_default(entity, index));
            }
        }
        Ok(false)
    }
}
