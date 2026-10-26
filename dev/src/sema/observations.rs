// Translated from cppgm++.cpp::report_types_stats. See NOTICE.
//! Scalar census of existing semantic stores; no syntax or graph is retained.
#[derive(Clone, Copy)]
pub(crate) struct Statistics {
    pub scopes: usize,
    pub bindings: usize,
    pub entities: usize,
    pub types: usize,
    pub names: usize,
    pub lookups: usize,
    pub member_cache: usize,
    pub arena_bytes: usize,
    pub work: super::Work,
}
