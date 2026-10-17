// Translated from sema/constant_expressions.cpp's target floating rules. See NOTICE.
//! Scalar arithmetic in explicit binary32, binary64 and x87 precision.
use super::FundamentalType;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Single,
    Double,
    Extended,
}
impl Format {
    pub(crate) fn of(ty: FundamentalType) -> Option<Self> {
        match ty {
            FundamentalType::Float => Some(Self::Single),
            FundamentalType::Double => Some(Self::Double),
            FundamentalType::LongDouble => Some(Self::Extended),
            _ => None,
        }
    }
    fn specification(self) -> (i64, i64) {
        match self {
            Self::Single => (24, 127),
            Self::Double => (53, 1023),
            Self::Extended => (64, 16383),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// An initialized x87 encoding, including signed zero and non-finite values.
/// Narrower values are held exactly; no host floating operation is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BinaryFloat(u128);
const SIGN: u128 = 1 << 79;
const INFINITY: u128 = (0x7fff << 64) | (1 << 63);

impl BinaryFloat {
    pub(crate) fn zero() -> Self {
        Self(0)
    }
    pub(crate) fn negative(self) -> bool {
        self.0 & SIGN != 0
    }
    pub(crate) fn negate(self) -> Self {
        Self(self.0 ^ SIGN)
    }
    pub(crate) fn is_nan(self) -> bool {
        self.0 & !SIGN > INFINITY
    }
    fn is_infinite(self) -> bool {
        self.0 & !SIGN == INFINITY
    }
    pub(crate) fn truth(self) -> bool {
        self.0 & !SIGN != 0
    }

    pub(crate) fn from_bits(format: Format, bits: u128) -> Option<Self> {
        if format == Format::Extended {
            return Some(Self(bits & ((1 << 80) - 1)));
        }
        let (precision, maximum) = format.specification();
        let fraction = precision - 1;
        let sign_bit = if format == Format::Single { 31 } else { 63 };
        let negative = bits & (1 << sign_bit) != 0;
        let exponent =
            i64::try_from((bits >> fraction) & u128::from((maximum * 2 + 1).unsigned_abs()))
                .ok()?;
        let significand = bits & ((1 << fraction) - 1);
        if exponent == maximum * 2 + 1 {
            return Some(Self(
                (if negative { SIGN } else { 0 }) | INFINITY | (significand << (63 - fraction)),
            ));
        }
        let value = significand | if exponent == 0 { 0 } else { 1 << fraction };
        Some(Self::pack(
            negative,
            value,
            exponent.max(1) - maximum - (precision - 1),
        ))
    }
    pub(crate) fn integer(bits: u64, signed: bool) -> Self {
        let negative = signed && bits & (1 << 63) != 0;
        Self::pack(
            negative,
            u128::from(if negative { bits.wrapping_neg() } else { bits }),
            0,
        )
    }
    fn parts(self) -> (u128, i64) {
        let significand = self.0 & ((1 << 64) - 1);
        let bytes = self.0.to_le_bytes();
        let exponent = i64::from(u16::from_le_bytes([bytes[8], bytes[9]]) & 0x7fff);
        (significand, exponent.max(1) - 16383 - 63)
    }
    fn pack(negative: bool, significand: u128, exponent: i64) -> Self {
        let sign = if negative { SIGN } else { 0 };
        if significand == 0 {
            return Self(sign);
        }
        let length = 128 - significand.leading_zeros();
        let top = exponent + i64::from(length) - 1;
        if top < -16382 {
            // A rounded x87 subnormal is an exact multiple of 2^-16445.
            let shift = exponent + 16445;
            let bits = if shift >= 0 {
                significand << shift
            } else {
                significand >> -shift
            };
            return Self(sign | bits);
        }
        let bits = if length > 64 {
            significand >> (length - 64)
        } else {
            significand << (64 - length)
        };
        Self(sign | (u128::from((top + 16383).unsigned_abs()) << 64) | bits)
    }
    fn round(negative: bool, significand: u128, exponent: i64, format: Format) -> Option<Self> {
        if significand == 0 {
            return Some(Self::pack(negative, 0, 0));
        }
        let (precision, maximum) = format.specification();
        let length = i64::from(128 - significand.leading_zeros());
        let quantum = (exponent + length - precision).max(1 - maximum - (precision - 1));
        let shift = quantum - exponent;
        let rounded = if shift > 0 {
            round_shift(significand, shift.unsigned_abs())
        } else {
            significand << shift.unsigned_abs()
        };
        let top = quantum + i64::from(128 - rounded.leading_zeros()) - 1;
        if rounded != 0 && top > maximum {
            return None;
        }
        Some(Self::pack(negative, rounded, quantum))
    }
    pub(crate) fn rounded(self, format: Format) -> Option<Self> {
        if self.is_nan() || self.is_infinite() {
            return Some(self);
        }
        let (significand, exponent) = self.parts();
        Self::round(self.negative(), significand, exponent, format)
    }
    pub(crate) fn bits(self, format: Format) -> Option<u128> {
        let value = self.rounded(format)?;
        if format == Format::Extended {
            return Some(value.0);
        }
        let sign = u128::from(value.negative()) << if format == Format::Single { 31 } else { 63 };
        let (precision, maximum) = format.specification();
        if value.is_nan() || value.is_infinite() {
            let payload = (value.0 & ((1 << 63) - 1)) >> (64 - precision);
            return Some(
                sign | (u128::from((maximum * 2 + 1).unsigned_abs()) << (precision - 1)) | payload,
            );
        }
        let (sig, exponent) = value.parts();
        if sig == 0 {
            return Some(sign);
        }
        let top = exponent + i64::from(128 - sig.leading_zeros()) - 1;
        let quantum = if top < 1 - maximum {
            1 - maximum - (precision - 1)
        } else {
            top - (precision - 1)
        };
        let sig = if quantum > exponent {
            sig >> (quantum - exponent)
        } else {
            sig << (exponent - quantum)
        };
        let encoded_exponent = if top < 1 - maximum { 0 } else { top + maximum };
        Some(
            sign | (u128::from(encoded_exponent.unsigned_abs()) << (precision - 1))
                | (sig & ((1 << (precision - 1)) - 1)),
        )
    }
    pub(crate) fn integral(self, width: u32, signed: bool) -> Option<u64> {
        if width == 0 || width > 64 {
            return None;
        }
        if self.is_nan() || self.is_infinite() {
            return None;
        }
        let (sig, exponent) = self.parts();
        let magnitude = if exponent < 0 {
            if exponent <= -128 {
                0
            } else {
                sig >> -exponent
            }
        } else {
            if exponent >= 64 && sig != 0 {
                return None;
            }
            sig << exponent
        };
        let limit = 1u128 << (width - u32::from(signed));
        if self.negative() {
            if (!signed && magnitude != 0) || (signed && magnitude > limit) {
                return None;
            }
            Some(u64::try_from(magnitude).ok()?.wrapping_neg())
        } else if magnitude < limit {
            u64::try_from(magnitude).ok()
        } else {
            None
        }
    }
    pub(crate) fn compare(self, other: Self) -> Option<Ordering> {
        if self.is_nan() || other.is_nan() {
            return None;
        }
        if !self.truth() && !other.truth() {
            return Some(Ordering::Equal);
        }
        Some(if self.negative() != other.negative() {
            other.negative().cmp(&self.negative())
        } else if self.negative() {
            (other.0 & !SIGN).cmp(&(self.0 & !SIGN))
        } else {
            self.0.cmp(&other.0)
        })
    }
    pub(crate) fn operation(self, op: Operation, right: Self, format: Format) -> Option<Self> {
        if self.is_nan() || right.is_nan() {
            return None;
        }
        let right = if matches!(op, Operation::Subtract) {
            right.negate()
        } else {
            right
        };
        if matches!(op, Operation::Divide) && !right.truth() {
            return None;
        }
        if self.is_infinite() || right.is_infinite() {
            return (matches!(op, Operation::Divide) && !self.is_infinite())
                .then(|| Self::pack(self.negative() ^ right.negative(), 0, 0));
        }
        if matches!(op, Operation::Add | Operation::Subtract) {
            if !self.truth() {
                return if right.truth() {
                    right.rounded(format)
                } else {
                    Some(Self::pack(self.negative() && right.negative(), 0, 0))
                };
            }
            if !right.truth() {
                return self.rounded(format);
            }
        }
        let (a, ae) = self.normalized();
        let (b, be) = right.normalized();
        let intermediate = if format == Format::Single {
            Format::Single
        } else {
            Format::Extended
        };
        let result = match op {
            Operation::Add | Operation::Subtract => {
                let exponent = ae.max(be);
                let a = jam(a << 3, (exponent - ae).unsigned_abs());
                let b = jam(b << 3, (exponent - be).unsigned_abs());
                let (negative, magnitude) = if self.negative() == right.negative() {
                    (self.negative(), a + b)
                } else if a >= b {
                    (self.negative() && a != b, a - b)
                } else {
                    (right.negative(), b - a)
                };
                Self::round(negative, magnitude, exponent - 3, intermediate)
            }
            Operation::Multiply => Self::round(
                self.negative() ^ right.negative(),
                a * b,
                ae + be,
                intermediate,
            ),
            Operation::Divide => {
                let mut quotient = a / b;
                let mut remainder = a % b;
                for _ in 0..67 {
                    remainder <<= 1;
                    quotient <<= 1;
                    if remainder >= b {
                        quotient |= 1;
                        remainder -= b;
                    }
                }
                if remainder != 0 {
                    quotient |= 1;
                }
                Self::round(
                    self.negative() ^ right.negative(),
                    quotient,
                    ae - be - 67,
                    intermediate,
                )
            }
        }?;
        result.rounded(format)
    }
    fn normalized(self) -> (u128, i64) {
        let (sig, exponent) = self.parts();
        if sig == 0 {
            return (0, 0);
        }
        let shift = sig.leading_zeros() - 64;
        (sig << shift, exponent - i64::from(shift))
    }
}
fn jam(value: u128, shift: u64) -> u128 {
    if shift == 0 {
        value
    } else if shift >= 128 {
        u128::from(value != 0)
    } else {
        (value >> shift) | u128::from(value & ((1 << shift) - 1) != 0)
    }
}
fn round_shift(value: u128, shift: u64) -> u128 {
    if shift > 128 {
        return 0;
    }
    let half = 1u128 << (shift - 1);
    if shift == 128 {
        return u128::from(value > half);
    }
    let high = value >> shift;
    let low = value & ((1 << shift) - 1);
    high + u128::from(low > half || low == half && high & 1 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_rounding_range_signed_zero_and_extended_precision() {
        let one = BinaryFloat::integer(1, false);
        let tiny =
            BinaryFloat::from_bits(Format::Extended, (16383 - 63) << 64 | (1 << 63)).unwrap();
        let extended = one
            .operation(Operation::Add, tiny, Format::Extended)
            .unwrap();
        assert_ne!(extended, one);
        assert_eq!(extended.rounded(Format::Double), Some(one));
        assert_eq!(
            BinaryFloat::integer(u64::MAX, false).integral(64, false),
            Some(u64::MAX)
        );
        assert!(
            BinaryFloat::from_bits(Format::Double, 0x43f0_0000_0000_0000)
                .unwrap()
                .integral(64, false)
                .is_none()
        );
        assert!(!BinaryFloat::zero().negate().truth());
        assert_eq!(
            one.operation(Operation::Subtract, one, Format::Extended),
            Some(BinaryFloat::zero())
        );
        assert!(
            one.operation(Operation::Divide, BinaryFloat::zero(), Format::Double)
                .is_none()
        );
        assert_eq!(
            one.operation(
                Operation::Divide,
                BinaryFloat::integer(3, false),
                Format::Double
            )
            .unwrap()
            .bits(Format::Double),
            Some(0x3fd5_5555_5555_5555)
        );
    }
    #[test]
    fn single_precision_arithmetic_matches_independently_rounded_values() {
        let mut state = 0x15ab_cdef_u32;
        for _ in 0..10_000 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let left = f32::from_bits(state);
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let right = f32::from_bits(state);
            if !left.is_finite() || !right.is_finite() {
                continue;
            }
            let a = BinaryFloat::from_bits(Format::Single, u128::from(left.to_bits())).unwrap();
            let b = BinaryFloat::from_bits(Format::Single, u128::from(right.to_bits())).unwrap();
            for (op, expected) in [
                (Operation::Add, left + right),
                (Operation::Subtract, left - right),
                (Operation::Multiply, left * right),
                (Operation::Divide, left / right),
            ] {
                let actual = a.operation(op, b, Format::Single);
                if expected.is_finite() && !(matches!(op, Operation::Divide) && right == 0.0) {
                    assert_eq!(
                        actual.unwrap().bits(Format::Single),
                        Some(u128::from(expected.to_bits())),
                        "left={left:?} right={right:?}"
                    );
                } else {
                    assert!(actual.is_none());
                }
            }
        }
    }
}
