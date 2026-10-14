// Translated from lower/statement_lowering.cpp. See NOTICE.
//! Procedural CFG order and function-local jump targets consume analyzed syntax.
use super::{Error, conversions::Context, function::Function, types};
use crate::{
    lex::vocabulary::Keyword,
    lowir::model::{BlockId, InstructionKind as I},
    syntax::ast::{self, Condition, StmtKind},
};

pub(super) struct JumpTargets {
    pub end: BlockId,
    pub continuation: Option<BlockId>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn new_block(&mut self, prefix: &str) -> Result<BlockId, Error> {
        let name = self.block_name(prefix)?;
        Ok(self.builder.block_id(&name)?)
    }
    pub(super) fn place(&mut self, block: BlockId) -> Result<(), Error> {
        self.builder.place_block(block)?;
        self.terminated = false;
        Ok(())
    }
    pub(super) fn jump(&mut self, block: BlockId) -> Result<(), Error> {
        self.effect(I::Jump(block))?;
        self.terminated = true;
        Ok(())
    }
    pub(super) fn statement(&mut self, id: ast::StmtId) -> Result<(), Error> {
        self.check_stack()?;
        if self.terminated && !self.reachable_label(id)? {
            let chosen = if matches!(
                self.tree.statements.get(id).kind,
                StmtKind::Ambiguous { .. }
            ) {
                self.graph.lower_statement(id)?
            } else {
                id
            };
            if let StmtKind::Declaration(declaration) = self.tree.statements.get(chosen).kind
                && let ast::DeclKind::Simple { declarators, .. } =
                    self.tree.declarations.get(declaration).kind
            {
                // A later label can enter the scope of an uninitialized object.
                // Preserve its storage without evaluating an unreachable initializer.
                for declarator in self.tree.init_declarators.get(declarators) {
                    self.local_variable(declarator.declarator, None)?;
                }
            }
            return Ok(());
        }
        match self.tree.statements.get(id).kind {
            StmtKind::Compound(statements) => {
                for &id in self.tree.stmt_lists.get(statements) {
                    self.statement(id)?;
                }
            }
            StmtKind::Return(operand) => {
                let ty = types::storage(self.graph, self.result)?;
                let value = if let Some(id) = operand {
                    if matches!(
                        self.graph.lower_type(self.result)?,
                        crate::sema::lowering::TypeView::Reference { .. }
                    ) {
                        Some(self.bind_reference(id, self.result, Context::Return)?)
                    } else {
                        let value = self.operand(id, Context::Return)?;
                        if ty == crate::lowir::vocabulary::Type::Void {
                            None
                        } else {
                            Some(self.fit(value, self.result)?.value)
                        }
                    }
                } else {
                    None
                };
                self.effect(I::Return(ty, value))?;
                self.terminated = true;
            }
            StmtKind::Expression(expression) => {
                if let Some(id) = expression {
                    self.discard(id)?;
                }
            }
            StmtKind::Declaration(id) => self.declaration(id)?,
            StmtKind::Ambiguous { .. } => self.statement(self.graph.lower_statement(id)?)?,
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.if_statement(condition, then_branch, else_branch)?;
            }
            StmtKind::ConditionalLoop {
                keyword: Keyword::While,
                condition,
                body,
            } => {
                self.while_statement(condition, body)?;
            }
            StmtKind::ConditionalLoop {
                keyword: Keyword::Switch,
                condition,
                body,
            } => {
                self.switch_statement(condition, body)?;
            }
            StmtKind::Do { body, condition } => self.do_statement(body, condition)?,
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => {
                self.for_statement(declaration, init, condition, iteration, body)?;
            }
            StmtKind::Break => {
                let target = self
                    .jumps
                    .last()
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "break without target",
                    )))?
                    .end;
                self.jump(target)?;
            }
            StmtKind::Continue => {
                let target =
                    self.jumps
                        .iter()
                        .rev()
                        .find_map(|t| t.continuation)
                        .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                            "continue without target",
                        )))?;
                self.jump(target)?;
            }
            StmtKind::Case { body, .. } | StmtKind::Default(body) => {
                let target = *self.switch_blocks.get(&id).ok_or(Error::Semantic(
                    crate::sema::Error::Invariant("case without block"),
                ))?;
                if !self.terminated {
                    self.jump(target)?;
                }
                self.place(target)?;
                self.statement(body)?;
            }
            StmtKind::Goto(name) => {
                let target = self.goto_label(name)?;
                self.jump(target)?;
            }
            StmtKind::Label { name, body } => {
                let target = self.goto_label(name)?;
                if !self.terminated {
                    self.jump(target)?;
                }
                self.place(target)?;
                self.statement(body)?;
            }
            _ => return Err(Error::Missing("extended procedural statement lowering")),
        }
        Ok(())
    }
    fn goto_label(&mut self, name: crate::lex::identifiers::NameId) -> Result<BlockId, Error> {
        if let Some(&block) = self.labels.get(&name) {
            return Ok(block);
        }
        let block = self.new_block("goto")?;
        self.labels.insert(name, block);
        Ok(block)
    }
    fn if_statement(
        &mut self,
        condition: Condition,
        then_branch: ast::StmtId,
        else_branch: Option<ast::StmtId>,
    ) -> Result<(), Error> {
        let then_block = self.new_block("if_then")?;
        let else_block = self.new_block("if_else")?;
        self.condition_branch(condition, then_block, else_block, true)?;
        self.place(then_block)?;
        self.statement(then_branch)?;
        let mut end = None;
        if !self.terminated {
            let block = self.new_block("if_end")?;
            end = Some(block);
            self.jump(block)?;
        }
        self.place(else_block)?;
        if let Some(id) = else_branch {
            self.statement(id)?;
        }
        if !self.terminated {
            let block = if let Some(block) = end {
                block
            } else {
                self.new_block("if_end")?
            };
            end = Some(block);
            self.jump(block)?;
        }
        if let Some(block) = end {
            self.place(block)?;
        }
        Ok(())
    }
    fn while_statement(&mut self, condition: Condition, body: ast::StmtId) -> Result<(), Error> {
        let test = self.new_block("while_cond")?;
        let run = self.new_block("while_body")?;
        let end = self.new_block("while_end")?;
        self.jump(test)?;
        self.place(test)?;
        self.condition_branch(condition, run, end, false)?;
        self.place(run)?;
        self.jumps.push(JumpTargets {
            end,
            continuation: Some(test),
        });
        self.statement(body)?;
        self.jumps.pop();
        if !self.terminated {
            self.jump(test)?;
        }
        self.place(end)
    }
    fn do_statement(&mut self, body: ast::StmtId, condition: ast::ExprId) -> Result<(), Error> {
        let run = self.new_block("do_body")?;
        let test = self.new_block("do_cond")?;
        let end = self.new_block("do_end")?;
        self.jump(run)?;
        self.place(run)?;
        self.jumps.push(JumpTargets {
            end,
            continuation: Some(test),
        });
        self.statement(body)?;
        self.jumps.pop();
        if !self.terminated {
            self.jump(test)?;
        }
        self.place(test)?;
        self.condition_branch(Condition::Expression(condition), run, end, false)?;
        self.place(end)
    }
    fn for_statement(
        &mut self,
        declaration: Option<ast::DeclId>,
        init: Option<ast::ExprId>,
        condition: Option<Condition>,
        iteration: Option<ast::ExprId>,
        body: ast::StmtId,
    ) -> Result<(), Error> {
        if let Some(id) = declaration {
            self.declaration(id)?;
        } else if let Some(id) = init {
            self.discard(id)?;
        }
        let test = self.new_block("for_cond")?;
        let run = self.new_block("for_body")?;
        let step = self.new_block("for_iter")?;
        let end = self.new_block("for_end")?;
        self.jump(test)?;
        self.place(test)?;
        if let Some(condition) = condition {
            self.condition_branch(condition, run, end, false)?;
        } else {
            self.jump(run)?;
        }
        self.place(run)?;
        self.jumps.push(JumpTargets {
            end,
            continuation: Some(step),
        });
        self.statement(body)?;
        self.jumps.pop();
        if !self.terminated {
            self.jump(step)?;
        }
        self.place(step)?;
        if let Some(id) = iteration {
            self.discard(id)?;
        }
        self.jump(test)?;
        self.place(end)
    }
    fn switch_statement(&mut self, condition: Condition, body: ast::StmtId) -> Result<(), Error> {
        let selector = self.condition_value(condition, false)?;
        let dispatch = self.new_block("switch_dispatch")?;
        let end = self.new_block("switch_end")?;
        let mut labels = Vec::new();
        self.scan_switch(body, &mut labels)?;
        let mut default = end;
        let mut cases = Vec::new();
        for &id in &labels {
            let block = self.switch_blocks[&id];
            if matches!(self.tree.statements.get(id).kind, StmtKind::Default(_)) {
                default = block;
            } else {
                // The C++ producer's integer literal sign-extends its 64-bit
                // pattern into LowIR's full literal, including unsigned cases.
                let bits = i64::from_ne_bytes(self.graph.lower_case(id)?.to_ne_bytes());
                let value = self
                    .builder
                    .integer(u128::from_ne_bytes(i128::from(bits).to_ne_bytes()))?;
                cases.push((value, block));
            }
        }
        self.jump(dispatch)?;
        self.place(dispatch)?;
        self.builder
            .effect(I::Switch(selector.value, default, cases))?;
        self.terminated = true;
        self.jumps.push(JumpTargets {
            end,
            continuation: None,
        });
        self.statement(body)?;
        self.jumps.pop();
        if !self.terminated {
            self.jump(end)?;
        }
        self.place(end)?;
        for id in labels {
            self.switch_blocks.remove(&id);
        }
        Ok(())
    }
    fn scan_switch(&mut self, id: ast::StmtId, labels: &mut Vec<ast::StmtId>) -> Result<(), Error> {
        self.check_stack()?;
        match self.tree.statements.get(id).kind {
            StmtKind::Case { body, .. } | StmtKind::Default(body) => {
                let prefix = if matches!(self.tree.statements.get(id).kind, StmtKind::Default(_)) {
                    "switch_default"
                } else {
                    "switch_case"
                };
                let block = self.new_block(prefix)?;
                self.switch_blocks.insert(id, block);
                labels.push(id);
                self.scan_switch(body, labels)?;
            }
            StmtKind::Compound(list) => {
                for &id in self.tree.stmt_lists.get(list) {
                    self.scan_switch(id, labels)?;
                }
            }
            StmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.scan_switch(then_branch, labels)?;
                if let Some(id) = else_branch {
                    self.scan_switch(id, labels)?;
                }
            }
            StmtKind::ConditionalLoop {
                keyword: Keyword::While,
                body,
                ..
            }
            | StmtKind::Do { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Label { body, .. } => self.scan_switch(body, labels)?,
            StmtKind::Ambiguous { .. } => {
                self.scan_switch(self.graph.lower_statement(id)?, labels)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn reachable_label(&mut self, id: ast::StmtId) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(&found) = self.label_memo.get(&id) {
            return Ok(found);
        }
        let found = match self.tree.statements.get(id).kind {
            StmtKind::Case { .. } | StmtKind::Default(_) | StmtKind::Label { .. } => true,
            StmtKind::Compound(list) => {
                let mut found = false;
                for &id in self.tree.stmt_lists.get(list) {
                    if self.reachable_label(id)? {
                        found = true;
                        break;
                    }
                }
                found
            }
            StmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.reachable_label(then_branch)?
                    || if let Some(id) = else_branch {
                        self.reachable_label(id)?
                    } else {
                        false
                    }
            }
            StmtKind::ConditionalLoop { body, .. }
            | StmtKind::Do { body, .. }
            | StmtKind::For { body, .. } => self.reachable_label(body)?,
            StmtKind::Ambiguous { .. } => self.reachable_label(self.graph.lower_statement(id)?)?,
            _ => false,
        };
        self.label_memo.insert(id, found);
        Ok(found)
    }
}
