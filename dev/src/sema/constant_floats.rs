// Translated from constant_expressions.cpp and constant_interpreter.cpp. See NOTICE.
//! Typed floating scalar values consume analyzed syntax and evaluation-owned places.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constants::{Constant, ConstantState},
    constexpr_calls::Returned,
    expression_facts::ValueCategory,
    graph::{EntityId, ScopeId},
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
    types::float_value::{BinaryFloat, Format, Operation},
};
use std::cmp::Ordering::{Equal, Greater, Less};

#[derive(Clone, Copy)]
pub(super) struct Floating {
    pub ty: TypeId,
    pub value: BinaryFloat,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct FloatingId(u32);
#[derive(Default)]
pub(super) struct Floats(Vec<Floating>);
impl Floats {
    fn insert(&mut self, value: Floating) -> Result<FloatingId, Error> {
        let id = FloatingId(
            u32::try_from(self.0.len()).map_err(|_| Error::Limit("floating constant limit"))?,
        );
        self.0.push(value);
        Ok(id)
    }
    pub(super) fn get(&self, id: FloatingId) -> Floating {
        self.0[id.0 as usize]
    }
}
impl Analyzer<'_> {
    pub(super) fn float_format(&self, ty: TypeId) -> Option<Format> {
        let TypeKind::Scalar(Scalar::Basic(ty)) = self.graph.types.get(ty).kind else {
            return None;
        };
        Format::of(ty)
    }
    pub(super) fn floating_expression(&self, id: ast::ExprId) -> bool {
        self.graph
            .expression_facts
            .get(id)
            .is_ok_and(|facts| self.float_format(facts.ty).is_some())
    }
    pub(super) fn floating_binary(&self, id: ast::ExprId) -> bool {
        let ExprKind::Binary {
            op,
            assignment: false,
            ..
        } = self.tree.expressions.get(id).kind
        else {
            return false;
        };
        if op.token.as_punctuator() == Some(P::Comma) {
            return false;
        }
        self.graph
            .arithmetic_types(id)
            .is_ok_and(|(ty, _)| self.float_format(ty).is_some())
            || matches!(self.tree.expressions.get(id).kind, ExprKind::Binary{left,right,..}
                if self.floating_expression(left) || self.floating_expression(right)
                    || self.graph.expression_facts.get(left).is_ok_and(|facts|facts.conversion.is_some_and(|step|self.float_format(step.result).is_some())))
    }
    pub(super) fn scalar_value(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<Value, Error> {
        self.in_constant_evaluation(|analyzer| analyzer.scalar_value_inner(id, scope))
    }
    fn scalar_value_inner(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<Value, Error> {
        if let Some(step) = self.graph.expression_facts.function_conversion(id) {
            let returned = self.constant_conversion_result(id, scope)?;
            let location = self.tree.expressions.get(id).location;
            let value = self.returned_scalar(returned, location)?;
            return self.convert_constant_scalar(value, step.after.result, location);
        }
        if self.floating_expression(id) {
            return Ok(Value::Floating(self.evaluate_floating(id, scope)?));
        }
        if self.is_address_expression(id) {
            return Ok(Value::Pointer(self.address_operand(id, scope)?));
        }
        self.evaluate_constant(id, scope, false).map(Value::Integer)
    }
    pub(super) fn returned_scalar(
        &mut self,
        returned: Returned,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        match returned {
            Returned::Value(value) => Ok(value),
            Returned::Place(place) => self.read_constant_scalar(place, location),
            Returned::Void => Err(nonconstant(location, "not an integral constant expression")),
        }
    }
    pub(super) fn read_constant_scalar(
        &mut self,
        place: Address,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        if self.float_format(place.designated).is_some() {
            self.read_constant_floating(place, location)
                .map(Value::Floating)
        } else if matches!(
            self.graph.types.get(place.designated).kind,
            TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)
        ) {
            self.read_constant_address(place, location)
                .map(Value::Pointer)
        } else {
            self.read_constant_integer(place, location)
                .map(Value::Integer)
        }
    }
    pub(super) fn convert_constant_scalar(
        &mut self,
        value: Value,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        let target = self.bare_type(target)?;
        if let Some(format) = self.float_format(target) {
            let value = match value {
                Value::Floating(value) => value.value,
                Value::Integer(value) => BinaryFloat::integer(
                    value.bits,
                    self.integral_kind(value.ty)
                        .ok_or(Error::Invariant("nonintegral scalar value"))?
                        .signed,
                ),
                Value::Pointer(_) => {
                    return Err(nonconstant(
                        location,
                        "a pointer converted to a floating type",
                    ));
                }
            };
            let value = value
                .rounded(format)
                .ok_or_else(|| nonconstant(location, "a floating conversion out of range"))?;
            return Ok(Value::Floating(Floating { ty: target, value }));
        }
        if let Some(kind) = self.integral_kind(target) {
            let bits = match value {
                Value::Integer(value) => kind.convert(value.bits),
                Value::Floating(value) => {
                    if kind.bits == 1 {
                        u64::from(value.value.truth())
                    } else {
                        kind.convert(value.value.integral(kind.bits, kind.signed).ok_or_else(
                            || {
                                nonconstant(
                                    location,
                                    "a floating value out of the integral type's range",
                                )
                            },
                        )?)
                    }
                }
                Value::Pointer(value) if kind.bits == 1 => u64::from(value.root != Root::Null),
                Value::Pointer(_) => {
                    return Err(nonconstant(
                        location,
                        "a pointer converted to an integer in a constant expression",
                    ));
                }
            };
            return Ok(Value::Integer(Constant { bits, ty: target }));
        }
        Ok(value)
    }
    pub(super) fn evaluate_floating(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Floating, Error> {
        self.in_constant_evaluation(|analyzer| analyzer.floating_value(id, scope))
    }
    fn floating_value(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<Floating, Error> {
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        self.constant_step(node.location)?;
        let facts = self.graph.expression_facts.get(id)?;
        if facts.literal_call.is_some()
            || facts.call_form == super::expression_facts::CallForm::Operator
        {
            let returned = self.constant_call_result(id, scope)?;
            let value = self.returned_scalar(returned, node.location)?;
            let Value::Floating(value) =
                self.convert_constant_scalar(value, facts.ty, node.location)?
            else {
                return Err(Error::Invariant("floating literal call result"));
            };
            return Ok(value);
        }
        if facts.category != ValueCategory::Prvalue
            && !matches!(node.kind, ExprKind::Conditional { .. })
        {
            let place = self.constant_place(id, scope)?;
            return self.read_constant_floating(place, node.location);
        }
        let value = match node.kind {
            ExprKind::Literal(literal) => {
                let PostTokenKind::Literal { ty, data, .. } = literal.kind else {
                    return Err(nonconstant(
                        node.location,
                        "not an integral constant expression",
                    ));
                };
                let format = Format::of(ty).ok_or(Error::Invariant("nonfloating literal"))?;
                let mut bytes = [0u8; 16];
                let data = data.as_slice();
                bytes[..data.len()].copy_from_slice(data);
                let value = BinaryFloat::from_bits(format, u128::from_le_bytes(bytes))
                    .ok_or(Error::Invariant("floating literal encoding"))?;
                Value::Floating(Floating {
                    ty: facts.ty,
                    value,
                })
            }
            ExprKind::Paren(inner) => self.scalar_value(inner, scope)?,
            ExprKind::Cast {
                keyword, operand, ..
            } => {
                if matches!(
                    keyword,
                    Some(Keyword::ReinterpretCast | Keyword::DynamicCast)
                ) {
                    return Err(nonconstant(
                        node.location,
                        "a reinterpret_cast or dynamic_cast in a constant expression",
                    ));
                }
                self.scalar_value(operand, scope)?
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                let arguments = if let Some(braced) = braced {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
                    else {
                        return Err(Error::Invariant("floating cast list"));
                    };
                    elements
                } else {
                    arguments
                };
                if let Some(&argument) = self.tree.expr_lists.get(arguments).first() {
                    self.scalar_value(argument, scope)?
                } else {
                    Value::Floating(Floating {
                        ty: facts.ty,
                        value: BinaryFloat::zero(),
                    })
                }
            }
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
                self.scalar_value(selected, scope)?
            }
            ExprKind::Binary {
                assignment: true,
                left,
                right,
                op,
            } => {
                let place = self.constant_assignment_place(
                    id,
                    left,
                    right,
                    op.token.as_punctuator(),
                    scope,
                )?;
                return self.read_constant_floating(place, node.location);
            }
            ExprKind::Binary {
                left, right, op, ..
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard_constant(left, scope)?;
                self.scalar_value(right, scope)?
            }
            ExprKind::Binary { .. } => self.floating_chain(id, scope)?,
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                match op.token.as_punctuator() {
                    Some(P::Increment | P::Decrement) => {
                        let (place, old) = self.constant_increment_values(
                            id,
                            operand,
                            op.token.as_punctuator() == Some(P::Decrement),
                            scope,
                        )?;
                        if matches!(node.kind, ExprKind::Postfix { .. }) {
                            old
                        } else {
                            self.read_constant_scalar(place, node.location)?
                        }
                    }
                    Some(P::Add | P::Subtract) => {
                        let value = self.scalar_value(operand, scope)?;
                        let Value::Floating(mut value) =
                            self.convert_constant_scalar(value, facts.ty, node.location)?
                        else {
                            return Err(Error::Invariant("floating unary value"));
                        };
                        if op.token.as_punctuator() == Some(P::Subtract) {
                            value.value = value.value.negate();
                        }
                        Value::Floating(value)
                    }
                    _ => {
                        return Err(nonconstant(
                            node.location,
                            "operator is not allowed in a constant expression",
                        ));
                    }
                }
            }
            ExprKind::Call { arguments, .. } if facts.functional_call => {
                if let Some(&argument) = self.tree.expr_lists.get(arguments).first() {
                    self.constant_operand_value(argument, facts.ty, scope)?
                } else {
                    Value::Floating(Floating {
                        ty: facts.ty,
                        value: BinaryFloat::zero(),
                    })
                }
            }
            ExprKind::Call { .. } => {
                let value = self.constant_call_result(id, scope)?;
                self.returned_scalar(value, node.location)?
            }
            _ => {
                return Err(nonconstant(
                    node.location,
                    "an expression that is not a constant expression",
                ));
            }
        };
        let Value::Floating(value) =
            self.convert_constant_scalar(value, facts.ty, node.location)?
        else {
            return Err(Error::Invariant(
                "floating evaluation returned nonfloating value",
            ));
        };
        Ok(value)
    }
    pub(super) fn integral_floating_operation(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        let ty = self.graph.expression_facts.get(id)?.ty;
        if skipping {
            return Ok(Constant { ty, bits: 0 });
        }
        let node = *self.tree.expressions.get(id);
        if let ExprKind::Binary {
            left,
            right,
            op,
            assignment: false,
        } = node.kind
            && matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr))
        {
            let left = self.constant_truth(left, scope, false)?;
            let truth = if op.token.as_punctuator() == Some(P::LogicalAnd) {
                left && self.constant_truth(right, scope, false)?
            } else {
                left || self.constant_truth(right, scope, false)?
            };
            return Ok(Constant {
                ty,
                bits: u64::from(truth),
            });
        }
        let Value::Integer(value) = self.floating_chain(id, scope)? else {
            return Err(nonconstant(
                node.location,
                "not an integral constant expression",
            ));
        };
        Ok(value)
    }
    pub(super) fn floating_chain(
        &mut self,
        root: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        let base = self.constant_spine.len();
        let mut next = root;
        while let ExprKind::Binary {
            left,
            op,
            assignment: false,
            ..
        } = self.tree.expressions.get(next).kind
        {
            if self.consumes_address(next)
                || matches!(
                    op.token.as_punctuator(),
                    Some(P::Comma | P::LogicalAnd | P::LogicalOr)
                )
            {
                break;
            }
            if next != root {
                self.constant_step(self.tree.expressions.get(next).location)?;
            }
            self.constant_spine.push(next);
            next = left;
        }
        let result = (|| {
            let mut value = self.scalar_value(next, scope)?;
            for index in (base..self.constant_spine.len()).rev() {
                let id = self.constant_spine[index];
                let ExprKind::Binary { right, op, .. } = self.tree.expressions.get(id).kind else {
                    return Err(Error::Invariant("floating spine node"));
                };
                let right = self.scalar_value(right, scope)?;
                value =
                    self.constant_numeric_operation(id, op.token.as_punctuator(), value, right)?;
            }
            Ok(value)
        })();
        self.constant_spine.truncate(base);
        result
    }
    pub(super) fn constant_numeric_operation(
        &mut self,
        id: ast::ExprId,
        op: Option<P>,
        left: Value,
        right: Value,
    ) -> Result<Value, Error> {
        let location = self.tree.expressions.get(id).location;
        let ty = match self.tree.expressions.get(id).kind {
            ExprKind::Binary {
                left,
                assignment: false,
                ..
            } => {
                self.graph
                    .expression_facts
                    .get(left)?
                    .conversion
                    .ok_or(Error::Invariant(
                        "numeric operation without operand conversion",
                    ))?
                    .result
            }
            _ => self.graph.arithmetic_types(id)?.0,
        };
        if let Some(format) = self.float_format(ty) {
            let Value::Floating(left) = self.convert_constant_scalar(left, ty, location)? else {
                return Err(Error::Invariant("floating left operand"));
            };
            let Value::Floating(right) = self.convert_constant_scalar(right, ty, location)? else {
                return Err(Error::Invariant("floating right operand"));
            };
            if let Some(op) = op.and_then(float_operation) {
                let value = left
                    .value
                    .operation(op, right.value, format)
                    .ok_or_else(|| {
                        nonconstant(location, "a floating operation with an undefined result")
                    })?;
                return Ok(Value::Floating(Floating { ty, value }));
            }
            let order = left.value.compare(right.value);
            let truth = match op {
                Some(P::Equal) => order == Some(Equal),
                Some(P::NotEqual) => order != Some(Equal),
                Some(P::Less) => order == Some(Less),
                Some(P::Greater) => order == Some(Greater),
                Some(P::LessEqual) => matches!(order, Some(Less | Equal)),
                Some(P::GreaterEqual) => matches!(order, Some(Greater | Equal)),
                _ => return Err(Error::Invariant("nonfloating numeric operation")),
            };
            return Ok(Value::Integer(Constant {
                ty: self.graph.types.scalar(Scalar::Bool)?,
                bits: u64::from(truth),
            }));
        }
        let (Value::Integer(left), Value::Integer(right)) = (left, right) else {
            return Err(Error::Invariant("nonintegral arithmetic operands"));
        };
        self.constant_arithmetic(
            op.ok_or(Error::Invariant("numeric operation token"))?,
            left,
            right,
            false,
            id,
        )
        .map(Value::Integer)
    }
    pub(super) fn read_constant_floating(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<Floating, Error> {
        if self.type_cv(place.designated).0 & 2 != 0 {
            return Err(nonconstant(location, "a read of a volatile glvalue"));
        }
        if place.past_end {
            return Err(nonconstant(
                location,
                "an access one past the end of an array",
            ));
        }
        if let Some(value) = self.published_scalar(place, location)? {
            return match value {
                Value::Floating(value) => Ok(value),
                _ => Err(Error::Invariant("published scalar read as floating")),
            };
        }
        let const_temporary = self.type_cv(place.designated).0 == 1;
        place.designated = self.bare_type(place.designated)?;
        let value = match place.root {
            Root::Object(_) => self.constexpr_frames.read(place, location)?,
            Root::Variable(entity) => {
                self.instantiate_variable_value(entity)?;
                let declared = self.graph.lower_entity_type(entity)?;
                if self.type_cv(declared).0 & 2 != 0 {
                    return Err(nonconstant(location, "a read of a volatile object"));
                }
                if !self.graph.entity(entity).constexpr {
                    return Err(nonconstant(
                        location,
                        "a variable is not usable in a constant expression",
                    ));
                }
                if place.offset != 0
                    || self
                        .graph
                        .entity(entity)
                        .ty
                        .map(|ty| self.bare_type(ty))
                        .transpose()?
                        != Some(place.designated)
                {
                    return Err(nonconstant(
                        location,
                        "an access outside the designated object",
                    ));
                }
                return match self.graph.entity(entity).constant {
                    ConstantState::Floating(id) => Ok(self.graph.floating_constants.get(id)),
                    ConstantState::Evaluating => Err(nonconstant(
                        location,
                        "a read of an object that is not initialized",
                    )),
                    _ => Err(nonconstant(
                        location,
                        "a variable is not usable in a constant expression",
                    )),
                };
            }
            Root::Ephemeral(id) => self
                .expression_temporary_value(id)
                .ok_or(Error::Invariant("floating temporary without storage"))?,
            Root::Temporary(entity) => {
                if !const_temporary {
                    return Err(nonconstant(
                        location,
                        "an access to a temporary that is not const, bound outside the evaluation",
                    ));
                }
                self.graph
                    .constant_addresses
                    .temporary_scalar(entity)
                    .ok_or(Error::Invariant(
                        "floating reference temporary without value",
                    ))?
            }
            Root::Null => {
                return Err(nonconstant(
                    location,
                    "an indirection through a null pointer",
                ));
            }
            Root::Function(_) => return Err(nonconstant(location, "a function is not an object")),
            Root::Literal(_) => {
                return Err(nonconstant(
                    location,
                    "an access outside the designated object",
                ));
            }
        };
        let Value::Floating(value) = value else {
            return Err(Error::Invariant("floating place holds nonfloating value"));
        };
        Ok(value)
    }
    pub(super) fn record_floating_initializer(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        self.graph.entity_mut(entity).constant = ConstantState::Evaluating;
        let result = (|| {
            let value = if let Some(expr) = self.single_initializer(initializer) {
                self.constant_operand_value(expr, ty, scope)?
            } else {
                self.zero_constant_scalar(ty)?
            };
            let Value::Floating(value) = value else {
                return Err(Error::Invariant("floating initializer type"));
            };
            self.graph.floating_constants.insert(value)
        })();
        self.graph.entity_mut(entity).constant = ConstantState::None;
        match result {
            Ok(value) => {
                self.graph.entity_mut(entity).constant = ConstantState::Floating(value);
                Ok(())
            }
            Err(Error::NonConstant { location, message }) => Err(Error::at(location, message)),
            Err(error) => Err(error),
        }
    }
}
fn float_operation(op: P) -> Option<Operation> {
    Some(match op {
        P::Add => Operation::Add,
        P::Subtract => Operation::Subtract,
        P::Multiply => Operation::Multiply,
        P::Divide => Operation::Divide,
        _ => return None,
    })
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
    fn floating_invocations_reuse_facts_and_release_local_storage() {
        let source = SourceBuffer::new(b"constexpr double f(double x){double y=x;y+=0.5;return y;}constexpr double a=f(1.5);constexpr double b=f(3.5);".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"floating-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        assert_eq!(analyzer.graph.floating_constants.0.len(), 2);
        let ast::DeclKind::Simple { declarators, .. } = analyzer
            .tree
            .declarations
            .get(analyzer.tree.decl_lists.get(declarations)[1])
            .kind
        else {
            panic!("variable declaration");
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let expr = analyzer.single_initializer(initializer).unwrap();
        let expression_count = analyzer.tree.expressions.mark();
        for _ in 0..3 {
            let value = analyzer.evaluate_floating(expr, scope).unwrap();
            assert_eq!(
                value.value.bits(Format::Double),
                Some(0x4000_0000_0000_0000)
            );
            assert!(analyzer.graph.constant_fact(expr).is_none());
            assert!(!analyzer.constexpr_frames.active());
            assert!(analyzer.constant_evaluation.is_none());
            assert_eq!(analyzer.graph.floating_constants.0.len(), 2);
            assert_eq!(analyzer.tree.expressions.mark(), expression_count);
        }
    }

    #[test]
    fn selected_scalar_conversions_own_and_release_their_evaluation() {
        let source = SourceBuffer::new(
            b"int object;struct F{constexpr operator double()const{return 3.5;}};\
              struct P{constexpr operator int*()const{return &object;}};\
              constexpr double x=F{};constexpr int*p=P{};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"scalar-conversions").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let expressions = analyzer.tree.expressions.mark();
        let floats = analyzer.graph.floating_constants.0.len();
        for index in [3, 4] {
            let ast::DeclKind::Simple { declarators, .. } = analyzer
                .tree
                .declarations
                .get(analyzer.tree.decl_lists.get(declarations)[index])
                .kind
            else {
                panic!("scalar declaration");
            };
            let initializer = analyzer.tree.init_declarators.get(declarators)[0]
                .initializer
                .unwrap();
            let expr = analyzer.single_initializer(initializer).unwrap();
            for _ in 0..32 {
                match analyzer.scalar_value(expr, scope).unwrap() {
                    Value::Floating(value) => {
                        assert_eq!(
                            value.value.bits(Format::Double),
                            Some(0x400c_0000_0000_0000)
                        );
                    }
                    Value::Pointer(value) => assert!(matches!(value.root, Root::Variable(_))),
                    Value::Integer(_) => panic!("conversion result"),
                }
                assert!(!analyzer.constexpr_frames.active());
                assert_eq!(analyzer.constexpr_frames.depth(), 0);
                assert!(analyzer.constant_evaluation.is_none());
                assert_eq!(analyzer.tree.expressions.mark(), expressions);
                assert_eq!(analyzer.graph.floating_constants.0.len(), floats);
            }
        }
    }
}
