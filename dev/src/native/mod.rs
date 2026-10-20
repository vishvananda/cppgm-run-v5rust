//! Typed native boundary and standalone compilation ordering. See NOTICE.
mod abi;
mod builtins;
mod executable;
mod global_data;
mod image;
mod mir;
mod mir_globals;
mod runtime;
mod slot_analysis;
mod startup;
mod value_selection;
mod x86;

use crate::lowir::{Error, model::Program};
use std::{io::Write, path::PathBuf};

#[derive(Default)]
pub(crate) struct Options {
    pub(crate) output: PathBuf,
    pub(crate) machine_ir: PathBuf,
    pub(crate) optimization_level: u8,
    pub(crate) stats: bool,
    pub(crate) permissions: Option<std::fs::Permissions>,
}

pub(crate) fn compile(program: &Program, options: &Options) -> Result<Statistics, Error> {
    let start = std::time::Instant::now();
    let startup = startup::Startup::find(program)?;
    if !options.output.as_os_str().is_empty() && startup.entry.is_none() {
        return Err(Error::Diagnostic(
            "no entry function for an executable".into(),
        ));
    }
    let mut image = image::Image::new(program.symbols().len());
    let runtime = runtime::Runtime::declare(&mut image)?;
    let mut stats = Statistics::default();
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
            .and_then(|()| global_data::emit(program, &mut image))
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
        global_data::emit(program, &mut image)?;
    }
    for function in program.functions().iter().filter(|f| f.body.is_some()) {
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let phase = options.stats.then(std::time::Instant::now);
        let selected =
            value_selection::select(program, symbol, function, options.optimization_level)?;
        if let Some(phase) = phase {
            stats.select_ms += phase.elapsed().as_secs_f64() * 1000.0;
        }
        stats.selected(&selected)?;
        if let Some(out) = &mut mir {
            selected.write(program, out).map_err(|error| match error {
                Error::Output(_) => {
                    Error::argument("cannot write ", options.machine_ir.as_os_str())
                }
                error => error,
            })?;
        }
        if !options.output.as_os_str().is_empty() {
            let phase = options.stats.then(std::time::Instant::now);
            let emitted = selected.emit(&mut image)?;
            if let Some(phase) = phase {
                stats.encode_ms += phase.elapsed().as_secs_f64() * 1000.0;
            }
            stats.encoded(emitted)?;
        }
        // Selected MIR and its ABI/alias scratch end before the next function.
    }
    if let Some(out) = &mut mir {
        out.flush()
            .map_err(|_| Error::argument("cannot write ", options.machine_ir.as_os_str()))?;
    }
    if !options.output.as_os_str().is_empty() {
        let entry = runtime.emit(&mut image, program, &startup)?;
        image.write(&options.output, entry, options.permissions.as_ref())?;
    }
    stats.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(stats)
}

#[derive(Default)]
pub(crate) struct Statistics {
    total_ms: f64,
    select_ms: f64,
    encode_ms: f64,
    functions: usize,
    lowir_instructions: usize,
    mir_instructions: usize,
    dead_slots: usize,
    promoted_slots: usize,
    native_returns: usize,
    fixups: usize,
    code_bytes: usize,
}
impl Statistics {
    fn selected(&mut self, function: &mir::Function) -> Result<(), Error> {
        for (counter, increment) in [
            (&mut self.functions, 1),
            (&mut self.lowir_instructions, function.lowir_instructions),
            (&mut self.mir_instructions, function.instructions.len()),
            (&mut self.dead_slots, function.dead_slots),
            (&mut self.promoted_slots, function.promoted_slots),
        ] {
            *counter = counter.checked_add(increment).ok_or(Error::Limit)?;
        }
        Ok(())
    }
    fn encoded(&mut self, emission: mir::Emission) -> Result<(), Error> {
        for (counter, increment) in [
            (&mut self.native_returns, emission.returns),
            (&mut self.fixups, emission.fixups),
            (&mut self.code_bytes, emission.code_bytes),
        ] {
            *counter = counter.checked_add(increment).ok_or(Error::Limit)?;
        }
        Ok(())
    }
    pub(crate) fn write(&self, out: &mut impl Write, peak_rss: u64) -> std::io::Result<()> {
        // Terminal selection uses aliases and local-slot elision; no spills,
        // cross-block placement or branch-layout passes run on these paths.
        writeln!(
            out,
            "lowir2native_stats functions={} lowir_instructions={} mir_instructions={} spills=0 homes=0 lazy_addresses=0 folded_loads=0 fused_compares=0 promoted_slots={} dead_slots={} global_registers=0 liveness_steps=0 hull_fallbacks=0 edge_blocks=0 threaded_blocks=0 layout_branches=0 strlen_prefixes=0 dynamic_copies=0 jump_tables=0 rotated_loops=0 loop_splits=0 home_splits=0 ordered_frames=0 dead_frame_stores=0 native_returns={} short_branches=0 long_branches=0 fixups={} scratch_carried_reloads=0 code_bytes={} select_ms={} encode_ms={} total_ms={} peak_rss_kb={peak_rss}",
            self.functions,
            self.lowir_instructions,
            self.mir_instructions,
            self.promoted_slots,
            self.dead_slots,
            self.native_returns,
            self.fixups,
            self.code_bytes,
            self.select_ms,
            self.encode_ms,
            self.total_ms
        )
    }
}
