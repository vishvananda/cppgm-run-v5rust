// Translated from lower/global_lowering.cpp and program_lowering.cpp. See NOTICE.
//! Static scalar definitions and demanded external declarations retain typed symbol identities.
use super::{Error, symbols::Symbols, types};
use crate::{
    lowir::{
        literals::{Constant, FloatKind},
        model::{Global, GlobalInit, Program},
        vocabulary::Storage,
    },
    sema::{
        Graph,
        lowering::{EntityId, StaticValue, TypeView},
    },
    syntax::ast,
    types::FundamentalType as F,
};
pub(super) fn define(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<(), Error> {
    let facts = graph.lower_variable(entity)?;
    if matches!(graph.lower_type(facts.ty)?, TypeView::Reference) {
        return Err(Error::Missing("global reference storage lowering"));
    }
    let ty = types::storage(graph, facts.ty)?;
    let symbol = symbols.variable_symbol(program, entity)?;
    if program
        .global(program.symbol(symbol))
        .is_some_and(|global| !matches!(global.init, GlobalInit::Declaration))
    {
        return Err(Error::Semantic(crate::sema::Error::Diagnostic {
            location,
            message: "redefinition of a variable across translation units",
        }));
    }
    let mut metadata = symbols.variable_metadata(program, entity, symbol)?;
    let init = if initializer.is_none() {
        GlobalInit::Zero
    } else {
        match facts.initializer.ok_or(Error::Missing(
            "dynamic and aggregate global initialization",
        ))? {
            StaticValue::Integer(bits) => GlobalInit::Constant(Constant::Integer(u128::from(bits))),
            StaticValue::Floating(ty, bits) => GlobalInit::Constant(Constant::Float(
                match ty {
                    F::Float => FloatKind::F32,
                    F::Double => FloatKind::F64,
                    _ => FloatKind::F80,
                },
                bits,
            )),
            StaticValue::Address(entity, offset) => {
                let target = if graph.lower_is_variable(entity) {
                    symbols.variable_symbol(program, entity)?
                } else {
                    symbols.demand(entity);
                    symbols.function_symbol(program, entity)?
                };
                GlobalInit::Address(target, offset)
            }
        }
    };
    if graph.lower_cv(facts.ty) == 1 {
        metadata.storage = Some(Storage::ReadOnly);
    }
    let name = program.symbol(symbol).name;
    program.add_global(
        symbol,
        Global {
            name,
            ty: Some(ty),
            metadata,
            init,
        },
    )?;
    Ok(())
}
pub(super) fn finish(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
) -> Result<(), Error> {
    for index in 0..symbols.globals.len() {
        let entity = symbols.globals[index];
        let symbol = symbols.variable_symbol(program, entity)?;
        if program.symbol(symbol).declared() {
            continue;
        }
        let facts = graph.lower_variable(entity)?;
        if facts.definition.is_some() {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "referenced global definition was not emitted",
            )));
        }
        let metadata = symbols.variable_metadata(program, entity, symbol)?;
        let name = program.symbol(symbol).name;
        let ty = types::storage(graph, facts.ty)?;
        program.add_global(
            symbol,
            Global {
                name,
                ty: Some(ty),
                metadata,
                init: GlobalInit::Declaration,
            },
        )?;
    }
    Ok(())
}
