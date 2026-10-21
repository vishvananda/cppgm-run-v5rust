//! Stable parallel moves into the bounded scalar ABI register set. See NOTICE.
use super::{
    memory::Base,
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    x86::Register as R,
};
use crate::lowir::{Error, vocabulary::Type};
#[derive(Clone, Copy)]
pub(super) struct Transfer {
    pub(super) destination: Operand,
    pub(super) source: Operand,
    pub(super) ty: Type,
    pub(super) source_type: Option<Type>,
    pub(super) address: bool,
}
impl Transfer {
    fn emit(self, selection: &mut Selection<'_>) {
        if let Operand::Register(destination) = self.destination {
            if self.address {
                selection.emit(ScalarOp::Lea(destination, self.source));
            } else {
                selection.load(
                    destination,
                    self.source,
                    self.source_type.unwrap_or(self.ty),
                );
            }
        } else {
            selection.store_transfer(
                self.destination,
                self.ty,
                self.source,
                self.source_type,
                self.address,
            );
        }
    }
    fn redirect(&mut self, from: R, to: R) {
        match &mut self.source {
            Operand::Register(reg) if *reg == from => *reg = to,
            Operand::Memory(memory) => {
                if matches!(memory.base,Base::Register(reg) if reg==from) {
                    memory.base = Base::Register(to);
                }
                if memory.index == Some(from) {
                    memory.index = Some(to);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn emit(selection: &mut Selection<'_>, moves: &mut Vec<Transfer>) -> Result<(), Error> {
    moves.retain(|m| m.address || m.destination != m.source);
    if moves.is_empty() {
        return Ok(());
    }
    if moves.iter().all(|m| m.destination.register().is_some()) {
        return registers(selection, moves);
    }
    let mut workspace = std::mem::take(&mut selection.parallel);
    let result = workspace.emit(selection, moves);
    selection.parallel = workspace;
    result
}
fn registers(selection: &mut Selection<'_>, moves: &mut Vec<Transfer>) -> Result<(), Error> {
    let mut destinations = 0_u32;
    for m in moves.iter() {
        let reg = m.destination.register().ok_or(Error::Limit)?;
        let bit = 1 << reg.code();
        if destinations & bit != 0 {
            return Err(Error::Construction(
                "duplicate parallel register destination".into(),
            ));
        }
        destinations |= bit;
    }
    while !moves.is_empty() {
        let ready = moves.iter().enumerate().position(|(i, m)| {
            let reg = m.destination.register().map_or(0, |r| 1 << r.code());
            moves
                .iter()
                .enumerate()
                .all(|(j, n)| i == j || n.source.mask() & reg == 0)
        });
        if let Some(i) = ready {
            moves.remove(i).emit(selection);
            continue;
        }
        let first = moves[0];
        let reg = first.destination.register().ok_or(Error::Limit)?;
        let scratch = if reg.code() >= 16 { R::XMM14 } else { R::R10 };
        selection.load(scratch, first.destination, first.ty);
        for m in moves.iter_mut() {
            m.redirect(reg, scratch);
        }
    }
    Ok(())
}
#[derive(Default)]
pub(super) struct Workspace {
    frame_writers: Vec<(i32, usize)>,
    dependencies: Vec<[Option<usize>; 3]>,
    counts: Vec<usize>,
    first: Vec<usize>,
    readers: Vec<usize>,
    fill: Vec<usize>,
    done: Vec<bool>,
    ready: std::collections::BinaryHeap<std::cmp::Reverse<usize>>,
}
impl Workspace {
    fn release_dependency(&mut self, writer: usize) {
        if self.done[writer] {
            return;
        }
        self.counts[writer] -= 1;
        if self.counts[writer] == 0 {
            self.ready.push(std::cmp::Reverse(writer));
        }
    }

    fn emit(
        &mut self,
        selection: &mut Selection<'_>,
        moves: &mut Vec<Transfer>,
    ) -> Result<(), Error> {
        let count = moves.len();
        let mut regs = [None; 32];
        self.frame_writers.clear();
        for (index, m) in moves.iter().enumerate() {
            if let Some(reg) = m.destination.register() {
                regs[usize::from(reg.code())] = Some(index);
            } else if let Some(d) = m.destination.frame_offset() {
                self.frame_writers.push((d, index));
            } else {
                return Err(Error::Construction(
                    "parallel destination is not a home or register".into(),
                ));
            }
        }
        self.frame_writers.sort_unstable();
        if self
            .frame_writers
            .windows(2)
            .any(|pair| pair[1].0.abs_diff(pair[0].0) < 8)
        {
            return Err(Error::Construction("parallel homes overlap".into()));
        }
        self.dependencies.clear();
        self.counts.clear();
        self.counts.resize(count, 0);
        self.first.clear();
        self.first.resize(count + 1, 0);
        self.done.clear();
        self.done.resize(count, false);
        self.ready.clear();
        for (index, m) in moves.iter().enumerate() {
            let mut dependencies = [None; 3];
            let mut next = 0;
            for (reg, writer) in regs.iter().enumerate() {
                if m.source.mask() & (1 << reg) != 0
                    && let Some(writer) = *writer
                {
                    dependencies[next] = Some(writer);
                    next += 1;
                }
            }
            if !m.address
                && let Some(d) = m.source.frame_offset()
                && let Ok(at) = self.frame_writers.binary_search_by_key(&d, |&(d, _)| d)
            {
                let writer = self.frame_writers[at].1;
                dependencies[next] = Some(writer);
            }
            for writer in dependencies.into_iter().flatten() {
                if writer != index {
                    self.counts[writer] += 1;
                }
                self.first[writer + 1] += 1;
            }
            self.dependencies.push(dependencies);
        }
        for index in 0..count {
            self.first[index + 1] += self.first[index];
        }
        self.readers.clear();
        self.readers.resize(self.first[count], 0);
        self.fill.clear();
        self.fill.extend_from_slice(&self.first[..count]);
        for (index, deps) in self.dependencies.iter().enumerate() {
            for writer in deps.iter().flatten() {
                self.readers[self.fill[*writer]] = index;
                self.fill[*writer] += 1;
            }
        }
        for index in 0..count {
            if self.counts[index] == 0 {
                self.ready.push(std::cmp::Reverse(index));
            }
        }
        let mut pending = count;
        let mut first_pending = 0;
        while pending != 0 {
            if let Some(std::cmp::Reverse(index)) = self.ready.pop() {
                if self.done[index] {
                    continue;
                }
                moves[index].emit(selection);
                self.done[index] = true;
                pending -= 1;
                for writer in self.dependencies[index].into_iter().flatten() {
                    self.release_dependency(writer);
                }
                continue;
            }
            while self.done[first_pending] {
                first_pending += 1;
            }
            let destination = moves[first_pending].destination;
            let scratch = if destination.register().is_some_and(|r| r.code() >= 16)
                || matches!(moves[first_pending].ty, Type::F32 | Type::F64)
            {
                R::XMM14
            } else {
                R::R10
            };
            selection.load(scratch, destination, moves[first_pending].ty);
            for at in self.first[first_pending]..self.first[first_pending + 1] {
                let reader = self.readers[at];
                if self.done[reader] {
                    continue;
                }
                if let Some(reg) = destination.register() {
                    moves[reader].redirect(reg, scratch);
                } else {
                    moves[reader].source = Operand::Register(scratch);
                }
                for dependency in &mut self.dependencies[reader] {
                    if *dependency == Some(first_pending) {
                        *dependency = None;
                        self.counts[first_pending] -= usize::from(reader != first_pending);
                    }
                }
            }
            self.ready.push(std::cmp::Reverse(first_pending));
        }
        moves.clear();
        Ok(())
    }
}
