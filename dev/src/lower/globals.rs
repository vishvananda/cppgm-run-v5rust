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
pub(super) struct GlobalOutput<'a> {
    pub program: &'a mut Program,
    pub startup: &'a mut super::startup::Startup,
    pub definitions: &'a mut super::definition_order::Definitions,
}
pub(super) fn lower_declaration<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    declarators: crate::syntax::storage::List<ast::InitDeclarator>,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    for declarator in tree.init_declarators.get(declarators) {
        let entity = graph.lower_declared(declarator.declarator)?;
        if graph.lower_is_variable(entity)
            && graph.lower_variable(entity)?.definition == Some(declarator.declarator)
        {
            let initialization = define(
                output.program,
                symbols,
                graph,
                tree,
                entity,
                declarator.initializer,
                tree.declarators.get(declarator.declarator).location,
            )?;
            if initialization == Initialization::Dynamic {
                output.startup.initialize(
                    output.program,
                    symbols,
                    (graph, tree),
                    entity,
                    declarator.initializer,
                    budget,
                )?;
            }
            if symbols.emit_tls_wrapper(output.program, entity)? {
                output.definitions.stream(output.program, None)?;
            }
        }
    }
    Ok(())
}
#[derive(PartialEq, Eq)]
enum Initialization {
    Static,
    Dynamic,
}
fn define(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    tree: &ast::Tree,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<Initialization, Error> {
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
) -> Result<Initialization, Error> {
    let facts = graph.lower_variable(entity)?;
    symbols.require_trivial_storage(facts.ty)?;
    let mut object = facts.ty;
    while let TypeView::Array { element, .. } = graph.lower_type(object)? {
        object = element;
    }
    let constructed = matches!(graph.lower_type(object)?, TypeView::Class(_))
        && (initializer.is_some() && graph.lower_constructor_arguments(entity).is_some()
            || initializer.is_none() && !graph.lower_trivial_default_construction(entity)?);
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
    let mut dynamic = constructed;
    let mut init = if matches!(graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        if facts.reference_temporary.is_some() {
            GlobalInit::Address(
                data::reference_temporary(program, symbols, graph, tree, entity, false)?,
                0,
            )
        } else if let Some(value) = facts.initializer {
            let (symbol, offset) = data::address(program, symbols, graph, tree, value)?;
            GlobalInit::Address(symbol, offset)
        } else {
            dynamic = true;
            GlobalInit::Zero
        }
    } else if array {
        GlobalInit::Data(
            if !constructed
                && initializer.is_some()
                && let Some(image) = graph.lower_array_image(entity)
            {
                data::items(program, symbols, graph, tree, image)?
            } else {
                dynamic |= initializer.is_some();
                if !constructed && let Some(prefix) = graph.lower_array_image_prefix(entity) {
                    // The C++ constant attempt demands addresses before publishing
                    // the object's zero image, even when a later member is dynamic.
                    let _ = data::items(program, symbols, graph, tree, prefix)?;
                }
                vec![crate::lowir::model::DataItem::Zero(
                    i64::try_from(types::layout(graph, facts.ty)?.0)
                        .map_err(|_| crate::lowir::Error::Limit)?,
                )]
            },
        )
    } else if initializer.is_none() {
        GlobalInit::Zero
    } else {
        match facts.initializer {
            Some(StaticValue::Integer(bits)) => {
                GlobalInit::Constant(Constant::Integer(u128::from(bits)))
            }
            Some(StaticValue::Floating(ty, bits)) => GlobalInit::Constant(Constant::Float(
                match ty {
                    F::Float => FloatKind::F32,
                    F::Double => FloatKind::F64,
                    _ => FloatKind::F80,
                },
                bits,
            )),
            Some(value @ (StaticValue::Literal(..) | StaticValue::Address(..))) => {
                let (symbol, offset) = data::address(program, symbols, graph, tree, value)?;
                GlobalInit::Address(symbol, offset)
            }
            None => {
                dynamic = true;
                GlobalInit::Zero
            }
        }
    };
    if dynamic && facts.thread_local {
        return Err(Error::Missing("dynamic thread-local initialization"));
    }
    if array && let GlobalInit::Data(items) = &init {
        ty = structured_type(graph, facts.ty, items)?;
    }
    align_storage(graph, entity, &mut ty, &mut init)?;
    if !dynamic && !facts.thread_local && symbols.readonly_object(facts.ty)? {
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
        symbols.request_initialization(program)?;
    }
    if matches!(graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        symbols.data.define_reference(entity);
    }
    if !dynamic {
        for &constructor in graph.lower_initializer_constructors(entity).iter().rev() {
            symbols.demand_entry(super::construction::Entry::Constructor(
                constructor,
                super::construction::Variant::Complete,
            ));
        }
    }
    Ok(if dynamic {
        Initialization::Dynamic
    } else {
        Initialization::Static
    })
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
    Ok(())
}
