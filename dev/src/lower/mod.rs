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
mod base_initialization;
mod base_projections;
mod bit_fields;
mod class_conditionals;
mod class_parameters;
mod class_results;
mod conditions;
mod constant_demands;
mod construction;
mod constructor_cleanup;
mod conversion_functions;
mod conversions;
mod copy_arrays;
mod copy_cleanup;
mod copy_construction;
mod data;
mod definition_demands;
mod definition_order;
mod destruction;
mod dispatch;
mod element_temporaries;
mod exception_helpers;
mod expressions;
mod field_updates;
mod floating_intrinsics;
mod function;
mod globals;
mod lexical_cleanup;
mod lifetime;
mod lifetime_regions;
mod lifetime_scan;
mod literal_calls;
mod local_storage;
mod materialization;
mod member_initialization;
mod mutations;
mod object_arrays;
mod operator_calls;
mod pointers;
mod reference_lifetimes;
mod references;
mod runtime_roles;
mod shutdown;
mod startup;
mod statements;
mod symbols;
mod temporary_scan;
mod thread_local;
mod throw_scan;
mod tracked_lifetimes;
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
        Ok(())
    }
    pub(crate) fn finish_entry_aliases(&mut self) {
        self.symbols.finish_entry_aliases(&mut self.program);
    }
}
fn finish_unit(
    program: &mut Program,
    definitions: &mut definition_order::Definitions,
    finalizers: &mut Vec<shutdown::Finalizer>,
    symbols: &mut symbols::Symbols<'_, '_>,
    graph: &Graph,
) -> Result<Option<crate::lowir::model::SymbolId>, Error> {
    aggregate_helpers::emit_pending(program, symbols, graph, definitions)?;
    definitions.release_held();
    globals::finish(program, symbols, graph)?;
    symbols.resolve_runtime_roles(program, definitions)?;
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
    for function in graph.lower_argument_functions(None) {
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
    exception_helpers::emit_pending(output.program, symbols, output.definitions)?;
    aggregate_helpers::emit_pending(output.program, symbols, graph, output.definitions)?;
    let mut explicit = 0;
    loop {
        globals::emit_instance_variables(output, symbols, (graph, tree))?;
        let Some(entry) = symbols.pending.next() else {
            if explicit == 2 {
                break;
            }
            if explicit == 0 {
                for entity in graph.lower_explicit_functions() {
                    if graph.lower_function_defined(entity) {
                        symbols.demand(entity);
                    }
                }
                for &class in graph.lower_explicit_classes() {
                    if graph.lower_polymorphic(class) {
                        symbols.vtable_symbol(output.program, class)?;
                    }
                }
            } else {
                for &entity in graph.lower_demanded_variables() {
                    symbols.variable_symbol(output.program, entity)?;
                }
            }
            explicit += 1;
            continue;
        };
        if !entries.insert(entry) {
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
                function::declare(output.program, symbols, graph, entry)?;
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
                function::declare(output.program, symbols, graph, entry)?;
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
        exception_helpers::emit_pending(output.program, symbols, output.definitions)?;
        aggregate_helpers::emit_pending(output.program, symbols, graph, output.definitions)?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) use reference_lifetimes::tests::assert_extended_ownership;

#[cfg(test)]
pub(crate) use temporary_scan::tests::assert_constructor_parameter_temporaries;

#[cfg(test)]
pub(crate) use symbols::assert_table_demand;
