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
    New,
    Conversion,
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
        ) || self.member_pointer_ahead(0)?
        {
            let class_name = if self.member_pointer_ahead(0)? {
                let name = self.type_name()?.0;
                self.expect(P::Scope)?;
                self.expect(P::Multiply)?;
                Some(name)
            } else {
                None
            };
            let op = if class_name.is_some() {
                Self::make_operator(P::Scope)?
            } else {
                self.operator()?
            };
            let cv_start = self.tree.cvs.begin();
            if class_name.is_some() || op.token.as_punctuator() == Some(P::Multiply) {
                while matches!(self.keyword()?, Some(K::Const | K::Volatile)) {
                    if let Some(word) = self.keyword()? {
                        self.tree.cvs.push(word);
                        self.cursor.advance()?;
                    }
                }
            }
            self.attributes_boundary()?;
            let cv = self.tree.cvs.finish(cv_start)?;
            self.tree.pointers.push(Pointer { op, class_name, cv });
        }
        let pointers = self.tree.pointers.finish(start)?;
        let pack = !matches!(mode, Mode::New | Mode::Conversion)
            && self.at(P::Ellipsis)?
            && (mode != Mode::Parameter || !self.at_ahead(P::RightParen, 1)?);
        if pack {
            self.cursor.advance()?;
        }
        let mut name = None;
        let mut nested = None;
        if !matches!(mode, Mode::New | Mode::Conversion)
            && self.at(P::LeftParen)?
            && self.nested_declarator_ahead(mode)?
        {
            self.cursor.advance()?;
            nested = Some(self.declarator(mode)?);
            self.expect(P::RightParen)?;
        } else if matches!(mode, Mode::Named | Mode::Parameter)
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
            if mode == Mode::Conversion {
                break;
            }
            if self.accept(P::LeftBracket)? {
                let bound = if self.at(P::RightBracket)? {
                    None
                } else {
                    let enclosing = self.scopes.current;
                    if let Some(scope) =
                        name.and_then(|name| self.tree.names.get(name).prefix_scope)
                    {
                        self.scopes.member_context(scope)?;
                    }
                    let result = self.without_angle(Self::expression);
                    self.scopes.current = enclosing;
                    Some(result?)
                };
                self.expect(P::RightBracket)?;
                self.attributes_boundary()?;
                self.tree.suffixes.push(Suffix::Array(bound));
                continue;
            }
            if mode == Mode::New || !self.at(P::LeftParen)? {
                break;
            }
            let definite = mode != Mode::Named
                || name.is_some_and(|name| {
                    self.tree
                        .name_lists
                        .get(self.tree.names.get(name).components)
                        .last()
                        .is_some_and(|component| {
                            !matches!(component.kind, super::ast::NameKind::Identifier(_))
                        })
                });
            let member = name.and_then(|name| self.tree.names.get(name).prefix_scope);
            let result = self.attempt(|parser| parser.parameter_clause_member(definite, member));
            let Some(parameters) = result? else {
                break;
            };
            let saved = self.scopes.current;
            if let Some(scope) = self.function_scope {
                self.scopes.current = scope;
            }
            let function_scope = self.function_scope;
            let result = self.function_qualifiers();
            self.function_scope = function_scope;
            self.scopes.current = saved;
            let qualifiers = result?;
            self.tree.suffixes.push(Suffix::Function {
                parameters,
                qualifiers,
            });
        }
        let suffixes = self.tree.suffixes.finish(suffix_start)?;
        self.tree.declarators.add(Declarator {
            location,
            abstract_form: matches!(mode, Mode::Abstract | Mode::New | Mode::Conversion),
            pointers,
            pack,
            name,
            nested,
            suffixes,
        })
    }
    fn member_pointer_ahead(&mut self, mut ahead: usize) -> Result<bool, ParseError> {
        if self.at_ahead(P::Scope, ahead)? {
            ahead += 1;
        }
        loop {
            if !matches!(self.cursor.peek(ahead)?.kind, TokenKind::Identifier(_)) {
                return Ok(false);
            }
            ahead += 1;
            if !self.at_ahead(P::Scope, ahead)? {
                return Ok(false);
            }
            ahead += 1;
            if self.at_ahead(P::Multiply, ahead)? {
                return Ok(true);
            }
        }
    }
    fn nested_declarator_ahead(&mut self, mode: Mode) -> Result<bool, ParseError> {
        if mode == Mode::Named || self.member_pointer_ahead(1)? {
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
                |binding| !matches!(binding.category, Category::Type | Category::TypeTemplate),
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
    pub(super) fn parameter_clause_member(
        &mut self,
        definite: bool,
        member: Option<super::scopes::ScopeId>,
    ) -> Result<ParameterClause, ParseError> {
        self.without_angle(|parser| {
            let pending = parser.pending_template;
            parser.pending_template = false;
            let result = parser.parameter_clause_inner(definite, member);
            parser.pending_template = pending;
            result
        })
    }
    fn parameter_clause_inner(
        &mut self,
        mut definite: bool,
        member: Option<super::scopes::ScopeId>,
    ) -> Result<ParameterClause, ParseError> {
        self.expect(P::LeftParen)?;
        let enclosing = self.scopes.current;
        let scope = self.scopes.push()?;
        if let Some(member) = member {
            self.scopes.set_member(scope, member);
        }
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
    pub(super) fn function_qualifiers(
        &mut self,
    ) -> Result<super::storage::List<Qualifier>, ParseError> {
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
