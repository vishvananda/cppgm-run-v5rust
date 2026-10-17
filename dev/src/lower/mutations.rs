// Translated from lower/expression_lowering.cpp's scalar updates. See NOTICE.
//! Compute each scalar place once and preserve load, RHS conversion, update, store order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    lowir::{
        literals::{Constant, FloatKind},
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Type},
    },
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn place_memory(
        &self,
        place: Value,
        source: crate::sema::lowering::TypeId,
    ) -> Result<Facts, Error> {
        if self.untyped_places.contains(&place) {
            Ok(Facts::memory(self.graph.lower_cv(source) & 2 != 0, false))
        } else {
            types::memory(self.graph, source)
        }
    }
    pub(super) fn scalar_place(&mut self, id: ast::ExprId) -> Result<Value, Error> {
        if self.graph.lower_builtin_operand(id)
            && let Some((call, _)) = self.graph.lower_function_conversion(id)?
        {
            return Ok(self
                .call_conversion(
                    id,
                    crate::sema::lowering::MemberCall {
                        place: false,
                        ..call
                    },
                )?
                .value);
        }
        self.original_scalar_place(id)
    }
    pub(super) fn original_scalar_place(&mut self, id: ast::ExprId) -> Result<Value, Error> {
        let facts = self.graph.lower_expression(id)?;
        if !facts.is_place
            && matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Class(_)
            )
        {
            return self.class_temporary(
                id,
                super::materialization::TemporaryUse::Designator,
                false,
            );
        }
        self.designated_place(id)
    }
    fn designated_place(&mut self, id: ast::ExprId) -> Result<Value, Error> {
        self.check_stack()?;
        if self.graph.lower_literal_call(id)?.is_some() {
            return Ok(self.literal_call(id)?.value);
        }
        if self.graph.lower_operator_call(id)? {
            return Ok(self.operator_call(id)?.value);
        }
        if self.graph.lower_expression(id)?.is_bit_field() {
            return Err(
                self.field_diagnostic(id, "a bit-field glvalue outside the supported forms")
            );
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::Paren(inner) => self.scalar_place(inner),
            ExprKind::Literal(literal) => {
                let crate::sema::lowering::TypeView::Array { element, .. } =
                    self.graph.lower_type(self.graph.lower_expression(id)?.ty)?
                else {
                    return Err(Error::Semantic(crate::sema::Error::Diagnostic {
                        location: self.tree.expressions.get(id).location,
                        message: "an invalid string literal",
                    }));
                };
                super::data::string(
                    self.builder.program(),
                    self.symbols,
                    self.graph,
                    literal,
                    element,
                )
                .map(Value::Symbol)
            }
            ExprKind::Subscript { base, index } => self.subscript(id, base, index),
            ExprKind::Member { base, op, .. } => {
                if let Some(function) = self.graph.lower_designated_function(id)? {
                    self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
                    self.symbols.demand(function);
                    return self
                        .symbols
                        .function_symbol(self.builder.program(), function)
                        .map(Value::Symbol);
                }
                let (entity, offset) = self.graph.lower_member(id)?;
                let source = self.graph.lower_expression(id)?.ty;
                let place = if let Some(offset) = offset {
                    let root = if op.token.as_punctuator() == Some(P::Arrow) {
                        self.arrow_operand(base)?.value
                    } else {
                        self.address(base)?
                    };
                    let root = self.member_base_projection(id, root, source)?;
                    let offset = self.builder.integer(u128::from(offset))?;
                    let place = self
                        .emit(
                            I::Index(
                                Type::I8,
                                Some(crate::lowir::vocabulary::Projection::Field),
                                root,
                                offset,
                            ),
                            source,
                        )?
                        .value;
                    if self.graph.lower_union_member(id)? || self.untyped_places.contains(&root) {
                        self.untyped_places.insert(place);
                    }
                    place
                } else {
                    self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
                    self.global_place(entity)?
                };
                self.reference_holder(place, self.graph.lower_entity_type(entity)?)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => self.conditional_place(id, condition, then_expr, else_expr),
            ExprKind::Id(name) => {
                if let Some(place) = self.injected_member_place(id)? {
                    return Ok(place);
                }
                if let Some(function) = self.graph.lower_designated_function(id)? {
                    self.symbols.demand(function);
                    return self
                        .symbols
                        .function_symbol(self.builder.program(), function)
                        .map(Value::Symbol);
                }
                if matches!(
                    self.graph.lower_object_source(id)?,
                    Some(crate::sema::lowering::ObjectSource::Implicit)
                ) {
                    let (entity, offset) = self.graph.lower_member(id)?;
                    let source = self.graph.lower_expression(id)?.ty;
                    let root = self.object_pointer(source)?.value;
                    let root = self.member_base_projection(id, root, source)?;
                    let offset =
                        self.builder
                            .integer(u128::from(offset.ok_or(Error::Semantic(
                                crate::sema::Error::Invariant(
                                    "implicit data member without offset",
                                ),
                            ))?))?;
                    let place = self
                        .emit(
                            I::Index(
                                Type::I8,
                                Some(crate::lowir::vocabulary::Projection::Field),
                                root,
                                offset,
                            ),
                            source,
                        )?
                        .value;
                    if self.graph.lower_union_member(id)? {
                        self.untyped_places.insert(place);
                    }
                    return self.reference_holder(place, self.graph.lower_entity_type(entity)?);
                }
                let entity = self.graph.lower_name(name)?;
                if let Some(&place) = self.locals.get(&entity) {
                    self.reference_holder(place, self.graph.lower_entity_type(entity)?)
                } else if !self.graph.lower_is_variable(entity) {
                    self.symbols.demand(entity);
                    self.symbols
                        .function_symbol(self.builder.program(), entity)
                        .map(Value::Symbol)
                } else {
                    if let Some(value) = self.graph.lower_evaluated_reference(entity)? {
                        return self
                            .evaluated_reference_place(value, self.graph.lower_expression(id)?.ty);
                    }
                    let place = self.global_place(entity)?;
                    self.reference_holder(place, self.graph.lower_entity_type(entity)?)
                }
            }
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::Multiply) => {
                Ok(self.operand(operand, Context::Operand)?.value)
            }
            ExprKind::Unary { op, operand }
                if matches!(op.token.as_punctuator(), Some(P::Increment | P::Decrement)) =>
            {
                Ok(self
                    .increment(
                        operand,
                        op.token.as_punctuator() == Some(P::Decrement),
                        true,
                    )?
                    .0)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: true,
            } => Ok(self
                .assignment_value(
                    id,
                    op.token
                        .as_punctuator()
                        .ok_or(Error::Missing("assignment operator"))?,
                    left,
                    right,
                )?
                .0),
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard(left)?;
                self.scalar_place(right)
            }
            ExprKind::Cast { operand, .. } => self.reference_cast_place(id, operand),
            ExprKind::Braced { .. } => Err(Error::Semantic(crate::sema::Error::Diagnostic {
                location: self
                    .tree
                    .expressions
                    .get(self.without_parentheses(id))
                    .location,
                message: "a glvalue outside the procedural lowering slice",
            })),
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if matches!(
                    self.graph.lower_type(self.graph.lower_expression(id)?.ty)?,
                    crate::sema::lowering::TypeView::Array { .. }
                ) {
                    return self.array_temporary(id, "arraytmp");
                }
                let operand = braced
                    .or_else(|| self.tree.expr_lists.get(arguments).first().copied())
                    .ok_or(Error::Missing("empty functional place"))?;
                self.scalar_place(operand)
            }
            ExprKind::Call { arguments, .. } if self.graph.lower_functional_call(id)? => {
                let operand = self
                    .tree
                    .expr_lists
                    .get(arguments)
                    .first()
                    .copied()
                    .ok_or(Error::Missing("empty functional place"))?;
                self.scalar_place(operand)
            }
            ExprKind::Call { callee, arguments } => Ok(self.call(id, callee, arguments)?.value),
            _ => Err(Error::Missing("reference and aggregate place lowering")),
        }
    }
    fn injected_member_place(&mut self, id: ast::ExprId) -> Result<Option<Value>, Error> {
        let Some((object, offset)) = self.graph.lower_injected_member(id)? else {
            return Ok(None);
        };
        let source = self.graph.lower_expression(id)?.ty;
        let place = if let Some(&local) = self.locals.get(&object) {
            local
        } else if self.graph.lower_program_storage(object) {
            self.global_place(object)?
        } else if self.graph.lower_explicit_static(object) {
            self.symbols.existing_local_storage(object).ok_or_else(|| {
                self.field_diagnostic(
                    id,
                    "a static local variable of a function not lowered first",
                )
            })?
        } else {
            return Err(
                self.field_diagnostic(id, "a variable outside the procedural lowering slice")
            );
        };
        let root = if matches!(place, Value::Slot(_) | Value::Symbol(_)) {
            self.emit(I::Addr(place), source)?.value
        } else {
            place
        };
        let offset = self.builder.integer(u128::from(offset))?;
        let place = self
            .emit(
                I::Index(
                    Type::I8,
                    Some(crate::lowir::vocabulary::Projection::Field),
                    root,
                    offset,
                ),
                source,
            )?
            .value;
        self.untyped_places.insert(place);
        self.reference_holder(place, source).map(Some)
    }
    pub(super) fn read_place(
        &mut self,
        place: Value,
        source: crate::sema::lowering::TypeId,
    ) -> Result<RValue, Error> {
        self.emit(
            I::Load(
                self.place_memory(place, source)?,
                types::storage(self.graph, source)?,
                place,
            ),
            source,
        )
    }
    pub(super) fn increment(
        &mut self,
        operand: ast::ExprId,
        decrement: bool,
        prefix: bool,
    ) -> Result<(Value, RValue), Error> {
        let source = self.graph.lower_builtin_operand_type(operand)?;
        let place = self.scalar_place(operand)?;
        let old = self.read_place(place, source)?;
        let result = if old.ty == Type::Ptr {
            self.pointer_step(old, decrement)?
        } else if types::boolean(self.graph, source)? {
            RValue {
                value: self.builder.integer(1)?,
                ty: old.ty,
                source,
                literal: false,
                bits: Some(1),
            }
        } else {
            let one = if let Some(kind) = FloatKind::from_type(old.ty) {
                let bits = match kind {
                    FloatKind::F32 => u128::from(1.0_f32.to_bits()),
                    FloatKind::F64 => u128::from(1.0_f64.to_bits()),
                    FloatKind::F80 => (0x3fff_u128 << 64) | (1_u128 << 63),
                };
                self.builder.constant(Constant::Float(kind, bits))?
            } else {
                self.builder.integer(1)?
            };
            let operation = if decrement { Binary::Sub } else { Binary::Add };
            let nowrap =
                types::signed(self.graph, source)? && matches!(old.ty, Type::I32 | Type::I64);
            self.emit(
                I::Binary(
                    operation,
                    Facts::arithmetic(nowrap, false),
                    old.ty,
                    old.value,
                    one,
                ),
                source,
            )?
        };
        self.effect(I::Store(
            self.place_memory(place, source)?,
            old.ty,
            result.value,
            place,
        ))?;
        let value = if prefix { result } else { old };
        Ok((place, value))
    }
    pub(super) fn assignment_value(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: ast::ExprId,
        right: ast::ExprId,
    ) -> Result<(Value, RValue), Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let ty = types::storage(self.graph, source)?;
        if ty == Type::Ptr && matches!(op, P::AddAssign | P::SubtractAssign) {
            return self.pointer_compound(left, right, op == P::SubtractAssign, source);
        }
        if op == P::AddAssign && types::boolean(self.graph, source)? {
            let (operation, _) = self.graph.lower_arithmetic_types(id)?;
            if self.pointer_element(operation)?.is_some() {
                return self.bool_pointer_compound(left, right, source, operation);
            }
        }
        let (place, value) = if op == P::Assign {
            let context = if !matches!(
                self.tree.expressions.get(right).kind,
                ExprKind::Braced { .. }
            ) && (self
                .graph
                .lower_union_assignment(self.without_parentheses(left))?
                || self
                    .graph
                    .lower_specialization_assignment(self.without_parentheses(left))?)
            {
                Context::Initialization
            } else {
                Context::Assignment
            };
            let value =
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(right).kind {
                    if let Some(&element) = self.tree.expr_lists.get(elements).first() {
                        self.operand(element, Context::Assignment)?
                    } else {
                        self.zero(source)?
                    }
                } else {
                    self.operand(right, context)?
                };
            let value = self.fit(value, source)?;
            let place = self.scalar_place(left)?;
            (place, value)
        } else {
            let place = self.scalar_place(left)?;
            let old = self.read_place(place, source)?;
            let (operation, _) = self.graph.lower_arithmetic_types(id)?;
            let mut rhs = self.operand(right, Context::CompoundAssignment)?;
            if matches!(op, P::ShiftLeftAssign | P::ShiftRightAssign) {
                rhs = self.convert(rhs, operation, Context::CompoundAssignment)?;
            }
            let lhs = self.convert(old, operation, Context::CompoundAssignment)?;
            let signed = types::signed(self.graph, operation)?;
            let code = match op {
                P::AddAssign => Binary::Add,
                P::SubtractAssign => Binary::Sub,
                P::MultiplyAssign => Binary::Mul,
                P::DivideAssign if signed || lhs.ty.float_rank() != 0 => Binary::Div,
                P::DivideAssign => Binary::UDiv,
                P::RemainderAssign if signed => Binary::Mod,
                P::RemainderAssign => Binary::UMod,
                P::AndAssign => Binary::And,
                P::OrAssign => Binary::Or,
                P::XorAssign => Binary::Xor,
                P::ShiftLeftAssign => Binary::Shl,
                P::ShiftRightAssign if signed => Binary::Shr,
                P::ShiftRightAssign => Binary::UShr,
                _ => return Err(Error::Missing("compound assignment operator")),
            };
            let nowrap = signed
                && matches!(lhs.ty, Type::I32 | Type::I64)
                && matches!(code, Binary::Add | Binary::Sub | Binary::Mul);
            let value = self.emit(
                I::Binary(
                    code,
                    Facts::arithmetic(nowrap, false),
                    lhs.ty,
                    lhs.value,
                    rhs.value,
                ),
                operation,
            )?;
            let value = self.convert(value, source, Context::CompoundAssignment)?;
            (place, self.fit(value, source)?)
        };
        self.effect(I::Store(
            self.place_memory(place, source)?,
            ty,
            value.value,
            place,
        ))?;
        Ok((place, RValue { source, ..value }))
    }
    pub(super) fn updated_value(&mut self, place: Value, value: RValue) -> Result<RValue, Error> {
        if self.graph.lower_cv(value.source) & 2 == 0 {
            Ok(value)
        } else {
            self.read_place(place, value.source)
        }
    }
}
