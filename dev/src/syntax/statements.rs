// Translated from syntax/parser_statements.cpp. See NOTICE.
use super::{
    ParseError,
    ast::{Condition, ExceptionDeclaration, FunctionBody, Handler, Statement, StmtId, StmtKind},
    conflicts::StatementReader,
    cursor::TokenKind,
    parser::Parser,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
#[derive(Clone, Copy)]
enum StatementHead {
    Declaration,
    TentativeDeclaration,
    Expression,
}
impl Parser<'_> {
    fn statement_head(&mut self) -> Result<StatementHead, ParseError> {
        match self.keyword()? {
            Some(word) if Self::is_specifier(word) && !Self::has_type_keyword(word) => {
                return Ok(StatementHead::Declaration);
            }
            Some(
                K::Auto
                | K::Class
                | K::Struct
                | K::Union
                | K::Enum
                | K::Namespace
                | K::Using
                | K::StaticAssert
                | K::Template,
            ) => {
                return Ok(StatementHead::Declaration);
            }
            Some(word) if Self::has_type_keyword(word) => {
                return Ok(if self.at_ahead(P::LeftBrace, 1)? {
                    StatementHead::Expression
                } else if self.at_ahead(P::LeftParen, 1)? {
                    StatementHead::TentativeDeclaration
                } else {
                    StatementHead::Declaration
                });
            }
            Some(K::Typename | K::Decltype) => return Ok(StatementHead::TentativeDeclaration),
            Some(_) => return Ok(StatementHead::Expression),
            None => (),
        }
        if !matches!(self.token()?.kind, TokenKind::Identifier(_)) && !self.at(P::Scope)? {
            return Ok(StatementHead::Expression);
        }
        let head = self.lookahead(|parser| {
            let (name, binding) = parser.type_name()?;
            let followed = matches!(parser.token()?.kind, TokenKind::Identifier(id) if !matches!(parser.cursor.name(id), b"final" | b"override"));
            if !parser.is_type_name(name, binding, false, followed) || parser.at(P::LeftBrace)? {
                Ok(StatementHead::Expression)
            } else if parser.at(P::LeftParen)? {
                Ok(StatementHead::TentativeDeclaration)
            } else { Ok(StatementHead::Declaration) }
        })?;
        Ok(head.unwrap_or(StatementHead::Expression))
    }
    fn declaration_or_expression(
        &mut self,
        attributes: super::storage::List<super::ast::Attribute>,
    ) -> Result<StmtKind, ParseError> {
        match self.statement_head()? {
            StatementHead::Declaration => {
                return Ok(StmtKind::Declaration(
                    self.declaration_with_attributes(attributes)?,
                ));
            }
            StatementHead::TentativeDeclaration => {
                if let Some(declaration) =
                    self.attempt(|parser| parser.declaration_with_attributes(attributes))?
                {
                    return Ok(StmtKind::Declaration(declaration));
                }
            }
            StatementHead::Expression => (),
        }
        let expression = self.expression()?;
        self.expect(P::Semicolon)?;
        Ok(StmtKind::Expression(Some(expression)))
    }
    pub(super) fn compound_statement(&mut self) -> Result<StmtId, ParseError> {
        self.compound_in_scope(true)
    }
    fn compound_in_scope(&mut self, new_scope: bool) -> Result<StmtId, ParseError> {
        self.with_conflict_scope(|parser| parser.compound_reading(new_scope))
    }
    fn compound_reading(&mut self, new_scope: bool) -> Result<StmtId, ParseError> {
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
        let attributes = self.collect_attributes(false)?;
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
            Some(K::Return | K::Throw) => {
                return self.read_statement(StatementReader::Value, attributes);
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
            Some(K::If | K::While | K::Switch | K::For) => {
                let names_conflicted = matches!(self.cursor.peek(2)?.kind, TokenKind::Identifier(name) if self.scopes.member_conflicts(name))
                    && (self.at_ahead(P::Less, 3)? || self.at_ahead(P::Scope, 3)?);
                return if names_conflicted {
                    self.read_statement(StatementReader::Control, attributes)
                } else {
                    self.statement_reading(StatementReader::Control, attributes)
                };
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
                } else {
                    return self.read_statement(StatementReader::Ordinary, attributes);
                }
            }
        };
        self.tree.statements.add(Statement { location, kind })
    }
    pub(super) fn statement_reading(
        &mut self,
        reader: StatementReader,
        attributes: super::storage::List<super::ast::Attribute>,
    ) -> Result<StmtId, ParseError> {
        let location = self.location()?;
        let kind = match reader {
            StatementReader::Ordinary => self.declaration_or_expression(attributes)?,
            StatementReader::Value => self.value_statement()?,
            StatementReader::Control => self.control_statement()?,
        };
        self.tree.statements.add(Statement { location, kind })
    }
    fn value_statement(&mut self) -> Result<StmtKind, ParseError> {
        let word = self
            .keyword()?
            .ok_or(ParseError::Invariant("missing value statement keyword"))?;
        Ok({
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
        })
    }
    fn control_statement(&mut self) -> Result<StmtKind, ParseError> {
        if self.keyword()? == Some(K::For) {
            return self.for_statement();
        }
        let word = self
            .keyword()?
            .ok_or(ParseError::Invariant("missing control statement keyword"))?;
        Ok({
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
        })
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
                self.declare_declarator(
                    declarator,
                    false,
                    self.specifier_scope(specifiers),
                    specifiers,
                );
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
        self.collect_attributes(false)?;
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
            parser.declare_declarator(declarator, false, None, super::storage::List::default());
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
        let location = self.location()?;
        self.cursor.advance()?;
        self.expect(P::LeftParen)?;
        let attributes = self.collect_attributes(false)?;
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
            parser.declare_declarator(declarator, false, None, super::storage::List::default());
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
        let mut readings = None;
        if !self.accept(P::Semicolon)? {
            let first = self.read_statement(StatementReader::Ordinary, attributes)?;
            match self.tree.statements.get(first).kind {
                StmtKind::Declaration(decl) => declaration = Some(decl),
                StmtKind::Expression(expression) => init = expression,
                StmtKind::Ambiguous { .. } => readings = Some(first),
                _ => return Err(ParseError::Invariant("invalid for-init reading")),
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
        let kind = StmtKind::For {
            declaration,
            init,
            condition,
            iteration,
            body,
        };
        if let Some(readings) = readings {
            let loop_statement = self.tree.statements.add(Statement { location, kind })?;
            let start = self.tree.stmt_lists.begin();
            self.tree.stmt_lists.push(readings);
            self.tree.stmt_lists.push(loop_statement);
            Ok(StmtKind::Compound(self.tree.stmt_lists.finish(start)?))
        } else {
            Ok(kind)
        }
    }
}
