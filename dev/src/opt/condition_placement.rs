//! Branch and select comparison placement from `opt/jump_threading.cpp`.
//! See NOTICE. Decisions use the pass-entry snapshots, as in the frozen driver.
use super::{
    alias_facts, flow_cleanup,
    jump_threading::{definition, incoming, last, phis, temp_type},
    numeric_fold as fold,
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{
        BlockId, Body, InstId, Instruction, InstructionKind as I, LinkEditor, OptimizerBlock,
        TempId, Value,
    },
    vocabulary::{Binary as B, Compare as C, Type},
};
fn single(value: Value, uses: &[u32]) -> bool {
    matches!(value,Value::Temp(t) if uses.get(t.index())==Some(&1))
}
fn xor_load(body: &Body, value: Value, uses: &[u32]) -> Option<(Value, Value, InstId, Type)> {
    let id = definition(body, value)?;
    let I::Binary(B::Xor, _, ty, left, right) = body.instruction(id).kind else {
        return None;
    };
    if !single(value, uses) {
        return None;
    }
    for (loaded, constant) in [(left, right), (right, left)] {
        if matches!(constant, Value::Integer(_))
            && let Some(load) = definition(body, loaded)
            && matches!(body.instruction(load).kind,I::Load(flags,..) if !flags.volatile())
        {
            return Some((loaded, constant, id, ty));
        }
    }
    None
}
struct OrTest {
    compare: InstId,
    or: InstId,
    xors: [InstId; 2],
    loaded: [Value; 2],
    constants: [Value; 2],
    types: [Type; 2],
    pred: C,
    condition: TempId,
    yes: BlockId,
    no: BlockId,
    second_load: InstId,
    move_load: bool,
    tail: InstId,
}
fn or_test(context: &Context<'_>, block: BlockId) -> Option<OrTest> {
    let body = &*context.body;
    let tail = last(body, block)?;
    let I::Branch(condition @ Value::Temp(temp), yes, no) = body.instruction(tail).kind else {
        return None;
    };
    let compare = definition(body, condition)?;
    let I::Compare(pred, ty, value, zero) = body.instruction(compare).kind else {
        return None;
    };
    if yes == no
        || !single(condition, context.entry_uses())
        || ty.bits() == 0
        || !matches!(pred, C::Eq | C::Ne)
        || !matches!(fold::literal(body, zero), Some(Constant::Integer(0)))
    {
        return None;
    }
    let or = definition(body, value)?;
    let I::Binary(B::Or, _, _, a, b) = body.instruction(or).kind else {
        return None;
    };
    if !single(value, context.entry_uses()) {
        return None;
    }
    let (l0, k0, x0, t0) = xor_load(body, a, context.entry_uses())?;
    let (l1, k1, x1, t1) = xor_load(body, b, context.entry_uses())?;
    let second_load = definition(body, l1)?;
    let ids = &body.block(block).instructions;
    if [compare, or, x0, x1].iter().any(|id| !ids.contains(id)) {
        return None;
    }
    let mut past = false;
    let mut writes = false;
    for &id in ids {
        if past && id != tail {
            writes |= alias_facts::writes(context.program, &body.instruction(id).kind);
        }
        past |= id == second_load;
    }
    let move_load = ids.contains(&second_load) && single(l1, context.entry_uses()) && !writes;
    Some(OrTest {
        compare,
        or,
        xors: [x0, x1],
        loaded: [l0, l1],
        constants: [k0, k1],
        types: [t0, t1],
        pred,
        condition: temp,
        yes,
        no,
        second_load,
        move_load,
        tail,
    })
}
fn split_or(
    context: &mut Context<'_>,
    links: &mut LinkEditor,
    block: BlockId,
    test: &OrTest,
) -> Result<BlockId, Error> {
    let split = context.body.optimizer_block(OptimizerBlock::Split)?;
    let equal = test.pred == C::Eq;
    context.body.replace_kind(
        test.compare,
        I::Compare(test.pred, test.types[0], test.loaded[0], test.constants[0]),
    );
    context.body.replace_kind(
        test.tail,
        I::Branch(
            Value::Temp(test.condition),
            if equal { split } else { test.yes },
            if equal { test.no } else { split },
        ),
    );
    for id in [test.or, test.xors[0], test.xors[1]] {
        links.unlink(block, id)?;
    }
    let mut after = None;
    if test.move_load {
        links.unlink(block, test.second_load)?;
        links.insert_after(split, None, test.second_load)?;
        after = Some(test.second_load);
    }
    let temp = context
        .body
        .generated_temp(temp_type(context.body, test.condition)?)?;
    let compare = context.body.append(
        split,
        Instruction::new(
            Some(temp),
            I::Compare(test.pred, test.types[1], test.loaded[1], test.constants[1]),
            0,
        ),
    )?;
    context.body.inherit_location(compare, test.tail);
    links.insert_after(split, after, compare)?;
    let branch = context.body.append(
        split,
        Instruction::new(None, I::Branch(Value::Temp(temp), test.yes, test.no), 0),
    )?;
    context.body.inherit_location(branch, test.tail);
    links.insert_after(split, Some(compare), branch)?;
    let mismatch = if equal { test.no } else { test.yes };
    context.body.rename_phi_predecessor(
        if mismatch == test.no {
            test.yes
        } else {
            test.no
        },
        block,
        split,
    );
    for phi in phis(context.body, mismatch).collect::<Vec<_>>() {
        let value = incoming(context.body, phi, block)
            .ok_or_else(|| Error::Construction("missing split branch phi input".into()))?;
        context.body.append_phi_predecessor(phi, split, value)?;
    }
    Ok(split)
}
pub(super) fn split_or_branches(context: &mut Context<'_>) -> Result<bool, Error> {
    context.prepare_uses(Pass::SplitOrBranches, true)?;
    let mut layout = Vec::new();
    let mut links = LinkEditor::new(context.body)?;
    let mut changed = false;
    for block in context.body.layout.clone() {
        layout.push(block);
        if let Some(test) = or_test(context, block) {
            let split = split_or(context, &mut links, block, &test)?;
            layout.push(split);
            changed = true;
        }
    }
    if changed {
        context.body.layout = layout;
        links.commit(context.body)?;
        context.invalidate_cfg();
    }
    Ok(changed)
}
struct PhiTest {
    phi: InstId,
    compare: InstId,
    branch: InstId,
    side: usize,
    preds: Vec<BlockId>,
}
fn phi_test(context: &Context<'_>, block: BlockId) -> Result<Option<PhiTest>, Error> {
    let body = &*context.body;
    let cfg = context.cfg()?;
    let count = cfg.predecessors(block).len();
    if !(2..=4).contains(&count) || body.layout.first() == Some(&block) {
        return Ok(None);
    }
    let [phi, compare, branch] = body.block(block).instructions.as_slice() else {
        return Ok(None);
    };
    let phi_inst = body.instruction(*phi);
    let I::Phi(ty, _) = phi_inst.kind else {
        return Ok(None);
    };
    let result = phi_inst.result.ok_or(Error::Limit)?;
    if ty.bits() == 0 || !single(Value::Temp(result), context.entry_uses()) {
        return Ok(None);
    }
    let cmp = body.instruction(*compare);
    let I::Compare(_, _, left, right) = cmp.kind else {
        return Ok(None);
    };
    let temp = cmp.result.ok_or(Error::Limit)?;
    if !single(Value::Temp(temp), context.entry_uses()) {
        return Ok(None);
    }
    let I::Branch(condition, yes, no) = body.instruction(*branch).kind else {
        return Ok(None);
    };
    if condition != Value::Temp(temp) {
        return Ok(None);
    }
    let value = Value::Temp(result);
    let side = if left == value {
        0
    } else if right == value {
        1
    } else {
        return Ok(None);
    };
    if left == right
        || yes == no
        || phis(body, yes).next().is_some()
        || phis(body, no).next().is_some()
        || cfg.handler(block)
    {
        return Ok(None);
    }
    let mut preds = Vec::new();
    for &pred in cfg.predecessors(block) {
        if !preds.contains(&pred) {
            preds.push(pred);
        }
    }
    if preds.iter().any(|&pred| {
        cfg.handler(pred)
            || last(body, pred)
                .is_none_or(|id| !matches!(body.instruction(id).kind, I::Branch(..) | I::Jump(_)))
    }) {
        return Ok(None);
    }
    Ok(Some(PhiTest {
        phi: *phi,
        compare: *compare,
        branch: *branch,
        side,
        preds,
    }))
}
pub(super) fn specialize_phi_compares(context: &mut Context<'_>) -> Result<bool, Error> {
    context.prepare_uses(Pass::SpecializePhiCompares, true)?;
    context.ensure_cfg()?;
    let mut extra = Vec::new();
    let mut changed = false;
    for block in context.body.layout.clone() {
        let Some(test) = phi_test(context, block)? else {
            continue;
        };
        let cmp = context.body.instruction(test.compare).clone();
        let branch = context.body.instruction(test.branch).clone();
        for pred in test.preds {
            let value = incoming(context.body, test.phi, pred)
                .ok_or_else(|| Error::Construction("missing specialized phi input".into()))?;
            let fresh = context.body.optimizer_block(OptimizerBlock::Thread)?;
            let mut instruction = cmp.clone();
            let old = instruction.result.ok_or(Error::Limit)?;
            let result = context.body.generated_temp(temp_type(context.body, old)?)?;
            instruction.result = Some(result);
            let I::Compare(_, _, left, right) = &mut instruction.kind else {
                return Err(Error::Limit);
            };
            if test.side == 0 {
                *left = value;
            } else {
                *right = value;
            }
            context.body.append(fresh, instruction)?;
            let mut instruction = branch.clone();
            let I::Branch(condition, _, _) = &mut instruction.kind else {
                return Err(Error::Limit);
            };
            *condition = Value::Temp(result);
            context.body.append(fresh, instruction)?;
            context.body.redirect_successors(
                last(context.body, pred).ok_or(Error::Limit)?,
                block,
                fresh,
            );
            extra.push(fresh);
        }
        changed = true;
    }
    if changed {
        context.body.layout.extend(extra);
        context.invalidate_cfg();
        flow_cleanup::cleanup(context)?;
    }
    Ok(changed)
}
fn clone_before(
    context: &mut Context<'_>,
    links: &mut LinkEditor,
    block: BlockId,
    before: InstId,
    source: InstId,
    temp: TempId,
) -> Result<Value, Error> {
    let mut copy = context.body.instruction(source).clone();
    let result = context
        .body
        .generated_temp(temp_type(context.body, temp)?)?;
    copy.result = Some(result);
    let id = context.body.append(block, copy)?;
    links.insert_after(block, links.previous(before), id)?;
    Ok(Value::Temp(result))
}
fn comparisons(
    context: &mut Context<'_>,
    branches: &[u32],
    selects: &[u32],
) -> Result<bool, Error> {
    let mut changed = false;
    let mut links = LinkEditor::new(context.body)?;
    for block in context.body.layout.clone() {
        let Some(tail) = last(context.body, block) else {
            continue;
        };
        let I::Branch(condition @ Value::Temp(temp), yes, no) = context.body.instruction(tail).kind
        else {
            continue;
        };
        let Some(def) = definition(context.body, condition) else {
            continue;
        };
        if temp.index() >= branches.len()
            || context.entry_uses()[temp.index()] < 1
            || branches[temp.index()] + selects[temp.index()] != context.entry_uses()[temp.index()]
            || def.index() >= context.block_map().len()
            || context.block_map()[def.index()] == Some(block)
        {
            continue;
        }
        let I::Compare(_, ty, _, _) = context.body.instruction(def).kind else {
            continue;
        };
        if matches!(ty, Type::I128 | Type::F80) {
            continue;
        }
        let value = clone_before(context, &mut links, block, tail, def, temp)?;
        context.body.replace_kind(tail, I::Branch(value, yes, no));
        changed = true;
    }
    for block in context.body.layout.clone() {
        let mut cursor = links.first(block);
        while let Some(id) = cursor {
            cursor = links.next(id);
            let I::Select(ty, condition @ Value::Temp(temp), yes, no) =
                context.body.instruction(id).kind
            else {
                continue;
            };
            let Some(def) = definition(context.body, condition) else {
                continue;
            };
            if Some(def) == links.previous(id)
                || context
                    .entry_uses()
                    .get(temp.index())
                    .is_none_or(|&n| n < 1)
                || branches.get(temp.index()).copied().unwrap_or(0)
                    + selects.get(temp.index()).copied().unwrap_or(0)
                    != context.entry_uses()[temp.index()]
            {
                continue;
            }
            let I::Compare(_, from, _, _) = context.body.instruction(def).kind else {
                continue;
            };
            if matches!(from, Type::I128 | Type::F80) || from.float_rank() != 0 {
                continue;
            }
            let value = clone_before(context, &mut links, block, id, def, temp)?;
            context.body.replace_kind(id, I::Select(ty, value, yes, no));
            changed = true;
        }
    }
    links.commit(context.body)?;
    Ok(changed)
}
fn bit_tests(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut changed = false;
    let mut links = LinkEditor::new(context.body)?;
    for block in context.body.layout.clone() {
        let Some(tail) = last(context.body, block) else {
            continue;
        };
        let I::Branch(Value::Temp(mut temp), _, _) = context.body.instruction(tail).kind else {
            continue;
        };
        if let Some(def) = definition(context.body, Value::Temp(temp))
            && let I::Compare(pred, _, left, zero) = context.body.instruction(def).kind
        {
            if context.entry_uses().get(temp.index()) != Some(&1)
                || context.block_map().get(def.index()) != Some(&Some(block))
                || !matches!(pred, C::Eq | C::Ne)
                || !matches!(left, Value::Temp(_))
                || !matches!(
                    fold::literal(context.body, zero),
                    Some(Constant::Integer(0))
                )
            {
                continue;
            }
            let Value::Temp(left) = left else {
                return Err(Error::Limit);
            };
            temp = left;
        }
        let Some(and) = definition(context.body, Value::Temp(temp)) else {
            continue;
        };
        let I::Binary(B::And, flags, ty, left, right) = context.body.instruction(and).kind else {
            continue;
        };
        if context.entry_uses().get(temp.index()) != Some(&1)
            || context.block_map().get(and.index()) != Some(&Some(block))
        {
            continue;
        }
        for (side, value) in [left, right].into_iter().enumerate() {
            let Value::Temp(mask) = value else {
                continue;
            };
            let Some(shl) = definition(context.body, value) else {
                continue;
            };
            let I::Binary(B::Shl, _, from, one, _) = context.body.instruction(shl).kind else {
                continue;
            };
            if from != ty || !matches!(fold::literal(context.body, one), Some(Constant::Integer(1)))
            {
                continue;
            }
            if context.entry_uses().get(mask.index()) == Some(&1)
                && context.block_map().get(shl.index()) == Some(&Some(block))
            {
                break;
            }
            let value = clone_before(context, &mut links, block, and, shl, mask)?;
            context.body.replace_kind(
                and,
                I::Binary(
                    B::And,
                    flags,
                    ty,
                    if side == 0 { value } else { left },
                    if side == 1 { value } else { right },
                ),
            );
            changed = true;
            break;
        }
    }
    links.commit(context.body)?;
    Ok(changed)
}
pub(super) fn split_branch_compares(context: &mut Context<'_>) -> Result<bool, Error> {
    context.prepare_uses(Pass::SplitBranchCompares, true)?;
    context.prepare_block_map(Pass::SplitBranchCompares, true);
    let mut branches = vec![0_u32; context.body.temps().len()];
    let mut selects = branches.clone();
    for &block in &context.body.layout {
        if let Some(tail) = last(context.body, block)
            && let I::Branch(Value::Temp(temp), _, _) = context.body.instruction(tail).kind
        {
            branches[temp.index()] = branches[temp.index()].checked_add(1).ok_or(Error::Limit)?;
        }
        for &id in &context.body.block(block).instructions {
            if let I::Select(_, Value::Temp(temp), _, _) = context.body.instruction(id).kind {
                selects[temp.index()] = selects[temp.index()].checked_add(1).ok_or(Error::Limit)?;
            }
        }
    }
    let changed = comparisons(context, &branches, &selects)?;
    if changed {
        context.prepare_uses(Pass::SplitBranchCompares, false)?;
        context.prepare_block_map(Pass::SplitBranchCompares, false);
        context.invalidate_uses();
        context.invalidate_block_map();
    }
    Ok(bit_tests(context)? || changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunk_mismatches_gain_an_edge_while_the_second_load_moves() {
        let (out, facts) = super::super::pass_view(
            "function @f(%p : ptr,%q : ptr)->i64 {block ^entry:%l0 = load i64 %p %l1 = load i64 %q %x0 = binary xor i64 %l0, 5 %x1 = binary xor i64 %l1, 7 %orr = binary or i64 %x0, %x1 %cmp = cmp eq i64 %orr, 0 branch %cmp, ^yes, ^no block ^yes:%a = phi i64 [^entry: 1] return i64 %a block ^no:%b = phi i64 [^entry: 0] return i64 %b}",
            2,
            Pass::SplitOrBranches,
        );
        assert!(facts.0);
        assert!(!out.contains("binary xor"));
        assert!(out.contains("block ^split_1:\n    %l1 = load i64 %q"));
        assert!(out.contains("phi i64 [^entry: 0, ^split_1: 0]"));
        assert!(out.contains("phi i64 [^split_1: 1]"));
    }
    #[test]
    fn comparisons_are_placed_at_branch_and_nonadjacent_select_uses() {
        let (out, facts) = super::super::pass_view(
            "function @f(%v : i64,%w : i64,%c : i64)->i64 {block ^entry:%cmp = cmp eq i64 %v, %w branch %c, ^a, ^b block ^a:%sel = select i64 %cmp, 3, 7 branch %cmp, ^yes, ^no block ^b:branch %cmp, ^yes, ^no block ^yes:return i64 1 block ^no:return i64 0}",
            2,
            Pass::SplitBranchCompares,
        );
        assert!(facts.0);
        assert_eq!(out.matches("cmp eq i64 %v, %w").count(), 4);
    }
}
