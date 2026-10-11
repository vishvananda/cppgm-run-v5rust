// Translated from dev/ppexpr.cpp. See NOTICE.
//! PA3 mock macro query and the exact buffered result view.
use super::TokenToolError;
use crate::lex::number::NumberDialect;
use crate::lex::pp_lexer::PpLexer;
use crate::preprocess::expression::{
    ExpressionError, ExpressionEvaluator, ExpressionStatus, MacroDefinitionQuery,
};
use crate::support::source::SourceBuffer;
use std::io::Write;

struct MockMacroDefinitions;

impl MacroDefinitionQuery for MockMacroDefinitions {
    fn is_defined(&self, spelling: &[u8]) -> bool {
        spelling.first().is_some_and(|byte| byte & 1 != 0)
    }
}

const FLUSH_SIZE: usize = 64 * 1024;

/// Render one result per nonempty logical line and the final EOF marker.
///
/// # Errors
/// Keeps prior results on a source failure; construction errors remain unlocated.
pub fn emit_expressions(
    source: &SourceBuffer,
    output: &mut (impl Write + ?Sized),
) -> Result<(), TokenToolError> {
    let mut lexer = PpLexer::new(source).map_err(TokenToolError::SourceInitialization)?;
    let macros = MockMacroDefinitions;
    let mut evaluator = ExpressionEvaluator::new(&macros, NumberDialect::Cxx11);
    let mut buffer = Vec::with_capacity(FLUSH_SIZE + 64);
    loop {
        let result = match evaluator.evaluate(&mut lexer) {
            Ok(result) => result,
            Err(error) => {
                flush(&mut buffer, output)?;
                return Err(match error {
                    ExpressionError::Input(error) => TokenToolError::Source(error),
                    ExpressionError::Invariant(message) => TokenToolError::Internal(message),
                });
            }
        };
        match result.status {
            ExpressionStatus::Empty => {}
            ExpressionStatus::Invalid => buffer.extend_from_slice(b"error\n"),
            ExpressionStatus::Valid(value) => {
                let negative = !value.is_unsigned && value.bits >> 63 != 0;
                let magnitude = if negative {
                    value.bits.wrapping_neg()
                } else {
                    value.bits
                };
                writeln!(
                    buffer,
                    "{}{magnitude}{}",
                    if negative { "-" } else { "" },
                    if value.is_unsigned { "u" } else { "" }
                )
                .map_err(TokenToolError::Output)?;
            }
        }
        if buffer.len() >= FLUSH_SIZE {
            flush(&mut buffer, output)?;
        }
        if result.end_of_file {
            break;
        }
    }
    buffer.extend_from_slice(b"eof\n");
    flush(&mut buffer, output)
}

fn flush(buffer: &mut Vec<u8>, output: &mut (impl Write + ?Sized)) -> Result<(), TokenToolError> {
    output.write_all(buffer).map_err(TokenToolError::Output)?;
    output.flush().map_err(TokenToolError::Output)?;
    buffer.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_failure_flushes_only_completed_lines_even_after_expression_error() {
        let source = SourceBuffer::new(b"2 + 3\n) bad\n7 + \"\\q\"\n".to_vec()).unwrap();
        let mut output = Vec::new();
        let Err(TokenToolError::Source(error)) = emit_expressions(&source, &mut output) else {
            panic!("expected a located source failure");
        };
        assert_eq!(output, b"5\nerror\n");
        assert_eq!(source.format_error(&error), "3:6: invalid escape sequence");

        let source = SourceBuffer::new(b"\xc0".to_vec()).unwrap();
        output.clear();
        assert!(matches!(
            emit_expressions(&source, &mut output),
            Err(TokenToolError::SourceInitialization(_))
        ));
        assert!(output.is_empty());
    }
}
