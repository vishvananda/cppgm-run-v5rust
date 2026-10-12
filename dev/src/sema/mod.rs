//! Typed declaration analysis on the single PA5 syntax tree. See NOTICE.
mod analyzer;
mod anonymous_types;
mod constants;
mod data_members;
mod declarations;
mod declarators;
mod dump;
mod enumerations;
mod exception_operands;
mod exceptions;
mod expression_keys;
mod expressions;
mod graph;
mod integral;
mod layout;
mod lookup;
mod name_keys;
mod queries;
mod special_members;
mod specifiers;
mod statements;
mod template_exceptions;
mod templates;
mod types;
use crate::support::source::SourceOffset;
#[derive(Debug)]
pub(crate) enum Error {
    Diagnostic {
        location: SourceOffset,
        message: &'static str,
    },
    NonConstant {
        location: SourceOffset,
        message: &'static str,
    },
    Missing(&'static str),
    Limit(&'static str),
    Invariant(&'static str),
}
impl Error {
    pub(super) fn at(location: SourceOffset, message: &'static str) -> Self {
        Self::Diagnostic { location, message }
    }
}
pub(crate) use analyzer::Analyzer;

fn record<T: Copy>(facts: &mut Vec<Option<T>>, index: usize, fact: T) {
    if index >= facts.len() {
        facts.resize(index + 1, None);
    }
    facts[index] = Some(fact);
}

#[cfg(test)]
mod tests;
