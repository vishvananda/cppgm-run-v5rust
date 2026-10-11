// Translated from preprocess/expr/controlling_expression.{h,cpp}. See NOTICE.
//! Parse and evaluate once on demand, with flat stacks reused between logical lines.
use super::arithmetic::{ArithmeticResult, Operator, PpValue, apply_binary, apply_unary};
use crate::lex::number::{NumberDialect, NumberLiteral, classify_number};
use crate::lex::pp_lexer::{PpToken, PpTokenCursor, PpTokenKind, Punctuator};
use crate::lex::text::evaluate_character;
use crate::support::source::SourceError;
use crate::types::FundamentalType;

/// The real preprocessor can supply its macro table without changing evaluation.
pub trait MacroDefinitionQuery {
    fn is_defined(&self, spelling: &[u8]) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionStatus {
    Empty,
    Valid(PpValue),
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpressionResult {
    pub status: ExpressionStatus,
    pub end_of_file: bool,
}

#[derive(Debug)]
pub enum ExpressionError {
    Source(SourceError),
    Invariant(&'static str),
}

impl From<SourceError> for ExpressionError {
    fn from(error: SourceError) -> Self {
        Self::Source(error)
    }
}

#[derive(Clone, Copy)]
enum Token {
    Operand(PpValue),
    Defined,
    Operator(Operator),
    Question,
    Colon,
    LeftParen,
    RightParen,
    End,
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    Operator(Operator),
    Condition,
    Alternative,
    Group,
}

/// Owns only pending operators and typed values, never a token sequence or tree.
pub struct ExpressionEvaluator<'a, M: MacroDefinitionQuery> {
    macros: &'a M,
    literals: NumberDialect,
    at_line_end: bool,
    end_of_file: bool,
    failed: bool,
    unevaluated: usize,
    values: Vec<PpValue>,
    pending: Vec<Pending>,
}

impl<'a, M: MacroDefinitionQuery> ExpressionEvaluator<'a, M> {
    #[must_use]
    pub fn new(macros: &'a M, literals: NumberDialect) -> Self {
        Self {
            macros,
            literals,
            at_line_end: false,
            end_of_file: false,
            failed: false,
            unevaluated: 0,
            values: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// Consume through the next newline or EOF, skipping the rest after the first error.
    ///
    /// # Errors
    /// Propagates source errors and reports broken parser stack invariants separately.
    pub fn evaluate(
        &mut self,
        input: &mut impl PpTokenCursor,
    ) -> Result<ExpressionResult, ExpressionError> {
        self.at_line_end = false;
        self.failed = false;
        self.unevaluated = 0;
        self.values.clear();
        self.pending.clear();
        let token = self.read_token(input)?;
        let status = if matches!(token, Token::End) {
            ExpressionStatus::Empty
        } else {
            self.parse(input, token)?;
            while !self.at_line_end {
                self.pull(input, |_| ())?;
            }
            if self.failed {
                ExpressionStatus::Invalid
            } else {
                ExpressionStatus::Valid(self.top_value()?)
            }
        };
        Ok(ExpressionResult {
            status,
            end_of_file: self.end_of_file,
        })
    }

    // Consume borrowed text within one demand; no spelling survives in the stacks.
    fn pull<T>(
        &mut self,
        input: &mut impl PpTokenCursor,
        consume: impl FnOnce(PpToken<'_>) -> T,
    ) -> Result<T, ExpressionError> {
        loop {
            let pp = input.next_token()?;
            if pp.kind == PpTokenKind::WhitespaceSequence {
                continue;
            }
            if matches!(pp.kind, PpTokenKind::NewLine | PpTokenKind::EndOfFile) {
                self.at_line_end = true;
                self.end_of_file = pp.kind == PpTokenKind::EndOfFile;
            }
            return Ok(consume(pp));
        }
    }

    fn read_token(&mut self, input: &mut impl PpTokenCursor) -> Result<Token, ExpressionError> {
        let dialect = self.literals;
        self.pull(input, |pp| classify_token(&pp, dialect))
    }

    fn read_defined(&mut self, input: &mut impl PpTokenCursor) -> Result<(), ExpressionError> {
        let macros = self.macros;
        let read_name = |pp: PpToken<'_>| {
            if pp.kind == PpTokenKind::Identifier {
                Some(macros.is_defined(pp.spelling))
            } else {
                None
            }
        };
        let (parenthesized, mut defined) = self.pull(input, |pp| {
            (
                pp.kind == PpTokenKind::PreprocessingOpOrPunc && pp.spelling == b"(",
                read_name(pp),
            )
        })?;
        if parenthesized {
            defined = self.pull(input, read_name)?;
        }
        let Some(defined) = defined else {
            self.failed = true;
            return Ok(());
        };
        if parenthesized
            && !self.pull(input, |pp| {
                pp.kind == PpTokenKind::PreprocessingOpOrPunc && pp.spelling == b")"
            })?
        {
            self.failed = true;
            return Ok(());
        }
        self.values.push(PpValue::boolean(defined));
        Ok(())
    }

    fn parse(
        &mut self,
        input: &mut impl PpTokenCursor,
        mut token: Token,
    ) -> Result<(), ExpressionError> {
        let mut want_operand = true;
        loop {
            if want_operand {
                want_operand = self.shift_operand(input, token)?;
            } else if matches!(token, Token::End) {
                self.reduce_operators()?;
                self.failed |= !self.pending.is_empty();
                return Ok(());
            } else {
                want_operand = self.shift_operator(token)?;
            }
            if self.failed {
                return Ok(());
            }
            token = self.read_token(input)?;
        }
    }

    fn shift_operand(
        &mut self,
        input: &mut impl PpTokenCursor,
        token: Token,
    ) -> Result<bool, ExpressionError> {
        match token {
            Token::Operand(value) => {
                self.values.push(value);
                return Ok(false);
            }
            Token::Defined => {
                self.read_defined(input)?;
                return Ok(false);
            }
            Token::Operator(op) => {
                let unary = match op {
                    Operator::Add => Operator::Plus,
                    Operator::Subtract => Operator::Minus,
                    _ if op.unary() => op,
                    _ => {
                        self.failed = true;
                        return Ok(true);
                    }
                };
                self.pending.push(Pending::Operator(unary));
            }
            Token::LeftParen => self.pending.push(Pending::Group),
            _ => self.failed = true,
        }
        Ok(true)
    }

    fn shift_operator(&mut self, token: Token) -> Result<bool, ExpressionError> {
        match token {
            Token::Operator(op) if !op.unary() => self.shift_binary(op)?,
            Token::Question => {
                while matches!(self.pending.last(), Some(Pending::Operator(_))) {
                    self.reduce_top()?;
                }
                if self.top_value()?.bits == 0 {
                    self.unevaluated += 1;
                }
                self.pending.push(Pending::Condition);
            }
            Token::Colon => {
                self.reduce_operators()?;
                if self.pending.last() == Some(&Pending::Condition) {
                    self.pending.pop();
                    self.pending.push(Pending::Alternative);
                    let condition = self
                        .values
                        .len()
                        .checked_sub(2)
                        .and_then(|i| self.values.get(i))
                        .ok_or(ExpressionError::Invariant("missing conditional value"))?;
                    if condition.bits == 0 {
                        self.end_unevaluated()?;
                    } else {
                        self.unevaluated += 1;
                    }
                } else {
                    self.failed = true;
                }
            }
            Token::RightParen => {
                self.reduce_operators()?;
                if self.pending.last() == Some(&Pending::Group) {
                    self.pending.pop();
                } else {
                    self.failed = true;
                }
                return Ok(false);
            }
            _ => self.failed = true,
        }
        Ok(true)
    }

    fn shift_binary(&mut self, op: Operator) -> Result<(), ExpressionError> {
        while self.pending.last().is_some_and(|pending| matches!(pending, Pending::Operator(top) if top.precedence() >= op.precedence())) {
            self.reduce_top()?;
        }
        if op.skips_right(self.top_value()?) {
            self.unevaluated += 1;
        }
        self.pending.push(Pending::Operator(op));
        Ok(())
    }

    fn reduce_operators(&mut self) -> Result<(), ExpressionError> {
        while matches!(
            self.pending.last(),
            Some(Pending::Operator(_) | Pending::Alternative)
        ) {
            self.reduce_top()?;
        }
        Ok(())
    }

    fn reduce_top(&mut self) -> Result<(), ExpressionError> {
        let pending = self
            .pending
            .pop()
            .ok_or(ExpressionError::Invariant("missing pending operator"))?;
        if pending == Pending::Alternative {
            let if_false = self.pop_value()?;
            let if_true = self.pop_value()?;
            let condition = self.pop_value()?;
            if condition.bits != 0 {
                self.end_unevaluated()?;
            }
            let mut value = if condition.bits != 0 {
                if_true
            } else {
                if_false
            };
            value.is_unsigned = if_true.is_unsigned || if_false.is_unsigned;
            self.values.push(value);
        } else if let Pending::Operator(op) = pending {
            let right = self.pop_value()?;
            let result = if op.unary() {
                apply_unary(op, right)
            } else {
                let left = self.pop_value()?;
                let result = apply_binary(op, left, right);
                // The operation itself is checked within the enclosing skipped scope.
                self.accept_arithmetic(result);
                if op.skips_right(left) {
                    self.end_unevaluated()?;
                }
                return Ok(());
            };
            self.accept_arithmetic(result);
        } else {
            return Err(ExpressionError::Invariant(
                "reduction crossed a grammar boundary",
            ));
        }
        Ok(())
    }

    fn accept_arithmetic(&mut self, result: ArithmeticResult) {
        if !result.valid && self.unevaluated == 0 {
            self.failed = true;
        }
        self.values.push(result.value);
    }

    fn end_unevaluated(&mut self) -> Result<(), ExpressionError> {
        self.unevaluated = self
            .unevaluated
            .checked_sub(1)
            .ok_or(ExpressionError::Invariant("unbalanced unevaluated scope"))?;
        Ok(())
    }

    fn top_value(&self) -> Result<PpValue, ExpressionError> {
        self.values
            .last()
            .copied()
            .ok_or(ExpressionError::Invariant("missing expression value"))
    }

    fn pop_value(&mut self) -> Result<PpValue, ExpressionError> {
        self.values
            .pop()
            .ok_or(ExpressionError::Invariant("missing expression operand"))
    }
}

fn unsigned_literal(ty: FundamentalType) -> bool {
    matches!(
        ty,
        FundamentalType::UnsignedInt
            | FundamentalType::UnsignedLongInt
            | FundamentalType::UnsignedLongLongInt
            | FundamentalType::Char16T
            | FundamentalType::Char32T
    )
}

fn classify_token(pp: &PpToken<'_>, dialect: NumberDialect) -> Token {
    match pp.kind {
        PpTokenKind::NewLine | PpTokenKind::EndOfFile => Token::End,
        PpTokenKind::Identifier if pp.spelling == b"defined" => Token::Defined,
        PpTokenKind::Identifier => Token::Operand(PpValue::boolean(pp.spelling == b"true")),
        PpTokenKind::PreprocessingOpOrPunc => classify_operator(pp.punctuator),
        PpTokenKind::PpNumber => match classify_number(pp.spelling, dialect) {
            NumberLiteral::Integer { ty, value } => Token::Operand(PpValue {
                bits: value,
                is_unsigned: unsigned_literal(ty),
            }),
            _ => Token::Invalid,
        },
        PpTokenKind::CharacterLiteral => match evaluate_character(pp.spelling) {
            Some(value) if !value.multicharacter && value.suffix_begin == pp.spelling.len() => {
                Token::Operand(PpValue {
                    bits: u64::from(value.code_point),
                    is_unsigned: unsigned_literal(value.ty),
                })
            }
            _ => Token::Invalid,
        },
        _ => Token::Invalid,
    }
}

fn classify_operator(punctuator: Option<Punctuator>) -> Token {
    Token::Operator(match punctuator {
        Some(Punctuator::Add) => Operator::Add,
        Some(Punctuator::Subtract) => Operator::Subtract,
        Some(Punctuator::LogicalNot) => Operator::LogicalNot,
        Some(Punctuator::Complement) => Operator::Complement,
        Some(Punctuator::Multiply) => Operator::Multiply,
        Some(Punctuator::Divide) => Operator::Divide,
        Some(Punctuator::Remainder) => Operator::Modulo,
        Some(Punctuator::ShiftLeft) => Operator::ShiftLeft,
        Some(Punctuator::ShiftRight) => Operator::ShiftRight,
        Some(Punctuator::Less) => Operator::Less,
        Some(Punctuator::Greater) => Operator::Greater,
        Some(Punctuator::LessEqual) => Operator::LessEqual,
        Some(Punctuator::GreaterEqual) => Operator::GreaterEqual,
        Some(Punctuator::Equal) => Operator::Equal,
        Some(Punctuator::NotEqual) => Operator::NotEqual,
        Some(Punctuator::BitAnd) => Operator::BitAnd,
        Some(Punctuator::Xor) => Operator::BitXor,
        Some(Punctuator::BitOr) => Operator::BitOr,
        Some(Punctuator::LogicalAnd) => Operator::LogicalAnd,
        Some(Punctuator::LogicalOr) => Operator::LogicalOr,
        Some(Punctuator::Question) => return Token::Question,
        Some(Punctuator::Colon) => return Token::Colon,
        Some(Punctuator::LeftParen) => return Token::LeftParen,
        Some(Punctuator::RightParen) => return Token::RightParen,
        _ => return Token::Invalid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::pp_lexer::PpLexer;
    use crate::support::source::SourceBuffer;
    use std::cell::Cell;

    struct Macros(Cell<usize>);
    impl MacroDefinitionQuery for Macros {
        fn is_defined(&self, spelling: &[u8]) -> bool {
            self.0.set(self.0.get() + 1);
            spelling == b"present"
        }
    }

    #[test]
    fn skipped_operands_are_converted_and_queried_but_not_required_to_have_values() {
        let source = SourceBuffer::new(
            b"0 && defined(present) / 0\nfalse ? 1 / 0u : -5\n1 || 1.0\n42\n".to_vec(),
        )
        .unwrap();
        let mut lexer = PpLexer::new(&source).unwrap();
        let macros = Macros(Cell::new(0));
        let mut evaluator = ExpressionEvaluator::new(&macros, NumberDialect::Cxx11);
        for expected in [
            ExpressionStatus::Valid(PpValue::boolean(false)),
            ExpressionStatus::Valid(PpValue {
                bits: 5u64.wrapping_neg(),
                is_unsigned: true,
            }),
            ExpressionStatus::Invalid,
            ExpressionStatus::Valid(PpValue {
                bits: 42,
                is_unsigned: false,
            }),
        ] {
            assert_eq!(evaluator.evaluate(&mut lexer).unwrap().status, expected);
        }
        assert_eq!(macros.0.get(), 1);
    }

    #[test]
    fn deep_nesting_reuses_flat_storage_and_line_failure_resets_scopes() {
        let depth = 50_000;
        let text = format!("{}1{}\n0 ? (\n7\n", "(".repeat(depth), ")".repeat(depth));
        let source = SourceBuffer::new(text.into_bytes()).unwrap();
        let mut lexer = PpLexer::new(&source).unwrap();
        let macros = Macros(Cell::new(0));
        let mut evaluator = ExpressionEvaluator::new(&macros, NumberDialect::Cxx11);
        assert_eq!(
            evaluator.evaluate(&mut lexer).unwrap().status,
            ExpressionStatus::Valid(PpValue::boolean(true))
        );
        let capacities = (evaluator.values.capacity(), evaluator.pending.capacity());
        assert_eq!(
            evaluator.evaluate(&mut lexer).unwrap().status,
            ExpressionStatus::Invalid
        );
        assert_eq!(
            evaluator.evaluate(&mut lexer).unwrap().status,
            ExpressionStatus::Valid(PpValue {
                bits: 7,
                is_unsigned: false
            })
        );
        assert_eq!(evaluator.unevaluated, 0);
        assert!(evaluator.pending.is_empty());
        assert_eq!(
            capacities,
            (evaluator.values.capacity(), evaluator.pending.capacity())
        );
    }
}
