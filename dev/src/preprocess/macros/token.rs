//! Stable preprocessing text and per-token recursion facts.
use super::hide_sets::HideSetId;
use crate::lex::identifiers::NameId;
use crate::lex::pp_lexer::{PpToken, PpTokenKind, Punctuator};
use crate::support::source::{SourceBuffer, SourceOffset};
use std::rc::Rc;

pub(crate) const LEADING_SPACE: u8 = 1;
pub(crate) const START_OF_LINE: u8 = 2;
pub(crate) const PAINTED: u8 = 4;
pub(crate) const ARGUMENT: u8 = 8;
pub(crate) const REPLACED_ARGUMENT: u8 = 16;
pub(crate) const FOLLOWS_IN_CONTEXT: u8 = 32;
pub(crate) const FROM_ARGUMENT: u8 = 64;

#[derive(Clone)]
pub(crate) enum Spelling<'a> {
    Source(&'a [u8]),
    Shared(Rc<[u8]>),
    Image {
        source: Rc<SourceBuffer>,
        begin: SourceOffset,
        end: SourceOffset,
    },
}
impl Spelling<'_> {
    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            Self::Source(text) => text,
            Self::Shared(text) => text,
            Self::Image { source, begin, end } => &source.bytes()[begin.index()..end.index()],
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ContextId(pub(crate) u32);

#[derive(Clone)]
pub(crate) struct Token<'a> {
    pub(crate) kind: PpTokenKind,
    pub(crate) spelling: Spelling<'a>,
    pub(crate) begin: SourceOffset,
    pub(crate) end: SourceOffset,
    pub(crate) name: Option<NameId>,
    pub(crate) punctuator: Option<Punctuator>,
    pub(crate) flags: u8,
    pub(crate) hide_set: HideSetId,
    pub(crate) origin: ContextId,
    pub(crate) placemarker: bool,
}
impl Token<'_> {
    pub(crate) fn end(location: SourceOffset) -> Self {
        Self {
            kind: PpTokenKind::EndOfFile,
            spelling: Spelling::Source(b""),
            begin: location,
            end: location,
            name: None,
            punctuator: None,
            flags: 0,
            hide_set: HideSetId::default(),
            origin: ContextId::default(),
            placemarker: false,
        }
    }
    pub(crate) fn is_punctuator(&self, punctuator: Punctuator) -> bool {
        self.kind == PpTokenKind::PreprocessingOpOrPunc && self.punctuator == Some(punctuator)
    }
    pub(crate) fn view(&self) -> PpToken<'_> {
        PpToken {
            kind: self.kind,
            name: self.name,
            spelling: self.spelling.bytes(),
            begin: self.begin,
            end: self.end,
            punctuator: self.punctuator,
        }
    }
}
