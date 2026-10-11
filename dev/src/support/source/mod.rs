//! Immutable physical source and stateless phase 1–2 views.

mod translation;
pub(crate) use translation::{END_OF_SOURCE, SourceChar, TranslationReader};

/// Compact byte identity within one physical source buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceOffset(pub(crate) u32);

impl SourceOffset {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

/// A fatal lexical error; presentation is deferred until the tool boundary.
#[derive(Debug)]
pub struct SourceError {
    pub(crate) offset: SourceOffset,
    pub(crate) message: &'static str,
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(self.message)
    }
}

impl std::error::Error for SourceError {}

impl SourceError {
    pub(crate) fn new(offset: SourceOffset, message: &'static str) -> Self {
        Self { offset, message }
    }
}

/// One retained image, including the implicit final newline when required.
pub struct SourceBuffer {
    bytes: Vec<u8>,
    begin: SourceOffset,
}

impl SourceBuffer {
    /// Apply the whole-file phase 1–2 rules without decoding the source.
    ///
    /// # Errors
    /// Returns an error if the image cannot fit in the compact offset space.
    pub fn new(mut bytes: Vec<u8>) -> Result<Self, &'static str> {
        if bytes.len() >= u32::MAX as usize {
            return Err("source file is too large");
        }
        let begin = if bytes.starts_with(b"\xef\xbb\xbf") {
            3
        } else {
            0
        };
        if bytes.len() > begin
            && (!bytes.ends_with(b"\n")
                || bytes[begin..].ends_with(b"\\\n")
                || bytes[begin..].ends_with(b"??/\n"))
        {
            bytes.push(b'\n');
        }
        Ok(Self {
            bytes,
            begin: SourceOffset(u32::try_from(begin).map_err(|_| "source file is too large")?),
        })
    }

    /// Token-paste input already past phases 1–2: keep any BOM and append a newline.
    ///
    /// # Errors
    /// Returns an error if the text cannot fit in the compact offset space.
    pub fn for_token_text(mut bytes: Vec<u8>) -> Result<Self, &'static str> {
        if bytes.len() >= u32::MAX as usize {
            return Err("token text is too large");
        }
        bytes.push(b'\n');
        Ok(Self {
            bytes,
            begin: SourceOffset(0),
        })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(crate) fn begin(&self) -> SourceOffset {
        self.begin
    }

    /// Render physical lines and byte columns, excluding a leading BOM.
    #[must_use]
    pub fn format_error(&self, error: &SourceError) -> String {
        let offset = error
            .offset
            .index()
            .clamp(self.begin.index(), self.bytes.len());
        let position = SourceLineCursor::new(self).advance_to_index(offset);
        format!("{}:{}: {}", position.line, position.column, error.message)
    }
}

/// Lazily derived physical line and byte column.
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

/// Monotonic location renderer; never stores one position per character or token.
pub struct SourceLineCursor<'a> {
    source: &'a SourceBuffer,
    offset: usize,
    line_start: usize,
    line: usize,
}

impl<'a> SourceLineCursor<'a> {
    #[must_use]
    pub fn new(source: &'a SourceBuffer) -> Self {
        Self {
            source,
            offset: source.begin.index(),
            line_start: source.begin.index(),
            line: 1,
        }
    }

    /// Locate a buffer offset at or after the last request.
    ///
    /// # Errors
    /// Rejects offsets outside the source or behind the previous request.
    pub fn advance_to(&mut self, offset: SourceOffset) -> Result<SourcePosition, SourceError> {
        let offset = offset.index();
        if offset < self.offset || offset > self.source.bytes.len() {
            return Err(SourceError::new(
                self.source.begin,
                "invalid source cursor offset",
            ));
        }
        Ok(self.advance_to_index(offset))
    }

    fn advance_to_index(&mut self, offset: usize) -> SourcePosition {
        for (i, &byte) in self.source.bytes[self.offset..offset].iter().enumerate() {
            if byte == b'\n' {
                self.line_start = self.offset + i + 1;
                self.line += 1;
            }
        }
        self.offset = offset;
        SourcePosition {
            line: self.line,
            column: offset - self.line_start + 1,
        }
    }
}
