// Translated from sema/constant_evaluator.cpp's types-mode evaluator. See NOTICE.
//! Integral values keep type identity; optional evaluation never hides semantic errors.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, EntityId, ScopeId},
    integral::{self, IntegralKind},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind, InitializerKind},
};
#[derive(Clone, Copy, Debug)]
pub(super) struct Constant {
    pub bits: u64,
    pub ty: TypeId,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ConstantState {
    None,
    Evaluating,
    Value(Constant),
}
fn not_constant(location: SourceOffset, message: &'static str) -> Error {
    Error::NonConstant { location, message }
}
impl Analyzer<'_> {
    pub(super) fn integral_kind(&self, ty: TypeId) -> Option<IntegralKind> {
        self.graph.integral_kind(ty)
    }
    pub(super) fn scoped_enum(&self, ty: TypeId) -> bool {
        if let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
            self.graph
                .entity(entity)
                .enumeration
                .is_some_and(|info| info.scoped)
        } else {
            false
        }
    }
    pub(super) fn check_integral(
        &self,
        value: Constant,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.scoped_enum(value.ty) {
            Err(Error::at(
                location,
                "a scoped enumeration value does not convert implicitly to an integer",
            ))
        } else {
            Ok(())
        }
    }
    fn check_constant_bool(&self, value: Constant, location: SourceOffset) -> Result<(), Error> {
        if self.evaluating_noexcept {
            self.exception_bool(value.ty, location)
        } else {
            self.check_integral(value, location)
        }
    }
    fn scalar_constant(&mut self, bits: u64, scalar: Scalar) -> Result<Constant, Error> {
        Ok(Constant {
            bits,
            ty: self.graph.types.scalar(scalar)?,
        })
    }
    fn boolean(&mut self, value: bool) -> Result<Constant, Error> {
        self.scalar_constant(u64::from(value), Scalar::Bool)
    }
    pub(super) fn constant_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        required: bool,
    ) -> Result<Option<Constant>, Error> {
        match self.evaluate_constant(id, scope, false) {
            Ok(value) => Ok(Some(value)),
            Err(Error::NonConstant { location, message }) if required => {
                Err(Error::at(location, message))
            }
            Err(Error::NonConstant { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub(super) fn evaluate_constant(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        self.check_stack()?;
        if self.evaluating_noexcept
            && let Some(value) = self.graph.constant_fact(id)
        {
            return Ok(value);
        }
        let expr = *self.tree.expressions.get(id);
        let location = expr.location;
        let value = match expr.kind {
            ExprKind::Literal(literal) => {
                let (PostTokenKind::Literal { ty, data, .. }
                | PostTokenKind::Character {
                    ty,
                    data,
                    suffix_begin: None,
                    ..
                }) = literal.kind
                else {
                    return Err(not_constant(
                        location,
                        if matches!(literal.kind, PostTokenKind::String { .. }) {
                            "a string literal is not an integral constant"
                        } else {
                            "not an integral constant expression"
                        },
                    ));
                };
                let scalar = Scalar::Basic(ty);
                let Some(kind) = IntegralKind::of(scalar) else {
                    return Err(not_constant(
                        location,
                        "floating-point constant evaluation is not supported here",
                    ));
                };
                let mut bytes = [0u8; 8];
                let data = data.as_slice();
                let count = data.len().min(8);
                bytes[..count].copy_from_slice(&data[..count]);
                self.scalar_constant(kind.convert(u64::from_le_bytes(bytes)), scalar)?
            }
            ExprKind::Keyword(Keyword::True | Keyword::False) => {
                self.boolean(matches!(expr.kind, ExprKind::Keyword(Keyword::True)))?
            }
            ExprKind::Keyword(Keyword::Nullptr) if skipping => {
                self.scalar_constant(0, Scalar::Nullptr)?
            }
            ExprKind::Paren(inner) => self.evaluate_constant(inner, scope, skipping)?,
            ExprKind::Id(name) => {
                let found = if self.evaluating_noexcept {
                    self.graph
                        .resolved_name(name)
                        .ok_or(Error::Invariant("unresolved exception operand name"))?
                } else {
                    self.lookup(name, scope, Filter::Ordinary)?
                };
                let binding = self.graph.binding(found);
                if matches!(
                    binding.kind,
                    BindingKind::Type
                        | BindingKind::Alias
                        | BindingKind::Namespace
                        | BindingKind::NamespaceAlias
                ) {
                    return Err(Error::at(location, "name does not denote a value"));
                }
                if let ConstantState::Value(value) = self.graph.entity(binding.entity).constant {
                    value
                } else {
                    let ty = self.binding_expression_type(found)?;
                    if skipping && self.integral_kind(ty).is_some() {
                        let ty = self
                            .graph
                            .types
                            .intern(self.graph.types.get(ty).kind.clone(), Cv::default())?;
                        Constant { bits: 0, ty }
                    } else {
                        return Err(not_constant(
                            location,
                            match binding.kind {
                                BindingKind::Variable => {
                                    "variable is not usable in a constant expression"
                                }
                                BindingKind::Parameter => {
                                    "a parameter is not usable in a constant expression"
                                }
                                _ => "not an integral constant expression",
                            },
                        ));
                    }
                }
            }
            ExprKind::Binary { .. } => self.constant_chain(id, scope, skipping)?,
            ExprKind::Unary { op, operand } => {
                self.constant_unary(op.token.as_punctuator(), operand, scope, skipping, id)?
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let condition_value = self.evaluate_constant(condition, scope, skipping)?;
                self.check_constant_bool(
                    condition_value,
                    self.tree.expressions.get(condition).location,
                )?;
                let (selected, other) = if condition_value.bits != 0 {
                    (then_expr, else_expr)
                } else {
                    (else_expr, then_expr)
                };
                let chosen = self.evaluate_constant(selected, scope, skipping)?;
                let unchosen = self.evaluate_constant(other, scope, true)?;
                if chosen.ty == unchosen.ty {
                    chosen
                } else {
                    self.check_integral(chosen, self.tree.expressions.get(selected).location)?;
                    self.check_integral(unchosen, self.tree.expressions.get(other).location)?;
                    let ty = if self.evaluating_noexcept {
                        self.graph
                            .expression_type(id)
                            .ok_or(Error::Invariant("untyped exception conditional"))?
                    } else {
                        self.common_integral_type(chosen.ty, unchosen.ty)?
                    };
                    let kind = self
                        .integral_kind(ty)
                        .ok_or(Error::Invariant("common nonintegral type"))?;
                    Constant {
                        bits: kind.convert(chosen.bits),
                        ty,
                    }
                }
            }
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => {
                if keyword.is_some_and(|key| key != Keyword::StaticCast) {
                    return Err(not_constant(
                        location,
                        "cast is not allowed in a constant expression",
                    ));
                }
                let ty = self.type_syntax(ty, scope)?;
                self.constant_cast(ty, Some(operand), scope, skipping, location)?
            }
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                let ty = if self.evaluating_noexcept {
                    self.graph
                        .expression_type(id)
                        .ok_or(Error::Invariant("untyped exception cast"))?
                } else {
                    self.specifiers(specifiers, scope, false)?.ty
                };
                let args = if let Some(braced) = braced {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
                    else {
                        return Err(Error::Invariant("nonbraced cast list"));
                    };
                    elements
                } else {
                    arguments
                };
                let args = self.tree.expr_lists.get(args);
                if args.len() > 1 {
                    return Err(not_constant(
                        location,
                        "not an integral constant expression",
                    ));
                }
                self.constant_cast(ty, args.first().copied(), scope, skipping, location)?
            }
            ExprKind::Subscript { base, index } => {
                self.constant_string_element(base, index, scope, skipping, location)?
            }
            ExprKind::Sizeof { ty, operand, .. } => {
                self.constant_query(ty, operand, scope, false, location)?
            }
            ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ty,
                operand,
            } => self.constant_query(ty, operand, scope, true, location)?,
            ExprKind::Call { .. } if self.evaluating_noexcept => {
                let ty = self.query_call_type(id, scope)?;
                if skipping {
                    Constant { bits: 0, ty }
                } else {
                    if self.graph.entity(self.graph.call_entity(id)?).constexpr {
                        return Err(Error::Missing("constexpr function interpretation"));
                    }
                    if self.graph.binding(self.graph.call_binding(id)?).kind
                        == BindingKind::Parameter
                    {
                        return Err(not_constant(
                            location,
                            "a parameter is not usable in a constant expression",
                        ));
                    }
                    if self.graph.binding(self.graph.call_binding(id)?).kind
                        == BindingKind::Variable
                    {
                        return Err(not_constant(
                            location,
                            "a variable is not usable in a constant expression",
                        ));
                    }
                    return Err(not_constant(
                        location,
                        "a call of a function that is not constexpr",
                    ));
                }
            }
            _ => {
                return Err(not_constant(
                    location,
                    "not an integral constant expression",
                ));
            }
        };
        if !skipping {
            super::record(&mut self.graph.constant_facts, id.index(), value);
        }
        Ok(value)
    }
    fn constant_string_element(
        &mut self,
        mut base: ast::ExprId,
        index: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(base).kind {
            base = inner;
        }
        let ExprKind::Literal(literal) = self.tree.expressions.get(base).kind else {
            return Err(not_constant(
                location,
                "not an integral constant expression",
            ));
        };
        let PostTokenKind::String {
            ty,
            elements,
            suffix: None,
        } = literal.kind
        else {
            return Err(not_constant(
                location,
                "not an integral constant expression",
            ));
        };
        let index_value = self.evaluate_constant(index, scope, skipping)?;
        self.check_integral(index_value, self.tree.expressions.get(index).location)?;
        let index_kind = self
            .integral_kind(index_value.ty)
            .ok_or(Error::Invariant("string index kind"))?;
        if (index_kind.signed && integral::signed(index_value.bits) < 0)
            || index_value.bits >= u64::from(elements)
        {
            return Err(not_constant(
                location,
                "a string literal subscript outside its array",
            ));
        }
        let offset = usize::try_from(index_value.bits)
            .ok()
            .and_then(|index| index.checked_mul(ty.size()))
            .ok_or(Error::Limit("string subscript host size"))?;
        let end = offset
            .checked_add(ty.size())
            .ok_or(Error::Limit("string element host size"))?;
        let data = self
            .literal_bytes
            .get(literal.data)
            .get(offset..end)
            .ok_or(Error::Invariant("string element outside decoded literal"))?;
        let mut bytes = [0u8; 8];
        bytes[..data.len()].copy_from_slice(data);
        let scalar = Scalar::Basic(ty);
        let kind =
            IntegralKind::of(scalar).ok_or(Error::Invariant("nonintegral character type"))?;
        self.scalar_constant(kind.convert(u64::from_le_bytes(bytes)), scalar)
    }
    fn constant_chain(
        &mut self,
        root: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Constant, Error> {
        // Parser-built binary left spines are unbounded; they consume no native recursion.
        let base = self.constant_spine.len();
        let mut id = root;
        while let ExprKind::Binary { left, .. } = self.tree.expressions.get(id).kind {
            self.constant_spine.push(id);
            id = left;
        }
        let result = (|| {
            let mut value = self.evaluate_constant(id, scope, skipping)?;
            let end = self.constant_spine.len();
            for index in (base..end).rev() {
                let id = self.constant_spine[index];
                let expr = *self.tree.expressions.get(id);
                let ExprKind::Binary {
                    op, left, right, ..
                } = expr.kind
                else {
                    return Err(Error::Invariant("nonbinary constant spine"));
                };
                let Some(op) = op.token.as_punctuator() else {
                    return Err(not_constant(
                        expr.location,
                        "operator is not allowed in a constant expression",
                    ));
                };
                if matches!(op, P::LogicalAnd | P::LogicalOr) {
                    self.check_constant_bool(value, self.tree.expressions.get(left).location)?;
                    let truth = value.bits != 0;
                    let decided = if op == P::LogicalOr { truth } else { !truth };
                    if decided {
                        match self.evaluate_constant(right, scope, true) {
                            Ok(right_value) => self.check_constant_bool(
                                right_value,
                                self.tree.expressions.get(right).location,
                            )?,
                            Err(Error::NonConstant { .. }) => (),
                            Err(error) => return Err(error),
                        }
                        value = self.boolean(truth)?;
                    } else {
                        let right_value = self.evaluate_constant(right, scope, skipping)?;
                        self.check_constant_bool(
                            right_value,
                            self.tree.expressions.get(right).location,
                        )?;
                        value = self.boolean(right_value.bits != 0)?;
                    }
                } else {
                    let right_value = self.evaluate_constant(right, scope, skipping)?;
                    value = if op == P::Comma {
                        right_value
                    } else {
                        self.constant_arithmetic(op, value, right_value, skipping, id)?
                    };
                }
                if !skipping {
                    super::record(&mut self.graph.constant_facts, id.index(), value);
                }
            }
            Ok(value)
        })();
        self.constant_spine.truncate(base);
        result
    }
    pub(super) fn promoted_type(&mut self, ty: TypeId) -> Result<TypeId, Error> {
        let ty = if let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
            let info = self
                .graph
                .entity(entity)
                .enumeration
                .ok_or(Error::Invariant("enum without metadata"))?;
            if info.scoped {
                return Ok(ty);
            }
            info.underlying
                .ok_or(Error::Invariant("enum without underlying type"))?
        } else {
            ty
        };
        let TypeKind::Scalar(scalar) = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("promotion of nonintegral type"));
        };
        self.graph.types.scalar(integral::promoted(scalar))
    }
    pub(super) fn common_integral_type(&mut self, a: TypeId, b: TypeId) -> Result<TypeId, Error> {
        let a = self.promoted_type(a)?;
        let b = self.promoted_type(b)?;
        let (TypeKind::Scalar(a), TypeKind::Scalar(b)) =
            (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
        else {
            return Err(Error::Invariant("nonintegral common type"));
        };
        let scalar = integral::common(*a, *b).ok_or(Error::Invariant("nonintegral common kind"))?;
        self.graph.types.scalar(scalar)
    }
    fn constant_arithmetic(
        &mut self,
        op: P,
        left: Constant,
        right: Constant,
        skipping: bool,
        id: ast::ExprId,
    ) -> Result<Constant, Error> {
        let location = self.tree.expressions.get(id).location;
        let comparison = matches!(
            op,
            P::Equal | P::NotEqual | P::Less | P::Greater | P::LessEqual | P::GreaterEqual
        );
        if comparison && (self.scoped_enum(left.ty) || self.scoped_enum(right.ty)) {
            if left.ty != right.ty {
                return Err(Error::at(
                    location,
                    "comparison between a scoped enumeration and another type",
                ));
            }
            let kind = self
                .integral_kind(left.ty)
                .ok_or(Error::Invariant("enum without underlying type"))?;
            return self.boolean(
                integral::comparison(op, kind.signed, left.bits, right.bits)
                    .ok_or(Error::Invariant("noncomparison operator"))?,
            );
        }
        self.check_integral(left, location)?;
        self.check_integral(right, location)?;
        let (ty, right_ty) = if self.evaluating_noexcept {
            self.graph.arithmetic_types(id)?
        } else if matches!(op, P::ShiftLeft | P::ShiftRight) {
            (self.promoted_type(left.ty)?, self.promoted_type(right.ty)?)
        } else {
            let common = self.common_integral_type(left.ty, right.ty)?;
            (common, common)
        };
        if matches!(op, P::ShiftLeft | P::ShiftRight) {
            let kind = self
                .integral_kind(ty)
                .ok_or(Error::Invariant("nonintegral shift"))?;
            let count_kind = self
                .integral_kind(right_ty)
                .ok_or(Error::Invariant("nonintegral count"))?;
            let (bits, valid) = kind.shift(op, left.bits, right.bits, count_kind);
            if !valid && !skipping {
                return Err(not_constant(location, "a shift outside its defined range"));
            }
            return Ok(Constant { bits, ty });
        }
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("nonintegral operation"))?;
        if let Some(result) = integral::comparison(
            op,
            kind.signed,
            kind.convert(left.bits),
            kind.convert(right.bits),
        ) {
            return self.boolean(result);
        }
        let (bits, valid) = kind.operation(op, left.bits, right.bits).ok_or_else(|| {
            not_constant(location, "operator is not allowed in a constant expression")
        })?;
        if !valid && !skipping {
            return Err(not_constant(
                location,
                "an arithmetic operation with an undefined result",
            ));
        }
        Ok(Constant { bits, ty })
    }
    fn constant_unary(
        &mut self,
        op: Option<P>,
        operand: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
        id: ast::ExprId,
    ) -> Result<Constant, Error> {
        let location = self.tree.expressions.get(id).location;
        if op == Some(P::BitAnd) {
            return Err(not_constant(
                location,
                "operator is not allowed in a constant expression",
            ));
        }
        let value = self.evaluate_constant(operand, scope, skipping)?;
        if op == Some(P::LogicalNot) {
            self.check_constant_bool(value, self.tree.expressions.get(operand).location)?;
            return self.boolean(value.bits == 0);
        }
        if !matches!(op, Some(P::Add | P::Subtract | P::Complement)) {
            return Err(not_constant(
                location,
                "operator is not allowed in a constant expression",
            ));
        }
        self.check_integral(value, location)?;
        let ty = if self.evaluating_noexcept {
            self.graph
                .expression_type(id)
                .ok_or(Error::Invariant("untyped exception unary operand"))?
        } else {
            self.promoted_type(value.ty)?
        };
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("nonintegral unary"))?;
        let bits = kind.convert(value.bits);
        let bits = match op {
            Some(P::Subtract) => {
                if kind.signed && integral::signed(bits) == kind.minimum() && !skipping {
                    return Err(not_constant(
                        location,
                        "signed overflow in a constant expression",
                    ));
                }
                kind.convert(0u64.wrapping_sub(bits))
            }
            Some(P::Complement) => kind.convert(!bits),
            _ => bits,
        };
        Ok(Constant { bits, ty })
    }
    fn constant_cast(
        &mut self,
        ty: TypeId,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
        skipping: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let ty = self
            .graph
            .types
            .intern(self.graph.types.get(ty).kind.clone(), Cv::default())?;
        let kind = self
            .integral_kind(ty)
            .ok_or_else(|| not_constant(location, "cast to a type that is not integral"))?;
        let source = operand
            .map(|id| self.evaluate_constant(id, scope, skipping))
            .transpose()?
            .unwrap_or(Constant { bits: 0, ty });
        if self.integral_kind(source.ty).is_none() {
            return Err(not_constant(
                location,
                "not an integral constant expression",
            ));
        }
        Ok(Constant {
            bits: kind.convert(source.bits),
            ty,
        })
    }
    pub(super) fn single_initializer(&self, initializer: ast::Initializer) -> Option<ast::ExprId> {
        match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id) => {
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                    let elements = self.tree.expr_lists.get(elements);
                    if elements.len() == 1 {
                        elements.first().copied()
                    } else {
                        None
                    }
                } else {
                    Some(id)
                }
            }
            InitializerKind::Parenthesized(args) => {
                let args = self.tree.expr_lists.get(args);
                if args.len() == 1 {
                    args.first().copied()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub(super) fn record_constant_initializer(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("constant object without type"))?;
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        let object = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        let Some(kind) = self.integral_kind(object) else {
            return Ok(());
        };
        if self.graph.types.get(object).cv.0 != 1 {
            return Ok(());
        }
        let Some(expr) = self.single_initializer(initializer) else {
            return Ok(());
        };
        self.graph.entity_mut(entity).constant = ConstantState::Evaluating;
        let mut inner = expr;
        while let ExprKind::Paren(id) = self.tree.expressions.get(inner).kind {
            inner = id;
        }
        let value = if reference && let ExprKind::Id(name) = self.tree.expressions.get(inner).kind {
            let found = self.lookup(name, scope, Filter::Ordinary)?;
            let binding = self.graph.binding(found);
            if binding.kind == BindingKind::Variable
                && let ConstantState::Value(value) = self.graph.entity(binding.entity).constant
            {
                Some(value)
            } else {
                None
            }
        } else {
            self.constant_value(expr, scope, false)?
        };
        self.graph.entity_mut(entity).constant = ConstantState::None;
        if let Some(value) = value {
            let target = self
                .graph
                .types
                .intern(self.graph.types.get(object).kind.clone(), Cv::default())?;
            if (self.scoped_enum(target) || self.scoped_enum(value.ty)) && value.ty != target {
                return Err(Error::at(
                    self.tree.expressions.get(expr).location,
                    "a scoped enumeration value does not convert implicitly",
                ));
            }
            if matches!(self.graph.types.get(target).kind, TypeKind::Enum(_)) && value.ty != target
            {
                return Err(Error::at(
                    self.tree.expressions.get(expr).location,
                    "an integer does not convert implicitly to an enumeration",
                ));
            }
            self.graph.entity_mut(entity).constant = ConstantState::Value(Constant {
                bits: kind.convert(value.bits),
                ty: target,
            });
        }
        Ok(())
    }
    pub(super) fn array_bound(&mut self, expr: ast::ExprId, scope: ScopeId) -> Result<u64, Error> {
        let value = self.constant_value(expr, scope, self.template_depth == 0)?;
        let Some(value) = value else {
            self.expression(expr, scope)?;
            return Ok(0);
        };
        let location = self.tree.expressions.get(expr).location;
        let kind = self
            .integral_kind(value.ty)
            .ok_or(Error::Invariant("nonintegral array bound"))?;
        if self.template_depth != 0
            && (value.bits == 0 || kind.signed && integral::signed(value.bits) < 0)
        {
            return Ok(0);
        }
        let failure = if self.scoped_enum(value.ty) {
            Some("a scoped enumeration value is not an array bound")
        } else if kind.signed && integral::signed(value.bits) < 0 {
            Some("negative array bound")
        } else if value.bits == 0 {
            Some("array bound is zero")
        } else {
            None
        };
        if let Some(message) = failure {
            return Err(Error::at(location, message));
        }
        Ok(value.bits)
    }
}
