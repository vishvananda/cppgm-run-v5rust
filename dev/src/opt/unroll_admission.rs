//! Counted branching/search loop admission from `opt/loop_passes.cpp`. See NOTICE.
use super::{
    alias_facts,
    induction::Facts,
    memory_contents,
    natural_loops::{Forest, LoopId},
    pass_context::Context,
    trip_counts,
};
use crate::lowir::{
    Error,
    model::{BlockId, InstructionKind as I, RuntimeInstruction as R, TempId, Value},
};

pub(super) struct Plan {
    pub(super) trips: usize,
    pub(super) member: Vec<bool>,
    pub(super) owned: Vec<bool>,
    pub(super) through: Option<BlockId>,
}
pub(super) fn forbidden(kind: &I) -> bool {
    alias_facts::eh(kind) || matches!(kind, I::Runtime(R::StackAlloc(_) | R::VaStart(_)))
}
pub(super) fn loop_value(context: &Context<'_>, member: &[bool], v: Value) -> bool {
    let Value::Temp(t) = v else { return false };
    !context.body.temp_is_parameter(t)
        && context
            .body
            .temp(t)
            .definition
            .and_then(|d| context.block_map().get(d.index()))
            .copied()
            .flatten()
            .is_some_and(|b| member[b.index()])
}
fn from_induction(context: &Context<'_>, member: &[bool], iv: TempId, v: Value, depth: u8) -> bool {
    if matches!(v, Value::Integer(_) | Value::Float(..)) {
        return true;
    }
    let Value::Temp(t) = v else { return false };
    if depth == 0 {
        return false;
    }
    if t == iv {
        return true;
    }
    if !loop_value(context, member, v) {
        return false;
    }
    let Some(id) = context.body.temp(t).definition else {
        return false;
    };
    let kind = &context.body.instruction(id).kind;
    if !matches!(
        kind,
        I::Compare(..) | I::Binary(..) | I::Unary(..) | I::Convert(..) | I::Copy(..)
    ) {
        return false;
    }
    let mut all = true;
    context.body.visit_values(kind, |o| {
        all &= from_induction(context, member, iv, o, depth - 1);
    });
    all
}
fn base(context: &Context<'_>, mut value: Value, slot: bool) -> bool {
    for _ in 0..8 {
        let Value::Temp(t) = value else {
            return slot && matches!(value, Value::Slot(_));
        };
        if context.body.temp_is_parameter(t) {
            return false;
        }
        let Some(id) = context.body.temp(t).definition else {
            return false;
        };
        match context.body.instruction(id).kind {
            I::Addr(v) => {
                return if slot {
                    matches!(v, Value::Slot(_))
                } else {
                    matches!(v, Value::Symbol(s) if memory_contents::contents(context.program, s, context.unwritten).is_some())
                };
            }
            I::Index(_, _, v, _) | I::Copy(_, v) => value = v,
            _ => return false,
        }
    }
    slot && matches!(value, Value::Slot(_))
}
fn counted(context: &Context<'_>, forest: &Forest, loop_: LoopId) -> Option<(usize, TempId)> {
    let l = forest.get(loop_);
    if l.latches.len() != 1 {
        return None;
    }
    let head = *context.body.block(l.header).instructions.last()?;
    let I::Branch(Value::Temp(condition), yes, no) = context.body.instruction(head).kind else {
        return None;
    };
    if forest.contains(loop_, yes) == forest.contains(loop_, no) {
        return None;
    }
    let cmp = context.body.temp(condition).definition?;
    if context.block_map().get(cmp.index()).copied().flatten() != Some(l.header) {
        return None;
    }
    let I::Compare(pred, ty, left, right) = context.body.instruction(cmp).kind else {
        return None;
    };
    let facts = Facts {
        body: context.body,
        cfg: context.cfg().ok()?,
        forest,
        block_of: context.block_map(),
    };
    for id in super::full_unrolling::phis(context.body, l.header) {
        let Some(iv) = facts.induction(loop_, id) else {
            continue;
        };
        let Some(Value::Integer(init)) = iv.init else {
            continue;
        };
        if iv.pointer || ty.bits() == 0 {
            continue;
        }
        for (value, bound, pred) in [(left, right, pred), (right, left, pred.swapped())] {
            let (Value::Temp(t), Value::Integer(bound)) = (value, bound) else {
                continue;
            };
            if t != iv.value && t != iv.next {
                continue;
            }
            let init = i128::from(alias_facts::signed_word(super::numeric_fold::normalize(
                ty,
                context.body.literal(init),
            )));
            let bound = i128::from(alias_facts::signed_word(super::numeric_fold::normalize(
                ty,
                context.body.literal(bound),
            )));
            if let Some((n, _)) = trip_counts::count(
                ty,
                pred,
                forest.contains(loop_, yes),
                init,
                iv.step,
                bound,
                t == iv.next,
            ) {
                return (1..=16)
                    .contains(&n)
                    .then_some((usize::try_from(n).ok()?, iv.value));
            }
        }
    }
    None
}
pub(super) fn branching(
    context: &Context<'_>,
    forest: &Forest,
    loop_: LoopId,
) -> Result<Option<Plan>, Error> {
    let Some((trips, iv)) = counted(context, forest, loop_) else {
        return Ok(None);
    };
    let l = forest.get(loop_);
    let cfg = context.cfg()?;
    let count = context.body.blocks().len();
    let mut member = vec![false; count];
    for &b in &l.blocks {
        if forest.innermost(b) != Some(loop_) || cfg.handler(b) {
            return Ok(None);
        }
        member[b.index()] = true;
    }
    let mut table = false;
    let mut slots = false;
    let mut size = 0_usize;
    let mut calling = vec![false; count];
    let head_test = context.body.block(l.header).instructions.last().copied();
    for &b in &l.blocks {
        for &id in &context.body.block(b).instructions {
            let kind = &context.body.instruction(id).kind;
            if forbidden(kind) {
                return Ok(None);
            }
            size += usize::from(!matches!(kind, I::Phi(..)) && !kind.terminator());
            calling[b.index()] |= matches!(kind, I::Call(_));
            match *kind {
                I::Load(flags, _, at) => {
                    slots |= base(context, at, true);
                    table |= !flags.volatile() && base(context, at, false);
                }
                I::Store(_, _, _, at) => slots |= base(context, at, true),
                I::Branch(v, ..) if Some(id) != head_test => {
                    table |= from_induction(context, &member, iv, v, 6);
                }
                _ => (),
            }
        }
    }
    let call_free = !slots && l.blocks.iter().all(|b| !calling[b.index()]);
    if !table {
        if trips > if call_free { 16 } else { 8 } || calling[l.header.index()] {
            return Ok(None);
        }
        let mut seen = vec![false; count];
        let mut work = vec![l.header];
        seen[l.header.index()] = true;
        let mut reaches = false;
        while let Some(b) = work.pop() {
            if b == l.latches[0] {
                reaches = true;
                break;
            }
            for &s in cfg.successors(b) {
                if member[s.index()] && !seen[s.index()] && !calling[s.index()] {
                    seen[s.index()] = true;
                    work.push(s);
                }
            }
        }
        if !reaches {
            return Ok(None);
        }
    }
    let mut owned = vec![false; count];
    let mut live_through = false;
    for &b in &context.body.layout {
        if member[b.index()] {
            continue;
        }
        let mut reads = false;
        let mut phi_reads = false;
        for &id in &context.body.block(b).instructions {
            if reads {
                break;
            }
            let kind = &context.body.instruction(id).kind;
            if let I::Phi(_, pairs) = kind {
                phi_reads |= pairs
                    .iter()
                    .any(|&(from, v)| !member[from.index()] && loop_value(context, &member, v));
            } else {
                context
                    .body
                    .visit_values(kind, |v| reads |= loop_value(context, &member, v));
            }
        }
        if !reads && !phi_reads {
            continue;
        }
        let ids = &context.body.block(b).instructions;
        if reads
            && cfg.predecessors(b).len() == 1
            && member[cfg.predecessors(b)[0].index()]
            && cfg.successors(b).is_empty()
            && !ids.is_empty()
            && !matches!(context.body.instruction(ids[0]).kind, I::Phi(..))
            && !cfg.handler(b)
            && ids.last().is_some_and(|&id| {
                matches!(
                    context.body.instruction(id).kind,
                    I::Return(..) | I::Unreachable
                )
            })
        {
            owned[b.index()] = true;
            size += ids
                .iter()
                .filter(|&&id| !context.body.instruction(id).kind.terminator())
                .count();
        } else {
            live_through = true;
        }
    }
    let mut through = None;
    if live_through {
        for &b in &l.blocks {
            for &s in cfg.successors(b) {
                if member[s.index()] || owned[s.index()] {
                    continue;
                }
                if through.is_some_and(|old| old != s) {
                    return Ok(None);
                }
                through = Some(s);
            }
        }
        if through.is_none_or(|b| cfg.handler(b)) {
            return Ok(None);
        }
    }
    let header_size = context
        .body
        .block(l.header)
        .instructions
        .iter()
        .filter(|&&id| {
            let kind = &context.body.instruction(id).kind;
            !matches!(kind, I::Phi(..)) && !kind.terminator()
        })
        .count();
    let cloned = size * trips + if table { size } else { header_size };
    if cloned > if table || call_free { 256 } else { 160 } {
        return Ok(None);
    }
    if l.blocks.iter().any(|&b| {
        cfg.successors(b)
            .iter()
            .any(|s| !member[s.index()] && cfg.handler(*s))
    }) {
        return Ok(None);
    }
    Ok(Some(Plan {
        trips,
        member,
        owned,
        through,
    }))
}
