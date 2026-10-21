//! Itanium catch matching and pointer qualification conversions. See NOTICE.
use super::program::{RuntimeProgram, Word};
use crate::lowir::{
    Error,
    vocabulary::{Binary as B, Compare as C, Convert as V, Type as T},
};

pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let kind = runtime.function(b"__cppgm_type_kind", T::I64, &[T::Ptr], false)?;
    let equal = runtime.function(b"__cppgm_type_equal", T::I64, &[T::Ptr, T::Ptr], false)?;
    let up = runtime.function(b"__cppgm_upcast", T::I64, &[T::Ptr, T::Ptr, T::Ptr], false)?;
    let symbol = runtime.function(
        b"__cppgm_do_catch",
        T::I64,
        &[T::Ptr, T::Ptr, T::Ptr, T::I64],
        true,
    )?;
    let nullptr_type = runtime.global(b"_ZTIDn", 0)?;
    let void_type = runtime.global(b"_ZTIv", 0)?;
    let null_data = runtime.words(
        b"__cppgm_null_data_member",
        &[Word {
            symbol: None,
            value: -1,
        }],
    )?;
    let null_function = runtime.words(
        b"__cppgm_null_function_member",
        &[Word {
            symbol: None,
            value: 0,
        }; 2],
    )?;
    let mut r = runtime.routine(symbol)?;
    let handler = r.parameter(0)?;
    let thrown = r.parameter(1)?;
    let object = r.parameter(2)?;
    let outer = r.local(T::I64)?;
    let p3 = r.parameter(3)?;
    r.store(T::I64, p3, outer)?;
    let yes = r.block()?;
    let no = r.block()?;
    let differ = r.block()?;
    let class_handler = r.block()?;
    let not_class = r.block()?;
    let shallow = r.block()?;
    let class_thrown = r.block()?;
    let pointerish = r.block()?;
    let condition = r.call(T::I64, equal, &[handler, thrown])?;
    r.branch(condition, yes, differ)?;
    r.enter(differ)?;
    let handler_kind = r.call(T::I64, kind, &[handler])?;
    let three = r.integer(3)?;
    let condition = r.compare(C::Ule, T::I64, handler_kind, three)?;
    r.branch(condition, class_handler, not_class)?;
    r.enter(class_handler)?;
    let known = r.block()?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::I64, handler_kind, zero)?;
    r.branch(condition, no, known)?;
    r.enter(known)?;
    let four = r.integer(4)?;
    let value = r.load(T::I64, outer)?;
    let condition = r.compare(C::Uge, T::I64, value, four)?;
    r.branch(condition, no, shallow)?;
    r.enter(shallow)?;
    let thrown_kind = r.call(T::I64, kind, &[thrown])?;
    let thrown_known = r.block()?;
    let three = r.integer(3)?;
    let condition = r.compare(C::Ule, T::I64, thrown_kind, three)?;
    r.branch(condition, thrown_known, no)?;
    r.enter(thrown_known)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::I64, thrown_kind, zero)?;
    r.branch(condition, no, class_thrown)?;
    r.enter(class_thrown)?;
    let value = r.call(T::I64, up, &[thrown, handler, object])?;
    r.ret(T::I64, value)?;
    r.enter(not_class)?;
    let pointer_like = r.block()?;
    let five = r.integer(5)?;
    let is_pointer = r.equal(T::I64, handler_kind, five)?;
    let six = r.integer(6)?;
    let is_member = r.equal(T::I64, handler_kind, six)?;
    let condition = r.binary(B::Or, T::I64, is_pointer, is_member)?;
    r.branch(condition, pointer_like, no)?;
    r.enter(pointer_like)?;
    let from_null = r.block()?;
    let not_null = r.block()?;
    let address = r.address(nullptr_type)?;
    let condition = r.call(T::I64, equal, &[thrown, address])?;
    r.branch(condition, from_null, not_null)?;
    r.enter(from_null)?;
    let to_pointer = r.block()?;
    let to_member = r.block()?;
    let to_function = r.block()?;
    let to_data = r.block()?;
    r.branch(is_pointer, to_pointer, to_member)?;
    r.enter(to_pointer)?;
    let zero = r.integer(0)?;
    r.store(T::Ptr, zero, object)?;
    let one = r.integer(1)?;
    r.ret(T::I64, one)?;
    r.enter(to_member)?;
    let value = r.load_offset(T::Ptr, handler, 24)?;
    let pointee_kind = r.call(T::I64, kind, &[value])?;
    let seven = r.integer(7)?;
    let condition = r.equal(T::I64, pointee_kind, seven)?;
    r.branch(condition, to_function, to_data)?;
    r.enter(to_function)?;
    let address = r.address(null_function)?;
    r.store(T::Ptr, address, object)?;
    let one = r.integer(1)?;
    r.ret(T::I64, one)?;
    r.enter(to_data)?;
    let address = r.address(null_data)?;
    r.store(T::Ptr, address, object)?;
    let one = r.integer(1)?;
    r.ret(T::I64, one)?;
    r.enter(not_null)?;
    let same_kind = r.block()?;
    let all_const = r.block()?;
    let qualified = r.block()?;
    let value = r.call(T::I64, kind, &[thrown])?;
    let condition = r.equal(T::I64, value, handler_kind)?;
    r.branch(condition, same_kind, no)?;
    r.enter(same_kind)?;
    let one = r.integer(1)?;
    let value = r.load(T::I64, outer)?;
    let condition = r.binary(B::And, T::I64, value, one)?;
    r.branch(condition, all_const, no)?;
    r.enter(all_const)?;
    let value = r.load_offset(T::U32, thrown, 16)?;
    let thrown_flags = r.convert(V::ZeroExtend, T::I64, T::U32, value)?;
    let value = r.load_offset(T::U32, handler, 16)?;
    let handler_flags = r.convert(V::ZeroExtend, T::I64, T::U32, value)?;
    let mask = r.integer(-1)?;
    let value = r.binary(B::Xor, T::I64, handler_flags, mask)?;
    let extra = r.binary(B::And, T::I64, thrown_flags, value)?;
    r.branch(extra, no, qualified)?;
    r.enter(qualified)?;
    let keep = r.block()?;
    let drop = r.block()?;
    let one = r.integer(1)?;
    let condition = r.binary(B::And, T::I64, handler_flags, one)?;
    r.branch(condition, keep, drop)?;
    r.enter(drop)?;
    let mask = r.integer(-2)?;
    let value = r.load(T::I64, outer)?;
    let value = r.binary(B::And, T::I64, value, mask)?;
    r.store(T::I64, value, outer)?;
    r.jump(keep)?;
    r.enter(keep)?;
    let member_class = r.block()?;
    let recurse = r.block()?;
    let first_level = r.block()?;
    r.branch(is_member, member_class, pointerish)?;
    r.enter(member_class)?;
    let a = r.load_offset(T::Ptr, handler, 32)?;
    let b = r.load_offset(T::Ptr, thrown, 32)?;
    let condition = r.call(T::I64, equal, &[a, b])?;
    r.branch(condition, recurse, no)?;
    r.enter(pointerish)?;
    let two = r.integer(2)?;
    let value = r.load(T::I64, outer)?;
    let condition = r.compare(C::Ult, T::I64, value, two)?;
    r.branch(condition, first_level, recurse)?;
    r.enter(first_level)?;
    let to_void = r.block()?;
    let pointee = r.load_offset(T::Ptr, handler, 24)?;
    let address = r.address(void_type)?;
    let condition = r.call(T::I64, equal, &[pointee, address])?;
    r.branch(condition, to_void, recurse)?;
    r.enter(to_void)?;
    let pointee = r.load_offset(T::Ptr, thrown, 24)?;
    let pointee = r.call(T::I64, kind, &[pointee])?;
    let seven = r.integer(7)?;
    let condition = r.equal(T::I64, pointee, seven)?;
    r.branch(condition, no, yes)?;
    r.enter(recurse)?;
    let a = r.load_offset(T::Ptr, handler, 24)?;
    let b = r.load_offset(T::Ptr, thrown, 24)?;
    let two = r.integer(2)?;
    let value = r.load(T::I64, outer)?;
    let value = r.add(T::I64, value, two)?;
    let value = r.call(T::I64, symbol, &[a, b, object, value])?;
    r.ret(T::I64, value)?;
    r.enter(yes)?;
    let one = r.integer(1)?;
    r.ret(T::I64, one)?;
    r.enter(no)?;
    let zero = r.integer(0)?;
    r.ret(T::I64, zero)?;
    r.finish()
}
