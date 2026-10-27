// Translated from syntax/parser_names.cpp::resolve_component/advance_prefix/note_type_name. See NOTICE.
//! Typed parser lookup context and the guesses retained for later readings.
use super::ast::SpecifierKind;
use super::{
    ast::{Name, PrefixState, Specifier, TemplateArgument, TemplatePrefix},
    parser::Parser,
    scopes::{Binding, Categories, Category, Guesses, ScopeId, Scopes},
    storage::List,
};
use crate::lex::identifiers::NameId;

#[derive(Clone, Copy)]
pub(super) struct NamePrefix {
    pub scope: Option<ScopeId>,
    pub state: PrefixState,
    pub template: TemplatePrefix,
    pub approximate: bool,
    pub uncertain: bool,
    arguments: Option<List<TemplateArgument>>,
}
impl NamePrefix {
    pub(super) fn new(global: bool) -> Self {
        Self {
            scope: global.then_some(Scopes::global()),
            state: if global {
                PrefixState::Known
            } else {
                PrefixState::Unqualified
            },
            template: TemplatePrefix::Ordinary,
            approximate: false,
            uncertain: false,
            arguments: None,
        }
    }
    pub(super) fn make_dependent(&mut self, uncertain: bool) {
        self.scope = None;
        self.state = PrefixState::Dependent;
        self.template = TemplatePrefix::Ordinary;
        self.approximate = false;
        self.uncertain = uncertain;
        self.arguments = None;
    }
}
impl Parser<'_> {
    pub(super) fn observe_name_binding(&mut self, binding: Option<Binding>) {
        if let Some(binding) = binding {
            if binding.pack {
                self.pack_mentions = self.pack_mentions.wrapping_add(1);
            }
            if binding.parameter {
                self.template_mentions = self.template_mentions.wrapping_add(1);
            }
        }
    }
    pub(super) fn arguments_for_name(
        &mut self,
        name: NameId,
        prefix: &NamePrefix,
        member_context: bool,
        template_keyword: bool,
    ) -> Result<Option<List<TemplateArgument>>, super::ParseError> {
        if !self.at(crate::lex::pp_lexer::Punctuator::Less)? {
            return Ok(None);
        }
        if template_keyword {
            return self.template_arguments().map(Some);
        }
        if member_context && prefix.state == PrefixState::Unqualified {
            return if self.scopes.ever_template(name) {
                self.optional_template_arguments(true)
            } else {
                Ok(None)
            };
        }
        let binding = self.resolve_component(name, prefix, false);
        if binding.is_some_and(|binding| {
            matches!(
                binding.category,
                Category::TypeTemplate | Category::FunctionTemplate
            )
        }) {
            self.template_arguments().map(Some)
        } else if binding.is_none() && prefix.state != PrefixState::Dependent {
            self.optional_template_arguments(false)
        } else {
            Ok(None)
        }
    }
    pub(super) fn resolve_component(
        &mut self,
        name: NameId,
        prefix: &NamePrefix,
        prefix_only: bool,
    ) -> Option<Binding> {
        match prefix.state {
            PrefixState::Unqualified => self.scopes.lookup_name(name, prefix_only),
            PrefixState::Known => {
                let scope = prefix.scope?;
                let mut binding = self.scopes.lookup_specialization(
                    scope,
                    name,
                    prefix_only,
                    prefix.template == TemplatePrefix::Specialization,
                );
                if binding.is_none()
                    && (prefix.template == TemplatePrefix::Specialization)
                    && let Some(arguments) = prefix.arguments
                {
                    binding = self.parameter_base_lookup(scope, arguments, name, prefix_only, 0);
                }
                if (prefix.template == TemplatePrefix::Dependent)
                    && binding.is_some()
                    && self.scopes.has_conflicts()
                    && self.scopes.member_categories_differ(scope, name)
                    && !self.scopes.current_instantiation(scope)
                {
                    return Some(Binding {
                        pack: false,
                        category: Category::Value,
                        parameter: false,
                        scope: None,
                        specialization: 0,
                        alternatives: Categories::default(),
                        guesses: Guesses::default(),
                    });
                }
                if binding.is_some_and(|binding| !binding.alternatives.is_empty())
                    || (prefix.approximate && !prefix_only)
                {
                    self.guessed_component(binding)
                } else {
                    binding
                }
            }
            PrefixState::Dependent if prefix.uncertain && !prefix_only => {
                self.guessed_component(None)
            }
            PrefixState::Unknown | PrefixState::Dependent => None,
        }
    }
    pub(super) fn advance_name_prefix(
        &mut self,
        name: NameId,
        arguments: Option<List<TemplateArgument>>,
        dependent_arguments: bool,
        prefix: &mut NamePrefix,
        declared: bool,
    ) {
        let binding = self.resolve_component(name, prefix, true);
        self.observe_name_binding(binding);
        let guesses = self.prefix_guesses(name, prefix, binding, declared);
        let was_dependent = prefix.state == PrefixState::Dependent;
        if let Some(binding) = binding
            && let Some(scope) = binding
                .scope
                .filter(|scope| !self.scopes.pending_class(*scope))
            && !guesses.dependent
        {
            prefix.scope = Some(scope);
            prefix.state = PrefixState::Known;
            prefix.template = if arguments.is_some() && binding.category == Category::TypeTemplate {
                if dependent_arguments {
                    TemplatePrefix::Dependent
                } else {
                    TemplatePrefix::Specialization
                }
            } else if arguments.is_none() && binding.specialization != 0 {
                TemplatePrefix::Specialization
            } else {
                TemplatePrefix::Ordinary
            };
            prefix.approximate = guesses.approximate;
            prefix.arguments = if prefix.template == TemplatePrefix::Specialization {
                arguments.or_else(|| self.recorded_arguments(binding.specialization))
            } else {
                None
            };
        } else if was_dependent
            || guesses.dependent
            || binding.is_some_and(|binding| {
                matches!(binding.category, Category::Type | Category::TypeTemplate)
            })
        {
            let uncertain = if was_dependent {
                prefix.uncertain
            } else {
                !self.scopes.in_template_definition()
            };
            prefix.make_dependent(uncertain);
        } else {
            *prefix = NamePrefix {
                state: PrefixState::Unknown,
                ..NamePrefix::new(false)
            };
        }
    }
    fn prefix_guesses(
        &self,
        name: NameId,
        prefix: &NamePrefix,
        binding: Option<Binding>,
        declared: bool,
    ) -> Guesses {
        let Some(binding) = binding.filter(|binding| binding.scope.is_some()) else {
            return Guesses::default();
        };
        let known = prefix.state == PrefixState::Known;
        let dependent = if !declared && known && (prefix.template == TemplatePrefix::Dependent) {
            prefix
                .scope
                .is_none_or(|scope| !self.scopes.current_instantiation(scope))
                || binding.guesses.dependent
        } else {
            !declared
                && binding.guesses.dependent
                && (prefix.state == PrefixState::Unqualified
                    || (known
                        && prefix
                            .scope
                            .is_some_and(|scope| self.scopes.current_instantiation(scope))))
        };
        if dependent {
            return Guesses {
                dependent: true,
                approximate: false,
            };
        }
        Guesses {
            dependent: false,
            approximate: binding.guesses.approximate
                || (known
                    && (prefix.approximate
                        || ((prefix.template == TemplatePrefix::Specialization)
                            && self.scopes.has_category_conflicts()
                            && prefix.scope.is_some_and(|scope| {
                                self.scopes.member_specialized(scope, name)
                            })))),
        }
    }
    pub(super) fn named_declaration_is_type(
        &mut self,
        name: Name,
        binding: Option<Binding>,
        definite: bool,
        followed: bool,
    ) -> bool {
        let node = self.tree.names.get(name);
        self.is_type_name(name, binding, definite, followed)
            || (binding.is_none()
                && definite
                && !(node.prefix_state != PrefixState::Unqualified && node.mentions_parameters)
                && self.last_identifier(name).is_some())
            || (self.scopes.declaration_kind() == Some(super::scopes::ScopeKind::Namespace)
                && self.tree.names.get(name).prefix_state == PrefixState::Known
                && binding.is_some_and(|binding| binding.category == Category::Value)
                && self.scopes.has_category_conflicts()
                && node
                    .prefix_scope
                    .zip(self.last_identifier(name))
                    .is_some_and(|(scope, identifier)| {
                        self.scopes
                            .lookup_specialization(
                                scope,
                                identifier,
                                false,
                                node.prefix_template == TemplatePrefix::Specialization,
                            )
                            .is_some_and(|found| found.alternatives.has_type())
                    }))
    }
    pub(super) fn note_type_guesses(
        &mut self,
        binding: &mut Binding,
        specifiers: List<Specifier>,
        alias_template: bool,
    ) {
        let Some(name) = self
            .tree
            .specs
            .get(specifiers)
            .iter()
            .find_map(|specifier| {
                if let SpecifierKind::Named(name) = &specifier.kind {
                    Some(*name)
                } else {
                    None
                }
            })
        else {
            return;
        };
        let node = *self.tree.names.get(name);
        if binding.scope.is_some() && self.last_identifier(name).is_some() {
            let known = node.prefix_state == PrefixState::Known;
            let approximate = node
                .binding
                .is_some_and(|binding| binding.guesses.approximate)
                || (known
                    && (node.prefix_guesses.approximate
                        || ((node.prefix_template == TemplatePrefix::Specialization)
                            && self.scopes.has_category_conflicts()
                            && node.prefix_scope.is_some_and(|scope| {
                                self.last_identifier(name).is_some_and(|identifier| {
                                    self.scopes.member_specialized(scope, identifier)
                                })
                            }))));
            let mut dependent = node.prefix_state == PrefixState::Unqualified
                && node
                    .binding
                    .is_some_and(|binding| binding.guesses.dependent);
            if !dependent
                && node.mentions_parameters
                && known
                && (node.prefix_template == TemplatePrefix::Dependent)
            {
                dependent = node
                    .prefix_scope
                    .is_none_or(|scope| !self.scopes.current_instantiation(scope));
            } else if !dependent
                && node.mentions_parameters
                && self.name_has_arguments(name)
                && node
                    .binding
                    .is_some_and(|binding| binding.category == Category::TypeTemplate)
            {
                dependent = node
                    .binding
                    .and_then(|binding| binding.scope)
                    .is_none_or(|scope| !self.scopes.current_instantiation(scope));
            }
            binding.guesses = Guesses {
                approximate,
                dependent: dependent && !alias_template,
            };
            if !approximate {
                return;
            }
        } else if binding.scope.is_some() {
            return;
        }
        self.scopes.note_uncertain();
    }
}
