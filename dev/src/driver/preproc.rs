// Translated from dev/preproc.cpp. See NOTICE.
//! Preprocessor text view over the shared typed phase-7 converter.
use crate::lex::{PostTokenError, PostTokenKind, PostTokenizer, number::NumberDialect};
use crate::preprocess::{error::PreprocessError, unit::Preprocessor};
use crate::support::source::{SourceBuffer, SourceError};
use std::io::Write;

#[derive(Debug)]
pub enum PreprocToolError {
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
    output: &mut (impl Write + ?Sized),
) -> Result<(), PreprocToolError> {
    let input = Preprocessor::with_file_services(
        source,
        name,
        crate::preprocess::files::FileServices {
            identity: super::file_identity::query,
            read: super::file_identity::read,
        },
    )
    .map_err(PreprocToolError::Initialization)?;
    let mut tokens = PostTokenizer::from_input(input, NumberDialect::Cxx11);
    loop {
        let token = match tokens.next_token() {
            Ok(token) => token,
            Err(error) => {
                let error = match error {
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
        PreprocessError::Source(error) => source.format_error(error),
        PreprocessError::Directive { location, message } => {
            source.format_error(&SourceError::new(*location, message))
        }
        PreprocessError::Limit(message)
        | PreprocessError::Invariant(message)
        | PreprocessError::NotImplemented(message) => (*message).to_owned(),
    }
}
