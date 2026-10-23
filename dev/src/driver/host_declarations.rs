// Translated from driver/host_environment.cpp::kBuiltinDeclarations. See NOTICE.
//! The hosted grammar prefix contributes token provenance to the `LowIR` view.
//! The ABI record is parsed into the same tree. Deferred hosted template aliases
//! retain their reserved token span until their owning grammar group is ported.
pub(super) const VA_DECLARATIONS: &[u8] = concat!(
    "struct __va_list_tag { unsigned int gp_offset; unsigned int fp_offset; void* overflow_arg_area; ",
    "void* reg_save_area; };\n",
    "typedef __va_list_tag __builtin_va_list[1];\n",
).as_bytes();
const TEMPLATE_DECLARATIONS: &[u8] = concat!(
    "template<template<class _Tp, _Tp...> class _Sp, class _Tp, _Tp _Np> using __make_integer_seq = ",
    "_Sp<_Tp, __integer_pack(_Np)...>;\n",
    "template<unsigned long _Ip, class... _Tp> struct __cppgm_type_pack_element;\n",
    "template<class _Hp, class... _Tp> struct __cppgm_type_pack_element<0, _Hp, _Tp...> { typedef _Hp type; };\n",
    "template<unsigned long _Ip, class _Hp, class... _Tp> struct __cppgm_type_pack_element<_Ip, _Hp, _Tp...> ",
    ": __cppgm_type_pack_element<_Ip - 1, _Tp...> {};\n",
    "template<unsigned long _Ip, class... _Tp> using __type_pack_element = ",
    "typename __cppgm_type_pack_element<_Ip, _Tp...>::type;\n",
).as_bytes();

// The actual prefix starts after the reserved hosted-template span. Source
// relative labels start after both spans, as in the frozen full hosted prefix.
#[derive(Clone, Copy)]
pub(super) struct TokenOrigins {
    pub first: u64,
    pub source: u64,
}
pub(super) fn token_origins() -> Result<TokenOrigins, &'static str> {
    static TOKENS: std::sync::OnceLock<Result<TokenOrigins, &'static str>> =
        std::sync::OnceLock::new();
    *TOKENS.get_or_init(|| {
        let first = count_tokens(TEMPLATE_DECLARATIONS)?;
        let source = first
            .checked_add(count_tokens(VA_DECLARATIONS)?)
            .ok_or("builtin declaration token limit")?;
        Ok(TokenOrigins { first, source })
    })
}
fn count_tokens(text: &[u8]) -> Result<u64, &'static str> {
    use crate::{
        lex::{PostTokenKind, PostTokenizer, number::NumberDialect, pp_lexer::Punctuator},
        support::source::SourceBuffer,
    };
    let source = SourceBuffer::new(text.to_vec())?;
    let mut tokens = PostTokenizer::new(&source, NumberDialect::Cxx11)
        .map_err(|_| "invalid builtin declaration source")?;
    let mut count = 0u64;
    loop {
        let token = tokens
            .next_token()
            .map_err(|_| "invalid builtin declaration token")?;
        if token.kind == PostTokenKind::EndOfFile {
            return Ok(count);
        }
        let width = if matches!(token.kind, PostTokenKind::Simple(simple) if simple.as_punctuator() == Some(Punctuator::ShiftRight))
        {
            2
        } else {
            1
        };
        count = count
            .checked_add(width)
            .ok_or("builtin declaration token limit")?;
    }
}
