//! Dynamic-cast subobject traversal and ambiguity accounting. See NOTICE.
use super::{
    program::RuntimeProgram,
    rtti::{load_global, store_global},
};
use crate::lowir::{
    Error,
    model::SymbolId,
    vocabulary::{Binary as B, Compare as C, Convert as V, Type as T},
};
#[derive(Clone, Copy)]
struct State {
    target: SymbolId,
    source: SymbolId,
    source_type: SymbolId,
    source_public: SymbolId,
    down_count: SymbolId,
    down_address: SymbolId,
    cross_count: SymbolId,
    cross_address: SymbolId,
    cross_public: SymbolId,
}
fn walk(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let equal = runtime.function(b"__cppgm_type_equal", T::I64, &[T::Ptr, T::Ptr], false)?;
    let kind = runtime.function(b"__cppgm_type_kind", T::I64, &[T::Ptr], false)?;
    let s = State {
        target: runtime.global(b"__cppgm_cast_target", 8)?,
        source: runtime.global(b"__cppgm_cast_source", 8)?,
        source_type: runtime.global(b"__cppgm_cast_source_type", 8)?,
        source_public: runtime.global(b"__cppgm_cast_source_public", 8)?,
        down_count: runtime.global(b"__cppgm_cast_down_count", 8)?,
        down_address: runtime.global(b"__cppgm_cast_down_address", 8)?,
        cross_count: runtime.global(b"__cppgm_cast_cross_count", 8)?,
        cross_address: runtime.global(b"__cppgm_cast_cross_address", 8)?,
        cross_public: runtime.global(b"__cppgm_cast_cross_public", 8)?,
    };
    let symbol = runtime.function(
        b"__cppgm_cast_walk",
        T::Void,
        &[T::Ptr, T::Ptr, T::I64, T::Ptr, T::I64],
        true,
    )?;
    let mut r = runtime.routine(symbol)?;
    let ty = r.parameter(0)?;
    let object = r.parameter(1)?;
    let open = r.parameter(2)?;
    let derived = r.local(T::Ptr)?;
    let derived_open = r.local(T::I64)?;
    let p3 = r.parameter(3)?;
    r.store(T::Ptr, p3, derived)?;
    let p4 = r.parameter(4)?;
    r.store(T::I64, p4, derived_open)?;
    let is_target = r.block()?;
    let source_check = r.block()?;
    let first = r.block()?;
    let again = r.block()?;
    let other = r.block()?;
    let same = r.block()?;
    let target = load_global(&mut r, T::Ptr, s.target)?;
    let condition = r.call(T::I64, equal, &[ty, target])?;
    r.branch(condition, is_target, source_check)?;
    r.enter(is_target)?;
    r.store(T::Ptr, object, derived)?;
    let one = r.integer(1)?;
    r.store(T::I64, one, derived_open)?;
    let zero = r.integer(0)?;
    let count = load_global(&mut r, T::I64, s.cross_count)?;
    let condition = r.equal(T::I64, count, zero)?;
    r.branch(condition, first, again)?;
    r.enter(first)?;
    let address = r.address(s.cross_count)?;
    let one = r.integer(1)?;
    r.store(T::I64, one, address)?;
    store_global(&mut r, T::Ptr, object, s.cross_address)?;
    store_global(&mut r, T::I64, open, s.cross_public)?;
    r.jump(source_check)?;
    r.enter(again)?;
    let address = load_global(&mut r, T::Ptr, s.cross_address)?;
    let condition = r.equal(T::Ptr, address, object)?;
    r.branch(condition, same, other)?;
    r.enter(other)?;
    let address = r.address(s.cross_count)?;
    let two = r.integer(2)?;
    r.store(T::I64, two, address)?;
    r.jump(source_check)?;
    r.enter(same)?;
    let value = load_global(&mut r, T::I64, s.cross_public)?;
    let value = r.binary(B::Or, T::I64, value, open)?;
    store_global(&mut r, T::I64, value, s.cross_public)?;
    r.jump(source_check)?;
    r.enter(source_check)?;
    let at_source = r.block()?;
    let source_type_check = r.block()?;
    let bases = r.block()?;
    let source = load_global(&mut r, T::Ptr, s.source)?;
    let condition = r.equal(T::Ptr, object, source)?;
    r.branch(condition, source_type_check, bases)?;
    r.enter(source_type_check)?;
    let source_type = load_global(&mut r, T::Ptr, s.source_type)?;
    let condition = r.call(T::I64, equal, &[ty, source_type])?;
    r.branch(condition, at_source, bases)?;
    r.enter(at_source)?;
    let value = load_global(&mut r, T::I64, s.source_public)?;
    let value = r.binary(B::Or, T::I64, value, open)?;
    store_global(&mut r, T::I64, value, s.source_public)?;
    let inside = r.block()?;
    let record = r.block()?;
    let first_down = r.block()?;
    let next_down = r.block()?;
    let other_down = r.block()?;
    let zero = r.integer(0)?;
    let value = r.load(T::Ptr, derived)?;
    let condition = r.equal(T::Ptr, value, zero)?;
    r.branch(condition, bases, inside)?;
    r.enter(inside)?;
    let condition = r.load(T::I64, derived_open)?;
    r.branch(condition, record, bases)?;
    r.enter(record)?;
    let zero = r.integer(0)?;
    let count = load_global(&mut r, T::I64, s.down_count)?;
    let condition = r.equal(T::I64, count, zero)?;
    r.branch(condition, first_down, next_down)?;
    r.enter(first_down)?;
    let address = r.address(s.down_count)?;
    let one = r.integer(1)?;
    r.store(T::I64, one, address)?;
    let value = r.load(T::Ptr, derived)?;
    store_global(&mut r, T::Ptr, value, s.down_address)?;
    r.jump(bases)?;
    r.enter(next_down)?;
    let recorded = load_global(&mut r, T::Ptr, s.down_address)?;
    let value = r.load(T::Ptr, derived)?;
    let condition = r.equal(T::Ptr, recorded, value)?;
    r.branch(condition, bases, other_down)?;
    r.enter(other_down)?;
    let address = r.address(s.down_count)?;
    let two = r.integer(2)?;
    r.store(T::I64, two, address)?;
    r.jump(bases)?;
    r.enter(bases)?;
    let code = r.call(T::I64, kind, &[ty])?;
    let single = r.block()?;
    let not_single = r.block()?;
    let multiple = r.block()?;
    let done = r.block()?;
    let two = r.integer(2)?;
    let condition = r.equal(T::I64, code, two)?;
    r.branch(condition, single, not_single)?;
    r.enter(single)?;
    let base = r.load_offset(T::Ptr, ty, 16)?;
    let d = r.load(T::Ptr, derived)?;
    let d_open = r.load(T::I64, derived_open)?;
    r.call_void(symbol, &[base, object, open, d, d_open])?;
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
    let virtual_base = r.block()?;
    let direct = r.block()?;
    let located = r.block()?;
    r.jump(test)?;
    r.enter(test)?;
    let value = r.load(T::I64, index)?;
    let condition = r.compare(C::Ult, T::I64, value, total)?;
    r.branch(condition, body, done)?;
    r.enter(body)?;
    let start = r.at_offset(ty, 24)?;
    let sixteen = r.integer(16)?;
    let value = r.load(T::I64, index)?;
    let value = r.binary(B::Mul, T::I64, value, sixteen)?;
    let entry = r.at(start, value)?;
    let flags = r.load_offset(T::I64, entry, 8)?;
    let eight = r.integer(8)?;
    let offset = r.binary(B::Shr, T::I64, flags, eight)?;
    let one = r.integer(1)?;
    let condition = r.binary(B::And, T::I64, flags, one)?;
    r.branch(condition, virtual_base, direct)?;
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
    let base_public = r.binary(B::UShr, T::I64, value, one)?;
    let base = r.load(T::Ptr, entry)?;
    let child_value = r.load(T::Ptr, child)?;
    let inherited = r.binary(B::And, T::I64, open, base_public)?;
    let d = r.load(T::Ptr, derived)?;
    let d_open = r.load(T::I64, derived_open)?;
    let d_open = r.binary(B::And, T::I64, d_open, base_public)?;
    r.call_void(symbol, &[base, child_value, inherited, d, d_open])?;
    let one = r.integer(1)?;
    let value = r.load(T::I64, index)?;
    let value = r.add(T::I64, value, one)?;
    r.store(T::I64, value, index)?;
    r.jump(test)?;
    r.enter(done)?;
    r.ret_void()?;
    r.finish()
}
pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    super::rtti::type_kind(runtime)?;
    super::rtti::type_equal(runtime)?;
    walk(runtime)?;
    let walk = runtime.function(
        b"__cppgm_cast_walk",
        T::Void,
        &[T::Ptr, T::Ptr, T::I64, T::Ptr, T::I64],
        false,
    )?;
    let symbol = runtime.function(
        b"__dynamic_cast",
        T::Ptr,
        &[T::Ptr, T::Ptr, T::Ptr, T::I64],
        true,
    )?;
    let mut r = runtime.routine(symbol)?;
    let null = r.block()?;
    let cast = r.block()?;
    let zero = r.integer(0)?;
    let p0 = r.parameter(0)?;
    let condition = r.equal(T::Ptr, p0, zero)?;
    r.branch(condition, null, cast)?;
    r.enter(null)?;
    let zero = r.integer(0)?;
    r.ret(T::Ptr, zero)?;
    r.enter(cast)?;
    for name in [
        b"__cppgm_cast_source_public".as_slice(),
        b"__cppgm_cast_down_count",
        b"__cppgm_cast_cross_count",
        b"__cppgm_cast_cross_public",
    ] {
        let state = r.runtime().global(name, 8)?;
        let address = r.address(state)?;
        let zero = r.integer(0)?;
        r.store(T::I64, zero, address)?;
    }
    for (index, name) in [
        b"__cppgm_cast_source".as_slice(),
        b"__cppgm_cast_source_type",
        b"__cppgm_cast_target",
    ]
    .into_iter()
    .enumerate()
    {
        let state = r.runtime().global(name, 8)?;
        let address = r.address(state)?;
        let value = r.parameter(index)?;
        r.store(T::Ptr, value, address)?;
    }
    let vtable = r.load(T::Ptr, p0)?;
    let offset = r.load_offset(T::I64, vtable, -16)?;
    let whole = r.at(p0, offset)?;
    let ty = r.load_offset(T::Ptr, vtable, -8)?;
    let one = r.integer(1)?;
    let zero = r.integer(0)?;
    let null_value = r.integer(0)?;
    r.call_void(walk, &[ty, whole, one, zero, null_value])?;
    let down = r.block()?;
    let not_down = r.block()?;
    let source_open = r.block()?;
    let unique = r.block()?;
    let open = r.block()?;
    let fail = r.block()?;
    let one = r.integer(1)?;
    let state = r.runtime().global(b"__cppgm_cast_down_count", 8)?;
    let count = load_global(&mut r, T::I64, state)?;
    let condition = r.equal(T::I64, count, one)?;
    r.branch(condition, down, not_down)?;
    r.enter(down)?;
    let state = r.runtime().global(b"__cppgm_cast_down_address", 8)?;
    let value = load_global(&mut r, T::Ptr, state)?;
    r.ret(T::Ptr, value)?;
    r.enter(not_down)?;
    let state = r.runtime().global(b"__cppgm_cast_source_public", 8)?;
    let condition = load_global(&mut r, T::I64, state)?;
    r.branch(condition, source_open, fail)?;
    r.enter(source_open)?;
    let one = r.integer(1)?;
    let state = r.runtime().global(b"__cppgm_cast_cross_count", 8)?;
    let count = load_global(&mut r, T::I64, state)?;
    let condition = r.equal(T::I64, count, one)?;
    r.branch(condition, unique, fail)?;
    r.enter(unique)?;
    let state = r.runtime().global(b"__cppgm_cast_cross_public", 8)?;
    let condition = load_global(&mut r, T::I64, state)?;
    r.branch(condition, open, fail)?;
    r.enter(open)?;
    let state = r.runtime().global(b"__cppgm_cast_cross_address", 8)?;
    let value = load_global(&mut r, T::Ptr, state)?;
    r.ret(T::Ptr, value)?;
    r.enter(fail)?;
    let zero = r.integer(0)?;
    r.ret(T::Ptr, zero)?;
    r.finish()
}
