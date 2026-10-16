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
                && let Some(offset) = self.graph.lower_base_conversion_offset(id)?
            {
                return self.adjust_base_pointer(value, target, offset, offset != 0);
            }
            return self.convert(value, target, context);
        }
        let value = self.expression(id, context)?;
        self.apply_conversion(id, value, context)
    }
    fn apply_conversion(
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
            if context != Context::Cast
                && let Some(offset) = self.graph.lower_base_conversion_offset(id)?
            {
                return self.adjust_base_pointer(
                    value,
                    target,
                    offset,
                    !self.nonnull_pointer(id) && offset != 0,
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
            ExprKind::Id(name) if integral && facts.folded.is_some() => {
                self.graph.lower_static_member(self.graph.lower_name(name)?)
            }
            ExprKind::Member { .. } if integral && facts.folded.is_some() => self
                .graph
                .lower_static_member(self.graph.lower_member(id)?.0),
            _ => false,
        })
    }
    fn expression_value(&mut self, id: ast::ExprId, context: Context) -> Result<RValue, Error> {
        self.check_stack()?;
        let facts = self.graph.lower_expression(id)?;
        let source = facts.ty;
        if self.graph.lower_builtin_operand(id)
            && let Some((call, _)) = self.graph.lower_function_conversion(id)?
        {
            return self.call_conversion(id, call);
        }
        if self.graph.lower_literal_call(id)?.is_some() {
            let value = self.text_literal_call(id)?;
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
            if let ExprKind::Member { base, op, .. } = self.tree.expressions.get(id).kind {
                self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
            }
            return Ok(RValue {
                value: self.builder.integer(u128::from(bits))?,
                ty,
                source,
                literal: false,
                bits: Some(bits),
            });
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::Paren(inner) => self.expression(inner, context),
            ExprKind::Literal(literal) => {
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
                    Constant::Integer(bits)
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
                if ty == Type::Void {
                    self.discard_to_void(operand)?;
                    return self.zero(source);
                }
                let value = self.operand(operand, Context::Cast)?;
                if let Some((delta, project_zero)) = self.graph.lower_cast_base_adjustment(id)?
                    && (delta != 0 || project_zero)
                {
                    return self.adjust_base_pointer(
                        value,
                        source,
                        delta,
                        delta != 0 && !self.nonnull_pointer(operand),
                    );
                }
                self.convert(value, source, Context::Cast)
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
            } => self.conditional(id, condition, then_expr, else_expr),
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
        let early_base = comparison && self.graph.lower_base_conversion_offset(left)?.is_some();
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
                if context == Context::Additive && self.is_size_query(right) {
                    Context::BesideSize
                } else {
                    context
                },
            )?
        };
        let b = self.apply_conversion(
            right,
            b,
            if context == Context::Additive && self.is_size_query(left) {
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
    fn is_size_query(&self, mut id: ast::ExprId) -> bool {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        matches!(
            self.tree.expressions.get(id).kind,
            ExprKind::Sizeof { .. }
                | ExprKind::TypeTrait {
                    keyword: Keyword::Alignof,
                    ..
                }
        )
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
                let zero = self.zero(value.source)?;
                self.emit(
                    I::Compare(Compare::Eq, value.ty, value.value, zero.value),
                    source,
                )
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
        let operand = braced.or_else(|| self.tree.expr_lists.get(arguments).first().copied());
        if let Some(operand) = operand {
            if braced.is_none() && types::storage(self.graph, source)? == Type::Void {
                self.discard_to_void(operand)?;
                return self.zero(source);
            }
            let value = self.operand(operand, Context::Cast)?;
            self.convert(value, source, Context::Cast)
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
        let mut values = Vec::with_capacity(parameters.len() + 1);
        if boundary.indirect {
            values.push(destination.ok_or(crate::sema::Error::Invariant(
                "class call without destination",
            ))?);
        }
        if let CallTarget::Direct(entity) = target {
            let object = self.graph.lower_object_source(callee_expression)?;
            if self.graph.lower_function(entity)?.object.is_some() {
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
                    .lower_receiver_base_offsets(callee_expression)?
                    .into_iter()
                    .flatten()
                {
                    value = self.base_projection(value, source, offset)?;
                }
                values.push(value);
            } else if let Some(crate::sema::lowering::ObjectSource::Explicit {
                expression,
                arrow,
            }) = object
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
                let value = self.operand(argument, Context::Argument)?;
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
                if dispatch {
                    let receiver = *values.get(usize::from(boundary.indirect)).ok_or(
                        crate::sema::Error::Invariant("virtual call without receiver"),
                    )?;
                    self.member_callee(entity, receiver, true, source)?
                } else {
                    self.symbols.demand(entity);
                    (self.symbols.callee(entity, &mut self.builder)?, None)
                }
            }
            CallTarget::Indirect(function_type, nothrow) => {
                let callee = self.operand(callee_expression, Context::Operand)?.value;
                let signature =
                    self.symbols
                        .call_signature(self.builder.program(), function_type, nothrow)?;
                (callee, Some(signature))
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

    fn conditional(
        &mut self,
        id: ast::ExprId,
        condition: ast::ExprId,
        then_expr: ast::ExprId,
        else_expr: ast::ExprId,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let ty = types::storage(self.graph, source)?;

        if ty == Type::Void {
            self.discard(id)?;
            return self.zero(source);
        }
        let slot = self.generated_slot("cond", ty)?;
        let then_name = self.block_name("cond_then")?;
        let else_name = self.block_name("cond_else")?;
        let end_name = self.block_name("cond_end")?;
        let then_block = self.builder.block_id(&then_name)?;
        let else_block = self.builder.block_id(&else_name)?;
        let end_block = self.builder.block_id(&end_name)?;
        let guard_start = self.lifetime.guard_start;
        if guard_start.is_none() {
            self.lifetime.guard_start = Some(self.builder.position()?);
        }
        let condition = self.truth(condition)?;
        let condition = if condition.ty.float_rank() != 0 {
            let zero = self.zero(condition.source)?;
            self.emit(
                I::Compare(Compare::Ne, condition.ty, condition.value, zero.value),
                condition.source,
            )?
        } else {
            condition
        };
        self.effect(I::Branch(condition.value, then_block, else_block))?;
        let local = self.branch_local(id);
        let opened = self.lifetime.regions_opened;
        for (index, (name, expression)) in [(&then_name, then_expr), (&else_name, else_expr)]
            .into_iter()
            .enumerate()
        {
            self.builder.block(name)?;
            if index != 0
                && self.lifetime.full_expression
                && !self.lifetime.region
                && self.lifetime.regions_opened != opened
                && !self.lifetime.tracking.active
                && (!self.lifetime.temporary_segment.allows_eager_arguments()
                    || self.expression_throws(expression)?
                    || self.expression_temporaries(expression)?.made)
            {
                let position = self.builder.position()?;
                self.open_temporary_region_at(Some(position))?;
            }
            let first = self.lifetime.cleanups.len();
            self.lifetime.guarded_operands += u32::from(!local);
            let value = self.operand(expression, Context::Operand)?;
            self.lifetime.guarded_operands -= u32::from(!local);
            let value = self.fit(value, source)?;
            self.effect(I::Store(
                types::memory(self.graph, source)?,
                ty,
                value.value,
                slot,
            ))?;
            if local {
                self.destroy_temporaries(first)?;
            }
            self.effect(I::Jump(end_block))?;
        }
        self.builder.block(&end_name)?;
        if guard_start.is_none() {
            self.lifetime.guard_start = None;
        }
        self.conditional_slot = Some(slot);
        self.emit(
            I::Load(types::memory(self.graph, source)?, ty, slot),
            source,
        )
    }
}
