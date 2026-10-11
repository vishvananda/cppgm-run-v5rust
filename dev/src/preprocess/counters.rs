// Translated from preprocess/preprocessor.h and macro/macro_expander.h. See NOTICE.
//! Work observations belong to one translation unit and never influence decisions.

#[derive(Clone, Copy, Default)]
pub struct ExpansionCounters {
    pub invocations: u64,
    pub produced_tokens: usize,
    pub argument_tokens: usize,
    pub argument_expansions: u64,
    pub nested_collections: u64,
    pub pastes: u64,
    pub stringizes: u64,
    pub raw_operands_hidden: u64,
    pub peak_contexts: usize,
    pub peak_context_tokens: usize,
    pub peak_invocations: usize,
}

#[derive(Clone, Copy, Default)]
pub struct PreprocessorCounters {
    pub source_tokens: u64,
    pub directives: u64,
    pub skipped_directives: u64,
    pub includes: u64,
    pub guarded_skips: u64,
    pub files: usize,
    pub source_bytes: usize,
    pub identifiers: usize,
    /// This implementation uses owned typed stores, not a bump arena.
    pub arena_bytes: usize,
    pub expansion: ExpansionCounters,
}
