//! Bounded constant-phi routing and branch cloning from `opt/jump_threading.cpp`.
//! See NOTICE. Wide merges apply predecessor edits in batches.
use super::{
    jump_threading::{Threader, definition, incoming, last, mapped, phis, temp_type},
    numeric_fold as fold,
    pass_context::Pass,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{BlockId, InstId, InstructionKind as I, LinkEditor, TempId, Value},
};
struct Router {
    scratch: Vec<Option<Value>>,
    local: Vec<u32>,
    stamp: Vec<Option<BlockId>>,
    use_dom: Vec<Option<BlockId>>,
    slots: Vec<Option<usize>>,
    inputs: Vec<Option<Value>>,
    touched: Vec<bool>,
}
impl Router {
    fn new(thread: &Threader<'_, '_>) -> Self {
        let body = &*thread.context.body;
        Self {
            scratch: vec![None; body.temps().len()],
            local: vec![0; body.temps().len()],
            stamp: vec![None; body.temps().len()],
            use_dom: Vec::new(),
            slots: vec![None; body.blocks().len()],
            inputs: Vec::new(),
            touched: vec![false; body.blocks().len()],
        }
    }
    fn evaluate(
        &mut self,
        thread: &mut Threader<'_, '_>,
        block: BlockId,
        inputs: &[Option<Value>],
    ) -> Result<Option<BlockId>, Error> {
        let mut touched = Vec::new();
        let mut input = 0;
        let mut target = None;
        for id in thread.context.body.block(block).instructions.clone() {
            let inst = thread.context.body.instruction(id).clone();
            if matches!(inst.kind, I::Phi(..)) {
                if let Some(value) = inputs.get(input).copied().flatten()
                    && fold::literal(thread.context.body, value).is_some()
                {
                    let result = inst.result.ok_or(Error::Limit)?;
                    self.scratch[result.index()] = Some(value);
                    touched.push(result);
                }
                input += 1;
                continue;
            }
            if let I::Branch(value, yes, no) = inst.kind {
                if let Some(Constant::Integer(bits)) =
                    fold::literal(thread.context.body, mapped(value, &self.scratch))
                {
                    target = Some(if bits != 0 { yes } else { no });
                }
                break;
            }
            let get = |value| fold::literal(thread.context.body, mapped(value, &self.scratch));
            let result = match inst.kind {
                I::Compare(op, ty, a, b) => get(a)
                    .zip(get(b))
                    .and_then(|(a, b)| fold::compare(op, ty, a, b)),
                I::Binary(op, _, ty, a, b) => get(a)
                    .zip(get(b))
                    .and_then(|(a, b)| fold::binary(op, ty, a, b)),
                I::Unary(op, ty, a) => get(a).and_then(|a| fold::unary(op, ty, a)),
                I::Convert(op, to, from, a) => get(a).and_then(|a| fold::convert(op, to, from, a)),
                I::Copy(ty, a) => get(a).and_then(|a| fold::copy(ty, a)),
                _ => None,
            };
            let Some(value) = result else {
                break;
            };
            let result = inst.result.ok_or(Error::Limit)?;
            self.scratch[result.index()] = Some(thread.context.body.constant(value)?);
            touched.push(result);
        }
        for temp in touched {
            self.scratch[temp.index()] = None;
        }
        Ok(target)
    }
    fn outside_uses(
        thread: &Threader<'_, '_>,
        block: BlockId,
        id: InstId,
        notes: &mut Vec<(Value, BlockId)>,
    ) -> Result<(), Error> {
        let body = &*thread.context.body;
        let cfg = thread.context.cfg()?;
        let kind = &body.instruction(id).kind;
        if let I::Phi(_, pairs) = kind {
            notes.extend(
                pairs
                    .iter()
                    .filter(|(pred, _)| cfg.reachable(*pred))
                    .map(|&(pred, value)| (value, pred)),
            );
        } else {
            body.visit_values(kind, |value| {
                if let Value::Temp(temp) = value
                    && body
                        .temp(temp)
                        .definition
                        .is_none_or(|d| thread.context.block_map()[d.index()] != Some(block))
                {
                    notes.push((value, block));
                }
            });
        }
        Ok(())
    }
    fn note(&mut self, thread: &Threader<'_, '_>, value: Value, at: BlockId) -> Result<(), Error> {
        let Value::Temp(temp) = value else {
            return Ok(());
        };
        let Some(known) = self.use_dom.get_mut(temp.index()) else {
            return Ok(());
        };
        *known = if let Some(previous) = *known {
            thread.context.cfg()?.common_dominator(previous, at)
        } else {
            Some(at)
        };
        Ok(())
    }
    fn find_use_dominators(&mut self, thread: &Threader<'_, '_>) -> Result<(), Error> {
        let body = &*thread.context.body;
        self.use_dom = vec![None; body.temps().len()];
        let mut notes = Vec::new();
        for &block in &body.layout {
            if !thread.context.cfg()?.reachable(block) {
                continue;
            }
            for &id in &body.block(block).instructions {
                notes.clear();
                Self::outside_uses(thread, block, id, &mut notes)?;
                for &(value, at) in &notes {
                    self.note(thread, value, at)?;
                }
            }
        }
        Ok(())
    }
    fn holder(
        &self,
        thread: &Threader<'_, '_>,
        block: BlockId,
        temp: TempId,
    ) -> Result<Option<BlockId>, Error> {
        let cfg = thread.context.cfg()?;
        let mut at = self.use_dom.get(temp.index()).copied().flatten();
        while let Some(next) = at {
            if next == block || cfg.dominator(next) == Some(block) {
                break;
            }
            at = cfg.dominator(next);
        }
        Ok(at.filter(|&at| at != block && cfg.predecessors(at).len() == 1))
    }
    fn private_values(
        &mut self,
        thread: &Threader<'_, '_>,
        block: BlockId,
        term: InstId,
        uses: &[u32],
    ) -> Result<Option<[Option<BlockId>; 2]>, Error> {
        for &id in &thread.context.body.block(block).instructions {
            thread
                .context
                .body
                .visit_values(&thread.context.body.instruction(id).kind, |value| {
                    if let Value::Temp(t) = value {
                        if self.stamp[t.index()] != Some(block) {
                            self.stamp[t.index()] = Some(block);
                            self.local[t.index()] = 0;
                        }
                        self.local[t.index()] += 1;
                    }
                });
        }
        let mut kept = [None; 2];
        for &id in &thread.context.body.block(block).instructions {
            if id == term {
                break;
            }
            let inst = thread.context.body.instruction(id);
            thread.context.body.visit_values(&inst.kind, |value| {
                if let Value::Temp(t) = value {
                    self.local[t.index()] -= 1;
                }
            });
            let Some(result) = inst.result else {
                return Ok(None);
            };
            if self.stamp[result.index()] == Some(block)
                && self.local[result.index()] == uses[result.index()]
            {
                continue;
            }
            if self.use_dom.is_empty() {
                self.find_use_dominators(thread)?;
            }
            let Some(holder) = self.holder(thread, block, result)? else {
                return Ok(None);
            };
            if self.touched[holder.index()]
                || kept[0].is_some_and(|p| p != holder) && kept[1].is_some()
            {
                return Ok(None);
            }
            if kept[0].is_none_or(|p| p == holder) {
                kept[0] = Some(holder);
            } else {
                kept[1] = Some(holder);
            }
        }
        Ok(Some(kept))
    }
    fn collect_inputs(
        &mut self,
        thread: &Threader<'_, '_>,
        block: BlockId,
        preds: &[BlockId],
        count: usize,
    ) -> Result<(), Error> {
        for (index, pred) in preds.iter().enumerate() {
            self.slots[pred.index()] = Some(index);
        }
        self.inputs = vec![None; preds.len().checked_mul(count).ok_or(Error::Limit)?];
        for (index, id) in phis(thread.context.body, block).enumerate() {
            let I::Phi(_, pairs) = &thread.context.body.instruction(id).kind else {
                return Err(Error::Limit);
            };
            for &(pred, value) in pairs.iter().rev() {
                if let Some(slot) = self.slots[pred.index()]
                    && preds.get(slot) == Some(&pred)
                {
                    self.inputs[slot * count + index] = Some(value);
                }
            }
        }
        Ok(())
    }
    fn apply_moves(
        &mut self,
        thread: &mut Threader<'_, '_>,
        block: BlockId,
        preds: &[BlockId],
        count: usize,
        moves: &mut Vec<(usize, BlockId)>,
    ) -> Result<bool, Error> {
        let mut targets = Vec::new();
        for &(_, target) in moves.iter() {
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
        let mut keep = vec![true; moves.len()];
        for &target in &targets {
            let has_phi = phis(thread.context.body, target).next().is_some();
            let pred = thread.context.cfg()?.predecessors(target);
            for (index, &(slot, to)) in moves.iter().enumerate() {
                let from = preds[slot];
                if to == target && has_phi && pred.contains(&from) || targets.contains(&from) {
                    keep[index] = false;
                }
            }
        }
        let mut index = 0;
        moves.retain(|_| {
            let result = keep[index];
            index += 1;
            result
        });
        if moves.is_empty() {
            return Ok(false);
        }
        let mut position = vec![None; thread.context.body.instructions().len()];
        for (index, id) in phis(thread.context.body, block).enumerate() {
            position[id.index()] = Some(index);
        }
        for target in targets {
            for id in phis(thread.context.body, target).collect::<Vec<_>>() {
                let value = incoming(thread.context.body, id, block);
                let through = value
                    .and_then(|v| definition(thread.context.body, v))
                    .filter(|&d| {
                        matches!(thread.context.body.instruction(d).kind, I::Phi(..))
                            && thread.context.block_map()[d.index()] == Some(block)
                    });
                for &(slot, to) in moves.iter() {
                    if to != target {
                        continue;
                    }
                    let value = if let Some(def) = through {
                        self.inputs[slot * count + position[def.index()].ok_or(Error::Limit)?]
                    } else {
                        value
                    };
                    thread.context.body.append_phi_predecessor(
                        id,
                        preds[slot],
                        value.ok_or_else(|| {
                            Error::Construction("missing threaded phi input".into())
                        })?,
                    )?;
                }
            }
        }
        let mut bypassed = vec![false; self.touched.len()];
        for &(slot, to) in moves.iter() {
            let pred = preds[slot];
            let term = last(thread.context.body, pred).ok_or(Error::Limit)?;
            let kind = match thread.context.body.instruction(term).kind {
                I::Jump(_) => I::Jump(to),
                I::Branch(c, yes, no) => {
                    if yes == block {
                        I::Branch(c, to, no)
                    } else {
                        I::Branch(c, yes, to)
                    }
                }
                _ => return Err(Error::Limit),
            };
            thread.context.body.replace_kind(term, kind);
            bypassed[pred.index()] = true;
            self.touched[pred.index()] = true;
            self.touched[to.index()] = true;
            thread.folded_branch()?;
        }
        for id in phis(thread.context.body, block).collect::<Vec<_>>() {
            let I::Phi(ty, mut pairs) = thread.context.body.instruction(id).kind.clone() else {
                return Err(Error::Limit);
            };
            pairs.retain(|&(p, _)| !bypassed[p.index()]);
            thread.context.body.replace_kind(id, I::Phi(ty, pairs));
        }
        Ok(true)
    }
    fn copy_branch(
        &mut self,
        thread: &mut Threader<'_, '_>,
        block: BlockId,
        pred: BlockId,
        inputs: &[Option<Value>],
    ) -> Result<bool, Error> {
        let term = last(thread.context.body, block).ok_or(Error::Limit)?;
        let I::Branch(condition, yes, no) = thread.context.body.instruction(term).kind else {
            return Err(Error::Limit);
        };
        if yes == no
            || [block, pred].contains(&yes)
            || [block, pred].contains(&no)
            || thread.context.cfg()?.handler(yes)
            || thread.context.cfg()?.handler(no)
        {
            return Ok(false);
        }
        let count = phis(thread.context.body, block).count();
        if inputs.iter().take(count).any(Option::is_none) {
            return Ok(false);
        }
        self.scratch.resize(thread.context.body.temps().len(), None);
        let mut touched = Vec::new();
        let mut input = 0;
        let pt = last(thread.context.body, pred).ok_or(Error::Limit)?;
        let mut links = LinkEditor::new(thread.context.body)?;
        let mut at = links.previous(pt);
        for id in thread.context.body.block(block).instructions.clone() {
            if id == term {
                break;
            }
            let mut inst = thread.context.body.instruction(id).clone();
            let old = inst.result.ok_or(Error::Limit)?;
            if matches!(inst.kind, I::Phi(..)) {
                self.scratch[old.index()] = inputs[input];
                input += 1;
                touched.push(old);
                continue;
            }
            let result = thread
                .context
                .body
                .generated_temp(temp_type(thread.context.body, old)?)?;
            inst.result = Some(result);
            let copy = thread.context.body.append(pred, inst)?;
            thread
                .context
                .body
                .rewrite_values(copy, |v| Ok(mapped(v, &self.scratch)))?;
            links.insert_after(pred, at, copy)?;
            thread.context.record_instruction_block(copy, pred);
            at = Some(copy);
            self.scratch[old.index()] = Some(Value::Temp(result));
            touched.push(old);
        }
        thread
            .context
            .body
            .replace_kind(pt, I::Branch(mapped(condition, &self.scratch), yes, no));
        for temp in touched {
            self.scratch[temp.index()] = None;
        }
        for target in [yes, no] {
            for phi in phis(thread.context.body, target).collect::<Vec<_>>() {
                let value = incoming(thread.context.body, phi, block)
                    .ok_or_else(|| Error::Construction("missing copied phi input".into()))?;
                thread
                    .context
                    .body
                    .append_phi_predecessor(phi, pred, value)?;
            }
        }
        thread.context.body.remove_phi_predecessor(block, pred);
        links.commit(thread.context.body)?;
        self.touched[pred.index()] = true;
        self.touched[yes.index()] = true;
        self.touched[no.index()] = true;
        thread.folded_branch()?;
        Ok(true)
    }
    fn route(
        &mut self,
        thread: &mut Threader<'_, '_>,
        block: BlockId,
        uses: &[u32],
    ) -> Result<bool, Error> {
        let cfg = thread.context.cfg()?;
        if !cfg.reachable(block)
            || cfg.predecessors(block).is_empty()
            || cfg.handler(block)
            || self.touched[block.index()]
            || thread.context.body.layout.first() == Some(&block)
        {
            return Ok(false);
        }
        let Some(term) = last(thread.context.body, block) else {
            return Ok(false);
        };
        if !matches!(thread.context.body.instruction(term).kind, I::Branch(..)) {
            return Ok(false);
        }
        let mut work = 0;
        let mut has_phi = false;
        let mut copyable = true;
        for &id in &thread.context.body.block(block).instructions {
            if id == term || work > 8 {
                break;
            }
            let kind = &thread.context.body.instruction(id).kind;
            let phi = matches!(kind, I::Phi(..));
            has_phi |= phi;
            work += u32::from(!phi);
            copyable &= super::pure(kind) || matches!(kind,I::Load(flags,..) if !flags.volatile());
        }
        if !has_phi || work > 8 {
            return Ok(false);
        }
        copyable &= work <= 2;
        let Some(kept) = self.private_values(thread, block, term, uses)? else {
            return Ok(false);
        };
        copyable &= kept[0].is_none();
        let count = phis(thread.context.body, block).count();
        let preds = thread.context.cfg()?.predecessors(block).to_vec();
        self.collect_inputs(thread, block, &preds, count)?;
        let mut moves = Vec::new();
        let mut copies = Vec::new();
        for (slot, &pred) in preds.iter().enumerate() {
            let Some(tail) = last(thread.context.body, pred) else {
                continue;
            };
            if self.touched[pred.index()] || pred == block {
                continue;
            }
            let kind = &thread.context.body.instruction(tail).kind;
            let jumps = matches!(kind, I::Jump(_));
            if !jumps && !matches!(kind,I::Branch(_,a,b) if a!=b) {
                continue;
            }
            let inputs = self.inputs[slot * count..(slot + 1) * count].to_vec();
            if let Some(target) = self.evaluate(thread, block, &inputs)? {
                if target != block
                    && !kept.contains(&Some(target))
                    && !thread.context.cfg()?.handler(target)
                {
                    moves.push((slot, target));
                }
            } else if jumps && copyable {
                copies.push(slot);
            }
        }
        let mut changed = false;
        if !moves.is_empty() && self.apply_moves(thread, block, &preds, count, &mut moves)? {
            changed = true;
            for holder in kept.into_iter().flatten() {
                self.touched[holder.index()] = true;
            }
        }
        let mut budget = 8;
        for slot in copies {
            if self.touched[preds[slot].index()] || budget < work {
                continue;
            }
            let inputs = self.inputs[slot * count..(slot + 1) * count].to_vec();
            if self.copy_branch(thread, block, preds[slot], &inputs)? {
                budget -= work;
                changed = true;
            }
        }
        Ok(changed)
    }
}
impl Threader<'_, '_> {
    pub(super) fn constant_phis(&mut self, counted: bool) -> Result<bool, Error> {
        let uses = if counted {
            self.context.entry_uses().to_vec()
        } else {
            self.context.body.recount_uses()?;
            self.context.body.temps().iter().map(|t| t.uses).collect()
        };
        self.context
            .prepare_block_map(Pass::ThreadJumps, !self.touched);
        let mut router = Router::new(self);
        let mut changed = false;
        for block in self.context.body.layout.clone() {
            changed = router.route(self, block, &uses)? || changed;
        }
        Ok(changed)
    }
}
