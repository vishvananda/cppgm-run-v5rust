// Translated from syntax/parser_declarations.cpp and parser_names.cpp. See NOTICE.
//! Template heads and arguments retain typed syntax, independently of instantiation.
use super::ast::SpecifierKind;
use super::{
    ParseError,
    ast::{
        DeclId, DeclKind, Declaration, ExprKind, Specifier, TemplateArgument, TemplateArgumentKind,
        TemplateParameter, TemplateParameterClause, TemplateParameterKind,
    },
    cursor::TokenKind,
    declarators::Mode,
    parser::Parser,
    scopes::{Binding, Category, ScopeKind},
    storage::List,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
use crate::support::source::SourceOffset;

impl Parser<'_> {
    pub(super) fn template_declaration(
        &mut self,
        location: SourceOffset,
    ) -> Result<DeclId, ParseError> {
        self.cursor.advance()?;
        let enclosing = self.scopes.current;
        self.scopes.push_kind(ScopeKind::Template)?;
        let parameters = self.template_parameter_clause()?;
        let head = self.template_head.replace(parameters);
        self.pending_template = !parameters.parameters.is_empty();
        let result = self.nested(Self::declaration);
        self.template_head = head;
        self.pending_template = false;
        self.scopes.current = enclosing;
        let declaration = result?;
        self.tree.declarations.add(Declaration {
            attributes: super::storage::List::default(),
            location,
            kind: DeclKind::Template {
                parameters,
                declaration,
            },
        })
    }
    pub(super) fn explicit_instantiation(
        &mut self,
        location: SourceOffset,
    ) -> Result<DeclId, ParseError> {
        let is_extern = self.accept_keyword(K::Extern)?;
        if !self.accept_keyword(K::Template)? {
            return self.fail();
        }
        let declaration = self.nested(Self::declaration)?;
        self.tree.declarations.add(Declaration {
            attributes: super::storage::List::default(),
            location,
            kind: DeclKind::ExplicitInstantiation {
                is_extern,
                declaration,
            },
        })
    }
    fn template_parameter_clause(&mut self) -> Result<TemplateParameterClause, ParseError> {
        let location = self.location()?;
        self.expect(P::Less)?;
        let saved = self.angle;
        self.angle = true;
        let start = self.tree.template_parameters.begin();
        let result = (|| {
            if !self.at(P::Greater)? {
                loop {
                    let parameter = self.nested(Self::template_parameter)?;
                    self.tree.template_parameters.push(parameter);
                    if !self.accept(P::Comma)? {
                        break;
                    }
                }
            }
            self.expect(P::Greater)?;
            let parameters = self.tree.template_parameters.finish(start)?;
            Ok(TemplateParameterClause {
                location,
                parameters,
            })
        })();
        self.angle = saved;
        result
    }
    fn template_parameter(&mut self) -> Result<TemplateParameter, ParseError> {
        let location = self.location()?;
        if self.accept_keyword(K::Template)? {
            let enclosing = self.scopes.current;
            self.scopes.push_kind(ScopeKind::Template)?;
            let result = self.template_parameter_clause();
            self.scopes.current = enclosing;
            return self.type_parameter(location, Some(result?));
        }
        let type_key = matches!(self.keyword()?, Some(K::Class | K::Typename));
        let type_parameter = type_key
            && (self.at_ahead(P::Ellipsis, 1)?
                || self.parameter_end_ahead(1)?
                || (matches!(self.cursor.peek(1)?.kind, TokenKind::Identifier(_))
                    && self.parameter_end_ahead(2)?));
        if type_parameter {
            return self.type_parameter(location, None);
        }
        let specifiers = self.specifier_sequence(false, false)?;
        let pack = self.accept(P::Ellipsis)?;
        let declarator = self.declarator(Mode::Parameter)?;
        let declarator = (!self.declarator_empty(declarator)).then_some(declarator);
        let name = declarator
            .and_then(|id| self.declarator_name(id))
            .and_then(|name| self.simple_name(name));
        let default = if self.accept(P::Assign)? {
            let value = self.assignment_expression()?;
            if name.is_none()
                && matches!(
                    self.tree.specs.get(specifiers),
                    [Specifier {
                        kind: SpecifierKind::Keyword(K::Int),
                        ..
                    }]
                )
            {
                let mut node = *self.tree.expressions.get(value);
                if let ExprKind::Literal(mut literal) = node.kind {
                    literal.token_form = true;
                    node.kind = ExprKind::Literal(literal);
                    self.tree.expressions.replace(value, node);
                }
            }
            Some(value)
        } else {
            None
        };
        if let Some(name) = name {
            self.scopes.declare(
                name,
                Binding {
                    pack,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    parameter: true,
                    category: Category::Value,
                    scope: None,
                },
            );
        }
        Ok(TemplateParameter {
            location,
            pack,
            kind: TemplateParameterKind::NonType {
                specifiers,
                declarator,
                default,
            },
        })
    }
    fn parameter_end_ahead(&mut self, ahead: usize) -> Result<bool, ParseError> {
        Ok(
            matches!(self.cursor.peek(ahead)?.kind, TokenKind::Simple(op) if matches!(op.token.as_punctuator(), Some(P::Comma | P::Greater | P::Assign))),
        )
    }
    fn type_parameter(
        &mut self,
        location: SourceOffset,
        parameters: Option<TemplateParameterClause>,
    ) -> Result<TemplateParameter, ParseError> {
        let Some(key @ (K::Class | K::Typename)) = self.keyword()? else {
            return self.fail();
        };
        self.cursor.advance()?;
        let pack = self.accept(P::Ellipsis)?;
        let name = if matches!(self.token()?.kind, TokenKind::Identifier(_)) {
            Some(self.identifier()?)
        } else {
            None
        };
        let default = if self.accept(P::Assign)? {
            Some(self.type_id(true)?)
        } else {
            None
        };
        if let Some(name) = name {
            self.scopes.declare(
                name,
                Binding {
                    pack,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    parameter: true,
                    category: if parameters.is_some() {
                        Category::TypeTemplate
                    } else {
                        Category::Type
                    },
                    scope: None,
                },
            );
        }
        Ok(TemplateParameter {
            location,
            pack,
            kind: TemplateParameterKind::Type {
                key,
                name,
                parameters,
                default,
            },
        })
    }
    pub(super) fn template_arguments(&mut self) -> Result<List<TemplateArgument>, ParseError> {
        self.nested(Self::template_arguments_inner)
    }
    fn template_arguments_inner(&mut self) -> Result<List<TemplateArgument>, ParseError> {
        self.expect(P::Less)?;
        let saved = self.angle;
        self.angle = true;
        let result = self.template_argument_list();
        self.angle = saved;
        result
    }
    fn template_argument_list(&mut self) -> Result<List<TemplateArgument>, ParseError> {
        let start = self.tree.template_arguments.begin();
        if !self.at(P::Greater)? {
            loop {
                let argument = self.nested(Self::template_argument)?;
                self.tree.template_arguments.push(argument);
                if !self.accept(P::Comma)? {
                    break;
                }
            }
        }
        self.expect(P::Greater)?;
        self.tree.template_arguments.finish(start)
    }
    fn template_argument(&mut self) -> Result<TemplateArgument, ParseError> {
        let pack_mentions = self.pack_mentions;
        let location = self.location()?;
        let named = matches!(self.token()?.kind, TokenKind::Identifier(_))
            || self.at(P::Scope)?
            || matches!(
                self.keyword()?,
                Some(K::Decltype | K::Operator | K::Typename)
            );
        let kind = if named {
            // Parse the name and its nested arguments once. Both readings borrow it.
            let (name, binding) = self.name()?;
            let ty = if self.is_type_name(name, binding, false, false) {
                self.attempt(|parser| {
                    let ty = parser.type_from_name(name, location)?;
                    parser.argument_end()?;
                    Ok(ty)
                })?
            } else {
                None
            };
            if let Some(ty) = ty {
                TemplateArgumentKind::Type(ty)
            } else {
                let primary = self.primary_from_name(name, binding, location)?;
                TemplateArgumentKind::Expression(self.continue_assignment_expression(primary)?)
            }
        } else {
            let ty = self.attempt(|parser| {
                let ty = parser.type_id(false)?;
                parser.argument_end()?;
                Ok(ty)
            })?;
            if let Some(ty) = ty {
                TemplateArgumentKind::Type(ty)
            } else {
                TemplateArgumentKind::Expression(self.assignment_expression()?)
            }
        };
        let pack = self.accept(P::Ellipsis)?;
        let type_pack = matches!(kind, TemplateArgumentKind::Type(ty) if self.tree.types.get(ty).declarator.is_some_and(|declarator| self.tree.declarators.get(declarator).pack));
        if pack || type_pack {
            self.pack_mentions = pack_mentions;
        }
        Ok(TemplateArgument { kind, pack })
    }
    fn argument_end(&mut self) -> Result<(), ParseError> {
        if matches!(
            self.punctuator()?,
            Some(P::Comma | P::Greater | P::Ellipsis)
        ) {
            Ok(())
        } else {
            self.fail()
        }
    }
}
