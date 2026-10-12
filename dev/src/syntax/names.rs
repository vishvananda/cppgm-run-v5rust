// Translated from syntax/parser_names.cpp. See NOTICE.
//! Structured name components; operator and conversion spellings are views.
use super::{
    ParseError,
    ast::{ExprId, Name, NameComponent, NameKind, QualifiedName},
    cursor::TokenKind,
    parser::Parser,
    scopes::{Binding, Category, Scopes},
};
use crate::lex::{identifiers::NameId, pp_lexer::Punctuator as P, vocabulary::Keyword as K};

impl Parser<'_> {
    pub(super) fn name(&mut self) -> Result<(Name, Option<Binding>), ParseError> {
        self.name_in_context(false)
    }
    pub(super) fn type_name(&mut self) -> Result<(Name, Option<Binding>), ParseError> {
        self.name_in_context(true)
    }
    fn name_in_context(
        &mut self,
        type_context: bool,
    ) -> Result<(Name, Option<Binding>), ParseError> {
        let typename_keyword = self.accept_keyword(K::Typename)?;
        let global = self.accept(P::Scope)?;
        let start = self.tree.name_lists.begin();
        let mut prefix = global.then_some(Scopes::global());
        let mut qualified = global;
        let binding = loop {
            let location = self.location()?;
            if !qualified && self.accept_keyword(K::Decltype)? {
                let operand = self.decltype_operand()?;
                self.tree.name_lists.push(NameComponent {
                    location,
                    kind: NameKind::Decltype(operand),
                });
                if self.accept(P::Scope)? {
                    qualified = true;
                    prefix = None;
                    continue;
                }
                break Some(Binding {
                    category: Category::Type,
                    scope: None,
                });
            }
            if !type_context && self.keyword()? == Some(K::Operator) {
                let enclosing = self.scopes.current;
                if let Some(scope) = prefix {
                    self.scopes.current = scope;
                }
                let result = self.operator_name();
                self.scopes.current = enclosing;
                let kind = result?;
                if self.at(P::Less)? && !matches!(kind, NameKind::Conversion(_)) {
                    return Err(ParseError::NotImplemented("operator template-id arguments"));
                }
                self.tree.name_lists.push(NameComponent { location, kind });
                break Some(Binding {
                    category: Category::Value,
                    scope: None,
                });
            }
            if !type_context && self.accept(P::Complement)? {
                let kind = if self.accept_keyword(K::Decltype)? {
                    NameKind::DestructorDecltype(self.decltype_operand()?)
                } else {
                    NameKind::Destructor(self.identifier()?)
                };
                if self.at(P::Less)? {
                    return Err(ParseError::NotImplemented(
                        "destructor template-id arguments",
                    ));
                }
                self.tree.name_lists.push(NameComponent { location, kind });
                break None;
            }
            let identifier = self.identifier()?;
            self.tree.name_lists.push(NameComponent {
                location,
                kind: NameKind::Identifier(identifier),
            });
            let prefix_only = self.at(P::Scope)? && !self.at_ahead(P::Multiply, 1)?;
            let binding = if let Some(scope) = prefix {
                self.scopes.lookup_in_name(scope, identifier, prefix_only)
            } else if qualified {
                None
            } else {
                self.scopes.lookup_name(identifier, prefix_only)
            };
            if self.at(P::Less)?
                && binding.is_none_or(|binding| binding.category != Category::Value)
                && self.cursor.name(identifier).contains(&b'T')
            {
                return Err(ParseError::NotImplemented(
                    "template-id name disambiguation",
                ));
            }
            if self.at(P::Scope)? && self.at_ahead(P::Multiply, 1)? {
                break binding;
            }
            if !self.accept(P::Scope)? {
                break binding;
            }
            qualified = true;
            prefix = binding.and_then(|binding| binding.scope);
        };
        let components = self.tree.name_lists.finish(start)?;
        let name = self.tree.names.add(QualifiedName {
            global,
            typename_keyword,
            components,
            prefix_scope: prefix,
            binding,
        })?;
        Ok((name, binding))
    }
    fn operator_name(&mut self) -> Result<NameKind, ParseError> {
        self.cursor.advance()?;
        if matches!(self.keyword()?, Some(K::New | K::Delete)) {
            let op = self.operator()?;
            let array = self.at(P::LeftBracket)? && self.at_ahead(P::RightBracket, 1)?;
            if array {
                self.cursor.advance()?;
                self.cursor.advance()?;
            }
            return Ok(NameKind::Operator { op, array });
        }
        if (self.at(P::LeftParen)? && self.at_ahead(P::RightParen, 1)?)
            || (self.at(P::LeftBracket)? && self.at_ahead(P::RightBracket, 1)?)
        {
            let op = self.operator()?;
            self.cursor.advance()?;
            return Ok(NameKind::Operator { op, array: false });
        }
        if self.at(P::Greater)?
            && self.token()?.shift_half == 1
            && self.cursor.peek(1)?.shift_half == 2
        {
            self.cursor.advance()?;
            self.cursor.advance()?;
            return Ok(NameKind::Operator {
                op: Self::make_operator(P::ShiftRight)?,
                array: false,
            });
        }
        if let TokenKind::Literal(literal) = self.token()?.kind
            && self.cursor.spellings.get(literal.spelling) == b"\"\""
        {
            let next = self.cursor.peek(1)?;
            let suffix = match next.kind {
                TokenKind::Identifier(name) => Some(name),
                TokenKind::Simple(_) if next.location.0 == self.location()?.0.wrapping_add(2) => {
                    next.suffix_name
                }
                _ => None,
            };
            if let Some(suffix) = suffix {
                self.cursor.advance()?;
                self.cursor.advance()?;
                return Ok(NameKind::LiteralOperator(suffix));
            }
        }
        if matches!(
            self.punctuator()?,
            Some(
                P::Add
                    | P::Subtract
                    | P::Multiply
                    | P::Divide
                    | P::Remainder
                    | P::Xor
                    | P::BitAnd
                    | P::BitOr
                    | P::Complement
                    | P::LogicalNot
                    | P::Assign
                    | P::Less
                    | P::Greater
                    | P::AddAssign
                    | P::SubtractAssign
                    | P::MultiplyAssign
                    | P::DivideAssign
                    | P::RemainderAssign
                    | P::XorAssign
                    | P::AndAssign
                    | P::OrAssign
                    | P::ShiftLeft
                    | P::ShiftRightAssign
                    | P::ShiftLeftAssign
                    | P::Equal
                    | P::NotEqual
                    | P::LessEqual
                    | P::GreaterEqual
                    | P::LogicalAnd
                    | P::LogicalOr
                    | P::Increment
                    | P::Decrement
                    | P::Comma
                    | P::ArrowStar
                    | P::Arrow
            )
        ) {
            return Ok(NameKind::Operator {
                op: self.operator()?,
                array: false,
            });
        }
        Ok(NameKind::Conversion(self.type_id_in_mode(
            true,
            super::declarators::Mode::Conversion,
        )?))
    }
    pub(super) fn decltype_operand(&mut self) -> Result<ExprId, ParseError> {
        self.expect(P::LeftParen)?;
        let operand = self.expression()?;
        self.expect(P::RightParen)?;
        Ok(operand)
    }
    pub(super) fn is_type_name(
        &self,
        name: Name,
        binding: Option<Binding>,
        definite: bool,
        followed_by_identifier: bool,
    ) -> bool {
        let name = self.tree.names.get(name);
        if name.typename_keyword {
            return true;
        }
        if let Some(binding) = binding {
            return binding.category == Category::Type;
        }
        let Some(last) = self.tree.name_lists.get(name.components).last() else {
            return false;
        };
        let NameKind::Identifier(identifier) = last.kind else {
            return matches!(last.kind, NameKind::Decltype(_));
        };
        definite
            || followed_by_identifier
            || self
                .cursor
                .name(identifier)
                .iter()
                .any(|byte| matches!(byte, b'C' | b'Y' | b'E' | b'T'))
    }
    pub(super) fn last_identifier(&self, name: Name) -> Option<NameId> {
        match self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()?
            .kind
        {
            NameKind::Identifier(identifier) => Some(identifier),
            _ => None,
        }
    }
    pub(super) fn simple_name(&self, name: Name) -> Option<NameId> {
        let node = self.tree.names.get(name);
        if !node.global && self.tree.name_lists.get(node.components).len() == 1 {
            self.last_identifier(name)
        } else {
            None
        }
    }
}
