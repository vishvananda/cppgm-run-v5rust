// Translated from lower/tracked_lifetime_lowering.cpp. See NOTICE.
//! A full-expression owns its branch-created flags, region markers and dispatch.
use super::{Error, function::Function, temporary_scan::ExpressionUse};
use crate::{
    lex::pp_lexer::Punctuator as P,
    lowir::{
        builder::Position,
        model::{BlockId, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::Type,
    },
    sema::lowering::TypeView,
    syntax::ast::{self, ExprKind},
};

#[derive(Default)]
pub(super) enum ConstructionTracking {
    #[default]
    Expression,
    Storage,
}
impl ConstructionTracking {
    pub(super) fn active(&self) -> bool {
        matches!(self, Self::Storage)
    }
}
#[derive(Default)]
pub(super) struct Tracking {
    pub active: bool,
    pub construction: ConstructionTracking,
    pub finishing: bool,
    pub made: bool,
    pub start: Option<Position>,
    pub head: Option<BlockId>,
    pub markers: Vec<Position>,
    pub floor: usize,
    resets: Vec<Value>,
    pub root: Option<ast::ExprId>,
    query_work: Vec<(ast::ExprId, bool, u32)>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn begin_tracking(
        &mut self,
        root: ast::ExprId,
        usage: ExpressionUse,
    ) -> Result<(), Error> {
        if !self.lifetime.full_expression
            || self.lifetime.tracking.active
            || self.static_initializer
        {
            return Ok(());
        }
        self.lifetime.tracking.root = (!matches!(usage, ExpressionUse::InitializerArgument))
            .then_some(self.without_parentheses(root));
        if !self.guarded_temporaries(root, usage)? {
            return Ok(());
        }
        self.lifetime.tracking.active = true;
        self.lifetime.tracking.construction = ConstructionTracking::Expression;
        self.lifetime.tracking.made = false;
        self.lifetime.tracking.start = self.lifetime.expression_start;
        self.lifetime.tracking.head = None;
        self.lifetime.tracking.resets.clear();
        self.lifetime.tracking.floor = self.lifetime.cleanups.len();
        if matches!(usage, ExpressionUse::Ordinary) && self.lifetime.cleanups.is_empty() {
            self.open_temporary_region()?;
        }
        Ok(())
    }
    pub(super) fn branch_local(&self, id: ast::ExprId) -> bool {
        !self.lifetime.tracking.active
            && self.lifetime.tracking.root == Some(self.without_parentheses(id))
    }
    fn guarded_temporaries(
        &mut self,
        root: ast::ExprId,
        usage: ExpressionUse,
    ) -> Result<bool, Error> {
        let mut work = std::mem::take(&mut self.lifetime.tracking.query_work);
        work.push((
            root,
            !matches!(usage, ExpressionUse::InitializerArgument),
            0,
        ));
        let answer =
            self.scan_guarded_temporaries(&mut work, matches!(usage, ExpressionUse::ClassResult));
        work.clear();
        self.lifetime.tracking.query_work = work;
        answer
    }
    fn scan_guarded_temporaries(
        &mut self,
        work: &mut Vec<(ast::ExprId, bool, u32)>,
        result: bool,
    ) -> Result<bool, Error> {
        while let Some((id, at_root, depth)) = work.pop() {
            let id = self.without_parentheses(id);
            let facts = self.graph.lower_expression(id)?;
            if depth > 64 || facts.folded.is_some() {
                continue;
            }
            let child = |id| (id, false, depth + 1);
            match self.tree.expressions.get(id).kind {
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    let class = !facts.is_place
                        && matches!(self.graph.lower_type(facts.ty)?, TypeView::Class(_));
                    if at_root && result {
                        work.extend([
                            (condition, false, depth + 1),
                            (then_expr, true, depth + 1),
                            (else_expr, true, depth + 1),
                        ]);
                        continue;
                    }
                    if !facts.is_place
                        && !class
                        && super::types::storage(self.graph, facts.ty)? != Type::Void
                        && let Some(selected) = self.graph.lower_expression(condition)?.folded
                    {
                        work.push((
                            if selected != 0 { then_expr } else { else_expr },
                            at_root,
                            depth + 1,
                        ));
                        continue;
                    }
                    if facts.is_place || class || !at_root {
                        let made = if class {
                            self.operand_temporaries(then_expr)?.made
                                || self.operand_temporaries(else_expr)?.made
                        } else {
                            self.expression_temporaries(then_expr)?.made
                                || self.expression_temporaries(else_expr)?.made
                        };
                        if made {
                            return Ok(true);
                        }
                    }
                    work.extend([condition, then_expr, else_expr].map(child));
                }
                ExprKind::Binary {
                    op, left, right, ..
                } => {
                    if !at_root
                        && matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr))
                        && !self.graph.lower_operator_call(id)?
                        && self.literal_truth(left)?.is_none()
                        && self.expression_temporaries(right)?.made
                    {
                        return Ok(true);
                    }
                    work.extend([left, right].map(child));
                }
                ExprKind::Unary { operand, .. }
                | ExprKind::Postfix { operand, .. }
                | ExprKind::Cast { operand, .. }
                | ExprKind::Member { base: operand, .. } => work.push(child(operand)),
                ExprKind::Subscript { base, index } => work.extend([base, index].map(child)),
                ExprKind::Call { callee, arguments } => {
                    work.push(child(callee));
                    work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .copied()
                            .map(child),
                    );
                }
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .copied()
                            .map(child),
                    );
                    work.extend(braced.map(child));
                }
                ExprKind::Braced { elements, .. } => work.extend(
                    self.tree
                        .expr_lists
                        .get(elements)
                        .iter()
                        .copied()
                        .map(child),
                ),
                ExprKind::New { initializer, .. } => {
                    let use_ = self.graph.lower_allocation(id)?;
                    let function = self.graph.lower_function(use_.function)?;
                    if function.nothrow && !use_.reserved {
                        let made = match initializer.map(|value| value.kind) {
                            Some(
                                ast::InitializerKind::Assign(root)
                                | ast::InitializerKind::Braced(root),
                            ) if matches!((self.graph.lower_type(use_.target)?,
                                    self.graph.lower_type(self.graph.lower_expression(root)?.ty)?),
                                    (TypeView::Class(a), TypeView::Class(b)) if a == b) =>
                            {
                                self.operand_temporaries(root)?.made
                            }
                            _ => self.initializer_temporaries(initializer)?,
                        };
                        if made {
                            return Ok(true);
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(false)
    }
    pub(super) fn track_temporary(&mut self) -> Result<(), Error> {
        if !self.lifetime.tracking.active && self.lifetime.guarded_operands == 0 {
            return Ok(());
        }
        let flag = self.generated_slot("lifetime", Type::U8)?;
        if self.lifetime.tracking.active {
            self.lifetime.tracking.resets.push(flag);
        } else {
            let start = self
                .lifetime
                .guard_start
                .ok_or(crate::sema::Error::Invariant(
                    "guarded temporary without branch boundary",
                ))?;
            let zero = self.builder.integer(0)?;
            self.lifetime.guard_start = Some(
                self.builder
                    .insert_effect(start, I::Store(Facts::default(), Type::U8, zero, flag))?,
            );
        }
        let one = self.builder.integer(1)?;
        self.effect(I::Store(Facts::default(), Type::U8, one, flag))?;
        let cleanup = self
            .lifetime
            .cleanups
            .last_mut()
            .ok_or(crate::sema::Error::Invariant("flag without temporary"))?;
        cleanup.guard = Some(flag);
        cleanup.branched = self.lifetime.guarded_operands != 0;
        if self.lifetime.tracking.active
            && let super::lifetime::CleanupPlace::Temporary(object) = cleanup.place
        {
            cleanup.made_at = Some(object);
            cleanup.place =
                super::lifetime::CleanupPlace::Temporary(self.builder.addressed_storage(object));
        }
        Ok(())
    }
    pub(super) fn tracked_target(&mut self) -> Result<BlockId, Error> {
        if let Some(head) = self.lifetime.tracking.head {
            return Ok(head);
        }
        let head = self.new_block("conditional_cleanup_landing")?;
        self.lifetime.tracking.head = Some(head);
        Ok(head)
    }
    pub(super) fn tracked_region(&mut self, start: Option<Position>) -> Result<(), Error> {
        let target = self.tracked_target()?;
        let marker = if let Some(start) = start.filter(|&p| self.builder.same_block(p)) {
            self.builder
                .insert_effect(start, I::Runtime(R::EhTry(target)))?
        } else {
            self.effect(I::Runtime(R::EhTry(target)))?;
            self.builder.position()?
        };
        if !self.lifetime.tracking.made {
            self.lifetime.tracking.markers.push(marker);
        }
        self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
        self.lifetime.region = true;
        self.lifetime.region_depth = self.lifetime.cleanups.len();
        self.lifetime.regions_opened += 1;
        Ok(())
    }
    pub(super) fn finish_tracking(&mut self) -> Result<(), Error> {
        if !self.lifetime.tracking.active || self.lifetime.tracking.made {
            return Ok(());
        }
        self.lifetime.tracking.made = true;
        let start = self
            .lifetime
            .tracking
            .start
            .ok_or(crate::sema::Error::Invariant("tracking without start"))?;
        let zero = self.builder.integer(0)?;
        self.builder.insert_effects(
            start,
            self.lifetime
                .tracking
                .resets
                .drain(..)
                .rev()
                .map(|flag| I::Store(Facts::default(), Type::U8, zero, flag)),
        )?;
        self.place_tracked_chain()
    }
}
