//! Demand-driven phase 3 lexer. Returned text borrows the source or one reused scratch buffer.
pub(crate) mod classes;
mod literals;
mod punctuators;
pub use punctuators::Punctuator;
pub(super) use punctuators::identifier_operator;

use crate::support::source::{
    END_OF_SOURCE, SourceBuffer, SourceChar, SourceError, SourceOffset, TranslationReader,
};
use crate::support::unicode::append_utf8;
use classes::{
    basic, control, digit, hex_value, horizontal, identifier_continue, identifier_start,
};

/// Preprocessing vocabulary shared by the lexer and its consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PpTokenKind {
    WhitespaceSequence,
    NewLine,
    HeaderName,
    Identifier,
    PpNumber,
    CharacterLiteral,
    UserDefinedCharacterLiteral,
    StringLiteral,
    UserDefinedStringLiteral,
    PreprocessingOpOrPunc,
    NonWhitespaceCharacter,
    EndOfFile,
}

impl PpTokenKind {
    /// Observable dump spelling.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::WhitespaceSequence => "whitespace-sequence",
            Self::NewLine => "new-line",
            Self::HeaderName => "header-name",
            Self::Identifier => "identifier",
            Self::PpNumber => "pp-number",
            Self::CharacterLiteral => "character-literal",
            Self::UserDefinedCharacterLiteral => "user-defined-character-literal",
            Self::StringLiteral => "string-literal",
            Self::UserDefinedStringLiteral => "user-defined-string-literal",
            Self::PreprocessingOpOrPunc => "preprocessing-op-or-punc",
            Self::NonWhitespaceCharacter => "non-whitespace-character",
            Self::EndOfFile => "eof",
        }
    }
}

/// Token text is valid until the next lexer demand; source identities remain compact.
pub struct PpToken<'a> {
    pub kind: PpTokenKind,
    pub name: Option<super::identifiers::NameId>,
    pub begin: SourceOffset,
    pub end: SourceOffset,
    pub punctuator: Option<Punctuator>,
    pub spelling: &'a [u8],
}

/// Borrowed pull boundary for bare and, later, macro-expanded preprocessing input.
pub trait PpTokenCursor {
    /// Demand one token; its spelling lasts until the next demand.
    ///
    /// # Errors
    /// Propagates source decoding and tokenization failures.
    fn next_token(&mut self) -> Result<PpToken<'_>, SourceError>;
}

impl PpTokenCursor for PpLexer<'_> {
    fn next_token(&mut self) -> Result<PpToken<'_>, SourceError> {
        PpLexer::next_token(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirectiveContext {
    LineStart,
    AfterHash,
    AfterInclude,
    Other,
}

/// Dialect and source-view choices passed explicitly by the owning phase.
#[derive(Clone, Copy, Default)]
pub struct PpLexerOptions {
    /// Buffer base in the translation unit's 32-bit location space.
    pub location_base: u32,
    /// Directive handling requests header names instead of implicit `#include` tracking.
    pub header_names_on_request: bool,
    /// Disable trigraphs and splices for spellings that have already passed phases 1–2.
    pub token_text: bool,
    /// C++17 hexadecimal floating pp-numbers continue across `p`/`P` followed by a sign.
    pub binary_exponents: bool,
}

/// One lexer owns no token sequence and retains capacity only for translated text.
pub struct PpLexer<'a> {
    source: &'a SourceBuffer,
    reader: TranslationReader<'a>,
    current: SourceChar,
    begin: SourceOffset,
    clean_end: SourceOffset,
    end: SourceOffset,
    scratch: Vec<u8>,
    dirty: bool,
    context: DirectiveContext,
    punctuator: Option<Punctuator>,
    options: PpLexerOptions,
}

impl<'a> PpLexer<'a> {
    /// Start at the first source byte after a possible BOM.
    ///
    /// # Errors
    /// Returns a lexical error if the first physical character cannot be decoded.
    pub fn new(source: &'a SourceBuffer) -> Result<Self, SourceError> {
        Self::with_options(source, PpLexerOptions::default())
    }

    /// Start a lexer using typed source and dialect options.
    ///
    /// # Errors
    /// Returns a lexical error if the first physical character cannot be decoded.
    pub fn with_options(
        source: &'a SourceBuffer,
        options: PpLexerOptions,
    ) -> Result<Self, SourceError> {
        let reader = TranslationReader::new(source, !options.token_text);
        let current = reader.logical(source.begin())?;
        Ok(Self {
            source,
            reader,
            current,
            begin: current.begin,
            clean_end: current.begin,
            end: current.begin,
            scratch: Vec::new(),
            dirty: false,
            context: if options.header_names_on_request {
                DirectiveContext::Other
            } else {
                DirectiveContext::LineStart
            },
            punctuator: None,
            options,
        })
    }

    /// Permit a header-name as the next non-whitespace token.
    pub fn request_header_name(&mut self) {
        self.context = DirectiveContext::AfterInclude;
    }

    /// Produce one token, performing only the lookahead needed to classify it.
    ///
    /// # Errors
    /// Returns the first lexical error; the partially scanned token is not emitted.
    pub fn next_token(&mut self) -> Result<PpToken<'_>, SourceError> {
        let kind = self.scan_next()?;
        Ok(self.token(kind))
    }

    /// Scan without borrowing the current spelling, permitting bounded lookahead.
    pub(super) fn next_non_whitespace_kind(&mut self) -> Result<PpTokenKind, SourceError> {
        loop {
            let kind = self.scan_next()?;
            if !matches!(kind, PpTokenKind::WhitespaceSequence | PpTokenKind::NewLine) {
                return Ok(kind);
            }
        }
    }

    fn scan_next(&mut self) -> Result<PpTokenKind, SourceError> {
        self.begin = self.current.begin;
        self.clean_end = self.begin;
        self.end = self.begin;
        self.dirty = false;
        self.punctuator = None;
        let kind = self.scan()?;
        self.update_context(kind);
        Ok(kind)
    }

    pub(crate) fn token(&self, kind: PpTokenKind) -> PpToken<'_> {
        let spelling = if matches!(
            kind,
            PpTokenKind::WhitespaceSequence | PpTokenKind::NewLine | PpTokenKind::EndOfFile
        ) {
            &[]
        } else {
            self.spelling()
        };
        PpToken {
            kind,
            name: None,
            // The frozen producer adds the base in target uint32 arithmetic.
            begin: SourceOffset(self.options.location_base.wrapping_add(self.begin.0)),
            end: SourceOffset(self.options.location_base.wrapping_add(self.end.0)),
            spelling,
            punctuator: self.punctuator,
        }
    }

    fn scan(&mut self) -> Result<PpTokenKind, SourceError> {
        let c = self.current.code;
        if c == END_OF_SOURCE {
            return Ok(PpTokenKind::EndOfFile);
        }
        if c == 0x0a {
            self.skip(self.current)?;
            return Ok(PpTokenKind::NewLine);
        }
        if horizontal(c) || self.at_comment_start()? {
            self.whitespace()?;
            return Ok(PpTokenKind::WhitespaceSequence);
        }
        if self.context == DirectiveContext::AfterInclude
            && matches!(c, 0x3c | 0x22)
            && self.header_name()?
        {
            return Ok(PpTokenKind::HeaderName);
        }
        if identifier_start(c) {
            return self.identifier_or_literal();
        }
        if digit(c) || (c == 0x2e && digit(self.after(self.current)?.code)) {
            return self.number();
        }
        if c == 0x27 {
            return self.quoted_literal(
                PpTokenKind::CharacterLiteral,
                PpTokenKind::UserDefinedCharacterLiteral,
            );
        }
        if c == 0x22 {
            return self.quoted_literal(
                PpTokenKind::StringLiteral,
                PpTokenKind::UserDefinedStringLiteral,
            );
        }
        if c == 0x5c {
            return self.backslash();
        }
        if self.punctuator()? {
            return Ok(PpTokenKind::PreprocessingOpOrPunc);
        }
        self.take(self.current)?;
        Ok(PpTokenKind::NonWhitespaceCharacter)
    }

    fn spelling(&self) -> &[u8] {
        if self.dirty {
            // Windows-1252 mapping can expand a source image past 4 GiB.
            // The frozen token producer exposes its length as uint32_t,
            // including when prefix classification reads the current text.
            &self.scratch[..token_spelling_length(self.scratch.len())]
        } else {
            &self.source.bytes()[self.begin.index()..self.clean_end.index()]
        }
    }

    fn make_dirty(&mut self) {
        if self.dirty {
            return;
        }
        self.scratch.clear();
        self.scratch
            .extend_from_slice(&self.source.bytes()[self.begin.index()..self.clean_end.index()]);
        self.dirty = true;
    }

    fn append(&mut self, c: SourceChar) -> Result<(), SourceError> {
        let scalar = char::from_u32(c.code)
            .ok_or_else(|| SourceError::new(c.begin, "invalid universal character value"))?;
        if !self.dirty
            && c.begin == self.clean_end
            && (c.end.0 - c.begin.0) as usize == scalar.len_utf8()
        {
            self.clean_end = c.end;
        } else {
            self.make_dirty();
            append_utf8(&mut self.scratch, scalar);
        }
        self.end = c.end;
        Ok(())
    }

    fn take(&mut self, c: SourceChar) -> Result<(), SourceError> {
        self.append(c)?;
        self.current = self.after(c)?;
        Ok(())
    }
    fn skip(&mut self, c: SourceChar) -> Result<(), SourceError> {
        self.end = c.end;
        self.current = self.after(c)?;
        Ok(())
    }
    fn after(&self, c: SourceChar) -> Result<SourceChar, SourceError> {
        self.reader.logical(c.end)
    }

    fn update_context(&mut self, kind: PpTokenKind) {
        if self.options.header_names_on_request {
            if kind != PpTokenKind::WhitespaceSequence {
                self.context = DirectiveContext::Other;
            }
            return;
        }
        self.context = match kind {
            PpTokenKind::WhitespaceSequence => self.context,
            PpTokenKind::NewLine => DirectiveContext::LineStart,
            PpTokenKind::PreprocessingOpOrPunc
                if self.context == DirectiveContext::LineStart
                    && self.punctuator == Some(Punctuator::Hash) =>
            {
                DirectiveContext::AfterHash
            }
            PpTokenKind::Identifier
                if self.context == DirectiveContext::AfterHash
                    && matches!(self.spelling(), b"include" | b"include_next") =>
            {
                DirectiveContext::AfterInclude
            }
            _ => DirectiveContext::Other,
        };
    }

    fn at_comment_start(&self) -> Result<bool, SourceError> {
        Ok(self.current.code == 0x2f && matches!(self.after(self.current)?.code, 0x2f | 0x2a))
    }

    fn whitespace(&mut self) -> Result<(), SourceError> {
        loop {
            if horizontal(self.current.code) {
                self.skip(self.current)?;
                continue;
            }
            if self.current.code != 0x2f {
                return Ok(());
            }
            let second = self.after(self.current)?;
            match second.code {
                0x2f => {
                    self.skip(second)?;
                    while !matches!(self.current.code, 0x0a | END_OF_SOURCE) {
                        self.skip(self.current)?;
                    }
                }
                0x2a => {
                    self.skip(second)?;
                    self.block_comment()?;
                }
                _ => return Ok(()),
            }
        }
    }

    fn block_comment(&mut self) -> Result<(), SourceError> {
        loop {
            if self.current.code == END_OF_SOURCE {
                return Err(SourceError::new(self.begin, "unterminated block comment"));
            }
            let star = self.current.code == 0x2a;
            self.skip(self.current)?;
            if star && self.current.code == 0x2f {
                self.skip(self.current)?;
                return Ok(());
            }
        }
    }

    fn universal_name(&self, backslash: SourceChar) -> Result<Option<SourceChar>, SourceError> {
        let mut c = self.after(backslash)?;
        let digits = match c.code {
            0x75 => 4,
            0x55 => 8,
            _ => return Ok(None),
        };
        let mut code = 0u32;
        for _ in 0..digits {
            c = self.after(c)?;
            let Some(value) = hex_value(c.code) else {
                return Ok(None);
            };
            // Exactly eight hex digits fit in the target's 32-bit UCN accumulator.
            code = (code << 4) | value;
        }
        Ok(Some(SourceChar {
            code,
            begin: backslash.begin,
            end: c.end,
        }))
    }

    fn outside_literal(c: SourceChar) -> Result<SourceChar, SourceError> {
        if char::from_u32(c.code).is_none() || control(c.code) || basic(c.code) {
            return Err(SourceError::new(
                c.begin,
                "invalid universal character value",
            ));
        }
        Ok(c)
    }

    fn identifier_char(&mut self, start: bool) -> Result<bool, SourceError> {
        let c = if self.current.code == 0x5c {
            let Some(c) = self.universal_name(self.current)? else {
                return Ok(false);
            };
            Self::outside_literal(c)?
        } else {
            self.current
        };
        if !(if start {
            identifier_start(c.code)
        } else {
            identifier_continue(c.code)
        }) {
            return Ok(false);
        }
        self.take(c)?;
        Ok(true)
    }

    fn identifier_or_literal(&mut self) -> Result<PpTokenKind, SourceError> {
        self.take(self.current)?;
        while self.identifier_char(false)? {}
        if self.current.code == 0x22 {
            if matches!(self.spelling(), b"u8" | b"u" | b"U" | b"L") {
                return self.quoted_literal(
                    PpTokenKind::StringLiteral,
                    PpTokenKind::UserDefinedStringLiteral,
                );
            }
            if matches!(self.spelling(), b"R" | b"u8R" | b"uR" | b"UR" | b"LR") {
                return self.raw_literal();
            }
        } else if self.current.code == 0x27 && matches!(self.spelling(), b"u" | b"U" | b"L") {
            return self.quoted_literal(
                PpTokenKind::CharacterLiteral,
                PpTokenKind::UserDefinedCharacterLiteral,
            );
        }
        self.punctuator = punctuators::identifier_operator(self.spelling());
        Ok(if self.punctuator.is_some() {
            PpTokenKind::PreprocessingOpOrPunc
        } else {
            PpTokenKind::Identifier
        })
    }

    fn backslash(&mut self) -> Result<PpTokenKind, SourceError> {
        let Some(c) = self.universal_name(self.current)? else {
            self.take(self.current)?;
            return Ok(PpTokenKind::NonWhitespaceCharacter);
        };
        let c = Self::outside_literal(c)?;
        self.take(c)?;
        if !identifier_start(c.code) {
            return Ok(PpTokenKind::NonWhitespaceCharacter);
        }
        while self.identifier_char(false)? {}
        Ok(PpTokenKind::Identifier)
    }

    fn number(&mut self) -> Result<PpTokenKind, SourceError> {
        if self.current.code == 0x2e {
            self.take(self.current)?;
        }
        self.take(self.current)?;
        loop {
            if matches!(self.current.code, 0x65 | 0x45)
                || (self.options.binary_exponents && matches!(self.current.code, 0x70 | 0x50))
            {
                let sign = self.after(self.current)?;
                self.take(self.current)?;
                if matches!(sign.code, 0x2b | 0x2d) {
                    self.take(sign)?;
                }
            } else if self.current.code == 0x2e {
                self.take(self.current)?;
            } else if !self.identifier_char(false)? {
                return Ok(PpTokenKind::PpNumber);
            }
        }
    }
}

fn token_spelling_length(length: usize) -> usize {
    // Deliberate target truncation; source offsets themselves remain checked.
    length & u32::MAX as usize
}

#[cfg(test)]
mod tests;
