// Translated from sema/analyzer_bodies.cpp and analyzer_range_for.cpp. See NOTICE.
//! Types-mode statement traversal creates scopes and checks names, without
//! imposing PA7 expression conversions or PA8 control-flow rules.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, ScopeId, ScopeKind},
    lookup::Filter,
    procedural_statements::Control,
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::{
        ast::{self, Condition, StmtKind},
        storage::List,
    },
};

impl Analyzer<'_> {
    fn block_scope(&mut self, parent: ScopeId) -> Result<ScopeId, Error> {
        self.graph
            .add_scope(ScopeKind::Block, parent, Some(parent), None)
    }

    pub(super) fn statement(&mut self, id: ast::StmtId, scope: ScopeId) -> Result<(), Error> {
        let initialized = self.switch_initialized();
        let result = self.statement_scoped(id, scope);
        let mut selected = id;
        if matches!(
            self.tree.statements.get(id).kind,
            StmtKind::Ambiguous { .. }
        ) && let Some(reading) = self
            .graph
            .statement_readings
            .get(id.index())
            .copied()
            .flatten()
        {
            selected = reading;
        }
        if !matches!(
            self.tree.statements.get(selected).kind,
            StmtKind::Declaration(_)
                | StmtKind::Case { .. }
                | StmtKind::Default(_)
                | StmtKind::Label { .. }
        ) {
            self.restore_switch_initialized(initialized);
        }
        result.map_err(|error| match error {
            Error::Limit("constructs nested too deeply") if self.semantic() => Error::at(
                self.tree.statements.get(id).location,
                "constructs nested too deeply",
            ),
            other => other,
        })
    }
    fn statement_scoped(&mut self, id: ast::StmtId, scope: ScopeId) -> Result<(), Error> {
        self.check_stack()?;
        self.reject_extended_statement(id)?;
        if self.semantic()
            && matches!(
                self.tree.statements.get(id).kind,
                StmtKind::Break | StmtKind::Continue | StmtKind::Default(_)
            )
        {
            self.semantic_jump(id)?;
        }
        match self.tree.statements.get(id).kind {
            StmtKind::Compound(list) => self.compound(list, scope, false),
            StmtKind::Declaration(decl) => self.declaration(decl, scope),
            StmtKind::Return(operand) if self.semantic() && self.template_depth == 0 => {
                self.semantic_return(id, operand, scope)
            }
            StmtKind::Expression(operand)
            | StmtKind::Return(operand)
            | StmtKind::Throw(operand) => operand.map_or(Ok(()), |id| self.expression(id, scope)),
            StmtKind::Break | StmtKind::Continue => Ok(()),
            StmtKind::Goto(_) => self.record_transfer(id, scope),
            StmtKind::Label { body, .. } => {
                self.record_transfer(id, scope)?;
                self.statement(body, scope)
            }
            StmtKind::Default(body) => self.statement(body, scope),
            StmtKind::Case { value, body } => {
                if self.semantic() {
                    self.semantic_case(id, value, scope)?;
                } else {
                    self.expression(value, scope)?;
                }
                self.statement(body, scope)
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let block = self.block_scope(scope)?;
                self.condition(condition, block)?;
                self.condition_conversion(condition, false)?;
                self.substatement(then_branch, block)?;
                if let Some(branch) = else_branch {
                    self.substatement(branch, block)?;
                }
                Ok(())
            }
            StmtKind::ConditionalLoop {
                keyword,
                condition,
                body,
            } => {
                let block = self.block_scope(scope)?;
                self.condition(condition, block)?;
                let switch = self.condition_conversion(condition, Self::is_switch(keyword))?;
                self.control_substatement(
                    body,
                    block,
                    if Self::is_switch(keyword) {
                        Control::Switch(switch)
                    } else {
                        Control::Loop
                    },
                    self.tree.statements.get(id).location,
                )
            }
            StmtKind::Do { body, condition } => {
                let block = self.block_scope(scope)?;
                self.control_substatement(
                    body,
                    block,
                    Control::Loop,
                    self.tree.statements.get(id).location,
                )?;
                self.expression(condition, block)?;
                if self.semantic() && self.template_depth == 0 {
                    self.convert_bool(condition)?;
                }
                Ok(())
            }
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => {
                let block = self.block_scope(scope)?;
                if let Some(declaration) = declaration {
                    self.declaration(declaration, block)?;
                } else if let Some(init) = init {
                    self.expression(init, block)?;
                }
                if let Some(condition) = condition {
                    self.condition(condition, block)?;
                    self.condition_conversion(condition, false)?;
                }
                if let Some(iteration) = iteration {
                    self.expression(iteration, block)?;
                }
                self.control_substatement(
                    body,
                    block,
                    Control::Loop,
                    self.tree.statements.get(id).location,
                )
            }
            StmtKind::RangeFor {
                specifiers,
                declarator,
                range,
                body,
            } => self.range_for(id, specifiers, declarator, range, body, scope),
            StmtKind::Try { body, handlers } => {
                self.statement(body, scope)?;
                self.handlers(handlers, scope)
            }
            StmtKind::Ambiguous {
                name,
                component,
                readings,
            } => {
                let selected = self.statement_reading(name, component, readings, scope)?;
                super::record(&mut self.graph.statement_readings, id.index(), selected);
                self.statement(selected, scope)
            }
        }
    }

    fn statement_reading(
        &mut self,
        name: ast::Name,
        component: u32,
        readings: List<ast::StatementReading>,
        scope: ScopeId,
    ) -> Result<ast::StmtId, Error> {
        let reading_list = readings;
        let readings = self.tree.statement_readings.get(reading_list);
        let first = readings
            .first()
            .copied()
            .ok_or(Error::Invariant("ambiguous statement without readings"))?;
        let count = usize::try_from(component)
            .map_err(|_| Error::Limit("statement name component limit"))?;
        let node = self.tree.names.get(name);
        let member = self
            .tree
            .name_lists
            .get(node.components)
            .get(count)
            .copied()
            .ok_or(Error::Invariant("statement name component outside name"))?;
        if count == 0 || !matches!(member.kind, ast::NameKind::Identifier(_)) {
            return Ok(first.statement);
        }
        let category = match self.prefix_components(name, scope, count) {
            Ok(Some(prefix)) => {
                let key = Self::component_name(member.kind)?;
                let Some(binding) = self.qualified(prefix, key, Filter::Ordinary)? else {
                    return Ok(first.statement);
                };
                let binding = self.graph.binding(binding);
                if self.graph.entity(binding.entity).template_head.is_some() {
                    ast::ReadingCategory::Template
                } else if matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
                    ast::ReadingCategory::Type
                } else {
                    ast::ReadingCategory::Value
                }
            }
            Err(Error::Missing("dependent qualified name lookup")) => ast::ReadingCategory::Value,
            Ok(None) => return Ok(first.statement),
            Err(error) => return Err(error),
        };
        Ok(self
            .tree
            .statement_readings
            .get(reading_list)
            .iter()
            .find(|reading| reading.category == category)
            .unwrap_or(&first)
            .statement)
    }

    pub(super) fn substatement(&mut self, id: ast::StmtId, scope: ScopeId) -> Result<(), Error> {
        if let StmtKind::Compound(list) = self.tree.statements.get(id).kind {
            return self.compound(list, scope, true);
        }
        let block = self.block_scope(scope)?;
        self.graph.scope_mut(block).guarded = true;
        self.statement(id, block)
    }

    pub(super) fn compound(
        &mut self,
        list: List<ast::StmtId>,
        scope: ScopeId,
        guarded: bool,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let block = self.block_scope(scope)?;
        self.graph.scope_mut(block).guarded = guarded;
        let initialized = self.switch_initialized();
        let result = (|| {
            for syntax_index in 0..self.tree.stmt_lists.get(list).len() {
                let item = self.tree.stmt_lists.get(list)[syntax_index];
                self.statement(item, block)?;
            }
            Ok(())
        })();
        self.restore_switch_initialized(initialized);
        result
    }

    fn condition(&mut self, condition: Condition, scope: ScopeId) -> Result<(), Error> {
        match condition {
            Condition::Expression(expr) => self.expression(expr, scope),
            Condition::Declaration {
                specifiers,
                declarator,
                initializer,
            } => {
                let specs = self.specifiers(specifiers, scope, false)?;
                self.declare_init(
                    specs,
                    ast::InitDeclarator {
                        declarator,
                        initializer: Some(initializer),
                        tokens: ast::TokenSpan { begin: 0, end: 0 },
                    },
                    scope,
                    self.tree.declarators.get(declarator).location,
                    &mut None,
                )
            }
        }
    }

    pub(super) fn handlers(
        &mut self,
        handlers: List<ast::Handler>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let mut catch_all = false;
        for syntax_index in 0..self.tree.handlers.get(handlers).len() {
            let syntax_item = self.tree.handlers.get(handlers)[syntax_index];
            let handler = &syntax_item;
            let location = self.tree.statements.get(handler.body).location;
            if catch_all {
                return Err(Error::at(
                    location,
                    "a handler after a `...` handler (15.3p5)",
                ));
            }
            catch_all = matches!(handler.exception, ast::ExceptionDeclaration::Ellipsis);
            let block = self.block_scope(scope)?;
            if let ast::ExceptionDeclaration::Typed {
                specifiers,
                declarator,
            } = handler.exception
            {
                let specs = self.specifiers(specifiers, block, false)?;
                let info = self.declarator(declarator, specs.ty, block)?;
                if let Some(name) = info.name {
                    let entity = self
                        .graph
                        .add_entity(Some(info.ty), Some(name), Keyword::Int)?;
                    let binding = self.graph.add_binding(
                        block,
                        Some(block),
                        Binding {
                            kind: BindingKind::Variable,
                            name: Some(name),
                            entity,
                            ty: Some(info.ty),
                            key: Keyword::Int,
                        },
                    )?;
                    self.graph
                        .set_binding_dependence(binding, self.graph.dependent_type(info.ty));
                    if let Some(declarator) = declarator {
                        super::record(
                            &mut self.graph.declared_bindings,
                            declarator.index(),
                            binding,
                        );
                    }
                }
            }
            self.substatement(handler.body, block)?;
        }
        Ok(())
    }

    fn range_for(
        &mut self,
        id: ast::StmtId,
        specifiers: List<ast::Specifier>,
        declarator: ast::DeclaratorId,
        range: ast::ExprId,
        body: ast::StmtId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        for syntax_index in 0..self.tree.specs.get(specifiers).len() {
            let syntax_item = self.tree.specs.get(specifiers)[syntax_index];
            let specifier = &syntax_item;
            if matches!(
                specifier.kind,
                ast::SpecifierKind::Keyword(
                    Keyword::Typedef
                        | Keyword::Friend
                        | Keyword::Extern
                        | Keyword::Static
                        | Keyword::Register
                        | Keyword::ThreadLocal
                        | Keyword::Mutable
                        | Keyword::Inline
                        | Keyword::Virtual
                        | Keyword::Explicit
                )
            ) {
                return Err(Error::at(
                    specifier.location,
                    "a for-range-declaration's decl-specifier is no type-specifier or constexpr",
                ));
            }
        }
        let block = self.block_scope(scope)?;
        self.expression(range, block)?;
        let specs = self.specifiers(specifiers, block, false)?;
        let info = self.declarator(Some(declarator), specs.ty, block)?;
        let name = info.name.ok_or_else(|| {
            Error::at(
                self.tree.statements.get(id).location,
                "range-based for declaration without a name",
            )
        })?;
        let entity = self
            .graph
            .add_entity(Some(info.ty), Some(name), Keyword::Int)?;
        let binding = self.graph.add_binding(
            block,
            Some(block),
            Binding {
                kind: BindingKind::Variable,
                name: Some(name),
                entity,
                ty: Some(info.ty),
                key: Keyword::Int,
            },
        )?;
        self.graph
            .set_binding_dependence(binding, self.graph.dependent_type(info.ty));
        super::record(
            &mut self.graph.declared_bindings,
            declarator.index(),
            binding,
        );
        self.substatement(body, block)
    }
}
