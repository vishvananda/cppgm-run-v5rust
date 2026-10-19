// Translated from sema/analyzer_bodies.cpp's label/goto checks. See NOTICE.
//! Body-owned transfers retain scope identities and point-of-declaration counts.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{ScopeId, ScopeKind},
};
use crate::{lex::identifiers::NameId, support::source::SourceOffset, syntax::ast};

#[derive(Clone, Copy)]
struct Transfer {
    name: NameId,
    location: SourceOffset,
    scope: ScopeId,
    initialized: u32,
}

#[derive(Default)]
pub(super) struct Transfers {
    labels: Vec<Transfer>,
    gotos: Vec<Transfer>,
}

impl Analyzer<'_> {
    pub(super) fn record_transfer(&mut self, id: ast::StmtId, scope: ScopeId) -> Result<(), Error> {
        if !self.semantic() {
            return Ok(());
        }
        let statement = self.tree.statements.get(id);
        let (ast::StmtKind::Goto(name) | ast::StmtKind::Label { name, .. }) = statement.kind else {
            return Err(Error::Invariant("transfer without label"));
        };
        let transfer = Transfer {
            name,
            location: statement.location,
            scope,
            initialized: self.graph.scope(scope).initialized,
        };
        let transfers = &mut self
            .body_context
            .as_mut()
            .ok_or(Error::Invariant("transfer outside function"))?
            .transfers;
        if matches!(statement.kind, ast::StmtKind::Goto(_)) {
            transfers.gotos.push(transfer);
        } else {
            transfers.labels.push(transfer);
        }
        Ok(())
    }

    pub(super) fn check_gotos(&mut self) -> Result<(), Error> {
        let mut transfers = std::mem::take(
            &mut self
                .body_context
                .as_mut()
                .ok_or(Error::Invariant("transfers outside function"))?
                .transfers,
        );
        transfers
            .labels
            .sort_unstable_by_key(|label| (label.name.bits(), label.location));
        if let Some(duplicate) = transfers
            .labels
            .windows(2)
            .filter(|pair| pair[0].name == pair[1].name)
            .map(|pair| pair[1])
            .min_by_key(|label| label.location)
        {
            return Err(Error::at(duplicate.location, "duplicate label"));
        }
        let mut goto_chain = Vec::new();
        let mut label_chain = Vec::new();
        for jump in transfers.gotos {
            let found = transfers
                .labels
                .binary_search_by_key(&jump.name.bits(), |label| label.name.bits())
                .map_err(|_| Error::at(jump.location, "goto names no label of the function"))?;
            let label = transfers.labels[found];
            self.block_chain(jump.scope, &mut goto_chain);
            self.block_chain(label.scope, &mut label_chain);
            let (mut g, mut l) = (goto_chain.len(), label_chain.len());
            while g > 0 && l > 0 && goto_chain[g - 1] == label_chain[l - 1] {
                g -= 1;
                l -= 1;
            }
            if label_chain[..l]
                .iter()
                .any(|&scope| self.graph.scope(scope).entry == super::graph::ScopeEntry::Exception)
            {
                return Err(Error::at(
                    jump.location,
                    "goto into a try block, handler (15p2) or statement expression",
                ));
            }
            for i in (0..label_chain.len()).rev() {
                let at_label = if i == 0 {
                    label.initialized
                } else {
                    self.graph.scope(label_chain[i - 1]).initialized_at_entry
                };
                let at_goto = if i < l {
                    0
                } else {
                    let shared = g + (i - l);
                    if shared == 0 {
                        jump.initialized
                    } else {
                        self.graph
                            .scope(goto_chain[shared - 1])
                            .initialized_at_entry
                    }
                };
                if at_label > at_goto {
                    return Err(Error::at(
                        jump.location,
                        "goto bypasses a variable's initialization",
                    ));
                }
            }
        }
        Ok(())
    }

    fn block_chain(&self, mut scope: ScopeId, chain: &mut Vec<ScopeId>) {
        chain.clear();
        while self.graph.scope(scope).kind == ScopeKind::Block {
            chain.push(scope);
            let Some(parent) = self.graph.scope(scope).parent else {
                break;
            };
            scope = parent;
        }
    }
}
