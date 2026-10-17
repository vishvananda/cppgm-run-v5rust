// Translated from sema/constant_values.h and constant_expressions.cpp. See NOTICE.
//! Symbolic addresses and declaration-owned constant bindings; no host pointers.
use super::{
    Error,
    analyzer::Analyzer,
    constants::Constant,
    expression_facts::ValueCategory,
    graph::{BindingKind, EntityId, ScopeId},
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Root {
    Null,
    Variable(EntityId),
    Function(EntityId),
    Literal(ast::ExprId),
    Temporary(EntityId),
    Ephemeral(ast::ExprId),
    Object(super::constexpr_frames::ObjectId),
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Address {
    pub root: Root,
    pub offset: i64,
    pub past_end: bool,
    pub bit_field: Option<EntityId>,
    // The designated object's type for a place; the pointer's element for a value.
    pub designated: TypeId,
}
impl Address {
    pub(super) fn new(root: Root, designated: TypeId) -> Self {
        Self {
            root,
            offset: 0,
            past_end: false,
            bit_field: None,
            designated,
        }
    }
}
#[derive(Clone, Copy)]
enum State {
    Evaluating,
    Unavailable,
    Value(Address),
}
#[derive(Clone, Copy)]
pub(super) enum TemporaryValue {
    Integer(Constant),
    Floating(super::constant_floats::Floating),
    Pointer(Address),
}
#[derive(Default)]
pub(super) struct Evaluation {
    values: HashMap<ast::ExprId, TemporaryValue>,
}
pub(super) struct Temporaries(HashMap<ast::ExprId, TemporaryValue>);
impl Evaluation {
    pub(super) fn suspend_temporaries(&mut self) -> Temporaries {
        Temporaries(std::mem::take(&mut self.values))
    }
    pub(super) fn resume_temporaries(&mut self, saved: Temporaries) {
        self.values = saved.0;
    }
}
#[derive(Default)]
pub(super) struct Addresses {
    values: HashMap<EntityId, State>,
    temporaries: HashMap<EntityId, TemporaryValue>,
    stored_references: std::collections::HashSet<EntityId>,
}
impl Addresses {
    pub(super) fn read_through_storage(&self, entity: EntityId) -> bool {
        self.stored_references.contains(&entity)
    }
    pub(super) fn temporary_scalar(&self, entity: EntityId) -> Option<TemporaryValue> {
        self.temporaries.get(&entity).copied()
    }
    fn evaluating(&self, entity: EntityId) -> bool {
        matches!(self.values.get(&entity), Some(State::Evaluating))
    }
    fn begin(&mut self, entity: EntityId) {
        self.values.insert(entity, State::Evaluating);
    }
    fn finish(&mut self, entity: EntityId, value: Option<Address>) {
        self.values
            .insert(entity, value.map_or(State::Unavailable, State::Value));
    }
    fn remove(&mut self, entity: EntityId) {
        self.values.remove(&entity);
    }
    pub(super) fn value(&self, entity: EntityId) -> Option<Address> {
        match self.values.get(&entity) {
            Some(State::Value(value)) => Some(*value),
            _ => None,
        }
    }
    pub(super) fn temporary(&self, entity: EntityId) -> Option<Constant> {
        match self.temporaries.get(&entity) {
            Some(TemporaryValue::Integer(value)) => Some(*value),
            _ => None,
        }
    }
    fn temporary_pointer(&self, entity: EntityId) -> Option<Address> {
        match self.temporaries.get(&entity) {
            Some(TemporaryValue::Pointer(value)) => Some(*value),
            _ => None,
        }
    }
}
pub(super) fn nonconstant(location: SourceOffset, message: &'static str) -> Error {
    Error::NonConstant { location, message }
}
impl Analyzer<'_> {
    pub(super) fn in_constant_evaluation<T>(
        &mut self,
        evaluate: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if self.constant_evaluation.is_some() {
            return evaluate(self);
        }
        self.constant_evaluation = Some(std::mem::take(&mut self.constant_scratch));
        let result = evaluate(self);
        let Some(mut evaluation) = self.constant_evaluation.take() else {
            return Err(Error::Invariant("constant evaluation lost its storage"));
        };
        evaluation.values.clear();
        self.constexpr_frames.clear();
        self.constant_scratch = evaluation;
        result
    }

    pub(super) fn expression_temporary_value(&self, id: ast::ExprId) -> Option<TemporaryValue> {
        self.constant_evaluation.as_ref()?.values.get(&id).copied()
    }
    pub(super) fn is_address_expression(&self, id: ast::ExprId) -> bool {
        self.graph.expression_facts.get(id).is_ok_and(|facts| {
            matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Pointer(_)
                    | TypeKind::Array { .. }
                    | TypeKind::Function { .. }
                    | TypeKind::Scalar(Scalar::Nullptr)
            )
        })
    }
    pub(super) fn address_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<Address>, Error> {
        match self.evaluate_address(id, scope) {
            Ok(value) => Ok(Some(value)),
            Err(Error::NonConstant { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub(super) fn evaluate_address(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.in_constant_evaluation(|analyzer| analyzer.evaluate_address_inner(id, scope))
    }
    fn evaluate_address_inner(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        let facts = self.graph.expression_facts.get(id)?;
        let location = node.location;
        self.constant_step(location)?;
        if facts.category == ValueCategory::Prvalue
            && facts.folded == Some(0)
            && self.graph.address_parameter_type(facts.ty)
        {
            return Ok(Address::new(Root::Null, facts.ty));
        }
        match self.graph.types.get(facts.ty).kind {
            TypeKind::Array { element, .. }
            | TypeKind::Function {
                result: element, ..
            } => {
                let mut place = self.constant_place(id, scope)?;
                place.designated =
                    if matches!(self.graph.types.get(facts.ty).kind, TypeKind::Array { .. }) {
                        element
                    } else {
                        facts.ty
                    };
                return Ok(place);
            }
            _ => (),
        }
        let mut value = match node.kind {
            ExprKind::Cast { .. } | ExprKind::FunctionalCast { .. } | ExprKind::Call { .. }
                if facts.reference_cast.is_some_and(|ty| {
                    matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                }) =>
            {
                let place = self.constant_place(id, scope)?;
                self.read_constant_address(place, location)?
            }
            ExprKind::Keyword(Keyword::Nullptr) => Address::new(Root::Null, facts.ty),
            ExprKind::Keyword(Keyword::This) => self.constant_receiver_place(
                super::expression_facts::ObjectSource::Implicit,
                scope,
                location,
            )?,
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::BitAnd) => {
                self.constant_place(operand, scope)?
            }
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::Add) => {
                self.evaluate_address(operand, scope)?
            }
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand }
                if matches!(op.token.as_punctuator(), Some(P::Increment | P::Decrement)) =>
            {
                let (place, old) = self.constant_increment_values(
                    id,
                    operand,
                    op.token.as_punctuator() == Some(P::Decrement),
                    scope,
                )?;
                if matches!(node.kind, ExprKind::Postfix { .. }) {
                    let TemporaryValue::Pointer(old) = old else {
                        return Err(Error::Invariant("pointer increment with integer value"));
                    };
                    old
                } else {
                    self.read_constant_address(place, location)?
                }
            }
            ExprKind::Id(_)
            | ExprKind::Unary { .. }
            | ExprKind::Subscript { .. }
            | ExprKind::Member { .. }
                if facts.category != ValueCategory::Prvalue =>
            {
                let place = self.constant_place(id, scope)?;
                self.read_constant_address(place, location)?
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } => match op.token.as_punctuator() {
                Some(P::Add | P::Subtract) => self.address_arithmetic(id, scope)?,
                Some(P::Comma) => {
                    self.discard_constant(left, scope)?;
                    self.evaluate_address(right, scope)?
                }
                _ => {
                    return Err(nonconstant(
                        location,
                        "pointer arithmetic outside constant evaluation",
                    ));
                }
            },
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: true,
            } => {
                let place = self.constant_assignment_place(
                    id,
                    left,
                    right,
                    op.token.as_punctuator(),
                    scope,
                )?;
                self.read_constant_address(place, location)?
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
                self.address_operand(selected, scope)?
            }
            ExprKind::Cast {
                keyword, operand, ..
            } => {
                if matches!(
                    keyword,
                    Some(Keyword::ReinterpretCast | Keyword::DynamicCast)
                ) {
                    return Err(nonconstant(
                        location,
                        "a reinterpret_cast or dynamic_cast in a constant expression",
                    ));
                }
                let value = self.address_cast_operand(operand, scope, location)?;
                self.check_address_cast(value, operand, facts.ty, location)?;
                value
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => self.functional_address(id, arguments, braced, scope)?,
            ExprKind::Braced { elements, .. } => match self.tree.expr_lists.get(elements) {
                [] => Address::new(Root::Null, facts.ty),
                [operand] => self.address_operand(*operand, scope)?,
                _ => return Err(Error::Invariant("typed scalar list with several elements")),
            },
            ExprKind::Call { arguments, .. } if facts.functional_call => {
                self.functional_address(id, arguments, None, scope)?
            }
            ExprKind::Call { .. } => match self.constant_call_result(id, scope)? {
                super::constexpr_calls::Returned::Value(TemporaryValue::Pointer(value)) => value,
                super::constexpr_calls::Returned::Place(place) => {
                    self.read_constant_address(place, location)?
                }
                _ => return Err(Error::Invariant("pointer call returned no address")),
            },
            _ => {
                return Err(nonconstant(
                    location,
                    "a pointer value outside constant evaluation",
                ));
            }
        };
        if let TypeKind::Pointer(element) = self.graph.types.get(facts.ty).kind {
            value.designated = element;
        }
        self.constexpr_frames.valid_pointer(value, location)?;
        Ok(value)
    }
    pub(super) fn address_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        if self.is_address_expression(id)
            || self
                .graph
                .address_parameter_type(self.graph.expression_facts.get(id)?.ty)
        {
            return self.evaluate_address(id, scope);
        }
        let value = self.evaluate_constant(id, scope, false)?;
        if value.bits != 0 {
            return Err(nonconstant(
                self.tree.expressions.get(id).location,
                "an integer converted to a pointer in a constant expression",
            ));
        }
        Ok(Address::new(Root::Null, value.ty))
    }
    fn functional_address(
        &mut self,
        id: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        braced: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let values = self.tree.expr_lists.get(arguments);
        let list = braced.or_else(|| {
            values.first().copied().filter(|_| {
                values.len() == 1
                    && matches!(
                        self.tree.expressions.get(values[0]).kind,
                        ExprKind::Braced { .. }
                    )
            })
        });
        let values = if let Some(list) = list {
            let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
                return Err(Error::Invariant("pointer cast list without list syntax"));
            };
            self.tree.expr_lists.get(elements)
        } else {
            values
        };
        match values {
            [] => Ok(Address::new(
                Root::Null,
                self.graph.expression_facts.get(id)?.ty,
            )),
            [operand] => {
                self.address_cast_operand(*operand, scope, self.tree.expressions.get(id).location)
            }
            _ => Err(Error::Invariant(
                "typed pointer cast with several arguments",
            )),
        }
    }
    fn address_cast_operand(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        self.address_operand(operand, scope)
            .map_err(|error| match error {
                Error::NonConstant {
                    message: "an integer converted to a pointer in a constant expression",
                    ..
                } => nonconstant(
                    location,
                    "an integer converted to a pointer in a constant expression",
                ),
                other => other,
            })
    }
    fn check_address_cast(
        &self,
        value: Address,
        operand: ast::ExprId,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let source = self.graph.expression_facts.get(operand)?.ty;
        let from = match self.graph.types.get(source).kind {
            TypeKind::Pointer(element) | TypeKind::Array { element, .. } => Some(element),
            TypeKind::Function { .. } => Some(source),
            _ => None,
        };
        if let (Some(from), TypeKind::Pointer(to)) =
            (from, self.graph.types.get(target).kind.clone())
        {
            let from_void = matches!(
                self.graph.types.get(from).kind,
                TypeKind::Scalar(Scalar::Void)
            );
            let to_void = matches!(
                self.graph.types.get(to).kind,
                TypeKind::Scalar(Scalar::Void)
            );
            if from_void
                && !to_void
                && !matches!(self.graph.types.get(to).kind, TypeKind::Function { .. })
            {
                return Err(nonconstant(
                    location,
                    "a cast from void* in a constant expression",
                ));
            }
            if value.root != Root::Null && !to_void && from != to {
                // CV changes have no storage adjustment. Base projection needs
                // the separate class-layout owner rather than a fabricated zero.
                if !self.same_array_shape(from, to) {
                    if self.graph.class_of(from).is_some() && self.graph.class_of(to).is_some() {
                        return Err(Error::Missing("constant base subobject projection"));
                    }
                    return Err(nonconstant(
                        location,
                        "a pointer cast between unrelated types in a constant expression",
                    ));
                }
            }
        }
        Ok(())
    }
    pub(super) fn constant_place(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.in_constant_evaluation(|analyzer| analyzer.constant_place_inner(id, scope))
    }
    fn constant_place_inner(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<Address, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        let facts = self.graph.expression_facts.get(id)?;
        let location = node.location;
        self.constant_step(location)?;
        if self.graph.class_of(facts.ty).is_some() && facts.category == ValueCategory::Prvalue {
            return self.constant_object_place(id, scope);
        }
        match node.kind {
            ExprKind::Id(name) => {
                let binding = facts
                    .function
                    .or_else(|| self.graph.resolved_name(name))
                    .ok_or(Error::Invariant("constant place without selected binding"))?;
                let binding = self.graph.binding(binding);
                if let Some(mut place) = self.constexpr_frames.local(binding.entity) {
                    place.designated = facts.ty;
                    return Ok(place);
                }
                if let Some(mut place) = self.graph.aggregate_constants.constructing(binding.entity)
                {
                    place.designated = facts.ty;
                    return Ok(place);
                }
                match binding.kind {
                    BindingKind::Function => {
                        Ok(Address::new(Root::Function(binding.entity), facts.ty))
                    }
                    BindingKind::Parameter => Err(nonconstant(
                        location,
                        "a parameter is not usable in a constant expression",
                    )),
                    BindingKind::Variable => {
                        let entity = self.graph.entity(binding.entity);
                        if entity.data_member == Some(super::graph::DataMemberKind::Instance)
                            && let Some(object) = self
                                .constexpr_frames
                                .current()
                                .ok()
                                .and_then(|frame| frame.object)
                        {
                            let object = self.constant_project_path(
                                object,
                                facts.member_naming_path,
                                location,
                            )?;
                            let object = self.constant_project_path(
                                object,
                                facts.member_path.or(facts.base_path),
                                location,
                            )?;
                            let object = Address {
                                offset: object.offset.wrapping_add(super::integral::signed(
                                    self.graph.lower_anonymous_member_offset(
                                        binding.entity,
                                        self.graph.class_of(object.designated),
                                    )?,
                                )),
                                ..object
                            };
                            return self.constant_member_designation(
                                object,
                                binding.entity,
                                facts.ty,
                                location,
                            );
                        }
                        if entity.ty.is_some_and(|ty| {
                            matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                        }) {
                            self.instantiate_variable_value(binding.entity)?;
                            let mut place = self
                                .graph
                                .constant_addresses
                                .value(binding.entity)
                                .ok_or_else(|| {
                                    nonconstant(
                                        location,
                                        "a reference is not usable in a constant expression",
                                    )
                                })?;
                            place.designated = facts.ty;
                            return Ok(place);
                        }
                        if entity.data_member == Some(super::graph::DataMemberKind::Instance) {
                            let storage = facts
                                .anonymous_member
                                .ok_or(Error::Missing("constant implicit object designation"))?;
                            let object = self.graph.binding(storage).entity;
                            let owner = self
                                .graph
                                .class_of(self.graph.lower_entity_type(object)?)
                                .ok_or(Error::Invariant("anonymous storage without class"))?;
                            self.class_layout(owner, location)?;
                            let (_, offset) = self
                                .graph
                                .lower_injected_member(id)?
                                .ok_or(Error::Invariant("anonymous member without storage"))?;
                            let mut place = Address::new(Root::Variable(object), facts.ty);
                            place.offset = super::integral::signed(offset);
                            return Ok(place);
                        }
                        Ok(Address::new(Root::Variable(binding.entity), facts.ty))
                    }
                    _ => Err(nonconstant(location, "a name that designates no object")),
                }
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: true,
            } => self.constant_assignment_place(id, left, right, op.token.as_punctuator(), scope),
            ExprKind::Literal(ast::Literal {
                kind: PostTokenKind::String { .. },
                ..
            }) => Ok(Address::new(Root::Literal(id), facts.ty)),
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::Multiply) => {
                let mut value = if matches!(
                    self.graph
                        .types
                        .get(self.graph.expression_facts.get(operand)?.ty)
                        .kind,
                    TypeKind::Function { .. }
                ) {
                    self.constant_place(operand, scope)?
                } else {
                    self.address_operand(operand, scope)?
                };
                if value.root == Root::Null {
                    return Err(nonconstant(
                        location,
                        "an indirection through a null pointer",
                    ));
                }
                value.designated = facts.ty;
                Ok(value)
            }
            ExprKind::Subscript { base, index } => {
                let (base, index) = if facts.commuted {
                    (index, base)
                } else {
                    (base, index)
                };
                let mut value = self.address_operand(base, scope)?;
                let index = self.evaluate_constant(index, scope, false)?;
                if value.root == Root::Null {
                    return Err(nonconstant(location, "a subscript of a null pointer"));
                }
                self.advance_address(&mut value, super::integral::signed(index.bits), location)?;
                value.designated = facts.ty;
                Ok(value)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard_constant(left, scope)?;
                self.constant_place(right, scope)
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
                self.constant_place(selected, scope)
            }
            ExprKind::Cast {
                operand, keyword, ..
            } if facts.reference_cast.is_some() => {
                self.constant_reference_cast_place(id, operand, keyword, scope)
            }
            ExprKind::Unary { op, operand }
                if matches!(op.token.as_punctuator(), Some(P::Increment | P::Decrement)) =>
            {
                self.constant_increment_values(
                    id,
                    operand,
                    op.token.as_punctuator() == Some(P::Decrement),
                    scope,
                )
                .map(|(place, _)| place)
            }
            ExprKind::FunctionalCast { arguments, .. } => {
                let [operand] = self.tree.expr_lists.get(arguments) else {
                    return Err(nonconstant(
                        location,
                        "a glvalue outside constant evaluation",
                    ));
                };
                self.constant_place(*operand, scope)
            }
            ExprKind::Call { arguments, .. } if facts.functional_call => {
                let [operand] = self.tree.expr_lists.get(arguments) else {
                    return Err(nonconstant(
                        location,
                        "a glvalue outside constant evaluation",
                    ));
                };
                self.constant_place(*operand, scope)
            }
            ExprKind::Member { base, op, .. } => self.constant_explicit_member_place(
                id,
                base,
                op.token.as_punctuator() == Some(P::Arrow),
                scope,
            ),
            ExprKind::Call { .. } => match self.constant_call_result(id, scope)? {
                super::constexpr_calls::Returned::Place(place) => Ok(place),
                _ => Err(Error::Missing("constexpr class result storage")),
            },
            ExprKind::Literal(_) | ExprKind::Keyword(_) | ExprKind::Unary { .. } => Err(
                nonconstant(location, "a glvalue outside constant evaluation"),
            ),
            _ => Err(Error::Missing("extended constant place evaluation")),
        }
    }
    fn constant_explicit_member_place(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        arrow: bool,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        let location = self.tree.expressions.get(id).location;
        let mut place = if arrow {
            self.address_operand(base, scope)?
        } else {
            self.constant_object_place(base, scope)?
        };
        let binding = facts
            .member
            .ok_or(Error::Missing("constant member place"))?;
        let entity = self.graph.binding(binding).entity;
        if self.graph.entity(entity).data_member == Some(super::graph::DataMemberKind::Instance) {
            let owner = facts
                .member_naming
                .ok_or(Error::Invariant("member place without class owner"))?;
            if place.root == Root::Null {
                return Err(nonconstant(
                    location,
                    "a member access through a null pointer",
                ));
            }
            self.class_layout(owner, location)?;
            place.offset = place.offset.wrapping_add(super::integral::signed(
                self.graph.lower_anonymous_field_offset(id)?,
            ));
            place = self.constant_project_path(place, facts.member_naming_path, location)?;
            place = self.constant_project_path(place, facts.member_path, location)?;
            return self.constant_member_designation(place, entity, facts.ty, location);
        }
        if self.graph.entity(entity).data_member == Some(super::graph::DataMemberKind::Static) {
            if self.graph.entity(entity).ty.is_some_and(|ty| {
                matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            }) {
                self.instantiate_variable_value(entity)?;
                let mut value = self.graph.constant_addresses.value(entity).ok_or_else(|| {
                    nonconstant(
                        location,
                        "a reference is not usable in a constant expression",
                    )
                })?;
                value.designated = facts.ty;
                return Ok(value);
            }
            return Ok(Address::new(Root::Variable(entity), facts.ty));
        }
        Err(Error::Invariant(
            "constant member designation without data member",
        ))
    }

    fn constant_reference_cast_place(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        keyword: Option<Keyword>,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        let location = self.tree.expressions.get(id).location;
        if matches!(
            keyword,
            Some(Keyword::ReinterpretCast | Keyword::DynamicCast)
        ) {
            return Err(nonconstant(
                location,
                "a reinterpret_cast or dynamic_cast in a constant expression",
            ));
        }
        let source = self.graph.expression_facts.get(operand)?;
        if source.category == ValueCategory::Prvalue
            || source
                .conversion
                .is_some_and(|conversion| conversion.temporary)
        {
            if self.constexpr_frames.active() {
                let value = self.constant_operand_value(operand, facts.ty, scope)?;
                return self.allocate_constant_object(facts.ty, Some(value), location);
            }
            if self.expression_temporary_value(id).is_none() {
                let value = if self.is_address_expression(id) {
                    TemporaryValue::Pointer(self.address_operand(operand, scope)?)
                } else if self.float_format(facts.ty).is_some() || self.floating_expression(operand)
                {
                    self.constant_operand_value(operand, facts.ty, scope)?
                } else {
                    let value = self.evaluate_constant(operand, scope, false)?;
                    let kind = self
                        .integral_kind(facts.ty)
                        .ok_or(Error::Missing("constant nonintegral cast temporary"))?;
                    TemporaryValue::Integer(Constant {
                        bits: kind.convert(value.bits),
                        ty: self.bare_type(facts.ty)?,
                    })
                };
                self.constant_evaluation
                    .as_mut()
                    .ok_or(Error::Invariant("temporary outside constant evaluation"))?
                    .values
                    .insert(id, value);
            }
            return Ok(Address::new(Root::Ephemeral(id), facts.ty));
        }
        let mut place = self.constant_place(operand, scope)?;
        place.designated = facts.ty;
        Ok(place)
    }
    pub(super) fn read_constant_address(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if self.graph.types.get(place.designated).cv.0 & 2 != 0 {
            return Err(nonconstant(location, "a read of a volatile glvalue"));
        }
        if let Some(value) = self.published_scalar(place, location)? {
            return match value {
                TemporaryValue::Pointer(value) => Ok(value),
                _ => Err(Error::Invariant("published scalar read as pointer")),
            };
        }
        if matches!(place.root, Root::Object(_)) {
            place.designated = self.bare_type(place.designated)?;
            return match self.constexpr_frames.read(place, location)? {
                TemporaryValue::Pointer(value) => {
                    self.constexpr_frames.valid_pointer(value, location)?;
                    Ok(value)
                }
                TemporaryValue::Integer(_) | TemporaryValue::Floating(_) => {
                    Err(Error::Invariant("integer object read as pointer"))
                }
            };
        }
        if let Root::Variable(entity) = place.root {
            self.instantiate_variable_value(entity)?;
            if !self.graph.entity(entity).constexpr
                && self.graph.constant_addresses.value(entity).is_none()
                && !self.graph.constant_addresses.evaluating(entity)
            {
                return Err(nonconstant(
                    location,
                    "a variable is not usable in a constant expression",
                ));
            }
            if place.offset != 0 || place.past_end {
                return Err(Error::Missing("constant pointer subobject read"));
            }
            if self.graph.constant_addresses.evaluating(entity) {
                return Err(nonconstant(
                    location,
                    "a read of an object that is not initialized",
                ));
            }
            return self.graph.constant_addresses.value(entity).ok_or_else(|| {
                nonconstant(
                    location,
                    "a variable is not usable in a constant expression",
                )
            });
        }
        if let Root::Temporary(entity) = place.root {
            if self.graph.types.get(place.designated).cv != super::types::Cv::CONST {
                return Err(nonconstant(
                    location,
                    "an access to a temporary that is not const, bound outside the evaluation",
                ));
            }
            return self
                .graph
                .constant_addresses
                .temporary_pointer(entity)
                .ok_or(Error::Missing("constant temporary pointer storage"));
        }
        if let Root::Ephemeral(expr) = place.root {
            return match self.expression_temporary_value(expr) {
                Some(TemporaryValue::Pointer(value)) => Ok(value),
                _ => Err(Error::Invariant("cast pointer temporary without value")),
            };
        }
        Err(Error::Missing("constant pointer place storage"))
    }
    pub(super) fn write_constant_value(
        &mut self,
        place: Address,
        value: TemporaryValue,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if place.past_end {
            return Err(nonconstant(
                location,
                "an access one past the end of an array",
            ));
        }
        let message = match place.root {
            Root::Object(_) => {
                if self.type_cv(place.designated).0 != 0 {
                    return Err(nonconstant(
                        location,
                        "a modification of a const or volatile object",
                    ));
                }
                let place = Address {
                    designated: self.bare_type(place.designated)?,
                    ..place
                };
                return self.store_constant_value(place, value, location);
            }
            Root::Variable(_) => {
                "a modification of an object whose lifetime began outside the evaluation"
            }
            Root::Literal(_) => "a modification of a string literal",
            Root::Temporary(_) => {
                "an access to a temporary that is not const, bound outside the evaluation"
            }
            Root::Function(_) => "a function is not an object",
            Root::Null => "an indirection through a null pointer",
            Root::Ephemeral(id) => {
                let old = self
                    .expression_temporary_value(id)
                    .ok_or(Error::Invariant("write to an uncreated constant temporary"))?;
                let matches = match old {
                    TemporaryValue::Integer(old) => self.bare_type(place.designated)? == old.ty,
                    TemporaryValue::Floating(old) => self.bare_type(place.designated)? == old.ty,
                    TemporaryValue::Pointer(_) => matches!(
                        self.graph.types.get(place.designated).kind,
                        TypeKind::Pointer(_)
                    ),
                };
                if place.offset != 0 || !matches {
                    return Err(nonconstant(
                        location,
                        "an access outside the designated object or to an inactive union member",
                    ));
                }
                self.constant_evaluation
                    .as_mut()
                    .ok_or(Error::Invariant("write outside constant evaluation"))?
                    .values
                    .insert(id, value);
                return Ok(());
            }
        };
        Err(nonconstant(location, message))
    }

    pub(super) fn read_constant_integer(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        if self.graph.types.get(place.designated).cv.0 & 2 != 0 {
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
                TemporaryValue::Integer(value) => Ok(value),
                _ => Err(Error::Invariant("published scalar read as integer")),
            };
        }
        match place.root {
            Root::Object(_) => {
                place.designated = self.bare_type(place.designated)?;
                match self.constexpr_frames.read(place, location)? {
                    TemporaryValue::Integer(value) => Ok(value),
                    TemporaryValue::Pointer(_) | TemporaryValue::Floating(_) => {
                        Err(Error::Invariant("pointer object read as integer"))
                    }
                }
            }
            Root::Variable(entity) => {
                self.instantiate_variable_value(entity)?;
                if !self.graph.entity(entity).constexpr
                    && matches!(
                        self.graph.entity(entity).constant,
                        super::constants::ConstantState::None
                    )
                {
                    return Err(nonconstant(
                        location,
                        "a variable is not usable in a constant expression",
                    ));
                }
                if place.bit_field.is_some()
                    || place.offset != 0
                    || self
                        .integral_kind(
                            self.graph
                                .entity(entity)
                                .ty
                                .ok_or(Error::Invariant("constant object without type"))?,
                        )
                        .is_none()
                {
                    return Err(Error::Missing("constant integral subobject storage"));
                }
                let ty = self
                    .graph
                    .entity(entity)
                    .ty
                    .ok_or(Error::Invariant("constant object without type"))?;
                if self.bare_type(ty)? != self.bare_type(place.designated)? {
                    return Err(nonconstant(
                        location,
                        "an access outside the designated object",
                    ));
                }
                match self.graph.entity(entity).constant {
                    super::constants::ConstantState::Value(value) => Ok(value),
                    super::constants::ConstantState::Evaluating => Err(nonconstant(
                        location,
                        "a read of an object that is not initialized",
                    )),
                    super::constants::ConstantState::None
                    | super::constants::ConstantState::Floating(_) => Err(nonconstant(
                        location,
                        "a variable is not usable in a constant expression",
                    )),
                }
            }
            Root::Temporary(entity) => {
                if self.type_cv(place.designated) != super::types::Cv::CONST {
                    return Err(nonconstant(
                        location,
                        "an access to a temporary that is not const, bound outside the evaluation",
                    ));
                }
                self.graph
                    .constant_addresses
                    .temporary(entity)
                    .ok_or(Error::Missing("constant temporary scalar storage"))
            }
            Root::Ephemeral(expr) => match self.expression_temporary_value(expr) {
                Some(TemporaryValue::Integer(value)) => Ok(value),
                _ => Err(Error::Invariant("cast temporary without value")),
            },
            Root::Literal(id) => {
                let ExprKind::Literal(literal) = self.tree.expressions.get(id).kind else {
                    return Err(Error::Invariant("literal place without literal syntax"));
                };
                let ty = self.bare_type(place.designated)?;
                let kind = self
                    .integral_kind(ty)
                    .ok_or(Error::Invariant("literal place without integral element"))?;
                let width = usize::try_from(kind.bits / 8)
                    .map_err(|_| Error::Limit("literal element width"))?;
                let offset = usize::try_from(place.offset).map_err(|_| {
                    nonconstant(location, "an access outside the designated object")
                })?;
                let data = self
                    .literal_bytes
                    .get(literal.data)
                    .get(
                        offset
                            ..offset
                                .checked_add(width)
                                .ok_or(Error::Limit("literal element host offset"))?,
                    )
                    .ok_or_else(|| {
                        nonconstant(location, "an access outside the designated object")
                    })?;
                let mut bytes = [0u8; 8];
                bytes[..data.len()].copy_from_slice(data);
                Ok(Constant {
                    bits: kind.convert(u64::from_le_bytes(bytes)),
                    ty,
                })
            }
            Root::Null => Err(nonconstant(
                location,
                "an indirection through a null pointer",
            )),
            Root::Function(_) => Err(nonconstant(location, "a function is not an object")),
        }
    }
    pub(super) fn discard_constant(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let base = self.constant_discard_work.len();
        self.constant_discard_work.push(id);
        let result = (|| {
            while self.constant_discard_work.len() > base {
                self.check_stack()?;
                let id = self
                    .constant_discard_work
                    .pop()
                    .ok_or(Error::Invariant("discard work disappeared"))?;
                let id = self.graph.expression_facts.root(id);
                let node = *self.tree.expressions.get(id);
                let facts = self.graph.expression_facts.get(id)?;
                let void = matches!(
                    self.graph.types.get(facts.ty).kind,
                    TypeKind::Scalar(Scalar::Void)
                );
                match node.kind {
                    ExprKind::Binary {
                        op,
                        left,
                        right,
                        assignment: false,
                    } if op.token.as_punctuator() == Some(P::Comma) => {
                        self.constant_discard_work.push(right);
                        self.constant_discard_work.push(left);
                    }
                    ExprKind::Cast { operand, .. } if void => {
                        self.constant_discard_work.push(operand);
                    }
                    ExprKind::Conditional {
                        condition,
                        then_expr,
                        else_expr,
                    } if void => {
                        let selected = self.constant_truth(condition, scope, false)?;
                        self.constant_discard_work.push(if selected {
                            then_expr
                        } else {
                            else_expr
                        });
                    }
                    ExprKind::Call { .. } if void => {
                        self.evaluate_constant(id, scope, false)?;
                    }
                    _ if void => {
                        return Err(nonconstant(
                            node.location,
                            "a void expression outside constant evaluation",
                        ));
                    }
                    _ if facts.category != ValueCategory::Prvalue => {
                        self.constant_place(id, scope)?;
                    }
                    _ if self.graph.class_of(facts.ty).is_some() => {
                        self.constant_object_place(id, scope)?;
                    }
                    _ if self.is_address_expression(id) => {
                        self.evaluate_address(id, scope)?;
                    }
                    _ if self.floating_expression(id) => {
                        self.evaluate_floating(id, scope)?;
                    }
                    _ => {
                        self.evaluate_constant(id, scope, false)?;
                    }
                }
            }
            Ok(())
        })();
        self.constant_discard_work.truncate(base);
        result
    }
    pub(super) fn record_address_initializer(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("constant address object without type"))?;
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        if !reference
            && (!self.graph.entity(entity).constexpr
                || !matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)
                ))
        {
            return Ok(());
        }
        if reference
            && self.graph.entity(entity).constexpr
            && let Some(mut expr) = self.single_initializer(initializer)
        {
            while let ExprKind::Paren(inner) = self.tree.expressions.get(expr).kind {
                expr = inner;
            }
            if matches!(
                self.tree.expressions.get(expr).kind,
                ExprKind::Conditional { .. }
            ) {
                self.graph
                    .constant_addresses
                    .stored_references
                    .insert(entity);
            }
        }
        self.graph.constant_addresses.begin(entity);
        let result = (|| {
            let expr = self.single_initializer(initializer);
            if reference {
                let expr = expr.ok_or_else(|| {
                    nonconstant(initializer.location, "a reference without an initializer")
                })?;
                let facts = self.graph.expression_facts.get(expr)?;
                if facts.category != ValueCategory::Prvalue
                    && !facts.conversion.is_some_and(|c| c.temporary)
                    || matches!(
                        self.graph.types.get(facts.ty).kind,
                        TypeKind::Function { .. }
                    )
                {
                    let place = self.constant_place(expr, scope)?;
                    return Ok(place);
                }
                if self.is_address_expression(expr) {
                    let value = self.evaluate_address(expr, scope)?;
                    self.graph
                        .constant_addresses
                        .temporaries
                        .insert(entity, TemporaryValue::Pointer(value));
                    return Ok(Address::new(Root::Temporary(entity), facts.ty));
                }
                let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind else {
                    return Err(Error::Invariant("reference temporary type"));
                };
                let value =
                    if self.float_format(element).is_some() || self.floating_expression(expr) {
                        self.constant_operand_value(expr, element, scope)?
                    } else {
                        TemporaryValue::Integer(self.evaluate_constant(expr, scope, false)?)
                    };
                self.graph
                    .constant_addresses
                    .temporaries
                    .insert(entity, value);
                return Ok(Address::new(Root::Temporary(entity), facts.ty));
            }
            let mut value = if let Some(expr) = expr {
                self.address_operand(expr, scope)?
            } else {
                Address::new(Root::Null, ty)
            };
            if let TypeKind::Pointer(element) = self.graph.types.get(ty).kind {
                value.designated = element;
            }
            Ok(value)
        })();
        match result {
            Ok(value) if self.address_publishable(value) => {
                self.graph.constant_addresses.finish(entity, Some(value));
                Ok(())
            }
            Ok(_) => {
                self.graph.constant_addresses.finish(entity, None);
                if self.graph.entity(entity).constexpr {
                    Err(Error::at(
                        initializer.location,
                        "a constant designates an object without static storage duration",
                    ))
                } else {
                    Ok(())
                }
            }
            Err(Error::NonConstant { location, message }) => {
                self.graph.constant_addresses.finish(entity, None);
                if self.graph.entity(entity).constexpr {
                    Err(Error::at(location, message))
                } else {
                    Ok(())
                }
            }
            Err(error) => {
                self.graph.constant_addresses.remove(entity);
                Err(error)
            }
        }
    }
    fn address_publishable(&self, address: Address) -> bool {
        match address.root {
            Root::Ephemeral(_) | Root::Object(_) => false,
            Root::Variable(entity) => {
                self.graph
                    .entity(entity)
                    .variable
                    .is_some_and(|properties| {
                        !properties.automatic
                            && properties.thread_storage
                                == super::specifiers::ThreadStorage::Ordinary
                    })
            }
            _ => true,
        }
    }
}
