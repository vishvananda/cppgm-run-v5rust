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
        model::{Call, InstructionKind as I, Value},
        vocabulary::{Compare, Convert, Type},
    },
    sema::lowering::{AllocationView, CallTarget, ConstructorArguments, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
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
        let source = self.graph.lower_expression(id)?.ty;
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
                self.operand(argument, Context::Argument)?.value
            };
            values.push(value);
        }
        let throws = self.lifetime_call_throws(CallTarget::Direct(use_.function))?;
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
        if use_.deallocator.is_some() && !use_.reserved && self.allocation_initializer_throws(id)? {
            return Err(Error::Missing("allocation failure deallocation cleanup"));
        }
        let work = initializer.is_some()
            || use_
                .constructor
                .is_some_and(|(id, _)| !self.graph.lower_constructor_trivial(id));
        let end = if function.nothrow && !use_.reserved && work {
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
        let saved = self.reference_lifetimes.aggregate.take();
        let initialized = self.initialize_allocation(&use_, object.value, initializer);
        self.reference_lifetimes.aggregate = saved;
        initialized?;
        if let Some(end) = end {
            self.jump(end)?;
            self.place(end)?;
        }
        Ok(object)
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
        if matches!(self.graph.lower_type(use_.target)?, TypeView::Class(_)) {
            if let Some(initializer) = initializer {
                if let ast::InitializerKind::Braced(list) = initializer.kind {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind
                    else {
                        return Err(
                            crate::sema::Error::Invariant("allocated aggregate list").into()
                        );
                    };
                    return self.aggregate_prvalue(
                        use_.target,
                        object,
                        self.tree.expr_lists.get(elements),
                    );
                }
                let source = self.initializer_expression(initializer)?;
                return self.materialize_class(source, use_.target, object);
            }
            return Err(Error::Missing("allocated class initialization shape"));
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
        self.initializer_throws(initializer)
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
