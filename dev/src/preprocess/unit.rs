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
    pragmas: super::pragmas::Pragmas,
    directives: u64,
}
impl<'a> Preprocessor<'a> {
    pub(crate) fn enter_builtin_declarations(
        &mut self,
        text: &[u8],
    ) -> Result<(), PreprocessError> {
        self.source.enter_builtin_declarations(text)
    }
    pub(crate) fn set_number_options(&mut self, options: crate::lex::number::NumberOptions) {
        self.source.set_binary_exponents(options.binary_exponents);
        self.expander.set_binary_exponents(options.binary_exponents);
        self.conditionals.set_number_dialect(options.dialect);
    }
    /// # Errors
    /// Returns lexical construction errors before token delivery begins.
    pub fn new(source: &'a SourceBuffer) -> Result<Self, PreprocessError> {
        Self::with_name(source, b"")
    }
    /// Construct an in-memory unit with a presumed filename and no host file identity.
    /// # Errors
    /// Returns lexical construction errors before token delivery begins.
    pub fn with_name(source: &'a SourceBuffer, name: &[u8]) -> Result<Self, PreprocessError> {
        Self::with_file_identity(source, name, |_| None)
    }
    /// Construct a unit with host identity queries and no include-file reader.
    /// # Errors
    /// Returns lexical construction errors before token delivery begins.
    pub fn with_file_identity(
        source: &'a SourceBuffer,
        name: &[u8],
        identity_query: super::files::FileIdentityQuery,
    ) -> Result<Self, PreprocessError> {
        Self::with_file_services(
            source,
            name,
            super::files::FileServices {
                identity: identity_query,
                read: |_| None,
            },
        )
    }
    /// Construct a unit with demand-only host file operations.
    /// # Errors
    /// Returns lexical construction errors before token delivery begins.
    pub fn with_file_services(
        source: &'a SourceBuffer,
        name: &[u8],
        services: super::files::FileServices,
    ) -> Result<Self, PreprocessError> {
        Self::with_options(
            source,
            name,
            services,
            &super::options::PredefinedValues::default(),
        )
    }
    /// Construct a unit with invocation-wide predefined values and host file services.
    /// # Errors
    /// Returns source construction or compact-storage errors before token delivery.
    pub fn with_options(
        source: &'a SourceBuffer,
        name: &[u8],
        services: super::files::FileServices,
        values: &super::options::PredefinedValues,
    ) -> Result<Self, PreprocessError> {
        Self::with_driver_options(
            source,
            name,
            services,
            values,
            &super::options::DriverOptions::default(),
        )
    }
    /// Construct a unit with explicit include search and command-line definitions.
    /// # Errors
    /// Returns source construction or compact-storage errors before token delivery.
    pub fn with_driver_options(
        source: &'a SourceBuffer,
        name: &[u8],
        services: super::files::FileServices,
        values: &super::options::PredefinedValues,
        options: &super::options::DriverOptions,
    ) -> Result<Self, PreprocessError> {
        let mut names = Identifiers::default();
        let definitions = Definitions::new(&mut names)?;
        let mut input = SourceInput::with_file_services(source, name, services)?;
        input.set_include_paths(options);
        let mut predefined = values.text();
        let command_line_begin = predefined.len();
        predefined.extend_from_slice(&options.command_line);
        input.enter_predefined(predefined, command_line_begin)?;
        Ok(Self {
            source: input,
            names,
            definitions,
            expander: Expander::default(),
            current: Token::end(source.begin()),
            conditionals: Conditionals::default(),
            line_tokens: Vec::new(),
            pragmas: super::pragmas::Pragmas::new(services.identity),
            directives: 0,
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
        if name.spelling.bytes() == b"defined"
            || (!self.source.in_predefined()
                && matches!(
                    name.spelling.bytes(),
                    b"__cplusplus"
                        | b"__DATE__"
                        | b"__FILE__"
                        | b"__LINE__"
                        | b"__STDC_HOSTED__"
                        | b"__TIME__"
                ))
        {
            return Err(PreprocessError::at(
                name.begin,
                "this name cannot be defined or undefined",
            ));
        }
        name.name
            .ok_or(PreprocessError::Invariant("macro name without identifier"))
    }
    fn run_directive(&mut self) -> Result<(), PreprocessError> {
        self.directives = self.directives.wrapping_add(1);
        let directive = self.source.next_in_line(&mut self.names)?;
        self.source
            .track_guard(directive.spelling.bytes(), self.conditionals.depth());
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
                let mut definition = Definition::parse(&mut self.source, &mut self.names, name)?;
                definition.replaceable = self.source.command_line_definition(directive.begin);
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
                let (name, angle) = super::inclusion::read_name(
                    &directive,
                    &mut self.source,
                    &mut self.names,
                    &self.definitions,
                    &mut self.expander,
                    &mut self.line_tokens,
                )?;
                self.source.include(
                    &directive,
                    &name,
                    angle,
                    self.conditionals.depth(),
                    &self.definitions,
                    &self.pragmas,
                )?;
            }
            b"line" => super::line_control::run_line(
                &directive,
                &mut self.source,
                &mut self.names,
                &self.definitions,
                &mut self.expander,
                &mut self.line_tokens,
            )?,
            b"pragma" => self
                .pragmas
                .run_directive(&mut self.source, &mut self.names)?,
            _ => {
                return Err(PreprocessError::at(
                    directive.begin,
                    "invalid preprocessing directive",
                ));
            }
        }
        self.source.set_open_conditional(
            self.conditionals
                .open_since(self.source.conditional_depth()),
        );
        self.source.end_directive();
        Ok(())
    }
    /// Maximum member alignment currently selected by pragma pack; zero means default.
    #[must_use]
    pub fn pack_alignment(&self) -> u32 {
        self.pragmas.pack_alignment()
    }
    pub(crate) fn name_spelling(&self, name: NameId) -> &[u8] {
        self.names.bytes(name)
    }
    pub(crate) fn identifiers_mut(&mut self) -> &mut Identifiers {
        &mut self.names
    }
    pub(crate) fn intern_name(&mut self, spelling: &[u8]) -> Result<NameId, PreprocessError> {
        self.names.intern(spelling).map_err(PreprocessError::Limit)
    }
    /// Snapshot all work demanded so far, including a failed operation's work.
    #[must_use]
    pub fn counters(&self) -> super::counters::PreprocessorCounters {
        super::counters::PreprocessorCounters {
            directives: self.directives,
            skipped_directives: self.conditionals.skipped_directives,
            identifiers: self.names.len(),
            expansion: self.expander.counters(),
            ..self.source.counters()
        }
    }
    pub(crate) fn describe(&self, error: &PreprocessError) -> Result<Vec<u8>, PreprocessError> {
        match error {
            PreprocessError::Source(error) => self.source.describe(error),
            PreprocessError::Directive { location, message } => {
                self.source.describe_at(*location, message)
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
                self.pragmas.run_operator(
                    &token,
                    &mut self.source,
                    &mut self.names,
                    &self.definitions,
                    &mut self.expander,
                )?;
                continue;
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
    fn compiler_numeric_dialect_reaches_conditionals_and_pasted_tokens() {
        use crate::lex::{PostTokenKind, PostTokenizer, number::NumberDialect};
        let source = SourceBuffer::new(
            b"#define join(a,b) a##b\n#if 0b1 == 1\njoin(0x1.p+,2_x)\n#endif\n".to_vec(),
        )
        .unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        input.set_number_options(crate::lex::number::NumberOptions {
            dialect: NumberDialect::Gnu,
            binary_exponents: true,
        });
        let mut tokens = PostTokenizer::from_input(input, NumberDialect::Gnu);
        let token = tokens.next_token().unwrap();
        assert_eq!(token.source, b"0x1.p+2_x");
        assert!(matches!(
            token.kind,
            PostTokenKind::UserDefinedNumber {
                suffix_begin: 7,
                ..
            }
        ));
        assert_eq!(tokens.next_token().unwrap().kind, PostTokenKind::EndOfFile);
    }

    #[test]
    fn predefined_values_are_owned_and_parsed_on_demand_in_each_unit() {
        let source = SourceBuffer::new(b"__CPPGM_AUTHOR__ __DATE__ __TIME__\n".to_vec()).unwrap();
        let values = super::super::options::PredefinedValues {
            author: b"a\"\\b".to_vec(),
            date: b"Jan  2 2003".to_vec(),
            time: b"04:05:06".to_vec(),
        };
        let mut input = Preprocessor::with_options(
            &source,
            b"primary",
            super::super::files::FileServices {
                identity: |_| None,
                read: |_| None,
            },
            &values,
        )
        .unwrap();
        drop(values);
        assert_eq!(input.counters().source_tokens, 0);
        for expected in [
            b"\"a\\\"\\\\b\"".as_slice(),
            b"\"Jan  2 2003\"",
            b"\"04:05:06\"",
        ] {
            let kind = input.next_non_whitespace_kind().unwrap();
            assert_eq!(input.token(kind).spelling, expected);
        }
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
        let counters = input.counters();
        assert_eq!(counters.directives, 8);
        assert_eq!(counters.source_tokens, 27);
        assert_eq!(counters.files, 2);
        assert_eq!(counters.identifiers, 31);
        assert_eq!(counters.expansion.invocations, 3);
        assert_eq!(counters.expansion.produced_tokens, 3);
        assert_eq!(input.expander.live_storage(), [0; 5]);
    }

    #[test]
    fn pragma_execution_waits_for_text_demand_and_follows_argument_prescan() {
        let source = SourceBuffer::new(
            br#"#define TWICE(x) x x
#define MAKE(x) _Pragma(#x)
before TWICE(MAKE(pack(push,1)) __COUNTER__) after
#pragma pack(pop)
middle
_Pragma(R"(pack(pop))") end
"#
            .to_vec(),
        )
        .unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        for (expected, alignment) in [
            (b"before".as_slice(), 0),
            (b"0", 1),
            (b"0", 1),
            (b"after", 1),
            (b"middle", 1),
            (b"end", 0),
        ] {
            let kind = input.next_non_whitespace_kind().unwrap();
            assert_eq!(input.token(kind).spelling, expected);
            assert_eq!(input.pack_alignment(), alignment);
        }
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
        assert_eq!(input.expander.live_storage(), [0; 5]);
    }

    #[test]
    fn wrapped_parameter_roles_report_a_limit_without_indexing_absent_arguments() {
        let parameters = (0..u16::MAX)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let source =
            SourceBuffer::new(format!("#define F({parameters},...) p0\nF()\n").into_bytes())
                .unwrap();
        let mut input = Preprocessor::new(&source).unwrap();
        assert!(matches!(
            input.next_non_whitespace_kind(),
            Err(PreprocessError::Limit(
                "macro parameter count exceeds its representation"
            ))
        ));
    }

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
        let mut tokens = PostTokenizer::from_input(input, NumberDialect::Gnu);
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

#[cfg(test)]
mod inclusion_tests {
    use super::*;
    use crate::preprocess::files::{FileIdentity, FileServices};

    fn services() -> FileServices {
        FileServices {
            identity: |name| match name {
                b"primary" => Some(FileIdentity::new(1, 1)),
                b"header" | b"alias" => Some(FileIdentity::new(1, 2)),
                _ => None,
            },
            read: |_| {
                Some(b"#ifndef GUARD\n#define GUARD\n#define ID(x) x\n#define SAVED 9\n__FILE__ __LINE__\nID\n#endif\n".to_vec())
            },
        }
    }
    #[test]
    fn header_definitions_survive_return_and_guards_skip_aliases_until_undefined() {
        let source = SourceBuffer::new(b"#include \"header\"\n(SAVED)\n#include \"alias\"\n__FILE__ __LINE__\n#undef GUARD\n#include \"header\"\n(7)\n".to_vec()).unwrap();
        let mut input = Preprocessor::with_file_services(&source, b"primary", services()).unwrap();
        let mut result = Vec::new();
        loop {
            let kind = input.next_non_whitespace_kind().unwrap();
            if kind == PpTokenKind::EndOfFile {
                break;
            }
            result.push(input.token(kind).spelling.to_vec());
        }
        assert_eq!(
            result,
            [
                b"\"header\"".as_slice(),
                b"5",
                b"ID",
                b"(",
                b"9",
                b")",
                b"\"primary\"",
                b"4",
                b"\"header\"",
                b"5",
                b"ID",
                b"(",
                b"7",
                b")"
            ]
        );
        assert_eq!(input.source.inclusion_count(), 4);
        assert_eq!(input.expander.live_storage(), [0; 5]);
    }
    #[test]
    fn macro_invocations_run_through_header_eof_without_a_directive_boundary() {
        let source = SourceBuffer::new(b"#include \"header\"\n(7)\n".to_vec()).unwrap();
        let mut services = services();
        services.read = |_| Some(b"#define F(x) x\nF\n".to_vec());
        let mut input = Preprocessor::with_file_services(&source, b"primary", services).unwrap();
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"7");
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
        assert_eq!(input.expander.live_storage(), [0; 5]);
    }
    #[test]
    fn builtin_locations_remain_with_a_header_after_lookahead_returns_to_parent() {
        let source = SourceBuffer::new(b"#include \"header\"\n".to_vec()).unwrap();
        let mut services = services();
        services.read = |_| Some(b"#define F(x) x\nF(__FILE__ __LINE__)\n".to_vec());
        let mut input = Preprocessor::with_file_services(&source, b"primary", services).unwrap();
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"\"header\"");
        let kind = input.next_non_whitespace_kind().unwrap();
        assert_eq!(input.token(kind).spelling, b"2");
        assert_eq!(
            input.next_non_whitespace_kind().unwrap(),
            PpTokenKind::EndOfFile
        );
    }
    #[test]
    fn included_files_cannot_close_a_parent_conditional() {
        for directive in [b"#endif\n".as_slice(), b"#else\n", b"#elif 1\n"] {
            let source =
                SourceBuffer::new(b"#if 1\n#include \"header\"\n#endif\n".to_vec()).unwrap();
            let mut services = services();
            services.read = match directive {
                b"#endif\n" => |_| Some(b"#endif\n".to_vec()),
                b"#else\n" => |_| Some(b"#else\n".to_vec()),
                _ => |_| Some(b"#elif 1\n".to_vec()),
            };
            let mut input =
                Preprocessor::with_file_services(&source, b"primary", services).unwrap();
            let error = input.next_non_whitespace_kind().unwrap_err();
            assert_eq!(
                input.describe(&error).unwrap(),
                b"header:1:2: #elif, #else or #endif without #if\nnote: included from primary:2"
            );
        }
    }
}
