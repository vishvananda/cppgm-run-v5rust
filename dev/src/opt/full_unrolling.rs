//! Bounded straight and branching loop copies from `opt/loop_passes.cpp`. See NOTICE.
use super::{
    induction::Facts,
    loop_motion,
    natural_loops::{Forest, LoopId},
    pass_context::{Context, Pass},
    unroll_admission,
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, InstId, Instruction, InstructionKind as I, OptimizerBlock, TempId, Value,
    },
};

pub(super) fn phis(body: &Body, block: BlockId) -> Vec<InstId> {
    body.block(block)
        .instructions
        .iter()
        .copied()
        .take_while(|&id| matches!(body.instruction(id).kind, I::Phi(..)))
        .collect()
}
pub(super) fn incoming(body: &Body, id: InstId, from: BlockId) -> Option<Value> {
    let I::Phi(_, pairs) = &body.instruction(id).kind else {
        return None;
    };
    pairs.iter().find(|&&(b, _)| b == from).map(|&(_, v)| v)
}
pub(super) fn mapped(map: &[Option<Value>], value: Value) -> Value {
    if let Value::Temp(t) = value {
        map.get(t.index()).copied().flatten().unwrap_or(value)
    } else {
        value
    }
}
fn header_values(
    body: &Body,
    header: BlockId,
    latch: BlockId,
    map: &mut [Option<Value>],
) -> Result<Vec<(TempId, Value)>, Error> {
    let mut back = Vec::new();
    for id in phis(body, header) {
        let inst = body.instruction(id);
        let I::Phi(_, pairs) = &inst.kind else {
            return Err(Error::Limit);
        };
        let result = inst.result.ok_or(Error::Limit)?;
        for &(from, v) in pairs {
            if from == latch {
                back.push((result, v));
            } else {
                map[result.index()] = Some(v);
            }
        }
    }
    Ok(back)
}
fn advance(map: &mut [Option<Value>], back: &[(TempId, Value)]) {
    let next = back
        .iter()
        .map(|&(_, v)| mapped(map, v))
        .collect::<Vec<_>>();
    for (&(t, _), value) in back.iter().zip(next) {
        map[t.index()] = Some(value);
    }
}
pub(super) fn copy(
    body: &mut Body,
    target: BlockId,
    id: InstId,
    map: &mut [Option<Value>],
    blocks: impl FnMut(BlockId) -> BlockId,
) -> Result<(), Error> {
    let old = body.instruction(id).result;
    let copied = body.copy_instruction(target, id, |v| mapped(map, v), blocks)?;
    if let Some(old) = old {
        map[old.index()] = body.instruction(copied).result.map(Value::Temp);
    }
    Ok(())
}
fn straight(context: &mut Context<'_>, forest: &Forest, loop_: LoopId) -> Result<bool, Error> {
    let l = forest.get(loop_);
    let Some(pre) = l.preheader else {
        return Ok(false);
    };
    if l.latches.len() != 1 {
        return Ok(false);
    }
    let facts = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    };
    let Some(analysis) = facts.exit(loop_) else {
        return Ok(false);
    };
    let Some(trips) = analysis.test.trips.filter(|&n| (0..=4).contains(&n)) else {
        return Ok(false);
    };
    if analysis.test.exiting != l.header {
        return Ok(false);
    }
    let exit = analysis.test.target;
    let last = *context
        .body
        .block(l.header)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    let I::Branch(_, yes, no) = context.body.instruction(last).kind else {
        return Ok(false);
    };
    let mut at = if yes == exit { no } else { yes };
    let mut chain = Vec::new();
    while at != l.header {
        if !forest.contains(loop_, at)
            || chain.len() > l.blocks.len()
            || context.cfg()?.predecessors(at).len() != 1
        {
            return Ok(false);
        }
        let id = *context
            .body
            .block(at)
            .instructions
            .last()
            .ok_or(Error::Limit)?;
        let I::Jump(next) = context.body.instruction(id).kind else {
            return Ok(false);
        };
        chain.push(at);
        at = next;
    }
    if chain.len() + 1 != l.blocks.len() {
        return Ok(false);
    }
    let mut size = 0_u64;
    for &b in &l.blocks {
        for &id in &context.body.block(b).instructions {
            let kind = &context.body.instruction(id).kind;
            if unroll_admission::forbidden(kind) {
                return Ok(false);
            }
            size += u64::from(!matches!(kind, I::Phi(..)) && !kind.terminator());
        }
    }
    if size * u64::try_from(trips + 1).map_err(|_| Error::Limit)? > 64 {
        return Ok(false);
    }
    let target = context.body.optimizer_block(OptimizerBlock::Unrolled)?;
    let mut map = vec![None; context.body.temps().len()];
    let back = header_values(context.body, l.header, l.latches[0], &mut map)?;
    for pass in 0..=trips {
        for b in std::iter::once(l.header).chain(chain.iter().copied().filter(|_| pass < trips)) {
            for id in context.body.block(b).instructions.clone() {
                let kind = &context.body.instruction(id).kind;
                if matches!(kind, I::Phi(..)) || kind.terminator() {
                    continue;
                }
                copy(context.body, target, id, &mut map, |b| b)?;
            }
        }
        if pass < trips {
            advance(&mut map, &back);
        }
    }
    context
        .body
        .append(target, Instruction::new(None, I::Jump(exit), 0))?;
    let last = *context
        .body
        .block(pre)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    context.body.redirect_successors(last, l.header, target);
    context.body.rename_phi_predecessor(exit, l.header, target);
    for b in context.body.layout.clone() {
        if forest.contains(loop_, b) {
            continue;
        }
        for id in context.body.block(b).instructions.clone() {
            context.body.rewrite_values(id, |v| Ok(mapped(&map, v)))?;
        }
    }
    replace_layout(context.body, forest, loop_, &[target], &[]);
    context.loops_unrolled += 1;
    Ok(true)
}
fn replace_layout(
    body: &mut Body,
    forest: &Forest,
    loop_: LoopId,
    copied: &[BlockId],
    owned: &[bool],
) {
    let mut layout = Vec::new();
    for &b in &body.layout {
        if b == forest.get(loop_).header {
            layout.extend_from_slice(copied);
        }
        if !forest.contains(loop_, b) && !owned.get(b.index()).copied().unwrap_or(false) {
            layout.push(b);
        }
    }
    body.layout = layout;
}
/// Add the one exit's transport phis before copying, with each outside read
/// captured by instruction and typed operand ordinal, including call arguments.
fn exit_phis(
    context: &mut Context<'_>,
    member: &[bool],
    owned: &[bool],
    exit: BlockId,
) -> Result<(), Error> {
    let body = &*context.body;
    let loop_value = |v| unroll_admission::loop_value(context, member, v);
    let mut reads = Vec::new();
    for &b in &body.layout {
        if member[b.index()] || owned[b.index()] {
            continue;
        }
        for &id in &body.block(b).instructions {
            let kind = &body.instruction(id).kind;
            if let I::Phi(_, pairs) = kind {
                for (k, &(from, v)) in pairs.iter().enumerate() {
                    if !member[from.index()] && loop_value(v) {
                        reads.push((id, k, v));
                    }
                }
            } else {
                let mut k = 0;
                body.visit_values(kind, |v| {
                    if loop_value(v) {
                        reads.push((id, k, v));
                    }
                    k += 1;
                });
            }
        }
    }
    let mut map = vec![None; body.temps().len()];
    let predecessors = context.cfg()?.predecessors(exit).to_vec();
    for &(_, _, v) in &reads {
        let Value::Temp(t) = v else {
            return Err(Error::Limit);
        };
        if map[t.index()].is_some() {
            continue;
        }
        let ty = context.body.temp(t).ty.ok_or(Error::Limit)?;
        let result = context.body.generated_temp(ty)?;
        let pairs = predecessors
            .iter()
            .map(|&p| {
                (
                    p,
                    if member[p.index()] {
                        v
                    } else {
                        Value::Temp(result)
                    },
                )
            })
            .collect();
        let id = context
            .body
            .append(exit, Instruction::new(Some(result), I::Phi(ty, pairs), 0))?;
        context
            .body
            .inherit_location(id, context.body.temp(t).definition.ok_or(Error::Limit)?);
        let mut links = context.body.block(exit).instructions.clone();
        links.pop();
        links.insert(0, id);
        context.body.replace_links(exit, links)?;
        map[t.index()] = Some(Value::Temp(result));
    }
    for (id, at, v) in reads {
        let mut k = 0;
        context.body.rewrite_values(id, |old| {
            let value = if k == at { mapped(&map, v) } else { old };
            k += 1;
            Ok(value)
        })?;
    }
    Ok(())
}
fn branching(context: &mut Context<'_>, forest: &Forest, loop_: LoopId) -> Result<bool, Error> {
    let Some(plan) = unroll_admission::branching(context, forest, loop_)? else {
        return Ok(false);
    };
    let l = forest.get(loop_);
    let Some(pre) = l.preheader else {
        return loop_motion::make_preheader(context, forest, loop_);
    };
    if let Some(exit) = plan.through {
        exit_phis(context, &plan.member, &plan.owned, exit)?;
    }
    let order = context
        .cfg()?
        .reverse_postorder()
        .iter()
        .copied()
        .filter(|b| plan.member[b.index()] || plan.owned[b.index()])
        .collect::<Vec<_>>();
    let original_layout = context.body.layout.clone();
    let mut map = vec![None; context.body.temps().len()];
    let back = header_values(context.body, l.header, l.latches[0], &mut map)?;
    let count = context.body.blocks().len();
    let mut passes = Vec::new();
    let mut copied = Vec::new();
    for pass in 0..=plan.trips {
        let mut mine = vec![None; count];
        for &b in &original_layout {
            if (plan.member[b.index()] || plan.owned[b.index()])
                && (pass < plan.trips || b == l.header || plan.owned[b.index()])
            {
                let fresh = context.body.unrolled_block(b)?;
                mine[b.index()] = Some(fresh);
                copied.push(fresh);
            }
        }
        passes.push(mine);
    }
    let dead = context.body.optimizer_block(OptimizerBlock::Dead)?;
    context
        .body
        .append(dead, Instruction::new(None, I::Unreachable, 0))?;
    copied.push(dead);
    let mut exit_pairs: Vec<Vec<(InstId, BlockId, Value)>> = vec![Vec::new(); count];
    let mut exits = Vec::new();
    for (pass, mine) in passes.iter().enumerate() {
        let target = |t: BlockId| {
            if t == l.header {
                passes
                    .get(pass + 1)
                    .and_then(|next| next[t.index()])
                    .unwrap_or(dead)
            } else if plan.member[t.index()] || plan.owned[t.index()] {
                mine[t.index()].unwrap_or(dead)
            } else {
                t
            }
        };
        for &b in &order {
            let Some(fresh) = mine[b.index()] else {
                continue;
            };
            for id in context.body.block(b).instructions.clone() {
                let phi = matches!(context.body.instruction(id).kind, I::Phi(..));
                if phi && b == l.header {
                    continue;
                }
                copy(context.body, fresh, id, &mut map, |t| {
                    if phi {
                        mine[t.index()].unwrap_or(dead)
                    } else {
                        target(t)
                    }
                })?;
            }
            if !plan.member[b.index()] {
                continue;
            }
            let successors = context.cfg()?.successors(b).to_vec();
            for t in successors {
                if plan.member[t.index()] || plan.owned[t.index()] {
                    continue;
                }
                for id in phis(context.body, t) {
                    if exit_pairs[t.index()].is_empty() {
                        exits.push(t);
                    }
                    let value = incoming(context.body, id, b).ok_or(Error::Limit)?;
                    exit_pairs[t.index()].push((id, fresh, mapped(&map, value)));
                }
            }
        }
        if pass + 1 < passes.len() {
            advance(&mut map, &back);
        }
    }
    for t in exits {
        for id in phis(context.body, t) {
            let mut inst = context.body.instruction(id).clone();
            let I::Phi(_, pairs) = &mut inst.kind else {
                return Err(Error::Limit);
            };
            pairs.retain(|&(b, _)| !plan.member[b.index()]);
            pairs.extend(
                exit_pairs[t.index()]
                    .iter()
                    .filter(|&&(phi, _, _)| phi == id)
                    .map(|&(_, b, v)| (b, v)),
            );
            context.body.update_instruction(id, inst);
        }
    }
    let last = *context
        .body
        .block(pre)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    context.body.redirect_successors(
        last,
        l.header,
        passes[0][l.header.index()].ok_or(Error::Limit)?,
    );
    replace_layout(context.body, forest, loop_, &copied, &plan.owned);
    context.loops_unrolled += 1;
    Ok(true)
}
pub(super) fn run(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    let mut forest = Forest::default();
    forest.find(context.body.blocks().len(), context.cfg()?)?;
    context.prepare_block_map(Pass::UnrollLoops, true);
    let mut any = false;
    for _ in 0..16 {
        let mut one = false;
        for loop_ in forest.ids().rev() {
            if straight(context, &forest, loop_)?
                || context.level >= 3 && branching(context, &forest, loop_)?
            {
                one = true;
                break;
            }
        }
        if !one {
            break;
        }
        any = true;
        context.invalidate_cfg();
        context.ensure_cfg()?;
        forest.find(context.body.blocks().len(), context.cfg()?)?;
        context.prepare_block_map(Pass::UnrollLoops, false);
    }
    Ok(any)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn header_phis_advance_simultaneously() {
        let mut program = crate::lowir::test_program(
            b"function @f(%a:i64,%b:i64)->i64 {block ^entry:return i64 %a}",
        )
        .unwrap();
        let symbol = program.publications()[0];
        let body = program.take_body(symbol).unwrap().unwrap();
        let a = body.parameter_value(0).unwrap();
        let b = body.parameter_value(1).unwrap();
        let (Value::Temp(a_id), Value::Temp(b_id)) = (a, b) else {
            panic!("parameters")
        };
        let mut map = vec![Some(a), Some(b)];
        advance(&mut map, &[(a_id, b), (b_id, a)]);
        assert_eq!(map, [Some(b), Some(a)]);
    }
    #[test]
    fn copying_a_call_owns_its_operands_and_preserves_the_source() {
        let mut program = crate::lowir::test_program(b"declare function @g(%v:i64)->i64 function @f(%v:i64)->i64 {block ^entry:%r=call i64 @g(%v) return i64 %r}").unwrap();
        let symbol = program.publications()[1];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let entry = body.layout[0];
        let source = body.block(entry).instructions[0];
        let seven = body.integer(7).unwrap();
        let target = body.optimizer_block(OptimizerBlock::Unrolled).unwrap();
        let copied = body
            .copy_instruction(
                target,
                source,
                |v| {
                    if matches!(v, Value::Temp(_)) {
                        seven
                    } else {
                        v
                    }
                },
                |b| b,
            )
            .unwrap();
        let I::Call(original) = &body.instruction(source).kind else {
            panic!("call")
        };
        assert_eq!(
            body.arguments(original.arguments),
            [body.parameter_value(0).unwrap()]
        );
        let I::Call(again) = &body.instruction(copied).kind else {
            panic!("call")
        };
        assert_eq!(body.arguments(again.arguments), [seven]);
        assert_ne!(
            body.instruction(source).result,
            body.instruction(copied).result
        );
    }
}
