//! Runtime conversions between two-word integers and IEEE/x87 values. See NOTICE.
use super::program::{Routine, RuntimeProgram};
use crate::lowir::{
    Error,
    literals::FloatKind,
    model::{SymbolId, Value},
    vocabulary::{Binary as B, Compare as C, Convert as V, Type as T},
};
const UNSIGNED_TO: [&[u8]; 3] = [b"__floatuntisf", b"__floatuntidf", b"__floatuntixf"];
const SIGNED_TO: [&[u8]; 3] = [b"__floattisf", b"__floattidf", b"__floattixf"];
const TO_UNSIGNED: [&[u8]; 3] = [b"__fixunssfti", b"__fixunsdfti", b"__fixunsxfti"];
const TO_SIGNED: [&[u8]; 3] = [b"__fixsfti", b"__fixdfti", b"__fixxfti"];
const FLOATS: [T; 3] = [T::F32, T::F64, T::F80];
// Runtime literals are exact powers of two, with a sign and no fraction.
fn floating(r: &mut Routine<'_>, ty: T, exponent: i32, negative: bool) -> Result<Value, Error> {
    let (kind, bits) = match ty {
        T::F32 => (
            FloatKind::F32,
            u128::from(
                u32::try_from(127 + exponent).map_err(|_| Error::Limit)? << 23
                    | u32::from(negative) << 31,
            ),
        ),
        T::F64 => (
            FloatKind::F64,
            u128::from(
                u64::try_from(1023 + exponent).map_err(|_| Error::Limit)? << 52
                    | u64::from(negative) << 63,
            ),
        ),
        T::F80 => (
            FloatKind::F80,
            (u128::try_from(16383 + exponent).map_err(|_| Error::Limit)?
                | u128::from(negative) << 15)
                << 64
                | 1 << 63,
        ),
        _ => return Err(Error::Limit),
    };
    r.floating(kind, bits)
}
fn zero(r: &mut Routine<'_>, ty: T) -> Result<Value, Error> {
    let kind = match ty {
        T::F32 => FloatKind::F32,
        T::F64 => FloatKind::F64,
        T::F80 => FloatKind::F80,
        _ => return Err(Error::Limit),
    };
    r.floating(kind, 0)
}
fn unsigned_to(runtime: &mut RuntimeProgram<'_>, index: usize) -> Result<SymbolId, Error> {
    let f = FLOATS[index];
    let symbol = runtime.function(UNSIGNED_TO[index], f, &[T::I128], true)?;
    let mut r = runtime.routine(symbol)?;
    let u = r.parameter(0)?;
    let low = r.convert(V::Truncate, T::I64, T::I128, u)?;
    let sixty_four = r.integer(64)?;
    let value = r.binary(B::UShr, T::I128, u, sixty_four)?;
    let high = r.convert(V::Truncate, T::I64, T::I128, value)?;
    let narrow = r.block()?;
    let wide = r.block()?;
    let null = r.integer(0)?;
    let condition = r.equal(T::I64, high, null)?;
    r.branch(condition, narrow, wide)?;
    r.enter(narrow)?;
    let value = r.convert(V::UnsignedToFloat, f, T::I64, low)?;
    r.ret(f, value)?;
    r.enter(wide)?;
    if f == T::F80 {
        let scale = floating(&mut r, f, 64, false)?;
        let value = r.convert(V::UnsignedToFloat, f, T::I64, high)?;
        let scaled = r.binary(B::Mul, f, value, scale)?;
        let value = r.convert(V::UnsignedToFloat, f, T::I64, low)?;
        let value = r.add(f, scaled, value)?;
        r.ret(f, value)?;
        r.finish()?;
        return Ok(symbol);
    }
    let length = r.local(T::I64)?;
    let rest = r.local(T::I64)?;
    let null = r.integer(0)?;
    r.store(T::I64, null, length)?;
    r.store(T::I64, high, rest)?;
    let test = r.block()?;
    let step = r.block()?;
    let done = r.block()?;
    r.jump(test)?;
    r.enter(test)?;
    let null = r.integer(0)?;
    let value = r.load(T::I64, rest)?;
    let condition = r.equal(T::I64, value, null)?;
    r.branch(condition, done, step)?;
    r.enter(step)?;
    let one = r.integer(1)?;
    let value = r.load(T::I64, rest)?;
    let value = r.binary(B::UShr, T::I64, value, one)?;
    r.store(T::I64, value, rest)?;
    let one = r.integer(1)?;
    let value = r.load(T::I64, length)?;
    let value = r.add(T::I64, value, one)?;
    r.store(T::I64, value, length)?;
    r.jump(test)?;
    r.enter(done)?;
    let s = r.load(T::I64, length)?;
    let value = r.binary(B::UShr, T::I128, u, s)?;
    let shifted = r.convert(V::Truncate, T::I64, T::I128, value)?;
    let bits = r.integer(128)?;
    let value = r.binary(B::Sub, T::I64, bits, s)?;
    let lost = r.binary(B::Shl, T::I128, u, value)?;
    let null = r.integer(0)?;
    let sticky = r.compare(C::Ne, T::I128, lost, null)?;
    let value = r.binary(B::Or, T::I64, shifted, sticky)?;
    let rounded = r.convert(V::UnsignedToFloat, f, T::I64, value)?;
    let one = r.integer(1)?;
    let value = r.binary(B::Sub, T::I64, s, one)?;
    let one = r.integer(1)?;
    let value = r.binary(B::Shl, T::I64, one, value)?;
    let scale = r.convert(V::UnsignedToFloat, f, T::I64, value)?;
    let two = floating(&mut r, f, 1, false)?;
    let value = r.binary(B::Mul, f, rounded, scale)?;
    let value = r.binary(B::Mul, f, value, two)?;
    r.ret(f, value)?;
    r.finish()?;
    Ok(symbol)
}
fn signed_to(
    runtime: &mut RuntimeProgram<'_>,
    index: usize,
    unsigned: SymbolId,
) -> Result<(), Error> {
    let f = FLOATS[index];
    let symbol = runtime.function(SIGNED_TO[index], f, &[T::I128], true)?;
    let mut r = runtime.routine(symbol)?;
    let v = r.parameter(0)?;
    let low = r.convert(V::Truncate, T::I64, T::I128, v)?;
    let narrow = r.block()?;
    let wide = r.block()?;
    let negative = r.block()?;
    let positive = r.block()?;
    let extended = r.convert(V::SignExtend, T::I128, T::I64, low)?;
    let condition = r.equal(T::I128, extended, v)?;
    r.branch(condition, narrow, wide)?;
    r.enter(narrow)?;
    let value = r.convert(V::SignedToFloat, f, T::I64, low)?;
    r.ret(f, value)?;
    r.enter(wide)?;
    let null = r.integer(0)?;
    let condition = r.compare(C::Lt, T::I128, v, null)?;
    r.branch(condition, negative, positive)?;
    r.enter(negative)?;
    let null = r.integer(0)?;
    let value = r.binary(B::Sub, T::I128, null, v)?;
    let magnitude = r.call(f, unsigned, &[value])?;
    let null = zero(&mut r, f)?;
    let value = r.binary(B::Sub, f, null, magnitude)?;
    r.ret(f, value)?;
    r.enter(positive)?;
    let value = r.call(f, unsigned, &[v])?;
    r.ret(f, value)?;
    r.finish()
}
fn decompose(r: &mut Routine<'_>, kind: T, x: Value) -> Result<(Value, Value, Value), Error> {
    let image = r.local(kind)?;
    r.store(kind, x, image)?;
    let address = r.slot_address(image)?;
    if kind == T::F64 {
        let bits = r.load(T::I64, address)?;
        let mask = r.integer(0x7ff)?;
        let shift = r.integer(52)?;
        let value = r.binary(B::UShr, T::I64, bits, shift)?;
        let biased = r.binary(B::And, T::I64, value, mask)?;
        let leading = r.integer(1 << 52)?;
        let mask = r.integer((1 << 52) - 1)?;
        let value = r.binary(B::And, T::I64, bits, mask)?;
        let significand = r.binary(B::Or, T::I64, value, leading)?;
        let exponent_bias = r.integer(1023)?;
        let top = r.binary(B::Sub, T::I64, biased, exponent_bias)?;
        let count = r.integer(52)?;
        let lowest = r.binary(B::Sub, T::I64, top, count)?;
        return Ok((significand, top, lowest));
    }
    let significand = r.load(T::I64, address)?;
    let mask = r.integer(0x7fff)?;
    let value = r.load_offset(T::U16, address, 8)?;
    let value = r.convert(V::ZeroExtend, T::I64, T::U16, value)?;
    let biased = r.binary(B::And, T::I64, value, mask)?;
    let exponent_bias = r.integer(16383)?;
    let top = r.binary(B::Sub, T::I64, biased, exponent_bias)?;
    let count = r.integer(63)?;
    let lowest = r.binary(B::Sub, T::I64, top, count)?;
    Ok((significand, top, lowest))
}
fn to_unsigned(runtime: &mut RuntimeProgram<'_>, index: usize) -> Result<SymbolId, Error> {
    let f = FLOATS[index];
    let symbol = runtime.function(TO_UNSIGNED[index], T::I128, &[f], true)?;
    let mut r = runtime.routine(symbol)?;
    let x = r.parameter(0)?;
    let small = r.block()?;
    let large = r.block()?;
    let direct = r.block()?;
    let null = r.block()?;
    let shift = r.block()?;
    let saturate = r.block()?;
    let bound = floating(&mut r, f, 64, false)?;
    let condition = r.compare(C::Lt, f, x, bound)?;
    r.branch(condition, small, large)?;
    r.enter(small)?;
    let bound = floating(&mut r, f, 0, true)?;
    let condition = r.compare(C::Gt, f, x, bound)?;
    r.branch(condition, direct, null)?;
    r.enter(direct)?;
    let value = r.convert(V::FloatToUnsigned, T::I64, f, x)?;
    let value = r.convert(V::ZeroExtend, T::I128, T::I64, value)?;
    r.ret(T::I128, value)?;
    r.enter(null)?;
    let value = r.integer(0)?;
    r.ret(T::I128, value)?;
    r.enter(large)?;
    let (significand, top, lowest) = decompose(&mut r, f, x)?;
    let limit = r.integer(128)?;
    let condition = r.compare(C::Lt, T::I64, top, limit)?;
    r.branch(condition, shift, saturate)?;
    r.enter(shift)?;
    let value = r.convert(V::ZeroExtend, T::I128, T::I64, significand)?;
    let value = r.binary(B::Shl, T::I128, value, lowest)?;
    r.ret(T::I128, value)?;
    r.enter(saturate)?;
    let value = r.integer(-1)?;
    r.ret(T::I128, value)?;
    r.finish()?;
    Ok(symbol)
}
fn to_signed(
    runtime: &mut RuntimeProgram<'_>,
    index: usize,
    unsigned: SymbolId,
) -> Result<SymbolId, Error> {
    let f = FLOATS[index];
    let symbol = runtime.function(TO_SIGNED[index], T::I128, &[f], true)?;
    let mut r = runtime.routine(symbol)?;
    let x = r.parameter(0)?;
    let below = r.block()?;
    let small = r.block()?;
    let large = r.block()?;
    let negative = r.block()?;
    let positive = r.block()?;
    let bound = floating(&mut r, f, 63, false)?;
    let condition = r.compare(C::Lt, f, x, bound)?;
    r.branch(condition, below, large)?;
    r.enter(below)?;
    let bound = floating(&mut r, f, 63, true)?;
    let condition = r.compare(C::Ge, f, x, bound)?;
    r.branch(condition, small, large)?;
    r.enter(small)?;
    let value = r.convert(V::FloatToSigned, T::I64, f, x)?;
    let value = r.convert(V::SignExtend, T::I128, T::I64, value)?;
    r.ret(T::I128, value)?;
    r.enter(large)?;
    let null = zero(&mut r, f)?;
    let condition = r.compare(C::Lt, f, x, null)?;
    r.branch(condition, negative, positive)?;
    r.enter(negative)?;
    let null = zero(&mut r, f)?;
    let value = r.binary(B::Sub, f, null, x)?;
    let magnitude = r.call(T::I128, unsigned, &[value])?;
    let null = r.integer(0)?;
    let value = r.binary(B::Sub, T::I128, null, magnitude)?;
    r.ret(T::I128, value)?;
    r.enter(positive)?;
    let value = r.call(T::I128, unsigned, &[x])?;
    r.ret(T::I128, value)?;
    r.finish()?;
    Ok(symbol)
}
pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let mut unsigned = Vec::with_capacity(3);
    for index in 0..3 {
        unsigned.push(unsigned_to(runtime, index)?);
    }
    for (index, symbol) in unsigned.into_iter().enumerate() {
        signed_to(runtime, index, symbol)?;
    }
    let mut unsigned = Vec::with_capacity(2);
    for index in 1..3 {
        unsigned.push(to_unsigned(runtime, index)?);
    }
    let mut signed = Vec::with_capacity(2);
    for (index, symbol) in unsigned.iter().copied().enumerate() {
        signed.push(to_signed(runtime, index + 1, symbol)?);
    }
    for (names, target) in [(TO_SIGNED, signed[0]), (TO_UNSIGNED, unsigned[0])] {
        let symbol = runtime.function(names[0], T::I128, &[T::F32], true)?;
        let mut r = runtime.routine(symbol)?;
        let p0 = r.parameter(0)?;
        let wide = r.convert(V::FloatExtend, T::F64, T::F32, p0)?;
        let value = r.call(T::I128, target, &[wide])?;
        r.ret(T::I128, value)?;
        r.finish()?;
    }
    Ok(())
}
