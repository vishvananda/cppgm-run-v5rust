// Translated from lower/expression_lowering.cpp's subscript/pointer_offset/pointer_difference. See NOTICE.
//! Element projection and byte-scaled pointer updates preserve the oracle's evaluation and conversion order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Projection, Type},
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast,
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn evaluated_pointer(&mut self, id: ast::ExprId) -> Result<Option<RValue>, Error> {
        let Some(value) = self.graph.lower_evaluated_pointer(self.tree, id)? else {
            return Ok(None);
        };
        let source = self.graph.lower_expression(id)?.ty;
        let place = if matches!(value, crate::sema::lowering::StaticValue::Integer(0)) {
            None
        } else {
            let (symbol, offset) = super::data::address(
                self.builder.program(),
                self.symbols,
                self.graph,
                self.tree,
                value,
            )?;
            let mut place = Value::Symbol(symbol);
            if offset != 0 {
                let address = self.emit(I::Addr(place), source)?.value;
                let offset = self
                    .builder
                    .integer(u128::from(u64::from_le_bytes(offset.to_le_bytes())))?;
                place = self
                    .emit(I::Index(Type::I8, None, address, offset), source)?
                    .value;
            }
            Some(place)
        };
        if let ast::ExprKind::Member { base, op, .. } = self.tree.expressions.get(id).kind {
            self.member_object(
                base,
                op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Arrow),
            )?;
        }
        Ok(Some(if let Some(place) = place {
            RValue {
                value: self.storage_address(place, source)?,
                ty: Type::Ptr,
                source,
                literal: false,
                bits: None,
            }
        } else {
            self.zero(source)?
        }))
    }
    pub(super) fn pointer_element(&self, source: TypeId) -> Result<Option<TypeId>, Error> {
        Ok(match self.graph.lower_type(source)? {
            TypeView::Pointer(element) | TypeView::Array { element, .. } => Some(element),
            _ => None,
        })
    }
    pub(super) fn subscript(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        index: ast::ExprId,
    ) -> Result<Value, Error> {
        let facts = self.graph.lower_expression(id)?;
        let (base, index) = if facts.commuted {
            (index, base)
        } else {
            (base, index)
        };
        let pointer = self.expression(base, Context::Operand)?;
        let offset = self.expression(index, Context::Operand)?;
        let (ty, offset) = if matches!(
            self.graph.lower_type(facts.ty)?,
            TypeView::Array { .. } | TypeView::Class(_)
        ) {
            let offset = self.convert_integer(
                offset,
                offset.source,
                Type::I64,
                types::signed(self.graph, offset.source)?,
                Context::Additive,
            )?;
            let size = self
                .builder
                .integer(u128::from(types::layout(self.graph, facts.ty)?.0))?;
            (
                Type::I8,
                self.emit(
                    I::Binary(Binary::Mul, Facts::default(), Type::I64, offset.value, size),
                    offset.source,
                )?,
            )
        } else {
            (types::storage(self.graph, facts.ty)?, offset)
        };
        let place = self
            .emit(
                I::Index(
                    ty,
                    Some(Projection::ArrayElement),
                    pointer.value,
                    offset.value,
                ),
                facts.ty,
            )?
            .value;
        if self.untyped_places.contains(&pointer.value) {
            self.untyped_places.insert(place);
        }
        Ok(place)
    }
    pub(super) fn pointer_offset(
        &mut self,
        pointer: RValue,
        offset: RValue,
        subtract: bool,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let count =
            self.convert_integer(offset, offset.source, Type::I64, true, Context::Additive)?;
        self.scaled_pointer(pointer, count.value, count.bits, subtract, source)
    }
    pub(super) fn pointer_step(
        &mut self,
        pointer: RValue,
        subtract: bool,
    ) -> Result<RValue, Error> {
        let one = self.builder.integer(1)?;
        self.scaled_pointer(pointer, one, Some(1), subtract, pointer.source)
    }
    fn scaled_pointer(
        &mut self,
        pointer: RValue,
        mut count: Value,
        bits: Option<u64>,
        subtract: bool,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let element = self
            .pointer_element(pointer.source)?
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "pointer offset without element",
            )))?;
        let size = types::layout(self.graph, element)?.0;
        if size != 1 {
            let size = self.builder.integer(u128::from(size))?;
            count = self
                .emit(
                    I::Binary(Binary::Mul, Facts::default(), Type::I64, count, size),
                    source,
                )?
                .value;
        }
        if subtract {
            if size == 1
                && let Some(bits) = bits
            {
                count = self.builder.integer(u128::from_le_bytes(
                    i128::from(i64::from_le_bytes(bits.wrapping_neg().to_le_bytes())).to_le_bytes(),
                ))?;
            } else {
                let zero = self.builder.integer(0)?;
                count = self
                    .emit(
                        I::Binary(Binary::Sub, Facts::default(), Type::I64, zero, count),
                        source,
                    )?
                    .value;
            }
        }
        self.emit(I::Index(Type::I8, None, pointer.value, count), source)
    }
    pub(super) fn pointer_compound(
        &mut self,
        left: ast::ExprId,
        right: ast::ExprId,
        subtract: bool,
        source: TypeId,
    ) -> Result<(Value, RValue), Error> {
        let place = self.scalar_place(left)?;
        let old = self.read_place(place, source)?;
        let rhs = self.expression(right, Context::Operand)?;
        let value = self.pointer_offset(old, rhs, subtract, source)?;
        self.effect(I::Store(
            types::memory(self.graph, source)?,
            Type::Ptr,
            value.value,
            place,
        ))?;
        Ok((place, value))
    }
    pub(super) fn bool_pointer_compound(
        &mut self,
        left: ast::ExprId,
        right: ast::ExprId,
        source: TypeId,
        operation: TypeId,
    ) -> Result<(Value, RValue), Error> {
        // E1 += E2 when E1 is bool and the recorded operation is a pointer:
        // evaluate E1 once, read its count, then E2, scale and convert the sum.
        let place = self.scalar_place(left)?;
        let old = self.read_place(place, source)?;
        let pointer = self.operand(right, Context::CompoundAssignment)?;
        let sum = self.pointer_offset(pointer, old, false, operation)?;
        let value = self.convert(sum, source, Context::CompoundAssignment)?;
        // The frozen bool-pointer path stores the comparison directly,
        // without the arithmetic path's fit/truncation instruction.
        self.effect(I::Store(
            types::memory(self.graph, source)?,
            Type::U8,
            value.value,
            place,
        ))?;
        Ok((place, value))
    }
    pub(super) fn pointer_arithmetic(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        subtract: bool,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let a = self.expression(left, Context::Operand)?;
        let b = self.expression(right, Context::Operand)?;
        let left_element = self.pointer_element(a.source)?;
        let right_element = self.pointer_element(b.source)?;
        if let (Some(element), Some(_)) = (left_element, right_element) {
            let size = types::layout(self.graph, element)?.0;
            let bytes = self.emit(
                I::Binary(Binary::Sub, Facts::default(), Type::Ptr, a.value, b.value),
                source,
            )?;
            if size == 1 {
                return Ok(RValue {
                    ty: Type::I64,
                    ..bytes
                });
            }
            let (op, divisor) = if size.is_power_of_two() {
                (Binary::Shr, u64::from(size.trailing_zeros()))
            } else {
                (Binary::Div, size)
            };
            let divisor = self.builder.integer(u128::from(divisor))?;
            return self.emit(
                I::Binary(
                    op,
                    Facts::arithmetic(false, true),
                    Type::I64,
                    bytes.value,
                    divisor,
                ),
                source,
            );
        }
        if left_element.is_some() {
            self.pointer_offset(a, b, subtract, source)
        } else {
            self.pointer_offset(b, a, false, source)
        }
    }
}
