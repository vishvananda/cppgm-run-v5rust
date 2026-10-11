// Translated from preprocess/preprocessor{,_directives}.cpp (macro group). See NOTICE.
//! Translation-unit state and the typed preprocessing/post-token boundary.
use super::conditionals::Conditionals;
use super::error::PreprocessError;
use super::input::SourceInput;
use super::macros::{
    definition::{Definition, Definitions},
    expander::Expander,
    token::Token,
};
use crate::lex::{
    PostTokenInput,
    identifiers::{Identifiers, NameId},
    pp_lexer::{PpToken, PpTokenKind},
};
use crate::support::source::SourceBuffer;

pub struct Preprocessor<'a> {
    source: SourceInput<'a>,
    names: Identifiers,
    definitions: Definitions<'a>,
    expander: Expander<'a>,
    current: Token<'a>,
    conditionals: Conditionals,
    line_tokens: Vec<Token<'a>>,
}
impl<'a> Preprocessor<'a> {
    /// # Errors
    /// Returns lexical construction errors before token delivery begins.
    pub fn new(source: &'a SourceBuffer) -> Result<Self, PreprocessError> {
        Self::with_name(source, b"")
    }
    /// Construct a translation unit with its command-line presumed filename.
    /// # Errors
    /// Returns lexical construction errors before token delivery begins.
    pub fn with_name(source: &'a SourceBuffer, name: &[u8]) -> Result<Self, PreprocessError> {
        let mut names = Identifiers::default();
        let definitions = Definitions::new(&mut names)?;
        Ok(Self {
            source: SourceInput::with_name(source, name)?,
            names,
            definitions,
            expander: Expander::default(),
            current: Token::end(source.begin()),
            conditionals: Conditionals::default(),
            line_tokens: Vec::new(),
        })
    }
    fn read_macro_name(&mut self, directive: &Token<'a>) -> Result<NameId, PreprocessError> {
        let name = self.source.next_in_line(&mut self.names)?;
        if name.kind != PpTokenKind::Identifier {
            return Err(PreprocessError::at(
                directive.begin,
                "a macro name must be an identifier",
            ));
        }
        if matches!(
            name.spelling.bytes(),
            b"defined"
                | b"__cplusplus"
                | b"__DATE__"
                | b"__FILE__"
                | b"__LINE__"
                | b"__STDC_HOSTED__"
                | b"__TIME__"
        ) {
            return Err(PreprocessError::at(
                name.begin,
                "this name cannot be defined or undefined",
            ));
        }
        name.name
            .ok_or(PreprocessError::Invariant("macro name without identifier"))
    }
    fn run_directive(&mut self) -> Result<(), PreprocessError> {
        let directive = self.source.next_in_line(&mut self.names)?;
        if directive.kind == PpTokenKind::NewLine {
            self.source.end_directive();
            return Ok(());
        }
        if directive.kind != PpTokenKind::Identifier {
            return Err(PreprocessError::at(
                directive.begin,
                "invalid preprocessing directive",
            ));
        }
        match directive.spelling.bytes() {
            b"define" => {
                let name = self.read_macro_name(&directive)?;
                let definition = Definition::parse(&mut self.source, &mut self.names, name)?;
                self.definitions.define(definition, directive.begin)?;
            }
            b"undef" => {
                let name = self.read_macro_name(&directive)?;
                if self.source.next_in_line(&mut self.names)?.kind != PpTokenKind::NewLine {
                    return Err(PreprocessError::at(
                        directive.begin,
                        "extra tokens at the end of the directive",
                    ));
                }
                self.definitions.undefine(name);
            }
            b"error" => return Err(PreprocessError::at(directive.begin, "#error")),
            b"if" | b"ifdef" | b"ifndef" | b"elif" | b"else" | b"endif" => {
                self.conditionals.run(
                    &directive,
                    &mut self.source,
                    &mut self.names,
                    &self.definitions,
                    &mut self.expander,
                )?;
            }
            b"include" => {
                return Err(PreprocessError::NotImplemented("file inclusion"));
            }
            b"line" => super::line_control::run_line(
                &directive,
                &mut self.source,
                &mut self.names,
                &self.definitions,
                &mut self.expander,
                &mut self.line_tokens,
            )?,
            b"pragma" => return Err(PreprocessError::NotImplemented("pragma directives")),
            _ => {
                return Err(PreprocessError::at(
                    directive.begin,
                    "invalid preprocessing directive",
                ));
            }
        }
        self.source.end_directive();
        Ok(())
    }
    pub(crate) fn describe(&self, error: &PreprocessError) -> Result<Vec<u8>, PreprocessError> {
        match error {
            PreprocessError::Source(error) => self.source.describe(error),
            PreprocessError::Directive { location, message } => {
                self.source
                    .describe(&crate::support::source::SourceError::new(
                        *location, message,
                    ))
            }
            PreprocessError::Limit(message)
            | PreprocessError::Invariant(message)
            | PreprocessError::NotImplemented(message) => Ok(message.as_bytes().to_vec()),
        }
    }
}
impl PostTokenInput for Preprocessor<'_> {
    type Error = PreprocessError;
    fn next_non_whitespace_kind(&mut self) -> Result<PpTokenKind, PreprocessError> {
        loop {
            let token = self
                .expander
                .next(&mut self.source, &mut self.names, &self.definitions)?;
            if token.kind == PpTokenKind::EndOfFile && self.source.at_directive() {
                self.run_directive()?;
                continue;
            }
            if token.kind == PpTokenKind::Identifier && token.spelling.bytes() == b"_Pragma" {
                return Err(PreprocessError::NotImplemented("pragma operator"));
            }
            if token.kind == PpTokenKind::EndOfFile {
                self.conditionals.check_end()?;
            }
            let kind = token.kind;
            self.current = token;
            return Ok(kind);
        }
    }
    fn token(&self, _kind: PpTokenKind) -> PpToken<'_> {
        self.current.view()
    }
    fn intern_identifier(&mut self, text: &[u8]) -> Result<Option<NameId>, PreprocessError> {
        self.names
            .intern(text)
            .map(Some)
            .map_err(PreprocessError::Limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::pp_lexer::Punctuator;

    #[test]
    fn line_operand_prescan_uses_current_location_and_releases_transient_tokens() {
        let source = SourceBuffer::new(b"#define ID(x) x\n#define LINE __LINE__\n#line 80 \"virtual\"\nID(\nLINE\n)\n#line __LINE__\n__LINE__\n".to_vec()).unwrap();
        let mut input = Preprocessor::with_name(&source, b"physical").unwrap();
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"81");
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"83");
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
        assert!(input.line_tokens.is_empty());
        assert_eq!(input.expander.live_storage(), [0; 5]);
    }

    #[test]
    fn conditional_lines_leave_no_expansion_storage_and_only_active_definitions() {
        let source = SourceBuffer::new(
            br"#define ID(x) x
#define SELECT defined
#define present absent
#if ID(SELECT(present)) && __has_cpp_attribute(__no_unique_address__)
#define chosen 17
#elif ignored(__LINE__, __VA_ARGS__)
#error skipped
#else
#error also_skipped
#endif
#if __has_attribute(__always_inline__) && !__has_cpp_attribute(gnu::no_unique_address)
chosen
#endif
"
            .to_vec(),
        )
        .unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"17");
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
        assert_eq!(input.expander.live_storage(), [0; 5]);
    }

    #[test]
    fn inactive_text_does_not_intern_names_or_demand_builtins() {
        let mut identities = Vec::new();
        for body in [
            b"".as_slice(),
            b"ignored __VA_ARGS__ __LINE__ _Pragma(bad)\n",
        ] {
            let mut text = b"#if 0\n".to_vec();
            text.extend_from_slice(body);
            text.extend_from_slice(b"#endif\nvisible");
            let source = SourceBuffer::new(text).unwrap();
            let mut input = Preprocessor::new(&source).unwrap();
            let kind = input.next_non_whitespace_kind().unwrap();
            assert_eq!(input.token(kind).spelling, b"visible");
            identities.push(input.token(kind).name);
        }
        assert_eq!(identities[0], identities[1]);
    }

    #[test]
    fn skipped_sections_use_flat_depth_storage_and_check_nested_order() {
        let depth = 50_000;
        let mut text = b"#if 0\n".to_vec();
        text.extend_from_slice("#if invalid ignored\n".repeat(depth).as_bytes());
        text.extend_from_slice("#else ignored\n#endif ignored\n".repeat(depth).as_bytes());
        text.extend_from_slice(b"#else\n17\n#endif");
        let source = SourceBuffer::new(text).unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"17");
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
        for (text, expected) in [
            (
                "#if 0\n#if bad\n#else ignored\n#elif ignored\n#endif\n#endif",
                "4:2: #elif or #else after #else",
            ),
            (
                "#if 0\n#if bad\n#else ignored",
                "1:2: unterminated conditional directive",
            ),
            ("#if 1\n", "1:2: unterminated conditional directive"),
            ("#else", "1:2: #elif, #else or #endif without #if"),
            (
                "#if 0\n#else extra",
                "2:2: extra tokens at the end of the directive",
            ),
            (
                "#if defined(3) \"\\q\"",
                "1:5: `defined` requires an identifier",
            ),
        ] {
            let source = SourceBuffer::new(text.as_bytes().to_vec()).unwrap();
            let mut input = Preprocessor::new(&source).unwrap();
            let error = input.next_non_whitespace_kind().unwrap_err();
            assert_eq!(crate::driver::preproc::describe(&source, &error), expected);
        }
    }

    #[test]
    fn nested_argument_prescan_uses_ranges_and_releases_all_invocation_storage() {
        let depth = 4096;
        let mut text = b"#define ID(x) x\n".to_vec();
        for _ in 0..depth {
            text.extend_from_slice(b"ID(");
        }
        text.extend_from_slice(b"17");
        text.extend(std::iter::repeat_n(b')', depth));
        let source = SourceBuffer::new(text).unwrap();
        let mut preprocessor = Preprocessor::new(&source).unwrap();
        let kind = preprocessor.next_non_whitespace_kind().unwrap();
        assert_eq!(kind, PpTokenKind::PpNumber);
        assert_eq!(preprocessor.token(kind).spelling, b"17");
        assert_eq!(
            preprocessor.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
        assert_eq!(preprocessor.expander.live_storage(), [0; 5]);
    }

    #[test]
    fn post_tokens_keep_interned_identity_through_substitution_and_suffix_splitting() {
        use crate::lex::{PostTokenKind, PostTokenizer, number::NumberDialect};
        let source =
            SourceBuffer::new(b"#define REPEAT(x) x x\nREPEAT(foo) operator \"\"foo".to_vec())
                .unwrap();
        let input = Preprocessor::new(&source).unwrap();
        let mut tokens = PostTokenizer::from_input(input, NumberDialect::Cxx11);
        let first = tokens.next_token().unwrap().name.unwrap();
        assert_eq!(tokens.next_token().unwrap().name, Some(first));
        assert!(matches!(
            tokens.next_token().unwrap().kind,
            PostTokenKind::Simple(_)
        ));
        assert!(matches!(
            tokens.next_token().unwrap().kind,
            PostTokenKind::String { .. }
        ));
        assert_eq!(tokens.next_token().unwrap().name, Some(first));
    }

    #[test]
    fn argument_demand_controls_builtins_and_prescan_happens_once() {
        let source = SourceBuffer::new(
            b"#define DROP(x) 1\n#define STRING(x) #x\nDROP(__LINE__) STRING(__LINE__)".to_vec(),
        )
        .unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::PpNumber
        );
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"\"__LINE__\"");
        let source = SourceBuffer::new(
            b"#define STRING(x) #x\n#define FORWARD(x) STRING(x)\nFORWARD(__LINE__)".to_vec(),
        )
        .unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"\"3\"");
        let source =
            SourceBuffer::new(b"#define TWICE(x) x x\nTWICE(__COUNTER__) __COUNTER__".to_vec())
                .unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        for expected in [b"0", b"0", b"1"] {
            let kind = input.next_non_whitespace_kind().unwrap();
            assert_eq!(input.token(kind).spelling, expected);
        }
    }

    #[test]
    fn paste_retains_typed_punctuator_and_translated_source_spans() {
        let source =
            SourceBuffer::new(b"#define JOIN(a,b) a ## b\nJOIN(+,=) JOIN(na,\\\nme)".to_vec())
                .unwrap();
        let mut preprocessor = Preprocessor::new(&source).unwrap();
        let kind = preprocessor.next_non_whitespace_kind().unwrap();
        let token = preprocessor.token(kind);
        assert_eq!(token.punctuator, Some(Punctuator::AddAssign));
        assert_eq!((token.begin.index(), token.end.index()), (25, 29));
        let kind = preprocessor.next_non_whitespace_kind().unwrap();
        let token = preprocessor.token(kind);
        assert_eq!(kind, PpTokenKind::Identifier);
        assert_eq!(token.spelling, b"name");
        assert_eq!((token.begin.index(), token.end.index()), (35, 39));
    }
}
