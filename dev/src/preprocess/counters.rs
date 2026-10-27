// Translated from preprocess/preprocessor.h and macro/macro_expander.h. See NOTICE.
//! Work observations belong to one translation unit and never influence decisions.

#[derive(Clone, Copy, Default)]
pub struct ExpansionCounters {
    pub invocations: crate::support::telemetry::Counter<u64>,
    pub produced_tokens: crate::support::telemetry::Counter<usize>,
    pub argument_tokens: crate::support::telemetry::Counter<usize>,
    pub argument_expansions: crate::support::telemetry::Counter<u64>,
    pub nested_collections: crate::support::telemetry::Counter<u64>,
    pub pastes: crate::support::telemetry::Counter<u64>,
    pub stringizes: crate::support::telemetry::Counter<u64>,
    pub raw_operands_hidden: crate::support::telemetry::Counter<u64>,
    pub peak_contexts: crate::support::telemetry::Counter<usize>,
    pub peak_context_tokens: crate::support::telemetry::Counter<usize>,
    pub peak_invocations: crate::support::telemetry::Counter<usize>,
}

#[derive(Clone, Copy, Default)]
pub struct PreprocessorCounters {
    pub source_tokens: crate::support::telemetry::Counter<u64>,
    pub directives: crate::support::telemetry::Counter<u64>,
    pub skipped_directives: crate::support::telemetry::Counter<u64>,
    pub includes: crate::support::telemetry::Counter<u64>,
    pub guarded_skips: crate::support::telemetry::Counter<u64>,
    pub files: crate::support::telemetry::Counter<usize>,
    pub source_bytes: crate::support::telemetry::Counter<usize>,
    pub identifiers: crate::support::telemetry::Counter<usize>,
    /// This implementation uses owned typed stores, not a bump arena.
    pub arena_bytes: crate::support::telemetry::Counter<usize>,
    pub expansion: ExpansionCounters,
}
