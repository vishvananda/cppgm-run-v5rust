//! Costed loop spills and block homes with exceptional-edge closure. See NOTICE.
use super::{
    allocation_ranges::{Candidate, Range},
    allocation_weights::{SCALE, Weights},
    value_facts::Facts,
    x86::Register,
};
use crate::lowir::model::{BlockId, Body, InstructionKind as I, TempId};

#[derive(Clone, Copy)]
pub(super) struct Move {
    pub(super) from: BlockId,
    pub(super) to: BlockId,
    pub(super) temp: TempId,
    pub(super) register: Register,
    pub(super) store: bool,
}

impl Move {
    pub(super) fn key(self, order: &[usize]) -> (usize, usize) {
        (order[self.from.index()], order[self.to.index()])
    }
}
#[derive(Default)]
pub(super) struct Splits {
    order: Vec<usize>,
    pub(super) moves: Vec<Move>,
    pub(super) home_first: Vec<usize>,
    pub(super) homes: Vec<TempId>,
    pub(super) loops: crate::support::telemetry::Counter<usize>,
    pub(super) blocks: crate::support::telemetry::Counter<usize>,
}
impl Splits {
    pub(super) fn transfers(&self, from: BlockId, to: BlockId) -> &[Move] {
        if self.moves.is_empty() {
            return &[];
        }
        let key = (self.order[from.index()], self.order[to.index()]);
        let first = self.moves.partition_point(|m| m.key(&self.order) < key);
        let past = self.moves.partition_point(|m| m.key(&self.order) <= key);
        &self.moves[first..past]
    }
    pub(super) fn install(&mut self, body: &Body, homes: &mut [(BlockId, TempId)]) {
        let n = body.blocks().len();
        self.order = vec![0; n];
        for (k, &b) in body.layout.iter().enumerate() {
            self.order[b.index()] = k;
        }
        homes.sort_unstable_by_key(|(b, t)| (b.index(), t.index()));
        self.home_first = vec![0; n + 1];
        for &(block, temp) in homes.iter() {
            self.home_first[block.index() + 1] += 1;
            self.homes.push(temp);
        }
        for b in 0..n {
            self.home_first[b + 1] += self.home_first[b];
        }
        crate::support::ordering::sort_by(&mut self.moves, |a, b| {
            a.key(&self.order) < b.key(&self.order)
        });
    }
}

// All scratch is owned by this function's allocation. Candidate and plan stamps
// avoid clearing block-sized maps for every register tried. No bodies are copied.
pub(super) struct Planner<'a> {
    body: &'a Body,
    facts: &'a Facts,
    bases: &'a [usize],
    weights: &'a Weights,
    exceptional: &'a [Vec<BlockId>],
    successors: Vec<Vec<BlockId>>,
    order: Vec<usize>,
    chosen: Vec<usize>,
    live: Vec<usize>,
    used: Vec<usize>,
    inside: Vec<usize>,
    loop_at: Vec<usize>,
    kept: Vec<usize>,
    reload: Vec<usize>,
    candidate_stamp: usize,
    stamp: usize,
    conflicts: Vec<BlockId>,
    best: Vec<BlockId>,
    pub(super) ranges: Vec<Range>,
    pub(super) moves: Vec<Move>,
    pub(super) homed: Vec<BlockId>,
    cost: u64,
}
impl<'a> Planner<'a> {
    pub(super) fn new(
        body: &'a Body,
        facts: &'a Facts,
        bases: &'a [usize],
        weights: &'a Weights,
        exceptional: &'a [Vec<BlockId>],
        region: Option<&super::region_flow::Plan>,
    ) -> Self {
        let n = body.blocks().len();
        let mut eh = vec![false; n];
        let mut successors = vec![Vec::new(); n];
        for &b in &body.layout {
            eh[b.index()] |= facts.flow.handlers[b.index()]
                || !exceptional[b.index()].is_empty()
                || region.is_some_and(|r| r.indexes[b.index()].is_some());
            for &from in &exceptional[b.index()] {
                eh[from.index()] = true;
                successors[from.index()].push(b);
            }
        }
        let mut order: Vec<_> = (0..weights.loops.len())
            .filter(|&l| {
                weights.loops[l]
                    .iter()
                    .all(|&b| b != body.layout[0] && !eh[b.index()])
            })
            .collect();
        order.sort_unstable_by_key(|&l| (std::cmp::Reverse(weights.loops[l].len()), l));
        Self {
            body,
            facts,
            bases,
            weights,
            exceptional,
            successors,
            order,
            chosen: Vec::new(),
            live: vec![0; n],
            used: vec![0; n],
            inside: vec![0; n],
            loop_at: vec![0; n],
            kept: vec![0; n],
            reload: vec![0; n],
            candidate_stamp: 0,
            stamp: 0,
            conflicts: Vec::new(),
            best: Vec::new(),
            ranges: Vec::new(),
            moves: Vec::new(),
            homed: Vec::new(),
            cost: 0,
        }
    }
    fn next_plan(&mut self) {
        if self.stamp == usize::MAX {
            self.kept.fill(0);
            self.reload.fill(0);
            self.stamp = 0;
        }
        self.stamp += 1;
    }
    fn candidates(&mut self, c: &Candidate) -> bool {
        if self.candidate_stamp == usize::MAX {
            self.live.fill(0);
            self.used.fill(0);
            self.inside.fill(0);
            self.candidate_stamp = 0;
        }
        self.candidate_stamp += 1;
        let e = self.candidate_stamp;
        self.chosen.clear();
        for r in &c.ranges {
            if r.first == self.bases[r.block.index()] {
                self.live[r.block.index()] = e;
            }
        }
        for &(b, _) in &c.uses {
            self.used[b.index()] = e;
        }
        if c.temp.index() >= self.body.parameter_count
            && let Some(p) = self.facts.definitions[c.temp.index()]
        {
            self.used[self.facts.flow.blocks[p].index()] = e;
        }
        for &l in &self.order {
            let blocks = &self.weights.loops[l];
            let h = blocks[0].index();
            if self.live[h] != e
                || self.inside[h] == e
                || blocks.iter().any(|b| self.used[b.index()] == e)
            {
                continue;
            }
            for &b in blocks {
                self.inside[b.index()] = e;
                self.loop_at[b.index()] = l;
            }
            self.chosen.push(l);
        }
        !self.chosen.is_empty()
    }
    // Try registers in physical-bit order; strict cost improvement preserves
    // the frozen allocator's first minimum. Rebuild that plan before returning.
    pub(super) fn select(
        &mut self,
        c: &Candidate,
        allowed: u32,
        blocked: &[(BlockId, u32)],
    ) -> Option<bool> {
        let loops = self.candidates(c);
        for homes in [false, true] {
            if !homes && !loops {
                continue;
            }
            let mut bound = u64::MAX;
            for reg in 0..16 {
                if allowed & (1 << reg) == 0 {
                    continue;
                }
                let mut conflicts = std::mem::take(&mut self.conflicts);
                conflicts.clear();
                let mut feasible = true;
                for &(b, mask) in blocked {
                    if mask & (1 << reg) == 0 {
                        continue;
                    }
                    conflicts.push(b);
                    if !homes && self.inside[b.index()] != self.candidate_stamp {
                        feasible = false;
                        break;
                    }
                }
                if feasible
                    && !conflicts.is_empty()
                    && self.plan(c, &conflicts, homes, bound)
                    && self.cost < bound
                {
                    bound = self.cost;
                    self.best.clear();
                    self.best.extend_from_slice(&conflicts);
                }
                self.conflicts = conflicts;
            }
            if bound != u64::MAX {
                let best = std::mem::take(&mut self.best);
                let ok = self.plan(c, &best, homes, u64::MAX);
                self.best = best;
                return ok.then_some(homes);
            }
        }
        None
    }
    fn seed(&mut self, c: &Candidate, conflicts: &[BlockId], homes: bool) -> bool {
        self.next_plan();
        self.homed.clear();
        let mut any = false;
        if homes {
            for r in &c.ranges {
                self.reload[r.block.index()] = self.stamp;
            }
            for &b in conflicts {
                self.add_home(b);
            }
            let mut k = 0;
            while k < self.homed.len() {
                let b = self.homed[k];
                k += 1;
                // Both directions of EH edges close over blocks where live.
                for i in 0..self.successors[b.index()].len() {
                    let to = self.successors[b.index()][i];
                    if self.reload[to.index()] == self.stamp {
                        self.add_home(to);
                    }
                }
                for &from in &self.exceptional[b.index()] {
                    if self.reload[from.index()] == self.stamp {
                        self.add_home(from);
                    }
                }
            }
            any = !self.homed.is_empty();
        } else {
            for &l in &self.chosen {
                if conflicts.iter().any(|b| {
                    self.inside[b.index()] == self.candidate_stamp && self.loop_at[b.index()] == l
                }) {
                    any = true;
                    for &b in &self.weights.loops[l] {
                        self.kept[b.index()] = self.stamp;
                    }
                }
            }
        }
        any
    }
    fn plan(&mut self, c: &Candidate, conflicts: &[BlockId], homes: bool, bound: u64) -> bool {
        if !self.seed(c, conflicts, homes) {
            return false;
        }
        let parameter = c.temp.index() < self.body.parameter_count;
        let def = self.facts.definitions[c.temp.index()];
        let block = if parameter {
            self.body.layout[0]
        } else if let Some(p) = def {
            self.facts.flow.blocks[p]
        } else {
            return false;
        };
        let defined_inside = homes && self.kept[block.index()] == self.stamp;
        if homes
            && !parameter
            && def.is_some_and(|p| {
                matches!(
                    self.body.instruction(self.facts.flow.instructions[p]).kind,
                    I::Phi(..)
                )
            })
            && (defined_inside
                || self
                    .facts
                    .flow
                    .predecessors(block)
                    .iter()
                    .any(|b| self.kept[b.index()] == self.stamp))
        {
            return false;
        }
        self.cost = if defined_inside {
            if parameter {
                SCALE
            } else {
                self.weights.frequency[block.index()]
            }
        } else {
            0
        };
        let limit = if homes { bound.min(c.weight) } else { c.weight };
        if self.cost >= limit {
            return false;
        }
        self.moves.clear();
        for r in &c.ranges {
            let x = r.block;
            if r.first != self.bases[x.index()] {
                continue;
            }
            for &q in self.facts.flow.predecessors(x) {
                let from_inside = self.kept[q.index()] == self.stamp;
                let to_inside = self.kept[x.index()] == self.stamp;
                if from_inside == to_inside || to_inside && defined_inside {
                    continue;
                }
                self.moves.push(Move {
                    from: q,
                    to: x,
                    temp: c.temp,
                    register: Register::RAX,
                    store: to_inside,
                });
                self.cost += if homes {
                    self.weights.frequency[q.index()].min(self.weights.frequency[x.index()])
                } else {
                    self.weights.frequency[if to_inside { q } else { x }.index()]
                };
                if homes && self.cost >= limit {
                    return false;
                }
            }
        }
        if homes {
            for &(b, _) in &c.uses {
                if self.kept[b.index()] == self.stamp {
                    self.cost += self.weights.frequency[b.index()];
                    if self.cost >= limit {
                        return false;
                    }
                }
            }
        }
        if self.cost >= limit {
            return false;
        }
        self.finish_ranges(c, homes);
        true
    }
    fn finish_ranges(&mut self, c: &Candidate, homes: bool) {
        crate::support::ordering::sort_by(&mut self.moves, |a, b| {
            a.key(&self.facts.flow.linear) < b.key(&self.facts.flow.linear)
        });
        self.moves.dedup_by_key(|m| (m.from.index(), m.to.index()));
        self.ranges.clear();
        self.ranges.extend(
            c.ranges
                .iter()
                .filter(|r| self.kept[r.block.index()] != self.stamp)
                .copied(),
        );
        if !homes {
            for m in &self.moves {
                if m.store
                    || self.reload[m.from.index()] == self.stamp
                    || self.facts.flow.successors(m.from).len() > 1
                {
                    continue;
                }
                self.reload[m.from.index()] = self.stamp;
                let written = self.bases[m.from.index()]
                    + 3
                    + 2 * self.facts.flow.ranges[m.from.index()].len();
                self.ranges.push(Range {
                    block: m.from,
                    first: written,
                    last: written,
                });
            }
            self.ranges
                .sort_by_key(|r| self.facts.flow.linear[r.block.index()]);
        }
    }
    fn add_home(&mut self, b: BlockId) {
        if self.kept[b.index()] != self.stamp {
            self.kept[b.index()] = self.stamp;
            self.homed.push(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{address_facts::Addresses, allocation_ranges::Ranges, control_flow::Flow};
    use super::*;
    use crate::lowir::{Error, model::Program};

    fn facts(program: &Program, body: &Body) -> Result<(Facts, Ranges, Weights), Error> {
        let flow = Flow::analyze(body)?;
        let slots = super::super::slot_analysis::analyze(body, &flow)?;
        let mut facts = Facts::analyze(body, &slots, flow, 2)?;
        let addresses = Addresses::analyze(
            program,
            body,
            &mut facts,
            super::super::object_compile::ObjectModel::Course,
        )?;
        let mut ranges = Ranges::collect(body, &facts, &addresses)?;
        let weights = Weights::analyze(program, body, &facts)?;
        ranges.live(
            body,
            &facts,
            &weights,
            &vec![Vec::new(); body.blocks().len()],
        )?;
        Ok((facts, ranges, weights))
    }
    #[test]
    fn homes_close_both_directions_of_exceptional_edges_without_transfers() -> Result<(), Error> {
        let program = crate::lowir::test_program(
            b"
            function @f(%v : i64) -> i64 {
              block ^entry: jump ^call
              block ^call: jump ^landing
              block ^landing: jump ^exit
              block ^exit:
                %a = binary add i64 %v, %v
                %b = binary add i64 %a, %v
                return i64 %b
            }",
        )?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let (facts, ranges, weights) = facts(&program, body)?;
        let [entry, call, landing, exit] = body.layout[..] else {
            return Err(Error::Limit);
        };
        let c = ranges
            .candidates
            .iter()
            .find(|c| c.temp.index() == 0)
            .ok_or(Error::Limit)?;
        let mut exceptional = vec![Vec::new(); body.blocks().len()];
        exceptional[landing.index()].push(call);
        let mut planner = Planner::new(body, &facts, &ranges.bases, &weights, &exceptional, None);
        assert!(!planner.plan(c, &[], true, u64::MAX));
        for seed in [call, landing] {
            assert!(planner.plan(c, &[seed], true, u64::MAX));
            assert_eq!(planner.homed.len(), 2);
            assert!(planner.homed.contains(&call) && planner.homed.contains(&landing));
            assert_eq!(planner.moves.len(), 2);
            assert!(
                planner
                    .moves
                    .iter()
                    .any(|m| m.from == entry && m.to == call && m.store)
            );
            assert!(
                planner
                    .moves
                    .iter()
                    .any(|m| m.from == landing && m.to == exit && !m.store)
            );
            assert!(
                planner
                    .ranges
                    .iter()
                    .all(|r| r.block == entry || r.block == exit)
            );
            let cost = planner.cost;
            assert!(!planner.plan(c, &[seed], true, cost));
        }
        Ok(())
    }
    #[test]
    fn phi_definition_and_its_transfer_predecessors_keep_the_register() -> Result<(), Error> {
        let program = crate::lowir::test_program(
            b"
            function @f(%gate : i64, %a : i64, %b : i64) -> i64 {
              block ^entry: branch %gate, ^left, ^right
              block ^left: jump ^join
              block ^right: jump ^join
              block ^join:
                %value = phi i64 [^left: %a, ^right: %b]
                jump ^exit
              block ^exit:
                %one = binary add i64 %value, %value
                %two = binary add i64 %one, %value
                return i64 %two
            }",
        )?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let (facts, ranges, weights) = facts(&program, body)?;
        let exceptional = vec![Vec::new(); body.blocks().len()];
        let c = ranges
            .candidates
            .iter()
            .find(|c| c.temp.index() == 3)
            .ok_or(Error::Limit)?;
        let mut planner = Planner::new(body, &facts, &ranges.bases, &weights, &exceptional, None);
        for &block in &body.layout[1..4] {
            assert!(!planner.plan(c, &[block], true, u64::MAX));
        }
        Ok(())
    }
    #[test]
    fn conditional_loop_exits_reload_on_edges_without_predecessor_ranges() -> Result<(), Error> {
        let program = crate::lowir::test_program(
            b"
            function @f(%value : i64, %gate : i64) -> i64 {
              block ^entry: jump ^head
              block ^head: branch %gate, ^body, ^conditional_exit
              block ^body: branch %gate, ^head, ^tail
              block ^tail: jump ^unconditional_exit
              block ^conditional_exit:
                %a = binary add i64 %value, %value
                return i64 %a
              block ^unconditional_exit:
                %b = binary add i64 %value, %value
                return i64 %b
            }",
        )?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let (facts, ranges, weights) = facts(&program, body)?;
        let exceptional = vec![Vec::new(); body.blocks().len()];
        let mut planner = Planner::new(body, &facts, &ranges.bases, &weights, &exceptional, None);
        let c = ranges
            .candidates
            .iter()
            .find(|c| c.temp.index() == 0)
            .ok_or(Error::Limit)?;
        assert!(planner.candidates(c));
        let head = body.layout[1];
        assert!(planner.plan(c, &[head], false, u64::MAX));
        assert!(!planner.ranges.iter().any(|r| r.block == head));
        // Repeating a plan must retain the same reload and range counts.
        let counts = (planner.moves.len(), planner.ranges.len(), planner.cost);
        assert!(planner.plan(c, &[head], false, u64::MAX));
        assert_eq!(
            counts,
            (planner.moves.len(), planner.ranges.len(), planner.cost)
        );
        // A reload with an unconditional predecessor writes in its transfer
        // slot; a conditional predecessor writes in the edge block instead.
        let tail = body.layout[3];
        planner.next_plan();
        planner.kept[head.index()] = planner.stamp;
        planner.kept[tail.index()] = planner.stamp;
        planner.moves = vec![
            Move {
                from: head,
                to: body.layout[4],
                temp: c.temp,
                register: Register::R8,
                store: false,
            },
            Move {
                from: tail,
                to: body.layout[5],
                temp: c.temp,
                register: Register::R8,
                store: false,
            },
        ];
        planner.finish_ranges(c, false);
        assert!(!planner.ranges.iter().any(|r| r.block == head));
        let reloads: Vec<_> = planner.ranges.iter().filter(|r| r.block == tail).collect();
        assert_eq!(reloads.len(), 1);
        let written = ranges.bases[tail.index()] + 3 + 2 * facts.flow.ranges[tail.index()].len();
        assert_eq!((reloads[0].first, reloads[0].last), (written, written));
        Ok(())
    }
}
