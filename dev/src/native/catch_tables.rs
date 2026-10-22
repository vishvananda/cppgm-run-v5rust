//! Action chains, catch type indexes and selector maps (`select/host_eh_tables.cpp`). See NOTICE.
use super::{
    handler_regions::{Regions, State},
    region_flow::{Analysis, Site, clause},
};
use crate::lowir::{
    Error,
    model::{
        BlockId, InstId, InstructionKind as I, RuntimeInstruction as Runtime, SymbolId, Value,
    },
};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

struct Chains {
    identity: bool,
    type_index: HashMap<Option<SymbolId>, i32>,
    filters: HashMap<InstId, i32>,
    entries: Vec<i32>,
    ranges: Vec<Option<Range<usize>>>,
    work: Vec<u8>,
}
fn selector(kind: &I) -> i64 {
    match kind {
        I::Runtime(Runtime::EhCatch(_, s) | Runtime::EhCatchAll(s) | Runtime::EhFilter(_, s)) => {
            s.unwrap_or(0)
        }
        _ => 0,
    }
}
impl Chains {
    fn choose(a: &mut Analysis<'_>) -> Result<Self, Error> {
        let mut chained = vec![false; a.body.blocks().len()];
        for &block in &a.body.layout {
            if a.plan.indexes[block.index()].is_none() {
                continue;
            }
            let mut node = a.landing_nodes[block.index()].ok_or(Error::Limit)?;
            while !node.empty() {
                let record = a.regions.get(node);
                let landing = record.landing.ok_or(Error::Limit)?;
                if chained[landing.index()] {
                    break;
                }
                chained[landing.index()] = true;
                node = a.regions.get(record.parent).protect;
            }
        }
        let mut by_selector = Vec::new();
        let mut clauses = 0;
        let mut identity = true;
        let mut filtered = false;
        for &block in &a.body.layout {
            if !chained[block.index()] {
                continue;
            }
            for &id in &a.body.block(block).instructions {
                let kind = &a.body.instruction(id).kind;
                if !clause(kind) {
                    break;
                }
                filtered |= matches!(kind, I::Runtime(Runtime::EhFilter(..)));
                let ty = match *kind {
                    I::Runtime(Runtime::EhCatch(ty, _)) => Some(ty),
                    I::Runtime(Runtime::EhCatchAll(_)) => None,
                    _ => continue,
                };
                let s = selector(kind);
                if !(1..=65536).contains(&s) {
                    return Err(Error::Diagnostic(
                        "a catch selector outside 1..65536".into(),
                    ));
                }
                let s = usize::try_from(s).map_err(|_| Error::Limit)?;
                clauses += 1;
                if s >= by_selector.len() {
                    by_selector.resize(s + 1, None);
                }
                if let Some(old) = by_selector[s] {
                    identity &= old == ty;
                } else {
                    by_selector[s] = Some(ty);
                }
            }
        }
        if by_selector.len() > 4 * clauses + 17 || filtered {
            identity = false;
        }
        if identity {
            a.plan
                .tables
                .types
                .extend(by_selector.iter().skip(1).map(|ty| ty.unwrap_or(None)));
        }
        let mut ranges = vec![None; a.regions.len()];
        ranges[0] = Some(0..0);
        Ok(Self {
            identity,
            type_index: HashMap::new(),
            filters: HashMap::new(),
            entries: Vec::new(),
            ranges,
            work: vec![0; a.body.blocks().len()],
        })
    }
    fn type_index(&mut self, a: &mut Analysis<'_>, ty: Option<SymbolId>) -> Result<i32, Error> {
        if let Some(&index) = self.type_index.get(&ty) {
            return Ok(index);
        }
        a.plan.tables.types.push(ty);
        let index = i32::try_from(a.plan.tables.types.len()).map_err(|_| Error::Limit)?;
        self.type_index.insert(ty, index);
        Ok(index)
    }
    fn clause_index(&mut self, a: &mut Analysis<'_>, id: InstId) -> Result<i32, Error> {
        let kind = &a.body.instruction(id).kind;
        if self.identity {
            return i32::try_from(selector(kind)).map_err(|_| Error::Limit);
        }
        let ty = if let I::Runtime(Runtime::EhCatch(ty, _)) = *kind {
            Some(ty)
        } else {
            None
        };
        self.type_index(a, ty)
    }
    fn filter(&mut self, a: &mut Analysis<'_>, id: InstId) -> Result<i32, Error> {
        if let Some(&value) = self.filters.get(&id) {
            return Ok(value);
        }
        let I::Runtime(Runtime::EhFilter(arguments, _)) = a.body.instruction(id).kind else {
            return Err(Error::Limit);
        };
        let value = i32::try_from(a.plan.tables.specs.len())
            .map_err(|_| Error::Limit)?
            .checked_add(1)
            .ok_or(Error::Limit)?
            .wrapping_neg();
        for &ty in a.body.arguments(arguments) {
            let Value::Symbol(ty) = ty else {
                return Err(Error::Limit);
            };
            let index = self.type_index(a, Some(ty))?;
            super::host_unwind::uleb(&mut a.plan.tables.specs, index.cast_unsigned().into());
        }
        a.plan.tables.specs.push(0);
        self.filters.insert(id, value);
        Ok(value)
    }
    fn own(&mut self, a: &mut Analysis<'_>, block: BlockId) -> Result<(Vec<i32>, bool), Error> {
        let mut filters = Vec::new();
        let mut catch_all = false;
        for &id in &a.body.block(block).instructions {
            let kind = &a.body.instruction(id).kind;
            if !clause(kind) {
                break;
            }
            let filter = match kind {
                I::Runtime(Runtime::EhFilter(..)) => self.filter(a, id)?,
                I::Runtime(Runtime::EhCleanup(None)) => 0,
                _ => self.clause_index(a, id)?,
            };
            filters.push(filter);
            catch_all |= matches!(kind, I::Runtime(Runtime::EhCatchAll(_)));
        }
        Ok((filters, catch_all))
    }
    // Effect and successor discovery for selector 0. Edges keep oracle DFS order.
    fn work_edges(a: &Analysis<'_>, block: BlockId) -> (bool, Vec<BlockId>) {
        let mut selector = None;
        let mut leading = a.landing_nodes[block.index()].is_some();
        let mut edges = Vec::new();
        for &id in &a.body.block(block).instructions {
            let kind = &a.body.instruction(id).kind;
            if !clause(kind) {
                leading = false;
            }
            match kind {
                I::Call(_)
                | I::Store(..)
                | I::CopyObject(..)
                | I::ZeroInit(..)
                | I::Return(..)
                | I::Runtime(
                    Runtime::AtomicStore(..)
                    | Runtime::AtomicUpdate(..)
                    | Runtime::AtomicCompareExchange { .. }
                    | Runtime::VaStart(_)
                    | Runtime::Throw(..),
                ) => return (true, edges),
                I::Runtime(Runtime::EhCatchAll(_)) if !leading => return (true, edges),
                I::Runtime(Runtime::ExceptionSelector(_)) => {
                    selector = a.body.instruction(id).result;
                }
                I::Runtime(Runtime::Resume) | I::Jump(_) => match a.plan.sites[id.index()] {
                    Site::Landing(to) => edges.push(to),
                    Site::Terminal => {}
                    _ if matches!(kind, I::Jump(_)) => {
                        edges.extend_from_slice(a.flow.successors(block));
                    }
                    _ => {}
                },
                I::Switch(..) => edges.extend_from_slice(a.flow.successors(block)),
                I::Branch(condition, yes, no) => {
                    let (y, n) = a.refine(*condition, selector, 1);
                    if y != 0 {
                        edges.push(*yes);
                    }
                    if n != 0 {
                        edges.push(*no);
                    }
                }
                _ => {}
            }
        }
        (false, edges)
    }
    // Explicit DFS frames preserve visiting/none/work memos without native recursion.
    fn unmatched_work(&mut self, a: &Analysis<'_>, block: BlockId) -> bool {
        if self.work[block.index()] != 0 {
            return self.work[block.index()] == 3;
        }
        let mut stack = vec![(block, Vec::new(), 0)];
        while let Some((block, edges, next)) = stack.last_mut() {
            if self.work[block.index()] == 0 {
                self.work[block.index()] = 1;
                let (work, found) = Self::work_edges(a, *block);
                *edges = found;
                if work {
                    self.work[block.index()] = 3;
                }
            }
            if self.work[block.index()] == 3 {
                let _ = stack.pop();
                if let Some((parent, _, _)) = stack.last() {
                    self.work[parent.index()] = 3;
                }
                continue;
            }
            if let Some(&child) = edges.get(*next) {
                *next += 1;
                if self.work[child.index()] == 0 {
                    stack.push((child, Vec::new(), 0));
                } else if self.work[child.index()] == 3 {
                    self.work[block.index()] = 3;
                }
            } else {
                self.work[block.index()] = 2;
                let _ = stack.pop();
            }
        }
        self.work[block.index()] == 3
    }
    fn chain(&mut self, a: &mut Analysis<'_>, node: State) -> Result<(), Error> {
        let mut pending = Vec::new();
        let mut k = node;
        while !k.empty() && self.ranges[Regions::index(k)].is_none() {
            pending.push(k);
            k = a.regions.get(a.regions.get(k).parent).protect;
        }
        while let Some(k) = pending.pop() {
            let node = a.regions.get(k);
            let landing = node.landing.ok_or(Error::Limit)?;
            let (mut own, catch_all) = self.own(a, landing)?;
            if own.is_empty() || node.cleanup && !catch_all {
                own.push(0);
            }
            let first = self.entries.len();
            let mut seen = HashSet::new();
            let mut cleanup = false;
            let mut ended = catch_all;
            for filter in own {
                cleanup |= filter == 0;
                if seen.insert(filter) {
                    self.entries.push(filter);
                }
            }
            let below = a.regions.get(node.parent).protect;
            if !below.empty() && !catch_all {
                let range = self.ranges[Regions::index(below)]
                    .clone()
                    .ok_or(Error::Limit)?;
                for index in range {
                    let filter = self.entries[index];
                    cleanup |= filter == 0;
                    ended |= filter > 0
                        && a.plan.tables.types
                            [usize::try_from(filter - 1).map_err(|_| Error::Limit)?]
                        .is_none();
                    if seen.insert(filter) {
                        self.entries.push(filter);
                    }
                }
            }
            if !cleanup && !ended && self.unmatched_work(a, landing) {
                self.entries.push(0);
            }
            self.ranges[Regions::index(k)] = Some(first..self.entries.len());
        }
        Ok(())
    }
    fn landing(&mut self, a: &mut Analysis<'_>, block: BlockId, index: usize) -> Result<(), Error> {
        let node = a.landing_nodes[block.index()].ok_or(Error::Limit)?;
        self.chain(a, node)?;
        let range = self.ranges[Regions::index(node)]
            .clone()
            .ok_or(Error::Limit)?;
        let first = a.plan.tables.filters.len();
        a.plan
            .tables
            .filters
            .extend_from_slice(&self.entries[range]);
        let filters = first..a.plan.tables.filters.len();
        let first = a.plan.tables.map.len();
        let mut seen = HashSet::new();
        let mut translate = a.mid_clauses;
        for &id in &a.body.block(block).instructions {
            let kind = &a.body.instruction(id).kind;
            if !clause(kind) {
                break;
            }
            let filter = match kind {
                I::Runtime(Runtime::EhFilter(..)) => self.filter(a, id)?,
                I::Runtime(Runtime::EhCatch(..) | Runtime::EhCatchAll(_)) => {
                    let f = self.clause_index(a, id)?;
                    if !seen.insert(f) {
                        continue;
                    }
                    f
                }
                _ => continue,
            };
            let source_selector = selector(kind);
            // The MIR map carries target int32 bits, but the identity test in
            // the frozen owner compares against the original int64 selector.
            let bits = source_selector.to_le_bytes();
            let s = i32::from_le_bytes([bits[0], bits[1], bits[2], bits[3]]);
            a.plan.tables.map.push((filter, s));
            translate |= i64::from(filter) != source_selector;
        }
        translate |= a.plan.tables.filters[filters.clone()]
            .iter()
            .any(|&f| f > 0 && !seen.contains(&f));
        let landing = &mut a.plan.tables.landings[index];
        landing.filters = filters;
        landing.map = first..a.plan.tables.map.len();
        landing.translate = translate;
        Ok(())
    }
}
pub(super) fn build(a: &mut Analysis<'_>) -> Result<(), Error> {
    if a.plan.tables.landings.is_empty() {
        return Ok(());
    }
    for &block in &a.body.layout {
        let b = block.index();
        // C++ rebuilds reachable predecessors after eh_prune, before checking
        // these edges. The shared Flow is restricted after table construction;
        // use the final live set here for both endpoints of each edge.
        if a.plan.live[b] != 0
            && a.landing_nodes[b].is_some()
            && a.plan.entry_selectors[b] == 0
            && a.landing_mask(block) & !1 != 0
            && a.flow
                .predecessors(block)
                .iter()
                .any(|p| a.plan.live[p.index()] != 0 && a.incoming[p.index()].is_some())
        {
            return Err(Error::Diagnostic(
                "a jump into a landing with catch clauses".into(),
            ));
        }
    }
    let mut chains = Chains::choose(a)?;
    for &block in &a.body.layout {
        if let Some(index) = a.plan.indexes[block.index()] {
            chains.landing(a, block, index.index())?;
        }
    }
    a.plan.tables.raw = a.plan.tables.landings.iter().any(|l| l.translate);
    Ok(())
}
