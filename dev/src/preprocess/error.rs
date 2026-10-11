//! Structured preprocessing failures; diagnostics are rendered at the tool boundary.
use crate::support::source::{SourceError, SourceOffset};

#[derive(Debug)]
pub enum PreprocessError {
    Source(SourceError),
    Directive {
        location: SourceOffset,
        message: &'static str,
    },
    NotImplemented(&'static str),
    Limit(&'static str),
    Invariant(&'static str),
}
impl PreprocessError {
    pub(crate) fn at(location: SourceOffset, message: &'static str) -> Self {
        Self::Directive { location, message }
    }
}
impl From<SourceError> for PreprocessError {
    fn from(error: SourceError) -> Self {
        Self::Source(error)
    }
}
