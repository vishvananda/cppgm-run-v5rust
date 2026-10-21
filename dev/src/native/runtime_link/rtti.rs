//! Course type identity and public base-subobject searches. See NOTICE.
use super::{
    VTABLES,
    program::{Routine, RuntimeProgram},
};
use crate::lowir::{
    Error,
    model::{SymbolId, Value},
    vocabulary::{Binary as B, Compare as C, Convert as V, Type as T},
};

pub(super) fn type_kind(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    if runtime.defined(b"__cppgm_type_kind")? {
        return Ok(());
    }
    let symbol = runtime.function(b"__cppgm_type_kind", T::I64, &[T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let p0 = r.parameter(0)?;
    let vptr = r.load(T::Ptr, p0)?;
    for (index, name) in VTABLES.iter().enumerate() {
        let found = r.block()?;
        let next = r.block()?;
        let table = r.runtime().global(name, 0)?;
        let address = r.address(table)?;
        let expected = r.at_offset(address, 16)?;
        let condition = r.equal(T::Ptr, vptr, expected)?;
        r.branch(condition, found, next)?;
        r.enter(found)?;
        let value = r.integer(i64::try_from(index).map_err(|_| Error::Limit)? + 1)?;
        r.ret(T::I64, value)?;
        r.enter(next)?;
    }
    let zero = r.integer(0)?;
    r.ret(T::I64, zero)?;
    r.finish()
}
pub(super) fn type_equal(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    if runtime.defined(b"__cppgm_type_equal")? {
        return Ok(());
    }
    let symbol = runtime.function(b"__cppgm_type_equal", T::I64, &[T::Ptr, T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let a = r.local(T::Ptr)?;
    let b = r.local(T::Ptr)?;
    let same = r.block()?;
    let differ = r.block()?;
    let names = r.block()?;
    let unique = r.block()?;
    let test = r.block()?;
    let step = r.block()?;
    let check_b = r.block()?;
    let compare_names = r.block()?;
    let ended = r.block()?;
    let p0 = r.parameter(0)?;
    let p1 = r.parameter(1)?;
    let condition = r.equal(T::Ptr, p0, p1)?;
    r.branch(condition, same, names)?;
    r.enter(names)?;
    let name_a = r.load_offset(T::Ptr, p0, 8)?;
    let name_b = r.load_offset(T::Ptr, p1, 8)?;
    r.store(T::Ptr, name_a, a)?;
    r.store(T::Ptr, name_b, b)?;
    let condition = r.equal(T::Ptr, name_a, name_b)?;
    r.branch(condition, same, unique)?;
    r.enter(unique)?;
    let star = r.integer(42)?;
    let value = r.load(T::I8, name_a)?;
    let condition = r.equal(T::I8, value, star)?;
    r.branch(condition, differ, check_b)?;
    r.enter(check_b)?;
    let star = r.integer(42)?;
    let value = r.load(T::I8, name_b)?;
    let condition = r.equal(T::I8, value, star)?;
    r.branch(condition, differ, test)?;
    r.enter(test)?;
    let address = r.load(T::Ptr, a)?;
    let left = r.load(T::I8, address)?;
    let address = r.load(T::Ptr, b)?;
    let right = r.load(T::I8, address)?;
    let condition = r.equal(T::I8, left, right)?;
    r.branch(condition, compare_names, differ)?;
    r.enter(compare_names)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::I8, left, zero)?;
    r.branch(condition, ended, step)?;
    r.enter(step)?;
    let address = r.load(T::Ptr, a)?;
    let value = r.at_offset(address, 1)?;
    r.store(T::Ptr, value, a)?;
    let address = r.load(T::Ptr, b)?;
    let value = r.at_offset(address, 1)?;
    r.store(T::Ptr, value, b)?;
    r.jump(test)?;
    r.enter(ended)?;
    r.jump(same)?;
    r.enter(same)?;
    let one = r.integer(1)?;
    r.ret(T::I64, one)?;
    r.enter(differ)?;
    let zero = r.integer(0)?;
    r.ret(T::I64, zero)?;
    r.finish()
}
// Address formation deliberately remains in the caller's expression order.
pub(super) fn load_global(r: &mut Routine<'_>, ty: T, symbol: SymbolId) -> Result<Value, Error> {
    let address = r.address(symbol)?;
    r.load(ty, address)
}
pub(super) fn store_global(
    r: &mut Routine<'_>,
    ty: T,
    value: Value,
    symbol: SymbolId,
) -> Result<(), Error> {
    let address = r.address(symbol)?;
    r.store(ty, value, address)
}
pub(super) fn find_base(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let target = runtime.global(b"__cppgm_search_target", 8)?;
    let count = runtime.global(b"__cppgm_search_count", 8)?;
    let found = runtime.global(b"__cppgm_search_address", 8)?;
    let reached = runtime.global(b"__cppgm_search_public", 8)?;
    let ambiguous = runtime.global(b"__cppgm_search_ambiguous", 8)?;
    let kind = runtime.function(b"__cppgm_type_kind", T::I64, &[T::Ptr], false)?;
    let equal = runtime.function(b"__cppgm_type_equal", T::I64, &[T::Ptr, T::Ptr], false)?;
    let symbol = runtime.function(
        b"__cppgm_find_base",
        T::Void,
        &[T::Ptr, T::Ptr, T::I64, T::I64],
        true,
    )?;
    let mut r = runtime.routine(symbol)?;
    let ty = r.parameter(0)?;
    let object = r.parameter(1)?;
    let open = r.parameter(2)?;
    let real = r.parameter(3)?;
    let matched = r.block()?;
    let bases = r.block()?;
    let first = r.block()?;
    let again = r.block()?;
    let other = r.block()?;
    let same = r.block()?;
    let done = r.block()?;
    let value = load_global(&mut r, T::Ptr, target)?;
    let condition = r.call(T::I64, equal, &[ty, value])?;
    r.branch(condition, matched, bases)?;
    r.enter(matched)?;
    let zero = r.integer(0)?;
    let value = load_global(&mut r, T::I64, count)?;
    let condition = r.equal(T::I64, value, zero)?;
    r.branch(condition, first, again)?;
    r.enter(first)?;
    let address = r.address(count)?;
    let one = r.integer(1)?;
    r.store(T::I64, one, address)?;
    store_global(&mut r, T::Ptr, object, found)?;
    store_global(&mut r, T::I64, open, reached)?;
    r.ret_void()?;
    r.enter(again)?;
    let value = load_global(&mut r, T::Ptr, found)?;
    let condition = r.equal(T::Ptr, value, object)?;
    r.branch(condition, same, other)?;
    r.enter(other)?;
    let address = r.address(ambiguous)?;
    let one = r.integer(1)?;
    r.store(T::I64, one, address)?;
    r.ret_void()?;
    r.enter(same)?;
    let value = load_global(&mut r, T::I64, reached)?;
    let now_open = r.binary(B::Or, T::I64, value, open)?;
    store_global(&mut r, T::I64, now_open, reached)?;
    r.ret_void()?;
    r.enter(bases)?;
    let code = r.call(T::I64, kind, &[ty])?;
    let single = r.block()?;
    let multiple = r.block()?;
    let not_single = r.block()?;
    let two = r.integer(2)?;
    let condition = r.equal(T::I64, code, two)?;
    r.branch(condition, single, not_single)?;
    r.enter(single)?;
    let base = r.load_offset(T::Ptr, ty, 16)?;
    r.call_void(symbol, &[base, object, open, real])?;
    r.ret_void()?;
    r.enter(not_single)?;
    let three = r.integer(3)?;
    let condition = r.equal(T::I64, code, three)?;
    r.branch(condition, multiple, done)?;
    r.enter(multiple)?;
    let index = r.local(T::I64)?;
    let child = r.local(T::Ptr)?;
    let zero = r.integer(0)?;
    r.store(T::I64, zero, index)?;
    let total = r.load_offset(T::U32, ty, 20)?;
    let total = r.convert(V::ZeroExtend, T::I64, T::U32, total)?;
    let test = r.block()?;
    let body = r.block()?;
    let located = r.block()?;
    let present = r.block()?;
    let virtual_base = r.block()?;
    let direct = r.block()?;
    let stand_in = r.block()?;
    let stand_in_virtual = r.block()?;
    r.jump(test)?;
    r.enter(test)?;
    let value = r.load(T::I64, index)?;
    let condition = r.compare(C::Ult, T::I64, value, total)?;
    r.branch(condition, body, done)?;
    r.enter(body)?;
    let bases_start = r.at_offset(ty, 24)?;
    let sixteen = r.integer(16)?;
    let value = r.load(T::I64, index)?;
    let value = r.binary(B::Mul, T::I64, value, sixteen)?;
    let entry = r.at(bases_start, value)?;
    let flags = r.load_offset(T::I64, entry, 8)?;
    let eight = r.integer(8)?;
    let offset = r.binary(B::Shr, T::I64, flags, eight)?;
    let one = r.integer(1)?;
    let is_virtual = r.binary(B::And, T::I64, flags, one)?;
    r.branch(real, present, stand_in)?;
    r.enter(stand_in)?;
    r.branch(is_virtual, stand_in_virtual, direct)?;
    r.enter(stand_in_virtual)?;
    let sixteen = r.integer(16)?;
    let value = r.load(T::Ptr, entry)?;
    let value = r.binary(B::Shl, T::I64, value, sixteen)?;
    r.store(T::Ptr, value, child)?;
    r.jump(located)?;
    r.enter(present)?;
    r.branch(is_virtual, virtual_base, direct)?;
    r.enter(virtual_base)?;
    let value = r.load(T::Ptr, object)?;
    let value = r.at(value, offset)?;
    let value = r.load(T::I64, value)?;
    let value = r.at(object, value)?;
    r.store(T::Ptr, value, child)?;
    r.jump(located)?;
    r.enter(direct)?;
    let value = r.at(object, offset)?;
    r.store(T::Ptr, value, child)?;
    r.jump(located)?;
    r.enter(located)?;
    let one = r.integer(1)?;
    let two = r.integer(2)?;
    let value = r.binary(B::And, T::I64, flags, two)?;
    let value = r.binary(B::UShr, T::I64, value, one)?;
    let inherited = r.binary(B::And, T::I64, open, value)?;
    let base = r.load(T::Ptr, entry)?;
    let address = r.load(T::Ptr, child)?;
    r.call_void(symbol, &[base, address, inherited, real])?;
    let one = r.integer(1)?;
    let value = r.load(T::I64, index)?;
    let value = r.add(T::I64, value, one)?;
    r.store(T::I64, value, index)?;
    r.jump(test)?;
    r.enter(done)?;
    r.ret_void()?;
    r.finish()
}
pub(super) fn upcast(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let find = runtime.function(
        b"__cppgm_find_base",
        T::Void,
        &[T::Ptr, T::Ptr, T::I64, T::I64],
        false,
    )?;
    let symbol = runtime.function(b"__cppgm_upcast", T::I64, &[T::Ptr, T::Ptr, T::Ptr], true)?;
    let mut r = runtime.routine(symbol)?;
    let target = r.runtime().global(b"__cppgm_search_target", 8)?;
    let count = r.runtime().global(b"__cppgm_search_count", 8)?;
    let found = r.runtime().global(b"__cppgm_search_address", 8)?;
    let reached = r.runtime().global(b"__cppgm_search_public", 8)?;
    let ambiguous = r.runtime().global(b"__cppgm_search_ambiguous", 8)?;
    let p0 = r.parameter(0)?;
    let p1 = r.parameter(1)?;
    let p2 = r.parameter(2)?;
    store_global(&mut r, T::Ptr, p1, target)?;
    for symbol in [count, reached, ambiguous] {
        let address = r.address(symbol)?;
        let zero = r.integer(0)?;
        r.store(T::I64, zero, address)?;
    }
    let object = r.load(T::Ptr, p2)?;
    let zero = r.integer(0)?;
    let real = r.compare(C::Ne, T::Ptr, object, zero)?;
    let one = r.integer(1)?;
    let flag = r.convert(V::ZeroExtend, T::I64, T::I1, real)?;
    r.call_void(find, &[p0, object, one, flag])?;
    let some = r.block()?;
    let unique = r.block()?;
    let open = r.block()?;
    let fail = r.block()?;
    let condition = load_global(&mut r, T::I64, count)?;
    r.branch(condition, some, fail)?;
    r.enter(some)?;
    let condition = load_global(&mut r, T::I64, ambiguous)?;
    r.branch(condition, fail, unique)?;
    r.enter(unique)?;
    let condition = load_global(&mut r, T::I64, reached)?;
    r.branch(condition, open, fail)?;
    r.enter(open)?;
    let converted = r.block()?;
    let null = r.block()?;
    r.branch(real, converted, null)?;
    r.enter(converted)?;
    let value = load_global(&mut r, T::Ptr, found)?;
    r.store(T::Ptr, value, p2)?;
    r.jump(null)?;
    r.enter(null)?;
    let one = r.integer(1)?;
    r.ret(T::I64, one)?;
    r.enter(fail)?;
    let zero = r.integer(0)?;
    r.ret(T::I64, zero)?;
    r.finish()
}
