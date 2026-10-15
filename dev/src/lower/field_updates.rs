// Translated from lower/aggregate_lowering.cpp::bit_field_value/update_bit_field. See NOTICE.
//! Resolve one designated unit, retain assignment operands, and preserve branch/update order.
use super::{
    Error,
    bit_fields::{Field, Unit},
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    lowir::{
        model::{Facts, InstructionKind as I},
        vocabulary::{Binary, Projection, Type},
    },
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy)]
enum UpdateKind {
    Assign,
    Compound(P),
    Increment { decrement: bool, prefix: bool },
}
#[derive(Clone, Copy)]
struct Update {
    kind: UpdateKind,
    expression: ast::ExprId,
    right: Option<RValue>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn bit_field_value(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        match self.tree.expressions.get(id).kind {
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard(left)?;
                self.bit_field_value(right)
            }
            ExprKind::Binary {
                op,
                left,
                assignment: true,
                ..
            } => {
                let op = op
                    .token
                    .as_punctuator()
                    .ok_or(Error::Missing("bit-field assignment operator"))?;
                let mut update = Update {
                    kind: if op == P::Assign {
                        UpdateKind::Assign
                    } else {
                        UpdateKind::Compound(op)
                    },
                    expression: id,
                    right: None,
                };
                self.update_field_target(left, &mut update)
            }
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                let prefix = matches!(self.tree.expressions.get(id).kind, ExprKind::Unary { .. });
                let mut update = Update {
                    kind: UpdateKind::Increment {
                        decrement: op.token.as_punctuator() == Some(P::Decrement),
                        prefix,
                    },
                    expression: id,
                    right: None,
                };
                self.update_field_target(operand, &mut update)
            }
            _ => {
                let (field, unit) = self.resolve_field(id)?.ok_or_else(|| {
                    self.field_diagnostic(id, "a bit-field glvalue outside the supported forms")
                })?;
                self.read_field(field, unit)
            }
        }
    }
    fn resolve_field(&mut self, id: ast::ExprId) -> Result<Option<(Field, Unit)>, Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        match self.tree.expressions.get(id).kind {
            ExprKind::Conditional { .. } => return Ok(None),
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard(left)?;
                return self.resolve_field(right);
            }
            ExprKind::Binary {
                op,
                left,
                assignment: true,
                ..
            } => {
                let op = op
                    .token
                    .as_punctuator()
                    .ok_or(Error::Missing("bit-field assignment operator"))?;
                let mut update = Update {
                    kind: if op == P::Assign {
                        UpdateKind::Assign
                    } else {
                        UpdateKind::Compound(op)
                    },
                    expression: id,
                    right: None,
                };
                let Some((field, unit)) = self.resolve_field(left)? else {
                    return Ok(None);
                };
                self.apply_field_update(field, unit, &mut update)?;
                return Ok(Some((field, unit)));
            }
            ExprKind::Unary { op, operand }
                if matches!(op.token.as_punctuator(), Some(P::Increment | P::Decrement)) =>
            {
                let mut update = Update {
                    kind: UpdateKind::Increment {
                        decrement: op.token.as_punctuator() == Some(P::Decrement),
                        prefix: true,
                    },
                    expression: id,
                    right: None,
                };
                let Some((field, unit)) = self.resolve_field(operand)? else {
                    return Ok(None);
                };
                self.apply_field_update(field, unit, &mut update)?;
                return Ok(Some((field, unit)));
            }
            ExprKind::Member { .. } | ExprKind::Id(_) => {}
            _ => {
                return Err(
                    self.field_diagnostic(id, "a bit-field glvalue outside the supported forms")
                );
            }
        }
        let entity = self.graph.lower_designated_bit_field(id)?;
        let field = self.field(entity)?;
        let source = self.graph.lower_expression(id)?.ty;
        let root = if let ExprKind::Member { base, op, .. } = self.tree.expressions.get(id).kind {
            if op.token.as_punctuator() == Some(P::Arrow) {
                self.arrow_operand(base)?.value
            } else {
                self.address(base)?
            }
        } else {
            self.object_pointer(source)?.value
        };
        let root = self.member_base_projection(id, root, source)?;
        let offset = self.builder.integer(u128::from(
            field
                .offset
                .wrapping_add(self.graph.lower_anonymous_field_offset(entity)?),
        ))?;
        let address = self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), root, offset),
                source,
            )?
            .value;
        Ok(Some((
            field,
            Unit {
                address,
                volatile: self.graph.lower_cv(source) & 2 != 0,
            },
        )))
    }
    pub(super) fn field_diagnostic(&self, id: ast::ExprId, message: &'static str) -> Error {
        Error::Semantic(crate::sema::Error::Diagnostic {
            location: self
                .tree
                .expressions
                .get(self.without_parentheses(id))
                .location,
            message,
        })
    }
    fn update_operand(
        &mut self,
        update: &mut Update,
        ty: crate::sema::lowering::TypeId,
    ) -> Result<Option<RValue>, Error> {
        if update.right.is_some() || matches!(update.kind, UpdateKind::Increment { .. }) {
            return Ok(update.right);
        }
        let ExprKind::Binary { right, .. } = self.tree.expressions.get(update.expression).kind
        else {
            return Err(Error::Missing("field assignment source"));
        };
        let context = if matches!(update.kind, UpdateKind::Compound(_)) {
            Context::CompoundAssignment
        } else {
            Context::BitFieldAssignment
        };
        let value = if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(right).kind
        {
            if let Some(&id) = self.tree.expr_lists.get(elements).first() {
                self.operand(id, context)?
            } else {
                self.zero(ty)?
            }
        } else {
            self.operand(right, context)?
        };
        update.right = Some(value);
        Ok(Some(value))
    }
    fn update_field_target(
        &mut self,
        id: ast::ExprId,
        update: &mut Update,
    ) -> Result<RValue, Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        if let ExprKind::Conditional {
            condition,
            then_expr,
            else_expr,
        } = self.tree.expressions.get(id).kind
        {
            let source = self.graph.lower_expression(id)?.ty;
            self.update_operand(update, source)?;
            let yes = self.new_block("cond_then")?;
            let no = self.new_block("cond_else")?;
            let end = self.new_block("cond_end")?;
            let condition = self.truth(condition)?;
            self.effect(I::Branch(condition.value, yes, no))?;
            self.place(yes)?;
            let first = self.update_field_target(then_expr, update)?;
            let slot = self.generated_slot("cond", first.ty)?;
            self.effect(I::Store(
                Facts::memory(false, !matches!(first.ty, Type::I8 | Type::U8)),
                first.ty,
                first.value,
                slot,
            ))?;
            self.jump(end)?;
            self.place(no)?;
            let mut second = self.update_field_target(else_expr, update)?;
            if second.ty != first.ty {
                second = self.emit(I::Copy(first.ty, second.value), source)?;
            }
            self.effect(I::Store(
                Facts::memory(false, !matches!(first.ty, Type::I8 | Type::U8)),
                first.ty,
                second.value,
                slot,
            ))?;
            self.jump(end)?;
            self.place(end)?;
            return self.emit(
                I::Load(
                    Facts::memory(false, !matches!(first.ty, Type::I8 | Type::U8)),
                    first.ty,
                    slot,
                ),
                source,
            );
        }
        if let ExprKind::Binary {
            op,
            left,
            right,
            assignment: false,
        } = self.tree.expressions.get(id).kind
            && op.token.as_punctuator() == Some(P::Comma)
        {
            self.discard(left)?;
            return self.update_field_target(right, update);
        }
        if !self.graph.lower_expression(id)?.is_bit_field() {
            return self.apply_object_field_update(id, update);
        }
        let (field, mut unit) = self.resolve_field(id)?.ok_or_else(|| {
            self.field_diagnostic(id, "a bit-field designator outside the supported forms")
        })?;
        if let UpdateKind::Increment {
            decrement,
            prefix: true,
        } = update.kind
            && self.field_side_effect_free(id, false, 0)?
        {
            let old = self.read_field(field, unit)?;
            let next = self.step_field(field, old, decrement)?;
            unit = self
                .resolve_field(id)?
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "repeated bit-field designator changed",
                )))?
                .1;
            let next = self.convert_field_unit(field, next)?;
            return self.write_field(field, unit, next);
        }
        self.apply_field_update(field, unit, update)
    }
    fn step_field(&mut self, field: Field, old: RValue, decrement: bool) -> Result<RValue, Error> {
        let widened = self.convert(old, field.operation, Context::CompoundAssignment)?;
        self.step_field_value(widened, decrement)
    }
    fn step_field_value(&mut self, old: RValue, decrement: bool) -> Result<RValue, Error> {
        if types::boolean(self.graph, old.source)? {
            return Ok(RValue {
                value: self.builder.integer(1)?,
                bits: Some(1),
                ..old
            });
        }
        let one = self.builder.integer(1)?;
        let nowrap =
            types::signed(self.graph, old.source)? && matches!(old.ty, Type::I32 | Type::I64);
        self.emit(
            I::Binary(
                if decrement { Binary::Sub } else { Binary::Add },
                Facts::arithmetic(nowrap, false),
                old.ty,
                old.value,
                one,
            ),
            old.source,
        )
    }
    fn apply_field_update(
        &mut self,
        field: Field,
        unit: Unit,
        update: &mut Update,
    ) -> Result<RValue, Error> {
        match update.kind {
            UpdateKind::Assign => {
                let value = self
                    .update_operand(update, field.source)?
                    .ok_or(Error::Missing("field assignment value"))?;
                let value = self.fit(value, field.source)?;
                let value = self.convert_field_unit(field, value)?;
                self.write_field(field, unit, value)
            }
            UpdateKind::Compound(op) => {
                let old = self.read_field(field, unit)?;
                let right = self
                    .update_operand(update, field.source)?
                    .ok_or(Error::Missing("field compound value"))?;
                let value = self.field_compound_value(update.expression, op, old, right)?;
                let value = self.convert_field_unit(field, value)?;
                self.write_field(field, unit, value)
            }
            UpdateKind::Increment { decrement, prefix } => {
                let old = self.read_field(field, unit)?;
                let next = self.step_field(field, old, decrement)?;
                let next = self.convert_field_unit(field, next)?;
                let written = self.write_field(field, unit, next)?;
                Ok(if prefix { written } else { old })
            }
        }
    }
    fn field_compound_value(
        &mut self,
        id: ast::ExprId,
        op: P,
        old: RValue,
        mut right: RValue,
    ) -> Result<RValue, Error> {
        let (operation, _) = self.graph.lower_arithmetic_types(id)?;
        let source = old.source;
        if matches!(op, P::ShiftLeftAssign | P::ShiftRightAssign) {
            right = self.convert(right, operation, Context::CompoundAssignment)?;
        }
        let left = self.convert(old, operation, Context::CompoundAssignment)?;
        let signed = types::signed(self.graph, operation)?;
        let code = match op {
            P::AddAssign => Binary::Add,
            P::SubtractAssign => Binary::Sub,
            P::MultiplyAssign => Binary::Mul,
            P::DivideAssign if signed || left.ty.float_rank() != 0 => Binary::Div,
            P::DivideAssign => Binary::UDiv,
            P::RemainderAssign if signed => Binary::Mod,
            P::RemainderAssign => Binary::UMod,
            P::AndAssign => Binary::And,
            P::OrAssign => Binary::Or,
            P::XorAssign => Binary::Xor,
            P::ShiftLeftAssign => Binary::Shl,
            P::ShiftRightAssign if signed => Binary::Shr,
            P::ShiftRightAssign => Binary::UShr,
            _ => return Err(Error::Missing("field compound operator")),
        };
        let nowrap = signed
            && matches!(left.ty, Type::I32 | Type::I64)
            && matches!(code, Binary::Add | Binary::Sub | Binary::Mul);
        let result = self.emit(
            I::Binary(
                code,
                Facts::arithmetic(nowrap, false),
                left.ty,
                left.value,
                right.value,
            ),
            operation,
        )?;
        let result = self.convert(result, source, Context::CompoundAssignment)?;
        self.fit(result, source)
    }
    fn apply_object_field_update(
        &mut self,
        id: ast::ExprId,
        update: &mut Update,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let place = self.scalar_place(id)?;
        let value = match update.kind {
            UpdateKind::Assign => {
                let value = self
                    .update_operand(update, source)?
                    .ok_or(Error::Missing("conditional assignment value"))?;
                self.fit(value, source)?
            }
            UpdateKind::Compound(op) => {
                let old = self.read_place(place, source)?;
                let right = self
                    .update_operand(update, source)?
                    .ok_or(Error::Missing("conditional compound value"))?;
                self.field_compound_value(update.expression, op, old, right)?
            }
            UpdateKind::Increment { decrement, prefix } => {
                let old = self.read_place(place, source)?;
                let next = self.step_field_value(old, decrement)?;
                self.effect(I::Store(
                    self.place_memory(place, source)?,
                    old.ty,
                    next.value,
                    place,
                ))?;
                return Ok(if prefix { next } else { old });
            }
        };
        self.effect(I::Store(
            self.place_memory(place, source)?,
            value.ty,
            value.value,
            place,
        ))?;
        Ok(value)
    }
    fn field_side_effect_free(
        &self,
        id: ast::ExprId,
        read: bool,
        depth: u8,
    ) -> Result<bool, Error> {
        if depth > 64 || read && self.graph.lower_cv(self.graph.lower_expression(id)?.ty) & 2 != 0 {
            return Ok(false);
        }
        Ok(
            match self.tree.expressions.get(self.without_parentheses(id)).kind {
                ExprKind::Id(_) | ExprKind::Literal(_) | ExprKind::Keyword(_) => true,
                ExprKind::Member { base, op, .. } => self.field_side_effect_free(
                    base,
                    op.token.as_punctuator() == Some(P::Arrow),
                    depth + 1,
                )?,
                ExprKind::Subscript { base, index } => {
                    let array = matches!(
                        self.graph
                            .lower_type(self.graph.lower_expression(base)?.ty)?,
                        crate::sema::lowering::TypeView::Array { .. }
                    );
                    self.field_side_effect_free(base, !array, depth + 1)?
                        && self.field_side_effect_free(index, true, depth + 1)?
                }
                _ => false,
            },
        )
    }
}
