// Translated from preprocess/preprocessor_directives.cpp. See NOTICE.
//! Conditional execution and inactive sections; controlling expressions share PA3's stacks.
use super::error::PreprocessError;
use super::expression::{
    ExpressionError, ExpressionEvaluator, ExpressionStatus, MacroDefinitionQuery,
};
use super::input::SourceInput;
use super::macros::{definition::Definitions, expander::Expander, token::Token};
use crate::lex::identifiers::Identifiers;
use crate::lex::number::NumberDialect;
use crate::lex::pp_lexer::{PpToken, PpTokenCursor, PpTokenKind};
use crate::support::source::SourceOffset;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionalDirective {
    If,
    Ifdef,
    Ifndef,
    Elif,
    Else,
    Endif,
}
impl ConditionalDirective {
    pub(crate) fn from_spelling(text: &[u8]) -> Option<Self> {
        Some(match text {
            b"if" => Self::If,
            b"ifdef" => Self::Ifdef,
            b"ifndef" => Self::Ifndef,
            b"elif" => Self::Elif,
            b"else" => Self::Else,
            b"endif" => Self::Endif,
            _ => return None,
        })
    }
}
struct Conditional {
    location: SourceOffset,
    taken: bool,
    in_else: bool,
}
// `defined` is already consumed by the rescanner, including in argument prescan.
struct ReplacedDefinitions;
impl MacroDefinitionQuery for ReplacedDefinitions {
    fn is_defined(&self, _spelling: &[u8]) -> bool {
        false
    }
}
static REPLACED_DEFINITIONS: ReplacedDefinitions = ReplacedDefinitions;

pub(crate) struct Conditionals {
    sections: Vec<Conditional>,
    skipped: Vec<bool>,
    evaluator: ExpressionEvaluator<'static, ReplacedDefinitions, PreprocessError>,
    pub(crate) skipped_directives: crate::support::telemetry::Counter<u64>,
}
impl Default for Conditionals {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            skipped: Vec::new(),
            evaluator: ExpressionEvaluator::new(&REPLACED_DEFINITIONS, NumberDialect::Cxx11),
            skipped_directives: crate::support::telemetry::counter(0),
        }
    }
}
impl Conditionals {
    pub(crate) fn set_number_dialect(&mut self, dialect: NumberDialect) {
        self.evaluator.set_number_dialect(dialect);
    }
    pub(crate) fn depth(&self) -> usize {
        self.sections.len()
    }
    pub(crate) fn open_since(&self, depth: usize) -> Option<SourceOffset> {
        (self.sections.len() > depth)
            .then(|| self.sections.last().map(|section| section.location))
            .flatten()
    }
    pub(crate) fn check_end(&self) -> Result<(), PreprocessError> {
        if let Some(section) = self.sections.last() {
            return Err(PreprocessError::at(
                section.location,
                "unterminated conditional directive",
            ));
        }
        Ok(())
    }
    pub(crate) fn run<'a>(
        &mut self,
        directive: &Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
        expander: &mut Expander<'a>,
    ) -> Result<(), PreprocessError> {
        let role = ConditionalDirective::from_spelling(directive.spelling.bytes())
            .ok_or(PreprocessError::Invariant("unknown conditional directive"))?;
        match role {
            ConditionalDirective::If
            | ConditionalDirective::Ifdef
            | ConditionalDirective::Ifndef => {
                let taken = if role == ConditionalDirective::If {
                    self.evaluate(directive, source, names, definitions, expander)?
                } else {
                    let name = source.next_in_line(names)?;
                    if name.kind != PpTokenKind::Identifier {
                        return Err(PreprocessError::at(
                            directive.begin,
                            "#ifdef and #ifndef require an identifier",
                        ));
                    }
                    expect_end(directive, source, names)?;
                    if role == ConditionalDirective::Ifndef {
                        source.open_guard(
                            name.name.ok_or(PreprocessError::Invariant(
                                "conditional name without identifier",
                            ))?,
                            self.sections.len(),
                        );
                    }
                    name.name.is_some_and(|name| definitions.is_defined(name))
                        == (role == ConditionalDirective::Ifdef)
                };
                self.sections.push(Conditional {
                    location: directive.begin,
                    taken,
                    in_else: false,
                });
                if taken {
                    return Ok(());
                }
            }
            ConditionalDirective::Elif
            | ConditionalDirective::Else
            | ConditionalDirective::Endif => {
                if self.sections.len() <= source.conditional_depth() {
                    return Err(PreprocessError::at(
                        directive.begin,
                        "#elif, #else or #endif without #if",
                    ));
                }
                let section = self.sections.last_mut().ok_or_else(|| {
                    PreprocessError::at(directive.begin, "#elif, #else or #endif without #if")
                })?;
                if role == ConditionalDirective::Endif {
                    expect_end(directive, source, names)?;
                    self.sections.pop();
                    return Ok(());
                }
                if section.in_else {
                    return Err(PreprocessError::at(
                        directive.begin,
                        "#elif or #else after #else",
                    ));
                }
                if role == ConditionalDirective::Else {
                    section.in_else = true;
                    expect_end(directive, source, names)?;
                } else {
                    source.skip_line()?;
                }
            }
        }
        self.skip_group(source, names, definitions, expander)
    }
    fn skip_group<'a>(
        &mut self,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
        expander: &mut Expander<'a>,
    ) -> Result<(), PreprocessError> {
        self.skipped.clear();
        loop {
            if !source.skip_to_directive()? {
                return self.check_end();
            }
            self.skipped_directives = self.skipped_directives.wrapping_add(1);
            source.allow_va_args = true;
            let name = source.next_in_line(names);
            source.allow_va_args = false;
            let name = name?;
            if name.kind == PpTokenKind::NewLine {
                continue;
            }
            let role = (name.kind == PpTokenKind::Identifier)
                .then(|| ConditionalDirective::from_spelling(name.spelling.bytes()))
                .flatten();
            match role {
                Some(
                    ConditionalDirective::If
                    | ConditionalDirective::Ifdef
                    | ConditionalDirective::Ifndef,
                ) => {
                    self.skipped.push(false);
                }
                Some(ConditionalDirective::Elif | ConditionalDirective::Else)
                    if !self.skipped.is_empty() =>
                {
                    let in_else = self
                        .skipped
                        .last_mut()
                        .ok_or(PreprocessError::Invariant("missing skipped section"))?;
                    if *in_else {
                        return Err(PreprocessError::at(
                            name.begin,
                            "#elif or #else after #else",
                        ));
                    }
                    *in_else = role == Some(ConditionalDirective::Else);
                }
                Some(ConditionalDirective::Endif) if !self.skipped.is_empty() => {
                    self.skipped.pop();
                }
                Some(role) => {
                    if self.resume_group(role, &name, source, names, definitions, expander)? {
                        return Ok(());
                    }
                    continue;
                }
                None => {
                    if name.kind == PpTokenKind::Identifier
                        && matches!(name.spelling.bytes(), b"include" | b"include_next")
                    {
                        source.request_header_name();
                    }
                }
            }
            source.skip_line()?;
        }
    }
    fn resume_group<'a>(
        &mut self,
        role: ConditionalDirective,
        directive: &Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
        expander: &mut Expander<'a>,
    ) -> Result<bool, PreprocessError> {
        source.track_guard(directive.spelling.bytes(), self.sections.len());
        if role == ConditionalDirective::Endif {
            expect_end(directive, source, names)?;
            self.sections.pop();
            return Ok(true);
        }
        let section = self
            .sections
            .last_mut()
            .ok_or(PreprocessError::Invariant("missing skipped conditional"))?;
        if section.in_else {
            return Err(PreprocessError::at(
                directive.begin,
                "#elif or #else after #else",
            ));
        }
        let taken = section.taken;
        if role == ConditionalDirective::Else {
            section.in_else = true;
            expect_end(directive, source, names)?;
        } else if taken {
            source.skip_line()?;
            return Ok(false);
        } else if !self.evaluate(directive, source, names, definitions, expander)? {
            return Ok(false);
        }
        if taken {
            return Ok(false);
        }
        self.sections
            .last_mut()
            .ok_or(PreprocessError::Invariant("missing resumed conditional"))?
            .taken = true;
        Ok(true)
    }
    fn evaluate<'a>(
        &mut self,
        directive: &Token<'a>,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        definitions: &Definitions<'a>,
        expander: &mut Expander<'a>,
    ) -> Result<bool, PreprocessError> {
        source.begin_line_input();
        expander.set_controlling_expression(true);
        let result = self.evaluator.evaluate(&mut ReplacedLine {
            source,
            names,
            definitions,
            expander,
            current: Token::end(directive.begin),
        });
        expander.set_controlling_expression(false);
        let result = result.map_err(|error| match error {
            ExpressionError::Input(error) => error,
            ExpressionError::Invariant(message) => PreprocessError::Invariant(message),
        })?;
        source.end_line_input()?;
        match result.status {
            ExpressionStatus::Empty => Err(PreprocessError::at(
                directive.begin,
                "missing controlling expression",
            )),
            ExpressionStatus::Invalid => Err(PreprocessError::at(
                directive.begin,
                "invalid controlling expression",
            )),
            ExpressionStatus::Valid(value) => Ok(value.bits != 0),
        }
    }
}
fn expect_end<'a>(
    directive: &Token<'a>,
    source: &mut SourceInput<'a>,
    names: &mut Identifiers,
) -> Result<(), PreprocessError> {
    if source.next_in_line(names)?.kind != PpTokenKind::NewLine {
        return Err(PreprocessError::at(
            directive.begin,
            "extra tokens at the end of the directive",
        ));
    }
    Ok(())
}
struct ReplacedLine<'a, 's> {
    source: &'s mut SourceInput<'a>,
    names: &'s mut Identifiers,
    definitions: &'s Definitions<'a>,
    expander: &'s mut Expander<'a>,
    current: Token<'a>,
}
impl PpTokenCursor for ReplacedLine<'_, '_> {
    type Error = PreprocessError;
    fn next_token(&mut self) -> Result<PpToken<'_>, PreprocessError> {
        self.current = self
            .expander
            .next(self.source, self.names, self.definitions)?;
        if self.current.kind == PpTokenKind::EndOfFile {
            self.current.kind = PpTokenKind::NewLine;
        }
        Ok(self.current.view())
    }
}
