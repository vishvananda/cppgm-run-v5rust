// Translated from lower/constant_lowering.cpp's construction and initializer-call demand. See NOTICE.
//! Evaluated initializers still request their selected construction helpers in
//! source order, before the semantic initializer-constructor chain is drained.
use super::{
    construction::{Entry, Variant},
    symbols::Symbols,
};
use crate::syntax::ast::{self, ExprKind, InitializerKind};

impl Symbols<'_, '_> {
    pub(super) fn demand_reference_initializer_calls(
        &mut self,
        graph: &crate::sema::Graph,
        tree: &ast::Tree,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), super::Error> {
        let expr = initializer.and_then(|initializer| match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id) => Some(id),
            InitializerKind::Parenthesized(arguments) => match tree.expr_lists.get(arguments) {
                [id] => Some(*id),
                _ => None,
            },
            _ => None,
        });
        let Some(expr) = expr else {
            return Ok(());
        };
        let mut work = vec![(expr, 0_u16)];
        while let Some((mut id, depth)) = work.pop() {
            while let ExprKind::Paren(inner) = tree.expressions.get(id).kind {
                id = inner;
            }
            if depth > 256 {
                continue;
            }
            let Some(demands) = graph.lower_initializer_demands(id)? else {
                continue;
            };
            if let Some(function) = demands.function {
                self.demand(function);
            }
            if let Some(constructor) = demands.constructor
                && !graph.lower_constructor_trivial(constructor)
            {
                self.demand_entry(Entry::Constructor(constructor, Variant::Complete));
            }
            match tree.expressions.get(id).kind {
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    work.extend([
                        (else_expr, depth + 1),
                        (then_expr, depth + 1),
                        (condition, depth + 1),
                    ]);
                }
                ExprKind::Binary { left, right, .. } => {
                    work.extend([(right, depth + 1), (left, depth + 1)]);
                }
                ExprKind::ComplexPart { operand, .. }
                | ExprKind::Unary { operand, .. }
                | ExprKind::Cast { operand, .. } => {
                    work.push((operand, depth + 1));
                }
                ExprKind::Member { base, .. } => work.push((base, depth + 1)),
                ExprKind::Call { callee, arguments } => {
                    work.extend(
                        tree.expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .map(|&id| (id, depth + 1)),
                    );
                    work.push((callee, depth + 1));
                }
                _ => (),
            }
        }
        Ok(())
    }
    pub(super) fn demand_constant_constructions(
        &mut self,
        graph: &crate::sema::Graph,
        tree: &ast::Tree,
        initializer: Option<ast::Initializer>,
    ) {
        let Some(initializer) = initializer else {
            return;
        };
        let mut work = Vec::new();
        match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id) => work.push((id, 0_u16)),
            InitializerKind::Parenthesized(arguments) => {
                work.extend(
                    tree.expr_lists
                        .get(arguments)
                        .iter()
                        .rev()
                        .map(|&id| (id, 0)),
                );
            }
            InitializerKind::Default | InitializerKind::Delete => (),
        }
        while let Some((mut id, depth)) = work.pop() {
            while let ExprKind::Paren(inner) = tree.expressions.get(id).kind {
                id = inner;
            }
            if depth > 256 {
                continue;
            }
            let Some(constructors) = graph.lower_constant_constructors(id) else {
                continue;
            };
            for constructor in constructors.into_iter().flatten() {
                if !graph.lower_constructor_trivial(constructor) {
                    self.demand_entry(Entry::Constructor(constructor, Variant::Complete));
                }
            }
            match tree.expressions.get(id).kind {
                ExprKind::Braced { elements, .. } => {
                    work.extend(
                        tree.expr_lists
                            .get(elements)
                            .iter()
                            .rev()
                            .map(|&id| (id, depth + 1)),
                    );
                }
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    if let Some(braced) = braced {
                        if let ExprKind::Braced { elements, .. } = tree.expressions.get(braced).kind
                        {
                            work.extend(
                                tree.expr_lists
                                    .get(elements)
                                    .iter()
                                    .rev()
                                    .map(|&id| (id, depth + 1)),
                            );
                        }
                    } else {
                        work.extend(
                            tree.expr_lists
                                .get(arguments)
                                .iter()
                                .rev()
                                .map(|&id| (id, depth + 1)),
                        );
                    }
                }
                ExprKind::Cast { operand, .. } => work.push((operand, depth + 1)),
                _ => (),
            }
        }
    }
}
