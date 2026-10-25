// Translated from lower/member_pointer_lowering.cpp. See NOTICE.
//! Itanium member values, null-preserving adjustments, field projections and bound calls.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    lowir::{
        model::{Facts, InstructionKind as I, SignatureId, Value},
        vocabulary::{Binary, Compare, Convert, Projection, Type},
    },
    sema::{
        Graph,
        lowering::{EntityId, TypeId},
    },
    syntax::ast::{self, ExprKind},
};

pub(super) fn is_data_pointer(graph: &Graph, source: TypeId) -> bool {
    graph
        .lower_member_pointer_type(source)
        .is_some_and(|(_, element)| {
            !matches!(
                graph.lower_type(element),
                Ok(crate::sema::lowering::TypeView::Function { .. })
            )
        })
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn member_pointer_constant(
        &mut self,
        value: crate::sema::lowering::MemberPointerValue,
    ) -> Result<RValue, Error> {
        let Some(member) = value.member else {
            return self.zero(value.ty);
        };
        if self.data_member_pointer(value.ty) {
            let offset = self
                .graph
                .lower_member_offset(member)?
                .wrapping_add(self.graph.lower_anonymous_member_offset(member, None)?)
                .wrapping_add(value.adjustment.cast_unsigned());
            let constant = self.builder.integer(u128::from_le_bytes(
                i128::from(i64::from_le_bytes(offset.to_le_bytes())).to_le_bytes(),
            ))?;
            return Ok(RValue {
                value: constant,
                ty: Type::I64,
                source: value.ty,
                literal: false,
                bits: Some(offset),
            });
        }
        self.member_function_word(member, value.ty, value.adjustment.cast_unsigned())
    }

    pub(super) fn bound_pointer_call(
        &mut self,
        access: ast::ExprId,
        object: &mut Value,
        nothrow: bool,
    ) -> Result<(Value, Option<SignatureId>), Error> {
        let callee = self.member_pointer_callee(access, object)?;
        let signature = if matches!(callee, Value::Symbol(_)) {
            None
        } else {
            let ast::ExprKind::Binary { right, .. } = self.tree.expressions.get(access).kind else {
                return Err(crate::sema::Error::Invariant("bound member access syntax").into());
            };
            Some(self.symbols.member_call_signature(
                self.builder.program(),
                self.graph.lower_expression(right)?.ty,
                nothrow,
            )?)
        };
        Ok((callee, signature))
    }
    pub(super) fn converted_member_address(
        &mut self,
        id: ast::ExprId,
    ) -> Result<Option<RValue>, Error> {
        let Some(adjustment) = self.graph.lower_member_pointer_conversion(id)? else {
            return Ok(None);
        };
        if let Some(function) = self.graph.lower_member_function_address(id)? {
            let source =
                self.graph
                    .lower_expression(id)?
                    .target
                    .ok_or(crate::sema::Error::Invariant(
                        "member address conversion target",
                    ))?;
            return self
                .member_function_word(function, source, adjustment)
                .map(Some);
        }
        let Some(offset) = self.graph.lower_data_member_address(id)? else {
            return Ok(None);
        };
        let target =
            self.graph
                .lower_expression(id)?
                .target
                .ok_or(crate::sema::Error::Invariant(
                    "member address conversion target",
                ))?;
        let offset = self
            .builder
            .integer(u128::from(offset.wrapping_add(adjustment)))?;
        self.emit(I::Const(Type::I64, offset), target).map(Some)
    }
    pub(super) fn convert_member_pointer(
        &mut self,
        value: RValue,
        target: TypeId,
        adjustment: u64,
    ) -> Result<RValue, Error> {
        if adjustment == 0 {
            return Ok(RValue {
                source: target,
                ..value
            });
        }
        if !self.data_member_pointer(value.source) {
            return self.convert_member_function_pointer(value, target, adjustment);
        }
        let null = self.builder.integer(u128::MAX)?;
        let present = self.emit(
            I::Compare(Compare::Ne, Type::I64, value.value, null),
            value.source,
        )?;
        let adjustment = self.builder.integer(u128::from_le_bytes(
            i128::from(i64::from_le_bytes(adjustment.to_le_bytes())).to_le_bytes(),
        ))?;
        let scaled = self.emit(
            I::Binary(
                Binary::Mul,
                Facts::default(),
                Type::I64,
                present.value,
                adjustment,
            ),
            value.source,
        )?;
        self.emit(
            I::Binary(
                Binary::Add,
                Facts::default(),
                Type::I64,
                value.value,
                scaled.value,
            ),
            target,
        )
    }
    pub(super) fn member_pointer_place(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        arrow: bool,
    ) -> Result<Value, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let object = self.member_pointer_object(id, left, arrow)?;
        let offset = self.operand(right, Context::Operand)?;
        let place = self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), object, offset.value),
                source,
            )?
            .value;
        if self
            .graph
            .lower_member_pointer_union(self.graph.lower_expression(left)?.ty, arrow)
        {
            self.untyped_places.insert(place);
        }
        Ok(place)
    }

    pub(super) fn member_pointer_object(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        arrow: bool,
    ) -> Result<Value, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let mut object = if arrow {
            self.operand(left, Context::Operand)?.value
        } else {
            self.address(left)?
        };
        if let Some(offset) = self.graph.lower_member_pointer_projection(id)?
            && !offset.is_zero()
        {
            object = self.base_projection(object, source, offset)?;
        }
        Ok(object)
    }

    pub(super) fn data_member_pointer(&self, source: TypeId) -> bool {
        is_data_pointer(self.graph, source)
    }

    pub(super) fn member_pointer_truth(&mut self, value: RValue) -> Result<RValue, Error> {
        if value.ty == Type::I128 {
            return self.emit(
                I::Convert(
                    crate::lowir::vocabulary::Convert::Truncate,
                    Type::I64,
                    Type::I128,
                    value.value,
                ),
                value.source,
            );
        }
        let one = self.builder.integer(1)?;
        self.emit(
            I::Binary(Binary::Add, Facts::default(), Type::I64, value.value, one),
            value.source,
        )
    }

    pub(super) fn member_pointer_equal(
        &mut self,
        left: RValue,
        right: RValue,
        equal: bool,
        source: TypeId,
    ) -> Result<RValue, Error> {
        // Itanium 2.3: null function words compare equal regardless of the
        // adjustment. Preserve the frozen right-before-left truncation order.
        let predicate = if equal { Compare::Eq } else { Compare::Ne };
        let values = self.emit(
            I::Compare(predicate, left.ty, left.value, right.value),
            source,
        )?;
        let right_word = self.member_pointer_truth(right)?;
        let left_word = self.member_pointer_truth(left)?;
        let words = self.emit(
            I::Binary(
                Binary::Or,
                Facts::default(),
                Type::I64,
                left_word.value,
                right_word.value,
            ),
            source,
        )?;
        let zero = self.builder.integer(0)?;
        let null = self.emit(I::Compare(predicate, Type::I64, words.value, zero), source)?;
        self.emit(
            I::Binary(
                if equal { Binary::Or } else { Binary::And },
                Facts::default(),
                Type::I64,
                values.value,
                null.value,
            ),
            source,
        )
    }
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn member_function_word(
        &mut self,
        function: EntityId,
        source: TypeId,
        adjustment: u64,
    ) -> Result<RValue, Error> {
        let word = if let Some(slot) = self.graph.lower_virtual_slot(function) {
            self.builder
                .integer(u128::from(slot.byte_offset().wrapping_add(1)))?
        } else {
            self.symbols.demand(function);
            let symbol = self
                .symbols
                .function_symbol(self.builder.program(), function)?;
            let address = self.emit(I::Addr(Value::Symbol(symbol)), source)?;
            self.emit(I::Copy(Type::I64, address.value), source)?.value
        };
        let word = self.emit(
            I::Convert(Convert::ZeroExtend, Type::I128, Type::I64, word),
            source,
        )?;
        if adjustment == 0 {
            return Ok(word);
        }
        let adjustment = self.builder.integer(u128::from(adjustment))?;
        let shift = self.builder.integer(64)?;
        let high = self.emit(
            I::Binary(Binary::Shl, Facts::default(), Type::I128, adjustment, shift),
            source,
        )?;
        self.emit(
            I::Binary(
                Binary::Add,
                Facts::default(),
                Type::I128,
                word.value,
                high.value,
            ),
            source,
        )
    }

    pub(super) fn convert_member_function_pointer(
        &mut self,
        value: RValue,
        target: TypeId,
        adjustment: u64,
    ) -> Result<RValue, Error> {
        let word = self.member_pointer_truth(value)?;
        let zero = self.builder.integer(0)?;
        let present = self.emit(
            I::Compare(Compare::Ne, Type::I64, word.value, zero),
            value.source,
        )?;
        let adjustment = self.builder.integer(u128::from_le_bytes(
            i128::from(i64::from_le_bytes(adjustment.to_le_bytes())).to_le_bytes(),
        ))?;
        let scaled = self.emit(
            I::Binary(
                Binary::Mul,
                Facts::default(),
                Type::I64,
                present.value,
                adjustment,
            ),
            value.source,
        )?;
        let scaled = self.emit(
            I::Convert(Convert::SignExtend, Type::I128, Type::I64, scaled.value),
            value.source,
        )?;
        let shift = self.builder.integer(64)?;
        let high = self.emit(
            I::Binary(
                Binary::Shl,
                Facts::default(),
                Type::I128,
                scaled.value,
                shift,
            ),
            value.source,
        )?;
        self.emit(
            I::Binary(
                Binary::Add,
                Facts::default(),
                Type::I128,
                value.value,
                high.value,
            ),
            target,
        )
    }

    pub(super) fn member_pointer_callee(
        &mut self,
        access: ast::ExprId,
        object: &mut Value,
    ) -> Result<Value, Error> {
        let ExprKind::Binary { right, .. } = self.tree.expressions.get(access).kind else {
            return Err(crate::sema::Error::Invariant("member call without access").into());
        };
        let source = self.graph.lower_expression(right)?.ty;
        let named = self.without_parentheses(right);
        if matches!(self.tree.expressions.get(named).kind, ExprKind::Unary { op, .. }
            if op.token.as_punctuator() == Some(P::BitAnd))
            && self.graph.lower_member_pointer_conversion(right)?.is_none()
            && let Some(function) = self.graph.lower_member_function_address(named)?
            && self.graph.lower_virtual_slot(function).is_none()
        {
            self.symbols.demand(function);
            return self.symbols.callee(function, &mut self.builder);
        }
        if self.graph.lower_member_pointer_conversion(right)?.is_none()
            && let Some(value) = self.graph.lower_member_argument(self.tree, named)
            && let Some(function) = value.member
            && self.graph.lower_virtual_slot(function).is_none()
        {
            self.symbols.demand(function);
            let callee = self.symbols.callee(function, &mut self.builder)?;
            return Ok(self.emit(I::Addr(callee), source)?.value);
        }
        let bits = self.operand(right, Context::Operand)?;
        let shift = self.builder.integer(64)?;
        let high = self.emit(
            I::Binary(Binary::Shr, Facts::default(), Type::I128, bits.value, shift),
            source,
        )?;
        let adjustment = self.emit(
            I::Convert(Convert::Truncate, Type::I64, Type::I128, high.value),
            source,
        )?;
        let word = self.member_pointer_truth(bits)?;
        let one = self.builder.integer(1)?;
        let flag = self.emit(
            I::Binary(Binary::And, Facts::default(), Type::I64, word.value, one),
            source,
        )?;
        let zero = self.builder.integer(0)?;
        let is_virtual = self.emit(I::Compare(Compare::Ne, Type::I64, flag.value, zero), source)?;
        *object = self
            .emit(I::Index(Type::I8, None, *object, adjustment.value), source)?
            .value;
        let callee = self.generated_slot("member_pointer_callee", Type::Ptr)?;
        let virtual_block = self.new_block("member_pointer_virtual")?;
        let direct_block = self.new_block("member_pointer_direct")?;
        let end = self.new_block("member_pointer_end")?;
        self.effect(I::Branch(is_virtual.value, virtual_block, direct_block))?;
        self.terminated = true;
        self.place(virtual_block)?;
        let table = self.emit(
            I::Load(Facts::memory(false, true), Type::Ptr, *object),
            source,
        )?;
        let offset = self.emit(
            I::Binary(Binary::Sub, Facts::default(), Type::I64, word.value, one),
            source,
        )?;
        let entry = self.emit(I::Index(Type::I8, None, table.value, offset.value), source)?;
        let address = self.emit(
            I::Load(Facts::memory(false, true), Type::Ptr, entry.value),
            source,
        )?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            address.value,
            callee,
        ))?;
        self.effect(I::Jump(end))?;
        self.terminated = true;
        self.place(direct_block)?;
        let address = self.emit(I::Copy(Type::Ptr, word.value), source)?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            address.value,
            callee,
        ))?;
        self.effect(I::Jump(end))?;
        self.terminated = true;
        self.place(end)?;
        Ok(self
            .emit(
                I::Load(Facts::memory(false, true), Type::Ptr, callee),
                source,
            )?
            .value)
    }
}
