//! Semi-pruned slot-to-SSA promotion from `opt/slot_promotion.cpp`. See NOTICE.
use super::{
    definition_order, numeric_fold as fold,
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    literals::{Constant, FloatKind},
    model::{BlockId, InstId, Instruction, InstructionKind as I, LinkEditor, SlotId, Value},
    vocabulary::{Convert, Type},
};

pub(super) fn phi_type(ty: Type) -> bool {
    !matches!(
        ty,
        Type::Void | Type::I128 | Type::F80 | Type::Object { .. }
    )
}
fn bits(ty: Type) -> u32 {
    if ty == Type::Ptr { 64 } else { ty.bits() }
}
fn stored_type(body: &crate::lowir::model::Body, ty: Type, v: Value) -> Type {
    if let Value::Temp(t) = v {
        body.temp(t).ty.unwrap_or(Type::Void)
    } else {
        ty
    }
}
fn signed(ty: Type) -> bool {
    matches!(
        ty,
        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128
    )
}
fn twins(a: Type, b: Type) -> bool {
    a != b && a.bits() != 0 && a != Type::I1 && b != Type::I1 && a.bits() == b.bits()
}
fn store_conversion(from: Type, held: Type, value: Value) -> I {
    let source = bits(from);
    let target = bits(held);
    if source == target {
        return I::Copy(held, value);
    }
    let operation = if source > target {
        Convert::Truncate
    } else if signed(from) {
        Convert::SignExtend
    } else {
        Convert::ZeroExtend
    };
    I::Convert(operation, held, from, value)
}
fn resolve(mut value: Value, replacements: &[Option<Value>]) -> Result<Value, Error> {
    let mut depth = 0;
    while let Value::Temp(temp) = value {
        let Some(next) = replacements.get(temp.index()).copied().flatten() else {
            break;
        };
        value = next;
        depth += 1;
        if depth > replacements.len() {
            return Err(Error::Construction("cyclic promoted value".into()));
        }
    }
    Ok(value)
}
struct PendingPhi {
    block: BlockId,
    candidate: usize,
    id: InstId,
    ty: Type,
    pairs: Vec<(BlockId, Option<Value>)>,
}
#[derive(Default)]
pub(super) struct Scratch {
    candidate: Vec<Option<usize>>,
    slots: Vec<SlotId>,
    accessed: Vec<bool>,
    frontier: Vec<Vec<BlockId>>,
    phis: Vec<PendingPhi>,
    by_block: Vec<Vec<usize>>,
    successor_slots: Vec<Vec<(BlockId, usize)>>,
    current: Vec<Option<Value>>,
    undo: Vec<(usize, Option<Value>)>,
    replacements: Vec<Option<Value>>,
    remove: Vec<bool>,
}
struct Promotion<'a, 'p> {
    context: &'a mut Context<'p>,
    candidate: Vec<Option<usize>>,
    slots: Vec<SlotId>,
    accessed: Vec<bool>,
    frontier: Vec<Vec<BlockId>>,
    phis: Vec<PendingPhi>,
    by_block: Vec<Vec<usize>>,
    successor_slots: Vec<Vec<(BlockId, usize)>>,
    current: Vec<Option<Value>>,
    undo: Vec<(usize, Option<Value>)>,
    replacements: Vec<Option<Value>>,
    remove: Vec<bool>,
    kept: bool,
    links: LinkEditor,
}
impl<'a, 'p> Promotion<'a, 'p> {
    fn new(context: &'a mut Context<'p>) -> Result<Self, Error> {
        let links = LinkEditor::new(context.body)?;
        let mut scratch = std::mem::take(&mut context.promotion_scratch);
        scratch.slots.clear();
        scratch.phis.clear();
        scratch.undo.clear();
        Ok(Self {
            context,
            candidate: scratch.candidate,
            slots: scratch.slots,
            accessed: scratch.accessed,
            frontier: scratch.frontier,
            phis: scratch.phis,
            by_block: scratch.by_block,
            successor_slots: scratch.successor_slots,
            current: scratch.current,
            undo: scratch.undo,
            replacements: scratch.replacements,
            remove: scratch.remove,
            kept: false,
            links,
        })
    }
    fn recycle(&mut self) {
        let scratch = &mut self.context.promotion_scratch;
        std::mem::swap(&mut self.candidate, &mut scratch.candidate);
        std::mem::swap(&mut self.slots, &mut scratch.slots);
        std::mem::swap(&mut self.accessed, &mut scratch.accessed);
        std::mem::swap(&mut self.frontier, &mut scratch.frontier);
        std::mem::swap(&mut self.phis, &mut scratch.phis);
        std::mem::swap(&mut self.by_block, &mut scratch.by_block);
        std::mem::swap(&mut self.successor_slots, &mut scratch.successor_slots);
        std::mem::swap(&mut self.current, &mut scratch.current);
        std::mem::swap(&mut self.undo, &mut scratch.undo);
        std::mem::swap(&mut self.replacements, &mut scratch.replacements);
        std::mem::swap(&mut self.remove, &mut scratch.remove);
    }
    fn classify(&mut self) -> Result<(), Error> {
        let body = &*self.context.body;
        let mut ok = body
            .slots()
            .iter()
            .map(|s| phi_type(s.ty))
            .collect::<Vec<_>>();
        self.accessed.resize(ok.len(), false);
        self.accessed.fill(false);
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                let kind = &body.instruction(id).kind;
                let mut position = 0;
                body.visit_values(kind, |v| {
                    let at = position;
                    position += 1;
                    let Value::Slot(slot) = v else {
                        return;
                    };
                    let index = slot.index();
                    self.accessed[index] = true;
                    let ty = body.slot(slot).ty;
                    let allowed = match *kind {
                        I::Load(flags, t, _) => {
                            at == 0 && !flags.volatile() && (t == ty || twins(t, ty))
                        }
                        I::Store(flags, t, value, _) => {
                            let from = stored_type(body, t, value);
                            at == 1
                                && !flags.volatile()
                                && (t == ty || twins(t, ty))
                                && !matches!(value, Value::Slot(_))
                                && (from == t || bits(from) != 0 && bits(t) != 0)
                        }
                        _ => false,
                    };
                    if !allowed {
                        ok[index] = false;
                    }
                });
            }
        }
        if ok.iter().zip(&self.accessed).any(|(&a, &b)| a && b) {
            super::handler_liveness::exclude(self.context, &mut ok)?;
        }
        self.candidate.resize(ok.len(), None);
        self.candidate.fill(None);
        for (index, allowed) in ok.into_iter().enumerate() {
            if allowed {
                let slot = self.context.body.slot_id(index).ok_or(Error::Limit)?;
                self.candidate[index] = Some(self.slots.len());
                self.slots.push(slot);
            }
        }
        Ok(())
    }
    fn any_accessed(&self) -> bool {
        self.slots
            .iter()
            .any(|s| self.candidate[s.index()].is_some() && self.accessed[s.index()])
    }
    fn frontiers(&mut self) -> Result<(), Error> {
        let cfg = self.context.cfg()?;
        let mut found = Vec::new();
        let mut last = vec![None; self.context.body.blocks().len()];
        for &block in cfg.reverse_postorder() {
            if cfg.predecessors(block).len() < 2 {
                continue;
            }
            for &pred in cfg.predecessors(block) {
                if !cfg.reachable(pred) {
                    continue;
                }
                let mut runner = Some(pred);
                while runner.is_some() && runner != cfg.dominator(block) {
                    let at = runner.ok_or(Error::Limit)?;
                    if last[at.index()] != Some(block) {
                        last[at.index()] = Some(block);
                        found.push((at, block));
                    }
                    runner = cfg.dominator(at);
                }
            }
        }
        self.frontier.resize_with(last.len(), Vec::new);
        for row in &mut self.frontier {
            row.clear();
        }
        for (at, block) in found {
            self.frontier[at.index()].push(block);
        }
        Ok(())
    }
    fn placements(&mut self) -> Result<(), Error> {
        let count = self.slots.len();
        let blocks = self.context.body.blocks().len();
        let mut budget = 4096_u64;
        for &block in &self.context.body.layout {
            for &id in &self.context.body.block(block).instructions {
                if !matches!(self.context.body.instruction(id).kind, I::Phi(..)) {
                    budget = budget.checked_add(4).ok_or(Error::Limit)?;
                }
            }
        }
        let mut stores = vec![Vec::new(); count];
        let mut exposed = vec![false; count];
        let mut stored_in = vec![None; count];
        for &block in self.context.cfg()?.reverse_postorder() {
            for &id in &self.context.body.block(block).instructions {
                match self.context.body.instruction(id).kind {
                    I::Load(_, _, Value::Slot(slot)) => {
                        if let Some(c) = self.candidate[slot.index()]
                            && stored_in[c] != Some(block)
                        {
                            exposed[c] = true;
                        }
                    }
                    I::Store(_, _, _, Value::Slot(slot)) => {
                        let Some(c) = self.candidate[slot.index()] else {
                            continue;
                        };
                        if stored_in[c] != Some(block) {
                            stores[c].push(block);
                        }
                        stored_in[c] = Some(block);
                    }
                    _ => (),
                }
            }
        }
        let mut placed = vec![None; blocks];
        let mut queued = vec![None; blocks];
        let mut used = 0_u64;
        self.by_block.resize_with(blocks, Vec::new);
        for row in &mut self.by_block {
            row.clear();
        }
        for c in 0..count {
            if !exposed[c] {
                continue;
            }
            let mut wanted = Vec::new();
            let mut work = std::mem::take(&mut stores[c]);
            for &block in &work {
                queued[block.index()] = Some(c);
            }
            let mut rejected = false;
            while let Some(block) = work.pop() {
                for &frontier in &self.frontier[block.index()] {
                    if placed[frontier.index()] == Some(c) {
                        continue;
                    }
                    if self.context.cfg()?.handler(frontier) {
                        rejected = true;
                        break;
                    }
                    placed[frontier.index()] = Some(c);
                    wanted.push(frontier);
                    if queued[frontier.index()] != Some(c) {
                        queued[frontier.index()] = Some(c);
                        work.push(frontier);
                    }
                }
                if rejected {
                    break;
                }
            }
            let mut inputs = 0_u64;
            for &block in &wanted {
                inputs = inputs
                    .checked_add(
                        u64::try_from(self.context.cfg()?.predecessors(block).len())
                            .map_err(|_| Error::Limit)?,
                    )
                    .ok_or(Error::Limit)?;
            }
            if rejected || used.checked_add(inputs).ok_or(Error::Limit)? > budget {
                self.candidate[self.slots[c].index()] = None;
                continue;
            }
            used += inputs;
            let ty = self.context.body.slot(self.slots[c]).ty;
            for block in wanted {
                let pairs = self
                    .context
                    .cfg()?
                    .predecessors(block)
                    .iter()
                    .map(|&p| (p, None))
                    .collect();
                let result = self.context.body.generated_temp(ty)?;
                let id = self.context.body.append(
                    block,
                    Instruction::new(Some(result), I::Phi(ty, Vec::new()), 0),
                )?;
                self.by_block[block.index()].push(self.phis.len());
                self.phis.push(PendingPhi {
                    block,
                    candidate: c,
                    id,
                    ty,
                    pairs,
                });
                self.context.phis_inserted = self
                    .context
                    .phis_inserted
                    .checked_add(1)
                    .ok_or(Error::Limit)?;
            }
        }
        Ok(())
    }
    fn zero(&mut self, ty: Type) -> Result<Value, Error> {
        let constant =
            FloatKind::from_type(ty).map_or(Constant::Integer(0), |kind| Constant::Float(kind, 0));
        self.context.body.constant(constant)
    }
    fn inserted(
        &mut self,
        block: BlockId,
        before: InstId,
        source: InstId,
        kind: I,
        ty: Type,
    ) -> Result<Value, Error> {
        let result = self.context.body.generated_temp(ty)?;
        let id = self
            .context
            .body
            .append(block, Instruction::new(Some(result), kind, 0))?;
        self.context.body.inherit_location(id, source);
        self.links
            .insert_after(block, self.links.previous(before), id)?;
        self.remove
            .resize(self.context.body.instructions().len(), false);
        self.replacements
            .resize(self.context.body.temps().len(), None);
        Ok(Value::Temp(result))
    }
    fn rename_block(&mut self, block: BlockId) -> Result<(), Error> {
        for &index in &self.by_block[block.index()] {
            let phi = &self.phis[index];
            if self.candidate[self.slots[phi.candidate].index()].is_none() {
                continue;
            }
            self.undo.push((phi.candidate, self.current[phi.candidate]));
            self.current[phi.candidate] = self
                .context
                .body
                .instruction(phi.id)
                .result
                .map(Value::Temp);
        }
        let mut cursor = self.links.first(block);
        while let Some(id) = cursor {
            let next = self.links.next(id);
            let instruction = self.context.body.instruction(id).clone();
            match instruction.kind {
                I::Load(_, ty, Value::Slot(slot)) if self.candidate[slot.index()].is_some() => {
                    let c = self.candidate[slot.index()].ok_or(Error::Limit)?;
                    if self.current[c].is_none() {
                        self.undo.push((c, None));
                        self.current[c] = Some(self.zero(ty)?);
                    }
                    let value = self.current[c].ok_or(Error::Limit)?;
                    let result = instruction.result.ok_or(Error::Limit)?;
                    self.replacements[result.index()] = Some(value);
                    if ty != self.context.body.slot(slot).ty && matches!(value, Value::Temp(_)) {
                        let value = self.inserted(block, id, id, I::Copy(ty, value), ty)?;
                        self.replacements[result.index()] = Some(value);
                    }
                    self.remove[id.index()] = true;
                }
                I::Store(_, ty, value, Value::Slot(slot))
                    if self.candidate[slot.index()].is_some() =>
                {
                    let c = self.candidate[slot.index()].ok_or(Error::Limit)?;
                    self.undo.push((c, self.current[c]));
                    self.current[c] = Some(value);
                    let from = stored_type(self.context.body, ty, value);
                    let held = self.context.body.slot(slot).ty;
                    if from != held {
                        let kind = store_conversion(from, held, value);
                        self.current[c] = Some(self.inserted(block, id, id, kind, held)?);
                    }
                    self.remove[id.index()] = true;
                }
                _ => (),
            }
            cursor = next;
        }
        for (successor, pair) in self.successor_slots[block.index()].clone() {
            for index in self.by_block[successor.index()].clone() {
                let c = self.phis[index].candidate;
                if self.candidate[self.slots[c].index()].is_none() {
                    continue;
                }
                let ty = self.phis[index].ty;
                let value = if let Some(value) = self.current[c] {
                    value
                } else {
                    self.zero(ty)?
                };
                let value = if let Some(constant) = fold::literal(self.context.body, value)
                    && ty.float_rank() != 0
                    && matches!(constant, Constant::Integer(_))
                {
                    self.context
                        .body
                        .constant(fold::phi_input(ty, constant).ok_or(Error::Limit)?)?
                } else {
                    value
                };
                if let Some((pred, input)) = self.phis[index].pairs.get_mut(pair)
                    && *pred == block
                {
                    *input = Some(value);
                }
            }
        }
        Ok(())
    }
    fn rename(&mut self) -> Result<(), Error> {
        // CFG predecessors are filled in source BlockId order, then successor
        // order. Each ordinary edge names its phi input in constant time.
        let mut sources = self.context.body.layout.clone();
        sources.sort_unstable_by_key(|b| b.index());
        let blocks = self.context.body.blocks().len();
        let mut next = vec![0_usize; blocks];
        self.successor_slots.resize_with(blocks, Vec::new);
        for row in &mut self.successor_slots {
            row.clear();
        }
        for source in sources {
            for &to in self.context.cfg()?.successors(source) {
                self.successor_slots[source.index()].push((to, next[to.index()]));
                next[to.index()] += 1;
            }
        }
        self.current.resize(self.slots.len(), None);
        self.current.fill(None);
        self.replacements
            .resize(self.context.body.temps().len(), None);
        self.replacements.fill(None);
        self.remove
            .resize(self.context.body.instructions().len(), false);
        self.remove.fill(false);
        let entry = *self
            .context
            .cfg()?
            .reverse_postorder()
            .first()
            .ok_or(Error::Limit)?;
        let mut stack = vec![(entry, 0_usize, 0_usize)];
        self.rename_block(entry)?;
        while let Some(&(block, child, mark)) = stack.last() {
            if let Some(&next) = self.context.cfg()?.children(block).get(child) {
                stack.last_mut().ok_or(Error::Limit)?.1 += 1;
                stack.push((next, 0, self.undo.len()));
                self.rename_block(next)?;
            } else {
                for (c, value) in self.undo.drain(mark..).rev() {
                    self.current[c] = value;
                }
                stack.pop();
            }
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<(), Error> {
        let mut phis = std::mem::take(&mut self.phis);
        let mut ordered = (0..phis.len()).collect::<Vec<_>>();
        ordered.sort_by_key(|&i| phis[i].block.index());
        for index in ordered {
            let phi = &mut phis[index];
            if self.candidate[self.slots[phi.candidate].index()].is_none() {
                continue;
            }
            let mut pairs = Vec::new();
            for &(pred, value) in &phi.pairs {
                pairs.push((
                    pred,
                    if let Some(value) = value {
                        value
                    } else {
                        self.zero(phi.ty)?
                    },
                ));
            }
            self.context
                .body
                .replace_kind(phi.id, I::Phi(phi.ty, pairs));
            self.links.insert_after(phi.block, None, phi.id)?;
        }
        for block in self.context.body.layout.clone() {
            let mut next = self.links.first(block);
            while let Some(id) = next {
                next = self.links.next(id);
                if self.remove.get(id.index()) == Some(&true) {
                    self.links.unlink(block, id)?;
                    continue;
                }
                let replacements = &self.replacements;
                self.context
                    .body
                    .rewrite_values(id, |value| resolve(value, replacements))?;
            }
        }
        Ok(())
    }
    fn run(&mut self) -> Result<bool, Error> {
        if self.context.body.slots().is_empty() {
            return Ok(false);
        }
        self.context.ensure_cfg()?;
        if self.context.cfg()?.reverse_postorder().is_empty() {
            return Ok(false);
        }
        self.classify()?;
        if self.slots.is_empty() {
            return Ok(false);
        }
        if !self.any_accessed() {
            self.kept = true;
            self.context.slots_promoted = self
                .context
                .slots_promoted
                .checked_add(u64::try_from(self.slots.len()).map_err(|_| Error::Limit)?)
                .ok_or(Error::Limit)?;
            return Ok(true);
        }
        self.frontiers()?;
        self.placements()?;
        let count = self
            .slots
            .iter()
            .filter(|s| self.candidate[s.index()].is_some())
            .count();
        if count == 0 {
            return Ok(false);
        }
        if self.any_accessed() {
            self.rename()?;
            self.finish()?;
        } else {
            self.kept = true;
        }
        self.context.slots_promoted = self
            .context
            .slots_promoted
            .checked_add(u64::try_from(count).map_err(|_| Error::Limit)?)
            .ok_or(Error::Limit)?;
        Ok(true)
    }
}
pub(super) fn promote(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut promotion = Promotion::new(context)?;
    let result = promotion.run();
    promotion.recycle();
    let ran = result?;
    let kept = promotion.kept;
    promotion.links.commit(promotion.context.body)?;
    if !ran {
        return Ok(false);
    }
    let settled = kept && context.running_pass() == Some(Pass::PromoteSlots);
    if settled && context.body_is_ordered() {
        context.keep_body(Pass::PromoteSlots, false);
        return Ok(true);
    }
    if !definition_order::restore(context)? && kept {
        context.keep_body(Pass::PromoteSlots, settled);
    }
    context.body.recount_uses()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frontier_phis_keep_predecessor_order_and_reverse_slot_placement() {
        let (out, facts) = super::super::pass_view(
            "function @f(%c : i64)->i64 {slot $a : i64 slot $b : i64 block ^entry:branch %c, ^a, ^b block ^a:store i64 1, $a jump ^join block ^b:store i64 0, $a store i64 1, $b jump ^join block ^join:%x = load i64 $a %y = load i64 $b return i64 %x}",
            1,
            Pass::PromoteSlots,
        );
        assert_eq!(facts, (true, 0, 2, 2));
        assert!(out.contains(
            "%t1 = phi i64 [^a: 0, ^b: 1]\n    %t2 = phi i64 [^a: 1, ^b: 0]\n    return i64 %t2"
        ));
        assert!(!out.contains("slot $"));
    }
    #[test]
    fn widening_stores_preserve_the_values_register_signedness() {
        let (out, facts) = super::super::pass_view(
            "function @f(%v : i8)->i64 {slot $a : i64 block ^entry:store i64 %v, $a %x = load i64 $a return i64 %x}",
            1,
            Pass::PromoteSlots,
        );
        assert_eq!(facts, (true, 0, 1, 0));
        assert!(out.contains("convert sext i64 i8 %v"));
    }
    #[test]
    fn handler_reads_keep_active_region_stores_in_memory() {
        let (out, facts) = super::super::pass_view(
            "function @f()->i64 {slot $a : i64 block ^entry:store i64 7, $a eh_try ^handler store i64 9, $a eh_end return i64 0 block ^handler:%x = load i64 $a return i64 %x}",
            1,
            Pass::PromoteSlots,
        );
        assert_eq!(facts, (false, 0, 0, 0));
        assert!(out.contains("load i64 $a"));
    }
    #[test]
    fn a_handler_that_kills_the_slot_before_reading_allows_promotion() {
        let (out, facts) = super::super::pass_view(
            "function @f()->i64 {slot $a : i64 block ^entry:eh_try ^handler store i64 9, $a eh_end return i64 0 block ^handler:store i64 11, $a %x = load i64 $a return i64 %x}",
            1,
            Pass::PromoteSlots,
        );
        assert_eq!(facts, (true, 0, 1, 0));
        assert!(!out.contains("load "));
        assert!(out.contains("return i64 11"));
    }
}
