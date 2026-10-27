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
    pub(super) fn with_full_root<T>(
        &mut self,
        root: Option<ast::ExprId>,
        evaluate: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        // full_root/choice_root_ is a source evaluation scope, independent
        // of whether the temporary query opens an exception region.
        let root = root.map(|id| self.without_parentheses(id));
        let saved = std::mem::replace(&mut self.lifetime.tracking.root, root);
        let result = evaluate(self);
        self.lifetime.tracking.root = saved;
        result
    }
    pub(super) fn branch_temporaries(&mut self, root: ast::ExprId) -> Result<bool, Error> {
        let mut work = std::mem::take(&mut self.lifetime.tracking.query_work);
        work.push((root, false, 0));
        let result = self.scan_branch_temporaries(&mut work);
        work.clear();
        self.lifetime.tracking.query_work = work;
        result
    }
    fn scan_branch_temporaries(
        &mut self,
        work: &mut Vec<(ast::ExprId, bool, u32)>,
    ) -> Result<bool, Error> {
        while let Some((id, _, depth)) = work.pop() {
            if depth > 64 {
                continue;
            }
            let id = self.without_parentheses(id);
            let child = |id| (id, false, depth + 1);
            match self.tree.expressions.get(id).kind {
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    let facts = self.graph.lower_expression(id)?;
                    let object = (!facts.is_place
                        && matches!(self.graph.lower_type(facts.ty)?, TypeView::Class(_)))
                    .then_some(facts.ty);
                    for operand in [then_expr, else_expr] {
                        let made = if let Some(object) = object {
                            self.conditional_operand_makes_temporaries(operand, object)?
                        } else {
                            self.expression_temporaries(operand)?.made
                        };
                        if made {
                            return Ok(true);
                        }
                    }
                    work.push(child(condition));
                }
                ExprKind::Binary {
                    op, left, right, ..
                } => {
                    if matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr))
                        && self.expression_temporaries(right)?.made
                    {
                        return Ok(true);
                    }
                    work.extend([child(right), child(left)]);
                }
                ExprKind::ComplexPart { operand, .. }
                | ExprKind::Unary { operand, .. }
                | ExprKind::Cast { operand, .. } => {
                    work.push(child(operand));
                }
                ExprKind::Call { arguments, .. } => {
                    work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .copied()
                            .map(child),
                    );
                }
                _ => (),
            }
        }
        Ok(false)
    }
    pub(super) fn begin_tracking(
        &mut self,
        root: ast::ExprId,
        usage: ExpressionUse,
    ) -> Result<(), Error> {
        if !self.lifetime.full_expression
            || self.lifetime.tracking.active
            || self.static_initializer
            || self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .host_objects()
            || self.element_temporaries.depth != 0
            || self.constructor_unwind_target().is_some()
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
        if matches!(usage, ExpressionUse::Ordinary) && self.lifetime_chain_empty() {
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
            // guarded_temporaries also excludes syntax-only scalar lists.
            if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. })
                && !self.graph.lower_list_is_typed(id)
            {
                continue;
            }
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
                        && super::types::storage(&*self.graph, facts.ty)? != Type::Void
                        && let Some(selected) = self
                            .graph
                            .lower_constant_conversion(&*self.tree, condition)?
                    {
                        work.push((
                            if selected { then_expr } else { else_expr },
                            at_root,
                            depth + 1,
                        ));
                        continue;
                    }
                    if facts.is_place || class || !at_root {
                        let made = if class {
                            self.conditional_operand_makes_temporaries(then_expr, facts.ty)?
                                || self
                                    .conditional_operand_makes_temporaries(else_expr, facts.ty)?
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
                ExprKind::ComplexPart { operand, .. }
                | ExprKind::Unary { operand, .. }
                | ExprKind::Postfix { operand, .. }
                | ExprKind::Cast { operand, .. }
                | ExprKind::Member { base: operand, .. } => work.push(child(operand)),
                ExprKind::Subscript { base, index } => work.extend([base, index].map(child)),
                ExprKind::Call { callee, arguments } => {
                    // Intrinsics retain an untyped source designation. Its
                    // operands still participate in guarded lifetime queries.
                    if !self.graph.expression_is_intrinsic(id)? {
                        work.push(child(callee));
                    }
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
        self.lifetime.region_entry = Some(target);
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
        self.place_tracked_chain()?;
        // The shared chain owns slot-based addresses. An unconditionally
        // extended object is already complete here, so later initializer
        // destruction uses its original address, even before tracking ends.
        for cleanup in &mut self.lifetime.cleanups[self.lifetime.tracking.floor..] {
            if cleanup.guard.is_some()
                && !cleanup.branched
                && matches!(cleanup.place, super::lifetime::CleanupPlace::Extended(_))
                && let Some(object) = cleanup.made_at
            {
                cleanup.place = super::lifetime::CleanupPlace::Extended(object);
            }
        }
        Ok(())
    }
}
