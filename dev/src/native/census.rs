//! On-demand object-backend movement and loop census. See NOTICE.
//! Translated from `native/object/function_census.cpp`.
use super::{
    memory::Base,
    mir::{BlockName, Function, Op},
    scalar_mir::{Operand, ScalarOp},
};
use crate::lowir::model::Program;

fn frame(operand: Operand) -> bool {
    matches!(operand, Operand::Frame(_))
        || matches!(operand, Operand::Memory(memory) if memory.base == Base::Frame)
}
fn successors(function: &Function) -> Vec<Vec<usize>> {
    let count = function.blocks.len();
    let mut edges = vec![Vec::new(); count];
    for (block, outgoing) in edges.iter_mut().enumerate() {
        let mut falls_through = true;
        let mut add = |target| {
            if target < count && !outgoing.contains(&target) {
                outgoing.push(target);
            }
        };
        for index in function.blocks[block].range.clone() {
            let mut op = function.instructions[index].op;
            super::block_placement::labels(&mut op, |target| {
                add(target.index());
                target
            });
            if let Op::JumpTable(table) = op {
                for target in &function.jump_targets[table.range()] {
                    add(target.index());
                }
            }
            falls_through &= !super::block_placement::terminal(op)
                && !matches!(
                    op,
                    Op::Floating(
                        super::floating_mir::FloatingOp::Return(_)
                            | super::floating_mir::FloatingOp::ReturnPair(..)
                    )
                );
        }
        if falls_through && block + 1 < count {
            add(block + 1);
        }
    }
    edges
}
fn reaches_without(edges: &[Vec<usize>], target: usize, omitted: Option<usize>) -> bool {
    if edges.is_empty() || omitted == Some(0) {
        return false;
    }
    let mut seen = vec![false; edges.len()];
    let mut work = vec![0];
    seen[0] = true;
    while let Some(block) = work.pop() {
        if block == target {
            return true;
        }
        for &next in &edges[block] {
            if Some(next) != omitted && !seen[next] {
                seen[next] = true;
                work.push(next);
            }
        }
    }
    false
}
struct Loop {
    header: usize,
    members: Vec<bool>,
}
fn loops(edges: &[Vec<usize>]) -> Vec<Loop> {
    let mut predecessors = vec![Vec::new(); edges.len()];
    for (block, successors) in edges.iter().enumerate() {
        for &next in successors {
            predecessors[next].push(block);
        }
    }
    let mut reachable = vec![false; edges.len()];
    let mut work = Vec::new();
    if !edges.is_empty() {
        reachable[0] = true;
        work.push(0);
    }
    while let Some(block) = work.pop() {
        for &next in &edges[block] {
            if !reachable[next] {
                reachable[next] = true;
                work.push(next);
            }
        }
    }
    let mut loops: Vec<Loop> = Vec::new();
    for (latch, outgoing) in edges.iter().enumerate() {
        if !reachable[latch] {
            continue;
        }
        for &header in outgoing {
            if header != latch && reaches_without(edges, latch, Some(header)) {
                continue;
            }
            let index = loops
                .iter()
                .position(|l| l.header == header)
                .unwrap_or(loops.len());
            if index == loops.len() {
                loops.push(Loop {
                    header,
                    members: vec![false; edges.len()],
                });
            }
            let members = &mut loops[index].members;
            members[header] = true;
            let mut work = Vec::new();
            if !members[latch] {
                members[latch] = true;
                if latch != header {
                    work.push(latch);
                }
            }
            while let Some(block) = work.pop() {
                for &previous in &predecessors[block] {
                    if !members[previous] {
                        members[previous] = true;
                        work.push(previous);
                    }
                }
            }
        }
    }
    loops.sort_unstable_by_key(|l| l.header);
    loops
}
fn loop_counts(function: &Function, loop_: &Loop) -> [usize; 5] {
    let mut counts = [0; 5];
    for (block, &member) in loop_.members.iter().enumerate() {
        if !member {
            continue;
        }
        counts[0] += 1;
        for index in function.blocks[block].range.clone() {
            let mut op = function.instructions[index].op;
            if matches!(op, Op::Nop) {
                continue;
            }
            counts[1] += 1;
            counts[2] += usize::from(matches!(op, Op::Call(_) | Op::TailJump(_)));
            counts[3] += usize::from(
                matches!(op, Op::Exception(_) | Op::HostEh(_))
                    || matches!(op, Op::Call(call) | Op::TailJump(call) if call.landing.is_some()),
            );
            if let Op::HostEh(effect) = op {
                effect.visit_slots(&function.eh, |_| counts[4] += 1);
            } else {
                super::frame_layout::operands(&mut op, |operand, _| {
                    counts[4] += usize::from(frame(*operand));
                });
            }
        }
    }
    counts
}
fn append_loops(program: &Program, function: &Function, symbol: &str, lines: &mut Vec<String>) {
    let edges = successors(function);
    let loops = loops(&edges);
    for (index, loop_) in loops.iter().enumerate() {
        let [blocks, instructions, calls, eh, frame_operands] = loop_counts(function, loop_);
        let depth = loops
            .iter()
            .enumerate()
            .filter(|&(outer, l)| {
                outer != index
                    && loop_
                        .members
                        .iter()
                        .zip(&l.members)
                        .all(|(&member, &outside)| !member || outside)
                    && loop_
                        .members
                        .iter()
                        .zip(&l.members)
                        .any(|(&member, &outside)| outside && !member)
            })
            .count();
        let name = match function.blocks[loop_.header].name {
            BlockName::Named(name) if !program.names.get(name).is_empty() => {
                program.names.get(name).to_owned()
            }
            _ => format!("block_{}", loop_.header),
        };
        lines.push(format!("loop_census symbol={symbol} header_label={name} header={} blocks={blocks} mir={instructions} calls={calls} eh={eh} depth={depth} frame_operands={frame_operands} frame_bindings={} callee_saved={}",loop_.header,function.bindings.len(),function.preserve.count_ones()));
    }
}
pub(super) fn append(program: &Program, function: &Function, lines: &mut Vec<String>) {
    let [mut loads, mut stores, mut copies] = [0; 3];
    for block in &function.blocks {
        for index in block.range.clone() {
            match function.instructions[index].op {
                Op::Scalar(ScalarOp::Load(_, _, source)) if frame(source) => loads += 1,
                Op::Scalar(ScalarOp::Store(destination, _, _)) if frame(destination) => stores += 1,
                Op::Scalar(ScalarOp::Move(_, destination, Operand::Register(source)))
                    if destination != source =>
                {
                    copies += 1;
                }
                Op::FloatCopy(copy) => {
                    if copy.destination.register().is_some() && frame(copy.source) {
                        loads += 1;
                    } else if frame(copy.destination) {
                        stores += 1;
                    } else if let (Some(destination), Some(source)) =
                        (copy.destination.register(), copy.source.register())
                        && destination != source
                    {
                        copies += 1;
                    }
                }
                _ => {}
            }
        }
    }
    let symbol = program.names.get(program.symbol(function.symbol).name);
    let work = function.allocation_work;
    let instructions: usize = function.blocks.iter().map(|b| b.range.len()).sum();
    lines.push(format!("function_census symbol={symbol} mir={instructions} mov_loads={loads} mov_stores={stores} mov_copies={copies} planned={} grants={} releases={} spills={} frame_homes={} grant_busy={} grant_busy_parameters={} grant_busy_values={} defined_in_plan={} defined_frame={} defined_other_register={}",function.global_registers,work.grants,work.releases,function.spills,function.homes,work.busy,work.busy_parameters,work.busy_values,work.defined_in_plan,work.defined_frame,work.defined_other_register));
    append_loops(program, function, symbol, lines);
}
