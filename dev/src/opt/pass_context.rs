//! Function-owned pass state and settled versions from `opt/passes.h`. See NOTICE.
use super::{
    condition_placement, control_flow::Cfg, dead_code, definition_order, exception_regions,
    flow_cleanup, jump_threading, memory_forward, slot_promotion, sparse_constants, value_reuse,
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, Program, SymbolId},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pass {
    SplitSlots,
    PromoteSlots,
    SimplifyValues,
    ForwardMemory,
    DeadCode,
    Constants,
    CleanupCfg,
    ThreadJumps,
    StripRegions,
    CoalesceCopies,
    RemoveLoops,
    UnrollLoops,
    FormSwitches,
    NameReturnedSlot,
    SplitOrBranches,
    SpecializePhiCompares,
    SplitBranchCompares,
    DefinitionOrder,
}
impl Pass {
    fn missing(self) -> &'static str {
        match self {
            Self::SplitSlots => "LowIR aggregate slot splitting",
            Self::SimplifyValues => "LowIR scalar simplification",
            Self::ForwardMemory => "LowIR memory forwarding",
            Self::Constants => "LowIR constant propagation",
            Self::CleanupCfg => "LowIR CFG cleanup",
            Self::CoalesceCopies => "LowIR copy coalescing",
            Self::RemoveLoops => "LowIR loop removal",
            Self::UnrollLoops => "LowIR loop unrolling",
            Self::FormSwitches => "LowIR switch formation",
            Self::NameReturnedSlot => "LowIR returned-slot naming",
            Self::DefinitionOrder => "LowIR definition order restoration",
            Self::DeadCode
            | Self::StripRegions
            | Self::PromoteSlots
            | Self::ThreadJumps
            | Self::SplitOrBranches
            | Self::SpecializePhiCompares
            | Self::SplitBranchCompares => "implemented function pass",
        }
    }
}
enum CachedCfg {
    Empty,
    Light(Cfg),
    Complete(Cfg),
}
pub(super) struct Context<'a> {
    pub(super) program: &'a Program,
    pub(super) symbol: SymbolId,
    pub(super) body: &'a mut Body,
    pub(super) level: u8,
    pub(super) nounwind: &'a [bool],
    pub(super) dead: u64,
    pub(super) propagated: u64,
    pub(super) unwritten: &'a [bool],
    pub(super) branches_folded: u64,
    pub(super) blocks_removed: u64,
    pub(super) blocks_merged: u64,
    pub(super) folded: u64,
    pub(super) reused: u64,
    pub(super) loads_reused: u64,
    pub(super) stores_removed: u64,
    pub(super) slots_promoted: u64,
    pub(super) phis_inserted: u64,
    pub(super) passed: Vec<u32>,
    pub(super) passed_stamp: u32,
    cfg: CachedCfg,
    version: u32,
    settled: Vec<(Pass, u32, bool)>,
    kept_body: Option<Pass>,
    pub(super) ordered_version: Option<u32>,
    pub(super) left_ordered: bool,
    running: Option<Pass>,
    block_of: Vec<Option<BlockId>>,
    block_of_key: Option<(u32, usize)>,
    uses: Vec<u32>,
    uses_key: Option<(u32, usize)>,
}
impl<'a> Context<'a> {
    pub(super) fn new(
        program: &'a Program,
        symbol: SymbolId,
        body: &'a mut Body,
        level: u8,
        nounwind: &'a [bool],
        unwritten: &'a [bool],
    ) -> Self {
        Self {
            program,
            symbol,
            body,
            level,
            nounwind,
            dead: 0,
            propagated: 0,
            unwritten,
            branches_folded: 0,
            blocks_removed: 0,
            blocks_merged: 0,
            folded: 0,
            reused: 0,
            loads_reused: 0,
            stores_removed: 0,
            slots_promoted: 0,
            phis_inserted: 0,
            passed: Vec::new(),
            passed_stamp: 0,
            cfg: CachedCfg::Empty,
            version: 0,
            settled: Vec::new(),
            kept_body: None,
            ordered_version: None,
            left_ordered: false,
            running: None,
            block_of: Vec::new(),
            block_of_key: None,
            uses: Vec::new(),
            uses_key: None,
        }
    }
    pub(super) fn ensure_cfg(&mut self) -> Result<(), Error> {
        let prior = std::mem::replace(&mut self.cfg, CachedCfg::Empty);
        self.cfg = match prior {
            CachedCfg::Empty => CachedCfg::Complete(Cfg::analyze(self.body)?),
            CachedCfg::Light(mut cfg) => {
                cfg.complete()?;
                CachedCfg::Complete(cfg)
            }
            CachedCfg::Complete(cfg) => CachedCfg::Complete(cfg),
        };
        Ok(())
    }
    pub(super) fn ensure_cfg_light(&mut self) -> Result<(), Error> {
        if matches!(self.cfg, CachedCfg::Empty) {
            self.cfg = CachedCfg::Light(Cfg::light(self.body)?);
        }
        Ok(())
    }
    pub(super) fn cfg(&self) -> Result<&Cfg, Error> {
        match &self.cfg {
            CachedCfg::Light(cfg) | CachedCfg::Complete(cfg) => Ok(cfg),
            CachedCfg::Empty => Err(Error::Limit),
        }
    }
    pub(super) fn cfg_mut(&mut self) -> Result<&mut Cfg, Error> {
        match &mut self.cfg {
            CachedCfg::Light(cfg) | CachedCfg::Complete(cfg) => Ok(cfg),
            CachedCfg::Empty => Err(Error::Limit),
        }
    }
    pub(super) fn invalidate_block_map(&mut self) {
        self.block_of_key = None;
    }
    pub(super) fn next_pass_stamp(&mut self) -> u32 {
        if self.passed.len() < self.body.instructions().len() {
            self.passed.resize(self.body.instructions().len(), 0);
        }
        self.passed_stamp = self.passed_stamp.wrapping_add(1);
        if self.passed_stamp == 0 {
            self.passed.fill(0);
            self.passed_stamp = 1;
        }
        self.passed_stamp
    }
    pub(super) fn invalidate_cfg(&mut self) {
        self.cfg = CachedCfg::Empty;
    }
    pub(super) fn running_pass(&self) -> Option<Pass> {
        self.running
    }
    pub(super) fn body_is_ordered(&self) -> bool {
        self.ordered_version == Some(self.version)
    }
    pub(super) fn keep_body(&mut self, pass: Pass, ordered: bool) {
        self.kept_body = Some(pass);
        if ordered {
            self.ordered_version = Some(self.version);
        }
    }
    pub(super) fn prepare_block_map(&mut self, pass: Pass, fresh: bool) {
        let share = fresh && self.running == Some(pass);
        let key = (self.version, self.body.instructions().len());
        if !share || self.block_of_key != Some(key) {
            self.block_of = vec![None; key.1];
            for &block in &self.body.layout {
                for &id in &self.body.block(block).instructions {
                    self.block_of[id.index()] = Some(block);
                }
            }
            self.block_of_key = share.then_some(key);
        }
    }
    pub(super) fn record_instruction_block(
        &mut self,
        id: crate::lowir::model::InstId,
        block: BlockId,
    ) {
        if self.block_of.len() <= id.index() {
            self.block_of.resize(id.index() + 1, None);
        }
        self.block_of[id.index()] = Some(block);
        self.invalidate_block_map();
    }
    pub(super) fn block_map(&self) -> &[Option<BlockId>] {
        &self.block_of
    }
    pub(super) fn prepare_uses(&mut self, pass: Pass, fresh: bool) -> Result<(), Error> {
        let share = fresh && self.running == Some(pass);
        let key = (self.version, self.body.temps().len());
        if !share || self.uses_key != Some(key) {
            self.body.recount_uses()?;
            self.uses = self.body.temps().iter().map(|t| t.uses).collect();
            self.uses_key = share.then_some(key);
        }
        Ok(())
    }
    pub(super) fn entry_uses(&self) -> &[u32] {
        &self.uses
    }
    pub(super) fn invalidate_uses(&mut self) {
        self.uses_key = None;
    }
    fn dispatch(&mut self, pass: Pass) -> Result<bool, Error> {
        match pass {
            Pass::DeadCode => dead_code::remove(self),
            Pass::StripRegions => exception_regions::strip(self),
            Pass::Constants => sparse_constants::propagate(self),
            Pass::SimplifyValues => value_reuse::simplify(self),
            Pass::CleanupCfg => flow_cleanup::cleanup(self),
            Pass::ForwardMemory => memory_forward::forward(self),
            Pass::PromoteSlots => slot_promotion::promote(self),
            Pass::ThreadJumps => jump_threading::thread(self),
            Pass::SplitOrBranches => condition_placement::split_or_branches(self),
            Pass::SpecializePhiCompares => condition_placement::specialize_phi_compares(self),
            Pass::SplitBranchCompares => condition_placement::split_branch_compares(self),
            Pass::DefinitionOrder => definition_order::restore(self),
            _ => Err(Error::NotImplemented(pass.missing())),
        }
    }
    pub(super) fn run(&mut self, pass: Pass) -> Result<bool, Error> {
        if let Some(&(_, _, reported)) = self
            .settled
            .iter()
            .find(|&&(p, v, _)| p == pass && v == self.version)
        {
            return Ok(reported);
        }
        self.kept_body = None;
        self.left_ordered = false;
        let ordered = self.ordered_version == Some(self.version);
        self.running = Some(pass);
        let result = self.dispatch(pass);
        self.running = None;
        let changed = result?;
        if changed && self.kept_body != Some(pass) {
            self.version = self.version.checked_add(1).ok_or(Error::Limit)?;
            if self.left_ordered || (ordered && matches!(pass, Pass::DeadCode | Pass::Constants)) {
                self.ordered_version = Some(self.version);
            }
            return Ok(true);
        }
        if let Some(entry) = self.settled.iter_mut().find(|entry| entry.0 == pass) {
            *entry = (pass, self.version, changed);
        } else {
            self.settled.push((pass, self.version, changed));
        }
        Ok(changed)
    }
    pub(super) fn restore_order(&mut self) -> Result<(), Error> {
        if self.ordered_version == Some(self.version) {
            return Ok(());
        }
        if self.dispatch(Pass::DefinitionOrder)? {
            self.version = self.version.checked_add(1).ok_or(Error::Limit)?;
        } else {
            self.ordered_version = Some(self.version);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        model::{Instruction, InstructionKind as I},
        vocabulary::Type,
    };
    #[test]
    fn settled_versions_preserve_order_and_cfg_for_dead_values() {
        let mut program = crate::lowir::test_program(
            b"function @f() -> void {block ^entry: %dead = const i64 7 return void}\n",
        )
        .unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&program, symbol, &mut body, 1, &[], &[]);
        context.ensure_cfg().unwrap();
        context.ordered_version = Some(0);
        assert!(context.run(Pass::DeadCode).unwrap());
        assert_eq!(context.version, 1);
        assert_eq!(context.dead, 1);
        assert_eq!(context.ordered_version, Some(1));
        assert!(matches!(context.cfg, CachedCfg::Complete(_)));
        assert!(!context.run(Pass::DeadCode).unwrap());
        assert_eq!(context.settled.len(), 1);
        assert!(!context.run(Pass::DeadCode).unwrap());
        assert_eq!(context.dead, 1);
        assert_eq!(context.version, 1);
    }
    #[test]
    fn light_cfg_upgrades_and_graph_edits_explicitly_invalidate_both_states() {
        let mut program = crate::lowir::test_program(
            b"function @f() -> void {block ^entry: jump ^exit block ^exit: return void}\n",
        )
        .unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&program, symbol, &mut body, 1, &[], &[]);
        context.ensure_cfg_light().unwrap();
        assert!(matches!(context.cfg, CachedCfg::Light(_)));
        context.ensure_cfg().unwrap();
        assert!(matches!(context.cfg, CachedCfg::Complete(_)));
        let entry = context.body.layout[0];
        assert_eq!(context.cfg().unwrap().successors(entry).len(), 1);
        let mut keep = vec![true; context.body.instructions().len()];
        keep[0] = false;
        context.body.retain_instructions(&keep).unwrap();
        context
            .body
            .append(
                entry,
                Instruction::new(None, I::Return(Type::Void, None), 0),
            )
            .unwrap();
        context.invalidate_cfg();
        assert!(matches!(context.cfg, CachedCfg::Empty));
        context.ensure_cfg_light().unwrap();
        assert!(context.cfg().unwrap().successors(entry).is_empty());
        context.ensure_cfg().unwrap();
        assert!(context.cfg().unwrap().successors(entry).is_empty());
    }
    #[test]
    fn an_unreported_edge_substitution_invalidates_the_entry_use_snapshot() {
        let mut program = crate::lowir::test_program(b"function @f(%v : i64) -> i64 {block ^entry: %c = cmp eq i64 %v, 7 branch %c, ^yes, ^no block ^yes: return i64 %v block ^no: return i64 %v}").unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&program, symbol, &mut body, 1, &[], &[]);
        assert!(!context.run(Pass::SimplifyValues).unwrap());
        assert_eq!(context.version, 0);
        assert_eq!(context.uses_key, None);
        assert_eq!(context.entry_uses()[0], 3);
        context.prepare_uses(Pass::ForwardMemory, true).unwrap();
        assert_eq!(context.entry_uses()[0], 2);
        assert_eq!(context.uses_key, None);
    }

    #[test]
    fn an_entry_use_cache_requires_the_pass_version_and_storage_size() {
        let mut program = crate::lowir::test_program(
            b"function @f(%v : i64) -> i64 {block ^entry: return i64 %v}",
        )
        .unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&program, symbol, &mut body, 1, &[], &[]);
        context.running = Some(Pass::SimplifyValues);
        context.prepare_uses(Pass::SimplifyValues, true).unwrap();
        assert_eq!(context.uses_key, Some((0, 1)));
        context.body.generated_temp(Type::I64).unwrap();
        context.prepare_uses(Pass::SimplifyValues, true).unwrap();
        assert_eq!(context.uses_key, Some((0, 2)));
        assert_eq!(context.entry_uses(), [1, 0]);
        context.version = 1;
        context.prepare_uses(Pass::SimplifyValues, true).unwrap();
        assert_eq!(context.uses_key, Some((1, 2)));
        context.prepare_uses(Pass::ForwardMemory, true).unwrap();
        assert_eq!(context.uses_key, None);
    }
}

#[cfg(test)]
mod promotion_tests {
    use super::*;
    #[test]
    fn unused_slot_reports_settle_without_changing_the_body_version() {
        let mut program = crate::lowir::test_program(
            b"function @f()->void {slot $unused : i64 block ^entry:return void}",
        )
        .unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let mut context = Context::new(&program, symbol, &mut body, 1, &[], &[]);
        context.ordered_version = Some(0);
        assert!(context.run(Pass::PromoteSlots).unwrap());
        assert_eq!(context.version, 0);
        assert_eq!(context.slots_promoted, 1);
        assert!(context.run(Pass::PromoteSlots).unwrap());
        assert_eq!(context.slots_promoted, 1);
        assert_eq!(context.ordered_version, Some(0));
    }
}
