// Translated from sema/analyzer_bodies.cpp's types-mode expression walk. See NOTICE.
//! Name checks and explicit class demand, without PA7 expression typing.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, BindingKind, ScopeId, ScopeKind},
    lookup::Filter,
    types::{TypeId, TypeKind},
};
use crate::syntax::ast::{self, ExprKind, InitializerKind, NameKind};
impl Analyzer<'_> {
    pub(super) fn expression(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<(), Error> {
        if self.semantic() && self.template_depth == 0 {
            self.typed_expression(id, scope)?;
            self.check_resolved_function(id)?;
            if self.template_depth == 0 && self.body_context.is_some() {
                self.check_temporary_destruction(
                    self.graph.expression_facts.get(id)?,
                    scope,
                    self.tree.expressions.get(id).location,
                )?;
            }
            return Ok(());
        }
        if self.semantic() {
            self.validate_fixed_expression(id, scope)?;
        }
        let base = self.expression_work.len();
        self.expression_work.push((id, false));
        while self.expression_work.len() > base {
            let (id, unevaluated) = self
                .expression_work
                .pop()
                .ok_or(Error::Invariant("missing expression work"))?;
            self.expression_names(id, scope, unevaluated)?;
        }
        Ok(())
    }
    fn expression_names(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        unevaluated: bool,
    ) -> Result<(), Error> {
        match self.tree.expressions.get(id).kind {
            ExprKind::Id(name) => {
                self.check_expression_name(name, scope, false, unevaluated)?;
            }
            ExprKind::Paren(operand)
            | ExprKind::Unary { operand, .. }
            | ExprKind::Postfix { operand, .. }
            | ExprKind::Delete { operand, .. } => self.expression_work.push((operand, unevaluated)),
            ExprKind::Binary { left, right, .. } => {
                self.expression_work.push((right, unevaluated));
                self.expression_work.push((left, unevaluated));
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.expression_work.extend([
                    (else_expr, unevaluated),
                    (then_expr, unevaluated),
                    (condition, unevaluated),
                ]);
            }
            ExprKind::Call { callee, arguments } => {
                let argument_buffer =
                    super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
                let arguments = argument_buffer.as_slice();
                self.expression_work
                    .extend(arguments.iter().rev().map(|&id| (id, unevaluated)));
                if let ExprKind::Id(name) = self.tree.expressions.get(callee).kind {
                    let callee =
                        self.check_expression_name(name, scope, !arguments.is_empty(), true)?;
                    if callee.is_none() && self.template_depth != 0 && !arguments.is_empty() {
                        self.check_unbound_callee(name, arguments, scope)?;
                    }
                    if let Some(binding) = callee
                        && arguments.is_empty()
                        && !unevaluated
                    {
                        let binding = self.graph.binding(binding);
                        if matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
                            self.value_initialized_class(
                                binding
                                    .ty
                                    .ok_or(Error::Invariant("type callee without type"))?,
                            )?;
                        }
                    }
                } else {
                    self.expression_work.push((callee, unevaluated));
                }
            }
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                let specs = self.specifiers(specifiers, scope, false)?;
                let empty = if let Some(braced) = braced {
                    matches!(self.tree.expressions.get(braced).kind, ExprKind::Braced { elements, .. } if elements.is_empty())
                } else {
                    arguments.is_empty()
                };
                if empty && !unevaluated {
                    self.value_initialized_class(specs.ty)?;
                }
                if let Some(braced) = braced {
                    self.expression_work.push((braced, unevaluated));
                } else {
                    self.expression_work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .map(|&id| (id, unevaluated)),
                    );
                }
            }
            ExprKind::Subscript { base, index } => self
                .expression_work
                .extend([(index, unevaluated), (base, unevaluated)]),
            ExprKind::Member { base, .. } => self.expression_work.push((base, unevaluated)),
            ExprKind::Cast { ty, operand, .. } => {
                self.type_syntax(ty, scope)?;
                self.expression_work.push((operand, unevaluated));
            }
            ExprKind::Sizeof {
                ty,
                operand,
                parenthesized,
            } => {
                if let Some(ty) = ty {
                    self.type_syntax(ty, scope)?;
                }
                if let Some(operand) = operand {
                    if !parenthesized
                        && let ExprKind::Id(name) = self.tree.expressions.get(operand).kind
                    {
                        self.check_expression_name(name, scope, false, false)?;
                    } else {
                        self.expression_work.push((operand, true));
                    }
                }
            }
            ExprKind::TypeTrait { ty, operand, .. } => {
                if let Some(ty) = ty {
                    self.type_syntax(ty, scope)?;
                }
                if let Some(operand) = operand {
                    self.expression_work.push((operand, true));
                }
            }
            ExprKind::New {
                placement,
                ty,
                initializer,
                ..
            } => {
                if let Some(placement) = placement {
                    self.expression_work.extend(
                        self.tree
                            .expr_lists
                            .get(placement)
                            .iter()
                            .map(|&id| (id, unevaluated)),
                    );
                }
                let (ty, bound) = self.allocation_type(ty, scope)?;
                if let Some(bound) = bound {
                    self.expression_work.push((bound, unevaluated));
                }
                let empty = initializer.is_none_or(|init| matches!(init.kind, InitializerKind::Parenthesized(arguments) if arguments.is_empty()));
                if empty && !unevaluated {
                    self.value_initialized_class(ty)?;
                }
                if let Some(initializer) = initializer {
                    self.push_initializer(initializer, unevaluated);
                }
            }
            ExprKind::Braced { elements, .. } => self.expression_work.extend(
                self.tree
                    .expr_lists
                    .get(elements)
                    .iter()
                    .rev()
                    .map(|&id| (id, unevaluated)),
            ),
            ExprKind::Throw(operand) => {
                if let Some(operand) = operand {
                    self.expression_work.push((operand, unevaluated));
                }
            }
            ExprKind::PackExpansion(_) | ExprKind::SizeofPack(_) => {
                return Err(Error::Missing("template expression names"));
            }
            ExprKind::Literal(_) | ExprKind::Keyword(_) | ExprKind::Lambda(_) => (),
        }
        Ok(())
    }
    fn check_expression_name(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
        allow_undeclared: bool,
        allow_type: bool,
    ) -> Result<Option<BindingId>, Error> {
        let syntax_item = *self.tree.names.get(name);
        let node = &syntax_item;
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty expression name"))?;
        if !matches!(last.kind, NameKind::Identifier(_)) {
            return Ok(None);
        }
        if self.template_depth != 0
            && !node.global
            && self.tree.name_lists.get(node.components).len() == 1
            && let Some(arguments) = last.arguments
            && let NameKind::Identifier(identifier) = last.kind
        {
            let found = self.unqualified(scope, identifier, Filter::Ordinary)?;
            if found.is_none() && allow_undeclared
                || found.is_some_and(|id| self.graph.binding(id).kind == BindingKind::Function)
            {
                self.template_argument_names(arguments, scope, last.location)?;
                return Ok(found);
            }
            if found.is_none() {
                return Err(Error::at(node.location, "undeclared name"));
            }
        }
        let binding = match self.lookup(name, scope, Filter::Ordinary) {
            Ok(binding) => binding,
            // type_qualifier has already resolved a typed dependent root.
            // Definition-time name checking does not select its members;
            // their retained argument syntax still needs its name checks.
            Err(Error::Missing("dependent qualified name lookup")) if self.template_depth != 0 => {
                for syntax_index in 0..self.tree.name_lists.get(node.components).len() {
                    let syntax_item = self.tree.name_lists.get(node.components)[syntax_index];
                    let component = &syntax_item;
                    if let Some(arguments) = component.arguments {
                        self.template_argument_names(arguments, scope, component.location)?;
                    }
                }
                return Ok(None);
            }
            Err(Error::Diagnostic {
                message: "undeclared name",
                ..
            }) if allow_undeclared
                && !node.global
                && self.tree.name_lists.get(node.components).len() == 1 =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let kind = self.graph.binding(binding).kind;
        if matches!(kind, BindingKind::Namespace | BindingKind::NamespaceAlias) {
            return Err(Error::at(node.location, "namespace name used as a value"));
        }
        if matches!(kind, BindingKind::Type | BindingKind::Alias) && !allow_type {
            return Err(Error::at(node.location, "type name used as a value"));
        }
        Ok(Some(binding))
    }
    fn value_initialized_class(&mut self, ty: TypeId) -> Result<(), Error> {
        if self.suppress_implicit || self.template_depth != 0 {
            return Ok(());
        }
        let TypeKind::Class(entity) = self.graph.types.get(ty).kind else {
            return Ok(());
        };
        let class = self.graph.entity(entity);
        if !class.complete
            || class.user_constructor.is_some()
            || class.implicit_default.is_some()
            || class.anonymous_class.is_some()
        {
            return Ok(());
        }
        // Types mode does not demand constructors for anonymous classes,
        // including ones whose printed name was adopted from a typedef.
        let scope = class
            .scope
            .ok_or(Error::Invariant("complete class without scope"))?;
        let name = class.name;
        let constructor = self
            .graph
            .add_scope(ScopeKind::Function, scope, Some(scope), name)?;
        self.graph.entity_mut(entity).implicit_default = Some(constructor);
        Ok(())
    }
    fn push_initializer(&mut self, initializer: ast::Initializer, unevaluated: bool) {
        match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id) => {
                self.expression_work.push((id, unevaluated));
            }
            InitializerKind::Parenthesized(arguments) => self.expression_work.extend(
                self.tree
                    .expr_lists
                    .get(arguments)
                    .iter()
                    .map(|&id| (id, unevaluated)),
            ),
            InitializerKind::Default | InitializerKind::Delete => (),
        }
    }
    pub(super) fn initializer_names(
        &mut self,
        initializer: ast::Initializer,
        scope: ScopeId,
        mut ty: TypeId,
    ) -> Result<(), Error> {
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let saved = self.suppress_implicit;
        self.suppress_implicit |= matches!(self.graph.types.get(ty).kind, TypeKind::Class(_));
        let result = match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id) => self.expression(id, scope),
            InitializerKind::Parenthesized(arguments) => {
                let mut result = Ok(());
                for syntax_index in 0..self.tree.expr_lists.get(arguments).len() {
                    let id = self.tree.expr_lists.get(arguments)[syntax_index];
                    result = self.expression(id, scope);
                    if result.is_err() {
                        break;
                    }
                }
                result
            }
            InitializerKind::Default | InitializerKind::Delete => Ok(()),
        };
        self.suppress_implicit = saved;
        result
    }
}
