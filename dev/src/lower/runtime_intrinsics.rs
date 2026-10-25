// Translated from lower/intrinsic_lowering.cpp and expression_lowering.cpp. See NOTICE.
//! Expand hosted hints and runtime queries without changing backend policy.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{Call, Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Type},
    },
    sema::lowering::{RuntimeIntrinsic, TypeId},
    syntax::{ast, storage::List},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeFunction {
    RoundingMode,
    Trap,
    Memset,
}
impl RuntimeFunction {
    pub(super) fn spelling(self) -> (&'static str, &'static str) {
        match self {
            Self::Trap => ("__cppgm_builtin_trap", "cppgm_builtin_trap"),
            Self::RoundingMode => ("__cppgm_libc_fegetround", "fegetround"),
            Self::Memset => ("__cppgm_builtin_memset", "cppgm_builtin_memset"),
        }
    }

    pub(super) fn index(self) -> usize {
        match self {
            Self::RoundingMode => 0,
            Self::Trap => 1,
            Self::Memset => 2,
        }
    }
    pub(super) fn result(self) -> Type {
        match self {
            Self::RoundingMode => Type::I32,
            Self::Trap => Type::Void,
            Self::Memset => Type::Ptr,
        }
    }
}

// Signatures and object spellings belong to the intrinsic owner. Symbols only
// allocates and memoizes the program-owned identity, shared by every function.
pub(super) fn declare_runtime(
    program: &mut crate::lowir::model::Program,
    symbol: crate::lowir::model::SymbolId,
    function: RuntimeFunction,
) -> Result<(), Error> {
    use crate::lowir::{
        model::{Boundary, Linkage, Metadata, Parameter, Returns, Signature, Unwind},
        vocabulary::Binding,
    };
    let mut parameters = Vec::new();
    if function == RuntimeFunction::Memset {
        for (index, ty) in [Type::Ptr, Type::I32, Type::I64].into_iter().enumerate() {
            parameters.push(Parameter {
                name: program.names.intern(&format!("arg{index}"))?,
                ty,
                pass: None,
                object_bytes: 0,
                no_alias: false,
                classes: None,
            });
        }
    }
    let signature = program.add_signature(Signature {
        parameters,
        result: function.result(),
        boundary: match function {
            RuntimeFunction::Trap => Boundary {
                unwind: Unwind::No,
                returns: Returns::NoReturn,
                ..Boundary::default()
            },
            RuntimeFunction::Memset => Boundary {
                unwind: Unwind::No,
                ..Boundary::default()
            },
            RuntimeFunction::RoundingMode => Boundary::default(),
        },
    })?;
    let object = program.names.intern(function.spelling().1)?;
    program.add(
        symbol,
        crate::lowir::model::Function {
            name: program.symbol(symbol).name,
            signature,
            metadata: Metadata {
                linkage: if function == RuntimeFunction::RoundingMode {
                    Linkage::C
                } else {
                    Linkage::Default
                },
                binding: Some(Binding::Strong),
                object: Some(object),
                ..Metadata::default()
            },
            body: None,
            location: None,
        },
    )?;
    Ok(())
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn runtime_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        operation: RuntimeIntrinsic,
    ) -> Result<RValue, Error> {
        let argument_list = arguments;
        let source = self.graph.lower_expression(id)?.ty;
        let arguments = self.tree.expr_lists.get(arguments);
        match operation {
            RuntimeIntrinsic::AddressOf => Ok(RValue {
                value: self.address(arguments[0])?,
                ty: Type::Ptr,
                source,
                literal: false,
                bits: None,
            }),
            RuntimeIntrinsic::Overflow(kind) => self.overflow_intrinsic(id, argument_list, kind),
            RuntimeIntrinsic::VectorReduction(kind) => self.reduce_vector(id, arguments[0], kind),
            RuntimeIntrinsic::VectorLane(builtin) => self.vector_builtin_lanes(
                id,
                arguments,
                builtin.operation().kind,
                builtin.selector(),
            ),
            RuntimeIntrinsic::ObjectSize { minimum } => {
                self.runtime_integer(Type::I64, if minimum { 0 } else { u64::MAX }, source)
            }
            RuntimeIntrinsic::Prefetch => {
                for &argument in arguments {
                    self.discard(argument)?;
                }
                self.zero(source)
            }
            RuntimeIntrinsic::AssumeAligned => {
                let value = self.operand(arguments[0], Context::Argument)?;
                for &argument in &arguments[1..] {
                    self.discard(argument)?;
                }
                Ok(value)
            }
            RuntimeIntrinsic::FltRounds => {
                let mode = self.hosted_runtime_call(RuntimeFunction::RoundingMode, source)?;
                let nine = self.runtime_integer(Type::I32, 9, source)?;
                let shifted = self.runtime_binary(Binary::Shr, mode, nine, source)?;
                let six = self.runtime_integer(Type::I32, 6, source)?;
                let shift = self.runtime_binary(Binary::And, shifted, six, source)?;
                let mapping = self.runtime_integer(Type::I32, 45, source)?;
                let mapped = self.runtime_binary(Binary::Shr, mapping, shift, source)?;
                let mask = self.runtime_integer(Type::I32, 3, source)?;
                self.runtime_binary(Binary::And, mapped, mask, source)
            }
        }
    }
    pub(super) fn hosted_runtime_call(
        &mut self,
        function: RuntimeFunction,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let callee = self
            .symbols
            .hosted_runtime_function(self.builder.program(), function)?;
        let arguments = self.builder.arguments(&[])?;
        let call = I::Call(Call {
            result: function.result(),
            callee: Value::Symbol(callee),
            arguments,
            signature: None,
            copy_elision: false,
            argument_classes: Vec::new(),
        });
        if function.result() == Type::Void {
            self.effect(call)?;
            self.zero(source)
        } else {
            self.emit(call, source)
        }
    }
    pub(super) fn runtime_integer(
        &mut self,
        ty: Type,
        bits: u64,
        source: TypeId,
    ) -> Result<RValue, Error> {
        // FunctionLowering::integer builds an IntLiteral from int64_t even
        // for an unsigned source result. Untyped call operands expose its sign.
        let literal =
            u128::from_le_bytes(i128::from(i64::from_le_bytes(bits.to_le_bytes())).to_le_bytes());
        Ok(RValue {
            value: self.builder.integer(literal)?,
            ty,
            source,
            literal: true,
            bits: Some(bits),
        })
    }
    fn runtime_binary(
        &mut self,
        operation: Binary,
        left: RValue,
        right: RValue,
        source: TypeId,
    ) -> Result<RValue, Error> {
        self.emit(
            I::Binary(
                operation,
                Facts::default(),
                left.ty,
                left.value,
                right.value,
            ),
            source,
        )
    }
}
