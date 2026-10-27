//! Frozen MIR successor probabilities, chain placement and label relocation. See NOTICE.
use super::{
    condition::Condition as C,
    condition_mir::FlagOp,
    exception::ExceptionOp,
    mir::{BlockId, Function, Op},
    scalar_mir::{Operand, ScalarOp},
};
use crate::lowir::{
    Error,
    vocabulary::{Binary, Type},
};
pub(super) fn terminal(op: Op) -> bool {
    matches!(
        op,
        Op::Jump(_)
            | Op::JumpTable(_)
            | Op::Return(_)
            | Op::FramelessReturn(_)
            | Op::Trap
            | Op::TailJump(_)
            | Op::Exception(ExceptionOp::Throw(_) | ExceptionOp::Resume)
            | Op::HostEh(super::landing_pad::Effect::Resume)
    ) || matches!(op,Op::WideCompare(w) if matches!(w.destination,super::wide_compare::Destination::Branch{..}))
        || matches!(op,Op::Call(c) if c.no_return)
        || matches!(
            op,
            Op::Floating(
                super::floating_mir::FloatingOp::Return(_)
                    | super::floating_mir::FloatingOp::ReturnPair(..)
            )
        )
}
pub(super) fn labels(op: &mut Op, mut map: impl FnMut(BlockId) -> BlockId) {
    match op {
        Op::Jump(to) | Op::Branch(_, to) | Op::Exception(ExceptionOp::Push { target: to, .. }) => {
            *to = map(*to);
        }
        Op::WideCompare(w) => {
            if let super::wide_compare::Destination::Branch { yes, no } = &mut w.destination {
                *yes = map(*yes);
                *no = map(*no);
            }
        }
        _ => {}
    }
}
fn root(parent: &mut [usize], mut b: usize) -> usize {
    while parent[b] != b {
        parent[b] = parent[parent[b]];
        b = parent[b];
    }
    b
}
pub(super) struct Flow {
    successors: Vec<Vec<usize>>,
    two_way: Vec<bool>,
    back: Vec<Vec<bool>>,
    cold: Vec<bool>,
    depth: Vec<usize>,
    inner: Vec<Option<usize>>,
    loops: Vec<Vec<usize>>,
    post: Vec<usize>,
    header: Vec<bool>,
    pred_count: Vec<usize>,
    pub(super) frequencies: Vec<u64>,
}
impl Flow {
    pub(super) fn new(f: &Function) -> Option<Self> {
        let n = f.blocks.len();
        if n == 0 {
            return None;
        }
        let mut out = Self {
            successors: vec![Vec::new(); n],
            two_way: vec![false; n],
            back: vec![Vec::new(); n],
            cold: vec![false; n],
            depth: vec![0; n],
            inner: vec![None; n],
            loops: Vec::new(),
            post: Vec::new(),
            header: vec![false; n],
            pred_count: vec![0; n],
            frequencies: vec![0; n],
        };
        for b in 0..n {
            let range = f.blocks[b].range.clone();
            let last = f.instructions.get(range.end.checked_sub(1)?)?.op;
            if range.is_empty() || !terminal(last) {
                return None;
            }
            let mut branches = 0;
            for i in range.clone() {
                let op = f.instructions[i].op;
                match op {
                    Op::Jump(t) | Op::Branch(_, t) => {
                        branches += 1;
                        out.successors[b].push(t.index());
                    }
                    Op::JumpTable(table) => {
                        branches += 1;
                        out.successors[b]
                            .extend(f.jump_targets[table.range()].iter().map(|t| t.index()));
                    }
                    Op::WideCompare(w) => {
                        if let super::wide_compare::Destination::Branch { yes, no } = w.destination
                        {
                            branches += 1;
                            out.successors[b].extend([yes.index(), no.index()]);
                        }
                    }
                    _ => {}
                }
                out.cold[b] |= matches!(
                    op,
                    Op::Trap
                        | Op::Exception(ExceptionOp::Throw(_) | ExceptionOp::Resume)
                        | Op::HostEh(super::landing_pad::Effect::Resume)
                ) || matches!(op,Op::Call(c) if c.no_return);
            }
            out.cold[b] |= f.blocks[b].landing;
            out.two_way[b] = branches == 2
                && out.successors[b].len() == 2
                && range.len() >= 2
                && matches!(last, Op::Jump(_))
                && matches!(f.instructions[range.end - 2].op, Op::Branch(..))
                && out.successors[b][0] != out.successors[b][1];
            out.back[b].resize(out.successors[b].len(), false);
            for &s in &out.successors[b] {
                if s >= n {
                    return None;
                }
                out.pred_count[s] += 1;
            }
        }
        loop {
            let mut changed = false;
            for b in (0..n).rev() {
                if !out.cold[b]
                    && !out.successors[b].is_empty()
                    && out.successors[b].iter().all(|&s| out.cold[s])
                {
                    out.cold[b] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        out.find_loops();
        out.weigh(f);
        Some(out)
    }
    fn find_loops(&mut self) {
        let n = self.successors.len();
        let mut state = vec![0; n];
        let mut next = vec![0; n];
        let mut stack = vec![0];
        state[0] = 1;
        while let Some(&b) = stack.last() {
            if next[b] == self.successors[b].len() {
                state[b] = 2;
                self.post.push(b);
                let _ = stack.pop();
                continue;
            }
            let k = next[b];
            next[b] += 1;
            let s = self.successors[b][k];
            if state[s] == 1 {
                self.back[b][k] = true;
            } else if state[s] == 0 {
                state[s] = 1;
                stack.push(s);
            }
        }
        let mut preds = vec![Vec::new(); n];
        for b in 0..n {
            for &s in &self.successors[b] {
                preds[s].push(b);
            }
        }
        let mut header_loop = vec![None; n];
        let mut mark = vec![None; n];
        let mut work = Vec::new();
        let back_edges: Vec<_> = self
            .back
            .iter()
            .enumerate()
            .flat_map(|(b, edges)| {
                edges
                    .iter()
                    .enumerate()
                    .filter_map(move |(k, &back)| back.then_some((b, k)))
            })
            .collect();
        for (b, k) in back_edges {
            let h = self.successors[b][k];
            let l = if let Some(l) = header_loop[h] {
                l
            } else {
                let l = self.loops.len();
                header_loop[h] = Some(l);
                self.loops.push(vec![h]);
                self.header[h] = true;
                l
            };
            mark[h] = Some(l);
            if mark[b] != Some(l) {
                mark[b] = Some(l);
                self.loops[l].push(b);
                work.push(b);
            }
            while let Some(x) = work.pop() {
                for &y in &preds[x] {
                    if state[y] == 0 || mark[y] == Some(l) {
                        continue;
                    }
                    mark[y] = Some(l);
                    self.loops[l].push(y);
                    work.push(y);
                }
            }
        }
        for l in 0..self.loops.len() {
            self.loops[l].sort_unstable();
            self.loops[l].dedup();
            if self.loops[l][0] == 0 {
                continue;
            }
            for &b in &self.loops[l] {
                self.depth[b] += 1;
                if self.inner[b].is_none_or(|i| self.loops[i].len() > self.loops[l].len()) {
                    self.inner[b] = Some(l);
                }
            }
        }
    }
    fn taken(&self, f: &Function, b: usize) -> u64 {
        let t = self.successors[b][0];
        let no = self.successors[b][1];
        if self.cold[t] != self.cold[no] {
            return if self.cold[t] { 20 } else { 980 };
        }
        if self.back[b][0] != self.back[b][1] {
            return if self.back[b][0] { 880 } else { 120 };
        }
        if let Some(l) = self.inner[b] {
            let t_exits = self.loops[l].binary_search(&t).is_err();
            let f_exits = self.loops[l].binary_search(&no).is_err();
            if t_exits != f_exits {
                return if t_exits { 120 } else { 880 };
            }
        }
        let calls = |s: usize| {
            f.blocks[s]
                .range
                .clone()
                .any(|i| matches!(f.instructions[i].op, Op::Call(_)))
        };
        if calls(t) != calls(no) {
            let p = 300 / self.pred_count[if calls(t) { t } else { no }].max(1) as u64;
            return if calls(t) { p } else { 1000 - p };
        }
        let returns = |s: usize| {
            f.blocks[s].range.clone().last().is_some_and(|i| {
                matches!(f.instructions[i].op, Op::Return(_) | Op::FramelessReturn(_))
            })
        };
        if returns(t) != returns(no) {
            return if returns(t) { 350 } else { 650 };
        }
        let end = f.blocks[b].range.end;
        let Op::Branch(cond, _) = f.instructions[end - 2].op else {
            return 500;
        };
        if f.blocks[b].range.len() >= 3 {
            let test = f.instructions[end - 3].op;
            let self_test = match test {
                Op::Scalar(ScalarOp::Condition(flag)) => flag.self_test(),
                _ => None,
            };
            let zero = self_test.is_some()
                || matches!(
                    test,
                    Op::Scalar(ScalarOp::Condition(FlagOp::Compare(
                        _,
                        Operand::Register(_),
                        Operand::Immediate(0)
                    )))
                );
            if zero && matches!(cond, C::Less | C::GreaterEqual) {
                return if cond == C::Less { 300 } else { 700 };
            }
            if matches!(cond, C::Equal | C::NotEqual) {
                if matches!(test,Op::Scalar(ScalarOp::Condition(FlagOp::Compare(_,a,Operand::Immediate(0)))) if a.is_memory())
                {
                    return if cond == C::Equal { 700 } else { 300 };
                }
                let byte = self_test
                    .is_some_and(|(ty, _)| matches!(ty, Type::I1 | Type::I8 | Type::U8))
                    || matches!(
                        test,
                        Op::Scalar(ScalarOp::Condition(FlagOp::Compare(
                            Type::I1 | Type::I8 | Type::U8,
                            _,
                            _
                        )))
                    );
                if byte && zero {
                    return if cond == C::Equal { 600 } else { 400 };
                }
            }
        }
        match cond {
            C::Equal => 450,
            C::NotEqual => 550,
            _ => 500,
        }
    }
    fn weigh(&mut self, f: &Function) {
        let mut inflow = vec![0_u64; self.successors.len()];
        inflow[0] = 1 << 20;
        for &b in self.post.iter().rev() {
            self.frequencies[b] = inflow[b]
                .wrapping_mul(if self.header[b] { 8 } else { 1 })
                .min(1 << 50);
            let p = if self.two_way[b] { self.taken(f, b) } else { 0 };
            for (k, &s) in self.successors[b].iter().enumerate() {
                let share = if self.two_way[b] {
                    if k == 0 { p } else { 1000 - p }
                } else {
                    1000 / self.successors[b].len() as u64
                };
                if !self.back[b][k] {
                    inflow[s] = inflow[s].wrapping_add(self.frequencies[b] * share / 1000);
                }
            }
        }
    }
    pub(super) fn place(&self, function: &mut Function) -> Result<(), Error> {
        let count = function.blocks.len();
        if count < 3 {
            return Ok(());
        }
        let mut edges = Vec::new();
        for b in 0..count {
            let probability = if self.two_way[b] {
                self.taken(function, b)
            } else {
                0
            };
            for (k, &successor) in self.successors[b].iter().enumerate() {
                if successor == b || successor == 0 {
                    continue;
                }
                let share = if self.two_way[b] {
                    if k == 0 {
                        probability
                    } else {
                        1000 - probability
                    }
                } else {
                    1000 / self.successors[b].len() as u64
                };
                edges.push((
                    self.frequencies[b] * share / 1000,
                    successor == b + 1,
                    b,
                    successor,
                ));
            }
        }
        edges.sort_by_key(|&(w, next, b, successor)| (std::cmp::Reverse(w), !next, b, successor));
        let mut parent: Vec<_> = (0..count).collect();
        let mut tail: Vec<_> = (0..count).collect();
        let mut next = vec![None; count];
        for (_, _, b, successor) in edges {
            let first = root(&mut parent, b);
            let second = root(&mut parent, successor);
            if first != second && tail[first] == b && second == successor {
                next[b] = Some(successor);
                tail[first] = tail[second];
                parent[second] = first;
            }
        }
        for (b, next_block) in next.iter_mut().enumerate() {
            if next_block.is_some() || self.cold[b] || self.successors[b].len() != 1 {
                continue;
            }
            let range = function.blocks[b].range.clone();
            if !matches!(function.instructions[range.end - 1].op, Op::Jump(_)) {
                continue;
            }
            let test = self.successors[b][0];
            if test == b
                || test == 0
                || !self.two_way[test]
                || function.blocks[test].landing
                || !copyable(function, test, 5, false)
                || root(&mut parent, test) == test
            {
                continue;
            }
            let probability = self.taken(function, test);
            let [first, second] = [self.successors[test][0], self.successors[test][1]];
            let order = if probability >= 500 {
                [first, second]
            } else {
                [second, first]
            };
            let successor = order.into_iter().find(|&successor| {
                successor != 0
                    && successor != b
                    && root(&mut parent, successor) == successor
                    && root(&mut parent, successor) != root(&mut parent, b)
                    && !self.cold[successor]
            });
            if let Some(successor) = successor {
                duplicate_test(function, b, test);
                let first = root(&mut parent, b);
                let second = root(&mut parent, successor);
                *next_block = Some(successor);
                tail[first] = tail[second];
                parent[second] = first;
            }
        }
        let mut chains: Vec<_> = (1..count).filter(|&b| root(&mut parent, b) == b).collect();
        chains.sort_by_key(|&b| (self.cold[b], b));
        let returns = |b: usize| {
            function.blocks[b].range.clone().last().is_some_and(|i| {
                matches!(
                    function.instructions[i].op,
                    Op::Return(_) | Op::FramelessReturn(_)
                )
            })
        };
        if returns(count - 1)
            && chains.last().is_some_and(|&second| !returns(tail[second]))
            && let Some(k) = chains.iter().rposition(|&second| returns(tail[second]))
        {
            let second = chains.remove(k);
            chains.push(second);
        }
        let mut order = Vec::with_capacity(count);
        for start in std::iter::once(0).chain(chains) {
            let mut b = Some(start);
            while let Some(i) = b {
                order.push(i);
                b = next[i];
            }
        }
        reorder(function, &order)
    }
}
pub(super) fn copyable(f: &Function, b: usize, cap: usize, sets: bool) -> bool {
    let range = f.blocks[b].range.clone();
    if range.len() < 2
        || range.len() > cap
        || !matches!(f.instructions[range.end - 1].op, Op::Jump(_))
        || !matches!(f.instructions[range.end - 2].op, Op::Branch(..))
    {
        return false;
    }
    range
        .take(f.blocks[b].range.len() - 2)
        .all(|i| match f.instructions[i].op {
            Op::Nop
            | Op::Move(..)
            | Op::Scalar(
                ScalarOp::Move(..)
                | ScalarOp::Load(..)
                | ScalarOp::Lea(..)
                | ScalarOp::Lea32(..)
                | ScalarOp::Extend(..)
                | ScalarOp::ExtendMove(..)
                | ScalarOp::Movzx(..)
                | ScalarOp::Shift(..)
                | ScalarOp::CompareZero(..)
                | ScalarOp::Binary(
                    Binary::Add | Binary::Sub | Binary::And | Binary::Or | Binary::Xor,
                    ..,
                )
                | ScalarOp::MemoryBinary(
                    Binary::Add | Binary::Sub | Binary::And | Binary::Or | Binary::Xor,
                    ..,
                )
                | ScalarOp::Condition(
                    FlagOp::Compare(..)
                    | FlagOp::Test(..)
                    | FlagOp::MaskTest(..)
                    | FlagOp::BitTest(..),
                ),
            ) => true,
            Op::Scalar(ScalarOp::Condition(FlagOp::Set(..))) => sets,
            _ => false,
        })
}
pub(super) fn duplicate_test(f: &mut Function, b: usize, t: usize) {
    let range = f.blocks[b].range.clone();
    let test = f.blocks[t].range.clone();
    let start = f.instructions.len();
    for i in range.start..range.end - 1 {
        f.instructions.push(f.instructions[i]);
    }
    for i in test {
        f.instructions.push(f.instructions[i]);
    }
    f.blocks[b].range = start..f.instructions.len();
}
pub(super) fn reorder(f: &mut Function, order: &[usize]) -> Result<(), Error> {
    let mut map = vec![BlockId(0); order.len()];
    for (k, &b) in order.iter().enumerate() {
        map[b] = BlockId(u32::try_from(k).map_err(|_| Error::Limit)?);
    }
    let mut old: Vec<_> = std::mem::take(&mut f.blocks)
        .into_iter()
        .map(Some)
        .collect();
    for &b in order {
        f.blocks.push(old[b].take().ok_or(Error::Limit)?);
    }
    for block in &f.blocks {
        for i in block.range.clone() {
            labels(&mut f.instructions[i].op, |id| map[id.index()]);
        }
    }
    for target in &mut f.jump_targets {
        *target = map[target.index()];
    }
    for landing in &mut f.eh.landings {
        landing.pad = landing.pad.map(|b| map[b.index()]);
        landing.select = landing.select.map(|b| map[b.index()]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{
        floating_mir::FloatingOp,
        mir::{BlockName, Instruction},
        object_compile::ObjectModel,
    };

    fn function(
        program: &crate::lowir::model::Program,
        blocks: impl IntoIterator<Item = Vec<Op>>,
    ) -> Result<Function, Error> {
        let symbol = program.publications()[0];
        let mut function = super::super::value_selection::select_for_object(
            program,
            symbol,
            program
                .function(program.symbol(symbol))
                .ok_or(Error::Limit)?,
            0,
            ObjectModel::Host,
        )?;
        function.instructions.clear();
        function.blocks.clear();
        for ops in blocks {
            let _ = function.begin_block(BlockName::Switch)?;
            function
                .instructions
                .extend(ops.into_iter().map(|op| Instruction {
                    op,
                    volatile: false,
                    location: None,
                }));
            function.end_block();
        }
        Ok(function)
    }

    #[test]
    fn pair_return_keeps_placement_frequency_analysis_available() -> Result<(), Error> {
        let program =
            crate::lowir::test_program(b"function @f() -> i64 {block ^entry: return i64 0}")?;
        let function = function(
            &program,
            [
                vec![Op::Branch(C::Equal, BlockId(1)), Op::Jump(BlockId(2))],
                vec![Op::Floating(FloatingOp::ReturnPair(
                    Operand::Frame(-32),
                    Operand::Frame(-16),
                ))],
                vec![Op::Trap],
            ],
        )?;
        let flow = Flow::new(&function).ok_or(Error::Limit)?;
        assert!(flow.frequencies[1] > flow.frequencies[2]);
        assert_eq!(flow.post.len(), function.blocks.len());
        Ok(())
    }

    #[test]
    fn host_layout_retains_frozen_floating_return_successor() -> Result<(), Error> {
        let program =
            crate::lowir::test_program(b"function @f() -> i64 {block ^entry: return i64 0}")?;
        let mut function = function(
            &program,
            [
                vec![Op::Floating(FloatingOp::Return(Operand::Frame(-16)))],
                vec![Op::Return(None)],
            ],
        )?;
        super::super::branch_layout::layout(&mut function, &program)?;
        assert_eq!(function.blocks.len(), 2);
        Ok(())
    }
}
