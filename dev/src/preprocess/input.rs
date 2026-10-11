// Translated from preprocess/source/source_stack.cpp (single-file input). See NOTICE.
//! Demand-driven text/directive boundary, with stable source views and interned names.
use super::error::PreprocessError;
use super::macros::token::{LEADING_SPACE, START_OF_LINE, Spelling, Token};
use crate::lex::identifiers::Identifiers;
use crate::lex::pp_lexer::{PpLexer, PpLexerOptions, PpTokenKind, Punctuator};
use crate::support::source::{SourceBuffer, SourceOffset};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Boundary {
    Text,
    Directive,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Text,
    Line,
    LineEnd,
}

pub(crate) struct SourceInput<'a> {
    source: &'a SourceBuffer,
    lexer: PpLexer<'a>,
    at_line_start: bool,
    boundary: Boundary,
    location: SourceOffset,
    mode: InputMode,
    peeked: Option<Token<'a>>,
    pub(crate) allow_va_args: bool,
    locations: super::locations::Locations<'a>,
}
impl<'a> SourceInput<'a> {
    pub(crate) fn with_name(
        source: &'a SourceBuffer,
        name: &[u8],
    ) -> Result<Self, PreprocessError> {
        Ok(Self {
            source,
            lexer: PpLexer::with_options(
                source,
                PpLexerOptions {
                    header_names_on_request: true,
                    ..PpLexerOptions::default()
                },
            )?,
            at_line_start: true,
            boundary: Boundary::Text,
            location: source.begin(),
            mode: InputMode::Text,
            peeked: None,
            allow_va_args: false,
            locations: super::locations::Locations::new(source, name),
        })
    }
    fn stabilize(
        &self,
        kind: PpTokenKind,
        flags: u8,
        names: &mut Identifiers,
    ) -> Result<Token<'a>, PreprocessError> {
        let pp = self.lexer.token(kind);
        let mut token = Token::end(pp.begin);
        token.kind = kind;
        token.end = pp.end;
        token.punctuator = pp.punctuator;
        token.flags = flags;
        if kind == PpTokenKind::Identifier {
            if pp.spelling == b"__VA_ARGS__" && !self.allow_va_args {
                return Err(PreprocessError::at(
                    pp.begin,
                    "__VA_ARGS__ outside a variadic macro's replacement list",
                ));
            }
            let name = names.intern(pp.spelling).map_err(PreprocessError::Limit)?;
            token.name = Some(name);
            token.spelling = Spelling::Shared(names.spelling(name));
        } else if let Some(text) = self
            .source
            .bytes()
            .get(pp.begin.index()..pp.end.index())
            .filter(|text| *text == pp.spelling)
        {
            token.spelling = Spelling::Source(text);
        } else {
            token.spelling = Spelling::Shared(pp.spelling.into());
        }
        Ok(token)
    }
    pub(crate) fn next(&mut self, names: &mut Identifiers) -> Result<Token<'a>, PreprocessError> {
        if let Some(token) = self.peeked.take() {
            return Ok(token);
        }
        if self.mode != InputMode::Text {
            if self.mode == InputMode::LineEnd {
                return Ok(Token::end(self.location));
            }
            let token = self.next_in_line(names)?;
            if token.kind == PpTokenKind::NewLine {
                self.mode = InputMode::LineEnd;
                self.location = token.begin;
                return Ok(Token::end(token.begin));
            }
            return Ok(token);
        }
        if self.boundary != Boundary::Text {
            return Ok(Token::end(self.location));
        }
        let mut flags = if self.at_line_start {
            LEADING_SPACE | START_OF_LINE
        } else {
            0
        };
        loop {
            let kind = self.lexer.next_token()?.kind;
            match kind {
                PpTokenKind::WhitespaceSequence => {
                    flags |= LEADING_SPACE;
                    continue;
                }
                PpTokenKind::NewLine => {
                    flags |= LEADING_SPACE | START_OF_LINE;
                    self.at_line_start = true;
                    continue;
                }
                PpTokenKind::EndOfFile => {
                    self.boundary = Boundary::End;
                    self.location = self.lexer.token(kind).begin;
                    return Ok(Token::end(self.location));
                }
                _ => {}
            }
            self.at_line_start = false;
            if flags & START_OF_LINE != 0
                && self.lexer.token(kind).punctuator == Some(Punctuator::Hash)
            {
                self.boundary = Boundary::Directive;
                self.location = self.lexer.token(kind).begin;
                return Ok(Token::end(self.location));
            }
            return self.stabilize(kind, flags, names);
        }
    }
    pub(crate) fn peek(&mut self, names: &mut Identifiers) -> Result<&Token<'a>, PreprocessError> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next(names)?);
        }
        self.peeked
            .as_ref()
            .ok_or(PreprocessError::Invariant("missing source lookahead"))
    }
    pub(crate) fn next_in_line(
        &mut self,
        names: &mut Identifiers,
    ) -> Result<Token<'a>, PreprocessError> {
        let mut flags = 0;
        loop {
            let kind = self.lexer.next_token()?.kind;
            if kind == PpTokenKind::WhitespaceSequence {
                flags |= LEADING_SPACE;
                continue;
            }
            if matches!(kind, PpTokenKind::NewLine | PpTokenKind::EndOfFile) {
                self.at_line_start = true;
                let mut token = Token::end(self.lexer.token(kind).begin);
                token.kind = PpTokenKind::NewLine;
                return Ok(token);
            }
            return self.stabilize(kind, flags, names);
        }
    }
    pub(crate) fn begin_line_input(&mut self) {
        self.mode = InputMode::Line;
        self.peeked = None;
    }
    pub(crate) fn end_line_input(&mut self) -> Result<(), PreprocessError> {
        if self.mode != InputMode::LineEnd {
            self.skip_line()?;
        }
        self.mode = InputMode::Text;
        self.peeked = None;
        Ok(())
    }
    pub(crate) fn request_header_name(&mut self) {
        self.lexer.request_header_name();
    }
    pub(crate) fn skip_line(&mut self) -> Result<(), PreprocessError> {
        if self.mode == InputMode::LineEnd {
            return Ok(());
        }
        self.peeked = None;
        loop {
            let kind = self.lexer.next_token()?.kind;
            if matches!(kind, PpTokenKind::NewLine | PpTokenKind::EndOfFile) {
                self.at_line_start = true;
                if self.mode != InputMode::Text {
                    self.mode = InputMode::LineEnd;
                }
                return Ok(());
            }
        }
    }
    // Inactive text is lexed for boundaries only: it is never stabilized or interned.
    pub(crate) fn skip_to_directive(&mut self) -> Result<bool, PreprocessError> {
        let mut line_start = self.at_line_start;
        self.peeked = None;
        loop {
            let pp = self.lexer.next_token()?;
            match pp.kind {
                PpTokenKind::WhitespaceSequence => continue,
                PpTokenKind::NewLine => {
                    line_start = true;
                    continue;
                }
                PpTokenKind::EndOfFile => {
                    self.at_line_start = true;
                    return Ok(false);
                }
                _ => {}
            }
            if line_start && pp.punctuator == Some(Punctuator::Hash) {
                self.at_line_start = false;
                self.location = pp.begin;
                return Ok(true);
            }
            line_start = false;
        }
    }
    pub(crate) fn at_directive(&self) -> bool {
        self.boundary == Boundary::Directive
    }
    pub(crate) fn end_directive(&mut self) {
        self.boundary = Boundary::Text;
        self.peeked = None;
    }
    pub(crate) fn presumed_line(&self, location: SourceOffset) -> Result<i64, PreprocessError> {
        self.locations.presumed_line(location)
    }
    pub(crate) fn file_literal(&mut self) -> std::rc::Rc<[u8]> {
        self.locations.file_literal()
    }
    pub(crate) fn set_line(
        &mut self,
        number: u32,
        name: Option<Vec<u8>>,
    ) -> Result<(), PreprocessError> {
        self.locations.set_line(self.location, number, name)
    }
    pub(crate) fn describe(
        &self,
        error: &crate::support::source::SourceError,
    ) -> Result<Vec<u8>, PreprocessError> {
        self.locations.describe(error)
    }
}
