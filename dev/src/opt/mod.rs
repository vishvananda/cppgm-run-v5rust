//! Typed unit optimization, translated from opt/optimizer.cpp. See NOTICE.
mod algebra;
mod alias_facts;
mod branch_ranges;
mod condition_placement;
mod control_flow;
mod dead_code;
mod definition_order;
mod diamond_reuse;
mod dominating_facts;
mod edge_demotion;
mod exception_regions;
mod flow_cleanup;
mod global_contents;
mod handler_liveness;
mod jump_threading;
mod load_placement;
mod local_pipeline;
mod memory_calls;
mod memory_contents;
mod memory_forward;
mod numeric_fold;
mod pass_context;
mod phi_threading;
mod range_threading;
mod slot_promotion;
mod sparse_constants;
mod storage_arguments;
mod store_liveness;
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
    let unwritten = global_contents::find_unwritten(program);
    storage_arguments::materialize(program)?;
    optimize_functions(program, level, &unwritten)?;
    unit_graph::prune_unreachable(program)?;
    Err(Error::NotImplemented("LowIR remaining inline waves"))
}

fn optimize_functions(program: &mut Program, level: u8, unwritten: &[bool]) -> Result<(), Error> {
    let graph = unit_graph::CallGraph::build(program)?;
    let mut nounwind = vec![false; program.symbols().len()];
    for &index in &graph.bottom_up {
        let symbol = graph.symbol(index);
        let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
        let result = {
            let mut context =
                pass_context::Context::new(program, symbol, &mut body, level, &nounwind, unwritten);
            local_pipeline::simplify(&mut context)
        }
        .and_then(|proven| {
            nounwind[symbol.index()] = proven;
            body.compact(&mut program.names)
        });
        program.restore_body(symbol, body)?;
        result?;
    }
    Err(Error::NotImplemented("LowIR inlining and function passes"))
}

#[cfg(test)]
fn pass_view(source: &str, level: u8, pass: pass_context::Pass) -> (String, (bool, u64, u64, u64)) {
    let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
    let symbol = program.publications()[0];
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
