// Translated from preprocess/macro/macro_definition.cpp. See NOTICE.
//! One-time replacement-list role analysis; definitions are immutable and shared by invocations.
use super::token::{LEADING_SPACE, Token};
use crate::lex::identifiers::{Identifiers, NameId};
use crate::lex::pp_lexer::{PpTokenKind, Punctuator};
use crate::preprocess::{error::PreprocessError, input::SourceInput};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Token,
    Parameter(u16),
    RawParameter(u16),
    Stringize(u16),
    StringizedParameter,
    Paste,
    CommaPaste,
}
#[derive(Clone)]
pub(crate) struct Replacement<'a> {
    pub(crate) token: Token<'a>,
    pub(crate) role: Role,
}
pub(crate) struct Definition<'a> {
    pub(crate) replaceable: bool,
    pub(crate) name: NameId,
    pub(crate) body: Vec<Replacement<'a>>,
    pub(crate) parameters: Vec<NameId>,
    pub(crate) expanded_uses: Vec<bool>,
    pub(crate) function_like: bool,
    pub(crate) variadic: bool,
    pub(crate) builtin: Option<Builtin>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Builtin {
    File,
    Line,
    HasCppAttribute,
    HasAttribute,
    Counter,
    HasProvidedName,
}
pub(crate) struct Definitions<'a> {
    defined_name: NameId,
    slots: Vec<Option<Rc<Definition<'a>>>>,
}
impl<'a> Definitions<'a> {
    pub(crate) fn add_hosted(&mut self, names: &mut Identifiers) -> Result<(), PreprocessError> {
        let name = names
            .intern(b"__has_builtin")
            .map_err(PreprocessError::Limit)?;
        let mut definition = Definition::empty(name);
        definition.builtin = Some(Builtin::HasProvidedName);
        self.define(definition, crate::support::source::SourceOffset(0))
    }
    pub(crate) fn new(names: &mut Identifiers) -> Result<Self, PreprocessError> {
        let mut definitions = Self {
            defined_name: names.intern(b"defined").map_err(PreprocessError::Limit)?,
            slots: Vec::new(),
        };
        // PreprocessorNames interns the directive/reserved vocabulary before any
        // file is read. Retain even hosted-only roles for the observable name count.
        for spelling in [
            b"if".as_slice(),
            b"ifdef",
            b"ifndef",
            b"elif",
            b"else",
            b"endif",
            b"include",
            b"define",
            b"undef",
            b"line",
            b"error",
            b"pragma",
            b"include_next",
            b"warning",
            b"__VA_ARGS__",
            b"_Pragma",
            b"once",
            b"__cplusplus",
            b"__DATE__",
            b"__FILE__",
            b"__LINE__",
            b"__STDC_HOSTED__",
            b"__TIME__",
        ] {
            names.intern(spelling).map_err(PreprocessError::Limit)?;
        }
        for (spelling, builtin) in [
            (b"__FILE__".as_slice(), Builtin::File),
            (b"__LINE__", Builtin::Line),
            (b"__COUNTER__", Builtin::Counter),
            (b"__has_cpp_attribute", Builtin::HasCppAttribute),
            (b"__has_attribute", Builtin::HasAttribute),
        ] {
            let name = names.intern(spelling).map_err(PreprocessError::Limit)?;
            let mut definition = Definition::empty(name);
            definition.builtin = Some(builtin);
            definitions.define(definition, crate::support::source::SourceOffset(0))?;
        }
        Ok(definitions)
    }
    pub(crate) fn is_defined_operator(&self, name: Option<NameId>) -> bool {
        name == Some(self.defined_name)
    }
    pub(crate) fn is_defined(&self, name: NameId) -> bool {
        self.slots.get(name.index()).is_some_and(Option::is_some)
    }
    pub(crate) fn get(&self, name: NameId) -> Option<Rc<Definition<'a>>> {
        self.slots.get(name.index()).and_then(Clone::clone)
    }
    pub(crate) fn define(
        &mut self,
        definition: Definition<'a>,
        at: crate::support::source::SourceOffset,
    ) -> Result<(), PreprocessError> {
        let name = definition.name;
        if let Some(previous) = self.get(name)
            && !previous.replaceable
        {
            if !previous.same_as(&definition) {
                return Err(PreprocessError::at(at, "macro redefined differently"));
            }
            return Ok(());
        }
        self.slots
            .resize_with(self.slots.len().max(name.index() + 1), || None);
        self.slots[name.index()] = Some(Rc::new(definition));
        Ok(())
    }
    pub(crate) fn undefine(&mut self, name: NameId) {
        if let Some(slot) = self.slots.get_mut(name.index()) {
            *slot = None;
        }
    }
}
impl<'a> Definition<'a> {
    pub(crate) fn parameter_count(&self) -> usize {
        // C++ stores the count in uint16_t. The ellipsis branch can append
        // one parameter after 65,535 named parameters, wrapping that count.
        self.parameters.len() & usize::from(u16::MAX)
    }
    fn empty(name: NameId) -> Self {
        Self {
            name,
            body: Vec::new(),
            parameters: Vec::new(),
            expanded_uses: Vec::new(),
            function_like: false,
            variadic: false,
            builtin: None,
            replaceable: false,
        }
    }
    fn same_as(&self, other: &Self) -> bool {
        self.builtin == other.builtin
            && self.function_like == other.function_like
            && self.variadic == other.variadic
            && self.parameter_count() == other.parameter_count()
            && self.parameters[..self.parameter_count()]
                == other.parameters[..other.parameter_count()]
            && self.body.len() == other.body.len()
            && self
                .body
                .iter()
                .zip(&other.body)
                .enumerate()
                .all(|(index, (x, y))| {
                    x.token.kind == y.token.kind
                        && x.token.spelling.bytes() == y.token.spelling.bytes()
                        && (index == 0 || (x.token.flags ^ y.token.flags) & LEADING_SPACE == 0)
                })
    }
    pub(crate) fn parse(
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        name: NameId,
    ) -> Result<Self, PreprocessError> {
        let mut token = source.next_in_line(names)?;
        let function_like =
            token.is_punctuator(Punctuator::LeftParen) && token.flags & LEADING_SPACE == 0;
        let mut definition = Self {
            replaceable: false,
            name,
            body: Vec::new(),
            parameters: Vec::new(),
            expanded_uses: Vec::new(),
            function_like,
            variadic: false,
            builtin: None,
        };
        let mut slots = HashMap::new();
        if function_like {
            definition.parse_parameters(source, names, &token, &mut slots)?;
            token = source.next_in_line(names)?;
        } else if token.kind != PpTokenKind::NewLine && token.flags & LEADING_SPACE == 0 {
            return Err(PreprocessError::at(
                token.begin,
                "missing white space after the macro name",
            ));
        }
        while token.kind != PpTokenKind::NewLine {
            token.flags &= LEADING_SPACE;
            definition.body.push(Replacement {
                token,
                role: Role::Token,
            });
            token = source.next_in_line(names)?;
        }
        source.allow_va_args = false;
        definition.assign_roles(&slots)?;
        definition
            .expanded_uses
            .resize(definition.parameters.len(), false);
        for replacement in &definition.body {
            if let Role::Parameter(slot) = replacement.role {
                definition.expanded_uses[usize::from(slot)] = true;
            }
        }
        Ok(definition)
    }
    fn parse_parameters(
        &mut self,
        source: &mut SourceInput<'a>,
        names: &mut Identifiers,
        open: &Token<'a>,
        slots: &mut HashMap<NameId, u16>,
    ) -> Result<(), PreprocessError> {
        let mut token = source.next_in_line(names)?;
        if token.is_punctuator(Punctuator::RightParen) {
            return Ok(());
        }
        loop {
            if token.is_punctuator(Punctuator::Ellipsis) {
                self.variadic = true;
                let va_args = names
                    .intern(b"__VA_ARGS__")
                    .map_err(PreprocessError::Limit)?;
                if self.parameters.len() == usize::from(u16::MAX) {
                    // The frozen parser wraps __VA_ARGS__'s one-based slot
                    // to zero, so it has no parameter role in this definition.
                    self.parameters.push(va_args);
                } else {
                    self.add_parameter(va_args, token.begin, slots)?;
                }
                source.allow_va_args = true;
                token = source.next_in_line(names)?;
                if !token.is_punctuator(Punctuator::RightParen) {
                    return Err(PreprocessError::at(
                        token.begin,
                        "`...` must end the macro parameters",
                    ));
                }
                return Ok(());
            }
            if token.kind != PpTokenKind::Identifier {
                return Err(PreprocessError::at(
                    if token.kind == PpTokenKind::NewLine {
                        open.begin
                    } else {
                        token.begin
                    },
                    "expected a macro parameter name",
                ));
            }
            self.add_parameter(
                token
                    .name
                    .ok_or(PreprocessError::Invariant("parameter without identifier"))?,
                token.begin,
                slots,
            )?;
            token = source.next_in_line(names)?;
            if token.is_punctuator(Punctuator::RightParen) {
                return Ok(());
            }
            if !token.is_punctuator(Punctuator::Comma) {
                return Err(PreprocessError::at(
                    if token.kind == PpTokenKind::NewLine {
                        open.begin
                    } else {
                        token.begin
                    },
                    "expected `,` or `)` in the macro parameters",
                ));
            }
            token = source.next_in_line(names)?;
        }
    }
    fn add_parameter(
        &mut self,
        name: NameId,
        location: crate::support::source::SourceOffset,
        slots: &mut HashMap<NameId, u16>,
    ) -> Result<(), PreprocessError> {
        if slots.contains_key(&name) {
            return Err(PreprocessError::at(
                location,
                "duplicate macro parameter name",
            ));
        }
        if self.parameters.len() >= usize::from(u16::MAX) {
            return Err(PreprocessError::at(location, "too many macro parameters"));
        }
        let slot = u16::try_from(self.parameters.len())
            .map_err(|_| PreprocessError::Limit("too many macro parameters"))?;
        slots.insert(name, slot);
        self.parameters.push(name);
        Ok(())
    }
    fn assign_roles(&mut self, slots: &HashMap<NameId, u16>) -> Result<(), PreprocessError> {
        if self.function_like {
            let mut at = 0;
            while at < self.body.len() {
                let token = &self.body[at].token;
                if let Some(slot) = token.name.and_then(|name| slots.get(&name)) {
                    self.body[at].role = Role::Parameter(*slot);
                } else if token.is_punctuator(Punctuator::Hash) {
                    let slot = self
                        .body
                        .get(at + 1)
                        .and_then(|r| r.token.name)
                        .and_then(|name| slots.get(&name));
                    let Some(&slot) = slot else {
                        return Err(PreprocessError::at(
                            token.begin,
                            "`#` is not followed by a macro parameter",
                        ));
                    };
                    self.body[at].role = Role::Stringize(slot);
                    self.body[at + 1].role = Role::StringizedParameter;
                    at += 1;
                }
                at += 1;
            }
        }
        for at in 0..self.body.len() {
            if !self.body[at].token.is_punctuator(Punctuator::HashHash) {
                continue;
            }
            if at == 0 || at + 1 == self.body.len() {
                return Err(PreprocessError::at(
                    self.body[at].token.begin,
                    "`##` cannot begin or end a replacement list",
                ));
            }
            if matches!(self.body[at - 1].role, Role::Paste | Role::CommaPaste) {
                continue;
            }
            self.body[at].role = Role::Paste;
            if !self.function_like {
                continue;
            }
            if let Role::Parameter(slot) = self.body[at - 1].role {
                self.body[at - 1].role = Role::RawParameter(slot);
            }
            if let Role::Parameter(slot) = self.body[at + 1].role {
                self.body[at + 1].role = Role::RawParameter(slot);
            }
            if self.variadic
                && self.body[at - 1].role == Role::Token
                && self.body[at - 1].token.is_punctuator(Punctuator::Comma)
                && matches!(self.body[at+1].role, Role::RawParameter(slot) if usize::from(slot)+1 == self.parameters.len())
            {
                self.body[at].role = Role::CommaPaste;
            }
        }
        Ok(())
    }
}
