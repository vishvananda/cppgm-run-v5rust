//! Typed native boundary and standalone compilation ordering. See NOTICE.
mod abi;
mod address_facts;
mod aggregate;
mod atomic;
mod builtins;
mod bulk;
mod call_facts;
mod call_selection;
mod condition;
mod condition_mir;
mod condition_selection;
mod control_flow;
mod control_selection;
mod conversion;
mod conversion_selection;
mod divide_sequences;
mod division;
mod elf_reader;
mod elf_writer;
mod emission;
mod exception;
mod executable;
mod float_transport;
mod floating_mir;
mod floating_selection;
mod frame_carries;
mod frame_layout;
mod global_data;
mod global_placement;
mod image;
mod integer_facts;
mod jump_thunk;
mod linker;
mod memory;
mod memory_selection;
mod mir;
mod mir_cleanup;
mod mir_globals;
mod object;
mod object_compile;
pub(crate) use elf_reader::read as read_object;
pub(crate) use elf_writer::write as write_object;
pub(crate) use linker::link as link_objects;
pub(crate) use object::Object;
pub(crate) use object_compile::ObjectBuilder;
pub(crate) use object_compile::compile as compile_object;
mod parallel_transfer;
mod placement;
mod runtime;
mod runtime_link;
mod scalar_mir;
mod scalar_selection;
mod slot_analysis;
mod stack;
mod startup;
mod value_facts;
mod value_selection;
mod variadic;
mod wide_compare;
mod wide_mir;
mod wide_selection;
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

pub(crate) fn compile(program: &mut Program, options: &Options) -> Result<Statistics, Error> {
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
            let emitted = selected.emit(&mut image, Some(runtime.symbols()))?;
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
        match image.write(&options.output, entry, options.permissions.as_ref()) {
            Ok(()) => {}
            Err(image::WriteError::Other(error)) => return Err(error),
            Err(image::WriteError::Undefined) => {
                drop(image);
                let source = program
                    .first_source()
                    .map(|p| p.as_os_str().as_encoded_bytes().to_vec())
                    .unwrap_or_default();
                let linked = object_compile::compile(program, source, options.optimization_level)
                    .and_then(|object| {
                        linker::link(vec![object], &options.output, options.permissions.as_ref())
                    });
                if let Err(error) = linked {
                    let _ = std::fs::remove_file(&options.output);
                    return Err(error);
                }
                stats.runtime_linked = true;
            }
        }
    }
    stats.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(stats)
}

#[derive(Default)]
pub(crate) struct Statistics {
    runtime_linked: bool,
    total_ms: f64,
    select_ms: f64,
    encode_ms: f64,
    functions: usize,
    spills: usize,
    homes: usize,
    lazy_addresses: usize,
    folded_loads: usize,
    fused_compares: usize,
    dynamic_copies: usize,
    lowir_instructions: usize,
    mir_instructions: usize,
    dead_slots: usize,
    promoted_slots: usize,
    native_returns: usize,
    global_registers: usize,
    liveness_steps: usize,
    hull_fallbacks: usize,
    edge_blocks: usize,
    short_branches: usize,
    long_branches: usize,
    fixups: usize,
    code_bytes: usize,
    carried_reloads: usize,
    ordered_frames: usize,
}
impl Statistics {
    fn selected(&mut self, function: &mir::Function) -> Result<(), Error> {
        for (counter, increment) in [
            (&mut self.functions, 1),
            (&mut self.ordered_frames, function.ordered_frames),
            (&mut self.global_registers, function.global_registers),
            (&mut self.liveness_steps, function.liveness_steps),
            (&mut self.hull_fallbacks, function.hull_fallbacks),
            (&mut self.edge_blocks, function.edge_blocks),
            (&mut self.spills, function.spills),
            (&mut self.homes, function.homes),
            (&mut self.lazy_addresses, function.lazy_addresses),
            (&mut self.folded_loads, function.folded_loads),
            (&mut self.fused_compares, function.fused_compares),
            (&mut self.dynamic_copies, function.dynamic_copies),
            (&mut self.lowir_instructions, function.lowir_instructions),
            (
                &mut self.mir_instructions,
                function
                    .instructions
                    .len()
                    .checked_add(function.removed_instructions)
                    .ok_or(Error::Limit)?,
            ),
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
            (&mut self.short_branches, emission.short_branches),
            (&mut self.long_branches, emission.long_branches),
            (&mut self.fixups, emission.fixups),
            (&mut self.code_bytes, emission.code_bytes),
            (&mut self.carried_reloads, emission.carried_reloads),
        ] {
            *counter = counter.checked_add(increment).ok_or(Error::Limit)?;
        }
        Ok(())
    }
    pub(crate) fn write(&self, out: &mut impl Write, peak_rss: u64) -> std::io::Result<()> {
        if self.runtime_linked {
            return out.write_all(b"\n");
        }
        // Counters follow the selected function plans and final native encoding.
        writeln!(
            out,
            "lowir2native_stats functions={} lowir_instructions={} mir_instructions={} spills={} homes={} lazy_addresses={} folded_loads={} fused_compares={} promoted_slots={} dead_slots={} global_registers={} liveness_steps={} hull_fallbacks={} edge_blocks={} threaded_blocks=0 layout_branches=0 strlen_prefixes=0 dynamic_copies={} jump_tables=0 rotated_loops=0 loop_splits=0 home_splits=0 ordered_frames={} dead_frame_stores=0 native_returns={} short_branches={} long_branches={} fixups={} scratch_carried_reloads={} code_bytes={} select_ms={} encode_ms={} total_ms={} peak_rss_kb={peak_rss}",
            self.functions,
            self.lowir_instructions,
            self.mir_instructions,
            self.spills,
            self.homes,
            self.lazy_addresses,
            self.folded_loads,
            self.fused_compares,
            self.promoted_slots,
            self.dead_slots,
            self.global_registers,
            self.liveness_steps,
            self.hull_fallbacks,
            self.edge_blocks,
            self.dynamic_copies,
            self.ordered_frames,
            self.native_returns,
            self.short_branches,
            self.long_branches,
            self.fixups,
            self.carried_reloads,
            self.code_bytes,
            self.select_ms,
            self.encode_ms,
            self.total_ms
        )
    }
}
