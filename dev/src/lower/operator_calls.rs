// Translated from lower/expression_lowering.cpp::operator_call. See NOTICE.
//! Selected operator calls consume original operands and retained base projections.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Call, InstructionKind as I},
        vocabulary::Type,
    },
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
        let mut values = Vec::with_capacity(parameters.len() + usize::from(member));
        if member {
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
            values.push(address);
        }
        for (index, &operand) in operands.iter().skip(usize::from(member)).enumerate() {
            let parameter = parameters[index];
            if matches!(
                self.graph.lower_type(parameter)?,
                TypeView::Reference { .. }
            ) {
                values.push(self.bind_reference(operand, parameter, Context::Argument)?);
            } else {
                let value = self.operand(operand, Context::Argument)?;
                values.push(self.fit(value, parameter)?.value);
            }
        }
        if postfix {
            values.push(self.builder.integer(0)?);
        }
        self.symbols.demand(entity);
        let callee = self.symbols.callee(entity, &mut self.builder)?;
        let arguments = self.builder.arguments(&values)?;
        let call = I::Call(Call {
            result: ty,
            callee,
            arguments,
            copy_elision: false,
            signature: None,
        });
        if ty == Type::Void {
            self.effect(call)?;
            self.zero(source)
        } else {
            self.emit(call, source)
        }
    }
}
