// Translated from preprocessor_directives.cpp::run_include/read_replaced_include_name. See NOTICE.
//! Direct header names bypass expansion; other operands use the existing line rescanner.
use super::{
    error::PreprocessError,
    input::SourceInput,
    line_control::decode_ordinary_string,
    macros::{
        definition::Definitions,
        expander::Expander,
        token::{LEADING_SPACE, Token},
    },
};
use crate::lex::{
    identifiers::Identifiers,
    pp_lexer::{PpTokenKind, Punctuator},
};

pub(crate) fn read_name<'a>(
    directive: &Token<'a>,
    source: &mut SourceInput<'a>,
    names: &mut Identifiers,
    definitions: &Definitions<'a>,
    expander: &mut Expander<'a>,
    tokens: &mut Vec<Token<'a>>,
) -> Result<(Vec<u8>, bool), PreprocessError> {
    source.request_header_name();
    let first = source.next_in_line(names)?;
    let fail = |message| PreprocessError::at(directive.begin, message);
    if first.kind == PpTokenKind::HeaderName {
        let text = first.spelling.bytes();
        if source.next_in_line(names)?.kind != PpTokenKind::NewLine {
            return Err(fail("extra tokens at the end of the directive"));
        }
        return Ok((text[1..text.len() - 1].to_vec(), text[0] == b'<'));
    }
    if first.kind == PpTokenKind::NewLine {
        return Err(fail("#include requires a file name"));
    }
    source.begin_line_with(first);
    tokens.clear();
    loop {
        let token = expander.next(source, names, definitions)?;
        if token.kind == PpTokenKind::EndOfFile {
            break;
        }
        tokens.push(token);
    }
    source.end_line_input()?;
    let angle = tokens.len() != 1;
    let result = if tokens.len() == 1 {
        decode_ordinary_string(&tokens[0])?
    } else if tokens
        .first()
        .is_some_and(|t| t.is_punctuator(Punctuator::Less))
        && tokens
            .last()
            .is_some_and(|t| t.is_punctuator(Punctuator::Greater))
    {
        let mut name = Vec::new();
        for (i, token) in tokens[1..tokens.len() - 1].iter().enumerate() {
            if i != 0 && token.flags & LEADING_SPACE != 0 {
                name.push(b' ');
            }
            name.extend_from_slice(token.spelling.bytes());
        }
        Some(name)
    } else {
        None
    };
    tokens.clear();
    result
        .map(|name| (name, angle))
        .ok_or_else(|| fail("#include requires a header name or string literal"))
}
