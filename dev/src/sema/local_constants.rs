// Translated from lower/statement_lowering.cpp's evaluated scalar initialization.
// See NOTICE.
//! Publish scalar initialization values during analysis, before lowering loses evaluation.
use super::{
    Error,
    analyzer::Analyzer,
    constants::ConstantState,
    graph::{EntityId, ScopeId},
    types::{Scalar, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    syntax::ast::{self, ExprKind},
    types::FundamentalType as F,
};
impl Analyzer<'_> {
    pub(super) fn queue_local_scalar(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let node = self.graph.entity(entity);
        let ty = node
            .ty
            .ok_or(Error::Invariant("local scalar without type"))?;
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Reference { .. }
                | TypeKind::Array { .. }
                | TypeKind::Class(_)
                | TypeKind::Complex(_)
        ) {
            return Ok(());
        }
        let Some(single) = self.single_initializer(initializer) else {
            return Ok(());
        };
        if self.initializer_names_member(single, 0) {
            return Ok(());
        }
        // Declaration-time evaluation already owns these completed values.
        // Publishing the local's image must not run that evaluation again.
        let completed = match node.constant {
            ConstantState::Value(value) => Some(self.static_integer_value(value)),
            ConstantState::Floating(value) => {
                let value = self.graph.floating_constants.get(value);
                let TypeKind::Scalar(Scalar::Basic(kind)) = self.graph.types.get(value.ty).kind
                else {
                    return Err(Error::Invariant("local floating constant type"));
                };
                let format = self
                    .float_format(value.ty)
                    .ok_or(Error::Invariant("local floating format"))?;
                Some(super::static_initialization::Value::Floating(
                    kind,
                    value
                        .value
                        .bits(format)
                        .ok_or(Error::Invariant("local floating encoding"))?,
                ))
            }
            ConstantState::MemberPointer(value) => {
                Some(super::static_initialization::Value::MemberPointer(value))
            }
            _ => None,
        };
        if let Some(value) = completed {
            self.graph.entity_mut(entity).static_initializer = Some(value);
            return Ok(());
        }
        if node.constexpr
            || self.type_cv(ty).0 & 3 == 1 && self.initializer_bfloat_arithmetic(single, 0)
        {
            self.static_initializers
                .push((entity, Some(initializer), scope));
        }
        Ok(())
    }
    fn initializer_names_member(&self, id: ast::ExprId, depth: u32) -> bool {
        let mut id = id;
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        if depth > 64 {
            return true;
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::Literal(_)
            | ExprKind::Id(_)
            | ExprKind::Sizeof { .. }
            | ExprKind::SizeofPack(_)
            | ExprKind::TypeTrait { .. }
            | ExprKind::TypePredicate { .. }
            | ExprKind::TypeTransform { .. } => false,
            ExprKind::Keyword(keyword) => keyword == Keyword::This,
            ExprKind::Binary { left, right, .. } => {
                self.initializer_names_member(left, depth + 1)
                    || self.initializer_names_member(right, depth + 1)
            }
            ExprKind::ComplexPart { operand, .. }
            | ExprKind::Unary { operand, .. }
            | ExprKind::Cast { operand, .. } => self.initializer_names_member(operand, depth + 1),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.initializer_names_member(condition, depth + 1)
                    || self.initializer_names_member(then_expr, depth + 1)
                    || self.initializer_names_member(else_expr, depth + 1)
            }
            ExprKind::Call { callee, arguments } => {
                self.initializer_names_member(callee, depth + 1)
                    || self
                        .tree
                        .expr_lists
                        .get(arguments)
                        .iter()
                        .any(|&id| self.initializer_names_member(id, depth + 1))
            }
            _ => true,
        }
    }
    fn initializer_bfloat_arithmetic(&self, id: ast::ExprId, depth: u32) -> bool {
        let mut id = id;
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        if depth > 64 {
            return false;
        }
        let Ok(facts) = self.graph.expression_facts.get(id) else {
            return false;
        };
        let bfloat = matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Basic(F::BFloat16))
        );
        match self.tree.expressions.get(id).kind {
            ExprKind::Binary {
                op, left, right, ..
            } => {
                bfloat
                    && matches!(
                        op.token.as_punctuator(),
                        Some(P::Add | P::Subtract | P::Multiply | P::Divide)
                    )
                    || self.initializer_bfloat_arithmetic(left, depth + 1)
                    || self.initializer_bfloat_arithmetic(right, depth + 1)
            }
            ExprKind::Unary { op, operand } => {
                bfloat && matches!(op.token.as_punctuator(), Some(P::Add | P::Subtract))
                    || self.initializer_bfloat_arithmetic(operand, depth + 1)
            }
            ExprKind::Cast { operand, .. } => {
                self.initializer_bfloat_arithmetic(operand, depth + 1)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.initializer_bfloat_arithmetic(condition, depth + 1)
                    || self.initializer_bfloat_arithmetic(then_expr, depth + 1)
                    || self.initializer_bfloat_arithmetic(else_expr, depth + 1)
            }
            _ => false,
        }
    }
}
