// Translated from lower/cleanup_lowering.cpp::may_throw/statement_may_throw.
// See NOTICE.
//! Local exception queries walk original syntax and selected call facts, excluding unevaluated operands.
use super::{
    Error,
    construction::{Entry, Variant},
    function::Function,
};
use crate::{
    sema::lowering::{CallTarget, ConstructorId, EntityId, TypeView},
    syntax::ast::{self, Condition, ExprKind, StmtKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn default_constructor_throws(&mut self, id: ConstructorId) -> Result<bool, Error> {
        let facts = Entry::Constructor(id, Variant::Complete).facts(self.graph)?;
        if !facts.nothrow {
            return Ok(true);
        }
        if let Some(entity) = self.graph.lower_constructor_entity(id)
            && let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)?
        {
            for index in 0..parameters.len() {
                if let Some(expr) = self.graph.lower_default(entity, index)
                    && self.expression_throws(expr)?
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub(super) fn expression_throws(&mut self, root: ast::ExprId) -> Result<bool, Error> {
        self.expression_throws_in_context(root, false)
    }
    pub(super) fn expression_throws_in_context(
        &mut self,
        root: ast::ExprId,
        hidden: bool,
    ) -> Result<bool, Error> {
        let mut work = std::mem::take(&mut self.lifetime.throw_work);
        let result = self.scan_expression_throws(root, &mut work, hidden);
        work.clear();
        self.lifetime.throw_work = work;
        result
    }
    fn scan_expression_throws(
        &self,
        root: ast::ExprId,
        work: &mut Vec<ast::ExprId>,
        hidden: bool,
    ) -> Result<bool, Error> {
        work.push(root);
        while let Some(id) = work.pop() {
            let id = self.without_parentheses(id);
            let facts = self.graph.lower_expression(id)?;
            let mut step = 0;
            while let Some(call) = self.graph.lower_arrow_call(id, step)? {
                if self.call_throws_in_context(CallTarget::Direct(call.function), hidden)? {
                    return Ok(true);
                }
                step += 1;
            }
            if let Some((call, _)) = self.graph.lower_function_conversion(id)?
                && self.call_throws_in_context(CallTarget::Direct(call.function), hidden)?
            {
                return Ok(true);
            }
            if facts.folded.is_some() {
                continue;
            }
            if let Some(crate::sema::lowering::CopyAction::Constructor { constructor, .. }) =
                self.graph.lower_class_copy(id)?
                && (hidden
                    || !Entry::Constructor(constructor, Variant::Complete)
                        .facts(self.graph)?
                        .nothrow)
            {
                return Ok(true);
            }
            if let Some((constructor, _)) = self.graph.lower_class_conversion(id)
                && (hidden
                    || !Entry::Constructor(constructor, Variant::Complete)
                        .facts(self.graph)?
                        .nothrow)
            {
                return Ok(true);
            }
            if (self.graph.lower_operator_call(id)? || self.graph.lower_literal_call(id)?.is_some())
                && self.call_throws_in_context(self.graph.lower_call(id)?, hidden)?
            {
                return Ok(true);
            }
            if let Some(constructor) = self.graph.lower_list_constructor(id)?
                && (hidden
                    || !Entry::Constructor(constructor, Variant::Complete)
                        .facts(self.graph)?
                        .nothrow)
            {
                return Ok(true);
            }
            match self.tree.expressions.get(id).kind {
                ExprKind::Call { callee, arguments } => {
                    if self.graph.lower_floating_intrinsic(id)?.is_none()
                        && !self.graph.lower_destructor_evaluation_only(id)?
                    {
                        let target = self.graph.lower_call(id)?;
                        if self.call_throws_in_context(target, hidden)? {
                            return Ok(true);
                        }
                        self.call_defaults(
                            target,
                            self.tree.expr_lists.get(arguments).len(),
                            work,
                        )?;
                    }
                    work.extend(self.tree.expr_lists.get(arguments));
                    if self.graph.lower_floating_intrinsic(id)?.is_none() {
                        work.push(callee);
                    }
                }
                ExprKind::Throw(_) => return Ok(true),
                ExprKind::New {
                    placement,
                    initializer,
                    ..
                } => {
                    let use_ = self.graph.lower_allocation(id)?;
                    if self.call_throws_in_context(CallTarget::Direct(use_.function), hidden)?
                        || self.array_length_throws(&use_)?
                    {
                        return Ok(true);
                    }
                    if let Some(array) = use_.array {
                        work.push(array.count);
                    }
                    if let Some((constructor, arguments)) = use_.constructor
                        && (hidden
                            || self.allocated_constructor_work(constructor, arguments, work)?)
                    {
                        return Ok(true);
                    }
                    work.extend(self.tree.expr_lists.get(placement.unwrap_or_default()));
                    if let Some(initializer) = initializer {
                        match initializer.kind {
                            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                                work.push(id);
                            }
                            ast::InitializerKind::Parenthesized(args) => {
                                work.extend(self.tree.expr_lists.get(args));
                            }
                            _ => (),
                        }
                    }
                }
                ExprKind::Delete { operand, .. } => {
                    let use_ = self.graph.lower_deletion(id)?;
                    if let TypeView::Class(class) = self.graph.lower_type(use_.target)?
                        && let Some(destructor) = self.graph.lower_destructor_entity(class)
                        && self.graph.lower_nontrivial_destructor(class)?
                        && self.lifetime_call_throws(CallTarget::Direct(destructor))?
                    {
                        return Ok(true);
                    }
                    work.push(operand);
                }
                ExprKind::Binary { left, right, .. } => {
                    work.push(right);
                    work.push(left);
                }
                ExprKind::Unary { operand, .. }
                | ExprKind::Postfix { operand, .. }
                | ExprKind::Cast { operand, .. } => work.push(operand),
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    work.push(else_expr);
                    work.push(then_expr);
                    work.push(condition);
                }
                ExprKind::Subscript { base, index } => {
                    work.push(index);
                    work.push(base);
                }
                ExprKind::Member { base, .. } => work.push(base),
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    work.extend(self.tree.expr_lists.get(arguments));
                    work.extend(braced);
                }
                ExprKind::Braced { elements, .. } => {
                    if let Some(constructor) = self.graph.lower_list_constructor(id)?
                        && (hidden
                            || !Entry::Constructor(constructor, Variant::Complete)
                                .facts(self.graph)?
                                .nothrow)
                    {
                        return Ok(true);
                    }
                    work.extend(self.tree.expr_lists.get(elements));
                }
                _ => {}
            }
        }
        Ok(false)
    }
    fn call_defaults(
        &self,
        target: CallTarget,
        written: usize,
        work: &mut Vec<ast::ExprId>,
    ) -> Result<(), Error> {
        if let CallTarget::Direct(entity) = target
            && let TypeView::Function { parameters, .. } = self
                .graph
                .lower_type(self.graph.lower_function(entity)?.ty)?
        {
            for index in written..parameters.len() {
                work.extend(self.graph.lower_default(entity, index));
            }
        }
        Ok(())
    }
    pub(super) fn initializer_throws(
        &mut self,
        initializer: Option<ast::Initializer>,
    ) -> Result<bool, Error> {
        let Some(initializer) = initializer else {
            return Ok(false);
        };
        match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                self.expression_throws(id)
            }
            ast::InitializerKind::Parenthesized(arguments) => {
                for &argument in self.tree.expr_lists.get(arguments) {
                    if self.expression_throws(argument)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
    pub(super) fn variable_throws(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
    ) -> Result<bool, Error> {
        if !self.graph.lower_is_variable(entity) || self.graph.lower_program_storage(entity) {
            return Ok(false);
        }
        if let Some(constructor) = self.graph.lower_construction(entity)
            && (initializer.is_some() || !self.graph.lower_constructor_trivial(constructor))
            && !Entry::Constructor(constructor, Variant::Complete)
                .facts(self.graph)?
                .nothrow
        {
            return Ok(true);
        }
        self.initializer_throws(initializer)
    }
    fn declaration_throws(&mut self, id: ast::DeclId) -> Result<bool, Error> {
        if let ast::DeclKind::Simple { declarators, .. } = self.tree.declarations.get(id).kind {
            for declarator in self.tree.init_declarators.get(declarators) {
                if self.variable_throws(
                    self.graph.lower_declared(declarator.declarator)?,
                    declarator.initializer,
                )? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub(super) fn condition_throws(&mut self, condition: Condition) -> Result<bool, Error> {
        match condition {
            Condition::Expression(id) => self.expression_throws(id),
            Condition::Declaration {
                declarator,
                initializer,
                ..
            } => {
                if let Some((call, _)) = self.graph.lower_condition_function(declarator)?
                    && self.lifetime_call_throws(CallTarget::Direct(call.function))?
                {
                    return Ok(true);
                }
                self.initializer_throws(Some(initializer))
            }
        }
    }
    pub(super) fn statement_throws(&mut self, id: ast::StmtId) -> Result<bool, Error> {
        self.check_stack()?;
        Ok(match self.tree.statements.get(id).kind {
            StmtKind::Compound(items) => {
                for &item in self.tree.stmt_lists.get(items) {
                    if self.statement_throws(item)? {
                        return Ok(true);
                    }
                }
                false
            }
            StmtKind::Expression(expr) | StmtKind::Return(expr) => expr
                .map(|id| self.expression_throws(id))
                .transpose()?
                .unwrap_or(false),
            StmtKind::Declaration(decl) => self.declaration_throws(decl)?,
            StmtKind::Ambiguous { .. } => self.statement_throws(self.graph.lower_statement(id)?)?,
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.condition_throws(condition)?
                    || self.statement_throws(then_branch)?
                    || else_branch
                        .map(|id| self.statement_throws(id))
                        .transpose()?
                        .unwrap_or(false)
            }
            StmtKind::ConditionalLoop {
                condition, body, ..
            } => self.condition_throws(condition)? || self.statement_throws(body)?,
            StmtKind::Do { condition, body } => {
                self.expression_throws(condition)? || self.statement_throws(body)?
            }
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => {
                declaration
                    .map(|id| self.declaration_throws(id))
                    .transpose()?
                    .unwrap_or(false)
                    || condition
                        .map(|c| self.condition_throws(c))
                        .transpose()?
                        .unwrap_or(false)
                    || init
                        .map(|id| self.expression_throws(id))
                        .transpose()?
                        .unwrap_or(false)
                    || iteration
                        .map(|id| self.expression_throws(id))
                        .transpose()?
                        .unwrap_or(false)
                    || self.statement_throws(body)?
            }
            StmtKind::Case { body, .. }
            | StmtKind::Default(body)
            | StmtKind::Label { body, .. } => self.statement_throws(body)?,
            _ => false,
        })
    }
    pub(super) fn constructor_plan_throws(&mut self, id: ConstructorId) -> Result<bool, Error> {
        self.check_stack()?;
        let plan = self.graph.lower_constructor(id);
        if let Some(actions) = plan.copy {
            for action in actions {
                if let Some(constructor) = action.constructor
                    && !self.graph.lower_constructor_trivial(constructor)
                    && self.default_constructor_throws(constructor)?
                {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        if let Some(delegation) = plan.delegation {
            return Ok(
                !Entry::Constructor(delegation.constructor, Variant::Complete)
                    .facts(self.graph)?
                    .nothrow
                    || self.initializer_throws(Some(delegation.initializer))?,
            );
        }
        for base in plan.bases {
            if let Some(constructor) = base.constructor
                && !self.graph.lower_constructor_trivial(constructor)
                && !Entry::Constructor(constructor, Variant::Base)
                    .facts(self.graph)?
                    .nothrow
            {
                return Ok(true);
            }
            if self.initializer_throws(base.initializer)? {
                return Ok(true);
            }
        }
        for &(_, initializer) in plan.members {
            if self.initializer_throws(initializer)? {
                return Ok(true);
            }
        }
        for child in plan.subobjects {
            let constructor = child.constructor;
            if self.graph.lower_anonymous_constructor(constructor) {
                if self.constructor_plan_throws(constructor)? {
                    return Ok(true);
                }
                continue;
            }
            if !Entry::Constructor(constructor, Variant::Complete)
                .facts(self.graph)?
                .nothrow
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
