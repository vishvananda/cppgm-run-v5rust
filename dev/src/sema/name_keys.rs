// Translated from template_placeholders.cpp::name_words/parameter_word/resolution_words. See NOTICE.
//! Names in structural keys use semantic identities or explicit free-name facts.
use super::{
    Error,
    analyzer::Analyzer,
    expression_keys::{ExpressionKey, KeyNode, KeyWord},
    graph::{BindingId, BindingKind, ScopeId},
    lookup::{Filter, Specialization},
    types::{Cv, TypeId, TypeKind},
};
use crate::syntax::{
    ast::{self, NameKind, TemplateArgumentKind},
    storage::List,
};

#[derive(Clone, Copy)]
pub(super) struct KeyCallee {
    pub binding: BindingId,
    pub component: ast::NameComponent,
    pub sole: bool,
}
impl Analyzer<'_> {
    pub(super) fn key_name(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
        resolve: bool,
        call: Option<List<ast::ExprId>>,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        let start = key.words.len();
        if resolve {
            key.words.push(KeyWord::Node(KeyNode::Id));
        }
        let node = *self.tree.names.get(name);
        let component_count = self.tree.name_lists.get(node.components).len();
        key.words.extend([
            KeyWord::Node(KeyNode::Name),
            KeyWord::Count(component_count),
            KeyWord::Flag(node.global),
        ]);
        let mut last_arguments = None;
        let mut callee = None;
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
                    let found = match self.unqualified_result(scope, name.into(), filter) {
                        Ok(found) => found,
                        Err(Error::Diagnostic { .. }) => return Ok(false),
                        Err(error) => return Err(error),
                    };
                    if let Some(binding) = found.binding {
                        if call.is_some() && component_count == 1 {
                            callee = Some(KeyCallee {
                                binding,
                                component,
                                sole: self.semantic() && found.functions.as_slice().len() == 1,
                            });
                        }
                        // A substituted integral argument is keyed as its
                        // literal value, using this same ordinary lookup.
                        if call.is_none()
                            && component_count == 1
                            && component.arguments.is_none()
                            && self.graph.binding(binding).kind == BindingKind::Enumerator
                            && self.graph.scope(self.graph.binding_scope(binding)).kind
                                == super::graph::ScopeKind::TemplateParameters
                            && let super::constants::ConstantState::Value(value) = self
                                .graph
                                .entity(self.graph.binding(binding).entity)
                                .constant
                        {
                            key.words.truncate(start);
                            key.words.push(KeyWord::Constant(value.ty, value.bits));
                            return Ok(true);
                        }
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
                last_arguments = Some(key.words.len());
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
            self.key_resolution(name, scope, last_arguments, key)?;
        }
        if let Some(callee) = callee
            && let Some(arguments) = call
        {
            self.key_call_resolution(callee, arguments, scope, start, key)?;
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
            // A frame can bind a non-type parameter to a retained expression,
            // whose entity has no positional template-parameter record.
            key.dependent |= self
                .graph
                .template_operands
                .pattern(binding.entity)
                .is_some();
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
        last_arguments: Option<usize>,
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
        // Structural resolution names the original overload set. Only a sole
        // template with a complete explicit head may resolve a signature here;
        // ordinary use-site specialization would eagerly reject unused keys.
        let found =
            self.lookup_result_in(name, scope, Filter::Ordinary, Specialization::NonFunction)?;
        let binding = found
            .binding
            .ok_or_else(|| Error::at(node.location, "undeclared name"))?;
        let binding_id = binding;
        let binding = self.graph.binding(binding);
        if let Some(member) = self.graph.dependent_members.get(binding.entity) {
            self.key_type(member.qualifier, key)?;
        } else if binding.kind == BindingKind::Function {
            let resolved = if self.semantic()
                && found.functions.as_slice().len() <= 1
                && let Some(start) = last_arguments.filter(|_| last.arguments.is_some())
            {
                let given = key.words[start..]
                    .iter()
                    .map(|word| {
                        if let KeyWord::Type(ty) = word {
                            Ok(*ty)
                        } else {
                            Err(Error::Invariant("qualified key without argument type"))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.key_explicit_specialization(binding_id, &given, last.location)?
            } else {
                None
            };
            key.words.push(KeyWord::Node(KeyNode::Function));
            if let Some(entity) = resolved {
                key.words.push(KeyWord::Entity(entity));
            } else if found.functions.as_slice().len() <= 1
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
