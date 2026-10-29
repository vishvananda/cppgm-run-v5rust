// Translated from sema/analyzer_bodies.cpp and analyzer_range_for.cpp. See NOTICE.
//! Types-mode statement traversal creates scopes and checks names, without
//! imposing PA7 expression conversions or PA8 control-flow rules.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, ScopeEntry, ScopeId, ScopeKind},
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
        if self.reserved_context.deferring
            && self
                .body_context
                .as_ref()
                .is_some_and(|body| body.reserved_pending)
        {
            return Ok(());
        }
        let initialized = self.switch_initialized();
        let unevaluated = self.unevaluated;
        let result = self.statement_scoped(id, scope);
        let result =
            if matches!(result, Err(Error::DeferredBuiltin)) && self.reserved_context.deferring {
                self.unevaluated = unevaluated;
                self.mark_reserved_body();
                if let StmtKind::Expression(Some(operand))
                | StmtKind::Return(Some(operand))
                | StmtKind::Throw(Some(operand)) = self.tree.statements.get(id).kind
                {
                    self.graph.reserved_deferrals.note_expression(operand);
                }
                if matches!(self.tree.statements.get(id).kind, StmtKind::Return(_))
                    && let Some(body) = self.body_context.as_mut()
                {
                    body.undeduced = true;
                }
                let saved = self.reserved_context.pattern;
                self.reserved_context.pattern = true;
                let result = self.statement_scoped(id, scope);
                self.reserved_context.pattern = saved;
                if matches!(result, Err(Error::DeferredBuiltin)) {
                    if let Some(body) = self.body_context.as_mut() {
                        body.reserved_pending = true;
                        body.undeduced = true;
                    }
                    Ok(())
                } else {
                    result
                }
            } else {
                result
            };
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
        if self.semantic()
            && matches!(
                self.tree.statements.get(id).kind,
                StmtKind::Break | StmtKind::Continue | StmtKind::Default(_)
            )
        {
            self.semantic_jump(id)?;
        }
        match self.tree.statements.get(id).kind {
            StmtKind::Asm(assembly) => {
                self.asm_statement(id, *self.tree.assembly.get(assembly), scope)
            }
            StmtKind::Compound(list) => self.compound_selection(
                list,
                scope,
                false,
                false,
                self.tree.statements.get(id).selection_init,
            ),
            StmtKind::Declaration(decl) => self.declaration(decl, scope),
            StmtKind::Return(operand)
                if self.semantic()
                    && self.template_depth == 0
                    && !self.reserved_context.pattern =>
            {
                self.semantic_return(id, operand, scope)
            }
            StmtKind::Throw(Some(operand))
                if self.semantic()
                    && self.template_depth == 0
                    && !self.reserved_context.pattern
                    && self.body_context.is_some() =>
            {
                // statement ignores the nullable thrown type; a throw
                // expression instead returns null before its own event.
                match self.throw_operand(operand, scope, self.tree.statements.get(id).location) {
                    Ok(_) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => {}
                    Err(error) => return Err(error),
                }
                self.note_potentially_throwing();
                Ok(())
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
                is_constexpr,
                condition,
                then_branch,
                else_branch,
            } => {
                let block = self.block_scope(scope)?;
                self.graph.scope_mut(block).guarded =
                    self.graph.scope(scope).entry == ScopeEntry::SelectionInit;
                self.condition(condition, block)?;
                self.condition_conversion(condition, false)?;
                let mut selected = None;
                if is_constexpr
                    && self.semantic()
                    && self.template_depth == 0
                    && !self.reserved_context.pattern
                    && self.body_context.is_some()
                    && !matches!(condition, Condition::Expression(expr) if self.graph.reserved_deferrals.expression(expr))
                {
                    let Condition::Expression(expr) = condition else {
                        return Err(Error::at(
                            self.tree.statements.get(id).location,
                            "`if constexpr` with a condition declaration",
                        ));
                    };
                    let value = self
                        .with_fresh_constant_query(|analyzer| {
                            analyzer.constant_truth(expr, block, false)
                        })
                        .map_err(|error| {
                            error.required_expression(self.tree.expressions.get(expr).location)
                        })?;
                    super::record(&mut self.graph.constexpr_selections, id.index(), value);
                    selected = Some(value);
                }
                if selected != Some(false) {
                    self.substatement(then_branch, block)?;
                }
                if selected != Some(true)
                    && let Some(branch) = else_branch
                {
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
                self.graph.scope_mut(block).guarded =
                    self.graph.scope(scope).entry == ScopeEntry::SelectionInit;
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
                let typed = self.reserved_expression(condition, block)?;
                if typed
                    && self.semantic()
                    && self.template_depth == 0
                    && !self.reserved_context.pattern
                {
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
                    self.reserved_expression(init, block)?;
                }
                if let Some(condition) = condition {
                    self.condition(condition, block)?;
                    self.condition_conversion(condition, false)?;
                }
                if let Some(iteration) = iteration {
                    self.reserved_expression(iteration, block)?;
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
            StmtKind::Try { body, handlers } => self.try_statement(body, handlers, scope),
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
            let selection_init = self.tree.statements.get(id).selection_init;
            return self.compound_selection(list, scope, !selection_init, false, selection_init);
        }
        let block = self.block_scope(scope)?;
        self.graph.scope_mut(block).guarded = true;
        self.statement(id, block)
    }

    pub(super) fn compound_region(
        &mut self,
        list: List<ast::StmtId>,
        scope: ScopeId,
        guarded: bool,
        handles: bool,
    ) -> Result<(), Error> {
        self.compound_selection(list, scope, guarded, handles, false)
    }
    fn compound_selection(
        &mut self,
        list: List<ast::StmtId>,
        scope: ScopeId,
        guarded: bool,
        handles: bool,
        selection_init: bool,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let block = self.block_scope(scope)?;
        self.graph.scope_mut(block).guarded = guarded;
        self.graph.scope_mut(block).entry = if handles {
            ScopeEntry::Exception
        } else if selection_init {
            ScopeEntry::SelectionInit
        } else {
            ScopeEntry::Ordinary
        };
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
            Condition::Expression(expr) => self.reserved_expression(expr, scope).map(|_| ()),
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
            self.graph.scope_mut(block).entry = super::graph::ScopeEntry::Exception;
            if let ast::ExceptionDeclaration::Typed {
                specifiers,
                declarator,
            } = handler.exception
            {
                let specs = self.specifiers(specifiers, block, false)?;
                if matches!(
                    self.graph.types.get(specs.ty).kind,
                    super::types::TypeKind::Auto(_)
                ) {
                    return Err(Error::at(location, "exception declaration without a type"));
                }
                let info = self.declarator(declarator, specs.ty, block)?;
                if self.semantic()
                    && self.template_depth == 0
                    && !self.reserved_context.pattern
                    && self.body_context.is_some()
                {
                    self.exception_declaration(handler.body, &info, declarator, block, location)?;
                } else if let Some(name) = info.name {
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
        if self.semantic() && self.template_depth == 0 && !self.reserved_context.pattern {
            let unevaluated = self.unevaluated;
            match self.range_for_equivalent(id, specifiers, declarator, range, block) {
                Err(Error::DeferredBuiltin) if self.reserved_context.deferring => {
                    self.unevaluated = unevaluated;
                    let specs = self.specifiers(specifiers, scope, false)?;
                    if self.holds_auto(specs.ty) {
                        return Err(Error::DeferredBuiltin);
                    }
                    self.mark_reserved_body();
                    let saved = self.reserved_context.pattern;
                    self.reserved_context.pattern = true;
                    let result = self.range_for_names(id, specifiers, declarator, range, block);
                    self.reserved_context.pattern = saved;
                    result?;
                }
                result => result?,
            }
        } else {
            self.range_for_names(id, specifiers, declarator, range, block)?;
        }
        self.control_substatement(
            body,
            block,
            Control::Loop,
            self.tree.statements.get(id).location,
        )
    }

    fn range_for_names(
        &mut self,
        id: ast::StmtId,
        specifiers: List<ast::Specifier>,
        declarator: ast::DeclaratorId,
        range: ast::ExprId,
        block: ScopeId,
    ) -> Result<(), Error> {
        let uses = self.dependent_uses.get();
        self.expression(range, block)?;
        let specs = self.specifiers(specifiers, block, false)?;
        let info = self.range_declarator(
            specs,
            declarator,
            range,
            block,
            self.dependent_uses.get() != uses,
            self.tree.statements.get(id).location,
        )?;
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
        Ok(())
    }
}
