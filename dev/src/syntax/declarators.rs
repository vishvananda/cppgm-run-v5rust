// Translated from syntax/parser_declarators.cpp. See NOTICE.
use super::{
    ParseError,
    ast::{
        Declarator, DeclaratorId, Initializer, Parameter, ParameterClause, Pointer, Qualifier,
        Suffix,
    },
    cursor::TokenKind,
    parser::Parser,
    scopes::Category,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Named,
    Parameter,
    Abstract,
}
impl Parser<'_> {
    pub(super) fn declarator(&mut self, mode: Mode) -> Result<DeclaratorId, ParseError> {
        self.nested(|parser| parser.declarator_inner(mode))
    }
    fn declarator_inner(&mut self, mode: Mode) -> Result<DeclaratorId, ParseError> {
        let location = self.location()?;
        let start = self.tree.pointers.begin();
        while matches!(
            self.punctuator()?,
            Some(P::Multiply | P::BitAnd | P::LogicalAnd)
        ) {
            let op = self.operator()?;
            let cv_start = self.tree.cvs.begin();
            if op.token.as_punctuator() == Some(P::Multiply) {
                while matches!(self.keyword()?, Some(K::Const | K::Volatile)) {
                    if let Some(word) = self.keyword()? {
                        self.tree.cvs.push(word);
                        self.cursor.advance()?;
                    }
                }
            }
            self.attributes_boundary()?;
            let cv = self.tree.cvs.finish(cv_start)?;
            self.tree.pointers.push(Pointer { op, cv });
        }
        let pointers = self.tree.pointers.finish(start)?;
        let pack = self.at(P::Ellipsis)?
            && (mode != Mode::Parameter || !self.at_ahead(P::RightParen, 1)?);
        if pack {
            self.cursor.advance()?;
        }
        let mut name = None;
        let mut nested = None;
        if self.at(P::LeftParen)? && self.nested_declarator_ahead(mode)? {
            self.cursor.advance()?;
            nested = Some(self.declarator(mode)?);
            self.expect(P::RightParen)?;
        } else if mode != Mode::Abstract
            && (matches!(self.token()?.kind, TokenKind::Identifier(_))
                || self.at(P::Scope)?
                || self.at(P::Complement)?
                || self.keyword()? == Some(K::Operator))
        {
            name = Some(self.name()?.0);
            self.attributes_boundary()?;
        } else if mode == Mode::Named
            || (mode == Mode::Parameter
                && self.at(P::LeftParen)?
                && (self.at_ahead(P::RightParen, 1)? || self.at_ahead(P::Ellipsis, 1)?))
        {
            return self.fail();
        }
        let suffix_start = self.tree.suffixes.begin();
        loop {
            if self.accept(P::LeftBracket)? {
                let bound = if self.at(P::RightBracket)? {
                    None
                } else {
                    Some(self.expression()?)
                };
                self.expect(P::RightBracket)?;
                self.attributes_boundary()?;
                self.tree.suffixes.push(Suffix::Array(bound));
                continue;
            }
            if !self.at(P::LeftParen)? {
                break;
            }
            let definite = mode != Mode::Named;
            let Some(parameters) = self.attempt(|parser| parser.parameter_clause(definite))? else {
                break;
            };
            let qualifiers = self.function_qualifiers()?;
            self.tree.suffixes.push(Suffix::Function {
                parameters,
                qualifiers,
            });
        }
        let suffixes = self.tree.suffixes.finish(suffix_start)?;
        self.tree.declarators.add(Declarator {
            location,
            abstract_form: mode == Mode::Abstract,
            pointers,
            pack,
            name,
            nested,
            suffixes,
        })
    }
    fn nested_declarator_ahead(&mut self, mode: Mode) -> Result<bool, ParseError> {
        if mode == Mode::Named {
            return Ok(true);
        }
        if matches!(self.cursor.peek(1)?.kind, TokenKind::Simple(op) if matches!(op.token.as_punctuator(), Some(P::Multiply | P::BitAnd | P::LogicalAnd | P::LeftBracket | P::LeftParen)))
        {
            return Ok(true);
        }
        if mode == Mode::Abstract {
            return Ok(false);
        }
        if let TokenKind::Identifier(name) = self.cursor.peek(1)?.kind {
            if self.at_ahead(P::Scope, 2)? {
                return Ok(false);
            }
            let binding = self.scopes.lookup(name);
            return Ok(binding.map_or_else(
                || {
                    !self
                        .cursor
                        .name(name)
                        .iter()
                        .any(|byte| matches!(byte, b'C' | b'Y' | b'E' | b'T'))
                },
                |binding| binding.category != Category::Type,
            ));
        }
        Ok(false)
    }
    pub(super) fn declarator_empty(&self, id: DeclaratorId) -> bool {
        let node = self.tree.declarators.get(id);
        node.pointers.is_empty()
            && !node.pack
            && node.name.is_none()
            && node.nested.is_none()
            && node.suffixes.is_empty()
    }
    pub(super) fn is_function(&self, id: DeclaratorId) -> bool {
        self.declarator_name(id).is_some() && self.first_derivation(id) == Some(true)
    }
    fn first_derivation(&self, id: DeclaratorId) -> Option<bool> {
        let node = self.tree.declarators.get(id);
        if let Some(nested) = node.nested
            && let Some(derived) = self.first_derivation(nested)
        {
            return Some(derived);
        }
        if let Some(suffix) = self.tree.suffixes.get(node.suffixes).first() {
            return Some(matches!(suffix, Suffix::Function { .. }));
        }
        (!node.pointers.is_empty()).then_some(false)
    }
    fn parameter_clause(&mut self, mut definite: bool) -> Result<ParameterClause, ParseError> {
        self.expect(P::LeftParen)?;
        let enclosing = self.scopes.current;
        let scope = self.scopes.push()?;
        let start = self.tree.parameters.begin();
        let mut ellipsis = false;
        let mut ellipsis_without_comma = false;
        while !self.at(P::RightParen)? {
            if self.accept(P::Ellipsis)? {
                ellipsis = true;
                break;
            }
            let specifiers = self.specifier_sequence(definite, false)?;
            definite = true;
            let declarator = if let Some(declarator) =
                self.attempt(|parser| parser.declarator(Mode::Parameter))?
            {
                declarator
            } else {
                self.declarator(Mode::Abstract)?
            };
            let declarator = (!self.declarator_empty(declarator)).then_some(declarator);
            self.attributes_boundary()?;
            let default = if self.accept(P::Assign)? {
                Some(Initializer::Assign(self.initializer_clause()?))
            } else {
                None
            };
            if let Some(declarator) = declarator {
                self.declare_declarator(declarator, false, None);
            }
            self.tree.parameters.push(Parameter {
                specifiers,
                declarator,
                default,
            });
            if self.accept(P::Comma)? {
                continue;
            }
            if self.accept(P::Ellipsis)? {
                ellipsis = true;
                ellipsis_without_comma = true;
            }
            break;
        }
        self.expect(P::RightParen)?;
        let parameters = self.tree.parameters.finish(start)?;
        self.scopes.current = enclosing;
        self.function_scope = Some(scope);
        Ok(ParameterClause {
            parameters,
            ellipsis,
            ellipsis_without_comma,
        })
    }
    fn function_qualifiers(&mut self) -> Result<super::storage::List<Qualifier>, ParseError> {
        let start = self.tree.qualifiers.begin();
        loop {
            self.attributes_boundary()?;
            let qualifier = match self.keyword()? {
                Some(word @ (K::Const | K::Volatile)) => {
                    self.cursor.advance()?;
                    Qualifier::Cv(word)
                }
                Some(K::Noexcept) => {
                    self.cursor.advance()?;
                    let operand = if self.accept(P::LeftParen)? {
                        let operand = self.expression()?;
                        self.expect(P::RightParen)?;
                        Some(operand)
                    } else {
                        None
                    };
                    Qualifier::Noexcept(operand)
                }
                Some(K::Throw) => {
                    self.cursor.advance()?;
                    Qualifier::Throw(self.throw_types()?)
                }
                _ if matches!(self.punctuator()?, Some(P::BitAnd | P::LogicalAnd)) => {
                    Qualifier::Ref(self.operator()?)
                }
                _ if self.accept(P::Arrow)? => Qualifier::Trailing(self.type_id(true)?),
                _ if matches!(self.token()?.kind, TokenKind::Identifier(name) if matches!(self.cursor.name(name), b"final" | b"override")) => {
                    Qualifier::Virt(self.identifier()?)
                }
                _ => break,
            };
            self.tree.qualifiers.push(qualifier);
        }
        self.tree.qualifiers.finish(start)
    }
    fn throw_types(&mut self) -> Result<super::storage::List<super::ast::TypeId>, ParseError> {
        self.expect(P::LeftParen)?;
        let start = self.tree.type_lists.begin();
        if !self.at(P::RightParen)? {
            loop {
                let ty = self.type_id(true)?;
                self.tree.type_lists.push(ty);
                if !self.accept(P::Comma)? {
                    break;
                }
            }
        }
        self.expect(P::RightParen)?;
        self.tree.type_lists.finish(start)
    }
}
