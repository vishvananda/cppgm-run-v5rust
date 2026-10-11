// Translated from preprocessor_directives.cpp::run_line. See NOTICE.
//! Macro-expanded line operands, decoded through the shared phase-5 converter.
use super::{
    error::PreprocessError,
    input::SourceInput,
    macros::{definition::Definitions, expander::Expander, token::Token},
};
use crate::lex::{
    identifiers::Identifiers,
    pp_lexer::PpTokenKind,
    strings::StringSequence,
    text::{EncodingPrefix, split_text},
};

pub(crate) fn run_line<'a>(
    directive: &Token<'a>,
    source: &mut SourceInput<'a>,
    names: &mut Identifiers,
    definitions: &Definitions<'a>,
    expander: &mut Expander<'a>,
    tokens: &mut Vec<Token<'a>>,
) -> Result<(), PreprocessError> {
    source.begin_line_input();
    tokens.clear();
    loop {
        let token = expander.next(source, names, definitions)?;
        if token.kind == PpTokenKind::EndOfFile {
            break;
        }
        tokens.push(token);
    }
    source.end_line_input()?;
    let fail = |message| PreprocessError::at(directive.begin, message);
    if tokens.is_empty() || tokens.len() > 2 || tokens[0].kind != PpTokenKind::PpNumber {
        return Err(fail("#line requires a line number"));
    }
    let mut number = 0_u32;
    for &byte in tokens[0].spelling.bytes() {
        if !byte.is_ascii_digit() {
            return Err(fail("#line requires a digit sequence"));
        }
        // The previous iteration is bounded by 2^31-1, so this fits in u64.
        let next = u64::from(number) * 10 + u64::from(byte - b'0');
        if next > 2_147_483_647 {
            return Err(fail("#line number out of range"));
        }
        number = u32::try_from(next).map_err(|_| fail("#line number out of range"))?;
    }
    if number == 0 {
        return Err(fail("#line number out of range"));
    }
    let name = if tokens.len() == 2 {
        Some(
            decode_ordinary_string(&tokens[1])?
                .ok_or_else(|| fail("#line requires an ordinary string literal"))?,
        )
    } else {
        None
    };
    source.set_line(number, name)?;
    tokens.clear();
    Ok(())
}

pub(crate) fn decode_ordinary_string(
    token: &Token<'_>,
) -> Result<Option<Vec<u8>>, PreprocessError> {
    if token.kind != PpTokenKind::StringLiteral
        || !split_text(token.spelling.bytes())
            .is_some_and(|parts| parts.prefix == EncodingPrefix::None && !parts.raw)
    {
        return Ok(None);
    }
    let mut sequence = StringSequence::default();
    sequence
        .add(token.spelling.bytes())
        .map_err(|_| PreprocessError::Limit("text literal implementation limit"))?;
    if sequence
        .concatenate()
        .map_err(|_| PreprocessError::Limit("text literal implementation limit"))?
        .is_none()
    {
        return Ok(None);
    }
    let units = sequence.data();
    Ok(Some(units[..units.len() - 1].to_vec()))
}
