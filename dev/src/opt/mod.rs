//! Typed unit optimization, translated from opt/optimizer.cpp. See NOTICE.
mod control_flow;
mod dead_code;
mod exception_regions;
mod global_contents;
mod local_pipeline;
mod pass_context;
mod storage_arguments;
mod unit_graph;
use crate::lowir::{Error, model::Program};

pub(crate) fn optimize(program: &mut Program, level: u8) -> Result<(), Error> {
    if level == 0 {
        return Ok(());
    }
    let _unwritten = global_contents::find_unwritten(program);
    storage_arguments::materialize(program)?;
    optimize_functions(program, level)?;
    unit_graph::prune_unreachable(program)?;
    Err(Error::NotImplemented("LowIR remaining inline waves"))
}

fn optimize_functions(program: &mut Program, level: u8) -> Result<(), Error> {
    let graph = unit_graph::CallGraph::build(program)?;
    let mut nounwind = vec![false; program.symbols().len()];
    for &index in &graph.bottom_up {
        let symbol = graph.symbol(index);
        let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
        let result = {
            let mut context =
                pass_context::Context::new(program, symbol, &mut body, level, &nounwind);
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
