// Translated from lower/program_lowering.cpp. See NOTICE.
//! Typed, demand-driven source lowering into the shared function-owned `LowIR` model.
mod aggregate_arrays;
mod aggregate_cleanup;
mod aggregate_helpers;
mod aggregates;
mod allocation;
mod anonymous_names;
mod array_allocation;
mod array_construction;
mod array_deletion;
mod array_initialization;
mod arrays;
mod arrow_calls;
mod assembly;
mod assignment;
mod atomic_intrinsics;
mod atomic_places;
mod base_initialization;
mod base_projections;
mod bindings;
mod bit_fields;
mod block_calls;
mod class_conditionals;
mod class_parameters;
mod class_results;
mod closures;
mod conditional_values;
mod conditions;
mod constant_demands;
mod construction;
mod construction_context;
mod construction_flow;
mod constructor_cleanup;
mod conversion_functions;
mod conversions;
mod copy_arrays;
mod copy_cleanup;
mod copy_construction;
mod data;
mod definition_demands;
mod definition_order;
mod definition_regions;
mod destruction;
mod dispatch;
mod dynamic_casts;
mod element_temporaries;
mod exception_chains;
mod exception_helpers;
mod expressions;
mod field_updates;
mod floating_intrinsics;
mod function;
mod globals;
mod handlers;
mod initializer_lists;
mod integer_intrinsics;
mod lexical_cleanup;
mod lifetime;
mod lifetime_regions;
mod lifetime_scan;
mod list_construction;
mod list_elements;
mod literal_calls;
mod local_storage;
mod materialization;
mod member_initialization;
mod member_pointers;
mod mutations;
mod null_initialization;
mod object_arrays;
mod object_classes;
mod object_storage;
mod observations;
pub(crate) use observations::Statistics;
mod operator_calls;
mod overflow_intrinsics;
mod pointers;
mod range_for;
mod reference_lifetimes;
mod references;
mod runtime_intrinsics;
mod runtime_roles;
mod shutdown;
mod source_locations;
use symbols::{complex, soft_float};
mod stack_intrinsics;
mod startup;
mod statement_expressions;
mod statements;
mod symbols;
mod temporary_scan;
mod thread_local;
mod throw_scan;
mod throws;
mod tracked_lifetimes;
mod type_queries;
mod types;
mod unit_frontend;
use crate::{
    lowir::model::Program,
    sema::{self, Graph},
    support::stack::StackBudget,
    syntax::{ast, cursor::Cursor, storage::List},
};
#[derive(Debug)]
pub(crate) enum Error {
    Semantic(sema::Error),
    Ir(crate::lowir::Error),
    Abi(crate::abi::Error),
    Missing(&'static str),
}
impl From<sema::Error> for Error {
    fn from(error: sema::Error) -> Self {
        Self::Semantic(error)
    }
}
impl From<crate::lowir::Error> for Error {
    fn from(error: crate::lowir::Error) -> Self {
        Self::Ir(error)
    }
}
impl From<crate::abi::Error> for Error {
    fn from(error: crate::abi::Error) -> Self {
        Self::Abi(error)
    }
}
#[derive(Default)]
pub(crate) struct Output {
    pub program: Program,
    pub definitions: definition_order::Definitions,
    pub initializer: Option<crate::lowir::model::SymbolId>,
    pub finalizer: Option<crate::lowir::model::SymbolId>,
    symbols: symbols::SymbolTable,
    fixed_names_interned: bool,
    startup: startup::Startup,
    finalizers: Vec<shutdown::Finalizer>,
}
impl Output {
    pub(crate) fn set_debug_locations(&mut self, enabled: bool) {
        self.symbols.debug_locations = if enabled {
            symbols::DebugLocations::Enabled
        } else {
            symbols::DebugLocations::Disabled
        };
    }

    pub(crate) fn statistics(&self, semantic: sema::Work) -> Statistics {
        self.symbols.work.snapshot(&self.program, semantic)
    }
    pub(crate) fn retain_definitions(&mut self) -> Result<(), Error> {
        self.program.retain_definitions()?;
        self.definitions.retain();
        Ok(())
    }
    pub(crate) fn retain_object_definitions(&mut self, host_objects: bool) -> Result<(), Error> {
        self.retain_definitions()?;
        self.set_object_mode(host_objects);
        Ok(())
    }
    pub(crate) fn set_object_mode(&mut self, host_objects: bool) {
        self.program.object_facts = true;
        self.symbols.object_mode = true;
        self.symbols.host_objects = host_objects;
    }

    pub(crate) fn finish_initialization(&mut self) -> Result<(), Error> {
        if let Some(symbol) = self.initializer {
            self.startup.finish(&mut self.program, symbol)?;
        }
        if !self.finalizers.is_empty() {
            let symbol = self.symbols.finalizer_symbol(&mut self.program)?;
            shutdown::finish(&mut self.program, symbol, &self.finalizers)?;
            self.finalizer = Some(symbol);
            self.finalizers.clear();
        }
        self.definitions
            .finish_retained(&mut self.program, [self.initializer, self.finalizer]);
        Ok(())
    }
    pub(crate) fn finish_entry_aliases(&mut self) {
        self.symbols.finish_entry_aliases(&mut self.program);
    }
    pub(crate) fn finish_program(mut self) -> Result<Program, Error> {
        self.finish_initialization()?;
        self.finish_entry_aliases();
        // Only LowIR crosses the completed lowering boundary. Its demand,
        // naming and definition-order caches have had their last consumer.
        Ok(self.program)
    }
    pub(crate) fn finish_program_observed(
        mut self,
        work: sema::Work,
    ) -> Result<(Program, Statistics), Error> {
        self.finish_initialization()?;
        self.finish_entry_aliases();
        let statistics = self.statistics(work);
        Ok((self.program, statistics))
    }
}
fn finish_unit(
    program: &mut Program,
    definitions: &mut definition_order::Definitions,
    finalizers: &mut Vec<shutdown::Finalizer>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    budget: StackBudget,
) -> Result<Option<crate::lowir::model::SymbolId>, Error> {
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .emit_thunks(program, definitions)?;
    aggregate_helpers::emit_pending(
        program,
        &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
        &*cx.graph,
        definitions,
    )?;
    definitions.release_held(program);
    definitions.generated(&mut cx.symbols.generated_definitions)?;
    thread_local::finish_unit(program, definitions, cx, budget)?;
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .finish_inline_declarations(program)?;
    globals::finish(
        program,
        &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
        &*cx.graph,
    )?;
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .finish_class_declarations(program, None)?;
    cx.finish_function_declarations(program)?;
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .resolve_runtime_roles(program, definitions)?;
    if cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .object_mode()
    {
        program.require_object_definitions();
    }
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .publish_finalizers(finalizers);
    Ok(cx.symbols.initialization)
}
pub(crate) fn lower_unit(
    output: &mut Output,
    graph: &mut Graph,
    tree: &mut ast::Tree,
    cursor: &mut Cursor<'_>,
    declarations: List<ast::DeclId>,
    budget: StackBudget,
) -> Result<(), Error> {
    if !output.fixed_names_interned {
        // ProgramLowering's fixed names belong to the model even when no
        // definition uses them. Presentation-only names remain separate.
        for name in [
            "entry", "this", "ret", "other", "__slot", "__range", "__begin", "__end", "__idx",
        ] {
            output.program.names.intern(name)?;
        }
    }
    output.fixed_names_interned = true;
    output.symbols.work.units += 1;
    let mut symbol_state = symbols::State::new(graph, budget, std::mem::take(&mut output.symbols));
    let mut cx = unit_frontend::Context {
        graph,
        tree,
        cursor,
        symbols: &mut symbol_state,
    };
    emit_source_definitions(
        &mut globals::GlobalOutput {
            program: &mut output.program,
            startup: &mut output.startup,
            definitions: &mut output.definitions,
            finalizers: &mut output.finalizers,
        },
        &mut cx,
        declarations,
        budget,
    )?;
    output.initializer = finish_unit(
        &mut output.program,
        &mut output.definitions,
        &mut output.finalizers,
        &mut cx,
        budget,
    )?
    .or(output.initializer);
    output.symbols = symbol_state.into_table();
    Ok(())
}

// The source-order walk owns syntax roots; the same output context follows
// every requested body and its immediate static-data dependencies.
fn emit_source_definitions(
    output: &mut globals::GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    declarations: List<ast::DeclId>,
    budget: StackBudget,
) -> Result<(), Error> {
    let mut declarations = anonymous_names::Declarations::new(&*cx.tree, declarations);
    let mut emitted = std::collections::HashSet::new();
    let mut emitted_entries = std::collections::HashSet::new();
    while let Some((id, explicit_specialization)) = declarations.next(
        &*cx.tree,
        &*cx.graph,
        &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
    )? {
        globals::lower_anonymous_declaration(output, cx, id, budget)?;
        match cx.tree.declarations.get(id).kind {
            ast::DeclKind::Function {
                declarator, body, ..
            } => {
                let entity = cx.graph.lower_declared(declarator)?;
                if cx.graph.lower_function(entity)?.emission
                    == crate::sema::lowering::Emission::Always
                    && emitted.insert(entity)
                {
                    function::lower(
                        output.function_output(),
                        cx,
                        construction::Entry::Function(entity),
                        Some(body),
                        budget,
                    )?;
                    cx.stream_definition(
                        output.program,
                        output.definitions,
                        construction::Entry::Function(entity),
                    )?;
                }
            }
            ast::DeclKind::SpecialMember {
                declarator,
                body,
                initializer,
                ..
            } => {
                if body.is_none()
                    && !initializer
                        .is_some_and(|init| matches!(init.kind, ast::InitializerKind::Default))
                {
                    continue;
                }
                let entity = cx.graph.lower_declared(declarator)?;
                let facts = cx.graph.lower_function(entity)?;
                if matches!(facts.name, sema::lowering::FunctionName::Conversion(_)) {
                    if facts.emission == sema::lowering::Emission::Always && emitted.insert(entity)
                    {
                        let entry = construction::Entry::Function(entity);
                        function::lower(
                            output.function_output(),
                            cx,
                            entry,
                            cx.graph.lower_function_body(entity),
                            budget,
                        )?;
                        cx.stream_definition(output.program, output.definitions, entry)?;
                    }
                    continue;
                }
                if matches!(facts.name, sema::lowering::FunctionName::Destructor(_)) {
                    if facts.emission == sema::lowering::Emission::Always && emitted.insert(entity)
                    {
                        // define_function publishes the complete symbol before
                        // any ABI entry; an explicit label may make them share it.
                        let _symbol = cx
                            .symbols
                            .view(&*cx.graph, &*cx.tree, cx.cursor)
                            .function_symbol(output.program, entity)?;
                        let variants = [
                            Some(construction::Entry::DestructorBase(entity)),
                            cx.graph
                                .lower_virtual_slot(entity)
                                .map(|_| construction::Entry::DestructorDeleting(entity)),
                            Some(construction::Entry::Function(entity)),
                        ];
                        for entry in variants.into_iter().flatten() {
                            emitted_entries.insert(entry);
                            function::lower(
                                output.function_output(),
                                cx,
                                entry,
                                cx.graph.lower_function_body(entity),
                                budget,
                            )?;
                            cx.stream_definition(output.program, output.definitions, entry)?;
                        }
                    }
                    continue;
                }
                if facts.emission == crate::sema::lowering::Emission::Always {
                    let _symbol = cx
                        .symbols
                        .view(&*cx.graph, &*cx.tree, cx.cursor)
                        .function_symbol(output.program, entity)?;
                    let id = cx
                        .graph
                        .lower_construction(entity)
                        .ok_or(Error::Missing("special member construction plan"))?;
                    for variant in [construction::Variant::Base, construction::Variant::Complete] {
                        let entry = construction::Entry::Constructor(id, variant);
                        if variant == construction::Variant::Complete
                            && cx.graph.lower_constructor_trivial(id)
                        {
                            continue;
                        }
                        emitted_entries.insert(entry);
                        function::lower(
                            output.function_output(),
                            cx,
                            construction::Entry::Constructor(id, variant),
                            cx.graph.lower_function_body(entity),
                            budget,
                        )?;
                        cx.stream_definition(output.program, output.definitions, entry)?;
                    }
                }
            }
            ast::DeclKind::Simple { declarators, .. } => {
                assignment::lower_definitions(
                    assignment::DefinitionOutput {
                        program: output.program,
                        definitions: output.definitions,
                        emitted: &mut emitted,
                    },
                    cx,
                    declarators,
                    budget,
                )?;
                globals::lower_declaration(
                    output,
                    cx,
                    declarators,
                    explicit_specialization,
                    budget,
                )?;
            }
            ast::DeclKind::Namespace { .. }
            | ast::DeclKind::Linkage { .. }
            | ast::DeclKind::Empty
            | ast::DeclKind::Alias { .. }
            | ast::DeclKind::NamespaceAlias { .. }
            | ast::DeclKind::Enum { .. }
            | ast::DeclKind::Using { .. }
            | ast::DeclKind::StaticAssert { .. }
            | ast::DeclKind::ExplicitInstantiation { .. }
            | ast::DeclKind::Class(_) => {}
            // A primary template has no emitted definition. Specializations
            // are ordinary demand entries, separate from this syntax walk.
            ast::DeclKind::Template { parameters, .. }
                if !cx
                    .tree
                    .template_parameters
                    .get(parameters.parameters)
                    .is_empty() => {}
            _ => return Err(Error::Missing("class and template declaration lowering")),
        }
        globals::emit_instance_variables(output, cx)?;
    }
    for id in cx.graph.lower_root_delegations() {
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .demand_entry(construction::Entry::Constructor(
                id,
                construction::Variant::Complete,
            ));
    }
    for function in cx.symbols.root_arguments.finish() {
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .demand(function);
    }
    emit_pending(output, cx, (&mut emitted, &mut emitted_entries), budget)?;
    Ok(())
}

// Explicit definitions root their own ABI entries, independently of calls.
fn demand_explicit_definitions(
    program: &mut Program,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
) -> Result<(), Error> {
    for entity in cx.graph.lower_explicit_functions() {
        if !cx.graph.lower_function_defined(entity) {
            continue;
        }
        let base = if let Some(id) = cx.graph.lower_construction(entity) {
            Some(construction::Entry::Constructor(
                id,
                construction::Variant::Base,
            ))
        } else if matches!(
            cx.graph.lower_function(entity)?.name,
            sema::lowering::FunctionName::Destructor(_)
        ) {
            Some(construction::Entry::DestructorBase(entity))
        } else {
            None
        };
        if let Some(base) = base
            && base.hidden_vtt(&*cx.graph)?
        {
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .demand_entry(base);
        }
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .demand(entity);
    }
    for index in 0..cx.graph.lower_explicit_classes().len() {
        let class = cx.graph.lower_explicit_classes()[index];
        cx.construction_table(program, class)?;
    }
    Ok(())
}

// The unit's demand worklist owns emission identity; each emitted body is
// streamed and released before the next one is lowered.
fn emit_pending(
    output: &mut globals::GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    (emitted, entries): (
        &mut std::collections::HashSet<sema::lowering::EntityId>,
        &mut std::collections::HashSet<construction::Entry>,
    ),
    budget: StackBudget,
) -> Result<(), Error> {
    let mut explicit = 0;
    let mut variable = 0;
    loop {
        globals::emit_instance_variables(output, cx)?;
        if let Some(entity) = cx.symbols.pending_copies.pop() {
            let entry = construction::Entry::InlineCopy(entity);
            let body = function::definition_body(&*cx.graph, &*cx.tree, entity)?;
            function::lower(output.function_output(), cx, entry, body, budget)?;
            cx.stream_definition(output.program, output.definitions, entry)?;
            continue;
        }
        exception_helpers::emit_pending(
            output.program,
            &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
            output.definitions,
        )?;
        aggregate_helpers::emit_pending(
            output.program,
            &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
            &*cx.graph,
            output.definitions,
        )?;
        let Some(entry) = cx.symbols.pending.next() else {
            // Each C++ emit_demanded drain finishes its thunks before the
            // next explicit-instantiation or variable-root demand boundary.
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .emit_thunks(output.program, output.definitions)?;
            if explicit == 0 {
                demand_explicit_definitions(output.program, cx)?;
                explicit = 1;
            } else if let Some(&entity) = cx.graph.lower_demanded_variables().get(variable) {
                // Each variable root owns a complete demand drain, including
                // functions and thunks its initializer requests.
                variable += 1;
                // Host objects demand an integral member's storage through
                // an emitted address use, unless explicitly instantiated.
                // A value-only semantic demand still roots course storage.
                if cx
                    .symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .host_objects()
                    && cx.graph.lower_value_only_variable_root(entity)
                {
                    continue;
                }
                cx.symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .variable_symbol(output.program, entity)?;
            } else {
                break;
            }
            continue;
        };
        if !entries.insert(entry) {
            continue;
        }
        if entry
            .entity(&*cx.graph)
            .is_some_and(|entity| cx.graph.lower_gnu_inline(entity))
        {
            continue;
        }
        let body = if let construction::Entry::Function(entity)
        | construction::Entry::DestructorBase(entity)
        | construction::Entry::DestructorDeleting(entity) = entry
        {
            if matches!(entry, construction::Entry::Function(_)) && !emitted.insert(entity) {
                continue;
            }
            let facts = cx.graph.lower_function(entity)?;
            if facts.definition.is_none()
                && cx.graph.lower_function_body(entity).is_none()
                && !cx.graph.lower_has_synthesized_body(entity)
            {
                // program_lowering.cpp::demand_entry declines unavailable or
                // suppressed bodies without creating a symbol. A use still
                // publishes its declaration; ancillary destructor entry demand
                // alone does not (notably an extern-instantiated class's D2).
                continue;
            }
            if facts.emission == crate::sema::lowering::Emission::Demanded {
                let symbol = cx
                    .symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .entry_symbol(output.program, entry)?;
                if output.program.function_defined(symbol) {
                    continue;
                }
            }
            function::definition_body(&*cx.graph, &*cx.tree, entity)?
        } else {
            let construction::Entry::Constructor(id, _) = entry else {
                return Err(Error::Missing("entry kind"));
            };
            if let Some(entity) = cx.graph.lower_constructor_entity(id)
                && !cx.graph.lower_function_defined(entity)
            {
                continue;
            }
            let facts = entry.facts(&*cx.graph)?;
            let symbol = cx
                .symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .entry_symbol(output.program, entry)?;
            if facts.emission == crate::sema::lowering::Emission::Demanded
                && output.program.function_defined(symbol)
            {
                continue;
            }
            cx.graph
                .lower_constructor_entity(id)
                .and_then(|entity| cx.graph.lower_function_body(entity))
        };
        function::lower(output.function_output(), cx, entry, body, budget)?;
        cx.stream_definition(output.program, output.definitions, entry)?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) use reference_lifetimes::tests::assert_extended_ownership;

#[cfg(test)]
pub(crate) use temporary_scan::tests::assert_constructor_parameter_temporaries;

#[cfg(test)]
pub(crate) use symbols::{
    assert_covariant_routes, assert_definition_destructor_targets, assert_hierarchy_flags,
    assert_table_demand,
};

#[cfg(test)]
pub(crate) use symbols::assert_base_routes;

#[cfg(test)]
pub(crate) use symbols::assert_construction_context;

#[cfg(test)]
pub(crate) use symbols::assert_label_name_demand;

mod vectors;
