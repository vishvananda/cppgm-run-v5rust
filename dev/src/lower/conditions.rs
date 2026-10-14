// Translated from lower/condition_lowering.cpp and statement_lowering.cpp. See NOTICE.
//! Scalar truth preserves source branch sense; logical values use short circuit slots.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    lowir::{
        model::{BlockId, Facts, InstructionKind as I},
        vocabulary::{Compare, Type},
    },
    syntax::ast::{self, Condition, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn conditional_place(
        &mut self,
        id: ast::ExprId,
        condition: ast::ExprId,
        then_expr: ast::ExprId,
        else_expr: ast::ExprId,
    ) -> Result<crate::lowir::model::Value, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        self.next_slot = self
            .next_slot
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        let slot = self
            .builder
            .slot(&format!("condaddr__{}", self.next_slot), Type::Ptr)?;
        let then_block = self.new_block("condaddr_then")?;
        let else_block = self.new_block("condaddr_else")?;
        let end = self.new_block("condaddr_end")?;
        let condition = self.truth(condition)?;
        self.effect(I::Branch(condition.value, then_block, else_block))?;
        for (block, expr) in [(then_block, then_expr), (else_block, else_expr)] {
            self.place(block)?;
            let place = self.scalar_place(expr)?;
            let address = if matches!(
                place,
                crate::lowir::model::Value::Slot(_) | crate::lowir::model::Value::Symbol(_)
            ) {
                self.emit(I::Addr(place), source)?.value
            } else {
                place
            };
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::Ptr,
                address,
                slot,
            ))?;
            self.effect(I::Jump(end))?;
        }
        self.place(end)?;
        Ok(self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)?
            .value)
    }
    fn without_parentheses(&self, mut id: ast::ExprId) -> ast::ExprId {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        id
    }
    fn literal_truth(&self, id: ast::ExprId) -> Result<Option<bool>, Error> {
        let id = self.without_parentheses(id);
        let facts = self.graph.lower_expression(id)?;
        match self.tree.expressions.get(id).kind {
            ExprKind::Keyword(Keyword::True) => Ok(Some(true)),
            ExprKind::Keyword(Keyword::False) => Ok(Some(false)),
            ExprKind::Id(_) if facts.folded.is_some() => Ok(facts.folded.map(|v| v != 0)),
            ExprKind::Literal(literal)
                if types::storage(self.graph, facts.ty)?.float_rank() == 0 =>
            {
                if let PostTokenKind::Literal { data, .. } | PostTokenKind::Character { data, .. } =
                    literal.kind
                {
                    Ok(Some(data.as_slice().iter().any(|&b| b != 0)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    pub(super) fn truth(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let id = self.without_parentheses(id);
        let value = self.expression(id, Context::Operand)?;
        self.scalar_truth(value)
    }
    fn scalar_truth(&mut self, value: RValue) -> Result<RValue, Error> {
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
    pub(super) fn condition_value(
        &mut self,
        condition: Condition,
        to_bool: bool,
    ) -> Result<RValue, Error> {
        match condition {
            Condition::Expression(id) => {
                if to_bool {
                    self.truth(id)
                } else {
                    self.operand(id, Context::Operand)
                }
            }
            Condition::Declaration {
                declarator,
                initializer,
                ..
            } => {
                self.local_variable(declarator, Some(initializer))?;
                let entity = self.graph.lower_declared(declarator)?;
                let source = self.graph.lower_entity_type(entity)?;
                let ty = types::storage(self.graph, source)?;
                let place = *self.locals.get(&entity).ok_or(Error::Semantic(
                    crate::sema::Error::Invariant("condition without storage"),
                ))?;
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
        }
    }
    pub(super) fn condition_branch(
        &mut self,
        condition: Condition,
        yes: BlockId,
        no: BlockId,
        fold: bool,
    ) -> Result<(), Error> {
        if let Condition::Expression(id) = condition {
            self.branch_condition(id, yes, no, fold)
        } else {
            let value = self.condition_value(condition, true)?;
            self.effect(I::Branch(value.value, yes, no))?;
            self.terminated = true;
            Ok(())
        }
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
            if !matches!(op, Some(P::LogicalAnd | P::LogicalOr)) {
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
            self.jump(if value { on_true } else { on_false })?;
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
        let ty = if value.ty == Type::Ptr {
            Type::Ptr
        } else {
            Type::I64
        };
        let zero = self.builder.integer(0)?;
        self.emit(I::Compare(Compare::Ne, ty, value.value, zero), value.source)
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
            if !matches!(op, Some(P::LogicalAnd | P::LogicalOr)) {
                break;
            }
            spine.push((node, left, right, op == Some(P::LogicalAnd)));
            node = self.without_parentheses(left);
        }
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
            self.next_slot = self
                .next_slot
                .checked_add(1)
                .ok_or(crate::lowir::Error::Limit)?;
            let prefix = if conjunction { "land" } else { "lor" };
            let slot = self
                .builder
                .slot(&format!("{prefix}__{}", self.next_slot), Type::I64)?;
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
            let value = self.normalized_truth(right)?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                value.value,
                slot,
            ))?;
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
    pub(super) fn discard(&mut self, id: ast::ExprId) -> Result<(), Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        match self.tree.expressions.get(id).kind {
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } if types::storage(self.graph, self.graph.lower_expression(id)?.ty)? == Type::Void => {
                let yes = self.new_block("discard_cond_then")?;
                let no = self.new_block("discard_cond_else")?;
                let end = self.new_block("discard_cond_end")?;
                let value = self.truth(condition)?;
                self.effect(I::Branch(value.value, yes, no))?;
                self.terminated = true;
                for (block, operand) in [(yes, then_expr), (no, else_expr)] {
                    self.place(block)?;
                    self.discard(operand)?;
                    if !self.terminated {
                        self.jump(end)?;
                    }
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
                if types::storage(self.graph, self.graph.lower_expression(id)?.ty)?
                    == Type::Void =>
            {
                self.discard_to_void(operand)
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
            ExprKind::Literal(_)
            | ExprKind::Keyword(_)
            | ExprKind::TypeTrait {
                keyword: Keyword::Alignof,
                ..
            } => Ok(()),
            _ => {
                self.expression(id, Context::Operand)?;
                Ok(())
            }
        }
    }
    pub(super) fn discard_to_void(&mut self, id: ast::ExprId) -> Result<(), Error> {
        let id = self.without_parentheses(id);
        let facts = self.graph.lower_expression(id)?;
        let writes = matches!(
            self.tree.expressions.get(id).kind,
            ExprKind::Binary {
                assignment: true,
                ..
            }
        ) || matches!(self.tree.expressions.get(id).kind,
                ExprKind::Unary { op, .. } if matches!(op.token.as_punctuator(), Some(P::Increment | P::Decrement)));
        if facts.is_place
            && facts.folded.is_none()
            && self.graph.lower_cv(facts.ty) & 2 == 0
            && !writes
            && !matches!(
                self.graph.lower_type(facts.ty)?,
                crate::sema::lowering::TypeView::Function { .. }
            )
        {
            // The course reads nonvolatile scalar glvalues cast to void,
            // while ordinary discarded indirections only evaluate the place.
            self.expression(id, Context::Operand)?;
            Ok(())
        } else {
            self.discard(id)
        }
    }
}
