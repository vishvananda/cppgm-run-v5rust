// Translated from lower/soft_float_lowering.cpp's static arithmetic. See NOTICE.
//! Exact binary encodings and bounded wide arithmetic for static soft values.
use super::{FundamentalType as F, float_value::Operation, floating::FloatError};
#[derive(Clone, Copy)]
struct Decoded {
    negative: bool,
    special: bool,
    nan: bool,
    magnitude: u128,
    exponent: i64,
    payload: u64,
}
pub(crate) fn soft(ty: F) -> bool {
    matches!(
        ty.floating_representation(),
        F::Float16 | F::BFloat16 | F::Float128
    )
}
pub(crate) fn sign(ty: F) -> u128 {
    1 << match ty.floating_representation() {
        F::Float16 | F::BFloat16 => 15,
        F::Float => 31,
        F::Double => 63,
        F::Float128 => 127,
        _ => 79,
    }
}
fn spec(ty: F) -> (u32, u32, i64) {
    match ty.floating_representation() {
        F::Float16 => (10, 5, 15),
        F::BFloat16 => (7, 8, 127),
        F::Float128 => (112, 15, 16383),
        F::Float => (23, 8, 127),
        F::Double => (52, 11, 1023),
        _ => (63, 15, 16383),
    }
}
fn decode(ty: F, bits: u128) -> Decoded {
    let (fraction, width, exponent_bias) = spec(ty);
    let extended = ty.floating_representation() == F::LongDouble;
    let shift = if extended { 64 } else { fraction };
    let exponent = i64::from(u16::try_from(bits >> shift & ((1 << width) - 1)).unwrap_or(0));
    let magnitude = if extended {
        bits & u128::from(u64::MAX)
    } else {
        (bits & ((1 << fraction) - 1)) | if exponent == 0 { 0 } else { 1 << fraction }
    };
    let rest = bits & ((1 << fraction) - 1);
    let payload = if fraction > 64 {
        rest >> (fraction - 64)
    } else {
        rest << (64 - fraction)
    };
    Decoded {
        negative: bits & sign(ty) != 0,
        special: exponent == (1 << width) - 1,
        nan: exponent == (1 << width) - 1 && rest != 0,
        magnitude,
        exponent: exponent.max(1) - exponent_bias - i64::from(fraction),
        payload: u64::try_from(payload).unwrap_or(0),
    }
}
fn infinity(ty: F) -> u128 {
    let (fraction, width, _) = spec(ty);
    if ty.floating_representation() == F::LongDouble {
        ((1 << width) - 1) << 64 | 1 << 63
    } else {
        ((1 << width) - 1) << fraction
    }
}
fn rounded(negative: bool, magnitude: u128, exponent: i64, ty: F) -> Result<u128, FloatError> {
    let sign = if negative { sign(ty) } else { 0 };
    if magnitude == 0 {
        return Ok(sign);
    }
    let (fraction, _, exponent_bias) = spec(ty);
    let precision = fraction + 1;
    let top = exponent + i64::from(128 - magnitude.leading_zeros()) - 1;
    let mut quantum = top.max(1 - exponent_bias) - i64::from(fraction);
    let drop = quantum - exponent;
    let mut significand = if drop > 0 {
        super::float_value::round_shift(magnitude, drop.unsigned_abs())
    } else {
        magnitude << drop.unsigned_abs()
    };
    if significand & (1 << precision) != 0 {
        significand >>= 1;
        quantum += 1;
    }
    let biased = if significand & (1 << fraction) != 0 {
        quantum + i64::from(fraction) + exponent_bias
    } else {
        0
    };
    if biased > 2 * exponent_bias {
        return Ok(sign | infinity(ty));
    }
    let biased = u128::from(u16::try_from(biased).map_err(|_| FloatError::HostSize)?);
    let bits = if ty.floating_representation() == F::LongDouble {
        significand | biased << 64
    } else {
        (significand & ((1 << fraction) - 1)) | biased << fraction
    };
    Ok(sign | bits)
}

pub(crate) fn convert(from: F, bits: u128, to: F) -> Result<u128, FloatError> {
    if from.floating_representation() == to.floating_representation() {
        return Ok(bits);
    }
    let value = decode(from, bits);
    if value.special {
        let fraction = spec(to).0;
        let payload = u128::from(value.payload);
        let payload = if fraction > 64 {
            payload << (fraction - 64)
        } else {
            payload >> (64 - fraction)
        };
        return Ok(infinity(to)
            | if value.negative { sign(to) } else { 0 }
            | if value.nan {
                payload | 1 << (fraction - 1)
            } else {
                0
            });
    }
    rounded(value.negative, value.magnitude, value.exponent, to)
}
pub(crate) fn from_integer(bits: u128, signed: bool, to: F) -> Result<u128, FloatError> {
    let negative = signed && bits >> 127 != 0;
    rounded(
        negative,
        if negative { bits.wrapping_neg() } else { bits },
        0,
        to,
    )
}
pub(crate) fn to_integer(from: F, bits: u128, width: u32, signed: bool) -> Option<u128> {
    let value = decode(from, bits);
    if width == 1 {
        return Some(u128::from(value.nan || value.magnitude != 0));
    }
    if value.special {
        return None;
    }
    let whole = if value.exponent >= 0 {
        if i64::from(128 - value.magnitude.leading_zeros()) + value.exponent > 128 {
            return None;
        }
        value
            .magnitude
            .checked_shl(u32::try_from(value.exponent).ok()?)
            .unwrap_or(0)
    } else if value.exponent <= -128 {
        0
    } else {
        value.magnitude >> -value.exponent
    };
    if signed {
        let limit = (1u128 << (width - 1)) - u128::from(!value.negative);
        if whole > limit {
            return None;
        }
    } else if value.negative && whole != 0 || width < 128 && whole >> width != 0 {
        return None;
    }
    Some(if value.negative {
        whole.wrapping_neg()
    } else {
        whole
    })
}
fn sticky(value: u128, shift: i64) -> u128 {
    if shift <= 0 {
        value
    } else if shift >= 128 {
        u128::from(value != 0)
    } else {
        (value >> shift) | u128::from((value >> shift) << shift != value)
    }
}
fn product(x: u128, y: u128) -> (u128, i64) {
    let mask = u128::from(u64::MAX);
    let (x0, x1, y0, y1) = (x & mask, x >> 64, y & mask, y >> 64);
    let (p00, p01, p10, p11) = (x0 * y0, x0 * y1, x1 * y0, x1 * y1);
    let middle = (p00 >> 64) + (p01 & mask) + (p10 & mask);
    let low = (middle << 64) | (p00 & mask);
    let high = p11 + (p01 >> 64) + (p10 >> 64) + (middle >> 64);
    if high == 0 {
        return (low, 0);
    }
    let shift = 128 - high.leading_zeros() + 2;
    (
        (high << (128 - shift)) | (low >> shift) | u128::from(low & ((1 << shift) - 1) != 0),
        i64::from(shift),
    )
}
pub(crate) fn arithmetic(
    op: Operation,
    ty: F,
    left: u128,
    right: u128,
) -> Result<Option<u128>, FloatError> {
    let mut a = decode(ty, left);
    let mut b = decode(ty, right);
    if matches!(op, Operation::Subtract) {
        b.negative = !b.negative;
    }
    let (mut x, mut y) = (a.magnitude, b.magnitude);
    if a.nan || b.nan {
        return Ok(None);
    }
    if a.special || b.special || matches!(op, Operation::Divide) && y == 0 {
        let (infinite, negative) = match op {
            Operation::Add | Operation::Subtract => {
                if a.special && b.special && a.negative != b.negative {
                    return Ok(None);
                }
                (true, if a.special { a.negative } else { b.negative })
            }
            Operation::Multiply => {
                if a.special && y == 0 || b.special && x == 0 {
                    return Ok(None);
                }
                (true, a.negative != b.negative)
            }
            Operation::Divide => {
                if a.special && b.special || !a.special && x == 0 && y == 0 && !b.special {
                    return Ok(None);
                }
                (!b.special, a.negative != b.negative)
            }
        };
        return Ok(Some(
            (if infinite { infinity(ty) } else { 0 }) | if negative { sign(ty) } else { 0 },
        ));
    }
    if matches!(op, Operation::Add | Operation::Subtract) {
        if x == 0 && y == 0 {
            return rounded(a.negative && b.negative, 0, 0, ty).map(Some);
        }
        if x == 0 {
            return rounded(b.negative, y, b.exponent, ty).map(Some);
        }
        if y == 0 {
            return rounded(a.negative, x, a.exponent, ty).map(Some);
        }
        if a.exponent < b.exponent {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut x, &mut y);
        }
        let exponent = a.exponent - 10;
        x <<= 10;
        y = sticky(y << 10, a.exponent - b.exponent);
        let (negative, magnitude) = if a.negative == b.negative {
            (a.negative, x + y)
        } else if x == y {
            (false, 0)
        } else if x > y {
            (a.negative, x - y)
        } else {
            (b.negative, y - x)
        };
        return rounded(negative, magnitude, exponent, ty).map(Some);
    }
    let negative = a.negative != b.negative;
    if x == 0 || matches!(op, Operation::Multiply) && y == 0 {
        return rounded(negative, 0, 0, ty).map(Some);
    }
    if matches!(op, Operation::Multiply) {
        let (product, shift) = product(x, y);
        return rounded(negative, product, a.exponent + b.exponent + shift, ty).map(Some);
    }
    let mut exponent = a.exponent - b.exponent;
    while x < y {
        x <<= 1;
        exponent -= 1;
    }
    while x >= y << 1 {
        y <<= 1;
        exponent += 1;
    }
    let mut quotient = 0;
    for _ in 0..125 {
        quotient <<= 1;
        if x >= y {
            x -= y;
            quotient |= 1;
        }
        x <<= 1;
    }
    rounded(negative, quotient | u128::from(x != 0), exponent - 124, ty).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_width_rounding_matches_the_rational_scanner() {
        let mut scanner = crate::types::floating::FloatingScanner::default();
        let mut magnitude = 1u128;
        for _ in 0..64 {
            magnitude = magnitude
                .wrapping_mul(0x2360_ed05_1fc6_5da4_4385_df64_9fcc_f645)
                .wrapping_add(1);
            for ty in [
                F::Float16,
                F::BFloat16,
                F::Float128,
                F::Float,
                F::Double,
                F::LongDouble,
            ] {
                for exponent in [-16500, -10000, -113, -64, -24, 0, 1024, 16383] {
                    let expected = scanner
                        .binary(magnitude, exponent, ty)
                        .unwrap()
                        .unwrap_or_else(|| infinity(ty));
                    assert_eq!(rounded(false, magnitude, exponent, ty).unwrap(), expected);
                }
            }
        }
    }
    #[test]
    fn narrow_round_trips_preserve_zero_sign_and_quiet_nan_payloads() {
        for ty in [F::Float16, F::BFloat16] {
            let fraction = spec(ty).0;
            for raw in 0..=u16::MAX {
                let bits = u128::from(raw);
                let wide = convert(ty, bits, F::Double).unwrap();
                let expected = bits
                    | if decode(ty, bits).nan {
                        1 << (fraction - 1)
                    } else {
                        0
                    };
                assert_eq!(convert(F::Double, wide, ty).unwrap(), expected);
            }
        }
    }
    #[test]
    fn quad_rounding_keeps_cancellation_products_and_division_remainders() {
        let one = 0x3fff_u128 << 112;
        let two = 0x4000_u128 << 112;
        let third = (0x3ffd_u128 << 112) | (((1u128 << 112) - 1) / 3);
        assert_eq!(
            arithmetic(Operation::Subtract, F::Float128, one, one).unwrap(),
            Some(0)
        );
        assert_eq!(
            arithmetic(Operation::Multiply, F::Float128, one + 1, two).unwrap(),
            Some(two + 1)
        );
        assert_eq!(
            arithmetic(Operation::Divide, F::Float128, one, two + (1 << 111)).unwrap(),
            Some(third)
        );
        let largest = (0x7ffe_u128 << 112) | ((1 << 112) - 1);
        assert_eq!(
            arithmetic(Operation::Multiply, F::Float128, largest, two).unwrap(),
            Some(0x7fff_u128 << 112)
        );
        assert_eq!(
            arithmetic(
                Operation::Multiply,
                F::Float128,
                one + (1 << 111),
                one + (1 << 111)
            )
            .unwrap(),
            Some(two + (1 << 109))
        );
    }
    #[test]
    fn conversions_preserve_payloads_and_reject_unrepresentable_integers() {
        assert_eq!(
            convert(F::Float128, (0x3fff_u128 << 112) | (1 << 101), F::Float16).unwrap(),
            0x3c00
        );
        assert_eq!(
            convert(F::Double, u128::from((-0.0f64).to_bits()), F::GnuFloat128).unwrap(),
            1 << 127
        );
        assert_eq!(
            convert(F::Float16, 0x7c01, F::Float128).unwrap(),
            (0x7fff_u128 << 112) | (1 << 111) | (1 << 102)
        );
        assert_eq!(
            to_integer(F::Float128, (0x4000_u128 << 112) | (1 << 111), 32, true),
            Some(3)
        );
        assert_eq!(to_integer(F::Float128, 0x7fff_u128 << 112, 32, true), None);
        assert_eq!(to_integer(F::Float16, 0xbc00, 32, false), None);
        assert_eq!(
            from_integer(u128::MAX, true, F::Float128).unwrap(),
            (1 << 127) | (0x3fff_u128 << 112)
        );
    }
}
