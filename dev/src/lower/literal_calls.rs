// Translated from lower/expression_lowering.cpp::literal. See NOTICE.
//! Lower selected text literal calls with their original target argument order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Call, InstructionKind as I, Value},
        vocabulary::{Convert, Type},
    },
    sema::lowering::{CallTarget, TextCall, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn text_literal_call(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        self.text_literal_call_into(id, None)
    }
    pub(super) fn text_literal_call_into(
        &mut self,
        id: ast::ExprId,
        destination: Option<Value>,
    ) -> Result<RValue, Error> {
        let id = self.without_parentheses(id);
        let ExprKind::Literal(literal) = self.tree.expressions.get(id).kind else {
            return Err(Error::Missing("literal call syntax"));
        };
        let CallTarget::Direct(entity) = self.graph.lower_call(id)? else {
            return Err(Error::Missing("indirect literal call"));
        };
        let source = self.graph.lower_expression(id)?.ty;
        let function = self.graph.lower_function(entity)?;
        let TypeView::Function {
            result, parameters, ..
        } = self.graph.lower_type(function.ty)?
        else {
            return Err(Error::Missing("literal operator signature"));
        };
        let throws = self.lifetime_call_throws(CallTarget::Direct(entity))?;
        self.begin_lifetime_call(throws)?;
        let shape = self
            .graph
            .lower_literal_call(id)?
            .ok_or(Error::Missing("literal operands"))?;
        let (values, count) = match shape {
            TextCall::Character { source, bits } => {
                let value = RValue {
                    value: self.builder.integer(u128::from_le_bytes(
                        i128::from(i64::from_le_bytes(bits.to_le_bytes())).to_le_bytes(),
                    ))?,
                    ty: types::storage(self.graph, source)?,
                    source,
                    literal: false,
                    bits: Some(bits),
                };
                let value = self.convert(value, parameters[0], Context::Initialization)?;
                let mut value = value;
                if let Some(bits) = value.bits
                    && types::signed(self.graph, value.source)?
                {
                    value.value = self.builder.integer(u128::from_le_bytes(
                        i128::from(i64::from_le_bytes(bits.to_le_bytes())).to_le_bytes(),
                    ))?;
                }
                ([value.value; 2], 1)
            }
            TextCall::String { element, units } => {
                let symbol = super::data::string(
                    self.builder.program(),
                    self.symbols,
                    self.graph,
                    literal,
                    element,
                )?;
                let address = self.emit(I::Addr(Value::Symbol(symbol)), source)?.value;
                let length = self.builder.integer(u128::from(units - 1))?;
                let length = self
                    .emit(
                        I::Convert(Convert::SignExtend, Type::I64, Type::I32, length),
                        source,
                    )?
                    .value;
                ([address, length], 2)
            }
        };
        self.symbols.demand(entity);
        let callee = self.symbols.callee(entity, &mut self.builder)?;
        let boundary = self.symbols.result_boundary(result)?;
        let mut arguments = [values[0], values[1], values[1]];
        if boundary.indirect {
            arguments = [
                destination.ok_or(crate::sema::Error::Invariant(
                    "literal result without destination",
                ))?,
                values[0],
                values[1],
            ];
        }
        let arguments = self
            .builder
            .arguments(&arguments[..count + usize::from(boundary.indirect)])?;
        let call = I::Call(Call {
            result: boundary.ty,
            callee,
            arguments,
            copy_elision: false,
            signature: None,
        });
        self.before_lifetime_call(throws)?;
        self.finish_result_call(call, source, destination, throws)
    }
}
