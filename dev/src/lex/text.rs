// Translated from lex/text_literal.cpp. See NOTICE.
//! Quoted spelling boundaries and typed scalar versus numeric escape values.
use super::number::user_defined_suffix;
use crate::support::unicode::decode_utf8;
use crate::types::FundamentalType;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EncodingPrefix {
    #[default]
    None,
    Utf8,
    Utf16,
    Utf32,
    Wide,
}

impl EncodingPrefix {
    #[must_use]
    pub fn width(self) -> usize {
        match self {
            Self::Utf16 => 2,
            Self::Utf32 | Self::Wide => 4,
            Self::None | Self::Utf8 => 1,
        }
    }

    #[must_use]
    pub fn element_type(self) -> FundamentalType {
        match self {
            Self::Utf16 => FundamentalType::Char16T,
            Self::Utf32 => FundamentalType::Char32T,
            Self::Wide => FundamentalType::WcharT,
            Self::None | Self::Utf8 => FundamentalType::Char,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextParts {
    pub prefix: EncodingPrefix,
    pub raw: bool,
    pub quote: u8,
    pub body_begin: u32,
    pub body_end: u32,
    pub suffix_begin: u32,
}

#[must_use]
pub fn split_text(text: &[u8]) -> Option<TextParts> {
    let (prefix, mut quote) = if text.starts_with(b"u8") {
        (EncodingPrefix::Utf8, 2)
    } else {
        match text.first()? {
            b'u' => (EncodingPrefix::Utf16, 1),
            b'U' => (EncodingPrefix::Utf32, 1),
            b'L' => (EncodingPrefix::Wide, 1),
            _ => (EncodingPrefix::None, 0),
        }
    };
    let raw = text.get(quote) == Some(&b'R');
    quote += usize::from(raw);
    let mark = *text.get(quote)?;
    if !matches!(mark, b'\'' | b'"') || (raw && mark != b'"') {
        return None;
    }
    let close = text.iter().rposition(|&c| c == mark)?;
    if close == quote {
        return None;
    }
    let (body_begin, body_end) = if raw {
        let open = quote + 1 + text[quote + 1..close].iter().position(|&c| c == b'(')?;
        let delimiter = &text[quote + 1..open];
        let end = close.checked_sub(delimiter.len() + 1)?;
        if end <= open || text[end] != b')' || &text[end + 1..close] != delimiter {
            return None;
        }
        (open + 1, end)
    } else {
        (quote + 1, close)
    };
    Some(TextParts {
        prefix,
        raw,
        quote: mark,
        body_begin: u32::try_from(body_begin).ok()?,
        body_end: u32::try_from(body_end).ok()?,
        suffix_begin: u32::try_from(close + 1).ok()?,
    })
}

/// Numeric escapes designate one code unit, even if it is a surrogate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralCharacter {
    Scalar(char),
    Numeric(u64),
}

impl LiteralCharacter {
    fn value(self) -> u64 {
        match self {
            Self::Scalar(c) => u64::from(c),
            Self::Numeric(n) => n,
        }
    }
}

pub fn decode_character(text: &[u8], pos: &mut usize) -> Option<LiteralCharacter> {
    if *text.get(*pos)? != b'\\' {
        let decoded = decode_utf8(&text[*pos..]).ok()?;
        *pos += usize::from(decoded.length);
        return Some(LiteralCharacter::Scalar(decoded.scalar));
    }
    let introducer = *text.get(*pos + 1)?;
    *pos += 2;
    let simple = match introducer {
        b'\'' | b'"' | b'?' | b'\\' => Some(introducer),
        b'a' => Some(7),
        b'b' => Some(8),
        b'f' => Some(12),
        b'n' => Some(10),
        b'r' => Some(13),
        b't' => Some(9),
        b'v' => Some(11),
        _ => None,
    };
    if let Some(c) = simple {
        return Some(LiteralCharacter::Scalar(char::from(c)));
    }
    if matches!(introducer, b'u' | b'U') {
        let digits = if introducer == b'u' { 4 } else { 8 };
        let mut value = 0u32;
        for _ in 0..digits {
            value = (value << 4) | hex(*text.get(*pos)?)?;
            *pos += 1;
        }
        return char::from_u32(value).map(LiteralCharacter::Scalar);
    }
    if introducer == b'x' {
        let first = *pos;
        let mut value = 0u64;
        while let Some(digit) = text.get(*pos).and_then(|&c| hex(c)) {
            // Frozen scanner saturates rather than wrapping values above 32 bits.
            value = ((value << 4) | u64::from(digit)).min(0x1_0000_0000);
            *pos += 1;
        }
        return (*pos != first).then_some(LiteralCharacter::Numeric(value));
    }
    if !matches!(introducer, b'0'..=b'7') {
        return None;
    }
    let mut value = u64::from(introducer - b'0');
    for _ in 1..3 {
        let Some(&c @ b'0'..=b'7') = text.get(*pos) else {
            break;
        };
        value = value * 8 + u64::from(c - b'0');
        *pos += 1;
    }
    Some(LiteralCharacter::Numeric(value))
}

fn hex(c: u8) -> Option<u32> {
    super::pp_lexer::classes::hex_value(u32::from(c))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterValue {
    pub ty: FundamentalType,
    pub code_point: u32,
    pub multicharacter: bool,
    pub initializer_bound: u32,
    pub suffix_begin: usize,
}

/// Evaluate once, retaining multicharacter facts for compiler consumers.
#[must_use]
pub fn evaluate_character(text: &[u8]) -> Option<CharacterValue> {
    let parts = split_text(text)?;
    let suffix_begin = usize::try_from(parts.suffix_begin).ok()?;
    let body_end = usize::try_from(parts.body_end).ok()?;
    let body_begin = usize::try_from(parts.body_begin).ok()?;
    if parts.quote != b'\''
        || parts.prefix == EncodingPrefix::Utf8
        || (suffix_begin != text.len() && !user_defined_suffix(&text[suffix_begin..]))
    {
        return None;
    }
    let body = &text[..body_end];
    let mut pos = body_begin;
    let c = decode_character(body, &mut pos)?.value();
    if pos != body_end {
        if parts.prefix != EncodingPrefix::None || suffix_begin != text.len() || c > 255 {
            return None;
        }
        let mut value = u32::try_from(c).ok()?;
        let mut initializer_bound = 2u32;
        while pos != body_end {
            let next = u32::try_from(decode_character(body, &mut pos)?.value()).ok()?;
            if next > 255 {
                return None;
            }
            value = value.wrapping_shl(8) | next;
            initializer_bound = initializer_bound.checked_add(1)?;
        }
        return Some(CharacterValue {
            ty: FundamentalType::Int,
            code_point: value,
            multicharacter: true,
            initializer_bound,
            suffix_begin,
        });
    }
    let code_point = u32::try_from(c).ok()?;
    char::from_u32(code_point)?;
    let ty = match parts.prefix {
        EncodingPrefix::None => {
            if code_point <= 127 {
                FundamentalType::Char
            } else {
                FundamentalType::Int
            }
        }
        EncodingPrefix::Utf16 if code_point > 0xffff => return None,
        prefix => prefix.element_type(),
    };
    Some(CharacterValue {
        ty,
        code_point,
        multicharacter: false,
        initializer_bound: 2,
        suffix_begin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_facts_preserve_target_packing_and_scalar_limits() {
        let value = evaluate_character(b"'ABCDE'").unwrap();
        assert!(value.multicharacter);
        assert_eq!(value.ty, FundamentalType::Int);
        assert_eq!(value.code_point, 0x4243_4445);
        assert_eq!(value.initializer_bound, 6);
        for text in [
            b"'AB'_x".as_slice(),
            b"u'AB'",
            b"'\\xD800'",
            b"'\\x100000000'",
            b"'a'suffix",
            b"u'\\x10000'",
        ] {
            assert_eq!(evaluate_character(text), None);
        }
        let value = evaluate_character(b"U'\\x10000'_x").unwrap();
        assert_eq!(value.ty, FundamentalType::Char32T);
        assert_eq!(value.code_point, 0x10000);
        assert!(!value.multicharacter);
    }
}
