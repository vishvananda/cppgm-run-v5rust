//! Typed declaration analysis on the single PA5 syntax tree. See NOTICE.
mod analyzer;
mod anonymous_types;
mod builtin_calls;
mod call_lookup;
mod constant_addresses;
mod constant_mutations;
mod constant_pointer_operations;
mod constants;
mod construction;
mod copy_initialization;
mod data_members;
mod declarations;
mod declarators;
mod default_arguments;
mod dump;
mod enumerations;
mod exception_operands;
mod exceptions;
mod expression_facts;
mod expression_keys;
mod expressions;
mod function_demands;
mod function_names;
mod function_resolution;
mod function_templates;
mod graph;
mod inheritance;
mod initialization;
mod integral;
mod jumps;
mod layout;
mod linkage;
mod lookup;
pub(crate) mod lowering;
mod member_expressions;
mod name_keys;
mod procedural_calls;
mod procedural_statements;
mod queries;
mod scalar_casts;
mod scalar_conversions;
mod scalar_expressions;
mod scalar_operators;
mod semantics_dump;
mod special_members;
mod specifiers;
mod statements;
mod static_arrays;
mod static_initialization;
mod template_exceptions;
mod templates;
mod throw_queries;
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
pub(crate) use graph::Graph;

fn record<T: Copy>(facts: &mut Vec<Option<T>>, index: usize, fact: T) {
    if index >= facts.len() {
        facts.resize(index + 1, None);
    }
    facts[index] = Some(fact);
}

#[cfg(test)]
mod tests;
