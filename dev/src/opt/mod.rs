//! Typed unit optimization, translated from opt/optimizer.cpp. See NOTICE.
mod aggregate_slots;
mod algebra;
mod alias_facts;
mod argument_specialization;
mod body_expansion;
mod branch_ranges;
mod coalescing;
mod condition_placement;
mod control_flow;
mod dead_code;
mod definition_order;
mod diamond_reuse;
mod dominating_facts;
mod edge_demotion;
mod exception_regions;
mod field_expansion;
mod field_layout;
mod fill_versioning;
mod flow_cleanup;
mod full_unrolling;
mod global_contents;
mod handler_liveness;
mod indexed_stores;
mod induction;
mod inline_arguments;
mod inline_policy;
mod inline_shape;
mod inline_wave;
mod iteration_values;
mod jump_threading;
mod load_placement;
mod local_pipeline;
mod loop_copies;
mod loop_counters;
mod loop_fills;
mod loop_memory;
mod loop_motion;
mod loop_removal;
mod memory_calls;
mod memory_contents;
mod memory_forward;
mod natural_loops;
mod numeric_fold;
mod partial_inline;
mod pass_context;
mod phi_threading;
mod range_threading;
mod reload_rotation;
mod return_values;
mod returned_storage;
mod select_formation;
mod slot_copies;
mod slot_promotion;
mod sparse_constants;
mod statistics;
mod storage_arguments;
mod store_liveness;
mod switch_formation;
mod trip_counts;
mod unit_graph;
mod unroll_admission;
mod value_reuse;
mod verification;
mod wide_fills;
mod wide_integers;
use crate::lowir::{Error, model::Program};
pub use inline_policy::Limits;
pub use statistics::Statistics;
pub use verification::Verification;

pub(super) fn pure(kind: &crate::lowir::model::InstructionKind) -> bool {
    use crate::lowir::model::InstructionKind as I;
    matches!(
        kind,
        I::Const(..)
            | I::Copy(..)
            | I::Phi(..)
            | I::Addr(..)
            | I::Index(..)
            | I::Unary(..)
            | I::Binary(..)
            | I::Compare(..)
            | I::Select(..)
            | I::Convert(..)
    )
}

#[cfg(test)]
pub fn optimize(program: &mut Program, level: u8) -> Result<(), Error> {
    optimize_with_limits(program, level, Limits::default(), false)
}

#[cfg(test)]
pub fn optimize_with_limits(
    program: &mut Program,
    level: u8,
    limits: Limits,
    inline_thunk_targets: bool,
) -> Result<(), Error> {
    optimize_observed(
        program,
        level,
        limits,
        inline_thunk_targets,
        Verification::default(),
    )
    .map(|_| ())
}

pub fn optimize_observed(
    program: &mut Program,
    level: u8,
    limits: Limits,
    inline_thunk_targets: bool,
    verification: Verification<'_>,
) -> Result<Statistics, Error> {
    #[cfg(feature = "telemetry")]
    let start = crate::support::telemetry::Clock::now();
    let mut observations = Statistics::default();
    #[cfg(feature = "telemetry")]
    {
        (observations.instructions_in, observations.blocks_in) = statistics::size(program)?;
    }
    if level == 0 {
        #[cfg(feature = "telemetry")]
        observations.finish(program, start)?;
        return Ok(observations);
    }
    let mut original = vec![0; program.symbols().len()];
    let mut input = 0_u64;
    for (index, symbol) in program.symbols().iter().enumerate() {
        if let Some(body) = program.function(symbol).and_then(|f| f.body.as_ref()) {
            let size = inline_wave::instruction_count(body)?;
            observations.functions += 1;
            original[index] = size;
            input = input.checked_add(u64::from(size)).ok_or(Error::Limit)?;
        }
    }
    let unwritten = global_contents::find_unwritten(program);
    let mut nounwind = vec![false; program.symbols().len()];
    let mut budgets = inline_policy::Budgets::new(input);
    let mut options = inline_policy::Options::new(level);
    options.limits = limits;
    options.inline_thunk_targets = inline_thunk_targets;
    let mut stats = inline_wave::Stats::default();
    let mut scratch = pass_context::Scratch::default();
    let pipeline = FunctionOptions {
        level,
        unwritten: &unwritten,
        verification,
    };
    storage_arguments::materialize(program)?;
    for wave in [
        inline_policy::Wave::Ordinary,
        inline_policy::Wave::Once,
        inline_policy::Wave::Late,
    ] {
        let mut simplify = |program: &mut Program, symbol, nounwind: &mut Vec<bool>| {
            simplify_function(
                program,
                symbol,
                nounwind,
                &mut observations,
                pipeline,
                &mut scratch,
            )
        };
        inline_wave::Runner::new(
            program,
            options,
            wave,
            inline_wave::Shared {
                budgets: &mut budgets,
                original: &original,
                nounwind: &mut nounwind,
                stats: &mut stats,
            },
            &mut simplify,
        )?
        .run()?;
        observations.functions_pruned += u64::from(unit_graph::prune_unreachable(program)?);
    }
    if level >= 2 {
        let specialized = argument_specialization::specialize(program, |program, symbol| {
            simplify_function(
                program,
                symbol,
                &mut nounwind,
                &mut observations,
                pipeline,
                &mut scratch,
            )
        })?;
        observations.specialized = crate::support::telemetry::counter(specialized);
        if specialized != 0 {
            observations.functions_pruned += u64::from(unit_graph::prune_unreachable(program)?);
        }
        let mut index = 0;
        while index < program.symbols().len() {
            let symbol = program.symbol_id(index).ok_or(Error::Limit)?;
            if program
                .function(program.symbol(symbol))
                .is_some_and(|f| f.body.is_some())
            {
                finish_function(
                    program,
                    symbol,
                    &nounwind,
                    &mut observations,
                    pipeline,
                    &mut scratch,
                )?;
            }
            index += 1;
        }
    }
    observations.calls_inlined = stats.calls;
    observations.inlined_instructions = stats.instructions;
    #[cfg(feature = "telemetry")]
    observations.finish(program, start)?;
    Ok(observations)
}

#[derive(Clone, Copy)]
struct FunctionOptions<'a> {
    level: u8,
    unwritten: &'a [bool],
    verification: Verification<'a>,
}

fn finish_function(
    program: &mut Program,
    symbol: crate::lowir::model::SymbolId,
    nounwind: &[bool],
    observations: &mut Statistics,
    options: FunctionOptions<'_>,
    scratch: &mut pass_context::Scratch,
) -> Result<(), Error> {
    let FunctionOptions {
        level,
        unwritten,
        verification,
    } = options;
    let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
    let result = {
        let mut context =
            pass_context::Context::new(program, symbol, &mut body, 2, nounwind, unwritten);
        context.exchange_scratch(scratch);
        context.verification = verification;
        let result = local_pipeline::finish(&mut context, level);
        observations.collect(&context);
        context.exchange_scratch(scratch);
        result
    }
    .and_then(|changed| if changed { body.compact() } else { Ok(()) });
    program.restore_body(symbol, body)?;
    result
}

fn simplify_function(
    program: &mut Program,
    symbol: crate::lowir::model::SymbolId,
    nounwind: &mut Vec<bool>,
    observations: &mut Statistics,
    options: FunctionOptions<'_>,
    scratch: &mut pass_context::Scratch,
) -> Result<(), Error> {
    let FunctionOptions {
        level,
        unwritten,
        verification,
    } = options;
    let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
    let result = {
        let mut context =
            pass_context::Context::new(program, symbol, &mut body, level, nounwind, unwritten);
        context.exchange_scratch(scratch);
        context.verification = verification;
        let result = context
            .verify_after("inlining")
            .and_then(|()| local_pipeline::simplify(&mut context));
        observations.collect(&context);
        context.exchange_scratch(scratch);
        result
    }
    .and_then(|proven| {
        nounwind.resize(program.symbols().len(), false);
        nounwind[symbol.index()] = proven;
        body.compact()
    });
    program.restore_body(symbol, body)?;
    result
}

#[cfg(test)]
fn pass_view(source: &str, level: u8, pass: pass_context::Pass) -> (String, (bool, u64, u64, u64)) {
    let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
    let symbol = program
        .publications()
        .iter()
        .copied()
        .find(|&symbol| {
            program
                .function(program.symbol(symbol))
                .is_some_and(|function| function.body.is_some())
        })
        .unwrap();
    let mut body = program.take_body(symbol).unwrap().unwrap();
    let facts = {
        let mut context =
            pass_context::Context::new(&mut program, symbol, &mut body, level, &[], &[]);
        let changed = context.run(pass).unwrap();
        (
            changed,
            context.branches_folded,
            context.slots_promoted,
            context.phis_inserted,
        )
    };
    body.compact().unwrap();
    program.restore_body(symbol, body).unwrap();
    let mut output = Vec::new();
    crate::lowir::writer::write(&program, &mut output).unwrap();
    (String::from_utf8(output).unwrap(), facts)
}
