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
mod assignment;
mod atomic_intrinsics;
mod base_initialization;
mod base_projections;
mod bit_fields;
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
    startup: startup::Startup,
    finalizers: Vec<shutdown::Finalizer>,
}
impl Output {
    pub(crate) fn retain_object_definitions(&mut self, host_objects: bool) -> Result<(), Error> {
        self.program.retain_definitions()?;
        self.set_object_mode(host_objects);
        self.definitions.retain();
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
}
fn finish_unit<'a>(
    program: &mut Program,
    definitions: &mut definition_order::Definitions,
    finalizers: &mut Vec<shutdown::Finalizer>,
    symbols: &mut symbols::Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    budget: StackBudget,
) -> Result<Option<crate::lowir::model::SymbolId>, Error> {
    symbols.emit_thunks(program, definitions)?;
    aggregate_helpers::emit_pending(program, symbols, graph, definitions)?;
    definitions.release_held(program);
    definitions.generated(&mut symbols.generated_definitions)?;
    thread_local::finish_unit(program, definitions, symbols, graph, tree, budget)?;
    symbols.finish_inline_declarations(program)?;
    globals::finish(program, symbols, graph)?;
    symbols.finish_class_declarations(program, None)?;
    symbols.resolve_runtime_roles(program, definitions)?;
    if symbols.object_mode() {
        program.require_object_definitions();
    }
    symbols.publish_finalizers(finalizers);
    Ok(symbols.initialization)
}
pub(crate) fn lower_unit(
    output: &mut Output,
    graph: &Graph,
    tree: &ast::Tree,
    cursor: &Cursor<'_>,
    declarations: List<ast::DeclId>,
    budget: StackBudget,
) -> Result<(), Error> {
    let mut symbols = symbols::Symbols::new(graph, tree, cursor, budget, &mut output.symbols);
    emit_source_definitions(
        &mut globals::GlobalOutput {
            program: &mut output.program,
            startup: &mut output.startup,
            definitions: &mut output.definitions,
            finalizers: &mut output.finalizers,
        },
        &mut symbols,
        (graph, tree),
        declarations,
        budget,
    )?;
    output.initializer = finish_unit(
        &mut output.program,
        &mut output.definitions,
        &mut output.finalizers,
        &mut symbols,
        graph,
        tree,
        budget,
    )?
    .or(output.initializer);
    Ok(())
}

// The source-order walk owns syntax roots; the same output context follows
// every requested body and its immediate static-data dependencies.
fn emit_source_definitions<'a>(
    output: &mut globals::GlobalOutput<'_>,
    symbols: &mut symbols::Symbols<'a, '_>,
    (graph, tree): (&'a Graph, &'a ast::Tree),
    declarations: List<ast::DeclId>,
    budget: StackBudget,
) -> Result<(), Error> {
    let mut declarations = anonymous_names::Declarations::new(tree, declarations);
    let mut emitted = std::collections::HashSet::new();
    let mut emitted_entries = std::collections::HashSet::new();
    while let Some((id, explicit_specialization)) = declarations.next(tree, graph, symbols)? {
        globals::lower_anonymous_declaration(output, symbols, (graph, tree), id, budget)?;
        match tree.declarations.get(id).kind {
            ast::DeclKind::Function {
                declarator, body, ..
            } => {
                let entity = graph.lower_declared(declarator)?;
                if graph.lower_function(entity)?.emission == crate::sema::lowering::Emission::Always
                    && emitted.insert(entity)
                {
                    function::lower(
                        output.function_output(),
                        symbols,
                        graph,
                        tree,
                        construction::Entry::Function(entity),
                        Some(body),
                        budget,
                    )?;
                    symbols.stream_definition(
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
                let entity = graph.lower_declared(declarator)?;
                let facts = graph.lower_function(entity)?;
                if matches!(facts.name, sema::lowering::FunctionName::Conversion(_)) {
                    if facts.emission == sema::lowering::Emission::Always && emitted.insert(entity)
                    {
                        let entry = construction::Entry::Function(entity);
                        function::lower(
                            output.function_output(),
                            symbols,
                            graph,
                            tree,
                            entry,
                            graph.lower_function_body(entity),
                            budget,
                        )?;
                        symbols.stream_definition(output.program, output.definitions, entry)?;
                    }
                    continue;
                }
                if matches!(facts.name, sema::lowering::FunctionName::Destructor(_)) {
                    if facts.emission == sema::lowering::Emission::Always && emitted.insert(entity)
                    {
                        let variants = [
                            Some(construction::Entry::DestructorBase(entity)),
                            graph
                                .lower_virtual_slot(entity)
                                .map(|_| construction::Entry::DestructorDeleting(entity)),
                            Some(construction::Entry::Function(entity)),
                        ];
                        for entry in variants.into_iter().flatten() {
                            emitted_entries.insert(entry);
                            function::lower(
                                output.function_output(),
                                symbols,
                                graph,
                                tree,
                                entry,
                                graph.lower_function_body(entity),
                                budget,
                            )?;
                            symbols.stream_definition(output.program, output.definitions, entry)?;
                        }
                    }
                    continue;
                }
                if facts.emission == crate::sema::lowering::Emission::Always {
                    let id = graph
                        .lower_construction(entity)
                        .ok_or(Error::Missing("special member construction plan"))?;
                    for variant in [construction::Variant::Base, construction::Variant::Complete] {
                        let entry = construction::Entry::Constructor(id, variant);
                        if variant == construction::Variant::Complete
                            && graph.lower_constructor_trivial(id)
                        {
                            continue;
                        }
                        emitted_entries.insert(entry);
                        function::lower(
                            output.function_output(),
                            symbols,
                            graph,
                            tree,
                            construction::Entry::Constructor(id, variant),
                            graph.lower_function_body(entity),
                            budget,
                        )?;
                        symbols.stream_definition(output.program, output.definitions, entry)?;
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
                    symbols,
                    graph,
                    tree,
                    declarators,
                    budget,
                )?;
                globals::lower_declaration(
                    output,
                    symbols,
                    graph,
                    tree,
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
                if !tree
                    .template_parameters
                    .get(parameters.parameters)
                    .is_empty() => {}
            _ => return Err(Error::Missing("class and template declaration lowering")),
        }
        globals::emit_instance_variables(output, symbols, (graph, tree))?;
    }
    for id in graph.lower_root_delegations() {
        symbols.demand_entry(construction::Entry::Constructor(
            id,
            construction::Variant::Complete,
        ));
    }
    for function in symbols.root_arguments.finish() {
        symbols.demand(function);
    }
    emit_pending(
        output,
        symbols,
        (graph, tree),
        (&mut emitted, &mut emitted_entries),
        budget,
    )?;
    Ok(())
}

// Explicit definitions root their own ABI entries, independently of calls.
fn demand_explicit_definitions(
    program: &mut Program,
    symbols: &mut symbols::Symbols<'_, '_>,
    graph: &Graph,
) -> Result<(), Error> {
    for entity in graph.lower_explicit_functions() {
        if !graph.lower_function_defined(entity) {
            continue;
        }
        let base = if let Some(id) = graph.lower_construction(entity) {
            Some(construction::Entry::Constructor(
                id,
                construction::Variant::Base,
            ))
        } else if matches!(
            graph.lower_function(entity)?.name,
            sema::lowering::FunctionName::Destructor(_)
        ) {
            Some(construction::Entry::DestructorBase(entity))
        } else {
            None
        };
        if let Some(base) = base
            && base.hidden_vtt(graph)?
        {
            symbols.demand_entry(base);
        }
        symbols.demand(entity);
    }
    for &class in graph.lower_explicit_classes() {
        symbols.construction_table(program, class)?;
    }
    Ok(())
}

// The unit's demand worklist owns emission identity; each emitted body is
// streamed and released before the next one is lowered.
fn emit_pending<'a>(
    output: &mut globals::GlobalOutput<'_>,
    symbols: &mut symbols::Symbols<'a, '_>,
    (graph, tree): (&'a Graph, &'a ast::Tree),
    (emitted, entries): (
        &mut std::collections::HashSet<sema::lowering::EntityId>,
        &mut std::collections::HashSet<construction::Entry>,
    ),
    budget: StackBudget,
) -> Result<(), Error> {
    let mut explicit = 0;
    let mut variable = 0;
    loop {
        globals::emit_instance_variables(output, symbols, (graph, tree))?;
        if let Some(entity) = symbols.pending_copies.pop() {
            let entry = construction::Entry::InlineCopy(entity);
            let body = function::definition_body(graph, tree, entity)?;
            function::lower(
                output.function_output(),
                symbols,
                graph,
                tree,
                entry,
                body,
                budget,
            )?;
            symbols.stream_definition(output.program, output.definitions, entry)?;
            continue;
        }
        exception_helpers::emit_pending(output.program, symbols, output.definitions)?;
        aggregate_helpers::emit_pending(output.program, symbols, graph, output.definitions)?;
        let Some(entry) = symbols.pending.next() else {
            // Each C++ emit_demanded drain finishes its thunks before the
            // next explicit-instantiation or variable-root demand boundary.
            symbols.emit_thunks(output.program, output.definitions)?;
            if explicit == 0 {
                demand_explicit_definitions(output.program, symbols, graph)?;
                explicit = 1;
            } else if let Some(&entity) = graph.lower_demanded_variables().get(variable) {
                // Each variable root owns a complete demand drain, including
                // functions and thunks its initializer requests.
                variable += 1;
                // Host objects demand an integral member's storage through
                // an emitted address use, unless explicitly instantiated.
                // A value-only semantic demand still roots course storage.
                if symbols.host_objects() && graph.lower_value_only_variable_root(entity) {
                    continue;
                }
                symbols.variable_symbol(output.program, entity)?;
            } else {
                break;
            }
            continue;
        };
        if !entries.insert(entry) {
            continue;
        }
        if entry
            .entity(graph)
            .is_some_and(|entity| graph.lower_gnu_inline(entity))
        {
            if symbols.has_entry_symbol(entry) {
                function::declare(output.program, symbols, graph, entry)?;
            }
            continue;
        }
        let body = if let construction::Entry::Function(entity)
        | construction::Entry::DestructorBase(entity)
        | construction::Entry::DestructorDeleting(entity) = entry
        {
            if matches!(entry, construction::Entry::Function(_)) && !emitted.insert(entity) {
                continue;
            }
            let facts = graph.lower_function(entity)?;
            if facts.definition.is_none()
                && graph.lower_function_body(entity).is_none()
                && !graph.lower_has_synthesized_body(entity)
            {
                // program_lowering.cpp::demand_entry declines unavailable or
                // suppressed bodies without creating a symbol. A use still
                // publishes its declaration; ancillary destructor entry demand
                // alone does not (notably an extern-instantiated class's D2).
                if symbols.has_entry_symbol(entry) {
                    function::declare(output.program, symbols, graph, entry)?;
                }
                continue;
            }
            if facts.emission == crate::sema::lowering::Emission::Demanded {
                let symbol = symbols.entry_symbol(output.program, entry)?;
                if output.program.function_defined(symbol) {
                    continue;
                }
            }
            function::definition_body(graph, tree, entity)?
        } else {
            let construction::Entry::Constructor(id, _) = entry else {
                return Err(Error::Missing("entry kind"));
            };
            if let Some(entity) = graph.lower_constructor_entity(id)
                && !graph.lower_function_defined(entity)
            {
                if symbols.has_entry_symbol(entry) {
                    function::declare(output.program, symbols, graph, entry)?;
                }
                continue;
            }
            let facts = entry.facts(graph)?;
            let symbol = symbols.entry_symbol(output.program, entry)?;
            if facts.emission == crate::sema::lowering::Emission::Demanded
                && output.program.function_defined(symbol)
            {
                continue;
            }
            graph
                .lower_constructor_entity(id)
                .and_then(|entity| graph.lower_function_body(entity))
        };
        function::lower(
            output.function_output(),
            symbols,
            graph,
            tree,
            entry,
            body,
            budget,
        )?;
        symbols.stream_definition(output.program, output.definitions, entry)?;
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
