// Translated from sema/constant_complex.cpp. See NOTICE.
//! Two component values use the interpreter's existing flat object storage and evaluation lifetime.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, TemporaryValue as Value, nonconstant},
    constant_floats::Floating,
    constants::Constant,
    graph::ScopeId,
    types::{Scalar, TypeId},
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
    types::float_value::Operation,
};
impl Analyzer<'_> {
    pub(super) fn constant_component_place(
        &mut self,
        operand: ast::ExprId,
        imaginary: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let ty = self.graph.expression_facts.get(operand)?.ty;
        let Some(part) = self.complex_part_type(ty) else {
            return Err(nonconstant(
                location,
                "a part of a real glvalue in a constant expression",
            ));
        };
        let mut place = self.constant_place(operand, scope)?;
        if imaginary {
            place.offset = place
                .offset
                .wrapping_add(super::integral::signed(self.type_size(part, location)?.0));
        }
        place.designated = self
            .graph
            .types
            .qualify(part, self.graph.types.get(ty).cv)?;
        Ok(place)
    }
    pub(super) fn constant_component(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        imaginary: bool,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        let ty = self.graph.expression_facts.get(operand)?.ty;
        if self.complex_part_type(ty).is_some() {
            return Ok(self.constant_complex_parts(operand, ty, scope)?[usize::from(imaginary)]);
        }
        if !imaginary
            && self.graph.expression_facts.get(id)?.category
                != super::expression_facts::ValueCategory::Prvalue
        {
            return Err(nonconstant(
                self.tree.expressions.get(id).location,
                "a part of a real glvalue in a constant expression",
            ));
        }
        let value = self.scalar_value(operand, scope)?;
        if imaginary {
            self.zero_constant_scalar(ty)
        } else {
            Ok(value)
        }
    }
    fn stored_complex_parts(
        &mut self,
        place: Address,
        part: TypeId,
        location: SourceOffset,
    ) -> Result<[Value; 2], Error> {
        let first = Address {
            designated: part,
            ..place
        };
        let second = Address {
            offset: place
                .offset
                .wrapping_add(super::integral::signed(self.type_size(part, location)?.0)),
            ..first
        };
        Ok([
            self.read_constant_scalar(first, location)?,
            self.read_constant_scalar(second, location)?,
        ])
    }
    pub(super) fn store_complex_parts(
        &mut self,
        target: Address,
        parts: [Value; 2],
        location: SourceOffset,
    ) -> Result<(), Error> {
        let part = self
            .complex_part_type(target.designated)
            .ok_or(Error::Invariant("constant complex target"))?;
        for (index, value) in parts.into_iter().enumerate() {
            let offset = if index == 0 {
                0
            } else {
                self.type_size(part, location)?.0
            };
            let place = Address {
                designated: part,
                offset: target.offset.wrapping_add(super::integral::signed(offset)),
                ..target
            };
            let value = self.convert_constant_scalar(value, part, location)?;
            self.store_constant_value(place, value, location)?;
        }
        Ok(())
    }
    pub(super) fn constant_complex_parts(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<[Value; 2], Error> {
        self.check_stack()?;
        self.check_deferred_constant(id)?;
        let id = self.graph.expression_facts.root(id);
        let location = self.tree.expressions.get(id).location;
        let part = self
            .complex_part_type(target)
            .ok_or(Error::Invariant("constant complex conversion type"))?;
        let facts = self.graph.expression_facts.get(id)?;
        let source = facts.ty;
        let parts = if self.complex_part_type(source).is_none() {
            [
                self.scalar_value(id, scope)?,
                self.zero_constant_scalar(part)?,
            ]
        } else if facts.category != super::expression_facts::ValueCategory::Prvalue
            && !matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. })
        {
            let place = self.constant_place(id, scope)?;
            self.stored_complex_parts(
                place,
                self.complex_part_type(source)
                    .ok_or(Error::Invariant("constant complex source part"))?,
                location,
            )?
        } else {
            self.evaluate_complex_parts(id, scope)?
        };
        Ok([
            self.convert_constant_scalar(parts[0], part, location)?,
            self.convert_constant_scalar(parts[1], part, location)?,
        ])
    }
    pub(super) fn initialize_constant_complex(
        &mut self,
        id: ast::ExprId,
        target: Address,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let parts = self.constant_complex_parts(id, target.designated, scope)?;
        self.store_complex_parts(target, parts, self.tree.expressions.get(id).location)
    }
    fn evaluate_complex_parts(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<[Value; 2], Error> {
        let node = *self.tree.expressions.get(id);
        let ty = self.graph.expression_facts.get(id)?.ty;
        let part = self
            .complex_part_type(ty)
            .ok_or(Error::Invariant("complex expression part"))?;
        match node.kind {
            ExprKind::Paren(inner) => self.constant_complex_parts(inner, ty, scope),
            ExprKind::Braced { elements, .. } => {
                self.constant_complex_list(elements, ty, scope, node.location)
            }
            ExprKind::Call { arguments, .. }
                if self.graph.expression_facts.complex_construction(id) =>
            {
                self.constant_complex_list(arguments, ty, scope, node.location)
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if let Some(braced) = braced {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
                    else {
                        return Err(Error::Invariant("complex cast list"));
                    };
                    return self.constant_complex_list(elements, ty, scope, node.location);
                }
                let first = self.tree.expr_lists.get(arguments).first().copied();
                if let Some(first) = first {
                    self.constant_complex_parts(first, ty, scope)
                } else {
                    Ok([
                        self.zero_constant_scalar(part)?,
                        self.zero_constant_scalar(part)?,
                    ])
                }
            }
            ExprKind::Cast { operand, .. } => self.constant_complex_parts(operand, ty, scope),
            ExprKind::Call { arguments, .. }
                if self.graph.expression_facts.get(id)?.functional_call =>
            {
                if let Some(&first) = self.tree.expr_lists.get(arguments).first() {
                    self.constant_complex_parts(first, ty, scope)
                } else {
                    Ok([
                        self.zero_constant_scalar(part)?,
                        self.zero_constant_scalar(part)?,
                    ])
                }
            }
            ExprKind::Call { .. } => {
                let super::constexpr_calls::Returned::Place(place) =
                    self.constant_call_result(id, scope)?
                else {
                    return Err(nonconstant(
                        node.location,
                        "a complex operand outside constant evaluation",
                    ));
                };
                self.stored_complex_parts(place, part, node.location)
            }
            ExprKind::Unary { op, operand } => {
                let mut parts = self.constant_complex_parts(operand, ty, scope)?;
                if op.token.as_punctuator() == Some(P::Add) {
                    return Ok(parts);
                }
                if op.token.as_punctuator() == Some(P::Subtract) {
                    parts[0] = self.constant_part_negate(part, parts[0], node.location)?;
                }
                parts[1] = self.constant_part_negate(part, parts[1], node.location)?;
                Ok(parts)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard_constant(left, scope)?;
                self.constant_complex_parts(right, ty, scope)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } => self.constant_complex_binary(
                id,
                op.token
                    .as_punctuator()
                    .ok_or(Error::Invariant("complex binary token"))?,
                [left, right],
                ty,
                scope,
            ),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let selected = if self.constant_truth(condition, scope, false)? {
                    then_expr
                } else {
                    else_expr
                };
                self.constant_complex_parts(selected, ty, scope)
            }
            ExprKind::Postfix { op, operand } => {
                let old = self.constant_complex_parts(operand, ty, scope)?;
                self.constant_complex_step(
                    id,
                    operand,
                    op.token.as_punctuator() == Some(P::Decrement),
                    scope,
                )?;
                Ok(old)
            }
            _ => Err(nonconstant(
                node.location,
                "a complex operand outside constant evaluation",
            )),
        }
    }
    pub(super) fn constant_complex_list(
        &mut self,
        elements: crate::syntax::storage::List<ast::ExprId>,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<[Value; 2], Error> {
        let args = super::call_arguments::Arguments::new(self.tree.expr_lists.get(elements));
        let part = self
            .complex_part_type(ty)
            .ok_or(Error::Invariant("complex list part"))?;
        match args.as_slice() {
            [] => Ok([
                self.zero_constant_scalar(part)?,
                self.zero_constant_scalar(part)?,
            ]),
            [single] => self.constant_complex_parts(*single, ty, scope),
            [real, imaginary] => Ok([
                self.constant_operand_value(*real, part, scope)?,
                self.constant_operand_value(*imaginary, part, scope)?,
            ]),
            _ => Err(nonconstant(location, "a complex value of no two parts")),
        }
    }
    fn constant_part_operation(
        &mut self,
        op: P,
        ty: TypeId,
        a: Value,
        b: Value,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        let a = self.convert_constant_scalar(a, ty, location)?;
        let b = self.convert_constant_scalar(b, ty, location)?;
        if let (Value::Floating(a), Value::Floating(b)) = (a, b) {
            let operation = match op {
                P::Add => Operation::Add,
                P::Subtract => Operation::Subtract,
                P::Multiply => Operation::Multiply,
                P::Divide => Operation::Divide,
                _ => return Err(Error::Invariant("constant complex arithmetic token")),
            };
            let format = self
                .float_format(ty)
                .ok_or(Error::Invariant("complex floating part format"))?;
            let value = a
                .value
                .operation(operation, b.value, format)
                .ok_or_else(|| {
                    nonconstant(location, "a floating operation with an undefined result")
                })?;
            return Ok(Value::Floating(Floating { ty, value }));
        }
        let (Value::Integer(a), Value::Integer(b)) = (a, b) else {
            return Err(nonconstant(
                location,
                "a complex part outside constant evaluation",
            ));
        };
        let kind = self
            .integral_kind(ty)
            .filter(|kind| kind.bits <= 64)
            .ok_or_else(|| nonconstant(location, "a complex part outside constant evaluation"))?;
        let (bits, valid) = kind.operation(op, a.bits, b.bits).ok_or_else(|| {
            nonconstant(location, "an arithmetic operation with an undefined result")
        })?;
        if !valid {
            return Err(nonconstant(
                location,
                "an arithmetic operation with an undefined result",
            ));
        }
        Ok(Value::Integer(Constant { ty, bits, high: 0 }))
    }
    fn constant_part_negate(
        &mut self,
        ty: TypeId,
        value: Value,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        if let Value::Floating(mut value) = self.convert_constant_scalar(value, ty, location)? {
            value.value = value.value.negate();
            return Ok(Value::Floating(value));
        }
        let zero = self.zero_constant_scalar(ty)?;
        self.constant_part_operation(P::Subtract, ty, zero, value, location)
    }
    pub(super) fn constant_complex_binary(
        &mut self,
        id: ast::ExprId,
        op: P,
        operands: [ast::ExprId; 2],
        ty: TypeId,
        scope: ScopeId,
    ) -> Result<[Value; 2], Error> {
        let [left, right] = operands;
        let left_complex = self
            .complex_part_type(self.graph.expression_facts.get(left)?.ty)
            .is_some();
        let right_complex = self
            .complex_part_type(self.graph.expression_facts.get(right)?.ty)
            .is_some();
        let [a, b] = self.constant_complex_parts(left, ty, scope)?;
        let [c, d] = self.constant_complex_parts(right, ty, scope)?;
        self.constant_complex_operation(
            op,
            ty,
            [a, b, c, d],
            [left_complex, right_complex],
            self.tree.expressions.get(id).location,
        )
    }
    fn constant_complex_operation(
        &mut self,
        op: P,
        ty: TypeId,
        values: [Value; 4],
        complex: [bool; 2],
        location: SourceOffset,
    ) -> Result<[Value; 2], Error> {
        let [a, b, c, d] = values;
        let [lc, rc] = complex;
        let part = self
            .complex_part_type(ty)
            .ok_or(Error::Invariant("complex constant operation part"))?;
        match op {
            P::Add | P::Subtract => {
                let real = self.constant_part_operation(op, part, a, c, location)?;
                let imaginary = if rc && lc {
                    self.constant_part_operation(op, part, b, d, location)?
                } else if rc {
                    if op == P::Subtract {
                        self.constant_part_negate(part, d, location)?
                    } else {
                        d
                    }
                } else {
                    b
                };
                Ok([real, imaginary])
            }
            P::Multiply if lc && rc => {
                let ac = self.constant_part_operation(op, part, a, c, location)?;
                let bd = self.constant_part_operation(op, part, b, d, location)?;
                let ad = self.constant_part_operation(op, part, a, d, location)?;
                let bc = self.constant_part_operation(op, part, b, c, location)?;
                Ok([
                    self.constant_part_operation(P::Subtract, part, ac, bd, location)?,
                    self.constant_part_operation(P::Add, part, ad, bc, location)?,
                ])
            }
            P::Multiply => Ok([
                self.constant_part_operation(op, part, a, c, location)?,
                self.constant_part_operation(
                    op,
                    part,
                    if lc { b } else { a },
                    if lc { c } else { d },
                    location,
                )?,
            ]),
            _ if !rc => Ok([
                self.constant_part_operation(op, part, a, c, location)?,
                self.constant_part_operation(op, part, b, c, location)?,
            ]),
            _ if self.float_format(part).is_some() => Err(nonconstant(
                location,
                "a quotient of complex values outside constant evaluation",
            )),
            _ => {
                let (Value::Integer(cc), Value::Integer(dd)) = (c, d) else {
                    return Err(nonconstant(
                        location,
                        "a complex part outside constant evaluation",
                    ));
                };
                let signed = self.integral_kind(part).is_some_and(|kind| kind.signed);
                let magnitude = |bits: u64| {
                    if signed && bits.cast_signed() < 0 {
                        bits.wrapping_neg()
                    } else {
                        bits
                    }
                };
                let by_d = magnitude(cc.bits) < magnitude(dd.bits);
                let (small, large) = if by_d { (c, d) } else { (d, c) };
                let ratio =
                    self.constant_part_operation(P::Divide, part, small, large, location)?;
                let scaled =
                    self.constant_part_operation(P::Multiply, part, small, ratio, location)?;
                let divisor =
                    self.constant_part_operation(P::Add, part, scaled, large, location)?;
                let scaled_real = self.constant_part_operation(
                    P::Multiply,
                    part,
                    if by_d { a } else { b },
                    ratio,
                    location,
                )?;
                let real = self.constant_part_operation(
                    P::Add,
                    part,
                    scaled_real,
                    if by_d { b } else { a },
                    location,
                )?;
                let scaled_imaginary = self.constant_part_operation(
                    P::Multiply,
                    part,
                    if by_d { b } else { a },
                    ratio,
                    location,
                )?;
                let imaginary = if by_d {
                    self.constant_part_operation(P::Subtract, part, scaled_imaginary, a, location)?
                } else {
                    self.constant_part_operation(P::Subtract, part, b, scaled_imaginary, location)?
                };
                Ok([
                    self.constant_part_operation(P::Divide, part, real, divisor, location)?,
                    self.constant_part_operation(P::Divide, part, imaginary, divisor, location)?,
                ])
            }
        }
    }
    pub(super) fn constant_complex_truth(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let ty = self.graph.expression_facts.get(id)?.ty;
        let parts = self.constant_complex_parts(id, ty, scope)?;
        Ok(parts.into_iter().any(|value| match value {
            Value::Integer(v) => v.wide() != 0,
            Value::Floating(v) => v.value.truth(),
            _ => false,
        }))
    }
    pub(super) fn constant_complex_equal(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        equal: bool,
        scope: ScopeId,
    ) -> Result<Constant, Error> {
        let ty = self
            .graph
            .expression_facts
            .get(left)?
            .conversion
            .ok_or(Error::Invariant("complex comparison type"))?
            .result;
        let a = self.constant_complex_parts(left, ty, scope)?;
        let b = self.constant_complex_parts(right, ty, scope)?;
        let same = a.into_iter().zip(b).all(|(a, b)| match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => a.wide() == b.wide(),
            (Value::Floating(a), Value::Floating(b)) => {
                a.value.compare(b.value) == Some(std::cmp::Ordering::Equal)
            }
            _ => false,
        });
        Ok(Constant {
            ty: self.graph.expression_facts.get(id)?.ty,
            bits: u64::from(equal == same),
            high: 0,
        })
    }
    pub(super) fn constant_complex_assignment(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        op: P,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let ty = self.graph.expression_facts.get(left)?.ty;
        let location = self.tree.expressions.get(id).location;
        if op == P::Assign {
            let parts = self.constant_complex_parts(right, ty, scope)?;
            let place = self.constant_place(left, scope)?;
            self.store_complex_parts(place, parts, location)?;
            return Ok(place);
        }
        let place = self.constant_place(left, scope)?;
        let common = self.graph.arithmetic_types(id)?.0;
        let [a, b] = self.constant_complex_parts(left, common, scope)?;
        let [c, d] = self.constant_complex_parts(right, common, scope)?;
        let token = match op {
            P::AddAssign => P::Add,
            P::SubtractAssign => P::Subtract,
            P::MultiplyAssign => P::Multiply,
            P::DivideAssign => P::Divide,
            _ => return Err(Error::Invariant("complex compound token")),
        };
        let rc = self
            .complex_part_type(self.graph.expression_facts.get(right)?.ty)
            .is_some();
        let parts =
            self.constant_complex_operation(token, common, [a, b, c, d], [true, rc], location)?;
        self.store_complex_parts(place, parts, location)?;
        Ok(place)
    }
    pub(super) fn constant_complex_step(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        decrement: bool,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let location = self.tree.expressions.get(id).location;
        let ty = self.graph.expression_facts.get(operand)?.ty;
        let place = self.constant_place(operand, scope)?;
        let part = self
            .complex_part_type(ty)
            .ok_or(Error::Invariant("complex increment type"))?;
        let mut parts = self.stored_complex_parts(place, part, location)?;
        let int = self
            .graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::Int))?;
        parts[0] = self.constant_part_operation(
            if decrement { P::Subtract } else { P::Add },
            part,
            parts[0],
            Value::Integer(Constant {
                ty: int,
                bits: 1,
                high: 0,
            }),
            location,
        )?;
        self.store_complex_parts(place, parts, location)?;
        Ok(place)
    }
}
