//! Target floating bits, scanning and canonical `LowIR` spelling. See NOTICE.
use super::{Error, vocabulary::Type};
use crate::{types::FundamentalType, types::floating::FloatingScanner};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FloatKind {
    F32,
    F64,
    F80,
}
impl FloatKind {
    pub(crate) fn from_type(ty: Type) -> Option<Self> {
        match ty {
            Type::F32 => Some(Self::F32),
            Type::F64 => Some(Self::F64),
            Type::F80 => Some(Self::F80),
            _ => None,
        }
    }
    fn fundamental(self) -> FundamentalType {
        match self {
            Self::F32 => FundamentalType::Float,
            Self::F64 => FundamentalType::Double,
            Self::F80 => FundamentalType::LongDouble,
        }
    }
    pub(crate) fn ty(self) -> Type {
        match self {
            Self::F32 => Type::F32,
            Self::F64 => Type::F64,
            Self::F80 => Type::F80,
        }
    }
    fn layout(self) -> (u32, u32, i64) {
        match self {
            Self::F32 => (23, 8, 127),
            Self::F64 => (52, 11, 1023),
            Self::F80 => (63, 15, 16_383),
        }
    }
    fn sign(self) -> u128 {
        1 << match self {
            Self::F32 => 31,
            Self::F64 => 63,
            Self::F80 => 79,
        }
    }
    fn suffix(self) -> &'static str {
        match self {
            Self::F32 => "f",
            Self::F64 => "",
            Self::F80 => "L",
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Constant {
    Integer(u128),
    Float(FloatKind, u128),
}
pub(crate) fn floating_spelling(text: &str) -> bool {
    let hex = text.starts_with("0x") || text.starts_with("0X");
    text.contains('.')
        || text.bytes().any(|c| {
            if hex {
                matches!(c, b'p' | b'P')
            } else {
                matches!(c, b'e' | b'E')
            }
        })
        || (!hex && text.len() > 1 && text.ends_with(['f', 'F', 'l', 'L']))
}
fn suffix(text: &str) -> FloatKind {
    if text.ends_with(['f', 'F']) {
        FloatKind::F32
    } else if text.ends_with(['l', 'L']) {
        FloatKind::F80
    } else {
        FloatKind::F64
    }
}
pub(crate) fn special(text: &str, context: Type, negative: bool) -> Constant {
    let written = if matches!(text, "inf" | "INFINITY" | "nan" | "NAN" | "snan" | "SNAN") {
        FloatKind::F64
    } else {
        suffix(text)
    };
    let kind = FloatKind::from_type(context).unwrap_or(written);
    let base = text.trim_end_matches(['f', 'F', 'l', 'L']);
    let (fraction, exponent, _) = kind.layout();
    let shift = if kind == FloatKind::F80 { 64 } else { fraction };
    let mut bits = ((1_u128 << exponent) - 1) << shift;
    if kind == FloatKind::F80 {
        bits |= 1 << 63;
    }
    if base.eq_ignore_ascii_case("nan") {
        bits |= 1 << (fraction - 1);
    } else if base.eq_ignore_ascii_case("snan") {
        bits |= 1 << (fraction - 2);
    }
    if negative {
        bits |= kind.sign();
    }
    Constant::Float(kind, bits)
}

/// Validate the C strto* grammar before using the shared exact rational scanner.
pub(crate) fn scan(
    scanner: &mut FloatingScanner,
    text: &str,
    context: Type,
    negative: bool,
) -> Result<Option<Constant>, Error> {
    let hex = text.starts_with("0x") || text.starts_with("0X");
    let exponent_marker = if hex { ['p', 'P'] } else { ['e', 'E'] };
    let written = if !hex || text.contains(exponent_marker) {
        suffix(text)
    } else {
        FloatKind::F64
    };
    let digits = if written == FloatKind::F64 {
        text
    } else {
        &text[..text.len() - 1]
    };
    let kind = FloatKind::from_type(context).unwrap_or(written);
    let (mantissa, exponent) = digits
        .split_once(exponent_marker)
        .map_or((digits, None), |(m, e)| (m, Some(e)));
    let mantissa = if hex { &mantissa[2..] } else { mantissa };
    let mut points = 0;
    let mut count = 0;
    for c in mantissa.bytes() {
        if c == b'.' {
            points += 1;
        } else if c.is_ascii_digit() || hex && c.is_ascii_hexdigit() {
            count += 1;
        } else {
            return Ok(None);
        }
    }
    if count == 0 || points > 1 {
        return Ok(None);
    }
    let mut exp = 0_i64;
    if let Some(e) = exponent {
        let negative = e.starts_with('-');
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        if e.is_empty() || !e.bytes().all(|c| c.is_ascii_digit()) {
            return Ok(None);
        }
        for c in e.bytes() {
            if exp < 1_000_000_000 {
                exp = exp * 10 + i64::from(c - b'0');
            }
        }
        if negative {
            exp = -exp;
        }
    }
    let bits = if hex {
        scanner
            .hexadecimal(mantissa.as_bytes(), exp, kind.fundamental())
            .map_err(|_| Error::Limit)?
    } else {
        scanner
            .scan(digits.as_bytes(), kind.fundamental())
            .map_err(|_| Error::Limit)?
            .map(|bytes| {
                let mut data = [0; 16];
                data[..bytes.as_slice().len()].copy_from_slice(bytes.as_slice());
                u128::from_le_bytes(data)
            })
    };
    Ok(bits.map(|bits| Constant::Float(kind, bits | if negative { kind.sign() } else { 0 })))
}

/// Match C++'s integer-to-x87 addition, followed by the target conversion.
pub(crate) fn integer_as_float(
    scanner: &mut FloatingScanner,
    bits: u128,
    kind: FloatKind,
    negative_zero: bool,
) -> Result<u128, Error> {
    let negative = bits >> 127 != 0;
    let magnitude = if negative { bits.wrapping_neg() } else { bits };
    let extended = scanner
        .binary(magnitude, 0, FundamentalType::LongDouble)
        .map_err(|_| Error::Limit)?
        .ok_or(Error::Limit)?;
    let (significand, exponent, _) = finite(FloatKind::F80, extended)?;
    let result = scanner
        .binary(significand, exponent, kind.fundamental())
        .map_err(|_| Error::Limit)?;
    // Every signed 128-bit integer is finite even in binary32.
    let bits = result.ok_or(Error::Limit)?;
    Ok(bits
        | if negative || negative_zero && magnitude == 0 {
            kind.sign()
        } else {
            0
        })
}
fn finite(kind: FloatKind, encoding: u128) -> Result<(u128, i64, bool), Error> {
    let (fraction, exponent_width, bias) = kind.layout();
    let shift = if kind == FloatKind::F80 { 64 } else { fraction };
    let exp = i64::try_from((encoding >> shift) & ((1 << exponent_width) - 1))
        .map_err(|_| Error::Limit)?;
    let significand = if kind == FloatKind::F80 {
        encoding & u128::from(u64::MAX)
    } else {
        (encoding & ((1 << fraction) - 1)) | if exp == 0 { 0 } else { 1 << fraction }
    };
    Ok((
        significand,
        exp.max(1) - bias - i64::from(fraction),
        exp == 0,
    ))
}

pub(crate) fn text(
    scanner: &mut FloatingScanner,
    kind: FloatKind,
    bits: u128,
    marked: bool,
) -> Result<String, Error> {
    let (fraction, exponent_bits, _) = kind.layout();
    let shift = if kind == FloatKind::F80 { 64 } else { fraction };
    let negative = bits & kind.sign() != 0;
    let sign = if negative { "-" } else { "" };
    let mantissa = bits & ((1 << fraction) - 1);
    if (bits >> shift) & ((1 << exponent_bits) - 1) == (1 << exponent_bits) - 1 {
        let name = if mantissa == 0 {
            match kind {
                FloatKind::F32 => "INFINITYf",
                FloatKind::F64 => "inf",
                FloatKind::F80 => "INFINITYL",
            }
        } else if mantissa & (1 << (fraction - 1)) != 0 {
            "nan"
        } else {
            "snan"
        };
        return Ok(if mantissa == 0 {
            format!("{sign}{name}")
        } else {
            format!("{sign}{name}{}", kind.suffix())
        });
    }
    let (significand, exponent, subnormal) = finite(kind, bits)?;
    let mut out = if significand == 0 {
        "0".into()
    } else if subnormal {
        hexadecimal(kind, significand)
    } else {
        let (digits, scale) = scanner
            .decimal_binary(significand, exponent)
            .map_err(|_| Error::Limit)?;
        let first = general_decimal(&digits, scale, 20)?;
        if kind == FloatKind::F80 {
            let read = scan(scanner, &first, kind.ty(), false)?;
            if matches!(read, Some(Constant::Float(_, b)) if b == bits & !kind.sign()) {
                first
            } else {
                general_decimal(&digits, scale, 21)?
            }
        } else {
            first
        }
    };
    if kind != FloatKind::F64 {
        out.push_str(kind.suffix());
    } else if marked && !out.contains(['.', 'e', 'p']) {
        out.push_str(".0");
    }
    Ok(format!("{sign}{out}"))
}
fn hexadecimal(kind: FloatKind, significand: u128) -> String {
    let (lead, fraction, digits, exponent) = match kind {
        FloatKind::F32 => {
            let top = significand.ilog2();
            let shift = 52 - top;
            (
                1,
                (significand << shift) & ((1_u128 << 52) - 1),
                13,
                i64::from(top) - 149,
            )
        }
        FloatKind::F64 => (0, significand, 13, -1022),
        FloatKind::F80 => (
            significand >> 60,
            significand & ((1_u128 << 60) - 1),
            15,
            -16_385,
        ),
    };
    let mut tail = format!("{fraction:0digits$x}")
        .trim_end_matches('0')
        .to_owned();
    if !tail.is_empty() {
        tail.insert(0, '.');
    }
    format!("0x{lead:x}{tail}p{exponent:+}")
}
fn general_decimal(digits: &str, scale: i64, precision: usize) -> Result<String, Error> {
    let mut exponent = i64::try_from(digits.len()).map_err(|_| Error::Limit)? + scale - 1;
    let mut rounded = digits.as_bytes()[..digits.len().min(precision)].to_vec();
    if digits.len() > precision {
        let rest = &digits.as_bytes()[precision..];
        let odd = !(rounded[precision - 1] - b'0').is_multiple_of(2);
        if rest[0] > b'5' || rest[0] == b'5' && (odd || rest[1..].iter().any(|&c| c != b'0')) {
            let mut carry = true;
            for digit in rounded.iter_mut().rev() {
                if *digit == b'9' {
                    *digit = b'0';
                } else {
                    *digit += 1;
                    carry = false;
                    break;
                }
            }
            if carry {
                rounded[0] = b'1';
                exponent += 1;
            }
        }
    }
    while rounded.len() > 1 && rounded.last() == Some(&b'0') {
        rounded.pop();
    }
    let mut rounded = String::from_utf8(rounded).map_err(|_| Error::Limit)?;
    if exponent < -4 || exponent >= i64::try_from(precision).map_err(|_| Error::Limit)? {
        if rounded.len() > 1 {
            rounded.insert(1, '.');
        }
        Ok(format!("{rounded}e{exponent:+03}"))
    } else if exponent < 0 {
        Ok(format!(
            "0.{}{rounded}",
            "0".repeat(usize::try_from(-exponent - 1).map_err(|_| Error::Limit)?)
        ))
    } else {
        let point = usize::try_from(exponent + 1).map_err(|_| Error::Limit)?;
        if point < rounded.len() {
            rounded.insert(point, '.');
        } else {
            rounded.push_str(&"0".repeat(point - rounded.len()));
        }
        Ok(rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_conversion_retains_x87_intermediate_rounding() -> Result<(), Error> {
        let mut scratch = FloatingScanner::default();
        // Just above a binary64 midpoint, but rounded to that midpoint in x87.
        let magnitude = (1_u128 << 100) + (1_u128 << 47) + 1;
        let staged = integer_as_float(&mut scratch, magnitude, FloatKind::F64, false)?;
        let direct = scratch
            .binary(magnitude, 0, FundamentalType::Double)
            .map_err(|_| Error::Limit)?
            .ok_or(Error::Limit)?;
        assert_eq!(staged, (1123_u128) << 52);
        assert_eq!(direct, staged + 1);
        assert_eq!(
            integer_as_float(&mut scratch, 0, FloatKind::F80, true)?,
            1 << 79
        );
        Ok(())
    }

    #[test]
    fn canonical_text_retains_special_bits_and_smallest_subnormals() -> Result<(), Error> {
        let mut scratch = FloatingScanner::default();
        assert!(matches!(
            special("inf", Type::Void, false),
            Constant::Float(FloatKind::F64, 0x7ff0_0000_0000_0000)
        ));
        assert!(matches!(
            special("inff", Type::Void, false),
            Constant::Float(FloatKind::F32, 0x7f80_0000)
        ));
        for (kind, bits, spelling) in [
            (FloatKind::F32, 1, "0x1p-149f"),
            (FloatKind::F64, 1, "0x0.0000000000001p-1022"),
            (FloatKind::F80, 1, "0x0.000000000000001p-16385L"),
            (FloatKind::F64, 1 << 63, "-0.0"),
            (FloatKind::F32, 0x7fa0_0000, "snanf"),
            (
                FloatKind::F80,
                (0x7fff_u128 << 64) | 0xa000_0000_0000_0000,
                "snanL",
            ),
        ] {
            assert_eq!(text(&mut scratch, kind, bits, true)?, spelling);
        }
        Ok(())
    }
}
