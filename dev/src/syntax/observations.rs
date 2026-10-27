// Translated from syntax/parser_state.cpp::counters. See NOTICE.
//! Owned observations of the existing parse, without revisiting its grammar.
use super::SyntaxUnit;
#[cfg(feature = "telemetry")]
use crate::preprocess::counters::PreprocessorCounters;

#[derive(Clone, Copy, Default)]
pub(crate) struct Statistics {
    #[cfg(feature = "telemetry")]
    pub preprocessor: PreprocessorCounters,
    #[cfg(feature = "telemetry")]
    pub tokens: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub peak_window: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub checkpoints: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub rewinds: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub deferred_bodies: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub deferred_tokens: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub deferred_lambdas: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub peak_run_bytes: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub lookups: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub bindings: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub kept_searches: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub peak_nesting: crate::support::telemetry::Counter<u32>,
    #[cfg(feature = "telemetry")]
    pub arena_bytes: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub spelling_bytes: crate::support::telemetry::Counter<usize>,
}
impl SyntaxUnit<'_> {
    #[cfg(feature = "telemetry")]
    pub(crate) fn statistics(&self) -> Statistics {
        let parser = &self.parser;
        let cursor = &parser.cursor;
        let (lookups, bindings, kept_searches) = parser.scopes.observations();
        Statistics {
            preprocessor: cursor.post.input().counters(),
            tokens: cursor.tokens_read,
            peak_window: cursor.peak_window,
            checkpoints: parser.checkpoints,
            rewinds: parser.rewinds,
            deferred_bodies: parser.deferred_bodies,
            deferred_tokens: parser.deferred_tokens,
            deferred_lambdas: parser.deferred_lambdas,
            peak_run_bytes: cursor
                .peak_run_tokens
                .saturating_mul(std::mem::size_of::<super::cursor::Token>()),
            lookups,
            bindings,
            kept_searches,
            peak_nesting: parser.peak_nesting,
            arena_bytes: parser
                .tree
                .reserved_bytes()
                .saturating_add(cursor.spellings.reserved_bytes()),
            spelling_bytes: cursor.spellings.spelling_bytes(),
        }
    }
    #[cfg(not(feature = "telemetry"))]
    pub(crate) fn statistics(&self) -> Statistics {
        let _ = self;
        Statistics::default()
    }
}
