//! Natural-loop membership and nesting from `opt/loops.cpp`. See NOTICE.
use super::control_flow::Cfg;
use crate::lowir::{Error, model::BlockId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LoopId(u32);
impl LoopId {
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}
pub(super) struct NaturalLoop {
    id: LoopId,
    pub(super) header: BlockId,
    pub(super) blocks: Vec<BlockId>,
    pub(super) latches: Vec<BlockId>,
    pub(super) preheader: Option<BlockId>,
    pub(super) parent: Option<LoopId>,
}
#[derive(Default)]
pub(super) struct Forest {
    loops: Vec<NaturalLoop>,
    innermost: Vec<Option<LoopId>>,
    enter: Vec<usize>,
    leave: Vec<usize>,
    mark: Vec<u32>,
    epoch: u32,
    work: Vec<BlockId>,
}
impl Forest {
    pub(super) fn get(&self, id: LoopId) -> &NaturalLoop {
        &self.loops[id.index()]
    }
    pub(super) fn ids(&self) -> impl DoubleEndedIterator<Item = LoopId> + '_ {
        self.loops.iter().map(|loop_| loop_.id)
    }
    pub(super) fn len(&self) -> usize {
        self.loops.len()
    }
    pub(super) fn innermost(&self, block: BlockId) -> Option<LoopId> {
        self.innermost.get(block.index()).copied().flatten()
    }
    pub(super) fn contains(&self, id: LoopId, block: BlockId) -> bool {
        self.innermost(block).is_some_and(|inner| {
            self.enter[id.index()] <= self.enter[inner.index()]
                && self.enter[inner.index()] < self.leave[id.index()]
        })
    }
    pub(super) fn find(&mut self, count: usize, cfg: &Cfg) -> Result<(), Error> {
        self.loops.clear();
        self.innermost = vec![None; count];
        if self.mark.len() < count {
            self.mark = vec![0; count];
        }
        let mut memberships = 0_u64;
        let budget = u64::try_from(cfg.reverse_postorder().len())
            .map_err(|_| Error::Limit)?
            .checked_mul(8)
            .and_then(|n| n.checked_add(4096))
            .ok_or(Error::Limit)?;
        for &header in cfg.reverse_postorder() {
            let latches = cfg
                .predecessors(header)
                .iter()
                .copied()
                .filter(|&b| cfg.reachable(b) && cfg.dominates(header, b))
                .collect::<Vec<_>>();
            if latches.is_empty() {
                continue;
            }
            self.epoch = self.epoch.checked_add(1).ok_or(Error::Limit)?;
            let mut loop_ = NaturalLoop {
                id: LoopId(u32::try_from(self.loops.len()).map_err(|_| Error::Limit)?),
                header,
                blocks: vec![header],
                latches,
                preheader: None,
                parent: self.innermost[header.index()],
            };
            self.mark[header.index()] = self.epoch;
            self.work.clear();
            for &latch in &loop_.latches {
                if self.mark[latch.index()] != self.epoch {
                    self.mark[latch.index()] = self.epoch;
                    loop_.blocks.push(latch);
                    self.work.push(latch);
                }
            }
            if !self.expand(cfg, &mut loop_) || loop_.blocks.iter().any(|&b| cfg.handler(b)) {
                continue;
            }
            let mut outside = None;
            let mut outside_count = 0_usize;
            for &pred in cfg.predecessors(header) {
                if self.mark[pred.index()] != self.epoch {
                    outside = Some(pred);
                    outside_count += 1;
                }
            }
            if outside_count == 1 && outside.is_some_and(|b| cfg.successors(b).len() == 1) {
                loop_.preheader = outside;
            }
            memberships = memberships
                .checked_add(u64::try_from(loop_.blocks.len()).map_err(|_| Error::Limit)?)
                .ok_or(Error::Limit)?;
            if memberships > budget {
                break;
            }
            let id = LoopId(u32::try_from(self.loops.len()).map_err(|_| Error::Limit)?);
            for &block in &loop_.blocks {
                self.innermost[block.index()] = Some(id);
            }
            self.loops.push(loop_);
        }
        self.intervals()
    }
    fn expand(&mut self, cfg: &Cfg, loop_: &mut NaturalLoop) -> bool {
        while let Some(block) = self.work.pop() {
            for &pred in cfg.predecessors(block) {
                if !cfg.reachable(pred) || self.mark[pred.index()] == self.epoch {
                    continue;
                }
                if !cfg.dominates(loop_.header, pred) {
                    return false;
                }
                self.mark[pred.index()] = self.epoch;
                loop_.blocks.push(pred);
                self.work.push(pred);
            }
        }
        true
    }
    fn intervals(&mut self) -> Result<(), Error> {
        let n = self.loops.len();
        let mut sizes = vec![1_usize; n];
        for k in (0..n).rev() {
            if let Some(parent) = self.loops[k].parent {
                sizes[parent.index()] = sizes[parent.index()]
                    .checked_add(sizes[k])
                    .ok_or(Error::Limit)?;
            }
        }
        self.enter = vec![0; n];
        self.leave = vec![0; n];
        let mut cursor = vec![0_usize; n];
        let mut roots = 0_usize;
        for (k, loop_) in self.loops.iter().enumerate() {
            self.enter[k] = if let Some(parent) = loop_.parent {
                let at = cursor[parent.index()];
                cursor[parent.index()] = at.checked_add(sizes[k]).ok_or(Error::Limit)?;
                at
            } else {
                let at = roots;
                roots = at.checked_add(sizes[k]).ok_or(Error::Limit)?;
                at
            };
            cursor[k] = self.enter[k].checked_add(1).ok_or(Error::Limit)?;
            self.leave[k] = self.enter[k].checked_add(sizes[k]).ok_or(Error::Limit)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn discovery_retains_latches_preheaders_and_nested_membership() {
        let program=crate::lowir::test_program(b"function @f(%c : i64)->void {block ^entry:jump ^outer block ^outer:branch %c, ^inner, ^exit block ^inner:branch %c, ^work, ^tail block ^work:jump ^inner block ^tail:jump ^outer block ^exit:return void}").unwrap();
        let body = program
            .function(program.symbol(program.publications()[0]))
            .unwrap()
            .body
            .as_ref()
            .unwrap();
        let cfg = super::Cfg::analyze(body).unwrap();
        let mut forest = super::Forest::default();
        forest.find(body.blocks().len(), &cfg).unwrap();
        let ids = forest.ids().collect::<Vec<_>>();
        assert_eq!(ids.len(), 2);
        let outer = forest.get(ids[0]);
        let inner = forest.get(ids[1]);
        assert_eq!(outer.preheader, Some(body.layout[0]));
        assert_eq!(inner.preheader, None);
        assert_eq!(inner.parent, Some(ids[0]));
        assert!(forest.contains(ids[0], inner.header));
        assert!(!forest.contains(ids[1], outer.header));
        assert_eq!(inner.latches, vec![body.layout[3]]);
    }
    #[test]
    fn handler_bodies_do_not_become_natural_loop_candidates() {
        let program=crate::lowir::test_program(b"function @f(%c : i64)->void {block ^entry:eh_try ^handler jump ^handler block ^handler:branch %c, ^handler, ^exit block ^exit:eh_end return void}").unwrap();
        let body = program
            .function(program.symbol(program.publications()[0]))
            .unwrap()
            .body
            .as_ref()
            .unwrap();
        let cfg = super::Cfg::analyze(body).unwrap();
        let mut forest = super::Forest::default();
        forest.find(body.blocks().len(), &cfg).unwrap();
        assert_eq!(forest.len(), 0);
    }
}
