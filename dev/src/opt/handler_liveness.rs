//! Exception-entry visibility and bounded liveness from `opt/slot_promotion.cpp`.
//! See NOTICE. These facts belong to one function and never outlive its CFG.
use super::pass_context::Context;
use crate::lowir::{
    Error,
    model::{BlockId, InstructionKind as I, RuntimeInstruction as R},
};

fn reaches_handlers(context: &Context<'_>) -> Result<Vec<bool>, Error> {
    let cfg = context.cfg()?;
    let mut reached = vec![false; context.body.blocks().len()];
    let mut work = Vec::new();
    for &block in &context.body.layout {
        if cfg.handler(block) {
            reached[block.index()] = true;
            work.push(block);
        }
    }
    while let Some(block) = work.pop() {
        for &next in cfg.successors(block).iter().chain(cfg.handlers(block)) {
            if !reached[next.index()] && cfg.placed(next) {
                reached[next.index()] = true;
                work.push(next);
            }
        }
    }
    Ok(reached)
}

fn normal_depths(
    context: &Context<'_>,
    unseen: &mut [bool],
) -> Result<(Vec<Option<u32>>, bool), Error> {
    let mut depth = vec![None; context.body.blocks().len()];
    let entry = *context.body.layout.first().ok_or(Error::Limit)?;
    depth[entry.index()] = Some(0_u32);
    let mut work = vec![entry];
    let mut fallback = false;
    while let Some(block) = work.pop() {
        if fallback {
            break;
        }
        let mut at = depth[block.index()].ok_or(Error::Limit)?;
        let mut opened = false;
        for &id in &context.body.block(block).instructions {
            match context.body.instruction(id).kind {
                I::Runtime(R::EhTry(..) | R::EhCleanup(..)) => {
                    at = at.checked_add(1).ok_or(Error::Limit)?;
                    opened = true;
                }
                I::Runtime(R::EhEnd) => {
                    if let Some(next) = at.checked_sub(1) {
                        at = next;
                    } else {
                        fallback = true;
                        break;
                    }
                }
                I::Store(_, _, _, crate::lowir::model::Value::Slot(slot)) if at != 0 || opened => {
                    unseen[slot.index()] = true;
                }
                _ => (),
            }
        }
        for &next in context.cfg()?.successors(block) {
            if let Some(known) = depth[next.index()] {
                fallback |= known != at;
            } else {
                depth[next.index()] = Some(at);
                work.push(next);
            }
        }
    }
    for &block in &context.body.layout {
        fallback |= context.cfg()?.handler(block) && depth[block.index()].is_some();
    }
    Ok((depth, fallback))
}

pub(super) fn exclude(context: &Context<'_>, allowed: &mut [bool]) -> Result<(), Error> {
    if !context
        .body
        .layout
        .iter()
        .any(|&block| context.cfg().is_ok_and(|cfg| cfg.handler(block)))
    {
        return Ok(());
    }
    let reached = reaches_handlers(context)?;
    let mut unseen = vec![false; allowed.len()];
    let (depth, fallback) = normal_depths(context, &mut unseen)?;
    let mut loaded = vec![false; allowed.len()];
    for &block in &context.body.layout {
        for &id in &context.body.block(block).instructions {
            match context.body.instruction(id).kind {
                I::Store(_, _, _, crate::lowir::model::Value::Slot(slot))
                    if depth[block.index()].is_none() =>
                {
                    unseen[slot.index()] = true;
                }
                I::Load(_, _, crate::lowir::model::Value::Slot(slot)) if reached[block.index()] => {
                    loaded[slot.index()] = true;
                }
                _ => (),
            }
        }
    }
    let mut relevant = vec![None; allowed.len()];
    let mut at_stake = Vec::new();
    for (slot, &ok) in allowed.iter().enumerate() {
        if ok && loaded[slot] && (unseen[slot] || fallback) {
            relevant[slot] = Some(at_stake.len());
            at_stake.push(slot);
        }
    }
    if at_stake.is_empty() {
        return Ok(());
    }
    let live = liveness(context, &reached, &relevant, at_stake.len())?;
    for (index, slot) in at_stake.into_iter().enumerate() {
        if live.as_ref().is_none_or(|live| live[index]) {
            allowed[slot] = false;
        }
    }
    Ok(())
}

fn predecessors(
    context: &Context<'_>,
    blocks: &[BlockId],
    dense: &[Option<usize>],
) -> Result<Vec<Vec<usize>>, Error> {
    let mut result = vec![Vec::new(); blocks.len()];
    for (index, &block) in blocks.iter().enumerate() {
        for &next in context
            .cfg()?
            .successors(block)
            .iter()
            .chain(context.cfg()?.handlers(block))
        {
            if let Some(target) = dense[next.index()] {
                result[target].push(index);
            }
        }
    }
    Ok(result)
}

fn liveness(
    context: &Context<'_>,
    reached: &[bool],
    relevant: &[Option<usize>],
    count: usize,
) -> Result<Option<Vec<bool>>, Error> {
    const BUDGET_BITS: usize = 1 << 22;
    let words = count.div_ceil(64);
    let blocks = context
        .body
        .layout
        .iter()
        .copied()
        .filter(|b| reached[b.index()])
        .collect::<Vec<_>>();
    if blocks
        .len()
        .checked_mul(words)
        .and_then(|n| n.checked_mul(64))
        .is_none_or(|n| n > BUDGET_BITS)
    {
        return Ok(None);
    }
    let mut dense = vec![None; reached.len()];
    for (index, block) in blocks.iter().enumerate() {
        dense[block.index()] = Some(index);
    }
    let mut exposed = vec![0_u64; blocks.len() * words];
    let mut killed = vec![0_u64; exposed.len()];
    for (index, &block) in blocks.iter().enumerate() {
        for &id in &context.body.block(block).instructions {
            let (load, slot) = match context.body.instruction(id).kind {
                I::Load(_, _, crate::lowir::model::Value::Slot(slot)) => (true, slot),
                I::Store(_, _, _, crate::lowir::model::Value::Slot(slot)) => (false, slot),
                _ => continue,
            };
            let Some(relevant) = relevant[slot.index()] else {
                continue;
            };
            let bit = 1_u64 << (relevant % 64);
            let position = index * words + relevant / 64;
            if load && killed[position] & bit == 0 {
                exposed[position] |= bit;
            }
            if !load {
                killed[position] |= bit;
            }
        }
    }
    let predecessors = predecessors(context, &blocks, &dense)?;
    let mut incoming = exposed;
    let mut queued = vec![true; blocks.len()];
    let mut work = (0..blocks.len()).rev().collect::<Vec<_>>();
    while let Some(index) = work.pop() {
        queued[index] = false;
        for &pred in &predecessors[index] {
            let mut grew = false;
            for word in 0..words {
                let position = pred * words + word;
                let add = incoming[index * words + word] & !killed[position] & !incoming[position];
                grew |= add != 0;
                incoming[position] |= add;
            }
            if grew && !queued[pred] {
                queued[pred] = true;
                work.push(pred);
            }
        }
    }
    let mut live = vec![false; count];
    for (index, &block) in blocks.iter().enumerate() {
        if context.cfg()?.handler(block) {
            for (bit, live) in live.iter_mut().enumerate() {
                *live |= incoming[index * words + bit / 64] & (1_u64 << (bit % 64)) != 0;
            }
        }
    }
    Ok(Some(live))
}
