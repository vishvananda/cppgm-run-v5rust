// Translated from sema/constant_expressions.cpp and constant_interpreter.cpp. See NOTICE.
//! Scalar effects operate on symbolic places in one evaluation's owned storage.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constants::Constant,
    graph::ScopeId,
    integral,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    syntax::ast::{self, ExprKind},
};

impl Analyzer<'_> {
    fn mutation_operand(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        self.constant_operand_value(id, target, scope)
    }

    pub(super) fn constant_assignment_place(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        op: Option<P>,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let location = self.tree.expressions.get(id).location;
        // Simple assignment evaluates and converts the RHS before designation.
        // Compound assignment designates the LHS once and reads it first.
        let (place, value) = if op == Some(P::Assign) {
            let target = self.graph.expression_facts.get(left)?.ty;
            let value =
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(right).kind {
                    if let Some(&element) = self.tree.expr_lists.get(elements).first() {
                        self.mutation_operand(element, target, scope)?
                    } else {
                        self.zero_constant_scalar(target)?
                    }
                } else {
                    self.mutation_operand(right, target, scope)?
                };
            (self.constant_place(left, scope)?, value)
        } else {
            self.compound_mutation(id, left, right, op, scope)?
        };
        self.write_constant_value(place, value, location)?;
        Ok(place)
    }

    fn compound_mutation(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        op: Option<P>,
        scope: ScopeId,
    ) -> Result<(Address, Value), Error> {
        let location = self.tree.expressions.get(id).location;
        let place = self.constant_place(left, scope)?;
        if self.is_address_expression(left) {
            let mut pointer = self.read_constant_address(place, location)?;
            let count = integral::signed(self.evaluate_constant(right, scope, false)?.bits);
            self.advance_address(
                &mut pointer,
                if op == Some(P::SubtractAssign) {
                    count.wrapping_neg()
                } else {
                    count
                },
                location,
            )?;
            return Ok((place, Value::Pointer(pointer)));
        }
        let (operation, _) = self.graph.arithmetic_types(id)?;
        if self.float_format(operation).is_some() {
            let old = self.read_constant_scalar(place, location)?;
            let right = self.scalar_value(right, scope)?;
            let op = match op {
                Some(P::AddAssign) => P::Add,
                Some(P::SubtractAssign) => P::Subtract,
                Some(P::MultiplyAssign) => P::Multiply,
                Some(P::DivideAssign) => P::Divide,
                _ => return Err(Error::Invariant("floating compound operation")),
            };
            let value = self.constant_numeric_operation(id, Some(op), old, right)?;
            return Ok((
                place,
                self.convert_constant_scalar(value, place.designated, location)?,
            ));
        }
        let old = self.read_constant_integer(place, location)?;
        let (operation, right_type) = self.graph.arithmetic_types(id)?;
        if self.is_address_expression(right) {
            let mut pointer = self.evaluate_address(right, scope)?;
            self.advance_address(&mut pointer, integral::signed(old.bits), location)?;
            return Ok((
                place,
                Value::Integer(Constant {
                    high: 0,
                    bits: u64::from(pointer.root != Root::Null),
                    ty: self.bare_type(place.designated)?,
                }),
            ));
        }
        let right = self.evaluate_constant(right, scope, false)?;
        let kind = self
            .integral_kind(operation)
            .ok_or(Error::Missing("floating compound constant evaluation"))?;
        let op = match op {
            Some(P::AddAssign) => P::Add,
            Some(P::SubtractAssign) => P::Subtract,
            Some(P::MultiplyAssign) => P::Multiply,
            Some(P::DivideAssign) => P::Divide,
            Some(P::RemainderAssign) => P::Remainder,
            Some(P::AndAssign) => P::BitAnd,
            Some(P::OrAssign) => P::BitOr,
            Some(P::XorAssign) => P::Xor,
            Some(P::ShiftLeftAssign) => P::ShiftLeft,
            Some(P::ShiftRightAssign) => P::ShiftRight,
            _ => return Err(Error::Invariant("unknown compound assignment")),
        };
        if kind.bits > 64 {
            let value = self.wide_arithmetic(op, old, right, (operation, right_type), false, id)?;
            let target = self.bare_type(place.designated)?;
            return Ok((
                place,
                Value::Integer(self.convert_integer_constant(value, target)?),
            ));
        }
        let (bits, valid) = if matches!(op, P::ShiftLeft | P::ShiftRight) {
            let count_kind = self
                .integral_kind(right_type)
                .ok_or(Error::Invariant("compound shift without integral count"))?;
            kind.shift(op, old.bits, right.bits, count_kind)
        } else {
            kind.operation(op, old.bits, right.bits)
                .ok_or(Error::Invariant("compound operation without arithmetic"))?
        };
        if !valid {
            return Err(nonconstant(
                location,
                "an arithmetic operation with an undefined result",
            ));
        }
        let target = self.bare_type(place.designated)?;
        let value = self.integral_value(operation, bits);
        Ok((
            place,
            Value::Integer(self.convert_integer_constant(value, target)?),
        ))
    }

    pub(super) fn constant_increment_values(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        decrement: bool,
        scope: ScopeId,
    ) -> Result<(Address, Value), Error> {
        let location = self.tree.expressions.get(id).location;
        let place = self.constant_place(operand, scope)?;
        let (old, stepped) = if self.is_address_expression(operand) {
            let old = self.read_constant_address(place, location)?;
            let mut pointer = old;
            self.advance_address(&mut pointer, if decrement { -1 } else { 1 }, location)?;
            (Value::Pointer(old), Value::Pointer(pointer))
        } else if let Some(format) = self.float_format(place.designated) {
            let old = self.read_constant_floating(place, location)?;
            let value = old
                .value
                .operation(
                    if decrement {
                        crate::types::float_value::Operation::Subtract
                    } else {
                        crate::types::float_value::Operation::Add
                    },
                    crate::types::float_value::BinaryFloat::integer(1, false),
                    format,
                )
                .ok_or_else(|| {
                    nonconstant(location, "a floating operation with an undefined result")
                })?;
            (
                Value::Floating(old),
                Value::Floating(super::constant_floats::Floating { ty: old.ty, value }),
            )
        } else {
            let old = self.read_constant_integer(place, location)?;
            let target = self.bare_type(place.designated)?;
            let target_kind = self
                .integral_kind(target)
                .ok_or(Error::Missing("floating increment constant evaluation"))?;
            if target_kind.bits > 64 {
                let op = if decrement { P::Subtract } else { P::Add };
                let (bits, valid) =
                    super::wide_integers::operation(op, target_kind.signed, old.wide(), 1)
                        .ok_or(Error::Invariant("wide increment operator"))?;
                if !valid {
                    return Err(nonconstant(location, "an increment that overflows"));
                }
                let stepped = Value::Integer(Constant::from_wide(target, bits));
                self.write_constant_value(place, stepped, location)?;
                return Ok((place, Value::Integer(old)));
            }
            let bits = if matches!(
                self.graph.types.get(target).kind,
                TypeKind::Scalar(Scalar::Bool)
            ) {
                1
            } else {
                let promoted = self.promoted_type(target)?;
                let kind = self
                    .integral_kind(promoted)
                    .ok_or(Error::Invariant("increment without integral promoted type"))?;
                let (bits, valid) = kind
                    .operation(if decrement { P::Subtract } else { P::Add }, old.bits, 1)
                    .ok_or(Error::Invariant("increment without integral operation"))?;
                if !valid {
                    return Err(nonconstant(location, "an increment that overflows"));
                }
                target_kind.convert(bits)
            };
            (
                Value::Integer(old),
                Value::Integer(Constant {
                    high: 0,
                    bits,
                    ty: target,
                }),
            )
        };
        self.write_constant_value(place, stepped, location)?;
        Ok((place, old))
    }
}
