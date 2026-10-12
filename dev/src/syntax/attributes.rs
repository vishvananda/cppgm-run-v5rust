// Translated from syntax/parser_specifiers.cpp and parser_state.cpp. See NOTICE.
//! Attribute collection owns typed operands; PA5 views intentionally omit attributes.
use super::ast::SpecifierKind;
use super::{
    ParseError,
    ast::{Attribute, AttributeForm, AttributeKind as A, AttributeSite, ExprId, Specifier},
    cursor::TokenKind,
    parser::Parser,
    storage::List,
};
use crate::lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword as K};

impl Parser<'_> {
    pub(super) fn at_attribute(&mut self, ahead: usize) -> Result<bool, ParseError> {
        Ok(match self.cursor.peek(ahead)?.kind {
            TokenKind::Identifier(name) => {
                matches!(self.cursor.name(name), b"__attribute__" | b"__attribute")
            }
            TokenKind::Simple(op) => {
                op.token.as_keyword() == Some(K::Alignas)
                    || (op.token.as_punctuator() == Some(P::LeftBracket)
                        && self.at_ahead(P::LeftBracket, ahead + 1)?)
            }
            _ => false,
        })
    }
    pub(super) fn skip_attributes(&mut self) -> Result<(), ParseError> {
        while self.at_attribute(0)? {
            if self.accept(P::LeftBracket)? {
                self.skip_balanced()?;
                self.expect(P::RightBracket)?;
            } else {
                self.cursor.advance()?;
                if !self.at(P::LeftParen)? {
                    return self.fail();
                }
                self.skip_balanced()?;
            }
        }
        Ok(())
    }
    pub(super) fn collect_attributes(
        &mut self,
        on_type: bool,
    ) -> Result<List<Attribute>, ParseError> {
        let start = self.tree.attributes.begin();
        while self.at_attribute(0)? {
            let location = self.location()?;
            if self.accept(P::LeftBracket)? {
                self.expect(P::LeftBracket)?;
                self.standard_attributes(on_type)?;
                self.expect(P::RightBracket)?;
                self.expect(P::RightBracket)?;
            } else if self.accept_keyword(K::Alignas)? {
                self.expect(P::LeftParen)?;
                let mut attribute = Attribute {
                    kind: A::Alignas,
                    location,
                    form: AttributeForm::Alignment,
                    site: if on_type {
                        AttributeSite::Type
                    } else {
                        AttributeSite::Entity
                    },
                    continued: false,
                    expression: None,
                    ty: None,
                };
                self.without_angle(|parser| parser.alignment_operand(&mut attribute))?;
                self.expect(P::RightParen)?;
                self.tree.attributes.push(attribute);
            } else {
                self.cursor.advance()?;
                self.expect(P::LeftParen)?;
                self.expect(P::LeftParen)?;
                self.gnu_attributes(on_type)?;
                self.expect(P::RightParen)?;
                self.expect(P::RightParen)?;
            }
        }
        self.tree.attributes.finish(start)
    }
    fn attribute_name(&mut self) -> Result<Option<crate::lex::identifiers::NameId>, ParseError> {
        if let TokenKind::Identifier(name) = self.token()?.kind {
            self.cursor.advance()?;
            Ok(Some(name))
        } else {
            Ok(None)
        }
    }
    fn standard_attributes(&mut self, on_type: bool) -> Result<(), ParseError> {
        while !self.at(P::RightBracket)? {
            let location = self.location()?;
            let space = if matches!(self.token()?.kind, TokenKind::Identifier(_))
                && self.at_ahead(P::Scope, 1)?
            {
                let name = self.identifier()?;
                self.cursor.advance()?;
                Some(name)
            } else {
                None
            };
            let gnu =
                space.is_some_and(|space| matches!(self.cursor.name(space), b"gnu" | b"__gnu__"));
            let name = if gnu || space.is_none() {
                self.attribute_name()?
            } else {
                None
            };
            let const_keyword = name.is_none() && self.keyword()? == Some(K::Const);
            let spelling = name.map(|name| self.cursor.name(name));
            let kind = if gnu {
                if const_keyword {
                    Some(A::Const)
                } else {
                    spelling.and_then(attribute_kind)
                }
            } else if space.is_none() {
                match spelling {
                    Some(b"no_unique_address" | b"__no_unique_address__") => {
                        Some(A::NoUniqueAddress)
                    }
                    Some(b"noreturn" | b"__noreturn__") => Some(A::NoReturn),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(kind) = kind {
                if name.is_none() && kind == A::Const {
                    self.cursor.advance()?;
                }
                let attribute = Attribute {
                    kind,
                    location,
                    form: AttributeForm::Standard,
                    site: if on_type {
                        AttributeSite::Type
                    } else {
                        AttributeSite::Entity
                    },
                    continued: false,
                    expression: None,
                    ty: None,
                };
                if !gnu && kind == A::NoReturn && self.at(P::LeftParen)? {
                    return self.fail();
                }
                self.attribute_operand(attribute)?;
            } else {
                self.skip_standard_attribute()?;
            }
            if !self.accept(P::Comma)? {
                break;
            }
        }
        Ok(())
    }
    fn skip_standard_attribute(&mut self) -> Result<(), ParseError> {
        while !self.at(P::Comma)? && !self.at(P::RightBracket)? {
            match self.punctuator()? {
                Some(P::LeftParen | P::LeftBracket | P::LeftBrace) => self.skip_balanced()?,
                Some(P::RightParen | P::RightBrace) => return self.fail(),
                _ if matches!(self.token()?.kind, TokenKind::End) => return self.fail(),
                _ => self.cursor.advance()?,
            }
        }
        Ok(())
    }
    fn gnu_attributes(&mut self, on_type: bool) -> Result<(), ParseError> {
        while !self.at(P::RightParen)? {
            let location = self.location()?;
            let kind = match self.token()?.kind {
                TokenKind::Identifier(name) => attribute_kind(self.cursor.name(name)),
                _ if self.keyword()? == Some(K::Const) => Some(A::Const),
                _ => None,
            };
            self.cursor.advance()?;
            if let Some(kind) = kind {
                self.attribute_operand(Attribute {
                    kind,
                    location,
                    form: AttributeForm::Gnu,
                    site: if on_type {
                        AttributeSite::Type
                    } else {
                        AttributeSite::Entity
                    },
                    continued: false,
                    expression: None,
                    ty: None,
                })?;
            } else if self.at(P::LeftParen)? {
                self.skip_balanced()?;
            }
            if !self.accept(P::Comma)? {
                break;
            }
        }
        Ok(())
    }
    fn attribute_operand(&mut self, mut attribute: Attribute) -> Result<(), ParseError> {
        match attribute.kind {
            A::AbiTag if self.accept(P::LeftParen)? => {
                loop {
                    attribute.location = self.location()?;
                    attribute.expression = Some(self.attribute_string()?);
                    self.tree.attributes.push(attribute);
                    attribute.continued = true;
                    if !self.accept(P::Comma)? {
                        break;
                    }
                }
                return self.expect(P::RightParen);
            }
            A::Section => {
                self.expect(P::LeftParen)?;
                attribute.expression = Some(self.attribute_string()?);
                self.expect(P::RightParen)?;
            }
            A::VectorSize | A::ExtVectorType => {
                self.expect(P::LeftParen)?;
                attribute.expression = Some(self.without_angle(Self::assignment_expression)?);
                self.expect(P::RightParen)?;
            }
            A::Alignas if self.accept(P::LeftParen)? => {
                attribute.expression = Some(self.without_angle(Self::assignment_expression)?);
                self.expect(P::RightParen)?;
            }
            _ => (),
        }
        self.tree.attributes.push(attribute);
        Ok(())
    }
    pub(super) fn attribute_string(&mut self) -> Result<ExprId, ParseError> {
        if !matches!(self.token()?.kind, TokenKind::Literal(literal) if matches!(literal.kind, PostTokenKind::String { .. }))
        {
            return self.fail();
        }
        self.primary()
    }
    fn alignment_operand(&mut self, attribute: &mut Attribute) -> Result<(), ParseError> {
        let location = self.location()?;
        if matches!(self.token()?.kind, TokenKind::Identifier(_))
            || self.at(P::Scope)?
            || matches!(self.keyword()?, Some(K::Decltype | K::Typename))
        {
            let (name, binding) = self.name()?;
            if self.is_type_name(name, binding, false, false)
                && let Some(ty) = self.attempt(|parser| {
                    let ty = parser.type_from_name(name, location)?;
                    parser.expect_alignment_end()?;
                    Ok(ty)
                })?
            {
                attribute.ty = Some(ty);
                return Ok(());
            }
            let primary = self.primary_from_name(name, binding, location)?;
            let primary = self.postfix(primary)?;
            let binary = self.binary_rest(primary, 3)?;
            let assignment = self.assignment_rest(binary)?;
            attribute.expression = Some(self.expression_rest(assignment)?);
        } else if let Some(ty) = self.attempt(|parser| {
            let ty = parser.type_id(false)?;
            parser.expect_alignment_end()?;
            Ok(ty)
        })? {
            attribute.ty = Some(ty);
        } else {
            attribute.expression = Some(self.assignment_expression()?);
        }
        Ok(())
    }
    fn expect_alignment_end(&mut self) -> Result<(), ParseError> {
        if self.at(P::RightParen)? {
            Ok(())
        } else {
            self.fail()
        }
    }
    pub(super) fn pending_class_attributes(&self) -> bool {
        !self.pending_class_tags.is_empty()
    }
    pub(super) fn without_declaration_attributes<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let pending = self.pending_attributes.take();
        let leading = self.leading_attributes;
        let tags = self.pending_class_tags;
        self.leading_attributes = List::default();
        self.pending_class_tags = List::default();
        let result = parse(self);
        self.pending_attributes = pending;
        self.leading_attributes = leading;
        self.pending_class_tags = tags;
        result
    }
    pub(super) fn append_pending_attributes(&mut self, attributes: List<Attribute>) {
        if self.pending_attributes.is_some() {
            self.tree.attributes.extend(attributes);
        }
    }
    pub(super) fn vector_specifiers(
        &mut self,
        attributes: List<Attribute>,
        keep: bool,
    ) -> Result<(), ParseError> {
        for index in 0..self.tree.attributes.get(attributes).len() {
            let attribute = self.tree.attributes.get(attributes)[index];
            if matches!(attribute.kind, A::VectorSize | A::ExtVectorType) {
                self.tree.specs.push(Specifier {
                    location: attribute.location,
                    kind: SpecifierKind::Vector {
                        extended: attribute.kind == A::ExtVectorType,
                        operand: attribute
                            .expression
                            .ok_or(ParseError::Invariant("vector attribute lacks operand"))?,
                    },
                });
            } else if keep && self.pending_attributes.is_some() {
                self.tree.attributes.push(attribute);
            }
        }
        Ok(())
    }
}
fn attribute_kind(spelling: &[u8]) -> Option<A> {
    Some(match spelling {
        b"aligned" | b"__aligned__" => A::Alignas,
        b"noinline" | b"__noinline__" => A::NoInline,
        b"packed" | b"__packed__" => A::Packed,
        b"weak" | b"__weak__" => A::Weak,
        b"section" | b"__section__" => A::Section,
        b"abi_tag" | b"__abi_tag__" => A::AbiTag,
        b"pure" | b"__pure__" => A::Pure,
        b"const" | b"__const__" => A::Const,
        b"using_if_exists" | b"__using_if_exists__" => A::UsingIfExists,
        b"vector_size" | b"__vector_size__" => A::VectorSize,
        b"ext_vector_type" | b"__ext_vector_type__" => A::ExtVectorType,
        b"exclude_from_explicit_instantiation" | b"__exclude_from_explicit_instantiation__" => {
            A::ExcludeFromExplicitInstantiation
        }
        b"always_inline" | b"__always_inline__" => A::AlwaysInline,
        b"gnu_inline" | b"__gnu_inline__" => A::GnuInline,
        b"used" | b"__used__" => A::Used,
        b"noreturn" | b"__noreturn__" => A::NoReturn,
        _ => return None,
    })
}
