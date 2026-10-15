// Translated from lower/program_lowering.cpp::resolve_singleton_roles and
// lower/function_lowering.cpp::lower_forwarder. See NOTICE.
//! Demanded runtime symbols share singleton roles. Secondary declarations become
//! function-owned forwarders; typed symbol records retain no semantic owners.
use super::{Error, definition_order::Definitions};
use crate::lowir::{
    builder::FunctionBuilder,
    model::{Call, InstructionKind as I, Metadata, Program, SymbolId, Value},
    vocabulary::{Binding, Role, Type},
};
pub(super) struct RoleSymbol {
    pub symbol: SymbolId,
    pub role: Role,
    pub origin: Origin,
}

// Runtime declarations are already published, unlike ordinary pending callees.
// They participate in ownership ranking but cannot become forwarding bodies.
pub(super) enum Origin {
    Primary,
    Secondary,
    Runtime,
}

pub(super) fn resolve(
    program: &mut Program,
    definitions: &mut Definitions,
    symbols: &[RoleSymbol],
) -> Result<(), Error> {
    for role in [Role::AllocateMemory, Role::FreeMemory, Role::Terminate] {
        let mut owner = None;
        let mut rank = 0;
        for symbol in symbols.iter().filter(|symbol| symbol.role == role) {
            let Some(function) = program.function(program.symbol(symbol.symbol)) else {
                continue;
            };
            if function.metadata.role != Some(role) {
                continue;
            }
            let candidate = 2 * u8::from(program.function_defined(symbol.symbol))
                + u8::from(!matches!(symbol.origin, Origin::Secondary));
            if owner.is_none() || candidate > rank {
                owner = Some(symbol.symbol);
                rank = candidate;
            }
        }
        let Some(owner) = owner else {
            continue;
        };
        for symbol in symbols.iter().filter(|symbol| symbol.role == role) {
            if symbol.symbol == owner
                || matches!(symbol.origin, Origin::Runtime)
                || program.function_defined(symbol.symbol)
            {
                continue;
            }
            let Some(function) = program.function(program.symbol(symbol.symbol)) else {
                continue;
            };
            if function.metadata.role != Some(role) {
                continue;
            }
            let signature = program.signature(function.signature);
            let result = signature.result;
            let boundary = crate::lowir::model::Boundary {
                arity: signature.boundary.arity,
                effects: signature.boundary.effects,
                unwind: signature.boundary.unwind,
                returns: signature.boundary.returns,
                result_classes: signature.boundary.result_classes,
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
            let metadata = function.metadata;
            let mut builder = FunctionBuilder::new(program, &name, &names, result)?;
            builder.metadata(Metadata {
                role: None,
                object: None,
                binding: Some(Binding::Internal),
                ..metadata
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
