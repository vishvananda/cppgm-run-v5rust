// Translated from preprocess/macro/macro_expander.cpp. See NOTICE.
//! Pull rescanning with explicit expansion/argument stacks and invocation-scoped storage.
use super::definition::{Builtin, Definition, Definitions, Replacement, Role};
use super::hide_sets::{HideSetId, HideSets};
use super::token::{
    ARGUMENT, ContextId, FOLLOWS_IN_CONTEXT, FROM_ARGUMENT, LEADING_SPACE, PAINTED,
    REPLACED_ARGUMENT, START_OF_LINE, Spelling, Token,
};
use crate::lex::identifiers::Identifiers;
use crate::lex::pp_lexer::{PpLexer, PpLexerOptions, PpTokenKind, Punctuator};
use crate::preprocess::{error::PreprocessError, input::SourceInput};
use crate::support::source::SourceBuffer;
use std::ops::Range;
use std::rc::Rc;

struct Context<'a> {
    range: Range<usize>,
    cursor: usize,
    serial: ContextId,
    definition: Option<Rc<Definition<'a>>>,
    chain: HideSetId,
    disabled: HideSetId,
    inherited: HideSetId,
    outer: HideSetId,
}
#[derive(Clone, Copy, Default)]
struct Nesting {
    parent: ContextId,
    chain: HideSetId,
}
struct Arguments {
    raw: Vec<Range<usize>>,
    expanded: Vec<Range<usize>>,
    token_mark: usize,
    expanded_mark: usize,
    disabled: HideSetId,
    outer: HideSetId,
}
impl Arguments {
    fn range(&self, slot: u16, expanded: bool) -> Result<Range<usize>, PreprocessError> {
        let ranges = if expanded { &self.expanded } else { &self.raw };
        ranges
            .get(usize::from(slot))
            .cloned()
            .ok_or(PreprocessError::Limit(
                "macro parameter count exceeds its representation",
            ))
    }
}
struct Invocation<'a> {
    definition: Rc<Definition<'a>>,
    head: Token<'a>,
    nesting: Nesting,
    disabled: HideSetId,
    inherited: HideSetId,
    arguments: Arguments,
    argument: usize,
    pending_space: bool,
}
struct Building<'a> {
    head: Token<'a>,
    serial: ContextId,
    chain: HideSetId,
}

#[derive(Default)]
pub(crate) struct Expander<'a> {
    hides: HideSets,
    contexts: Vec<Context<'a>>,
    context_tokens: Vec<Token<'a>>,
    argument_tokens: Vec<Token<'a>>,
    argument_links: Vec<Option<usize>>,
    invocations: Vec<Invocation<'a>>,
    expanded_tokens: Vec<Token<'a>>,
    pending_space: bool,
    next_serial: u32,
    read_serial: ContextId,
    read_disabled: HideSetId,
    counter: u64,
    controlling_expression: bool,
    binary_exponents: bool,
    counters: crate::preprocess::counters::ExpansionCounters,
}
impl<'a> Expander<'a> {
    pub(crate) fn set_binary_exponents(&mut self, enabled: bool) {
        self.binary_exponents = enabled;
    }
    #[cfg(feature = "telemetry")]
    pub(crate) fn counters(&self) -> crate::preprocess::counters::ExpansionCounters {
        self.counters
    }
    pub(crate) fn set_controlling_expression(&mut self, enabled: bool) {
        self.controlling_expression = enabled;
    }
    pub(crate) fn next(
        &mut self,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
    ) -> Result<Token<'a>, PreprocessError> {
        loop {
            let Some(mut token) = self.read(source, names)? else {
                self.finish_argument(names)?;
                continue;
            };
            if token.kind == PpTokenKind::Identifier
                && self.start_expansion(&mut token, source, names, definitions)?
            {
                continue;
            }
            if self.invocations.is_empty() {
                return Ok(token);
            }
            self.expanded_tokens.push(token);
        }
    }
    fn read(
        &mut self,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
    ) -> Result<Option<Token<'a>>, PreprocessError> {
        let mut token;
        loop {
            let Some(context) = self.contexts.last_mut() else {
                token = source.next(names)?;
                self.read_serial = ContextId::default();
                self.read_disabled = HideSetId::default();
                break;
            };
            if context.cursor < context.range.end {
                token = if context.definition.is_some() {
                    self.context_tokens[context.cursor].clone()
                } else {
                    self.argument_tokens[context.cursor].clone()
                };
                context.cursor += 1;
                self.read_serial = if context.definition.is_some() {
                    context.serial
                } else {
                    ContextId::default()
                };
                self.read_disabled =
                    if context.definition.is_none() && token.flags & FROM_ARGUMENT != 0 {
                        context.outer
                    } else {
                        context.disabled
                    };
                break;
            }
            if context.definition.is_none() {
                return Ok(None);
            }
            self.pop_context()?;
        }
        if self.pending_space {
            token.flags |= LEADING_SPACE;
            self.pending_space = false;
        }
        Ok(Some(token))
    }
    fn peek_is_left_paren(
        &mut self,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
    ) -> Result<bool, PreprocessError> {
        loop {
            let Some(context) = self.contexts.last() else {
                return Ok(source.peek(names)?.is_punctuator(Punctuator::LeftParen));
            };
            if context.cursor < context.range.end {
                let token = if context.definition.is_some() {
                    &self.context_tokens[context.cursor]
                } else {
                    &self.argument_tokens[context.cursor]
                };
                return Ok(token.is_punctuator(Punctuator::LeftParen));
            }
            if context.definition.is_none() {
                return Ok(false);
            }
            self.pop_context()?;
        }
    }
    fn pop_context(&mut self) -> Result<(), PreprocessError> {
        let context = self
            .contexts
            .pop()
            .ok_or(PreprocessError::Invariant("missing expansion context"))?;
        if context.definition.is_some() {
            self.context_tokens.truncate(context.range.start);
        }
        Ok(())
    }
    fn open_context(&self, serial: ContextId) -> Option<&Context<'a>> {
        let at = self
            .contexts
            .partition_point(|context| context.serial.0 < serial.0);
        self.contexts
            .get(at)
            .filter(|context| context.serial == serial && context.definition.is_some())
    }
    fn nesting_of(&self, head: &Token<'a>, from_argument: bool, function_like: bool) -> Nesting {
        if head.flags & ARGUMENT != 0 || head.origin.0 == 0 {
            return Nesting::default();
        }
        if let Some(context) = self.open_context(head.origin) {
            return Nesting {
                parent: context.serial,
                chain: context.chain,
            };
        }
        if from_argument
            && let Some(barrier) = self.contexts.last()
            && (!function_like
                || self
                    .argument_tokens
                    .get(barrier.cursor)
                    .is_some_and(|token| token.flags & FOLLOWS_IN_CONTEXT != 0))
        {
            return Nesting {
                parent: head.origin,
                chain: head.hide_set,
            };
        }
        Nesting::default()
    }
    fn begin_inherited(
        &self,
        head: &Token<'a>,
        head_serial: ContextId,
        from_argument: bool,
    ) -> HideSetId {
        let Some(top) = self.contexts.last() else {
            return HideSetId::default();
        };
        if top.definition.is_none() {
            return if from_argument && head.flags & FROM_ARGUMENT != 0 {
                top.outer
            } else {
                top.inherited
            };
        }
        if top.serial == head_serial && head.flags & (ARGUMENT | REPLACED_ARGUMENT) == ARGUMENT {
            return HideSetId::default();
        }
        top.inherited
    }
    fn passes_disabled(head: &Token<'a>, nesting: Nesting) -> bool {
        head.flags & ARGUMENT == 0 && (nesting.parent.0 != 0 || head.origin.0 == 0)
    }
    fn start_expansion(
        &mut self,
        token: &mut Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
    ) -> Result<bool, PreprocessError> {
        if self.controlling_expression && definitions.is_defined_operator(token.name) {
            self.replace_defined(token, source, names, definitions)?;
            return Ok(false);
        }
        let Some(definition) = token.name.and_then(|name| definitions.get(name)) else {
            return Ok(false);
        };
        if token.flags & PAINTED != 0 {
            return Ok(false);
        }
        let disabled = if self.read_serial.0 != 0 && token.flags & ARGUMENT != 0 {
            HideSetId::default()
        } else {
            self.read_disabled
        };
        let hidden_by_set = self.hides.contains(token.hide_set, definition.name)?;
        if hidden_by_set || self.hides.contains(disabled, definition.name)? {
            if hidden_by_set && token.flags & (ARGUMENT | REPLACED_ARGUMENT) == ARGUMENT {
                self.counters.raw_operands_hidden =
                    self.counters.raw_operands_hidden.wrapping_add(1);
            }
            token.flags |= PAINTED;
            return Ok(false);
        }
        if let Some(builtin) = definition.builtin {
            match builtin {
                Builtin::File => {
                    token.kind = PpTokenKind::StringLiteral;
                    token.name = None;
                    token.spelling = Spelling::Shared(source.file_literal(token.begin));
                    token.hide_set = HideSetId::default();
                    return Ok(false);
                }
                Builtin::Line => {
                    Self::set_number(
                        token,
                        Spelling::Shared(
                            source
                                .presumed_line(token.begin)?
                                .to_string()
                                .into_bytes()
                                .into(),
                        ),
                    );
                    return Ok(false);
                }
                Builtin::HasCppAttribute | Builtin::HasAttribute => {
                    if self.controlling_expression {
                        self.replace_attribute(
                            token,
                            source,
                            names,
                            builtin == Builtin::HasAttribute,
                        )?;
                    }
                    return Ok(false);
                }
                Builtin::Counter => {
                    Self::set_number(
                        token,
                        Spelling::Shared(self.counter.to_string().into_bytes().into()),
                    );
                    self.counter = self.counter.wrapping_add(1);
                    return Ok(false);
                }
                Builtin::HasInclude | Builtin::HasIncludeNext => {
                    if self.controlling_expression {
                        self.replace_has_include(
                            token,
                            source,
                            names,
                            builtin == Builtin::HasIncludeNext,
                        )?;
                    }
                    return Ok(false);
                }
                Builtin::HasFeature
                | Builtin::HasExtension
                | Builtin::HasWarning
                | Builtin::HasDeclspecAttribute
                | Builtin::HasConstantEvaluation
                | Builtin::HasReadModifyWrite
                | Builtin::BuildingModule
                | Builtin::IsIdentifier => {
                    if self.controlling_expression {
                        self.replace_probe(token, source, names, builtin)?;
                    }
                    return Ok(false);
                }
                Builtin::HasProvidedName => {
                    if self.controlling_expression {
                        self.replace_has_builtin(token, source, names)?;
                    }
                    return Ok(false);
                }
            }
        }
        let context_disabled = self.read_disabled;
        let head_serial = self.read_serial;
        let from_argument = head_serial.0 == 0 && !self.contexts.is_empty();
        if !definition.function_like {
            let nesting = self.nesting_of(token, from_argument, false);
            let passes = Self::passes_disabled(token, nesting);
            let inherited = if passes {
                self.begin_inherited(token, head_serial, from_argument)
            } else {
                HideSetId::default()
            };
            self.push_expansion(
                &definition,
                token,
                nesting,
                if passes {
                    context_disabled
                } else {
                    HideSetId::default()
                },
                inherited,
                None,
                names,
            )?;
            return Ok(true);
        }
        if !self.peek_is_left_paren(source, names)? {
            return Ok(false);
        }
        let inherited = self.begin_inherited(token, head_serial, from_argument);
        let nesting = self.nesting_of(token, from_argument, true);
        let disabled = self.hides.merge(
            if nesting.parent.0 != 0 || token.flags & ARGUMENT != 0 {
                token.hide_set
            } else {
                HideSetId::default()
            },
            inherited,
        )?;
        let mut arguments = Arguments {
            raw: Vec::new(),
            expanded: Vec::new(),
            token_mark: self.argument_tokens.len(),
            expanded_mark: self.expanded_tokens.len(),
            disabled,
            outer: disabled,
        };
        if !self.collect_nested_arguments(&definition, &mut arguments)? {
            self.read(source, names)?;
            self.collect_arguments(&definition, token, &mut arguments, source, names)?;
        }
        Self::check_argument_count(&definition, token, &mut arguments)?;
        let passes = Self::passes_disabled(token, nesting);
        self.invocations.push(Invocation {
            definition,
            head: token.clone(),
            nesting,
            disabled: if passes {
                context_disabled
            } else {
                HideSetId::default()
            },
            inherited: if passes {
                inherited
            } else {
                HideSetId::default()
            },
            arguments,
            argument: 0,
            pending_space: self.pending_space,
        });
        self.counters.peak_invocations = self.counters.peak_invocations.max(self.invocations.len());
        self.advance_arguments(names)?;
        Ok(true)
    }
    fn set_number(token: &mut Token<'a>, spelling: Spelling<'a>) {
        token.kind = PpTokenKind::PpNumber;
        token.name = None;
        token.punctuator = None;
        token.spelling = spelling;
        token.hide_set = HideSetId::default();
        token.flags &= LEADING_SPACE | START_OF_LINE;
    }
    fn read_operator_operand(
        &mut self,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
    ) -> Result<Token<'a>, PreprocessError> {
        // A raw read deliberately bypasses macro replacement. At an argument barrier
        // C++ exposes its default end token, so malformed operators retain their own error.
        Ok(self
            .read(source, names)?
            .unwrap_or_else(|| Token::end(crate::support::source::SourceOffset(0))))
    }
    fn identifier_like_operator(token: &Token<'a>) -> bool {
        token.kind == PpTokenKind::PreprocessingOpOrPunc
            && token
                .spelling
                .bytes()
                .first()
                .is_some_and(u8::is_ascii_lowercase)
    }
    fn replace_defined(
        &mut self,
        head: &mut Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
    ) -> Result<(), PreprocessError> {
        let mut operand = self.read_operator_operand(source, names)?;
        let parenthesized = operand.is_punctuator(Punctuator::LeftParen);
        if parenthesized {
            operand = self.read_operator_operand(source, names)?;
        }
        let defined = if operand.kind == PpTokenKind::Identifier {
            operand
                .name
                .is_some_and(|name| definitions.is_defined(name))
        } else if Self::identifier_like_operator(&operand) {
            false
        } else {
            return Err(PreprocessError::at(
                head.begin,
                "`defined` requires an identifier",
            ));
        };
        if parenthesized
            && !self
                .read_operator_operand(source, names)?
                .is_punctuator(Punctuator::RightParen)
        {
            return Err(PreprocessError::at(
                head.begin,
                "missing `)` after `defined`",
            ));
        }
        Self::set_number(head, Spelling::Source(if defined { b"1" } else { b"0" }));
        Ok(())
    }
    fn replace_attribute(
        &mut self,
        head: &mut Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        gnu: bool,
    ) -> Result<(), PreprocessError> {
        if !self
            .read_operator_operand(source, names)?
            .is_punctuator(Punctuator::LeftParen)
        {
            return Err(PreprocessError::at(
                head.begin,
                "expected `(` after __has_cpp_attribute",
            ));
        }
        let operand = self.read_operator_operand(source, names)?;
        if operand.kind != PpTokenKind::Identifier && !Self::identifier_like_operator(&operand) {
            return Err(PreprocessError::at(
                head.begin,
                "expected an attribute name",
            ));
        }
        let mut reported = reported_attribute(operand.spelling.bytes(), gnu);
        let mut close = self.read_operator_operand(source, names)?;
        if close.is_punctuator(Punctuator::Scope) {
            let operand = self.read_operator_operand(source, names)?;
            if operand.kind != PpTokenKind::Identifier && !Self::identifier_like_operator(&operand)
            {
                return Err(PreprocessError::at(
                    head.begin,
                    "expected an attribute name",
                ));
            }
            reported = false;
            close = self.read_operator_operand(source, names)?;
        }
        if !close.is_punctuator(Punctuator::RightParen) {
            return Err(PreprocessError::at(
                head.begin,
                "expected `)` after the attribute name",
            ));
        }
        Self::set_number(head, Spelling::Source(if reported { b"1" } else { b"0" }));
        Ok(())
    }
    fn replace_has_builtin(
        &mut self,
        head: &mut Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
    ) -> Result<(), PreprocessError> {
        let operand = self.read_probe_operand(head, source, names)?;
        if operand.kind != PpTokenKind::Identifier && !Self::identifier_like_operator(&operand) {
            return Err(PreprocessError::at(
                head.begin,
                "expected an identifier after __has_builtin",
            ));
        }
        let answer = operand.kind == PpTokenKind::Identifier
            && crate::lex::builtins::is_provided(operand.spelling.bytes());
        Self::set_number(head, Spelling::Source(if answer { b"1" } else { b"0" }));
        Ok(())
    }
    fn read_probe_operand(
        &mut self,
        head: &Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
    ) -> Result<Token<'a>, PreprocessError> {
        if !self
            .read_operator_operand(source, names)?
            .is_punctuator(Punctuator::LeftParen)
        {
            return Err(PreprocessError::at(
                head.begin,
                "expected `(` after the probe",
            ));
        }
        let operand = self.read_operator_operand(source, names)?;
        if matches!(operand.kind, PpTokenKind::EndOfFile | PpTokenKind::NewLine) {
            return Err(PreprocessError::at(
                head.begin,
                "expected an operand of the probe",
            ));
        }
        if !self
            .read_operator_operand(source, names)?
            .is_punctuator(Punctuator::RightParen)
        {
            return Err(PreprocessError::at(
                head.begin,
                "expected `)` after the probe's operand",
            ));
        }
        Ok(operand)
    }
    fn replace_probe(
        &mut self,
        head: &mut Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        builtin: Builtin,
    ) -> Result<(), PreprocessError> {
        let operand = self.read_probe_operand(head, source, names)?;
        let spelling = operand.spelling.bytes();
        let answer = if builtin == Builtin::HasWarning {
            if operand.kind != PpTokenKind::StringLiteral {
                return Err(PreprocessError::at(
                    head.begin,
                    "expected a string literal after __has_warning",
                ));
            }
            false
        } else if builtin == Builtin::IsIdentifier {
            operand.kind == PpTokenKind::Identifier
                && crate::lex::vocabulary::SimpleToken::identifier(spelling).is_none()
                && crate::lex::gnu_keywords::syntax_keyword(spelling).is_none()
                && !crate::lex::builtins::is_trait(spelling)
        } else {
            if operand.kind != PpTokenKind::Identifier && !Self::identifier_like_operator(&operand)
            {
                return Err(PreprocessError::at(
                    head.begin,
                    "expected an identifier operand",
                ));
            }
            matches!(builtin, Builtin::HasFeature | Builtin::HasExtension)
                && super::features::is_provided(spelling, source.modes)
        };
        Self::set_number(head, Spelling::Source(if answer { b"1" } else { b"0" }));
        Ok(())
    }
    fn replace_has_include(
        &mut self,
        head: &mut Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        next: bool,
    ) -> Result<(), PreprocessError> {
        if !self
            .read_operator_operand(source, names)?
            .is_punctuator(Punctuator::LeftParen)
        {
            return Err(PreprocessError::at(
                head.begin,
                "expected `(` after __has_include",
            ));
        }
        let first = self.read_operator_operand(source, names)?;
        let mut name = Vec::new();
        let angle = first.is_punctuator(Punctuator::Less);
        if angle {
            let mut first = true;
            loop {
                let token = self.read_operator_operand(source, names)?;
                if token.is_punctuator(Punctuator::Greater) {
                    break;
                }
                if matches!(token.kind, PpTokenKind::EndOfFile | PpTokenKind::NewLine) {
                    return Err(PreprocessError::at(
                        head.begin,
                        "missing `>` in __has_include",
                    ));
                }
                if !first && token.flags & LEADING_SPACE != 0 {
                    name.push(b' ');
                }
                name.extend_from_slice(token.spelling.bytes());
                first = false;
            }
        } else if let Some(parts) =
            crate::lex::text::split_text(first.spelling.bytes()).filter(|parts| {
                first.kind == PpTokenKind::StringLiteral
                    && parts.prefix == crate::lex::text::EncodingPrefix::None
                    && !parts.raw
            })
        {
            name.extend_from_slice(
                &first.spelling.bytes()[parts.body_begin as usize..parts.body_end as usize],
            );
        } else {
            return Err(PreprocessError::at(
                head.begin,
                "expected a header name after __has_include",
            ));
        }
        if !self
            .read_operator_operand(source, names)?
            .is_punctuator(Punctuator::RightParen)
        {
            return Err(PreprocessError::at(
                head.begin,
                "expected `)` after the header name",
            ));
        }
        let answer = source.can_include(&name, angle, next)?;
        Self::set_number(head, Spelling::Source(if answer { b"1" } else { b"0" }));
        Ok(())
    }
    fn collect_arguments(
        &mut self,
        definition: &Definition<'a>,
        head: &Token<'a>,
        arguments: &mut Arguments,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
    ) -> Result<(), PreprocessError> {
        let variable = definition
            .variadic
            .then(|| definition.parameter_count().checked_sub(1))
            .flatten();
        let mut start = arguments.token_mark;
        let mut links = Vec::new();
        let mut previous_serial = ContextId::default();
        loop {
            let Some(mut token) = self
                .read(source, names)?
                .filter(|t| t.kind != PpTokenKind::EndOfFile)
            else {
                return Err(PreprocessError::at(
                    head.begin,
                    "unterminated macro invocation",
                ));
            };
            if self.read_serial.0 != 0 && self.read_serial == previous_serial {
                token.flags |= FOLLOWS_IN_CONTEXT;
            }
            previous_serial = self.read_serial;
            if self.read_serial.0 == 0 && !self.contexts.is_empty() {
                arguments.outer = self.hides.merge(arguments.outer, self.read_disabled)?;
                token.flags |= FROM_ARGUMENT;
            }
            let position = self.argument_tokens.len();
            if token.is_punctuator(Punctuator::LeftParen) {
                links.push(position);
            } else if token.is_punctuator(Punctuator::RightParen) {
                let Some(previous) = links.pop() else {
                    break;
                };
                self.argument_links[previous] = Some(position);
            } else if token.is_punctuator(Punctuator::Comma) {
                if let Some(previous) = links.last_mut() {
                    self.argument_links[*previous] = Some(position);
                    *previous = position;
                } else if Some(arguments.raw.len()) != variable {
                    arguments.raw.push(start..position);
                    start = position;
                    continue;
                }
            }
            self.argument_tokens.push(token);
            self.argument_links.push(None);
        }
        arguments.raw.push(start..self.argument_tokens.len());
        self.counters.argument_tokens = self
            .counters
            .argument_tokens
            .wrapping_add(self.argument_tokens.len() - arguments.token_mark);
        Ok(())
    }
    fn collect_nested_arguments(
        &mut self,
        definition: &Definition<'a>,
        arguments: &mut Arguments,
    ) -> Result<bool, PreprocessError> {
        let Some(context) = self
            .contexts
            .last_mut()
            .filter(|context| context.definition.is_none())
        else {
            return Ok(false);
        };
        arguments.outer = self.hides.merge(arguments.disabled, context.outer)?;
        self.counters.nested_collections = self.counters.nested_collections.wrapping_add(1);
        let variable = definition
            .variadic
            .then(|| definition.parameter_count().checked_sub(1))
            .flatten();
        let mut start = context.cursor + 1;
        let mut at = self.argument_links[context.cursor]
            .ok_or(PreprocessError::Invariant("unlinked argument parenthesis"))?;
        while !self.argument_tokens[at].is_punctuator(Punctuator::RightParen) {
            if Some(arguments.raw.len()) != variable {
                arguments.raw.push(start..at);
                start = at + 1;
            }
            at = self.argument_links[at]
                .ok_or(PreprocessError::Invariant("unlinked argument separator"))?;
        }
        arguments.raw.push(start..at);
        context.cursor = at + 1;
        Ok(true)
    }
    fn check_argument_count(
        definition: &Definition<'a>,
        head: &Token<'a>,
        arguments: &mut Arguments,
    ) -> Result<(), PreprocessError> {
        let parameters = definition.parameter_count();
        if parameters == 0 {
            if arguments.raw.len() != 1 || !arguments.raw[0].is_empty() {
                return Err(PreprocessError::at(head.begin, "too many macro arguments"));
            }
            arguments.raw.clear();
        } else if definition.variadic && arguments.raw.len() + 1 == parameters {
            let end = arguments
                .raw
                .last()
                .map_or(arguments.token_mark, |range| range.end);
            arguments.raw.push(end..end);
        } else if arguments.raw.len() != parameters {
            return Err(PreprocessError::at(
                head.begin,
                if arguments.raw.len() < parameters {
                    "too few macro arguments"
                } else {
                    "too many macro arguments"
                },
            ));
        }
        Ok(())
    }
    fn advance_arguments(&mut self, names: &mut Identifiers) -> Result<(), PreprocessError> {
        let invocation = self
            .invocations
            .last_mut()
            .ok_or(PreprocessError::Invariant("missing macro invocation"))?;
        while invocation.argument < invocation.arguments.raw.len() {
            if invocation.definition.expanded_uses[invocation.argument] {
                self.counters.argument_expansions =
                    self.counters.argument_expansions.wrapping_add(1);
                let begin = self.expanded_tokens.len();
                invocation.arguments.expanded.push(begin..begin);
                let range = invocation.arguments.raw[invocation.argument].clone();
                self.contexts.push(Context {
                    cursor: range.start,
                    range,
                    serial: ContextId(self.next_serial),
                    definition: None,
                    chain: HideSetId::default(),
                    disabled: invocation.arguments.disabled,
                    inherited: invocation.arguments.disabled,
                    outer: invocation.arguments.outer,
                });
                self.pending_space = false;
                return Ok(());
            }
            invocation.arguments.expanded.push(0..0);
            invocation.argument += 1;
        }
        self.complete_invocation(names)
    }
    fn finish_argument(&mut self, names: &mut Identifiers) -> Result<(), PreprocessError> {
        self.pop_context()?;
        let invocation = self
            .invocations
            .last_mut()
            .ok_or(PreprocessError::Invariant("argument without invocation"))?;
        invocation
            .arguments
            .expanded
            .last_mut()
            .ok_or(PreprocessError::Invariant(
                "argument without expansion range",
            ))?
            .end = self.expanded_tokens.len();
        invocation.argument += 1;
        self.advance_arguments(names)
    }
    fn complete_invocation(&mut self, names: &mut Identifiers) -> Result<(), PreprocessError> {
        let invocation = self
            .invocations
            .pop()
            .ok_or(PreprocessError::Invariant("missing completed invocation"))?;
        self.pending_space = invocation.pending_space;
        self.push_expansion(
            &invocation.definition,
            &invocation.head,
            invocation.nesting,
            invocation.disabled,
            invocation.inherited,
            Some(&invocation.arguments),
            names,
        )?;
        self.argument_tokens
            .truncate(invocation.arguments.token_mark);
        self.argument_links
            .truncate(invocation.arguments.token_mark);
        self.expanded_tokens
            .truncate(invocation.arguments.expanded_mark);
        Ok(())
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "One expansion consumes its immutable definition, head, nesting and argument facts"
    )]
    fn push_expansion(
        &mut self,
        definition: &Rc<Definition<'a>>,
        head: &Token<'a>,
        nesting: Nesting,
        disabled: HideSetId,
        inherited: HideSetId,
        arguments: Option<&Arguments>,
        names: &mut Identifiers,
    ) -> Result<(), PreprocessError> {
        self.counters.invocations = self.counters.invocations.wrapping_add(1);
        let begin = self.context_tokens.len();
        let argument_hide = if arguments.is_some() {
            self.hides.add(
                if head.flags & ARGUMENT != 0 {
                    head.hide_set
                } else {
                    HideSetId::default()
                },
                definition.name,
            )?
        } else {
            HideSetId::default()
        };
        self.next_serial = self.next_serial.checked_add(1).ok_or_else(|| {
            PreprocessError::at(
                head.begin,
                "too many macro invocations in one translation unit",
            )
        })?;
        let building = Building {
            head: head.clone(),
            serial: ContextId(self.next_serial),
            chain: self.hides.add(nesting.chain, definition.name)?,
        };
        let mut paste = false;
        let mut comma_paste = false;
        let mut first_item_emitted = false;
        for (index, replacement) in definition.body.iter().enumerate() {
            match replacement.role {
                Role::Token => {
                    let mut token = replacement.token.clone();
                    token.begin = head.begin;
                    token.end = head.end;
                    token.hide_set = building.chain;
                    token.origin = building.serial;
                    self.emit(token, &mut paste, &building, names)?;
                }
                Role::Parameter(slot) => {
                    let arguments = arguments
                        .ok_or(PreprocessError::Invariant("parameter without arguments"))?;
                    self.substitute(
                        arguments.range(slot, true)?,
                        replacement,
                        Some(argument_hide),
                        &mut paste,
                        &building,
                        names,
                    )?;
                }
                Role::RawParameter(slot) => {
                    let arguments = arguments.ok_or(PreprocessError::Invariant(
                        "raw parameter without arguments",
                    ))?;
                    let range = arguments.range(slot, false)?;
                    if comma_paste {
                        comma_paste = false;
                        if range.is_empty() {
                            self.context_tokens.pop();
                        }
                    }
                    if range.is_empty() {
                        let mut token = Token::end(head.begin);
                        token.end = head.end;
                        token.placemarker = true;
                        self.emit(token, &mut paste, &building, names)?;
                    } else {
                        self.substitute(range, replacement, None, &mut paste, &building, names)?;
                    }
                }
                Role::Stringize(slot) => {
                    let arguments = arguments
                        .ok_or(PreprocessError::Invariant("stringize without arguments"))?;
                    let token = self.stringize(arguments.range(slot, false)?, replacement, head);
                    self.emit(token, &mut paste, &building, names)?;
                }
                Role::StringizedParameter => {}
                Role::Paste => paste = true,
                Role::CommaPaste => comma_paste = true,
            }
            if index == 0 {
                first_item_emitted = self.context_tokens.len() != begin;
            }
        }
        first_item_emitted = first_item_emitted && !self.context_tokens[begin].placemarker;
        let mut out = begin;
        for at in begin..self.context_tokens.len() {
            if !self.context_tokens[at].placemarker {
                if out != at {
                    self.context_tokens[out] = self.context_tokens[at].clone();
                }
                out += 1;
            }
        }
        self.context_tokens.truncate(out);
        self.counters.produced_tokens = self.counters.produced_tokens.wrapping_add(out - begin);
        if out == begin {
            if head.flags & LEADING_SPACE != 0 {
                self.pending_space = true;
            }
            return Ok(());
        }
        let first = &mut self.context_tokens[begin];
        let own = if first_item_emitted { 0 } else { first.flags };
        first.flags = (first.flags & !(LEADING_SPACE | START_OF_LINE))
            | ((head.flags | own) & (LEADING_SPACE | START_OF_LINE));
        self.contexts.push(Context {
            range: begin..out,
            cursor: begin,
            serial: building.serial,
            definition: Some(definition.clone()),
            chain: building.chain,
            disabled,
            inherited,
            outer: HideSetId::default(),
        });
        self.counters.peak_contexts = self.counters.peak_contexts.max(self.contexts.len());
        self.counters.peak_context_tokens = self
            .counters
            .peak_context_tokens
            .max(self.context_tokens.len());
        Ok(())
    }
    fn substitute(
        &mut self,
        range: Range<usize>,
        parameter: &Replacement<'a>,
        hide: Option<HideSetId>,
        paste: &mut bool,
        building: &Building<'a>,
        names: &mut Identifiers,
    ) -> Result<(), PreprocessError> {
        let begin = range.start;
        for at in range {
            let mut token = if hide.is_some() {
                self.expanded_tokens[at].clone()
            } else {
                self.argument_tokens[at].clone()
            };
            if let Some(hide) = hide {
                token.hide_set = hide;
                token.origin = ContextId::default();
            } else if token.flags & REPLACED_ARGUMENT != 0 {
                token.hide_set = self.hides.without_last(token.hide_set);
            }
            token.flags = (token.flags
                & !(START_OF_LINE | REPLACED_ARGUMENT | FOLLOWS_IN_CONTEXT | FROM_ARGUMENT))
                | ARGUMENT
                | if hide.is_some() { REPLACED_ARGUMENT } else { 0 };
            if at == begin {
                token.flags =
                    (token.flags & !LEADING_SPACE) | (parameter.token.flags & LEADING_SPACE);
            }
            self.emit(token, paste, building, names)?;
        }
        Ok(())
    }
    fn emit(
        &mut self,
        token: Token<'a>,
        paste: &mut bool,
        building: &Building<'a>,
        names: &mut Identifiers,
    ) -> Result<(), PreprocessError> {
        if !*paste {
            self.context_tokens.push(token);
            return Ok(());
        }
        *paste = false;
        let left = self
            .context_tokens
            .pop()
            .ok_or(PreprocessError::Invariant("paste without left operand"))?;
        let result = self.paste_tokens(left, token, building, names)?;
        self.context_tokens.push(result);
        Ok(())
    }
    fn stringize(
        &mut self,
        range: Range<usize>,
        operator: &Replacement<'a>,
        head: &Token<'a>,
    ) -> Token<'a> {
        self.counters.stringizes = self.counters.stringizes.wrapping_add(1);
        let mut text = vec![b'"'];
        let begin = range.start;
        for at in range {
            let token = &self.argument_tokens[at];
            if at != begin && token.flags & LEADING_SPACE != 0 {
                text.push(b' ');
            }
            let literal = matches!(
                token.kind,
                PpTokenKind::CharacterLiteral
                    | PpTokenKind::UserDefinedCharacterLiteral
                    | PpTokenKind::StringLiteral
                    | PpTokenKind::UserDefinedStringLiteral
            );
            for &byte in token.spelling.bytes() {
                if literal && matches!(byte, b'"' | b'\\') {
                    text.push(b'\\');
                }
                text.push(byte);
            }
        }
        text.push(b'"');
        let mut token = Token::end(head.begin);
        token.end = head.end;
        token.kind = PpTokenKind::StringLiteral;
        token.spelling = Spelling::Shared(text.into());
        token.flags = operator.token.flags & LEADING_SPACE;
        token
    }
    fn paste_tokens(
        &mut self,
        left: Token<'a>,
        right: Token<'a>,
        building: &Building<'a>,
        names: &mut Identifiers,
    ) -> Result<Token<'a>, PreprocessError> {
        if left.placemarker || right.placemarker {
            let mut result = if left.placemarker { right } else { left };
            result.flags &= !(ARGUMENT | REPLACED_ARGUMENT | FOLLOWS_IN_CONTEXT | FROM_ARGUMENT);
            if !result.placemarker {
                result.hide_set = self.hides.merge(building.chain, result.hide_set)?;
            }
            result.origin = building.serial;
            return Ok(result);
        }
        let mut text = left.spelling.bytes().to_vec();
        self.counters.pastes = self.counters.pastes.wrapping_add(1);
        text.extend_from_slice(right.spelling.bytes());
        let length = text.len();
        let source = SourceBuffer::for_token_text(text).map_err(PreprocessError::Limit)?;
        let mut lexer = PpLexer::with_options(
            &source,
            PpLexerOptions {
                header_names_on_request: true,
                token_text: true,
                binary_exponents: self.binary_exponents,
                ..PpLexerOptions::default()
            },
        )
        .map_err(|_| {
            PreprocessError::at(
                building.head.begin,
                "token pasting does not form a valid preprocessing token",
            )
        })?;
        let pp = lexer.next_token().map_err(|_| {
            PreprocessError::at(
                building.head.begin,
                "token pasting does not form a valid preprocessing token",
            )
        })?;
        if matches!(
            pp.kind,
            PpTokenKind::WhitespaceSequence | PpTokenKind::NewLine
        ) || pp.end.index() != length
        {
            return Err(PreprocessError::at(
                building.head.begin,
                "token pasting does not form one preprocessing token",
            ));
        }
        let mut result = Token::end(building.head.begin);
        result.end = building.head.end;
        result.kind = pp.kind;
        result.punctuator = pp.punctuator;
        if pp.kind == PpTokenKind::Identifier {
            if pp.spelling == b"__VA_ARGS__" {
                return Err(PreprocessError::at(
                    building.head.begin,
                    "__VA_ARGS__ formed by token pasting",
                ));
            }
            let name = names.intern(pp.spelling).map_err(PreprocessError::Limit)?;
            result.name = Some(name);
            result.spelling = Spelling::Shared(names.spelling(name));
        } else {
            result.spelling = Spelling::Shared(pp.spelling.into());
        }
        result.flags = left.flags & (LEADING_SPACE | START_OF_LINE);
        result.hide_set = building.chain;
        result.origin = building.serial;
        Ok(result)
    }
}

#[cfg(test)]
impl Expander<'_> {
    pub(crate) fn live_storage(&self) -> [usize; 5] {
        [
            self.contexts.len(),
            self.context_tokens.len(),
            self.argument_tokens.len(),
            self.invocations.len(),
            self.expanded_tokens.len(),
        ]
    }
}

// Exact attribute-reporting vocabulary from macro_expander.cpp; namespace probes
// intentionally answer false even when the unqualified spelling is recognized.
fn reported_attribute(mut name: &[u8], gnu: bool) -> bool {
    if name.len() > 4 && name.starts_with(b"__") && name.ends_with(b"__") {
        name = &name[2..name.len() - 2];
    }
    if gnu {
        matches!(
            name,
            b"always_inline"
                | b"noinline"
                | b"noreturn"
                | b"pure"
                | b"const"
                | b"used"
                | b"weak"
                | b"packed"
                | b"aligned"
                | b"section"
                | b"vector_size"
                | b"gnu_inline"
                | b"abi_tag"
                | b"using_if_exists"
                | b"exclude_from_explicit_instantiation"
        )
    } else {
        name == b"no_unique_address"
    }
}
