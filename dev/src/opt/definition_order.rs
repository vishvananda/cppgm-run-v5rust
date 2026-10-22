//! Stable definition-before-use layout from `opt/cfg_cleanup.cpp`. See NOTICE.
use super::pass_context::Context;
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstructionKind as I, Value},
};
use std::{cmp::Reverse, collections::BinaryHeap};

fn defines_before_uses(body: &Body, layout: &[BlockId]) -> bool {
    let mut positions = vec![None; body.instructions().len()];
    let mut at = 0_usize;
    for &block in layout {
        for &id in &body.block(block).instructions {
            positions[id.index()] = Some(at);
            at += 1;
        }
    }
    for &block in layout {
        for &id in &body.block(block).instructions {
            let kind = &body.instruction(id).kind;
            if matches!(kind, I::Phi(..)) {
                continue;
            }
            let mut ordered = true;
            body.visit_values(kind, |value| {
                if let Value::Temp(temp) = value
                    && !body.temp_is_parameter(temp)
                {
                    ordered &= body
                        .temp(temp)
                        .definition
                        .and_then(|d| positions[d.index()])
                        .is_some_and(|def| positions[id.index()].is_some_and(|at| def < at));
                }
            });
            if !ordered {
                return false;
            }
        }
    }
    true
}
fn topological(body: &Body, fallback: Vec<BlockId>) -> Result<Vec<BlockId>, Error> {
    let count = body.blocks().len();
    let mut rank = vec![None; count];
    let mut block_of = vec![None; body.instructions().len()];
    for (index, &block) in body.layout.iter().enumerate() {
        rank[block.index()] = Some(index);
        for &id in &body.block(block).instructions {
            block_of[id.index()] = Some(block);
        }
    }
    let mut edges = Vec::new();
    for &block in &body.layout {
        for &id in &body.block(block).instructions {
            let kind = &body.instruction(id).kind;
            if matches!(kind, I::Phi(..)) {
                continue;
            }
            body.visit_values(kind, |value| {
                if let Value::Temp(temp) = value
                    && !body.temp_is_parameter(temp)
                    && let Some(def) = body.temp(temp).definition.and_then(|d| block_of[d.index()])
                    && def != block
                {
                    edges.push((def, block));
                }
            });
        }
    }
    let mut first = vec![0_usize; count + 1];
    let mut indegree = vec![0_usize; count];
    for &(def, used) in &edges {
        first[def.index() + 1] += 1;
        indegree[used.index()] += 1;
    }
    for b in 0..count {
        first[b + 1] += first[b];
    }
    let mut fill = first[..count].to_vec();
    let mut next = vec![None; edges.len()];
    for (def, used) in edges {
        next[fill[def.index()]] = Some(used);
        fill[def.index()] += 1;
    }
    let mut ready = BinaryHeap::new();
    for &block in &body.layout {
        if indegree[block.index()] == 0 {
            ready.push(Reverse(rank[block.index()].ok_or(Error::Limit)?));
        }
    }
    let mut layout = Vec::with_capacity(body.layout.len());
    while let Some(Reverse(position)) = ready.pop() {
        let block = body.layout[position];
        layout.push(block);
        for &next in &next[first[block.index()]..first[block.index() + 1]] {
            let next = next.ok_or(Error::Limit)?;
            indegree[next.index()] -= 1;
            if indegree[next.index()] == 0 {
                ready.push(Reverse(rank[next.index()].ok_or(Error::Limit)?));
            }
        }
    }
    if layout.len() != body.layout.len() || layout.first() != body.layout.first() {
        Ok(fallback)
    } else {
        Ok(layout)
    }
}
pub(super) fn restore(context: &mut Context<'_>) -> Result<bool, Error> {
    let stamp = context.next_pass_stamp();
    let body = &*context.body;
    let mut ordered = true;
    'blocks: for &block in &body.layout {
        for &id in &body.block(block).instructions {
            let kind = &body.instruction(id).kind;
            if !matches!(kind, I::Phi(..)) {
                body.visit_values(kind, |value| {
                    if let Value::Temp(temp) = value
                        && !body.temp_is_parameter(temp)
                    {
                        ordered &= body
                            .temp(temp)
                            .definition
                            .is_some_and(|d| context.passed[d.index()] == stamp);
                    }
                });
            }
            context.passed[id.index()] = stamp;
            if !ordered {
                break 'blocks;
            }
        }
    }
    context.left_ordered = ordered;
    if ordered {
        return Ok(false);
    }
    context.ensure_cfg_light()?;
    let cfg = context.cfg()?;
    let mut layout = cfg.reverse_postorder().to_vec();
    layout.extend(
        context
            .body
            .layout
            .iter()
            .copied()
            .filter(|&block| !cfg.reachable(block)),
    );
    if defines_before_uses(context.body, &layout) {
        context.left_ordered = true;
    } else {
        layout = topological(context.body, layout)?;
    }
    let moved = layout != context.body.layout;
    context.body.layout = layout;
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::model::Program;
    fn setup(source: &str) -> (Program, crate::lowir::model::SymbolId, Body) {
        let mut p = crate::lowir::test_program(source.as_bytes()).unwrap();
        let s = p.publications()[0];
        let body = p.take_body(s).unwrap().unwrap();
        (p, s, body)
    }
    #[test]
    fn reordered_layout_places_a_dominating_definition_before_its_use() {
        let (p, s, mut body) = setup(
            "function @f() -> i64 {block ^entry: jump ^def block ^def: %v = const i64 5 jump ^use block ^use: return i64 %v}",
        );
        body.layout.swap(1, 2);
        let mut c = Context::new(&p, s, &mut body, 1, &[], &[]);
        assert!(restore(&mut c).unwrap());
        assert!(c.left_ordered);
        assert_eq!(p.names.get(c.body.block(c.body.layout[1]).name), "def");
        assert!(!restore(&mut c).unwrap());
    }
    #[test]
    fn stable_topological_ties_preserve_nondominating_definition_constraints() {
        let (p, s, mut body) = setup(
            "function @f(%c : i64) -> i64 {block ^entry: branch %c, ^a, ^b block ^a: %v = const i64 5 return i64 0 block ^b: return i64 %v}",
        );
        body.layout.swap(1, 2);
        let mut c = Context::new(&p, s, &mut body, 1, &[], &[]);
        assert!(restore(&mut c).unwrap());
        assert!(!c.left_ordered);
        assert_eq!(p.names.get(c.body.block(c.body.layout[1]).name), "a");
    }
    #[test]
    fn stamp_wrap_clears_old_marks_and_phis_do_not_impose_ordinary_order() {
        let (p, s, mut body) = setup(
            "function @f(%c : i64) -> i64 {block ^entry: jump ^loop block ^loop: %p = phi i64 [^entry: 0, ^loop: %next] %next = binary add i64 %p, 1 branch %c, ^loop, ^end block ^end: return i64 %p}",
        );
        let mut c = Context::new(&p, s, &mut body, 1, &[], &[]);
        c.passed_stamp = u32::MAX;
        c.passed = vec![1; c.body.instructions().len()];
        assert!(!restore(&mut c).unwrap());
        assert_eq!(c.passed_stamp, 1);
        assert!(c.left_ordered);
    }
}
