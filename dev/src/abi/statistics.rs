//! Reader and encoder work observations, translated from `abi_stats.h`. See NOTICE.

#[derive(Default)]
pub struct Statistics {
    pub source_files: crate::support::telemetry::Counter<usize>,
    pub source_bytes: crate::support::telemetry::Counter<usize>,
    pub cases: crate::support::telemetry::Counter<usize>,
    pub records: crate::support::telemetry::Counter<usize>,
    pub definitions: crate::support::telemetry::Counter<usize>,
    pub peak_graph_nodes: crate::support::telemetry::Counter<usize>,
    pub peak_graph_bytes: crate::support::telemetry::Counter<usize>,
    pub substitution_lookups: crate::support::telemetry::Counter<usize>,
    pub substitution_hits: crate::support::telemetry::Counter<usize>,
    pub substitution_entries: crate::support::telemetry::Counter<usize>,
    pub entity_references: crate::support::telemetry::Counter<usize>,
    pub output_bytes: crate::support::telemetry::Counter<usize>,
}
impl Statistics {
    /// Emit the inherited field names and order, with actual Rust storage costs.
    ///
    /// # Errors
    /// Returns a statistics sink error.
    #[cfg(feature = "telemetry")]
    pub fn write(
        &self,
        output: &mut impl std::io::Write,
        mangle_ms: f64,
        peak_rss_kb: u64,
    ) -> std::io::Result<()> {
        writeln!(
            output,
            "abimangle stats: source_files={} source_bytes={} cases={} records={} definitions={} peak_graph_nodes={} peak_graph_bytes={} substitution_lookups={} substitution_hits={} substitution_entries={} entity_references={} output_bytes={} mangle_ms={} peak_rss_kb={}",
            self.source_files,
            self.source_bytes,
            self.cases,
            self.records,
            self.definitions,
            self.peak_graph_nodes,
            self.peak_graph_bytes,
            self.substitution_lookups,
            self.substitution_hits,
            self.substitution_entries,
            self.entity_references,
            self.output_bytes,
            mangle_ms,
            peak_rss_kb
        )
    }
    /// # Errors
    /// Returns the unavailable-statistics diagnostic.
    #[cfg(not(feature = "telemetry"))]
    pub fn write(
        &self,
        output: &mut impl std::io::Write,
        mangle_ms: f64,
        peak_rss_kb: u64,
    ) -> std::io::Result<()> {
        let _ = (self, output, mangle_ms, peak_rss_kb);
        Err(std::io::Error::other(
            crate::support::telemetry::UNAVAILABLE,
        ))
    }
}
