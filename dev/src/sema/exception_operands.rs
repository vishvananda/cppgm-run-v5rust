// Translated from analyzer_exceptions.cpp::noexcept_value and its expression/conversion owners. See NOTICE.
//! Type the ordinary specification's supported operands before evaluating constants.
//! Dense occurrence facts retain lookup and call decisions, including skipped operands.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
    types::FundamentalType as F,
};

#[derive(Clone, Copy)]
pub(super) enum OperandStep {
    Visit,
    Finish,
    Boolean,
}

impl Analyzer<'_> {
    pub(super) fn exception_operand_type(
        &mut self,
        root: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        self.check_stack()?;
        if self.semantic() {
            if let Some(ty) = self.graph.expression_type(root) {
                return Ok(ty);
            }
            // A specification records normal expression/conversion facts but
            // never demands output or storage for the functions it mentions.
            let saved = self.unevaluated;
            self.unevaluated = 1;
            let result = self.typed_expression(root, scope).map(|facts| facts.ty);
            self.unevaluated = saved;
            return result;
        }
        let base = self.exception_work.len();
        self.exception_work.push((root, OperandStep::Visit));
        let result = (|| {
            while self.exception_work.len() > base {
                let (id, step) = self
                    .exception_work
                    .pop()
                    .ok_or(Error::Invariant("missing exception operand frame"))?;
                if matches!(step, OperandStep::Boolean) {
                    self.exception_bool(
                        self.exception_type(id)?,
                        self.tree.expressions.get(id).location,
                    )?;
                    continue;
                }
                if self.graph.expression_type(id).is_some() {
                    continue;
                }
                if matches!(step, OperandStep::Visit) {
                    self.exception_work.push((id, OperandStep::Finish));
                    self.queue_exception_children(self.tree.expressions.get(id).kind)?;
                    continue;
                }
                let ty = self.finish_exception_operand(id, scope)?;
                super::record(&mut self.graph.expression_types, id.index(), ty);
            }
            self.exception_type(root)
        })();
        self.exception_work.truncate(base);
        result
    }
    fn queue_exception_children(&mut self, kind: ExprKind) -> Result<(), Error> {
        use OperandStep::{Boolean, Visit};
        match kind {
            ExprKind::Paren(inner)
            | ExprKind::Unary { operand: inner, .. }
            | ExprKind::Cast { operand: inner, .. } => {
                self.exception_work.push((inner, Visit));
            }
            ExprKind::Binary { left, right, .. } => {
                self.exception_work.extend([(right, Visit), (left, Visit)]);
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.exception_work.extend([
                    (else_expr, Visit),
                    (then_expr, Visit),
                    (condition, Boolean),
                    (condition, Visit),
                ]);
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                let args = if let Some(braced) = braced {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
                    else {
                        return Err(Error::Invariant("nonbraced cast list"));
                    };
                    elements
                } else {
                    arguments
                };
                self.exception_work.extend(
                    self.tree
                        .expr_lists
                        .get(args)
                        .iter()
                        .rev()
                        .map(|&id| (id, Visit)),
                );
            }
            ExprKind::Subscript { index, .. } => {
                self.exception_work.push((index, Visit));
            }
            _ => (),
        }
        Ok(())
    }
    fn exception_type(&self, id: ast::ExprId) -> Result<TypeId, Error> {
        self.graph
            .expression_type(id)
            .ok_or(Error::Invariant("untyped exception operand"))
    }
    pub(super) fn exception_bool(&self, ty: TypeId, location: SourceOffset) -> Result<(), Error> {
        if self.scoped_enum(ty)
            || matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        {
            return Err(Error::at(location, "invalid implicit conversion"));
        }
        if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
            return Err(Error::Missing("class contextual boolean conversions"));
        }
        Ok(())
    }
    fn finish_exception_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let node = *self.tree.expressions.get(id);
        let location = node.location;
        match node.kind {
            ExprKind::Literal(literal) => match literal.kind {
                PostTokenKind::Literal { ty, .. } | PostTokenKind::Character { ty, .. } => {
                    self.graph.types.scalar(Scalar::Basic(ty))
                }
                PostTokenKind::String { ty, elements, .. } => {
                    let ty = self.graph.types.scalar(Scalar::Basic(ty))?;
                    let element = self.graph.types.qualify(ty, Cv::CONST)?;
                    self.graph.types.intern(
                        TypeKind::Array {
                            element,
                            bound: u64::from(elements),
                            operand: None,
                        },
                        Cv::default(),
                    )
                }
                _ => Err(Error::Missing("extended exception operand literals")),
            },
            ExprKind::Keyword(Keyword::True | Keyword::False) => {
                self.graph.types.scalar(Scalar::Bool)
            }
            ExprKind::Keyword(Keyword::Nullptr) => self.graph.types.scalar(Scalar::Nullptr),
            ExprKind::Id(name) => {
                let found = self.lookup(name, scope, Filter::Ordinary)?;
                if matches!(
                    self.graph.binding(found).kind,
                    BindingKind::Type
                        | BindingKind::Alias
                        | BindingKind::Namespace
                        | BindingKind::NamespaceAlias
                ) {
                    return Err(Error::at(location, "name does not denote a value"));
                }
                self.binding_expression_type(found)
            }
            ExprKind::Paren(inner) => self.exception_type(inner),
            ExprKind::Call { .. } => self.query_call_type(id, scope),
            ExprKind::Sizeof { ty, operand, .. } => {
                self.exception_query(id, ty, operand, scope, false, location)
            }
            ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ty,
                operand,
            } => self.exception_query(id, ty, operand, scope, true, location),
            ExprKind::TypeTrait {
                keyword: Keyword::Noexcept,
                operand: Some(operand),
                ..
            } => Ok(self.noexcept_expression(id, operand, scope)?.ty),
            ExprKind::Unary { op, operand } => {
                let ty = self.exception_type(operand)?;
                if !matches!(
                    op.token.as_punctuator(),
                    Some(P::Add | P::Subtract | P::Complement | P::LogicalNot)
                ) {
                    return Err(Error::Missing("extended unary exception operand typing"));
                }
                if op.token.as_punctuator() == Some(P::LogicalNot) {
                    self.exception_bool(ty, self.tree.expressions.get(operand).location)?;
                    self.graph.types.scalar(Scalar::Bool)
                } else if self.integral_kind(ty).is_some() && !self.scoped_enum(ty) {
                    self.promoted_type(ty)
                } else if matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
                ) && matches!(op.token.as_punctuator(), Some(P::Add | P::Subtract))
                {
                    Ok(ty)
                } else if self.scoped_enum(ty)
                    || matches!(
                        self.graph.types.get(ty).kind,
                        TypeKind::Scalar(Scalar::Void)
                    )
                {
                    Err(Error::at(
                        location,
                        if op.token.as_punctuator() == Some(P::Complement) {
                            "~ requires an integral operand"
                        } else {
                            "unary + or - requires an arithmetic operand"
                        },
                    ))
                } else {
                    Err(Error::Missing("extended unary exception operand typing"))
                }
            }
            ExprKind::Binary {
                op, left, right, ..
            } => {
                let a = self.exception_type(left)?;
                let b = self.exception_type(right)?;
                match op.token.as_punctuator() {
                    Some(P::LogicalAnd | P::LogicalOr) => {
                        self.exception_bool(a, self.tree.expressions.get(left).location)?;
                        self.exception_bool(b, self.tree.expressions.get(right).location)?;
                        self.graph.types.scalar(Scalar::Bool)
                    }
                    Some(P::Comma) => Ok(b),
                    Some(
                        P::Equal
                        | P::NotEqual
                        | P::Less
                        | P::Greater
                        | P::LessEqual
                        | P::GreaterEqual,
                    ) => {
                        if (self.scoped_enum(a) || self.scoped_enum(b)) && a != b {
                            return Err(Error::at(
                                location,
                                "comparison of a scoped enumeration with another type",
                            ));
                        }
                        if a == b && self.scoped_enum(a) {
                            self.graph.record_arithmetic_types(id, a, b);
                        } else {
                            let common = self.exception_arithmetic_type(a, b, false, location)?;
                            self.graph.record_arithmetic_types(id, common, common);
                        }
                        self.graph.types.scalar(Scalar::Bool)
                    }
                    Some(P::ShiftLeft | P::ShiftRight) => {
                        self.exception_arithmetic_type(a, b, true, location)?;
                        let left = self.promoted_type(a)?;
                        let right = self.promoted_type(b)?;
                        self.graph.record_arithmetic_types(id, left, right);
                        Ok(left)
                    }
                    Some(P::BitAnd | P::BitOr | P::Xor | P::Remainder) => {
                        let common = self.exception_arithmetic_type(a, b, true, location)?;
                        self.graph.record_arithmetic_types(id, common, common);
                        Ok(common)
                    }
                    Some(P::Add | P::Subtract | P::Multiply | P::Divide) => {
                        let common = self.exception_arithmetic_type(a, b, false, location)?;
                        self.graph.record_arithmetic_types(id, common, common);
                        Ok(common)
                    }
                    _ => Err(Error::Missing("extended binary exception operand typing")),
                }
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.exception_bool(
                    self.exception_type(condition)?,
                    self.tree.expressions.get(condition).location,
                )?;
                let a = self.exception_type(then_expr)?;
                let b = self.exception_type(else_expr)?;
                let void = |ty| {
                    matches!(
                        self.graph.types.get(ty).kind,
                        TypeKind::Scalar(Scalar::Void)
                    )
                };
                if void(a) != void(b) {
                    return Err(Error::at(location, "only one operand of ?: is void"));
                }
                if a == b {
                    Ok(a)
                } else if self.scoped_enum(a) || self.scoped_enum(b) {
                    Err(Error::at(location, "incompatible operands of ?:"))
                } else {
                    self.exception_arithmetic_type(a, b, false, location)
                }
            }
            ExprKind::Cast { ty, .. } => self.type_syntax(ty, scope),
            ExprKind::FunctionalCast { specifiers, .. } => {
                Ok(self.specifiers(specifiers, scope, false)?.ty)
            }
            ExprKind::Subscript { base, .. } => {
                let mut base = base;
                while let ExprKind::Paren(inner) = self.tree.expressions.get(base).kind {
                    base = inner;
                }
                if let ExprKind::Literal(literal) = self.tree.expressions.get(base).kind
                    && let PostTokenKind::String { ty, .. } = literal.kind
                {
                    self.graph.types.scalar(Scalar::Basic(ty))
                } else {
                    Err(Error::Missing(
                        "extended subscript exception operand typing",
                    ))
                }
            }
            _ => Err(Error::Missing("extended exception operand typing")),
        }
    }
    fn exception_arithmetic_type(
        &mut self,
        a: TypeId,
        b: TypeId,
        integral_only: bool,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        if [a, b].iter().any(|&ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        }) {
            return Err(Error::at(location, "operand of type void"));
        }
        if self.scoped_enum(a) || self.scoped_enum(b) {
            return Err(Error::at(location, "invalid operands to a binary operator"));
        }
        if self.integral_kind(a).is_some() && self.integral_kind(b).is_some() {
            return self.common_integral_type(a, b);
        }
        for scalar in [F::LongDouble, F::Double, F::Float] {
            if [a,b].iter().any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Scalar(Scalar::Basic(s)) if s == scalar)) {
                if integral_only { return Err(Error::at(location, "invalid operands to a binary operator")); }
                return self.graph.types.scalar(Scalar::Basic(scalar));
            }
        }
        Err(Error::Missing("extended binary exception operand typing"))
    }
    fn exception_query(
        &mut self,
        id: ast::ExprId,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
        alignment: bool,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let value = self.constant_query(ty, operand, scope, alignment, location)?;
        super::record(&mut self.graph.constant_facts, id.index(), value);
        Ok(value.ty)
    }
}
