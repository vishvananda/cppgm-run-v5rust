//! Model census and phase observations for the `LowIR` tool. See NOTICE.
/// Counts read from the completed typed model, with independently measured times.
#[derive(Default)]
pub struct Statistics {
    pub source_bytes: usize,
    pub symbols: usize,
    pub functions: usize,
    pub globals: usize,
    pub data_items: usize,
    pub signatures: usize,
    pub blocks: usize,
    pub instructions: usize,
    pub temps: usize,
    pub literals: usize,
    pub extra_operands: usize,
    pub names: usize,
    pub name_arena_bytes: usize,
    pub parse_ms: f64,
    pub validate_ms: f64,
    pub build_ms: f64,
    pub write_ms: f64,
}
impl Statistics {
    /// Emit the inherited statistics field names and order.
    ///
    /// # Errors
    /// Returns an output error if the statistics sink cannot be written.
    pub fn write(&self, output: &mut impl std::io::Write, peak_rss_kb: u64) -> std::io::Result<()> {
        writeln!(
            output,
            "lowir stats: source_bytes={} symbols={} functions={} globals={} data_items={} signatures={} blocks={} instructions={} temps={} literals={} extra_operands={} names={} name_arena_bytes={} parse_ms={} validate_ms={} build_ms={} write_ms={} peak_rss_kb={}",
            self.source_bytes,
            self.symbols,
            self.functions,
            self.globals,
            self.data_items,
            self.signatures,
            self.blocks,
            self.instructions,
            self.temps,
            self.literals,
            self.extra_operands,
            self.names,
            self.name_arena_bytes,
            self.parse_ms,
            self.validate_ms,
            self.build_ms,
            self.write_ms,
            peak_rss_kb
        )
    }
}
