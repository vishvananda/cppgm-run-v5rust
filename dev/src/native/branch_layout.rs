//! Optimized MIR jump threading, loop tests and explicit fallthrough layout. See NOTICE.
use super::{
    block_placement::{self, Flow},
    mir::{BlockId, Function, Op},
    object_compile::ObjectModel,
};
use crate::lowir::Error;
#[derive(Clone, Copy)]
enum End {
    Unknown,
    Walking,
    Cycle,
    Block(usize),
}
fn jump_only(f: &Function, b: usize) -> Option<usize> {
    let range = &f.blocks[b].range;
    if b != 0
        && range.len() == 1
        && let Op::Jump(t) = f.instructions[range.start].op
    {
        Some(t.index())
    } else {
        None
    }
}
fn rotate(f: &mut Function) -> usize {
    let mut count = 0;
    for b in 0..f.blocks.len() {
        let range = f.blocks[b].range.clone();
        if range.is_empty() || f.blocks[b].landing {
            continue;
        }
        if let Op::Jump(t) = f.instructions[range.end - 1].op
            && t.index() < b
            && !f.blocks[t.index()].landing
            && t.index() != 0
            && block_placement::copyable(f, t.index(), 12, true)
        {
            block_placement::duplicate_test(f, b, t.index());
            count += 1;
        }
    }
    count
}
fn reach_table(targets: &[BlockId], live: &mut [bool], work: &mut Vec<usize>) {
    for &id in targets {
        if !live[id.index()] {
            live[id.index()] = true;
            work.push(id.index());
        }
    }
}
pub(super) fn layout(
    f: &mut Function,
    program: &crate::lowir::model::Program,
) -> Result<(), Error> {
    let rotated = rotate(f);
    f.rotated_loops += rotated;
    super::decided_tests::thread(f, program);
    if let Some(flow) = Flow::new(f) {
        flow.place(f)?;
    }
    if super::shrink_wrap::run(f)? || rotated != 0 {
        super::home_forwarding::forward(f);
    }
    if rotated != 0 {
        super::flag_cleanup::drop_tests(f);
    }
    let n = f.blocks.len();
    let bare = f.preserve == 0 && f.stack_size == 0;
    for b in 0..n {
        let range = f.blocks[b].range.clone();
        if range.is_empty() {
            continue;
        }
        let last = range.end - 1;
        let Op::Jump(mut t) = f.instructions[last].op else {
            continue;
        };
        for _ in 0..4 {
            if let Some(next) = jump_only(f, t.index()) {
                t = BlockId(u32::try_from(next).map_err(|_| Error::Limit)?);
            } else {
                break;
            }
        }
        if t.index() != b
            && t.index() < n
            && f.blocks[t.index()].range.len() == 1
            && (matches!(
                f.instructions[f.blocks[t.index()].range.start].op,
                Op::FramelessReturn(_)
            ) || bare
                && matches!(
                    f.instructions[f.blocks[t.index()].range.start].op,
                    Op::Return(_)
                ))
        {
            f.instructions[last] = f.instructions[f.blocks[t.index()].range.start];
        }
    }
    let mut live = vec![f.object_model != ObjectModel::Host; n];
    if f.object_model == ObjectModel::Host {
        let mut work = vec![0];
        live[0] = true;
        for (b, is_live) in live.iter_mut().enumerate() {
            if f.blocks[b].landing && !*is_live {
                *is_live = true;
                work.push(b);
            }
        }
        for landing in &f.eh.landings {
            for b in [landing.pad, landing.select].into_iter().flatten() {
                if !live[b.index()] {
                    live[b.index()] = true;
                    work.push(b.index());
                }
            }
        }
        while let Some(b) = work.pop() {
            let mut last = Op::Nop;
            for i in f.blocks[b].range.clone() {
                let mut op = f.instructions[i].op;
                if matches!(op, Op::Nop) {
                    continue;
                }
                if let Op::JumpTable(table) = op {
                    reach_table(&f.jump_targets[table.range()], &mut live, &mut work);
                }
                block_placement::labels(&mut op, |id| {
                    if !live[id.index()] {
                        live[id.index()] = true;
                        work.push(id.index());
                    }
                    id
                });
                last = op;
            }
            if !block_placement::terminal(last) && b + 1 < n && !live[b + 1] {
                live[b + 1] = true;
                work.push(b + 1);
            }
        }
    }
    let mut ends = vec![End::Unknown; n];
    let mut chain = Vec::new();
    for b in 0..n {
        let mut t = b;
        chain.clear();
        while matches!(ends[t], End::Unknown)
            && let Some(next) = jump_only(f, t)
        {
            ends[t] = End::Walking;
            chain.push(t);
            t = next;
        }
        let end = match ends[t] {
            End::Unknown => End::Block(t),
            End::Walking => End::Cycle,
            other => other,
        };
        if matches!(ends[t], End::Unknown) {
            ends[t] = End::Block(t);
        }
        for &c in &chain {
            ends[c] = end;
        }
    }
    let mut map = vec![None; n];
    let mut kept = Vec::new();
    for b in 0..n {
        if matches!(ends[b], End::Cycle) {
            ends[b] = End::Block(b);
        }
        if matches!(ends[b],End::Block(t) if t==b) && live[b] {
            map[b] = Some(BlockId(
                u32::try_from(kept.len()).map_err(|_| Error::Limit)?,
            ));
            kept.push(b);
        }
    }
    for b in 0..n {
        if let End::Block(t) = ends[b]
            && b != t
        {
            map[b] = map[t];
        }
    }
    let frequency = Flow::new(f).map(|flow| flow.frequencies);
    for target in &mut f.jump_targets {
        *target = map[target.index()].unwrap_or(*target);
    }
    for landing in &mut f.eh.landings {
        landing.pad = landing.pad.and_then(|b| map[b.index()]);
        landing.select = landing.select.and_then(|b| map[b.index()]);
    }
    for (index, &b) in kept.iter().enumerate() {
        let range = f.blocks[b].range.clone();
        let mut colder = false;
        if range.len() >= 2
            && let (Op::Branch(_, a), Op::Jump(j)) = (
                f.instructions[range.end - 2].op,
                f.instructions[range.end - 1].op,
            )
            && let Some(freq) = &frequency
        {
            colder = freq[a.index()] < freq[j.index()];
        }
        for i in range.clone() {
            block_placement::labels(&mut f.instructions[i].op, |id| {
                map[id.index()].unwrap_or(id)
            });
        }
        if index + 1 >= kept.len() {
            continue;
        }
        let next = BlockId(u32::try_from(index + 1).map_err(|_| Error::Limit)?);
        let end = fallthrough(f, range.clone(), next, colder);
        f.layout_branches += range.end - end;
        f.blocks[b].range.end = end;
    }
    let mut blocks = std::mem::take(&mut f.blocks)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    for b in kept {
        f.blocks.push(blocks[b].take().ok_or(Error::Limit)?);
    }
    f.threaded_blocks += n - f.blocks.len();
    f.branches_laid_out = super::mir::BranchLayout::Final;
    Ok(())
}

fn fallthrough(
    f: &mut Function,
    range: std::ops::Range<usize>,
    next: BlockId,
    colder: bool,
) -> usize {
    let mut end = range.end;
    if end == range.start {
        return end;
    }
    match f.instructions[end - 1].op {
        Op::Jump(t) if t == next => end -= 1,
        Op::Jump(t) if end - range.start >= 2 => {
            if let Op::Branch(c, to) = f.instructions[end - 2].op {
                if to == next {
                    f.instructions[end - 2].op = Op::Branch(c.invert(), t);
                    end -= 1;
                } else if colder {
                    f.instructions[end - 2].op = Op::Branch(c.invert(), t);
                    f.instructions[end - 1].op = Op::Jump(to);
                }
            }
        }
        _ => {}
    }
    while end > range.start && matches!(f.instructions[end-1].op,Op::Branch(_,t) if t==next) {
        end -= 1;
    }
    end
}
