// Translated from lower/program_lowering.cpp. See NOTICE.
//! Typed, demand-driven source lowering into the shared function-owned `LowIR` model.
mod aggregate_arrays;
mod aggregate_cleanup;
mod aggregate_helpers;
mod aggregates;
mod allocation;
mod allocation_runtime;
mod anonymous_names;
mod array_allocation;
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
mod construction;
mod constructor_cleanup;
mod conversions;
mod copy_arrays;
mod copy_cleanup;
mod copy_construction;
mod data;
mod definition_order;
mod destruction;
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
    symbols.resolve_allocation_roles(program, definitions)?;
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
    let mut declarations = anonymous_names::Declarations::new(tree, declarations);
    let mut emitted = std::collections::HashSet::new();
    let mut emitted_entries = std::collections::HashSet::new();
    while let Some(id) = declarations.next(tree, graph, &mut symbols)? {
        globals::lower_anonymous_declaration(
            &mut globals::GlobalOutput {
                program: &mut output.program,
                startup: &mut output.startup,
                definitions: &mut output.definitions,
                finalizers: &mut output.finalizers,
            },
            &mut symbols,
            (graph, tree),
            id,
            budget,
        )?;
        match tree.declarations.get(id).kind {
            ast::DeclKind::Function {
                declarator, body, ..
            } => {
                let entity = graph.lower_declared(declarator)?;
                if graph.lower_function(entity)?.emission == crate::sema::lowering::Emission::Always
                    && emitted.insert(entity)
                {
                    function::lower(
                        &mut output.program,
                        &mut symbols,
                        graph,
                        tree,
                        construction::Entry::Function(entity),
                        Some(body),
                        budget,
                    )?;
                    symbols.stream_definition(
                        &mut output.program,
                        &mut output.definitions,
                        construction::Entry::Function(entity),
                    )?;
                }
            }
            ast::DeclKind::SpecialMember { declarator, .. } => {
                let entity = graph.lower_declared(declarator)?;
                let facts = graph.lower_function(entity)?;
                if matches!(facts.name, sema::lowering::FunctionName::Destructor(_)) {
                    if facts.emission == sema::lowering::Emission::Always && emitted.insert(entity)
                    {
                        let base = construction::Entry::DestructorBase(entity);
                        emitted_entries.insert(base);
                        function::lower(
                            &mut output.program,
                            &mut symbols,
                            graph,
                            tree,
                            base,
                            graph.lower_function_body(entity),
                            budget,
                        )?;
                        symbols.stream_definition(
                            &mut output.program,
                            &mut output.definitions,
                            base,
                        )?;
                        let entry = construction::Entry::Function(entity);
                        function::lower(
                            &mut output.program,
                            &mut symbols,
                            graph,
                            tree,
                            entry,
                            graph.lower_function_body(entity),
                            budget,
                        )?;
                        symbols.stream_definition(
                            &mut output.program,
                            &mut output.definitions,
                            entry,
                        )?;
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
                            &mut output.program,
                            &mut symbols,
                            graph,
                            tree,
                            construction::Entry::Constructor(id, variant),
                            graph.lower_function_body(entity),
                            budget,
                        )?;
                        symbols.stream_definition(
                            &mut output.program,
                            &mut output.definitions,
                            entry,
                        )?;
                    }
                }
            }
            ast::DeclKind::Simple { declarators, .. } => {
                assignment::lower_definitions(
                    assignment::DefinitionOutput {
                        program: &mut output.program,
                        definitions: &mut output.definitions,
                        emitted: &mut emitted,
                    },
                    &mut symbols,
                    graph,
                    tree,
                    declarators,
                    budget,
                )?;
                globals::lower_declaration(
                    &mut globals::GlobalOutput {
                        program: &mut output.program,
                        startup: &mut output.startup,
                        definitions: &mut output.definitions,
                        finalizers: &mut output.finalizers,
                    },
                    &mut symbols,
                    graph,
                    tree,
                    declarators,
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
            | ast::DeclKind::Class(_) => {}
            _ => return Err(Error::Missing("class and template declaration lowering")),
        }
    }
    for id in graph.lower_root_delegations() {
        symbols.demand_entry(construction::Entry::Constructor(
            id,
            construction::Variant::Complete,
        ));
    }
    emit_pending(
        (&mut output.program, &mut output.definitions),
        &mut symbols,
        (graph, tree),
        (emitted, emitted_entries),
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

// The unit's demand worklist owns emission identity; each emitted body is
// streamed and released before the next one is lowered.
fn emit_pending<'a>(
    (program, definitions): (&mut Program, &mut definition_order::Definitions),
    symbols: &mut symbols::Symbols<'a, '_>,
    (graph, tree): (&'a Graph, &'a ast::Tree),
    (mut emitted, mut entries): (
        std::collections::HashSet<sema::lowering::EntityId>,
        std::collections::HashSet<construction::Entry>,
    ),
    budget: StackBudget,
) -> Result<(), Error> {
    let mut next = 0;
    exception_helpers::emit_pending(program, symbols, definitions)?;
    aggregate_helpers::emit_pending(program, symbols, graph, definitions)?;
    while let Some(&entry) = symbols.pending.get(next) {
        next += 1;
        if !entries.insert(entry) {
            continue;
        }
        let body = if let construction::Entry::Function(entity)
        | construction::Entry::DestructorBase(entity) = entry
        {
            if matches!(entry, construction::Entry::Function(_)) && !emitted.insert(entity) {
                continue;
            }
            let facts = graph.lower_function(entity)?;
            if facts.definition.is_none() && !graph.lower_has_synthesized_body(entity) {
                function::declare(program, symbols, graph, entry)?;
                continue;
            }
            if facts.emission == crate::sema::lowering::Emission::Demanded {
                let symbol = symbols.entry_symbol(program, entry)?;
                if program.function_defined(symbol) {
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
                function::declare(program, symbols, graph, entry)?;
                continue;
            }
            let facts = entry.facts(graph)?;
            let symbol = symbols.entry_symbol(program, entry)?;
            if facts.emission == crate::sema::lowering::Emission::Demanded
                && program.function_defined(symbol)
            {
                continue;
            }
            graph
                .lower_constructor_entity(id)
                .and_then(|entity| graph.lower_function_body(entity))
        };
        function::lower(program, symbols, graph, tree, entry, body, budget)?;
        symbols.stream_definition(program, definitions, entry)?;
        exception_helpers::emit_pending(program, symbols, definitions)?;
        aggregate_helpers::emit_pending(program, symbols, graph, definitions)?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) use reference_lifetimes::tests::assert_extended_ownership;
