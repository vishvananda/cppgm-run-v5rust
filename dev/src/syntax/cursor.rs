// Translated from syntax/parse_token_stream.{h,cpp}. See NOTICE.
//! Pull window: no whole-unit token vector; checkpoints retain only their region.
use super::{
    ParseError,
    ast::{Literal, LiteralData, Operator},
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
const _: () = assert!(std::mem::size_of::<Token>() <= 32);

#[derive(Clone, Copy, Debug)]
pub(super) struct TokenRun {
    begin: u32,
    end: u32,
    absolute: u64,
}
impl TokenRun {
    pub(super) fn len(self) -> u32 {
        self.end - self.begin
    }
}
struct PackChange {
    ordinal: u64,
    alignment: u32,
}
struct Replay {
    run: TokenRun,
    resume: usize,
}
pub struct Cursor<'a> {
    pub post: PostTokenizer<Preprocessor<'a>>,
    pub spellings: TextStore,
    window: Vec<Token>,
    pack_changes: Vec<PackChange>,
    last_pack: u32,
    null_literal: Option<Literal>,
    runs: Vec<Token>,
    replay: Vec<Replay>,
    pub peak_run_tokens: crate::support::telemetry::Counter<usize>,
    pub position: usize,
    retained: usize,
    base: u64,
    origin: u64,
    end: Option<Token>,
    pub tokens_read: crate::support::telemetry::Counter<u64>,
    pub peak_window: crate::support::telemetry::Counter<usize>,
}
impl<'a> Cursor<'a> {
    pub(super) fn new(post: PostTokenizer<Preprocessor<'a>>) -> Self {
        Self {
            post,
            spellings: TextStore::default(),
            window: Vec::new(),
            pack_changes: Vec::new(),
            last_pack: 0,
            null_literal: None,
            runs: Vec::new(),
            replay: Vec::new(),
            peak_run_tokens: crate::support::telemetry::counter(0),
            position: 0,
            retained: 0,
            base: 0,
            origin: 0,
            end: None,
            tokens_read: crate::support::telemetry::counter(0),
            peak_window: crate::support::telemetry::counter(0),
        }
    }
    pub(super) fn set_origin(&mut self, origin: u64) {
        self.base = origin;
        self.origin = origin;
    }
    pub(super) fn set_source_origin(&mut self, origin: u64) {
        self.origin = origin;
    }
    #[inline]
    pub(super) fn peek(&mut self, ahead: usize) -> Result<Token, ParseError> {
        let index = self
            .position
            .checked_add(ahead)
            .ok_or(ParseError::Limit("parser lookahead limit"))?;
        if let Some(replay) = self.replay.last() {
            let run = replay.run;
            return Ok(if index < (run.end - run.begin) as usize {
                self.runs[run.begin as usize + index]
            } else {
                Self::synthetic_end_token()
            });
        }
        if self.window.len() <= index {
            self.fill(index)?;
        }
        Ok(self.window[index])
    }
    #[cold]
    fn fill(&mut self, index: usize) -> Result<(), ParseError> {
        while self.window.len() <= index {
            if self.end.is_some() {
                self.window.push(Self::synthetic_end_token());
            } else {
                self.pull()?;
            }
        }
        self.peak_window = self.peak_window.max(self.window.len());
        Ok(())
    }
    fn synthetic_end_token() -> Token {
        // Exhausted lookahead uses the constructor's synthetic EOF, independent
        // of the physical input EOF that may already have been observed.
        Token {
            kind: TokenKind::End,
            location: SourceOffset(0),
            shift_half: 0,
            suffix_name: None,
        }
    }
    fn pull(&mut self) -> Result<(), ParseError> {
        let post = self.post.next_token().map_err(|error| match error {
            PostTokenError::Source(error) => ParseError::Preprocess(error),
            PostTokenError::Numeric(_) => ParseError::Limit("numeric literal implementation limit"),
            PostTokenError::Text(_) => ParseError::Limit("text literal implementation limit"),
        })?;
        self.tokens_read += 1;
        let alias = if matches!(post.kind, PostTokenKind::Identifier) {
            crate::lex::gnu_keywords::syntax_keyword(post.source)
        } else {
            None
        };
        let null = matches!(post.kind, PostTokenKind::Identifier) && post.source == b"__null";
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
            PostTokenKind::Identifier => {
                if let Some(crate::lex::gnu_keywords::SyntaxKeyword::Keyword(keyword)) = alias {
                    TokenKind::Simple(Operator {
                        token: SimpleToken::keyword(keyword),
                        alternative: false,
                    })
                } else {
                    TokenKind::Identifier(
                        post.name
                            .ok_or(ParseError::Invariant("missing parser identifier"))?,
                    )
                }
            }
            PostTokenKind::Invalid => {
                return Err(ParseError::CommittedSyntax {
                    location: post.begin,
                    message: "invalid token",
                });
            }
            PostTokenKind::EndOfFile => {
                self.end = Some(token);
                TokenKind::End
            }
            kind => {
                let literal = LiteralData {
                    kind,
                    token_form: false,
                    spelling: if matches!(kind, PostTokenKind::String { .. }) {
                        self.spellings.copy_string(post.source)?
                    } else {
                        self.spellings.add(post.source)?
                    },
                    data: self.spellings.add(post.data)?,
                    suffix_name: None,
                };
                TokenKind::Literal(self.spellings.literal(literal)?)
            }
        };
        if let TokenKind::Literal(id) = token.kind {
            let mut literal = *self.spellings.literals.get(id);
            let suffix = match literal.kind {
                PostTokenKind::String {
                    suffix: Some((begin, end)),
                    ..
                } => Some((begin, end)),
                PostTokenKind::Character {
                    suffix_begin: Some(begin),
                    ..
                }
                | PostTokenKind::UserDefinedNumber {
                    suffix_begin: begin,
                    ..
                } => Some((begin, self.spellings.get(literal.spelling).len())),
                _ => None,
            };
            if let Some((begin, end)) = suffix {
                literal.suffix_name = Some(
                    self.post
                        .input_mut()
                        .intern_name(&self.spellings.get(literal.spelling)[begin..end])
                        .map_err(ParseError::Preprocess)?,
                );
                self.spellings.literals.replace(id, literal);
            }
        }
        let alignment = self.post.input().pack_alignment();
        if alignment != self.last_pack {
            let ordinal = self
                .base
                .checked_add(
                    u64::try_from(self.window.len())
                        .map_err(|_| ParseError::Limit("packing token ordinal limit"))?,
                )
                .ok_or(ParseError::Limit("packing token ordinal limit"))?;
            self.pack_changes.push(PackChange { ordinal, alignment });
            self.last_pack = alignment;
        }
        if matches!(
            alias,
            Some(crate::lex::gnu_keywords::SyntaxKeyword::Ignored)
        ) {
            return Ok(());
        }
        if null {
            // The reserved name denotes a synthetic 0L token in the oracle.
            // Its immutable spelling and target bytes are owned only once.
            let literal = if let Some(literal) = self.null_literal {
                literal
            } else {
                let ty = crate::types::FundamentalType::LongInt;
                let data = crate::lex::floating::ScalarBytes::integer(ty, 0);
                let literal = LiteralData {
                    kind: PostTokenKind::Literal {
                        form: crate::lex::LiteralForm::Integer,
                        ty,
                        data,
                    },
                    spelling: self.spellings.add(b"0L")?,
                    token_form: false,
                    data: self.spellings.add(data.as_slice())?,
                    suffix_name: None,
                };
                let literal = self.spellings.literal(literal)?;
                self.null_literal = Some(literal);
                literal
            };
            token.kind = TokenKind::Literal(literal);
            token.suffix_name = None;
        }
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
    pub(super) fn pack_at(&self, ordinal: u64) -> u32 {
        // Lookahead and deferred replay observe the state at the original
        // token ordinal, never the preprocessor's later current state.
        let end = self
            .pack_changes
            .partition_point(|change| change.ordinal <= ordinal);
        end.checked_sub(1)
            .map_or(0, |index| self.pack_changes[index].alignment)
    }
    /// Source ordinals exclude the hosted declaration prefix, including in
    /// retained runs and after window compaction (`parse_token_stream::unit_position`).
    pub(super) fn unit_position(&self) -> Result<u64, ParseError> {
        Ok(self.absolute_position()?.saturating_sub(self.origin))
    }
    pub(super) fn absolute_position(&self) -> Result<u64, ParseError> {
        self.replay
            .last()
            .map_or(self.base, |replay| replay.run.absolute)
            .checked_add(
                u64::try_from(self.position)
                    .map_err(|_| ParseError::Limit("parser position limit"))?,
            )
            .ok_or(ParseError::Limit("parser position limit"))
    }
    pub(super) fn advance(&mut self) -> Result<(), ParseError> {
        if let Some(replay) = self.replay.last() {
            if self.position < (replay.run.end - replay.run.begin) as usize {
                self.position += 1;
            }
            return Ok(());
        }
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
    pub(super) fn runs_mark(&self) -> usize {
        self.runs.len()
    }
    pub(super) fn rewind_runs(&mut self, mark: usize) {
        self.runs.truncate(mark);
    }
    pub(super) fn capture(&mut self, start: usize, absolute: u64) -> Result<TokenRun, ParseError> {
        let begin = u32::try_from(self.runs.len())
            .map_err(|_| ParseError::Limit("deferred token limit"))?;
        let count = u32::try_from(self.position - start)
            .map_err(|_| ParseError::Limit("deferred token limit"))?;
        let end = begin
            .checked_add(count)
            .ok_or(ParseError::Limit("deferred token limit"))?;
        if let Some(replay) = self.replay.last() {
            let offset = replay.run.begin;
            let start =
                u32::try_from(start).map_err(|_| ParseError::Limit("deferred token limit"))?;
            return Ok(TokenRun {
                begin: offset + start,
                end: offset + start + count,
                absolute,
            });
        }
        let tokens = &self.window[start..self.position];
        self.runs.extend_from_slice(tokens);
        self.peak_run_tokens = self.peak_run_tokens.max(self.runs.len());
        Ok(TokenRun {
            begin,
            end,
            absolute,
        })
    }
    pub(super) fn push_run(&mut self, run: TokenRun) {
        self.replay.push(Replay {
            run,
            resume: self.position,
        });
        self.position = 0;
    }
    pub(super) fn pop_run(&mut self) -> Result<(), ParseError> {
        let replay = self
            .replay
            .pop()
            .ok_or(ParseError::Invariant("missing deferred token run"))?;
        self.position = replay.resume;
        Ok(())
    }
    pub(super) fn retain(&mut self) {
        self.retained += 1;
    }
    pub(super) fn release(&mut self) {
        self.retained -= 1;
    }
    #[inline]
    #[must_use]
    pub fn name(&self, name: NameId) -> &[u8] {
        self.post.input().name_spelling(name)
    }
    #[inline]
    pub fn intern_name(&mut self, spelling: &[u8]) -> Result<NameId, ParseError> {
        self.post
            .input_mut()
            .intern_name(spelling)
            .map_err(ParseError::Preprocess)
    }
    pub(super) fn describe(&self, error: &ParseError) -> Result<Vec<u8>, PreprocessError> {
        match error {
            ParseError::CloneAbort => Ok(b"unconsumed syntax clone abort".to_vec()),
            ParseError::Syntax { location, message }
            | ParseError::CommittedSyntax { location, message } => {
                self.post.input().describe(&PreprocessError::Directive {
                    location: *location,
                    message,
                })
            }
            ParseError::Preprocess(error) => self.post.input().describe(error),
            ParseError::Limit(message) | ParseError::Invariant(message) => {
                Ok(message.as_bytes().to_vec())
            }
        }
    }
}
