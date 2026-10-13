//! Typed construction over the same private stores used by the reader. See NOTICE.
use super::{
    Error,
    model::{
        BlockId, Body, Boundary, Function, Instruction, InstructionKind, Metadata, Parameter,
        Program, Signature, SymbolId, Value,
    },
    vocabulary::Type,
};

pub(super) struct FunctionBuilder<'a> {
    program: &'a mut Program,
    symbol: SymbolId,
    function: Function,
    body: Body,
    block: Option<BlockId>,
}

impl<'a> FunctionBuilder<'a> {
    pub(super) fn new(
        program: &'a mut Program,
        name: &str,
        parameters: &[(&str, Type)],
        result: Type,
    ) -> Result<Self, Error> {
        let name = program.names.intern(name)?;
        let source = program.add_source("")?;
        let symbol = program.reference(name, source, 0)?;
        if program.symbol(symbol).function.is_some() {
            return Err(Error::Diagnostic("duplicate constructed function".into()));
        }
        let mut signature = Signature {
            parameters: Vec::new(),
            result,
            boundary: Boundary::default(),
        };
        let mut body = Body::new("", parameters.len());
        for &(text, ty) in parameters {
            let name = program.names.intern(text)?;
            if body.find_temp(name).is_some() {
                return Err(Error::Diagnostic("duplicate constructed parameter".into()));
            }
            signature.parameters.push(Parameter {
                name,
                ty,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            });
            let id = body.use_temp(name, 0)?;
            body.temp_mut(id).ty = Some(ty);
        }
        let signature = program.add_signature(signature)?;
        let function = Function {
            name,
            signature,
            metadata: Metadata::default(),
            body: None,
        };
        Ok(Self {
            program,
            symbol,
            function,
            body,
            block: None,
        })
    }
    pub(super) fn signature(
        &mut self,
        name: &str,
        ty: Type,
        result: Type,
    ) -> Result<super::model::SignatureId, Error> {
        let name = self.program.names.intern(name)?;
        self.program.add_signature(Signature {
            parameters: vec![Parameter {
                name,
                ty,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            }],
            result,
            boundary: Boundary::default(),
        })
    }
    pub(super) fn parameter(&self, index: usize) -> Result<Value, Error> {
        let signature = self.program.signature(self.function.signature);
        let parameter = signature.parameters.get(index).ok_or(Error::Limit)?;
        // Parameters are installed in order; the private lookup returns their typed identity.
        self.body
            .find_temp(parameter.name)
            .map(Value::Temp)
            .ok_or(Error::Limit)
    }
    pub(super) fn integer(&mut self, value: u128) -> Result<Value, Error> {
        self.body.integer(value)
    }
    pub(super) fn arguments(&mut self, values: &[Value]) -> Result<super::model::Arguments, Error> {
        self.body.add_arguments(values)
    }
    pub(super) fn block(&mut self, name: &str) -> Result<(), Error> {
        let name = self.program.names.intern(name)?;
        let id = self.body.use_block(name, 0)?;
        if self.body.block(id).placed {
            return Err(Error::Diagnostic("duplicate constructed block".into()));
        }
        self.body.block_mut(id).placed = true;
        self.body.layout.push(id);
        self.block = Some(id);
        Ok(())
    }
    pub(super) fn value(&mut self, kind: InstructionKind, name: &str) -> Result<Value, Error> {
        let name = self.program.names.intern(name)?;
        let id = self.body.use_temp(name, 0)?;
        if self.body.temp_is_parameter(id) || self.body.temp(id).definition.is_some() {
            return Err(Error::Diagnostic("duplicate constructed temporary".into()));
        }
        self.body.temp_mut(id).ty = Some(kind.result_type());
        self.append(kind, Some(id))?;
        Ok(Value::Temp(id))
    }
    pub(super) fn effect(&mut self, kind: InstructionKind) -> Result<(), Error> {
        self.append(kind, None)
    }
    fn append(
        &mut self,
        kind: InstructionKind,
        result: Option<super::model::TempId>,
    ) -> Result<(), Error> {
        let block = self
            .block
            .ok_or_else(|| Error::Diagnostic("instruction outside constructed block".into()))?;
        let _ = self.body.append(
            block,
            Instruction {
                kind,
                result,
                line: 0,
            },
        )?;
        Ok(())
    }
    pub(super) fn finish(mut self) -> Result<(), Error> {
        self.function.body = Some(self.body);
        self.program.add(self.symbol, self.function)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{exercises, model::InstructionKind as I, validator};

    #[test]
    fn calls_keep_body_owned_argument_slices_and_count_callee_uses() -> Result<(), Error> {
        let mut program = Program::default();
        exercises::build(&mut program, "call")?;
        validator::validate_constructed(&program)?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let calls: Vec<_> = body
            .instructions()
            .iter()
            .filter_map(|inst| match &inst.kind {
                I::Call(call) => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 2);
        let Value::Temp(callee) = calls[0].callee else {
            return Err(Error::Limit);
        };
        assert_eq!(body.temp(callee).uses, 2);
        assert!(
            matches!(body.arguments(calls[1].arguments), [Value::Temp(id)] if body.temp(*id).definition.is_some())
        );
        // Repeated construction owns another body, while the first function's IDs/facts survive.
        exercises::build(&mut program, "sum")?;
        validator::validate_constructed(&program)?;
        let first = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        assert_eq!(first.temp(callee).uses, 2);
        Ok(())
    }

    #[test]
    fn construction_audit_rejects_missing_value_results_and_parameter_collisions()
    -> Result<(), Error> {
        let mut program = Program::default();
        assert!(
            FunctionBuilder::new(
                &mut program,
                "bad_params",
                &[("x", Type::I64), ("x", Type::Ptr)],
                Type::Void
            )
            .is_err()
        );
        let mut program = Program::default();
        let mut b = FunctionBuilder::new(&mut program, "bad_result", &[], Type::I64)?;
        b.block("entry")?;
        let literal = b.integer(1)?;
        b.effect(I::Const(Type::I64, literal))?;
        b.effect(I::Return(Type::I64, Some(literal)))?;
        b.finish()?;
        let error = validator::validate_constructed(&program);
        assert!(
            matches!(error, Err(Error::Diagnostic(message)) if message == "value instruction without its result temporary in function @bad_result")
        );
        Ok(())
    }
}
