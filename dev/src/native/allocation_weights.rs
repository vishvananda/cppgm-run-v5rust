//! Bounded natural-loop discovery and fixed-point execution weights. See NOTICE.
use super::{control_flow::Flow, value_facts::Facts};
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstructionKind as I, Program, RuntimeInstruction as Runtime},
};
pub(super) const SCALE: u64 = 64;
pub(super) struct Weights {
    depth: Vec<usize>,
    pub(super) loops: Vec<Vec<BlockId>>,
    pub(super) frequency: Vec<u64>,
}
struct Walk {
    post: Vec<BlockId>,
    back: Vec<(BlockId, BlockId)>,
}
fn walk(body: &Body, flow: &Flow) -> Walk {
    let mut state = vec![0; body.blocks().len()];
    let mut next = vec![0; state.len()];
    let mut stack = vec![body.layout[0]];
    let mut post = Vec::new();
    let mut back = Vec::new();
    state[body.layout[0].index()] = 1;
    while let Some(&b) = stack.last() {
        let successors = flow.successors(b);
        if next[b.index()] == successors.len() {
            state[b.index()] = 2;
            post.push(b);
            let _ = stack.pop();
            continue;
        }
        let to = successors[next[b.index()]];
        next[b.index()] += 1;
        if state[to.index()] == 1 {
            back.push((to, b));
        } else if state[to.index()] == 0 && flow.reachable[to.index()] {
            state[to.index()] = 1;
            stack.push(to);
        }
    }
    let mut rank = vec![0; state.len()];
    for (k, &b) in body.layout.iter().enumerate() {
        rank[b.index()] = k;
    }
    back.sort_by_key(|&(h, l)| (rank[h.index()], rank[l.index()]));
    Walk { post, back }
}
impl Weights {
    pub(super) fn loop_membership(body: &Body, flow: &Flow) -> Result<Vec<bool>, Error> {
        let loops = Self::discover(body, flow, &walk(body, flow))?;
        Ok(loops.depth.into_iter().map(|depth| depth != 0).collect())
    }
    fn discover(body: &Body, flow: &Flow, walk: &Walk) -> Result<Self, Error> {
        let n = body.blocks().len();
        let mut out = Self {
            depth: vec![0; n],
            loops: Vec::new(),
            frequency: vec![0; n],
        };
        let mut mark = vec![0; n];
        let mut work = Vec::new();
        let budget = n
            .checked_mul(16)
            .and_then(|x| x.checked_add(4096))
            .ok_or(Error::Limit)?;
        let mut steps = 0;
        let mut k = 0;
        let mut epoch = 0;
        while k < walk.back.len() && steps <= budget {
            let header = walk.back[k].0;
            epoch += 1;
            mark[header.index()] = epoch;
            out.depth[header.index()] += 1;
            let mut members = vec![header];
            while k < walk.back.len() && walk.back[k].0 == header {
                let latch = walk.back[k].1;
                k += 1;
                if mark[latch.index()] != epoch {
                    mark[latch.index()] = epoch;
                    out.depth[latch.index()] += 1;
                    work.push(latch);
                    members.push(latch);
                }
            }
            while !work.is_empty() && steps <= budget {
                let x = work.pop().ok_or(Error::Limit)?;
                for &q in flow.predecessors(x) {
                    steps += 1;
                    if mark[q.index()] != epoch {
                        mark[q.index()] = epoch;
                        out.depth[q.index()] += 1;
                        work.push(q);
                        members.push(q);
                    }
                }
            }
            work.clear();
            if steps <= budget {
                out.loops.push(members);
            }
        }
        Ok(out)
    }
    pub(super) fn analyze(program: &Program, body: &Body, facts: &Facts) -> Result<Self, Error> {
        let walk = walk(body, &facts.flow);
        let mut out = Self::discover(body, &facts.flow, &walk)?;
        let mut cold = vec![false; body.blocks().len()];
        for (pos, &id) in facts.flow.instructions.iter().enumerate() {
            let kind = &body.instruction(id).kind;
            let no_return = if let I::Call(call) = kind {
                program
                    .call_boundary(call)
                    .is_some_and(|s| s.boundary.returns == crate::lowir::model::Returns::NoReturn)
            } else {
                false
            };
            cold[facts.flow.blocks[pos].index()] |= matches!(
                kind,
                I::Unreachable | I::Runtime(Runtime::Throw(..) | Runtime::Resume)
            ) || no_return;
        }
        loop {
            let mut changed = false;
            for &b in body.layout.iter().rev() {
                let succ = facts.flow.successors(b);
                if !cold[b.index()] && !succ.is_empty() && succ.iter().all(|s| cold[s.index()]) {
                    cold[b.index()] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        out.settle(body, &facts.flow, &walk, &cold);
        Ok(out)
    }
    fn settle(&mut self, body: &Body, flow: &Flow, walk: &Walk, cold: &[bool]) {
        const ONE: u64 = 1 << 16;
        const MAX: u64 = ONE << 20;
        let n = body.blocks().len();
        let mut order = vec![usize::MAX; n];
        for (k, &b) in walk.post.iter().rev().enumerate() {
            order[b.index()] = k;
        }
        let mut by_size: Vec<_> = (0..self.loops.len()).collect();
        by_size.sort_by_key(|&l| (self.loops[l].len(), l));
        let mut inner = vec![None; n];
        let mut parent = vec![None; self.loops.len()];
        let mut header = vec![false; n];
        for &l in by_size.iter().rev() {
            let h = self.loops[l][0];
            parent[l] = inner[h.index()];
            for &b in &self.loops[l] {
                inner[b.index()] = Some(l);
            }
            header[h.index()] = true;
        }
        let contains = |l, b: BlockId| {
            let mut x = inner[b.index()];
            while let Some(i) = x {
                if i == l {
                    return true;
                }
                x = parent[i];
            }
            false
        };
        let mut first = vec![0; n + 1];
        let back_edges: crate::support::hash::HashSet<_> = walk.back.iter().copied().collect();
        let mut probabilities = Vec::new();
        let mut backward = Vec::new();
        for (index, _) in body.blocks().iter().enumerate() {
            let b = body.block_id(index).unwrap_or(body.layout[0]);
            first[index] = probabilities.len();
            let succ = flow.successors(b);
            let warm = succ.iter().filter(|s| !cold[s.index()]).count() as u64;
            let exits = succ
                .iter()
                .filter(|s| !cold[s.index()] && inner[index].is_some_and(|l| !contains(l, **s)))
                .count() as u64;
            for &s in succ {
                let exit = inner[index].is_some_and(|l| !contains(l, s));
                let p = if cold[s.index()] {
                    0
                } else if exits == 0 || exits == warm {
                    ONE / warm
                } else if exit {
                    ONE / 10 / exits
                } else {
                    (ONE - ONE / 10) / (warm - exits)
                };
                probabilities.push(p);
                backward.push(back_edges.contains(&(s, b)));
            }
        }
        first[n] = probabilities.len();
        let mut incoming = vec![Vec::new(); n];
        for &b in &body.layout {
            for (k, &s) in flow.successors(b).iter().enumerate() {
                incoming[s.index()].push((
                    b,
                    probabilities[first[b.index()] + k],
                    backward[first[b.index()] + k],
                ));
            }
        }
        let mut again = vec![0; n];
        let mut current = vec![0; n];
        let mut stamp = vec![0; n];
        let inflow = |b: BlockId, within, back, current: &[u64], stamp: &[usize]| {
            incoming[b.index()]
                .iter()
                .filter(|&&(p, _, v)| (within == 0 || stamp[p.index()] == within) && v == back)
                .map(|&(p, v, _)| current[p.index()] * v / ONE)
                .fold(0_u64, u64::wrapping_add)
        };
        let settle = |b: BlockId, f: u64, again: &[u64]| {
            let f = f.min(MAX);
            if header[b.index()] {
                (f * ONE / (ONE - again[b.index()])).min(MAX)
            } else {
                f
            }
        };
        let mut blocks = Vec::new();
        for l in by_size {
            let h = self.loops[l][0];
            blocks.clear();
            blocks.extend_from_slice(&self.loops[l]);
            blocks.sort_by_key(|b| order[b.index()]);
            for &b in &blocks {
                current[b.index()] = 0;
                stamp[b.index()] = l + 1;
            }
            current[h.index()] = ONE;
            for &b in &blocks {
                if b != h && order[b.index()] != usize::MAX {
                    current[b.index()] =
                        settle(b, inflow(b, l + 1, false, &current, &stamp), &again);
                }
            }
            again[h.index()] = inflow(h, l + 1, true, &current, &stamp).min(ONE * 9 / 10);
        }
        for &b in walk.post.iter().rev() {
            current[b.index()] = settle(
                b,
                u64::from(b == body.layout[0]) * ONE + inflow(b, 0, false, &current, &stamp),
                &again,
            );
        }
        for &b in &body.layout {
            if !cold[b.index()] && flow.reachable[b.index()] {
                self.frequency[b.index()] = ((current[b.index()] * SCALE + ONE / 2) / ONE).max(1);
            }
        }
    }
}
