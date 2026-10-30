//! Weighted precise function-register allocation and emission contracts. See NOTICE.
use super::{
    abi::{CALLER_SAVED, Layout},
    address_facts::Addresses,
    allocation_pressure::Pressure,
    allocation_ranges::{Range, Ranges, eligible, tied},
    allocation_weights::{SCALE, Weights},
    global_placement::{Entry, Globals},
    placement::SAVED,
    value_facts::{Alias, Facts},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, InstructionKind as I, Program, RuntimeInstruction as Runtime, TempId, Value,
    },
    vocabulary::{Binary, Convert, Type},
};
const CALLERS: [R; 7] = [R::R8, R::R9, R::RDI, R::RSI, R::RDX, R::RCX, R::RAX];
fn first(kind: &I) -> Option<Value> {
    match *kind {
        I::Binary(_, _, _, a, _) | I::Unary(_, _, a) | I::Convert(_, _, _, a) => Some(a),
        _ => None,
    }
}
fn resolved(body: &Body, facts: &Facts, v: Value) -> Option<TempId> {
    if let Alias::Temporary(t) = facts.resolve(body, v) {
        Some(t)
    } else {
        None
    }
}
fn find(group: &mut [usize], mut c: usize) -> usize {
    while group[c] != c {
        group[c] = group[group[c]];
        c = group[c];
    }
    c
}
struct Assignment<'a> {
    body: &'a Body,
    facts: &'a Facts,
    ranges: &'a Ranges,
    layout: &'a Layout,
    gpr: Pressure,
    xmm: Pressure,
    late: super::global_placement::Ranges,
    early: super::global_placement::Ranges,
    hints: Vec<Option<R>>,
    web_ranges: Vec<Range>,
    registers: Vec<Option<R>>,
    saved: u32,
    reserved: u32,
    rbp_free: bool,
}
impl Assignment<'_> {
    fn choose(
        &self,
        c: usize,
        ranges: &[Range],
        weight: u64,
        web: Option<R>,
        tie: Option<R>,
    ) -> Option<R> {
        let temp = self.ranges.candidates[c].temp;
        let vector = self.body.temp(temp).ty.is_some_and(|t| t.float_rank() != 0);
        let tree = if vector { &self.xmm } else { &self.gpr };
        let mut forbid = 0;
        for range in ranges {
            forbid |= self
                .late
                .query(range.first + 1, range.last.max(range.first + 1))
                | self.early.query(range.first + 1, range.last + 1);
            let (pressure, regs) = tree.query(range.first, range.last);
            let capacity = if vector {
                8
            } else {
                12 + i32::from(self.rbp_free) - i32::from(self.reserved != 0)
            };
            if pressure + 1 > capacity {
                return None;
            }
            forbid |= regs;
        }
        if let Some(p) = self.facts.definitions[temp.index()]
            && matches!(
                self.body.instruction(self.facts.flow.instructions[p]).kind,
                I::Binary(Binary::Shl | Binary::Shr | Binary::UShr, ..)
            )
        {
            forbid |= self.facts.clobbers[p].1;
        }
        let saved_mask = SAVED
            .iter()
            .chain([R::RBP].iter())
            .fold(0, |m, r| m | (1 << r.code()));
        let mut allowed = if vector {
            0x00ff_0000
        } else {
            (CALLER_SAVED | saved_mask & !(if self.rbp_free { 0 } else { 1 << R::RBP.code() }))
                & !self.reserved
        } & !forbid;
        if !vector && weight < 2 * SCALE && self.saved & saved_mask != 0 {
            allowed &= !(saved_mask & !self.saved);
        }
        let incoming = self.layout.arguments.get(temp.index()).and_then(|a| {
            if let [p] = self.layout.parts(a) {
                p.register
            } else {
                None
            }
        });
        let returned = self.facts.definitions[temp.index()]
            .filter(|&p| {
                matches!(
                    self.body.instruction(self.facts.flow.instructions[p]).kind,
                    I::Call(_)
                )
            })
            .map(|_| if vector { R::XMM0 } else { R::RAX });
        for pref in [
            web,
            tie,
            if temp.index() < self.body.parameter_count {
                incoming
            } else {
                None
            },
            returned,
            self.hints[c],
        ]
        .into_iter()
        .flatten()
        {
            if allowed & (1 << pref.code()) != 0 {
                return Some(pref);
            }
        }
        if vector {
            return (0..8)
                .filter_map(|i| R::xmm(i).ok())
                .find(|r| allowed & (1 << r.code()) != 0);
        }
        CALLERS
            .into_iter()
            .chain(
                SAVED
                    .into_iter()
                    .chain([R::RBP])
                    .filter(|r| self.saved & (1 << r.code()) != 0),
            )
            .chain(SAVED)
            .chain([R::RBP])
            .find(|r| allowed & (1 << r.code()) != 0)
    }
    fn split_conflicts(&self, c: usize, blocked: &mut Vec<(BlockId, u32)>) -> u32 {
        blocked.clear();
        for range in &self.ranges.candidates[c].ranges {
            let (pressure, taken) = self.gpr.query(range.first, range.last);
            let capacity = 12 + i32::from(self.rbp_free) - i32::from(self.reserved != 0);
            let mask = if pressure + 1 > capacity {
                u32::MAX
            } else {
                taken
                    | self
                        .late
                        .query(range.first + 1, range.last.max(range.first + 1))
                    | self.early.query(range.first + 1, range.last + 1)
            };
            blocked.push((range.block, mask));
        }
        let saved_mask = SAVED
            .iter()
            .chain([R::RBP].iter())
            .fold(0, |m, r| m | (1 << r.code()));
        let mut allowed = (CALLER_SAVED
            | saved_mask & !(if self.rbp_free { 0 } else { 1 << R::RBP.code() }))
            & !self.reserved;
        if self.ranges.candidates[c].weight < 2 * SCALE && self.saved & saved_mask != 0 {
            allowed &= !(saved_mask & !self.saved);
        }
        allowed
    }
    fn assign_web(&mut self, c: usize, members: &[usize], tied: Option<R>) -> Option<R> {
        if members.len() < 2 {
            return None;
        }
        let vector = self
            .body
            .temp(self.ranges.candidates[c].temp)
            .ty
            .is_some_and(|t| t.float_rank() != 0);
        if members.iter().any(|&w| {
            self.body
                .temp(self.ranges.candidates[w].temp)
                .ty
                .is_some_and(|t| t.float_rank() != 0)
                != vector
        }) {
            return None;
        }
        let mut web = std::mem::take(&mut self.web_ranges);
        web.clear();
        web.extend(
            members
                .iter()
                .flat_map(|&w| self.ranges.candidates[w].ranges.iter().copied()),
        );
        web.sort_by_key(|r| (r.first, r.last));
        let weight = members
            .iter()
            .map(|&w| self.ranges.candidates[w].weight)
            .fold(0_u64, u64::wrapping_add);
        let chosen = if web.windows(2).any(|w| w[0].last >= w[1].first) {
            None
        } else {
            self.choose(c, &web, weight, None, tied)
        };
        self.web_ranges = web;
        let reg = chosen?;
        for &w in members {
            if let Some(p) = self.facts.definitions[self.ranges.candidates[w].temp.index()]
                && matches!(
                    self.body.instruction(self.facts.flow.instructions[p]).kind,
                    I::Binary(Binary::Shl | Binary::Shr | Binary::UShr, ..)
                )
                && self.facts.clobbers[p].1 & (1 << reg.code()) != 0
            {
                return None;
            }
        }
        for &w in members {
            self.commit(w, reg);
        }
        Some(reg)
    }
    fn commit(&mut self, c: usize, r: R) {
        self.commit_ranges(c, r, &self.ranges.candidates[c].ranges);
    }
    fn commit_ranges(&mut self, c: usize, r: R, ranges: &[Range]) {
        let temp = self.ranges.candidates[c].temp;
        let tree = if self.body.temp(temp).ty.is_some_and(|t| t.float_rank() != 0) {
            &mut self.xmm
        } else {
            &mut self.gpr
        };
        for range in ranges {
            tree.add(range.first, range.last, 1, 1 << r.code());
        }
        self.registers[temp.index()] = Some(r);
        if SAVED.contains(&r) || r == R::RBP {
            self.saved |= 1 << r.code();
        }
    }
}
fn pressure(
    body: &Body,
    facts: &Facts,
    addresses: &Addresses,
    ranges: &Ranges,
) -> Result<(Pressure, Pressure), Error> {
    let mut gpr = vec![0; ranges.total + 2];
    let mut xmm = vec![0; ranges.total + 2];
    for i in 0..body.temps().len() {
        let t = body.temp_id(i).ok_or(Error::Limit)?;
        if facts.global[i] || !eligible(body, facts, addresses, t) {
            continue;
        }
        let def = facts.definitions[i];
        let b = def.map_or(body.layout[0], |p| facts.flow.blocks[p]);
        if !facts.flow.reachable[b.index()] {
            continue;
        }
        let base = ranges.bases[b.index()];
        let first = def.map_or(base, |p| {
            base + 3 + 2 * (p - facts.flow.ranges[b.index()].start)
        });
        let last = facts.intervals[i]
            .last
            .or(def)
            .unwrap_or(facts.flow.ranges[b.index()].start);
        let past =
            base + 4 + 2 * (last.max(def.unwrap_or(last)) - facts.flow.ranges[b.index()].start);
        let diff = if body.temp(t).ty.is_some_and(|ty| ty.float_rank() != 0) {
            &mut xmm
        } else {
            &mut gpr
        };
        diff[first] += 1;
        diff[past] -= 1;
    }
    for s in 1..gpr.len() {
        gpr[s] += gpr[s - 1];
        xmm[s] += xmm[s - 1];
    }
    gpr.pop();
    xmm.pop();
    Ok((Pressure::new(&gpr)?, Pressure::new(&xmm)?))
}
fn use_hints(
    body: &Body,
    facts: &Facts,
    index: &[Option<usize>],
    calls: &super::call_facts::Calls,
    m: usize,
) -> Vec<Option<R>> {
    let mut hints = vec![None; m];
    for (p, &id) in facts.flow.instructions.iter().enumerate() {
        if !facts.active[p] {
            continue;
        }
        let inst = body.instruction(id);
        let mut hint = |v, r| {
            if let Some(t) = resolved(body, facts, v)
                && let Some(c) = index[t.index()]
                && hints[c].is_none()
            {
                hints[c] = Some(r);
            }
        };
        match &inst.kind {
            I::Return(ty, Some(v)) if !matches!(ty, Type::Object { .. }) => hint(
                *v,
                if ty.float_rank() != 0 {
                    R::XMM0
                } else {
                    R::RAX
                },
            ),
            I::Call(call) => {
                if let Some(plan) = calls.at(p) {
                    for (&v, arg) in body
                        .arguments(call.arguments)
                        .iter()
                        .zip(&plan.layout.arguments)
                    {
                        if let [part] = plan.layout.parts(arg)
                            && let Some(r) = part.register
                        {
                            hint(v, r);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    hints
}
#[expect(
    clippy::too_many_arguments,
    reason = "one typed function allocation boundary"
)]
pub(super) fn analyze(
    program: &Program,
    body: &Body,
    facts: &mut Facts,
    addresses: &Addresses,
    layout: &Layout,
    reserved: u32,
    rbp_free: bool,
    host: bool,
    region: Option<&super::region_flow::Plan>,
    calls: &super::call_facts::Calls,
) -> Result<Globals, Error> {
    let mut out = Globals {
        splits: super::split_placement::Splits::default(),
        precise: true,
        entries: Vec::new(),
        defined: vec![0; body.blocks().len()],
        rbp_free,
        registers: vec![None; body.temps().len()],
        reserved: vec![0; body.blocks().len()],
        binding_first: vec![0; body.blocks().len() + 1],
        bindings: Vec::new(),
        count: crate::support::telemetry::counter(0),
        steps: 0,
        fallbacks: crate::support::telemetry::counter(0),
    };
    let has_eh = facts.flow.instructions.iter().enumerate().any(|(p, &id)| {
        facts.active[p]
            && matches!(
                body.instruction(id).kind,
                I::Runtime(Runtime::EhTry(_) | Runtime::EhCleanup(_))
            )
    });
    if has_eh && !host {
        return Ok(out);
    }
    let mut ranges = Ranges::collect(body, facts, addresses)?;
    if ranges.candidates.is_empty() {
        return Ok(out);
    }
    let mut exceptional = vec![Vec::new(); body.blocks().len()];
    if let Some(region) = region {
        for (p, &id) in facts.flow.instructions.iter().enumerate() {
            if facts.active[p]
                && let super::region_flow::Site::Landing(to) = region.sites[id.index()]
            {
                if body
                    .block(to)
                    .instructions
                    .iter()
                    .any(|&i| matches!(body.instruction(i).kind, I::Phi(..)))
                {
                    return Ok(out);
                }
                let b = facts.flow.blocks[p];
                exceptional[to.index()].push(b);
            }
        }
        for edges in &mut exceptional {
            edges.sort_by_key(|b| facts.flow.linear[b.index()]);
            edges.dedup();
        }
    }
    let weights = Weights::analyze(program, body, facts)?;
    ranges.live(body, facts, &weights, &exceptional)?;
    out.steps = ranges.steps;
    out.fallbacks = ranges.fallbacks;
    let m = ranges.candidates.len();
    let mut index = vec![None; body.temps().len()];
    for (c, candidate) in ranges.candidates.iter().enumerate() {
        index[candidate.temp.index()] = Some(c);
    }
    let mut group: Vec<_> = (0..m).collect();
    let mut tie = vec![None; m];
    for (c, candidate) in ranges.candidates.iter().enumerate() {
        let Some(p) = facts.definitions[candidate.temp.index()] else {
            continue;
        };
        let kind = &body.instruction(facts.flow.instructions[p]).kind;
        if let I::Phi(_, pairs) = kind {
            for &(_, v) in pairs {
                if let Some(t) = resolved(body, facts, v)
                    && let Some(other) = index[t.index()]
                {
                    let root = find(&mut group, c);
                    let r = find(&mut group, other);
                    group[r] = root;
                }
            }
        } else if tied(kind) {
            tie[c] = first(kind)
                .and_then(|v| resolved(body, facts, v))
                .and_then(|t| index[t.index()]);
        }
    }
    let hints = use_hints(body, facts, &index, calls, m);
    let (gpr, xmm) = pressure(body, facts, addresses, &ranges)?;
    let mut late = vec![0; ranges.total + 1];
    let mut early = late.clone();
    for (p, &(after, before)) in facts.clobbers.iter().enumerate() {
        if after | before != 0 {
            let b = facts.flow.blocks[p];
            let read = ranges.bases[b.index()] + 2 + 2 * (p - facts.flow.ranges[b.index()].start);
            late[read + 1] |= after;
            early[read] |= before;
        }
    }
    let mut assignment = Assignment {
        body,
        facts,
        ranges: &ranges,
        layout,
        gpr,
        xmm,
        late: super::global_placement::Ranges::new(&late, true)?,
        early: super::global_placement::Ranges::new(&early, true)?,
        hints,
        web_ranges: Vec::new(),
        registers: out.registers,
        saved: 0,
        reserved,
        rbp_free,
    };
    let mut order: Vec<_> = (0..m).collect();
    order.sort_by_key(|&c| {
        let cand = &ranges.candidates[c];
        (
            std::cmp::Reverse(cand.weight),
            cand.ranges
                .iter()
                .map(|r| r.last - r.first + 1)
                .sum::<usize>(),
            cand.temp.index(),
        )
    });
    let mut members = vec![Vec::new(); m];
    for c in 0..m {
        members[find(&mut group, c)].push(c);
    }
    let mut group_reg = vec![None; m];
    let mut splitter = super::split_placement::Planner::new(
        body,
        facts,
        &ranges.bases,
        &weights,
        &exceptional,
        region,
    );
    let mut replacements: Vec<Option<Vec<Range>>> = (0..m).map(|_| None).collect();
    let mut homed = Vec::new();
    let mut blocked = Vec::new();
    for c in order {
        if assignment.registers[ranges.candidates[c].temp.index()].is_some() {
            continue;
        }
        let root = find(&mut group, c);
        let tied = tie[c].and_then(|c| assignment.registers[ranges.candidates[c].temp.index()]);
        if group_reg[root].is_none()
            && let Some(reg) = assignment.assign_web(c, &members[root], tied)
        {
            group_reg[root] = Some(reg);
            continue;
        }
        let candidate = &ranges.candidates[c];
        let mut reg = assignment.choose(
            c,
            &candidate.ranges,
            candidate.weight,
            group_reg[root],
            tied,
        );
        if reg.is_none()
            && body
                .temp(candidate.temp)
                .ty
                .is_some_and(|t| t.float_rank() == 0)
        {
            let allowed = assignment.split_conflicts(c, &mut blocked);
            if let Some(homes) = splitter.select(candidate, allowed, &blocked) {
                reg =
                    assignment.choose(c, &splitter.ranges, candidate.weight, group_reg[root], tied);
                if let Some(r) = reg {
                    assignment.commit_ranges(c, r, &splitter.ranges);
                    replacements[c] = Some(std::mem::take(&mut splitter.ranges));
                    for mut movement in splitter.moves.iter().copied() {
                        movement.register = r;
                        out.splits.moves.push(movement);
                    }
                    if homes {
                        homed.extend(splitter.homed.iter().map(|&b| (b, candidate.temp)));
                        out.splits.blocks += 1;
                    } else {
                        out.splits.loops += 1;
                    }
                }
            }
        }
        if let Some(reg) = reg {
            if replacements[c].is_none() {
                assignment.commit(c, reg);
            }
            if group_reg[root].is_none() {
                group_reg[root] = Some(reg);
            }
        }
    }
    out.registers = assignment.registers;
    out.count =
        crate::support::telemetry::observe(|| out.registers.iter().filter(|r| r.is_some()).count());
    for (candidate, replacement) in ranges.candidates.iter_mut().zip(replacements) {
        if let Some(replacement) = replacement {
            candidate.ranges = replacement;
        }
    }
    out.splits.install(body, &mut homed);
    emission(body, facts, addresses, &ranges, &mut out)?;
    Ok(out)
}
fn computed(body: &Body, facts: &Facts, g: TempId, from: TempId) -> bool {
    let Some(p) = facts.definitions[g.index()] else {
        return false;
    };
    let kind = &body.instruction(facts.flow.instructions[p]).kind;
    if first(kind).and_then(|v| resolved(body, facts, v)) != Some(from) {
        return false;
    }
    match *kind {
        I::Convert(
            Convert::ZeroExtend | Convert::SignExtend | Convert::Truncate,
            to,
            source,
            _,
        ) => to.bits() != 0 && to != Type::I128 && source != Type::I128,
        I::Binary(op, _, _, _, b) => {
            tied(kind)
                && !matches!(op, Binary::Div | Binary::UDiv | Binary::Mod | Binary::UMod)
                && resolved(body, facts, b) != Some(from)
        }
        I::Unary(..) => tied(kind),
        _ => false,
    }
}
fn hint_chain(
    body: &Body,
    facts: &mut Facts,
    addresses: &Addresses,
    mut value: TempId,
    b: BlockId,
    reg: R,
) {
    for _ in 0..8 {
        let i = value.index();
        if facts.global[i]
            || !eligible(body, facts, addresses, value)
            || facts.intervals[i].hint.is_some()
            || i < body.parameter_count
            || body.temp(value).ty.is_some_and(|t| t.float_rank() != 0) != (reg.code() >= 16)
        {
            return;
        }
        let Some(p) = facts.definitions[i] else {
            return;
        };
        if facts.flow.blocks[p] != b {
            return;
        }
        facts.intervals[i].hint = Some(reg);
        let kind = &body.instruction(facts.flow.instructions[p]).kind;
        if !tied(kind) {
            return;
        }
        let Some(u) = first(kind).and_then(|v| resolved(body, facts, v)) else {
            return;
        };
        if facts.intervals[u.index()].last != Some(p) {
            return;
        }
        value = u;
    }
}
fn emission(
    body: &Body,
    facts: &mut Facts,
    addresses: &Addresses,
    ranges: &Ranges,
    out: &mut Globals,
) -> Result<(), Error> {
    let mut entries = Vec::new();
    for candidate in &ranges.candidates {
        let Some(reg) = out.registers[candidate.temp.index()] else {
            continue;
        };
        for r in &candidate.ranges {
            let base = ranges.bases[r.block.index()];
            let count = facts.flow.ranges[r.block.index()].len();
            let live_in = r.first == base;
            if !live_in && r.first >= base + 3 + 2 * count {
                continue;
            }
            entries.push((
                r.block,
                Entry {
                    temp: candidate.temp,
                    live_in,
                    release: if r.last >= base + 2 + 2 * count {
                        None
                    } else {
                        Some(facts.flow.ranges[r.block.index()].start + (r.last - base - 2) / 2)
                    },
                },
            ));
            if !live_in {
                out.defined[r.block.index()] |= 1 << reg.code();
            }
        }
    }
    // Storage uses block identities; sorting by identity retains temp order in
    // each CSR bucket even when forward references changed LowIR IDs.
    entries.sort_by_key(|(b, e)| (b.index(), e.temp.index()));
    for (b, e) in entries {
        out.binding_first[b.index() + 1] += 1;
        out.entries.push(e);
    }
    for b in 0..body.blocks().len() {
        out.binding_first[b + 1] += out.binding_first[b];
    }
    for &b in &body.layout {
        if !facts.flow.reachable[b.index()] {
            continue;
        }
        for edge in 0..facts.flow.successors(b).len() {
            let to = facts.flow.successors(b)[edge];
            for &id in &body.block(to).instructions {
                let inst = body.instruction(id);
                let I::Phi(_, pairs) = &inst.kind else {
                    break;
                };
                let Some(t) = inst.result else {
                    continue;
                };
                let Some(reg) = out.registers[t.index()] else {
                    continue;
                };
                if !facts.active[facts.flow.positions[id.index()]] || pairs.len() > 8 {
                    continue;
                }
                for &(pred, v) in pairs {
                    if pred != b {
                        continue;
                    }
                    let Some(v) = resolved(body, facts, v) else {
                        continue;
                    };
                    hint_chain(body, facts, addresses, v, b, reg);
                }
            }
        }
    }
    let mut defs = Vec::new();
    for (i, reg) in out.registers.iter().enumerate() {
        if let Some(reg) = reg
            && let Some(p) = facts.definitions[i]
            && !matches!(
                body.instruction(facts.flow.instructions[p]).kind,
                I::Phi(..)
            )
        {
            defs.push((
                facts.flow.blocks[p],
                p,
                body.temp_id(i).ok_or(Error::Limit)?,
                *reg,
            ));
        }
    }
    defs.sort_by_key(|(b, p, _, _)| (b.index(), *p));
    let mut homed = vec![false; body.temps().len()];
    for t in &out.splits.homes {
        homed[t.index()] = true;
    }
    for (i, &homed) in homed.iter().enumerate() {
        let t = body.temp_id(i).ok_or(Error::Limit)?;
        if out.registers[i].is_some() && !homed || !eligible(body, facts, addresses, t) {
            continue;
        }
        let def = facts.definitions[i];
        let b = def.map_or(body.layout[0], |p| facts.flow.blocks[p]);
        let last = facts.intervals[i].last.or(def);
        let first = defs.partition_point(|&(block, p, _, _)| {
            block.index() < b.index() || block == b && def.is_some_and(|d| p <= d)
        });
        for &(block, p, g, reg) in defs[first..]
            .iter()
            .take_while(|&&(block, p, _, _)| block == b && last.is_some_and(|last| p <= last))
        {
            if out.registers[i].is_none()
                && facts.intervals[i].last == Some(p)
                && computed(body, facts, g, t)
            {
                hint_chain(body, facts, addresses, t, block, reg);
            } else {
                facts.intervals[i].forbid |= 1 << reg.code();
            }
        }
    }
    Ok(())
}

pub(super) fn rbp_available(
    body: &Body,
    facts: &Facts,
    calls: &super::call_facts::Calls,
    arity: crate::lowir::model::Arity,
    reserved: u32,
    model: super::object_compile::ObjectModel,
) -> bool {
    facts.level >= 1
        && model == super::object_compile::ObjectModel::Host
        && arity != crate::lowir::model::Arity::Variadic
        && reserved == 0
        && facts.flow.instructions.iter().enumerate().all(|(p, &id)| {
            if !facts.active[p] {
                return true;
            }
            match body.instruction(id).kind {
                I::Runtime(Runtime::StackAlloc(_) | Runtime::VaStart(_) | Runtime::VaArg(..)) => {
                    false
                }
                I::Convert(_, to, from, _) if to == Type::I128 || from == Type::I128 => false,
                I::Call(_) => calls
                    .at(p)
                    .is_none_or(|call| call.layout.stack == 0 && call.layout.stack_align <= 16),
                _ => true,
            }
        })
}
