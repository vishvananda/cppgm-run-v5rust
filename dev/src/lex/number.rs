// Translated from lex/number_literal.cpp and lex/ud_suffix.cpp. See NOTICE.
//! Classify a pp-number once; retain its typed value or its deferred ud-suffix.
use crate::lex::pp_lexer::classes::{hex_value, identifier_continue};
use crate::types::FundamentalType;

/// The standalone course tools use the strict C++11 literal grammar.
#[derive(Clone, Copy, Debug, Default)]
pub enum NumberDialect {
    #[default]
    Cxx11,
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
pub(crate) fn user_defined_suffix(text: &[u8]) -> bool {
    text.first() == Some(&b'_')
        && std::str::from_utf8(&text[1..])
            .is_ok_and(|suffix| suffix.chars().all(|c| identifier_continue(u32::from(c))))
}

/// Classify grammar and integer type/value without scanning floating data.
#[must_use]
pub fn classify_number(text: &[u8], dialect: NumberDialect) -> NumberLiteral {
    match dialect {
        NumberDialect::Cxx11 => classify_cxx11(text),
    }
}

/// Decode an already classified ud-integer's digits. Raw literal operators
/// need no numeric value; only a selected cooked operator demands this check.
pub(crate) fn user_integer_value(digits: &[u8]) -> Option<u64> {
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

fn classify_cxx11(text: &[u8]) -> NumberLiteral {
    if text.starts_with(b"0x") || text.starts_with(b"0X") {
        let mut end = 2;
        while text.get(end).is_some_and(u8::is_ascii_hexdigit) {
            end += 1;
        }
        // Hexadecimal floats require a later dialect. A point never belongs
        // to an integer suffix, and an empty digit sequence is invalid.
        return if end == 2 || text.get(end) == Some(&b'.') {
            NumberLiteral::Invalid
        } else {
            classify_integer(text, 2, end, 16)
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
    let exponent_end = exponent_end(text, pos);
    if floating || exponent_end != pos {
        return classify_floating(text, exponent_end);
    }
    if text[0] != b'0' {
        return classify_integer(text, 0, integer_digits, 10);
    }
    if text[1..integer_digits]
        .iter()
        .any(|c| !matches!(c, b'0'..=b'7'))
    {
        return NumberLiteral::Invalid;
    }
    classify_integer(text, 0, integer_digits, 8)
}

fn exponent_end(text: &[u8], pos: usize) -> usize {
    if !matches!(text.get(pos), Some(b'e' | b'E')) {
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

fn classify_floating(text: &[u8], value_end: usize) -> NumberLiteral {
    let ty = match &text[value_end..] {
        b"" => FundamentalType::Double,
        b"f" | b"F" => FundamentalType::Float,
        b"l" | b"L" => FundamentalType::LongDouble,
        suffix if user_defined_suffix(suffix) => {
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

fn classify_integer(text: &[u8], begin: usize, end: usize, radix: u64) -> NumberLiteral {
    let Some(suffix) = integer_suffix(&text[end..]) else {
        // A literal operator, demanded later, owns ud-integer range checks.
        return if user_defined_suffix(&text[end..]) {
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
