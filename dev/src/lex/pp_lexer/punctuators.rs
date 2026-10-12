//! Longest-first punctuator spellings and the exceptional `<::` boundary.
use super::{PpLexer, SourceChar, SourceError};

/// Semantic identity shared by alternative spellings. No later phase must re-lex text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Punctuator {
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Semicolon,
    Question,
    Comma,
    Complement,
    HashHash,
    Hash,
    Scope,
    Colon,
    Ellipsis,
    DotStar,
    Dot,
    Increment,
    AddAssign,
    Add,
    ArrowStar,
    Decrement,
    SubtractAssign,
    Arrow,
    Subtract,
    MultiplyAssign,
    Multiply,
    DivideAssign,
    Divide,
    RemainderAssign,
    Remainder,
    XorAssign,
    Xor,
    LogicalAnd,
    AndAssign,
    BitAnd,
    LogicalOr,
    OrAssign,
    BitOr,
    NotEqual,
    LogicalNot,
    Equal,
    Assign,
    ShiftLeftAssign,
    ShiftLeft,
    LessEqual,
    Less,
    ShiftRightAssign,
    ShiftRight,
    GreaterEqual,
    Greater,
    New,
    Delete,
}

impl Punctuator {
    pub(crate) fn spelling(self) -> &'static str {
        match self {
            Self::LeftBrace => "{",
            Self::RightBrace => "}",
            Self::LeftBracket => "[",
            Self::RightBracket => "]",
            Self::LeftParen => "(",
            Self::RightParen => ")",
            Self::Semicolon => ";",
            Self::Question => "?",
            Self::Comma => ",",
            Self::Complement => "~",
            Self::HashHash => "##",
            Self::Hash => "#",
            Self::Scope => "::",
            Self::Colon => ":",
            Self::Ellipsis => "...",
            Self::DotStar => ".*",
            Self::Dot => ".",
            Self::Increment => "++",
            Self::AddAssign => "+=",
            Self::Add => "+",
            Self::ArrowStar => "->*",
            Self::Decrement => "--",
            Self::SubtractAssign => "-=",
            Self::Arrow => "->",
            Self::Subtract => "-",
            Self::MultiplyAssign => "*=",
            Self::Multiply => "*",
            Self::DivideAssign => "/=",
            Self::Divide => "/",
            Self::RemainderAssign => "%=",
            Self::Remainder => "%",
            Self::XorAssign => "^=",
            Self::Xor => "^",
            Self::LogicalAnd => "&&",
            Self::AndAssign => "&=",
            Self::BitAnd => "&",
            Self::LogicalOr => "||",
            Self::OrAssign => "|=",
            Self::BitOr => "|",
            Self::NotEqual => "!=",
            Self::LogicalNot => "!",
            Self::Equal => "==",
            Self::Assign => "=",
            Self::ShiftLeftAssign => "<<=",
            Self::ShiftLeft => "<<",
            Self::LessEqual => "<=",
            Self::Less => "<",
            Self::ShiftRightAssign => ">>=",
            Self::ShiftRight => ">>",
            Self::GreaterEqual => ">=",
            Self::Greater => ">",
            Self::New => "new",
            Self::Delete => "delete",
        }
    }
}

fn candidates(first: u32) -> &'static [(&'static [u8], Punctuator)] {
    match first {
        0x7b => &[(b"{", Punctuator::LeftBrace)],
        0x7d => &[(b"}", Punctuator::RightBrace)],
        0x5b => &[(b"[", Punctuator::LeftBracket)],
        0x5d => &[(b"]", Punctuator::RightBracket)],
        0x28 => &[(b"(", Punctuator::LeftParen)],
        0x29 => &[(b")", Punctuator::RightParen)],
        0x3b => &[(b";", Punctuator::Semicolon)],
        0x3f => &[(b"?", Punctuator::Question)],
        0x2c => &[(b",", Punctuator::Comma)],
        0x7e => &[(b"~", Punctuator::Complement)],
        0x23 => &[(b"##", Punctuator::HashHash), (b"#", Punctuator::Hash)],
        0x3a => &[
            (b"::", Punctuator::Scope),
            (b":>", Punctuator::RightBracket),
            (b":", Punctuator::Colon),
        ],
        0x2e => &[
            (b"...", Punctuator::Ellipsis),
            (b".*", Punctuator::DotStar),
            (b".", Punctuator::Dot),
        ],
        0x2b => &[
            (b"++", Punctuator::Increment),
            (b"+=", Punctuator::AddAssign),
            (b"+", Punctuator::Add),
        ],
        0x2d => &[
            (b"->*", Punctuator::ArrowStar),
            (b"--", Punctuator::Decrement),
            (b"-=", Punctuator::SubtractAssign),
            (b"->", Punctuator::Arrow),
            (b"-", Punctuator::Subtract),
        ],
        0x2a => &[
            (b"*=", Punctuator::MultiplyAssign),
            (b"*", Punctuator::Multiply),
        ],
        0x2f => &[
            (b"/=", Punctuator::DivideAssign),
            (b"/", Punctuator::Divide),
        ],
        0x25 => &[
            (b"%:%:", Punctuator::HashHash),
            (b"%=", Punctuator::RemainderAssign),
            (b"%>", Punctuator::RightBrace),
            (b"%:", Punctuator::Hash),
            (b"%", Punctuator::Remainder),
        ],
        0x5e => &[(b"^=", Punctuator::XorAssign), (b"^", Punctuator::Xor)],
        0x26 => &[
            (b"&&", Punctuator::LogicalAnd),
            (b"&=", Punctuator::AndAssign),
            (b"&", Punctuator::BitAnd),
        ],
        0x7c => &[
            (b"||", Punctuator::LogicalOr),
            (b"|=", Punctuator::OrAssign),
            (b"|", Punctuator::BitOr),
        ],
        0x21 => &[
            (b"!=", Punctuator::NotEqual),
            (b"!", Punctuator::LogicalNot),
        ],
        0x3d => &[(b"==", Punctuator::Equal), (b"=", Punctuator::Assign)],
        0x3c => &[
            (b"<<=", Punctuator::ShiftLeftAssign),
            (b"<<", Punctuator::ShiftLeft),
            (b"<=", Punctuator::LessEqual),
            (b"<%", Punctuator::LeftBrace),
            (b"<:", Punctuator::LeftBracket),
            (b"<", Punctuator::Less),
        ],
        0x3e => &[
            (b">>=", Punctuator::ShiftRightAssign),
            (b">>", Punctuator::ShiftRight),
            (b">=", Punctuator::GreaterEqual),
            (b">", Punctuator::Greater),
        ],
        _ => &[],
    }
}

pub(in crate::lex) fn identifier_operator(text: &[u8]) -> Option<Punctuator> {
    Some(match text {
        b"new" => Punctuator::New,
        b"delete" => Punctuator::Delete,
        b"and" => Punctuator::LogicalAnd,
        b"and_eq" => Punctuator::AndAssign,
        b"bitand" => Punctuator::BitAnd,
        b"bitor" => Punctuator::BitOr,
        b"compl" => Punctuator::Complement,
        b"not" => Punctuator::LogicalNot,
        b"not_eq" => Punctuator::NotEqual,
        b"or" => Punctuator::LogicalOr,
        b"or_eq" => Punctuator::OrAssign,
        b"xor" => Punctuator::Xor,
        b"xor_eq" => Punctuator::XorAssign,
        _ => return None,
    })
}

// Lookahead is lazy: a decoding error after a punctuator must be observed at
// the same point as in the C++ scanner, before that token can be emitted.
struct Lookahead {
    chars: [SourceChar; 4],
    count: usize,
}
impl Lookahead {
    fn at(&mut self, lexer: &PpLexer<'_>, index: usize) -> Result<SourceChar, SourceError> {
        while self.count <= index {
            self.chars[self.count] = lexer.after(self.chars[self.count - 1])?;
            self.count += 1;
        }
        Ok(self.chars[index])
    }
    fn spells(&mut self, lexer: &PpLexer<'_>, text: &[u8]) -> Result<bool, SourceError> {
        for (i, &byte) in text.iter().enumerate() {
            if self.at(lexer, i)?.code != u32::from(byte) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl PpLexer<'_> {
    pub(super) fn punctuator(&mut self) -> Result<bool, SourceError> {
        let candidates = candidates(self.current.code);
        if candidates.is_empty() {
            return Ok(false);
        }
        let mut ahead = Lookahead {
            chars: [self.current; 4],
            count: 1,
        };
        if ahead.spells(self, b"<::")? && !matches!(ahead.at(self, 3)?.code, 0x3a | 0x3e) {
            self.take(self.current)?;
            self.punctuator = Some(Punctuator::Less);
            return Ok(true);
        }
        for &(text, kind) in candidates {
            if !ahead.spells(self, text)? {
                continue;
            }
            for i in 0..text.len() - 1 {
                self.append(ahead.at(self, i)?)?;
            }
            self.take(ahead.at(self, text.len() - 1)?)?;
            self.punctuator = Some(kind);
            return Ok(true);
        }
        Ok(false)
    }
}
