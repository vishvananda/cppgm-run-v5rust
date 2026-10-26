// Translated from syntax/parser_state.cpp::counters. See NOTICE.
//! Owned observations of the existing parse, without revisiting its grammar.
use super::SyntaxUnit;
use crate::preprocess::counters::PreprocessorCounters;

#[derive(Clone, Copy, Default)]
pub(crate) struct Statistics {
    pub preprocessor: PreprocessorCounters,
    pub tokens: u64,
    pub peak_window: usize,
    pub checkpoints: u64,
    pub rewinds: u64,
    pub deferred_bodies: u64,
    pub deferred_tokens: u64,
    pub deferred_lambdas: u64,
    pub peak_run_bytes: usize,
    pub lookups: u64,
    pub bindings: usize,
    pub kept_searches: usize,
    pub peak_nesting: u32,
    pub arena_bytes: usize,
    pub spelling_bytes: usize,
}
impl SyntaxUnit<'_> {
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
}
