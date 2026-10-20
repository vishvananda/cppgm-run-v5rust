//! Function-owned layout positions, reachable edges and predecessor CSR. See NOTICE.
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstId, InstructionKind as I},
};
use std::ops::Range;

pub(super) struct Flow {
    pub(super) instructions: Vec<InstId>,
    pub(super) blocks: Vec<BlockId>,
    pub(super) ranges: Vec<Range<usize>>,
    pub(super) reachable: Vec<bool>,
    pub(super) linear: Vec<usize>,
    pub(super) positions: Vec<usize>,
    successors: Vec<BlockId>,
    succ_first: Vec<usize>,
    predecessors: Vec<BlockId>,
    pred_first: Vec<usize>,
}
impl Flow {
    pub(super) fn analyze(body: &Body) -> Result<Self, Error> {
        let count = body.blocks().len();
        let mut out = Self {
            instructions: Vec::with_capacity(body.instructions().len()),
            blocks: Vec::with_capacity(body.instructions().len()),
            ranges: vec![0..0; count],
            reachable: vec![false; count],
            linear: vec![0; count],
            positions: vec![0; body.instructions().len()],
            successors: Vec::new(),
            succ_first: vec![0; count + 1],
            predecessors: Vec::new(),
            pred_first: vec![0; count + 1],
        };
        // CSR is keyed by the LowIR block identity, independently of layout order.
        let mut added = vec![None; count];
        for (index, block) in body.blocks().iter().enumerate() {
            out.succ_first[index] = out.successors.len();
            if let Some(&last) = block.instructions.last() {
                let mut add = |id: BlockId| {
                    if added[id.index()] != Some(index) {
                        added[id.index()] = Some(index);
                        out.successors.push(id);
                    }
                };
                match &body.instruction(last).kind {
                    I::Jump(to) => add(*to),
                    I::Branch(_, yes, no) => {
                        add(*yes);
                        add(*no);
                    }
                    I::Switch(_, fallback, cases) => {
                        add(*fallback);
                        for &(_, to) in cases {
                            add(to);
                        }
                    }
                    _ => {}
                }
            }
        }
        out.succ_first[count] = out.successors.len();
        let Some(&entry) = body.layout.first() else {
            return Err(Error::Limit);
        };
        let mut work = vec![entry];
        out.reachable[entry.index()] = true;
        while let Some(block) = work.pop() {
            for index in out.succ_first[block.index()]..out.succ_first[block.index() + 1] {
                let next = out.successors[index];
                if !out.reachable[next.index()] {
                    out.reachable[next.index()] = true;
                    work.push(next);
                }
            }
        }
        for &block in &body.layout {
            if out.reachable[block.index()] {
                for index in out.succ_first[block.index()]..out.succ_first[block.index() + 1] {
                    let next = out.successors[index];
                    out.pred_first[next.index() + 1] += 1;
                }
            }
        }
        for index in 0..count {
            out.pred_first[index + 1] += out.pred_first[index];
        }
        out.predecessors.resize(out.pred_first[count], entry);
        let mut fill = out.pred_first[..count].to_vec();
        for &block in &body.layout {
            if !out.reachable[block.index()] {
                continue;
            }
            for index in out.succ_first[block.index()]..out.succ_first[block.index() + 1] {
                let next = out.successors[index];
                out.predecessors[fill[next.index()]] = block;
                fill[next.index()] += 1;
            }
        }
        let mut linear = 0_usize;
        for &block in &body.layout {
            let start = out.instructions.len();
            for &inst in &body.block(block).instructions {
                out.positions[inst.index()] = out.instructions.len();
                out.instructions.push(inst);
                out.blocks.push(block);
            }
            out.ranges[block.index()] = start..out.instructions.len();
            out.linear[block.index()] = linear;
            if out.reachable[block.index()] {
                linear = linear
                    .checked_add(out.ranges[block.index()].len() + 2)
                    .ok_or(Error::Limit)?;
            }
        }
        Ok(out)
    }
    pub(super) fn successors(&self, block: BlockId) -> &[BlockId] {
        &self.successors[self.succ_first[block.index()]..self.succ_first[block.index() + 1]]
    }
    pub(super) fn predecessors(&self, block: BlockId) -> &[BlockId] {
        &self.predecessors[self.pred_first[block.index()]..self.pred_first[block.index() + 1]]
    }
    pub(super) fn linear_position(&self, pos: usize) -> usize {
        let block = self.blocks[pos];
        self.linear[block.index()] + 1 + pos - self.ranges[block.index()].start
    }
    pub(super) fn end(&self, block: BlockId) -> usize {
        self.linear[block.index()] + self.ranges[block.index()].len() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{builder::FunctionBuilder, model::Program, vocabulary::Type};
    #[test]
    fn unreachable_predecessors_are_excluded_when_ids_differ_from_layout() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::I64)?;
        builder.block("entry")?;
        let entry = builder.block_id("entry")?;
        let join = builder.block_id("join")?;
        builder.effect(I::Jump(join))?;
        builder.block("dead")?;
        let dead = builder.block_id("dead")?;
        builder.effect(I::Jump(join))?;
        builder.block("join")?;
        let one = builder.integer(1)?;
        builder.effect(I::Return(Type::I64, Some(one)))?;
        builder.finish()?;
        let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
        let flow = Flow::analyze(body)?;
        assert!(flow.reachable[join.index()]);
        assert!(!flow.reachable[dead.index()]);
        assert_eq!(flow.predecessors(join), [entry]);
        assert_eq!(flow.successors(entry), [join]);
        assert_ne!(body.layout[1], join);
        assert_eq!(flow.linear[dead.index()], flow.linear[join.index()]);
        Ok(())
    }
}
