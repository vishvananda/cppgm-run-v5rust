//! One-token, borrowed-buffer `LowIR` scanner. See NOTICE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    End,
    Word,
    Number,
    Temp,
    Slot,
    Global,
    Block,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Equals,
    Plus,
    Minus,
    Arrow,
    Debug,
    Invalid(&'static str),
}
#[derive(Clone, Copy)]
pub(super) struct DebugToken<'a> {
    pub(super) file: &'a [u8],
    pub(super) line: u64,
    pub(super) column: u64,
}
#[derive(Clone, Copy)]
pub(super) struct Token<'a> {
    pub(super) kind: Kind,
    pub(super) text: &'a [u8],
    pub(super) line: u32,
    pub(super) debug: Option<DebugToken<'a>>,
}
impl<'a> Token<'a> {
    pub(super) fn word(self) -> &'a str {
        std::str::from_utf8(self.text).unwrap_or("")
    }
    pub(super) fn append_spelling(self, out: &mut Vec<u8>) {
        let sigil = match self.kind {
            Kind::Temp => "%",
            Kind::Slot => "$",
            Kind::Global => "@",
            Kind::Block => "^",
            Kind::End => {
                out.extend_from_slice(b"end of input");
                return;
            }
            _ => "",
        };
        out.extend_from_slice(sigil.as_bytes());
        out.extend_from_slice(self.text);
    }
}
pub(super) struct Lexer<'a> {
    source: &'a [u8],
    position: usize,
    line: u32,
    pub(super) token: Token<'a>,
}
impl<'a> Lexer<'a> {
    pub(super) fn new(source: &'a [u8]) -> Self {
        let mut lexer = Self {
            source,
            position: 0,
            line: 1,
            token: Token {
                kind: Kind::End,
                text: &[],
                line: 1,
                debug: None,
            },
        };
        lexer.advance();
        lexer
    }
    fn space(&mut self) {
        while let Some(&c) = self.source.get(self.position) {
            if c == b'#' {
                while self.source.get(self.position).is_some_and(|&c| c != b'\n') {
                    self.position += 1;
                }
            } else if is_space(c) {
                if c == b'\n' {
                    self.line = self.line.wrapping_add(1);
                }
                self.position += 1;
            } else {
                break;
            }
        }
    }
    pub(super) fn advance(&mut self) {
        self.space();
        let mut start = self.position;
        let line = self.line;
        let Some(&c) = self.source.get(start) else {
            self.token = Token {
                kind: Kind::End,
                text: &[],
                line,
                debug: None,
            };
            return;
        };
        self.position += 1;
        let mut debug = None;
        let kind = match c {
            b'%' | b'$' | b'@' | b'^' => {
                start += 1;
                while self
                    .source
                    .get(self.position)
                    .is_some_and(|&c| name_char(c))
                {
                    self.position += 1;
                }
                if self.position == start {
                    Kind::Invalid("expected a name after its sigil")
                } else {
                    match c {
                        b'%' => Kind::Temp,
                        b'$' => Kind::Slot,
                        b'@' => Kind::Global,
                        _ => Kind::Block,
                    }
                }
            }
            b'-' => {
                if self.source.get(self.position) == Some(&b'>') {
                    self.position += 1;
                    Kind::Arrow
                } else {
                    Kind::Minus
                }
            }
            b'(' => Kind::LParen,
            b')' => Kind::RParen,
            b'[' => Kind::LBracket,
            b']' => Kind::RBracket,
            b'{' => Kind::LBrace,
            b'}' => Kind::RBrace,
            b',' => Kind::Comma,
            b':' => Kind::Colon,
            b'=' => Kind::Equals,
            b'+' => Kind::Plus,
            b'!' => {
                let (kind, payload) = self.scan_debug(start);
                debug = payload;
                kind
            }
            c if c.is_ascii_digit()
                || (c == b'.'
                    && self
                        .source
                        .get(self.position)
                        .is_some_and(u8::is_ascii_digit)) =>
            {
                let hex = c == b'0' && matches!(self.source.get(self.position), Some(b'x' | b'X'));
                while let Some(&c) = self.source.get(self.position) {
                    let exponent = if hex {
                        matches!(c, b'p' | b'P')
                    } else {
                        matches!(c, b'e' | b'E')
                    };
                    if exponent && matches!(self.source.get(self.position + 1), Some(b'+' | b'-')) {
                        self.position += 2;
                    } else if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.') {
                        self.position += 1;
                    } else {
                        break;
                    }
                }
                Kind::Number
            }
            c if name_char(c) => {
                while self
                    .source
                    .get(self.position)
                    .is_some_and(|&c| name_char(c))
                {
                    self.position += 1;
                }
                if &self.source[start..self.position] == b"obj"
                    && self.source.get(self.position) == Some(&b'<')
                {
                    while self
                        .source
                        .get(self.position)
                        .is_some_and(|&c| c != b'>' && !is_space(c))
                    {
                        self.position += 1;
                    }
                    if self.source.get(self.position) == Some(&b'>') {
                        self.position += 1;
                    }
                }
                Kind::Word
            }
            _ => Kind::Invalid("unexpected character"),
        };
        self.token = Token {
            kind,
            text: &self.source[start..self.position],
            line,
            debug,
        };
    }
    fn scan_debug(&mut self, start: usize) -> (Kind, Option<DebugToken<'a>>) {
        let malformed = Kind::Invalid("malformed !dbg(file, line, column) location");
        if !self.source[start..].starts_with(b"!dbg") {
            return (Kind::Invalid("unexpected character"), None);
        }
        self.position = start + 4;
        if !self.debug_punctuation(b'(') {
            return (malformed, None);
        }
        self.space();
        let file_start = self.position;
        while self
            .source
            .get(self.position)
            .is_some_and(|&c| !is_space(c) && !matches!(c, b',' | b'(' | b')'))
        {
            self.position += 1;
        }
        if self.position == file_start {
            return (malformed, None);
        }
        let file = &self.source[file_start..self.position];
        if !self.debug_punctuation(b',') {
            return (malformed, None);
        }
        let Some(line) = self.debug_decimal() else {
            return (malformed, None);
        };
        if !self.debug_punctuation(b',') {
            return (malformed, None);
        }
        let Some(column) = self.debug_decimal() else {
            return (malformed, None);
        };
        if !self.debug_punctuation(b')') {
            return (malformed, None);
        }
        (Kind::Debug, Some(DebugToken { file, line, column }))
    }
    fn debug_punctuation(&mut self, expected: u8) -> bool {
        self.space();
        if self.source.get(self.position) != Some(&expected) {
            return false;
        }
        self.position += 1;
        true
    }
    fn debug_decimal(&mut self) -> Option<u64> {
        self.space();
        let start = self.position;
        let mut value = 0_u64;
        let mut overflow = false;
        while let Some(&digit) = self
            .source
            .get(self.position)
            .filter(|c| c.is_ascii_digit())
        {
            overflow |= value > (u64::MAX - 9) / 10;
            value = value.wrapping_mul(10).wrapping_add(u64::from(digit - b'0'));
            self.position += 1;
        }
        if self.position == start {
            None
        } else if overflow || self.source[start] == b'0' {
            Some(0)
        } else {
            Some(value)
        }
    }
}
fn name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'$')
}

fn is_space(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}
