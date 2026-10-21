//! Single-thread guards and termination callback registration. See NOTICE.
use super::program::RuntimeProgram;
use crate::lowir::{Error, vocabulary::Type as T};

pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let symbol = runtime.function(b"__cxa_guard_acquire", T::I32, &[T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let done = r.block()?;
    let pending = r.block()?;
    let p0 = r.parameter(0)?;
    let value = r.load(T::I8, p0)?;
    r.branch(value, done, pending)?;
    r.enter(done)?;
    let zero = r.integer(0)?;
    r.ret(T::I32, zero)?;
    r.enter(pending)?;
    let one = r.integer(1)?;
    r.ret(T::I32, one)?;
    r.finish()?;
    let symbol = runtime.function(b"__cxa_guard_release", T::Void, &[T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let p0 = r.parameter(0)?;
    let one = r.integer(1)?;
    r.store(T::I8, one, p0)?;
    r.ret_void()?;
    r.finish()?;
    let symbol = runtime.function(b"__cxa_guard_abort", T::Void, &[T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    r.ret_void()?;
    r.finish()?;
    let head = runtime.global(b"__cppgm_atexit_head", 0)?;
    let thread_head = runtime.global(b"__cppgm_thread_atexit_head", 0)?;
    let symbols = [
        runtime.function(b"atexit", T::I32, &[T::Ptr], true)?,
        runtime.function(b"__cxa_atexit", T::I32, &[T::Ptr, T::Ptr, T::Ptr], true)?,
        runtime.function(
            b"__cxa_thread_atexit",
            T::I32,
            &[T::Ptr, T::Ptr, T::Ptr],
            true,
        )?,
    ];
    for (index, symbol) in symbols.into_iter().enumerate() {
        let list = if index == 2 { thread_head } else { head };
        let mut r = runtime.routine(symbol)?;
        let bytes = r.integer(24)?;
        let allocate = r
            .runtime()
            .function(b"__cppgm_allocate", T::Ptr, &[T::I64], false)?;
        let node = r.call(T::Ptr, allocate, &[bytes])?;
        let p0 = r.parameter(0)?;
        r.store(T::Ptr, p0, node)?;
        let address = r.address(list)?;
        let next = r.load(T::Ptr, address)?;
        r.store_offset(T::Ptr, next, node, 8)?;
        let argument = if index == 0 {
            r.integer(0)?
        } else {
            r.parameter(1)?
        };
        r.store_offset(T::Ptr, argument, node, 16)?;
        let address = r.address(list)?;
        r.store(T::Ptr, node, address)?;
        let zero = r.integer(0)?;
        r.ret(T::I32, zero)?;
        r.finish()?;
    }
    Ok(())
}
