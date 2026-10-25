//! Bounded comparison regions from `opt/switch_formation.cpp`. See NOTICE.
use super::{
    jump_threading::{definition, last},
    numeric_fold as fold,
    pass_context::{Context, Pass},
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstId, InstructionKind as I, TempId, Value},
    vocabulary::{Compare as C, Type},
};

type Ranges = Vec<(i64, i64)>;
fn bounds(ty: Type) -> Option<(i64, i64)> {
    Some(match ty {
        Type::I8 => (i64::from(i8::MIN), i64::from(i8::MAX)),
        Type::U8 => (0, i64::from(u8::MAX)),
        Type::I16 => (i64::from(i16::MIN), i64::from(i16::MAX)),
        Type::U16 => (0, i64::from(u16::MAX)),
        Type::I32 => (i64::from(i32::MIN), i64::from(i32::MAX)),
        Type::U32 => (0, i64::from(u32::MAX)),
        Type::I64 => (i64::MIN, i64::MAX),
        _ => return None,
    })
}
fn integer(body: &Body, ty: Type, value: Value) -> Option<i64> {
    let bits = fold::integer_bits(ty, fold::literal(body, value)?)?;
    Some(i64::from_le_bytes(bits.to_le_bytes()[..8].try_into().ok()?))
}
fn clip(set: &Ranges, lo: i64, hi: i64) -> Ranges {
    set.iter()
        .filter_map(|&(a, b)| {
            let (a, b) = (a.max(lo), b.min(hi));
            (a <= b).then_some((a, b))
        })
        .collect()
}
fn unite(a: &Ranges, b: &Ranges) -> Ranges {
    let mut all = a.clone();
    all.extend_from_slice(b);
    all.sort_unstable();
    let mut out: Ranges = Vec::new();
    for (a, b) in all {
        if let Some(last) = out.last_mut()
            && a <= last.1.saturating_add(1)
        {
            last.1 = last.1.max(b);
            continue;
        }
        out.push((a, b));
    }
    out
}
fn subtract(all: &Ranges, part: &Ranges, lo: i64, hi: i64) -> Ranges {
    let mut out = Vec::new();
    let mut from = lo;
    for &(a, b) in part {
        if a > from {
            out = unite(&out, &clip(all, from, a - 1));
        }
        if b == hi {
            return out;
        }
        from = b + 1;
    }
    unite(&out, &clip(all, from, hi))
}
fn count(set: &Ranges) -> u64 {
    const CAP: u64 = 1 << 62;
    set.iter().fold(0, |sum, &(a, b)| {
        let width = b.cast_unsigned().wrapping_sub(a.cast_unsigned());
        (sum + if width >= CAP { CAP } else { width + 1 }).min(CAP)
    })
}
#[derive(Clone, Copy)]
struct Test {
    x: TempId,
    ty: Type,
    term: InstId,
    pred: Option<C>,
    k: i64,
    yes: BlockId,
    no: BlockId,
    cost: usize,
}
fn switch_test(body: &Body, block: BlockId, head: bool) -> Option<Test> {
    let term = last(body, block)?;
    let inst = body.instruction(term);
    let I::Switch(Value::Temp(x), default, pairs) = &inst.kind else {
        return None;
    };
    let ty = body.temp(*x).ty?;
    bounds(ty)?;
    if !head && body.block(block).instructions.len() != 1
        || inst.switch_type().is_some_and(|t| t != ty)
    {
        return None;
    }
    let mut cases = pairs
        .iter()
        .map(|&(v, b)| Some((integer(body, ty, v)?, b)))
        .collect::<Option<Vec<_>>>()?;
    cases.sort_by_key(|&(v, _)| v);
    let cost = cases
        .iter()
        .enumerate()
        .filter(|&(i, &(v, b))| {
            i == 0 || b != cases[i - 1].1 || v != cases[i - 1].0.wrapping_add(1)
        })
        .count();
    Some(Test {
        x: *x,
        ty,
        term,
        pred: None,
        k: 0,
        yes: *default,
        no: *default,
        cost,
    })
}
fn test(context: &Context<'_>, block: BlockId, head: bool) -> Option<Test> {
    let body = &*context.body;
    let term = last(body, block)?;
    if matches!(body.instruction(term).kind, I::Switch(..)) {
        return switch_test(body, block, head);
    }
    let I::Branch(condition @ Value::Temp(t), yes, no) = body.instruction(term).kind else {
        return None;
    };
    if yes == no || context.entry_uses().get(t.index()) != Some(&1) {
        return None;
    }
    let compare = definition(body, condition)?;
    let I::Compare(mut pred, ty, mut x, mut k) = body.instruction(compare).kind else {
        return None;
    };
    if context.block_map()[compare.index()] != Some(block)
        || !head && body.block(block).instructions != [compare, term]
    {
        return None;
    }
    if !matches!(x, Value::Temp(_)) {
        std::mem::swap(&mut x, &mut k);
        pred = pred.swapped();
    }
    let Value::Temp(x) = x else { return None };
    if body.temp(x).ty != Some(ty) {
        return None;
    }
    let signed = bounds(ty)?.0 < 0;
    if matches!(pred, C::Lt | C::Le | C::Gt | C::Ge) && !signed
        || matches!(pred, C::Ult | C::Ule | C::Ugt | C::Uge) && signed
    {
        return None;
    }
    Some(Test {
        x,
        ty,
        term,
        pred: Some(pred),
        k: integer(body, ty, k)?,
        yes,
        no,
        cost: 1,
    })
}
fn sides(body: &Body, test: Test, entry: &Ranges, lo: i64, hi: i64) -> Vec<(BlockId, Ranges)> {
    let Some(pred) = test.pred else {
        let I::Switch(_, _, pairs) = &body.instruction(test.term).kind else {
            return Vec::new();
        };
        let mut taken = Vec::new();
        let mut out = Vec::with_capacity(pairs.len() + 1);
        for &(v, b) in pairs {
            // Admission checked every case at the selector's durable type.
            if let Some(v) = integer(body, test.ty, v) {
                let one = subtract(&clip(entry, v, v), &taken, lo, hi);
                taken = unite(&taken, &one);
                out.push((b, one));
            }
        }
        out.push((test.no, subtract(entry, &taken, lo, hi)));
        return out;
    };
    let at = clip(entry, test.k, test.k);
    let yes = match pred {
        C::Eq => at,
        C::Ne => subtract(entry, &at, lo, hi),
        C::Lt | C::Ult if test.k == lo => Vec::new(),
        C::Lt | C::Ult => clip(entry, lo, test.k - 1),
        C::Le | C::Ule => clip(entry, lo, test.k),
        C::Gt | C::Ugt if test.k == hi => Vec::new(),
        C::Gt | C::Ugt => clip(entry, test.k + 1, hi),
        _ => clip(entry, test.k, hi),
    };
    let no = subtract(entry, &yes, lo, hi);
    vec![(test.yes, yes), (test.no, no)]
}
fn region(
    context: &Context<'_>,
    head: BlockId,
    consumed: &[bool],
) -> Option<(Vec<BlockId>, Vec<Test>)> {
    let first = test(context, head, true)?;
    let cfg = context.cfg().ok()?;
    let mut region = vec![head];
    let mut tests = vec![first];
    let mut grew = true;
    while grew && region.len() < 24 {
        grew = false;
        let mut r = 0;
        while r < region.len() && region.len() < 24 {
            let t = tests[r];
            let mut outs = vec![t.yes, t.no];
            if let I::Switch(_, _, pairs) = &context.body.instruction(t.term).kind {
                outs.extend(pairs.iter().map(|&(_, b)| b));
            }
            for b in outs {
                if region.contains(&b)
                    || consumed[b.index()]
                    || cfg.handler(b)
                    || Some(&b) == cfg.reverse_postorder().first()
                {
                    continue;
                }
                let Some(t) = test(context, b, false).filter(|t| t.x == first.x) else {
                    continue;
                };
                let preds = cfg.predecessors(b);
                if preds.is_empty()
                    || !preds.iter().all(|b| region.contains(b))
                    || region.len() >= 24
                {
                    continue;
                }
                region.push(b);
                tests.push(t);
                grew = true;
            }
            r += 1;
        }
    }
    (region.len() >= 2).then_some((region, tests))
}
fn exits(
    context: &Context<'_>,
    region: &[BlockId],
    tests: &[Test],
) -> Option<Vec<(BlockId, Ranges)>> {
    let cfg = context.cfg().ok()?;
    let (lo, hi) = bounds(tests[0].ty)?;
    let mut order = (0..region.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|&r| cfg.rpo_position(region[r]));
    let mut entry = vec![Vec::new(); region.len()];
    entry[0].push((lo, hi));
    let mut exits: Vec<(BlockId, Ranges)> = Vec::new();
    for r in order {
        for (to, values) in sides(context.body, tests[r], &entry[r], lo, hi) {
            if to == region[0] {
                return None;
            }
            if let Some(at) = region.iter().position(|&b| b == to) {
                if cfg.rpo_position(to)? <= cfg.rpo_position(region[r])? {
                    return None;
                }
                entry[at] = unite(&entry[at], &values);
            } else if let Some((_, set)) = exits.iter_mut().find(|(b, _)| *b == to) {
                *set = unite(set, &values);
            } else {
                exits.push((to, values));
            }
        }
    }
    Some(exits)
}
fn form(context: &mut Context<'_>, head: BlockId, consumed: &mut [bool]) -> Result<bool, Error> {
    let Some((region, tests)) = region(context, head, consumed) else {
        return Ok(false);
    };
    let Some(exits) = exits(context, &region, &tests).filter(|e| e.len() >= 2) else {
        return Ok(false);
    };
    // First target wins a tie in both cardinality and run count.
    let mut default = 0;
    for e in 1..exits.len() {
        if (count(&exits[e].1), exits[e].1.len())
            > (count(&exits[default].1), exits[default].1.len())
        {
            default = e;
        }
    }
    let mut cases = 0;
    let mut runs = 0;
    for (e, (_, set)) in exits.iter().enumerate() {
        if e != default {
            cases += count(set);
            runs += set.len();
        }
    }
    if cases > 256 || runs >= tests.iter().map(|t| t.cost).sum() {
        return Ok(false);
    }
    for &(to, _) in &exits {
        for &id in &context.body.block(to).instructions {
            let I::Phi(_, pairs) = &context.body.instruction(id).kind else {
                break;
            };
            let mut seen = None;
            for &(b, v) in pairs {
                if !region.contains(&b) {
                    continue;
                }
                if seen.is_some_and(|old| old != v) {
                    return Ok(false);
                }
                seen = Some(v);
            }
        }
    }
    for (e, (to, set)) in exits.iter().enumerate() {
        let reached = e == default || !set.is_empty();
        let preds = context
            .cfg()?
            .predecessors(*to)
            .iter()
            .copied()
            .filter(|b| region.contains(b))
            .collect::<Vec<_>>();
        let from_head = preds.contains(&head);
        let keep = reached
            .then(|| preds.iter().copied().find(|&b| b != head))
            .flatten();
        for pred in preds {
            if pred == head && !reached || pred != head && (from_head || Some(pred) != keep) {
                context.body.remove_phi_predecessor(*to, pred);
            }
        }
        if reached
            && !from_head
            && let Some(keep) = keep
        {
            context.body.rename_phi_predecessor(*to, keep, head);
        }
    }
    let mut pairs = Vec::with_capacity(usize::try_from(cases).map_err(|_| Error::Limit)?);
    for (e, (to, set)) in exits.iter().enumerate() {
        if e == default {
            continue;
        }
        for &(a, b) in set {
            let mut v = a;
            loop {
                pairs.push((
                    context
                        .body
                        .integer(u128::from_le_bytes(i128::from(v).to_le_bytes()))?,
                    *to,
                ));
                if v == b {
                    break;
                }
                v += 1;
            }
        }
    }
    context.body.replace_kind(
        tests[0].term,
        I::Switch(Value::Temp(tests[0].x), exits[default].0, pairs),
    );
    context.body.record_switch_types();
    for b in region {
        consumed[b.index()] = true;
    }
    Ok(true)
}
pub(super) fn form_switches(context: &mut Context<'_>) -> Result<bool, Error> {
    context.ensure_cfg()?;
    context.prepare_uses(Pass::FormSwitches, true)?;
    context.prepare_block_map(Pass::FormSwitches, true);
    let mut consumed = vec![false; context.body.blocks().len()];
    let rpo = context.cfg()?.reverse_postorder().to_vec();
    let mut changed = false;
    for b in rpo {
        if !consumed[b.index()] && form(context, b, &mut consumed)? {
            changed = true;
            context.invalidate_cfg();
            context.ensure_cfg()?;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_ranges_partition_the_integer_domain_at_both_extremes() {
        let all = vec![(i64::MIN, i64::MAX)];
        assert_eq!(count(&all), 1 << 62);
        let part = vec![(i64::MIN, -4), (7, i64::MAX)];
        assert_eq!(subtract(&all, &part, i64::MIN, i64::MAX), [(-3, 6)]);
        assert_eq!(unite(&part, &vec![(-3, 6)]), all);
        for lo in -4..=4 {
            for hi in lo..=4 {
                let part = clip(&vec![(-4, 4)], lo, hi);
                let rest = subtract(&vec![(-4, 4)], &part, -4, 4);
                assert_eq!(count(&part) + count(&rest), 9);
                assert_eq!(unite(&part, &rest), [(-4, 4)]);
            }
        }
    }
    #[test]
    fn phi_agreement_is_required_even_for_an_empty_value_edge() {
        let source = "function @f(%x:i32,%shared:i32)->i32 {block ^entry: %a=cmp eq i32 %x,1 branch %a,^hit,^second block ^second:%b=cmp eq i32 %x,1 branch %b,^hit,^miss block ^hit:%v=phi i32 [^entry:%shared,^second:%shared] return i32 %v block ^miss:return i32 0}";
        let (out, changed) = super::super::pass_view(source, 2, Pass::FormSwitches);
        assert!(changed.0);
        assert!(out.contains("switch %x"));
        assert!(out.contains("phi i32 [^entry: %shared]"));
        assert!(
            !super::super::pass_view(
                &source.replace("^second:%shared", "^second:%x"),
                2,
                Pass::FormSwitches
            )
            .1
            .0
        );
    }
    #[test]
    fn repeated_switch_cases_keep_the_first_target() {
        let source = "function @f(%x:i32)->i32 {block ^entry:switch %x,^second,1:^one,1:^ignored block ^second:%c=cmp eq i32 %x,2 branch %c,^two,^miss block ^one:return i32 1 block ^ignored:return i32 99 block ^two:return i32 2 block ^miss:return i32 0}";
        let (out, changed) = super::super::pass_view(source, 2, Pass::FormSwitches);
        assert!(changed.0);
        let switch = out.lines().find(|l| l.contains("switch")).unwrap();
        assert!(switch.contains("1:^one") || switch.contains("1: ^one"));
        assert!(!switch.contains("ignored"));
    }
}
