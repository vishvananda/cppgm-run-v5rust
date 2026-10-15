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
        let bytes = if use_.incomplete {
            0
        } else {
            types::layout(self.graph, use_.target)?.0
        };
        if class && let Some(destructor) = self.symbols.class_destructor(use_.target)? {
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
        let throws = may_throw && self.lifetime_call_throws(CallTarget::Direct(function))?;
        let arguments = if let Some(arguments) = placement {
            arguments
        } else if parameters.len() == 2 {
            self.builder.arguments(&[storage, size])?
        } else {
            self.builder.arguments(&[storage])?
        };
        self.symbols.demand(function);
        let callee = self.symbols.callee(function, &mut self.builder)?;
        // The frozen delete expression emits this call directly: unlike its
        // destructor it does not begin/end a lifetime call around deallocation.
        self.before_lifetime_call(throws)?;
        let call = I::Call(Call {
            result: Type::Void,
            callee,
            arguments,
            signature: None,
            copy_elision: false,
        });
        self.lifetime_result(call, result)?;
        if !self.lifetime.full_expression && !self.lifetime.construction {
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
        self.begin_storage_expression(placement.is_some_and(|values| !values.is_empty()))?;
        let bytes = types::layout(self.graph, use_.target)?.0;
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
        let mut values = vec![size];
        let written = self.tree.expr_lists.get(placement.unwrap_or_default());
        // The frozen allocation loop emits only written placement operands,
        // including when overload viability accepted an unwritten default.
        for (index, &argument) in written.iter().enumerate() {
            let index = index + 1;
            let value = if let Some(&parameter) = parameters.get(index) {
                self.operator_argument(argument, parameter)?
            } else {
                self.ellipsis_argument(argument)?
            };
            values.push(value);
        }
        let throws = self.lifetime_call_throws(CallTarget::Direct(use_.function))?
            || self.lifetime.full_expression
                && self.lifetime.temporary_segment.allows_eager_arguments()
                && function.builtin.is_none();
        self.symbols.demand(use_.function);
        let callee = self.symbols.callee(use_.function, &mut self.builder)?;
        let arguments = self.builder.arguments(&values)?;
        self.before_lifetime_call(throws)?;
        let object = self.lifetime_result(
            I::Call(Call {
                result: Type::Ptr,
                callee,
                arguments,
                signature: None,
                copy_elision: false,
            }),
            source,
        )?;
        let object = self.finish_lifetime_call(object, throws)?;
        let deallocator = use_.deallocator.filter(|_| !use_.reserved);
        let deallocator = if self.allocation_initializer_throws(id)? {
            deallocator
        } else {
            None
        };
        let work = initializer.is_some()
            || use_
                .constructor
                .is_some_and(|(id, _)| !self.graph.lower_constructor_trivial(id));
        let end = if function.nothrow && !use_.reserved && work {
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
        let saved = self.reference_lifetimes.aggregate.take();
        let initialized = self.initialize_allocation(&use_, object.value, initializer);
        self.reference_lifetimes.aggregate = saved;
        initialized?;
        if deallocator.is_some() {
            self.close_lifetime_region()?;
            self.release_aggregate_members(first);
        }
        if let Some(end) = end {
            self.jump(end)?;
            self.place(end)?;
        }
        if opened && !self.lifetime.region && self.lifetime.cleanups.len() > first {
            self.open_storage_completion_region(first)?;
        }
        Ok(object)
    }

    fn begin_storage_expression(&mut self, allow_empty_chain: bool) -> Result<(), Error> {
        if self.lifetime.full_expression
            && self.lifetime.temporary_segment.is_present()
            && self.lifetime.region_start.is_pending()
            && !self.lifetime.region
            && !self.lifetime.tracking.active
            && (allow_empty_chain || !self.lifetime.cleanups.is_empty())
        {
            self.open_temporary_region_at(self.lifetime.expression_start)?;
        }
        Ok(())
    }

    fn initialize_allocation(
        &mut self,
        use_: &AllocationView,
        object: Value,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        if let Some((constructor, arguments)) = use_.constructor {
            if initializer.is_none() && self.graph.lower_constructor_trivial(constructor) {
                return Ok(());
            }
            return self.construct_at(constructor, use_.target, object, Some(arguments));
        }
        if let TypeView::Class(class) = self.graph.lower_type(use_.target)? {
            let initializer =
                initializer.ok_or(Error::Missing("allocated class initialization shape"))?;
            if let ast::InitializerKind::Braced(list) = initializer.kind {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
                    return Err(crate::sema::Error::Invariant("allocated aggregate list").into());
                };
                if elements.is_empty() {
                    return if self.symbols.empty_list_constructs(class)? {
                        self.aggregate_array(use_.target, object, &[])
                    } else {
                        self.zero_class_object(use_.target, object, None)
                    };
                }
                return self.aggregate_prvalue(
                    use_.target,
                    object,
                    self.tree.expr_lists.get(elements),
                );
            }
            let source = self.initializer_expression(initializer)?;
            return self.materialize_class(source, use_.target, object);
        }
        // The frozen allocation_initializer starts with an ordinary Place;
        // scalar initialization stores are typed even for a volatile target.
        self.initialize_scalar_memory(
            use_.target,
            object,
            initializer,
            crate::lowir::model::Facts::memory(false, true),
        )
    }

    pub(super) fn allocation_initializer_throws(&mut self, id: ast::ExprId) -> Result<bool, Error> {
        let use_ = self.graph.lower_allocation(id)?;
        if let Some((constructor, arguments)) = use_.constructor {
            let mut defaults = Vec::new();
            if self.allocated_constructor_work(constructor, arguments, &mut defaults)? {
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
            return self.omitted_aggregate_throws(class, 0);
        }
        Ok(false)
    }

    pub(super) fn allocated_constructor_work(
        &self,
        constructor: crate::sema::lowering::ConstructorId,
        arguments: ConstructorArguments,
        work: &mut Vec<ast::ExprId>,
    ) -> Result<bool, Error> {
        if !self.graph.lower_constructor_trivial(constructor)
            && !self.graph.lower_constructor_function(constructor)?.nothrow
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
            for index in written..parameters.len() {
                work.extend(self.graph.lower_default(entity, index));
            }
        }
        Ok(false)
    }
}
