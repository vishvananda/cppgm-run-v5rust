//! Typed unit optimization, translated from opt/optimizer.cpp. See NOTICE.
mod control_flow;
mod dead_code;
mod definition_order;
mod exception_regions;
mod flow_cleanup;
mod global_contents;
mod local_pipeline;
mod numeric_fold;
mod pass_context;
mod sparse_constants;
mod storage_arguments;
mod unit_graph;
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
