//! Clean dominator cuts for partial copies from `opt/inliner.cpp`. See NOTICE.
use super::{control_flow::Cfg, pure};
use crate::lowir::{
    Error,
    model::{Body, InstructionKind as I},
};
#[derive(Default)]
pub(super) struct Cut {
    pub(super) copy: Vec<bool>,
    pub(super) boundary: Vec<bool>,
    pub(super) size: u32,
    pub(super) cuts: u32,
}
impl Cut {
    fn region(&mut self, body: &Body, cfg: &Cfg) -> Result<bool, Error> {
        self.copy = vec![false; body.blocks().len()];
        self.size = 0;
        self.cuts = 0;
        let entry = *body.layout.first().ok_or(Error::Limit)?;
        let mut work = vec![entry];
        self.copy[entry.index()] = true;
        let mut reached = vec![false; body.blocks().len()];
        let mut returns = false;
        while let Some(b) = work.pop() {
            self.size = self
                .size
                .checked_add(
                    u32::try_from(body.block(b).instructions.len()).map_err(|_| Error::Limit)?,
                )
                .ok_or(Error::Limit)?;
            returns |= body
                .block(b)
                .instructions
                .iter()
                .any(|&id| matches!(body.instruction(id).kind, I::Return(..)));
            for &s in cfg.successors(b) {
                if self.copy[s.index()] || reached[s.index()] {
                    continue;
                }
                if self.boundary[s.index()] {
                    reached[s.index()] = true;
                    self.cuts = self.cuts.checked_add(1).ok_or(Error::Limit)?;
                    self.size = self.size.checked_add(2).ok_or(Error::Limit)?;
                } else {
                    self.copy[s.index()] = true;
                    work.push(s);
                }
            }
        }
        self.boundary = reached;
        Ok(returns)
    }
    pub(super) fn find(body: &Body, cap: u32) -> Result<Option<Self>, Error> {
        let cfg = Cfg::analyze(body)?;
        let n = body.blocks().len();
        let mut side = vec![false; n];
        let mut calls = vec![false; n];
        let mut fast = vec![false; n];
        for &b in &body.layout {
            let mut returns = false;
            for &id in &body.block(b).instructions {
                let kind = &body.instruction(id).kind;
                calls[b.index()] |= matches!(kind, I::Call(_));
                returns |= matches!(kind, I::Return(..));
                side[b.index()] |= !pure(kind)
                    && !kind.terminator()
                    && !matches!(kind,I::Load(f,_,_)if !f.volatile());
            }
            fast[b.index()] = returns && !calls[b.index()];
        }
        let mut clean = vec![true; n];
        loop {
            let mut changed = false;
            for &b in cfg.reverse_postorder() {
                let all = cfg
                    .predecessors(b)
                    .iter()
                    .all(|&p| !cfg.reachable(p) || clean[p.index()] && !side[p.index()]);
                if !all && clean[b.index()] {
                    clean[b.index()] = false;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        loop {
            let mut changed = false;
            for &b in cfg.reverse_postorder().iter().rev() {
                if fast[b.index()] || calls[b.index()] {
                    continue;
                }
                fast[b.index()] = cfg.successors(b).iter().any(|s| fast[s.index()]);
                changed |= fast[b.index()];
            }
            if !changed {
                break;
            }
        }
        let entry = *body.layout.first().ok_or(Error::Limit)?;
        let mut out = Self {
            boundary: vec![false; n],
            ..Self::default()
        };
        for &b in cfg.reverse_postorder() {
            if !calls[b.index()] {
                continue;
            }
            let mut at = None;
            let mut d = Some(b);
            while let Some(block) = d.filter(|&b| b != entry) {
                if clean[block.index()] && !fast[block.index()] && !cfg.handler(block) {
                    at = Some(block);
                }
                d = cfg.dominator(block).filter(|&p| p != block);
            }
            if let Some(at) = at {
                out.boundary[at.index()] = true;
            }
        }
        let returns = out.region(body, &cfg)?;
        Ok((out.cuts != 0 && returns && out.size <= cap).then_some(out))
    }
}
