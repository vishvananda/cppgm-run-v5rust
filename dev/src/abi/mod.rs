//! Typed Itanium ABI graph and name construction. See NOTICE.

pub mod encoder;
mod fact_expressions;
mod fact_functions;
mod fact_types;
pub mod facts;
pub mod graph;
#[cfg(test)]
mod tests;
pub mod vocabulary;

/// A fact/encoding diagnostic retains its original bytes until tool output.
#[derive(Debug)]
pub enum Error {
    Diagnostic(Vec<u8>),
    NotImplemented(&'static str),
    StorageLimit,
}
impl Error {
    pub(crate) fn diagnostic(message: impl AsRef<[u8]>) -> Self {
        Self::Diagnostic(message.as_ref().to_vec())
    }
    pub(crate) fn word(prefix: &str, word: &[u8], suffix: &str) -> Self {
        let mut message = prefix.as_bytes().to_vec();
        message.extend_from_slice(word);
        message.extend_from_slice(suffix.as_bytes());
        Self::Diagnostic(message)
    }
}
