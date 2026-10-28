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
impl GlobalOutput<'_> {
    pub(super) fn function_output(&mut self) -> super::startup::FunctionOutput<'_> {
        super::startup::FunctionOutput::new(
            self.program,
            self.startup,
            self.definitions,
            self.finalizers,
        )
    }
}
impl data::Output for GlobalOutput<'_> {
    fn program(&mut self) -> &mut Program {
        self.program
    }
    fn referenced_variable(
        &mut self,
        cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    ) -> Result<(), Error> {
        emit_instance_variables(self, cx)
    }
}
pub(super) fn lower_anonymous_declaration(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    id: ast::DeclId,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    if let Some(entity) = cx.graph.lower_declaration_storage(id) {
        define_variable(
            output,
            cx,
            entity,
            None,
            cx.tree.declarations.get(id).location,
            budget,
        )?;
    }
    Ok(())
}
pub(super) fn lower_declaration(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    declarators: crate::syntax::storage::List<ast::InitDeclarator>,
    explicit_specialization: bool,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    for declarator in cx.tree.init_declarators.get(declarators).to_vec() {
        let entity = cx.graph.lower_declared(declarator.declarator)?;
        if cx.graph.lower_is_variable(entity)
            && cx.graph.lower_variable(entity)?.definition == Some(declarator.declarator)
            && cx.graph.lower_variable_arguments(entity).is_none()
        {
            // This syntactic emission rule also covers an ordinary static
            // member of an explicitly specialized class. Its semantic
            // redeclaration state remains independent of storage emission.
            if explicit_specialization
                && cx.graph.lower_static_member(entity)
                && declarator.initializer.is_none()
            {
                cx.symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .note_declaration_only_global(entity);
                continue;
            }
            let initializer = declarator
                .initializer
                .or(cx.graph.lower_variable(entity)?.in_class_initializer);
            define_variable(
                output,
                cx,
                entity,
                initializer,
                cx.tree.declarators.get(declarator.declarator).location,
                budget,
            )?;
        }
    }
    Ok(())
}
pub(super) fn define_variable(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .check_stack()?;
    let pending = std::mem::take(&mut cx.symbols.instance_variables);
    let previous_variable = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .begin_variable_definition(entity);
    for function in cx.graph.lower_initializer_argument_functions(entity) {
        cx.symbols.root_arguments.note(function);
    }
    let request = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .variable_request(entity);
    let owner = cx.symbols.pending.replace_owner(request);
    let result = define_variable_contents(output, cx, entity, initializer, location, budget);
    cx.symbols.pending.replace_owner(owner);
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .end_variable_definition(entity, previous_variable);
    let nested = std::mem::replace(&mut cx.symbols.instance_variables, pending);
    cx.symbols.instance_variables.extend(nested);
    result
}
fn define_variable_contents(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    let initialization = define(output, cx, entity, initializer, location)?;
    let registered = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .object_mode()
        && cx
            .symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .storage_destructor(cx.graph.lower_entity_type(entity)?)?
            .is_some();
    let mut thread_initialization = None;
    let mut constructed = None;
    if (initialization == Initialization::Dynamic || registered)
        && cx.graph.lower_variable(entity)?.thread_local
    {
        thread_initialization = Some(super::thread_local::initialize(
            output,
            cx,
            entity,
            initializer,
            initialization == Initialization::Dynamic,
            budget,
        )?);
    } else if initialization == Initialization::Dynamic || registered {
        let global = cx
            .symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .variable_symbol(output.program, entity)?;
        let guard = if output
            .program
            .global(output.program.symbol(global))
            .is_some_and(|global| {
                global.metadata.binding == Some(crate::lowir::vocabulary::Binding::Weak)
            }) {
            if !registered
                && cx
                    .symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .storage_destructor(cx.graph.lower_entity_type(entity)?)?
                    .is_some()
            {
                constructed = Some(
                    cx.symbols
                        .view(&*cx.graph, &*cx.tree, cx.cursor)
                        .instance_constructed(output.program, entity)?,
                );
            }
            Some(super::startup::Guard {
                shared: cx
                    .symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .instance_guard(output.program, entity, false)?,
                constructed,
            })
        } else {
            None
        };
        output.startup.initialize(
            (output.program, output.definitions, output.finalizers),
            cx,
            (
                entity,
                initializer,
                initialization == Initialization::Dynamic,
            ),
            budget,
            guard,
        )?;
    }
    emit_instance_variables(output, cx)?;
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .publish_finalizers(output.finalizers);
    {
        let facts = cx.graph.lower_variable(entity)?;
        let global = cx
            .symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .variable_symbol(output.program, entity)?;
        if !registered
            && let Some(mut finalizer) = super::shutdown::for_type(
                output.program,
                &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
                &*cx.graph,
                global,
                facts.ty,
                thread_initialization.and_then(|(guard, _)| guard),
            )?
        {
            finalizer.constructed = constructed;
            output.finalizers.push(finalizer);
        }
    }
    if cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .emit_tls_wrapper(
            output.program,
            entity,
            thread_initialization.map(|(_, init)| init),
        )?
    {
        cx.prepare_forward_boundaries(output.program)?;
        output.definitions.stream(output.program, None)?;
    }
    if thread_initialization.is_none() {
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .tls_static_entry(output.program, entity)?;
    }
    Ok(())
}
pub(super) fn emit_instance_variables(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
) -> Result<(), Error> {
    while let Some(entity) = cx.symbols.instance_variables.pop_front() {
        let symbol = cx
            .symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .variable_symbol(output.program, entity)?;
        if output
            .program
            .global(output.program.symbol(symbol))
            .is_some()
        {
            continue;
        }
        let initializer =
            cx.graph
                .lower_demanded_initializer(entity)
                .ok_or(crate::sema::Error::Invariant(
                    "instance storage without definition",
                ))?;
        let budget = cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor).budget();
        define_variable(
            output,
            cx,
            entity,
            initializer.initializer,
            crate::support::source::SourceOffset(0),
            budget,
        )?;
    }
    Ok(())
}
#[derive(PartialEq, Eq)]
enum Initialization {
    Static,
    Dynamic,
}
fn define(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<Initialization, Error> {
    let saved = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .replace_temporary_owner(Some((entity, 0)));
    let result = define_storage(output, cx, entity, initializer, location);
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .replace_temporary_owner(saved);
    result
}
fn storage_needs_construction(
    graph: &Graph,
    entity: EntityId,
    ty: crate::sema::lowering::TypeId,
    initializer: Option<ast::Initializer>,
) -> Result<bool, Error> {
    let mut object = ty;
    while let TypeView::Array { element, .. } = graph.lower_type(object)? {
        object = element;
    }
    Ok(matches!(graph.lower_type(object)?, TypeView::Class(_))
        && (initializer.is_some() && graph.lower_constructor_arguments(entity).is_some()
            || initializer.is_none()
                && !graph.lower_omitted_variable_initialization(entity)
                && !graph.lower_trivial_default_construction(entity)?))
}
fn storage_symbol(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    entity: EntityId,
    location: crate::support::source::SourceOffset,
) -> Result<(crate::lowir::model::SymbolId, crate::lowir::model::Metadata), Error> {
    let symbol = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .variable_symbol(output.program, entity)?;
    if cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .global_definition_conflicts(output.program, symbol)
    {
        return Err(Error::Semantic(crate::sema::Error::Diagnostic {
            location,
            message: "redefinition of a variable across translation units",
        }));
    }
    let metadata = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .variable_metadata(output.program, entity, symbol)?;
    Ok((symbol, metadata))
}
fn check_flexible_initialization(
    graph: &Graph,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<(), Error> {
    if initializer.is_some_and(|initializer| match initializer.kind {
        ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr) => {
            graph.lower_flexible_elements(expr)
        }
        _ => false,
    }) && graph.lower_array_image(entity).is_none()
    {
        return Err(crate::sema::Error::at(
            location,
            "a flexible array member's elements in a dynamic initialization",
        )
        .into());
    }
    Ok(())
}
fn define_storage(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    location: crate::support::source::SourceOffset,
) -> Result<Initialization, Error> {
    let facts = cx.graph.lower_variable(entity)?;
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .storage_destructor(facts.ty)?;
    let constructed = storage_needs_construction(cx.graph, entity, facts.ty, initializer)?;
    let array = matches!(
        cx.graph.lower_type(facts.ty)?,
        TypeView::Array { .. }
            | TypeView::Class(_)
            | TypeView::Complex(_)
            | TypeView::Vector { .. }
    );
    let mut ty = if array {
        None
    } else {
        Some(types::storage(&*cx.graph, facts.ty)?)
    };
    let (symbol, mut metadata) = storage_symbol(output, cx, entity, location)?;
    let mut dynamic = constructed;
    check_flexible_initialization(cx.graph, entity, initializer, location)?;
    // A failed attempt can already have referred to tables or addresses.
    // Replay only its recorded symbolic demand, before publishing storage.
    let table_image = cx
        .graph
        .lower_static_construction(entity)
        .map(<[_]>::to_vec);
    let table_image = table_image.as_deref();
    let demands = cx.graph.lower_static_construction_demands(entity).to_vec();
    let _ = data::items(output, cx, &demands)?;
    let mut init = if matches!(cx.graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        if facts.reference_temporary.is_some() && facts.initializer.is_some() {
            GlobalInit::Address(data::reference_temporary(output, cx, entity, false)?, 0)
        } else if let Some(value) = facts.initializer {
            let (symbol, offset) = data::address(output, cx, value)?;
            GlobalInit::Address(symbol, offset)
        } else {
            dynamic = true;
            GlobalInit::Zero
        }
    } else if matches!(
        cx.graph.lower_type(facts.ty)?,
        TypeView::Vector { .. } | TypeView::Complex(_)
    ) && cx.graph.lower_array_image(entity).is_none()
    {
        ty = Some(types::storage(&*cx.graph, facts.ty)?);
        dynamic |= initializer.is_some();
        GlobalInit::Zero
    } else if array {
        GlobalInit::Data(if let Some(image) = table_image {
            dynamic = false;
            data::items(output, cx, image)?
        } else if (initializer.is_some()
            || facts.name.is_none()
            || cx.graph.lower_evaluated_construction(entity))
            && let Some(image) = cx.graph.lower_array_image(entity).map(<[_]>::to_vec)
        {
            dynamic = false;
            data::items(output, cx, &image)?
        } else {
            dynamic |= initializer.is_some();
            if !constructed
                && let Some(prefix) = cx.graph.lower_array_image_prefix(entity).map(<[_]>::to_vec)
            {
                // The C++ constant attempt demands addresses before publishing
                // the object's zero image, even when a later member is dynamic.
                let _ = data::items(output, cx, &prefix)?;
            }
            vec![crate::lowir::model::DataItem::Zero(
                i64::try_from(types::layout(&*cx.graph, facts.ty)?.0)
                    .map_err(|_| crate::lowir::Error::Limit)?,
            )]
        })
    } else if initializer.is_none() {
        GlobalInit::Zero
    } else {
        match facts.initializer {
            Some(value) if data::soft_item(value).is_some() => {
                let crate::lowir::model::DataItem::Constant(stored, constant) =
                    data::soft_item(value)
                        .ok_or(crate::sema::Error::Invariant("soft static item"))?
                else {
                    return Err(crate::sema::Error::Invariant("soft scalar constant").into());
                };
                ty = Some(stored);
                GlobalInit::Constant(constant)
            }
            Some(value @ StaticValue::MemberPointer(_)) => GlobalInit::Data(data::member_items(
                output,
                &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
                &*cx.graph,
                value,
            )?),
            Some(StaticValue::WideInteger(bits)) => GlobalInit::Constant(Constant::Integer(bits)),
            Some(StaticValue::Integer(bits)) => GlobalInit::Constant(Constant::Integer(
                i128::from(bits.cast_signed()).cast_unsigned(),
            )),
            Some(StaticValue::Floating(ty, bits)) => GlobalInit::Constant(Constant::Float(
                match ty.floating_representation() {
                    F::Float => FloatKind::F32,
                    F::Double => FloatKind::F64,
                    _ => FloatKind::F80,
                },
                bits,
            )),
            Some(
                value @ (StaticValue::Literal(..)
                | StaticValue::Address(..)
                | StaticValue::Temporary(..)
                | StaticValue::TypeInformation(..)),
            ) => {
                let (symbol, offset) = data::address(output, cx, value)?;
                GlobalInit::Address(symbol, offset)
            }
            None => {
                dynamic = true;
                GlobalInit::Zero
            }
        }
    };
    if let GlobalInit::Data(items) = &mut init {
        let budget = cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor).budget();
        super::null_initialization::static_nulls(
            &mut cx.symbols.data.nulls,
            &*cx.graph,
            facts.ty,
            items,
            budget,
        )?;
        data::preserve_static_offsets(items)?;
    } else if matches!(init, GlobalInit::Zero)
        && cx.graph.lower_member_pointer_type(facts.ty).is_some()
        && cx.graph.lower_holds_data_member_pointer(facts.ty)?
    {
        init = GlobalInit::Constant(Constant::Integer(u128::MAX));
    }
    if !dynamic
        && matches!(cx.graph.lower_type(facts.ty)?, TypeView::Reference { .. })
        && cx.graph.lower_reference_reads_storage(entity)
    {
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .demand_reference_initializer_calls(&*cx.graph, &*cx.tree, initializer)?;
    }
    if let GlobalInit::Data(items) = &init {
        ty = structured_type(
            &*cx.graph,
            facts.ty,
            items,
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .object_mode(),
        )?;
    }
    align_storage(
        &*cx.graph,
        entity,
        &mut ty,
        &mut init,
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .object_mode(),
    )?;
    if !dynamic
        && !facts.thread_local
        && cx
            .symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .readonly_object(facts.ty)?
    {
        metadata.storage = Some(Storage::ReadOnly);
    }
    let name = output.program.symbol(symbol).name;
    emit_instance_variables(output, cx)?;
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
        && cx.graph.lower_empty_initialization_step(entity)?
    {
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .request_initialization(output.program)?;
    }
    if matches!(cx.graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
        cx.symbols.data.define_reference(entity);
    }
    if !dynamic && (table_image.is_none() || cx.graph.lower_evaluated_construction(entity)) {
        if constructed
            && (facts.name.is_none() || cx.graph.lower_evaluated_construction(entity))
            && let Some(constructor) = cx.graph.lower_construction(entity)
        {
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .demand_entry(super::construction::Entry::Constructor(
                    constructor,
                    super::construction::Variant::Complete,
                ));
        }
        if cx.graph.lower_evaluated_construction(entity) {
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .demand_constant_constructions(&*cx.graph, &*cx.tree, initializer);
        }
        if initializer.is_none()
            && let Some(constructor) = cx.graph.lower_absent_initializer_constructor()
        {
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .demand_entry(super::construction::Entry::Constructor(
                    constructor,
                    super::construction::Variant::Complete,
                ));
        }
        for &constructor in cx.graph.lower_initializer_constructors(entity).iter().rev() {
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .demand_entry(super::construction::Entry::Constructor(
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
    object_mode: bool,
) -> Result<(), Error> {
    use crate::lowir::{model::DataItem, vocabulary::Type};
    let ty = graph.lower_entity_type(entity)?;
    let (size, natural) = types::layout(graph, ty)?;
    let mut requested = u64::from(graph.lower_requested_alignment(entity));
    if object_mode && size >= 16 && matches!(graph.lower_type(ty)?, TypeView::Array { .. }) {
        requested = requested.max(16);
    }
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
        .map_or(if object_mode { 16 } else { 1 }, |align| 1u64 << align);
    *storage = (requested > implied).then_some(Type::object(
        if object_mode {
            size.max(requested)
        } else {
            size.wrapping_add(requested - 1) / requested * requested
        },
        u8::try_from(requested.trailing_zeros()).map_err(|_| crate::lowir::Error::Limit)?,
    ));
    Ok(())
}
pub(super) fn structured_type(
    graph: &Graph,
    ty: crate::sema::lowering::TypeId,
    items: &[crate::lowir::model::DataItem],
    object_mode: bool,
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
        .map_or(if object_mode { 16 } else { 1 }, |align| 1u64 << align);
    let alignment = 1u64 << align;
    Ok(
        (alignment > implied).then_some(crate::lowir::vocabulary::Type::object(
            if object_mode {
                bytes.max(alignment)
            } else {
                bytes.wrapping_add(alignment - 1) / alignment * alignment
            },
            align,
        )),
    )
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
                TypeView::Array { .. }
                    | TypeView::Class(_)
                    | TypeView::Complex(_)
                    | TypeView::Vector { .. }
            ) {
            Some(types::storage(graph, facts.ty)?)
        } else {
            None
        };
        symbols.finish_class_declarations(program, Some(symbol))?;
        program.add_global(
            symbol,
            Global {
                name,
                ty,
                metadata,
                init: GlobalInit::Declaration,
            },
        )?;
        symbols.note_deferred_global(symbol);
    }
    Ok(())
}
