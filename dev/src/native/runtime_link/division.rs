//! Course runtime restoring division over two-word integers. See NOTICE.
use super::program::RuntimeProgram;
use crate::lowir::{
    Error,
    model::SymbolId,
    vocabulary::{Binary as B, Compare as C, Convert as V, Type as T},
};

fn unsigned_core(runtime: &mut RuntimeProgram<'_>) -> Result<SymbolId, Error> {
    let core = runtime.function(
        b"__cppgm_udivmod128",
        T::I128,
        &[T::I128, T::I128, T::I64],
        true,
    )?;
    let mut r = runtime.routine(core)?;
    let n = r.parameter(0)?;
    let d = r.parameter(1)?;
    let remainder = r.parameter(2)?;
    let narrow = r.block()?;
    let wide = r.block()?;
    let narrow_remainder = r.block()?;
    let narrow_quotient = r.block()?;
    let sixty_four = r.integer(64)?;
    let n_high = r.binary(B::UShr, T::I128, n, sixty_four)?;
    let sixty_four = r.integer(64)?;
    let d_high = r.binary(B::UShr, T::I128, d, sixty_four)?;
    let high = r.binary(B::Or, T::I128, n_high, d_high)?;
    let zero = r.integer(0)?;
    let condition = r.equal(T::I128, high, zero)?;
    r.branch(condition, narrow, wide)?;
    r.enter(narrow)?;
    let n64 = r.convert(V::Truncate, T::I64, T::I128, n)?;
    let d64 = r.convert(V::Truncate, T::I64, T::I128, d)?;
    r.branch(remainder, narrow_remainder, narrow_quotient)?;
    r.enter(narrow_remainder)?;
    let rest = r.binary(B::UMod, T::I64, n64, d64)?;
    let value = r.convert(V::ZeroExtend, T::I128, T::I64, rest)?;
    r.ret(T::I128, value)?;
    r.enter(narrow_quotient)?;
    let quotient = r.binary(B::UDiv, T::I64, n64, d64)?;
    let value = r.convert(V::ZeroExtend, T::I128, T::I64, quotient)?;
    r.ret(T::I128, value)?;
    r.enter(wide)?;
    let quotient = r.local(T::I128)?;
    let rest = r.local(T::I128)?;
    let bit = r.local(T::I64)?;
    let zero = r.integer(0)?;
    r.store(T::I128, zero, quotient)?;
    let zero = r.integer(0)?;
    r.store(T::I128, zero, rest)?;
    let last = r.integer(127)?;
    r.store(T::I64, last, bit)?;
    let test = r.block()?;
    let body = r.block()?;
    let subtract = r.block()?;
    let next = r.block()?;
    let done = r.block()?;
    let give_remainder = r.block()?;
    let give_quotient = r.block()?;
    r.jump(test)?;
    r.enter(test)?;
    let zero = r.integer(0)?;
    let value = r.load(T::I64, bit)?;
    let condition = r.compare(C::Lt, T::I64, value, zero)?;
    r.branch(condition, done, body)?;
    r.enter(body)?;
    let i = r.load(T::I64, bit)?;
    let one = r.integer(1)?;
    let value = r.binary(B::UShr, T::I128, n, i)?;
    let incoming = r.binary(B::And, T::I128, value, one)?;
    let one = r.integer(1)?;
    let value = r.load(T::I128, rest)?;
    let value = r.binary(B::Shl, T::I128, value, one)?;
    let shifted = r.binary(B::Or, T::I128, value, incoming)?;
    r.store(T::I128, shifted, rest)?;
    let condition = r.compare(C::Uge, T::I128, shifted, d)?;
    r.branch(condition, subtract, next)?;
    r.enter(subtract)?;
    let value = r.binary(B::Sub, T::I128, shifted, d)?;
    r.store(T::I128, value, rest)?;
    let so_far = r.load(T::I128, quotient)?;
    let one = r.integer(1)?;
    let value = r.binary(B::Shl, T::I128, one, i)?;
    let value = r.binary(B::Or, T::I128, so_far, value)?;
    r.store(T::I128, value, quotient)?;
    r.jump(next)?;
    r.enter(next)?;
    let one = r.integer(1)?;
    let value = r.binary(B::Sub, T::I64, i, one)?;
    r.store(T::I64, value, bit)?;
    r.jump(test)?;
    r.enter(done)?;
    r.branch(remainder, give_remainder, give_quotient)?;
    r.enter(give_remainder)?;
    let value = r.load(T::I128, rest)?;
    r.ret(T::I128, value)?;
    r.enter(give_quotient)?;
    let value = r.load(T::I128, quotient)?;
    r.ret(T::I128, value)?;
    r.finish()?;
    Ok(core)
}
pub(super) fn build(runtime: &mut RuntimeProgram<'_>) -> Result<(), Error> {
    let core = unsigned_core(runtime)?;
    for (index, name) in [
        b"__divti3",
        b"__udivti3".as_slice(),
        b"__modti3",
        b"__umodti3",
    ]
    .into_iter()
    .enumerate()
    {
        let signed = index == 0 || index == 2;
        let remainder = index >= 2;
        let symbol = runtime.function(name, T::I128, &[T::I128, T::I128], true)?;
        let mut r = runtime.routine(symbol)?;
        if !signed {
            let a = r.parameter(0)?;
            let b = r.parameter(1)?;
            let flag = r.integer(i64::from(remainder))?;
            let value = r.call(T::I128, core, &[a, b, flag])?;
            r.ret(T::I128, value)?;
            r.finish()?;
            continue;
        }
        let mut magnitudes = Vec::with_capacity(2);
        let mut negatives = Vec::with_capacity(2);
        for operand in 0..2 {
            let magnitude = r.local(T::I128)?;
            let zero = r.integer(0)?;
            let value = r.parameter(operand)?;
            let negative = r.compare(C::Lt, T::I128, value, zero)?;
            let flip = r.block()?;
            let keep = r.block()?;
            let joined = r.block()?;
            r.branch(negative, flip, keep)?;
            r.enter(flip)?;
            let zero = r.integer(0)?;
            let value = r.binary(B::Sub, T::I128, zero, value)?;
            r.store(T::I128, value, magnitude)?;
            r.jump(joined)?;
            r.enter(keep)?;
            let value = r.parameter(operand)?;
            r.store(T::I128, value, magnitude)?;
            r.jump(joined)?;
            r.enter(joined)?;
            magnitudes.push(magnitude);
            negatives.push(negative);
        }
        let a = r.load(T::I128, magnitudes[0])?;
        let b = r.load(T::I128, magnitudes[1])?;
        let flag = r.integer(i64::from(remainder))?;
        let result = r.call(T::I128, core, &[a, b, flag])?;
        let flips = if remainder {
            negatives[0]
        } else {
            r.binary(B::Xor, T::I64, negatives[0], negatives[1])?
        };
        let negate = r.block()?;
        let plain = r.block()?;
        r.branch(flips, negate, plain)?;
        r.enter(negate)?;
        let zero = r.integer(0)?;
        let value = r.binary(B::Sub, T::I128, zero, result)?;
        r.ret(T::I128, value)?;
        r.enter(plain)?;
        r.ret(T::I128, result)?;
        r.finish()?;
    }
    Ok(())
}
