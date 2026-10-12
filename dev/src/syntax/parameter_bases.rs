// Translated from syntax/parser_names.cpp::parameter_base_lookup and its owners. See NOTICE.
//! Syntactic lookup through substituted type-parameter bases, without instantiation.
use super::{
    ParseError,
    ast::{
        Name, NameKind, Specifier, TemplateArgument, TemplateArgumentKind, TemplateParameterKind,
    },
    parser::Parser,
    scopes::{Binding, Category, ParameterBase, ScopeId, ScopeKind, Scopes},
    storage::List,
};
use crate::lex::identifiers::NameId;

impl Parser<'_> {
    pub(super) fn record_parameter_base(
        &mut self,
        scope: ScopeId,
        name: NameId,
        pack: bool,
    ) -> Result<(), ParseError> {
        let Some(head) = self.template_head else {
            return Ok(());
        };
        for (index, parameter) in self
            .tree
            .template_parameters
            .get(head.parameters)
            .iter()
            .enumerate()
        {
            if let TemplateParameterKind::Type {
                name: Some(parameter_name),
                parameters: None,
                ..
            } = parameter.kind
                && parameter_name == name
            {
                if parameter.pack == pack {
                    let index = u32::try_from(index)
                        .map_err(|_| ParseError::Limit("template parameter index limit"))?;
                    self.scopes
                        .add_parameter_base(scope, ParameterBase { index, pack });
                }
                break;
            }
        }
        Ok(())
    }
    pub(super) fn parameter_base_lookup(
        &mut self,
        scope: ScopeId,
        arguments: List<TemplateArgument>,
        name: NameId,
        prefix_only: bool,
        depth: u32,
    ) -> Option<Binding> {
        // The frozen parser bounds this syntactic approximation at eight
        // nested substitutions; semantic instantiation is a later owner.
        if depth > 8 {
            return None;
        }
        for base_index in 0..self.scopes.parameter_bases(scope).len() {
            let base = self.scopes.parameter_bases(scope)[base_index];
            let start = base.index as usize;
            let arguments_len = self.tree.template_arguments.get(arguments).len();
            let end = if base.pack {
                arguments_len
            } else {
                start.saturating_add(1)
            };
            for index in start..end.min(arguments_len) {
                let argument = self.tree.template_arguments.get(arguments)[index];
                let Some((class, class_name)) = self.argument_class(argument) else {
                    continue;
                };
                let component = *self
                    .tree
                    .name_lists
                    .get(self.tree.names.get(class_name).components)
                    .last()?;
                let record = if component.arguments.is_none()
                    && !self.scopes.parameter_bases(class).is_empty()
                {
                    self.specialization_record(class_name)
                } else {
                    0
                };
                let nested_arguments = component
                    .arguments
                    .or_else(|| self.recorded_arguments(record));
                let found = self.scopes.lookup_specialization(
                    class,
                    name,
                    prefix_only,
                    nested_arguments.is_some(),
                );
                if found.is_some() {
                    return found;
                }
                if let Some(arguments) = nested_arguments
                    && let Some(found) =
                        self.parameter_base_lookup(class, arguments, name, prefix_only, depth + 1)
                {
                    return Some(found);
                }
            }
        }
        None
    }
    fn argument_class(&mut self, argument: TemplateArgument) -> Option<(ScopeId, Name)> {
        let TemplateArgumentKind::Type(ty) = argument.kind else {
            return None;
        };
        let ty = self.tree.types.get(ty);
        if argument.pack || ty.declarator.is_some() {
            return None;
        }
        let [Specifier::Named(name)] = self.tree.specs.get(ty.specifiers) else {
            return None;
        };
        let name = *name;
        let node = *self.tree.names.get(name);
        let mut scope = node.global.then_some(Scopes::global());
        let count = self.tree.name_lists.get(node.components).len();
        for index in 0..count {
            let NameKind::Identifier(identifier) =
                self.tree.name_lists.get(node.components)[index].kind
            else {
                return None;
            };
            let prefix_only = index + 1 < count;
            let binding = if let Some(scope) = scope {
                self.scopes
                    .lookup_specialization(scope, identifier, prefix_only, false)
            } else {
                self.scopes.lookup_name(identifier, prefix_only)
            }?;
            scope = binding.scope;
            if scope.is_none_or(|scope| self.scopes.pending_class(scope)) {
                return None;
            }
        }
        scope
            .filter(|scope| {
                matches!(
                    self.scopes.kind(*scope),
                    ScopeKind::Class(_) | ScopeKind::AnonymousClass
                )
            })
            .map(|scope| (scope, name))
    }
    pub(super) fn recorded_arguments(&self, record: u8) -> Option<List<TemplateArgument>> {
        record
            .checked_sub(1)
            .and_then(|index| self.specialization_records.get(usize::from(index)))
            .copied()
    }
    fn specialization_record(&mut self, name: Name) -> u8 {
        let node = *self.tree.names.get(name);
        let Some(component) = self.tree.name_lists.get(node.components).last() else {
            return 0;
        };
        if let Some(arguments) = component.arguments {
            // Match the frozen binding's one-byte record capacity. The stored
            // list is a handle into the existing syntax, never a token replay.
            let Ok(record) = u8::try_from(self.specialization_records.len() + 1) else {
                return 0;
            };
            self.specialization_records.push(arguments);
            return record;
        }
        if let Some(identifier) = self.simple_name(name) {
            return self
                .scopes
                .lookup(identifier)
                .map_or(0, |binding| binding.specialization);
        }
        0
    }
    pub(super) fn typedef_record(&mut self, specifiers: List<Specifier>) -> u8 {
        for index in 0..self.tree.specs.get(specifiers).len() {
            if let Specifier::Named(name) = self.tree.specs.get(specifiers)[index] {
                let node = *self.tree.names.get(name);
                let Some(binding) = node.binding else {
                    return 0;
                };
                if binding
                    .scope
                    .is_none_or(|scope| self.scopes.parameter_bases(scope).is_empty())
                {
                    return 0;
                }
                if self.name_has_arguments(name)
                    && (binding.category != Category::TypeTemplate || node.mentions_parameters)
                {
                    return 0;
                }
                return self.specialization_record(name);
            }
        }
        0
    }
}
