// Translated from lex/number_literal.cpp and lex/ud_suffix.cpp. See NOTICE.
//! Classify a pp-number once; retain its typed value or its deferred ud-suffix.
use crate::lex::pp_lexer::classes::{hex_value, identifier_continue, identifier_start};
use crate::types::FundamentalType;

/// The standalone course tools use the strict C++11 literal grammar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NumberDialect {
    #[default]
    Cxx11,
    /// Source compiler extensions: binary integers and hexadecimal floating numbers.
    Gnu,
    /// Preprocessor text view: GNU numeric forms with course suffix rules.
    GnuStream,
}

#[derive(Clone, Copy)]
pub struct NumberOptions {
    pub dialect: NumberDialect,
    pub binary_exponents: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberLiteral {
    Invalid,
    Integer {
        ty: FundamentalType,
        value: u64,
    },
    Floating {
        ty: FundamentalType,
        value_end: usize,
    },
    UserDefined {
        floating: bool,
        suffix_begin: usize,
    },
}

/// The suffix grammar is Annex E, including UTF-8 continuation characters.
#[inline]
#[must_use]
pub fn user_defined_suffix(text: &[u8]) -> bool {
    text.first() == Some(&b'_')
        && std::str::from_utf8(&text[1..])
            .is_ok_and(|suffix| suffix.chars().all(|c| identifier_continue(u32::from(c))))
}

#[must_use]
pub fn literal_suffix(text: &[u8], dialect: NumberDialect) -> bool {
    if dialect != NumberDialect::Gnu {
        return user_defined_suffix(text);
    }
    text.first()
        .is_some_and(|&byte| identifier_start(u32::from(byte)))
        && std::str::from_utf8(&text[1..])
            .is_ok_and(|tail| tail.chars().all(|c| identifier_continue(u32::from(c))))
}

/// Classify grammar and integer type/value without scanning floating data.
#[must_use]
#[inline]
pub fn classify_number(text: &[u8], dialect: NumberDialect) -> NumberLiteral {
    classify(text, dialect)
}

/// Decode an already classified ud-integer's digits. Raw literal operators
/// need no numeric value; only a selected cooked operator demands this check.
#[must_use]
pub fn user_integer_value(digits: &[u8]) -> Option<u64> {
    let (digits, radix) = if digits.starts_with(b"0x") || digits.starts_with(b"0X") {
        (&digits[2..], 16)
    } else if digits.starts_with(b"0b") || digits.starts_with(b"0B") {
        (&digits[2..], 2)
    } else {
        (digits, if digits.first() == Some(&b'0') { 8 } else { 10 })
    };
    digits.iter().try_fold(0u64, |value, &byte| {
        value
            .checked_mul(radix)?
            .checked_add(u64::from(hex_value(u32::from(byte))?))
    })
}

fn classify(text: &[u8], dialect: NumberDialect) -> NumberLiteral {
    if dialect != NumberDialect::Cxx11 && (text.starts_with(b"0b") || text.starts_with(b"0B")) {
        let mut end = 2;
        while matches!(text.get(end), Some(b'0' | b'1')) {
            end += 1;
        }
        return if end == 2 {
            NumberLiteral::Invalid
        } else {
            classify_integer(text, 2, end, 2, dialect)
        };
    }
    if text.starts_with(b"0x") || text.starts_with(b"0X") {
        let mut end = 2;
        while text.get(end).is_some_and(u8::is_ascii_hexdigit) {
            end += 1;
        }
        let integer_end = end;
        let point = text.get(end) == Some(&b'.');
        if point {
            end += 1;
            while text.get(end).is_some_and(u8::is_ascii_hexdigit) {
                end += 1;
            }
        }
        let has_digits = integer_end > 2 || point && end > integer_end + 1;
        if dialect != NumberDialect::Cxx11 && has_digits {
            let exponent = exponent_end(text, end, *b"pP");
            if exponent != end {
                return classify_floating(text, exponent, dialect);
            }
        }
        return if integer_end == 2 || point {
            NumberLiteral::Invalid
        } else {
            classify_integer(text, 2, integer_end, 16, dialect)
        };
    }
    let mut pos = 0;
    while text.get(pos).is_some_and(u8::is_ascii_digit) {
        pos += 1;
    }
    let integer_digits = pos;
    let floating = text.get(pos) == Some(&b'.');
    let mut fraction_digits = 0;
    if floating {
        pos += 1;
        while text.get(pos).is_some_and(u8::is_ascii_digit) {
            fraction_digits += 1;
            pos += 1;
        }
    }
    if integer_digits + fraction_digits == 0 {
        return NumberLiteral::Invalid;
    }
    let exponent_end = exponent_end(text, pos, *b"eE");
    if floating || exponent_end != pos {
        return classify_floating(text, exponent_end, dialect);
    }
    if text[0] != b'0' {
        return classify_integer(text, 0, integer_digits, 10, dialect);
    }
    if text[1..integer_digits]
        .iter()
        .any(|c| !matches!(c, b'0'..=b'7'))
    {
        return NumberLiteral::Invalid;
    }
    classify_integer(text, 0, integer_digits, 8, dialect)
}

fn exponent_end(text: &[u8], pos: usize, markers: [u8; 2]) -> usize {
    if !text.get(pos).is_some_and(|byte| markers.contains(byte)) {
        return pos;
    }
    let mut end = pos + 1;
    if matches!(text.get(end), Some(b'+' | b'-')) {
        end += 1;
    }
    let digits = end;
    while text.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if end == digits { pos } else { end }
}

fn classify_floating(text: &[u8], value_end: usize, dialect: NumberDialect) -> NumberLiteral {
    let ty = match &text[value_end..] {
        b"" => FundamentalType::Double,
        b"f" | b"F" => FundamentalType::Float,
        b"l" | b"L" => FundamentalType::LongDouble,
        b"q" | b"Q" if dialect != NumberDialect::Cxx11 => FundamentalType::GnuFloat128,
        [b'f' | b'F', rest @ ..]
            if dialect != NumberDialect::Cxx11
                && matches!(rest, b"16" | b"32" | b"64" | b"128" | b"32x" | b"64x") =>
        {
            match rest {
                b"16" => FundamentalType::Float16,
                b"32" => FundamentalType::Float32,
                b"64" => FundamentalType::Float64,
                b"128" => FundamentalType::Float128,
                b"32x" => FundamentalType::Float32x,
                _ => FundamentalType::Float64x,
            }
        }
        suffix if literal_suffix(suffix, dialect) => {
            return NumberLiteral::UserDefined {
                floating: true,
                suffix_begin: value_end,
            };
        }
        _ => return NumberLiteral::Invalid,
    };
    NumberLiteral::Floating { ty, value_end }
}

#[derive(Clone, Copy)]
struct IntegerSuffix {
    unsigned: bool,
    longs: u8,
}

fn take_unsigned(text: &[u8], pos: &mut usize) -> bool {
    if matches!(text.get(*pos), Some(b'u' | b'U')) {
        *pos += 1;
        true
    } else {
        false
    }
}

fn integer_suffix(text: &[u8]) -> Option<IntegerSuffix> {
    let mut pos = 0;
    let mut unsigned = take_unsigned(text, &mut pos);
    let mut longs = 0;
    if matches!(text.get(pos), Some(b'l' | b'L')) {
        longs = if text.get(pos + 1) == text.get(pos) {
            2
        } else {
            1
        };
        pos += usize::from(longs);
    }
    if !unsigned {
        unsigned = take_unsigned(text, &mut pos);
    }
    (pos == text.len()).then_some(IntegerSuffix { unsigned, longs })
}

fn candidates(suffix: IntegerSuffix, decimal: bool) -> &'static [FundamentalType] {
    use FundamentalType::{
        Int, LongInt, LongLongInt, UnsignedInt, UnsignedLongInt, UnsignedLongLongInt,
    };
    match (suffix.longs, suffix.unsigned, decimal) {
        (0, false, true) => &[Int, LongInt, LongLongInt],
        (0, false, false) => &[
            Int,
            UnsignedInt,
            LongInt,
            UnsignedLongInt,
            LongLongInt,
            UnsignedLongLongInt,
        ],
        (0, true, _) => &[UnsignedInt, UnsignedLongInt, UnsignedLongLongInt],
        (1, false, true) => &[LongInt, LongLongInt],
        (1, false, false) => &[LongInt, UnsignedLongInt, LongLongInt, UnsignedLongLongInt],
        (1, true, _) => &[UnsignedLongInt, UnsignedLongLongInt],
        (_, false, true) => &[LongLongInt],
        (_, false, false) => &[LongLongInt, UnsignedLongLongInt],
        (_, true, _) => &[UnsignedLongLongInt],
    }
}

fn classify_integer(
    text: &[u8],
    begin: usize,
    end: usize,
    radix: u64,
    dialect: NumberDialect,
) -> NumberLiteral {
    let Some(suffix) = integer_suffix(&text[end..]) else {
        // A literal operator, demanded later, owns ud-integer range checks.
        return if literal_suffix(&text[end..], dialect) {
            NumberLiteral::UserDefined {
                floating: false,
                suffix_begin: end,
            }
        } else {
            NumberLiteral::Invalid
        };
    };
    let value = text[begin..end].iter().try_fold(0u64, |value, &c| {
        value
            .checked_mul(radix)?
            .checked_add(u64::from(hex_value(u32::from(c))?))
    });
    let Some(value) = value else {
        return NumberLiteral::Invalid;
    };
    candidates(suffix, radix == 10)
        .iter()
        .copied()
        .find(|ty| value <= ty.integer_max())
        .map_or(NumberLiteral::Invalid, |ty| NumberLiteral::Integer {
            ty,
            value,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_dialect_keeps_hex_exponents_and_ud_digit_boundaries() {
        assert_eq!(
            classify_number(b"0b101ULL", NumberDialect::Gnu),
            NumberLiteral::Integer {
                ty: FundamentalType::UnsignedLongLongInt,
                value: 5
            }
        );
        assert_eq!(
            classify_number(b"0x1.fp+2_x", NumberDialect::Gnu),
            NumberLiteral::UserDefined {
                floating: true,
                suffix_begin: 8
            }
        );
        assert_eq!(
            classify_number(b"0X.8P-1f", NumberDialect::Gnu),
            NumberLiteral::Floating {
                ty: FundamentalType::Float,
                value_end: 7
            }
        );
        assert_eq!(
            classify_number(b"12tag", NumberDialect::Gnu),
            NumberLiteral::UserDefined {
                floating: false,
                suffix_begin: 2
            }
        );
        for spelling in [b"0x1.fp+2_x".as_slice(), b"0b101ULL", b"12tag"] {
            assert_eq!(
                classify_number(spelling, NumberDialect::Cxx11),
                NumberLiteral::Invalid
            );
        }
        for spelling in [b"0x.p1".as_slice(), b"0x1.p+_x", b"0x1._x", b"0b102"] {
            assert_eq!(
                classify_number(spelling, NumberDialect::Gnu),
                NumberLiteral::Invalid
            );
        }
    }

    #[test]
    fn decimal_and_nondecimal_follow_distinct_candidate_orders() {
        for (text, ty, value) in [
            ("2147483648", FundamentalType::LongInt, 0x8000_0000),
            ("0x80000000", FundamentalType::UnsignedInt, 0x8000_0000),
            (
                "0xffffffffffffffffL",
                FundamentalType::UnsignedLongInt,
                u64::MAX,
            ),
            (
                "18446744073709551615uLL",
                FundamentalType::UnsignedLongLongInt,
                u64::MAX,
            ),
        ] {
            assert_eq!(
                classify_number(text.as_bytes(), NumberDialect::Cxx11),
                NumberLiteral::Integer { ty, value }
            );
        }
        for text in [
            "18446744073709551616u",
            "9223372036854775808",
            "09_x",
            "0x",
            "1lL",
            "1u_x",
        ] {
            assert_eq!(
                classify_number(text.as_bytes(), NumberDialect::Cxx11),
                NumberLiteral::Invalid
            );
        }
    }

    #[test]
    fn grammar_and_deferred_range_do_not_depend_on_numeric_scanning() {
        for (text, floating, suffix_begin) in [
            ("18446744073709551616_x", false, 20),
            ("1e999999999_x", true, 11),
            ("123_e3", false, 3),
            ("1_π", false, 1),
        ] {
            assert_eq!(
                classify_number(text.as_bytes(), NumberDialect::Cxx11),
                NumberLiteral::UserDefined {
                    floating,
                    suffix_begin
                }
            );
        }
        for text in ["1e+", "1..", "0x1p3", "0b1", "1.0Q", "1.0f128"] {
            assert_eq!(
                classify_number(text.as_bytes(), NumberDialect::Cxx11),
                NumberLiteral::Invalid
            );
        }
    }
}
