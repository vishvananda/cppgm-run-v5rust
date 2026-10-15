// Translated from lower/program_lowering.cpp::resolve_singleton_roles and
// lower/function_lowering.cpp::lower_forwarder. See NOTICE.
//! Demanded allocation symbols share one runtime role; secondary declarations
//! become function-owned forwarding bodies, preserving scalar replacements.
use super::{Error, definition_order::Definitions};
use crate::{
    lowir::{
        builder::FunctionBuilder,
        model::{Call, InstructionKind as I, Metadata, Program, SymbolId, Value},
        vocabulary::{Binding, Type},
    },
    sema::lowering::AllocationRole,
};
pub(super) struct AllocationSymbol {
    pub symbol: SymbolId,
    pub role: AllocationRole,
    pub array: bool,
}

pub(super) fn resolve(
    program: &mut Program,
    definitions: &mut Definitions,
    allocations: &[AllocationSymbol],
) -> Result<(), Error> {
    for role in [AllocationRole::Allocate, AllocationRole::Free] {
        let expected = match role {
            AllocationRole::Allocate => crate::lowir::vocabulary::Role::AllocateMemory,
            AllocationRole::Free => crate::lowir::vocabulary::Role::FreeMemory,
        };
        let mut owner = None;
        let mut rank = 0;
        for allocation in allocations
            .iter()
            .filter(|allocation| allocation.role == role)
        {
            let Some(function) = program.function(program.symbol(allocation.symbol)) else {
                continue;
            };
            if function.metadata.role != Some(expected) {
                continue;
            }
            let candidate = 2 * u8::from(program.function_defined(allocation.symbol))
                + u8::from(!allocation.array);
            if owner.is_none() || candidate > rank {
                owner = Some(allocation.symbol);
                rank = candidate;
            }
        }
        let Some(owner) = owner else {
            continue;
        };
        for allocation in allocations
            .iter()
            .filter(|allocation| allocation.role == role)
        {
            if allocation.symbol == owner || program.function_defined(allocation.symbol) {
                continue;
            }
            let Some(function) = program.function(program.symbol(allocation.symbol)) else {
                continue;
            };
            if function.metadata.role != Some(expected) {
                continue;
            }
            let signature = program.signature(function.signature);
            let result = signature.result;
            let boundary = crate::lowir::model::Boundary {
                unwind: signature.boundary.unwind,
                ..crate::lowir::model::Boundary::default()
            };
            let params: Vec<_> = signature
                .parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| (format!("__param{index}"), parameter.ty))
                .collect();
            let names: Vec<_> = params
                .iter()
                .map(|(name, ty)| (name.as_str(), *ty))
                .collect();
            let name = program.names.get(function.name).to_owned();
            let mut builder = FunctionBuilder::new(program, &name, &names, result)?;
            builder.metadata(Metadata {
                binding: Some(Binding::Internal),
                ..Metadata::default()
            });
            builder.boundary(boundary);
            builder.block("entry")?;
            let args = (0..params.len())
                .map(|index| builder.parameter(index))
                .collect::<Result<Vec<_>, _>>()?;
            let arguments = builder.arguments(&args)?;
            let call = I::Call(Call {
                result,
                callee: Value::Symbol(owner),
                arguments,
                signature: None,
                copy_elision: false,
            });
            let value = if result == Type::Void {
                builder.effect(call)?;
                None
            } else {
                Some(builder.generated(call)?)
            };
            builder.effect(I::Return(result, value))?;
            builder.finish()?;
            definitions.stream(program, None)?;
        }
    }
    Ok(())
}
