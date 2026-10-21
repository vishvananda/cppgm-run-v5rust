// Translated from sema/analyzer_bodies.cpp::statement_block/statement_expression.
// See NOTICE.
//! A statement expression shares its enclosing function's return and jump context.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    types::{Scalar, TypeKind},
};
use crate::syntax::ast::{self, StmtKind};

impl Analyzer<'_> {
    pub(super) fn statement_block(
        &mut self,
        id: ast::ExprId,
        body: ast::StmtId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let StmtKind::Compound(items) = self.tree.statements.get(body).kind else {
            return Err(Error::Invariant("statement expression without compound"));
        };
        if !self.semantic() {
            return self.compound_region(items, scope, false, true);
        }
        let context = self.body_context.as_mut().ok_or_else(|| {
            Error::at(
                self.tree.expressions.get(id).location,
                "a statement expression outside a function body",
            )
        })?;
        context.handled += 1;
        let result = self.compound_region(items, scope, false, true);
        if let Some(context) = &mut self.body_context {
            context.handled -= 1;
        }
        result
    }

    pub(super) fn statement_expression(
        &mut self,
        id: ast::ExprId,
        body: ast::StmtId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        self.statement_block(id, body, scope)?;
        let value = self.graph.statement_expression_value(self.tree, body)?;
        let Some(value) = value else {
            let ty = self.graph.types.scalar(Scalar::Void)?;
            return self.new_expression_facts(ty, ValueCategory::Prvalue);
        };
        let facts = self.graph.expression_facts.get(value)?;
        let ty = self.decayed_type(facts.ty)?;
        if !matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            self.convert_scalar(value, ty, false)?;
            self.record_class_copy(
                value,
                ty,
                scope,
                super::copy_initialization::CopyUse::Value { in_place: true },
                self.tree.expressions.get(id).location,
            )?;
        }
        self.new_expression_facts(ty, ValueCategory::Prvalue)
    }
}

impl super::graph::Graph {
    pub(crate) fn statement_expression_value(
        &self,
        tree: &ast::Tree,
        body: ast::StmtId,
    ) -> Result<Option<ast::ExprId>, Error> {
        let StmtKind::Compound(items) = tree.statements.get(body).kind else {
            return Err(Error::Invariant("statement expression without compound"));
        };
        let Some(&last) = tree.stmt_lists.get(items).last() else {
            return Ok(None);
        };
        let mut last = last;
        loop {
            match tree.statements.get(last).kind {
                StmtKind::Ambiguous { .. } => {
                    last = self
                        .statement_readings
                        .get(last.index())
                        .copied()
                        .flatten()
                        .ok_or(Error::Invariant("ambiguous statement without reading"))?;
                }
                StmtKind::Label { body, .. } => last = body,
                StmtKind::Expression(value) => return Ok(value),
                _ => return Ok(None),
            }
        }
    }
}
