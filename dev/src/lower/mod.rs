// Translated from lower/program_lowering.cpp. See NOTICE.
//! Typed, demand-driven source lowering into the shared function-owned `LowIR` model.
mod aggregate_helpers;
mod aggregates;
mod arrays;
mod base_projections;
mod bit_fields;
mod class_parameters;
mod conditions;
mod construction;
mod constructor_cleanup;
mod conversions;
mod data;
mod definition_order;
mod destruction;
mod exception_helpers;
mod expressions;
mod field_updates;
mod floating_intrinsics;
mod function;
mod globals;
mod lifetime;
mod lifetime_regions;
mod lifetime_scan;
mod literal_calls;
mod local_storage;
mod materialization;
mod mutations;
mod object_arrays;
mod operator_calls;
mod pointers;
mod references;
mod shutdown;
mod startup;
mod statements;
mod symbols;
mod thread_local;
mod throw_scan;
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
    exception_helpers::emit_pending(&mut output.program, &mut symbols, &mut output.definitions)?;
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
        let body = if let construction::Entry::Function(entity)
        | construction::Entry::DestructorBase(entity) = entry
        {
            if matches!(entry, construction::Entry::Function(_)) && !emitted.insert(entity) {
                continue;
            }
            let facts = graph.lower_function(entity)?;
            let declaration = facts.definition;
            if declaration.is_none() && !graph.lower_implicit_destructor(entity) {
                function::declare(&mut output.program, &mut symbols, graph, entry)?;
                continue;
            }
            if facts.emission == crate::sema::lowering::Emission::Demanded {
                let symbol = symbols.entry_symbol(&mut output.program, entry)?;
                if output.program.function_defined(symbol) {
                    continue;
                }
            }
            match declaration.map(|id| &tree.declarations.get(id).kind) {
                Some(ast::DeclKind::Function { body, .. }) => Some(*body),
                Some(ast::DeclKind::SpecialMember { .. }) => graph.lower_function_body(entity),
                None => None,
                _ => return Err(Error::Missing("special function definition lowering")),
            }
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
            graph
                .lower_constructor_entity(id)
                .and_then(|entity| graph.lower_function_body(entity))
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
        symbols.stream_definition(&mut output.program, &mut output.definitions, entry)?;
        exception_helpers::emit_pending(
            &mut output.program,
            &mut symbols,
            &mut output.definitions,
        )?;
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
    output.definitions.release_held();
    globals::finish(&mut output.program, &mut symbols, graph)?;
    if let Some(symbol) = symbols.initialization {
        output.initializer = Some(symbol);
    }
    Ok(())
}
