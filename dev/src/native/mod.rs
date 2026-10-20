//! Typed native boundary and standalone compilation ordering. See NOTICE.
mod mir_globals;
mod startup;

use crate::lowir::{Error, model::Program};
use std::{io::Write, path::PathBuf};

#[derive(Default)]
pub(crate) struct Options {
    pub(crate) output: PathBuf,
    pub(crate) machine_ir: PathBuf,
    pub(crate) optimization_level: u8,
    pub(crate) stats: bool,
}

pub(crate) fn compile(program: &Program, options: &Options) -> Result<Statistics, Error> {
    let start = std::time::Instant::now();
    let startup = startup::Startup::find(program)?;
    if !options.output.as_os_str().is_empty() && startup.entry.is_none() {
        return Err(Error::Diagnostic(
            "no entry function for an executable".into(),
        ));
    }
    let mut mir = if options.machine_ir.as_os_str().is_empty() {
        None
    } else {
        Some(std::io::BufWriter::new(
            std::fs::File::create(&options.machine_ir)
                .map_err(|_| Error::argument("cannot write ", options.machine_ir.as_os_str()))?,
        ))
    };
    // Data and startup are views of the same validated unit. Function selection
    // will append one function at a time here, releasing its MIR before the next.
    if let Some(out) = &mut mir {
        let result = startup
            .write(program, out)
            .and_then(|()| mir_globals::validate_storage(program))
            .and_then(|()| mir_globals::write(program, out));
        match result {
            Err(Error::Output(_)) => {
                return Err(Error::argument(
                    "cannot write ",
                    options.machine_ir.as_os_str(),
                ));
            }
            result => result?,
        }
    } else {
        mir_globals::validate_storage(program)?;
    }
    if program
        .functions()
        .iter()
        .any(|function| function.body.is_some())
    {
        if let Some(out) = &mut mir {
            out.flush()
                .map_err(|_| Error::argument("cannot write ", options.machine_ir.as_os_str()))?;
        }
        return Err(Error::NotImplemented(
            "native function selection and emission",
        ));
    }
    if let Some(out) = &mut mir {
        out.flush()
            .map_err(|_| Error::argument("cannot write ", options.machine_ir.as_os_str()))?;
    }
    Ok(Statistics {
        total_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

pub(crate) struct Statistics {
    total_ms: f64,
}
impl Statistics {
    pub(crate) fn write(&self, out: &mut impl Write, peak_rss: u64) -> std::io::Result<()> {
        // The supported declarations/data-only lane runs no selector or encoder.
        // Deterministic counters will be accumulated by those owners as ported.
        writeln!(
            out,
            "lowir2native_stats functions=0 lowir_instructions=0 mir_instructions=0 spills=0 homes=0 lazy_addresses=0 folded_loads=0 fused_compares=0 promoted_slots=0 dead_slots=0 global_registers=0 liveness_steps=0 hull_fallbacks=0 edge_blocks=0 threaded_blocks=0 layout_branches=0 strlen_prefixes=0 dynamic_copies=0 jump_tables=0 rotated_loops=0 loop_splits=0 home_splits=0 ordered_frames=0 dead_frame_stores=0 native_returns=0 short_branches=0 long_branches=0 fixups=0 scratch_carried_reloads=0 code_bytes=0 select_ms=0 encode_ms=0 total_ms={} peak_rss_kb={peak_rss}",
            self.total_ms
        )
    }
}
