// Translated from sema/integral_arithmetic.cpp. See NOTICE.
//! LP64 integer representations and operations; host overflow is never target overflow.
use super::types::Scalar;
use crate::{lex::pp_lexer::Punctuator as P, types::FundamentalType as F};

#[derive(Clone, Copy, Debug)]
pub(super) struct IntegralKind {
    pub bits: u32,
    pub signed: bool,
}
impl IntegralKind {
    pub(super) fn of(scalar: Scalar) -> Option<Self> {
        let (bits, signed) = match scalar {
            Scalar::Bool => (1, false),
            Scalar::SignedChar | Scalar::Basic(F::Char) => (8, true),
            Scalar::UnsignedChar => (8, false),
            Scalar::Short => (16, true),
            Scalar::UnsignedShort | Scalar::Basic(F::Char16T) => (16, false),
            Scalar::Basic(F::Int | F::WcharT) => (32, true),
            Scalar::Basic(F::UnsignedInt | F::Char32T) => (32, false),
            Scalar::Basic(F::LongInt | F::LongLongInt) => (64, true),
            Scalar::Basic(F::UnsignedLongInt | F::UnsignedLongLongInt) => (64, false),
            Scalar::Basic(F::Int128) => (128, true),
            Scalar::Basic(F::UnsignedInt128) => (128, false),
            _ => return None,
        };
        Some(Self { bits, signed })
    }
    pub(super) fn convert(self, bits: u64) -> u64 {
        if self.bits == 1 {
            return u64::from(bits != 0);
        }
        if self.bits >= 64 {
            return bits;
        }
        let mask = self.mask();
        let value = bits & mask;
        if self.signed && value & (1 << (self.bits - 1)) != 0 {
            value | !mask
        } else {
            value
        }
    }
    fn mask(self) -> u64 {
        u64::MAX >> (64 - self.bits.min(64))
    }
    pub(super) fn minimum(self) -> i64 {
        i64::MIN >> (64 - self.bits.min(64))
    }
    fn maximum(self) -> i64 {
        i64::MAX >> (64 - self.bits.min(64))
    }
    pub(super) fn represents(self, bits: u64, from: Self) -> bool {
        let negative = from.signed && signed(bits) < 0;
        if self.bits == 1 {
            bits <= 1
        } else if !self.signed {
            !negative && bits <= self.mask()
        } else if self.bits > 64 {
            true
        } else {
            (negative || bits <= self.maximum().cast_unsigned()) && signed(bits) >= self.minimum()
        }
    }
    pub(super) fn operation(self, op: P, a: u64, b: u64) -> Option<(u64, bool)> {
        let a = self.convert(a);
        let b = self.convert(b);
        let mut valid = true;
        let bits = match op {
            P::Add | P::Subtract | P::Multiply => {
                let sa = i128::from(signed(a));
                let sb = i128::from(signed(b));
                let result = match op {
                    P::Add => sa + sb,
                    P::Subtract => sa - sb,
                    _ => sa * sb,
                };
                valid = !self.signed
                    || (result >= i128::from(self.minimum())
                        && result <= i128::from(self.maximum()));
                match op {
                    P::Add => a.wrapping_add(b),
                    P::Subtract => a.wrapping_sub(b),
                    _ => a.wrapping_mul(b),
                }
            }
            P::Divide | P::Remainder => {
                if b == 0 || self.signed && signed(a) == self.minimum() && signed(b) == -1 {
                    valid = false;
                    0
                } else if self.signed {
                    let result = if op == P::Divide {
                        signed(a) / signed(b)
                    } else {
                        signed(a) % signed(b)
                    };
                    result.cast_unsigned()
                } else if op == P::Divide {
                    a / b
                } else {
                    a % b
                }
            }
            P::BitAnd => a & b,
            P::BitOr => a | b,
            P::Xor => a ^ b,
            _ => return None,
        };
        Some((self.convert(bits), valid))
    }
    pub(super) fn shift(self, op: P, a: u64, count: u64, count_kind: Self) -> (u64, bool) {
        let a = self.convert(a);
        let count = count_kind.convert(count);
        if count >= u64::from(self.bits) || count_kind.signed && signed(count) < 0 {
            return (0, false);
        }
        // The type-only evaluator operates on a u64 even for an extended
        // type. Preserve the frozen x86-64 seed's masked limb shifts here;
        // typed semantic evaluation uses wide_integers instead.
        if op == P::ShiftRight {
            return (
                if self.signed {
                    signed(a)
                        .wrapping_shr(u32::from(count.to_le_bytes()[0] & 63))
                        .cast_unsigned()
                } else {
                    a.wrapping_shr(u32::from(count.to_le_bytes()[0] & 63))
                },
                true,
            );
        }
        let amount = u32::from(count.to_le_bytes()[0] & 63);
        let valid =
            !self.signed || signed(a) >= 0 && (count == 0 || a <= self.mask().wrapping_shr(amount));
        (self.convert(a.wrapping_shl(amount)), valid)
    }
}
pub(super) fn signed(bits: u64) -> i64 {
    i64::from_ne_bytes(bits.to_ne_bytes())
}
pub(super) fn promoted(scalar: Scalar) -> Scalar {
    match scalar {
        Scalar::Bool
        | Scalar::SignedChar
        | Scalar::UnsignedChar
        | Scalar::Short
        | Scalar::UnsignedShort
        | Scalar::Basic(F::Char | F::Char16T | F::WcharT) => Scalar::Basic(F::Int),
        Scalar::Basic(F::Char32T) => Scalar::Basic(F::UnsignedInt),
        _ => scalar,
    }
}
fn rank(scalar: Scalar) -> u8 {
    match scalar {
        Scalar::Basic(F::Int | F::UnsignedInt) => 1,
        Scalar::Basic(F::LongInt | F::UnsignedLongInt) => 2,
        Scalar::Basic(F::Int128 | F::UnsignedInt128) => 4,
        _ => 3,
    }
}
pub(super) fn common(a: Scalar, b: Scalar) -> Option<Scalar> {
    let ka = IntegralKind::of(a)?;
    let kb = IntegralKind::of(b)?;
    if ka.signed == kb.signed {
        return Some(if rank(a) >= rank(b) { a } else { b });
    }
    let (unsigned, signed, ku, ks) = if ka.signed {
        (b, a, kb, ka)
    } else {
        (a, b, ka, kb)
    };
    Some(if rank(unsigned) >= rank(signed) {
        unsigned
    } else if ks.bits > ku.bits {
        signed
    } else {
        match signed {
            Scalar::Basic(F::Int) => Scalar::Basic(F::UnsignedInt),
            Scalar::Basic(F::LongInt) => Scalar::Basic(F::UnsignedLongInt),
            Scalar::Basic(F::Int128) => Scalar::Basic(F::UnsignedInt128),
            _ => Scalar::Basic(F::UnsignedLongLongInt),
        }
    })
}
pub(super) fn comparison(op: P, signed_type: bool, a: u64, b: u64) -> Option<bool> {
    let less = if signed_type {
        signed(a) < signed(b)
    } else {
        a < b
    };
    Some(match op {
        P::Equal => a == b,
        P::NotEqual => a != b,
        P::Less => less,
        P::Greater => !less && a != b,
        P::LessEqual => less || a == b,
        P::GreaterEqual => !less,
        _ => return None,
    })
}
