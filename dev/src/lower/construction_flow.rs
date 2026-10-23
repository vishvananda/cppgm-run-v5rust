// Translated from lower/cleanup_lowering.cpp::branched_regions. See NOTICE.
//! Construction closes regions at terminators only when its initializer can split them on one path.
use super::{Error, function::Function};
use crate::{
    lex::pp_lexer::Punctuator,
    sema::lowering::{CallTarget, TypeView},
    syntax::ast::{self, ExprKind},
};
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ConstructionFlow {
    #[default]
    Linear,
    Branched,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn initializer_branches(
        &mut self,
        initializer: Option<ast::Initializer>,
    ) -> Result<ConstructionFlow, Error> {
        let mut work = std::mem::take(&mut self.lifetime.branch_work);
        let result = self.scan_initializer_branches(initializer, &mut work);
        work.clear();
        self.lifetime.branch_work = work;
        result.map(|branched| {
            if branched {
                ConstructionFlow::Branched
            } else {
                ConstructionFlow::Linear
            }
        })
    }
    fn scan_initializer_branches(
        &self,
        initializer: Option<ast::Initializer>,
        work: &mut Vec<(ast::ExprId, bool)>,
    ) -> Result<bool, Error> {
        self.push_initializer_branches(initializer, false, work);
        while let Some((id, conditional)) = work.pop() {
            let id = self.without_parentheses(id);
            if !matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. })
                && self.graph.lower_expression(id)?.folded.is_some()
            {
                continue;
            }
            match self.tree.expressions.get(id).kind {
                ExprKind::Statement(_) if conditional => return Ok(true),
                ExprKind::Conditional { .. } => return Ok(true),
                ExprKind::Throw(operand) => work.extend(operand.map(|id| (id, conditional))),
                ExprKind::Binary {
                    op, left, right, ..
                } => {
                    let logical = matches!(
                        op.token.as_punctuator(),
                        Some(Punctuator::LogicalAnd | Punctuator::LogicalOr)
                    ) && !self.graph.lower_operator_call(id)?;
                    work.push((right, conditional || logical));
                    work.push((left, conditional));
                }
                ExprKind::Unary { operand, .. }
                | ExprKind::Postfix { operand, .. }
                | ExprKind::Cast { operand, .. } => work.push((operand, conditional)),
                ExprKind::Call { callee, arguments } => {
                    let callee = self.without_parentheses(callee);
                    let callee = if let ExprKind::Member { base, .. } =
                        self.tree.expressions.get(callee).kind
                    {
                        base
                    } else {
                        callee
                    };
                    // Reserved intrinsic callees are untyped source designations;
                    // the C++ walk visits them as leaves with no facts.
                    let intrinsic = self.graph.expression_is_intrinsic(id)?;
                    if !intrinsic {
                        work.push((callee, conditional));
                    }
                    let written = self.tree.expr_lists.get(arguments);
                    work.extend(written.iter().map(|&id| (id, conditional)));
                    if !intrinsic
                        && !self.graph.lower_destructor_evaluation_only(id)?
                        && let CallTarget::Direct(function) = self.graph.lower_call(id)?
                        && let TypeView::Function { parameters, .. } = self
                            .graph
                            .lower_type(self.graph.lower_function(function)?.ty)?
                    {
                        for index in written.len()..parameters.len() {
                            work.extend(
                                self.graph
                                    .lower_default(function, index)
                                    .map(|id| (id, conditional)),
                            );
                        }
                    }
                }
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .map(|&id| (id, conditional)),
                    );
                    work.extend(braced.map(|id| (id, conditional)));
                }
                ExprKind::Braced { elements, .. } => {
                    work.extend(
                        self.tree
                            .expr_lists
                            .get(elements)
                            .iter()
                            .map(|&id| (id, conditional)),
                    );
                }
                ExprKind::Subscript { base, index } => {
                    work.push((index, conditional));
                    work.push((base, conditional));
                }
                ExprKind::Member { base, .. } => work.push((base, conditional)),
                ExprKind::Delete { operand, .. } => {
                    let use_ = self.graph.lower_deletion(id)?;
                    if let TypeView::Class(class) = self.graph.lower_type(use_.target)?
                        && let Some(destructor) = self.graph.lower_destructor_entity(class)
                        && self.graph.lower_nontrivial_destructor(class)?
                        && self.lifetime_call_throws(CallTarget::Direct(destructor))?
                    {
                        return Ok(true);
                    }
                    work.push((operand, conditional));
                }
                ExprKind::New {
                    placement,
                    initializer,
                    ..
                } => {
                    let use_ = self.graph.lower_allocation(id)?;
                    let function = self.graph.lower_function(use_.function)?;
                    let work_required = initializer.is_some()
                        || use_
                            .constructor
                            .is_some_and(|(id, _)| !self.graph.lower_constructor_trivial(id));
                    if conditional
                        || use_.array.is_none()
                            && function.nothrow
                            && !use_.reserved
                            && work_required
                    {
                        return Ok(true);
                    }
                    if let Some(placement) = placement {
                        work.extend(
                            self.tree
                                .expr_lists
                                .get(placement)
                                .iter()
                                .map(|&id| (id, conditional)),
                        );
                    }
                    work.extend(use_.array.map(|array| (array.count, conditional)));
                    self.push_initializer_branches(initializer, conditional, work);
                }
                _ => (),
            }
        }
        Ok(false)
    }
    fn push_initializer_branches(
        &self,
        initializer: Option<ast::Initializer>,
        conditional: bool,
        work: &mut Vec<(ast::ExprId, bool)>,
    ) {
        if let Some(initializer) = initializer {
            match initializer.kind {
                ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                    work.push((id, conditional));
                }
                ast::InitializerKind::Parenthesized(arguments) => work.extend(
                    self.tree
                        .expr_lists
                        .get(arguments)
                        .iter()
                        .map(|&id| (id, conditional)),
                ),
                _ => (),
            }
        }
    }
}
