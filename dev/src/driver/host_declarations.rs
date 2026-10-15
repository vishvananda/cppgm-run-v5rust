// Translated from driver/host_environment.cpp::kBuiltinDeclarations. See NOTICE.
//! The hosted grammar prefix contributes token provenance to the `LowIR` view.
//! Its type/template semantics belong to the later hosted compilation stages.
use crate::{
    lex::{PostTokenKind, PostTokenizer, number::NumberDialect, pp_lexer::Punctuator},
    support::source::SourceBuffer,
};
const DECLARATIONS: &[u8] = concat!(
    "struct __va_list_tag { unsigned int gp_offset; unsigned int fp_offset; void* overflow_arg_area; ",
    "void* reg_save_area; };\n",
    "typedef __va_list_tag __builtin_va_list[1];\n",
    "template<template<class _Tp, _Tp...> class _Sp, class _Tp, _Tp _Np> using __make_integer_seq = ",
    "_Sp<_Tp, __integer_pack(_Np)...>;\n",
    "template<unsigned long _Ip, class... _Tp> struct __cppgm_type_pack_element;\n",
    "template<class _Hp, class... _Tp> struct __cppgm_type_pack_element<0, _Hp, _Tp...> { typedef _Hp type; };\n",
    "template<unsigned long _Ip, class _Hp, class... _Tp> struct __cppgm_type_pack_element<_Ip, _Hp, _Tp...> ",
    ": __cppgm_type_pack_element<_Ip - 1, _Tp...> {};\n",
    "template<unsigned long _Ip, class... _Tp> using __type_pack_element = ",
    "typename __cppgm_type_pack_element<_Ip, _Tp...>::type;\n",
).as_bytes();

pub(super) fn token_origin() -> Result<u64, &'static str> {
    static TOKENS: std::sync::OnceLock<Result<u64, &'static str>> = std::sync::OnceLock::new();
    *TOKENS.get_or_init(count_tokens)
}
fn count_tokens() -> Result<u64, &'static str> {
    let source = SourceBuffer::new(DECLARATIONS.to_vec())?;
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
        // The parser's token stream splits a >> into its two recorded halves.
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
