//! Cost-bounded frame-free regions and register stand-ins (`shrink_wrap.cpp`). See NOTICE.
use super::{
    mir::{Function, Op},
    mir_registers::{named, written},
    scalar_mir::{Operand, ScalarOp},
    x86::Register as R,
};

const CALLEES: [R; 6] = [R::RBX, R::RBP, R::R12, R::R13, R::R14, R::R15];
const CALLERS: [R; 7] = [R::RAX, R::RCX, R::RDX, R::RSI, R::RDI, R::R8, R::R9];
pub(super) struct Spill {
    pub(super) at: usize,
    pub(super) offset: i32,
    pub(super) width: u8,
    pub(super) source: R,
}
pub(super) fn reload(op: Op, spills: &[Spill]) -> Option<usize> {
    if let Op::Scalar(ScalarOp::Load(r, ty, source)) = op
        && r.code() < 16
        && ty != crate::lowir::vocabulary::Type::I1
        && let Some(offset) = source.frame_offset()
    {
        spills
            .iter()
            .position(|s| s.offset == offset && s.width == super::scalar_mir::bytes(ty))
    } else {
        None
    }
}
pub(super) struct Graph {
    pub(super) needs: Vec<bool>,
    pub(super) framed: Vec<bool>,
    pub(super) successors: Vec<Vec<usize>>,
    pub(super) spills: Vec<Spill>,
}
pub(super) struct Plan {
    pub(super) region: Vec<bool>,
    pub(super) copied: Vec<bool>,
    pub(super) exits: Vec<usize>,
    pub(super) copies: Vec<(R, R)>,
    pub(super) spare: Vec<Option<R>>,
    pub(super) rename: [R; 16],
}
impl Graph {
    pub(super) fn plan(&self, f: &Function) -> Option<Plan> {
        // Ordered admissions: wide joins, short returns, then three minimal
        // early-exit budgets. Failed plans have no effect on the function.
        for mode in [64, 0, 16, 8, 4] {
            if let Some(plan) = self.attempt(f, mode) {
                return Some(plan);
            }
        }
        None
    }
    fn short_return(&self, f: &Function, b: usize) -> bool {
        let range = &f.blocks[b].range;
        !self.needs[b]
            && range.len() <= 4
            && matches!(f.instructions[range.end - 1].op, Op::Return(_))
    }
    fn attempt(&self, f: &Function, mode: usize) -> Option<Plan> {
        let n = f.blocks.len();
        let minimal = mode != 64 && mode != 0;
        let mut budget = if minimal { mode } else { 64 };
        let mut plan = Plan {
            region: vec![false; n],
            copied: vec![false; n],
            exits: Vec::new(),
            copies: Vec::new(),
            spare: vec![None; self.spills.len()],
            rename: [
                R::RAX,
                R::RCX,
                R::RDX,
                R::RBX,
                R::RSP,
                R::RBP,
                R::RSI,
                R::RDI,
                R::R8,
                R::R9,
                R::R10,
                R::R11,
                R::R12,
                R::R13,
                R::R14,
                R::R15,
            ],
        };
        plan.region[0] = true;
        let mut work = vec![0];
        let mut returns = false;
        let mut names = 0;
        let mut exit = vec![false; n];
        while let Some(b) = work.pop() {
            for i in f.blocks[b].range.clone() {
                let op = f.instructions[i].op;
                returns |= matches!(op, Op::Return(_));
                names |= named(op);
            }
            for &s in &self.successors[b] {
                if s >= n || minimal && s == 0 {
                    return None;
                }
                let short = self.short_return(f, s);
                if minimal {
                    if plan.copied[s] {
                        continue;
                    }
                    if short || !self.needs[s] && f.blocks[s].range.len() <= budget {
                        budget -= usize::from(!short) * f.blocks[s].range.len();
                        plan.copied[s] = true;
                        work.push(s);
                    } else if !exit[s] {
                        exit[s] = true;
                        plan.exits.push(s);
                    }
                } else if self.framed[s]
                    && (plan.copied[s]
                        || if mode == 64 {
                            !self.needs[s] && f.blocks[s].range.len() <= budget
                        } else {
                            short
                        })
                {
                    if !plan.copied[s] {
                        plan.copied[s] = true;
                        budget = budget.wrapping_sub(f.blocks[s].range.len());
                        work.push(s);
                    }
                } else if self.framed[s] {
                    if !exit[s] {
                        exit[s] = true;
                        plan.exits.push(s);
                    }
                } else if !plan.region[s] {
                    plan.region[s] = true;
                    work.push(s);
                }
            }
        }
        if !returns || plan.exits.is_empty() {
            return None;
        }
        self.registers(f, plan, names)
    }
    fn registers(&self, f: &Function, mut plan: Plan, names: u32) -> Option<Plan> {
        let n = f.blocks.len();
        let included = |b: usize| plan.region[b] || plan.copied[b];
        // Backward reachability replaces the frozen monotone boolean rounds;
        // each edge is visited once, with the identical final to-exit facts.
        let mut predecessors = vec![Vec::new(); n];
        let mut to_exit = vec![false; n];
        let mut work = Vec::new();
        for (b, to_exit) in to_exit.iter_mut().enumerate() {
            if !included(b) {
                continue;
            }
            for &s in &self.successors[b] {
                if included(s) {
                    predecessors[s].push(b);
                } else if !*to_exit {
                    *to_exit = true;
                    work.push(b);
                }
            }
        }
        while let Some(b) = work.pop() {
            for &p in &predecessors[b] {
                if !to_exit[p] {
                    to_exit[p] = true;
                    work.push(p);
                }
            }
        }
        let mut writes = [0_u32; 16];
        let mut exit_writes = [0_u32; 16];
        let mut leaf_names = 0;
        let mut kept = vec![false; self.spills.len()];
        for (b, &exits) in to_exit.iter().enumerate() {
            if !included(b) {
                continue;
            }
            for i in f.blocks[b].range.clone() {
                let op = f.instructions[i].op;
                if !exits {
                    leaf_names |= named(op);
                }
                if let Some(r) = written(op).filter(|r| r.code() < 16) {
                    writes[usize::from(r.code())] += 1;
                    if exits {
                        exit_writes[usize::from(r.code())] += 1;
                    }
                }
                if let Some(k) = reload(op, &self.spills) {
                    kept[k] = true;
                }
            }
        }
        for (k, spill) in self.spills.iter().enumerate() {
            kept[k] |= exit_writes[usize::from(spill.source.code())] != 0;
        }
        let incoming = f
            .layout
            .arguments
            .iter()
            .flat_map(|a| f.layout.parts(a))
            .filter_map(|p| p.register)
            .filter(|r| r.code() < 16)
            .fold(0, |bits, r| bits | (1 << r.code()));
        let mut callers = CALLERS
            .into_iter()
            .filter(|r| (names | incoming) & (1 << r.code()) == 0);
        for c in CALLEES {
            if names & (1 << c.code()) == 0 {
                continue;
            }
            let first = f.blocks[0]
                .range
                .clone()
                .find(|&i| named(f.instructions[i].op) & (1 << c.code()) != 0);
            let incoming_copy = first.and_then(|i| {
                if let Op::Scalar(ScalarOp::Move(to, Operand::Register(from))) =
                    f.instructions[i].op
                    && to == c
                    && from.code() < 16
                    && exit_writes[usize::from(c.code())] == 1
                    && incoming & (1 << from.code()) != 0
                    && writes[usize::from(from.code())] == 0
                    && (writes[usize::from(c.code())] == 1 || leaf_names & (1 << from.code()) == 0)
                {
                    Some(from)
                } else {
                    None
                }
            });
            let stand_in = incoming_copy.or_else(|| callers.next())?;
            plan.rename[usize::from(c.code())] = stand_in;
            plan.copies.push((c, stand_in));
        }
        for (spare, keep) in plan.spare.iter_mut().zip(kept) {
            if keep {
                *spare = Some(callers.next()?);
            }
        }
        Some(plan)
    }
}
