//! Path range proofs and bounded test routing from `opt/jump_threading.cpp`.
//! See NOTICE.
use super::{
    alias_facts::signed_word,
    branch_ranges::{self, ValueSet, width},
    edge_demotion::plain,
    jump_threading::{Threader, definition, incoming, last, phis, successors},
    numeric_fold as fold,
    pass_context::Pass,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{BlockId, InstId, InstructionKind as I, TempId, Value},
    vocabulary::Compare as C,
};
struct Test {
    value: Value,
    bits: u32,
    query: ValueSet,
    arms: Vec<(BlockId, ValueSet)>,
    branch: Option<(BlockId, BlockId)>,
    result: Option<TempId>,
    tail: InstId,
    copied: usize,
    phis: usize,
}
impl Test {
    fn decide(&self, facts: ValueSet) -> Option<BlockId> {
        if let Some((yes, no)) = self.branch {
            facts
                .decides(self.query)
                .map(|truth| if truth { yes } else { no })
        } else {
            self.arms
                .iter()
                .find_map(|&(target, query)| (facts.decides(query) == Some(true)).then_some(target))
        }
    }
    fn on_edge(
        &self,
        thread: &Threader<'_, '_>,
        from: BlockId,
        to: BlockId,
        value: Value,
    ) -> Option<BlockId> {
        let (known, facts, bits) = branch_ranges::edge_fact(thread.context.body, from, to)?;
        self.decide(branch_ranges::related(
            thread.context.body,
            known,
            facts,
            bits,
            value,
            self.bits,
        ))
    }
    fn outcome(
        &self,
        thread: &Threader<'_, '_>,
        pred: BlockId,
        into: BlockId,
        value: Value,
    ) -> Result<Option<BlockId>, Error> {
        if !matches!(value, Value::Temp(_)) {
            return Ok(None);
        }
        let cfg = thread.context.cfg()?;
        let mut from = pred;
        let mut to = into;
        for _ in 0..4 {
            if let Some(target) = self.on_edge(thread, from, to, value) {
                return Ok(Some(target));
            }
            if cfg.predecessors(from).len() != 1 {
                break;
            }
            to = from;
            from = cfg.predecessors(from)[0];
        }
        let mut at = Some(pred);
        for _ in 0..8 {
            let Some(block) = at else {
                break;
            };
            if cfg.predecessors(block).len() == 1
                && let Some(target) = self.on_edge(thread, cfg.predecessors(block)[0], block, value)
            {
                return Ok(Some(target));
            }
            at = cfg.dominator(block);
        }
        Ok(None)
    }
}
impl Threader<'_, '_> {
    fn test(&self, block: BlockId) -> Result<Option<Test>, Error> {
        let body = &*self.context.body;
        let ids = &body.block(block).instructions;
        let Some(&last) = ids.last() else {
            return Ok(None);
        };
        let phis = ids
            .iter()
            .take(ids.len().saturating_sub(1))
            .take_while(|&&id| matches!(body.instruction(id).kind, I::Phi(..)))
            .count();
        let first = ids[phis];
        let term = &body.instruction(last).kind;
        if let I::Switch(value @ Value::Temp(temp), _, _) = *term
            && first == last
            && self.context.level >= 2
        {
            let bits = width(body.temp(temp).ty.ok_or(Error::Limit)?);
            if bits != 0 {
                let mut targets = successors(term);
                targets.sort_unstable_by_key(|b| b.index());
                targets.dedup();
                let arms = targets
                    .into_iter()
                    .map(|target| (target, ValueSet::switched(body, term, target, bits)))
                    .collect();
                return Ok(Some(Test {
                    value,
                    bits,
                    query: ValueSet::default(),
                    arms,
                    branch: None,
                    result: None,
                    tail: last,
                    copied: 0,
                    phis,
                }));
            }
        }
        let I::Branch(condition @ Value::Temp(temp), yes, no) = *term else {
            return Ok(None);
        };
        if yes == no {
            return Ok(None);
        }
        let Some(def) = definition(body, condition) else {
            return Ok(None);
        };
        let mut tail = last;
        let mut copied = 0;
        let def_position = ids[phis..ids.len() - 1]
            .iter()
            .position(|&id| id == def)
            .map(|p| p + phis);
        if let Some(position) = def_position {
            if ids.get(position + 1) != Some(&last) {
                return Ok(None);
            }
            tail = def;
        }
        for &id in &ids[phis..] {
            if id == tail {
                break;
            }
            copied += 1;
            if copied > 6 || !plain(&body.instruction(id).kind) || id == def {
                return Ok(None);
            }
        }
        if copied != 0 && self.context.level < 2 {
            return Ok(None);
        }
        let first = if copied != 0 { tail } else { first };
        let compare = &body.instruction(def);
        let alone = first == last && def != last;
        let paired = first == def
            && ids.get(phis + copied + 1) == Some(&last)
            && compare
                .result
                .is_some_and(|t| self.context.entry_uses().get(t.index()) == Some(&1));
        let ty = body.temp(temp).ty.ok_or(Error::Limit)?;
        let bare = alone && !matches!(compare.kind, I::Compare(..)) && ty.bits() != 0;
        let (mut pred, ty, mut value, mut constant, result) = if bare {
            (C::Ne, ty, condition, None, None)
        } else {
            let I::Compare(pred, ty, left, right) = compare.kind else {
                return Ok(None);
            };
            if !alone && !paired {
                return Ok(None);
            }
            (pred, ty, left, Some(right), compare.result)
        };
        if !matches!(value, Value::Temp(_)) {
            let Some(other) = constant else {
                return Ok(None);
            };
            constant = Some(value);
            value = other;
            pred = pred.swapped();
        }
        let bits = width(ty);
        if bits == 0 || !matches!(value, Value::Temp(_)) {
            return Ok(None);
        }
        let raw = if bare {
            0
        } else {
            let Some(Constant::Integer(raw)) = constant.and_then(|v| fold::literal(body, v)) else {
                return Ok(None);
            };
            u64::from_le_bytes(signed_word(raw).to_le_bytes())
        };
        Ok(Some(Test {
            value,
            bits,
            query: ValueSet::admitted(pred, raw, bits),
            arms: Vec::new(),
            branch: Some((yes, no)),
            result,
            tail,
            copied,
            phis,
        }))
    }
    fn redirects(
        &mut self,
        block: BlockId,
        test: &Test,
        x_phi: Option<InstId>,
    ) -> Result<(Vec<(BlockId, BlockId)>, bool), Error> {
        let mut redirects = Vec::new();
        let mut merges = Vec::new();
        for pred in self.context.cfg()?.predecessors(block).to_vec() {
            if pred == block {
                continue;
            }
            let value = x_phi.map_or(Some(test.value), |id| incoming(self.context.body, id, pred));
            let Some(value) = value else {
                continue;
            };
            if let Some(target) = test.outcome(self, pred, block, value)? {
                redirects.push((pred, target));
                continue;
            }
            if x_phi.is_none() || !self.mergeable(pred, block, value)? {
                continue;
            }
            let phi = definition(self.context.body, value).ok_or(Error::Limit)?;
            for &from in self.context.cfg()?.predecessors(pred) {
                if from == pred || from == block || self.context.cfg()?.handler(from) {
                    continue;
                }
                if let Some(value) = incoming(self.context.body, phi, from)
                    && let Some(target) = test.outcome(self, from, pred, value)?
                {
                    merges.push((pred, from, target));
                }
            }
        }
        let changed = !merges.is_empty();
        for (pred, from, target) in merges {
            let copy = self.copy_merge(pred, from, block)?;
            redirects.push((copy, target));
        }
        if changed {
            self.rebuild()?;
        }
        Ok((redirects, changed))
    }
    fn route_edges(
        &mut self,
        block: BlockId,
        test: &Test,
        redirects: &[(BlockId, BlockId)],
    ) -> Result<bool, Error> {
        let mut changed = false;
        for &(pred, target) in redirects {
            let mut okay = true;
            let target_phis = phis(self.context.body, target).collect::<Vec<_>>();
            for &phi in &target_phis {
                let source = incoming(self.context.body, phi, block);
                let previous = incoming(self.context.body, phi, pred);
                okay &= source.is_some()
                    && source != test.result.map(Value::Temp)
                    && previous.is_none_or(|old| Some(old) == source);
            }
            if !okay {
                continue;
            }
            let term = last(self.context.body, pred).ok_or(Error::Limit)?;
            let had_edge = successors(&self.context.body.instruction(term).kind).contains(&target);
            if !had_edge {
                for phi in target_phis {
                    let value = incoming(self.context.body, phi, block).ok_or(Error::Limit)?;
                    self.context.body.append_phi_predecessor(phi, pred, value)?;
                }
            }
            self.context.body.redirect_successors(term, block, target);
            self.folded_branch()?;
            changed = true;
        }
        Ok(changed)
    }
    pub(super) fn known_compares(&mut self) -> Result<bool, Error> {
        self.context
            .prepare_uses(Pass::ThreadJumps, !self.touched)?;
        // Merge checks read defining-block facts; keep the snapshot invalidated
        // when explicit cloning rebuilds it below.
        self.context
            .prepare_block_map(Pass::ThreadJumps, !self.touched);
        let mut changed = false;
        for block in self.context.body.layout.clone() {
            let cfg = self.context.cfg()?;
            if !cfg.reachable(block)
                || cfg.handler(block)
                || cfg.reverse_postorder().first() == Some(&block)
            {
                continue;
            }
            let Some(test) = self.test(block)? else {
                continue;
            };
            let mut x_phi = None;
            let mut redefined = false;
            for &id in &self.context.body.block(block).instructions {
                let inst = self.context.body.instruction(id);
                if inst.result.map(Value::Temp) == Some(test.value) {
                    if matches!(inst.kind, I::Phi(..)) {
                        x_phi = Some(id);
                    } else {
                        redefined = true;
                        break;
                    }
                }
            }
            if redefined {
                continue;
            }
            let (mut redirects, merged) = self.redirects(block, &test, x_phi)?;
            changed |= merged;
            if redirects.is_empty() {
                continue;
            }
            if test.copied != 0 {
                changed = self.copy_thread(block, test.tail, &mut redirects)? || changed;
                continue;
            }
            if test.phis != 0 && !self.demote_phis(block, &mut redirects)? {
                continue;
            }
            changed = self.route_edges(block, &test, &redirects)? || changed;
            if !redirects.is_empty() && changed {
                self.rebuild()?;
            }
        }
        Ok(changed)
    }
}
