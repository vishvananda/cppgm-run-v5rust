//! Speculated diamond and triangle values from `opt/select_formation.cpp`.
//! See NOTICE.
use super::{
    jump_threading::{incoming, phis},
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstId, Instruction, InstructionKind as I, LinkEditor, Value},
    vocabulary::{Binary as B, Convert as C, Type},
};
fn selectable(ty: Type) -> bool {
    (ty.bits() != 0 || ty == Type::Ptr) && ty != Type::I128
}
fn speculatable(body: &Body, id: InstId) -> bool {
    let inst = body.instruction(id);
    if inst
        .result
        .is_none_or(|t| body.temp(t).ty.is_none_or(|ty| !selectable(ty)))
    {
        return false;
    }
    match inst.kind {
        I::Copy(..) | I::Index(..) | I::Addr(..) => true,
        I::Unary(_, ty, _) | I::Compare(_, ty, ..) | I::Select(ty, ..) => selectable(ty),
        I::Binary(op, _, ty, ..) => {
            selectable(ty) && !matches!(op, B::Div | B::Mod | B::UDiv | B::UMod)
        }
        I::Convert(op, _, from, _) => {
            matches!(op, C::SignExtend | C::ZeroExtend | C::Truncate) && selectable(from)
        }
        _ => false,
    }
}
fn arm(
    context: &Context<'_>,
    links: &LinkEditor,
    touched: &[bool],
    a: BlockId,
    from: BlockId,
) -> Result<Option<BlockId>, Error> {
    let cfg = context.cfg()?;
    if a == from
        || touched[a.index()]
        || cfg.handler(a)
        || cfg.has_eh(a)
        || cfg.predecessors(a) != [from]
    {
        return Ok(None);
    }
    let Some(last) = links.last(a) else {
        return Ok(None);
    };
    let I::Jump(to) = context.body.instruction(last).kind else {
        return Ok(None);
    };
    let mut count = 0;
    let mut next = links.first(a);
    while let Some(i) = next.filter(|&i| i != last) {
        count += 1;
        if count > 2 || !speculatable(context.body, i) {
            return Ok(None);
        }
        next = links.next(i);
    }
    Ok(Some(to))
}
fn hoist(
    body: &mut Body,
    links: &mut LinkEditor,
    arm: BlockId,
    e: BlockId,
    mut after: Option<InstId>,
) -> Result<Option<InstId>, Error> {
    while let Some(i) = links.first(arm).filter(|&i| Some(i) != links.last(arm)) {
        links.unlink(arm, i)?;
        links.insert_after(e, after, i)?;
        body.clear_speculation_facts(i);
        after = Some(i);
    }
    Ok(after)
}
fn form(
    context: &mut Context<'_>,
    links: &mut LinkEditor,
    touched: &mut [bool],
    entry: BlockId,
) -> Result<bool, Error> {
    let Some(term) = links.last(entry) else {
        return Ok(false);
    };
    let I::Branch(cond @ Value::Temp(temp), yes, no) = context.body.instruction(term).kind else {
        return Ok(false);
    };
    if yes == no || context.body.temp(temp).ty.is_none_or(|ty| !selectable(ty)) {
        return Ok(false);
    }
    let jt = arm(context, links, touched, yes, entry)?;
    let jf = arm(context, links, touched, no, entry)?;
    let (join, from_t, from_f) = match (jt, jf) {
        (Some(jt), Some(jf)) if jt == jf => (jt, yes, no),
        (Some(jt), _) if jt == no => (no, yes, entry),
        (_, Some(jf)) if jf == yes => (yes, entry, no),
        _ => return Ok(false),
    };
    if join == entry || touched[join.index()] || context.cfg()?.handler(join) {
        return Ok(false);
    }
    let phis = phis(context.body, join).collect::<Vec<_>>();
    let mut selects = 0;
    for &id in &phis {
        let left = incoming(context.body, id, from_t);
        let right = incoming(context.body, id, from_f);
        if left == right {
            continue;
        }
        selects += 1;
        if selects > 2
            || !selectable(context.body.instruction(id).kind.result_type())
            || left.is_none_or(|v| matches!(v, Value::Slot(_)))
            || right.is_none_or(|v| matches!(v, Value::Slot(_)))
        {
            return Ok(false);
        }
    }
    let mut at = links.previous(term);
    if from_t == yes {
        at = hoist(context.body, links, yes, entry, at)?;
    }
    if from_f == no {
        at = hoist(context.body, links, no, entry, at)?;
    }
    for id in phis {
        let left = incoming(context.body, id, from_t).ok_or(Error::Limit)?;
        let right = incoming(context.body, id, from_f).ok_or(Error::Limit)?;
        let merged = if left == right {
            left
        } else {
            let ty = context.body.instruction(id).kind.result_type();
            let temp = context.body.generated_temp(ty)?;
            let sel = context.body.append(
                entry,
                Instruction::new(Some(temp), I::Select(ty, cond, left, right), 0),
            )?;
            context.body.inherit_location(sel, term);
            links.insert_after(entry, at, sel)?;
            at = Some(sel);
            Value::Temp(temp)
        };
        if from_t == entry || from_f == entry {
            let I::Phi(ty, mut pairs) = context.body.instruction(id).kind.clone() else {
                return Err(Error::Limit);
            };
            for (block, value) in &mut pairs {
                if *block == entry {
                    *value = merged;
                }
            }
            context.body.replace_kind(id, I::Phi(ty, pairs));
        } else {
            context.body.append_phi_predecessor(id, entry, merged)?;
        }
    }
    for left in [(from_t == yes).then_some(yes), (from_f == no).then_some(no)]
        .into_iter()
        .flatten()
    {
        context.body.remove_phi_predecessor(join, left);
        context
            .body
            .replace_kind(links.last(left).ok_or(Error::Limit)?, I::Unreachable);
    }
    context.body.replace_kind(term, I::Jump(join));
    for right in [entry, yes, no, join] {
        touched[right.index()] = true;
    }
    context.branches_folded = context.branches_folded.checked_add(1).ok_or(Error::Limit)?;
    Ok(true)
}
pub(super) fn form_selects(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    let mut touched = vec![false; context.body.blocks().len()];
    let mut links = LinkEditor::new(context.body)?;
    let mut changed = false;
    for b in context.body.layout.clone() {
        if context.cfg()?.reachable(b) && !touched[b.index()] && !context.cfg()?.has_eh(b) {
            changed = form(context, &mut links, &mut touched, b)? || changed;
        }
    }
    if changed {
        links.commit(context.body)?;
        context.invalidate_cfg();
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::super::{pass_context::Pass, pass_view};
    #[test]
    fn speculation_moves_arms_in_order_and_drops_conditional_facts() {
        let source = "function @f(%c:i1,%x:i32)->i32 {block ^entry:branch %c,^yes,^no block ^yes:%a=binary add nowrap i32 %x,1 jump ^merge block ^no:%b=binary shr exact i32 %x,2 jump ^merge block ^merge:%r=phi i32 [^yes:%a,^no:%b] return i32 %r}";
        let (out, changed) = pass_view(source, 2, Pass::FormSelects);
        assert!(changed.0);
        assert!(out.contains("select i32 %c, %a, %b"));
        assert!(!out.contains("nowrap") && !out.contains("exact"));
        assert!(out.find("binary add").unwrap() < out.find("binary shr").unwrap());
        assert!(out.contains("jump ^merge"));
        assert!(
            !pass_view(
                &source.replace("binary add nowrap", "binary div"),
                2,
                Pass::FormSelects
            )
            .1
            .0
        );
    }
    #[test]
    fn triangles_update_the_existing_head_phi_without_duplicate_inputs() {
        let source = "function @f(%c:i1,%x:i32)->i32 {block ^entry:branch %c,^yes,^merge block ^yes:%a=binary add i32 %x,1 jump ^merge block ^merge:%r=phi i32 [^yes:%a,^entry:%x] return i32 %r}";
        let (out, changed) = pass_view(source, 2, Pass::FormSelects);
        assert!(changed.0);
        assert!(out.contains("select i32 %c, %a, %x"));
        assert_eq!(
            out.lines()
                .find(|l| l.contains("phi i32"))
                .unwrap()
                .matches("^entry:")
                .count(),
            1
        );
    }
}
