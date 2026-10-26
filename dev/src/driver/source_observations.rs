// Translated from cppgm++.cpp::report_ast_stats. See NOTICE.
//! Source-view observations are inert after their translation unit ends.
use std::io::Write;

/// Work and resource observations from one parsed source view.
#[derive(Default)]
pub struct SourceObservations {
    pub(crate) syntax: Option<crate::syntax::observations::Statistics>,
    pub(crate) parse_ms: f64,
    pub(crate) print_ms: f64,
}
impl SourceObservations {
    /// Write the source view's collected observations without retaining its tree.
    /// # Errors
    /// Returns a diagnostic-sink failure.
    pub fn write(&self, out: &mut impl Write, path: &[u8], peak_rss: u64) -> std::io::Result<()> {
        let Some(s) = self.syntax else {
            return Ok(());
        };
        out.write_all(b"cppgm++ stats ")?;
        out.write_all(path)?;
        writeln!(
            out,
            ": source_bytes={} pp_tokens={} invocations={} tokens={} peak_window={} checkpoints={} rewinds={} deferred_bodies={} deferred_tokens={} deferred_lambdas={} peak_run_bytes={} lookups={} bindings={} kept_searches={} peak_nesting={} syntax_arena_bytes={} spelling_bytes={} pp_arena_bytes={} parse_ms={} print_ms={} peak_rss_kb={peak_rss}",
            s.preprocessor.source_bytes,
            s.preprocessor.source_tokens,
            s.preprocessor.expansion.invocations,
            s.tokens,
            s.peak_window,
            s.checkpoints,
            s.rewinds,
            s.deferred_bodies,
            s.deferred_tokens,
            s.deferred_lambdas,
            s.peak_run_bytes,
            s.lookups,
            s.bindings,
            s.kept_searches,
            s.peak_nesting,
            s.arena_bytes,
            s.spelling_bytes,
            s.preprocessor.arena_bytes,
            self.parse_ms,
            self.print_ms
        )
    }
}
