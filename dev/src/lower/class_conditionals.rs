// Translated from lower/object_lowering.cpp::materialize_prvalue/conditional_operand.
// See NOTICE.
//! Selected branches initialize one destination from their recorded copy actions.
use super::{Error, function::Function};
use crate::{
    lowir::model::{InstructionKind as I, RuntimeInstruction as R, Value},
    sema::lowering::TypeId,
    syntax::ast,
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn materialize_conditional(
        &mut self,
        condition: ast::ExprId,
        operands: [ast::ExprId; 2],
        ty: TypeId,
        object: Value,
        volatile: bool,
    ) -> Result<(), Error> {
        let then_block = self.new_block("condobj_then")?;
        let else_block = self.new_block("condobj_else")?;
        let end = self.new_block("condobj_end")?;
        let guard_start = self.lifetime.guard_start;
        if guard_start.is_none() {
            self.lifetime.guard_start = Some(self.builder.position()?);
        }
        let condition = self.truth(condition)?;
        self.effect(I::Branch(condition.value, then_block, else_block))?;
        let opened = self.lifetime.regions_opened;
        for (index, (block, operand)) in [then_block, else_block]
            .into_iter()
            .zip(operands)
            .enumerate()
        {
            self.place(block)?;
            let reopened = index == 1
                && self.lifetime.tracking.active
                && self.lifetime.tracking.construction.active()
                && self.lifetime.full_expression
                && !self.lifetime.region
                && self.handlers.explicit == 0
                && self.lifetime.regions_opened != opened;
            if reopened {
                // materialize_prvalue's second arm protects only the objects
                // below the tracked expression, before its member addresses.
                let floor = self.lifetime.tracking.floor;
                let entry = self.cleanup_prefix(true, floor)?;
                self.effect(I::Runtime(R::EhTry(entry)))?;
                self.lifetime.region = true;
                self.lifetime.region_depth = floor;
                self.lifetime.region_entry = Some(entry);
                self.aggregation.reopened = true;
            }
            self.lifetime.guarded_operands += 1;
            let result = if matches!(
                self.tree
                    .expressions
                    .get(self.without_parentheses(operand))
                    .kind,
                ast::ExprKind::Throw(_)
            ) {
                self.discard(operand)
            } else {
                self.materialize_class_qualified(operand, ty, object, volatile)
            };
            self.lifetime.guarded_operands -= 1;
            self.aggregation.reopened = false;
            result?;
            if !self.terminated {
                // A segment ends at this branch edge. Tracked regions own
                // their cross-branch continuation separately.
                if !self.lifetime.tracking.active {
                    self.close_lifetime_region()?;
                }
                self.jump(end)?;
            }
        }
        if guard_start.is_none() {
            self.lifetime.guard_start = None;
        }
        self.place(end)
    }
}
