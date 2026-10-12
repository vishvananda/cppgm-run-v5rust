// Translated from syntax/parse_token_stream.{h,cpp}. See NOTICE.
//! Pull window: no whole-unit token vector; checkpoints retain only their region.
use super::{
    ParseError,
    ast::{Literal, Operator},
    storage::TextStore,
};
use crate::lex::{
    PostTokenError, PostTokenKind, PostTokenizer, identifiers::NameId, pp_lexer::Punctuator,
    vocabulary::SimpleToken,
};
use crate::preprocess::{error::PreprocessError, unit::Preprocessor};
use crate::support::source::SourceOffset;
#[derive(Clone, Copy, Debug)]
pub(super) enum TokenKind {
    Simple(Operator),
    Identifier(NameId),
    Literal(Literal),
    End,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Token {
    pub kind: TokenKind,
    pub location: SourceOffset,
    pub shift_half: u8,
    pub suffix_name: Option<NameId>,
}
pub(super) struct Cursor<'a> {
    pub post: PostTokenizer<Preprocessor<'a>>,
    pub spellings: TextStore,
    window: Vec<Token>,
    pub position: usize,
    retained: usize,
    base: u64,
    end: Option<Token>,
    pub tokens_read: u64,
    pub peak_window: usize,
}
impl<'a> Cursor<'a> {
    pub(super) fn new(post: PostTokenizer<Preprocessor<'a>>) -> Self {
        Self {
            post,
            spellings: TextStore::default(),
            window: Vec::new(),
            position: 0,
            retained: 0,
            base: 0,
            end: None,
            tokens_read: 0,
            peak_window: 0,
        }
    }
    pub(super) fn peek(&mut self, ahead: usize) -> Result<Token, ParseError> {
        let index = self
            .position
            .checked_add(ahead)
            .ok_or(ParseError::Limit("parser lookahead limit"))?;
        while self.window.len() <= index {
            if let Some(end) = self.end {
                return Ok(end);
            }
            self.pull()?;
        }
        self.peak_window = self.peak_window.max(self.window.len());
        Ok(self.window[index])
    }
    fn pull(&mut self) -> Result<(), ParseError> {
        let post = self.post.next_token().map_err(|error| match error {
            PostTokenError::Source(error) => ParseError::Preprocess(error),
            PostTokenError::Numeric(_) => ParseError::Limit("numeric literal implementation limit"),
            PostTokenError::Text(_) => ParseError::Limit("text literal implementation limit"),
        })?;
        self.tokens_read += 1;
        let mut token = Token {
            location: post.begin,
            shift_half: 0,
            suffix_name: post.name,
            kind: TokenKind::End,
        };
        token.kind = match post.kind {
            PostTokenKind::Simple(simple) => TokenKind::Simple(Operator {
                token: simple,
                alternative: matches!(
                    post.source,
                    b"and"
                        | b"or"
                        | b"not"
                        | b"bitand"
                        | b"bitor"
                        | b"xor"
                        | b"compl"
                        | b"and_eq"
                        | b"or_eq"
                        | b"xor_eq"
                        | b"not_eq"
                ),
            }),
            PostTokenKind::Identifier => TokenKind::Identifier(
                post.name
                    .ok_or(ParseError::Invariant("missing parser identifier"))?,
            ),
            PostTokenKind::Invalid => {
                return Err(ParseError::Syntax {
                    location: post.begin,
                    message: "invalid token",
                });
            }
            PostTokenKind::EndOfFile => {
                self.end = Some(token);
                TokenKind::End
            }
            kind => TokenKind::Literal(Literal {
                kind,
                spelling: self.spellings.add(post.source)?,
                data: self.spellings.add(post.data)?,
                suffix: self.spellings.add(post.suffix)?,
            }),
        };
        if matches!(token.kind, TokenKind::Simple(Operator { token: simple, .. }) if simple.as_punctuator() == Some(Punctuator::ShiftRight))
        {
            let greater = SimpleToken::punctuator(Punctuator::Greater)
                .ok_or(ParseError::Invariant("missing greater token"))?;
            token.kind = TokenKind::Simple(Operator {
                token: greater,
                alternative: false,
            });
            token.shift_half = 1;
            self.window.push(token);
            token.shift_half = 2;
            token.location = SourceOffset(token.location.0.wrapping_add(1));
        }
        self.window.push(token);
        Ok(())
    }
    pub(super) fn absolute_position(&self) -> Result<u64, ParseError> {
        self.base
            .checked_add(
                u64::try_from(self.position)
                    .map_err(|_| ParseError::Limit("parser position limit"))?,
            )
            .ok_or(ParseError::Limit("parser position limit"))
    }
    pub(super) fn advance(&mut self) -> Result<(), ParseError> {
        if self.position < self.window.len()
            && !matches!(self.window[self.position].kind, TokenKind::End)
        {
            self.position += 1;
        }
        if self.retained == 0 && self.position >= 4096 {
            self.window.drain(..self.position);
            self.base = self.absolute_position()?;
            self.position = 0;
        }
        Ok(())
    }
    pub(super) fn retain(&mut self) {
        self.retained += 1;
    }
    pub(super) fn release(&mut self) {
        self.retained -= 1;
    }
    pub(super) fn name(&self, name: NameId) -> &[u8] {
        self.post.input().name_spelling(name)
    }
    pub(super) fn describe(&self, error: &ParseError) -> Result<Vec<u8>, PreprocessError> {
        match error {
            ParseError::Syntax { location, message } => {
                self.post.input().describe(&PreprocessError::Directive {
                    location: *location,
                    message,
                })
            }
            ParseError::Preprocess(error) => self.post.input().describe(error),
            ParseError::Limit(message)
            | ParseError::Invariant(message)
            | ParseError::NotImplemented(message) => Ok(message.as_bytes().to_vec()),
        }
    }
}
