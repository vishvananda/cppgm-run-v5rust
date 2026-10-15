// Translated from lower/program_lowering.cpp::terminate_helper/terminate_function.
// See NOTICE.
//! Runtime declarations and the demanded terminate body have program-owned symbol identities.
use super::{Error, symbols::Symbols};
use crate::lowir::{
    builder::FunctionBuilder,
    model::{
        Boundary, Call, InstructionKind as I, Linkage, Metadata, Program, Returns, SymbolId,
        Unwind, Value,
    },
    vocabulary::{Binding, Role, Type},
};
#[derive(Clone, Copy)]
pub(super) struct Helpers {
    pub begin_catch: SymbolId,
    pub terminate: SymbolId,
    pub helper: SymbolId,
}
pub(super) fn declare(program: &mut Program, symbols: Helpers) -> Result<(), Error> {
    let mut declaration = |symbol: SymbolId,
                           parameters: &[(&str, Type)],
                           result: Type,
                           role,
                           object: &str,
                           linkage,
                           boundary|
     -> Result<(), Error> {
        let name = program.symbol(symbol).name;
        let object = program.names.intern(object)?;
        let mut params = Vec::new();
        for &(text, ty) in parameters {
            params.push(crate::lowir::model::Parameter {
                name: program.names.intern(text)?,
                ty,
                pass: None,
                object_bytes: 0,
                no_alias: false,
                classes: None,
            });
        }
        let signature = program.add_signature(crate::lowir::model::Signature {
            parameters: params,
            result,
            boundary,
        })?;
        program.add(
            symbol,
            crate::lowir::model::Function {
                name,
                signature,
                metadata: Metadata {
                    role: Some(role),
                    object: Some(object),
                    linkage,
                    binding: Some(Binding::Strong),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        Ok(())
    };
    declaration(
        symbols.begin_catch,
        &[("arg0", Type::Ptr)],
        Type::Ptr,
        Role::EhBeginCatch,
        "__cxa_begin_catch",
        Linkage::C,
        Boundary::default(),
    )?;
    declaration(
        symbols.terminate,
        &[],
        Type::Void,
        Role::Terminate,
        "_ZSt9terminatev",
        Linkage::Default,
        Boundary {
            unwind: Unwind::No,
            returns: Returns::NoReturn,
            ..Boundary::default()
        },
    )?;
    program.publish_runtime_declaration(symbols.begin_catch);
    program.publish_runtime_declaration(symbols.terminate);
    Ok(())
}
pub(super) fn emit_pending(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    definitions: &mut super::definition_order::Definitions,
) -> Result<(), Error> {
    let Some(helpers) = symbols.take_terminate_helper() else {
        return Ok(());
    };
    let name = program
        .names
        .get(program.symbol(helpers.helper).name)
        .to_owned();
    let mut builder = FunctionBuilder::new(
        program,
        &name,
        &[("exception_object", Type::Ptr)],
        Type::Void,
    )?;
    builder.metadata(Metadata {
        binding: Some(Binding::Internal),
        flags: 32,
        ..Metadata::default()
    });
    builder.boundary(Boundary {
        unwind: Unwind::No,
        returns: Returns::NoReturn,
        ..Boundary::default()
    });
    builder.block("entry")?;
    let parameter = builder.parameter(0)?;
    let arguments = builder.arguments(&[parameter])?;
    builder.generated(I::Call(Call {
        result: Type::Ptr,
        callee: Value::Symbol(helpers.begin_catch),
        arguments,
        signature: None,
        copy_elision: false,
    }))?;
    let arguments = builder.arguments(&[])?;
    builder.effect(I::Call(Call {
        result: Type::Void,
        callee: Value::Symbol(helpers.terminate),
        arguments,
        signature: None,
        copy_elision: false,
    }))?;
    builder.effect(I::Return(Type::Void, None))?;
    builder.finish()?;
    definitions.stream(program, None)
}

/// ABI runtime entry demanded only by a potentially invalid array bound.
pub(super) fn declare_bad_array_length(
    program: &mut Program,
    symbol: SymbolId,
) -> Result<(), Error> {
    let name = program.symbol(symbol).name;
    let object = program.names.intern("__cxa_throw_bad_array_new_length")?;
    let signature = program.add_signature(crate::lowir::model::Signature {
        parameters: Vec::new(),
        result: Type::Void,
        boundary: Boundary {
            returns: Returns::NoReturn,
            ..Boundary::default()
        },
    })?;
    program.add(
        symbol,
        crate::lowir::model::Function {
            name,
            signature,
            metadata: Metadata {
                role: Some(Role::BadArrayNewLength),
                object: Some(object),
                linkage: Linkage::C,
                binding: Some(Binding::Strong),
                ..Metadata::default()
            },
            body: None,
            location: None,
        },
    )?;
    program.publish_runtime_declaration(symbol);
    Ok(())
}
