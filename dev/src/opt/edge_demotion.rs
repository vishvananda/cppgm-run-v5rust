//! Scoped edge cloning and slot demotion from `opt/jump_threading.cpp`. See NOTICE.
use super::{
    alias_facts,
    jump_threading::{
        Threader, Use, append_layout, collect_uses, definition, incoming, last, mapped, phis,
        replace_use, temp_type, use_value,
    },
    pass_context::Pass,
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, Facts, InstId, Instruction, InstructionKind as I, LinkEditor,
        OptimizerBlock, OptimizerSlot, RuntimeInstruction as R, Value,
    },
    vocabulary::{Binary as B, Type},
};
pub(super) fn plain(kind: &I) -> bool {
    !matches!(kind, I::Call(_) | I::Phi(_, _))
        && !alias_facts::eh(kind)
        && !matches!(
            kind,
            I::Runtime(
                R::StackAlloc(_)
                    | R::VaStart(_)
                    | R::VaArg(..)
                    | R::AtomicLoad(..)
                    | R::AtomicStore(..)
                    | R::AtomicUpdate(..)
                    | R::AtomicCompareExchange { .. }
            )
        )
        && !volatile(kind)
}
fn volatile(kind: &I) -> bool {
    match kind {
        I::Load(f, ..) | I::Store(f, ..) => f.volatile(),
        _ => false,
    }
}
fn compute(kind: &I) -> bool {
    match kind {
        I::Binary(B::Div | B::Mod | B::UDiv | B::UMod, ..) => false,
        I::Copy(..)
        | I::Index(..)
        | I::Addr(..)
        | I::Unary(..)
        | I::Compare(..)
        | I::Select(..)
        | I::Binary(..)
        | I::Convert(..) => true,
        _ => false,
    }
}
fn clone_instruction(
    body: &mut Body,
    block: BlockId,
    source: InstId,
    renamed: &mut [Option<Value>],
) -> Result<InstId, Error> {
    let mut inst = body.instruction(source).clone();
    let old = inst.result;
    if let Some(temp) = old {
        inst.result = Some(body.generated_temp(temp_type(body, temp)?)?);
    }
    let id = body.append(block, inst)?;
    body.rewrite_values(id, |v| Ok(mapped(v, renamed)))?;
    if let Some(old) = old {
        renamed[old.index()] = body.instruction(id).result.map(Value::Temp);
    }
    Ok(id)
}
fn insert(
    body: &mut Body,
    links: &mut LinkEditor,
    block: BlockId,
    before: InstId,
    source: InstId,
    kind: I,
    result: Option<Type>,
) -> Result<InstId, Error> {
    let result = result.map(|ty| body.generated_temp(ty)).transpose()?;
    let id = body.append(block, Instruction::new(result, kind, 0))?;
    body.inherit_location(id, source);
    links.insert_after(block, links.previous(before), id)?;
    Ok(id)
}
impl Threader<'_, '_> {
    pub(super) fn mergeable(
        &self,
        pred: BlockId,
        block: BlockId,
        value: Value,
    ) -> Result<bool, Error> {
        let cfg = self.context.cfg()?;
        let Some(def) = definition(self.context.body, value) else {
            return Ok(false);
        };
        if pred == block
            || cfg.handler(pred)
            || cfg.predecessors(pred).len() < 2
            || cfg.predecessors(block).len() < 2
            || !matches!(self.context.body.instruction(def).kind, I::Phi(..))
            || self.context.block_map().get(def.index()) != Some(&Some(pred))
        {
            return Ok(false);
        }
        let Some(tail) = last(self.context.body, pred) else {
            return Ok(false);
        };
        if !matches!(self.context.body.instruction(tail).kind,I::Jump(to) if to==block) {
            return Ok(false);
        }
        let mut count = 0;
        for &id in &self.context.body.block(pred).instructions {
            if id == tail {
                break;
            }
            let kind = &self.context.body.instruction(id).kind;
            if matches!(kind, I::Phi(..)) {
                continue;
            }
            count += 1;
            if count > 8 || !plain(kind) {
                return Ok(false);
            }
        }
        let from_pred = |value| {
            definition(self.context.body, value)
                .is_some_and(|d| self.context.block_map().get(d.index()) == Some(&Some(pred)))
        };
        for &at in &self.context.body.layout {
            if at == pred {
                continue;
            }
            for &id in &self.context.body.block(at).instructions {
                let kind = &self.context.body.instruction(id).kind;
                if let I::Phi(_, pairs) = kind {
                    if pairs
                        .iter()
                        .any(|&(from, value)| from_pred(value) && (at != block || from != pred))
                    {
                        return Ok(false);
                    }
                } else {
                    let mut bad = false;
                    self.context
                        .body
                        .visit_values(kind, |v| bad |= from_pred(v));
                    if bad {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }
    pub(super) fn copy_merge(
        &mut self,
        pred: BlockId,
        from: BlockId,
        block: BlockId,
    ) -> Result<BlockId, Error> {
        let copy = self.context.body.optimizer_block(OptimizerBlock::Thread)?;
        append_layout(self.context.body, from, copy);
        let mut renamed = vec![None; self.context.body.temps().len()];
        for id in self.context.body.block(pred).instructions.clone() {
            let inst = self.context.body.instruction(id);
            if matches!(inst.kind, I::Phi(..)) {
                renamed[inst.result.ok_or(Error::Limit)?.index()] = Some(
                    incoming(self.context.body, id, from)
                        .ok_or_else(|| Error::Construction("missing merge phi input".into()))?,
                );
                continue;
            }
            if matches!(inst.kind, I::Jump(_)) {
                break;
            }
            clone_instruction(self.context.body, copy, id, &mut renamed)?;
        }
        self.context
            .body
            .append(copy, Instruction::new(None, I::Jump(block), 0))?;
        for phi in phis(self.context.body, block).collect::<Vec<_>>() {
            let value = incoming(self.context.body, phi, pred)
                .ok_or_else(|| Error::Construction("missing merge target phi input".into()))?;
            self.context
                .body
                .append_phi_predecessor(phi, copy, mapped(value, &renamed))?;
        }
        self.context.body.redirect_successors(
            last(self.context.body, from).ok_or(Error::Limit)?,
            pred,
            copy,
        );
        self.context.body.remove_phi_predecessor(pred, from);
        self.context.prepare_block_map(Pass::ThreadJumps, false);
        self.context.invalidate_block_map();
        self.folded_branch()?;
        Ok(copy)
    }
    fn computing_diamond(&self, block: BlockId) -> Result<bool, Error> {
        let cfg = self.context.cfg()?;
        if cfg.predecessors(block).len() != 2 {
            return Ok(false);
        }
        let head = cfg.dominator(block);
        for &pred in cfg.predecessors(block) {
            if cfg.successors(pred).len() == 2 {
                if Some(pred) != head {
                    return Ok(false);
                }
                continue;
            }
            if cfg.predecessors(pred).len() != 1 || Some(cfg.predecessors(pred)[0]) != head {
                return Ok(false);
            }
            let ids = &self.context.body.block(pred).instructions;
            for (index, &id) in ids.iter().take(ids.len().saturating_sub(1)).enumerate() {
                if index >= 2 || !compute(&self.context.body.instruction(id).kind) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    fn split_edges(
        &mut self,
        block: BlockId,
        redirects: &mut [(BlockId, BlockId)],
    ) -> Result<Vec<(BlockId, BlockId)>, Error> {
        let mut edges = Vec::new();
        for pred in self.context.cfg()?.predecessors(block).to_vec() {
            if edges.iter().any(|&(p, _)| p == pred) {
                continue;
            }
            let mut at = pred;
            if self.context.cfg()?.successors(pred).len() != 1 {
                at = self.context.body.optimizer_block(OptimizerBlock::Split)?;
                self.context
                    .body
                    .append(at, Instruction::new(None, I::Jump(block), 0))?;
                append_layout(self.context.body, pred, at);
                self.context.body.redirect_successors(
                    last(self.context.body, pred).ok_or(Error::Limit)?,
                    block,
                    at,
                );
                for (from, _) in redirects.iter_mut() {
                    if *from == pred {
                        *from = at;
                    }
                }
            }
            edges.push((pred, at));
        }
        Ok(edges)
    }
    fn demote_values(
        &mut self,
        block: BlockId,
        defined: &[InstId],
        uses: &[Use],
        edges: &[(BlockId, BlockId)],
        kind: OptimizerSlot,
        check_operand: bool,
    ) -> Result<(), Error> {
        let mut links = LinkEditor::new(self.context.body)?;
        for &source in defined {
            let inst = self.context.body.instruction(source).clone();
            let result = inst.result.ok_or(Error::Limit)?;
            let ty = if kind == OptimizerSlot::Phi {
                if let I::Phi(ty, _) = inst.kind {
                    ty
                } else {
                    return Err(Error::Limit);
                }
            } else {
                temp_type(self.context.body, result)?
            };
            let slot = self.context.body.optimizer_slot(ty, kind)?;
            if let I::Phi(_, pairs) = inst.kind {
                for (from, value) in pairs {
                    let at = edges
                        .iter()
                        .find_map(|&(p, at)| (p == from).then_some(at))
                        .unwrap_or(from);
                    let before = links.last(at).ok_or(Error::Limit)?;
                    insert(
                        self.context.body,
                        &mut links,
                        at,
                        before,
                        source,
                        I::Store(Facts::default(), ty, value, slot),
                        None,
                    )?;
                }
            } else {
                let id = self.context.body.append(
                    block,
                    Instruction::new(
                        None,
                        I::Store(Facts::default(), ty, Value::Temp(result), slot),
                        0,
                    ),
                )?;
                self.context.body.inherit_location(id, source);
                links.insert_after(block, Some(source), id)?;
            }
            for &site in uses {
                if check_operand {
                    if use_value(self.context.body, site) != Some(Value::Temp(result)) {
                        continue;
                    }
                } else if site.definition != source {
                    continue;
                }
                let before = if site.phi {
                    links.last(site.block).ok_or(Error::Limit)?
                } else {
                    site.instruction
                };
                let load = insert(
                    self.context.body,
                    &mut links,
                    site.block,
                    before,
                    source,
                    I::Load(Facts::default(), ty, slot),
                    Some(ty),
                )?;
                replace_use(
                    self.context.body,
                    site,
                    Value::Temp(
                        self.context
                            .body
                            .instruction(load)
                            .result
                            .ok_or(Error::Limit)?,
                    ),
                )?;
            }
            if matches!(self.context.body.instruction(source).kind, I::Phi(..)) {
                links.unlink(block, source)?;
            }
        }
        links.commit(self.context.body)
    }
    pub(super) fn demote_phis(
        &mut self,
        block: BlockId,
        redirects: &mut [(BlockId, BlockId)],
    ) -> Result<bool, Error> {
        if self.demoted == 32 {
            return Ok(false);
        }
        for &pred in self.context.cfg()?.predecessors(block) {
            if pred == block || self.context.cfg()?.handler(pred) {
                return Ok(false);
            }
        }
        if self.computing_diamond(block)? {
            return Ok(false);
        }
        let defined = phis(self.context.body, block).collect::<Vec<_>>();
        if self.cross_phi_inputs(block, &defined) {
            return Ok(false);
        }
        let Some(uses) = collect_uses(self.context, block, &defined, true)? else {
            return Ok(false);
        };
        self.demoted += 1;
        let edges = self.split_edges(block, redirects)?;
        self.demote_values(block, &defined, &uses, &edges, OptimizerSlot::Phi, true)?;
        self.rebuild()?;
        Ok(true)
    }
    fn cross_phi_inputs(&self, block: BlockId, defined: &[InstId]) -> bool {
        phis(self.context.body, block).any(|id| {
            let mut bad = false;
            self.context
                .body
                .visit_values(&self.context.body.instruction(id).kind, |value| {
                    bad |= definition(self.context.body, value)
                        .is_some_and(|def| defined.contains(&def));
                });
            bad
        })
    }
    pub(super) fn copy_thread(
        &mut self,
        block: BlockId,
        tail: InstId,
        redirects: &mut [(BlockId, BlockId)],
    ) -> Result<bool, Error> {
        if self.demoted == 32 {
            return Ok(false);
        }
        for &pred in self.context.cfg()?.predecessors(block) {
            if pred == block
                || self.context.cfg()?.handler(pred)
                || self.context.cfg()?.reachable(pred) && self.context.cfg()?.dominates(block, pred)
            {
                return Ok(false);
            }
        }
        let entry = *self
            .context
            .cfg()?
            .reverse_postorder()
            .first()
            .ok_or(Error::Limit)?;
        let mut links = LinkEditor::new(self.context.body)?;
        for id in self.context.body.block(block).instructions.clone() {
            if id == tail {
                break;
            }
            if matches!(
                self.context.body.instruction(id).kind,
                I::Addr(Value::Slot(_) | Value::Symbol(_))
            ) {
                links.unlink(block, id)?;
                let before = links.last(entry).ok_or(Error::Limit)?;
                links.insert_after(entry, links.previous(before), id)?;
            }
        }
        links.commit(self.context.body)?;
        self.context.prepare_block_map(Pass::ThreadJumps, false);
        self.context.invalidate_block_map();
        let defined = self
            .context
            .body
            .block(block)
            .instructions
            .iter()
            .copied()
            .take_while(|&id| id != tail)
            .filter(|&id| self.context.body.instruction(id).result.is_some())
            .collect::<Vec<_>>();
        if self.cross_phi_inputs(block, &defined) {
            return Ok(false);
        }
        let Some(uses) = collect_uses(self.context, block, &defined, false)? else {
            return Ok(false);
        };
        self.demoted += 1;
        let edges = self.split_edges(block, redirects)?;
        self.demote_values(block, &defined, &uses, &edges, OptimizerSlot::Demote, false)?;
        let skip =
            matches!(self.context.body.instruction(tail).kind, I::Compare(..)).then_some(tail);
        for &(pred, target) in redirects.iter() {
            let copy = self.context.body.optimizer_block(OptimizerBlock::Thread)?;
            append_layout(self.context.body, pred, copy);
            let mut renamed = vec![None; self.context.body.temps().len()];
            for source in self.context.body.block(block).instructions.clone() {
                if matches!(
                    self.context.body.instruction(source).kind,
                    I::Branch(..) | I::Jump(_)
                ) {
                    break;
                }
                if Some(source) == skip {
                    continue;
                }
                clone_instruction(self.context.body, copy, source, &mut renamed)?;
            }
            self.context
                .body
                .append(copy, Instruction::new(None, I::Jump(target), 0))?;
            for phi in phis(self.context.body, target).collect::<Vec<_>>() {
                let value = incoming(self.context.body, phi, block)
                    .ok_or_else(|| Error::Construction("missing copied target phi input".into()))?;
                self.context
                    .body
                    .append_phi_predecessor(phi, copy, mapped(value, &renamed))?;
            }
            self.context.body.redirect_successors(
                last(self.context.body, pred).ok_or(Error::Limit)?,
                block,
                copy,
            );
            self.folded_branch()?;
        }
        self.rebuild()?;
        Ok(true)
    }
}
