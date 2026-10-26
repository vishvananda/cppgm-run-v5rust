// Translated from cppgm++.cpp::report_ast_stats. See NOTICE.
//! Source-view observations are inert after their translation unit ends.
use std::io::Write;

/// Work and resource observations from one parsed source view.
#[derive(Default)]
pub struct SourceObservations {
    pub(crate) view: Option<super::ast::EmitMode>,
    pub(crate) syntax: Option<crate::syntax::observations::Statistics>,
    pub(crate) semantic: Option<crate::sema::Statistics>,
    pub(crate) parse_ms: f64,
    pub(crate) analyze_ms: f64,
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
        if matches!(
            self.view,
            Some(super::ast::EmitMode::Types | super::ast::EmitMode::Semantics)
        ) {
            let Some(g) = self.semantic else {
                return Ok(());
            };
            out.write_all(b"cppgm++ stats ")?;
            out.write_all(path)?;
            writeln!(
                out,
                ": tokens={} scopes={} bindings={} entities={} types={} names={} lookups={} member_cache={} parser_kept_searches={} constant_evaluations={} constant_steps={} syntax_arena_bytes={} semantic_arena_bytes={} parse_ms={} analyze_ms={} print_ms={} peak_rss_kb={peak_rss}",
                s.tokens,
                g.scopes,
                g.bindings,
                g.entities,
                g.types,
                g.names,
                g.lookups,
                g.member_cache,
                s.kept_searches,
                g.work.constant_evaluations,
                g.work.constant_steps,
                s.arena_bytes,
                g.arena_bytes,
                self.parse_ms,
                self.analyze_ms,
                self.print_ms
            )?;
            if self.view == Some(super::ast::EmitMode::Semantics) {
                out.write_all(b"cppgm++ semantics ")?;
                out.write_all(path)?;
                let work = g.work;
                writeln!(
                    out,
                    ": expressions={} conversions={} resolutions={} candidates={} specializations={} demands={} override_steps={} virtual_slots={} instantiated_classes={} instantiated_functions={} instantiated_arguments={} instantiated_variables={} value_substitutions={} partial_candidates={} substitution_failures={} limited_lookups={} base_steps={}",
                    work.expressions,
                    work.conversions,
                    work.resolutions,
                    work.candidates,
                    work.specializations,
                    work.demands,
                    work.override_steps,
                    work.virtual_slots,
                    work.instantiated_classes,
                    work.instantiated_functions,
                    work.instantiated_arguments,
                    work.instantiated_variables,
                    work.value_substitutions,
                    work.partial_candidates,
                    work.substitution_failures,
                    work.limited_lookups,
                    work.base_steps
                )?;
            }
            return Ok(());
        }
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
