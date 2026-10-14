// Translated from lower/expression_lowering.cpp::to_base/adjusted_pointer. See NOTICE.
//! Mechanical projections consume semantic paths and ready layout offsets.
use super::{
    Error,
    function::{Function, RValue},
};
use crate::{
    lex::{pp_lexer::Punctuator, vocabulary::Keyword},
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Compare, Type},
    },
    sema::lowering::TypeId,
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn nonnull_pointer(&self, id: ast::ExprId) -> bool {
        match self.tree.expressions.get(self.without_parentheses(id)).kind {
            ExprKind::Keyword(Keyword::This) => true,
            ExprKind::Unary { op, .. } => op.token.as_punctuator() == Some(Punctuator::BitAnd),
            _ => false,
        }
    }
    pub(super) fn base_projection(
        &mut self,
        object: Value,
        source: TypeId,
        offset: u64,
    ) -> Result<Value, Error> {
        let offset = self.builder.integer(u128::from_le_bytes(
            i128::from(i64::from_le_bytes(offset.to_le_bytes())).to_le_bytes(),
        ))?;
        Ok(self
            .emit(I::Index(Type::I8, None, object, offset), source)?
            .value)
    }
    pub(super) fn member_base_projection(
        &mut self,
        id: ast::ExprId,
        mut object: Value,
        source: TypeId,
    ) -> Result<Value, Error> {
        for offset in self
            .graph
            .lower_member_base_offsets(id)?
            .into_iter()
            .flatten()
        {
            object = self.base_projection(object, source, offset)?;
        }
        Ok(object)
    }
    pub(super) fn adjust_base_pointer(
        &mut self,
        value: RValue,
        target: TypeId,
        delta: u64,
        null_test: bool,
    ) -> Result<RValue, Error> {
        if !null_test {
            return Ok(RValue {
                value: self.base_projection(value.value, target, delta)?,
                source: target,
                bits: None,
                literal: false,
                ..value
            });
        }
        let slot = self.generated_slot("basecast", Type::Ptr)?;
        let null_block = self.new_block("basecast_null")?;
        let adjust = self.new_block("basecast_adjust")?;
        let end = self.new_block("basecast_end")?;
        let zero = self.builder.integer(0)?;
        let condition = self.emit(
            I::Compare(Compare::Eq, Type::Ptr, value.value, zero),
            target,
        )?;
        self.effect(I::Branch(condition.value, null_block, adjust))?;
        self.terminated = true;
        self.place(null_block)?;
        self.effect(I::Store(Facts::memory(false, true), Type::Ptr, zero, slot))?;
        self.jump(end)?;
        self.place(adjust)?;
        let adjusted = self.base_projection(value.value, target, delta)?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            adjusted,
            slot,
        ))?;
        self.jump(end)?;
        self.place(end)?;
        self.read_place(slot, target)
    }
}
