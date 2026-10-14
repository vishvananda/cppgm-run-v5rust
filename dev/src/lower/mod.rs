// Translated from lower/program_lowering.cpp. See NOTICE.
//! Typed, demand-driven source lowering into the shared function-owned `LowIR` model.
mod conditions;
mod conversions;
mod expressions;
mod function;
mod mutations;
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
    pub definitions: Vec<u8>,
}
pub(crate) fn lower_unit(
    output: &mut Output,
    graph: &Graph,
    tree: &ast::Tree,
    cursor: &Cursor<'_>,
    declarations: List<ast::DeclId>,
    budget: StackBudget,
) -> Result<(), Error> {
    let mut symbols = symbols::Symbols::new(graph, cursor, budget);
    let mut work: Vec<_> = tree
        .decl_lists
        .get(declarations)
        .iter()
        .rev()
        .copied()
        .collect();
    let mut emitted = std::collections::HashSet::new();
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
                        entity,
                        body,
                        budget,
                    )?;
                    output
                        .program
                        .stream_last_definition(&mut output.definitions)?;
                }
            }
            ast::DeclKind::Simple { declarators, .. } => {
                for declarator in tree.init_declarators.get(declarators) {
                    let entity = graph.lower_declared(declarator.declarator)?;
                    if graph.lower_is_variable(entity) {
                        return Err(Error::Missing("namespace variable lowering"));
                    }
                }
            }
            ast::DeclKind::Empty
            | ast::DeclKind::Alias { .. }
            | ast::DeclKind::Enum { .. }
            | ast::DeclKind::Using { .. }
            | ast::DeclKind::StaticAssert { .. } => {}
            _ => return Err(Error::Missing("class and template declaration lowering")),
        }
    }
    let mut next = 0;
    while let Some(&entity) = symbols.pending.get(next) {
        next += 1;
        if !emitted.insert(entity) {
            continue;
        }
        let declaration = graph
            .lower_function(entity)?
            .definition
            .ok_or(Error::Missing("external function declaration lowering"))?;
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(declaration).kind else {
            return Err(Error::Missing("special function definition lowering"));
        };
        function::lower(
            &mut output.program,
            &mut symbols,
            graph,
            tree,
            entity,
            body,
            budget,
        )?;
        output
            .program
            .stream_last_definition(&mut output.definitions)?;
    }
    Ok(())
}
