// Translated from sema/constant_statements.cpp. See NOTICE.
//! Execute analyzed scalar bodies, releasing block and iteration objects on every path.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{TemporaryValue as Value, nonconstant},
    constexpr_calls::Returned,
    graph::{BindingKind, EntityId},
    specifiers::{StaticSpecifier, ThreadStorage},
    types::TypeKind,
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, StmtKind},
};

#[derive(Clone, Copy)]
pub(super) enum Flow {
    Next,
    Break,
    Continue,
    Return(Returned),
}
impl Analyzer<'_> {
    pub(super) fn execute_constant_statement(
        &mut self,
        mut id: ast::StmtId,
    ) -> Result<Flow, Error> {
        self.constant_step(self.tree.statements.get(id).location)?;
        if matches!(
            self.tree.statements.get(id).kind,
            StmtKind::Ambiguous { .. }
        ) {
            id = self.graph.lower_statement(id)?;
        }
        let stmt = *self.tree.statements.get(id);
        if self.constexpr_frames.seeking().is_some() {
            match stmt.kind {
                StmtKind::Compound(_) => (),
                StmtKind::Case { body, .. }
                | StmtKind::Default(body)
                | StmtKind::Label { body, .. } => {
                    if self.constexpr_frames.seeking() == Some(id) {
                        self.constexpr_frames.seek(None);
                    }
                    return self.execute_constant_statement(body);
                }
                _ => return Ok(Flow::Next),
            }
        }
        let scope = self.constexpr_frames.current()?.scope;
        match stmt.kind {
            StmtKind::Compound(_) => self.constant_compound(id),
            StmtKind::Expression(expr) => {
                if let Some(expr) = expr {
                    self.discard_constant(expr, scope)?;
                }
                Ok(Flow::Next)
            }
            StmtKind::Declaration(decl) => {
                self.constant_declaration(decl)?;
                Ok(Flow::Next)
            }
            StmtKind::Return(expr) => {
                let target = self.constexpr_frames.current()?.result;
                let value = if let Some(expr) = expr {
                    if matches!(
                        self.graph.types.get(target).kind,
                        TypeKind::Scalar(super::types::Scalar::Void)
                    ) {
                        self.discard_constant(expr, scope)?;
                        Returned::Void
                    } else if let TypeKind::Reference { element, .. } =
                        self.graph.types.get(target).kind
                    {
                        Returned::Place(self.bind_constant_reference(expr, element, scope)?)
                    } else {
                        Returned::Value(self.constant_operand_value(expr, target, scope)?)
                    }
                } else {
                    Returned::Void
                };
                Ok(Flow::Return(value))
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let region = self.constexpr_frames.region();
                let result = (|| {
                    if self.constant_condition(condition, stmt.location)? {
                        self.execute_constant_statement(then_branch)
                    } else if let Some(branch) = else_branch {
                        self.execute_constant_statement(branch)
                    } else {
                        Ok(Flow::Next)
                    }
                })();
                self.constexpr_frames.release(region);
                result
            }
            StmtKind::ConditionalLoop {
                keyword: Keyword::Switch,
                condition,
                body,
            } => self.constant_switch(condition, body, stmt.location),
            StmtKind::ConditionalLoop {
                condition, body, ..
            } => self.constant_loop(id, None, None, Some(condition), None, body, None),
            StmtKind::Do { body, condition } => {
                self.constant_loop(id, None, None, None, None, body, Some(condition))
            }
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => self.constant_loop(id, declaration, init, condition, iteration, body, None),
            StmtKind::Break => Ok(Flow::Break),
            StmtKind::Continue => Ok(Flow::Continue),
            StmtKind::Case { body, .. }
            | StmtKind::Default(body)
            | StmtKind::Label { body, .. } => self.execute_constant_statement(body),
            StmtKind::RangeFor { .. } => Err(Error::Missing("constexpr range-for execution")),
            _ => Err(nonconstant(
                stmt.location,
                "a statement that is not allowed in a constexpr function",
            )),
        }
    }
    pub(super) fn constant_compound(&mut self, id: ast::StmtId) -> Result<Flow, Error> {
        let StmtKind::Compound(items) = self.tree.statements.get(id).kind else {
            return Err(Error::Invariant("constant compound without block"));
        };
        let region = self.constexpr_frames.region();
        let result = (|| {
            for index in 0..self.tree.stmt_lists.get(items).len() {
                let item = self.tree.stmt_lists.get(items)[index];
                let statement = self.constexpr_frames.region();
                let flow = self.execute_constant_statement(item)?;
                if !self.constant_statement_declares(item)? {
                    self.constexpr_frames.release(statement);
                }
                if !matches!(flow, Flow::Next) {
                    return Ok(flow);
                }
            }
            Ok(Flow::Next)
        })();
        self.constexpr_frames.release(region);
        result
    }
    fn constant_statement_declares(&self, mut id: ast::StmtId) -> Result<bool, Error> {
        loop {
            match self.tree.statements.get(id).kind {
                StmtKind::Case { body, .. }
                | StmtKind::Default(body)
                | StmtKind::Label { body, .. } => id = body,
                StmtKind::Ambiguous { .. } => id = self.graph.lower_statement(id)?,
                StmtKind::Declaration(_) => return Ok(true),
                _ => return Ok(false),
            }
        }
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "One loop executor retains the parsed for/while/do components and their distinct evaluation order"
    )]
    fn constant_loop(
        &mut self,
        stmt: ast::StmtId,
        declaration: Option<ast::DeclId>,
        init: Option<ast::ExprId>,
        condition: Option<ast::Condition>,
        iteration: Option<ast::ExprId>,
        body: ast::StmtId,
        after: Option<ast::ExprId>,
    ) -> Result<Flow, Error> {
        let scope = self.constexpr_frames.current()?.scope;
        let location = self.tree.statements.get(stmt).location;
        let region = self.constexpr_frames.region();
        let result = (|| {
            if let Some(declaration) = declaration {
                self.constant_declaration(declaration)?;
            } else if let Some(init) = init {
                self.discard_constant(init, scope)?;
            }
            loop {
                let iteration_region = self.constexpr_frames.region();
                let result = (|| {
                    self.constant_step(location)?;
                    if let Some(condition) = condition
                        && !self.constant_condition(condition, location)?
                    {
                        return Ok(Flow::Break);
                    }
                    match self.execute_constant_statement(body)? {
                        flow @ (Flow::Break | Flow::Return(_)) => return Ok(flow),
                        Flow::Next | Flow::Continue => (),
                    }
                    if let Some(iteration) = iteration {
                        self.discard_constant(iteration, scope)?;
                    }
                    if let Some(after) = after
                        && !self.constant_truth(after, scope, false)?
                    {
                        return Ok(Flow::Break);
                    }
                    Ok(Flow::Next)
                })();
                self.constexpr_frames.release(iteration_region);
                match result? {
                    Flow::Return(value) => return Ok(Flow::Return(value)),
                    Flow::Break => return Ok(Flow::Next),
                    Flow::Next | Flow::Continue => (),
                }
            }
        })();
        self.constexpr_frames.release(region);
        result
    }
    fn constant_condition(
        &mut self,
        condition: ast::Condition,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let scope = self.constexpr_frames.current()?.scope;
        match condition {
            ast::Condition::Expression(expr) => self.constant_truth(expr, scope, false),
            ast::Condition::Declaration {
                declarator,
                initializer,
                ..
            } => {
                let entity = self.graph.lower_declared(declarator)?;
                self.constant_local_variable(entity, Some(initializer), location)?;
                let place = self
                    .constexpr_frames
                    .local(entity)
                    .ok_or(Error::Invariant("condition local missing"))?;
                match self.constexpr_frames.read(place, location)? {
                    Value::Integer(value) => Ok(value.bits != 0),
                    Value::Pointer(value) => {
                        Ok(value.root != super::constant_addresses::Root::Null)
                    }
                }
            }
        }
    }
    fn constant_declaration(&mut self, decl: ast::DeclId) -> Result<(), Error> {
        let node = *self.tree.declarations.get(decl);
        let ast::DeclKind::Simple { declarators, .. } = node.kind else {
            return Ok(());
        };
        for index in 0..self.tree.init_declarators.get(declarators).len() {
            let item = self.tree.init_declarators.get(declarators)[index];
            let Some(binding) = self
                .graph
                .declared_bindings
                .get(item.declarator.index())
                .copied()
                .flatten()
            else {
                continue;
            };
            let binding = self.graph.binding(binding);
            if binding.kind == BindingKind::Variable {
                self.constant_local_variable(binding.entity, item.initializer, node.location)?;
            }
        }
        Ok(())
    }
    fn constant_local_variable(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let variable = self
            .graph
            .entity(entity)
            .variable
            .ok_or(Error::Invariant("constant local without variable facts"))?;
        if variable.is_extern {
            return Ok(());
        }
        if variable.static_specifier == StaticSpecifier::Present
            || variable.thread_storage == ThreadStorage::ThreadLocal
        {
            return Err(nonconstant(
                location,
                "a static or thread_local variable in a constant expression",
            ));
        }
        let ty = self.graph.lower_entity_type(entity)?;
        let scope = self.constexpr_frames.current()?.scope;
        if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            let single = initializer
                .and_then(|init| self.single_initializer(init))
                .ok_or_else(|| {
                    nonconstant(location, "a reference variable outside constant evaluation")
                })?;
            let place = self.bind_constant_reference(single, element, scope)?;
            return self.constexpr_frames.bind(entity, place);
        }
        let place = self.allocate_constant_object(ty, None, location)?;
        self.constexpr_frames.bind(entity, place)?;
        let constructor = self.graph.entity(entity).constructor;
        self.construct_constant_object(place, initializer, constructor, scope, location)?;
        Ok(())
    }
    fn constant_switch(
        &mut self,
        condition: ast::Condition,
        body: ast::StmtId,
        location: SourceOffset,
    ) -> Result<Flow, Error> {
        let region = self.constexpr_frames.region();
        let saved_seek = self.constexpr_frames.seeking();
        let result = (|| {
            let scope = self.constexpr_frames.current()?.scope;
            let value = match condition {
                ast::Condition::Expression(expr) => {
                    self.evaluate_constant(expr, scope, false)?.bits
                }
                ast::Condition::Declaration {
                    declarator,
                    initializer,
                    ..
                } => {
                    let entity = self.graph.lower_declared(declarator)?;
                    self.constant_local_variable(entity, Some(initializer), location)?;
                    let place = self
                        .constexpr_frames
                        .local(entity)
                        .ok_or(Error::Invariant("switch local missing"))?;
                    self.read_constant_integer(place, location)?.bits
                }
            };
            let Some(target) = self.constant_switch_target(body, value)? else {
                return Ok(Flow::Next);
            };
            self.constexpr_frames.seek(Some(target));
            match self.execute_constant_statement(body)? {
                Flow::Break => Ok(Flow::Next),
                other => Ok(other),
            }
        })();
        self.constexpr_frames.seek(saved_seek);
        self.constexpr_frames.release(region);
        result
    }
    fn constant_switch_target(
        &self,
        body: ast::StmtId,
        value: u64,
    ) -> Result<Option<ast::StmtId>, Error> {
        let mut work = vec![(body, 0u32)];
        let mut fallback = None;
        while let Some((id, depth)) = work.pop() {
            if depth > 256 {
                continue;
            }
            match self.tree.statements.get(id).kind {
                StmtKind::Compound(items) => work.extend(
                    self.tree
                        .stmt_lists
                        .get(items)
                        .iter()
                        .rev()
                        .map(|&id| (id, depth + 1)),
                ),
                StmtKind::Case { body, .. } => {
                    let converted = self
                        .graph
                        .case_values
                        .get(id.index())
                        .copied()
                        .flatten()
                        .ok_or(Error::Invariant("untyped constant switch case"))?;
                    if converted == value {
                        return Ok(Some(id));
                    }
                    work.push((body, depth + 1));
                }
                StmtKind::Default(body) => {
                    if fallback.is_none() {
                        fallback = Some(id);
                    }
                    work.push((body, depth + 1));
                }
                StmtKind::Label { body, .. } => work.push((body, depth + 1)),
                _ => (),
            }
        }
        Ok(fallback)
    }
}
