// Translated from lower/expression_lowering.cpp::literal. See NOTICE.
//! Lower selected literal calls with their original target argument order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        literals::{Constant, FloatKind},
        model::{Call, InstructionKind as I, Value},
        vocabulary::{Convert, Type},
    },
    sema::lowering::{CallTarget, LiteralCall, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn literal_call(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        self.literal_call_into(id, None)
    }
    pub(super) fn literal_call_into(
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
            LiteralCall::Template => (None, 0),
            LiteralCall::Character { source, bits } => {
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
                (Some([value.value; 2]), 1)
            }
            LiteralCall::Integer { source, bits } => {
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
                (Some([value.value; 2]), 1)
            }
            LiteralCall::Floating { digits } => {
                let bytes = self.symbols.numeric_literal_value(literal, digits)?.ok_or(
                    crate::sema::Error::at(
                        self.tree.expressions.get(id).location,
                        "a floating literal outside its type's range",
                    ),
                )?;
                let mut bits = [0; 16];
                bits[..bytes.as_slice().len()].copy_from_slice(bytes.as_slice());
                let value = self
                    .builder
                    .constant(Constant::Float(FloatKind::F80, u128::from_le_bytes(bits)))?;
                (Some([value; 2]), 1)
            }
            LiteralCall::Raw { .. } => {
                let symbol = super::data::literal_object(
                    self.builder.program(),
                    self.symbols,
                    self.graph,
                    self.tree,
                    id,
                )?;
                let address = self.emit(I::Addr(Value::Symbol(symbol)), source)?.value;
                (Some([address; 2]), 1)
            }
            LiteralCall::String { units, .. } => {
                let symbol = super::data::literal_object(
                    self.builder.program(),
                    self.symbols,
                    self.graph,
                    self.tree,
                    id,
                )?;
                let address = self.emit(I::Addr(Value::Symbol(symbol)), source)?.value;
                let length = self.builder.integer(u128::from(units - 1))?;
                let length = self
                    .emit(
                        I::Convert(Convert::SignExtend, Type::I64, Type::I32, length),
                        source,
                    )?
                    .value;
                (Some([address, length]), 2)
            }
        };
        self.symbols.demand(entity);
        let callee = self.symbols.callee(entity, &mut self.builder)?;
        let boundary = self.symbols.result_boundary(result)?;
        let mut arguments = [callee; 3];
        let prefix = usize::from(boundary.indirect);
        if let Some(values) = values {
            arguments[prefix..prefix + count].copy_from_slice(&values[..count]);
        }
        if boundary.indirect {
            arguments[0] = destination.ok_or(crate::sema::Error::Invariant(
                "literal result without destination",
            ))?;
        }
        let arguments = self.builder.arguments(&arguments[..count + prefix])?;
        let call = I::Call(Call {
            result: boundary.ty,
            callee,
            arguments,
            copy_elision: false,
            argument_classes: Vec::new(),
            signature: None,
        });
        self.before_lifetime_call(throws)?;
        self.finish_result_call(call, source, destination, throws)
    }
}
