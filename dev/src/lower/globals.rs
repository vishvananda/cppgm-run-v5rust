// Translated from lower/global_lowering.cpp and program_lowering.cpp. See NOTICE.
//! Static scalar definitions and demanded external declarations retain typed symbol identities.
use super::{Error, data, symbols::Symbols, types};
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
    tree: &ast::Tree,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<(), Error> {
    let saved = symbols.replace_temporary_owner(Some((entity, 0)));
    let result = define_storage(program, symbols, graph, tree, entity, initializer, location);
    symbols.replace_temporary_owner(saved);
    result
}
fn define_storage(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    tree: &ast::Tree,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<(), Error> {
    let facts = graph.lower_variable(entity)?;
    symbols.require_trivial_storage(facts.ty)?;
    let mut object = facts.ty;
    while let TypeView::Array { element, .. } = graph.lower_type(object)? {
        object = element;
    }
    if initializer.is_none()
        && matches!(graph.lower_type(object)?, TypeView::Class(_))
        && !graph.lower_trivial_default_construction(entity)?
    {
        return Err(Error::Missing("global class construction lowering"));
    }
    let array = matches!(
        graph.lower_type(facts.ty)?,
        TypeView::Array { .. } | TypeView::Class(_)
    );
    let mut ty = if array {
        None
    } else {
        Some(types::storage(graph, facts.ty)?)
    };
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
    let mut init = if matches!(graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        if facts.reference_temporary.is_some() {
            GlobalInit::Address(
                data::reference_temporary(program, symbols, graph, tree, entity, false)?,
                0,
            )
        } else {
            let value = facts
                .initializer
                .ok_or(Error::Missing("dynamic global reference initialization"))?;
            let (symbol, offset) = data::address(program, symbols, graph, tree, value)?;
            GlobalInit::Address(symbol, offset)
        }
    } else if array {
        GlobalInit::Data(if initializer.is_some() {
            let image = graph
                .lower_array_image(entity)
                .ok_or(Error::Missing("dynamic global array initialization"))?;
            data::items(program, symbols, graph, tree, image)?
        } else {
            vec![crate::lowir::model::DataItem::Zero(
                i64::try_from(types::layout(graph, facts.ty)?.0)
                    .map_err(|_| crate::lowir::Error::Limit)?,
            )]
        })
    } else if initializer.is_none() {
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
            value @ (StaticValue::Literal(..) | StaticValue::Address(..)) => {
                let (symbol, offset) = data::address(program, symbols, graph, tree, value)?;
                GlobalInit::Address(symbol, offset)
            }
        }
    };
    if array && let GlobalInit::Data(items) = &init {
        ty = structured_type(graph, facts.ty, items)?;
    }
    align_storage(graph, entity, &mut ty, &mut init)?;
    if !facts.thread_local && symbols.readonly_object(facts.ty)? {
        metadata.storage = Some(Storage::ReadOnly);
    }
    let name = program.symbol(symbol).name;
    program.add_global(
        symbol,
        Global {
            name,
            ty,
            metadata,
            init,
        },
    )?;
    if initializer.is_none() && graph.lower_empty_initialization_step(entity)? {
        symbols.request_empty_initialization(program)?;
    }
    if matches!(graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        symbols.data.define_reference(entity);
    }
    Ok(())
}
pub(super) fn align_storage(
    graph: &Graph,
    entity: EntityId,
    storage: &mut Option<crate::lowir::vocabulary::Type>,
    init: &mut GlobalInit,
) -> Result<(), Error> {
    use crate::lowir::{model::DataItem, vocabulary::Type};
    let ty = graph.lower_entity_type(entity)?;
    let requested = u64::from(graph.lower_requested_alignment(entity));
    if requested == 0 {
        return Ok(());
    }
    let (size, natural) = types::layout(graph, ty)?;
    if requested <= 1u64 << natural {
        return Ok(());
    }
    if !matches!(init, GlobalInit::Data(_)) {
        let scalar = storage.ok_or(Error::Semantic(crate::sema::Error::Invariant(
            "untyped scalar global",
        )))?;
        let item = match std::mem::replace(init, GlobalInit::Zero) {
            GlobalInit::Zero => {
                DataItem::Zero(i64::try_from(size).map_err(|_| crate::lowir::Error::Limit)?)
            }
            GlobalInit::Constant(value) => DataItem::Constant(scalar, value),
            GlobalInit::Address(symbol, offset) => DataItem::Address(scalar, symbol, offset),
            _ => {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "aligned global without initializer",
                )));
            }
        };
        *init = GlobalInit::Data(vec![item]);
    }
    let GlobalInit::Data(items) = init else {
        return Err(Error::Semantic(crate::sema::Error::Invariant(
            "aligned global without data",
        )));
    };
    let implied = items
        .iter()
        .filter_map(|item| match item {
            DataItem::Constant(ty, _) => Some(types::storage_alignment(*ty)),
            DataItem::Address(..) => Some(3),
            DataItem::Zero(_) => None,
        })
        .max()
        .unwrap_or(0);
    *storage = (requested > 1u64 << implied).then_some(Type::Object {
        bytes: size.wrapping_add(requested - 1) / requested * requested,
        align: u8::try_from(requested.trailing_zeros()).map_err(|_| crate::lowir::Error::Limit)?,
    });
    Ok(())
}
pub(super) fn structured_type(
    graph: &Graph,
    ty: crate::sema::lowering::TypeId,
    items: &[crate::lowir::model::DataItem],
) -> Result<Option<crate::lowir::vocabulary::Type>, Error> {
    let (bytes, align) = types::layout(graph, ty)?;
    let implied = items
        .iter()
        .filter_map(|item| match item {
            crate::lowir::model::DataItem::Zero(_) => None,
            crate::lowir::model::DataItem::Address(ty, ..)
            | crate::lowir::model::DataItem::Constant(ty, _) => Some(types::storage_alignment(*ty)),
        })
        .max()
        .unwrap_or(0);
    Ok((align > implied).then_some(crate::lowir::vocabulary::Type::Object { bytes, align }))
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
        let ty = if graph.lower_complete_object(facts.ty)? {
            Some(types::storage(graph, facts.ty)?)
        } else {
            None
        };
        program.add_global(
            symbol,
            Global {
                name,
                ty,
                metadata,
                init: GlobalInit::Declaration,
            },
        )?;
    }
    if let Some(symbol) = symbols.empty_initialization
        && !program.function_defined(symbol)
    {
        use crate::lowir::{
            builder::FunctionBuilder,
            model::{InstructionKind, Metadata},
            vocabulary::{Binding, Role, Type},
        };
        let name = program.names.get(program.symbol(symbol).name).to_owned();
        let mut builder = FunctionBuilder::new(program, &name, &[], Type::Void)?;
        builder.metadata(Metadata {
            role: Some(Role::Init),
            binding: Some(Binding::Internal),
            ..Metadata::default()
        });
        builder.block("entry")?;
        builder.effect(InstructionKind::Return(Type::Void, None))?;
        builder.finish()?;
    }
    Ok(())
}
