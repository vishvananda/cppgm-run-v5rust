// Translated from syntax/parser_specifiers.cpp::parse_transform_specifier and
// syntax/parser_expressions.cpp::parse_builtin_trait. See NOTICE.
use super::{
    ParseError,
    ast::{ExprId, ExprKind, Expression},
    cursor::TokenKind,
    parser::Parser,
};
use crate::lex::{pp_lexer::Punctuator as P, type_transforms::TypeTransform};

impl Parser<'_> {
    pub(super) fn at_integer_pack(&mut self, ahead: usize) -> Result<bool, ParseError> {
        Ok(
            matches!(self.cursor.peek(ahead)?.kind, TokenKind::Identifier(name)
            if self.cursor.name(name) == b"__integer_pack")
                && self.at_ahead(P::LeftParen, ahead + 1)?,
        )
    }
    pub(super) fn at_offsetof(&mut self, ahead: usize) -> Result<bool, ParseError> {
        Ok(
            matches!(self.cursor.peek(ahead)?.kind, TokenKind::Identifier(name)
            if self.cursor.name(name) == b"__builtin_offsetof")
                && self.at_ahead(P::LeftParen, ahead + 1)?,
        )
    }

    pub(super) fn offsetof_expression(&mut self) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        self.cursor.advance()?;
        self.expect(P::LeftParen)?;
        let (ty, designator) = self.without_angle(|parser| {
            let ty = parser.type_id(false)?;
            parser.expect(P::Comma)?;
            let object_location = parser.location()?;
            let mut designator = parser.tree.expressions.add(Expression {
                location: object_location,
                kind: ExprKind::OffsetOfBase(ty),
            })?;
            let mut member = true;
            loop {
                let location = parser.location()?;
                let kind = if member {
                    if !matches!(parser.token()?.kind, TokenKind::Identifier(_)) {
                        return parser.fail();
                    }
                    let (member, _) = parser.member_name()?;
                    ExprKind::Member {
                        base: designator,
                        op: Self::make_operator(P::Dot)?,
                        member,
                    }
                } else {
                    parser.cursor.advance()?;
                    let index = parser.expression()?;
                    parser.expect(P::RightBracket)?;
                    ExprKind::Subscript {
                        base: designator,
                        index,
                    }
                };
                designator = parser.tree.expressions.add(Expression { location, kind })?;
                if parser.accept(P::Dot)? {
                    member = true;
                } else if parser.at(P::LeftBracket)? {
                    member = false;
                } else {
                    break;
                }
            }
            parser.expect(P::RightParen)?;
            Ok((ty, designator))
        })?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::OffsetOf { ty, designator },
        })
    }

    pub(super) fn at_type_predicate(
        &mut self,
        ahead: usize,
    ) -> Result<Option<crate::lex::predicates::TypePredicate>, ParseError> {
        Ok(
            if let TokenKind::Identifier(name) = self.cursor.peek(ahead)?.kind
                && let Some(predicate) =
                    crate::lex::predicates::TypePredicate::find(self.cursor.name(name))
                && self.at_ahead(P::LeftParen, ahead + 1)?
            {
                Some(predicate)
            } else {
                None
            },
        )
    }

    pub(super) fn predicate_operand(
        &mut self,
        predicate: crate::lex::predicates::TypePredicate,
    ) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        self.cursor.advance()?;
        self.expect(P::LeftParen)?;
        let types = self.without_angle(|parser| {
            let start = parser.tree.type_lists.begin();
            loop {
                let ty = parser.builtin_type_id(true)?;
                parser.tree.type_lists.push(ty);
                if !parser.accept(P::Comma)? {
                    break;
                }
            }
            let types = parser.tree.type_lists.finish(start)?;
            // Frozen arity rejection does not add a furthest grammar failure.
            if predicate.arity() != 0
                && parser.tree.type_lists.get(types).len() != predicate.arity()
            {
                return Err(ParseError::Syntax {
                    location: parser.error_mark().1,
                    message: "syntax error",
                });
            }
            parser.expect(P::RightParen)?;
            Ok(types)
        })?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::TypePredicate { predicate, types },
        })
    }

    pub(super) fn at_type_transform(&mut self) -> Result<Option<TypeTransform>, ParseError> {
        Ok(
            if let TokenKind::Identifier(name) = self.token()?.kind
                && let Some(transform) = TypeTransform::find(self.cursor.name(name))
                && self.at_ahead(P::LeftParen, 1)?
            {
                Some(transform)
            } else {
                None
            },
        )
    }

    pub(super) fn transform_operand(
        &mut self,
        transform: TypeTransform,
    ) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        self.cursor.advance()?;
        self.expect(P::LeftParen)?;
        let ty = self.without_angle(|parser| {
            let ty = parser.transform_type_id()?;
            let mut extra_type = false;
            while parser.accept(P::Comma)? {
                parser.transform_type_id()?;
                extra_type = true;
            }
            // The frozen arity check abandons the reading before expect(),
            // without recording a new furthest grammar failure.
            if extra_type {
                return Err(ParseError::Syntax {
                    location: parser.error_mark().1,
                    message: "syntax error",
                });
            }
            parser.expect(P::RightParen)?;
            Ok(ty)
        })?;
        let start = self.tree.type_lists.begin();
        self.tree.type_lists.push(ty);
        let types = self.tree.type_lists.finish(start)?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::TypeTransform { transform, types },
        })
    }
    fn transform_type_id(&mut self) -> Result<super::ast::TypeId, ParseError> {
        self.builtin_type_id(false)
    }

    fn builtin_type_id(&mut self, definite: bool) -> Result<super::ast::TypeId, ParseError> {
        let mentions = self.pack_mentions;
        let ty = self.type_id(definite)?;
        if self
            .tree
            .types
            .get(ty)
            .declarator
            .is_some_and(|id| self.tree.declarators.get(id).pack)
        {
            self.pack_mentions = mentions;
        }
        Ok(ty)
    }
}
