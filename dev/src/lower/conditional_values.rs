// Translated from lower/condition_lowering.cpp::conditional_value/choice_operand.
// See NOTICE.
//! Nested value choices retain one lifetime policy and unwind in source block order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    lifetime::ChoiceRegion,
    types,
};
use crate::{
    lowir::{
        model::{BlockId, InstructionKind as I, Value},
        vocabulary::Type,
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

struct Choice {
    then_expr: ast::ExprId,
    else_expr: ast::ExprId,
    source: TypeId,
    storage: Type,
    slot: Value,
    else_block: BlockId,
    end: BlockId,
    nested_then: bool,
    local: bool,
}

impl Function<'_, '_, '_, '_> {
    fn continues_choice(&self, id: ast::ExprId) -> Result<bool, Error> {
        let id = self.without_parentheses(id);
        if !matches!(
            self.tree.expressions.get(id).kind,
            ExprKind::Conditional { .. }
        ) {
            return Ok(false);
        }
        let facts = self.graph.lower_expression(id)?;
        Ok(facts.folded.is_none()
            && self.graph.lower_function_conversion(id)?.is_none()
            && !matches!(
                self.graph.lower_type(facts.ty)?,
                TypeView::Array { .. } | TypeView::Function { .. }
            )
            && types::storage(&*self.graph, facts.ty)? != Type::Void)
    }

    pub(super) fn conditional_value(
        &mut self,
        id: ast::ExprId,
        condition: ast::ExprId,
        then_expr: ast::ExprId,
        else_expr: ast::ExprId,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        if types::storage(&*self.graph, source)? == Type::Void {
            self.discard(id)?;
            return self.zero(source);
        }
        if let Some(selected) = self
            .graph
            .lower_constant_conversion(&*self.tree, condition)?
        {
            let value = self.operand(
                if selected { then_expr } else { else_expr },
                Context::Operand,
            )?;
            return self.fit(value, source);
        }
        let local = self.branch_local(id);
        let guards =
            !local || self.continues_choice(then_expr)? || self.continues_choice(else_expr)?;
        let guard_start = self.lifetime.guard_start;
        if guards && guard_start.is_none() {
            self.lifetime.guard_start = Some(self.builder.position()?);
        }
        let result = self.lower_choices(id, local);
        // Guard insertion advances the shared cursor. Restore only the scope
        // that opened it, including on failure; an enclosing scope keeps it.
        if guards && guard_start.is_none() {
            self.lifetime.guard_start = None;
        }
        result
    }

    fn lower_choices(&mut self, root: ast::ExprId, local: bool) -> Result<RValue, Error> {
        let mut choices = Vec::new();
        let mut current = root;
        loop {
            self.check_stack()?;
            let ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } = self.tree.expressions.get(current).kind
            else {
                return Err(crate::sema::Error::Invariant("choice without conditional").into());
            };
            let source = self.graph.lower_expression(current)?.ty;
            let storage = types::storage(&*self.graph, source)?;
            let slot = self.generated_slot("cond", storage)?;
            let then_block = self.new_block("cond_then")?;
            let else_block = self.new_block("cond_else")?;
            let end = self.new_block("cond_end")?;
            let mut level = Choice {
                then_expr,
                else_expr,
                source,
                storage,
                slot,
                else_block,
                end,
                nested_then: false,
                local,
            };
            // The condition of a continuing level is inside its parent's
            // branch, independently of where that level's operands end.
            self.lifetime.guarded_operands += u32::from(current != root);
            let condition = self.truth(condition);
            self.lifetime.guarded_operands -= u32::from(current != root);
            self.effect(I::Branch(condition?.value, then_block, else_block))?;
            self.terminated = true;
            self.place(then_block)?;
            if self.continues_choice(then_expr)? {
                level.nested_then = true;
                choices.push(level);
                current = self.without_parentheses(then_expr);
                continue;
            }
            let opened = self.lifetime.regions_opened;
            self.choice_operand(then_expr, &level)?;
            if !self.terminated {
                self.jump(end)?;
            }
            self.place(else_block)?;
            if self.continues_choice(else_expr)? {
                choices.push(level);
                current = self.without_parentheses(else_expr);
                continue;
            }
            self.choice_else_operand(&level, opened)?;
            if !self.terminated {
                self.jump(end)?;
            }
            choices.push(level);
            break;
        }
        let inner = choices
            .pop()
            .ok_or(crate::sema::Error::Invariant("empty choice chain"))?;
        self.place(inner.end)?;
        let mut value = self.load_choice(&inner)?;
        let mut result_slot = inner.slot;
        while let Some(outer) = choices.pop() {
            let nested = if outer.nested_then {
                outer.then_expr
            } else {
                outer.else_expr
            };
            let converted = self.apply_conversion(nested, value, Context::Operand)?;
            self.store_choice(converted, &outer)?;
            self.jump(outer.end)?;
            if outer.nested_then {
                self.place(outer.else_block)?;
                self.choice_operand(outer.else_expr, &outer)?;
                self.jump(outer.end)?;
            }
            self.place(outer.end)?;
            value = self.load_choice(&outer)?;
            result_slot = outer.slot;
        }
        self.conditional_slot = Some(result_slot);
        Ok(value)
    }

    fn store_choice(&mut self, value: RValue, level: &Choice) -> Result<(), Error> {
        let value = self.fit(value, level.source)?;
        self.effect(I::Store(
            types::memory(&*self.graph, level.source)?,
            level.storage,
            value.value,
            level.slot,
        ))
    }

    fn load_choice(&mut self, level: &Choice) -> Result<RValue, Error> {
        self.emit(
            I::Load(
                types::memory(&*self.graph, level.source)?,
                level.storage,
                level.slot,
            ),
            level.source,
        )
    }

    fn choice_operand(&mut self, id: ast::ExprId, level: &Choice) -> Result<(), Error> {
        let first = self.lifetime.cleanups.len();
        self.lifetime.guarded_operands += u32::from(!level.local);
        let result = if matches!(
            self.tree.expressions.get(self.without_parentheses(id)).kind,
            ExprKind::Throw(_)
        ) {
            self.discard(id)
        } else {
            self.operand(id, Context::Operand)
                .and_then(|value| self.store_choice(value, level))
        };
        self.lifetime.guarded_operands -= u32::from(!level.local);
        result?;
        if level.local {
            self.destroy_temporaries(first)?;
        }
        Ok(())
    }

    fn choice_else_operand(&mut self, level: &Choice, opened: usize) -> Result<(), Error> {
        let reopened = self.lifetime.full_expression
            && !self.lifetime.region
            && self.lifetime.regions_opened != opened
            && !self.lifetime.tracking.active
            && (!self.lifetime.temporary_segment.allows_eager_arguments()
                || self.expression_throws(level.else_expr)?
                || self.expression_temporaries(level.else_expr)?.made);
        if reopened {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        let outer_choice = self.lifetime.choice_region;
        if reopened {
            self.lifetime.choice_region = ChoiceRegion::Reopened;
        }
        let result = self.choice_operand(level.else_expr, level);
        self.lifetime.choice_region = outer_choice;
        result?;
        if reopened && self.lifetime.region {
            self.close_lifetime_region()?;
        }
        Ok(())
    }
}
