//! Typed unit optimization, translated from opt/optimizer.cpp. See NOTICE.
mod control_flow;
mod global_contents;
mod storage_arguments;
mod unit_graph;
use crate::lowir::{Error, model::Program};

pub(crate) fn optimize(program: &mut Program, level: u8) -> Result<(), Error> {
    if level == 0 {
        return Ok(());
    }
    let _unwritten = global_contents::find_unwritten(program);
    storage_arguments::materialize(program)?;
    optimize_functions(program)?;
    unit_graph::prune_unreachable(program)?;
    Err(Error::NotImplemented("LowIR remaining inline waves"))
}

fn optimize_functions(program: &Program) -> Result<(), Error> {
    let graph = unit_graph::CallGraph::build(program)?;
    for &index in &graph.bottom_up {
        let symbol = graph.symbol(index);
        let body = program
            .function(program.symbol(symbol))
            .and_then(|function| function.body.as_ref())
            .ok_or(Error::Limit)?;
        let _cfg = control_flow::Cfg::analyze(body)?;
    }
    Err(Error::NotImplemented("LowIR inlining and function passes"))
}
