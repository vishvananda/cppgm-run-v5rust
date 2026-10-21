//! Course exception headers, in-flight/caught stacks and handler accounting. See NOTICE.
use super::{
    program::RuntimeProgram,
    rtti::{load_global, store_global},
};
use crate::lowir::{
    Error,
    model::SymbolId,
    vocabulary::{Binary as B, Compare as C, Type as T},
};
#[derive(Clone, Copy)]
struct State {
    payload: SymbolId,
    selector: SymbolId,
    caught: SymbolId,
    in_flight: SymbolId,
    dispatch: SymbolId,
}

pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    super::rtti::type_kind(runtime)?;
    super::rtti::type_equal(runtime)?;
    super::rtti::find_base(runtime)?;
    super::rtti::upcast(runtime)?;
    super::catch::build(runtime)?;
    let state = State {
        payload: runtime.global(b"__cppgm_eh_payload", 0)?,
        selector: runtime.global(b"__cppgm_eh_selector", 0)?,
        caught: runtime.global(b"__cppgm_eh_caught", 8)?,
        in_flight: runtime.global(b"__cppgm_eh_in_flight", 8)?,
        dispatch: runtime.function(b"__cppgm_eh_dispatch", T::Void, &[], false)?,
    };
    let kind = runtime.function(b"__cppgm_type_kind", T::I64, &[T::Ptr], false)?;
    let predicate_routine = runtime.function(
        b"__cppgm_do_catch",
        T::I64,
        &[T::Ptr, T::Ptr, T::Ptr, T::I64],
        false,
    )?;
    allocation(runtime)?;
    throw(runtime, state)?;
    catch(runtime, state, kind, predicate_routine)?;
    begin(runtime, state)?;
    end(runtime, state.caught)?;
    rethrow(runtime, state)?;
    let symbol = runtime.function(b"__cxa_get_exception_ptr", T::Ptr, &[T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let p0 = r.parameter(0)?;
    let value = r.load_offset(T::Ptr, p0, -32)?;
    r.ret(T::Ptr, value)?;
    r.finish()
}
fn allocation(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let symbol = runtime.function(b"__cxa_allocate_exception", T::Ptr, &[T::I64], true)?;
    let mut r = runtime.routine(symbol)?;
    let header = r.integer(64)?;
    let p0 = r.parameter(0)?;
    let bytes = r.add(T::I64, p0, header)?;
    let allocate = r
        .runtime()
        .function(b"__cppgm_allocate", T::Ptr, &[T::I64], false)?;
    let block = r.call(T::Ptr, allocate, &[bytes])?;
    for field in (0..64).step_by(8) {
        let zero = r.integer(0)?;
        r.store_offset(T::I64, zero, block, field)?;
    }
    let value = r.at_offset(block, 64)?;
    r.ret(T::Ptr, value)?;
    r.finish()?;
    let symbol = runtime.function(b"__cxa_free_exception", T::Void, &[T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let p0 = r.parameter(0)?;
    let block = r.at_offset(p0, -64)?;
    let release = r
        .runtime()
        .function(b"__cppgm_release", T::Void, &[T::Ptr], false)?;
    r.call_void(release, &[block])?;
    r.ret_void()?;
    r.finish()
}
fn throw(runtime: &mut RuntimeProgram<'_>, s: State) -> Result<(), Error> {
    let symbol = runtime.function(b"__cxa_throw", T::Void, &[T::Ptr, T::Ptr, T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let p0 = r.parameter(0)?;
    let p1 = r.parameter(1)?;
    let p2 = r.parameter(2)?;
    let header = r.at_offset(p0, -64)?;
    r.store_offset(T::Ptr, p1, header, 16)?;
    r.store_offset(T::Ptr, p2, header, 24)?;
    r.store_offset(T::Ptr, p0, header, 32)?;
    let zero = r.integer(0)?;
    r.store_offset(T::I64, zero, header, 40)?;
    let value = load_global(&mut r, T::Ptr, s.in_flight)?;
    r.store_offset(T::Ptr, value, header, 8)?;
    store_global(&mut r, T::Ptr, header, s.in_flight)?;
    store_global(&mut r, T::Ptr, p0, s.payload)?;
    r.call_void(s.dispatch, &[])?;
    r.unreachable()?;
    r.finish()
}
fn catch(
    runtime: &mut RuntimeProgram<'_>,
    s: State,
    kind: SymbolId,
    predicate_routine: SymbolId,
) -> Result<(), Error> {
    let symbol = runtime.function(b"__cppgm_eh_catch", T::Void, &[T::Ptr, T::I64], true)?;
    let mut r = runtime.routine(symbol)?;
    let p0 = r.parameter(0)?;
    let p1 = r.parameter(1)?;
    let adjusted = r.local(T::Ptr)?;
    let open = r.block()?;
    let done = r.block()?;
    let typed = r.block()?;
    let matched = r.block()?;
    let test = r.block()?;
    let pointer = r.block()?;
    let condition = load_global(&mut r, T::I64, s.selector)?;
    r.branch(condition, done, open)?;
    r.enter(open)?;
    let object = load_global(&mut r, T::Ptr, s.payload)?;
    let header = r.at_offset(object, -64)?;
    r.store(T::Ptr, object, adjusted)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, p0, zero)?;
    r.branch(condition, matched, typed)?;
    r.enter(typed)?;
    let thrown = r.load_offset(T::Ptr, header, 16)?;
    let five = r.integer(5)?;
    let value = r.call(T::I64, kind, &[thrown])?;
    let condition = r.equal(T::I64, value, five)?;
    r.branch(condition, pointer, test)?;
    r.enter(pointer)?;
    let value = r.load(T::Ptr, object)?;
    r.store(T::Ptr, value, adjusted)?;
    r.jump(test)?;
    r.enter(test)?;
    let address = r.slot_address(adjusted)?;
    let one = r.integer(1)?;
    let condition = r.call(T::I64, predicate_routine, &[p0, thrown, address, one])?;
    r.branch(condition, matched, done)?;
    r.enter(matched)?;
    let value = r.load(T::Ptr, adjusted)?;
    r.store_offset(T::Ptr, value, header, 32)?;
    r.store_offset(T::I64, p1, header, 48)?;
    store_global(&mut r, T::I64, p1, s.selector)?;
    r.ret_void()?;
    r.enter(done)?;
    r.ret_void()?;
    r.finish()
}
fn begin(runtime: &mut RuntimeProgram<'_>, s: State) -> Result<(), Error> {
    let symbol = runtime.function(b"__cxa_begin_catch", T::Ptr, &[T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let p0 = r.parameter(0)?;
    let link = r.local(T::Ptr)?;
    let header = r.at_offset(p0, -64)?;
    let top = r.block()?;
    let search = r.block()?;
    let test = r.block()?;
    let step = r.block()?;
    let unlink = r.block()?;
    let removed = r.block()?;
    let restore = r.block()?;
    let counted = r.block()?;
    let rethrown = r.block()?;
    let fresh = r.block()?;
    let stacked = r.block()?;
    let push = r.block()?;
    let first = load_global(&mut r, T::Ptr, s.in_flight)?;
    let condition = r.equal(T::Ptr, first, header)?;
    r.branch(condition, top, search)?;
    r.enter(top)?;
    let next = r.load_offset(T::Ptr, header, 8)?;
    store_global(&mut r, T::Ptr, next, s.in_flight)?;
    r.jump(removed)?;
    r.enter(search)?;
    r.store(T::Ptr, first, link)?;
    r.jump(test)?;
    r.enter(test)?;
    let node = r.load(T::Ptr, link)?;
    let walk = r.block()?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, node, zero)?;
    r.branch(condition, removed, walk)?;
    r.enter(walk)?;
    let successor = r.load_offset(T::Ptr, node, 8)?;
    let condition = r.equal(T::Ptr, successor, header)?;
    r.branch(condition, unlink, step)?;
    r.enter(unlink)?;
    let value = r.load_offset(T::Ptr, header, 8)?;
    r.store_offset(T::Ptr, value, node, 8)?;
    r.jump(removed)?;
    r.enter(step)?;
    r.store(T::Ptr, successor, link)?;
    r.jump(test)?;
    r.enter(removed)?;
    let enclosing = load_global(&mut r, T::Ptr, s.in_flight)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, enclosing, zero)?;
    r.branch(condition, counted, restore)?;
    r.enter(restore)?;
    let object = r.at_offset(enclosing, 64)?;
    store_global(&mut r, T::Ptr, object, s.payload)?;
    let selector = r.load_offset(T::I64, enclosing, 48)?;
    store_global(&mut r, T::I64, selector, s.selector)?;
    r.jump(counted)?;
    r.enter(counted)?;
    let handlers = r.local(T::I64)?;
    let count = r.load_offset(T::I64, header, 40)?;
    let zero = r.integer(0)?;
    let condition = r.compare(C::Lt, T::I64, count, zero)?;
    r.branch(condition, rethrown, fresh)?;
    r.enter(rethrown)?;
    let one = r.integer(1)?;
    let value = r.binary(B::Sub, T::I64, one, count)?;
    r.store(T::I64, value, handlers)?;
    r.jump(stacked)?;
    r.enter(fresh)?;
    let one = r.integer(1)?;
    let value = r.add(T::I64, count, one)?;
    r.store(T::I64, value, handlers)?;
    r.jump(stacked)?;
    r.enter(stacked)?;
    let value = r.load(T::I64, handlers)?;
    r.store_offset(T::I64, value, header, 40)?;
    let done = r.block()?;
    let value = load_global(&mut r, T::Ptr, s.caught)?;
    let condition = r.equal(T::Ptr, value, header)?;
    r.branch(condition, done, push)?;
    r.enter(push)?;
    let value = load_global(&mut r, T::Ptr, s.caught)?;
    r.store_offset(T::Ptr, value, header, 0)?;
    store_global(&mut r, T::Ptr, header, s.caught)?;
    r.jump(done)?;
    r.enter(done)?;
    let value = r.load_offset(T::Ptr, header, 32)?;
    r.ret(T::Ptr, value)?;
    r.finish()
}
fn end(runtime: &mut RuntimeProgram<'_>, caught: SymbolId) -> Result<(), Error> {
    let symbol = runtime.function(b"__cxa_end_catch", T::Void, &[], true)?;
    let mut r = runtime.routine(symbol)?;
    let signature = r.runtime().signature(T::Void, &[T::Ptr])?;
    let some = r.block()?;
    let none = r.block()?;
    let rethrown = r.block()?;
    let handled = r.block()?;
    let still_rethrown = r.block()?;
    let uncaught = r.block()?;
    let keep = r.block()?;
    let destroy = r.block()?;
    let call = r.block()?;
    let free = r.block()?;
    let header = load_global(&mut r, T::Ptr, caught)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, header, zero)?;
    r.branch(condition, none, some)?;
    r.enter(none)?;
    r.ret_void()?;
    r.enter(some)?;
    let count = r.load_offset(T::I64, header, 40)?;
    let zero = r.integer(0)?;
    let condition = r.compare(C::Lt, T::I64, count, zero)?;
    r.branch(condition, rethrown, handled)?;
    r.enter(rethrown)?;
    let one = r.integer(1)?;
    let up_count = r.add(T::I64, count, one)?;
    r.store_offset(T::I64, up_count, header, 40)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::I64, up_count, zero)?;
    r.branch(condition, uncaught, still_rethrown)?;
    r.enter(uncaught)?;
    let next = r.load_offset(T::Ptr, header, 0)?;
    store_global(&mut r, T::Ptr, next, caught)?;
    r.jump(still_rethrown)?;
    r.enter(still_rethrown)?;
    r.ret_void()?;
    r.enter(handled)?;
    let one = r.integer(1)?;
    let down_count = r.binary(B::Sub, T::I64, count, one)?;
    r.store_offset(T::I64, down_count, header, 40)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::I64, down_count, zero)?;
    r.branch(condition, destroy, keep)?;
    r.enter(keep)?;
    r.ret_void()?;
    r.enter(destroy)?;
    let below = r.load_offset(T::Ptr, header, 0)?;
    store_global(&mut r, T::Ptr, below, caught)?;
    let destructor = r.load_offset(T::Ptr, header, 24)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, destructor, zero)?;
    r.branch(condition, free, call)?;
    r.enter(call)?;
    let object = r.at_offset(header, 64)?;
    r.call_indirect_void(destructor, Some(signature), &[object])?;
    r.jump(free)?;
    r.enter(free)?;
    let release = r
        .runtime()
        .function(b"__cppgm_release", T::Void, &[T::Ptr], false)?;
    r.call_void(release, &[header])?;
    r.ret_void()?;
    r.finish()
}
fn rethrow(runtime: &mut RuntimeProgram<'_>, s: State) -> Result<(), Error> {
    let symbol = runtime.function(b"__cxa_rethrow", T::Void, &[], true)?;
    let mut r = runtime.routine(symbol)?;
    let none = r.block()?;
    let some = r.block()?;
    let header = load_global(&mut r, T::Ptr, s.caught)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, header, zero)?;
    r.branch(condition, none, some)?;
    r.enter(none)?;
    let terminate = r
        .runtime()
        .function(b"_ZSt9terminatev", T::Void, &[], false)?;
    r.call_void(terminate, &[])?;
    r.unreachable()?;
    r.enter(some)?;
    let value = r.load_offset(T::I64, header, 40)?;
    let zero = r.integer(0)?;
    let value = r.binary(B::Sub, T::I64, zero, value)?;
    r.store_offset(T::I64, value, header, 40)?;
    let zero = r.integer(0)?;
    r.store_offset(T::I64, zero, header, 48)?;
    let value = load_global(&mut r, T::Ptr, s.in_flight)?;
    r.store_offset(T::Ptr, value, header, 8)?;
    store_global(&mut r, T::Ptr, header, s.in_flight)?;
    let object = r.at_offset(header, 64)?;
    store_global(&mut r, T::Ptr, object, s.payload)?;
    r.call_void(s.dispatch, &[])?;
    r.unreachable()?;
    r.finish()
}
