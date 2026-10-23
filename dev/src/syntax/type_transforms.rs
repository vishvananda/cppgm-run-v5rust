// Translated from syntax/parser_specifiers.cpp::parse_transform_specifier. See NOTICE.
use super::{
    ParseError,
    ast::{ExprId, ExprKind, Expression},
    cursor::TokenKind,
    parser::Parser,
};
use crate::lex::{pp_lexer::Punctuator as P, type_transforms::TypeTransform};

impl Parser<'_> {
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
        let mentions = self.pack_mentions;
        let ty = self.type_id(false)?;
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
