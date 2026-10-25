// Translated from lower/allocation_lowering.cpp::array_allocation,
// array_new_shape, invalid_length and zero_bytes. See NOTICE.
//! Runtime bounds remain values; the fixed list prefix reuses aggregate lowering.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{BlockId, Call, Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Compare, Type},
    },
    sema::lowering::{AllocationView, ArrayAllocation, CallTarget, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

pub(super) struct Shape {
    pub element: TypeId,
    pub per_element: u64,
    pub element_size: u64,
    pub cookie_bytes: u64,
    pub stride: u64,
    pub listed: u64,
    largest: u64,
    checked: bool,
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn array_length_throws(&self, use_: &AllocationView) -> Result<bool, Error> {
        let Some(array) = use_.array else {
            return Ok(false);
        };
        Ok(
            (!self.graph.lower_function(use_.function)?.nothrow || use_.reserved)
                && self.array_storage_shape(use_, array)?.checked,
        )
    }
    fn array_storage_shape(
        &self,
        use_: &AllocationView,
        array: ArrayAllocation,
    ) -> Result<Shape, Error> {
        let mut element = use_.target;
        let mut per_element = 1_u64;
        while let TypeView::Array {
            element: inner,
            bound,
        } = self.graph.lower_type(element)?
        {
            per_element = per_element.wrapping_mul(bound);
            element = inner;
        }
        let (element_size, _) = types::layout(self.graph, element)?;
        let stride = element_size.wrapping_mul(per_element);
        let signature = self.graph.lower_function(use_.function)?.ty;
        let variadic = matches!(
            self.graph.lower_type(signature)?,
            TypeView::Function { variadic: true, .. }
        );
        let cookie_bytes =
            self.array_cookie_bytes(element, use_.sized_array_delete, use_.reserved && !variadic)?;
        let listed = match array
            .prefix
            .map(|ty| self.graph.lower_type(ty))
            .transpose()?
        {
            Some(TypeView::Array { bound, .. }) => bound,
            _ => 0,
        };
        let largest = i64::MAX
            .cast_unsigned()
            .wrapping_sub(cookie_bytes)
            .checked_div(stride)
            .unwrap_or(i64::MAX.cast_unsigned());
        let checked = if let Some(value) = array.constant {
            value > largest || value < listed
        } else {
            let facts = self.graph.lower_expression(array.count)?;
            let ty = facts.target.unwrap_or(facts.ty);
            let bits = types::storage(self.graph, ty)?.bits();
            listed != 0
                || types::signed(self.graph, ty)?
                || bits >= 64
                || ((1_u64 << bits) - 1) > largest
        };
        Ok(Shape {
            element,
            per_element,
            element_size,
            cookie_bytes,
            stride,
            listed,
            largest,
            checked,
        })
    }

    pub(super) fn array_allocation(
        &mut self,
        id: ast::ExprId,
        use_: &AllocationView,
        array: ArrayAllocation,
    ) -> Result<RValue, Error> {
        let ExprKind::New { placement, .. } = self.tree.expressions.get(id).kind else {
            return Err(crate::sema::Error::Invariant("array allocation without syntax").into());
        };
        let shape = self.array_storage_shape(use_, array)?;
        let source = self.graph.lower_expression(id)?.ty;
        let function = self.graph.lower_function(use_.function)?;
        let nothrow = function.nothrow && !use_.reserved;
        let mut end = None;
        let mut slot = None;
        if nothrow && shape.checked {
            slot = Some(self.generated_slot("call", Type::Ptr)?);
            end = Some(self.new_block("new_end")?);
        }
        let (size, size_slot) = self.array_storage_size(source, array, &shape, slot.zip(end))?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(function.ty)? else {
            return Err(crate::sema::Error::Invariant("array allocation signature").into());
        };
        let mut values = vec![size];
        let mut argument_classes = Vec::new();
        let written = self.tree.expr_lists.get(placement.unwrap_or_default());
        for (index, &argument) in written.iter().enumerate() {
            let value = if let Some(&parameter) = parameters.get(index + 1) {
                self.operator_argument(argument, parameter)?
            } else {
                self.push_ellipsis_argument(argument, &mut values, &mut argument_classes)?;
                continue;
            };
            values.push(value);
        }
        self.symbols.demand(use_.function);
        let callee = self.symbols.callee(use_.function, &mut self.builder)?;
        let arguments = self.builder.arguments(&values)?;
        let throws = self.lifetime_call_throws(CallTarget::Direct(use_.function))?;
        if throws
            && self.lifetime.full_expression
            && self.lifetime.region_start.is_pending()
            && !self.lifetime.region
            && self.lifetime.cleanups.len() > self.handler_floor()
            && !self.lifetime.tracking.active
        {
            self.open_temporary_region_at(self.lifetime.expression_start)?;
        }
        self.before_lifetime_call(throws)?;
        let raw = self.lifetime_result(
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
        let raw = self.finish_lifetime_call(raw, throws)?;
        if nothrow {
            let destination = if let Some(slot) = slot {
                slot
            } else {
                let value = self.generated_slot("call", Type::Ptr)?;
                slot = Some(value);
                value
            };
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                raw.value,
                destination,
            ))?;
            let initialize = self.new_block("new_init")?;
            let target = if let Some(end) = end {
                end
            } else {
                let block = self.new_block("new_end")?;
                end = Some(block);
                block
            };
            let zero = self.builder.integer(0)?;
            let test = self.emit(I::Compare(Compare::Ne, Type::Ptr, raw.value, zero), source)?;
            self.effect(I::Branch(test.value, initialize, target))?;
            self.place(initialize)?;
        }
        let known_size = size_slot
            .map(|slot| self.emit(I::Load(Facts::memory(false, true), Type::I64, slot), source))
            .transpose()?
            .map_or(size, |value| value.value);
        let elements = self.initialize_allocated_array(
            id,
            use_,
            array,
            &shape,
            super::array_initialization::Storage {
                raw: raw.value,
                size,
                known_size,
            },
            &values,
        )?;
        if let Some((slot, end)) = slot.zip(end) {
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                elements,
                slot,
            ))?;
            self.jump(end)?;
            self.place(end)?;
            return self.emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source);
        }
        Ok(RValue {
            value: elements,
            ..raw
        })
    }

    fn array_storage_size(
        &mut self,
        source: TypeId,
        array: ArrayAllocation,
        shape: &Shape,
        null: Option<(Value, BlockId)>,
    ) -> Result<(Value, Option<Value>), Error> {
        if let Some(count) = array.constant {
            if shape.checked {
                let invalid = if null.is_some() {
                    let zero = self.builder.integer(0)?;
                    Some(
                        self.emit(I::Compare(Compare::Eq, Type::I64, zero, zero), source)?
                            .value,
                    )
                } else {
                    None
                };
                self.invalid_array_length(invalid, null)?;
            }
            let size = if shape.checked {
                u128::MAX
            } else {
                u128::from(
                    count
                        .wrapping_mul(shape.stride)
                        .wrapping_add(shape.cookie_bytes),
                )
            };
            return Ok((self.builder.integer(size)?, None));
        }
        let promoted = self.operand(array.count, Context::Operand)?;
        let count = if promoted.ty.bits() == 64 {
            promoted.value
        } else {
            let op = if types::signed(self.graph, promoted.source)? {
                crate::lowir::vocabulary::Convert::SignExtend
            } else {
                crate::lowir::vocabulary::Convert::ZeroExtend
            };
            self.emit(
                I::Convert(op, Type::I64, promoted.ty, promoted.value),
                source,
            )?
            .value
        };
        if shape.checked {
            let over = if shape.listed == 0 {
                count
            } else {
                let listed = self.builder.integer(u128::from(shape.listed))?;
                self.emit(
                    I::Binary(Binary::Sub, Facts::default(), Type::I64, count, listed),
                    source,
                )?
                .value
            };
            let limit = self
                .builder
                .integer(u128::from(shape.largest.wrapping_sub(shape.listed)))?;
            let invalid = self
                .emit(I::Compare(Compare::Ugt, Type::I64, over, limit), source)?
                .value;
            self.invalid_array_length(Some(invalid), null)?;
        }
        let mut size = if shape.stride == 1 {
            count
        } else {
            let stride = self.builder.integer(u128::from(shape.stride))?;
            self.emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, count, stride),
                source,
            )?
            .value
        };
        if shape.cookie_bytes != 0 {
            let cookie = self.builder.integer(u128::from(shape.cookie_bytes))?;
            size = self
                .emit(
                    I::Binary(Binary::Add, Facts::default(), Type::I64, size, cookie),
                    source,
                )?
                .value;
        }
        let slot = self.generated_slot("array_new_size", Type::I64)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, size, slot))?;
        Ok((size, Some(slot)))
    }

    fn invalid_array_length(
        &mut self,
        invalid: Option<Value>,
        null: Option<(Value, BlockId)>,
    ) -> Result<(), Error> {
        let valid = if let Some(invalid) = invalid {
            if null.is_none() {
                if self.lifetime.full_expression
                    && self.lifetime.region_start.is_pending()
                    && !self.lifetime.region
                    && self.lifetime.cleanups.len() > self.handler_floor()
                    && !self.lifetime.tracking.active
                {
                    self.open_temporary_region_at(self.lifetime.expression_start)?;
                }
                self.before_lifetime_call(true)?;
            }
            let fail = self.new_block("array_new_length_invalid")?;
            let valid = self.new_block("array_new_length_valid")?;
            self.effect(I::Branch(invalid, fail, valid))?;
            self.terminated = true;
            self.place(fail)?;
            Some(valid)
        } else {
            None
        };
        if let Some((slot, end)) = null {
            let zero = self.builder.integer(0)?;
            self.effect(I::Store(Facts::memory(false, true), Type::Ptr, zero, slot))?;
            self.jump(end)?;
        } else {
            let symbol = self.symbols.bad_array_length(self.builder.program())?;
            let arguments = self.builder.arguments(&[])?;
            self.before_lifetime_call(true)?;
            self.effect(I::Call(Call {
                result: Type::Void,
                callee: Value::Symbol(symbol),
                arguments,
                signature: None,
                copy_elision: false,
                argument_classes: Vec::new(),
            }))?;
            if std::mem::take(&mut self.lifetime.call_region)
                == super::lifetime::CallRegion::Transient
            {
                self.close_lifetime_region()?;
            }
            if valid.is_some() {
                self.effect(I::Unreachable)?;
                self.terminated = true;
            }
        }
        if let Some(valid) = valid {
            self.place(valid)?;
        }
        Ok(())
    }

    pub(super) fn zero_allocated_bytes(
        &mut self,
        source: TypeId,
        storage: Value,
        size: Value,
    ) -> Result<(), Error> {
        // Frozen zero_bytes uses the shared fill runtime for host objects;
        // course LowIR retains its explicit byte loop.
        if self.symbols.host_objects() {
            let callee = self.symbols.hosted_runtime_function(
                self.builder.program(),
                super::runtime_intrinsics::RuntimeFunction::Memset,
            )?;
            let zero = self.builder.integer(0)?;
            let arguments = self.builder.arguments(&[storage, zero, size])?;
            let _result = self.emit(
                I::Call(Call {
                    result: Type::Ptr,
                    callee: Value::Symbol(callee),
                    arguments,
                    signature: None,
                    copy_elision: false,
                    argument_classes: Vec::new(),
                }),
                source,
            )?;
            return Ok(());
        }
        let slot = self.generated_slot("zeroinit_offset", Type::I64)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, zero, slot))?;
        let condition = self.new_block("zeroinit_cond")?;
        let body = self.new_block("zeroinit_body")?;
        let end = self.new_block("zeroinit_end")?;
        self.jump(condition)?;
        self.place(condition)?;
        let offset = self
            .emit(I::Load(Facts::memory(false, true), Type::I64, slot), source)?
            .value;
        let test = self
            .emit(I::Compare(Compare::Ult, Type::I64, offset, size), source)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.place(body)?;
        let byte = self
            .emit(I::Index(Type::I8, None, storage, offset), source)?
            .value;
        self.effect(I::Store(Facts::memory(false, false), Type::I8, zero, byte))?;
        let one = self.builder.integer(1)?;
        let next = self
            .emit(
                I::Binary(Binary::Add, Facts::default(), Type::I64, offset, one),
                source,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, next, slot))?;
        self.jump(condition)?;
        self.place(end)
    }
}
