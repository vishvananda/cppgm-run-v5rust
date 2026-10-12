// Translated from template_placeholders.cpp::name_words/parameter_word/resolution_words. See NOTICE.
//! Names in structural keys use semantic identities or explicit free-name facts.
use super::{
    Error,
    analyzer::Analyzer,
    expression_keys::{ExpressionKey, KeyNode, KeyWord},
    graph::{BindingId, BindingKind, ScopeId},
    lookup::Filter,
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
        let components = self.tree.name_lists.get(node.components);
        key.words.extend([
            KeyWord::Node(KeyNode::Name),
            KeyWord::Count(components.len()),
            KeyWord::Flag(node.global),
        ]);
        for (index, component) in components.iter().enumerate() {
            match component.kind {
                NameKind::Operator { op, array } => key
                    .words
                    .extend([KeyWord::Operator(op.token), KeyWord::Flag(array)]),
                NameKind::Identifier(name) if !resolve || index != 0 || node.global => {
                    key.words.push(KeyWord::Identifier(name));
                }
                NameKind::Identifier(name) => {
                    let filter = if components.len() > 1 {
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
                        if !self.key_binding(binding, key)? {
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
                let arguments = self.tree.template_arguments.get(arguments);
                key.words.extend([
                    KeyWord::Node(KeyNode::Arguments),
                    KeyWord::Count(arguments.len()),
                ]);
                for argument in arguments {
                    if argument.pack {
                        return Err(Error::Missing("template pack name key"));
                    }
                    match argument.kind {
                        TemplateArgumentKind::Type(ty) => {
                            let ty = self.type_syntax(ty, scope)?;
                            self.key_type(ty, key)?;
                        }
                        TemplateArgumentKind::Expression(_) => {
                            return Err(Error::Missing("non-type template argument key"));
                        }
                    }
                }
            }
        }
        if resolve && (components.len() > 1 || node.global) {
            self.key_resolution(name, scope, key)?;
        }
        Ok(true)
    }

    fn key_binding(&mut self, id: BindingId, key: &mut ExpressionKey) -> Result<bool, Error> {
        let binding = self.graph.binding(id);
        if binding.kind == BindingKind::Parameter {
            let Some(ty) = binding.ty else {
                return Ok(false);
            };
            if let Some(position) = self.graph.prototype_parameter(binding.entity) {
                key.words.push(KeyWord::Parameter(position));
                self.key_type(ty, key)?;
                return Ok(true);
            }
        }
        if self
            .graph
            .entity(binding.entity)
            .template_parameter
            .is_some()
        {
            let ty = binding
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

    fn key_resolution(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<(), Error> {
        let node = self.tree.names.get(name);
        let Some(last) = self.tree.name_lists.get(node.components).last() else {
            return Err(Error::Invariant("empty name in structural key"));
        };
        if !matches!(last.kind, NameKind::Identifier(_)) {
            key.words.push(KeyWord::Absent);
            return Ok(());
        }
        let binding = self.lookup(name, scope, Filter::Ordinary)?;
        let binding = self.graph.binding(binding);
        if binding.kind == BindingKind::Function {
            key.words.push(KeyWord::Node(KeyNode::Function));
            if last.arguments.is_none() && self.graph.entity(binding.entity).template_head.is_none()
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
