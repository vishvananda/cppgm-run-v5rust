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
    pub finalizers: &'a mut Vec<super::shutdown::Finalizer>,
}
pub(super) fn lower_anonymous_declaration<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    (graph, tree): (&'a Graph, &'a ast::Tree),
    id: ast::DeclId,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    if let Some(entity) = graph.lower_declaration_storage(id) {
        define_variable(
            output,
            symbols,
            (graph, tree),
            entity,
            None,
            tree.declarations.get(id).location,
            budget,
        )?;
    }
    Ok(())
}
pub(super) fn lower_declaration<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    declarators: crate::syntax::storage::List<ast::InitDeclarator>,
    explicit_specialization: bool,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    for declarator in tree.init_declarators.get(declarators) {
        let entity = graph.lower_declared(declarator.declarator)?;
        if graph.lower_is_variable(entity)
            && graph.lower_variable(entity)?.definition == Some(declarator.declarator)
            && graph.lower_variable_arguments(entity).is_none()
        {
            // This syntactic emission rule also covers an ordinary static
            // member of an explicitly specialized class. Its semantic
            // redeclaration state remains independent of storage emission.
            if explicit_specialization
                && graph.lower_static_member(entity)
                && declarator.initializer.is_none()
            {
                symbols.note_declaration_only_global(entity);
                continue;
            }
            let initializer = declarator
                .initializer
                .or(graph.lower_variable(entity)?.in_class_initializer);
            define_variable(
                output,
                symbols,
                (graph, tree),
                entity,
                initializer,
                tree.declarators.get(declarator.declarator).location,
                budget,
            )?;
        }
    }
    Ok(())
}
pub(super) fn define_variable<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    (graph, tree): (&'a Graph, &'a ast::Tree),
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    symbols.check_stack()?;
    let pending = std::mem::take(&mut symbols.instance_variables);
    let previous_variable = symbols.begin_variable_definition(entity);
    let request = symbols.variable_request(entity);
    let owner = symbols.pending.replace_owner(request);
    let result = define_variable_contents(
        output,
        symbols,
        (graph, tree),
        entity,
        initializer,
        location,
        budget,
    );
    symbols.pending.replace_owner(owner);
    symbols.end_variable_definition(entity, previous_variable);
    let nested = std::mem::replace(&mut symbols.instance_variables, pending);
    symbols.instance_variables.extend(nested);
    result
}
fn define_variable_contents<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    (graph, tree): (&'a Graph, &'a ast::Tree),
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    let initialization = define(output, symbols, graph, tree, entity, initializer, location)?;
    let mut thread_initialization = None;
    let mut constructed = None;
    if initialization == Initialization::Dynamic && graph.lower_variable(entity)?.thread_local {
        thread_initialization = Some(super::thread_local::initialize(
            output,
            symbols,
            (graph, tree),
            entity,
            initializer,
            budget,
        )?);
    } else if initialization == Initialization::Dynamic {
        let guard = if graph.lower_variable(entity)?.emission
            == crate::sema::lowering::Emission::Demanded
        {
            if symbols
                .storage_destructor(graph.lower_entity_type(entity)?)?
                .is_some()
            {
                constructed = Some(symbols.instance_constructed(output.program, entity)?);
            }
            Some(super::startup::Guard {
                shared: symbols.instance_guard(output.program, entity)?,
                constructed,
            })
        } else {
            None
        };
        output.startup.initialize(
            (output.program, output.definitions, output.finalizers),
            symbols,
            (graph, tree),
            (entity, initializer),
            budget,
            guard,
        )?;
    }
    emit_instance_variables(output, symbols, (graph, tree))?;
    symbols.publish_finalizers(output.finalizers);
    {
        let facts = graph.lower_variable(entity)?;
        let global = symbols.variable_symbol(output.program, entity)?;
        if let Some(mut finalizer) = super::shutdown::for_type(
            output.program,
            symbols,
            graph,
            global,
            facts.ty,
            thread_initialization.map(|(guard, _)| guard),
        )? {
            finalizer.constructed = constructed;
            output.finalizers.push(finalizer);
        }
    }
    if symbols.emit_tls_wrapper(
        output.program,
        entity,
        thread_initialization.map(|(_, init)| init),
    )? {
        output.definitions.stream(output.program, None)?;
    }
    Ok(())
}
pub(super) fn emit_instance_variables<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    (graph, tree): (&'a Graph, &'a ast::Tree),
) -> Result<(), Error> {
    while let Some(entity) = symbols.instance_variables.pop_front() {
        let symbol = symbols.variable_symbol(output.program, entity)?;
        if output
            .program
            .global(output.program.symbol(symbol))
            .is_some()
        {
            continue;
        }
        let initializer =
            graph
                .lower_instance_initializer(entity)
                .ok_or(crate::sema::Error::Invariant(
                    "instance storage without definition",
                ))?;
        define_variable(
            output,
            symbols,
            (graph, tree),
            entity,
            initializer.initializer,
            crate::support::source::SourceOffset(0),
            symbols.budget(),
        )?;
    }
    Ok(())
}
#[derive(PartialEq, Eq)]
enum Initialization {
    Static,
    Dynamic,
}
fn define<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<Initialization, Error> {
    let saved = symbols.replace_temporary_owner(Some((entity, 0)));
    let result = define_storage(output, symbols, graph, tree, entity, initializer, location);
    symbols.replace_temporary_owner(saved);
    result
}
fn define_storage<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<Initialization, Error> {
    let facts = graph.lower_variable(entity)?;
    symbols.storage_destructor(facts.ty)?;
    let mut object = facts.ty;
    while let TypeView::Array { element, .. } = graph.lower_type(object)? {
        object = element;
    }
    let constructed = matches!(graph.lower_type(object)?, TypeView::Class(_))
        && (initializer.is_some() && graph.lower_constructor_arguments(entity).is_some()
            || initializer.is_none()
                && !graph.lower_omitted_variable_initialization(entity)
                && !graph.lower_trivial_default_construction(entity)?);
    let array = matches!(
        graph.lower_type(facts.ty)?,
        TypeView::Array { .. } | TypeView::Class(_)
    );
    let mut ty = if array {
        None
    } else {
        Some(types::storage(graph, facts.ty)?)
    };
    let symbol = symbols.variable_symbol(output.program, entity)?;
    if output
        .program
        .global(output.program.symbol(symbol))
        .is_some_and(|global| !matches!(global.init, GlobalInit::Declaration))
    {
        return Err(Error::Semantic(crate::sema::Error::Diagnostic {
            location,
            message: "redefinition of a variable across translation units",
        }));
    }
    let mut metadata = symbols.variable_metadata(output.program, entity, symbol)?;
    let mut dynamic = constructed;
    // A failed attempt can already have referred to tables or addresses.
    // Replay only its recorded symbolic demand, before publishing storage.
    let table_image = graph.lower_static_construction(entity);
    let _ = data::items(
        output.program,
        symbols,
        graph,
        tree,
        graph.lower_static_construction_demands(entity),
    )?;
    let mut init = if matches!(graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        if facts.reference_temporary.is_some() && facts.initializer.is_some() {
            GlobalInit::Address(
                data::reference_temporary(output.program, symbols, graph, tree, entity, false)?,
                0,
            )
        } else if let Some(value) = facts.initializer {
            let (symbol, offset) = data::address(output.program, symbols, graph, tree, value)?;
            GlobalInit::Address(symbol, offset)
        } else {
            dynamic = true;
            GlobalInit::Zero
        }
    } else if array {
        GlobalInit::Data(if let Some(image) = table_image {
            dynamic = false;
            data::items(output.program, symbols, graph, tree, image)?
        } else if (initializer.is_some()
            || facts.name.is_none()
            || graph.lower_evaluated_construction(entity))
            && let Some(image) = graph.lower_array_image(entity)
        {
            dynamic = false;
            data::items(output.program, symbols, graph, tree, image)?
        } else {
            dynamic |= initializer.is_some();
            if !constructed && let Some(prefix) = graph.lower_array_image_prefix(entity) {
                // The C++ constant attempt demands addresses before publishing
                // the object's zero image, even when a later member is dynamic.
                let _ = data::items(output.program, symbols, graph, tree, prefix)?;
            }
            vec![crate::lowir::model::DataItem::Zero(
                i64::try_from(types::layout(graph, facts.ty)?.0)
                    .map_err(|_| crate::lowir::Error::Limit)?,
            )]
        })
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
                let (symbol, offset) = data::address(output.program, symbols, graph, tree, value)?;
                GlobalInit::Address(symbol, offset)
            }
            None => {
                dynamic = true;
                GlobalInit::Zero
            }
        }
    };
    if array && let GlobalInit::Data(items) = &init {
        ty = structured_type(graph, facts.ty, items)?;
    }
    align_storage(graph, entity, &mut ty, &mut init)?;
    if !dynamic && !facts.thread_local && symbols.readonly_object(facts.ty)? {
        metadata.storage = Some(Storage::ReadOnly);
    }
    let name = output.program.symbol(symbol).name;
    emit_instance_variables(output, symbols, (graph, tree))?;
    output.program.add_global(
        symbol,
        Global {
            name,
            ty,
            metadata,
            init,
        },
    )?;
    if initializer.is_none()
        && !constructed
        && !facts.thread_local
        && graph.lower_empty_initialization_step(entity)?
    {
        symbols.request_initialization(output.program)?;
    }
    if matches!(graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        symbols.data.define_reference(entity);
    }
    if !dynamic && (table_image.is_none() || graph.lower_evaluated_construction(entity)) {
        if constructed
            && (facts.name.is_none() || graph.lower_evaluated_construction(entity))
            && let Some(constructor) = graph.lower_construction(entity)
        {
            symbols.demand_entry(super::construction::Entry::Constructor(
                constructor,
                super::construction::Variant::Complete,
            ));
        }
        if initializer.is_none()
            && let Some(constructor) = graph.lower_absent_initializer_constructor()
        {
            symbols.demand_entry(super::construction::Entry::Constructor(
                constructor,
                super::construction::Variant::Complete,
            ));
        }
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
        if facts.definition.is_some() && !symbols.declaration_only_global(entity) {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "referenced global definition was not emitted",
            )));
        }
        let metadata = symbols.variable_metadata(program, entity, symbol)?;
        let name = program.symbol(symbol).name;
        let ty = if graph.lower_complete_object(facts.ty)?
            && !matches!(
                graph.lower_type(facts.ty)?,
                TypeView::Array { .. } | TypeView::Class(_)
            ) {
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
