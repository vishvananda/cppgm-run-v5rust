//! Temporary copy forwarding and slot contents from `opt/coalescing.cpp`.
//! See NOTICE.
use super::{
    alias_facts,
    coalescing::{Copies, Rooted, rooted},
    memory_contents::{bytes, contents},
};
use crate::lowir::{
    Error,
    model::{BlockId, InstId, InstructionKind as I, SlotId, Value},
    vocabulary::Type,
};

impl Copies<'_, '_> {
    fn forwardable(&self, source: Value, destination: Value, written: bool) -> bool {
        if matches!(source, Value::Temp(t) if matches!(self.context.body.temp(t).ty, Some(Type::Object { .. })))
        {
            return true;
        }
        if written {
            return false;
        }
        let to = rooted(self.context.body, destination);
        if matches!(to, Some(Rooted { root: Value::Slot(s), .. }) if !self.escaped[s.index()]) {
            return true;
        }
        rooted(self.context.body, source)
            .zip(to)
            .is_some_and(|(a, b)| self.disjoint(a.root, b.root))
    }
    pub(super) fn forward(&mut self, block: BlockId) -> Result<bool, Error> {
        let mut first: Option<(InstId, SlotId)> = None;
        let mut written = false;
        let mut changed = false;
        let mut cursor = self.links.first(block);
        while let Some(id) = cursor {
            cursor = self.links.next(id);
            let kind = self.context.body.instruction(id).kind.clone();
            if let Some((previous, slot)) = first {
                let I::CopyObject(size, _, source, _) =
                    self.context.body.instruction(previous).kind
                else {
                    return Err(Error::Limit);
                };
                if let I::CopyObject(count, align, from, to) = kind
                    && count == size
                    && self.slot_uses[slot.index()] == 2
                    && matches!(rooted(self.context.body,from),Some(Rooted{root:Value::Slot(s),offset:0}) if s == slot)
                    && self.forwardable(source, to, written)
                {
                    self.context
                        .body
                        .replace_kind(id, I::CopyObject(count, align, source, to));
                    self.links.unlink(block, previous)?;
                    self.removed_store()?;
                    first = None;
                    changed = true;
                    continue;
                }
                if let I::Return(ty, Some(Value::Slot(s))) = kind
                    && s == slot
                    && !written
                    && let Some(Rooted {
                        root: Value::Slot(origin),
                        offset: 0,
                    }) = rooted(self.context.body, source)
                    && bytes(self.context.body.slot(origin).ty) == size
                {
                    self.context
                        .body
                        .replace_kind(id, I::Return(ty, Some(Value::Slot(origin))));
                    self.links.unlink(block, previous)?;
                    self.removed_store()?;
                    first = None;
                    changed = true;
                    continue;
                }
            }
            if let I::CopyObject(size, _, _, to) = kind
                && let Some(Rooted {
                    root: Value::Slot(slot),
                    offset: 0,
                }) = rooted(self.context.body, to)
                && size == bytes(self.context.body.slot(slot).ty)
            {
                first = Some((id, slot));
                written = false;
                continue;
            }
            if first.is_some()
                && (matches!(kind, I::Store(..) | I::CopyObject(..) | I::ZeroInit(..))
                    || alias_facts::writes(self.context.program, &kind))
            {
                written = true;
            }
        }
        Ok(changed)
    }
    fn zero_candidates(&self) -> Vec<bool> {
        let mut ok = self
            .context
            .body
            .slots()
            .iter()
            .map(|s| matches!(s.ty,Type::Object{bytes,..} if bytes<=64))
            .collect::<Vec<_>>();
        for &block in &self.context.body.layout {
            for id in self.links.ids(block) {
                let inst = self.context.body.instruction(id);
                let kind = &inst.kind;
                if matches!(kind, I::Addr(_))
                    || matches!(kind, I::Index(..) | I::Copy(..))
                        && inst.result.is_some_and(|t| self.uses[t.index()] == 0)
                {
                    continue;
                }
                let mut position = 0;
                self.context.body.visit_values(kind, |v| {
                    let at = position;
                    position += 1;
                    let Some(Rooted {
                        root: Value::Slot(slot),
                        ..
                    }) = rooted(self.context.body, v)
                    else {
                        return;
                    };
                    let allowed = match kind {
                        I::Load(f, _, _) => at == 0 && !f.volatile(),
                        I::Store(f, ty, Value::Integer(zero), _) => {
                            at == 1
                                && !f.volatile()
                                && (ty.bits() != 0 || *ty == Type::Ptr)
                                && self.context.body.literal(*zero) == 0
                        }
                        I::CopyObject(..) | I::ZeroInit(..) | I::Index(..) => at == 0,
                        I::Return(..) => matches!(v, Value::Slot(_)),
                        _ => false,
                    };
                    if !allowed {
                        ok[slot.index()] = false;
                    }
                });
            }
        }
        ok
    }
    pub(super) fn zero_slots(&mut self) -> Result<bool, Error> {
        let ok = self.zero_candidates();
        let mut changed = false;
        for block in self.context.body.layout.clone() {
            let mut done = vec![false; ok.len()];
            let mut cursor = self.links.first(block);
            while let Some(id) = cursor {
                cursor = self.links.next(id);
                let I::Store(_, _, _, pointer) = self.context.body.instruction(id).kind else {
                    continue;
                };
                let Some(Rooted {
                    root: Value::Slot(slot),
                    ..
                }) = rooted(self.context.body, pointer)
                else {
                    continue;
                };
                if !ok[slot.index()] {
                    continue;
                }
                if done[slot.index()] {
                    self.links.unlink(block, id)?;
                    self.removed_store()?;
                } else {
                    done[slot.index()] = true;
                    let Type::Object { bytes, align } = self.context.body.slot(slot).ty else {
                        return Err(Error::Limit);
                    };
                    self.context
                        .body
                        .replace_kind(id, I::ZeroInit(bytes, align, Value::Slot(slot)));
                }
                changed = true;
            }
        }
        Ok(changed)
    }
    fn constant_addresses(&self) -> Vec<Option<SlotId>> {
        let mut derived = vec![None; self.context.body.temps().len()];
        for &block in &self.context.body.layout {
            for id in self.links.ids(block) {
                let inst = self.context.body.instruction(id);
                let Some(result) = inst.result else {
                    continue;
                };
                let slot = match inst.kind {
                    I::Addr(Value::Slot(slot)) => Some(slot),
                    I::Index(_, _, v, _) | I::Copy(Type::Ptr, v) => slot_of(v, &derived),
                    _ => None,
                };
                if let Some(slot) = slot {
                    derived[result.index()] = Some(slot);
                }
            }
        }
        derived
    }
    fn constant_fills(&self) -> (Vec<Option<InstId>>, Vec<bool>) {
        let derived = self.constant_addresses();
        let slots = self.context.body.slots().len();
        let mut fill = vec![None; slots];
        let mut bad = vec![false; slots];
        for &block in &self.context.body.layout {
            for id in self.links.ids(block) {
                let inst = self.context.body.instruction(id);
                let step = inst.result.is_some_and(|t| derived[t.index()].is_some());
                let mut position = 0;
                self.context.body.visit_values(&inst.kind, |v| {
                    let at = position;
                    position += 1;
                    let Some(slot) = slot_of(v, &derived) else {
                        return;
                    };
                    if step && at == 0
                        || matches!(inst.kind,I::Load(f,_,_) if at==0 && !f.volatile())
                        || matches!(inst.kind,I::CopyObject(..) if at==0)
                    {
                        return;
                    }
                    if let I::CopyObject(size, _, Value::Symbol(global), _) = inst.kind
                        && at == 1
                        && fill[slot.index()].is_none()
                        && rooted(self.context.body, v).is_some_and(|r| r.offset == 0)
                        && size == bytes(self.context.body.slot(slot).ty)
                        && contents(self.context.program, global, self.context.unwritten).is_some()
                    {
                        fill[slot.index()] = Some(id);
                        return;
                    }
                    bad[slot.index()] = true;
                });
            }
        }
        (fill, bad)
    }
    pub(super) fn constant_slots(&mut self) -> Result<bool, Error> {
        let (fill, bad) = self.constant_fills();
        let mut replacements = vec![None; fill.len()];
        let mut dead = vec![false; self.context.body.instructions().len()];
        let mut changed = false;
        for (slot, fill) in fill.into_iter().enumerate() {
            let Some(id) = fill.filter(|_| !bad[slot]) else {
                continue;
            };
            let I::CopyObject(_, _, global, _) = self.context.body.instruction(id).kind else {
                return Err(Error::Limit);
            };
            replacements[slot] = Some(global);
            dead[id.index()] = true;
            self.removed_store()?;
            changed = true;
        }
        if !changed {
            return Ok(false);
        }
        // Each replacement maps a slot to a global, so they commute. Publish
        // all substitutions in one traversal rather than scanning once per slot.
        for block in self.context.body.layout.clone() {
            let mut cursor = self.links.first(block);
            while let Some(id) = cursor {
                cursor = self.links.next(id);
                if dead[id.index()] {
                    self.links.unlink(block, id)?;
                    continue;
                }
                self.context.body.rewrite_values(id, |v| {
                    Ok(match v {
                        Value::Slot(slot) => replacements[slot.index()].unwrap_or(v),
                        _ => v,
                    })
                })?;
            }
        }
        Ok(true)
    }
}
fn slot_of(value: Value, derived: &[Option<SlotId>]) -> Option<SlotId> {
    match value {
        Value::Slot(slot) => Some(slot),
        Value::Temp(t) => derived[t.index()],
        _ => None,
    }
}
