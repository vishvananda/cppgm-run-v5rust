// Translated from cppgm++.cpp::report_types_stats. See NOTICE.
//! Scalar census of existing semantic stores; no syntax or graph is retained.
#[derive(Clone, Copy, Default)]
pub(crate) struct Statistics {
    #[cfg(feature = "telemetry")]
    pub scopes: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub bindings: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub entities: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub types: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub names: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub lookups: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub member_cache: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub arena_bytes: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub work: super::Work,
}
