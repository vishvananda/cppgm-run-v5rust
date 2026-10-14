// Translated from lower/program_lowering.cpp. See NOTICE.
//! Typed, demand-driven source lowering into the shared function-owned `LowIR` model.
mod aggregate_helpers;
mod aggregates;
mod arrays;
mod base_projections;
mod bit_fields;
mod conditions;
mod construction;
mod conversions;
mod data;
mod definition_order;
mod expressions;
mod field_updates;
mod function;
mod globals;
mod local_storage;
mod mutations;
mod operator_calls;
mod pointers;
mod references;
mod statements;
mod symbols;
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
    symbols: symbols::SymbolTable,
}
impl Output {
    pub(crate) fn finish_entry_aliases(&mut self) {
        self.symbols.finish_entry_aliases(&mut self.program);
    }
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
    let mut work: Vec<_> = tree
        .decl_lists
        .get(declarations)
        .iter()
        .rev()
        .copied()
        .collect();
    let mut emitted = std::collections::HashSet::new();
    let mut emitted_entries = std::collections::HashSet::new();
    while let Some(id) = work.pop() {
        match tree.declarations.get(id).kind {
            ast::DeclKind::Namespace { declarations, .. }
            | ast::DeclKind::Linkage { declarations, .. } => {
                work.extend(tree.decl_lists.get(declarations).iter().rev().copied());
            }
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
                for declarator in tree.init_declarators.get(declarators) {
                    let entity = graph.lower_declared(declarator.declarator)?;
                    if graph.lower_is_variable(entity)
                        && graph.lower_variable(entity)?.definition == Some(declarator.declarator)
                    {
                        globals::define(
                            &mut output.program,
                            &mut symbols,
                            graph,
                            tree,
                            entity,
                            declarator.initializer,
                            tree.declarators.get(declarator.declarator).location,
                        )?;
                        if symbols.emit_tls_wrapper(&mut output.program, entity)? {
                            output.definitions.stream(&mut output.program, None)?;
                        }
                    }
                }
            }
            ast::DeclKind::Empty
            | ast::DeclKind::Alias { .. }
            | ast::DeclKind::NamespaceAlias { .. }
            | ast::DeclKind::Enum { .. }
            | ast::DeclKind::Using { .. }
            | ast::DeclKind::StaticAssert { .. }
            | ast::DeclKind::Class(_) => {}
            _ => return Err(Error::Missing("class and template declaration lowering")),
        }
    }
    let mut next = 0;
    aggregate_helpers::emit_pending(
        &mut output.program,
        &mut symbols,
        graph,
        &mut output.definitions,
    )?;
    while let Some(&entry) = symbols.pending.get(next) {
        next += 1;
        if !emitted_entries.insert(entry) {
            continue;
        }
        if let construction::Entry::Function(entity) = entry {
            if !emitted.insert(entity) {
                continue;
            }
            let facts = graph.lower_function(entity)?;
            let Some(declaration) = facts.definition else {
                function::declare(&mut output.program, &mut symbols, graph, entry)?;
                continue;
            };
            if facts.emission == crate::sema::lowering::Emission::Demanded {
                let symbol = symbols.entry_symbol(&mut output.program, entry)?;
                if output.program.function_defined(symbol) {
                    continue;
                }
            }
            let body = match tree.declarations.get(declaration).kind {
                ast::DeclKind::Function { body, .. } => Some(body),
                ast::DeclKind::SpecialMember { .. } => graph.lower_function_body(entity),
                _ => return Err(Error::Missing("special function definition lowering")),
            };
            function::lower(
                &mut output.program,
                &mut symbols,
                graph,
                tree,
                entry,
                body,
                budget,
            )?;
        } else {
            let construction::Entry::Constructor(id, _) = entry else {
                return Err(Error::Missing("entry kind"));
            };
            if let Some(entity) = graph.lower_constructor_entity(id)
                && !graph.lower_function_defined(entity)
            {
                function::declare(&mut output.program, &mut symbols, graph, entry)?;
                continue;
            }
            let facts = entry.facts(graph)?;
            let symbol = symbols.entry_symbol(&mut output.program, entry)?;
            if facts.emission == crate::sema::lowering::Emission::Demanded
                && output.program.function_defined(symbol)
            {
                continue;
            }
            let body = graph
                .lower_constructor_entity(id)
                .and_then(|entity| graph.lower_function_body(entity));
            function::lower(
                &mut output.program,
                &mut symbols,
                graph,
                tree,
                entry,
                body,
                budget,
            )?;
        }
        symbols.stream_definition(&mut output.program, &mut output.definitions, entry)?;
        aggregate_helpers::emit_pending(
            &mut output.program,
            &mut symbols,
            graph,
            &mut output.definitions,
        )?;
    }
    aggregate_helpers::emit_pending(
        &mut output.program,
        &mut symbols,
        graph,
        &mut output.definitions,
    )?;
    globals::finish(&mut output.program, &mut symbols, graph)?;
    if let Some(symbol) = symbols.empty_initialization {
        output.initializer = Some(symbol);
    }
    Ok(())
}
