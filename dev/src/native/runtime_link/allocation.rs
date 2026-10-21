//! Size-class allocation and C allocation interfaces of the course runtime. See NOTICE.
use super::program::{Routine, RuntimeProgram};
use crate::lowir::{
    Error,
    model::{SymbolId, Value},
    vocabulary::{Binary as B, Compare as C, Type as T},
};
const SMALL: i64 = 1024;
const MEDIUM: i64 = 131_072;
const CHUNK: i64 = 1_048_576;
const HEADER: i64 = 16;
const LIMIT: i64 = 1 << 47;
fn size_class(
    r: &mut Routine<'_>,
    total: Value,
    rounded: Value,
    offset: Value,
) -> Result<(), Error> {
    let value = r.integer(2048)?;
    r.store(T::I64, value, rounded)?;
    let zero = r.integer(0)?;
    r.store(T::I64, zero, offset)?;
    let test = r.block()?;
    let step = r.block()?;
    let done = r.block()?;
    r.jump(test)?;
    r.enter(test)?;
    let value = r.load(T::I64, rounded)?;
    let condition = r.compare(C::Ult, T::I64, value, total)?;
    r.branch(condition, step, done)?;
    r.enter(step)?;
    let one = r.integer(1)?;
    let value = r.load(T::I64, rounded)?;
    let value = r.binary(B::Shl, T::I64, value, one)?;
    r.store(T::I64, value, rounded)?;
    let eight = r.integer(8)?;
    let value = r.load(T::I64, offset)?;
    let value = r.add(T::I64, value, eight)?;
    r.store(T::I64, value, offset)?;
    r.jump(test)?;
    r.enter(done)
}
pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let next = runtime.global(b"__cppgm_heap_next", 8)?;
    let end = runtime.global(b"__cppgm_heap_end", 8)?;
    let small = runtime.global(
        b"__cppgm_free_small",
        8 * (u64::try_from(SMALL).map_err(|_| Error::Limit)? / 16 + 1),
    )?;
    let medium = runtime.global(b"__cppgm_free_medium", 64)?;
    let mmap = runtime.function(b"__cppgm_mmap", T::Ptr, &[T::I64], false)?;
    let munmap = runtime.function(b"__cppgm_munmap", T::Void, &[T::Ptr, T::I64], false)?;
    let allocate = runtime.function(b"__cppgm_allocate", T::Ptr, &[T::I64], true)?;
    emit_allocate(runtime, allocate, [next, end, small, medium], mmap)?;
    let release = runtime.function(b"__cppgm_release", T::Void, &[T::Ptr], true)?;
    emit_release(runtime, release, small, medium, munmap)?;
    let operator_new = runtime.function(b"__cppgm_operator_new", T::Ptr, &[T::I64], true)?;
    emit_operator_new(runtime, operator_new, allocate)?;
    for name in [&b"cppgm_builtin_operator_new"[..], &b"_Znwm"[..]] {
        runtime.alias(name, operator_new)?;
    }
    for name in [
        &b"cppgm_builtin_operator_delete"[..],
        &b"_ZdlPv"[..],
        &b"_ZdlPvm"[..],
        &b"_ZdaPvm"[..],
    ] {
        runtime.alias(name, release)?;
    }
    for (array, scalar, allocates) in [
        (
            &b"cppgm_builtin_operator_new_array"[..],
            &b"cppgm_builtin_operator_new"[..],
            true,
        ),
        (&b"_Znam"[..], &b"_Znwm"[..], true),
        (
            &b"cppgm_builtin_operator_delete_array"[..],
            &b"cppgm_builtin_operator_delete"[..],
            false,
        ),
        (&b"_ZdaPv"[..], &b"_ZdlPv"[..], false),
    ] {
        let result = if allocates { T::Ptr } else { T::Void };
        let parameter = if allocates { T::I64 } else { T::Ptr };
        let scalar = runtime.function(scalar, result, &[parameter], false)?;
        let array = runtime.function(array, result, &[parameter], true)?;
        let mut r = runtime.routine(array)?;
        let p0 = r.parameter(0)?;
        if allocates {
            let value = r.call(result, scalar, &[p0])?;
            r.ret(T::Ptr, value)?;
        } else {
            r.call_void(scalar, &[p0])?;
            r.ret_void()?;
        }
        r.finish()?;
    }
    runtime.alias(b"malloc", allocate)?;
    runtime.alias(b"free", release)?;
    c_allocation(runtime, allocate, release)
}
fn copy_words(
    r: &mut Routine<'_>,
    to: Value,
    from: Option<Value>,
    bytes: Value,
) -> Result<(), Error> {
    let at = r.local(T::I64)?;
    let zero = r.integer(0)?;
    r.store(T::I64, zero, at)?;
    let test = r.block()?;
    let body = r.block()?;
    let done = r.block()?;
    r.jump(test)?;
    r.enter(test)?;
    let value = r.load(T::I64, at)?;
    let condition = r.compare(C::Ult, T::I64, value, bytes)?;
    r.branch(condition, body, done)?;
    r.enter(body)?;
    let offset = r.load(T::I64, at)?;
    let word = if let Some(from) = from {
        let address = r.at(from, offset)?;
        r.load(T::I64, address)?
    } else {
        r.integer(0)?
    };
    let address = r.at(to, offset)?;
    r.store(T::I64, word, address)?;
    let eight = r.integer(8)?;
    let value = r.add(T::I64, offset, eight)?;
    r.store(T::I64, value, at)?;
    r.jump(test)?;
    r.enter(done)
}
fn c_allocation(
    runtime: &mut RuntimeProgram<'_>,
    allocate: SymbolId,
    release: SymbolId,
) -> Result<(), Error> {
    let calloc = runtime.function(b"calloc", T::Ptr, &[T::I64, T::I64], true)?;
    {
        let mut r = runtime.routine(calloc)?;
        let p0 = r.parameter(0)?;
        let p1 = r.parameter(1)?;
        let check = r.block()?;
        let overflow = r.block()?;
        let sized = r.block()?;
        let failed = r.block()?;
        let clear = r.block()?;
        let bytes = r.binary(B::Mul, T::I64, p0, p1)?;
        let zero = r.integer(0)?;
        let condition = r.equal(T::I64, p1, zero)?;
        r.branch(condition, sized, check)?;
        r.enter(check)?;
        let quotient = r.binary(B::UDiv, T::I64, bytes, p1)?;
        let condition = r.equal(T::I64, quotient, p0)?;
        r.branch(condition, sized, overflow)?;
        r.enter(overflow)?;
        let zero = r.integer(0)?;
        r.ret(T::Ptr, zero)?;
        r.enter(sized)?;
        let block = r.call(T::Ptr, allocate, &[bytes])?;
        let zero = r.integer(0)?;
        let condition = r.equal(T::Ptr, block, zero)?;
        r.branch(condition, failed, clear)?;
        r.enter(failed)?;
        let zero = r.integer(0)?;
        r.ret(T::Ptr, zero)?;
        r.enter(clear)?;
        let block_bytes = r.load_offset(T::I64, block, -HEADER)?;
        let reused = r.block()?;
        let zeroed = r.block()?;
        let limit = r.integer(MEDIUM)?;
        let condition = r.compare(C::Ugt, T::I64, block_bytes, limit)?;
        r.branch(condition, zeroed, reused)?;
        r.enter(reused)?;
        copy_words(&mut r, block, None, block_bytes)?;
        r.jump(zeroed)?;
        r.enter(zeroed)?;
        r.ret(T::Ptr, block)?;
        r.finish()?;
    }
    let realloc = runtime.function(b"realloc", T::Ptr, &[T::Ptr, T::I64], true)?;
    {
        let mut r = runtime.routine(realloc)?;
        let p0 = r.parameter(0)?;
        let p1 = r.parameter(1)?;
        let fresh = r.block()?;
        let existing = r.block()?;
        let grow = r.block()?;
        let keep = r.block()?;
        let failed = r.block()?;
        let copy = r.block()?;
        let zero = r.integer(0)?;
        let condition = r.equal(T::Ptr, p0, zero)?;
        r.branch(condition, fresh, existing)?;
        r.enter(fresh)?;
        let value = r.call(T::Ptr, allocate, &[p1])?;
        r.ret(T::Ptr, value)?;
        r.enter(existing)?;
        let old_bytes = r.load_offset(T::I64, p0, -HEADER)?;
        let condition = r.compare(C::Ule, T::I64, p1, old_bytes)?;
        r.branch(condition, keep, grow)?;
        r.enter(keep)?;
        r.ret(T::Ptr, p0)?;
        r.enter(grow)?;
        let mremap =
            r.runtime()
                .function(b"__cppgm_mremap", T::Ptr, &[T::Ptr, T::I64, T::I64], false)?;
        let sized = r.block()?;
        let remap = r.block()?;
        let movement = r.block()?;
        let remapped = r.block()?;
        let limit = r.integer(MEDIUM)?;
        let condition = r.compare(C::Ugt, T::I64, old_bytes, limit)?;
        r.branch(condition, sized, movement)?;
        r.enter(sized)?;
        let limit = r.integer(LIMIT)?;
        let condition = r.compare(C::Ult, T::I64, p1, limit)?;
        r.branch(condition, remap, movement)?;
        r.enter(remap)?;
        let mask = r.integer(-16)?;
        let fifteen = r.integer(15)?;
        let value = r.add(T::I64, p1, fifteen)?;
        let new_bytes = r.binary(B::And, T::I64, value, mask)?;
        let header = r.at_offset(p0, -HEADER)?;
        // Braced call arguments are sequenced left to right by C++.
        let h = r.integer(HEADER)?;
        let old_size = r.add(T::I64, old_bytes, h)?;
        let h = r.integer(HEADER)?;
        let new_size = r.add(T::I64, new_bytes, h)?;
        let mapping = r.call(T::Ptr, mremap, &[header, old_size, new_size])?;
        let zero = r.integer(0)?;
        let condition = r.equal(T::Ptr, mapping, zero)?;
        r.branch(condition, failed, remapped)?;
        r.enter(remapped)?;
        r.store(T::I64, new_bytes, mapping)?;
        let value = r.at_offset(mapping, HEADER)?;
        r.ret(T::Ptr, value)?;
        r.enter(movement)?;
        let block = r.call(T::Ptr, allocate, &[p1])?;
        let zero = r.integer(0)?;
        let condition = r.equal(T::Ptr, block, zero)?;
        r.branch(condition, failed, copy)?;
        r.enter(failed)?;
        let zero = r.integer(0)?;
        r.ret(T::Ptr, zero)?;
        r.enter(copy)?;
        copy_words(&mut r, block, Some(p0), old_bytes)?;
        r.call_void(release, &[p0])?;
        r.ret(T::Ptr, block)?;
        r.finish()?;
    }
    Ok(())
}

fn emit_allocate(
    runtime: &mut RuntimeProgram<'_>,
    allocate: SymbolId,
    state: [SymbolId; 4],
    mmap: SymbolId,
) -> Result<(), Error> {
    let [next, end, small, medium] = state;
    let mut r = runtime.routine(allocate)?;
    let p0 = r.parameter(0)?;
    let total = r.local(T::I64)?;
    let rounded = r.local(T::I64)?;
    let offset = r.local(T::I64)?;
    let list = r.local(T::Ptr)?;
    let empty = r.block()?;
    let small_block = r.block()?;
    let larger = r.block()?;
    let medium_block = r.block()?;
    let pop = r.block()?;
    let bump = r.block()?;
    let carve = r.block()?;
    let refill = r.block()?;
    let large = r.block()?;
    let exhausted = r.block()?;
    let mapped = r.block()?;
    let refilled = r.block()?;
    let possible = r.block()?;
    let limit = r.integer(LIMIT)?;
    let condition = r.compare(C::Uge, T::I64, p0, limit)?;
    r.branch(condition, exhausted, possible)?;
    r.enter(possible)?;
    let mask = r.integer(-16)?;
    let fifteen = r.integer(15)?;
    let sum = r.add(T::I64, p0, fifteen)?;
    let size = r.binary(B::And, T::I64, sum, mask)?;
    r.store(T::I64, size, total)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::I64, size, zero)?;
    r.branch(condition, empty, small_block)?;
    r.enter(empty)?;
    let sixteen = r.integer(16)?;
    r.store(T::I64, sixteen, total)?;
    r.jump(small_block)?;
    r.enter(small_block)?;
    let size = r.load(T::I64, total)?;
    let small_list = r.block()?;
    let limit = r.integer(SMALL)?;
    let condition = r.compare(C::Ule, T::I64, size, limit)?;
    r.branch(condition, small_list, larger)?;
    r.enter(small_list)?;
    let small_lists = r.address(small)?;
    let one = r.integer(1)?;
    let half = r.binary(B::UShr, T::I64, size, one)?;
    let address = r.at(small_lists, half)?;
    r.store(T::Ptr, address, list)?;
    r.jump(pop)?;
    r.enter(larger)?;
    let limit = r.integer(MEDIUM)?;
    let condition = r.compare(C::Ugt, T::I64, size, limit)?;
    r.branch(condition, large, medium_block)?;
    r.enter(medium_block)?;
    size_class(&mut r, size, rounded, offset)?;
    let value = r.load(T::I64, rounded)?;
    r.store(T::I64, value, total)?;
    let medium_lists = r.address(medium)?;
    let value = r.load(T::I64, offset)?;
    let address = r.at(medium_lists, value)?;
    r.store(T::Ptr, address, list)?;
    r.jump(pop)?;
    r.enter(pop)?;
    let address = r.load(T::Ptr, list)?;
    let head = r.load(T::Ptr, address)?;
    let reuse = r.block()?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, head, zero)?;
    r.branch(condition, bump, reuse)?;
    r.enter(reuse)?;
    let rest = r.load(T::Ptr, head)?;
    let address = r.load(T::Ptr, list)?;
    r.store(T::Ptr, rest, address)?;
    r.ret(T::Ptr, head)?;
    r.enter(bump)?;
    let address = r.address(next)?;
    let start = r.load(T::Ptr, address)?;
    let header = r.integer(HEADER)?;
    let value = r.load(T::I64, total)?;
    let limit = r.add(T::I64, value, header)?;
    let after = r.at(start, limit)?;
    let fits = r.block()?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, start, zero)?;
    r.branch(condition, refill, fits)?;
    r.enter(fits)?;
    let address = r.address(end)?;
    let end_value = r.load(T::Ptr, address)?;
    let condition = r.compare(C::Ugt, T::Ptr, after, end_value)?;
    r.branch(condition, refill, carve)?;
    r.enter(carve)?;
    let address = r.address(next)?;
    r.store(T::Ptr, after, address)?;
    let value = r.load(T::I64, total)?;
    r.store(T::I64, value, start)?;
    let result = r.at_offset(start, HEADER)?;
    r.ret(T::Ptr, result)?;
    r.enter(refill)?;
    let bytes = r.integer(CHUNK)?;
    let chunk = r.call(T::Ptr, mmap, &[bytes])?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, chunk, zero)?;
    r.branch(condition, exhausted, refilled)?;
    r.enter(refilled)?;
    let address = r.address(next)?;
    r.store(T::Ptr, chunk, address)?;
    let chunk_end = r.at_offset(chunk, CHUNK)?;
    let address = r.address(end)?;
    r.store(T::Ptr, chunk_end, address)?;
    r.jump(bump)?;
    r.enter(large)?;
    let header = r.integer(HEADER)?;
    let bytes = r.add(T::I64, size, header)?;
    let block = r.call(T::Ptr, mmap, &[bytes])?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, block, zero)?;
    r.branch(condition, exhausted, mapped)?;
    r.enter(mapped)?;
    r.store(T::I64, size, block)?;
    let result = r.at_offset(block, HEADER)?;
    r.ret(T::Ptr, result)?;
    r.enter(exhausted)?;
    let zero = r.integer(0)?;
    r.ret(T::Ptr, zero)?;
    r.finish()?;
    Ok(())
}

fn emit_release(
    runtime: &mut RuntimeProgram<'_>,
    release: SymbolId,
    small: SymbolId,
    medium: SymbolId,
    munmap: SymbolId,
) -> Result<(), Error> {
    let mut r = runtime.routine(release)?;
    let p0 = r.parameter(0)?;
    let rounded = r.local(T::I64)?;
    let offset = r.local(T::I64)?;
    let list = r.local(T::Ptr)?;
    let some = r.block()?;
    let small_block = r.block()?;
    let larger = r.block()?;
    let medium_block = r.block()?;
    let push = r.block()?;
    let large = r.block()?;
    let none = r.block()?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, p0, zero)?;
    r.branch(condition, none, some)?;
    r.enter(none)?;
    r.ret_void()?;
    r.enter(some)?;
    let header = r.at_offset(p0, -HEADER)?;
    let size = r.load(T::I64, header)?;
    let limit = r.integer(SMALL)?;
    let condition = r.compare(C::Ule, T::I64, size, limit)?;
    r.branch(condition, small_block, larger)?;
    r.enter(small_block)?;
    let small_lists = r.address(small)?;
    let one = r.integer(1)?;
    let value = r.binary(B::UShr, T::I64, size, one)?;
    let address = r.at(small_lists, value)?;
    r.store(T::Ptr, address, list)?;
    r.jump(push)?;
    r.enter(larger)?;
    let limit = r.integer(MEDIUM)?;
    let condition = r.compare(C::Ugt, T::I64, size, limit)?;
    r.branch(condition, large, medium_block)?;
    r.enter(medium_block)?;
    size_class(&mut r, size, rounded, offset)?;
    let medium_lists = r.address(medium)?;
    let value = r.load(T::I64, offset)?;
    let address = r.at(medium_lists, value)?;
    r.store(T::Ptr, address, list)?;
    r.jump(push)?;
    r.enter(push)?;
    let address = r.load(T::Ptr, list)?;
    let value = r.load(T::Ptr, address)?;
    r.store(T::Ptr, value, p0)?;
    let address = r.load(T::Ptr, list)?;
    r.store(T::Ptr, p0, address)?;
    r.ret_void()?;
    r.enter(large)?;
    let h = r.integer(HEADER)?;
    let bytes = r.add(T::I64, size, h)?;
    r.call_void(munmap, &[header, bytes])?;
    r.ret_void()?;
    r.finish()?;
    Ok(())
}

fn emit_operator_new(
    runtime: &mut RuntimeProgram<'_>,
    operator_new: SymbolId,
    allocate: SymbolId,
) -> Result<(), Error> {
    let mut r = runtime.routine(operator_new)?;
    let p0 = r.parameter(0)?;
    let failed = r.block()?;
    let done = r.block()?;
    let block = r.call(T::Ptr, allocate, &[p0])?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::Ptr, block, zero)?;
    r.branch(condition, failed, done)?;
    r.enter(failed)?;
    let terminate = r
        .runtime()
        .function(b"_ZSt9terminatev", T::Void, &[], false)?;
    r.call_void(terminate, &[])?;
    r.unreachable()?;
    r.enter(done)?;
    r.ret(T::Ptr, block)?;
    r.finish()?;
    Ok(())
}
