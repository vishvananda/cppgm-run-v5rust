// Translated from lex/identifier_table.{h,cpp}. See NOTICE.
//! Translation-unit name identity; spellings are interned once on lexer entry.
use crate::support::hash::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NameId(u32);

impl NameId {
    #[inline]
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
    #[inline]
    #[must_use]
    pub fn bits(self) -> u32 {
        self.0
    }
}

#[derive(Default)]
pub struct Identifiers {
    index: HashMap<Rc<[u8]>, NameId>,
    spellings: Vec<Rc<[u8]>>,
}

impl Identifiers {
    #[inline]
    #[must_use]
    pub fn find(&self, text: &[u8]) -> Option<NameId> {
        self.index.get(text).copied()
    }
    #[cfg(feature = "telemetry")]
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.spellings.len()
    }
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn intern(&mut self, text: &[u8]) -> Result<NameId, &'static str> {
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
    #[inline]
    #[must_use]
    pub fn spelling(&self, id: NameId) -> Rc<[u8]> {
        self.spellings[id.index()].clone()
    }
    #[inline]
    #[must_use]
    pub fn bytes(&self, id: NameId) -> &[u8] {
        &self.spellings[id.index()]
    }
}
