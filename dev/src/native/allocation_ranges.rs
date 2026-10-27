//! Precise read/write-slot liveness with bounded predecessor walks. See NOTICE.
use super::{
    address_facts::Addresses,
    allocation_weights::{SCALE, Weights},
    value_facts::Facts,
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstructionKind as I, TempId, Value},
    vocabulary::Type,
};
#[derive(Clone, Copy)]
pub(super) struct Range {
    pub(super) block: BlockId,
    pub(super) first: usize,
    pub(super) last: usize,
}
pub(super) struct Candidate {
    pub(super) temp: TempId,
    pub(super) uses: Vec<(BlockId, usize)>,
    pub(super) ranges: Vec<Range>,
    pub(super) weight: u64,
}
pub(super) struct Ranges {
    pub(super) bases: Vec<usize>,
    pub(super) total: usize,
    pub(super) candidates: Vec<Candidate>,
    pub(super) steps: usize,
    pub(super) fallbacks: crate::support::telemetry::Counter<usize>,
}
pub(super) fn tied(kind: &I) -> bool {
    matches!(kind, I::Binary(..) | I::Unary(..))
        && kind.result_type().float_rank() == 0
        && kind.result_type() != Type::I128
}
pub(super) fn eligible(body: &Body, facts: &Facts, addresses: &Addresses, t: TempId) -> bool {
    let i = t.index();
    facts.intervals[i].uses != 0
        && !addresses.lazy[i]
        && addresses.folded[i].is_none()
        && !facts.fused[i]
        && !facts.tested[i]
        && !facts.updates[i]
        && !facts.rotated[i]
        && matches!(facts.resolve(body,Value::Temp(t)),super::value_facts::Alias::Temporary(v) if v == t)
        && facts.definitions[i].is_none_or(|p| facts.active[p])
        && body
            .temp(t)
            .ty
            .is_some_and(|ty| !matches!(ty, Type::I128 | Type::F80 | Type::Object { .. }))
}
impl Ranges {
    pub(super) fn collect(
        body: &Body,
        facts: &Facts,
        addresses: &Addresses,
    ) -> Result<Self, Error> {
        let mut out = Self {
            bases: vec![0; body.blocks().len()],
            total: 0,
            candidates: Vec::new(),
            steps: 0,
            fallbacks: crate::support::telemetry::counter(0),
        };
        for &b in &body.layout {
            out.bases[b.index()] = out.total;
            if facts.flow.reachable[b.index()] {
                out.total = out
                    .total
                    .checked_add(
                        facts.flow.ranges[b.index()]
                            .len()
                            .checked_mul(2)
                            .and_then(|v| v.checked_add(4))
                            .ok_or(Error::Limit)?,
                    )
                    .ok_or(Error::Limit)?;
            }
        }
        i32::try_from(out.total).map_err(|_| Error::Limit)?;
        let mut index = vec![None; body.temps().len()];
        for (i, t) in body.temps().iter().enumerate() {
            let id = body.temp_id(i).ok_or(Error::Limit)?;
            if facts.global[i]
                && eligible(body, facts, addresses, id)
                && !(i < body.parameter_count && t.ty.is_some_and(|ty| ty.float_rank() != 0))
            {
                index[i] = Some(out.candidates.len());
                out.candidates.push(Candidate {
                    temp: id,
                    uses: Vec::new(),
                    ranges: Vec::new(),
                    weight: 0,
                });
            }
        }
        for (pos, &id) in facts.flow.instructions.iter().enumerate() {
            let inst = body.instruction(id);
            let lazy = inst.result.is_some_and(|t| addresses.lazy[t.index()]);
            if !facts.flow.reachable[facts.flow.blocks[pos].index()]
                || !facts.active[pos] && facts.reads[pos] == pos && !lazy
            {
                continue;
            }
            let mut record = |v, b, early, late, tie| {
                for (part, t) in super::global_placement::parts(body, facts, addresses, v)
                    .into_iter()
                    .enumerate()
                {
                    let Some(c) = t.and_then(|t| index[t.index()]) else {
                        continue;
                    };
                    let at = if tie && part == 0 { early } else { late };
                    out.candidates[c].uses.push((b, at));
                }
            };
            if let I::Phi(_, pairs) = &inst.kind {
                for &(b, v) in pairs {
                    if facts.flow.reachable[b.index()] {
                        let at = out.bases[b.index()] + 2 + 2 * facts.flow.ranges[b.index()].len();
                        record(v, b, at, at, false);
                    }
                }
            } else {
                let read = facts.reads[pos];
                let b = facts.flow.blocks[pos];
                let early =
                    out.bases[b.index()] + 2 + 2 * (read - facts.flow.ranges[b.index()].start);
                match inst.kind {
                    I::Binary(_, _, _, a, bv) if tied(&inst.kind) => {
                        record(a, b, early, early + 1, true);
                        record(bv, b, early, early + 1, false);
                    }
                    I::Unary(_, _, a) if tied(&inst.kind) => record(a, b, early, early + 1, true),
                    _ => body.visit_values(&inst.kind, |v| record(v, b, early, early + 1, false)),
                }
            }
        }
        Ok(out)
    }
    pub(super) fn live(
        &mut self,
        body: &Body,
        facts: &Facts,
        weights: &Weights,
        exceptional: &[Vec<BlockId>],
    ) -> Result<(), Error> {
        let n = body.blocks().len();
        let mut seen = vec![0; n];
        let mut incoming = vec![0; n];
        let mut outgoing = vec![0; n];
        let mut transfer = vec![0; n];
        let mut last = vec![None; n];
        let mut work = Vec::new();
        let mut present = Vec::new();
        let mut rank = vec![0; n];
        for (k, &b) in body.layout.iter().enumerate() {
            rank[b.index()] = k;
        }
        let items: usize = body
            .layout
            .iter()
            .filter(|b| facts.flow.reachable[b.index()])
            .map(|b| facts.flow.ranges[b.index()].len() + 1)
            .sum();
        let budget = items
            .checked_mul(64)
            .and_then(|n| n.checked_add(4096))
            .ok_or(Error::Limit)?;
        for c in 0..self.candidates.len() {
            let candidate = &mut self.candidates[c];
            let temp = candidate.temp;
            let epoch = c + 1;
            let param = temp.index() < body.parameter_count;
            let def = facts.definitions[temp.index()];
            let def_block = if param {
                None
            } else {
                def.map(|p| facts.flow.blocks[p])
            };
            let phi = def.is_some_and(|p| {
                matches!(
                    body.instruction(facts.flow.instructions[p]).kind,
                    I::Phi(..)
                )
            });
            present.clear();
            let touch = |b: BlockId,
                         seen: &mut [usize],
                         last: &mut [Option<usize>],
                         present: &mut Vec<BlockId>| {
                if seen[b.index()] != epoch {
                    seen[b.index()] = epoch;
                    last[b.index()] = None;
                    present.push(b);
                }
            };
            candidate.weight = def_block.map_or(SCALE, |b| weights.frequency[b.index()]);
            self.steps += candidate.uses.len();
            for &(b, at) in &candidate.uses {
                touch(b, &mut seen, &mut last, &mut present);
                last[b.index()] = Some(last[b.index()].map_or(at, |v| v.max(at)));
                candidate.weight += weights.frequency[b.index()];
                if Some(b) != def_block && incoming[b.index()] != epoch {
                    incoming[b.index()] = epoch;
                    work.push(b);
                }
            }
            if let Some(b) = def_block {
                touch(b, &mut seen, &mut last, &mut present);
                if phi {
                    incoming[b.index()] = epoch;
                    for &p in facts.flow.predecessors(b) {
                        touch(p, &mut seen, &mut last, &mut present);
                        transfer[p.index()] = epoch;
                    }
                }
            }
            while !work.is_empty() && self.steps <= budget {
                let x = work.pop().ok_or(Error::Limit)?;
                let preds = facts.flow.predecessors(x);
                self.steps += preds.len() + exceptional[x.index()].len();
                for &q in preds.iter().chain(&exceptional[x.index()]) {
                    touch(q, &mut seen, &mut last, &mut present);
                    outgoing[q.index()] = epoch;
                    if Some(q) == def_block || incoming[q.index()] == epoch {
                        continue;
                    }
                    incoming[q.index()] = epoch;
                    work.push(q);
                }
            }
            if self.steps > budget {
                work.clear();
                self.fallbacks = crate::support::telemetry::counter(self.candidates.len() - c);
                self.candidates.truncate(c);
                break;
            }
            present.sort_by_key(|b| rank[b.index()]);
            for &b in &present {
                let base = self.bases[b.index()];
                let count = facts.flow.ranges[b.index()].len();
                let written = base + 3 + 2 * count;
                let live_out = outgoing[b.index()] == epoch;
                let live_in = incoming[b.index()] == epoch;
                let edge_write =
                    phi && transfer[b.index()] == epoch && facts.flow.successors(b).len() > 1;
                let mut push = |first, last| {
                    candidate.ranges.push(Range {
                        block: b,
                        first,
                        last,
                    });
                };
                if edge_write && !live_out {
                    if live_in {
                        push(base, last[b.index()].unwrap_or(base).max(base));
                    }
                    continue;
                }
                if transfer[b.index()] == epoch && !live_out && live_in {
                    push(base, last[b.index()].unwrap_or(base).max(base));
                    push(written, written);
                    continue;
                }
                let start = if live_in {
                    base
                } else if Some(b) == def_block && !phi {
                    base + 3 + 2 * (def.ok_or(Error::Limit)? - facts.flow.ranges[b.index()].start)
                } else {
                    written
                };
                let end = if live_out || transfer[b.index()] == epoch {
                    written
                } else {
                    last[b.index()].unwrap_or(start)
                };
                push(start, end.max(start));
            }
        }
        Ok(())
    }
}
