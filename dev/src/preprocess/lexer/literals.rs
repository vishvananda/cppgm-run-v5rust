//! Literal validation, raw view switching, and contextual header names.
use super::classes::octal;
use super::{PpLexer, PpTokenKind, SourceChar, SourceError, basic, control, hex_value};
use crate::preprocess::source::{END_OF_SOURCE, SourceOffset};

impl PpLexer<'_> {
    pub(super) fn quoted_literal(
        &mut self,
        plain: PpTokenKind,
        user_defined: PpTokenKind,
    ) -> Result<PpTokenKind, SourceError> {
        let quote = self.current.code;
        self.take(self.current)?;
        loop {
            match self.current.code {
                c if c == quote => {
                    self.take(self.current)?;
                    return self.literal_suffix(plain, user_defined);
                }
                0x0a | END_OF_SOURCE => {
                    return Err(SourceError::new(self.begin, "unterminated quoted literal"));
                }
                0x5c => self.escape()?,
                _ => self.take(self.current)?,
            }
        }
    }

    fn escape(&mut self) -> Result<(), SourceError> {
        let backslash = self.current;
        let introducer = self.after(backslash)?;
        let c = introducer.code;
        if matches!(
            c,
            0x27 | 0x22 | 0x3f | 0x5c | 0x61 | 0x62 | 0x66 | 0x6e | 0x72 | 0x74 | 0x76
        ) {
            self.take(backslash)?;
            return self.take(introducer);
        }
        if octal(c) {
            self.take(backslash)?;
            for _ in 0..3 {
                if !octal(self.current.code) {
                    break;
                }
                self.take(self.current)?;
            }
            return Ok(());
        }
        if c == 0x78 {
            self.take(backslash)?;
            self.take(introducer)?;
            if hex_value(self.current.code).is_none() {
                return Err(SourceError::new(
                    backslash.begin,
                    "hex escape has no digits",
                ));
            }
            while hex_value(self.current.code).is_some() {
                self.take(self.current)?;
            }
            return Ok(());
        }
        if matches!(c, 0x75 | 0x55) {
            return self.literal_universal_name(backslash);
        }
        Err(SourceError::new(backslash.begin, "invalid escape sequence"))
    }

    fn literal_universal_name(&mut self, backslash: SourceChar) -> Result<(), SourceError> {
        let c = self
            .universal_name(backslash)?
            .ok_or_else(|| SourceError::new(backslash.begin, "invalid escape sequence"))?;
        if char::from_u32(c.code).is_none() {
            return Err(SourceError::new(
                backslash.begin,
                "invalid universal character value",
            ));
        }
        if control(c.code) || basic(c.code) {
            self.take(backslash)?;
            while self.current.begin < c.end {
                self.take(self.current)?;
            }
            return Ok(());
        }
        self.take(c)
    }

    fn literal_suffix(
        &mut self,
        plain: PpTokenKind,
        user_defined: PpTokenKind,
    ) -> Result<PpTokenKind, SourceError> {
        if !self.identifier_char(true)? {
            return Ok(plain);
        }
        while self.identifier_char(false)? {}
        Ok(user_defined)
    }

    pub(super) fn raw_literal(&mut self) -> Result<PpTokenKind, SourceError> {
        // The opening quote still uses the translated view. Everything after it,
        // including the delimiter, is scanned physically until the closing quote.
        self.append(self.current)?;
        let begin = self.current.end;
        let (end, verbatim) = self.raw_extent(begin)?;
        if verbatim {
            if !self.dirty && begin == self.clean_end {
                self.clean_end = end;
            } else {
                self.make_dirty();
                self.scratch
                    .extend_from_slice(&self.source.bytes()[begin.index()..end.index()]);
            }
            self.end = end;
        } else {
            let mut offset = begin;
            while offset < end {
                let c = self.reader.raw(offset)?;
                self.append(c)?;
                offset = c.end;
            }
        }
        self.current = self.reader.logical(end)?;
        self.literal_suffix(
            PpTokenKind::StringLiteral,
            PpTokenKind::UserDefinedStringLiteral,
        )
    }

    fn raw_extent(&self, begin: SourceOffset) -> Result<(SourceOffset, bool), SourceError> {
        let mut delimiter = [0u32; 16];
        let mut count = 0;
        let mut verbatim = true;
        let mut c = self.reader.raw(begin)?;
        while c.code != 0x28 {
            if c.code == END_OF_SOURCE {
                return Err(SourceError::new(
                    self.begin,
                    "unterminated raw string literal",
                ));
            }
            if matches!(c.code, 0x20 | 0x29 | 0x5c | 0x09 | 0x0b | 0x0c | 0x0a) {
                return Err(SourceError::new(c.begin, "invalid raw string delimiter"));
            }
            if count == delimiter.len() {
                return Err(SourceError::new(
                    c.begin,
                    "raw string delimiter is too long",
                ));
            }
            delimiter[count] = c.code;
            count += 1;
            verbatim &= !mapped_byte(c);
            c = self.reader.raw(c.end)?;
        }
        loop {
            c = self.reader.raw(c.end)?;
            if c.code == END_OF_SOURCE {
                return Err(SourceError::new(
                    self.begin,
                    "unterminated raw string literal",
                ));
            }
            verbatim &= !mapped_byte(c);
            if c.code != 0x29 {
                continue;
            }
            let mut close = c;
            let mut matched = 0;
            for expected in delimiter[..count]
                .iter()
                .copied()
                .chain(std::iter::once(0x22))
            {
                close = self.reader.raw(close.end)?;
                if close.code != expected {
                    break;
                }
                verbatim &= !mapped_byte(close);
                matched += 1;
            }
            if matched > count {
                return Ok((close.end, verbatim));
            }
        }
    }

    pub(super) fn header_name(&mut self) -> Result<bool, SourceError> {
        let close = if self.current.code == 0x3c {
            0x3e
        } else {
            0x22
        };
        let mut c = self.after(self.current)?;
        if c.code == close {
            return Ok(false);
        }
        while c.code != close {
            if matches!(c.code, 0x0a | END_OF_SOURCE) {
                return Ok(false);
            }
            c = self.after(c)?;
        }
        let end = c.end;
        while self.current.begin < end {
            if self.current.code == 0x5c
                && let Some(ucn) = self.universal_name(self.current)?
            {
                self.take(Self::outside_literal(ucn)?)?;
            } else {
                self.take(self.current)?;
            }
        }
        Ok(true)
    }
}

fn mapped_byte(c: SourceChar) -> bool {
    c.code >= 0x80 && c.end.0 - c.begin.0 == 1
}
