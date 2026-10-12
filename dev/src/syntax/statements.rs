// Translated from syntax/parser_statements.cpp. See NOTICE.
use super::{
    ParseError,
    ast::{Condition, Statement, StmtId, StmtKind},
    cursor::TokenKind,
    parser::Parser,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
impl Parser<'_> {
    pub(super) fn compound_statement(&mut self) -> Result<StmtId, ParseError> {
        let location = self.location()?;
        self.expect(P::LeftBrace)?;
        let enclosing = self.scopes.current;
        self.scopes.push()?;
        let start = self.tree.stmt_lists.begin();
        while !self.at(P::RightBrace)? {
            let statement = self.statement()?;
            self.tree.stmt_lists.push(statement);
        }
        self.expect(P::RightBrace)?;
        self.scopes.current = enclosing;
        let statements = self.tree.stmt_lists.finish(start)?;
        self.tree.statements.add(Statement {
            location,
            kind: StmtKind::Compound(statements),
        })
    }
    fn statement(&mut self) -> Result<StmtId, ParseError> {
        self.nested(Self::statement_inner)
    }
    fn statement_inner(&mut self) -> Result<StmtId, ParseError> {
        self.attributes_boundary()?;
        let location = self.location()?;
        if self.at(P::LeftBrace)? {
            return self.compound_statement();
        }
        if let TokenKind::Identifier(name) = self.token()?.kind
            && self.at_ahead(P::Colon, 1)?
        {
            self.cursor.advance()?;
            self.cursor.advance()?;
            let body = self.statement()?;
            return self.tree.statements.add(Statement {
                location,
                kind: StmtKind::Label { name, body },
            });
        }
        let kind = match self.keyword()? {
            Some(word @ (K::Return | K::Throw)) => {
                self.cursor.advance()?;
                let value = if self.at(P::Semicolon)? {
                    None
                } else if word == K::Return {
                    Some(self.expression()?)
                } else {
                    Some(self.assignment_expression()?)
                };
                self.expect(P::Semicolon)?;
                if word == K::Return {
                    StmtKind::Return(value)
                } else {
                    StmtKind::Throw(value)
                }
            }
            Some(word @ (K::Break | K::Continue)) => {
                self.cursor.advance()?;
                self.expect(P::Semicolon)?;
                if word == K::Break {
                    StmtKind::Break
                } else {
                    StmtKind::Continue
                }
            }
            Some(K::Goto) => {
                self.cursor.advance()?;
                let name = self.identifier()?;
                self.expect(P::Semicolon)?;
                StmtKind::Goto(name)
            }
            Some(K::Case) => {
                self.cursor.advance()?;
                let value = self.expression()?;
                self.expect(P::Colon)?;
                StmtKind::Case {
                    value,
                    body: self.statement()?,
                }
            }
            Some(K::Default) => {
                self.cursor.advance()?;
                self.expect(P::Colon)?;
                StmtKind::Default(self.statement()?)
            }
            Some(word @ (K::If | K::While | K::Switch)) => {
                self.cursor.advance()?;
                self.expect(P::LeftParen)?;
                let enclosing = self.scopes.current;
                self.scopes.push()?;
                let condition = self.condition()?;
                self.expect(P::RightParen)?;
                let body = self.statement()?;
                let kind = if word == K::If {
                    let else_branch = if self.accept_keyword(K::Else)? {
                        Some(self.statement()?)
                    } else {
                        None
                    };
                    StmtKind::If {
                        condition,
                        then_branch: body,
                        else_branch,
                    }
                } else {
                    StmtKind::ConditionalLoop {
                        keyword: word,
                        condition,
                        body,
                    }
                };
                self.scopes.current = enclosing;
                kind
            }
            Some(K::Do) => {
                self.cursor.advance()?;
                let body = self.statement()?;
                if !self.accept_keyword(K::While)? {
                    return self.fail();
                }
                self.expect(P::LeftParen)?;
                let condition = self.expression()?;
                self.expect(P::RightParen)?;
                self.expect(P::Semicolon)?;
                StmtKind::Do { body, condition }
            }
            Some(K::For) => self.for_statement()?,
            Some(K::Try) => {
                return Err(ParseError::NotImplemented(
                    "try statements and exception handlers",
                ));
            }
            Some(K::Asm) => return Err(ParseError::NotImplemented("assembly statement syntax")),
            _ => {
                if self.accept(P::Semicolon)? {
                    StmtKind::Expression(None)
                } else if let Some(declaration) = self.attempt(Self::declaration)? {
                    StmtKind::Declaration(declaration)
                } else {
                    let expression = self.expression()?;
                    self.expect(P::Semicolon)?;
                    StmtKind::Expression(Some(expression))
                }
            }
        };
        self.tree.statements.add(Statement { location, kind })
    }
    fn condition(&mut self) -> Result<Condition, ParseError> {
        if let Some(condition) = self.attempt(|parser| {
            let specifiers = parser.specifier_sequence(false, false)?;
            let declarator = parser.declarator(super::declarators::Mode::Named)?;
            if !matches!(parser.punctuator()?, Some(P::Assign | P::LeftBrace)) {
                // This optional alternative does not call the oracle's fail():
                // its missing initializer must not replace a later expression error.
                return Err(ParseError::Syntax {
                    location: parser.location()?,
                    message: "syntax error",
                });
            }
            let initializer = parser.initializer()?;
            parser.declare_declarator(declarator, false, None);
            Ok(Condition::Declaration {
                specifiers,
                declarator,
                initializer,
            })
        })? {
            return Ok(condition);
        }
        Ok(Condition::Expression(self.expression()?))
    }
    fn for_statement(&mut self) -> Result<StmtKind, ParseError> {
        self.cursor.advance()?;
        self.expect(P::LeftParen)?;
        let enclosing = self.scopes.current;
        self.scopes.push()?;
        if let Some((specifiers, declarator)) = self.attempt(|parser| {
            let specifiers = parser.specifier_sequence(false, false)?;
            let declarator = parser.declarator(super::declarators::Mode::Named)?;
            if !parser.accept(P::Colon)? {
                return Err(ParseError::Syntax {
                    location: parser.location()?,
                    message: "syntax error",
                });
            }
            parser.declare_declarator(declarator, false, None);
            Ok((specifiers, declarator))
        })? {
            let range = self.expression()?;
            self.expect(P::RightParen)?;
            let body = self.statement()?;
            self.scopes.current = enclosing;
            return Ok(StmtKind::RangeFor {
                specifiers,
                declarator,
                range,
                body,
            });
        }
        let mut declaration = None;
        let mut init = None;
        if !self.accept(P::Semicolon)? {
            declaration = self.attempt(Self::declaration)?;
            if declaration.is_none() {
                init = Some(self.expression()?);
                self.expect(P::Semicolon)?;
            }
        }
        let condition = if self.at(P::Semicolon)? {
            None
        } else {
            Some(self.condition()?)
        };
        self.expect(P::Semicolon)?;
        let iteration = if self.at(P::RightParen)? {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(P::RightParen)?;
        let body = self.statement()?;
        self.scopes.current = enclosing;
        Ok(StmtKind::For {
            declaration,
            init,
            condition,
            iteration,
            body,
        })
    }
}
