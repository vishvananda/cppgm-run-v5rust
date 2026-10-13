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
pub(super) struct Token<'a> {
    pub(super) kind: Kind,
    pub(super) text: &'a [u8],
    pub(super) line: u32,
}
impl<'a> Token<'a> {
    pub(super) fn word(self) -> &'a str {
        std::str::from_utf8(self.text).unwrap_or("")
    }
    pub(super) fn spelling(self) -> String {
        let sigil = match self.kind {
            Kind::Temp => "%",
            Kind::Slot => "$",
            Kind::Global => "@",
            Kind::Block => "^",
            Kind::End => return "end of input".into(),
            _ => "",
        };
        format!("{sigil}{}", String::from_utf8_lossy(self.text))
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
            };
            return;
        };
        self.position += 1;
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
            b'!' => Kind::Debug,
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
        };
    }
}
fn name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'$')
}

fn is_space(c: u8) -> bool {
    c == b' ' || (b'\t'..=b'\r').contains(&c)
}
