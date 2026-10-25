//! Bounded CFG cleanup and dead-definition preservation from `opt/cfg_cleanup.cpp`.
//! See NOTICE.
use super::{definition_order, numeric_fold as fold, pass_context::Context, pure};
use crate::lowir::{
    Error,
    literals::{Constant, FloatKind},
    model::{BlockId, Body, InstId, Instruction, InstructionKind as I, TempId, Value},
    vocabulary::Type,
};

const CFG_ROUNDS: usize = 8;
const BYPASS_PREDECESSORS: usize = 32;
fn bump(counter: &mut u64) -> Result<(), Error> {
    *counter = counter.checked_add(1).ok_or(Error::Limit)?;
    Ok(())
}
fn targets(body: &Body, block: BlockId) -> Vec<BlockId> {
    let Some(&id) = body.block(block).instructions.last() else {
        return Vec::new();
    };
    match &body.instruction(id).kind {
        I::Jump(target) => vec![*target],
        I::Branch(_, a, b) => vec![*a, *b],
        I::Switch(_, default, cases) => std::iter::once(*default)
            .chain(cases.iter().map(|&(_, b)| b))
            .collect(),
        _ => Vec::new(),
    }
}
fn phi_incoming(body: &Body, id: InstId, pred: BlockId) -> Option<Value> {
    let I::Phi(_, pairs) = &body.instruction(id).kind else {
        return None;
    };
    pairs.iter().find_map(|&(b, v)| (b == pred).then_some(v))
}
fn phis(body: &Body, block: BlockId) -> impl Iterator<Item = InstId> + '_ {
    body.block(block)
        .instructions
        .iter()
        .copied()
        .take_while(|&i| matches!(body.instruction(i).kind, I::Phi(..)))
}
fn make_jump(body: &mut Body, id: InstId, target: BlockId) {
    body.replace_kind(id, I::Jump(target));
}
fn fold_terminators(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut changed = false;
    for index in 0..context.body.layout.len() {
        let block = context.body.layout[index];
        let Some(&id) = context.body.block(block).instructions.last() else {
            continue;
        };
        let instruction = context.body.instruction(id);
        let target = match &instruction.kind {
            I::Branch(value, a, b) => {
                if a == b {
                    Some(*a)
                } else if let Some(Constant::Integer(bits)) = fold::literal(context.body, *value) {
                    Some(if bits != 0 { *a } else { *b })
                } else {
                    None
                }
            }
            I::Switch(value, default, cases) => {
                if cases.iter().all(|&(_, b)| b == *default) {
                    Some(*default)
                } else if let Some(c @ Constant::Integer(_)) = fold::literal(context.body, *value) {
                    let ty = instruction.switch_type().unwrap_or(Type::I64);
                    Some(
                        cases
                            .iter()
                            .find_map(|&(label, b)| {
                                fold::literal(context.body, label)
                                    .filter(|&v| fold::same(ty, v, c))
                                    .map(|_| b)
                            })
                            .unwrap_or(*default),
                    )
                } else {
                    None
                }
            }
            _ => None,
        };
        let Some(target) = target else {
            continue;
        };
        let mut dropped = Vec::new();
        for successor in targets(context.body, block) {
            if successor != target && !dropped.contains(&successor) {
                dropped.push(successor);
            }
        }
        for successor in dropped {
            context.body.remove_phi_predecessor(successor, block);
        }
        make_jump(context.body, id, target);
        bump(&mut context.branches_folded)?;
        changed = true;
    }
    Ok(changed)
}
fn truncate_noreturn(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut changed = false;
    for index in 0..context.body.layout.len() {
        let block = context.body.layout[index];
        let ids = &context.body.block(block).instructions;
        let Some(position) = ids.iter().enumerate().find_map(|(p, &id)| {
            let kind = &context.body.instruction(id).kind;
            (matches!(kind, I::Call(_)) && context.program.may_end_block(kind) && p + 1 < ids.len())
                .then_some(p)
        }) else {
            continue;
        };
        if position + 2 == ids.len()
            && matches!(
                context.body.instruction(ids[position + 1]).kind,
                I::Unreachable
            )
        {
            continue;
        }
        let mut dropped = Vec::new();
        for successor in targets(context.body, block) {
            if !dropped.contains(&successor) {
                dropped.push(successor);
            }
        }
        for successor in dropped {
            context.body.remove_phi_predecessor(successor, block);
        }
        let end = context
            .body
            .append(block, Instruction::new(None, I::Unreachable, 0))?;
        let mut kept = context.body.block(block).instructions[..=position].to_vec();
        kept.push(end);
        context.body.replace_links(block, kept)?;
        changed = true;
    }
    Ok(changed)
}
#[derive(Clone, Copy)]
enum Availability {
    Yes,
    No,
    Operands,
}
struct DeadDefinitions<'a> {
    body: &'a mut Body,
    dead: &'a [bool],
    block_of: Vec<Option<BlockId>>,
    moved: Vec<bool>,
    failed: Vec<bool>,
    active: Vec<bool>,
    moves: Vec<InstId>,
    entry: BlockId,
}
impl<'a> DeadDefinitions<'a> {
    fn new(body: &'a mut Body, dead: &'a [bool]) -> Result<Self, Error> {
        let entry = *body.layout.first().ok_or(Error::Limit)?;
        let mut block_of = vec![None; body.instructions().len()];
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                block_of[id.index()] = Some(block);
            }
        }
        let count = body.temps().len();
        Ok(Self {
            body,
            dead,
            block_of,
            moved: vec![false; count],
            failed: vec![false; count],
            active: vec![false; count],
            moves: Vec::new(),
            entry,
        })
    }
    fn check(&mut self, temp: TempId) -> Availability {
        if self.body.temp_is_parameter(temp) || self.moved[temp.index()] {
            return Availability::Yes;
        }
        let Some(id) = self.body.temp(temp).definition else {
            return Availability::No;
        };
        if self.failed[temp.index()] {
            return Availability::No;
        }
        let movable = self.block_of[id.index()].is_some_and(|b| self.dead[b.index()])
            && pure(&self.body.instruction(id).kind)
            && !matches!(self.body.instruction(id).kind, I::Phi(..));
        if movable {
            Availability::Operands
        } else {
            self.failed[temp.index()] = true;
            Availability::No
        }
    }
    fn operand(&self, id: InstId, index: usize) -> Option<TempId> {
        let mut result = None;
        let mut at = 0;
        self.body
            .visit_values(&self.body.instruction(id).kind, |v| {
                if at == index
                    && let Value::Temp(t) = v
                {
                    result = Some(t);
                }
                at += 1;
            });
        result
    }
    fn movable(&mut self, root: TempId) -> Result<bool, Error> {
        match self.check(root) {
            Availability::Yes => return Ok(true),
            Availability::No => return Ok(false),
            Availability::Operands => (),
        }
        let mut stack = vec![(root, 0_usize)];
        self.active[root.index()] = true;
        let mut result = true;
        while let Some(&(temp, _)) = stack.last() {
            if !result {
                self.failed[temp.index()] = true;
                self.active[temp.index()] = false;
                stack.pop();
                continue;
            }
            let id = self.body.temp(temp).definition.ok_or(Error::Limit)?;
            let mut descended = false;
            while stack.last().is_some_and(|&(_, operand)| operand < 3) {
                let frame = stack.last_mut().ok_or(Error::Limit)?;
                let operand = frame.1;
                frame.1 += 1;
                let Some(input) = self.operand(id, operand) else {
                    continue;
                };
                match self.check(input) {
                    Availability::Yes => (),
                    Availability::No => {
                        result = false;
                        break;
                    }
                    Availability::Operands => {
                        if self.active[input.index()] {
                            return Err(Error::Construction(
                                "cyclic ordinary dead definition".into(),
                            ));
                        }
                        self.active[input.index()] = true;
                        stack.push((input, 0));
                        descended = true;
                        break;
                    }
                }
            }
            if descended || !result {
                continue;
            }
            self.moves.push(id);
            self.block_of[id.index()] = Some(self.entry);
            self.moved[temp.index()] = true;
            self.active[temp.index()] = false;
            stack.pop();
        }
        Ok(result)
    }
    fn apply_moves(&mut self) -> Result<(), Error> {
        if self.moves.is_empty() {
            return Ok(());
        }
        let mut moved = vec![false; self.body.instructions().len()];
        for &id in &self.moves {
            moved[id.index()] = true;
            self.body.clear_speculation_facts(id);
        }
        // Batch stable extraction avoids shifting a Vec for every node of a
        // long dead chain; move order is the frozen iterative postorder.
        for index in 0..self.body.layout.len() {
            let block = self.body.layout[index];
            if block == self.entry {
                continue;
            }
            let ids = self
                .body
                .block(block)
                .instructions
                .iter()
                .copied()
                .filter(|id| !moved[id.index()])
                .collect();
            self.body.replace_links(block, ids)?;
        }
        let mut entry = std::mem::take(&mut self.moves);
        entry.extend_from_slice(&self.body.block(self.entry).instructions);
        self.body.replace_links(self.entry, entry)
    }
    fn run(mut self) -> Result<bool, Error> {
        let mut kept = true;
        let mut values = Vec::new();
        let layout = self.body.layout.clone();
        for block in layout {
            if self.dead[block.index()] {
                continue;
            }
            let ids = self.body.block(block).instructions.clone();
            for id in ids {
                values.clear();
                self.body
                    .visit_values(&self.body.instruction(id).kind, |v| values.push(v));
                for value in &mut values {
                    let Value::Temp(temp) = *value else {
                        continue;
                    };
                    if self.body.temp_is_parameter(temp) {
                        continue;
                    }
                    let Some(def) = self.body.temp(temp).definition else {
                        continue;
                    };
                    if self.block_of[def.index()].is_none_or(|b| !self.dead[b.index()])
                        || self.moved[temp.index()]
                        || self.movable(temp)?
                    {
                        continue;
                    }
                    let ty = self.body.temp(temp).ty.ok_or(Error::Limit)?;
                    if matches!(ty, Type::Object { .. }) {
                        kept = false;
                        continue;
                    }
                    *value = self.body.constant(
                        FloatKind::from_type(ty)
                            .map_or(Constant::Integer(0), |k| Constant::Float(k, 0)),
                    )?;
                }
                let mut values = values.iter().copied();
                self.body
                    .rewrite_values(id, |_| values.next().ok_or(Error::Limit))?;
            }
        }
        self.apply_moves()?;
        Ok(kept)
    }
}
fn remove_unreachable(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg_light()?;
    let mut dead = vec![false; context.body.blocks().len()];
    let mut any = false;
    for &block in &context.body.layout {
        if !context.cfg()?.reachable(block) {
            dead[block.index()] = true;
            any = true;
        }
    }
    if !any {
        return Ok(false);
    }
    context.invalidate_block_map();
    let stamp = context.next_pass_stamp();
    for &block in &context.body.layout {
        if dead[block.index()] {
            for &id in &context.body.block(block).instructions {
                context.passed[id.index()] = stamp;
            }
        }
    }
    let defined = context
        .body
        .temps()
        .iter()
        .skip(context.body.parameter_count)
        .any(|t| {
            t.definition
                .is_some_and(|d| context.passed[d.index()] == stamp)
        });
    if defined && !DeadDefinitions::new(context.body, &dead)?.run()? {
        return Ok(false);
    }
    for index in 0..context.body.layout.len() {
        let block = context.body.layout[index];
        if !dead[block.index()] {
            continue;
        }
        let successors = context.cfg()?.successors(block).to_vec();
        for successor in successors {
            if !dead[successor.index()] {
                context.body.remove_phi_predecessor(successor, block);
            }
        }
        bump(&mut context.blocks_removed)?;
    }
    context.body.retain_layout(&dead)?;
    let layout = context.body.layout.clone();
    context.cfg_mut()?.drop_blocks(&dead, &layout)?;
    Ok(true)
}
fn only_jump(body: &Body, block: BlockId) -> Option<BlockId> {
    let [id] = body.block(block).instructions.as_slice() else {
        return None;
    };
    if let I::Jump(target) = body.instruction(*id).kind {
        Some(target)
    } else {
        None
    }
}
fn only_unreachable(body: &Body, block: BlockId) -> bool {
    matches!(body.block(block).instructions.as_slice(),[id] if matches!(body.instruction(*id).kind,I::Unreachable))
}
fn drop_unreachable_edges(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut changed = false;
    for index in 0..context.body.layout.len() {
        let block = context.body.layout[index];
        let Some(&id) = context.body.block(block).instructions.last() else {
            continue;
        };
        let I::Branch(_, yes, no) = context.body.instruction(id).kind else {
            continue;
        };
        if yes == no || context.cfg()?.handler(yes) || context.cfg()?.handler(no) {
            continue;
        }
        let dead_yes = only_unreachable(context.body, yes);
        let dead_no = only_unreachable(context.body, no);
        if dead_yes == dead_no {
            continue;
        }
        context
            .body
            .remove_phi_predecessor(if dead_yes { yes } else { no }, block);
        make_jump(context.body, id, if dead_yes { no } else { yes });
        changed = true;
    }
    Ok(changed)
}
fn bypass_safe(
    context: &Context<'_>,
    block: BlockId,
    target: BlockId,
    touched: &[bool],
) -> Result<bool, Error> {
    let cfg = context.cfg()?;
    let preds = cfg.predecessors(block);
    if preds.is_empty() {
        return Ok(false);
    }
    for &pred in preds {
        if pred == block || touched[pred.index()] {
            return Ok(false);
        }
        for id in phis(context.body, target) {
            let direct = phi_incoming(context.body, id, pred);
            let through = phi_incoming(context.body, id, block);
            if direct.is_none() || direct == through {
                continue;
            }
            let I::Phi(ty, _) = context.body.instruction(id).kind else {
                return Err(Error::Limit);
            };
            if !direct
                .and_then(|v| fold::literal(context.body, v))
                .zip(through.and_then(|v| fold::literal(context.body, v)))
                .is_some_and(|(a, b)| fold::same(ty, a, b))
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn bypass_empty_blocks(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg_light()?;
    let Some(&entry) = context.body.layout.first() else {
        return Ok(false);
    };
    let mut touched = vec![false; context.body.blocks().len()];
    let mut changed = false;
    for index in 0..context.body.layout.len() {
        let block = context.body.layout[index];
        let Some(target) = only_jump(context.body, block) else {
            continue;
        };
        let cfg = context.cfg()?;
        if block == entry
            || cfg.handler(block)
            || target == block
            || !cfg.reachable(block)
            || touched[block.index()]
            || touched[target.index()]
        {
            continue;
        }
        if cfg.predecessors(target).len() > BYPASS_PREDECESSORS
            && phis(context.body, target).next().is_some()
        {
            continue;
        }
        if !bypass_safe(context, block, target, &touched)? {
            continue;
        }
        let preds = context.cfg()?.predecessors(block).to_vec();
        let phis = phis(context.body, target).collect::<Vec<_>>();
        for &pred in &preds {
            for &phi in &phis {
                if phi_incoming(context.body, phi, pred).is_none() {
                    let through = phi_incoming(context.body, phi, block)
                        .ok_or(Error::Construction("phi lacks bypassed predecessor".into()))?;
                    context.body.append_phi_predecessor(phi, pred, through)?;
                }
            }
            let id = *context
                .body
                .block(pred)
                .instructions
                .last()
                .ok_or(Error::Limit)?;
            context.body.redirect_successors(id, block, target);
        }
        context.body.remove_phi_predecessor(target, block);
        touched[block.index()] = true;
        touched[target.index()] = true;
        for pred in preds {
            touched[pred.index()] = true;
        }
        changed = true;
        bump(&mut context.blocks_removed)?;
    }
    Ok(changed)
}
fn merge_blocks(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg_light()?;
    let mut dead = vec![false; context.body.blocks().len()];
    let mut replacements = vec![None; context.body.temps().len()];
    let mut replaced = false;
    let mut changed = false;
    for index in 0..context.body.layout.len() {
        let a = context.body.layout[index];
        if dead[a.index()] {
            continue;
        }
        while let Some(&last) = context.body.block(a).instructions.last() {
            let I::Jump(b) = context.body.instruction(last).kind else {
                break;
            };
            if a == b
                || dead[b.index()]
                || Some(&b) == context.body.layout.first()
                || context.cfg()?.handler(b)
                || context.cfg()?.predecessors(b).len() != 1
            {
                break;
            }
            let mut skip = 0;
            for id in phis(context.body, b) {
                let phi = context.body.instruction(id);
                let I::Phi(_, pairs) = &phi.kind else {
                    return Err(Error::Limit);
                };
                replacements[phi.result.ok_or(Error::Limit)?.index()] =
                    pairs.first().map(|&(_, v)| v);
                replaced = true;
                skip += 1;
            }
            context.body.join_blocks(a, b, skip)?;
            for successor in targets(context.body, a) {
                context.body.rename_phi_predecessor(successor, b, a);
            }
            dead[b.index()] = true;
            bump(&mut context.blocks_merged)?;
            changed = true;
        }
    }
    if !changed {
        return Ok(false);
    }
    context.body.retain_layout(&dead)?;
    if replaced {
        for index in 0..replacements.len() {
            if let Some(mut value) = replacements[index] {
                let mut steps = 0;
                while let Value::Temp(temp) = value {
                    let Some(next) = replacements[temp.index()] else {
                        break;
                    };
                    value = next;
                    steps += 1;
                    if steps > replacements.len() {
                        return Err(Error::Construction("cyclic merged phi replacement".into()));
                    }
                }
                replacements[index] = Some(value);
            }
        }
        let mut live = vec![true; context.body.instructions().len()];
        for index in 0..context.body.layout.len() {
            let block = context.body.layout[index];
            let ids = context.body.block(block).instructions.clone();
            for id in ids {
                context.body.rewrite_values(id, |v| {
                    Ok(if let Value::Temp(t) = v {
                        replacements[t.index()].unwrap_or(v)
                    } else {
                        v
                    })
                })?;
                let instruction = context.body.instruction(id);
                if matches!(instruction.kind, I::Phi(..))
                    && instruction
                        .result
                        .is_some_and(|t| replacements[t.index()].is_some())
                {
                    live[id.index()] = false;
                }
            }
        }
        context.body.retain_instructions(&live)?;
    }
    Ok(true)
}
fn step(context: &mut Context<'_>, changed: bool) -> bool {
    if changed {
        context.invalidate_cfg();
    }
    changed
}
pub(super) fn cleanup(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut any = false;
    for round in 0..CFG_ROUNDS {
        let mut changed = false;
        if round == 0 {
            let truncated = truncate_noreturn(context)?;
            changed = step(context, truncated);
        }
        let folded = fold_terminators(context)?;
        changed = step(context, folded) || changed;
        changed = remove_unreachable(context)? || changed;
        context.ensure_cfg_light()?;
        let dropped = drop_unreachable_edges(context)?;
        changed = step(context, dropped) || changed;
        let bypassed = bypass_empty_blocks(context)?;
        changed = step(context, bypassed) || changed;
        changed = remove_unreachable(context)? || changed;
        let merged = merge_blocks(context)?;
        changed = step(context, merged) || changed;
        if !changed {
            break;
        }
        any = true;
    }
    if any {
        definition_order::restore(context)?;
    }
    context.body.recount_uses()?;
    Ok(any)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{model::Program, writer};
    use std::fmt::Write as _;
    fn run(source: &str) -> (Program, bool, [u64; 3]) {
        let mut p = crate::lowir::test_program(source.as_bytes()).unwrap();
        let symbol = p
            .publications()
            .iter()
            .copied()
            .find(|&s| p.function(p.symbol(s)).is_some_and(|f| f.body.is_some()))
            .unwrap();
        let mut body = p.take_body(symbol).unwrap().unwrap();
        body.record_switch_types();
        let (changed, stats) = {
            let mut c = Context::new(&mut p, symbol, &mut body, 1, &[], &[]);
            let changed = c.run(super::super::pass_context::Pass::CleanupCfg).unwrap();
            (
                changed,
                [c.branches_folded, c.blocks_removed, c.blocks_merged],
            )
        };
        body.compact(&mut p.names).unwrap();
        p.restore_body(symbol, body).unwrap();
        crate::lowir::validate_test_program(&p).unwrap();
        (p, changed, stats)
    }
    fn text(p: &Program) -> String {
        let mut out = Vec::new();
        writer::write(p, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }
    #[test]
    fn unreachable_pure_dependencies_move_in_postorder_and_lose_path_facts() {
        let (p, changed, stats) = run(r"function @f(%v : i64) -> i64 {
 block ^entry: jump ^live
 block ^dead: %a = binary add nowrap i64 %v, 1 %b = binary mul i64 %a, 3 jump ^live
 block ^live: return i64 %b
}");
        assert!(changed);
        assert_eq!(stats, [0, 1, 1]);
        let out = text(&p);
        assert!(!out.contains("nowrap") && !out.contains("block ^dead"));
        assert!(out.find("%a = binary add").unwrap() < out.find("%b = binary mul").unwrap());
        assert!(out.contains("return i64 %b"));
    }
    #[test]
    fn unreachable_effectful_definitions_become_one_zero_literal_per_live_use() {
        let (p, changed, _) = run(r"function @f(%p : ptr) -> i64 {
 block ^entry: jump ^live
 block ^dead: %x = load i64 %p jump ^live
 block ^live: %r = binary add i64 %x, %x return i64 %r
}");
        assert!(changed);
        let out = text(&p);
        assert!(out.contains("binary add i64 0, 0"));
        assert!(!out.contains("load"));
        assert_eq!(p.statistics().literals, 2);
    }
    #[test]
    fn merging_resolves_transitive_single_input_phis_and_keeps_typed_values() {
        let (p, changed, stats) = run(r"function @f() -> i64 {
 block ^entry: %v = const i64 5 jump ^a
 block ^a: %x = phi i64 [^entry: %v] jump ^b
 block ^b: %y = phi i64 [^a: %x] return i64 %y
}");
        assert!(changed);
        assert_eq!(stats, [0, 0, 2]);
        let out = text(&p);
        assert!(!out.contains("phi") && !out.contains("jump"));
        assert!(out.contains("return i64 %v"));
    }
    #[test]
    fn noreturn_truncation_drops_successor_inputs_and_appends_an_unlocated_terminator() {
        let (p, changed, _) = run(r"declare function @stop() -> void [return=noreturn]
function @f() -> i64 {
 block ^entry: call void @stop() %x = const i64 1 jump ^exit
 block ^exit: %p = phi i64 [^entry: %x] return i64 %p
}");
        assert!(changed);
        let out = text(&p);
        assert!(out.contains("call void @stop()\n    unreachable"));
        assert!(!out.contains("phi") && !out.contains("%x"));
    }
    #[test]
    fn unprotected_unreachable_arm_drops_but_a_handler_target_is_preserved() {
        let source = "function @f(%c : i64) -> i64 {block ^entry: branch %c, ^bad, ^good block ^bad: unreachable block ^good: return i64 7}";
        let (p, changed, _) = run(source);
        assert!(changed);
        assert!(!text(&p).contains("branch"));
        let source = source
            .replace("block ^entry:", "block ^entry: eh_try ^bad")
            .replace("block ^good: return", "block ^good: eh_end return");
        let (p, changed, _) = run(&source);
        assert!(!changed);
        assert!(text(&p).contains("branch %c, ^bad, ^good"));
    }
    #[test]
    fn phi_bypass_bound_and_eight_round_limit_are_frozen() {
        for (count, removed) in [(32, 8), (33, 0)] {
            let mut source =
                "function @f(%c : i64) -> i64 {block ^entry: switch %c, ^p0".to_owned();
            for i in 1..count {
                write!(source, ", {i}:^p{i}").unwrap();
            }
            for i in 0..count {
                write!(source, " block ^p{i}: jump ^join").unwrap();
            }
            source.push_str(" block ^join: %v = phi i64 [");
            for i in 0..count {
                if i != 0 {
                    source.push_str(", ");
                }
                write!(source, "^p{i}: 7").unwrap();
            }
            source.push_str("] return i64 %v}");
            let (p, changed, stats) = run(&source);
            assert_eq!(changed, removed != 0);
            assert_eq!(stats[1], removed * 2);
            assert_eq!(
                p.statistics().blocks,
                usize::try_from(count + 2 - removed).unwrap()
            );
        }
    }
}
