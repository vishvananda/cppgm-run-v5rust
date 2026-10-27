// Translated from sema/constant_complex.cpp and constant_expressions.cpp. See NOTICE.
//! Constant vectors project lanes in the interpreter's flat, evaluation-owned storage.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constants::Constant,
    constexpr_calls::Returned,
    expression_facts::ValueCategory,
    graph::ScopeId,
    types::{Scalar, TypeId},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::{
        ast::{self, ExprKind, InitializerKind},
        storage::List,
    },
};

impl Analyzer<'_> {
    pub(super) fn constant_lane_vector(&self, ty: TypeId) -> Option<(TypeId, u64)> {
        self.vector_shape(ty).filter(|&(lane, bound)| {
            bound != 0
                && self.graph.types.get(lane).kind != super::types::TypeKind::Scalar(Scalar::Bool)
        })
    }
    pub(super) fn constant_vector_lane_place(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        index: ast::ExprId,
        scope: ScopeId,
        value: bool,
    ) -> Result<Address, Error> {
        let location = self.tree.expressions.get(id).location;
        let ty = self.graph.expression_facts.get(base)?.ty;
        let Some((lane, bound)) = self.constant_lane_vector(ty) else {
            return Err(nonconstant(
                location,
                "a lane of a vector of packed lanes in a constant expression",
            ));
        };
        // Reading a prvalue retains its storage alias across index evaluation.
        // Designating an lvalue does not perform an lvalue-to-rvalue conversion.
        let place = if value {
            self.constant_vector_value(base, scope)?
        } else {
            self.constant_place(base, scope)?
        };
        let facts = self.graph.expression_facts.get(index)?;
        let target = facts
            .conversion
            .map_or(facts.ty, |conversion| conversion.result);
        let Value::Integer(position) = self.constant_operand_value(index, target, scope)? else {
            return Err(nonconstant(location, "a vector lane outside the vector"));
        };
        if position.indeterminate()
            || self
                .integral_kind(position.ty)
                .is_some_and(|kind| kind.signed)
                && super::integral::signed(position.bits) < 0
            || position.bits >= bound
        {
            return Err(nonconstant(location, "a vector lane outside the vector"));
        }
        Ok(Address {
            offset: place.offset.wrapping_add(super::integral::signed(
                position
                    .bits
                    .wrapping_mul(self.type_size(lane, location)?.0),
            )),
            designated: self.graph.expression_facts.get(id)?.ty,
            ..place
        })
    }
    pub(super) fn constant_vector_lane_value(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        index: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        let prvalue = self.graph.expression_facts.get(id)?.category == ValueCategory::Prvalue;
        let place = self.constant_vector_lane_place(id, base, index, scope, prvalue)?;
        if !prvalue {
            return self.read_constant_scalar(place, self.tree.expressions.get(id).location);
        }
        self.constant_arithmetic_value(place, self.tree.expressions.get(id).location)
    }
    pub(super) fn initialize_constant_vector(
        &mut self,
        id: ast::ExprId,
        target: Address,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let source = self.constant_vector_value(id, scope)?;
        self.copy_constant_object(target, source, self.tree.expressions.get(id).location)
    }
    pub(super) fn construct_constant_vector(
        &mut self,
        target: Address,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let location = initializer.location;
        let single = match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id) => {
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                    return self.initialize_vector_list(target, elements, scope, location);
                }
                Some(id)
            }
            InitializerKind::Parenthesized(elements) => {
                let elements = self.tree.expr_lists.get(elements);
                if elements.len() > 1 {
                    return Err(nonconstant(
                        location,
                        "a scalar initialized from several values",
                    ));
                }
                elements.first().copied()
            }
            _ => {
                return Err(nonconstant(
                    location,
                    "an initializer outside constant evaluation",
                ));
            }
        };
        if let Some(single) = single {
            self.initialize_constant_vector(single, target, scope)
        } else {
            self.constant_zero_object(target, location)
        }
    }
    fn initialize_vector_list(
        &mut self,
        target: Address,
        elements: List<ast::ExprId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.fill_vector_list(target, elements, scope, location)
    }
    fn constant_vector_list(
        &mut self,
        elements: List<ast::ExprId>,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let place = self.allocate_constant_value_storage(ty, location)?;
        self.fill_vector_list(place, elements, scope, location)?;
        Ok(place)
    }
    fn fill_vector_list(
        &mut self,
        place: Address,
        elements: List<ast::ExprId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let ty = place.designated;
        let (lane, bound) = self
            .vector_shape(ty)
            .ok_or(Error::Invariant("constant vector list type"))?;
        let count = u64::try_from(self.tree.expr_lists.get(elements).len())
            .map_err(|_| Error::Limit("vector lane count"))?;
        if count > bound {
            return Err(nonconstant(location, "more lanes than the vector has"));
        }
        let opaque = self.constant_lane_vector(ty).is_none();
        let stride = self.type_size(lane, location)?.0;
        for index in 0..count {
            let id = self.tree.expr_lists.get(elements)
                [usize::try_from(index).map_err(|_| Error::Limit("vector lane index"))?];
            if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. }) {
                return Err(nonconstant(location, "a braced vector lane"));
            }
            let value = self.constant_operand_value(id, lane, scope)?;
            let value = self.convert_constant_scalar(value, lane, location)?;
            let value = value.clear_zeroed();
            if !opaque {
                let target = Address {
                    offset: place
                        .offset
                        .wrapping_add(super::integral::signed(index.wrapping_mul(stride))),
                    designated: lane,
                    ..place
                };
                self.store_constant_value(target, value, location)?;
            }
        }
        if opaque {
            self.store_constant_value(
                place,
                Value::Integer(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    ty,
                    bits: 0,
                    high: 0,
                }),
                location,
            )?;
        } else {
            for index in count..bound {
                let target = Address {
                    offset: place
                        .offset
                        .wrapping_add(super::integral::signed(index.wrapping_mul(stride))),
                    designated: lane,
                    ..place
                };
                let zero = self.zero_constant_scalar(lane)?;
                self.store_constant_value(target, zero, location)?;
            }
        }
        Ok(())
    }
    pub(super) fn constant_vector_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.check_stack()?;
        self.check_deferred_constant(id)?;
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        self.constant_step(node.location)?;
        let facts = self.graph.expression_facts.get(id)?;
        if facts.literal_call.is_some()
            || facts.call_form == super::expression_facts::CallForm::Operator
        {
            let Returned::Place(place) = self.constant_call_result(id, scope)? else {
                return Err(Error::Invariant("constant vector operator result"));
            };
            return self.read_vector_storage(place, node.location);
        }
        if self
            .graph
            .expression_facts
            .function_conversion(id)
            .is_some()
        {
            let Returned::Place(place) = self.constant_conversion_result(id, scope)? else {
                return Err(Error::Invariant("constant vector conversion result"));
            };
            return self.read_vector_storage(place, node.location);
        }
        if facts.category != ValueCategory::Prvalue && !matches!(node.kind, ExprKind::Braced { .. })
        {
            let place = self.constant_place(id, scope)?;
            return self.read_vector_storage(place, node.location);
        }
        match node.kind {
            ExprKind::Braced { elements, .. } => {
                self.constant_vector_list(elements, facts.ty, scope, node.location)
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if let Some(braced) = braced {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
                    else {
                        return Err(Error::Invariant("constant vector cast list"));
                    };
                    return self.constant_vector_list(elements, facts.ty, scope, node.location);
                }
                self.constant_vector_cast_arguments(arguments, facts.ty, scope, node.location)
            }
            ExprKind::Call { arguments, .. } if facts.functional_call => {
                self.constant_vector_cast_arguments(arguments, facts.ty, scope, node.location)
            }
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
                self.constant_vector_cast(operand, facts.ty, scope, node.location)
            }
            ExprKind::Paren(inner) => self.constant_vector_value(inner, scope),
            ExprKind::BuiltinValue { .. } => Err(nonconstant(
                node.location,
                "a builtin's value outside constant evaluation",
            )),
            ExprKind::ComplexPart { imaginary, operand } => {
                let value = self.constant_vector_value(operand, scope)?;
                if !imaginary {
                    return Ok(value);
                }
                let place = self.allocate_constant_value_storage(facts.ty, node.location)?;
                self.constant_zero_object(place, node.location)?;
                Ok(place)
            }
            ExprKind::Unary { op, operand } => {
                let value = self.constant_vector_value(operand, scope)?;
                if op.token.as_punctuator() == Some(P::Add) {
                    return Ok(value);
                }
                Err(nonconstant(
                    node.location,
                    "a unary operand that is not an integer",
                ))
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
                self.constant_vector_value(selected, scope)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard_constant(left, scope)?;
                self.constant_vector_value(right, scope)
            }
            ExprKind::Call { .. } => {
                let Returned::Place(place) = self.constant_call_result(id, scope)? else {
                    return Err(nonconstant(
                        node.location,
                        "not an integral constant expression",
                    ));
                };
                self.read_vector_storage(place, node.location)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } => {
                self.constant_vector_operand(left, scope)?;
                self.constant_vector_operand(right, scope)?;
                Err(nonconstant(
                    node.location,
                    if matches!(
                        op.token.as_punctuator(),
                        Some(
                            P::Equal
                                | P::NotEqual
                                | P::Less
                                | P::LessEqual
                                | P::Greater
                                | P::GreaterEqual
                        )
                    ) {
                        "a complex comparison outside constant evaluation"
                    } else {
                        "an arithmetic operand that is not an integer"
                    },
                ))
            }
            _ => Err(nonconstant(
                node.location,
                "an expression that is not a constant expression",
            )),
        }
    }
    fn constant_vector_operand(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<(), Error> {
        let facts = self.graph.expression_facts.get(id)?;
        if self.vector_shape(facts.ty).is_some() {
            self.constant_vector_value(id, scope)?;
        } else {
            let target = facts
                .conversion
                .map_or(facts.ty, |conversion| conversion.result);
            self.constant_operand_value(id, target, scope)?;
        }
        Ok(())
    }
    fn constant_vector_cast_arguments(
        &mut self,
        arguments: List<ast::ExprId>,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if let Some(&first) = self.tree.expr_lists.get(arguments).first() {
            return self.constant_vector_cast(first, ty, scope, location);
        }
        let place = self.allocate_constant_value_storage(ty, location)?;
        self.constant_zero_object(place, location)?;
        Ok(place)
    }
    fn read_vector_storage(
        &mut self,
        place: Address,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if self.type_cv(place.designated).0 & 2 != 0 {
            return Err(nonconstant(location, "a read of a volatile glvalue"));
        }
        let place = Address {
            designated: self.bare_type(place.designated)?,
            ..place
        };
        if matches!(place.root, Root::Object(_)) {
            self.constexpr_frames
                .locate(&self.graph.constant_member_spans, place, location)?;
            if self.constant_lane_vector(place.designated).is_none() {
                self.constexpr_frames
                    .read(&self.graph.constant_member_spans, place, location)?;
            }
            return Ok(place);
        }
        let target = self.allocate_constant_value_storage(place.designated, location)?;
        self.copy_constant_object(target, place, location)?;
        Ok(target)
    }
    pub(super) fn constant_vector_assignment(
        &mut self,
        left: ast::ExprId,
        right: ast::ExprId,
        op: Option<P>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if op != Some(P::Assign) {
            let place = self.constant_place(left, scope)?;
            self.read_vector_storage(place, location)?;
            self.constant_vector_operand(right, scope)?;
            return Err(nonconstant(
                location,
                "a compound assignment of values that are not integers",
            ));
        }
        let ty = self.graph.expression_facts.get(left)?.ty;
        let source =
            if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(right).kind {
                if let Some(&first) = self.tree.expr_lists.get(elements).first() {
                    self.constant_vector_cast(first, ty, scope, location)?
                } else {
                    let place = self.allocate_constant_value_storage(ty, location)?;
                    self.constant_zero_object(place, location)?;
                    place
                }
            } else {
                self.constant_vector_cast(right, ty, scope, location)?
            };
        let target = self.constant_place(left, scope)?;
        self.copy_constant_object(target, source, location)?;
        Ok(target)
    }
}
