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
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn operand(&mut self, id: ast::ExprId, context: Context) -> Result<RValue, Error> {
        self.check_stack()?;
        let facts = self.graph.lower_expression(id)?;
        let value = self.expression(id, context)?;
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
            return self.zero(target);
        }
        if let Some(target) = facts.target {
            self.convert(value, target, context)
        } else {
            Ok(value)
        }
    }
    fn expression(&mut self, id: ast::ExprId, context: Context) -> Result<RValue, Error> {
        let facts = self.graph.lower_expression(id)?;
        let source = facts.ty;
        let ty = types::storage(self.graph, source)?;
        if let Some(bits) = facts.folded.or(facts.query) {
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
            ExprKind::Keyword(Keyword::True | Keyword::False | Keyword::Nullptr) => {
                let bits = u64::from(matches!(
                    self.tree.expressions.get(id).kind,
                    ExprKind::Keyword(Keyword::True)
                ));
                Ok(RValue {
                    value: self.builder.integer(u128::from(bits))?,
                    ty,
                    source,
                    literal: true,
                    bits: Some(bits),
                })
            }
            ExprKind::Id(name) => {
                let entity = self.graph.lower_name(name)?;
                let place = self
                    .locals
                    .get(&entity)
                    .copied()
                    .ok_or(Error::Missing("global and reference value lowering"))?;
                self.emit(
                    I::Load(types::memory(self.graph, source)?, ty, place),
                    source,
                )
            }
            ExprKind::Unary { op, operand } => {
                let value = self.operand(operand, Context::Unary)?;
                match op.token.as_punctuator() {
                    Some(P::Add) => Ok(value),
                    Some(P::Subtract) => {
                        self.emit(I::Unary(Unary::Neg, value.ty, value.value), source)
                    }
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
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } => {
                let op = op
                    .token
                    .as_punctuator()
                    .ok_or(Error::Missing("non-punctuator binary lowering"))?;
                if op == P::Comma {
                    self.operand(left, Context::Operand)?;
                    return self.operand(right, context);
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
                let a = self.operand(left, context)?;
                let b = self.operand(right, context)?;
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
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: true,
            } => {
                if op.token.as_punctuator() != Some(P::Assign) {
                    return Err(Error::Missing("compound assignment lowering"));
                }
                let value = self.operand(right, Context::Assignment)?;
                let ExprKind::Id(name) = self.tree.expressions.get(left).kind else {
                    return Err(Error::Missing("computed assignment place lowering"));
                };
                let entity = self.graph.lower_name(name)?;
                let place = self
                    .locals
                    .get(&entity)
                    .copied()
                    .ok_or(Error::Missing("non-local assignment lowering"))?;
                let value = self.fit(value, source)?;
                self.builder.effect(I::Store(
                    types::memory(self.graph, source)?,
                    ty,
                    value.value,
                    place,
                ))?;
                Ok(value)
            }
            ExprKind::Cast { operand, .. } => {
                let value = self.operand(operand, Context::Operand)?;
                self.convert(value, source, Context::Cast)
            }
            ExprKind::Call { arguments, .. } => self.call(id, arguments),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => self.conditional(id, condition, then_expr, else_expr),
            ExprKind::Braced { elements, .. } => {
                if let Some(&id) = self.tree.expr_lists.get(elements).first() {
                    self.operand(id, context)
                } else {
                    self.zero(source)
                }
            }
            _ => Err(Error::Missing(
                "call, conditional and aggregate expression lowering",
            )),
        }
    }
    fn call(
        &mut self,
        id: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let ty = types::storage(self.graph, source)?;
        let entity = self.graph.lower_call(id)?;
        let facts = self.graph.lower_function(entity)?;
        if facts.definition.is_none() {
            return Err(Error::Missing("external and indirect call lowering"));
        }
        let crate::sema::lowering::TypeView::Function {
            parameters,
            variadic,
            ..
        } = self.graph.lower_type(facts.ty)?
        else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "call signature missing",
            )));
        };
        if variadic {
            return Err(Error::Missing("variadic call lowering"));
        }
        let supplied = self.tree.expr_lists.get(arguments);
        let mut values = Vec::with_capacity(parameters.len());
        for (index, &parameter) in parameters.iter().enumerate() {
            let argument = supplied
                .get(index)
                .copied()
                .or_else(|| self.graph.lower_default(entity, index))
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "call argument missing",
                )))?;
            let value = self.operand(argument, Context::Initialization)?;
            let value = self.fit(value, parameter)?;
            values.push(value.value);
        }
        self.symbols.demand(entity);
        let callee = self.symbols.callee(entity, &mut self.builder)?;
        let arguments = self.builder.arguments(&values)?;
        let call = I::Call(crate::lowir::model::Call {
            result: ty,
            callee,
            arguments,
            copy_elision: false,
            signature: None,
        });
        if ty == Type::Void {
            self.builder.effect(call)?;
            self.zero(source)
        } else {
            self.emit(call, source)
        }
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

        if self.graph.lower_expression(id)?.is_place {
            return Err(Error::Missing("conditional lvalue address lowering"));
        }
        self.next_slot = self
            .next_slot
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        let slot = self
            .builder
            .slot(&format!("cond__{}", self.next_slot), ty)?;
        let then_name = self.block_name("cond_then")?;
        let else_name = self.block_name("cond_else")?;
        let end_name = self.block_name("cond_end")?;
        let then_block = self.builder.block_id(&then_name)?;
        let else_block = self.builder.block_id(&else_name)?;
        let end_block = self.builder.block_id(&end_name)?;
        let condition = self.expression(condition, Context::Operand)?;
        let condition = if condition.ty.float_rank() != 0 {
            let zero = self.zero(condition.source)?;
            self.emit(
                I::Compare(Compare::Ne, condition.ty, condition.value, zero.value),
                condition.source,
            )?
        } else {
            condition
        };
        self.builder
            .effect(I::Branch(condition.value, then_block, else_block))?;
        for (name, expression) in [(&then_name, then_expr), (&else_name, else_expr)] {
            self.builder.block(name)?;
            let value = self.operand(expression, Context::Operand)?;
            let value = self.fit(value, source)?;
            self.builder.effect(I::Store(
                types::memory(self.graph, source)?,
                ty,
                value.value,
                slot,
            ))?;
            self.builder.effect(I::Jump(end_block))?;
        }
        self.builder.block(&end_name)?;
        self.emit(
            I::Load(types::memory(self.graph, source)?, ty, slot),
            source,
        )
    }
}
