// Translated from lower/program_lowering.cpp's source-unit ownership. See NOTICE.
//! Exclusive frontend access between short symbol queries and evaluation demands.
use super::symbols::State;
use crate::{
    sema::Graph,
    syntax::{ast, cursor::Cursor},
};
pub(super) struct Context<'front, 'source, 'state> {
    pub graph: &'front mut Graph,
    pub tree: &'front mut ast::Tree,
    pub cursor: &'front mut Cursor<'source>,
    pub symbols: &'state mut State,
}
impl<'source> Context<'_, 'source, '_> {
    pub(super) fn borrow(&mut self) -> Context<'_, 'source, '_> {
        Context {
            graph: self.graph,
            tree: self.tree,
            cursor: self.cursor,
            symbols: self.symbols,
        }
    }
}
