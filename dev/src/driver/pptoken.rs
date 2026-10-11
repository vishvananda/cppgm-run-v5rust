// (C) 2013 CPPGM Foundation. All Rights Reserved. www.cppgm.org
// Adapted from DebugPPTokenStream; see NOTICE for attribution.
//! Debug rendering adapter; text is output, never transport between phases.
use crate::preprocess::{
    lexer::{PpLexer, PpTokenKind},
    source::SourceBuffer,
};
use std::io::Write;

use super::TokenToolError;

/// Dump the on-demand preprocessing tokens to the requested output.
///
/// # Errors
/// Returns the first lexical or output error, preserving prior emitted tokens.
pub fn emit_pptokens(
    source: &SourceBuffer,
    output: &mut (impl Write + ?Sized),
) -> Result<(), TokenToolError> {
    let mut lexer = PpLexer::new(source).map_err(TokenToolError::Source)?;
    loop {
        let token = lexer.next_token().map_err(TokenToolError::Source)?;
        if token.kind == PpTokenKind::EndOfFile {
            writeln!(output, "eof").map_err(TokenToolError::Output)?;
            return Ok(());
        }
        write!(output, "{} {} ", token.kind.name(), token.spelling.len())
            .map_err(TokenToolError::Output)?;
        output
            .write_all(token.spelling)
            .map_err(TokenToolError::Output)?;
        writeln!(output).map_err(TokenToolError::Output)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_error_locations_and_lookahead_preserve_output_prefix() {
        let source = SourceBuffer::new(b"\xef\xbb\xbfok\n  /*".to_vec()).unwrap();
        let mut output = Vec::new();
        let Err(TokenToolError::Source(error)) = emit_pptokens(&source, &mut output) else {
            panic!("expected lexical error");
        };
        assert_eq!(
            source.format_error(&error),
            "2:1: unterminated block comment"
        );
        assert_eq!(output, b"identifier 2 ok\nnew-line 0 \n");
        let source = SourceBuffer::new(b"ok +\xff".to_vec()).unwrap();
        let mut output = Vec::new();
        let Err(TokenToolError::Source(error)) = emit_pptokens(&source, &mut output) else {
            panic!("expected decoding error");
        };
        assert_eq!(
            source.format_error(&error),
            "1:5: invalid UTF-8 leading byte"
        );
        assert_eq!(output, b"identifier 2 ok\nwhitespace-sequence 0 \n");
    }
}
