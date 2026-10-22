//! Bounded reuse across consecutive quiet diamonds, from `value_simplify.cpp`.
//! See NOTICE.
use super::{numeric_fold as fold, pure, value_reuse::Simplifier};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{
        BlockId, Body, InstId, Instruction, InstructionKind as I, Program, RuntimeInstruction as R,
        Value,
    },
    vocabulary::Effects,
};
#[derive(Clone, Copy)]
struct Diamond {
    yes: BlockId,
    no: BlockId,
    merge: BlockId,
    condition: Value,
}
fn diamond(s: &Simplifier<'_, '_>, head: BlockId) -> Result<Option<Diamond>, Error> {
    let body = &*s.context.body;
    let Some(&last) = body.block(head).instructions.last() else {
        return Ok(None);
    };
    let I::Branch(condition, yes, no) = body.instruction(last).kind else {
        return Ok(None);
    };
    if yes == no {
        return Ok(None);
    }
    let mut merges = [None, None];
    for (index, arm) in [yes, no].into_iter().enumerate() {
        let Some(&last) = body.block(arm).instructions.last() else {
            return Ok(None);
        };
        let cfg = s.context.cfg()?;
        if cfg.predecessors(arm).len() != 1 || cfg.handler(arm) {
            return Ok(None);
        }
        let I::Jump(target) = body.instruction(last).kind else {
            return Ok(None);
        };
        merges[index] = Some(target);
    }
    if merges[0] != merges[1] {
        return Ok(None);
    }
    let merge = merges[0].ok_or(Error::Limit)?;
    Ok(
        (s.context.cfg()?.predecessors(merge).len() == 2).then_some(Diamond {
            yes,
            no,
            merge,
            condition,
        }),
    )
}
fn writes(program: &Program, kind: &I) -> bool {
    match kind {
        I::Call(call) => program.call_boundary(call).is_none_or(|right| {
            !matches!(
                right.boundary.effects,
                Some(Effects::ReadNone | Effects::ReadOnly)
            )
        }),
        I::Runtime(R::StackAlloc(_)) => false,
        I::Store(..) | I::CopyObject(..) | I::ZeroInit(..) | I::Runtime(_) => true,
        _ => false,
    }
}
fn write_free(program: &Program, body: &Body, block: BlockId) -> bool {
    body.block(block).instructions.iter().all(|&id| {
        let kind = &body.instruction(id).kind;
        !writes(program, kind) && !matches!(kind,I::Load(facts,..) if facts.volatile())
    })
}
fn phis(body: &Body, block: BlockId) -> impl Iterator<Item = InstId> + '_ {
    body.block(block)
        .instructions
        .iter()
        .copied()
        .take_while(|&id| matches!(body.instruction(id).kind, I::Phi(..)))
}
fn incoming(body: &Body, id: InstId, pred: BlockId) -> Option<Value> {
    let I::Phi(_, pairs) = &body.instruction(id).kind else {
        return None;
    };
    pairs
        .iter()
        .find_map(|&(right, v)| (right == pred).then_some(v))
}
fn same_shape(left: &I, right: &I) -> bool {
    match (left, right) {
        (I::Const(ty, _), I::Const(u, _))
        | (I::Copy(ty, _), I::Copy(u, _))
        | (I::Select(ty, ..), I::Select(u, ..)) => ty == u,
        (I::Unary(op, ty, _), I::Unary(other, u, _)) => op == other && ty == u,
        (I::Binary(op, facts, ty, ..), I::Binary(other, other_facts, u, ..)) => {
            op == other && facts == other_facts && ty == u
        }
        (I::Compare(op, ty, ..), I::Compare(other, u, ..)) => op == other && ty == u,
        (I::Convert(op, ty, facts, _), I::Convert(other, u, other_facts, _)) => {
            op == other && ty == u && facts == other_facts
        }
        (I::Addr(_), I::Addr(_)) => true,
        (I::Index(ty, p, ..), I::Index(u, q, ..)) => ty == u && p == q,
        (I::Load(facts, ty, _), I::Load(other_facts, u, _)) => facts == other_facts && ty == u,
        _ => false,
    }
}
// The frozen diamond proof admits Symbols and then indexes the temporary
// store by their numeric index. Preserve this observable aliasing only here;
// do not forge a TempId or let an out-of-store C++ access become unsafe Rust.
fn diamond_definition(body: &Body, value: Value) -> Option<(InstId, &Instruction)> {
    let index = match value {
        Value::Temp(temp) => temp.index(),
        Value::Symbol(symbol) => symbol.index(),
        _ => return None,
    };
    if index < body.parameter_count {
        return None;
    }
    let definition = body.temps().get(index)?.definition?;
    Some((definition, body.instruction(definition)))
}
fn same_computation(
    s: &Simplifier<'_, '_>,
    left: Option<Value>,
    right: Option<Value>,
    depth: u8,
    quiet: &[BlockId; 5],
) -> bool {
    if left == right {
        return true;
    }
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    let body = &*s.context.body;
    if let (Some(left), Some(right)) = (fold::literal(body, left), fold::literal(body, right)) {
        return matches!((left,right),(Constant::Integer(left),Constant::Integer(right))|(Constant::Float(_,left),Constant::Float(_,right)) if left==right);
    }
    if depth == 0 {
        return false;
    }
    let (Some((ai, p)), Some((bi, q))) = (
        diamond_definition(body, left),
        diamond_definition(body, right),
    ) else {
        return false;
    };
    let holds = |id: InstId| {
        s.context
            .block_map()
            .get(id.index())
            .copied()
            .flatten()
            .is_some_and(|right| quiet.contains(&right))
    };
    let load = matches!(p.kind,I::Load(facts,..) if !facts.volatile()) && holds(ai) && holds(bi);
    if !same_shape(&p.kind, &q.kind) || !(pure(&p.kind) || load) || matches!(p.kind, I::Phi(..)) {
        return false;
    }
    let mut pa = [None; 3];
    let mut qa = [None; 3];
    let mut count = 0;
    body.visit_values(&p.kind, |v| {
        if count < 3 {
            pa[count] = Some(v);
        }
        count += 1;
    });
    count = 0;
    body.visit_values(&q.kind, |v| {
        if count < 3 {
            qa[count] = Some(v);
        }
        count += 1;
    });
    pa.into_iter()
        .zip(qa)
        .all(|(left, right)| same_computation(s, left, right, depth - 1, quiet))
}
pub(super) fn merge(s: &mut Simplifier<'_, '_>) -> Result<bool, Error> {
    let mut changed = false;
    for index in 0..s.context.body.layout.len() {
        let head = s.context.body.layout[index];
        if !s.context.cfg()?.reachable(head) {
            continue;
        }
        let Some(first) = diamond(s, head)? else {
            continue;
        };
        let Some(second) = diamond(s, first.merge)? else {
            continue;
        };
        if first.condition != second.condition {
            continue;
        }
        let quiet = [first.yes, first.no, first.merge, second.yes, second.no];
        if !quiet
            .iter()
            .all(|&right| write_free(s.context.program, s.context.body, right))
        {
            continue;
        }
        let first_phis = phis(s.context.body, first.merge).collect::<Vec<_>>();
        let second_phis = phis(s.context.body, second.merge).collect::<Vec<_>>();
        if first_phis
            .len()
            .checked_mul(second_phis.len())
            .is_none_or(|count| count > 1024)
        {
            continue;
        }
        for id in second_phis {
            let instruction = s.context.body.instruction(id);
            let result = instruction.result.ok_or(Error::Limit)?;
            if s.replacements[result.index()].is_some() {
                continue;
            }
            let I::Phi(ty, _) = instruction.kind else {
                return Err(Error::Limit);
            };
            let yes = incoming(s.context.body, id, second.yes);
            let no = incoming(s.context.body, id, second.no);
            for &first_id in &first_phis {
                let instruction = s.context.body.instruction(first_id);
                let I::Phi(other, _) = instruction.kind else {
                    return Err(Error::Limit);
                };
                if ty != other {
                    continue;
                }
                if same_computation(
                    s,
                    incoming(s.context.body, first_id, first.yes),
                    yes,
                    4,
                    &quiet,
                ) && same_computation(
                    s,
                    incoming(s.context.body, first_id, first.no),
                    no,
                    4,
                    &quiet,
                ) {
                    s.replacements[result.index()] = instruction.result.map(Value::Temp);
                    changed = true;
                    break;
                }
            }
        }
    }
    Ok(changed)
}
