// Output format adapted from lex/post_token_writer.cpp. See NOTICE.
//! Text view of typed post-tokens; never a production transport format.
use super::TokenToolError;
use crate::lex::{
    LiteralForm, PostTokenError, PostTokenKind, PostTokenizer, number::NumberDialect,
};
use crate::preprocess::source::SourceBuffer;
use std::io::Write;

/// Render the token stream while preserving source failure/output ordering.
///
/// # Errors
/// Returns the first source, output, numeric-limit or unported-group error.
pub fn emit_posttokens(
    source: &SourceBuffer,
    output: &mut (impl Write + ?Sized),
) -> Result<(), TokenToolError> {
    // C++ constructs the lexer outside its located SourceError handler.
    let mut tokens = PostTokenizer::new(source, NumberDialect::Cxx11)
        .map_err(TokenToolError::SourceInitialization)?;
    loop {
        let token = tokens.next_token().map_err(|error| match error {
            PostTokenError::Source(source) => TokenToolError::Source(source),
            PostTokenError::Numeric(_) => {
                TokenToolError::Limit("numeric literal implementation limit")
            }
            PostTokenError::Text(_) => TokenToolError::Limit("text literal implementation limit"),
        })?;
        write_posttoken(&token, output).map_err(TokenToolError::Output)?;
        if token.kind == PostTokenKind::EndOfFile {
            return Ok(());
        }
    }
}

pub(crate) fn write_posttoken(
    token: &crate::lex::PostToken<'_>,
    output: &mut (impl Write + ?Sized),
) -> std::io::Result<()> {
    let result = match token.kind {
        PostTokenKind::Invalid => write_source(output, b"invalid ", token.source),
        PostTokenKind::Identifier => write_source(output, b"identifier ", token.source),
        PostTokenKind::Simple(kind) => write_source(output, b"simple ", token.source)
            .and_then(|()| write!(output, " {}", kind.name())),
        PostTokenKind::Literal { ty, data, .. } => write_source(output, b"literal ", token.source)
            .and_then(|()| write!(output, " {} ", ty.name()))
            .and_then(|()| write_hex(output, data.as_slice())),
        PostTokenKind::UserDefinedNumber { form, suffix_begin } => {
            write_source(output, b"user-defined-literal ", token.source)
                .and_then(|()| write_source(output, b" ", &token.source[suffix_begin..]))
                .and_then(|()| write!(output, " {} ", form.name()))
                .and_then(|()| output.write_all(&token.source[..suffix_begin]))
        }
        PostTokenKind::Character {
            ty,
            data,
            multicharacter,
            suffix_begin,
        } => {
            if multicharacter {
                write_source(output, b"invalid ", token.source)
            } else {
                write_text_prefix(
                    output,
                    token.source,
                    suffix_begin.map(|begin| &token.source[begin..]),
                    LiteralForm::Character,
                )
                .and_then(|()| write!(output, " {} ", ty.name()))
                .and_then(|()| write_hex(output, data.as_slice()))
            }
        }
        PostTokenKind::String {
            ty,
            elements,
            suffix,
        } => write_text_prefix(
            output,
            token.source,
            suffix.map(|_| token.suffix),
            LiteralForm::String,
        )
        .and_then(|()| write!(output, " array of {} {} ", elements, ty.name()))
        .and_then(|()| write_hex(output, token.data)),
        PostTokenKind::EndOfFile => output.write_all(b"eof"),
    };
    result?;
    writeln!(output)
}

fn write_source(
    output: &mut (impl Write + ?Sized),
    prefix: &[u8],
    source: &[u8],
) -> std::io::Result<()> {
    output.write_all(prefix)?;
    output.write_all(source)
}

fn write_hex(output: &mut (impl Write + ?Sized), bytes: &[u8]) -> std::io::Result<()> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in bytes {
        output.write_all(&[HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 15)]])?;
    }
    Ok(())
}

fn write_text_prefix(
    output: &mut (impl Write + ?Sized),
    source: &[u8],
    suffix: Option<&[u8]>,
    form: LiteralForm,
) -> std::io::Result<()> {
    if let Some(suffix) = suffix {
        write_source(output, b"user-defined-literal ", source)?;
        write_source(output, b" ", suffix)?;
        write!(output, " {}", form.name())
    } else {
        write_source(output, b"literal ", source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_failure_stays_distinct_from_stream_failure() {
        let source = SourceBuffer::new(b"\xef\xbb\xbf??/\n\xc0".to_vec()).unwrap();
        let mut output = Vec::new();
        let Err(TokenToolError::SourceInitialization(error)) =
            emit_posttokens(&source, &mut output)
        else {
            panic!("expected an unlocated construction failure");
        };
        assert_eq!(error.to_string(), "invalid UTF-8 leading byte");
        assert!(output.is_empty());

        let source = SourceBuffer::new(b"1\n\xc0".to_vec()).unwrap();
        let Err(TokenToolError::Source(error)) = emit_posttokens(&source, &mut output) else {
            panic!("expected a located stream failure");
        };
        assert_eq!(
            source.format_error(&error),
            "2:1: invalid UTF-8 leading byte"
        );
        assert_eq!(output, b"literal 1 int 01000000\n");
    }
}
