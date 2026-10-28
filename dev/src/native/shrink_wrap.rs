//! Delayed frame setup, spill sinking and frame-free returns. See NOTICE.
use super::{
    block_placement::{self, Flow},
    mir::{Block, BlockId, BlockName, Function, Instruction, Op},
    scalar_mir::{Operand, ScalarOp, bytes},
    shrink_region::{Graph, Plan, Spill, reload},
};
use crate::lowir::{Error, vocabulary::Type};
fn instruction(op: Op) -> Instruction {
    Instruction {
        op,
        volatile: false,
        location: None,
    }
}
fn spills(f: &Function) -> Vec<Spill> {
    let mut out = Vec::new();
    for i in f.blocks[0].range.clone() {
        let Op::Scalar(ScalarOp::Store(destination, ty, Operand::Register(source))) =
            f.instructions[i].op
        else {
            continue;
        };
        let Some(offset) = destination.frame_offset() else {
            continue;
        };
        if source.code() >= 16 || source == super::x86::Register::RSP {
            continue;
        }
        let width = bytes(ty);
        let mut alone = true;
        for j in f.blocks[0].range.clone() {
            if j == i {
                continue;
            }
            let mut op = f.instructions[j].op;
            let other_width = match op {
                Op::Scalar(
                    ScalarOp::Store(_, ty, Operand::Register(r)) | ScalarOp::Load(r, ty, _),
                ) if r.code() < 16 => i32::from(bytes(ty)),
                _ => 16,
            };
            super::frame_layout::operands(&mut op, |operand, _| {
                if let Some(m) = operand.memory()
                    && m.base == super::memory::Base::Frame
                {
                    alone &= m.index.is_none()
                        && (m.displacement.wrapping_add(other_width) <= offset
                            || offset.wrapping_add(i32::from(width)) <= m.displacement);
                }
            });
            if !alone {
                break;
            }
        }
        if alone {
            out.push(Spill {
                at: i,
                offset,
                width,
                source,
            });
        }
    }
    out
}
fn graph(f: &Function) -> Option<Graph> {
    let n = f.blocks.len();
    let spills = spills(f);
    let mut sunk = vec![false; f.instructions.len()];
    for s in &spills {
        sunk[s.at] = true;
    }
    let mut graph = Graph {
        needs: vec![false; n],
        framed: vec![false; n],
        successors: vec![Vec::new(); n],
        spills,
    };
    for (b, block) in f.blocks.iter().enumerate() {
        if block.range.is_empty()
            || !block_placement::terminal(f.instructions[block.range.end - 1].op)
        {
            return None;
        }
        for i in block.range.clone() {
            let op = f.instructions[i].op;
            graph.needs[b] |= !super::mir_registers::frameless(op)
                && !sunk[i]
                && reload(op, &graph.spills).is_none();
            match op {
                Op::Jump(t) | Op::Branch(_, t) => graph.successors[b].push(t.index()),
                Op::JumpTable(t) => {
                    graph.successors[b].extend(f.jump_targets[t.range()].iter().map(|t| t.index()));
                }
                Op::WideCompare(w) => {
                    if let super::wide_compare::Destination::Branch { yes, no } = w.destination {
                        graph.successors[b].extend([yes.index(), no.index()]);
                    }
                }
                _ => {}
            }
        }
    }
    graph.framed.clone_from(&graph.needs);
    let mut work: Vec<_> = (0..n).filter(|&b| graph.framed[b]).collect();
    while let Some(b) = work.pop() {
        for &s in &graph.successors[b] {
            if s < n && !graph.framed[s] {
                graph.framed[s] = true;
                work.push(s);
            }
        }
    }
    (!graph.framed[0]).then_some(graph)
}
fn append(
    f: &mut Function,
    name: BlockName,
    ops: impl IntoIterator<Item = Instruction>,
) -> Result<BlockId, Error> {
    let id = BlockId(u32::try_from(f.blocks.len()).map_err(|_| Error::Limit)?);
    let start = f.instructions.len();
    f.instructions.extend(ops);
    f.blocks.push(Block {
        name,
        range: start..f.instructions.len(),
        landing: false,
    });
    Ok(id)
}
fn apply(f: &mut Function, graph: &Graph, plan: &Plan) -> Result<(), Error> {
    let n = plan.region.len();
    let mut target = vec![None; n];
    for (b, &copied) in plan.copied.iter().enumerate() {
        if copied {
            let ops = f.instructions[f.blocks[b].range.clone()].to_vec();
            target[b] = Some(append(f, f.blocks[b].name, ops)?);
        }
    }
    let frameless_end = f.blocks.len();
    for &s in &plan.exits {
        let mut ops = vec![instruction(Op::Prologue)];
        ops.extend(plan.copies.iter().map(|&(to, from)| {
            instruction(Op::Scalar(ScalarOp::Move(
                Type::Void,
                to,
                Operand::Register(from),
            )))
        }));
        for (spill, &spare) in graph.spills.iter().zip(&plan.spare) {
            let mut store = f.instructions[spill.at];
            if let Some(r) = spare
                && let Op::Scalar(ScalarOp::Store(_, _, source)) = &mut store.op
            {
                *source = Operand::Register(r);
            }
            ops.push(store);
        }
        ops.push(instruction(Op::Jump(BlockId(
            u32::try_from(s).map_err(|_| Error::Limit)?,
        ))));
        target[s] = Some(append(f, BlockName::Switch, ops)?);
    }
    for (spill, &spare) in graph.spills.iter().zip(&plan.spare) {
        f.instructions[spill.at].op = spare.map_or(Op::Nop, |r| {
            Op::Scalar(ScalarOp::Move(
                Type::Void,
                r,
                Operand::Register(spill.source),
            ))
        });
    }
    for b in 0..frameless_end {
        if b < n && !plan.region[b] {
            continue;
        }
        for i in f.blocks[b].range.clone() {
            let mut op = f.instructions[i].op;
            if let Some(k) = reload(op, &graph.spills)
                && let Op::Scalar(ScalarOp::Load(to, ty, _)) = op
            {
                let from = plan.spare[k].ok_or(Error::Limit)?;
                op = Op::Scalar(if bytes(ty) == 8 {
                    ScalarOp::Move(Type::Void, to, Operand::Register(from))
                } else {
                    ScalarOp::ExtendMove(ty, to, from)
                });
            }
            match &mut op {
                Op::Jump(t) | Op::Branch(_, t) => {
                    if let Some(Some(to)) = target.get(t.index()) {
                        *t = *to;
                    }
                }
                Op::JumpTable(t) => {
                    for t in &mut f.jump_targets[t.range()] {
                        if let Some(Some(to)) = target.get(t.index()) {
                            *t = *to;
                        }
                    }
                }
                _ => {}
            }
            super::mir_registers::map(&mut op, |r| {
                if r.code() < 16 {
                    plan.rename[usize::from(r.code())]
                } else {
                    r
                }
            });
            if matches!(op, Op::Scalar(ScalarOp::Move(_, to, Operand::Register(from))) if to == from)
            {
                op = Op::Nop;
            }
            if let Op::Return(r) = op {
                op = Op::FramelessReturn(r);
            }
            f.instructions[i].op = op;
        }
    }
    order(f)?;
    f.frame_policy.shrink_wrapped = true;
    Ok(())
}
fn order(f: &mut Function) -> Result<(), Error> {
    if let Some(flow) = Flow::new(f) {
        flow.place(f)?;
    }
    let n = f.blocks.len();
    let mut part = vec![2; n];
    for (b, block) in f.blocks.iter().enumerate() {
        if matches!(f.instructions[block.range.start].op, Op::Prologue) {
            part[b] = 1;
        }
    }
    part[0] = 0;
    let mut work = vec![0];
    while let Some(b) = work.pop() {
        for i in f.blocks[b].range.clone() {
            if let Op::Jump(t) | Op::Branch(_, t) = f.instructions[i].op
                && part[t.index()] == 2
            {
                part[t.index()] = 0;
                work.push(t.index());
            }
        }
    }
    let mut hot = None;
    for (b, &kind) in part.iter().enumerate() {
        if kind == 1
            && let Op::Jump(t) = f.instructions[f.blocks[b].range.end - 1].op
            && hot.is_none_or(|(_, target)| t.index() < target)
        {
            hot = Some((b, t.index()));
        }
    }
    let (hot, target) = hot.ok_or(Error::Limit)?;
    let mut order = Vec::with_capacity(n);
    for k in 0..3 {
        if k == 2 {
            order.extend([hot, target]);
        }
        order.extend((0..n).filter(|&b| part[b] == k && b != hot && b != target));
    }
    block_placement::reorder(f, &order)
}
pub(super) fn run(f: &mut Function) -> Result<bool, Error> {
    if !f.frame_policy.omit_pointer
        || f.variadic
        || f.frame_policy.records_pushed
        || f.blocks.len() < 2
        || f.preserve == 0 && f.stack_size == 0
    {
        return Ok(false);
    }
    let Some(graph) = graph(f) else {
        return Ok(false);
    };
    let Some(plan) = graph.plan(f) else {
        return Ok(false);
    };
    apply(f, &graph, &plan)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::vocabulary::Type;
    use crate::native::{
        condition::Condition, mir::CallSite, object_compile::ObjectModel, x86::Register as R,
    };
    #[test]
    fn sunk_spill_reload_and_distinct_prologues_keep_one_exception_area() -> Result<(), Error> {
        let program = crate::lowir::test_program(
            b"function @f(%arg: i32) -> i32 { block ^entry: return i32 %arg }",
        )?;
        let symbol = program.publications()[0];
        let mut f = super::super::value_selection::select_for_object(
            &program,
            symbol,
            program
                .function(program.symbol(symbol))
                .ok_or(Error::Limit)?,
            0,
            ObjectModel::Host,
        )?;
        f.instructions.clear();
        f.blocks.clear();
        f.optimized = true;
        f.frame_policy.omit_pointer = true;
        f.stack_size = 32;
        let ops = [
            vec![
                Op::Scalar(ScalarOp::Store(
                    Operand::Frame(-8),
                    Type::I32,
                    Operand::Register(R::RDI),
                )),
                Op::Branch(Condition::Equal, BlockId(1)),
                Op::Jump(BlockId(2)),
            ],
            vec![
                Op::Scalar(ScalarOp::Load(R::RAX, Type::I32, Operand::Frame(-8))),
                Op::Return(Some(R::RAX)),
            ],
            vec![
                Op::Branch(Condition::Equal, BlockId(3)),
                Op::Jump(BlockId(4)),
            ],
            vec![
                Op::Call(CallSite {
                    target: Operand::Symbol(symbol),
                    arguments: 0,
                    stack: 0,
                    variadic: false,
                    no_unwind: true,
                    no_return: false,
                    prefix: super::super::mir::CallPrefix::None,
                    landing: None,
                }),
                Op::Return(Some(R::RAX)),
            ],
            vec![
                Op::Call(CallSite {
                    target: Operand::Symbol(symbol),
                    arguments: 0,
                    stack: 0,
                    variadic: false,
                    no_unwind: true,
                    no_return: false,
                    prefix: super::super::mir::CallPrefix::None,
                    landing: None,
                }),
                Op::Return(Some(R::RAX)),
            ],
        ];
        for ops in ops {
            let _ = append(&mut f, BlockName::Switch, ops.into_iter().map(instruction))?;
        }
        assert!(run(&mut f)?);
        assert!(f.frame_policy.shrink_wrapped);
        assert_eq!(
            f.instructions
                .iter()
                .filter(|i| matches!(i.op, Op::Prologue))
                .count(),
            2
        );
        assert!(
            f.instructions
                .iter()
                .any(|i| matches!(i.op, Op::Scalar(ScalarOp::ExtendMove(Type::I32, R::RAX, _))))
        );
        assert!(
            f.instructions
                .iter()
                .any(|i| matches!(i.op, Op::FramelessReturn(_)))
        );
        let mut image = super::super::image::Image::new(program.symbols().len());
        let mut frame = super::super::host_unwind::Frame::default();
        let _ = super::super::emission::emit_with_frame(&f, &mut image, None, Some(&mut frame))?;
        assert_eq!(frame.prologues.len(), 2);
        assert!(frame.prologues[0].start > 0);
        assert!(frame.prologues[1].start > frame.prologues[0].start);
        assert!(!frame.encode(false)?.is_empty());
        Ok(())
    }
}
