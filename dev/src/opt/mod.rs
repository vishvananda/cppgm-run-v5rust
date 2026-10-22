//! Typed unit optimization, translated from opt/optimizer.cpp. See NOTICE.
mod global_contents;
mod storage_arguments;
use crate::lowir::{Error, model::Program};

pub(crate) fn optimize(program: &mut Program, level: u8) -> Result<(), Error> {
    if level == 0 {
        return Ok(());
    }
    let _unwritten = global_contents::find_unwritten(program);
    storage_arguments::materialize(program)?;
    Err(Error::NotImplemented(
        "LowIR call graph, inlining and function passes",
    ))
}
