// Translated from syntax/parser_statements.cpp. See NOTICE.
use super::{
    ParseError,
    ast::{Condition, ExceptionDeclaration, FunctionBody, Handler, Statement, StmtId, StmtKind},
    cursor::TokenKind,
    parser::Parser,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
impl Parser<'_> {
    pub(super) fn compound_statement(&mut self) -> Result<StmtId, ParseError> {
        self.compound_in_scope(true)
    }
    fn compound_in_scope(&mut self, new_scope: bool) -> Result<StmtId, ParseError> {
        let location = self.location()?;
        self.expect(P::LeftBrace)?;
        let enclosing = self.scopes.current;
        if new_scope {
            self.scopes.push_kind(super::scopes::ScopeKind::Block)?;
        }
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
                self.cursor.advance()?;
                let body = self.compound_statement()?;
                let handlers = self.handlers()?;
                StmtKind::Try { body, handlers }
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
    pub(super) fn function_body(&mut self, constructor: bool) -> Result<FunctionBody, ParseError> {
        let try_block = self.accept_keyword(K::Try)?;
        let initializers = if constructor && self.at(P::Colon)? {
            self.ctor_initializers()?
        } else {
            super::storage::List::default()
        };
        let compound = self.compound_in_scope(false)?;
        let handlers = if try_block {
            Some(self.handlers()?)
        } else {
            None
        };
        Ok(FunctionBody {
            initializers,
            compound,
            handlers,
        })
    }
    fn handlers(&mut self) -> Result<super::storage::List<Handler>, ParseError> {
        if self.keyword()? != Some(K::Catch) {
            return self.fail();
        }
        let start = self.tree.handlers.begin();
        while self.accept_keyword(K::Catch)? {
            self.expect(P::LeftParen)?;
            let enclosing = self.scopes.current;
            self.scopes.push()?;
            let result = self.handler();
            self.scopes.current = enclosing;
            self.tree.handlers.push(result?);
        }
        self.tree.handlers.finish(start)
    }
    fn handler(&mut self) -> Result<Handler, ParseError> {
        let exception = if self.accept(P::Ellipsis)? {
            ExceptionDeclaration::Ellipsis
        } else {
            let specifiers = self.specifier_sequence(false, false)?;
            if !self.specifiers_have_type(specifiers) {
                return self.fail();
            }
            let declarator = if let Some(declarator) =
                self.attempt(|parser| parser.declarator(super::declarators::Mode::Parameter))?
            {
                declarator
            } else {
                self.declarator(super::declarators::Mode::Abstract)?
            };
            let declarator = (!self.declarator_empty(declarator)).then_some(declarator);
            if let Some(declarator) = declarator {
                self.declare_declarator(declarator, false, self.specifier_scope(specifiers));
            }
            ExceptionDeclaration::Typed {
                specifiers,
                declarator,
            }
        };
        self.expect(P::RightParen)?;
        // The catch parameter and the outermost body share one block scope.
        let body = self.compound_in_scope(false)?;
        Ok(Handler { exception, body })
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
