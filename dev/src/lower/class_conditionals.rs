// Translated from lower/object_lowering.cpp::materialize_prvalue/conditional_operand.
// See NOTICE.
//! Selected branches initialize one destination from their recorded copy actions.
use super::{Error, function::Function};
use crate::{
    lowir::model::{InstructionKind as I, Value},
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
        let condition = self.truth(condition)?;
        self.effect(I::Branch(condition.value, then_block, else_block))?;
        for (block, operand) in [then_block, else_block].into_iter().zip(operands) {
            self.place(block)?;
            self.lifetime.guarded_operands += 1;
            let result = self.materialize_class_qualified(operand, ty, object, volatile);
            self.lifetime.guarded_operands -= 1;
            result?;
            if !self.terminated {
                self.jump(end)?;
            }
        }
        self.place(end)
    }
}
