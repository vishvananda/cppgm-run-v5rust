// Translated from lower/global_lowering.cpp's static_value/static_converted queries. See NOTICE.
//! Language values remain recorded; automatic array images are queried at emitted declarations.
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
    types::{
        FundamentalType as F,
        float_value::{BinaryFloat, Format},
        floating::FloatingScanner,
    },
};
#[derive(Clone, Copy)]
pub(crate) enum Value {
    Integer(u64),
    WideInteger(u128),
    MemberPointer(super::symbolic_members::MemberPointer),
    Floating(F, u128),
    Address(EntityId, i64),
    TypeInformation(TypeId, i64),
    Literal(ast::ExprId, i64),
    Temporary(super::constexpr_values::TemporaryId, i64),
}
#[derive(Clone, Copy)]
pub(crate) struct ReferenceTemporary {
    pub binding: TypeId,
    pub value: TypeId,
}
impl super::graph::Graph {
    pub(crate) fn lower_demand_array_image(
        &mut self,
        tree: &mut ast::Tree,
        cursor: &mut crate::syntax::cursor::Cursor<'_>,
        budget: crate::support::stack::StackBudget,
        entity: EntityId,
    ) -> Result<(), Error> {
        let request = self.array_images.requested(entity).ok_or(Error::Invariant(
            "automatic array image without declaration request",
        ))?;
        let graph = std::mem::replace(self, Self::new());
        let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(graph))?;
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.graph.array_images.clear(entity);
        let result = (|| {
            if !analyzer.record_completed_image(entity)? {
                let target = analyzer.graph.lower_entity_type(entity)?;
                analyzer.record_array_image(entity, request.initializer, target, request.scope)?;
            }
            Ok(())
        })();
        *self = analyzer.graph;
        result
    }
    /// The shared lowering policy for automatic array images. Class leaves
    /// always retain construction; mutable arrays require scalar elements.
    pub(crate) fn lower_array_image_eligible(&self, entity: EntityId) -> Result<bool, Error> {
        let ty = self.lower_entity_type(entity)?;
        let TypeKind::Array { element, bound, .. } = self.types.get(ty).kind else {
            return Ok(false);
        };
        let mut leaf = element;
        while let TypeKind::Array { element, .. } = self.types.get(leaf).kind {
            leaf = element;
        }
        if matches!(self.types.get(leaf).kind, TypeKind::Class(_))
            || self.types.get(leaf).cv.0 & 6 != 0
        {
            return Ok(false);
        }
        if self.entity(entity).constexpr {
            return Ok(true);
        }
        Ok(bound != 0
            && self.types.get(ty).cv.0 & 6 == 0
            && self.types.get(element).cv.0 & 6 == 0
            && match self.types.get(element).kind {
                TypeKind::Scalar(scalar) => scalar != Scalar::Void,
                TypeKind::Enum(_) | TypeKind::Pointer(_) => true,
                _ => false,
            })
    }
}
impl Analyzer<'_> {
    pub(super) fn static_storage_value(&self, value: Value) -> bool {
        !matches!(value, Value::Address(entity, _) if self.graph.entity(entity).variable
            .is_some_and(|variable| variable.thread_storage == super::specifiers::ThreadStorage::ThreadLocal))
    }
    pub(super) fn finish_static_initializers(&mut self) -> Result<(), Error> {
        // A new unit-end initialization query may finish a reference whose
        // earlier declaration-time binding was unavailable. Preserve request
        // order: a preceding dynamic initializer cannot see a later retry.
        for (entity, initializer, scope) in std::mem::take(&mut self.static_initializers) {
            let ty = self.graph.lower_entity_type(entity)?;
            if self.vector_shape(ty).is_some()
                && let Some(initializer) = initializer
                && self.record_vector_list_image(entity, initializer, scope)?
            {
                continue;
            }
            if self.graph.lower_automatic(entity) {
                if let Some(initializer) = initializer {
                    self.record_static_initializer(entity, initializer, scope)?;
                }
                continue;
            }
            if let Some(initializer) = initializer
                && let ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr) =
                    initializer.kind
                && self.graph.lower_flexible_elements(expr)
            {
                // The evaluated constant image has the class's nominal size;
                // only this initializer's physical image includes the tail.
                self.record_static_initializer(entity, initializer, scope)?;
                continue;
            }
            let constructed = self
                .graph
                .lower_construction(entity)
                .is_some_and(|id| !self.graph.lower_constructor_trivial(id));
            if !self.graph.lower_variable(entity)?.thread_local
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Class(_)
                        | TypeKind::Complex(_)
                        | TypeKind::Vector { .. }
                        | TypeKind::Array { .. }
                )
                && !self
                    .graph
                    .class_of(ty)
                    .is_some_and(|class| self.graph.lower_polymorphic(class))
            {
                if constructed || self.complex_part_type(ty).is_some() {
                    self.record_aggregate_constant(entity, initializer, scope, false)?;
                }
                if self.record_completed_image(entity)? {
                    continue;
                }
            }
            if let Some(initializer) = initializer {
                if self.graph.constant_addresses.value(entity).is_none() {
                    self.record_address_initializer(entity, initializer, scope)?;
                }
                self.record_static_initializer(entity, initializer, scope)?;
            }
            // global_lowering.cpp::static_global retries initialization_value
            // after static_object cannot form an aggregate's image. This is
            // still a probe: core constant failures retain dynamic startup.
            if !constructed
                && initializer.is_some()
                && !self.graph.lower_variable(entity)?.thread_local
                && self.graph.lower_array_image(entity).is_none()
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Class(_)
                        | TypeKind::Complex(_)
                        | TypeKind::Vector { .. }
                        | TypeKind::Array { .. }
                )
                && !self
                    .graph
                    .class_of(ty)
                    .is_some_and(|class| self.graph.lower_polymorphic(class))
            {
                self.record_aggregate_constant(entity, initializer, scope, false)?;
                if self.record_completed_image(entity)? {
                    continue;
                }
            }
            self.record_static_construction(entity, initializer, scope)?;
        }
        self.finish_variable_storage_layouts()
    }
    pub(super) fn request_static_initializer(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        home: ScopeId,
    ) -> Result<(), Error> {
        let eligible_array = self.graph.lower_array_image_eligible(entity)?;
        if self.graph.lower_automatic(entity) {
            if let Some(initializer) = initializer {
                self.queue_local_scalar(entity, initializer, scope)?;
                if eligible_array {
                    self.graph.array_images.request(entity, initializer, scope);
                }
            }
            if eligible_array {
                return Ok(());
            }
        }
        if self.graph.scope(home).kind == super::graph::ScopeKind::Namespace
            || !self.graph.lower_automatic(entity)
        {
            self.static_initializers.push((entity, initializer, scope));
        }
        Ok(())
    }
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
        if let TypeKind::Reference { element, .. } = self.graph.types.get(target).kind {
            let Some(expr) = self.single_initializer(initializer) else {
                return Ok(());
            };
            let facts = self.graph.expression_facts.get(expr)?;
            self.graph
                .constant_addresses
                .set_reference_evaluated(entity, false);
            if facts.conversion.is_some_and(|c| c.temporary) {
                let object = facts.conversion.map_or(element, |c| c.result);
                let value = if self.graph.class_of(object).is_some()
                    || matches!(
                        self.graph.types.get(object).kind,
                        TypeKind::MemberPointer { .. }
                    ) {
                    None
                } else {
                    self.static_scalar(expr, object, scope)?
                };
                // define_reference first tries its syntax-shaped scalar. A
                // failed view falls back to the already published core value.
                if value.is_none()
                    && let Some(id) = self.graph.lower_extended_temporary(entity)
                    && let Some(value) = self.graph.static_temporary_value(id, 0)
                {
                    let variable = self.graph.entity_mut(entity);
                    variable.static_initializer = Some(value);
                    variable.reference_temporary = None;
                    return Ok(());
                }
                let value_type = self.bare_type(object)?;
                let entity_id = entity;
                let entity = self.graph.entity_mut(entity);
                entity.reference_temporary = Some(ReferenceTemporary {
                    binding: object,
                    value: value_type,
                });
                entity.static_initializer = value;
                if value.is_some() {
                    self.record_static_reference_storage(entity_id, object);
                }
            } else {
                let address = self.static_binding(entity, expr, scope);
                let value = match address {
                    Ok(address) => match address.root {
                        Root::Variable(root)
                            if self
                                .graph
                                .entity(root)
                                .variable
                                .is_some_and(|v| !v.automatic) =>
                        {
                            Some(Value::Address(root, address.offset))
                        }
                        Root::Function(root) => Some(Value::Address(root, address.offset)),
                        Root::Published(root) => {
                            self.graph.static_temporary_value(root, address.offset)
                        }
                        Root::Literal(root) => Some(Value::Literal(root, address.offset)),
                        Root::TypeInformation(ty) => {
                            Some(Value::TypeInformation(ty, address.offset))
                        }
                        _ => None,
                    },
                    Err(Error::NonConstant { .. } | Error::Missing(_)) => None,
                    Err(error) => return Err(error),
                };
                self.graph.entity_mut(entity).static_initializer =
                    value.filter(|value| self.static_storage_value(*value));
            }
            return Ok(());
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Array { .. } | TypeKind::Class(_)
        ) {
            return self.record_array_image(entity, initializer, target, scope);
        }
        if !matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(_)
                | TypeKind::Enum(_)
                | TypeKind::Pointer(_)
                | TypeKind::MemberPointer { .. }
        ) {
            return Ok(());
        }
        // initialization_value consumes the already evaluated variable before
        // asking for another constant probe. Its initializer may be a call or
        // size query whose expression has no scalar fold fact of its own.
        let recorded = match self.graph.entity(entity).constant {
            super::constants::ConstantState::Value(value) => {
                Some(super::constant_addresses::TemporaryValue::Integer(value))
            }
            super::constants::ConstantState::Floating(id) => {
                Some(super::constant_addresses::TemporaryValue::Floating(
                    self.graph.floating_constants.get(id),
                ))
            }
            super::constants::ConstantState::MemberPointer(value) => Some(
                super::constant_addresses::TemporaryValue::MemberPointer(value),
            ),
            _ => self
                .graph
                .constant_addresses
                .value(entity)
                .map(super::constant_addresses::TemporaryValue::Pointer),
        };
        let held = recorded
            .map(|value| self.static_held_scalar(value))
            .transpose()?
            .flatten();
        let mut value = if held.is_some() {
            held
        } else if let Some(expr) = self.single_initializer(initializer) {
            self.static_scalar(expr, target, scope)?
        } else if matches!(
            self.graph.types.get(target).kind,
            TypeKind::MemberPointer { .. }
        ) {
            Some(Value::MemberPointer(
                super::symbolic_members::MemberPointer::null(self.bare_type(target)?),
            ))
        } else {
            Some(Value::Integer(0))
        };
        // static_global first asks for a syntax-shaped value. An integral
        // source has already had its full constant query; other scalar
        // initializations can still require construction and conversion.
        if value.is_none()
            && recorded.is_none()
            && !self.graph.lower_variable(entity)?.thread_local
            && self.single_initializer(initializer).is_some_and(|expr| {
                self.graph
                    .expression_facts
                    .known(expr)
                    .is_some_and(|facts| self.integral_kind(facts.ty).is_none())
            })
        {
            let probe = self.with_fresh_constant_query(|analyzer| {
                analyzer.initialized_scalar_value(target, initializer, scope)
            });
            value = match probe {
                Ok(core) => self.static_held_scalar(core)?,
                Err(Error::NonConstant { .. } | Error::Missing(_)) => None,
                Err(error) => return Err(error),
            };
        }
        self.graph.entity_mut(entity).static_initializer = value;
        Ok(())
    }
    pub(super) fn static_scalar(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<Option<Value>, Error> {
        self.check_stack()?;
        let source = self.graph.expression_facts.get(id)?.ty;
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::MemberPointer { .. }
        ) {
            return match self
                .with_fresh_constant_query(|a| a.member_pointer_operand(id, target, scope))
            {
                Ok(value) => Ok(Some(Value::MemberPointer(value))),
                Err(Error::NonConstant { .. } | Error::Missing(_)) => Ok(None),
                Err(error) => Err(error),
            };
        }
        // global_lowering.cpp::static_converted: only a bare lambda's
        // conversion is an effect-free static function address.
        if matches!(
            self.tree
                .expressions
                .get(self.graph.expression_facts.root(id))
                .kind,
            ExprKind::Lambda(_)
        ) && matches!(self.graph.types.get(target).kind, TypeKind::Pointer(_))
            && let Some(conversion) = self.graph.expression_facts.function_conversion(id)
            && let Some(invoker) = self
                .graph
                .lower_closure_converter(self.graph.binding(conversion.binding).entity)
        {
            return Ok(Some(Value::Address(invoker, 0)));
        }
        let value = self.static_scalar_value(id, scope)?;
        if let Some(Value::Floating(ty, bits)) = value
            && let Some(kind) = self.integral_kind(target)
            && kind.bits != 1
            && !crate::types::interchange::soft(ty)
        {
            let floating = Format::of(ty)
                .and_then(|format| BinaryFloat::from_bits(format, bits))
                .ok_or(Error::Invariant("static floating integer value"))?;
            if floating.integral_wide(kind.bits, kind.signed).is_none()
                && !self.static_floating_fallback(id, scope)?
            {
                return Ok(None);
            }
        }
        value.map_or(Ok(None), |value| {
            self.static_conversion(value, source, target)
        })
    }
    // When constant conversion fails, global_lowering.cpp::static_value
    // folds only its explicit expression forms. In particular a signaling
    // builtin, a named floating object or a constexpr call is not this fallback.
    fn static_floating_fallback(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<bool, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let facts = self.graph.expression_facts.get(id)?;
        if facts.literal_call.is_some()
            || facts.call_form == super::expression_facts::CallForm::Operator
        {
            return Ok(false);
        }
        Ok(match self.tree.expressions.get(id).kind {
            ExprKind::Literal(_) => true,
            ExprKind::Paren(inner) | ExprKind::Cast { operand: inner, .. } => {
                self.static_floating_fallback(inner, scope)?
            }
            ExprKind::Unary { operand, op }
                if matches!(op.token.as_punctuator(), Some(P::Add | P::Subtract)) =>
            {
                self.static_floating_fallback(operand, scope)?
            }
            ExprKind::Binary {
                left,
                right,
                op,
                assignment: false,
            } if matches!(
                op.token.as_punctuator(),
                Some(P::Add | P::Subtract | P::Multiply | P::Divide)
            ) =>
            {
                self.static_floating_fallback(left, scope)?
                    && self.static_floating_fallback(right, scope)?
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => match self.constant_value(condition, scope, false)? {
                Some(value) => self.static_floating_fallback(
                    if value.bits != 0 {
                        then_expr
                    } else {
                        else_expr
                    },
                    scope,
                )?,
                None => false,
            },
            ExprKind::Call { .. } if facts.intrinsic.is_some() => {
                facts.intrinsic == Some(super::floating_intrinsics::FloatingIntrinsic::Constant)
            }
            ExprKind::FunctionalCast { arguments, .. } | ExprKind::Call { arguments, .. }
                if facts.functional_call
                    || matches!(
                        self.tree.expressions.get(id).kind,
                        ExprKind::FunctionalCast { .. }
                    ) =>
            {
                match self.tree.expr_lists.get(arguments) {
                    [operand] => self.static_floating_fallback(*operand, scope)?,
                    _ => false,
                }
            }
            _ => false,
        })
    }
    fn static_held_scalar(
        &self,
        value: super::constant_addresses::TemporaryValue,
    ) -> Result<Option<Value>, Error> {
        if value.indeterminate() {
            return Ok(None);
        }
        let value = match value {
            super::constant_addresses::TemporaryValue::MemberPointer(value) => {
                Some(Value::MemberPointer(value))
            }
            super::constant_addresses::TemporaryValue::Integer(value) => {
                Some(self.static_integer_value(value))
            }
            super::constant_addresses::TemporaryValue::Floating(value) => {
                self.static_floating_constant(value)?
            }
            super::constant_addresses::TemporaryValue::Pointer(value) => match value.root {
                Root::Null => Some(Value::Integer(0)),
                Root::Variable(entity)
                    if self
                        .graph
                        .entity(entity)
                        .variable
                        .is_some_and(|variable| !variable.automatic) =>
                {
                    Some(Value::Address(entity, value.offset))
                }
                Root::Function(entity) => Some(Value::Address(entity, value.offset)),
                Root::Published(id) => self.graph.static_temporary_value(id, value.offset),
                Root::Literal(expr) => Some(Value::Literal(expr, value.offset)),
                Root::TypeInformation(ty) => Some(Value::TypeInformation(ty, value.offset)),
                _ => None,
            },
        };
        Ok(value.filter(|value| self.static_storage_value(*value)))
    }
    pub(super) fn static_floating_constant(
        &self,
        value: super::constant_floats::Floating,
    ) -> Result<Option<Value>, Error> {
        let TypeKind::Scalar(Scalar::Basic(kind)) = self.graph.types.get(value.ty).kind else {
            return Err(Error::Invariant("stored floating constant type"));
        };
        // constant_item/constant_rvalue can store a held bfloat value, but
        // binary16 and binary128 objects retain their runtime initialization.
        if value.state == super::constant_addresses::ScalarState::Indeterminate
            || matches!(kind.floating_representation(), F::Float16 | F::Float128)
        {
            return Ok(None);
        }
        let format = self
            .float_format(value.ty)
            .ok_or(Error::Invariant("stored floating constant format"))?;
        Ok(Some(Value::Floating(
            kind,
            value
                .value
                .bits(format)
                .ok_or(Error::Invariant("stored floating constant encoding"))?,
        )))
    }
    fn static_scalar_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<Value>, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let source = self.graph.expression_facts.get(id)?.ty;
        if self.graph.expression_facts.get(id)?.intrinsic
            == Some(super::floating_intrinsics::FloatingIntrinsic::Constant)
        {
            let value = self.graph.expression_facts.floating_value(id)?;
            return self.static_held_scalar(super::constant_addresses::TemporaryValue::Floating(
                self.graph.floating_constants.get(value),
            ));
        }
        if self.member_pointer_expression(id) {
            return match self
                .with_fresh_constant_query(|analyzer| analyzer.evaluate_member_pointer(id, scope))
            {
                Ok(value) => Ok(Some(Value::MemberPointer(value))),
                Err(Error::NonConstant { .. } | Error::Missing(_)) => Ok(None),
                Err(error) => Err(error),
            };
        }
        let soft = matches!(self.graph.types.get(source).kind, TypeKind::Scalar(Scalar::Basic(ty)) if crate::types::interchange::soft(ty));
        if self.integral_kind(source).is_some() {
            // A static-value request is an interpreter query, even if an
            // earlier context recorded this expression's scalar. Its frame
            // and substitution context are inputs to the answer.
            let value = match self
                .with_fresh_constant_query(|analyzer| analyzer.evaluate_constant(id, scope, false))
            {
                Ok(value) => Some(value),
                Err(Error::Missing(_) | Error::NonConstant { .. }) => None,
                Err(error) => return Err(error),
            };
            if let Some(value) = value.filter(|value| !value.indeterminate()) {
                return Ok(Some(self.static_integer_value(value)));
            }
        }
        if matches!(
            self.graph.types.get(source).kind,
            TypeKind::Pointer(_) | TypeKind::Array { .. } | TypeKind::Function { .. }
        ) {
            let value = match self.static_address_operand(id, scope) {
                Ok(address) => match address.root {
                    Root::Null => Some(Value::Integer(0)),
                    Root::Variable(entity)
                        if self
                            .graph
                            .entity(entity)
                            .variable
                            .is_some_and(|variable| !variable.automatic) =>
                    {
                        Some(Value::Address(entity, address.offset))
                    }
                    Root::Function(entity) => Some(Value::Address(entity, address.offset)),
                    Root::Published(id) => self.graph.static_temporary_value(id, address.offset),
                    Root::Literal(expr) => Some(Value::Literal(expr, address.offset)),
                    Root::TypeInformation(ty) => Some(Value::TypeInformation(ty, address.offset)),
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
                if ty.floating_rank() == 0 {
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
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if soft || self.float_format(source).is_some() => {
                use crate::types::float_value::Operation as O;
                let op = match op.token.as_punctuator() {
                    Some(P::Add) => O::Add,
                    Some(P::Subtract) => O::Subtract,
                    Some(P::Multiply) => O::Multiply,
                    Some(P::Divide) => O::Divide,
                    _ => return Ok(None),
                };
                let (Some(Value::Floating(ty, a)), Some(Value::Floating(_, b))) = (
                    self.static_scalar(left, source, scope)?,
                    self.static_scalar(right, source, scope)?,
                ) else {
                    return Ok(None);
                };
                if soft {
                    Ok(crate::types::interchange::arithmetic(op, ty, a, b)
                        .map_err(|_| Error::Limit("soft static arithmetic"))?
                        .map(|bits| Value::Floating(ty, bits)))
                } else {
                    let format =
                        Format::of(ty).ok_or(Error::Invariant("static arithmetic format"))?;
                    let a = BinaryFloat::from_bits(format, a)
                        .ok_or(Error::Invariant("static left floating value"))?;
                    let b = BinaryFloat::from_bits(format, b)
                        .ok_or(Error::Invariant("static right floating value"))?;
                    let operation_format = if format == Format::Single {
                        format
                    } else {
                        Format::Extended
                    };
                    Ok(a.operation(op, b, operation_format)
                        .and_then(|value| value.converted(format))
                        .and_then(|value| value.bits(format))
                        .map(|bits| Value::Floating(ty, bits)))
                }
            }
            ExprKind::Cast { operand, .. } => self.static_scalar(operand, source, scope),
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if braced.is_some() {
                    return Ok(None);
                }
                match self.tree.expr_lists.get(arguments) {
                    [operand] => self.static_scalar(*operand, source, scope),
                    _ => Ok(None),
                }
            }
            ExprKind::Call { arguments, .. }
                if self.graph.expression_facts.get(id)?.functional_call =>
            {
                match self.tree.expr_lists.get(arguments) {
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
    pub(super) fn static_integer_value(&self, value: super::constants::Constant) -> Value {
        if self
            .integral_kind(value.ty)
            .is_some_and(|kind| kind.bits > 64)
        {
            Value::WideInteger(value.wide())
        } else {
            Value::Integer(value.bits)
        }
    }
    pub(super) fn static_conversion(
        &self,
        value: Value,
        source: TypeId,
        target: TypeId,
    ) -> Result<Option<Value>, Error> {
        if !self.static_storage_value(value) {
            return Ok(None);
        }
        let soft_target = matches!(self.graph.types.get(target).kind, TypeKind::Scalar(Scalar::Basic(ty)) if crate::types::interchange::soft(ty));
        let soft_source =
            matches!(value, Value::Floating(ty, _) if crate::types::interchange::soft(ty));
        if soft_target || soft_source {
            return self.static_soft_conversion(value, source, target);
        }
        if self
            .integral_kind(target)
            .is_some_and(|kind| kind.bits > 64)
        {
            let kind = self
                .integral_kind(target)
                .ok_or(Error::Invariant("wide static target"))?;
            let bits = match value {
                Value::WideInteger(bits) => bits,
                Value::Integer(bits) => {
                    if self.integral_kind(source).is_some_and(|kind| kind.signed) {
                        i128::from(bits.cast_signed()).cast_unsigned()
                    } else {
                        u128::from(bits)
                    }
                }
                Value::Floating(ty, bits) => {
                    let format =
                        Format::of(ty).ok_or(Error::Invariant("wide static floating source"))?;
                    let floating = BinaryFloat::from_bits(format, bits)
                        .ok_or(Error::Invariant("wide static floating encoding"))?;
                    let Some(bits) = floating.integral_wide(128, kind.signed) else {
                        return Ok(None);
                    };
                    bits
                }
                _ => return Ok(None),
            };
            return Ok(Some(Value::WideInteger(bits)));
        }
        if let Value::WideInteger(bits) = value {
            if let Some(kind) = self.integral_kind(target) {
                let low = u64::try_from(bits & u128::from(u64::MAX))
                    .map_err(|_| Error::Invariant("wide static low bits"))?;
                return Ok(Some(Value::Integer(if kind.bits == 1 {
                    u64::from(bits != 0)
                } else {
                    kind.convert(low)
                })));
            }
            if let Some(format) = self.float_format(target) {
                let signed = self.integral_kind(source).is_some_and(|kind| kind.signed);
                let Some(value) = BinaryFloat::wide_integer(bits, signed, format) else {
                    return Ok(None);
                };
                let TypeKind::Scalar(Scalar::Basic(ty)) = self.graph.types.get(target).kind else {
                    return Err(Error::Invariant("wide static floating target"));
                };
                return Ok(value.bits(format).map(|bits| Value::Floating(ty, bits)));
            }
            return Ok(None);
        }
        if let Some(kind) = self.integral_kind(target) {
            let bits = match value {
                Value::Integer(bits) => bits,
                Value::WideInteger(_) => {
                    return Err(Error::Invariant("unconverted wide static integer"));
                }
                Value::MemberPointer(value) if kind.bits == 1 => u64::from(value.member.is_some()),
                Value::MemberPointer(_) => return Ok(None),
                Value::Address(_, _)
                | Value::Literal(_, _)
                | Value::Temporary(_, _)
                | Value::TypeInformation(_, _)
                    if kind.bits == 1 =>
                {
                    1
                }
                Value::Address(_, _)
                | Value::Literal(_, _)
                | Value::Temporary(_, _)
                | Value::TypeInformation(_, _)
                    if kind.bits >= 64 =>
                {
                    return Ok(Some(value));
                }
                Value::Address(_, _)
                | Value::Literal(_, _)
                | Value::Temporary(_, _)
                | Value::TypeInformation(_, _) => {
                    return Ok(None);
                }
                Value::Floating(ty, bits) => {
                    if kind.bits == 1 {
                        return Ok(Some(Value::Integer(u64::from(bits & !float_sign(ty) != 0))));
                    }
                    let format =
                        Format::of(ty).ok_or(Error::Invariant("static floating integer source"))?;
                    let value = BinaryFloat::from_bits(format, bits)
                        .ok_or(Error::Invariant("static floating integer encoding"))?;
                    // global_lowering.cpp converts through a host x87 i64/u64,
                    // then truncates to the target width. Model its indefinite
                    // result explicitly; constexpr range checks stay separate.
                    let high = BinaryFloat::integer(1 << 63, false);
                    if !kind.signed && value.compare(high).is_some_and(|order| !order.is_lt()) {
                        value.integral(64, false).unwrap_or(0)
                    } else {
                        value.integral(64, true).unwrap_or(1 << 63)
                    }
                }
            };
            return Ok(Some(Value::Integer(kind.convert(bits))));
        }
        if let TypeKind::Scalar(Scalar::Basic(target)) = self.graph.types.get(target).kind
            && target.floating_rank() != 0
        {
            if let Value::Floating(from, bits) = value
                && from != target
            {
                let (Some(from), Some(format)) = (Format::of(from), Format::of(target)) else {
                    return Ok(None);
                };
                let floating = BinaryFloat::from_bits(from, bits)
                    .ok_or(Error::Invariant("static floating encoding"))?;
                if floating.is_nan() || floating.is_infinite() {
                    let bits = floating
                        .converted(format)
                        .and_then(|value| value.bits(format))
                        .ok_or(Error::Invariant("static nonfinite floating conversion"))?;
                    return Ok(Some(Value::Floating(target, bits)));
                }
            }
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
                Value::WideInteger(_)
                | Value::MemberPointer(_)
                | Value::Address(_, _)
                | Value::Literal(_, _)
                | Value::Temporary(_, _)
                | Value::TypeInformation(_, _) => {
                    return Ok(None);
                }
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
    match ty.floating_representation() {
        F::Float16 => (10, 5, 15),
        F::BFloat16 => (7, 8, 127),
        F::Float128 => (112, 15, 16383),
        F::Float => (23, 8, 127),
        F::Double => (52, 11, 1023),
        _ => (63, 15, 16383),
    }
}
fn float_sign(ty: F) -> u128 {
    1 << match ty.floating_representation() {
        F::Float16 | F::BFloat16 => 15,
        F::Float128 => 127,
        F::Float => 31,
        F::Double => 63,
        _ => 79,
    }
}
fn float_infinity(ty: F) -> u128 {
    let (fraction, width, _) = float_layout(ty);
    if ty.floating_representation() == F::LongDouble {
        ((1 << width) - 1) << 64 | 1 << 63
    } else {
        ((1 << width) - 1) << fraction
    }
}
fn float_parts(ty: F, bits: u128) -> Option<(u128, i64, bool)> {
    let (fraction, width, exponent_bias) = float_layout(ty);
    let shift = if ty.floating_representation() == F::LongDouble {
        64
    } else {
        fraction
    };
    let exponent = i64::try_from(bits >> shift & ((1 << width) - 1)).ok()?;
    if exponent == (1 << width) - 1 {
        return None;
    }
    let significand = if ty.floating_representation() == F::LongDouble {
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
        let source = SourceBuffer::new(b"extern const int y; int x=y; const int y=7; double d=-0.0; int target; int *p=&target; int callee(int); int dynamic=callee(3); int *pointers[4]={&target,nullptr,0}; float values[3]={1.5f}; const int &reference=y; const int &temporary=7; const int &same=temporary; extern const int &later; const int *const &indirect=&later; const int &later=9;".to_vec()).unwrap();
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
            let reference = graph.entity(entity(b"reference"));
            let temporary_id = entity(b"temporary");
            let temporary = graph.entity(temporary_id);
            let same = graph.entity(entity(b"same"));
            if lowering {
                assert!(
                    matches!(reference.static_initializer, Some(Value::Address(root, 0)) if root == entity(b"y"))
                );
                assert!(reference.reference_temporary.is_none());
                assert!(matches!(
                    temporary.static_initializer,
                    Some(Value::Integer(7))
                ));
                let object = temporary.reference_temporary.unwrap().binding;
                assert_eq!(graph.lower_cv(object), 1);
                assert_eq!(
                    graph.lower_cv(temporary.reference_temporary.unwrap().value),
                    0
                );
                let Some(Value::Temporary(indirect, 0)) =
                    graph.entity(entity(b"indirect")).static_initializer
                else {
                    panic!("evaluated forward reference backing");
                };
                assert_eq!(
                    Some(indirect),
                    graph.lower_extended_temporary(entity(b"indirect"))
                );
                let (_, image) = graph.lower_constant_temporary(indirect).unwrap();
                assert!(
                    matches!(image, [super::super::static_arrays::Item::Scalar(_, Value::Temporary(id, 0))]
                    if Some(*id) == graph.lower_extended_temporary(entity(b"later")))
                );
                let Some(Value::Temporary(id, 0)) = same.static_initializer else {
                    panic!("shared published temporary");
                };
                assert_eq!(Some(id), graph.lower_extended_temporary(temporary_id));
                assert!(same.reference_temporary.is_none());
            } else {
                assert!(reference.static_initializer.is_none());
                assert!(temporary.static_initializer.is_none());
                assert!(temporary.reference_temporary.is_none());
                assert!(same.static_initializer.is_none());
            }
            let pointers = graph.lower_array_image(entity(b"pointers"));
            let values = graph.lower_array_image(entity(b"values"));
            if lowering {
                use super::super::static_arrays::Item;
                assert!(
                    matches!(pointers, Some([Item::Scalar(_, Value::Address(root, 0)), Item::Zero(24)]) if *root == entity(b"target"))
                );
                assert!(
                    matches!(values, Some([Item::Scalar(_, Value::Floating(F::Float, bits)), Item::Scalar(_, Value::Floating(F::Float, 0)), Item::Scalar(_, Value::Floating(F::Float, 0))]) if *bits == u128::from(1.5_f32.to_bits()))
                );
            } else {
                assert!(pointers.is_none());
                assert!(values.is_none());
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
