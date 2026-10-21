//! O0 bounded backward liveness, scalar hulls and function register plans. See NOTICE.
use super::{
    abi::{CALLER_SAVED, Layout},
    address_facts::{AddressBase, Addresses},
    placement::SAVED,
    value_facts::{Alias, Facts},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstructionKind as I, TempId, Value},
    vocabulary::Binary,
};
#[derive(Clone, Copy)]
struct Hull {
    temp: TempId,
    start: usize,
    end: usize,
}
pub(super) struct Globals {
    pub(super) registers: Vec<Option<R>>,
    pub(super) reserved: Vec<u32>,
    pub(super) binding_first: Vec<usize>,
    pub(super) bindings: Vec<TempId>,
    pub(super) count: usize,
    pub(super) steps: usize,
    pub(super) fallbacks: usize,
}
// Counts of each physical clobber bit: fixed-width prefix differences answer
// the same range OR in constant time, with linear function-owned storage.
struct Ranges {
    prefix: Vec<[u32; 16]>,
}
impl Ranges {
    fn new(values: &[u32]) -> Result<Self, Error> {
        let mut prefix = Vec::with_capacity(values.len() + 1);
        prefix.push([0_u32; 16]);
        let mut counts = [0_u32; 16];
        for &value in values {
            for (reg, count) in counts.iter_mut().enumerate() {
                if value & (1 << reg) != 0 {
                    *count = count.checked_add(1).ok_or(Error::Limit)?;
                }
            }
            prefix.push(counts);
        }
        Ok(Self { prefix })
    }
    fn query(&self, first: usize, past: usize) -> u32 {
        if first >= past {
            return 0;
        }
        let mut mask = 0;
        for reg in 0..16 {
            if self.prefix[first][reg] != self.prefix[past][reg] {
                mask |= 1 << reg;
            }
        }
        mask
    }
}
pub(super) fn parts(
    body: &Body,
    facts: &Facts,
    addresses: &Addresses,
    value: Value,
) -> [Option<TempId>; 3] {
    let Alias::Temporary(temp) = facts.resolve(body, value) else {
        return [None; 3];
    };
    if addresses.lazy[temp.index()]
        && let Some(expr) = addresses.expressions[temp.index()]
    {
        [
            None,
            if let AddressBase::Temporary(base) = expr.base {
                Some(base)
            } else {
                None
            },
            expr.index,
        ]
    } else {
        [Some(temp), None, None]
    }
}
impl Globals {
    pub(super) fn analyze(
        body: &Body,
        facts: &Facts,
        addresses: &Addresses,
        layout: &Layout,
        reserved: u32,
    ) -> Result<Self, Error> {
        let mut out = Self {
            registers: vec![None; body.temps().len()],
            reserved: vec![0; body.blocks().len()],
            binding_first: vec![0; body.blocks().len() + 1],
            bindings: Vec::new(),
            count: 0,
            steps: 0,
            fallbacks: 0,
        };
        let mut candidates = Vec::new();
        for (index, temp) in body.temps().iter().enumerate() {
            if !facts.global[index]
                || facts.intervals[index].uses == 0
                || addresses.lazy[index]
                || addresses.folded[index].is_some()
                || facts.fused[index]
            {
                continue;
            }
            let id = body.find_temp(temp.name).ok_or(Error::Limit)?;
            if !matches!(facts.resolve(body,Value::Temp(id)),Alias::Temporary(t) if t==id) {
                continue;
            }
            if index >= body.parameter_count
                && facts.definitions[index]
                    .is_none_or(|p| !facts.flow.reachable[facts.flow.blocks[p].index()])
            {
                continue;
            }
            let ty = temp.ty.ok_or(Error::Limit)?;
            if matches!(
                ty,
                crate::lowir::vocabulary::Type::Object { .. }
                    | crate::lowir::vocabulary::Type::I128
            ) {
                continue;
            }
            super::memory::scalar(ty)?;
            candidates.push(id);
        }
        if candidates.is_empty() {
            return Ok(out);
        }
        let mut hulls = out.hulls(body, facts, addresses, &candidates)?;
        crate::support::ordering::sort_by(&mut hulls, |a, b| (a.start, a.end) < (b.start, b.end));
        let positions = body
            .layout
            .iter()
            .filter(|b| facts.flow.reachable[b.index()])
            .map(|&b| facts.flow.end(b) + 1)
            .max()
            .unwrap_or(1);
        i32::try_from(positions).map_err(|_| Error::Limit)?;
        let mut clobbers = vec![0; positions];
        let mut early = vec![0; positions];
        for (pos, &(late, before)) in facts.clobbers.iter().enumerate() {
            if facts.active[pos] {
                let at = facts.flow.linear_position(pos);
                clobbers[at] = late | before;
                early[at] = before;
            }
        }
        let ranges = Ranges::new(&clobbers)?;
        let incoming = |temp: TempId| {
            layout.arguments.get(temp.index()).and_then(|arg| {
                let [part] = layout.parts(arg) else {
                    return None;
                };
                part.register
            })
        };
        let forbidden = |h: Hull| ranges.query(h.start + 1, h.end) | early[h.end];
        let displaced = hulls
            .iter()
            .filter(|h| h.temp.index() < body.parameter_count)
            .filter(|&&h| incoming(h.temp).is_none_or(|r| forbidden(h) & (1 << r.code()) != 0))
            .count();
        let mut active: Vec<(Hull, R)> = Vec::new();
        let mut bindings = Vec::new();
        for hull in hulls {
            active.retain(|(h, _)| h.end >= hull.start);
            if active.len() >= 7 {
                continue;
            }
            let occupied = active.iter().fold(0, |mask, (_, r)| mask | (1 << r.code()));
            let mut forbid = forbidden(hull);
            if let Some(def) = facts.definitions[hull.temp.index()]
                && matches!(
                    body.instruction(facts.flow.instructions[def]).kind,
                    I::Binary(Binary::Shl | Binary::Shr | Binary::UShr, ..)
                )
            {
                forbid |= facts.clobbers[def].1;
            }
            let allowed = (CALLER_SAVED | SAVED.iter().fold(0, |mask, r| mask | (1 << r.code())))
                & !forbid
                & !occupied
                & !reserved;
            let mut chosen = None;
            if hull.temp.index() < body.parameter_count {
                chosen = incoming(hull.temp).filter(|r| allowed & (1 << r.code()) != 0);
                if chosen.is_none() && displaced > 4 {
                    continue;
                }
            }
            if chosen.is_none() && forbid & CALLER_SAVED != 0 {
                chosen = SAVED.into_iter().find(|r| allowed & (1 << r.code()) != 0);
            }
            if chosen.is_none() {
                chosen = [R::R8, R::R9, R::RDI, R::RSI, R::RDX, R::RCX, R::RAX]
                    .into_iter()
                    .chain(SAVED)
                    .find(|r| allowed & (1 << r.code()) != 0);
            }
            let Some(reg) = chosen else {
                continue;
            };
            out.registers[hull.temp.index()] = Some(reg);
            out.count += 1;
            active.push((hull, reg));
            let first = body
                .layout
                .partition_point(|b| facts.flow.linear[b.index()] <= hull.start)
                .saturating_sub(1);
            for &block in body.layout[first..]
                .iter()
                .take_while(|b| facts.flow.linear[b.index()] <= hull.end)
            {
                if facts.flow.reachable[block.index()] && facts.flow.end(block) >= hull.start {
                    out.reserved[block.index()] |= 1 << reg.code();
                    bindings.push((block, hull.temp));
                }
            }
        }
        for &(block, _) in &bindings {
            out.binding_first[block.index() + 1] += 1;
        }
        for block in 0..body.blocks().len() {
            out.binding_first[block + 1] += out.binding_first[block];
        }
        if let Some(&(_, temp)) = bindings.first() {
            out.bindings.resize(bindings.len(), temp);
            let mut fill = out.binding_first[..body.blocks().len()].to_vec();
            for (block, temp) in bindings {
                out.bindings[fill[block.index()]] = temp;
                fill[block.index()] += 1;
            }
        }
        Ok(out)
    }
    fn hulls(
        &mut self,
        body: &Body,
        facts: &Facts,
        addresses: &Addresses,
        candidates: &[TempId],
    ) -> Result<Vec<Hull>, Error> {
        let mut index = vec![None; body.temps().len()];
        for (at, temp) in candidates.iter().enumerate() {
            index[temp.index()] = Some(at);
        }
        let mut records = Vec::new();
        let mut first = vec![0; candidates.len() + 1];
        let mut record = |value, block, at| {
            for part in parts(body, facts, addresses, value).into_iter().flatten() {
                if let Some(i) = index[part.index()] {
                    first[i + 1] += 1;
                    records.push((i, block, at));
                }
            }
        };
        for (pos, &id) in facts.flow.instructions.iter().enumerate() {
            let inst = body.instruction(id);
            let lazy = inst.result.is_some_and(|t| addresses.lazy[t.index()]);
            if !facts.flow.reachable[facts.flow.blocks[pos].index()]
                || !facts.active[pos] && facts.reads[pos] == pos && !lazy
            {
                continue;
            }
            if let I::Phi(_, pairs) = &inst.kind {
                for &(pred, value) in pairs {
                    if facts.flow.reachable[pred.index()] {
                        record(value, pred, facts.flow.end(pred));
                    }
                }
            } else {
                let read = facts.reads[pos];
                body.visit_values(&inst.kind, |v| {
                    record(v, facts.flow.blocks[read], facts.flow.linear_position(read));
                });
            }
        }
        for i in 0..candidates.len() {
            first[i + 1] += first[i];
        }
        let mut fill = first[..candidates.len()].to_vec();
        let mut locations = vec![(body.layout[0], 0); records.len()];
        for (owner, block, at) in records {
            locations[fill[owner]] = (block, at);
            fill[owner] += 1;
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
        let mut marked = vec![0; body.blocks().len()];
        let mut work = Vec::new();
        let mut hulls = Vec::with_capacity(candidates.len());
        for (at, &temp) in candidates.iter().enumerate() {
            let def = facts.definitions[temp.index()];
            let block = def.map_or(body.layout[0], |p| facts.flow.blocks[p]);
            let phi = def.is_some_and(|p| {
                matches!(
                    body.instruction(facts.flow.instructions[p]).kind,
                    I::Phi(..)
                )
            });
            let mut hull = Hull {
                temp,
                start: if temp.index() < body.parameter_count || phi {
                    facts.flow.linear[block.index()]
                } else {
                    facts.flow.linear_position(def.ok_or(Error::Limit)?)
                },
                end: 0,
            };
            hull.end = hull.start;
            let epoch = at + 1;
            let live_in =
                |b: BlockId, h: &mut Hull, marked: &mut [usize], work: &mut Vec<BlockId>| {
                    if marked[b.index()] != epoch {
                        marked[b.index()] = epoch;
                        h.start = h.start.min(facts.flow.linear[b.index()]);
                        h.end = h.end.max(facts.flow.end(b));
                        work.push(b);
                    }
                };
            let used = &locations[first[at]..first[at + 1]];
            self.steps += used.len();
            for &(used, lin) in used {
                hull.end = hull.end.max(lin);
                hull.start = hull.start.min(lin);
                if used != block
                    || lin
                        < if temp.index() < body.parameter_count || phi {
                            facts.flow.linear[block.index()]
                        } else {
                            facts.flow.linear_position(def.ok_or(Error::Limit)?)
                        }
                {
                    live_in(used, &mut hull, &mut marked, &mut work);
                }
            }
            if phi {
                for &pred in facts.flow.predecessors(block) {
                    hull.start = hull.start.min(facts.flow.end(pred));
                    hull.end = hull.end.max(facts.flow.end(pred));
                }
            }
            while !work.is_empty() && self.steps <= budget {
                let current = work.pop().ok_or(Error::Limit)?;
                let preds = facts.flow.predecessors(current);
                self.steps += preds.len();
                for &pred in preds {
                    hull.end = hull.end.max(facts.flow.end(pred));
                    if pred != block {
                        live_in(pred, &mut hull, &mut marked, &mut work);
                    }
                }
            }
            if self.steps > budget {
                self.fallbacks = candidates.len() - at;
                break;
            }
            hulls.push(hull);
        }
        Ok(hulls)
    }
}
