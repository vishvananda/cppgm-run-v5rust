//! Safe UTF-8 primitives shared by source translation and literal processing.

/// Exact decoding failure categories used by the frozen compiler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Utf8Error {
    InvalidLeadingByte,
    InvalidContinuationByte,
    Truncated,
    InvalidScalarValue,
}

impl Utf8Error {
    /// Observable diagnostic spelling.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidLeadingByte => "invalid UTF-8 leading byte",
            Self::InvalidContinuationByte => "invalid UTF-8 continuation byte",
            Self::Truncated => "truncated UTF-8 character",
            Self::InvalidScalarValue => "invalid UTF-8 scalar value",
        }
    }
}

/// A validated scalar and its physical byte extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Utf8Decoded {
    pub scalar: char,
    pub length: u8,
}

/// Decode only the first character, preserving the oracle's error precedence.
///
/// # Errors
/// Reports the first malformed character; empty input reports truncation.
pub fn decode_utf8(bytes: &[u8]) -> Result<Utf8Decoded, Utf8Error> {
    let Some(&lead) = bytes.first() else {
        return Err(Utf8Error::Truncated);
    };
    let (length, mut code, minimum): (u8, u32, u32) = match lead {
        0..=0x7f => {
            return Ok(Utf8Decoded {
                scalar: char::from(lead),
                length: 1,
            });
        }
        0xc2..=0xdf => (2, u32::from(lead & 0x1f), 0x80),
        0xe0..=0xef => (3, u32::from(lead & 0xf), 0x800),
        0xf0..=0xf4 => (4, u32::from(lead & 7), 0x10000),
        _ => return Err(Utf8Error::InvalidLeadingByte),
    };
    for &byte in bytes.iter().take(usize::from(length)).skip(1) {
        if byte & 0xc0 != 0x80 {
            return Err(Utf8Error::InvalidContinuationByte);
        }
        code = (code << 6) | u32::from(byte & 0x3f);
    }
    if bytes.len() < usize::from(length) {
        return Err(Utf8Error::Truncated);
    }
    if code < minimum {
        return Err(Utf8Error::InvalidScalarValue);
    }
    let scalar = char::from_u32(code).ok_or(Utf8Error::InvalidScalarValue)?;
    Ok(Utf8Decoded { scalar, length })
}

/// Append the canonical UTF-8 encoding of a validated scalar.
#[inline]
pub fn append_utf8(out: &mut Vec<u8>, scalar: char) {
    let mut bytes = [0; 4];
    out.extend_from_slice(scalar.encode_utf8(&mut bytes).as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_sequences_preserve_failure_precedence() {
        for (bytes, error) in [
            (&b"\xc0\x80"[..], Utf8Error::InvalidLeadingByte),
            (&b"\xe0\x80\x80"[..], Utf8Error::InvalidScalarValue),
            (&b"\xed\xa0\x80"[..], Utf8Error::InvalidScalarValue),
            (&b"\xf4\x90\x80\x80"[..], Utf8Error::InvalidScalarValue),
            (&b"\xf0\x90"[..], Utf8Error::Truncated),
            (&b"\xf0x"[..], Utf8Error::InvalidContinuationByte),
        ] {
            assert_eq!(decode_utf8(bytes), Err(error));
        }
    }
}
