//! Demand-driven phase-7 conversion of typed preprocessing tokens.
pub mod floating;
pub mod number;
pub mod pp_lexer;
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
}

impl LiteralForm {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Floating => "floating",
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
    EndOfFile,
}

/// Spelling borrows the lexer image or its reusable translation buffer.
pub struct PostToken<'a> {
    pub kind: PostTokenKind,
    pub begin: SourceOffset,
    pub end: SourceOffset,
    pub source: &'a [u8],
}

#[derive(Debug)]
pub enum PostTokenError {
    Source(SourceError),
    Numeric(FloatError),
    NotImplemented(&'static str),
}

/// Bare-lexer input preserves null identifier identity, as the C++ tool does.
/// Interned preprocessor input and text sequences belong to later groups.
pub struct PostTokenizer<'a> {
    input: PpLexer<'a>,
    dialect: NumberDialect,
    floating: FloatingScanner,
}

impl<'a> PostTokenizer<'a> {
    /// Construct a source-backed pull tokenizer with explicit literal dialect.
    ///
    /// # Errors
    /// Returns the first source decoding error.
    pub fn new(source: &'a SourceBuffer, dialect: NumberDialect) -> Result<Self, SourceError> {
        Ok(Self {
            input: PpLexer::new(source)?,
            dialect,
            floating: FloatingScanner::default(),
        })
    }

    /// Convert exactly one token on demand, retaining typed literal facts.
    ///
    /// # Errors
    /// Returns lexical/numeric limits or the explicit unported text boundary.
    pub fn next_token(&mut self) -> Result<PostToken<'_>, PostTokenError> {
        let pp = self
            .input
            .next_non_whitespace()
            .map_err(PostTokenError::Source)?;
        let kind = match pp.kind {
            PpTokenKind::Identifier => Keyword::find(pp.spelling)
                .map_or(PostTokenKind::Identifier, |word| {
                    PostTokenKind::Simple(SimpleToken::keyword(word))
                }),
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
            PpTokenKind::CharacterLiteral
            | PpTokenKind::UserDefinedCharacterLiteral
            | PpTokenKind::StringLiteral
            | PpTokenKind::UserDefinedStringLiteral => {
                return Err(PostTokenError::NotImplemented(
                    "text literal decoding and concatenation",
                ));
            }
            PpTokenKind::EndOfFile => PostTokenKind::EndOfFile,
            PpTokenKind::HeaderName
            | PpTokenKind::NonWhitespaceCharacter
            | PpTokenKind::WhitespaceSequence
            | PpTokenKind::NewLine => PostTokenKind::Invalid,
        };
        Ok(PostToken {
            kind,
            begin: pp.begin,
            end: pp.end,
            source: pp.spelling,
        })
    }
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
    fn lexical_failure_precedes_an_unported_literal_conversion() {
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
}
