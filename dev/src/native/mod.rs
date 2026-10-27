//! Typed native boundary and standalone compilation ordering. See NOTICE.
mod abi;
mod address_facts;
mod address_fusion;
mod aggregate;
mod allocation_pressure;
mod allocation_ranges;
mod allocation_weights;
mod atomic;
mod block_placement;
mod branch_layout;
mod branch_tests;
mod builtins;
mod bulk;
mod call_facts;
mod call_selection;
mod catch_tables;
#[cfg(feature = "telemetry")]
mod census;
mod compilation_observations;
mod condition;
mod condition_mir;
mod condition_selection;
mod control_flow;
mod control_selection;
mod conversion;
mod conversion_selection;
mod decided_tests;
mod divide_sequences;
mod division;
mod elf_reader;
mod elf_writer;
mod emission;
mod exception;
mod executable;
mod flag_cleanup;
mod float_transport;
mod floating_mir;
mod floating_selection;
mod frame_carries;
mod frame_encoding;
mod frame_layout;
mod frame_lifetimes;
mod global_data;
mod global_placement;
mod handler_regions;
mod home_forwarding;
mod host_parameters;
mod host_unwind;
mod image;
mod integer_facts;
mod jump_thunk;
mod landing_pad;
mod link_statistics;
mod linker;
mod lsda;
mod memory;
mod memory_selection;
mod memory_updates;
mod mir;
mod mir_cleanup;
mod mir_globals;
mod mir_registers;
mod object;
mod object_compile;
mod object_storage;
pub(crate) use compilation_observations::{ObservationOptions, Observations};
mod region_flow;
mod register_allocation;
mod rotations;
mod shrink_region;
mod shrink_wrap;
mod split_placement;
mod store_coalescing;
mod unwind_rules;
pub(crate) use elf_reader::read as read_object;
#[cfg(test)]
pub(crate) use elf_writer::bytes as object_bytes;
pub(crate) use elf_writer::write as write_object;
pub(crate) use jump_thunk::recover as recover_jump_thunks;
pub(crate) use linker::link as link_objects;
pub(crate) use object::Object;
#[cfg(test)]
pub(crate) use object_compile::compile as compile_object;
pub(crate) use object_compile::compile_observed as compile_object_observed;
pub(crate) use object_compile::{ObjectBuilder, ObjectModel};
mod parallel_transfer;
mod placement;
mod return_slots;
mod runtime;
mod runtime_link;
mod scalar_mir;
mod scalar_selection;
mod slot_analysis;
mod stack;
mod startup;
mod switch_plan;
mod switch_selection;
mod symbol_access;
mod tail_calls;
mod value_facts;
mod value_selection;
mod variadic;
mod variadic_objects;
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
    let start = crate::support::telemetry::Clock::now();
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
        let phase = options.stats.then(crate::support::telemetry::Clock::now);
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
            let phase = options.stats.then(crate::support::telemetry::Clock::now);
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
                recover_jump_thunks(program, ObjectModel::Course);
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
    stats.total_ms = crate::support::telemetry::counter(start.elapsed().as_secs_f64() * 1000.0);
    Ok(stats)
}

#[derive(Default)]
pub(crate) struct Statistics {
    runtime_linked: bool,
    total_ms: crate::support::telemetry::Counter<f64>,
    select_ms: crate::support::telemetry::Counter<f64>,
    encode_ms: crate::support::telemetry::Counter<f64>,
    functions: crate::support::telemetry::Counter<usize>,
    spills: crate::support::telemetry::Counter<usize>,
    homes: crate::support::telemetry::Counter<usize>,
    lazy_addresses: crate::support::telemetry::Counter<usize>,
    folded_loads: crate::support::telemetry::Counter<usize>,
    fused_compares: crate::support::telemetry::Counter<usize>,
    strlen_prefixes: crate::support::telemetry::Counter<usize>,
    dynamic_copies: crate::support::telemetry::Counter<usize>,
    jump_tables: crate::support::telemetry::Counter<usize>,
    lowir_instructions: crate::support::telemetry::Counter<usize>,
    mir_instructions: crate::support::telemetry::Counter<usize>,
    dead_slots: crate::support::telemetry::Counter<usize>,
    promoted_slots: crate::support::telemetry::Counter<usize>,
    native_returns: crate::support::telemetry::Counter<usize>,
    global_registers: crate::support::telemetry::Counter<usize>,
    liveness_steps: crate::support::telemetry::Counter<usize>,
    hull_fallbacks: crate::support::telemetry::Counter<usize>,
    loop_splits: crate::support::telemetry::Counter<usize>,
    home_splits: crate::support::telemetry::Counter<usize>,
    edge_blocks: crate::support::telemetry::Counter<usize>,
    rotated_loops: crate::support::telemetry::Counter<usize>,
    threaded_blocks: crate::support::telemetry::Counter<usize>,
    layout_branches: crate::support::telemetry::Counter<usize>,
    short_branches: crate::support::telemetry::Counter<usize>,
    long_branches: crate::support::telemetry::Counter<usize>,
    fixups: crate::support::telemetry::Counter<usize>,
    code_bytes: crate::support::telemetry::Counter<usize>,
    carried_reloads: crate::support::telemetry::Counter<usize>,
    ordered_frames: crate::support::telemetry::Counter<usize>,
    dead_frame_stores: crate::support::telemetry::Counter<usize>,
}
impl Statistics {
    fn selected(&mut self, function: &mir::Function) -> Result<(), Error> {
        for (counter, increment) in [
            (&mut self.functions, crate::support::telemetry::counter(1)),
            (&mut self.rotated_loops, function.rotated_loops),
            (&mut self.threaded_blocks, function.threaded_blocks),
            (&mut self.layout_branches, function.layout_branches),
            (&mut self.ordered_frames, function.ordered_frames),
            (&mut self.dead_frame_stores, function.dead_frame_stores),
            (&mut self.global_registers, function.global_registers),
            (&mut self.liveness_steps, function.liveness_steps),
            (&mut self.hull_fallbacks, function.hull_fallbacks),
            (&mut self.loop_splits, function.loop_splits),
            (&mut self.home_splits, function.home_splits),
            (&mut self.edge_blocks, function.edge_blocks),
            (&mut self.spills, function.spills),
            (&mut self.homes, function.homes),
            (&mut self.lazy_addresses, function.lazy_addresses),
            (&mut self.folded_loads, function.folded_loads),
            (&mut self.fused_compares, function.fused_compares),
            (&mut self.strlen_prefixes, function.strlen_prefixes),
            (&mut self.dynamic_copies, function.dynamic_copies),
            (&mut self.jump_tables, function.jump_tables),
            (&mut self.lowir_instructions, function.lowir_instructions),
            (
                &mut self.mir_instructions,
                crate::support::telemetry::observe(|| {
                    function.blocks.iter().map(|b| b.range.len()).sum()
                }),
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

    #[cfg(feature = "telemetry")]
    pub(crate) fn write(&self, out: &mut impl Write, peak_rss: u64) -> std::io::Result<()> {
        if self.runtime_linked {
            return out.write_all(b"\n");
        }
        // Counters follow the selected function plans and final native encoding.
        writeln!(
            out,
            "lowir2native_stats functions={} lowir_instructions={} mir_instructions={} spills={} homes={} lazy_addresses={} folded_loads={} fused_compares={} promoted_slots={} dead_slots={} global_registers={} liveness_steps={} hull_fallbacks={} edge_blocks={} threaded_blocks={} layout_branches={} strlen_prefixes={} dynamic_copies={} jump_tables={} rotated_loops={} loop_splits={} home_splits={} ordered_frames={} dead_frame_stores={} native_returns={} short_branches={} long_branches={} fixups={} scratch_carried_reloads={} code_bytes={} select_ms={} encode_ms={} total_ms={} peak_rss_kb={peak_rss}",
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
            self.threaded_blocks,
            self.layout_branches,
            self.strlen_prefixes,
            self.dynamic_copies,
            self.jump_tables,
            self.rotated_loops,
            self.loop_splits,
            self.home_splits,
            self.ordered_frames,
            self.dead_frame_stores,
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
    #[cfg(not(feature = "telemetry"))]
    pub(crate) fn write(&self, out: &mut impl Write, peak_rss: u64) -> std::io::Result<()> {
        let _ = (self, out, peak_rss);
        Err(std::io::Error::other(
            crate::support::telemetry::UNAVAILABLE,
        ))
    }
}
