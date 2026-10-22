//! Target scalar evaluation from `opt/scalar_fold.cpp`. See NOTICE.
use crate::{
    lowir::{
        literals::{Constant, FloatKind},
        model::{Body, Value},
        vocabulary::{Binary, Compare, Convert, Type, Unary},
    },
    types::float_value::{BinaryFloat, Format, Operation},
};
use std::cmp::Ordering;

pub(super) fn literal(body: &Body, value: Value) -> Option<Constant> {
    match value {
        Value::Integer(id) => Some(Constant::Integer(body.literal(id))),
        Value::Float(id, kind) => Some(Constant::Float(kind, body.literal(id))),
        _ => None,
    }
}

pub(super) fn width(ty: Type) -> u32 {
    if ty == Type::Ptr { 64 } else { ty.bits() }
}
fn signed(ty: Type) -> bool {
    matches!(
        ty,
        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::Ptr
    )
}
fn mask(width: u32) -> u128 {
    if width >= 128 {
        u128::MAX
    } else {
        (1_u128 << width) - 1
    }
}
fn extend(bits: u128, width: u32, signed: bool) -> u128 {
    if width >= 128 {
        return bits;
    }
    let bits = bits & mask(width);
    if signed && width != 0 && bits & (1_u128 << (width - 1)) != 0 {
        bits | !mask(width)
    } else {
        bits
    }
}
fn as_signed(bits: u128) -> i128 {
    i128::from_le_bytes(bits.to_le_bytes())
}
pub(super) fn normalize(ty: Type, bits: u128) -> u128 {
    let width = width(ty);
    if width == 0 {
        bits
    } else {
        extend(bits, width, signed(ty))
    }
}
pub(super) fn integer(ty: Type, bits: u128) -> Constant {
    Constant::Integer(normalize(ty, bits))
}
fn raw(value: Constant) -> Option<u128> {
    if let Constant::Integer(bits) = value {
        Some(bits)
    } else {
        None
    }
}
fn format(kind: FloatKind) -> Format {
    match kind {
        FloatKind::F32 => Format::Single,
        FloatKind::F64 => Format::Double,
        FloatKind::F80 => Format::Extended,
    }
}
fn floating(value: Constant) -> Option<BinaryFloat> {
    match value {
        Constant::Integer(bits) => BinaryFloat::wide_integer(bits, true, Format::Extended),
        Constant::Float(kind, bits) => {
            let value = BinaryFloat::from_bits(format(kind), bits)?;
            // Loading a single/double into x87 quiets a signaling NaN. An
            // already extended literal is returned in its original encoding.
            Some(if kind == FloatKind::F80 {
                value
            } else {
                value.quiet()
            })
        }
    }
}
fn stored(kind: FloatKind, value: BinaryFloat) -> Option<Constant> {
    if kind == FloatKind::F80 {
        // An already extended conversion has no precision-changing operation;
        // float_literal copies even a pseudo-denormal or unsupported encoding.
        return Some(Constant::Float(kind, value.extended_bits()));
    }
    let value = value.stored(format(kind));
    let mut bits = value.bits(format(kind))?;
    if value.is_nan() && kind != FloatKind::F80 {
        // float_literal retains top payload bits; an empty truncated payload
        // becomes a quiet NaN rather than an infinity.
        let (fraction, exponent) = if kind == FloatKind::F32 {
            (23, 8)
        } else {
            (52, 11)
        };
        let sign = u128::from(value.negative()) << (fraction + exponent);
        let payload = (value.extended_bits() >> (63 - fraction)) & ((1 << fraction) - 1);
        bits = sign
            | (((1 << exponent) - 1) << fraction)
            | if payload == 0 {
                1 << (fraction - 1)
            } else {
                payload
            };
    }
    Some(Constant::Float(kind, bits))
}
pub(super) fn copy(ty: Type, value: Constant) -> Option<Constant> {
    if let Some(kind) = FloatKind::from_type(ty) {
        return matches!(value, Constant::Float(k, _) if k == kind).then_some(value);
    }
    (width(ty) != 0)
        .then(|| raw(value).map(|bits| integer(ty, bits)))
        .flatten()
}
pub(super) fn unary(op: Unary, ty: Type, value: Constant) -> Option<Constant> {
    if let Some(kind) = FloatKind::from_type(ty) {
        let Constant::Float(source, bits) = value else {
            return None;
        };
        if op != Unary::Neg || source != kind {
            return None;
        }
        let sign = match kind {
            FloatKind::F32 => 31,
            FloatKind::F64 => 63,
            FloatKind::F80 => 79,
        };
        return Some(Constant::Float(kind, bits ^ (1 << sign)));
    }
    let width = width(ty);
    if width == 0 {
        return None;
    }
    let value = extend(raw(value)?, width, signed(ty));
    let result = match op {
        Unary::Neg => value.wrapping_neg(),
        Unary::Not => u128::from(value & mask(width) == 0),
        Unary::BitNot => !value,
        Unary::ByteSwap => {
            if !matches!(width, 16 | 32 | 64) {
                return None;
            }
            value.swap_bytes() >> (128 - width)
        }
        Unary::Ctz | Unary::Clz => {
            let value = value & mask(width);
            if !matches!(width, 32 | 64) || value == 0 {
                return None;
            }
            u128::from(if op == Unary::Ctz {
                value.trailing_zeros()
            } else {
                value.leading_zeros() - (128 - width)
            })
        }
    };
    Some(integer(ty, result))
}
pub(super) fn binary(op: Binary, ty: Type, a: Constant, b: Constant) -> Option<Constant> {
    if let Some(kind) = FloatKind::from_type(ty) {
        let operation = match op {
            Binary::Add => Operation::Add,
            Binary::Sub => Operation::Subtract,
            Binary::Mul => Operation::Multiply,
            Binary::Div => Operation::Divide,
            _ => return None,
        };
        let mut a = floating(a)?;
        let mut b = floating(b)?;
        if kind == FloatKind::F32 {
            a = a.stored(Format::Single);
            b = b.stored(Format::Single);
        }
        return stored(kind, a.evaluated(operation, b, format(kind)));
    }
    let width = width(ty);
    if width <= 1 {
        return None;
    }
    let (a, b) = (raw(a)?, raw(b)?);
    let (ua, ub) = (a & mask(width), b & mask(width));
    let (sa, sb) = (
        as_signed(extend(a, width, true)),
        as_signed(extend(b, width, true)),
    );
    let result = match op {
        Binary::Add => a.wrapping_add(b),
        Binary::Sub => a.wrapping_sub(b),
        Binary::Mul => a.wrapping_mul(b),
        Binary::And => a & b,
        Binary::Or => a | b,
        Binary::Xor => a ^ b,
        Binary::Div | Binary::Mod => {
            if ub == 0 || ua == (1 << (width - 1)) && ub == mask(width) {
                return None;
            }
            let value = if op == Binary::Div {
                sa.checked_div(sb)?
            } else {
                sa.checked_rem(sb)?
            };
            u128::from_le_bytes(value.to_le_bytes())
        }
        Binary::UDiv | Binary::UMod => {
            if ub == 0 {
                return None;
            }
            if op == Binary::UDiv { ua / ub } else { ua % ub }
        }
        Binary::Shl | Binary::Shr | Binary::UShr => {
            if ub >= u128::from(width) {
                return None;
            }
            let shift = u32::try_from(ub).ok()?;
            match op {
                Binary::Shl => a.wrapping_shl(shift),
                Binary::UShr => ua >> shift,
                _ => u128::from_le_bytes((sa >> shift).to_le_bytes()),
            }
        }
    };
    Some(integer(ty, result))
}
fn ordered(pred: Compare, order: Option<Ordering>) -> Option<bool> {
    use Ordering::{Equal, Greater, Less};
    Some(match pred {
        Compare::Eq => order == Some(Equal),
        Compare::Ne => order != Some(Equal),
        Compare::Lt => order == Some(Less),
        Compare::Le => matches!(order, Some(Less | Equal)),
        Compare::Gt => order == Some(Greater),
        Compare::Ge => matches!(order, Some(Greater | Equal)),
        _ => return None,
    })
}
pub(super) fn compare(pred: Compare, ty: Type, a: Constant, b: Constant) -> Option<Constant> {
    let result = if FloatKind::from_type(ty).is_some() {
        ordered(
            pred,
            floating(a)?.canonical().compare(floating(b)?.canonical()),
        )?
    } else {
        let width = width(ty);
        if width == 0 {
            return None;
        }
        let (a, b) = (raw(a)?, raw(b)?);
        let unsigned = (a & mask(width)).cmp(&(b & mask(width)));
        match pred {
            Compare::Eq | Compare::Ne => ordered(pred, Some(unsigned))?,
            Compare::Ult => unsigned.is_lt(),
            Compare::Ule => !unsigned.is_gt(),
            Compare::Ugt => unsigned.is_gt(),
            Compare::Uge => !unsigned.is_lt(),
            _ => ordered(
                pred,
                Some(
                    as_signed(extend(a, width, width > 1)).cmp(&as_signed(extend(
                        b,
                        width,
                        width > 1,
                    ))),
                ),
            )?,
        }
    };
    Some(integer(Type::I64, u128::from(result)))
}
pub(super) fn convert(op: Convert, to: Type, from: Type, value: Constant) -> Option<Constant> {
    match op {
        Convert::SignExtend | Convert::ZeroExtend | Convert::Truncate => {
            let width = width(from);
            if width == 0 || self::width(to) == 0 {
                return None;
            }
            Some(integer(
                to,
                extend(raw(value)?, width, op == Convert::SignExtend),
            ))
        }
        Convert::SignedToFloat | Convert::UnsignedToFloat => {
            let width = width(from);
            if width == 0 || width > 64 {
                return None;
            }
            let kind = FloatKind::from_type(to)?;
            let bits = extend(raw(value)?, width, op == Convert::SignedToFloat);
            let bits = u64::try_from(bits & u128::from(u64::MAX)).ok()?;
            stored(
                kind,
                BinaryFloat::integer(bits, op == Convert::SignedToFloat),
            )
        }
        Convert::FloatToSigned | Convert::FloatToUnsigned => {
            let width = width(to);
            if width == 0
                || width > 64
                || FloatKind::from_type(from).is_none()
                || !matches!(value, Constant::Float(..))
            {
                return None;
            }
            let bits = floating(value)?
                .canonical()
                .integral_wide(width, op == Convert::FloatToSigned)?;
            Some(integer(to, bits))
        }
        Convert::FloatExtend | Convert::FloatTruncate => {
            FloatKind::from_type(from)?;
            stored(FloatKind::from_type(to)?, floating(value)?)
        }
    }
}
pub(super) fn same(ty: Type, a: Constant, b: Constant) -> bool {
    if FloatKind::from_type(ty).is_some() {
        matches!((a,b),(Constant::Float(ka,a),Constant::Float(kb,b)) if ka==kb && a==b)
    } else {
        matches!((a,b),(Constant::Integer(a),Constant::Integer(b)) if normalize(ty,a)==normalize(ty,b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_types_wrap_while_opcode_signedness_controls_division_and_comparison() {
        let a = Constant::Integer(255);
        let b = Constant::Integer(2);
        assert!(matches!(
            binary(Binary::Div, Type::U8, a, b),
            Some(Constant::Integer(0))
        ));
        assert!(matches!(
            binary(Binary::UDiv, Type::U8, a, b),
            Some(Constant::Integer(127))
        ));
        assert!(matches!(
            compare(Compare::Lt, Type::U8, a, b),
            Some(Constant::Integer(1))
        ));
        assert!(matches!(
            compare(Compare::Ult, Type::U8, a, b),
            Some(Constant::Integer(0))
        ));
        assert!(
            matches!(binary(Binary::Add,Type::I8,Constant::Integer(127),Constant::Integer(1)),Some(Constant::Integer(bits)) if bits==u128::MAX-127)
        );
        assert!(matches!(
            unary(Unary::Neg, Type::Ptr, Constant::Integer(1)),
            Some(Constant::Integer(u128::MAX))
        ));
    }
    #[test]
    fn trapping_integer_folds_remain_unknown_including_full_width_minimum_division() {
        for ty in [Type::I8, Type::I32, Type::I64, Type::I128] {
            let width = width(ty);
            let minimum = Constant::Integer(1 << (width - 1));
            let minus_one = Constant::Integer(u128::MAX);
            assert!(binary(Binary::Div, ty, minimum, minus_one).is_none());
            assert!(binary(Binary::Mod, ty, minimum, minus_one).is_none());
            assert!(binary(Binary::Div, ty, minimum, Constant::Integer(0)).is_none());
            assert!(
                binary(
                    Binary::Shl,
                    ty,
                    minimum,
                    Constant::Integer(u128::from(width))
                )
                .is_none()
            );
        }
        assert!(unary(Unary::Ctz, Type::I32, Constant::Integer(0)).is_none());
        assert!(unary(Unary::ByteSwap, Type::I128, Constant::Integer(1)).is_none());
        assert!(
            binary(
                Binary::Add,
                Type::I1,
                Constant::Integer(0),
                Constant::Integer(1)
            )
            .is_none()
        );
    }
    #[test]
    fn floating_sign_negation_is_a_bit_operation_and_equality_is_not_bit_identity() {
        let bits = 0xffff_0000_0000_7fff_8000_0000_0000_0001_u128;
        let a = Constant::Float(FloatKind::F80, bits);
        assert!(
            matches!(unary(Unary::Neg,Type::F80,a),Some(Constant::Float(FloatKind::F80,b)) if b==bits^(1<<79))
        );
        let zero = Constant::Float(FloatKind::F64, 0);
        let negative_zero = Constant::Float(FloatKind::F64, 1 << 63);
        assert!(!same(Type::F64, zero, negative_zero));
        assert!(matches!(
            compare(Compare::Eq, Type::F64, zero, negative_zero),
            Some(Constant::Integer(1))
        ));
        let nan = Constant::Float(FloatKind::F32, 0x7fc0_0001);
        assert!(same(Type::F32, nan, nan));
        assert!(matches!(
            compare(Compare::Eq, Type::F32, nan, nan),
            Some(Constant::Integer(0))
        ));
    }
    #[test]
    fn nonfinite_results_and_right_operand_overflow_keep_the_frozen_sign() {
        let zero = Constant::Float(FloatKind::F64, 0);
        let one = Constant::Float(FloatKind::F64, 0x3ff0_0000_0000_0000);
        assert!(matches!(
            binary(Binary::Div, Type::F64, one, zero),
            Some(Constant::Float(FloatKind::F64, 0x7ff0_0000_0000_0000))
        ));
        assert!(matches!(
            binary(Binary::Div, Type::F64, zero, zero),
            Some(Constant::Float(FloatKind::F64, 0xfff8_0000_0000_0000))
        ));
        let huge = Constant::Float(FloatKind::F80, (0x7ffe << 64) | u128::from(u64::MAX));
        assert!(matches!(
            binary(Binary::Sub, Type::F64, zero, huge),
            Some(Constant::Float(FloatKind::F64, 0xfff0_0000_0000_0000))
        ));
        let negative_one = Constant::Float(FloatKind::F64, 0xbff0_0000_0000_0000);
        assert!(matches!(
            binary(Binary::Add, Type::F64, negative_one, huge),
            Some(Constant::Float(FloatKind::F64, 0x7ff0_0000_0000_0000))
        ));
    }
    #[test]
    fn conversions_check_truncated_ranges_and_preserve_extension_rules() {
        assert!(matches!(
            convert(
                Convert::SignExtend,
                Type::I64,
                Type::U8,
                Constant::Integer(255)
            ),
            Some(Constant::Integer(u128::MAX))
        ));
        assert!(matches!(
            convert(
                Convert::ZeroExtend,
                Type::I64,
                Type::I8,
                Constant::Integer(u128::MAX)
            ),
            Some(Constant::Integer(255))
        ));
        let negative_fraction = Constant::Float(FloatKind::F64, 0xbfe0_0000_0000_0000);
        assert!(matches!(
            convert(
                Convert::FloatToUnsigned,
                Type::U8,
                Type::F64,
                negative_fraction
            ),
            Some(Constant::Integer(0))
        ));
        let limit = Constant::Float(FloatKind::F64, 0x4060_0000_0000_0000);
        assert!(convert(Convert::FloatToSigned, Type::I8, Type::F64, limit).is_none());
        assert!(
            convert(
                Convert::SignedToFloat,
                Type::F64,
                Type::I128,
                Constant::Integer(1)
            )
            .is_none()
        );
    }
    #[test]
    fn extended_storage_preserves_raw_encodings_while_arithmetic_uses_x87_rules() {
        let unsupported = Constant::Float(FloatKind::F80, (0x3fff << 64) | 1);
        assert!(
            matches!(convert(Convert::FloatExtend, Type::F80, Type::F80, unsupported), Some(Constant::Float(FloatKind::F80, bits)) if bits == (0x3fff << 64) | 1)
        );
        assert!(matches!(
            convert(Convert::FloatTruncate, Type::F64, Type::F80, unsupported),
            Some(Constant::Float(FloatKind::F64, 0xfff8_0000_0000_0000))
        ));
        let nan = Constant::Float(FloatKind::F80, (0x7fff << 64) | (3 << 62) | 3);
        assert!(
            matches!(binary(Binary::Add, Type::F80, unsupported, nan), Some(Constant::Float(FloatKind::F80, bits)) if bits == (0xffff << 64) | (3 << 62))
        );
        let pseudo = Constant::Float(FloatKind::F80, 1 << 63);
        assert!(
            matches!(convert(Convert::FloatExtend, Type::F80, Type::F80, pseudo), Some(Constant::Float(FloatKind::F80, bits)) if bits == 1 << 63)
        );
        assert!(
            matches!(binary(Binary::Add, Type::F80, pseudo, Constant::Float(FloatKind::F80, 0)), Some(Constant::Float(FloatKind::F80, bits)) if bits == (1 << 64) | (1 << 63))
        );
    }
}
