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
        if let TypeKind::Pointer(element) = self.graph.types.get(target).kind {
            let mut value = self.address_operand(id, scope)?;
            value.designated = element;
            return Ok(Value::Pointer(value));
        }
        let target = self.bare_type(target)?;
        let kind = self
            .integral_kind(target)
            .ok_or(Error::Missing("nonintegral constant mutation operand"))?;
        let bits = if self.is_address_expression(id) {
            if !matches!(
                self.graph.types.get(target).kind,
                TypeKind::Scalar(Scalar::Bool)
            ) {
                return Err(Error::Invariant(
                    "implicit address to integer mutation operand",
                ));
            }
            u64::from(self.evaluate_address(id, scope)?.root != Root::Null)
        } else {
            self.evaluate_constant(id, scope, false)?.bits
        };
        Ok(Value::Integer(Constant {
            bits: kind.convert(bits),
            ty: target,
        }))
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
                    } else if let TypeKind::Pointer(element) = self.graph.types.get(target).kind {
                        Value::Pointer(Address::new(Root::Null, element))
                    } else {
                        Value::Integer(Constant {
                            bits: 0,
                            ty: self.bare_type(target)?,
                        })
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
        let old = self.read_constant_integer(place, location)?;
        let (operation, right_type) = self.graph.arithmetic_types(id)?;
        if self.is_address_expression(right) {
            let mut pointer = self.evaluate_address(right, scope)?;
            self.advance_address(&mut pointer, integral::signed(old.bits), location)?;
            return Ok((
                place,
                Value::Integer(Constant {
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
        let kind = self.integral_kind(target).ok_or(Error::Invariant(
            "compound mutation without integral target",
        ))?;
        Ok((
            place,
            Value::Integer(Constant {
                bits: kind.convert(bits),
                ty: target,
            }),
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
        } else {
            let old = self.read_constant_integer(place, location)?;
            let target = self.bare_type(place.designated)?;
            let target_kind = self
                .integral_kind(target)
                .ok_or(Error::Missing("floating increment constant evaluation"))?;
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
                Value::Integer(Constant { bits, ty: target }),
            )
        };
        self.write_constant_value(place, stepped, location)?;
        Ok((place, old))
    }
}
