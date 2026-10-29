// Translated from syntax/parser_statements.cpp::parse_asm. See NOTICE.
//! Structured operands retain parsed expressions; templates remain literal data.
use super::{ParseError, ast, cursor::TokenKind, parser::Parser};
use crate::lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword as K};

impl Parser<'_> {
    fn asm_literal(&mut self) -> Result<ast::Literal, ParseError> {
        if let TokenKind::Literal(literal) = self.token()?.kind
            && matches!(
                literal.kind(&self.cursor.spellings),
                PostTokenKind::String { .. }
            )
        {
            self.cursor.advance()?;
            return Ok(literal);
        }
        self.fail()
    }

    pub(super) fn asm_statement(&mut self) -> Result<ast::StmtKind, ParseError> {
        self.cursor.advance()?;
        loop {
            if !self.accept_keyword(K::Volatile)? && !self.accept_keyword(K::Inline)? {
                break;
            }
        }
        self.expect(P::LeftParen)?;
        let text = self.asm_literal()?;
        let mut outputs = Vec::new();
        let mut inputs = Vec::new();
        let mut clobbers = Vec::new();
        let mut section = 0;
        let mut pending = 0;
        loop {
            if pending == 0 {
                if self.accept(P::Colon)? {
                    pending = 1;
                } else if self.accept(P::Scope)? {
                    pending = 2;
                } else {
                    break;
                }
            }
            pending -= 1;
            section += 1;
            if section > 3 {
                return self.fail();
            }
            if pending != 0 || self.at(P::Colon)? || self.at(P::Scope)? || self.at(P::RightParen)? {
                continue;
            }
            loop {
                if section == 3 {
                    clobbers.push(self.asm_literal()?);
                } else {
                    let location = self.location()?;
                    let name = if self.accept(P::LeftBracket)? {
                        let name = self.identifier()?;
                        self.expect(P::RightBracket)?;
                        Some(name)
                    } else {
                        None
                    };
                    let constraint = self.asm_literal()?;
                    self.expect(P::LeftParen)?;
                    let expression = self.expression()?;
                    self.expect(P::RightParen)?;
                    let operand = ast::AsmOperand {
                        location,
                        name,
                        constraint,
                        expression,
                    };
                    if section == 1 {
                        outputs.push(operand);
                    } else {
                        inputs.push(operand);
                    }
                }
                if !self.accept(P::Comma)? {
                    break;
                }
            }
        }
        self.expect(P::RightParen)?;
        self.expect(P::Semicolon)?;
        let start = self.tree.asm_operands.begin();
        for operand in outputs {
            self.tree.asm_operands.push(operand);
        }
        let outputs = self.tree.asm_operands.finish(start)?;
        let start = self.tree.asm_operands.begin();
        for operand in inputs {
            self.tree.asm_operands.push(operand);
        }
        let inputs = self.tree.asm_operands.finish(start)?;
        let start = self.tree.asm_clobbers.begin();
        for clobber in clobbers {
            self.tree.asm_clobbers.push(clobber);
        }
        let clobbers = self.tree.asm_clobbers.finish(start)?;
        Ok(ast::StmtKind::Asm(self.tree.assembly.add(
            ast::AsmStatement {
                text,
                outputs,
                inputs,
                clobbers,
            },
        )?))
    }
}
