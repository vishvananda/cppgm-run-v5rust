//! Available frame-home holders and reload forwarding from `mir_cleanup.cpp`.
//! See NOTICE.
use super::{
    abi::CALLER_SAVED,
    condition_mir::FlagOp,
    frame_layout::operands,
    frame_lifetimes::{fixed_frame, successors},
    memory::Base,
    mir::{BindingKind, Function, Op},
    mir_cleanup::{SimpleWrite, simple_write},
    scalar_mir::{Operand, ScalarOp, bytes},
    x86::Register as R,
};
use crate::lowir::vocabulary::{Type, Unary};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Held {
    Any,
    Nothing,
    Home(i32, u8),
}
impl Held {
    fn home(offset: i32, count: u8) -> Self {
        // The frozen packed holder aliases a full-width home at INT32_MIN
        // with kHoldsNothing. Narrow holders at that offset remain distinct.
        if offset == i32::MIN && count >= 8 {
            Self::Nothing
        } else {
            Self::Home(offset, count)
        }
    }
    fn meet(self, other: Self) -> Self {
        if self == Self::Any {
            other
        } else if other == Self::Any || self == other {
            self
        } else {
            Self::Nothing
        }
    }
}
struct Graph {
    pred_first: Vec<usize>,
    preds: Vec<usize>,
    cut: Vec<bool>,
    order: Vec<usize>,
}
impl Graph {
    fn new(function: &Function) -> Self {
        let n = function.blocks.len();
        let mut first = vec![0; n + 1];
        let mut succ = Vec::new();
        let mut cut = function
            .blocks
            .iter()
            .map(|b| b.landing)
            .collect::<Vec<_>>();
        cut[0] = true;
        let mut targets = Vec::new();
        for b in 0..n {
            targets.clear();
            successors(function, b, |to| {
                if to < n {
                    targets.push(to);
                }
            });
            for inst in &function.instructions[function.blocks[b].range.clone()] {
                if let Op::Exception(super::exception::ExceptionOp::Push { target, .. }) = inst.op
                    && target.index() < n
                {
                    targets.push(target.index());
                    cut[target.index()] = true;
                }
            }
            targets.sort_unstable();
            targets.dedup();
            succ.extend_from_slice(&targets);
            first[b + 1] = succ.len();
        }
        let mut pred_first = vec![0; n + 1];
        for &to in &succ {
            pred_first[to + 1] += 1;
        }
        for b in 0..n {
            pred_first[b + 1] += pred_first[b];
        }
        let mut fill = pred_first[..n].to_vec();
        let mut preds = vec![0; succ.len()];
        for b in 0..n {
            for &to in &succ[first[b]..first[b + 1]] {
                preds[fill[to]] = b;
                fill[to] += 1;
            }
        }
        let mut seen = vec![false; n];
        let mut next = first[..n].to_vec();
        let mut stack = vec![0];
        let mut order = Vec::with_capacity(n);
        seen[0] = true;
        while let Some(&b) = stack.last() {
            if next[b] == first[b + 1] {
                order.push(b);
                stack.pop();
            } else {
                let to = succ[next[b]];
                next[b] += 1;
                if !seen[to] {
                    seen[to] = true;
                    stack.push(to);
                }
            }
        }
        order.reverse();
        if !function.eh.landings.is_empty() {
            for b in 0..n {
                if !seen[b] {
                    for &to in &succ[first[b]..first[b + 1]] {
                        cut[to] |= seen[to];
                    }
                }
            }
        }
        Self {
            pred_first,
            preds,
            cut,
            order,
        }
    }
    fn meet(&self, b: usize, out: &[[Held; 16]]) -> [Held; 16] {
        let predecessors = &self.preds[self.pred_first[b]..self.pred_first[b + 1]];
        let mut holds = [if self.cut[b] || predecessors.is_empty() {
            Held::Nothing
        } else {
            Held::Any
        }; 16];
        for &pred in predecessors {
            for (h, &o) in holds.iter_mut().zip(&out[pred]) {
                *h = h.meet(o);
            }
        }
        holds.map(|h| if h == Held::Any { Held::Nothing } else { h })
    }
}
fn forget(holds: &mut [Held; 16], lo: i32, hi: i32) {
    for h in holds {
        if let Held::Home(offset, _) = *h
            && offset < hi
            && lo < offset.wrapping_add(8)
        {
            *h = Held::Nothing;
        }
    }
}
fn holder(holds: &[Held; 16], offset: i32, need: u8) -> Option<R> {
    const REGISTERS: [R; 16] = [
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
    ];
    holds.iter().enumerate().find_map(|(r, &h)| {
        matches!(h, Held::Home(at, bytes) if at == offset && bytes >= need).then_some(REGISTERS[r])
    })
}
fn writer(op: Op) -> Option<R> {
    match simple_write(op) {
        Some(SimpleWrite::Register(r)) => Some(r),
        _ => None,
    }
}
// Only simple instructions reach this visitor. Source memory always reads its
// base/index; a two-address or partial register write also reads its destination.
fn reads(op: &mut Op, mut visit: impl FnMut(&mut Operand)) {
    let mut reg = |r: &mut R| {
        let mut value = Operand::Register(*r);
        visit(&mut value);
        if let Operand::Register(changed) = value {
            *r = changed;
        }
    };
    match op {
        Op::Scalar(
            ScalarOp::Move(_, _, src)
            | ScalarOp::Load(_, _, src)
            | ScalarOp::Lea(_, src)
            | ScalarOp::Lea32(_, src),
        ) => visit(src),
        Op::Scalar(
            ScalarOp::Binary(_, _, r, _)
            | ScalarOp::Shift(_, _, r, _)
            | ScalarOp::Rotate(_, _, r, _)
            | ScalarOp::Unary(Unary::Neg | Unary::BitNot, _, r)
            | ScalarOp::Extend(_, r)
            | ScalarOp::Movzx(r)
            | ScalarOp::SetEqual(r)
            | ScalarOp::Condition(
                FlagOp::Set(_, r) | FlagOp::Move(_, r, _) | FlagOp::BitTest(_, r, _),
            ),
        ) => {
            reg(r);
            operands(op, |v, _| visit(v));
        }
        Op::Scalar(ScalarOp::ExtendMove(_, _, src) | ScalarOp::Condition(FlagOp::Test(_, src))) => {
            reg(src);
        }
        _ => {
            operands(op, |v, _| visit(v));
        }
    }
}
fn reads_register(op: Op, register: R) -> bool {
    let mut copied = op;
    let mut found = false;
    reads(&mut copied, |operand| {
        found |= operand.mask() & (1 << register.code()) != 0;
    });
    found
}
fn reads_destination(op: Op) -> bool {
    !matches!(
        op,
        Op::Move(..)
            | Op::Scalar(
                ScalarOp::Move(..)
                    | ScalarOp::Load(..)
                    | ScalarOp::Lea(..)
                    | ScalarOp::Lea32(..)
                    | ScalarOp::ExtendMove(..)
            )
    )
}
fn forward_scratch(function: &mut Function, at: usize, end: usize, held: R) -> bool {
    let Some(scratch) = writer(function.instructions[at].op) else {
        return false;
    };
    let mut dirty = false;
    let mut last = at;
    for i in at + 1..end {
        let op = function.instructions[i].op;
        if matches!(op, Op::Branch(..)) {
            continue;
        }
        if simple_write(op).is_none() {
            if !matches!(op, Op::Jump(_)) {
                return false;
            }
            break;
        }
        let w = writer(op);
        if reads_register(op, scratch) {
            if dirty || w == Some(scratch) && reads_destination(op) {
                return false;
            }
            last = i;
        }
        if w == Some(scratch) {
            break;
        }
        if w == Some(held) {
            dirty = true;
        }
    }
    for inst in &mut function.instructions[at + 1..=last] {
        reads(&mut inst.op, |operand| match operand {
            Operand::Register(r) if *r == scratch => *r = held,
            Operand::Memory(m) => {
                if m.base == Base::Register(scratch) {
                    m.base = Base::Register(held);
                }
                if m.index == Some(scratch) {
                    m.index = Some(held);
                }
            }
            _ => {}
        });
    }
    function.instructions[at].op = Op::Nop;
    true
}
struct Forwarding {
    homes: Vec<i32>,
}
impl Forwarding {
    fn drop_slots(&self, holds: &mut [Held; 16]) {
        for h in holds {
            if let Held::Home(offset, _) = *h
                && self.homes.binary_search(&offset).is_err()
            {
                *h = Held::Nothing;
            }
        }
    }
    fn caller_saved(holds: &mut [Held; 16]) {
        for (r, h) in holds.iter_mut().enumerate() {
            if (CALLER_SAVED | (1 << R::R10.code()) | (1 << R::R11.code())) & (1 << r) != 0 {
                *h = Held::Nothing;
            }
        }
    }
    fn transfer(&self, function: &mut Function, b: usize, holds: &mut [Held; 16], rewrite: bool) {
        let range = function.blocks[b].range.clone();
        let mut exits = [Held::Any; 16];
        for i in range.clone() {
            let op = function.instructions[i].op;
            if matches!(op, Op::Branch(..)) {
                for (e, &h) in exits.iter_mut().zip(holds.iter()) {
                    *e = e.meet(h);
                }
                continue;
            }
            if matches!(op, Op::Jump(_)) {
                continue;
            }
            if matches!(op, Op::JumpTable(_)) {
                holds[usize::from(R::R10.code())] = Held::Nothing;
                holds[usize::from(R::R11.code())] = Held::Nothing;
                for (e, &h) in exits.iter_mut().zip(holds.iter()) {
                    *e = e.meet(h);
                }
                continue;
            }
            if matches!(op, Op::Call(_)) {
                Self::caller_saved(holds);
                self.drop_slots(holds);
                continue;
            }
            if let Op::Bulk(bulk) = op
                && (bulk.source.is_some()
                    || bulk.bytes() == 16
                    || function.optimized && bulk.bytes() > 16 && bulk.bytes() <= 128)
            {
                if bulk.source.is_some() {
                    Self::caller_saved(holds);
                }
                if let Some(m) = bulk.destination.memory().filter(|m| m.base == Base::Frame) {
                    forget(
                        holds,
                        m.displacement,
                        m.displacement.wrapping_add(bulk.bytes().cast_signed()),
                    );
                } else {
                    self.drop_slots(holds);
                }
                continue;
            }
            let Some(write) = simple_write(op) else {
                holds.fill(Held::Nothing);
                continue;
            };
            let w = match write {
                SimpleWrite::None => None,
                SimpleWrite::Register(r) => Some(r),
            };
            if function.instructions[i].volatile {
                let mut copied = op;
                operands(&mut copied, |value, _| {
                    if let Some(offset) = fixed_frame(*value) {
                        forget(holds, offset, offset.wrapping_add(8));
                    }
                });
                if let Some(w) = w {
                    holds[usize::from(w.code())] = Held::Nothing;
                }
                continue;
            }
            match op {
                Op::Scalar(ScalarOp::Load(dst, ty, src)) if fixed_frame(src).is_some() => {
                    let offset = fixed_frame(src).unwrap_or(0);
                    let wide = matches!(ty, Type::I64 | Type::Ptr);
                    let count = if wide { 8 } else { bytes(ty) };
                    if !wide && (count >= 8 || offset & 7 != 0) {
                        holds[usize::from(dst.code())] = Held::Nothing;
                        continue;
                    }
                    let source = holder(holds, offset, count);
                    holds[usize::from(dst.code())] = if dst == R::R11 || offset & 7 != 0 {
                        Held::Nothing
                    } else {
                        Held::home(offset, count)
                    };
                    if !rewrite {
                        continue;
                    }
                    let Some(source) = source else { continue };
                    let extended = if ty == Type::I1 { Type::U8 } else { ty };
                    function.instructions[i].op = if !wide {
                        Op::Scalar(ScalarOp::ExtendMove(extended, dst, source))
                    } else if dst == source {
                        Op::Nop
                    } else {
                        Op::Scalar(ScalarOp::Move(Type::Void, dst, Operand::Register(source)))
                    };
                    if wide
                        && dst != source
                        && matches!(dst, R::R10 | R::R11)
                        && source != R::RSP
                        && forward_scratch(function, i, range.end, source)
                    {
                        holds[usize::from(dst.code())] = Held::Nothing;
                    }
                    continue;
                }
                Op::Scalar(ScalarOp::Condition(
                    flag @ (FlagOp::Compare(ty, a, c) | FlagOp::MaskTest(ty, a, c)),
                )) if rewrite => {
                    let mut values = [a, c];
                    for k in 0..2 {
                        if let Some(offset) =
                            fixed_frame(values[k]).filter(|o| o.trailing_zeros() >= 3)
                            && ty != Type::Void
                            && !values[1 - k].is_memory()
                            && let Some(held) =
                                holder(holds, offset, bytes(ty)).filter(|&r| r != R::RSP)
                        {
                            values[k] = Operand::Register(held);
                        }
                    }
                    function.instructions[i].op = Op::Scalar(ScalarOp::Condition(match flag {
                        FlagOp::Compare(..) => FlagOp::Compare(ty, values[0], values[1]),
                        _ => FlagOp::MaskTest(ty, values[0], values[1]),
                    }));
                }
                Op::Scalar(ScalarOp::CompareZero(ty, src)) if rewrite => {
                    if let Some(offset) = fixed_frame(src).filter(|o| o.trailing_zeros() >= 3)
                        && let Some(held) =
                            holder(holds, offset, bytes(ty)).filter(|&r| r != R::RSP)
                    {
                        function.instructions[i].op =
                            Op::Scalar(ScalarOp::CompareZero(ty, Operand::Register(held)));
                    }
                }
                Op::Scalar(ScalarOp::Condition(FlagOp::CompareFloatLiteral(kind, src, bits)))
                    if rewrite =>
                {
                    if let Some(offset) = fixed_frame(src).filter(|o| o.trailing_zeros() >= 3)
                        && let Some(held) =
                            holder(holds, offset, bytes(kind.ty())).filter(|&r| r != R::RSP)
                    {
                        function.instructions[i].op = Op::Scalar(ScalarOp::Condition(
                            FlagOp::CompareFloatLiteral(kind, Operand::Register(held), bits),
                        ));
                    }
                }
                Op::Scalar(ScalarOp::MemoryBinary(_, ty, dst, _)) => {
                    if let Some(offset) = fixed_frame(dst) {
                        forget(holds, offset, offset.wrapping_add(i32::from(bytes(ty))));
                    } else {
                        self.drop_slots(holds);
                    }
                    continue;
                }
                Op::Scalar(ScalarOp::Store(dst, ty, src)) => {
                    if let Some(offset) = fixed_frame(dst) {
                        let count = bytes(ty);
                        forget(holds, offset, offset.wrapping_add(i32::from(count)));
                        if let Some(r) = src.register().filter(|&r| r != R::R11)
                            && offset.trailing_zeros() >= 3
                            && matches!(count, 1 | 2 | 4 | 8)
                        {
                            holds[usize::from(r.code())] = Held::home(offset, count);
                        }
                    } else {
                        self.drop_slots(holds);
                    }
                    continue;
                }
                _ => {}
            }
            if let Some(w) = w {
                holds[usize::from(w.code())] = Held::Nothing;
            }
        }
        for (e, &h) in exits.iter_mut().zip(holds.iter()) {
            *e = e.meet(h);
        }
        *holds = exits;
        holds[usize::from(R::R10.code())] = Held::Nothing;
        holds[usize::from(R::R11.code())] = Held::Nothing;
    }
}
pub(super) fn forward(function: &mut Function) {
    if function.blocks.is_empty() || function.stack_size == 0 {
        return;
    }
    let mut homes = function
        .bindings
        .iter()
        .filter(|b| {
            b.kind != BindingKind::Slot
                && !matches!(b.ty, Type::Object { .. } | Type::F80 | Type::I128)
        })
        .map(|b| b.offset)
        .collect::<Vec<_>>();
    homes.sort_unstable();
    let forwarding = Forwarding { homes };
    let graph = Graph::new(function);
    let n = function.blocks.len();
    let mut output = vec![[Held::Any; 16]; n];
    let mut input = vec![[Held::Nothing; 16]; n];
    let mut evaluated = vec![false; n];
    loop {
        let mut changed = false;
        for &b in &graph.order {
            let mut holds = graph.meet(b, &output);
            if evaluated[b] && input[b] == holds {
                continue;
            }
            evaluated[b] = true;
            input[b] = holds;
            forwarding.transfer(function, b, &mut holds, false);
            changed |= output[b] != holds;
            output[b] = holds;
        }
        if !changed {
            break;
        }
    }
    for &b in &graph.order {
        forwarding.transfer(function, b, &mut input[b], true);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        condition::Condition,
        mir::{Block, BlockId, Instruction},
    };
    use super::*;
    use crate::lowir::Error;

    fn sample(ops: &[Op], ranges: &[std::ops::Range<usize>]) -> Result<Function, Error> {
        let p = crate::lowir::test_program(b"function @f() -> void {block ^entry: return void}")?;
        let symbol = p.publications()[0];
        let mut f = super::super::value_selection::select(
            &p,
            symbol,
            p.function(p.symbol(symbol)).ok_or(Error::Limit)?,
            0,
        )?;
        let name = f.blocks[0].name;
        f.instructions = ops
            .iter()
            .map(|&op| Instruction {
                op,
                volatile: false,
                location: None,
            })
            .collect();
        f.blocks = ranges
            .iter()
            .map(|range| Block {
                name,
                range: range.clone(),
                landing: false,
            })
            .collect();
        f.stack_size = 16;
        Ok(f)
    }
    #[test]
    fn packed_sentinel_alias_only_forgets_full_width_homes() -> Result<(), Error> {
        for (ty, forwarded) in [(Type::I64, false), (Type::U32, true)] {
            let ops = [
                Op::Scalar(ScalarOp::Store(
                    Operand::Frame(i32::MIN),
                    ty,
                    Operand::Register(R::RDI),
                )),
                Op::Scalar(ScalarOp::Load(R::RAX, ty, Operand::Frame(i32::MIN))),
                Op::Return(Some(R::RAX)),
            ];
            let mut f = sample(&ops, std::slice::from_ref(&(0..ops.len())))?;
            forward(&mut f);
            assert_eq!(
                matches!(
                    f.instructions[1].op,
                    Op::Scalar(ScalarOp::ExtendMove(Type::U32, R::RAX, R::RDI))
                ),
                forwarded
            );
            assert_eq!(
                matches!(f.instructions[1].op, Op::Scalar(ScalarOp::Load(..))),
                !forwarded
            );
        }
        Ok(())
    }
    #[test]
    fn floating_literal_bits_do_not_invalidate_home_holders() -> Result<(), Error> {
        let kind = crate::lowir::literals::FloatKind::F64;
        let bits = u128::from(1.0_f64.to_bits());
        let ops = [
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I64,
                Operand::Register(R::RDI),
            )),
            Op::Scalar(ScalarOp::Condition(FlagOp::CompareFloatLiteral(
                kind,
                Operand::Frame(-8),
                bits,
            ))),
            Op::Jump(BlockId(1)),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
            Op::Return(Some(R::RAX)),
        ];
        let mut function = sample(&ops, &[0..3, 3..5])?;
        forward(&mut function);
        assert!(matches!(
            function.instructions[1].op,
            Op::Scalar(ScalarOp::Condition(FlagOp::CompareFloatLiteral(
                _, Operand::Register(R::RDI), value
            ))) if value == bits
        ));
        assert!(matches!(
            function.instructions[3].op,
            Op::Scalar(ScalarOp::Move(_, R::RAX, Operand::Register(R::RDI)))
        ));
        Ok(())
    }
    #[test]
    fn scratch_forwarding_rewrites_both_bit_test_readers() -> Result<(), Error> {
        let ops = [
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I64,
                Operand::Register(R::RDI),
            )),
            Op::Scalar(ScalarOp::Load(R::R10, Type::I64, Operand::Frame(-8))),
            Op::Scalar(ScalarOp::Condition(FlagOp::BitTest(
                Type::I64,
                R::R10,
                Operand::Register(R::R10),
            ))),
            Op::Move(R::R10, 0),
            Op::Return(None),
        ];
        let mut f = sample(&ops, std::slice::from_ref(&(0..5)))?;
        f.bindings.push(super::super::mir::Binding {
            name: None,
            ty: Type::I64,
            offset: -8,
            kind: BindingKind::Temp,
        });
        forward(&mut f);
        assert!(matches!(f.instructions[1].op, Op::Nop));
        assert!(matches!(
            f.instructions[2].op,
            Op::Scalar(ScalarOp::Condition(FlagOp::BitTest(
                Type::I64,
                R::RDI,
                Operand::Register(R::RDI)
            )))
        ));
        Ok(())
    }
    #[test]
    fn calls_forget_scratch_holders_but_keep_callee_saved_home_holders() -> Result<(), Error> {
        for register in [R::R10, R::RBX] {
            let mut function = sample(
                &[
                    Op::Scalar(ScalarOp::Store(
                        Operand::Frame(-8),
                        Type::I64,
                        Operand::Register(register),
                    )),
                    Op::Nop,
                    Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
                    Op::Return(Some(R::RAX)),
                ],
                std::slice::from_ref(&(0..4)),
            )?;
            function.bindings.push(super::super::mir::Binding {
                name: None,
                ty: Type::I64,
                offset: -8,
                kind: BindingKind::Temp,
            });
            function.instructions[1].op = Op::Call(super::super::mir::CallSite {
                target: Operand::Symbol(function.symbol),
                arguments: 0,
                stack: 0,
                variadic: false,
                no_unwind: true,
                no_return: false,
                prefix: super::super::mir::CallPrefix::None,
                landing: None,
            });
            forward(&mut function);
            assert_eq!(
                matches!(function.instructions[2].op, Op::Scalar(ScalarOp::Load(..))),
                register == R::R10
            );
        }
        Ok(())
    }
    #[test]
    fn a_join_forwards_only_the_holder_every_edge_preserves() -> Result<(), Error> {
        let ops = [
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I64,
                Operand::Register(R::RDI),
            )),
            Op::Branch(Condition::Equal, BlockId(2)),
            Op::Jump(BlockId(1)),
            Op::Move(R::RDI, 13),
            Op::Jump(BlockId(2)),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
            Op::Return(None),
        ];
        let mut f = sample(&ops, &[0..3, 3..5, 5..7])?;
        forward(&mut f);
        assert!(matches!(
            f.instructions[5].op,
            Op::Scalar(ScalarOp::Load(..))
        ));
        f.instructions[3].op = Op::Nop;
        forward(&mut f);
        assert!(matches!(
            f.instructions[5].op,
            Op::Scalar(ScalarOp::Move(_, R::RAX, Operand::Register(R::RDI)))
        ));
        Ok(())
    }
    #[test]
    fn a_narrow_holder_cannot_forward_a_wider_or_volatile_load() -> Result<(), Error> {
        let ops = [
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I16,
                Operand::Register(R::RDI),
            )),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I16, Operand::Frame(-8))),
            Op::Scalar(ScalarOp::Load(R::RCX, Type::I64, Operand::Frame(-8))),
            Op::Scalar(ScalarOp::Load(R::RDX, Type::I16, Operand::Frame(-8))),
            Op::Scalar(ScalarOp::Load(R::RSI, Type::I16, Operand::Frame(-8))),
            Op::Return(None),
        ];
        let mut f = sample(&ops, std::slice::from_ref(&(0..6)))?;
        f.instructions[3].volatile = true;
        forward(&mut f);
        assert!(matches!(
            f.instructions[1].op,
            Op::Scalar(ScalarOp::ExtendMove(Type::I16, R::RAX, R::RDI))
        ));
        for at in [2, 3, 4] {
            assert!(matches!(
                f.instructions[at].op,
                Op::Scalar(ScalarOp::Load(..))
            ));
        }
        Ok(())
    }

    #[test]
    fn scratch_overwrite_forwards_its_source_address_only() -> Result<(), Error> {
        let ops = [
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::Ptr,
                Operand::Register(R::RDI),
            )),
            Op::Scalar(ScalarOp::Load(R::R10, Type::Ptr, Operand::Frame(-8))),
            Op::Scalar(ScalarOp::Load(
                R::R10,
                Type::I64,
                Operand::Memory(super::super::memory::Memory::new(Base::Register(R::R10), 8)),
            )),
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-16),
                Type::I64,
                Operand::Register(R::R10),
            )),
            Op::Return(None),
        ];
        let mut f = sample(&ops, std::slice::from_ref(&(0..5)))?;
        forward(&mut f);
        assert!(matches!(f.instructions[1].op, Op::Nop));
        assert!(matches!(
            f.instructions[2].op,
            Op::Scalar(ScalarOp::Load(R::R10, Type::I64, Operand::Memory(m)))
                if m.base == Base::Register(R::RDI) && m.displacement == 8
        ));
        assert!(matches!(
            f.instructions[3].op,
            Op::Scalar(ScalarOp::Store(_, _, Operand::Register(R::R10)))
        ));
        // A two-address operation needs the scratch's old destination value.
        let mut partial = sample(&ops, std::slice::from_ref(&(0..5)))?;
        partial.instructions[2].op = Op::Scalar(ScalarOp::Binary(
            crate::lowir::vocabulary::Binary::Add,
            Type::I64,
            R::R10,
            Operand::Immediate(1),
        ));
        forward(&mut partial);
        assert!(matches!(
            partial.instructions[1].op,
            Op::Scalar(ScalarOp::Move(_, R::R10, Operand::Register(R::RDI)))
        ));
        Ok(())
    }
}
