// Translated from sema/template_dependence.cpp and template_placeholders.cpp::name_words.
// See NOTICE.
//! Conservative definition-time type dependence, independent of concrete facts.
use super::{
    Error,
    analyzer::Analyzer,
    expression_keys::{ExpressionKey, KeyWord},
    graph::{BindingKind, ScopeId, ScopeKind},
    lookup::Filter,
    types::{TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::{
        ast::{self, ExprKind, NameKind},
        storage::List,
    },
};

impl Analyzer<'_> {
    pub(super) fn note_dependent_use(&self) {
        self.dependent_uses
            .set(self.dependent_uses.get().wrapping_add(1));
    }

    /// C++ `lookup.cpp::DependentClassScope`, including concrete instances and
    /// local classes: dependence here describes the pattern, not today's type.
    pub(super) fn dependent_lookup_class(&self, mut scope: ScopeId) -> bool {
        loop {
            let node = self.graph.scope(scope);
            if node.kind != ScopeKind::Class {
                return node.kind != ScopeKind::Namespace;
            }
            if node.owner.is_none_or(|class| {
                self.graph.entity(class).template_head.is_some()
                    || self.graph.class_templates.instance(class).is_some()
                    || self
                        .graph
                        .class_templates
                        .definitions
                        .source(class)
                        .is_some()
            }) {
                return true;
            }
            let Some(parent) = node.parent else {
                return false;
            };
            scope = parent;
        }
    }

    pub(super) fn parameter_dependence(
        &self,
        function: super::graph::EntityId,
        count: usize,
    ) -> Result<Option<TypeId>, Error> {
        let mut source = function;
        for _ in 0..64 {
            let next = self
                .graph
                .function_templates
                .primary(source)
                .map(|binding| self.graph.binding(binding).entity)
                .or_else(|| self.graph.function_templates.bodies.source(source));
            let Some(next) = next.filter(|&next| next != source) else {
                break;
            };
            source = next;
        }
        if source == function {
            return Ok(None);
        }
        let ty = self
            .graph
            .entity(source)
            .ty
            .ok_or(Error::Invariant("parameter pattern without type"))?;
        Ok((self.graph.types.function_parameter_count(ty)? == count).then_some(ty))
    }

    pub(super) fn key_call_resolution(
        &mut self,
        callee: ast::ExprId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
        start: usize,
        key: &mut ExpressionKey,
    ) -> Result<(), Error> {
        let ExprKind::Id(name) = self.tree.expressions.get(callee).kind else {
            return Ok(());
        };
        let node = self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if node.global || components.len() != 1 {
            return Ok(());
        }
        let NameKind::Identifier(name) = components[0].kind else {
            return Ok(());
        };
        let found = self.unqualified_result(scope, name.into(), Filter::Ordinary)?;
        let Some(binding) = found.binding else {
            return Ok(());
        };
        if self.graph.binding(binding).kind != BindingKind::Function {
            return Ok(());
        }
        let mut dependent = false;
        for index in 0..self.tree.expr_lists.get(arguments).len() {
            let argument = self.tree.expr_lists.get(arguments)[index];
            if self.type_dependent_expression(argument, scope)? {
                dependent = true;
                break;
            }
        }
        let Some(word @ KeyWord::Entity(_)) = key.words.get_mut(start + 4) else {
            return Ok(());
        };
        let entity = self.graph.binding(binding).entity;
        if dependent {
            *word = KeyWord::DependentCallee(entity, name);
            key.dependent = true;
        } else if self.semantic()
            && found.functions.as_slice().len() == 1
            && self.graph.entity(entity).template_head.is_none()
        {
            *word = KeyWord::ResolvedFunction(entity);
        }
        Ok(())
    }

    pub(super) fn type_dependent_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        match self.tree.expressions.get(id).kind {
            ExprKind::Literal(_)
            | ExprKind::SizeofPack(_)
            | ExprKind::Sizeof { .. }
            | ExprKind::TypeTrait { .. }
            | ExprKind::Delete { .. }
            | ExprKind::Throw(_) => Ok(false),
            ExprKind::Keyword(keyword) => {
                Ok(keyword == Keyword::This && self.dependent_class_scope(scope))
            }
            ExprKind::Paren(inner)
            | ExprKind::Unary { operand: inner, .. }
            | ExprKind::Postfix { operand: inner, .. } => {
                self.type_dependent_expression(inner, scope)
            }
            ExprKind::Binary { left, right, .. }
            | ExprKind::Subscript {
                base: left,
                index: right,
            } => Ok(self.type_dependent_expression(left, scope)?
                || self.type_dependent_expression(right, scope)?),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => Ok(self.type_dependent_expression(condition, scope)?
                || self.type_dependent_expression(then_expr, scope)?
                || self.type_dependent_expression(else_expr, scope)?),
            ExprKind::Call { callee, arguments } => {
                if self.type_dependent_expression(callee, scope)? {
                    return Ok(true);
                }
                for index in 0..self.tree.expr_lists.get(arguments).len() {
                    let argument = self.tree.expr_lists.get(arguments)[index];
                    if self.type_dependent_expression(argument, scope)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ExprKind::Member { base, member, .. } => {
                if self.type_dependent_expression(base, scope)? {
                    return Ok(true);
                }
                let name = self.tree.names.get(member);
                Ok(self
                    .tree
                    .name_lists
                    .get(name.components)
                    .iter()
                    .any(|component| {
                        !matches!(component.kind, NameKind::Identifier(_))
                            || component.arguments.is_some()
                    }))
            }
            ExprKind::Cast { ty, operand, .. } => {
                let ty = self.type_syntax(ty, scope)?;
                Ok(self.exception_dependent_type(ty)
                    || self.type_dependent_expression(operand, scope)?)
            }
            ExprKind::Braced { elements, .. } => {
                for index in 0..self.tree.expr_lists.get(elements).len() {
                    let element = self.tree.expr_lists.get(elements)[index];
                    if self.type_dependent_expression(element, scope)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ExprKind::Id(name) => self.dependent_name_expression(name, scope),
            // These forms likewise remain conservative in C++'s ordinary
            // dependence walk. Precise member-template checking is separate.
            ExprKind::FunctionalCast { .. }
            | ExprKind::New { .. }
            | ExprKind::Lambda(_)
            | ExprKind::PackExpansion(_) => Ok(true),
        }
    }

    pub(super) fn dependent_class_scope(&self, mut scope: ScopeId) -> bool {
        loop {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::Class
                && node
                    .owner
                    .is_some_and(|entity| self.graph.entity(entity).template_head.is_some())
            {
                return true;
            }
            let Some(parent) = node.parent else {
                return false;
            };
            scope = parent;
        }
    }

    fn dependent_name_expression(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if components.iter().any(|component| {
            !matches!(component.kind, NameKind::Identifier(_)) || component.arguments.is_some()
        }) {
            return Ok(true);
        }
        let NameKind::Identifier(name_key) = components
            .last()
            .ok_or(Error::Invariant("empty dependence name"))?
            .kind
        else {
            return Ok(true);
        };
        let prefix = match self.prefix(name, scope) {
            Ok(prefix) => prefix,
            Err(Error::Missing("dependent qualified name lookup")) => return Ok(true),
            Err(error) => return Err(error),
        };
        if let Some(mut prefix) = prefix {
            while self.graph.scope(prefix).kind != ScopeKind::Namespace {
                let node = self.graph.scope(prefix);
                if node.kind != ScopeKind::Class
                    || node.owner.is_none()
                    || self.dependent_class_scope(prefix)
                {
                    return Ok(true);
                }
                let Some(parent) = node.parent else {
                    return Ok(true);
                };
                prefix = parent;
            }
        }
        let found = if let Some(prefix) = prefix {
            self.qualified_result(prefix, name_key.into(), Filter::Ordinary)?
        } else {
            self.unqualified_result(scope, name_key.into(), Filter::Ordinary)?
        };
        let Some(binding) = found.binding.filter(|_| !found.ambiguous) else {
            return Ok(prefix.is_none() && found.binding.is_none());
        };
        let binding = self.graph.binding(binding);
        if self
            .graph
            .entity(binding.entity)
            .home
            .is_some_and(|scope| self.dependent_class_scope(scope))
        {
            return Ok(true);
        }
        Ok(match binding.kind {
            BindingKind::Variable | BindingKind::Parameter => binding
                .ty
                .is_none_or(|ty| self.exception_dependent_type(ty)),
            BindingKind::Enumerator => false,
            BindingKind::Function => found.functions.iter().any(|&binding| {
                self.graph
                    .binding(binding)
                    .ty
                    .is_some_and(|ty| self.exception_dependent_type(ty))
            }),
            _ => true,
        })
    }
}

use std::collections::HashSet;

impl super::graph::Graph {
    fn type_has_template_owner(&self, entity: super::graph::EntityId) -> bool {
        let node = self.entity(entity);
        let mut scope = node
            .enumeration
            .map(|enumeration| enumeration.owner)
            .or(node.home);
        while let Some(current) = scope {
            let node = self.scope(current);
            if node.kind != ScopeKind::Class {
                break;
            }
            if node
                .owner
                .is_some_and(|owner| self.entity(owner).template_head.is_some())
            {
                return true;
            }
            scope = node.parent;
        }
        false
    }

    pub(super) fn dependent_type(&self, root: TypeId) -> bool {
        let mut work = vec![root];
        let mut visited = HashSet::new();
        while let Some(ty) = work.pop() {
            if !visited.insert(ty) {
                continue;
            }
            match &self.types.get(ty).kind {
                TypeKind::TemplateParameter { .. }
                | TypeKind::Expansion(_)
                | TypeKind::BoundPack { .. } => return true,
                TypeKind::Pack(elements) => work.extend(elements.iter().copied()),
                TypeKind::Class(entity)
                    if self.entity(*entity).template_head.is_some()
                        || self.type_has_template_owner(*entity)
                        || self
                            .class_templates
                            .instance(*entity)
                            .is_some_and(|instance| instance.dependent) =>
                {
                    return true;
                }
                TypeKind::Enum(entity) if self.type_has_template_owner(*entity) => return true,
                TypeKind::Pointer(element) | TypeKind::Reference { element, .. } => {
                    work.push(*element);
                }
                TypeKind::Array {
                    element, operand, ..
                } => {
                    work.push(*element);
                    work.extend(operand);
                }
                TypeKind::MemberPointer { owner, element } => {
                    work.push(*owner);
                    work.push(*element);
                }
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters.iter().copied());
                }
                _ => (),
            }
        }
        false
    }
}
