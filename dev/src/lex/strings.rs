// Translated from lex/string_concatenation.cpp. See NOTICE.
//! One maximal phase-6 sequence, with reusable spelling and code-unit storage.
use super::number::{NumberDialect, literal_suffix};
use super::text::{EncodingPrefix, LiteralCharacter, TextParts, decode_character, split_text};
use crate::support::unicode::{append_utf8, decode_utf8};
use crate::types::FundamentalType;
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub enum TextError {
    HostSize,
}

struct Piece {
    offset: u32,
    length: u32,
    parts: Option<TextParts>,
}

#[derive(Default)]
pub struct StringSequence {
    source: Vec<u8>,
    pieces: Vec<Piece>,
    units: Vec<u8>,
}

pub struct StringValue {
    pub ty: FundamentalType,
    pub elements: u32,
    pub initializer_bound: u64,
    pub suffix: Option<Range<usize>>,
}

impl StringSequence {
    #[inline]
    pub fn clear(&mut self) {
        self.source.clear();
        self.pieces.clear();
        self.units.clear();
    }

    #[inline]
    #[must_use]
    pub fn source(&self) -> &[u8] {
        &self.source[..target_length(self.source.len())]
    }
    #[inline]
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.units[..target_length(self.units.len())]
    }

    #[inline]
    #[must_use]
    pub fn suffix(&self, range: Range<usize>) -> &[u8] {
        &self.source[range]
    }

    pub fn add(&mut self, text: &[u8]) -> Result<(), TextError> {
        let separator = usize::from(!self.pieces.is_empty());
        let offset = self
            .source
            .len()
            .checked_add(separator)
            .ok_or(TextError::HostSize)?;
        let end = offset.checked_add(text.len()).ok_or(TextError::HostSize)?;
        // C++ pieces expose their offsets as uint32_t, independently of the
        // host-sized joined buffer. Preserve that target truncation explicitly.
        let offset = u32::try_from(target_length(offset)).map_err(|_| TextError::HostSize)?;
        let length = u32::try_from(text.len()).map_err(|_| TextError::HostSize)?;
        if separator != 0 {
            self.source.push(b' ');
        }
        self.source.extend_from_slice(text);
        debug_assert_eq!(self.source.len(), end);
        let begin = usize::try_from(offset).map_err(|_| TextError::HostSize)?;
        let piece_end = begin.checked_add(text.len()).ok_or(TextError::HostSize)?;
        // A wrapped offset names an earlier region, just as the C++ owner does.
        // Parse that region once, before encoding consumes its recorded bounds.
        let parts = self.source.get(begin..piece_end).and_then(split_text);
        self.pieces.push(Piece {
            offset,
            length,
            parts,
        });
        Ok(())
    }

    pub fn concatenate(
        &mut self,
        dialect: NumberDialect,
    ) -> Result<Option<StringValue>, TextError> {
        self.units.clear();
        let Some((prefix, suffix)) = self.combine(dialect) else {
            return Ok(None);
        };
        let mut initializer_bound = 1u64;
        for piece in &self.pieces {
            let Some(parts) = piece.parts else {
                return Ok(None);
            };
            let offset = usize::try_from(piece.offset).map_err(|_| TextError::HostSize)?;
            let length = usize::try_from(piece.length).map_err(|_| TextError::HostSize)?;
            let mut piece_bound = 0;
            if !encode_body(
                &mut self.units,
                &self.source[offset..offset + length],
                parts,
                prefix,
                &mut piece_bound,
            ) {
                return Ok(None);
            }
            initializer_bound = initializer_bound.wrapping_add(piece_bound);
        }
        append_unit(&mut self.units, 0, prefix.width());
        let bytes =
            u32::try_from(target_length(self.units.len())).map_err(|_| TextError::HostSize)?;
        let width = u32::try_from(prefix.width()).map_err(|_| TextError::HostSize)?;
        Ok(Some(StringValue {
            ty: prefix.element_type(),
            elements: bytes / width,
            initializer_bound,
            suffix,
        }))
    }

    fn combine(&self, dialect: NumberDialect) -> Option<(EncodingPrefix, Option<Range<usize>>)> {
        let mut prefix = EncodingPrefix::None;
        let mut suffix: Option<Range<usize>> = None;
        for piece in &self.pieces {
            let parts = piece.parts?;
            if parts.quote != b'"' {
                return None;
            }
            if parts.prefix != EncodingPrefix::None {
                if prefix != EncodingPrefix::None && prefix != parts.prefix {
                    return None;
                }
                prefix = parts.prefix;
            }
            let offset = usize::try_from(piece.offset).ok()?;
            let end = offset + usize::try_from(piece.length).ok()?;
            let range = offset + usize::try_from(parts.suffix_begin).ok()?..end;
            if range.is_empty() {
                continue;
            }
            if !literal_suffix(&self.source[range.clone()], dialect) {
                return None;
            }
            if let Some(prior) = &suffix {
                if self.source[prior.clone()] != self.source[range] {
                    return None;
                }
            } else {
                suffix = Some(range);
            }
        }
        Some((prefix, suffix))
    }
}

// Target token lengths are uint32_t even when a host buffer grows larger.
fn target_length(length: usize) -> usize {
    length & u32::MAX as usize
}

fn encode_body(
    units: &mut Vec<u8>,
    text: &[u8],
    parts: TextParts,
    prefix: EncodingPrefix,
    initializer_bound: &mut u64,
) -> bool {
    let width = prefix.width();
    let (Ok(body_begin), Ok(body_end)) = (
        usize::try_from(parts.body_begin),
        usize::try_from(parts.body_end),
    ) else {
        return false;
    };
    let text = &text[..body_end];
    let mut pos = body_begin;
    if parts.raw {
        *initializer_bound = u64::from(parts.body_end - parts.body_begin);
    }
    if width == 1 && parts.raw {
        units.extend_from_slice(&text[pos..]);
        return true;
    }
    while pos < text.len() {
        if width == 1 && text[pos] != b'\\' {
            let end = pos
                + text[pos..]
                    .iter()
                    .position(|&c| c == b'\\')
                    .unwrap_or(text.len() - pos);
            let run = &text[pos..end];
            *initializer_bound = run.iter().fold(*initializer_bound, |count, &byte| {
                count.wrapping_add(u64::from(byte & 0xc0 != 0x80))
            });
            units.extend_from_slice(run);
            pos = end;
            continue;
        }
        let c = if parts.raw {
            let Ok(decoded) = decode_utf8(&text[pos..]) else {
                return false;
            };
            pos += usize::from(decoded.length);
            LiteralCharacter::Scalar(decoded.scalar)
        } else {
            let Some(c) = decode_character(text, &mut pos) else {
                return false;
            };
            c
        };
        if !parts.raw {
            *initializer_bound = initializer_bound.wrapping_add(1);
        }
        match c {
            LiteralCharacter::Numeric(value) => {
                let maximum = match width {
                    1 => 255,
                    2 => 65535,
                    _ => u64::from(u32::MAX),
                };
                if value > maximum {
                    return false;
                }
                let Ok(unit) = u32::try_from(value) else {
                    return false;
                };
                append_unit(units, unit, width);
            }
            LiteralCharacter::Scalar(c) => append_scalar(units, c, width),
        }
    }
    true
}

fn append_unit(units: &mut Vec<u8>, value: u32, width: usize) {
    units.extend_from_slice(&value.to_le_bytes()[..width]);
}

fn append_scalar(units: &mut Vec<u8>, c: char, width: usize) {
    if width == 1 {
        append_utf8(units, c);
    } else if width == 2 && u32::from(c) > 0xffff {
        let offset = u32::from(c) - 0x10000;
        append_unit(units, 0xd800 | (offset >> 10), width);
        append_unit(units, 0xdc00 | (offset & 0x3ff), width);
    } else {
        append_unit(units, u32::from(c), width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializer_bounds_and_encoded_units_retain_distinct_source_rules() {
        for (source, elements, initializer_bound) in [
            ("u\"🙂\"", 3, 2),
            ("uR\"(🙂)\"", 3, 5),
            ("u8\"π\"", 3, 2),
            ("u8R\"(π)\"", 3, 3),
            ("u\"\\xD800\"", 2, 2),
        ] {
            let mut strings = StringSequence::default();
            strings.add(source.as_bytes()).unwrap();
            let value = strings.concatenate(NumberDialect::Cxx11).unwrap().unwrap();
            assert_eq!(value.elements, elements);
            assert_eq!(value.initializer_bound, initializer_bound);
        }
        let mut strings = StringSequence::default();
        strings.add(b"u\"a\"").unwrap();
        strings.add("\"🙂\"".as_bytes()).unwrap();
        let value = strings.concatenate(NumberDialect::Cxx11).unwrap().unwrap();
        assert_eq!(value.elements, 4);
        assert_eq!(value.initializer_bound, 3);
    }

    #[test]
    fn sequence_reset_releases_contents_and_keeps_only_capacity() {
        let mut strings = StringSequence::default();
        strings.add(b"u\"abc\"_x").unwrap();
        strings.add(b"U\"def\"_y").unwrap();
        assert!(strings.concatenate(NumberDialect::Cxx11).unwrap().is_none());
        let capacity = strings.source.capacity();
        strings.clear();
        assert!(strings.source().is_empty());
        assert!(strings.pieces.is_empty());
        assert!(strings.data().is_empty());
        assert_eq!(strings.source.capacity(), capacity);
        strings.add(b"\"\\xff\"").unwrap();
        let value = strings.concatenate(NumberDialect::Cxx11).unwrap().unwrap();
        assert_eq!(value.ty, FundamentalType::Char);
        assert!(value.suffix.is_none());
        assert_eq!(value.elements, 2);
        assert_eq!(strings.data(), [255, 0]);
    }

    #[test]
    fn joined_spelling_and_array_size_use_target_32_bit_lengths() {
        let boundary = usize::try_from(u64::from(u32::MAX) + 1).unwrap();
        assert_eq!(target_length(boundary - 1), u32::MAX as usize);
        assert_eq!(target_length(boundary), 0);
        assert_eq!(target_length(boundary + 4), 4);
        assert_eq!(
            target_length(boundary + 4) / EncodingPrefix::Utf32.width(),
            1
        );
    }
}
