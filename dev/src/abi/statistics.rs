//! Reader and encoder work observations, translated from `abi_stats.h`. See NOTICE.

#[derive(Default)]
pub struct Statistics {
    pub source_files: usize,
    pub source_bytes: usize,
    pub cases: usize,
    pub records: usize,
    pub definitions: usize,
    pub peak_graph_nodes: usize,
    pub peak_graph_bytes: usize,
    pub substitution_lookups: usize,
    pub substitution_hits: usize,
    pub substitution_entries: usize,
    pub entity_references: usize,
    pub output_bytes: usize,
}
impl Statistics {
    /// Emit the inherited field names and order, with actual Rust storage costs.
    ///
    /// # Errors
    /// Returns a statistics sink error.
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
}
