// Translated from lower/expression_lowering.cpp::operator_call. See NOTICE.
//! Selected operator calls consume original operands and retained base projections.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::model::{Call, InstructionKind as I},
    sema::lowering::{CallTarget, TypeView},
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn operator_call(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let id = self.without_parentheses(id);
        let source = self.graph.lower_expression(id)?.ty;
        let CallTarget::Direct(entity) = self.graph.lower_call(id)? else {
            return Err(Error::Missing("indirect operator call"));
        };
        let throws = self.lifetime_call_throws(CallTarget::Direct(entity))?;
        self.begin_lifetime_call(throws)?;
        let (operands, count, postfix) = match self.tree.expressions.get(id).kind {
            ExprKind::Binary { left, right, .. } => ([left, right], 2, false),
            ExprKind::Subscript { base, index } => ([base, index], 2, false),
            ExprKind::Unary { operand, .. } => ([operand, operand], 1, false),
            ExprKind::Postfix { operand, .. } => ([operand, operand], 1, true),
            _ => return Err(Error::Missing("operator call syntax")),
        };
        let operands = &operands[..count];
        let function = self.graph.lower_function(entity)?;
        let TypeView::Function {
            result, parameters, ..
        } = self.graph.lower_type(function.ty)?
        else {
            return Err(Error::Missing("operator signature"));
        };
        let ty = types::storage(self.graph, result)?;
        let member = function.object.is_some();
        let first = if member {
            let object = operands[0];
            let mut address = self.address(object)?;
            for offset in self
                .graph
                .lower_receiver_base_offsets(object)?
                .into_iter()
                .flatten()
            {
                address = self.base_projection(address, source, offset)?;
            }
            address
        } else {
            self.operator_argument(operands[0], parameters[0])?
        };
        // Unary and binary operators, including a postfix dummy int, carry
        // at most two lowered values. Nested calls cannot disturb this view.
        let mut values = [first; 2];
        if operands.len() == 2 {
            values[1] = self.operator_argument(operands[1], parameters[usize::from(!member)])?;
        }
        if postfix {
            values[1] = self.builder.integer(0)?;
        }
        self.symbols.demand(entity);
        let callee = self.symbols.callee(entity, &mut self.builder)?;
        let arguments = self
            .builder
            .arguments(&values[..count + usize::from(postfix)])?;
        let call = I::Call(Call {
            result: ty,
            callee,
            arguments,
            copy_elision: false,
            signature: None,
        });
        self.before_lifetime_call(throws)?;
        let value = self.lifetime_result(call, source)?;
        self.finish_lifetime_call(value, throws)
    }
    fn operator_argument(
        &mut self,
        operand: ast::ExprId,
        parameter: crate::sema::lowering::TypeId,
    ) -> Result<crate::lowir::model::Value, Error> {
        if matches!(
            self.graph.lower_type(parameter)?,
            TypeView::Reference { .. }
        ) {
            self.bind_reference(operand, parameter, Context::Argument)
        } else if matches!(self.graph.lower_type(parameter)?, TypeView::Class(_)) {
            self.class_argument(operand, parameter)
        } else {
            let value = self.operand(operand, Context::Argument)?;
            Ok(self.fit(value, parameter)?.value)
        }
    }
}
