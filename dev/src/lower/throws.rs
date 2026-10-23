// Translated from lower/exception_lowering.cpp::throw_object/exception_object. See NOTICE.
//! Throw lowering consumes the retained object type and selected initialization plan.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Call, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Role, Type},
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast,
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn throw_object(
        &mut self,
        operand: Option<ast::ExprId>,
        statement: bool,
    ) -> Result<(), Error> {
        let nested = !statement && self.lifetime.full_expression;
        let staged = statement
            && !self.lifetime.full_expression
            && !self.lifetime.region
            && self.lifetime.cleanups.len()
                > self
                    .lifetime
                    .temporary_segment
                    .floor()
                    .max(self.handler_floor());
        let Some(operand) = operand else {
            if staged {
                self.generated_slot("throw_alloc", Type::Ptr)?;
            }
            if nested {
                self.begin_lifetime_call(true)?;
            }
            self.throw_runtime_call(Role::EhRethrow, Type::Void, &[], self.result, true)?;
            if nested {
                self.lifetime.call_marks.pop();
            }
            return self.throw_exit(statement);
        };
        let ty = self.graph.lower_thrown_type(operand)?;
        if nested {
            self.begin_lifetime_call(true)?;
        }
        let size = self
            .builder
            .integer(u128::from(types::layout(self.graph, ty)?.0))?;
        let object = if statement {
            if staged {
                self.before_lifetime_call(true)?;
            }
            let value = self.throw_runtime_call(
                Role::EhAllocateException,
                Type::Ptr,
                &[size],
                Some(ty),
                true,
            )?;
            if staged {
                let slot = self.generated_slot("throw_alloc", Type::Ptr)?;
                self.effect(I::Store(Facts::memory(false, true), Type::Ptr, value, slot))?;
                self.close_lifetime_region_to("throw_alloc_unwind_end")?;
                self.emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), ty)?
                    .value
            } else {
                value
            }
        } else {
            if nested {
                self.generated_slot("throw_alloc", Type::Ptr)?;
                self.before_lifetime_call(true)?;
            }
            self.throw_runtime_call(
                Role::EhAllocateException,
                Type::Ptr,
                &[size],
                Some(ty),
                false,
            )?
        };
        if nested {
            self.lifetime.call_marks.pop();
        }
        self.initialize_exception(operand, ty, object, statement)?;
        if nested {
            self.begin_lifetime_call(true)?;
        } else if statement && !self.lifetime.full_expression {
            self.before_lifetime_call(true)?;
        }
        let information = self
            .symbols
            .exception_type_information(self.builder.program(), ty)?;
        let information = self.emit(I::Addr(Value::Symbol(information)), ty)?.value;
        self.symbols.thrown_type(self.builder.program(), ty)?;
        let destructor = if let Some(destructor) = self.symbols.class_destructor(ty)? {
            self.symbols.demand(destructor);
            let symbol = self.symbols.entry_symbol(
                self.builder.program(),
                super::construction::Entry::Function(destructor),
            )?;
            self.emit(I::Addr(Value::Symbol(symbol)), ty)?.value
        } else {
            self.builder.integer(0)?
        };
        self.throw_runtime_call(
            Role::EhThrow,
            Type::Void,
            &[object, information, destructor],
            Some(ty),
            true,
        )?;
        if nested {
            self.lifetime.call_marks.pop();
        }
        self.throw_exit(statement)
    }
    fn initialize_exception(
        &mut self,
        operand: ast::ExprId,
        ty: TypeId,
        object: Value,
        statement: bool,
    ) -> Result<(), Error> {
        let first = self.lifetime.cleanups.len();
        let began = statement
            && self.begin_full_expression(
                Some(operand),
                // exception_object opens an ordinary, non-discarded segment.
                // Unlike a return/condition, it supplies no hidden EH context
                // that could make a selected noexcept copy appear throwing.
                super::temporary_scan::ExpressionUse::Initialization,
            )?;
        let initialize = |this: &mut Self| -> Result<(), Error> {
            if matches!(this.graph.lower_type(ty)?, TypeView::Class(_)) {
                this.materialize_class(operand, ty, object)
            } else {
                let value = this.operand(operand, Context::Initialization)?;
                let value = this.fit(value, ty)?;
                let storage = types::storage(this.graph, ty)?;
                this.effect(I::Store(
                    Facts::memory(false, !matches!(storage, Type::I8 | Type::U8)),
                    storage,
                    value.value,
                    object,
                ))
            }
        };
        if statement {
            self.with_full_root(Some(operand), initialize)?;
        } else {
            initialize(self)?;
        }
        if statement {
            self.destroy_temporaries(first)?;
            self.end_lifetime_expression(began)?;
        }
        Ok(())
    }
    fn throw_runtime_call(
        &mut self,
        role: Role,
        result: Type,
        operands: &[Value],
        source: Option<TypeId>,
        throwing: bool,
    ) -> Result<Value, Error> {
        if throwing {
            self.before_lifetime_call(true)?;
        }
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
            argument_classes: Vec::new(),
        });
        if result == Type::Void {
            self.effect(call)?;
            if throwing
                && std::mem::take(&mut self.lifetime.call_region)
                    == super::lifetime::CallRegion::Transient
            {
                self.close_lifetime_region()?;
            }
            Ok(self.builder.integer(0)?)
        } else {
            let source = source.ok_or(crate::sema::Error::Invariant(
                "exception call without source type",
            ))?;
            Ok(self.emit(call, source)?.value)
        }
    }
    fn throw_exit(&mut self, statement: bool) -> Result<(), Error> {
        // A statement closes its call region in place. A nested throw's dead
        // continuation keeps the full-expression record until the block ends.
        if statement && std::mem::take(&mut self.lifetime.region) {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        if statement {
            let count = self
                .handlers
                .contexts
                .iter()
                .rev()
                .take_while(|c| c.attempt)
                .count();
            for _ in 0..count {
                self.effect(I::Runtime(R::EhEnd))?;
            }
        } else if self.handlers.contexts.last().is_some_and(|c| c.attempt) {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        let expression = std::mem::replace(&mut self.lifetime.full_expression, false);
        let source = self.result.ok_or(crate::sema::Error::Invariant(
            "throw without function result",
        ))?;
        let ty = self.symbols.result_boundary(source)?.ty;
        // A no-return continuation leaves the constructor's subobject record
        // for the actual unwind, unlike a source return from its body.
        let placing = std::mem::replace(&mut self.construction.placing, true);
        let result = self.fallback_return(source, ty);
        self.construction.placing = placing;
        self.lifetime.full_expression = expression;
        self.lifetime.region = false;
        result
    }
    pub(super) fn throw_value(
        &mut self,
        id: ast::ExprId,
        operand: Option<ast::ExprId>,
    ) -> Result<RValue, Error> {
        self.throw_object(operand, false)?;
        let source = self.graph.lower_expression(id)?.ty;
        Ok(RValue {
            value: self.builder.integer(0)?,
            ty: Type::Void,
            source,
            literal: false,
            bits: None,
        })
    }
}
