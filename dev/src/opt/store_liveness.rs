//! Private-slot write removal and store sinking from `memory_forwarding.cpp`. See NOTICE.
use super::{
    alias_facts::{self as facts, Address},
    memory_contents::bytes,
    memory_forward::MemoryPass,
};
use crate::lowir::{
    Error,
    model::{BlockId, InstructionKind as I, SlotId, Value},
    vocabulary::Type,
};

impl MemoryPass<'_, '_> {
    pub(super) fn drop_unread_writes(&mut self) -> Result<(), Error> {
        self.addresses
            .values
            .resize(self.context.body.temps().len(), Address::default());
        let mut unread = self.addresses.escaped.iter().filter(|&&b| !b).count();
        if unread == 0 {
            return Ok(());
        }
        let mut read = vec![false; self.addresses.escaped.len()];
        'blocks: for &block in &self.context.body.layout {
            for &id in &self.context.body.block(block).instructions {
                if unread == 0 {
                    break 'blocks;
                }
                let address = match self.context.body.instruction(id).kind {
                    I::Load(_, _, address) | I::CopyObject(_, _, address, _) => Some(address),
                    _ => None,
                };
                if let Some(address) = address
                    && let Some(Value::Slot(slot)) = self.at(address).base
                    && !read[slot.index()]
                {
                    read[slot.index()] = true;
                    if !self.addresses.escaped[slot.index()] {
                        unread -= 1;
                    }
                }
            }
        }
        if unread == 0 {
            return Ok(());
        }
        for block in self.context.body.layout.clone() {
            let mut kept = Vec::new();
            for id in self.context.body.block(block).instructions.clone() {
                let kind = &self.context.body.instruction(id).kind;
                let address = match *kind {
                    I::Store(flags, _, _, address) if !flags.volatile() => Some(address),
                    I::CopyObject(_, _, _, address) | I::ZeroInit(_, _, address) => Some(address),
                    _ => None,
                };
                let remove=address.is_some_and(|address|matches!(self.at(address).base,Some(Value::Slot(slot)) if !self.addresses.escaped[slot.index()]&&!read[slot.index()]));
                if remove {
                    self.removed_store()?;
                } else {
                    kept.push(id);
                }
            }
            self.context.body.replace_links(block, kept)?;
        }
        Ok(())
    }
    fn may_read_slot(&self, kind: &I, slot: SlotId, size: u64) -> bool {
        if matches!(kind, I::Call(_)) || facts::eh(kind) {
            return true;
        }
        if matches!(kind, I::Addr(_) | I::Index(..) | I::Copy(Type::Ptr, _)) {
            return false;
        }
        let own = Address {
            base: Some(Value::Slot(slot)),
            offset: 0,
        };
        let reads = |address, width| {
            let at = self.at(address);
            matches!(at.base,Some(Value::Slot(s)) if s==slot)
                || !matches!(at.base, Some(Value::Slot(_) | Value::Symbol(_)))
                    && width <= size
                    && self.addresses.may_alias(at, width, own, size)
        };
        if let I::Load(flags, ty, address) = *kind {
            return flags.volatile() || reads(address, bytes(ty));
        }
        if let I::CopyObject(width, _, source, _) = *kind
            && reads(source, width)
        {
            return true;
        }
        let mut named = false;
        let mut position = 0;
        self.context.body.visit_values(kind, |value| {
            let at = position;
            position += 1;
            let address_use = matches!(kind, I::Load(..)) && at == 0
                || matches!(kind, I::Store(..)) && at == 1
                || matches!(kind, I::CopyObject(..)) && at <= 1
                || matches!(kind, I::ZeroInit(..)) && at == 0;
            if !address_use {
                named |= self.at(value).base == Some(Value::Slot(slot));
            }
        });
        named
    }
    fn write_target(kind: &I) -> Option<(Value, u64)> {
        match *kind {
            I::Store(_, ty, _, address) => Some((address, bytes(ty))),
            I::CopyObject(size, _, _, address) | I::ZeroInit(size, _, address) => {
                Some((address, size))
            }
            _ => None,
        }
    }
    fn may_write_slot(&self, kind: &I, slot: SlotId, size: u64) -> bool {
        Self::write_target(kind).is_some_and(|(target, width)| {
            self.addresses.may_alias(
                self.at(target),
                width,
                Address {
                    base: Some(Value::Slot(slot)),
                    offset: 0,
                },
                size,
            )
        })
    }
    fn overwrites_slot(&self, kind: &I, slot: SlotId, size: u64) -> bool {
        Self::write_target(kind).is_some_and(|(target, width)| {
            self.at(target)
                == Address {
                    base: Some(Value::Slot(slot)),
                    offset: 0,
                }
                && width >= size
        })
    }
    fn slot_dead_from(&self, start: BlockId, slot: SlotId, size: u64) -> Result<bool, Error> {
        let mut work = vec![start];
        let mut seen = vec![start];
        while let Some(block) = work.pop() {
            if self.context.cfg()?.handler(block) {
                return Ok(false);
            }
            let mut overwritten = false;
            for &id in &self.context.body.block(block).instructions {
                let kind = &self.context.body.instruction(id).kind;
                if self.may_read_slot(kind, slot, size) {
                    return Ok(false);
                }
                if self.overwrites_slot(kind, slot, size) {
                    overwritten = true;
                    break;
                }
            }
            if overwritten {
                continue;
            }
            for &next in self.context.cfg()?.successors(block) {
                if seen.contains(&next) {
                    continue;
                }
                if seen.len() >= 64 {
                    return Ok(false);
                }
                seen.push(next);
                work.push(next);
            }
        }
        Ok(true)
    }
    pub(super) fn sink_stores(&mut self) -> Result<(), Error> {
        if self.context.level < 2 {
            return Ok(());
        }
        for block in self.context.body.layout.clone() {
            let ids = &self.context.body.block(block).instructions;
            let Some(&last) = ids.last() else {
                continue;
            };
            let cfg = self.context.cfg()?;
            if !matches!(self.context.body.instruction(last).kind, I::Branch(..))
                || cfg.successors(block).len() != 2
            {
                continue;
            }
            let successors = [cfg.successors(block)[0], cfg.successors(block)[1]];
            if successors[0] == successors[1] {
                continue;
            }
            let mut store = None;
            for &id in ids {
                let kind = &self.context.body.instruction(id).kind;
                if let Some((_, slot)) = store
                    && {
                        let size = bytes(self.context.body.slot(slot).ty);
                        self.may_read_slot(kind, slot, size)
                            || self.may_write_slot(kind, slot, size)
                    }
                {
                    store = None;
                }
                if let I::Store(flags, _, _, Value::Slot(slot)) = *kind
                    && !flags.volatile()
                {
                    store = Some((id, slot));
                }
            }
            let Some((store, slot)) = store else {
                continue;
            };
            let size = bytes(self.context.body.slot(slot).ty);
            if !self.overwrites_slot(&self.context.body.instruction(store).kind, slot, size) {
                continue;
            }
            let live = if self.slot_dead_from(successors[1], slot, size)? {
                Some(successors[0])
            } else if self.slot_dead_from(successors[0], slot, size)? {
                Some(successors[1])
            } else {
                None
            };
            let Some(live) = live else {
                continue;
            };
            if self.context.cfg()?.predecessors(live).len() != 1
                || self.context.cfg()?.handler(live)
            {
                continue;
            }
            let mut head = self.context.body.block(block).instructions.clone();
            head.retain(|&id| id != store);
            self.context.body.replace_links(block, head)?;
            let mut target = self.context.body.block(live).instructions.clone();
            let at = target
                .iter()
                .take_while(|&&id| matches!(self.context.body.instruction(id).kind, I::Phi(..)))
                .count();
            target.insert(at, store);
            self.context.body.replace_links(live, target)?;
            self.removed_store()?;
        }
        Ok(())
    }
}
