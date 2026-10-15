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
        initializer: Option<ast::Initializer>,
        live: Option<LiveId>,
    ) -> Result<Option<LiveId>, Error> {
        if self.graph.lower_is_variable(entity)
            && self.graph.lower_automatic(entity)
            && (self.automatic_destructor(entity)?.is_some()
                || self.initializer_extends_temporary(initializer)?)
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
                live = self.declared_live(
                    self.graph.lower_declared(decl.declarator)?,
                    decl.initializer,
                    live,
                )?;
            }
        }
        Ok(live)
    }
    fn condition_live(
        &mut self,
        condition: Condition,
        live: Option<LiveId>,
    ) -> Result<Option<LiveId>, Error> {
        if let Condition::Declaration {
            declarator,
            initializer,
            ..
        } = condition
        {
            return self.declared_live(
                self.graph.lower_declared(declarator)?,
                Some(initializer),
                live,
            );
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
                    .is_some_and(|cleanup| cleanup.variable == Some(live.variable))
            {
                depth = found;
                break;
            }
            node = live.parent;
        }
        self.destroy_cleanups(depth)
    }
}

// Translated from lexical_cleanup_lowering.cpp::body_unwinds/detached_body.
impl Function<'_, '_, '_, '_> {
    fn declaration_unwinds(
        &mut self,
        declaration: ast::DeclId,
        alive: &mut bool,
    ) -> Result<bool, Error> {
        let ast::DeclKind::Simple { declarators, .. } =
            self.tree.declarations.get(declaration).kind
        else {
            return Ok(false);
        };
        for init in self.tree.init_declarators.get(declarators) {
            let entity = self.graph.lower_declared(init.declarator)?;
            if *alive && self.variable_throws(entity, init.initializer)? {
                return Ok(true);
            }
            if !self.graph.lower_is_variable(entity) || !self.graph.lower_automatic(entity) {
                continue;
            }
            let ty = self.graph.lower_entity_type(entity)?;
            if matches!(
                self.graph.lower_type(ty)?,
                crate::sema::lowering::TypeView::Reference { .. }
            ) {
                *alive |= self.initializer_extends_temporary(init.initializer)?;
                continue;
            }
            if let Some(destructor) = self.symbols.class_destructor(ty)? {
                if !self.graph.lower_function(destructor)?.nothrow {
                    return Ok(true);
                }
                *alive |= self.automatic_destructor(entity)?.is_some();
            }
        }
        Ok(false)
    }
    pub(super) fn body_unwinds(
        &mut self,
        id: ast::StmtId,
        alive: &mut bool,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        match self.tree.statements.get(id).kind {
            StmtKind::Compound(items) => {
                let mut inner = *alive;
                for &item in self.tree.stmt_lists.get(items) {
                    if self.body_unwinds(item, &mut inner)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            StmtKind::Ambiguous { .. } => self.body_unwinds(self.graph.lower_statement(id)?, alive),
            StmtKind::Declaration(declaration) => self.declaration_unwinds(declaration, alive),
            StmtKind::Try { .. } => Ok(true),
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if *alive && self.condition_throws(condition)? {
                    return Ok(true);
                }
                let known = if let Condition::Expression(expr) = condition {
                    self.literal_truth(expr)?
                } else {
                    None
                };
                let mut then_alive = *alive;
                let mut else_alive = *alive;
                Ok(
                    known != Some(false) && self.body_unwinds(then_branch, &mut then_alive)?
                        || known != Some(true)
                            && else_branch
                                .map(|branch| self.body_unwinds(branch, &mut else_alive))
                                .transpose()?
                                .unwrap_or(false),
                )
            }
            StmtKind::ConditionalLoop {
                condition, body, ..
            } => {
                if *alive && self.condition_throws(condition)? {
                    return Ok(true);
                }
                self.body_unwinds(body, &mut { *alive })
            }
            StmtKind::Do { condition, body } => Ok(*alive && self.expression_throws(condition)?
                || self.body_unwinds(body, &mut { *alive })?),
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => {
                let mut inner = *alive;
                if let Some(decl) = declaration
                    && self.declaration_unwinds(decl, &mut inner)?
                {
                    return Ok(true);
                }
                if inner
                    && (init
                        .map(|id| self.expression_throws(id))
                        .transpose()?
                        .unwrap_or(false)
                        || iteration
                            .map(|id| self.expression_throws(id))
                            .transpose()?
                            .unwrap_or(false)
                        || condition
                            .map(|condition| self.condition_throws(condition))
                            .transpose()?
                            .unwrap_or(false))
                {
                    return Ok(true);
                }
                self.body_unwinds(body, &mut inner)
            }
            StmtKind::Case { body, .. }
            | StmtKind::Default(body)
            | StmtKind::Label { body, .. } => self.body_unwinds(body, alive),
            _ => Ok(*alive && self.statement_throws(id)?),
        }
    }
}
