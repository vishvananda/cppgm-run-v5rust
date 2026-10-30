// Translated from lex/gnu_builtins.cpp::kGnuKeywords. See NOTICE.
//! Source-parser keyword aliases; ordinary phase-7 tools retain identifiers.
use super::vocabulary::Keyword;

#[derive(Clone, Copy)]
pub enum SyntaxKeyword {
    Keyword(Keyword),
    Ignored,
}
#[must_use]
pub fn syntax_keyword(spelling: &[u8]) -> Option<SyntaxKeyword> {
    Some(match spelling {
        b"__inline" | b"__inline__" => SyntaxKeyword::Keyword(Keyword::Inline),
        b"__const" | b"__const__" => SyntaxKeyword::Keyword(Keyword::Const),
        b"__volatile" | b"__volatile__" => SyntaxKeyword::Keyword(Keyword::Volatile),
        b"__signed" | b"__signed__" => SyntaxKeyword::Keyword(Keyword::Signed),
        b"__decltype" => SyntaxKeyword::Keyword(Keyword::Decltype),
        b"__alignof" | b"__alignof__" => SyntaxKeyword::Keyword(Keyword::Alignof),
        b"__asm" | b"__asm__" => SyntaxKeyword::Keyword(Keyword::Asm),
        b"__thread" => SyntaxKeyword::Keyword(Keyword::ThreadLocal),
        b"__restrict" | b"__restrict__" | b"__extension__" => SyntaxKeyword::Ignored,
        _ => return None,
    })
}
