// Translated from preprocessor.cpp and preprocessor_directives.cpp. See NOTICE.
//! Pragma execution after macro replacement, with unit-owned packing and once state.
use super::{
    error::PreprocessError,
    files::{FileIdentity, FileIdentityQuery},
    input::SourceInput,
    macros::{definition::Definitions, expander::Expander, token::Token},
};
use crate::{
    lex::{
        identifiers::Identifiers,
        pp_lexer::{PpLexer, PpLexerOptions, PpToken, PpTokenKind, Punctuator},
        text::split_text,
    },
    support::source::SourceBuffer,
};
use std::collections::HashSet;

pub(crate) struct Pragmas {
    pack: u32,
    pack_stack: Vec<u32>,
    once_files: HashSet<FileIdentity>,
    identity_query: FileIdentityQuery,
}
impl Pragmas {
    pub(crate) fn new(identity_query: FileIdentityQuery) -> Self {
        Self {
            pack: 0,
            pack_stack: Vec::new(),
            once_files: HashSet::new(),
            identity_query,
        }
    }
    pub(crate) fn pack_alignment(&self) -> u32 {
        self.pack
    }
    fn mark_once(&mut self, name: &[u8]) {
        if let Some(identity) = (self.identity_query)(name) {
            self.once_files.insert(identity);
        }
    }
    pub(crate) fn run_directive(
        &mut self,
        source: &mut SourceInput<'_>,
        names: &mut Identifiers,
    ) -> Result<(), PreprocessError> {
        let first = source.next_in_line(names)?;
        if first.kind == PpTokenKind::NewLine {
            return Ok(());
        }
        if first.kind == PpTokenKind::Identifier {
            match first.spelling.bytes() {
                b"once" => self.mark_once(source.presumed_name()),
                b"pack" => {
                    let mut operands = PackOperands::default();
                    loop {
                        let token = source.next_in_line(names)?;
                        if matches!(token.kind, PpTokenKind::NewLine | PpTokenKind::EndOfFile) {
                            break;
                        }
                        operands.observe(&token.view());
                    }
                    self.apply_pack(operands);
                    return Ok(());
                }
                _ => {}
            }
        }
        source.skip_line()
    }
    pub(crate) fn run_operator<'a>(
        &mut self,
        pragma: &Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
        expander: &mut Expander<'a>,
    ) -> Result<(), PreprocessError> {
        let invalid = || {
            PreprocessError::at(
                pragma.begin,
                "_Pragma requires a parenthesized string literal",
            )
        };
        if !expander
            .next(source, names, definitions)?
            .is_punctuator(Punctuator::LeftParen)
        {
            return Err(invalid());
        }
        let literal = expander.next(source, names, definitions)?;
        if literal.kind != PpTokenKind::StringLiteral {
            return Err(invalid());
        }
        let parts = split_text(literal.spelling.bytes()).ok_or_else(invalid)?;
        if !expander
            .next(source, names, definitions)?
            .is_punctuator(Punctuator::RightParen)
        {
            return Err(invalid());
        }
        let begin = usize::try_from(parts.body_begin)
            .map_err(|_| PreprocessError::Limit("pragma text exceeds host size"))?;
        let end = usize::try_from(parts.body_end)
            .map_err(|_| PreprocessError::Limit("pragma text exceeds host size"))?;
        let body = &literal.spelling.bytes()[begin..end];
        let mut text = Vec::with_capacity(body.len());
        let mut cursor = body.iter().copied().peekable();
        while let Some(byte) = cursor.next() {
            if !parts.raw && byte == b'\\' && matches!(cursor.peek(), Some(b'"' | b'\\')) {
                // Only these two escapes are destringized; phase 5 decoding is not used.
                if let Some(escaped) = cursor.next() {
                    text.push(escaped);
                }
            } else {
                text.push(byte);
            }
        }
        self.run_text(text, source.presumed_name())
    }
    fn run_text(&mut self, text: Vec<u8>, name: &[u8]) -> Result<(), PreprocessError> {
        let buffer = SourceBuffer::for_token_text(text).map_err(PreprocessError::Limit)?;
        // Construction errors precede the C++ handler for unrecognizable pragma text.
        let mut lexer = PpLexer::with_options(
            &buffer,
            PpLexerOptions {
                header_names_on_request: true,
                token_text: true,
                ..PpLexerOptions::default()
            },
        )?;
        let first = loop {
            let Ok(token) = lexer.next_token() else {
                return Ok(());
            };
            if token.kind == PpTokenKind::WhitespaceSequence {
                continue;
            }
            break token;
        };
        if first.kind != PpTokenKind::Identifier {
            return Ok(());
        }
        match first.spelling {
            b"once" => self.mark_once(name),
            b"pack" => {
                let mut operands = PackOperands::default();
                loop {
                    let Ok(token) = lexer.next_token() else {
                        return Ok(());
                    };
                    if token.kind == PpTokenKind::EndOfFile {
                        break;
                    }
                    operands.observe(&token);
                }
                self.apply_pack(operands);
            }
            _ => {}
        }
        Ok(())
    }
    fn apply_pack(&mut self, operands: PackOperands) {
        if operands.show {
            return;
        }
        if operands.push {
            self.pack_stack.push(self.pack);
        }
        if operands.pop {
            if let Some(pack) = self.pack_stack.pop() {
                self.pack = pack;
            }
            if operands.value.is_none() {
                return;
            }
        }
        if let Some(value) = operands.value {
            if value != 0 && !value.is_power_of_two() {
                return;
            }
            self.pack = value;
        } else if !operands.push && !operands.pop {
            self.pack = 0;
        }
    }
}

/// Operand facts stay local until the entire pragma has lexed successfully.
/// C++ accepts loose syntax, the last pp-number's decimal prefix, and uint32 wrap.
#[derive(Clone, Copy, Default)]
struct PackOperands {
    push: bool,
    pop: bool,
    show: bool,
    value: Option<u32>,
}
impl PackOperands {
    fn observe(&mut self, token: &PpToken<'_>) {
        if self.show {
            return;
        }
        match token.kind {
            PpTokenKind::Identifier => match token.spelling {
                b"push" => self.push = true,
                b"pop" => self.pop = true,
                b"show" => self.show = true,
                _ => {}
            },
            PpTokenKind::PpNumber => {
                let mut value = 0u32;
                for &digit in token.spelling.iter().take_while(|c| c.is_ascii_digit()) {
                    value = value.wrapping_mul(10).wrapping_add(u32::from(digit - b'0'));
                }
                self.value = Some(value);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_preserves_push_pop_order_wrapping_and_failed_lex_transactions() {
        let mut pragmas = Pragmas::new(|_| None);
        for (text, alignment, stack) in [
            ("pack(8)", 8, vec![]),
            ("pack(push,3)", 8, vec![8]),
            ("pack(push,1)", 1, vec![8, 8]),
            ("pack(pop,show,16)", 1, vec![8, 8]),
            ("pack(pop,3)", 8, vec![8]),
            ("pack(push,pop,2)", 2, vec![8]),
            ("pack(4294967297suffix)", 1, vec![8]),
            ("pack(0x10)", 0, vec![8]),
            ("pack(pop)", 8, vec![]),
            ("pack(pop)", 8, vec![]),
            ("pack(push,1) \"unterminated", 8, vec![]),
            ("pack(push,show,1) \"unterminated", 8, vec![]),
            ("pack(push,4)", 4, vec![8]),
            ("pack(.5)", 0, vec![8]),
            ("pack(3,16)", 16, vec![8]),
            ("pack()", 0, vec![8]),
            ("pack(pop)", 8, vec![]),
            ("\npack(1)", 8, vec![]),
            ("/*comment*/pack(2)", 2, vec![]),
        ] {
            pragmas.run_text(text.as_bytes().to_vec(), b"").unwrap();
            assert_eq!(pragmas.pack_alignment(), alignment, "{text}");
            assert_eq!(pragmas.pack_stack, stack, "{text}");
        }
    }

    #[test]
    fn once_uses_latest_presumed_name_regular_file_identity_and_unit_lifetime() {
        let name = "physical";
        let query: FileIdentityQuery = |name| match name {
            b"physical" | b"alias" => Some(FileIdentity::new(1, 2)),
            _ => None,
        };
        let mut pragmas = Pragmas::new(query);
        pragmas.mark_once(name.as_bytes());
        pragmas.mark_once(b"alias");
        assert_eq!(pragmas.once_files.len(), 1);
        pragmas.mark_once(b"");
        assert_eq!(pragmas.once_files.len(), 1);
        assert!(Pragmas::new(query).once_files.is_empty());

        let source =
            SourceBuffer::new(format!("#line 7 \"{name}\"\n#pragma once\n").into_bytes()).unwrap();
        let mut input = SourceInput::with_name(&source, b"absent").unwrap();
        let mut names = Identifiers::default();
        let definitions = Definitions::new(&mut names).unwrap();
        let mut expander = Expander::default();
        // Execute line control through its typed owner before registering once.
        input.next(&mut names).unwrap();
        let directive = input.next_in_line(&mut names).unwrap();
        super::super::line_control::run_line(
            &directive,
            &mut input,
            &mut names,
            &definitions,
            &mut expander,
            &mut Vec::new(),
        )
        .unwrap();
        input.end_directive();
        input.next(&mut names).unwrap();
        input.next_in_line(&mut names).unwrap(); // pragma
        let mut fresh = Pragmas::new(query);
        fresh.run_directive(&mut input, &mut names).unwrap();
        assert_eq!(fresh.once_files.len(), 1);
    }
}
