// Translated from dev/preproc.cpp. See NOTICE.
//! Preprocessor text view over the shared typed phase-7 converter.
use crate::lex::{PostTokenError, PostTokenKind, PostTokenizer, number::NumberDialect};
use crate::preprocess::{error::PreprocessError, options::PredefinedValues, unit::Preprocessor};
use crate::support::source::{SourceBuffer, SourceError};
use std::io::Write;

#[derive(Debug)]
pub enum PreprocToolError {
    Input(crate::support::source::SourceReadError),
    Initialization(PreprocessError),
    Preprocess {
        error: PreprocessError,
        diagnostic: Vec<u8>,
    },
    InvalidToken,
    Output(std::io::Error),
}
/// # Errors
/// Returns the first demanded preprocessing or post-token failure.
pub fn emit_preprocessed(
    source: &SourceBuffer,
    name: &[u8],
    values: &PredefinedValues,
    output: &mut (impl Write + ?Sized),
    counters: &mut Option<crate::preprocess::counters::PreprocessorCounters>,
) -> Result<(), PreprocToolError> {
    let input = Preprocessor::with_options(
        source,
        name,
        crate::preprocess::files::FileServices {
            identity: super::file_identity::query,
            read: super::file_identity::read,
        },
        values,
    )
    .map_err(PreprocToolError::Initialization)?;
    let mut tokens = PostTokenizer::from_input(input, NumberDialect::Cxx11);
    let result = write_tokens(&mut tokens, output);
    *counters = if matches!(&result, Err(PreprocToolError::Input(_))) {
        None
    } else {
        Some(tokens.input().counters())
    };
    result
}
/// Emit one hosted translation unit through the shared token stream.
/// # Errors
/// Returns the first demanded preprocessing, literal or output failure.
pub(super) fn emit_hosted(
    source: &SourceBuffer,
    name: &[u8],
    values: &PredefinedValues,
    options: &crate::preprocess::options::DriverOptions,
    output: &mut (impl Write + ?Sized),
) -> Result<(), PreprocToolError> {
    let mut input = Preprocessor::with_driver_options(
        source,
        name,
        crate::preprocess::files::FileServices {
            identity: super::file_identity::query,
            read: super::file_identity::read,
        },
        values,
        options,
    )
    .map_err(PreprocToolError::Initialization)?;
    input.set_number_options(crate::lex::number::NumberOptions {
        dialect: NumberDialect::Gnu,
        binary_exponents: true,
    });
    write_tokens(
        &mut PostTokenizer::from_input(input, NumberDialect::GnuStream),
        output,
    )
}
fn write_tokens(
    tokens: &mut PostTokenizer<Preprocessor<'_>>,
    output: &mut (impl Write + ?Sized),
) -> Result<(), PreprocToolError> {
    loop {
        let token = match tokens.next_token() {
            Ok(token) => token,
            Err(error) => {
                let error = match error {
                    PostTokenError::Source(PreprocessError::Read(error)) => {
                        return Err(PreprocToolError::Input(error));
                    }
                    PostTokenError::Source(error) => error,
                    PostTokenError::Numeric(_) => {
                        PreprocessError::Limit("numeric literal implementation limit")
                    }
                    PostTokenError::Text(_) => {
                        PreprocessError::Limit("text literal implementation limit")
                    }
                };
                let diagnostic = tokens
                    .input()
                    .describe(&error)
                    .map_err(PreprocToolError::Initialization)?;
                return Err(PreprocToolError::Preprocess { error, diagnostic });
            }
        };
        if token.kind == PostTokenKind::Invalid {
            return Err(PreprocToolError::InvalidToken);
        }
        super::posttoken::write_posttoken(&token, output).map_err(PreprocToolError::Output)?;
        if token.kind == PostTokenKind::EndOfFile {
            return Ok(());
        }
    }
}
/// Render the source location lazily at the file-tool boundary.
#[must_use]
pub fn describe(source: &SourceBuffer, error: &PreprocessError) -> String {
    match error {
        PreprocessError::Read(error) => error.to_string(),
        PreprocessError::Source(error) => source.format_error(error),
        PreprocessError::Directive { location, message } => {
            source.format_error(&SourceError::new(*location, message))
        }
        PreprocessError::Limit(message)
        | PreprocessError::Invariant(message)
        | PreprocessError::NotImplemented(message) => (*message).to_owned(),
    }
}

/// Like ostream, retain a failed write while later source diagnostics are demanded.
/// The buffer bounds text retention independently of translation-unit size.
pub(super) struct HostedOutput<W: Write> {
    writer: std::io::BufWriter<W>,
    failed: bool,
}
impl<W: Write> HostedOutput<W> {
    pub(super) fn new(writer: W) -> Self {
        Self {
            writer: std::io::BufWriter::with_capacity(1 << 16, writer),
            failed: false,
        }
    }
    pub(super) fn finish(self) -> std::io::Result<()> {
        if self.failed {
            Err(std::io::Error::other("preprocessing output failed"))
        } else {
            Ok(())
        }
    }
}
impl<W: Write> Write for HostedOutput<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if !self.failed && self.writer.write_all(bytes).is_err() {
            self.failed = true;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.writer.flush().is_err() {
            self.failed = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn failed_output_keeps_demanding_source_diagnostics() {
        let source = SourceBuffer::new(b"#error later source failure\n".to_vec()).unwrap();
        let mut output = HostedOutput::new(FailingWriter);
        output.write_all(&vec![b'x'; 1 << 17]).unwrap(); // fail before reading the source
        let result = emit_hosted(
            &source,
            b"probe",
            &PredefinedValues::default(),
            &crate::preprocess::options::DriverOptions::default(),
            &mut output,
        );
        assert!(
            matches!(result, Err(PreprocToolError::Preprocess { diagnostic, .. }) if diagnostic == b"probe:1:2: #error")
        );
        output.flush().unwrap();
        assert!(output.finish().is_err());
    }
}
