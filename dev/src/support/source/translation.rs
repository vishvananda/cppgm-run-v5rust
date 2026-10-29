//! Random access decoding; no record or allocation per source character.
use super::{SourceBuffer, SourceError, SourceOffset};
use crate::support::unicode::{Utf8Error, decode_utf8};

pub(crate) const END_OF_SOURCE: u32 = u32::MAX;

#[derive(Clone, Copy)]
pub(crate) struct SourceChar {
    pub code: u32,
    pub begin: SourceOffset,
    pub end: SourceOffset,
}

pub(crate) struct TranslationReader<'a> {
    bytes: &'a [u8],
    translate: bool,
}

impl<'a> TranslationReader<'a> {
    pub fn new(source: &'a SourceBuffer, translate: bool) -> Self {
        Self {
            bytes: source.bytes(),
            translate,
        }
    }

    #[inline]
    pub fn logical(&self, offset: SourceOffset) -> Result<SourceChar, SourceError> {
        if let Some(&byte) = self.bytes.get(offset.index())
            && byte < 0x80
            && byte != b'\\'
            && byte != b'?'
        {
            return Ok(SourceChar {
                code: u32::from(byte),
                begin: offset,
                end: SourceOffset(offset.0 + 1),
            });
        }
        self.logical_slow(offset)
    }

    fn logical_slow(&self, mut offset: SourceOffset) -> Result<SourceChar, SourceError> {
        if !self.translate {
            return self.raw(offset);
        }
        loop {
            let rest = &self.bytes[offset.index()..];
            let skip = if rest.starts_with(b"\\\n") {
                2
            } else if rest.starts_with(b"??/\n") {
                4
            } else {
                0
            };
            if skip == 0 {
                break;
            }
            offset.0 += skip;
        }
        let rest = &self.bytes[offset.index()..];
        if rest.starts_with(b"??") && rest.len() >= 3 {
            let code = match rest[2] {
                b'=' => b'#',
                b'/' => b'\\',
                b'\'' => b'^',
                b'(' => b'[',
                b')' => b']',
                b'!' => b'|',
                b'<' => b'{',
                b'>' => b'}',
                b'-' => b'~',
                _ => 0,
            };
            if code != 0 {
                return Ok(SourceChar {
                    code: u32::from(code),
                    begin: offset,
                    end: SourceOffset(offset.0 + 3),
                });
            }
        }
        self.raw(offset)
    }

    #[inline]
    pub fn raw(&self, offset: SourceOffset) -> Result<SourceChar, SourceError> {
        let bytes = &self.bytes[offset.index()..];
        let Some(&lead) = bytes.first() else {
            return Ok(SourceChar {
                code: END_OF_SOURCE,
                begin: offset,
                end: offset,
            });
        };
        if lead < 0x80 {
            return Ok(SourceChar {
                code: u32::from(lead),
                begin: offset,
                end: SourceOffset(offset.0 + 1),
            });
        }
        Self::multibyte(offset, bytes)
    }

    fn multibyte(offset: SourceOffset, bytes: &[u8]) -> Result<SourceChar, SourceError> {
        const WINDOWS_1252: [u32; 32] = [
            0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160,
            0x2039, 0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013,
            0x2014, 0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
        ];
        match decode_utf8(bytes) {
            Ok(decoded) => Ok(SourceChar {
                code: u32::from(decoded.scalar),
                begin: offset,
                end: SourceOffset(offset.0 + u32::from(decoded.length)),
            }),
            Err(Utf8Error::InvalidLeadingByte) if (0x80..=0xbf).contains(&bytes[0]) => {
                // The implementation-defined physical mapping permits stray
                // continuation bytes as Windows-1252; all other errors remain fatal.
                let byte = bytes[0];
                let code = if byte < 0xa0 {
                    WINDOWS_1252[usize::from(byte - 0x80)]
                } else {
                    u32::from(byte)
                };
                Ok(SourceChar {
                    code,
                    begin: offset,
                    end: SourceOffset(offset.0 + 1),
                })
            }
            Err(error) => Err(SourceError::new(offset, error.message())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_path_preserves_translation_and_error_boundaries() {
        let mut input: Vec<u8> = (0..=127).collect();
        input.extend_from_slice(b"\\\n??/\n??=??!??x\xc2\xaa\x80\xff");
        let source = SourceBuffer::for_token_text(input).unwrap();
        for translate in [false, true] {
            let reader = TranslationReader::new(&source, translate);
            for index in 0..=source.bytes().len() {
                let offset = SourceOffset(u32::try_from(index).unwrap());
                let value = |result: Result<SourceChar, SourceError>| {
                    result
                        .map(|c| (c.code, c.begin, c.end))
                        .map_err(|error| (error.offset, error.message))
                };
                assert_eq!(
                    value(reader.logical(offset)),
                    value(reader.logical_slow(offset))
                );
            }
        }
    }
}
