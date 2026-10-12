//! Demand-driven phase-7 conversion of typed preprocessing tokens.
pub mod floating;
pub mod identifiers;
pub mod number;
pub mod pp_lexer;
pub(crate) mod strings;
pub use strings::TextError;
pub mod text;
pub mod vocabulary;

use crate::support::source::{SourceBuffer, SourceError, SourceOffset};
use crate::types::FundamentalType;
use floating::{FloatError, FloatingScanner, ScalarBytes};
use number::{NumberDialect, NumberLiteral, classify_number};
use pp_lexer::{PpLexer, PpTokenKind};
use vocabulary::{Keyword, SimpleToken};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LiteralForm {
    Integer,
    Floating,
    Character,
    String,
}

impl LiteralForm {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Floating => "floating",
            Self::Character => "character",
            Self::String => "string",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostTokenKind {
    Invalid,
    Identifier,
    Simple(SimpleToken),
    Literal {
        form: LiteralForm,
        ty: FundamentalType,
        data: ScalarBytes,
    },
    UserDefinedNumber {
        form: LiteralForm,
        suffix_begin: usize,
    },
    Character {
        ty: FundamentalType,
        data: ScalarBytes,
        multicharacter: bool,
        suffix_begin: Option<usize>,
    },
    String {
        ty: FundamentalType,
        elements: u32,
        suffix: Option<(usize, usize)>,
    },
    EndOfFile,
}

/// Spelling borrows the lexer image, translation buffer or current string sequence.
pub struct PostToken<'a> {
    pub kind: PostTokenKind,
    pub name: Option<identifiers::NameId>,
    pub begin: SourceOffset,
    pub end: SourceOffset,
    pub source: &'a [u8],
    /// Sequence-owned little-endian array units, including the terminator.
    pub data: &'a [u8],
    /// Text suffix borrows its owner independently of the exposed source length.
    pub suffix: &'a [u8],
}

#[derive(Debug)]
pub enum PostTokenError<E = SourceError> {
    Source(E),
    Numeric(FloatError),
    Text(TextError),
}

/// Bare-lexer input preserves null identifier identity, as the C++ tool does.
/// One lexer lookahead and one maximal string sequence bound token retention.
pub struct PostTokenizer<I> {
    input: I,
    dialect: NumberDialect,
    floating: FloatingScanner,
    strings: strings::StringSequence,
    lookahead: Option<PpTokenKind>,
    split_suffix: Vec<u8>,
    split_span: Option<(SourceOffset, SourceOffset)>,
    after_operator: bool,
}

/// A typed pull source retaining its current spelling until the next demand.
pub trait PostTokenInput {
    type Error;
    /// # Errors
    /// Propagates the input phase's structured failure.
    fn next_non_whitespace_kind(&mut self) -> Result<PpTokenKind, Self::Error>;
    fn token(&self, kind: PpTokenKind) -> pp_lexer::PpToken<'_>;
    /// Intern a literal-operator suffix in input sources that own name identity.
    ///
    /// # Errors
    /// Returns the input owner's identifier storage limit.
    fn intern_identifier(
        &mut self,
        _text: &[u8],
    ) -> Result<Option<identifiers::NameId>, Self::Error> {
        Ok(None)
    }
}

impl PostTokenInput for PpLexer<'_> {
    type Error = SourceError;
    fn next_non_whitespace_kind(&mut self) -> Result<PpTokenKind, SourceError> {
        self.next_non_whitespace_kind()
    }
    fn token(&self, kind: PpTokenKind) -> pp_lexer::PpToken<'_> {
        self.token(kind)
    }
}

impl<'a> PostTokenizer<PpLexer<'a>> {
    /// Construct a source-backed pull tokenizer with explicit literal dialect.
    ///
    /// # Errors
    /// Returns the first source decoding error.
    pub fn new(source: &'a SourceBuffer, dialect: NumberDialect) -> Result<Self, SourceError> {
        Ok(Self::from_input(PpLexer::new(source)?, dialect))
    }
}

impl<I: PostTokenInput> PostTokenizer<I> {
    pub(crate) fn input(&self) -> &I {
        &self.input
    }
    #[must_use]
    pub fn from_input(input: I, dialect: NumberDialect) -> Self {
        Self {
            input,
            dialect,
            floating: FloatingScanner::default(),
            strings: strings::StringSequence::default(),
            lookahead: None,
            split_suffix: Vec::new(),
            split_span: None,
            after_operator: false,
        }
    }

    /// Convert exactly one token on demand, retaining typed literal facts.
    ///
    /// # Errors
    /// Returns lexical errors or numeric/text storage limits.
    pub fn next_token(&mut self) -> Result<PostToken<'_>, PostTokenError<I::Error>> {
        if let Some((begin, end)) = self.split_span.take() {
            self.strings.clear();
            let kind = identifier_kind(&self.split_suffix);
            let name = self
                .input
                .intern_identifier(&self.split_suffix)
                .map_err(PostTokenError::Source)?;
            self.after_operator = is_operator(kind);
            return Ok(PostToken {
                kind,
                // The parser also needs the interned identity of adjacent
                // keyword suffixes in literal-operator names (CWG 1473).
                name,
                begin,
                end,
                source: &self.split_suffix,
                data: &[],
                suffix: &[],
            });
        }
        // Release the preceding sequence contents after its last consumer.
        self.strings.clear();
        self.split_suffix.clear();
        let pp_kind = if let Some(kind) = self.lookahead.take() {
            kind
        } else {
            self.input
                .next_non_whitespace_kind()
                .map_err(PostTokenError::Source)?
        };
        if matches!(
            pp_kind,
            PpTokenKind::StringLiteral | PpTokenKind::UserDefinedStringLiteral
        ) {
            return self.next_string(pp_kind);
        }
        let pp = self.input.token(pp_kind);
        let kind = match pp.kind {
            PpTokenKind::Identifier => identifier_kind(pp.spelling),
            PpTokenKind::PreprocessingOpOrPunc => pp
                .punctuator
                .and_then(SimpleToken::punctuator)
                .map_or(PostTokenKind::Invalid, PostTokenKind::Simple),
            PpTokenKind::PpNumber => match classify_number(pp.spelling, self.dialect) {
                NumberLiteral::Invalid => PostTokenKind::Invalid,
                NumberLiteral::Integer { ty, value } => PostTokenKind::Literal {
                    form: LiteralForm::Integer,
                    ty,
                    data: ScalarBytes::integer(ty, value),
                },
                NumberLiteral::Floating { ty, value_end } => self
                    .floating
                    .scan(&pp.spelling[..value_end], ty)
                    .map_err(PostTokenError::Numeric)?
                    .map_or(PostTokenKind::Invalid, |data| PostTokenKind::Literal {
                        form: LiteralForm::Floating,
                        ty,
                        data,
                    }),
                NumberLiteral::UserDefined {
                    floating,
                    suffix_begin,
                } => PostTokenKind::UserDefinedNumber {
                    form: if floating {
                        LiteralForm::Floating
                    } else {
                        LiteralForm::Integer
                    },
                    suffix_begin,
                },
            },
            PpTokenKind::CharacterLiteral | PpTokenKind::UserDefinedCharacterLiteral => {
                text::evaluate_character(pp.spelling).map_or(PostTokenKind::Invalid, |value| {
                    PostTokenKind::Character {
                        ty: value.ty,
                        data: ScalarBytes::integer(value.ty, u64::from(value.code_point)),
                        multicharacter: value.multicharacter,
                        suffix_begin: (value.suffix_begin != pp.spelling.len())
                            .then_some(value.suffix_begin),
                    }
                })
            }
            PpTokenKind::StringLiteral | PpTokenKind::UserDefinedStringLiteral => {
                PostTokenKind::Invalid
            }
            PpTokenKind::EndOfFile => PostTokenKind::EndOfFile,
            PpTokenKind::HeaderName
            | PpTokenKind::NonWhitespaceCharacter
            | PpTokenKind::WhitespaceSequence
            | PpTokenKind::NewLine => PostTokenKind::Invalid,
        };
        self.after_operator = is_operator(kind);
        Ok(PostToken {
            kind,
            name: if kind == PostTokenKind::Identifier {
                pp.name
            } else {
                None
            },
            begin: pp.begin,
            end: pp.end,
            source: pp.spelling,
            data: &[],
            suffix: &[],
        })
    }

    fn next_string(
        &mut self,
        first_kind: PpTokenKind,
    ) -> Result<PostToken<'_>, PostTokenError<I::Error>> {
        let first = self.input.token(first_kind);
        let begin;
        let mut end = first.end;
        let split = self.after_operator
            && first_kind == PpTokenKind::UserDefinedStringLiteral
            && first.spelling.len() >= 3
            && first.spelling.starts_with(b"\"\"");
        if split {
            begin = first.begin;
            self.strings
                .add(&first.spelling[..2])
                .map_err(PostTokenError::Text)?;
            self.split_suffix.extend_from_slice(&first.spelling[2..]);
            end = SourceOffset(begin.0.wrapping_add(2));
            self.split_span = Some((end, first.end));
        } else {
            self.strings
                .add(first.spelling)
                .map_err(PostTokenError::Text)?;
            loop {
                let kind = self
                    .input
                    .next_non_whitespace_kind()
                    .map_err(PostTokenError::Source)?;
                if !matches!(
                    kind,
                    PpTokenKind::StringLiteral | PpTokenKind::UserDefinedStringLiteral
                ) {
                    // The frozen bare lexer overwrites the PPToken referenced
                    // as `first` during lookahead. Its final first.begin is
                    // therefore this token's begin, even beyond the run's end.
                    // Preserve that published fact without aliasing storage.
                    begin = self.input.token(kind).begin;
                    self.lookahead = Some(kind);
                    break;
                }
                let pp = self.input.token(kind);
                self.strings
                    .add(pp.spelling)
                    .map_err(PostTokenError::Text)?;
                end = pp.end;
            }
        }
        let kind = self
            .strings
            .concatenate()
            .map_err(PostTokenError::Text)?
            .map_or(PostTokenKind::Invalid, |value| PostTokenKind::String {
                ty: value.ty,
                elements: value.elements,
                suffix: value.suffix.map(|range| (range.start, range.end)),
            });
        let suffix = match kind {
            PostTokenKind::String {
                suffix: Some((begin, end)),
                ..
            } => self.strings.suffix(begin..end),
            _ => &[],
        };
        self.after_operator = false;
        Ok(PostToken {
            kind,
            name: None,
            begin,
            end,
            source: self.strings.source(),
            data: self.strings.data(),
            suffix,
        })
    }
}

fn identifier_kind(text: &[u8]) -> PostTokenKind {
    SimpleToken::identifier(text).map_or(PostTokenKind::Identifier, PostTokenKind::Simple)
}

fn is_operator(kind: PostTokenKind) -> bool {
    kind == PostTokenKind::Simple(SimpleToken::keyword(Keyword::Operator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translated_numeric_text_retains_physical_spans_and_typed_values() {
        let source = SourceBuffer::new(b"1\\\n23 0x80000000 _\\u03c0".to_vec()).unwrap();
        let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
        let token = tokens.next_token().unwrap();
        assert_eq!((token.begin, token.end), (SourceOffset(0), SourceOffset(5)));
        assert_eq!(token.source, b"123");
        let PostTokenKind::Literal { form, ty, data } = token.kind else {
            panic!("expected typed integer");
        };
        assert_eq!(form, LiteralForm::Integer);
        assert_eq!(ty, FundamentalType::Int);
        assert_eq!(data.as_slice(), 123u32.to_le_bytes());
        let token = tokens.next_token().unwrap();
        let PostTokenKind::Literal { ty, data, .. } = token.kind else {
            panic!("expected typed nondecimal integer");
        };
        assert_eq!(ty, FundamentalType::UnsignedInt);
        assert_eq!(data.as_slice(), 0x8000_0000u32.to_le_bytes());
        let token = tokens.next_token().unwrap();
        assert_eq!(token.kind, PostTokenKind::Identifier);
        assert_eq!(token.source, "_π".as_bytes());
        assert_eq!(tokens.next_token().unwrap().kind, PostTokenKind::EndOfFile);
        assert_eq!(tokens.next_token().unwrap().kind, PostTokenKind::EndOfFile);
    }

    #[test]
    fn lexical_failure_precedes_literal_conversion() {
        let source = SourceBuffer::new(b"1.0 \"\\q\"".to_vec()).unwrap();
        let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
        assert!(matches!(
            tokens.next_token().unwrap().kind,
            PostTokenKind::Literal {
                form: LiteralForm::Floating,
                ..
            }
        ));
        let Err(PostTokenError::Source(error)) = tokens.next_token() else {
            panic!("expected lexical failure");
        };
        assert_eq!(source.format_error(&error), "1:6: invalid escape sequence");
    }

    #[test]
    fn string_lookahead_keeps_translated_spelling_and_physical_spans() {
        let source =
            SourceBuffer::new(b"u\"\\xD800\" \"\\U00010000\"_x _\\u03c0 7".to_vec()).unwrap();
        let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
        let token = tokens.next_token().unwrap();
        assert!(matches!(
            token.kind,
            PostTokenKind::String {
                ty: FundamentalType::Char16T,
                elements: 4,
                suffix: Some(_),
            }
        ));
        assert_eq!(token.source, "u\"\\xD800\" \"𐀀\"_x".as_bytes());
        assert_eq!(token.data, [0, 0xd8, 0, 0xd8, 0, 0xdc, 0, 0]);
        assert_eq!(
            (token.begin, token.end),
            (SourceOffset(25), SourceOffset(24))
        );
        let token = tokens.next_token().unwrap();
        assert_eq!(token.source, "_π".as_bytes());
        assert_eq!(token.kind, PostTokenKind::Identifier);
        assert_eq!(
            (token.begin, token.end),
            (SourceOffset(25), SourceOffset(32))
        );
        assert!(matches!(
            tokens.next_token().unwrap().kind,
            PostTokenKind::Literal { .. }
        ));
    }

    #[test]
    fn literal_operator_split_preserves_earliest_suffix_span_and_keyword_identity() {
        let source = SourceBuffer::new(b"operator \"\"\\\noperator \"next\"".to_vec()).unwrap();
        let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
        assert!(is_operator(tokens.next_token().unwrap().kind));
        let token = tokens.next_token().unwrap();
        assert_eq!(token.source, b"\"\"");
        assert_eq!(token.data, [0]);
        assert_eq!(
            (token.begin, token.end),
            (SourceOffset(9), SourceOffset(11))
        );
        let token = tokens.next_token().unwrap();
        assert!(is_operator(token.kind));
        assert_eq!(token.source, b"operator");
        assert_eq!(
            (token.begin, token.end),
            (SourceOffset(11), SourceOffset(21))
        );
        assert_eq!(tokens.next_token().unwrap().data, b"next\0");
        assert_eq!(tokens.next_token().unwrap().kind, PostTokenKind::EndOfFile);
    }

    #[test]
    fn string_run_errors_precede_conversion_but_split_operators_do_not_pull_ahead() {
        for input in [b"1 \"ok\" /*".as_slice(), b"1 \"ok\" \"\\q\""] {
            let source = SourceBuffer::new(input.to_vec()).unwrap();
            let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
            assert!(matches!(
                tokens.next_token().unwrap().kind,
                PostTokenKind::Literal { .. }
            ));
            assert!(matches!(
                tokens.next_token(),
                Err(PostTokenError::Source(_))
            ));
        }
        let source = SourceBuffer::new(b"operator \"\"suffix /*".to_vec()).unwrap();
        let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
        assert!(is_operator(tokens.next_token().unwrap().kind));
        assert!(matches!(
            tokens.next_token().unwrap().kind,
            PostTokenKind::String { .. }
        ));
        assert_eq!(tokens.next_token().unwrap().source, b"suffix");
        assert!(matches!(
            tokens.next_token(),
            Err(PostTokenError::Source(_))
        ));
    }

    #[test]
    fn split_literal_operator_suffix_uses_all_simple_identifier_spellings() {
        for word in [
            "and", "or", "not", "bitand", "bitor", "xor", "compl", "and_eq", "or_eq", "xor_eq",
            "not_eq", "new", "delete",
        ] {
            let source = SourceBuffer::new(format!("operator \"\"{word}").into_bytes()).unwrap();
            let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
            assert!(is_operator(tokens.next_token().unwrap().kind));
            assert!(matches!(
                tokens.next_token().unwrap().kind,
                PostTokenKind::String { .. }
            ));
            assert!(matches!(
                tokens.next_token().unwrap().kind,
                PostTokenKind::Simple(_)
            ));
        }
    }

    #[test]
    fn bare_string_span_preserves_frozen_lookahead_overwrite() {
        for (input, begin, end, valid) in [
            (b"\"abc\" \"def\" 7".as_slice(), 12, 11, true),
            (b"\"abc\"", 6, 5, true),
            (b"u\"a\" U\"b\" 7", 10, 9, false),
        ] {
            let source = SourceBuffer::new(input.to_vec()).unwrap();
            let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11).unwrap();
            let token = tokens.next_token().unwrap();
            assert_eq!(
                (token.begin, token.end),
                (SourceOffset(begin), SourceOffset(end))
            );
            assert_eq!(matches!(token.kind, PostTokenKind::String { .. }), valid);
            let token = tokens.next_token().unwrap();
            assert_eq!(token.begin, SourceOffset(begin));
            if token.kind == PostTokenKind::EndOfFile {
                let token = tokens.next_token().unwrap();
                assert_eq!(
                    (token.begin, token.end),
                    (SourceOffset(begin), SourceOffset(begin))
                );
            } else {
                assert_eq!(token.source, b"7");
            }
        }
    }
}
