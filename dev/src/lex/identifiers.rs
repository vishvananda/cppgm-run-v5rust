// Translated from lex/identifier_table.{h,cpp}. See NOTICE.
//! Translation-unit name identity; spellings are interned once on lexer entry.
use crate::support::hash::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NameId(u32);

impl NameId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
    pub(crate) fn bits(self) -> u32 {
        self.0
    }
}

#[derive(Default)]
pub(crate) struct Identifiers {
    index: HashMap<Rc<[u8]>, NameId>,
    spellings: Vec<Rc<[u8]>>,
}

impl Identifiers {
    pub(crate) fn find(&self, text: &[u8]) -> Option<NameId> {
        self.index.get(text).copied()
    }
    #[cfg(feature = "telemetry")]
    pub(crate) fn len(&self) -> usize {
        self.spellings.len()
    }
    pub(crate) fn intern(&mut self, text: &[u8]) -> Result<NameId, &'static str> {
        if let Some(&id) = self.index.get(text) {
            return Ok(id);
        }
        let id = NameId(
            u32::try_from(self.spellings.len())
                .ok()
                .filter(|&id| id < 0x7fff_ffff)
                .ok_or("too many identifiers in one translation unit")?,
        );
        let spelling: Rc<[u8]> = text.into();
        self.index.insert(spelling.clone(), id);
        self.spellings.push(spelling);
        Ok(id)
    }
    pub(crate) fn spelling(&self, id: NameId) -> Rc<[u8]> {
        self.spellings[id.index()].clone()
    }
    pub(crate) fn bytes(&self, id: NameId) -> &[u8] {
        &self.spellings[id.index()]
    }
}
