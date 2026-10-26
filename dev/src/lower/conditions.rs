// Translated from lower/condition_lowering.cpp and statement_lowering.cpp. See NOTICE.
//! Scalar truth preserves source branch sense; logical values use short circuit slots.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    lowir::{
        model::{BlockId, Facts, InstructionKind as I},
        vocabulary::{Compare, Type},
    },
    syntax::ast::{self, Condition, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    // Match the frozen address/value effect query for static member objects.
    // Its semantic depth cap is independent of the native stack budget.
    pub(super) fn side_effect_free(
        &self,
        id: ast::ExprId,
        read: bool,
        depth: u32,
    ) -> Result<bool, Error> {
        let id = self.without_parentheses(id);
        if self.graph.lower_arrow_call(id, 0)?.is_some()
            || depth > 64
            || read && self.graph.lower_cv(self.graph.lower_expression(id)?.ty) & 2 != 0
        {
            return Ok(false);
        }
        Ok(match self.tree.expressions.get(id).kind {
            ExprKind::Id(_) | ExprKind::Literal(_) | ExprKind::Keyword(_) => true,
            ExprKind::Member { base, op, .. } => {
                self.side_effect_free(base, op.token.as_punctuator() == Some(P::Arrow), depth + 1)?
            }
            ExprKind::Subscript { base, index } => {
                let array = matches!(
                    self.graph
                        .lower_type(self.graph.lower_expression(base)?.ty)?,
                    crate::sema::lowering::TypeView::Array { .. }
                );
                self.side_effect_free(base, !array, depth + 1)?
                    && self.side_effect_free(index, true, depth + 1)?
            }
            _ => false,
        })
    }
    pub(super) fn member_object(&mut self, base: ast::ExprId, arrow: bool) -> Result<(), Error> {
        if arrow && self.graph.lower_arrow_call(base, 0)?.is_some() {
            self.arrow_operand(base)?;
        } else if !self.side_effect_free(base, arrow, 0)? {
            self.discard(base)?;
        }
        Ok(())
    }
    pub(super) fn conditional_place(
        &mut self,
        id: ast::ExprId,
        condition: ast::ExprId,
        then_expr: ast::ExprId,
        else_expr: ast::ExprId,
    ) -> Result<crate::lowir::model::Value, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let slot = self.generated_slot("condaddr", Type::Ptr)?;
        let then_block = self.new_block("condaddr_then")?;
        let else_block = self.new_block("condaddr_else")?;
        let end = self.new_block("condaddr_end")?;
        let guard_start = self.lifetime.guard_start;
        if guard_start.is_none() {
            self.lifetime.guard_start = Some(self.builder.position()?);
        }
        let condition = self.truth(condition)?;
        if self.lifetime.construction && !self.lifetime.full_expression {
            self.close_lifetime_region()?;
        }
        self.effect(I::Branch(condition.value, then_block, else_block))?;
        self.terminated = true;
        let offsets = self.graph.lower_conditional_projections(id)?;
        for ((block, expr), offset) in [(then_block, then_expr), (else_block, else_expr)]
            .into_iter()
            .zip(offsets)
        {
            self.place(block)?;
            self.lifetime.guarded_operands += 1;
            let result = if matches!(
                self.tree
                    .expressions
                    .get(self.without_parentheses(expr))
                    .kind,
                ExprKind::Throw(_)
            ) {
                self.discard(expr)
            } else {
                self.address(expr).and_then(|mut address| {
                    if let Some(offset) = offset {
                        address = self.base_projection(address, source, offset)?;
                    }
                    self.effect(I::Store(
                        Facts::memory(false, true),
                        Type::Ptr,
                        address,
                        slot,
                    ))
                })
            };
            self.lifetime.guarded_operands -= 1;
            result?;
            if !self.terminated {
                self.jump(end)?;
            }
        }
        self.place(end)?;
        if guard_start.is_none() {
            self.lifetime.guard_start = None;
        }
        Ok(self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)?
            .value)
    }
    pub(super) fn without_parentheses(&self, mut id: ast::ExprId) -> ast::ExprId {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        id
    }
    pub(super) fn literal_truth(&self, id: ast::ExprId) -> Result<Option<bool>, Error> {
        self.graph
            .lower_literal_truth(self.tree, id)
            .map_err(Error::from)
    }
    pub(super) fn truth(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let id = self.without_parentheses(id);
        let value = if let Some((call, _)) = self.graph.lower_function_conversion(id)? {
            self.call_truth_conversion(id, call)?
        } else if matches!(self.tree.expressions.get(id).kind,
            ExprKind::Binary { op, .. } if matches!(op.token.as_punctuator(),
                Some(P::Equal | P::NotEqual | P::Less | P::Greater | P::LessEqual | P::GreaterEqual | P::LogicalAnd | P::LogicalOr)))
            && !self.graph.lower_operator_call(id)?
        {
            // truth_operand lowers a built-in comparison/logical directly:
            // its operands have scopes, but the operation inherits its caller.
            self.expression_located(id, Context::Operand)?
        } else {
            self.expression(id, Context::Operand)?
        };
        self.scalar_truth(value)
    }
    fn scalar_truth(&mut self, value: RValue) -> Result<RValue, Error> {
        if super::complex::part(self.graph, value.source)?.is_some() {
            return self.complex_truth(value, value.source);
        }
        if self.graph.lower_member_pointer_type(value.source).is_some() {
            return self.member_pointer_truth(value);
        }
        if let Some(kind) = super::soft_float::kind(self.graph, value.source)? {
            let zero = self.soft_constant(kind, 0, value.source)?;
            return self.soft_compare(Compare::Ne, kind, value, zero, value.source);
        }
        if value.ty.float_rank() != 0 {
            let zero = self.zero(value.source)?;
            self.emit(
                I::Compare(Compare::Ne, value.ty, value.value, zero.value),
                value.source,
            )
        } else {
            Ok(value)
        }
    }
    fn declared_condition_value(
        &mut self,
        declarator: ast::DeclaratorId,
        entity: crate::sema::lowering::EntityId,
        to_bool: bool,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_entity_type(entity)?;
        if let Some((call, target)) = self.graph.lower_condition_function(declarator)? {
            let place = *self
                .locals
                .get(&entity)
                .ok_or(crate::sema::Error::Invariant("condition without storage"))?;
            let value = self.condition_conversion_call(place, source, call)?;
            if call.place && !to_bool {
                // condition_variable converts the declared reference result
                // itself, without a load. Its convert_integer sees a reference
                // as unsigned and the ptr boundary as zero integer bits.
                return self.emit(
                    I::Convert(
                        crate::lowir::vocabulary::Convert::ZeroExtend,
                        types::storage(self.graph, target)?,
                        value.ty,
                        value.value,
                    ),
                    target,
                );
            }
            return self.convert(value, target, Context::Operand);
        }
        let ty = types::storage(self.graph, source)?;
        let place =
            *self
                .locals
                .get(&entity)
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "condition without storage",
                )))?;
        let value = self.emit(
            I::Load(types::memory(self.graph, source)?, ty, place),
            source,
        )?;
        if to_bool {
            self.scalar_truth(value)
        } else {
            self.convert(
                value,
                self.graph.lower_condition_target(declarator)?,
                Context::Operand,
            )
        }
    }
    pub(super) fn condition_value(
        &mut self,
        condition: Condition,
        to_bool: bool,
    ) -> Result<RValue, Error> {
        let id = match condition {
            Condition::Expression(id) => id,
            Condition::Declaration {
                declarator,
                initializer,
                ..
            } => {
                self.local_variable(declarator, Some(initializer), None)?;
                let entity = self.graph.lower_declared(declarator)?;
                return self.declared_condition_value(declarator, entity, to_bool);
            }
        };
        let first = self.lifetime.cleanups.len();
        let began =
            self.begin_full_expression(Some(id), super::temporary_scan::ExpressionUse::Value)?;
        // The frozen switch selector is a full-expression without full_root;
        // its conditional temporaries remain guarded until selector evaluation.
        let value = if to_bool {
            self.truth(id)
        } else {
            self.operand(id, Context::Operand)
        };
        self.destroy_temporaries(first)?;
        self.end_lifetime_expression(began)?;
        value
    }
    pub(super) fn condition_branch(
        &mut self,
        condition: Condition,
        yes: BlockId,
        no: BlockId,
        fold: bool,
    ) -> Result<(), Error> {
        let root = match condition {
            Condition::Expression(id) => Some(id),
            Condition::Declaration { .. } => None,
        };
        self.with_full_root(root, |this| {
            this.condition_branch_in_context(condition, yes, no, fold)
        })
    }
    fn condition_branch_in_context(
        &mut self,
        condition: Condition,
        yes: BlockId,
        no: BlockId,
        fold: bool,
    ) -> Result<(), Error> {
        if matches!(condition, Condition::Declaration { .. }) {
            let value = self.condition_value(condition, true)?;
            self.effect(I::Branch(value.value, yes, no))?;
            self.terminated = true;
            return Ok(());
        }
        let first = self.lifetime.cleanups.len();
        let began = if let Condition::Expression(id) = condition {
            self.begin_full_expression(Some(id), super::temporary_scan::ExpressionUse::Value)?
        } else {
            self.begin_lifetime_expression()
        };
        if let Condition::Expression(id) = condition {
            if self.expression_temporaries(id)?.made {
                let value = self.truth(id)?;
                let root = self.without_parentheses(id);
                let logical_left = match self.tree.expressions.get(root).kind {
                    ExprKind::Binary { op, left, .. }
                        if matches!(
                            op.token.as_punctuator(),
                            Some(P::LogicalAnd | P::LogicalOr)
                        ) =>
                    {
                        Some(left)
                    }
                    _ => None,
                };
                if let Some(left) = logical_left
                    && (self.lifetime.cleanups.len() == first
                        || self.literal_truth(left)?.is_some())
                {
                    self.destroy_temporaries(first)?;
                    self.end_lifetime_expression(began)?;
                    self.effect(I::Branch(value.value, yes, no))?;
                    self.terminated = true;
                    return Ok(());
                }
                let true_cleanup = self.new_block("cond_true_cleanup")?;
                let false_cleanup = self.new_block("cond_false_cleanup")?;
                self.effect(I::Branch(value.value, true_cleanup, false_cleanup))?;
                self.end_lifetime_expression(began)?;
                self.place(true_cleanup)?;
                self.destroy_temporaries_from(first, false)?;
                self.jump(yes)?;
                self.place(false_cleanup)?;
                self.destroy_temporaries(first)?;
                return self.jump(no);
            }
            let root = self.without_parentheses(id);
            let logical = matches!(self.tree.expressions.get(root).kind,
                ExprKind::Binary { op, .. } if matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr)))
                && !self.graph.lower_operator_call(root)?;
            if logical && !self.lifetime.cleanups.is_empty() && self.expression_throws(id)? {
                let value = self.truth(id)?;
                self.end_lifetime_expression(began)?;
                self.effect(I::Branch(value.value, yes, no))?;
                self.terminated = true;
                return Ok(());
            }
            self.branch_condition(id, yes, no, fold)?;
        } else {
            let value = self.condition_value(condition, true)?;
            self.effect(I::Branch(value.value, yes, no))?;
            self.terminated = true;
        }
        self.end_lifetime_expression(began)
    }
    fn branch_condition(
        &mut self,
        id: ast::ExprId,
        yes: BlockId,
        no: BlockId,
        fold: bool,
    ) -> Result<(), Error> {
        self.check_stack()?;
        // Allocate outer right blocks first, then place them innermost first.
        let mut spine = Vec::new();
        let mut id = self.without_parentheses(id);
        let (mut on_true, mut on_false) = (yes, no);
        while let ExprKind::Binary {
            op,
            left,
            right,
            assignment: false,
        } = self.tree.expressions.get(id).kind
        {
            let op = op.token.as_punctuator();
            if !matches!(op, Some(P::LogicalAnd | P::LogicalOr))
                || self.graph.lower_operator_call(id)?
            {
                break;
            }
            let conjunction = op == Some(P::LogicalAnd);
            let block = self.new_block(if conjunction { "land_rhs" } else { "lor_rhs" })?;
            spine.push((right, block, on_true, on_false));
            if conjunction {
                on_true = block;
            } else {
                on_false = block;
            }
            id = self.without_parentheses(left);
        }
        if let Some(value) = if fold { self.literal_truth(id)? } else { None } {
            let target = if value { on_true } else { on_false };
            self.folded_targets.push(target);
            self.jump(target)?;
        } else {
            let value = self.truth(id)?;
            self.effect(I::Branch(value.value, on_true, on_false))?;
            self.terminated = true;
        }
        for (right, block, yes, no) in spine.into_iter().rev() {
            self.place(block)?;
            self.branch_condition(right, yes, no, fold)?;
        }
        Ok(())
    }
    fn normalized_truth(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let value = self.truth(id)?;
        let zero = self.builder.integer(0)?;
        self.emit(
            I::Compare(Compare::Ne, value.ty, value.value, zero),
            value.source,
        )
    }
    pub(super) fn logical_value(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let mut spine = Vec::new();
        let mut node = self.without_parentheses(id);
        while let ExprKind::Binary {
            op,
            left,
            right,
            assignment: false,
        } = self.tree.expressions.get(node).kind
        {
            let op = op.token.as_punctuator();
            if !matches!(op, Some(P::LogicalAnd | P::LogicalOr))
                || self.graph.lower_operator_call(node)?
            {
                break;
            }
            spine.push((node, left, right, op == Some(P::LogicalAnd)));
            node = self.without_parentheses(left);
        }
        // Inner logical operands can outlive their branch. Clear their
        // completion flags before the entire chain, as begin_guard_scope does.
        let guards = !self.branch_local(id) || spine.len() > 1;
        let guard_start = self.lifetime.guard_start;
        if guards && guard_start.is_none() {
            self.lifetime.guard_start = Some(self.builder.position()?);
        }
        let result = self.logical_chain(spine);
        if guards && guard_start.is_none() {
            self.lifetime.guard_start = None;
        }
        result
    }
    fn logical_chain(
        &mut self,
        spine: Vec<(ast::ExprId, ast::ExprId, ast::ExprId, bool)>,
    ) -> Result<RValue, Error> {
        let mut result = None;
        for (node, left, right, conjunction) in spine.into_iter().rev() {
            let source = self.graph.lower_expression(node)?.ty;
            if result.is_none()
                && let Some(value) = self.literal_truth(left)?
            {
                result = Some(if value == conjunction {
                    let value = self.normalized_truth(right)?;
                    RValue { source, ..value }
                } else {
                    RValue {
                        value: self.builder.integer(u128::from(value))?,
                        ty: Type::U8,
                        source,
                        literal: false,
                        bits: Some(u64::from(value)),
                    }
                });
                continue;
            }
            let local = self.branch_local(node);
            let prefix = if conjunction { "land" } else { "lor" };
            let slot = self.generated_slot(prefix, Type::I64)?;
            let rhs = self.new_block(&format!("{prefix}_rhs"))?;
            let shortcut = self.new_block(&format!("{prefix}_short"))?;
            let end = self.new_block(&format!("{prefix}_end"))?;
            let value = if let Some(value) = result {
                value
            } else {
                self.truth(left)?
            };
            let (yes, no) = if conjunction {
                (rhs, shortcut)
            } else {
                (shortcut, rhs)
            };
            self.effect(I::Branch(value.value, yes, no))?;
            self.place(rhs)?;
            // Only the right operand's temporaries end on this branch.
            // Evaluating the left may already have registered objects owned
            // by the enclosing full-expression, including by-value arguments.
            let first = self.lifetime.cleanups.len();
            self.lifetime.guarded_operands += u32::from(!local);
            let value = self.normalized_truth(right)?;
            self.lifetime.guarded_operands -= u32::from(!local);
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                value.value,
                slot,
            ))?;
            if local {
                self.destroy_temporaries(first)?;
            }
            self.jump(end)?;
            self.place(shortcut)?;
            let value = self.builder.integer(u128::from(!conjunction))?;
            self.effect(I::Store(Facts::memory(false, true), Type::I64, value, slot))?;
            self.jump(end)?;
            self.place(end)?;
            result = Some(self.emit(I::Load(Facts::memory(false, true), Type::I64, slot), source)?);
        }
        result.ok_or(Error::Semantic(crate::sema::Error::Invariant(
            "logical expression without operator",
        )))
    }
    fn read_when_discarded(&self, id: ast::ExprId) -> Result<bool, Error> {
        self.check_stack()?;
        Ok(
            match self.tree.expressions.get(self.without_parentheses(id)).kind {
                ExprKind::Id(_) | ExprKind::Subscript { .. } | ExprKind::Member { .. } => true,
                ExprKind::Unary { op, .. } => op.token.as_punctuator() == Some(P::Multiply),
                ExprKind::Conditional {
                    then_expr,
                    else_expr,
                    ..
                } => self.read_when_discarded(then_expr)? && self.read_when_discarded(else_expr)?,
                ExprKind::Binary {
                    op,
                    right,
                    assignment: false,
                    ..
                } if op.token.as_punctuator() == Some(P::Comma) => {
                    self.read_when_discarded(right)?
                }
                _ => false,
            },
        )
    }
    fn read_discarded(&mut self, id: ast::ExprId) -> Result<(), Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        if self.graph.lower_expression(id)?.is_bit_field() {
            self.bit_field_value(id)?;
            return Ok(());
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let yes = self.new_block("discard_cond_then")?;
                let no = self.new_block("discard_cond_else")?;
                let end = self.new_block("discard_cond_end")?;
                let value = self.truth(condition)?;
                self.effect(I::Branch(value.value, yes, no))?;
                for (block, operand) in [(yes, then_expr), (no, else_expr)] {
                    self.place(block)?;
                    self.lifetime.guarded_operands += 1;
                    self.read_discarded(operand)?;
                    self.lifetime.guarded_operands -= 1;
                    self.jump(end)?;
                }
                self.place(end)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard(left)?;
                self.read_discarded(right)
            }
            _ => {
                let source = self.graph.lower_expression(id)?.ty;
                let place = self.scalar_place(id)?;
                self.emit(
                    I::Load(
                        Facts::memory(true, false),
                        types::storage(self.graph, source)?,
                        place,
                    ),
                    source,
                )?;
                Ok(())
            }
        }
    }
    pub(super) fn discard(&mut self, id: ast::ExprId) -> Result<(), Error> {
        let location = self
            .tree
            .expressions
            .get(self.without_parentheses(id))
            .location;
        self.with_debug_location(location, |this| this.discard_located(id))
    }
    fn discard_located(&mut self, id: ast::ExprId) -> Result<(), Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        if let ExprKind::BuiltinValue {
            builtin, operand, ..
        } = self.tree.expressions.get(id).kind
        {
            self.builtin_value(id, builtin, operand)?;
            return Ok(());
        }
        // The builtin advances the list and produces bytes directly, even
        // when its class result is discarded (discard_lowering.cpp).
        if let ExprKind::VaArg { operand, .. } = self.tree.expressions.get(id).kind {
            self.va_arg(id, operand)?;
            return Ok(());
        }
        let facts = self.graph.lower_expression(id)?;
        if !facts.is_place
            && matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Class(_)
            )
        {
            self.class_temporary(id, super::materialization::TemporaryUse::Discard, false)?;
            return Ok(());
        }
        if self.graph.lower_literal_call(id)?.is_some() {
            self.literal_call(id)?;
            return Ok(());
        }
        if self.graph.lower_operator_call(id)? {
            self.operator_call(id)?;
            return Ok(());
        }
        if self.graph.lower_lvalue(id)?
            && self.graph.lower_cv(facts.ty) & 2 != 0
            && !matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Array { .. }
            )
            && self.read_when_discarded(id)?
        {
            return self.read_discarded(id);
        }
        if facts.is_bit_field()
            && matches!(
                self.tree.expressions.get(id).kind,
                ExprKind::Binary {
                    assignment: true,
                    ..
                } | ExprKind::Unary { .. }
            )
        {
            self.bit_field_value(id)?;
            return Ok(());
        }
        if facts.is_bit_field()
            && matches!(self.tree.expressions.get(id).kind, ExprKind::Member { .. })
        {
            if let ExprKind::Member { base, op, .. } = self.tree.expressions.get(id).kind {
                self.member_object(base, op.token.as_punctuator() == Some(P::Arrow))?;
            }
            return Ok(());
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::FunctionalCast { .. } | ExprKind::Call { .. }
                if matches!(
                    self.graph.lower_type(facts.ty)?,
                    crate::sema::lowering::TypeView::Scalar(crate::sema::lowering::Scalar::Void)
                ) && (matches!(
                    self.tree.expressions.get(id).kind,
                    ExprKind::FunctionalCast { .. }
                ) || self.graph.lower_functional_call(id)?) =>
            {
                // The frozen discard owner does not evaluate a void functional
                // cast. Its rvalue path still evaluates the cast's operand.
                Ok(())
            }
            ExprKind::FunctionalCast {
                braced: Some(_), ..
            } if matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Array { .. }
            ) =>
            {
                self.array_temporary(id, "discardarr")?;
                Ok(())
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } if matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Scalar(crate::sema::lowering::Scalar::Void)
            ) =>
            {
                let yes = self.new_block("discard_cond_then")?;
                let no = self.new_block("discard_cond_else")?;
                let end = self.new_block("discard_cond_end")?;
                let value = self.truth(condition)?;
                self.effect(I::Branch(value.value, yes, no))?;
                self.terminated = true;
                let local = self.branch_local(id);
                for (block, operand) in [(yes, then_expr), (no, else_expr)] {
                    self.place(block)?;
                    let first = self.lifetime.cleanups.len();
                    self.lifetime.guarded_operands += u32::from(!local);
                    self.discard(operand)?;
                    self.lifetime.guarded_operands -= u32::from(!local);
                    if local {
                        self.destroy_temporaries(first)?;
                    }
                    self.jump(end)?;
                }
                self.place(end)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard(left)?;
                self.discard(right)
            }
            ExprKind::Cast { operand, .. }
                if matches!(
                    self.graph.lower_type(facts.ty)?,
                    crate::sema::lowering::TypeView::Scalar(crate::sema::lowering::Scalar::Void)
                ) =>
            {
                self.discard_to_void(operand)
            }
            ExprKind::Call { callee, arguments } if !self.graph.lower_functional_call(id)? => {
                self.call(id, callee, arguments)?;
                Ok(())
            }
            ExprKind::Binary {
                assignment: true, ..
            } => {
                self.scalar_place(id)?;
                Ok(())
            }
            ExprKind::Unary { op, .. }
                if matches!(op.token.as_punctuator(), Some(P::Increment | P::Decrement)) =>
            {
                self.scalar_place(id)?;
                Ok(())
            }
            ExprKind::Unary { op, .. }
                if op.token.as_punctuator() == Some(P::Multiply)
                    && self.graph.lower_cv(self.graph.lower_expression(id)?.ty) & 2 == 0 =>
            {
                self.scalar_place(id)?;
                Ok(())
            }
            ExprKind::Id(_)
                if self.graph.lower_cv(self.graph.lower_expression(id)?.ty) & 2 == 0 =>
            {
                Ok(())
            }
            ExprKind::TypeTrait {
                keyword: Keyword::Typeid,
                ..
            } => {
                if self
                    .graph
                    .lower_type_query(id)
                    .is_some_and(|query| query.dynamic)
                {
                    self.scalar_place(id)?;
                }
                Ok(())
            }
            ExprKind::Literal(_)
            | ExprKind::Keyword(_)
            | ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ..
            } => Ok(()),
            _ if self.graph.lower_expression(id)?.is_place => {
                self.scalar_place(id)?;
                Ok(())
            }
            _ => {
                self.expression(id, Context::Operand)?;
                Ok(())
            }
        }
    }
    pub(super) fn discard_to_void(&mut self, id: ast::ExprId) -> Result<(), Error> {
        let id = self.without_parentheses(id);
        if matches!(self.tree.expressions.get(id).kind, ExprKind::Call { .. }) {
            return self.discard(id);
        }
        let facts = self.graph.lower_expression(id)?;
        let kind = self.graph.lower_type(facts.ty)?;
        let class_or_array = matches!(
            kind,
            crate::sema::lowering::TypeView::Class(_)
                | crate::sema::lowering::TypeView::Array { .. }
        );
        if self.graph.lower_cv(facts.ty) & 2 != 0 && !class_or_array {
            return self.discard(id);
        }
        let writes = matches!(
            self.tree.expressions.get(id).kind,
            ExprKind::Binary {
                assignment: true,
                ..
            }
        ) || matches!(self.tree.expressions.get(id).kind,
                ExprKind::Unary { op, .. } if matches!(op.token.as_punctuator(), Some(P::Increment | P::Decrement)));
        let named = matches!(self.tree.expressions.get(id).kind, ExprKind::Id(_));
        if !named
            && facts.is_place
            && facts.folded.is_none()
            && !writes
            && !class_or_array
            && !matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Function { .. }
            )
        {
            // The course reads nonvolatile scalar glvalues cast to void,
            // while ordinary discarded indirections only evaluate the place.
            self.expression(id, Context::Operand)?;
            Ok(())
        } else if named
            && self.graph.lower_lvalue(id)?
            && facts.folded.is_none()
            && !facts.is_bit_field()
            && matches!(self.tree.expressions.get(id).kind, ExprKind::Id(name) if self.graph.lower_named_object(name)?)
        {
            if class_or_array {
                self.address(id)?;
            } else {
                self.expression(id, Context::Operand)?;
            }
            Ok(())
        } else {
            self.discard(id)
        }
    }
}
