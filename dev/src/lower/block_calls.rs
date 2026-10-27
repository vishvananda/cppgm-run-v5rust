// Translated from lower/expression_lowering.cpp::block_call. See NOTICE.
//! Clang block literals pass their address before the source arguments.
use super::{Error, conversions::Context, function::Function};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, SignatureId, Value},
        vocabulary::{Binary, Projection, Type},
    },
    sema::lowering::TypeView,
    syntax::ast,
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn block_literal(&mut self, callee: ast::ExprId) -> Result<Option<Value>, Error> {
        let source = self
            .graph
            .lower_expression(self.without_parentheses(callee))?
            .ty;
        if matches!(self.graph.lower_type(source)?, TypeView::BlockPointer(_)) {
            Ok(Some(self.operand(callee, Context::Operand)?.value))
        } else {
            Ok(None)
        }
    }

    pub(super) fn block_invoke(
        &mut self,
        callee: ast::ExprId,
        literal: Value,
    ) -> Result<(Value, Option<SignatureId>), Error> {
        let source = self
            .graph
            .lower_expression(self.without_parentheses(callee))?
            .ty;
        // isa pointer, flags and reserved words precede the invoke pointer.
        let element = self.builder.integer(2)?;
        let stride = self.builder.integer(8)?;
        let offset = self
            .emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, element, stride),
                source,
            )?
            .value;
        let slot = self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), literal, offset),
                source,
            )?
            .value;
        let invoke_type = self.graph.lower_block_invocation(source)?;
        let signature = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .call_signature(self.builder.program(), invoke_type, false)?;
        let invoke = self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)?
            .value;
        Ok((invoke, Some(signature)))
    }
}
