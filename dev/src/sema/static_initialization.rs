// Translated from lower/global_lowering.cpp's static_value/static_converted queries. See NOTICE.
//! Demand static scalar images while analysis owns constant evaluation. Lowering reads the image.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::Root,
    graph::{EntityId, ScopeId},
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    syntax::ast::{self, ExprKind},
    types::{FundamentalType as F, floating::FloatingScanner},
};
#[derive(Clone, Copy)]
pub(crate) enum Value {
    Integer(u64),
    Floating(F, u128),
    Address(EntityId, i64),
}
impl Analyzer<'_> {
    pub(super) fn record_static_initializer(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let target = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("static object without type"))?;
        if !matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(_) | TypeKind::Enum(_) | TypeKind::Pointer(_)
        ) {
            return Ok(());
        }
        let value = if let Some(expr) = self.single_initializer(initializer) {
            self.static_scalar(expr, target, scope)?
        } else {
            Some(Value::Integer(0))
        };
        self.graph.entity_mut(entity).static_initializer = value;
        Ok(())
    }
    fn static_scalar(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<Option<Value>, Error> {
        self.check_stack()?;
        let source = self.graph.expression_facts.get(id)?.ty;
        let value = self.static_scalar_value(id, scope)?;
        value.map_or(Ok(None), |value| {
            self.static_conversion(value, source, target)
        })
    }
    fn static_scalar_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<Value>, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let source = self.graph.expression_facts.get(id)?.ty;
        if self.integral_kind(source).is_some() {
            let value = if let Some(value) = self.graph.constant_fact(id) {
                Some(value)
            } else {
                match self.constant_value(id, scope, false) {
                    Ok(value) => value,
                    Err(Error::Missing(_)) => None,
                    Err(error) => return Err(error),
                }
            };
            if let Some(value) = value {
                return Ok(Some(Value::Integer(value.bits)));
            }
        }
        if matches!(
            self.graph.types.get(source).kind,
            TypeKind::Pointer(_) | TypeKind::Array { .. } | TypeKind::Function { .. }
        ) {
            let value = match self.address_operand(id, scope) {
                Ok(address) => match address.root {
                    Root::Null => Some(Value::Integer(0)),
                    Root::Variable(entity) | Root::Function(entity) => {
                        Some(Value::Address(entity, address.offset))
                    }
                    _ => None,
                },
                Err(Error::NonConstant { .. } | Error::Missing(_)) => None,
                Err(error) => return Err(error),
            };
            if value.is_some() {
                return Ok(value);
            }
        }
        let node = *self.tree.expressions.get(id);
        match node.kind {
            ExprKind::Paren(inner) => self.static_scalar_value(inner, scope),
            ExprKind::Keyword(Keyword::Nullptr) => Ok(Some(Value::Integer(0))),
            ExprKind::Literal(literal) => {
                let PostTokenKind::Literal { ty, data, .. } = literal.kind else {
                    return Ok(None);
                };
                if !matches!(ty, F::Float | F::Double | F::LongDouble) {
                    return Ok(None);
                }
                let bits = data
                    .as_slice()
                    .iter()
                    .enumerate()
                    .fold(0_u128, |bits, (index, &byte)| {
                        bits | u128::from(byte) << (index * 8)
                    });
                Ok(Some(Value::Floating(ty, bits)))
            }
            ExprKind::Unary { op, operand }
                if matches!(op.token.as_punctuator(), Some(P::Add | P::Subtract)) =>
            {
                let value = self.static_scalar(operand, source, scope)?;
                Ok(value.map(|value| match value {
                    Value::Floating(ty, bits) if op.token.as_punctuator() == Some(P::Subtract) => {
                        Value::Floating(ty, bits ^ float_sign(ty))
                    }
                    _ => value,
                }))
            }
            ExprKind::Cast { operand, .. } => self.static_scalar(operand, source, scope),
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if let Some(id) = braced {
                    return self.static_scalar(id, source, scope);
                }
                match self.tree.expr_lists.get(arguments) {
                    [] => self.static_conversion(Value::Integer(0), source, source),
                    [operand] => self.static_scalar(*operand, source, scope),
                    _ => Ok(None),
                }
            }
            ExprKind::Call { arguments, .. }
                if self.graph.expression_facts.get(id)?.functional_call =>
            {
                match self.tree.expr_lists.get(arguments) {
                    [] => self.static_conversion(Value::Integer(0), source, source),
                    [operand] => self.static_scalar(*operand, source, scope),
                    _ => Ok(None),
                }
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => match self.constant_value(condition, scope, false) {
                Ok(Some(value)) => self.static_scalar(
                    if value.bits == 0 {
                        else_expr
                    } else {
                        then_expr
                    },
                    source,
                    scope,
                ),
                Ok(None) | Err(Error::Missing(_)) => Ok(None),
                Err(error) => Err(error),
            },
            ExprKind::Braced { elements, .. } => match self.tree.expr_lists.get(elements) {
                [] => Ok(Some(Value::Integer(0))),
                [operand] => self.static_scalar(*operand, source, scope),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }
    fn static_conversion(
        &self,
        value: Value,
        source: TypeId,
        target: TypeId,
    ) -> Result<Option<Value>, Error> {
        if let Some(kind) = self.integral_kind(target) {
            let bits = match value {
                Value::Integer(bits) => bits,
                Value::Address(_, _) if kind.bits == 1 => 1,
                Value::Address(_, _) if kind.bits >= 64 => return Ok(Some(value)),
                Value::Address(_, _) => return Ok(None),
                Value::Floating(ty, bits) => {
                    let Some((significand, exponent, negative)) = float_parts(ty, bits) else {
                        return Ok(None);
                    };
                    if kind.bits == 1 {
                        return Ok(Some(Value::Integer(u64::from(significand != 0))));
                    }
                    let magnitude = if exponent < 0 {
                        significand
                            .checked_shr(
                                u32::try_from(-exponent)
                                    .map_err(|_| Error::Limit("floating exponent"))?,
                            )
                            .unwrap_or(0)
                    } else {
                        let shift = u32::try_from(exponent)
                            .map_err(|_| Error::Limit("floating exponent"))?;
                        if shift >= 128 || significand > u128::from(u64::MAX) >> shift {
                            return Ok(None);
                        }
                        significand << shift
                    };
                    let magnitude = u64::try_from(magnitude)
                        .map_err(|_| Error::Limit("floating integer range"))?;
                    if kind.signed
                        && ((!negative && magnitude >= 1 << 63)
                            || (negative && magnitude > 1 << 63))
                        || !kind.signed && negative && magnitude != 0
                    {
                        return Ok(None);
                    }
                    if negative {
                        magnitude.wrapping_neg()
                    } else {
                        magnitude
                    }
                }
            };
            return Ok(Some(Value::Integer(kind.convert(bits))));
        }
        if let TypeKind::Scalar(Scalar::Basic(target)) = self.graph.types.get(target).kind
            && matches!(target, F::Float | F::Double | F::LongDouble)
        {
            let (significand, exponent, negative) = match value {
                Value::Floating(from, _) if from == target => return Ok(Some(value)),
                Value::Floating(from, bits) => {
                    let Some(parts) = float_parts(from, bits) else {
                        return Ok(None);
                    };
                    parts
                }
                Value::Integer(bits) => {
                    let negative = self.integral_kind(source).is_some_and(|kind| kind.signed)
                        && bits >> 63 != 0;
                    (
                        u128::from(if negative { bits.wrapping_neg() } else { bits }),
                        0,
                        negative,
                    )
                }
                Value::Address(_, _) => return Ok(None),
            };
            let bits = FloatingScanner::default()
                .binary(significand, exponent, target)
                .map_err(|_| Error::Limit("static floating conversion"))?;
            let bits = bits.unwrap_or_else(|| float_infinity(target));
            return Ok(Some(Value::Floating(
                target,
                bits | if negative { float_sign(target) } else { 0 },
            )));
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)
        ) && !matches!(value, Value::Floating(_, _))
        {
            return Ok(Some(value));
        }
        Ok(None)
    }
}
fn float_layout(ty: F) -> (u32, u32, i64) {
    match ty {
        F::Float => (23, 8, 127),
        F::Double => (52, 11, 1023),
        _ => (63, 15, 16383),
    }
}
fn float_sign(ty: F) -> u128 {
    1 << match ty {
        F::Float => 31,
        F::Double => 63,
        _ => 79,
    }
}
fn float_infinity(ty: F) -> u128 {
    let (fraction, width, _) = float_layout(ty);
    if ty == F::LongDouble {
        ((1 << width) - 1) << 64 | 1 << 63
    } else {
        ((1 << width) - 1) << fraction
    }
}
fn float_parts(ty: F, bits: u128) -> Option<(u128, i64, bool)> {
    let (fraction, width, exponent_bias) = float_layout(ty);
    let shift = if ty == F::LongDouble { 64 } else { fraction };
    let exponent = i64::try_from(bits >> shift & ((1 << width) - 1)).ok()?;
    if exponent == (1 << width) - 1 {
        return None;
    }
    let significand = if ty == F::LongDouble {
        bits & u128::from(u64::MAX)
    } else {
        (bits & ((1 << fraction) - 1)) | if exponent == 0 { 0 } else { 1 << fraction }
    };
    Some((
        significand,
        exponent.max(1) - exponent_bias - i64::from(fraction),
        bits & float_sign(ty) != 0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };
    #[test]
    fn images_are_demanded_after_unit_analysis_without_changing_syntax() {
        let source = SourceBuffer::new(b"extern const int y; int x=y; const int y=7; double d=-0.0; int target; int *p=&target; int callee(int); int dynamic=callee(3);".to_vec()).unwrap();
        let input = Preprocessor::with_name(&source, b"static-images").unwrap();
        let mut unit = SyntaxUnit::new(input);
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        for lowering in [false, true] {
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_semantics(true);
            if lowering {
                analyzer.set_lowering();
            }
            let graph = analyzer.analyze(declarations).unwrap();
            let mut entity = |name: &[u8]| {
                let name = cursor.intern_name(name).unwrap();
                graph
                    .binding(
                        graph
                            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                            .unwrap(),
                    )
                    .entity
            };
            let x = graph.entity(entity(b"x"));
            assert!(x.variable_definition.is_some());
            if lowering {
                assert!(matches!(x.static_initializer, Some(Value::Integer(7))));
                assert!(
                    matches!(graph.entity(entity(b"d")).static_initializer, Some(Value::Floating(F::Double, bits)) if bits == 1 << 63)
                );
                assert!(
                    matches!(graph.entity(entity(b"p")).static_initializer, Some(Value::Address(root, 0)) if root == entity(b"target"))
                );
            } else {
                assert!(x.static_initializer.is_none());
                assert!(graph.entity(entity(b"p")).static_initializer.is_none());
            }
            assert!(
                graph
                    .entity(entity(b"dynamic"))
                    .static_initializer
                    .is_none()
            );
            assert_eq!(tree.expressions.mark(), count);
        }
    }
}
