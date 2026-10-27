//! Host handler reachability and selector dataflow (`select/host_eh.cpp`). See NOTICE.
use super::{
    control_flow::Flow,
    handler_regions::{Regions, State, Top},
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, InstId, InstructionKind as I, Program, RuntimeInstruction as Runtime,
        SymbolId, TempId, Unwind, Value,
    },
    vocabulary::Compare,
};
use std::{cmp::Reverse, collections::BinaryHeap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct LandingId(std::num::NonZeroU32);
impl LandingId {
    pub(super) fn new(index: usize) -> Result<Self, Error> {
        u32::try_from(index)
            .ok()
            .and_then(|n| n.checked_add(1))
            .and_then(std::num::NonZeroU32::new)
            .map(Self)
            .ok_or(Error::Limit)
    }
    pub(super) fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Site {
    #[default]
    None,
    Landing(BlockId),
    Terminal,
    Clause,
}
#[derive(Default)]
pub(super) struct Landing {
    pub(super) filters: std::ops::Range<usize>,
    pub(super) map: std::ops::Range<usize>,
    pub(super) translate: bool,
    pub(super) pad: Option<super::mir::BlockId>,
    pub(super) select: Option<super::mir::BlockId>,
}
#[derive(Default)]
pub(super) struct Tables {
    pub(super) walks: crate::support::telemetry::Counter<usize>,
    pub(super) states: crate::support::telemetry::Counter<usize>,
    pub(super) landings: Vec<Landing>,
    pub(super) filters: Vec<i32>,
    pub(super) map: Vec<(i32, i32)>,
    pub(super) types: Vec<Option<SymbolId>>,
    pub(super) specs: Vec<u8>,
    pub(super) slot: i32,
    pub(super) raw: bool,
}
pub(super) struct Plan {
    pub(super) sites: Vec<Site>,
    pub(super) live: Vec<u8>,
    pub(super) resumed: Vec<bool>,
    pub(super) indexes: Vec<Option<LandingId>>,
    pub(super) entry_selectors: Vec<i64>,
    pub(super) tables: Tables,
}
pub(super) struct Analysis<'a> {
    pub(super) body: &'a Body,
    pub(super) flow: &'a Flow,
    pub(super) regions: Regions,
    pub(super) incoming: Vec<Option<State>>,
    masks: Vec<u64>,
    pub(super) landing_nodes: Vec<Option<State>>,
    pub(super) mid_clauses: bool,
    pub(super) plan: Plan,
}
pub(super) fn clause(kind: &I) -> bool {
    matches!(
        kind,
        I::Runtime(
            Runtime::EhCatch(..)
                | Runtime::EhCatchAll(_)
                | Runtime::EhFilter(..)
                | Runtime::EhCleanup(None)
        )
    )
}
pub(super) fn selector_bit(value: i64) -> u64 {
    1 << if (0..=62).contains(&value) { value } else { 63 }
}
pub(super) fn needed(body: &Body, flow: &Flow) -> bool {
    flow.instructions.iter().enumerate().any(|(pos, &id)| {
        flow.reachable[flow.blocks[pos].index()]
            && matches!(
                body.instruction(id).kind,
                I::Runtime(
                    Runtime::Exception(_)
                        | Runtime::ExceptionSelector(_)
                        | Runtime::Resume
                        | Runtime::Throw(..)
                        | Runtime::EhTry(_)
                        | Runtime::EhCleanup(_)
                        | Runtime::EhCatch(..)
                        | Runtime::EhCatchAll(_)
                        | Runtime::EhFilter(..)
                        | Runtime::EhEnd
                )
            )
    })
}
impl Analysis<'_> {
    fn enter(
        &mut self,
        block: BlockId,
        state: State,
        mask: u64,
        work: &mut Vec<BlockId>,
    ) -> Result<(), Error> {
        let at = block.index();
        if let Some(old) = self.incoming[at] {
            let merged = self.regions.merge(old, state)?;
            if merged != old || self.masks[at] | mask != self.masks[at] {
                self.incoming[at] = Some(merged);
                self.masks[at] |= mask;
                work.push(block);
            }
        } else {
            self.incoming[at] = Some(state);
            self.masks[at] = mask;
            work.push(block);
        }
        Ok(())
    }
    pub(super) fn landing_mask(&self, block: BlockId) -> u64 {
        let mut mask = 1;
        let mut catch_all = false;
        for &id in &self.body.block(block).instructions {
            match &self.body.instruction(id).kind {
                I::Runtime(
                    Runtime::EhCatch(_, s) | Runtime::EhCatchAll(s) | Runtime::EhFilter(_, s),
                ) => mask |= selector_bit(s.unwrap_or(0)),
                kind if clause(kind) => {}
                _ => break,
            }
            catch_all |= matches!(
                self.body.instruction(id).kind,
                I::Runtime(Runtime::EhCatchAll(_))
            );
        }
        if catch_all { mask & !1 } else { mask }
    }
    fn edge_mask(&self, block: BlockId, mask: u64) -> u64 {
        let entry = self.plan.entry_selectors[block.index()];
        if entry != 0 && mask & 1 != 0 {
            (mask & !1) | selector_bit(entry)
        } else {
            mask
        }
    }
    pub(super) fn refine(
        &self,
        condition: Value,
        selector: Option<TempId>,
        mask: u64,
    ) -> (u64, u64) {
        let Some(selector) = selector else {
            return (mask, mask);
        };
        let Value::Temp(temp) = condition else {
            return (mask, mask);
        };
        let Some(id) = self.body.temp(temp).definition else {
            return (mask, mask);
        };
        let I::Compare(pred @ (Compare::Eq | Compare::Ne), _, a, b) =
            self.body.instruction(id).kind
        else {
            return (mask, mask);
        };
        let constant = if a == Value::Temp(selector) {
            b
        } else if b == Value::Temp(selector) {
            a
        } else {
            return (mask, mask);
        };
        let Value::Integer(value) = constant else {
            return (mask, mask);
        };
        let bytes = self.body.literal(value).to_le_bytes();
        let k = i64::from_le_bytes(std::array::from_fn(|index| bytes[index]));
        let equal = mask & selector_bit(k);
        let unequal = if (0..=62).contains(&k) {
            mask & !selector_bit(k)
        } else {
            mask
        };
        if pred == Compare::Ne {
            (unequal, equal)
        } else {
            (equal, unequal)
        }
    }
    fn resume(
        &mut self,
        id: InstId,
        node: State,
        record: bool,
        work: &mut Vec<BlockId>,
    ) -> Result<(), Error> {
        let current = self.regions.get(node);
        let protect = current.protect;
        if protect.empty() {
            return Ok(());
        }
        let target = self.regions.get(protect);
        if protect == node {
            let mut k = node;
            while !k.empty() && !self.regions.get(k).active {
                let next = self.regions.get(k);
                if next.inside && self.landing_mask(next.landing.ok_or(Error::Limit)?) & !1 != 0 {
                    return Err(Error::Diagnostic(
                        "resume into a catching region opened while handling the exception".into(),
                    ));
                }
                k = next.parent;
            }
        }
        let landing = target.landing.ok_or(Error::Limit)?;
        if record {
            self.plan.sites[id.index()] = Site::Landing(landing);
        } else {
            let active = self
                .regions
                .record(target.parent, landing, target.cleanup, true)?;
            self.enter(landing, active, self.landing_mask(landing), work)?;
        }
        Ok(())
    }
    fn walk(
        &mut self,
        program: &Program,
        block: BlockId,
        record: bool,
        work: &mut Vec<BlockId>,
    ) -> Result<(), Error> {
        let mut node = self.incoming[block.index()].ok_or(Error::Limit)?;
        let mut mask = self.masks[block.index()];
        let mut selector = None;
        let mut leading = self.landing_nodes[block.index()].is_some();
        for &id in &self.body.block(block).instructions {
            let inst = self.body.instruction(id);
            if !clause(&inst.kind) {
                leading = false;
            } else if !leading && !matches!(inst.kind, I::Runtime(Runtime::EhCleanup(_))) {
                let I::Runtime(Runtime::EhCatchAll(s)) = inst.kind else {
                    return Err(Error::Diagnostic(
                        "a typed catch clause outside the leading clauses of a landing".into(),
                    ));
                };
                self.mid_clauses = true;
                if record {
                    self.plan.sites[id.index()] = Site::Clause;
                }
                if mask & 1 != 0 {
                    mask = (mask & !1) | selector_bit(s.unwrap_or(0));
                }
                selector = None;
            }
            let current = self.regions.get(node);
            let base_id = if current.tail.is_some() {
                current.parent
            } else {
                node
            };
            let base = self.regions.get(base_id);
            let base_try = !base_id.empty() && base.active && !base.cleanup && base.tail.is_none();
            match &inst.kind {
                I::Runtime(Runtime::EhTry(landing) | Runtime::EhCleanup(Some(landing))) => {
                    let cleanup = matches!(inst.kind, I::Runtime(Runtime::EhCleanup(_)));
                    node = self.regions.record(node, *landing, cleanup, false)?;
                    if record {
                        if self.landing_nodes[landing.index()] != Some(node) {
                            return Err(Error::Diagnostic(
                                "a landing installed under different enclosing regions".into(),
                            ));
                        }
                    } else {
                        self.landing_nodes[landing.index()] = Some(node);
                        let active = self.regions.record(
                            self.regions.get(node).parent,
                            *landing,
                            cleanup,
                            true,
                        )?;
                        self.enter(*landing, active, self.landing_mask(*landing), work)?;
                    }
                }
                I::Runtime(Runtime::EhCleanup(None)) => {
                    if let Some(tail) = current.tail {
                        if tail.lo == 0 || tail.top == Top::Mixed {
                            return Err(Error::Diagnostic(
                                "a cleanup clause where merged landing records differ".into(),
                            ));
                        }
                        if tail.top == Top::Try {
                            node = self.regions.tail(
                                current.parent,
                                u32::from(tail.lo),
                                u32::from(tail.hi),
                                Top::Cleanup,
                            )?;
                        }
                    } else if base_try {
                        node = self.regions.record(
                            current.parent,
                            current.landing.ok_or(Error::Limit)?,
                            true,
                            true,
                        )?;
                    }
                }
                I::Runtime(Runtime::EhEnd) => {
                    if let Some(tail) = current.tail {
                        if tail.lo == 0 {
                            return Err(Error::Diagnostic(
                                "eh_end where merged landing records differ".into(),
                            ));
                        }
                        node = self.regions.tail(
                            current.parent,
                            u32::from(tail.lo - 1),
                            u32::from(tail.hi - 1),
                            Top::Mixed,
                        )?;
                    } else {
                        if node.empty() {
                            return Err(Error::Diagnostic(
                                "eh_end without an open region of the function".into(),
                            ));
                        }
                        node = current.parent;
                    }
                }
                I::Runtime(Runtime::Exception(_)) => {
                    if let Some(tail) = current.tail {
                        if tail.lo == 0 && base_try || tail.top == Top::Mixed {
                            return Err(Error::Diagnostic(
                                "an exception read where merged landing records differ".into(),
                            ));
                        }
                        if tail.top == Top::Try {
                            node = self.regions.tail(
                                current.parent,
                                u32::from(tail.lo.saturating_sub(1)),
                                u32::from(tail.hi - 1),
                                Top::Mixed,
                            )?;
                        }
                    } else if base_try {
                        node = current.parent;
                    }
                }
                I::Runtime(Runtime::ExceptionSelector(_)) => selector = inst.result,
                I::Call(call) if record => {
                    if !program
                        .call_boundary(call)
                        .is_some_and(|sig| sig.boundary.unwind == Unwind::No)
                        && !current.protect.empty()
                    {
                        self.plan.sites[id.index()] = Site::Landing(
                            self.regions
                                .get(current.protect)
                                .landing
                                .ok_or(Error::Limit)?,
                        );
                    }
                }
                I::Jump(target) if self.resume_only(*target) => {
                    if record && current.protect.empty() {
                        self.plan.sites[id.index()] = Site::Terminal;
                    }
                    self.resume(id, node, record, work)?;
                    return Ok(());
                }
                I::Runtime(Runtime::Resume) => {
                    self.resume(id, node, record, work)?;
                    return Ok(());
                }
                I::Jump(target) if !record => {
                    self.enter(*target, node, self.edge_mask(*target, mask), work)?;
                }
                I::Branch(condition, yes, no) if !record => {
                    let (yes_mask, no_mask) = self.refine(*condition, selector, mask);
                    if yes_mask != 0 {
                        self.enter(*yes, node, self.edge_mask(*yes, yes_mask), work)?;
                    }
                    if no_mask != 0 {
                        self.enter(*no, node, self.edge_mask(*no, no_mask), work)?;
                    }
                }
                I::Switch(..) if !record => {
                    for &target in self.flow.successors(block) {
                        self.enter(target, node, self.edge_mask(target, mask), work)?;
                    }
                }
                I::Runtime(Runtime::Throw(..)) => return Err(Error::Diagnostic(
                    "a throw instruction (host objects throw through the eh_throw runtime role)"
                        .into(),
                )),
                _ => {}
            }
        }
        Ok(())
    }
    fn resume_only(&self, block: BlockId) -> bool {
        matches!(&self.body.block(block).instructions[..], [id] if matches!(self.body.instruction(*id).kind, I::Runtime(Runtime::Resume)))
    }
    fn prune(&mut self) -> Result<(), Error> {
        let mut work = vec![self.body.layout[0]];
        self.plan.live[self.body.layout[0].index()] = 1;
        let mut reads = false;
        while let Some(block) = work.pop() {
            let mut reach = |to: BlockId, how| {
                if self.plan.live[to.index()] == 0 {
                    work.push(to);
                }
                self.plan.live[to.index()] = self.plan.live[to.index()].max(how);
            };
            for &to in self.flow.successors(block) {
                reach(to, 1);
            }
            for &id in &self.body.block(block).instructions {
                match self.body.instruction(id).kind {
                    I::Runtime(Runtime::Exception(_) | Runtime::ExceptionSelector(_)) => {
                        reads = true;
                    }
                    I::Call(_) => {
                        if let Site::Landing(to) = self.plan.sites[id.index()] {
                            reach(to, 2);
                        }
                    }
                    I::Runtime(Runtime::Resume) | I::Jump(_) => {
                        reads |=
                            matches!(self.body.instruction(id).kind, I::Runtime(Runtime::Resume));
                        if let Site::Landing(to) = self.plan.sites[id.index()] {
                            self.plan.resumed[to.index()] = true;
                            reach(to, 1);
                        }
                    }
                    _ => {}
                }
            }
        }
        for &block in &self.body.layout {
            let b = block.index();
            if self.landing_nodes[b].is_some()
                && self.incoming[b].is_some()
                && (self.plan.live[b] == 2 || self.plan.resumed[b])
            {
                self.plan.indexes[b] = Some(LandingId::new(self.plan.tables.landings.len())?);
                self.plan.tables.landings.push(Landing::default());
            }
        }
        if self.plan.tables.landings.is_empty() && reads {
            return Err(Error::Diagnostic(
                "an exception read or resume outside every landing".into(),
            ));
        }
        Ok(())
    }
}

// Reverse postorder includes ordinary successors, then installed landing edges.
fn ranks(body: &Body, flow: &Flow) -> Vec<usize> {
    let mut edges = vec![Vec::new(); body.blocks().len()];
    for &block in &body.layout {
        edges[block.index()].extend_from_slice(flow.successors(block));
        for &id in &body.block(block).instructions {
            if let I::Runtime(runtime) = &body.instruction(id).kind
                && let Some(target) = runtime.handler_target()
            {
                edges[block.index()].push(target);
            }
        }
    }
    let mut ranks = vec![body.blocks().len(); body.blocks().len()];
    let mut seen = vec![false; ranks.len()];
    let mut stack = vec![(body.layout[0], 0)];
    seen[body.layout[0].index()] = true;
    let mut finished = 0;
    while let Some((block, next)) = stack.last_mut() {
        if let Some(&target) = edges[block.index()].get(*next) {
            *next += 1;
            if !seen[target.index()] {
                seen[target.index()] = true;
                stack.push((target, 0));
            }
        } else {
            ranks[block.index()] = body.blocks().len() - 1 - finished;
            finished += 1;
            let _ = stack.pop();
        }
    }
    ranks
}
pub(super) fn analyze(
    program: &Program,
    symbol: SymbolId,
    body: &Body,
    flow: &mut Flow,
) -> Result<Plan, Error> {
    let count = body.blocks().len();
    let entry_selectors = body
        .blocks()
        .iter()
        .enumerate()
        .map(|(b, block)| {
            if !flow.handlers[b] {
                return 0;
            }
            for &id in &block.instructions {
                match &body.instruction(id).kind {
                    I::Runtime(Runtime::EhCatchAll(s)) => return s.unwrap_or(0),
                    I::Runtime(Runtime::EhCatch(..)) => return 0,
                    kind if clause(kind) => {}
                    _ => return 0,
                }
            }
            0
        })
        .collect();
    let mut analysis = Analysis {
        body,
        flow,
        regions: Regions::default(),
        incoming: vec![None; count],
        masks: vec![0; count],
        landing_nodes: vec![None; count],
        mid_clauses: false,
        plan: Plan {
            sites: vec![Site::None; body.instructions().len()],
            live: vec![0; count],
            resumed: vec![false; count],
            indexes: vec![None; count],
            entry_selectors,
            tables: Tables::default(),
        },
    };
    let result = (|| {
        let entry = body.layout[0];
        analysis.incoming[entry.index()] = Some(State::default());
        analysis.masks[entry.index()] = u64::MAX;
        let mut changed = Vec::new();
        if count <= 32 {
            let mut work = vec![entry];
            while let Some(block) = work.pop() {
                changed.clear();
                analysis.walk(program, block, false, &mut changed)?;
                analysis.plan.tables.walks += 1;
                work.extend_from_slice(&changed);
            }
        } else {
            let rank = ranks(body, flow);
            let mut ordinal = vec![0; count];
            for (index, &block) in body.layout.iter().enumerate() {
                ordinal[block.index()] = index;
            }
            let mut heap =
                BinaryHeap::from([Reverse((rank[entry.index()], ordinal[entry.index()]))]);
            let mut listed = vec![false; count];
            listed[entry.index()] = true;
            while let Some(Reverse((_, index))) = heap.pop() {
                let block = body.layout[index];
                listed[block.index()] = false;
                changed.clear();
                analysis.walk(program, block, false, &mut changed)?;
                analysis.plan.tables.walks += 1;
                for &next in &changed {
                    if !listed[next.index()] {
                        listed[next.index()] = true;
                        heap.push(Reverse((rank[next.index()], ordinal[next.index()])));
                    }
                }
            }
        }
        analysis.plan.tables.states = crate::support::telemetry::counter(analysis.regions.len());
        for &block in &body.layout {
            if flow.reachable[block.index()] && analysis.incoming[block.index()].is_some() {
                analysis.walk(program, block, true, &mut changed)?;
            }
        }
        analysis.prune()?;
        super::catch_tables::build(&mut analysis)?;
        Ok(())
    })();
    if let Err(Error::Diagnostic(message)) = result {
        return Err(Error::Diagnostic(format!(
            "host EH: {message} (in @{})",
            String::from_utf8_lossy(program.names.bytes(program.symbol(symbol).name))
        )));
    }
    result?;
    let plan = analysis.plan;
    flow.restrict(body, &plan.live)?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Boundary, Call, Signature},
        vocabulary::Type,
    };

    #[test]
    fn only_live_predecessors_can_reject_a_jump_into_typed_clauses() -> Result<(), Error> {
        for live in [false, true] {
            let mut program = Program::default();
            let mut builder = FunctionBuilder::new(&mut program, "g", &[], Type::Void)?;
            builder.block("entry")?;
            builder.effect(I::Return(Type::Void, None))?;
            builder.finish()?;
            let g = program
                .find_symbol(program.functions()[0].name)
                .ok_or(Error::Limit)?;
            let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void)?;
            let dead = builder.block_id("dead")?;
            let landing = builder.block_id("landing")?;
            builder.block("entry")?;
            let arguments = builder.arguments(&[])?;
            let call = || Call {
                result: Type::Void,
                callee: Value::Symbol(g),
                arguments,
                copy_elision: false,
                argument_classes: Vec::new(),
                signature: None,
            };
            builder.effect(I::Runtime(Runtime::EhTry(dead)))?;
            if live {
                builder.effect(I::Call(call()))?;
            }
            builder.effect(I::Runtime(Runtime::EhEnd))?;
            builder.effect(I::Runtime(Runtime::EhTry(landing)))?;
            builder.effect(I::Call(call()))?;
            builder.effect(I::Runtime(Runtime::EhEnd))?;
            builder.effect(I::Return(Type::Void, None))?;
            for block in ["dead", "landing"] {
                builder.block(block)?;
                builder.effect(I::Runtime(Runtime::EhCatch(g, Some(1))))?;
                if block == "dead" {
                    builder.effect(I::Jump(landing))?;
                } else {
                    let _ =
                        builder.value(I::Runtime(Runtime::Exception(Type::Ptr)), "exception")?;
                    builder.effect(I::Return(Type::Void, None))?;
                }
            }
            builder.finish()?;
            let function = &program.functions()[1];
            let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
            let body = function.body.as_ref().ok_or(Error::Limit)?;
            let mut flow = Flow::analyze(body)?;
            let result = analyze(&program, symbol, body, &mut flow);
            if live {
                assert!(matches!(result, Err(Error::Diagnostic(message))
                    if message == "host EH: a jump into a landing with catch clauses (in @f)"));
            } else {
                let plan = result?;
                assert_eq!(plan.tables.landings.len(), 1);
                assert!(!flow.reachable[dead.index()]);
                assert!(flow.predecessors(landing).is_empty());
            }
        }
        Ok(())
    }

    #[test]
    fn filter_selector_maps_truncate_only_after_the_identity_comparison() -> Result<(), Error> {
        for (selector, mapped, raw) in [
            (-1, -1, false),
            (0, 0, true),
            (0xffff_ffff, -1, true),
            (i64::MIN, 0, true),
            (i64::MAX, -1, true),
        ] {
            let mut program = Program::default();
            let mut builder =
                FunctionBuilder::new(&mut program, "f", &[("g", Type::Ptr)], Type::Void)?;
            builder.block("entry")?;
            let landing = builder.block_id("landing")?;
            builder.effect(I::Runtime(Runtime::EhTry(landing)))?;
            let signature = builder.program().add_signature(Signature {
                parameters: Vec::new(),
                result: Type::Void,
                boundary: Boundary::default(),
            })?;
            let call = Call {
                result: Type::Void,
                callee: builder.parameter(0)?,
                arguments: builder.arguments(&[])?,
                copy_elision: false,
                argument_classes: Vec::new(),
                signature: Some(signature),
            };
            builder.effect(I::Call(call))?;
            builder.effect(I::Runtime(Runtime::EhEnd))?;
            builder.effect(I::Return(Type::Void, None))?;
            builder.block("landing")?;
            let types = builder.arguments(&[])?;
            builder.effect(I::Runtime(Runtime::EhFilter(types, Some(selector))))?;
            builder.effect(I::Runtime(Runtime::Resume))?;
            builder.finish()?;
            let function = &program.functions()[0];
            let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
            let body = function.body.as_ref().ok_or(Error::Limit)?;
            let mut flow = Flow::analyze(body)?;
            let plan = analyze(&program, symbol, body, &mut flow)?;
            assert_eq!(plan.tables.map, [(-1, mapped)]);
            assert_eq!(plan.tables.raw, raw);
            assert_eq!(plan.tables.specs, [0]);
        }
        Ok(())
    }
}
