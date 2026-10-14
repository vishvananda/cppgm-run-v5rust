// Translated from lower/global_lowering.cpp's static data, string_object and constexpr_image. See NOTICE.
//! Typed images become ordered `LowIR` items; unit-local caches share data without sharing local storage.
use super::{Error, symbols::Symbols, types};
use crate::{
    lex::PostTokenKind,
    lowir::{
        literals::{Constant, FloatKind},
        model::{DataItem, Global, GlobalInit, Metadata, Program, SymbolId},
        vocabulary::{Binding, Storage},
    },
    sema::{
        Graph,
        lowering::{StaticItem, StaticValue},
    },
    syntax::ast,
};
use std::collections::HashMap;
#[derive(Default)]
pub(super) struct Objects {
    strings: HashMap<(crate::lowir::vocabulary::Type, Vec<u64>), SymbolId>,
    images: HashMap<Vec<DataItem>, SymbolId>,
    reference_temporaries: HashMap<crate::sema::lowering::EntityId, SymbolId>,
}
fn integer(bits: u64) -> Constant {
    Constant::Integer(u128::from_le_bytes(
        i128::from(i64::from_le_bytes(bits.to_le_bytes())).to_le_bytes(),
    ))
}
pub(super) fn constant(value: StaticValue) -> Result<Constant, Error> {
    Ok(match value {
        StaticValue::Integer(bits) => integer(bits),
        StaticValue::Floating(ty, bits) => Constant::Float(
            match ty {
                crate::types::FundamentalType::Float => FloatKind::F32,
                crate::types::FundamentalType::Double => FloatKind::F64,
                _ => FloatKind::F80,
            },
            bits,
        ),
        _ => {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "address used as scalar data",
            )));
        }
    })
}
pub(super) fn address(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    tree: &ast::Tree,
    value: StaticValue,
) -> Result<(SymbolId, i64), Error> {
    match value {
        StaticValue::Address(entity, offset) => {
            let symbol = if graph.lower_is_variable(entity)
                && graph.lower_variable(entity)?.reference_temporary.is_some()
            {
                reference_temporary(program, symbols, graph, entity)?
            } else if graph.lower_is_variable(entity) {
                symbols.variable_symbol(program, entity)?
            } else {
                symbols.demand(entity);
                symbols.function_symbol(program, entity)?
            };
            Ok((symbol, offset))
        }
        StaticValue::Literal(expr, offset) => {
            let ast::ExprKind::Literal(literal) = tree.expressions.get(expr).kind else {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "string address without literal",
                )));
            };
            let crate::sema::lowering::TypeView::Array { element, .. } =
                graph.lower_type(graph.lower_expression(expr)?.ty)?
            else {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "string address without array",
                )));
            };
            Ok((string(program, symbols, graph, literal, element)?, offset))
        }
        _ => Err(Error::Semantic(crate::sema::Error::Invariant(
            "scalar used as address data",
        ))),
    }
}
pub(super) fn items(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    tree: &ast::Tree,
    image: &[StaticItem],
) -> Result<Vec<DataItem>, Error> {
    let mut out = Vec::with_capacity(image.len());
    for &item in image {
        out.push(match item {
            StaticItem::Zero(bytes) => {
                DataItem::Zero(i64::try_from(bytes).map_err(|_| crate::lowir::Error::Limit)?)
            }
            StaticItem::Scalar(ty, value) => match value {
                StaticValue::Address(..) | StaticValue::Literal(..) => {
                    let (symbol, offset) = address(program, symbols, graph, tree, value)?;
                    DataItem::Address(symbol, offset)
                }
                _ => DataItem::Constant(types::storage(graph, ty)?, constant(value)?),
            },
        });
    }
    Ok(out)
}
pub(super) fn string(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    literal: ast::Literal,
    element: crate::sema::lowering::TypeId,
) -> Result<SymbolId, Error> {
    let PostTokenKind::String { ty, .. } = literal.kind else {
        return Err(Error::Missing("non-string literal object"));
    };
    let storage = types::storage(graph, element)?;
    let units: Vec<_> = symbols
        .literal_bytes(literal)
        .chunks_exact(ty.size())
        .map(|unit| {
            let mut bytes = [0u8; 8];
            bytes[..unit.len()].copy_from_slice(unit);
            u64::from_le_bytes(bytes)
        })
        .collect();
    let key = (storage, units);
    if let Some(&symbol) = symbols.data.strings.get(&key) {
        return Ok(symbol);
    }
    let items = key
        .1
        .iter()
        .map(|&unit| DataItem::Constant(storage, integer(unit)))
        .collect();
    let symbol = symbols.data_symbol(program, None)?;
    define(program, symbol, items)?;
    symbols.data.strings.insert(key, symbol);
    Ok(symbol)
}
pub(super) fn image(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    tree: &ast::Tree,
    image: &[StaticItem],
) -> Result<SymbolId, Error> {
    let items = items(program, symbols, graph, tree, image)?;
    if let Some(&symbol) = symbols.data.images.get(&items) {
        return Ok(symbol);
    }
    let symbol = symbols.data_symbol(program, Some(symbols.data.images.len()))?;
    define(program, symbol, items.clone())?;
    symbols.data.images.insert(items, symbol);
    Ok(symbol)
}
fn define(program: &mut Program, symbol: SymbolId, items: Vec<DataItem>) -> Result<(), Error> {
    let name = program.symbol(symbol).name;
    program.add_global(
        symbol,
        Global {
            name,
            ty: None,
            metadata: Metadata {
                binding: Some(Binding::Internal),
                storage: Some(Storage::ReadOnly),
                ..Metadata::default()
            },
            init: GlobalInit::Data(items),
        },
    )?;
    Ok(())
}

pub(super) fn reference_temporary(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    entity: crate::sema::lowering::EntityId,
) -> Result<SymbolId, Error> {
    if let Some(&symbol) = symbols.data.reference_temporaries.get(&entity) {
        return Ok(symbol);
    }
    let facts = graph.lower_variable(entity)?;
    let ty = facts
        .reference_temporary
        .ok_or(Error::Semantic(crate::sema::Error::Invariant(
            "reference temporary type missing",
        )))?;
    let value = facts
        .initializer
        .ok_or(Error::Missing("dynamic global reference temporary"))?;
    let (symbol, object) = symbols.reference_temporary_symbol(program, entity)?;
    let init = match value {
        StaticValue::Address(..) | StaticValue::Literal(..) => GlobalInit::Zero,
        _ => GlobalInit::Constant(constant(value)?),
    };
    let name = program.symbol(symbol).name;
    program.add_global(
        symbol,
        Global {
            name,
            ty: Some(types::storage(graph, ty)?),
            metadata: Metadata {
                binding: Some(Binding::Internal),
                storage: if graph.lower_cv(ty) == 1 && matches!(init, GlobalInit::Constant(_)) {
                    Some(Storage::ReadOnly)
                } else {
                    None
                },
                object,
                ..Metadata::default()
            },
            init,
        },
    )?;
    symbols.data.reference_temporaries.insert(entity, symbol);
    Ok(symbol)
}
