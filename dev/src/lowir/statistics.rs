//! Model census and phase observations for the `LowIR` tool. See NOTICE.
/// Completed builder work, independent of whether a consumer retains bodies.
#[derive(Clone, Copy, Default)]
pub(crate) struct Construction {
    #[cfg(feature = "telemetry")]
    pub(crate) functions: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub(crate) instructions: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub(crate) blocks: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    pub(crate) slots: crate::support::telemetry::Counter<usize>,
}
impl Construction {
    #[cfg(feature = "telemetry")]
    pub(super) fn record(&mut self, body: &super::model::Body) {
        self.functions += 1;
        self.instructions += body
            .blocks()
            .iter()
            .map(|block| block.instructions.len())
            .sum::<usize>();
        // The function's entry is implicit in FunctionLowering's constructor.
        self.blocks += body.blocks().len().saturating_sub(1);
        self.slots += body.slots().len();
    }
    #[cfg(not(feature = "telemetry"))]
    pub(super) fn record(&mut self, body: &super::model::Body) {
        let _ = self;
        let _ = body;
    }
}
/// Counts read from the completed typed model, with independently measured times.
#[derive(Default)]
pub struct Statistics {
    pub source_bytes: crate::support::telemetry::Counter<usize>,
    pub symbols: crate::support::telemetry::Counter<usize>,
    pub functions: crate::support::telemetry::Counter<usize>,
    pub globals: crate::support::telemetry::Counter<usize>,
    pub data_items: crate::support::telemetry::Counter<usize>,
    pub signatures: crate::support::telemetry::Counter<usize>,
    pub blocks: crate::support::telemetry::Counter<usize>,
    pub instructions: crate::support::telemetry::Counter<usize>,
    pub temps: crate::support::telemetry::Counter<usize>,
    pub literals: crate::support::telemetry::Counter<usize>,
    pub extra_operands: crate::support::telemetry::Counter<usize>,
    pub names: crate::support::telemetry::Counter<usize>,
    pub name_arena_bytes: crate::support::telemetry::Counter<usize>,
    pub parse_ms: crate::support::telemetry::Counter<f64>,
    pub validate_ms: crate::support::telemetry::Counter<f64>,
    pub build_ms: crate::support::telemetry::Counter<f64>,
    pub write_ms: crate::support::telemetry::Counter<f64>,
}
impl Statistics {
    /// Emit the inherited statistics field names and order.
    ///
    /// # Errors
    /// Returns an output error if the statistics sink cannot be written.
    #[cfg(feature = "telemetry")]
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
    /// # Errors
    /// Returns the unavailable-statistics diagnostic.
    #[cfg(not(feature = "telemetry"))]
    pub fn write(&self, output: &mut impl std::io::Write, peak_rss_kb: u64) -> std::io::Result<()> {
        let _ = (self, output, peak_rss_kb);
        Err(std::io::Error::other(
            crate::support::telemetry::UNAVAILABLE,
        ))
    }
}
