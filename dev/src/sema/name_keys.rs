// Translated from template_placeholders.cpp::name_words/parameter_word/resolution_words. See NOTICE.
//! Names in structural keys use semantic identities or explicit free-name facts.
use super::{
    Error,
    analyzer::Analyzer,
    expression_keys::{ExpressionKey, KeyNode, KeyWord},
    graph::{BindingId, BindingKind, ScopeId},
    lookup::Filter,
    types::{Cv, TypeId, TypeKind},
};
use crate::syntax::ast::{self, NameKind, TemplateArgumentKind};
impl Analyzer<'_> {
    pub(super) fn key_name(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
        resolve: bool,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        let node = *self.tree.names.get(name);
        let component_count = self.tree.name_lists.get(node.components).len();
        key.words.extend([
            KeyWord::Node(KeyNode::Name),
            KeyWord::Count(component_count),
            KeyWord::Flag(node.global),
        ]);
        for index in 0..component_count {
            let component = self.tree.name_lists.get(node.components)[index];
            match component.kind {
                NameKind::Operator { op, array } => key
                    .words
                    .extend([KeyWord::Operator(op.token), KeyWord::Flag(array)]),
                NameKind::Identifier(name) if !resolve || index != 0 || node.global => {
                    key.words.push(KeyWord::Identifier(name));
                }
                NameKind::Identifier(name) => {
                    let filter = if component_count > 1 {
                        Filter::Prefix
                    } else {
                        Filter::Ordinary
                    };
                    let found = match self.unqualified(scope, name, filter) {
                        Ok(found) => found,
                        Err(Error::Diagnostic { .. }) => return Ok(false),
                        Err(error) => return Err(error),
                    };
                    if let Some(binding) = found {
                        if !self.key_binding(binding, scope, key)? {
                            return Ok(false);
                        }
                    } else {
                        key.words.push(KeyWord::FreeName(name));
                        key.dependent = true;
                    }
                }
                // These are also unknown forms in C++ name_words.
                _ => return Ok(false),
            }
            if let Some(arguments) = component.arguments {
                let argument_count = self.tree.template_arguments.get(arguments).len();
                key.words.extend([
                    KeyWord::Node(KeyNode::Arguments),
                    KeyWord::Count(argument_count),
                ]);
                for index in 0..argument_count {
                    let argument = self.tree.template_arguments.get(arguments)[index];
                    let Some(ty) =
                        self.key_template_argument(argument, scope, component.location)?
                    else {
                        return Ok(false);
                    };
                    self.key_type(ty, key)?;
                }
            }
        }
        if resolve && (component_count > 1 || node.global) {
            self.key_resolution(name, scope, key)?;
        }
        Ok(true)
    }

    /// `name_words` tests for template names before interpreting a type-id.
    /// A non-template expression expansion makes the structural key unknown.
    fn key_template_argument(
        &mut self,
        argument: ast::TemplateArgument,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<TypeId>, Error> {
        if let Some((name, expansion)) = self.plain_template_name(argument)
            && let Some(ty) = self.template_name_argument(name, scope, false)?
        {
            let ty = if expansion {
                self.graph
                    .types
                    .intern(TypeKind::Expansion(ty), Cv::default())?
            } else {
                ty
            };
            return Ok(Some(ty));
        }
        if argument.pack {
            return Ok(None);
        }
        let ty = match argument.kind {
            TemplateArgumentKind::Type(ty) => self.type_syntax(ty, scope)?,
            TemplateArgumentKind::Expression(_) => {
                self.template_argument(argument, scope, location)?
            }
        };
        Ok(Some(ty))
    }

    pub(super) fn key_binding(
        &mut self,
        id: BindingId,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        let binding = self.graph.binding(id);
        if binding.kind == BindingKind::Parameter {
            let Some(ty) = binding.ty else {
                return Ok(false);
            };
            if let Some(position) = self.graph.prototype_parameter(binding.entity) {
                key.words.push(KeyWord::Parameter {
                    index: position,
                    level: self.parameter_level(id, scope)?,
                });
                self.key_type(ty, key)?;
                return Ok(true);
            }
        }
        if binding.kind == BindingKind::Alias
            && self
                .graph
                .entity(binding.entity)
                .template_parameter
                .is_some()
        {
            self.key_type(
                binding
                    .ty
                    .ok_or(Error::Invariant("argument alias without type"))?,
                key,
            )?;
        } else if self
            .graph
            .entity(binding.entity)
            .template_parameter
            .is_some()
        {
            let ty = self
                .graph
                .entity(binding.entity)
                .ty
                .ok_or(Error::Invariant("template parameter without type"))?;
            self.key_type(ty, key)?;
        } else {
            if matches!(
                binding.kind,
                BindingKind::Type | BindingKind::Alias | BindingKind::Variable
            ) {
                key.dependent |= binding
                    .ty
                    .is_some_and(|ty| self.exception_dependent_type(ty));
            }
            key.words.push(KeyWord::Entity(binding.entity));
        }
        Ok(true)
    }

    fn parameter_level(&self, binding: BindingId, mut scope: ScopeId) -> Result<u32, Error> {
        let owner = self.graph.binding_scope(binding);
        let mut level = 0u32;
        let mut within = false;
        loop {
            let node = self.graph.scope(scope);
            if node.kind == super::graph::ScopeKind::Function && node.owner.is_none() {
                if node.guarded && !within {
                    level = level
                        .checked_add(1)
                        .ok_or(Error::Limit("function parameter nesting limit"))?;
                }
                within = node.guarded;
            }
            if scope == owner {
                return Ok(level);
            }
            let Some(parent) = node.parent else {
                return Ok(level);
            };
            scope = parent;
        }
    }

    fn key_resolution(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<(), Error> {
        let node = *self.tree.names.get(name);
        let Some(last) = self.tree.name_lists.get(node.components).last().copied() else {
            return Err(Error::Invariant("empty name in structural key"));
        };
        if !matches!(last.kind, NameKind::Identifier(_)) {
            key.words.push(KeyWord::Absent);
            return Ok(());
        }
        let found = self.lookup_result(name, scope, Filter::Ordinary)?;
        let binding = found
            .binding
            .ok_or_else(|| Error::at(node.location, "undeclared name"))?;
        let binding = self.graph.binding(binding);
        if let Some(member) = self.graph.dependent_members.get(binding.entity) {
            self.key_type(member.qualifier, key)?;
        } else if binding.kind == BindingKind::Function {
            key.words.push(KeyWord::Node(KeyNode::Function));
            if found.functions.as_slice().len() <= 1
                && last.arguments.is_none()
                && self.graph.entity(binding.entity).template_head.is_none()
            {
                key.words.push(KeyWord::Entity(binding.entity));
            } else {
                key.words.push(KeyWord::Absent);
            }
        } else {
            key.words.push(KeyWord::Entity(binding.entity));
        }
        Ok(())
    }
}
