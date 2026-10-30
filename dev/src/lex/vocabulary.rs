// (C) 2013 CPPGM Foundation. See NOTICE.
//! Phase-7 keyword identity and presentation vocabulary.
use crate::lex::pp_lexer::Punctuator;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Keyword {
    Alignas,
    Alignof,
    Asm,
    Auto,
    Bool,
    Break,
    Case,
    Catch,
    Char,
    Char16T,
    Char32T,
    Class,
    Const,
    Constexpr,
    ConstCast,
    Continue,
    Decltype,
    Default,
    Delete,
    Do,
    Double,
    DynamicCast,
    Else,
    Enum,
    Explicit,
    Export,
    Extern,
    False,
    Float,
    For,
    Friend,
    Goto,
    If,
    Inline,
    Int,
    Long,
    Mutable,
    Namespace,
    New,
    Noexcept,
    Nullptr,
    Operator,
    Private,
    Protected,
    Public,
    Register,
    ReinterpretCast,
    Return,
    Short,
    Signed,
    Sizeof,
    Static,
    StaticAssert,
    StaticCast,
    Struct,
    Switch,
    Template,
    This,
    ThreadLocal,
    Throw,
    True,
    Try,
    Typedef,
    Typeid,
    Typename,
    Union,
    Unsigned,
    Using,
    Virtual,
    Void,
    Volatile,
    WcharT,
    While,
}

impl Keyword {
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Alignas => "alignas",
            Self::Alignof => "alignof",
            Self::Asm => "asm",
            Self::Auto => "auto",
            Self::Bool => "bool",
            Self::Break => "break",
            Self::Case => "case",
            Self::Catch => "catch",
            Self::Char => "char",
            Self::Char16T => "char16_t",
            Self::Char32T => "char32_t",
            Self::Class => "class",
            Self::Const => "const",
            Self::Constexpr => "constexpr",
            Self::ConstCast => "const_cast",
            Self::Continue => "continue",
            Self::Decltype => "decltype",
            Self::Default => "default",
            Self::Delete => "delete",
            Self::Do => "do",
            Self::Double => "double",
            Self::DynamicCast => "dynamic_cast",
            Self::Else => "else",
            Self::Enum => "enum",
            Self::Explicit => "explicit",
            Self::Export => "export",
            Self::Extern => "extern",
            Self::False => "false",
            Self::Float => "float",
            Self::For => "for",
            Self::Friend => "friend",
            Self::Goto => "goto",
            Self::If => "if",
            Self::Inline => "inline",
            Self::Int => "int",
            Self::Long => "long",
            Self::Mutable => "mutable",
            Self::Namespace => "namespace",
            Self::New => "new",
            Self::Noexcept => "noexcept",
            Self::Nullptr => "nullptr",
            Self::Operator => "operator",
            Self::Private => "private",
            Self::Protected => "protected",
            Self::Public => "public",
            Self::Register => "register",
            Self::ReinterpretCast => "reinterpret_cast",
            Self::Return => "return",
            Self::Short => "short",
            Self::Signed => "signed",
            Self::Sizeof => "sizeof",
            Self::Static => "static",
            Self::StaticAssert => "static_assert",
            Self::StaticCast => "static_cast",
            Self::Struct => "struct",
            Self::Switch => "switch",
            Self::Template => "template",
            Self::This => "this",
            Self::ThreadLocal => "thread_local",
            Self::Throw => "throw",
            Self::True => "true",
            Self::Try => "try",
            Self::Typedef => "typedef",
            Self::Typeid => "typeid",
            Self::Typename => "typename",
            Self::Union => "union",
            Self::Unsigned => "unsigned",
            Self::Using => "using",
            Self::Virtual => "virtual",
            Self::Void => "void",
            Self::Volatile => "volatile",
            Self::WcharT => "wchar_t",
            Self::While => "while",
        }
    }

    pub(super) fn find(text: &[u8]) -> Option<Self> {
        Some(match text {
            b"alignas" => Self::Alignas,
            b"alignof" => Self::Alignof,
            b"asm" => Self::Asm,
            b"auto" => Self::Auto,
            b"bool" => Self::Bool,
            b"break" => Self::Break,
            b"case" => Self::Case,
            b"catch" => Self::Catch,
            b"char" => Self::Char,
            b"char16_t" => Self::Char16T,
            b"char32_t" => Self::Char32T,
            b"class" => Self::Class,
            b"const" => Self::Const,
            b"constexpr" => Self::Constexpr,
            b"const_cast" => Self::ConstCast,
            b"continue" => Self::Continue,
            b"decltype" => Self::Decltype,
            b"default" => Self::Default,
            b"delete" => Self::Delete,
            b"do" => Self::Do,
            b"double" => Self::Double,
            b"dynamic_cast" => Self::DynamicCast,
            b"else" => Self::Else,
            b"enum" => Self::Enum,
            b"explicit" => Self::Explicit,
            b"export" => Self::Export,
            b"extern" => Self::Extern,
            b"false" => Self::False,
            b"float" => Self::Float,
            b"for" => Self::For,
            b"friend" => Self::Friend,
            b"goto" => Self::Goto,
            b"if" => Self::If,
            b"inline" => Self::Inline,
            b"int" => Self::Int,
            b"long" => Self::Long,
            b"mutable" => Self::Mutable,
            b"namespace" => Self::Namespace,
            b"new" => Self::New,
            b"noexcept" => Self::Noexcept,
            b"nullptr" => Self::Nullptr,
            b"operator" => Self::Operator,
            b"private" => Self::Private,
            b"protected" => Self::Protected,
            b"public" => Self::Public,
            b"register" => Self::Register,
            b"reinterpret_cast" => Self::ReinterpretCast,
            b"return" => Self::Return,
            b"short" => Self::Short,
            b"signed" => Self::Signed,
            b"sizeof" => Self::Sizeof,
            b"static" => Self::Static,
            b"static_assert" => Self::StaticAssert,
            b"static_cast" => Self::StaticCast,
            b"struct" => Self::Struct,
            b"switch" => Self::Switch,
            b"template" => Self::Template,
            b"this" => Self::This,
            b"thread_local" => Self::ThreadLocal,
            b"throw" => Self::Throw,
            b"true" => Self::True,
            b"try" => Self::Try,
            b"typedef" => Self::Typedef,
            b"typeid" => Self::Typeid,
            b"typename" => Self::Typename,
            b"union" => Self::Union,
            b"unsigned" => Self::Unsigned,
            b"using" => Self::Using,
            b"virtual" => Self::Virtual,
            b"void" => Self::Void,
            b"volatile" => Self::Volatile,
            b"wchar_t" => Self::WcharT,
            b"while" => Self::While,
            _ => return None,
        })
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Alignas => "KW_ALIGNAS",
            Self::Alignof => "KW_ALIGNOF",
            Self::Asm => "KW_ASM",
            Self::Auto => "KW_AUTO",
            Self::Bool => "KW_BOOL",
            Self::Break => "KW_BREAK",
            Self::Case => "KW_CASE",
            Self::Catch => "KW_CATCH",
            Self::Char => "KW_CHAR",
            Self::Char16T => "KW_CHAR16_T",
            Self::Char32T => "KW_CHAR32_T",
            Self::Class => "KW_CLASS",
            Self::Const => "KW_CONST",
            Self::Constexpr => "KW_CONSTEXPR",
            Self::ConstCast => "KW_CONST_CAST",
            Self::Continue => "KW_CONTINUE",
            Self::Decltype => "KW_DECLTYPE",
            Self::Default => "KW_DEFAULT",
            Self::Delete => "KW_DELETE",
            Self::Do => "KW_DO",
            Self::Double => "KW_DOUBLE",
            Self::DynamicCast => "KW_DYNAMIC_CAST",
            Self::Else => "KW_ELSE",
            Self::Enum => "KW_ENUM",
            Self::Explicit => "KW_EXPLICIT",
            Self::Export => "KW_EXPORT",
            Self::Extern => "KW_EXTERN",
            Self::False => "KW_FALSE",
            Self::Float => "KW_FLOAT",
            Self::For => "KW_FOR",
            Self::Friend => "KW_FRIEND",
            Self::Goto => "KW_GOTO",
            Self::If => "KW_IF",
            Self::Inline => "KW_INLINE",
            Self::Int => "KW_INT",
            Self::Long => "KW_LONG",
            Self::Mutable => "KW_MUTABLE",
            Self::Namespace => "KW_NAMESPACE",
            Self::New => "KW_NEW",
            Self::Noexcept => "KW_NOEXCEPT",
            Self::Nullptr => "KW_NULLPTR",
            Self::Operator => "KW_OPERATOR",
            Self::Private => "KW_PRIVATE",
            Self::Protected => "KW_PROTECTED",
            Self::Public => "KW_PUBLIC",
            Self::Register => "KW_REGISTER",
            Self::ReinterpretCast => "KW_REINTERPET_CAST",
            Self::Return => "KW_RETURN",
            Self::Short => "KW_SHORT",
            Self::Signed => "KW_SIGNED",
            Self::Sizeof => "KW_SIZEOF",
            Self::Static => "KW_STATIC",
            Self::StaticAssert => "KW_STATIC_ASSERT",
            Self::StaticCast => "KW_STATIC_CAST",
            Self::Struct => "KW_STRUCT",
            Self::Switch => "KW_SWITCH",
            Self::Template => "KW_TEMPLATE",
            Self::This => "KW_THIS",
            Self::ThreadLocal => "KW_THREAD_LOCAL",
            Self::Throw => "KW_THROW",
            Self::True => "KW_TRUE",
            Self::Try => "KW_TRY",
            Self::Typedef => "KW_TYPEDEF",
            Self::Typeid => "KW_TYPEID",
            Self::Typename => "KW_TYPENAME",
            Self::Union => "KW_UNION",
            Self::Unsigned => "KW_UNSIGNED",
            Self::Using => "KW_USING",
            Self::Virtual => "KW_VIRTUAL",
            Self::Void => "KW_VOID",
            Self::Volatile => "KW_VOLATILE",
            Self::WcharT => "KW_WCHAR_T",
            Self::While => "KW_WHILE",
        }
    }
}

/// Canonical simple identity: hashes are invalid and new/delete are keywords.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SimpleToken(SimpleIdentity);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SimpleIdentity {
    Keyword(Keyword),
    Punctuator(Punctuator),
}

impl SimpleToken {
    #[inline]
    pub fn identifier(text: &[u8]) -> Option<Self> {
        Keyword::find(text)
            .map(Self::keyword)
            .or_else(|| super::pp_lexer::identifier_operator(text).and_then(Self::punctuator))
    }

    #[inline]
    #[must_use]
    pub fn keyword(word: Keyword) -> Self {
        Self(SimpleIdentity::Keyword(word))
    }

    #[must_use]
    pub fn punctuator(punctuator: Punctuator) -> Option<Self> {
        Some(match punctuator {
            Punctuator::Hash | Punctuator::HashHash => return None,
            Punctuator::New => Self::keyword(Keyword::New),
            Punctuator::Delete => Self::keyword(Keyword::Delete),
            operator => Self(SimpleIdentity::Punctuator(operator)),
        })
    }

    #[must_use]
    #[inline]
    pub fn name(self) -> &'static str {
        match self.0 {
            SimpleIdentity::Keyword(word) => word.name(),
            SimpleIdentity::Punctuator(punctuator) => punctuator_name(punctuator),
        }
    }

    #[inline]
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self.0 {
            SimpleIdentity::Keyword(word) => word.spelling(),
            SimpleIdentity::Punctuator(operator) => operator.spelling(),
        }
    }

    #[inline]
    #[must_use]
    pub fn as_keyword(self) -> Option<Keyword> {
        match self.0 {
            SimpleIdentity::Keyword(word) => Some(word),
            SimpleIdentity::Punctuator(_) => None,
        }
    }

    #[inline]
    #[must_use]
    pub fn as_punctuator(self) -> Option<Punctuator> {
        match self.0 {
            SimpleIdentity::Punctuator(operator) => Some(operator),
            SimpleIdentity::Keyword(_) => None,
        }
    }
}

fn punctuator_name(punctuator: Punctuator) -> &'static str {
    match punctuator {
        Punctuator::LeftBrace => "OP_LBRACE",
        Punctuator::RightBrace => "OP_RBRACE",
        Punctuator::LeftBracket => "OP_LSQUARE",
        Punctuator::RightBracket => "OP_RSQUARE",
        Punctuator::LeftParen => "OP_LPAREN",
        Punctuator::RightParen => "OP_RPAREN",
        Punctuator::Semicolon => "OP_SEMICOLON",
        Punctuator::Question => "OP_QMARK",
        Punctuator::Comma => "OP_COMMA",
        Punctuator::Complement => "OP_COMPL",
        Punctuator::Scope => "OP_COLON2",
        Punctuator::Colon => "OP_COLON",
        Punctuator::Ellipsis => "OP_DOTS",
        Punctuator::DotStar => "OP_DOTSTAR",
        Punctuator::Dot => "OP_DOT",
        Punctuator::Increment => "OP_INC",
        Punctuator::AddAssign => "OP_PLUSASS",
        Punctuator::Add => "OP_PLUS",
        Punctuator::ArrowStar => "OP_ARROWSTAR",
        Punctuator::Decrement => "OP_DEC",
        Punctuator::SubtractAssign => "OP_MINUSASS",
        Punctuator::Arrow => "OP_ARROW",
        Punctuator::Subtract => "OP_MINUS",
        Punctuator::MultiplyAssign => "OP_STARASS",
        Punctuator::Multiply => "OP_STAR",
        Punctuator::DivideAssign => "OP_DIVASS",
        Punctuator::Divide => "OP_DIV",
        Punctuator::RemainderAssign => "OP_MODASS",
        Punctuator::Remainder => "OP_MOD",
        Punctuator::XorAssign => "OP_XORASS",
        Punctuator::Xor => "OP_XOR",
        Punctuator::LogicalAnd => "OP_LAND",
        Punctuator::AndAssign => "OP_BANDASS",
        Punctuator::BitAnd => "OP_AMP",
        Punctuator::LogicalOr => "OP_LOR",
        Punctuator::OrAssign => "OP_BORASS",
        Punctuator::BitOr => "OP_BOR",
        Punctuator::NotEqual => "OP_NE",
        Punctuator::LogicalNot => "OP_LNOT",
        Punctuator::Equal => "OP_EQ",
        Punctuator::Assign => "OP_ASS",
        Punctuator::ShiftLeftAssign => "OP_LSHIFTASS",
        Punctuator::ShiftLeft => "OP_LSHIFT",
        Punctuator::LessEqual => "OP_LE",
        Punctuator::Less => "OP_LT",
        Punctuator::ShiftRightAssign => "OP_RSHIFTASS",
        Punctuator::ShiftRight => "OP_RSHIFT",
        Punctuator::GreaterEqual => "OP_GE",
        Punctuator::Greater => "OP_GT",
        Punctuator::New => "KW_NEW",
        Punctuator::Delete => "KW_DELETE",
        Punctuator::Hash | Punctuator::HashHash => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_simple_identity_excludes_hashes_and_unifies_keywords() {
        assert_eq!(
            SimpleToken::punctuator(Punctuator::New),
            Some(SimpleToken::keyword(Keyword::New))
        );
        assert_eq!(
            SimpleToken::punctuator(Punctuator::Delete),
            Some(SimpleToken::keyword(Keyword::Delete))
        );
        assert_eq!(SimpleToken::punctuator(Punctuator::Hash), None);
        assert_eq!(SimpleToken::punctuator(Punctuator::HashHash), None);
    }
}
