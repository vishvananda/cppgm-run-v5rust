// Translated from lower/expression_lowering.cpp. See NOTICE.
//! Scalar operands consume recorded bindings and conversions in source evaluation order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    lowir::{
        literals::{Constant, FloatKind},
        model::{Facts, InstructionKind as I},
        vocabulary::{Binary, Compare, Type, Unary},
    },
    sema::lowering::{CallTarget, TypeView},
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn operand(&mut self, id: ast::ExprId, context: Context) -> Result<RValue, Error> {
        if let Some((call, target)) = self.graph.lower_function_conversion(id)? {
            let value = self.call_conversion(id, call)?;
            if context != Context::Cast
                && let Some(offset) = self.graph.lower_base_conversion(id)?
            {
                return self.convert_base_pointer(value, target, offset, !offset.is_zero());
            }
            return self.convert(value, target, context);
        }
        if let Some(value) = self.converted_member_address(id)? {
            return Ok(value);
        }
        let value = self.expression(id, context)?;
        self.apply_conversion(id, value, context)
    }
    pub(super) fn apply_conversion(
        &mut self,
        id: ast::ExprId,
        mut value: RValue,
        context: Context,
    ) -> Result<RValue, Error> {
        let facts = self.graph.lower_expression(id)?;
        // A converted immediate is not necessarily a literal written at this
        // use. C++ tests the original expression when deciding widening folds.
        value.literal = self.written_literal(id)?;
        if facts.null_pointer
            && let Some(target) = facts.target
            && self.graph.lower_member_pointer_type(target).is_some()
        {
            return self.zero(target);
        }
        if facts.null_pointer
            && value.bits.is_some()
            && !matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Scalar(crate::sema::lowering::Scalar::Nullptr)
            )
        {
            let target = facts
                .target
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "null conversion without target",
                )))?;
            let zero = self.zero(target)?;
            // A converted integer null constant is a ptr value even when
            // its semantic target is nullptr_t rather than a pointer type.
            return Ok(RValue {
                ty: Type::Ptr,
                ..zero
            });
        }
        if let Some(target) = facts.target {
            if let Some(adjustment) = self.graph.lower_member_pointer_conversion(id)? {
                return self.convert_member_pointer(value, target, adjustment);
            }
            if context != Context::Cast
                && let Some(offset) = self.graph.lower_base_conversion(id)?
            {
                return self.convert_base_pointer(
                    value,
                    target,
                    offset,
                    !self.nonnull_pointer(id) && !offset.is_zero(),
                );
            }
            self.convert(value, target, context)
        } else {
            Ok(value)
        }
    }
    pub(super) fn expression(
        &mut self,
        id: ast::ExprId,
        context: Context,
    ) -> Result<RValue, Error> {
        let mut value = self.expression_value(id, context)?;
        value.literal = self.written_literal(id)?;
        Ok(value)
    }
    fn written_literal(&self, id: ast::ExprId) -> Result<bool, Error> {
        let id = self.without_parentheses(id);
        let facts = self.graph.lower_expression(id)?;
        let scalar = matches!(self.graph.lower_type(facts.ty)?, TypeView::Scalar(_));
        let integral = scalar && types::storage(self.graph, facts.ty)?.float_rank() == 0;
        Ok(match self.tree.expressions.get(id).kind {
            ExprKind::Literal(_) => integral && self.graph.lower_literal_call(id)?.is_none(),
            ExprKind::Keyword(Keyword::True | Keyword::False) => true,
            ExprKind::Id(_)
                if integral
                    && facts.folded.is_some()
                    && facts.origin == crate::sema::lowering::ExpressionOrigin::Implicit =>
            {
                true
            }
            ExprKind::Id(name) if integral && facts.folded.is_some() => {
                let entity = self.graph.lower_name(name)?;
                self.graph.lower_static_member(entity)
                    || self.graph.lower_variable_arguments(entity).is_some()
            }
            ExprKind::Member { .. } if integral && facts.folded.is_some() => self
                .graph
                .lower_static_member(self.graph.lower_member(id)?.0),
            _ => false,
        })
    }
    fn evaluated_wide(
        &mut self,
        id: ast::ExprId,
        ty: Type,
        source: crate::sema::lowering::TypeId,
    ) -> Result<Option<RValue>, Error> {
        if let Some(bits) = self.graph.lower_evaluated_wide(self.tree, id)? {
            if let ExprKind::Member { base, op, .. } =
                self.tree.expressions.get(self.without_parentheses(id)).kind
            {
                self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
            }
            return Ok(Some(RValue {
                value: self.builder.integer(bits)?,
                ty,
                source,
                literal: false,
                bits: Some(
                    u64::try_from(bits & u128::from(u64::MAX))
                        .map_err(|_| crate::lowir::Error::Limit)?,
                ),
            }));
        }
        Ok(None)
    }
    fn evaluated_floating(&mut self, id: ast::ExprId) -> Result<Option<RValue>, Error> {
        let id = self.without_parentheses(id);
        let Some(bits) = self.graph.lower_evaluated_floating(self.tree, id)? else {
            return Ok(None);
        };
        let source = self.graph.lower_expression(id)?.ty;
        let ty = types::storage(self.graph, source)?;
        let kind = FloatKind::from_type(ty).ok_or(crate::sema::Error::Invariant(
            "floating constant storage type",
        ))?;
        if let ExprKind::Member { base, op, .. } = self.tree.expressions.get(id).kind {
            self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
        }
        Ok(Some(RValue {
            value: self.builder.constant(Constant::Float(kind, bits))?,
            ty,
            source,
            literal: false,
            bits: None,
        }))
    }
    fn folded_value(
        &mut self,
        id: ast::ExprId,
        ty: Type,
        source: crate::sema::lowering::TypeId,
        bits: u64,
    ) -> Result<RValue, Error> {
        if let ExprKind::Member { base, op, .. } = self.tree.expressions.get(id).kind {
            self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
        }
        if self.graph.lower_member_pointer_type(source).is_some() {
            return self.zero(source);
        }
        Ok(RValue {
            value: self.builder.integer(
                if ty != Type::I128 || types::signed(self.graph, source)? {
                    i128::from(bits.cast_signed()).cast_unsigned()
                } else {
                    u128::from(bits)
                },
            )?,
            ty,
            source,
            literal: false,
            bits: Some(bits),
        })
    }
    fn literal_value(
        &mut self,
        literal: ast::Literal,
        ty: Type,
        source: crate::sema::lowering::TypeId,
    ) -> Result<RValue, Error> {
        let (PostTokenKind::Literal { data, .. } | PostTokenKind::Character { data, .. }) =
            literal.kind
        else {
            return Err(Error::Missing("string and user-defined literal lowering"));
        };
        let bits = data
            .as_slice()
            .iter()
            .enumerate()
            .fold(0_u128, |bits, (index, &byte)| {
                bits | (u128::from(byte) << (index * 8))
            });
        let constant = if let Some(kind) = FloatKind::from_type(ty) {
            Constant::Float(kind, bits)
        } else {
            Constant::Integer(if ty == Type::I64 {
                i128::from(
                    u64::try_from(bits)
                        .map_err(|_| crate::lowir::Error::Limit)?
                        .cast_signed(),
                )
                .cast_unsigned()
            } else {
                bits
            })
        };
        Ok(RValue {
            value: self.builder.constant(constant)?,
            ty,
            source,
            literal: true,
            bits: if ty.float_rank() == 0 {
                Some(u64::try_from(bits).map_err(|_| crate::lowir::Error::Limit)?)
            } else {
                None
            },
        })
    }
    fn expression_value(&mut self, id: ast::ExprId, context: Context) -> Result<RValue, Error> {
        self.check_stack()?;
        let facts = self.graph.lower_expression(id)?;
        let source = facts.ty;
        if self.graph.lower_type_identity(id) {
            return self.type_identity_value(id);
        }
        if self.graph.lower_builtin_operand(id)
            && let Some((call, _)) = self.graph.lower_function_conversion(id)?
        {
            return self.call_conversion(id, call);
        }
        if self.graph.lower_literal_call(id)?.is_some() {
            let value = self.literal_call(id)?;
            return if facts.is_place
                && !matches!(
                    self.graph.lower_type(source)?,
                    TypeView::Array { .. } | TypeView::Function { .. }
                ) {
                self.read_place(value.value, source)
            } else {
                Ok(value)
            };
        }
        if self.graph.lower_operator_call(id)? {
            let value = self.operator_call(id)?;
            return if facts.is_place
                && !matches!(
                    self.graph.lower_type(source)?,
                    crate::sema::lowering::TypeView::Function { .. }
                        | crate::sema::lowering::TypeView::Array { .. }
                ) {
                self.read_place(value.value, source)
            } else {
                Ok(value)
            };
        }
        if facts.is_bit_field()
            && facts.is_place
            && !matches!(
                self.tree.expressions.get(self.without_parentheses(id)).kind,
                ExprKind::Conditional { .. }
            )
            || matches!(self.tree.expressions.get(id).kind, ExprKind::Postfix { operand, .. } if self.graph.lower_expression(operand)?.is_bit_field())
        {
            return self.bit_field_value(id);
        }
        if matches!(
            self.graph.lower_type(source)?,
            crate::sema::lowering::TypeView::Function { .. }
                | crate::sema::lowering::TypeView::Array { .. }
        ) {
            let place = self.scalar_place(id)?;
            if matches!(
                place,
                crate::lowir::model::Value::Symbol(_) | crate::lowir::model::Value::Slot(_)
            ) {
                return self.emit(I::Addr(place), source);
            }
            return Ok(RValue {
                value: place,
                ty: Type::Ptr,
                source,
                literal: false,
                bits: None,
            });
        }
        if facts.is_place
            && matches!(
                self.tree.expressions.get(id).kind,
                ExprKind::Cast { .. } | ExprKind::FunctionalCast { .. } | ExprKind::Call { .. }
            )
        {
            let place = self.scalar_place(id)?;
            return self.read_place(place, source);
        }
        let ty = types::storage(self.graph, source)?;
        if let Some(bits) = facts.query {
            let value = self.builder.integer(u128::from(bits))?;
            return self.emit(I::Const(ty, value), source);
        }
        if let Some(bits) = facts.folded {
            return self.folded_value(id, ty, source, bits);
        }
        if let Some(value) = self.evaluated_wide(id, ty, source)? {
            return Ok(value);
        }
        if let Some(pointer) = self.evaluated_pointer(id)? {
            return Ok(pointer);
        }
        if let Some(value) = self.graph.lower_evaluated_member_pointer(self.tree, id)? {
            if let ExprKind::Member { base, op, .. } =
                self.tree.expressions.get(self.without_parentheses(id)).kind
            {
                self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
            }
            return self.member_pointer_constant(value);
        }
        if let Some(value) = self.evaluated_floating(id)? {
            return Ok(value);
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::Statement(body) => self.statement_expression(id, body, None),
            ExprKind::Throw(operand) => self.throw_value(id, operand),
            ExprKind::Paren(inner) => self.expression(inner, context),
            ExprKind::Literal(literal) => self.literal_value(literal, ty, source),
            ExprKind::Keyword(keyword @ (Keyword::True | Keyword::False | Keyword::Nullptr)) => {
                let bits = u64::from(keyword == Keyword::True);
                Ok(RValue {
                    value: self.builder.integer(u128::from(bits))?,
                    // keyword_literal emits ptr for nullptr, independently
                    // of nullptr_t's i64 object storage and ABI boundary.
                    ty: if keyword == Keyword::Nullptr {
                        Type::Ptr
                    } else {
                        ty
                    },
                    source,
                    literal: true,
                    bits: Some(bits),
                })
            }
            ExprKind::Keyword(Keyword::This) => self.object_pointer(source),
            ExprKind::New { .. } => self.allocation(id),
            ExprKind::Delete { .. } => self.deletion(id),
            ExprKind::Id(_) | ExprKind::Subscript { .. } | ExprKind::Member { .. } => {
                let place = self.scalar_place(id)?;
                self.read_place(place, source)
            }
            ExprKind::Unary { op, operand } => self.unary_expression(id, op, operand),
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } => self.binary_expression(id, op, left, right, context),
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: true,
            } => {
                let (place, value) = self.assignment_value(
                    id,
                    op.token
                        .as_punctuator()
                        .ok_or(Error::Missing("assignment operator"))?,
                    left,
                    right,
                )?;
                self.updated_value(place, value)
            }
            ExprKind::Postfix { op, operand } => Ok(self
                .increment(
                    operand,
                    op.token.as_punctuator() == Some(P::Decrement),
                    false,
                )?
                .1),
            ExprKind::Cast { operand, .. } => {
                if self.graph.lower_runtime_cast(id)?.is_some() {
                    return self.dynamic_cast_value(id, operand);
                }
                if ty == Type::Void {
                    self.discard_to_void(operand)?;
                    return self.zero(source);
                }
                let value = self.operand(operand, Context::Cast)?;
                if let Some(adjustment) = self.graph.lower_member_pointer_cast(id)? {
                    return self.convert_member_pointer(value, source, adjustment);
                }
                if let Some((delta, project_zero)) = self.graph.lower_cast_base_adjustment(id)?
                    && (!delta.is_zero() || project_zero)
                {
                    return self.convert_base_pointer(
                        value,
                        source,
                        delta,
                        !delta.is_zero() && !self.nonnull_pointer(operand),
                    );
                }
                let value = self.convert(value, source, Context::Cast)?;
                self.fit(value, source)
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => self.functional_cast(source, arguments, braced),
            ExprKind::Call { arguments, .. } if self.graph.lower_functional_call(id)? => {
                self.functional_cast(source, arguments, None)
            }
            ExprKind::Call { callee, arguments } => self.call(id, callee, arguments),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => self.conditional_value(id, condition, then_expr, else_expr),
            ExprKind::Braced { elements, .. } => {
                if let Some(&id) = self.tree.expr_lists.get(elements).first() {
                    let value = self.operand(id, Context::Initialization)?;
                    self.fit(value, source)
                } else {
                    self.zero(source)
                }
            }
            _ => Err(Error::Missing(
                "call, conditional and aggregate expression lowering",
            )),
        }
    }
    fn binary_expression(
        &mut self,
        id: ast::ExprId,
        op: ast::Operator,
        left: ast::ExprId,
        right: ast::ExprId,
        context: Context,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let op = op
            .token
            .as_punctuator()
            .ok_or(Error::Missing("non-punctuator binary lowering"))?;
        if op == P::Comma {
            self.discard(left)?;
            return self.operand(right, context);
        }
        if matches!(op, P::LogicalAnd | P::LogicalOr) {
            return self.logical_value(id);
        }
        if matches!(op, P::DotStar | P::ArrowStar) {
            let place = self.member_pointer_place(id, left, right, op == P::ArrowStar)?;
            return self.read_place(place, source);
        }
        if matches!(op, P::Add | P::Subtract)
            && (self
                .pointer_element(self.graph.lower_builtin_operand_type(left)?)?
                .is_some()
                || self
                    .pointer_element(self.graph.lower_builtin_operand_type(right)?)?
                    .is_some())
        {
            return self.pointer_arithmetic(id, left, right, op == P::Subtract);
        }
        let comparison = matches!(
            op,
            P::Equal | P::NotEqual | P::Less | P::LessEqual | P::Greater | P::GreaterEqual
        );
        let context = if comparison {
            Context::Comparison
        } else if matches!(op, P::Add | P::Subtract) {
            Context::Additive
        } else {
            Context::Operand
        };
        // The frozen comparison owner adjusts a base pointer before evaluating
        // the right operand; ordinary scalar conversions follow both evaluations.
        let early_base = comparison
            && (self.graph.lower_base_conversion(left)?.is_some()
                || self.graph.lower_member_pointer_conversion(left)?.is_some());
        let a = self.expression(left, context)?;
        let a = if early_base {
            self.apply_conversion(left, a, context)?
        } else {
            a
        };
        let b = self.expression(right, context)?;
        let a = if early_base {
            a
        } else {
            self.apply_conversion(
                left,
                a,
                if context == Context::Additive && self.is_size_query(right)? {
                    Context::BesideSize
                } else {
                    context
                },
            )?
        };
        let b = self.apply_conversion(
            right,
            b,
            if context == Context::Additive && self.is_size_query(left)? {
                Context::BesideSize
            } else {
                context
            },
        )?;
        let b = if matches!(op, P::ShiftLeft | P::ShiftRight) {
            self.convert(b, source, Context::Operand)?
        } else {
            b
        };
        if comparison {
            let unsigned = !types::signed(self.graph, a.source)? && a.ty.float_rank() == 0;
            let predicate = match (op, unsigned) {
                (P::Equal, _) => Compare::Eq,
                (P::NotEqual, _) => Compare::Ne,
                (P::Less, false) => Compare::Lt,
                (P::Less, true) => Compare::Ult,
                (P::LessEqual, false) => Compare::Le,
                (P::LessEqual, true) => Compare::Ule,
                (P::Greater, false) => Compare::Gt,
                (P::Greater, true) => Compare::Ugt,
                (P::GreaterEqual, false) => Compare::Ge,
                _ => Compare::Uge,
            };
            return self.emit(I::Compare(predicate, a.ty, a.value, b.value), source);
        }
        let signed = types::signed(self.graph, a.source)?;
        let operation = match op {
            P::Add => Binary::Add,
            P::Subtract => Binary::Sub,
            P::Multiply => Binary::Mul,
            P::Divide if signed || a.ty.float_rank() != 0 => Binary::Div,
            P::Divide => Binary::UDiv,
            P::Remainder if signed => Binary::Mod,
            P::Remainder => Binary::UMod,
            P::BitAnd => Binary::And,
            P::BitOr => Binary::Or,
            P::Xor => Binary::Xor,
            P::ShiftLeft => Binary::Shl,
            P::ShiftRight if signed => Binary::Shr,
            P::ShiftRight => Binary::UShr,
            _ => return Err(Error::Missing("logical expression lowering")),
        };
        if a.ty == Type::Ptr {
            return Err(Error::Missing("pointer arithmetic lowering"));
        }
        let nowrap = signed
            && matches!(a.ty, Type::I32 | Type::I64)
            && matches!(operation, Binary::Add | Binary::Sub | Binary::Mul);
        self.emit(
            I::Binary(
                operation,
                Facts::arithmetic(nowrap, false),
                a.ty,
                a.value,
                b.value,
            ),
            source,
        )
    }
    fn is_size_query(&self, mut id: ast::ExprId) -> Result<bool, Error> {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        Ok(!self
            .graph
            .lower_expression(id)?
            .query_kind
            .is_specialization()
            && matches!(
                self.tree.expressions.get(id).kind,
                ExprKind::Sizeof { .. }
                    | ExprKind::SizeofPack(_)
                    | ExprKind::TypeTrait {
                        keyword: Keyword::Alignof,
                        ..
                    }
            ))
    }
    fn unary_expression(
        &mut self,
        id: ast::ExprId,
        op: ast::Operator,
        operand: ast::ExprId,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        match op.token.as_punctuator() {
            Some(P::Increment | P::Decrement) => {
                let (place, value) = self.increment(
                    operand,
                    op.token.as_punctuator() == Some(P::Decrement),
                    true,
                )?;
                return self.updated_value(place, value);
            }
            Some(P::Multiply) => {
                let place = self.scalar_place(id)?;
                return self.read_place(place, source);
            }
            Some(P::BitAnd) => {
                if let Some(function) = self.graph.lower_member_function_address(id)? {
                    return self.member_function_word(function, source, 0);
                }
                if let Some(offset) = self.graph.lower_data_member_address(id)? {
                    let offset = self.builder.integer(u128::from(offset))?;
                    return self.emit(I::Const(Type::I64, offset), source);
                }
                let place = self.scalar_place(operand)?;
                if matches!(
                    place,
                    crate::lowir::model::Value::Slot(_) | crate::lowir::model::Value::Symbol(_)
                ) {
                    return self.emit(I::Addr(place), source);
                }
                return Ok(RValue {
                    value: place,
                    ty: Type::Ptr,
                    source,
                    literal: false,
                    bits: None,
                });
            }
            _ => {}
        }
        let value = if op.token.as_punctuator() == Some(P::LogicalNot) {
            // The boolean conversion fact belongs to semantic checking. C++
            // emits ! as one equality comparison on the raw truth operand.
            if let Some((call, _)) = self.graph.lower_function_conversion(operand)? {
                self.call_truth_conversion(operand, call)?
            } else {
                self.expression(operand, Context::Unary)?
            }
        } else {
            self.operand(operand, Context::Unary)?
        };
        match op.token.as_punctuator() {
            Some(P::Add) => Ok(value),
            Some(P::Subtract) => self.emit(I::Unary(Unary::Neg, value.ty, value.value), source),
            Some(P::Complement) => {
                self.emit(I::Unary(Unary::BitNot, value.ty, value.value), source)
            }
            Some(P::LogicalNot) => {
                let member = self.graph.lower_member_pointer_type(value.source).is_some();
                let value = if member {
                    self.member_pointer_truth(value)?
                } else {
                    value
                };
                let zero = if member {
                    self.builder.integer(0)?
                } else {
                    self.zero(value.source)?.value
                };
                self.emit(I::Compare(Compare::Eq, value.ty, value.value, zero), source)
            }
            _ => Err(Error::Missing("address and increment expression lowering")),
        }
    }
    fn functional_cast(
        &mut self,
        source: crate::sema::lowering::TypeId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        braced: Option<ast::ExprId>,
    ) -> Result<RValue, Error> {
        if let Some(braced) = braced {
            let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind else {
                return Err(crate::sema::Error::Invariant("functional cast without list").into());
            };
            // Scalar list initialization records the conversion on its element;
            // the enclosing list is syntax, with no expression facts of its own.
            return if let Some(&element) = self.tree.expr_lists.get(elements).first() {
                let value = self.operand(element, Context::Cast)?;
                self.fit(value, source)
            } else {
                self.zero(source)
            };
        }
        let operand = self.tree.expr_lists.get(arguments).first().copied();
        if let Some(operand) = operand {
            if types::storage(self.graph, source)? == Type::Void {
                self.discard_to_void(operand)?;
                return self.zero(source);
            }
            let value = self.operand(operand, Context::Cast)?;
            let value = self.convert(value, source, Context::Cast)?;
            self.fit(value, source)
        } else {
            self.zero(source)
        }
    }
    pub(super) fn call(
        &mut self,
        id: ast::ExprId,
        callee_expression: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
    ) -> Result<RValue, Error> {
        self.call_into(id, callee_expression, arguments, None)
    }
    pub(super) fn call_into(
        &mut self,
        id: ast::ExprId,
        callee_expression: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        destination: Option<crate::lowir::model::Value>,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        if let Some(kind) = self.graph.lower_floating_intrinsic(id)? {
            return self.floating_intrinsic(id, arguments, kind);
        }
        if self.graph.lower_destructor_evaluation_only(id)? {
            let callee = self.without_parentheses(callee_expression);
            let ExprKind::Member { base, op, .. } = self.tree.expressions.get(callee).kind else {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "destructor call without member",
                )));
            };
            if op.token.as_punctuator() == Some(P::Arrow) {
                self.arrow_operand(base)?;
            } else {
                self.address(base)?;
            }
            return self.zero(source);
        }
        let target = self.graph.lower_call(id)?;
        if let CallTarget::Direct(entity) = target
            && let Some(invoker) = self.graph.lower_closure_converter(entity)
        {
            return self.explicit_closure_conversion(callee_expression, invoker, source);
        }
        let invoker = self.graph.lower_closure_invocation(id);
        if let CallTarget::Direct(entity) = target
            && matches!(
                self.graph.lower_function(entity)?.builtin,
                Some(crate::sema::lowering::BuiltinFunction::Expect)
            )
        {
            let [value, expected] = *self.tree.expr_lists.get(arguments) else {
                return Err(
                    crate::sema::Error::Invariant("expect builtin without two arguments").into(),
                );
            };
            let value = self.operand(value, Context::Argument)?;
            let value = self.convert(value, source, Context::Argument)?;
            self.discard(expected)?;
            return Ok(value);
        }
        if let CallTarget::Direct(entity) = target
            && matches!(
                self.graph.lower_function(entity)?.builtin,
                Some(crate::sema::lowering::BuiltinFunction::Unreachable)
            )
        {
            self.effect(I::Unreachable)?;
            self.terminated = true;
            return self.zero(source);
        }
        let throws = self.lifetime_call_throws(target)?;
        self.begin_argument_call(throws, self.tree.expr_lists.get(arguments))?;
        let function_type = match target {
            CallTarget::Direct(entity) => {
                let facts = self.graph.lower_function(entity)?;
                facts.ty
            }
            CallTarget::Indirect(ty, _) => ty,
        };
        let TypeView::Function {
            result,
            parameters,
            variadic,
        } = self.graph.lower_type(function_type)?
        else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "call signature missing",
            )));
        };
        let boundary = self.symbols.result_boundary(result)?;
        let ty = boundary.ty;
        let supplied = self.tree.expr_lists.get(arguments);
        let member_access = self
            .graph
            .lower_bound_member_pointer(callee_expression)?
            .then(|| self.without_parentheses(callee_expression));
        let mut values = Vec::with_capacity(parameters.len() + 1);
        if boundary.indirect {
            values.push(destination.ok_or(crate::sema::Error::Invariant(
                "class call without destination",
            ))?);
        }
        if let Some(access) = member_access {
            let ExprKind::Binary { left, op, .. } = self.tree.expressions.get(access).kind else {
                return Err(crate::sema::Error::Invariant("bound member access syntax").into());
            };
            values.push(self.member_pointer_object(
                access,
                left,
                op.token.as_punctuator() == Some(P::ArrowStar),
            )?);
        }
        if let CallTarget::Direct(entity) = target {
            let receiver = if self.graph.lower_callable_object_call(id)? {
                id
            } else {
                callee_expression
            };
            let object = self.graph.lower_object_source(receiver)?;
            if self.graph.lower_function(entity)?.object.is_some() && invoker.is_none() {
                let callable = self.graph.lower_callable_object_call(id)?;
                if callable {
                    self.call_object_class = None;
                }
                let value = match object.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "member call without receiver",
                )))? {
                    crate::sema::lowering::ObjectSource::Implicit => {
                        self.object_pointer(source)?.value
                    }
                    crate::sema::lowering::ObjectSource::Explicit { expression, arrow } => {
                        if arrow {
                            self.arrow_operand(expression)?.value
                        } else {
                            self.address(expression)?
                        }
                    }
                };
                let mut value = value;
                for offset in self
                    .graph
                    .lower_receiver_projections(receiver)?
                    .into_iter()
                    .flatten()
                {
                    value = self.base_projection(value, source, offset)?;
                }
                if !callable {
                    self.call_object_class = self.graph.lower_receiver_class(receiver)?;
                }
                values.push(value);
            } else if invoker.is_none()
                && let Some(crate::sema::lowering::ObjectSource::Explicit { expression, arrow }) =
                    object
            {
                self.member_object(expression, arrow)?;
            }
        }
        for (index, &parameter) in parameters.iter().enumerate() {
            let argument = supplied
                .get(index)
                .copied()
                .or_else(|| match target {
                    CallTarget::Direct(entity) => self.graph.lower_default(entity, index),
                    CallTarget::Indirect(_, _) => None,
                })
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "call argument missing",
                )))?;
            if matches!(
                self.graph.lower_type(parameter)?,
                TypeView::Reference { .. }
            ) {
                values.push(self.bind_reference(argument, parameter, Context::Argument)?);
            } else if matches!(self.graph.lower_type(parameter)?, TypeView::Class(_)) {
                values.push(self.class_argument(argument, parameter)?);
            } else {
                // argument() applies its recorded standard sequence to the
                // raw rvalue; operand()'s member-address folding belongs to
                // expression operands, not this call boundary.
                let value = if self.graph.lower_function_conversion(argument)?.is_some() {
                    self.operand(argument, Context::Argument)?
                } else {
                    let value = self.expression(argument, Context::Argument)?;
                    self.apply_conversion(argument, value, Context::Argument)?
                };
                let value = self.fit(value, parameter)?;
                values.push(value.value);
            }
        }
        if variadic {
            for &argument in supplied.iter().skip(parameters.len()) {
                values.push(self.ellipsis_argument(argument)?);
            }
        }
        let (callee, signature) = match target {
            CallTarget::Direct(entity) => {
                let dispatch = self.graph.lower_virtual_dispatch(id)?;
                if let Some(invoker) = invoker {
                    self.symbols.demand(invoker);
                    let symbol = self
                        .symbols
                        .function_symbol(self.builder.program(), invoker)?;
                    let value = self
                        .emit(I::Addr(crate::lowir::model::Value::Symbol(symbol)), source)?
                        .value;
                    let signature = self.symbols.call_signature(
                        self.builder.program(),
                        self.graph.lower_function(invoker)?.ty,
                        self.graph.lower_function(invoker)?.nothrow,
                    )?;
                    (value, Some(signature))
                } else if dispatch {
                    let receiver = *values.get(usize::from(boundary.indirect)).ok_or(
                        crate::sema::Error::Invariant("virtual call without receiver"),
                    )?;
                    let callee = self.member_callee(entity, receiver, true, source)?;
                    self.demand_virtual_call_table()?;
                    callee
                } else {
                    self.symbols.demand(entity);
                    (self.symbols.callee(entity, &mut self.builder)?, None)
                }
            }
            CallTarget::Indirect(function_type, nothrow) => {
                if let Some(access) = member_access {
                    self.bound_pointer_call(
                        access,
                        &mut values[usize::from(boundary.indirect)],
                        nothrow,
                    )?
                } else {
                    let callee = self.operand(callee_expression, Context::Operand)?.value;
                    let signature = self.symbols.call_signature(
                        self.builder.program(),
                        function_type,
                        nothrow,
                    )?;
                    (callee, Some(signature))
                }
            }
        };
        let arguments = self.builder.arguments(&values)?;
        let call = I::Call(crate::lowir::model::Call {
            result: ty,
            callee,
            arguments,
            copy_elision: false,
            signature,
        });
        self.before_lifetime_call(throws)?;
        self.finish_result_call(call, source, destination, throws)
    }
}
