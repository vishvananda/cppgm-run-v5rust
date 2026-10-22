//! Optimizer CFG, dominance and intervals from `opt/function_ir.cpp`. See NOTICE.
use crate::lowir::{
    Error,
    model::{BlockId, Body, InstructionKind as I, RuntimeInstruction as R},
};

const PLACED: u8 = 1;
const REACHABLE: u8 = 2;
const HANDLER: u8 = 4;
const EH: u8 = 8;
const ITERATIVE_DOMINATOR_BLOCKS: usize = 32;

#[derive(Default)]
struct Edges {
    first: Vec<usize>,
    targets: Vec<BlockId>,
}
impl Edges {
    fn from_count(count: usize) -> Self {
        Self {
            first: vec![0; count + 1],
            targets: Vec::new(),
        }
    }
    fn at(&self, block: BlockId) -> &[BlockId] {
        &self.targets[self.first[block.index()]..self.first[block.index() + 1]]
    }
    fn reverse(
        &self,
        count: usize,
        sources: impl Iterator<Item = BlockId> + Clone,
    ) -> Result<Self, Error> {
        let mut out = Self::from_count(count);
        for block in sources.clone() {
            for &target in self.at(block) {
                out.first[target.index() + 1] += 1;
            }
        }
        for index in 0..count {
            out.first[index + 1] += out.first[index];
        }
        if out.first[count] == 0 {
            return Ok(out);
        }
        let placeholder = sources.clone().next().ok_or(Error::Limit)?;
        out.targets.resize(out.first[count], placeholder);
        let mut fill = out.first[..count].to_vec();
        for block in sources {
            for &target in self.at(block) {
                out.targets[fill[target.index()]] = block;
                fill[target.index()] += 1;
            }
        }
        Ok(out)
    }
}
pub(super) struct Cfg {
    successors: Edges,
    predecessors: Edges,
    handlers: Edges,
    flags: Vec<u8>,
    rpo: Vec<BlockId>,
    rpo_index: Vec<Option<usize>>,
    idom: Vec<Option<BlockId>>,
    dom_in: Vec<usize>,
    dom_out: Vec<usize>,
    children: Edges,
}
impl Cfg {
    pub(super) fn successor_edges(&self) -> (&[usize], &[BlockId]) {
        (&self.successors.first, &self.successors.targets)
    }
    pub(super) fn predecessor_edges(&self) -> (&[usize], &[BlockId]) {
        (&self.predecessors.first, &self.predecessors.targets)
    }
    pub(super) fn reverse_postorder(&self) -> &[BlockId] {
        &self.rpo
    }
    pub(super) fn predecessors(&self, block: BlockId) -> &[BlockId] {
        self.predecessors.at(block)
    }
    pub(super) fn reachable(&self, block: BlockId) -> bool {
        self.flags[block.index()] & REACHABLE != 0
    }
    pub(super) fn handler(&self, block: BlockId) -> bool {
        self.flags[block.index()] & HANDLER != 0
    }
    /// Removing unreachable blocks retains all existing dominance facts.
    pub(super) fn drop_blocks(&mut self, dead: &[bool], layout: &[BlockId]) -> Result<(), Error> {
        if dead.len() != self.flags.len() {
            return Err(Error::Limit);
        }
        let mut s = 0;
        let mut h = 0;
        for (b, &dead) in dead.iter().enumerate() {
            let sr = self.successors.first[b]..self.successors.first[b + 1];
            let hr = self.handlers.first[b]..self.handlers.first[b + 1];
            self.successors.first[b] = s;
            self.handlers.first[b] = h;
            if dead {
                self.flags[b] = 0;
                continue;
            }
            self.flags[b] &= !HANDLER;
            for k in sr {
                self.successors.targets[s] = self.successors.targets[k];
                s += 1;
            }
            for k in hr {
                self.handlers.targets[h] = self.handlers.targets[k];
                h += 1;
            }
        }
        self.successors.first[dead.len()] = s;
        self.handlers.first[dead.len()] = h;
        self.successors.targets.truncate(s);
        self.handlers.targets.truncate(h);
        for &target in &self.handlers.targets {
            self.flags[target.index()] |= HANDLER;
        }
        let mut placed = vec![None; dead.len()];
        for &block in layout {
            placed[block.index()] = Some(block);
        }
        self.predecessors = self
            .successors
            .reverse(dead.len(), placed.iter().copied().flatten())?;
        Ok(())
    }
    pub(super) fn analyze(body: &Body) -> Result<Self, Error> {
        let mut out = Self::light(body)?;
        out.complete()?;
        Ok(out)
    }
    pub(super) fn light(body: &Body) -> Result<Self, Error> {
        let count = body.blocks().len();
        let mut out = Self {
            successors: Edges::from_count(count),
            predecessors: Edges::default(),
            handlers: Edges::from_count(count),
            flags: vec![0; count],
            rpo: Vec::new(),
            rpo_index: vec![None; count],
            idom: Vec::new(),
            dom_in: Vec::new(),
            dom_out: Vec::new(),
            children: Edges::default(),
        };
        out.build_edges(body)?;
        if let Some(&entry) = body.layout.first() {
            out.build_order(entry);
        }
        Ok(out)
    }
    fn build_edges(&mut self, body: &Body) -> Result<(), Error> {
        let count = body.blocks().len();
        let mut placed = vec![None; count];
        let mut mark = vec![None; count];
        for &block in &body.layout {
            self.flags[block.index()] |= PLACED;
            placed[block.index()] = Some(block);
        }
        for (index, &block) in placed.iter().enumerate() {
            self.successors.first[index] = self.successors.targets.len();
            self.handlers.first[index] = self.handlers.targets.len();
            let Some(block) = block else {
                continue;
            };
            let instructions = &body.block(block).instructions;
            for &id in instructions {
                if let I::Runtime(runtime) = &body.instruction(id).kind {
                    if is_eh(runtime) {
                        self.flags[index] |= EH;
                    }
                    if let Some(target) = runtime.handler_target() {
                        self.handlers.targets.push(target);
                    }
                }
            }
            if let Some(&last) = instructions.last() {
                let mut add = |target: BlockId| {
                    if mark[target.index()] != Some(block) {
                        mark[target.index()] = Some(block);
                        self.successors.targets.push(target);
                    }
                };
                match &body.instruction(last).kind {
                    I::Jump(target) => add(*target),
                    I::Branch(_, yes, no) => {
                        add(*yes);
                        add(*no);
                    }
                    I::Switch(_, fallback, cases) => {
                        add(*fallback);
                        for &(_, target) in cases {
                            add(target);
                        }
                    }
                    _ => (),
                }
            }
        }
        self.successors.first[count] = self.successors.targets.len();
        self.handlers.first[count] = self.handlers.targets.len();
        for &target in &self.handlers.targets {
            self.flags[target.index()] |= HANDLER;
        }
        // Ordinary predecessors include all placed sources, even unreachable
        // ones. Handler edges stay separate, including repeated markers.
        self.predecessors = self
            .successors
            .reverse(count, placed.iter().copied().flatten())?;
        Ok(())
    }
    fn target(&self, block: BlockId, index: usize) -> Option<BlockId> {
        let ordinary = self.successors.at(block);
        if index < ordinary.len() {
            Some(ordinary[index])
        } else {
            self.handlers.at(block).get(index - ordinary.len()).copied()
        }
    }
    fn build_order(&mut self, entry: BlockId) {
        let mut stack = vec![(entry, 0)];
        self.flags[entry.index()] |= REACHABLE;
        while let Some((block, mut next)) = stack.pop() {
            let mut child = None;
            while let Some(target) = self.target(block, next) {
                next += 1;
                if self.flags[target.index()] & (PLACED | REACHABLE) == PLACED {
                    self.flags[target.index()] |= REACHABLE;
                    child = Some(target);
                    break;
                }
            }
            if let Some(child) = child {
                stack.push((block, next));
                stack.push((child, 0));
            } else {
                self.rpo.push(block);
            }
        }
        self.rpo.reverse();
        for (index, &block) in self.rpo.iter().enumerate() {
            self.rpo_index[block.index()] = Some(index);
        }
    }
    pub(super) fn complete(&mut self) -> Result<(), Error> {
        let count = self.flags.len();
        self.idom = vec![None; count];
        if self.rpo.len() > 1 {
            let handlers = self.handlers.reverse(count, self.rpo.iter().copied())?;
            if self.rpo.len() <= ITERATIVE_DOMINATOR_BLOCKS {
                self.iterative_dominators(&handlers)?;
            } else {
                self.large_dominators(&handlers)?;
            }
        }
        self.build_intervals()
    }
    pub(super) fn successors(&self, block: BlockId) -> &[BlockId] {
        self.successors.at(block)
    }
    fn intersect(&self, mut a: BlockId, mut b: BlockId) -> Result<BlockId, Error> {
        while a != b {
            while self.rpo_index[a.index()] > self.rpo_index[b.index()] {
                a = self.idom[a.index()].ok_or(Error::Limit)?;
            }
            while self.rpo_index[b.index()] > self.rpo_index[a.index()] {
                b = self.idom[b.index()].ok_or(Error::Limit)?;
            }
        }
        Ok(a)
    }
    fn iterative_dominators(&mut self, handlers: &Edges) -> Result<(), Error> {
        let entry = self.rpo[0];
        self.idom[entry.index()] = Some(entry);
        loop {
            let mut changed = false;
            for &block in &self.rpo[1..] {
                let mut dom = None;
                for &pred in self.predecessors.at(block).iter().chain(handlers.at(block)) {
                    if self.idom[pred.index()].is_none() {
                        continue;
                    }
                    dom = Some(dom.map_or(Ok(pred), |previous| self.intersect(pred, previous))?);
                }
                if self.idom[block.index()] != dom {
                    self.idom[block.index()] = dom;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        self.idom[entry.index()] = None;
        Ok(())
    }
    fn large_dominators(&mut self, handlers: &Edges) -> Result<(), Error> {
        let entry = self.rpo[0];
        let mut number = vec![None; self.flags.len()];
        let mut vertex = vec![entry];
        let mut parent = vec![0];
        let mut stack = vec![(entry, 0)];
        number[entry.index()] = Some(0);
        // DFS preorder over ordinary successors followed by handler targets.
        while let Some((block, next)) = stack.pop() {
            let Some(target) = self.target(block, next) else {
                continue;
            };
            stack.push((block, next + 1));
            if self.flags[target.index()] & REACHABLE == 0 || number[target.index()].is_some() {
                continue;
            }
            number[target.index()] = Some(vertex.len());
            vertex.push(target);
            parent.push(number[block.index()].ok_or(Error::Limit)?);
            stack.push((target, 0));
        }
        let count = vertex.len();
        let mut forest = Forest::new(count);
        let mut idom = vec![0; count];
        let mut bucket = vec![None; count];
        let mut next = vec![None; count];
        for w in (1..count).rev() {
            let block = vertex[w];
            for &pred in self.predecessors.at(block).iter().chain(handlers.at(block)) {
                if let Some(index) = number[pred.index()] {
                    let u = forest.eval(index)?;
                    forest.semi[w] = forest.semi[w].min(forest.semi[u]);
                }
            }
            next[w] = bucket[forest.semi[w]];
            bucket[forest.semi[w]] = Some(w);
            let p = parent[w];
            forest.ancestor[w] = Some(p);
            let mut member = bucket[p];
            while let Some(v) = member {
                let u = forest.eval(v)?;
                idom[v] = if forest.semi[u] < forest.semi[v] {
                    u
                } else {
                    p
                };
                member = next[v];
            }
            bucket[p] = None;
        }
        for w in 1..count {
            if idom[w] != forest.semi[w] {
                idom[w] = idom[idom[w]];
            }
            self.idom[vertex[w].index()] = Some(vertex[idom[w]]);
        }
        Ok(())
    }
    fn build_intervals(&mut self) -> Result<(), Error> {
        let count = self.flags.len();
        self.dom_in = vec![0; count];
        self.dom_out = vec![0; count];
        self.children = Edges::from_count(count);
        let Some(&entry) = self.rpo.first() else {
            return Ok(());
        };
        for &block in &self.rpo {
            if let Some(parent) = self.idom[block.index()] {
                self.children.first[parent.index() + 1] += 1;
            }
        }
        for index in 0..count {
            self.children.first[index + 1] += self.children.first[index];
        }
        self.children
            .targets
            .resize(self.children.first[count], entry);
        let mut fill = self.children.first[..count].to_vec();
        for &block in &self.rpo {
            if let Some(parent) = self.idom[block.index()] {
                self.children.targets[fill[parent.index()]] = block;
                fill[parent.index()] += 1;
            }
        }
        let mut clock = 1_usize;
        let mut stack = vec![(entry, 0)];
        while let Some((block, next)) = stack.pop() {
            if let Some(&child) = self.children.at(block).get(next) {
                self.dom_in[child.index()] = clock;
                clock = clock.checked_add(1).ok_or(Error::Limit)?;
                stack.push((block, next + 1));
                stack.push((child, 0));
            } else {
                self.dom_out[block.index()] = clock;
                clock = clock.checked_add(1).ok_or(Error::Limit)?;
            }
        }
        Ok(())
    }
}
pub(super) fn is_eh(runtime: &R) -> bool {
    matches!(
        runtime,
        R::EhTry(_)
            | R::EhCleanup(_)
            | R::EhCatch(..)
            | R::EhFilter(..)
            | R::EhCatchAll(_)
            | R::EhEnd
            | R::Throw(..)
            | R::Resume
            | R::Exception(_)
            | R::ExceptionSelector(_)
    )
}

struct Forest {
    semi: Vec<usize>,
    label: Vec<usize>,
    ancestor: Vec<Option<usize>>,
    chain: Vec<usize>,
}
impl Forest {
    fn new(count: usize) -> Self {
        Self {
            semi: (0..count).collect(),
            label: (0..count).collect(),
            ancestor: vec![None; count],
            chain: Vec::new(),
        }
    }
    fn eval(&mut self, vertex: usize) -> Result<usize, Error> {
        if self.ancestor[vertex].is_none() {
            return Ok(vertex);
        }
        self.chain.clear();
        let mut current = vertex;
        while let Some(parent) = self.ancestor[current] {
            if self.ancestor[parent].is_none() {
                break;
            }
            self.chain.push(current);
            current = parent;
        }
        for &vertex in self.chain.iter().rev() {
            let parent = self.ancestor[vertex].ok_or(Error::Limit)?;
            if self.semi[self.label[parent]] < self.semi[self.label[vertex]] {
                self.label[vertex] = self.label[parent];
            }
            self.ancestor[vertex] = self.ancestor[parent];
        }
        Ok(self.label[vertex])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{builder::FunctionBuilder, model::Program, vocabulary::Type};

    #[test]
    fn edge_kinds_order_unreachable_sources_and_dominator_intervals_are_exact() {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void).unwrap();
        builder.block("entry").unwrap();
        let entry = builder.block_id("entry").unwrap();
        let landing = builder.block_id("landing").unwrap();
        let join = builder.block_id("join").unwrap();
        let left = builder.block_id("left").unwrap();
        builder.effect(I::Runtime(R::EhTry(landing))).unwrap();
        builder
            .effect(I::Runtime(R::EhCleanup(Some(landing))))
            .unwrap();
        builder.effect(I::Runtime(R::EhEnd)).unwrap();
        builder.effect(I::Runtime(R::EhEnd)).unwrap();
        let condition = builder.integer(1).unwrap();
        builder.effect(I::Branch(condition, left, left)).unwrap();
        builder.block("dead").unwrap();
        let dead = builder.block_id("dead").unwrap();
        builder.effect(I::Jump(join)).unwrap();
        builder.block("left").unwrap();
        builder.effect(I::Jump(join)).unwrap();
        builder.block("landing").unwrap();
        builder.effect(I::Jump(join)).unwrap();
        builder.block("join").unwrap();
        builder.effect(I::Return(Type::Void, None)).unwrap();
        builder.finish().unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        let body = program.functions()[0].body.as_ref().unwrap();
        let cfg = Cfg::analyze(body).unwrap();
        assert_eq!(cfg.successors.at(entry), [left]);
        assert_eq!(cfg.handlers.at(entry), [landing, landing]);
        assert_eq!(cfg.predecessors.at(join), [landing, left, dead]);
        assert_eq!(cfg.flags[entry.index()], PLACED | REACHABLE | EH);
        assert_eq!(cfg.flags[landing.index()], PLACED | REACHABLE | HANDLER);
        assert_eq!(cfg.flags[dead.index()], PLACED);
        assert_eq!(cfg.rpo, [entry, landing, left, join]);
        assert_eq!(cfg.idom[dead.index()], None);
        assert_eq!(cfg.rpo_index[dead.index()], None);
        assert_eq!(cfg.children.at(entry), [landing, left, join]);
        for (block, inside, outside) in [(entry, 0, 7), (landing, 1, 2), (left, 3, 4), (join, 5, 6)]
        {
            assert_eq!(
                (cfg.dom_in[block.index()], cfg.dom_out[block.index()]),
                (inside, outside)
            );
            assert_eq!(cfg.idom[block.index()], (block != entry).then_some(entry));
        }
    }

    fn random_body(count: usize, mut seed: usize) -> Program {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void).unwrap();
        let mut blocks = vec![None; count];
        // Allocate identities in the opposite order from layout.
        for index in (0..count).rev() {
            blocks[index] = Some(builder.block_id(&format!("b{index}")).unwrap());
        }
        let blocks = blocks.into_iter().map(Option::unwrap).collect::<Vec<_>>();
        let condition = builder.integer(1).unwrap();
        for index in 0..count {
            builder.block(&format!("b{index}")).unwrap();
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let target = blocks[seed % count];
            if index % 11 == 0 {
                builder.effect(I::Runtime(R::EhTry(target))).unwrap();
                builder.effect(I::Runtime(R::EhEnd)).unwrap();
            }
            if index + 1 == count {
                builder.effect(I::Return(Type::Void, None)).unwrap();
            } else {
                builder
                    .effect(I::Branch(condition, target, blocks[index + 1]))
                    .unwrap();
            }
        }
        builder.finish().unwrap();
        program
    }
    // Independent path-set fixed point, deliberately unlike either production
    // dominator algorithm. It is small and bounded to test-sized graphs.
    fn reference_dominators(cfg: &Cfg) -> Vec<Vec<bool>> {
        let count = cfg.flags.len();
        let entry = cfg.rpo[0];
        let mut sets = vec![vec![true; count]; count];
        sets[entry.index()].fill(false);
        sets[entry.index()][entry.index()] = true;
        loop {
            let mut changed = false;
            for &block in &cfg.rpo[1..] {
                let mut incoming = vec![true; count];
                for &pred in &cfg.rpo {
                    if !cfg.successors.at(pred).contains(&block)
                        && !cfg.handlers.at(pred).contains(&block)
                    {
                        continue;
                    }
                    for (value, &predecessor) in incoming.iter_mut().zip(&sets[pred.index()]) {
                        *value &= predecessor;
                    }
                }
                incoming[block.index()] = true;
                if sets[block.index()] != incoming {
                    sets[block.index()] = incoming;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        sets
    }
    #[test]
    fn both_dominator_algorithms_match_path_sets_across_threshold_and_cycles() {
        for count in [2, 4, 16, 32, 33, 64, 97] {
            for seed in 0..8 {
                let program = random_body(count, seed);
                crate::lowir::validate_test_program(&program).unwrap();
                let body = program.functions()[0].body.as_ref().unwrap();
                let cfg = Cfg::analyze(body).unwrap();
                assert_eq!(cfg.rpo.len(), count);
                let reference = reference_dominators(&cfg);
                let mut other = Cfg::light(body).unwrap();
                other.idom = vec![None; count];
                let handlers = other
                    .handlers
                    .reverse(count, other.rpo.iter().copied())
                    .unwrap();
                if count <= ITERATIVE_DOMINATOR_BLOCKS {
                    other.large_dominators(&handlers).unwrap();
                } else {
                    other.iterative_dominators(&handlers).unwrap();
                }
                other.build_intervals().unwrap();
                assert_eq!(cfg.idom, other.idom, "count={count} seed={seed}");
                assert_eq!(cfg.dom_in, other.dom_in);
                assert_eq!(cfg.dom_out, other.dom_out);
                for &a in &cfg.rpo {
                    for &b in &cfg.rpo {
                        let dominates = cfg.dom_in[a.index()] <= cfg.dom_in[b.index()]
                            && cfg.dom_out[b.index()] <= cfg.dom_out[a.index()];
                        assert_eq!(
                            dominates,
                            reference[b.index()][a.index()],
                            "count={count} seed={seed} a={a:?} b={b:?}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn long_chain_and_fanin_stay_iterative_and_keep_the_exact_tree() {
        let count = 4096;
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void).unwrap();
        let join = builder.block_id("join").unwrap();
        let mut chain = Vec::new();
        let condition = builder.integer(1).unwrap();
        for index in 0..count {
            builder.block(&format!("b{index}")).unwrap();
            let current = builder.block_id(&format!("b{index}")).unwrap();
            chain.push(current);
            if index + 1 == count {
                builder.effect(I::Jump(join)).unwrap();
            } else {
                let next = builder.block_id(&format!("b{}", index + 1)).unwrap();
                builder.effect(I::Branch(condition, next, join)).unwrap();
            }
        }
        builder.block("join").unwrap();
        builder.effect(I::Return(Type::Void, None)).unwrap();
        builder.finish().unwrap();
        let body = program.functions()[0].body.as_ref().unwrap();
        let cfg = Cfg::analyze(body).unwrap();
        assert_eq!(cfg.rpo.len(), count + 1);
        assert_eq!(cfg.idom[chain[0].index()], None);
        assert_eq!(cfg.idom[join.index()], Some(chain[0]));
        for pair in chain.windows(2) {
            assert_eq!(cfg.idom[pair[1].index()], Some(pair[0]));
        }
    }
    #[test]
    fn empty_and_single_block_bodies_have_no_immediate_dominator() {
        let cfg = Cfg::analyze(&Body::default()).unwrap();
        assert!(cfg.rpo.is_empty() && cfg.idom.is_empty());
        let program =
            crate::lowir::test_program(b"function @f() -> void {block ^entry: return void}\n")
                .unwrap();
        let body = program.functions()[0].body.as_ref().unwrap();
        let cfg = Cfg::analyze(body).unwrap();
        assert_eq!(cfg.idom, [None]);
        assert_eq!(cfg.dom_in, [0]);
        assert_eq!(cfg.dom_out, [1]);
    }
}
