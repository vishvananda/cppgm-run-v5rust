// Translated from lower/statement_lowering.cpp::count_returns/declared_live/goto_statement.
// See NOTICE.
//! One lazy syntax walk records persistent live-object chains and shared-return counts.
use super::{Error, function::Function, lifetime::LiveId};
use crate::syntax::ast::{self, Condition, StmtKind};
impl Function<'_, '_, '_, '_> {
    pub(super) fn scan_lifetimes(&mut self) -> Result<(), Error> {
        if matches!(self.lifetime.scan, super::lifetime::ScanState::Complete)
            || self.lifetime.cleanups.is_empty()
        {
            return Ok(());
        }
        self.lifetime.scan = super::lifetime::ScanState::Complete;
        if let Some(body) = self.lifetime.body {
            self.count_live(body, None)?;
        }
        Ok(())
    }
    fn declared_live(
        &mut self,
        entity: crate::sema::lowering::EntityId,
        live: Option<LiveId>,
    ) -> Result<Option<LiveId>, Error> {
        if self.graph.lower_is_variable(entity)
            && self.graph.lower_automatic(entity)
            && self.automatic_destructor(entity)?.is_some()
        {
            return Ok(Some(self.lifetime.add_live(entity, live)?));
        }
        Ok(live)
    }
    fn declaration_live(
        &mut self,
        decl: ast::DeclId,
        mut live: Option<LiveId>,
    ) -> Result<Option<LiveId>, Error> {
        if let ast::DeclKind::Simple { declarators, .. } = self.tree.declarations.get(decl).kind {
            for decl in self.tree.init_declarators.get(declarators) {
                live = self.declared_live(self.graph.lower_declared(decl.declarator)?, live)?;
            }
        }
        Ok(live)
    }
    fn condition_live(
        &mut self,
        condition: Condition,
        live: Option<LiveId>,
    ) -> Result<Option<LiveId>, Error> {
        if let Condition::Declaration { declarator, .. } = condition {
            return self.declared_live(self.graph.lower_declared(declarator)?, live);
        }
        Ok(live)
    }
    fn count_live(
        &mut self,
        id: ast::StmtId,
        live: Option<LiveId>,
    ) -> Result<Option<LiveId>, Error> {
        self.check_stack()?;
        match self.tree.statements.get(id).kind {
            StmtKind::Compound(items) => {
                let mut inner = live;
                for &item in self.tree.stmt_lists.get(items) {
                    inner = self.count_live(item, inner)?;
                }
            }
            StmtKind::Ambiguous { .. } => {
                return self.count_live(self.graph.lower_statement(id)?, live);
            }
            StmtKind::Declaration(decl) => return self.declaration_live(decl, live),
            StmtKind::Return(_) => {
                if let Some(live) = live {
                    let variable = self.lifetime.live_node(live).variable;
                    let count = self.lifetime.returns.entry(variable).or_default();
                    *count = count.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let inner = self.condition_live(condition, live)?;
                self.count_live(then_branch, inner)?;
                if let Some(body) = else_branch {
                    self.count_live(body, inner)?;
                }
            }
            StmtKind::ConditionalLoop {
                condition, body, ..
            } => {
                let inner = self.condition_live(condition, live)?;
                self.count_live(body, inner)?;
            }
            StmtKind::For {
                declaration,
                condition,
                body,
                ..
            } => {
                let mut inner = if let Some(decl) = declaration {
                    self.declaration_live(decl, live)?
                } else {
                    live
                };
                if let Some(condition) = condition {
                    inner = self.condition_live(condition, inner)?;
                }
                self.count_live(body, inner)?;
            }
            StmtKind::Label { name, body } => {
                self.lifetime.labels.insert(name, live);
                return self.count_live(body, live);
            }
            StmtKind::Case { body, .. } | StmtKind::Default(body) => {
                return self.count_live(body, live);
            }
            StmtKind::Do { body, .. } => {
                self.count_live(body, live)?;
            }
            _ => {}
        }
        Ok(live)
    }
    pub(super) fn goto_cleanup(
        &mut self,
        name: crate::lex::identifiers::NameId,
    ) -> Result<(), Error> {
        self.scan_lifetimes()?;
        let mut node = self.lifetime.labels.get(&name).copied().flatten();
        let mut depth = 0;
        while let Some(id) = node {
            let live = self.lifetime.live_node(id);
            if let Some(&found) = self.lifetime.depths.get(&live.variable)
                && self
                    .lifetime
                    .cleanups
                    .get(found - 1)
                    .is_some_and(|cleanup| cleanup.variable == live.variable)
            {
                depth = found;
                break;
            }
            node = live.parent;
        }
        self.destroy_cleanups(depth)
    }
}
