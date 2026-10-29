// Translated from lower/symbol_lowering.cpp's locality memo. See NOTICE.
//! Locality results belong to a read-only semantic snapshot, not an analyzer.
use super::{Error, types::TypeId};
use crate::support::hash::{HashMap, HashSet};
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Mode {
    Source,
    Object,
    Private,
    NoLinkage,
}

#[derive(Default)]
pub(super) struct Scratch {
    pub work: Vec<TypeId>,
    pub seen: HashSet<TypeId>,
}

#[derive(Default)]
pub(super) struct Queries {
    reading: Cell<bool>,
    memo: RefCell<HashMap<(TypeId, Mode), bool>>,
    scratch: RefCell<Scratch>,
}
impl Queries {
    pub(super) fn begin_analysis(&mut self) {
        // Resumed semantic demand can publish arguments, closure contexts and
        // linkage facts. End this read snapshot before giving the analyzer
        // mutable access; retain unrelated semantic caches and scratch capacity.
        self.reading.set(false);
        self.memo.get_mut().clear();
    }
    pub(super) fn begin_reading(&self) {
        self.reading.set(true);
    }
    pub(super) fn query(
        &self,
        ty: TypeId,
        mode: Mode,
        action: impl FnOnce(&mut Scratch) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        if self.reading.get()
            && let Some(&known) = self.memo.borrow().get(&(ty, mode))
        {
            return Ok(known);
        }
        // A query may call another locality query. No RefCell borrow spans
        // that call; nested queries receive independent scratch and memos.
        let mut scratch = std::mem::take(&mut *self.scratch.borrow_mut());
        let result = action(&mut scratch);
        scratch.work.clear();
        scratch.seen.clear();
        let mut retained = self.scratch.borrow_mut();
        if scratch.work.capacity() + scratch.seen.capacity()
            >= retained.work.capacity() + retained.seen.capacity()
        {
            *retained = scratch;
        }
        if self.reading.get()
            && let Ok(value) = result
        {
            self.memo.borrow_mut().insert((ty, mode), value);
        }
        result
    }
}
