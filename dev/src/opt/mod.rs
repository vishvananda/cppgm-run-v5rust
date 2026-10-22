//! Typed unit optimization, translated from opt/optimizer.cpp. See NOTICE.
mod aggregate_slots;
mod algebra;
mod alias_facts;
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
mod flow_cleanup;
mod global_contents;
mod handler_liveness;
mod induction;
mod inline_arguments;
mod inline_policy;
mod inline_shape;
mod inline_wave;
mod iteration_values;
mod jump_threading;
mod load_placement;
mod local_pipeline;
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
mod return_values;
mod returned_storage;
mod slot_copies;
mod slot_promotion;
mod sparse_constants;
mod storage_arguments;
mod store_liveness;
mod trip_counts;
mod unit_graph;
mod value_reuse;
use crate::lowir::{Error, model::Program};

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

pub(crate) fn optimize(program: &mut Program, level: u8) -> Result<(), Error> {
    if level == 0 {
        return Ok(());
    }
    let mut original = vec![0; program.symbols().len()];
    let mut input = 0_u64;
    for (index, symbol) in program.symbols().iter().enumerate() {
        if let Some(body) = program.function(symbol).and_then(|f| f.body.as_ref()) {
            let size = inline_wave::instruction_count(body)?;
            original[index] = size;
            input = input.checked_add(u64::from(size)).ok_or(Error::Limit)?;
        }
    }
    let unwritten = global_contents::find_unwritten(program);
    let mut nounwind = vec![false; program.symbols().len()];
    let mut budgets = inline_policy::Budgets::new(input);
    let options = inline_policy::Options::new(level);
    let mut stats = inline_wave::Stats::default();
    storage_arguments::materialize(program)?;
    for wave in [
        inline_policy::Wave::Ordinary,
        inline_policy::Wave::Once,
        inline_policy::Wave::Late,
    ] {
        let mut simplify = |program: &mut Program, symbol, nounwind: &mut Vec<bool>| {
            simplify_function(program, symbol, level, nounwind, &unwritten)
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
        unit_graph::prune_unreachable(program)?;
    }
    if level >= 2 {
        return Err(Error::NotImplemented(
            "LowIR argument specialization and finishing passes",
        ));
    }
    Ok(())
}

fn simplify_function(
    program: &mut Program,
    symbol: crate::lowir::model::SymbolId,
    level: u8,
    nounwind: &mut Vec<bool>,
    unwritten: &[bool],
) -> Result<(), Error> {
    let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
    let result = {
        let mut context =
            pass_context::Context::new(program, symbol, &mut body, level, nounwind, unwritten);
        local_pipeline::simplify(&mut context)
    }
    .and_then(|proven| {
        nounwind.resize(program.symbols().len(), false);
        nounwind[symbol.index()] = proven;
        body.compact(&mut program.names)
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
        let mut context = pass_context::Context::new(&program, symbol, &mut body, level, &[], &[]);
        let changed = context.run(pass).unwrap();
        (
            changed,
            context.branches_folded,
            context.slots_promoted,
            context.phis_inserted,
        )
    };
    body.compact(&mut program.names).unwrap();
    program.restore_body(symbol, body).unwrap();
    let mut output = Vec::new();
    crate::lowir::writer::write(&program, &mut output).unwrap();
    (String::from_utf8(output).unwrap(), facts)
}
