//! Return-object byte provenance and frame release (frozen `mir_cleanup.cpp`). See NOTICE.
use super::{
    frame_layout::operands,
    frame_lifetimes::{fixed_frame, frame_based},
    mir::{Function, Instruction, Op},
    mir_cleanup::{SimpleWrite, simple_write},
    scalar_mir::{Operand, ScalarOp, bytes},
    x86::Register as R,
};
use crate::lowir::vocabulary::Type;

#[derive(Clone, Copy)]
enum Source {
    Open,
    Bad,
    Immediate(u8),
    Register(usize, i32),
    Copy(usize, i32),
}
struct FrameWrites {
    all: bool,
    ranges: Vec<(i32, i32)>,
}
fn read_only(op: Op) -> bool {
    use super::condition_mir::FlagOp;
    matches!(
        op,
        Op::Scalar(
            ScalarOp::Load(..)
                | ScalarOp::CompareZero(..)
                | ScalarOp::Condition(
                    FlagOp::Compare(..)
                        | FlagOp::CompareFloatLiteral(..)
                        | FlagOp::Test(..)
                        | FlagOp::MaskTest(..)
                        | FlagOp::BitTest(..)
                )
        )
    )
}
fn simple_width(op: Op) -> u32 {
    use super::condition_mir::FlagOp;
    let ty = match op {
        Op::Scalar(
            ScalarOp::Load(_, ty, _)
            | ScalarOp::Store(_, ty, _)
            | ScalarOp::Binary(_, ty, _, _)
            | ScalarOp::MemoryBinary(_, ty, _, _)
            | ScalarOp::Unary(_, ty, _)
            | ScalarOp::Extend(ty, _)
            | ScalarOp::ExtendMove(ty, _, _)
            | ScalarOp::CompareZero(ty, _)
            | ScalarOp::Condition(
                FlagOp::Compare(ty, ..)
                | FlagOp::Test(ty, _)
                | FlagOp::MaskTest(ty, ..)
                | FlagOp::BitTest(ty, ..),
            ),
        ) => ty,
        Op::Scalar(ScalarOp::Condition(FlagOp::CompareFloatLiteral(kind, ..))) => kind.ty(),
        _ => Type::I64,
    };
    u32::from(bytes(ty))
}
fn lea(op: Op) -> Option<Operand> {
    if let Op::Scalar(ScalarOp::Lea(_, value) | ScalarOp::Lea32(_, value)) = op {
        Some(value)
    } else {
        None
    }
}
impl FrameWrites {
    fn new(function: &Function) -> Self {
        let mut out = Self {
            all: false,
            ranges: Vec::new(),
        };
        for inst in &function.instructions {
            if matches!(inst.op, Op::Nop) {
                continue;
            }
            let readonly = read_only(inst.op);
            if lea(inst.op).is_some_and(frame_based) {
                out.all = true;
            }
            let width = if let Op::Bulk(b) = inst.op {
                b.bytes()
            } else if simple_write(inst.op).is_some() {
                simple_width(inst.op)
            } else {
                16
            };
            let mut at = 0;
            let mut op = inst.op;
            operands(&mut op, |value, _| {
                let skip = matches!(inst.op, Op::Bulk(b) if b.source.is_some()) && at == 1
                    || lea(inst.op).is_some();
                at += 1;
                if !frame_based(*value) {
                    return;
                }
                if readonly {
                    out.all |= fixed_frame(*value).is_none();
                } else if !skip {
                    if let Some(offset) = fixed_frame(*value) {
                        out.ranges
                            .push((offset, offset.wrapping_add(width.cast_signed())));
                    } else {
                        out.all = true;
                    }
                }
            });
            // Host effects keep their frame operands in the landing table.
            if let Op::HostEh(effect) = inst.op {
                use super::landing_pad::Effect;
                let slot = function.eh.slot;
                match effect {
                    Effect::Landing { copy_selector } => {
                        out.ranges.push((slot, slot.wrapping_add(16)));
                        let next = slot.wrapping_add(if function.eh.raw { 16 } else { 8 });
                        out.ranges.push((next, next.wrapping_add(16)));
                        if copy_selector {
                            out.ranges
                                .push((slot.wrapping_add(8), slot.wrapping_add(24)));
                        }
                    }
                    Effect::Select(_) => {
                        out.ranges
                            .push((slot.wrapping_add(8), slot.wrapping_add(32)));
                    }
                    Effect::CatchAll(_) => {
                        out.ranges
                            .push((slot.wrapping_add(8), slot.wrapping_add(24)));
                    }
                    Effect::Resume => {}
                }
            }
        }
        out
    }
    fn written(&self, at: i32) -> bool {
        self.all || self.ranges.iter().any(|&(lo, hi)| lo <= at && at < hi)
    }
}
fn store(op: Op) -> Option<(i32, u32, Option<Operand>, bool)> {
    match op {
        Op::Scalar(ScalarOp::Store(to, ty, source)) => {
            Some((fixed_frame(to)?, u32::from(bytes(ty)), Some(source), false))
        }
        Op::Bulk(b) => Some((fixed_frame(b.destination)?, b.bytes(), b.source, true)),
        _ => None,
    }
}
fn load(op: Op) -> Option<(R, Type, Operand)> {
    if let Op::Scalar(ScalarOp::Load(r, ty, source)) = op {
        Some((r, ty, source))
    } else {
        None
    }
}
fn move_parts(op: Op) -> Option<(R, Operand)> {
    match op {
        Op::Move(r, bits) => Some((r, Operand::Immediate(bits))),
        Op::SymbolMove(r, symbol) => Some((r, Operand::Symbol(symbol))),
        Op::Scalar(ScalarOp::Load(r, _, value) | ScalarOp::Move(r, value)) => Some((r, value)),
        Op::Scalar(ScalarOp::ExtendMove(ty, r, source)) if !super::scalar_mir::signed(ty) => {
            Some((r, Operand::Register(source)))
        }
        _ => None,
    }
}
fn stable_source(value: Operand, clobbered: u32) -> bool {
    if value.is_memory() {
        value.memory().is_some_and(|m| {
            !matches!(m.base, super::memory::Base::Register(r) if r.code() >= 16)
                && m.index.is_none_or(|r| r.code() < 16)
                && m.mask() & clobbered == 0
        })
    } else {
        value
            .register()
            .is_none_or(|r| r.code() < 16 && clobbered & (1 << r.code()) == 0)
    }
}
fn resolve(
    function: &Function,
    first: usize,
    end: usize,
    original: Op,
    writes: &mut Option<FrameWrites>,
) -> Option<Op> {
    let (destination, ty, from) = load(original)?;
    let a = fixed_frame(from)?;
    let width = usize::from(bytes(ty));
    let mut sources = [Source::Open; 8];
    let mut open = width;
    let mut clobbered = 0;
    let mut memory_written = false;
    for i in (first..end).rev() {
        if open == 0 {
            break;
        }
        let instruction = &function.instructions[i];
        let op = instruction.op;
        if matches!(op, Op::Nop) {
            continue;
        }
        if instruction.volatile {
            break;
        }
        if let Some((offset, count, source, bulk)) =
            store(op).filter(|&(_, n, s, b)| !b || s.is_none() || n <= 16)
        {
            let copy = bulk && source.is_some();
            let okay = copy
                && source.is_some_and(|value| {
                    !frame_based(value) && !memory_written && stable_source(value, clobbered)
                });
            for (p, reading) in sources[..width].iter_mut().enumerate() {
                let byte = a.wrapping_add(i32::try_from(p).ok()?);
                if !matches!(reading, Source::Open)
                    || byte < offset
                    || byte >= offset.wrapping_add(count.cast_signed())
                {
                    continue;
                }
                open -= 1;
                let delta = byte.wrapping_sub(offset);
                *reading = match source {
                    None => Source::Immediate(0),
                    Some(_) if copy => {
                        if okay {
                            Source::Copy(i, delta)
                        } else {
                            Source::Bad
                        }
                    }
                    Some(Operand::Immediate(bits)) => {
                        Source::Immediate(bits.to_le_bytes()[usize::try_from(delta).ok()?])
                    }
                    Some(Operand::Register(r))
                        if r.code() < 16 && clobbered & (1 << r.code()) == 0 =>
                    {
                        Source::Register(i, delta)
                    }
                    _ => Source::Bad,
                };
            }
            if copy {
                clobbered |= (1 << R::R10.code()) | (1 << R::R11.code());
            }
            continue;
        }
        let Some(simple) = simple_write(op) else {
            break;
        };
        // A remaining simple memory destination is an indirect store.
        if let Op::Scalar(ScalarOp::Store(to, _, _) | ScalarOp::MemoryBinary(_, _, to, _)) = op {
            if let Some(offset) = fixed_frame(to) {
                let end = offset.wrapping_add(simple_width(op).cast_signed());
                let past = a.wrapping_add(i32::try_from(width).ok()?);
                if offset < past && a < end {
                    break;
                }
            } else if frame_based(to) {
                break;
            } else {
                memory_written = true;
            }
        }
        if let SimpleWrite::Register(r) = simple {
            clobbered |= 1 << r.code();
        }
    }
    for (p, source) in sources[..width].iter_mut().enumerate() {
        if matches!(source, Source::Open) {
            let writes = writes.get_or_insert_with(|| FrameWrites::new(function));
            if writes.written(a.wrapping_add(i32::try_from(p).ok()?)) {
                return None;
            }
            *source = Source::Immediate(0);
        }
    }
    match sources[0] {
        Source::Immediate(_) => {
            let mut word = [0_u8; 8];
            for (i, s) in sources[..width].iter().enumerate() {
                let Source::Immediate(b) = *s else {
                    return None;
                };
                word[i] = b;
            }
            let value = i64::from_le_bytes(word);
            Some(Op::Move(destination, i128::from(value).cast_unsigned()))
        }
        Source::Register(definition, offset) | Source::Copy(definition, offset) => {
            let copied = matches!(sources[0], Source::Copy(..));
            for (p, s) in sources[..width].iter().enumerate() {
                let (Source::Register(w, d) | Source::Copy(w, d)) = *s else {
                    return None;
                };
                if matches!(s, Source::Copy(..)) != copied
                    || w != definition
                    || d != offset.wrapping_add(i32::try_from(p).ok()?)
                {
                    return None;
                }
            }
            let (_, _, source, _) = store(function.instructions[definition].op)?;
            let source = source?;
            if copied {
                let source = if let Some(r) = source.register() {
                    Operand::Memory(super::memory::Memory::new(
                        super::memory::Base::Register(r),
                        0,
                    ))
                } else {
                    source
                };
                Some(Op::Scalar(ScalarOp::Load(
                    destination,
                    ty,
                    source.displaced(offset),
                )))
            } else if offset != 0 {
                None
            } else if width == 8 {
                Some(Op::Scalar(ScalarOp::Move(destination, source)))
            } else {
                Some(Op::Scalar(ScalarOp::ExtendMove(
                    Type::U32,
                    destination,
                    source.register()?,
                )))
            }
        }
        _ => None,
    }
}
fn prune_writes(function: &mut Function, first: usize, ret: usize, escapes: bool) -> usize {
    let mut reads: Vec<(i32, i32)> = Vec::new();
    let mut removed = 0;
    for i in (first..ret).rev() {
        let instruction = &mut function.instructions[i];
        if matches!(instruction.op, Op::Nop) {
            continue;
        }
        if instruction.volatile {
            break;
        }
        if let Some((offset, width, source, bulk)) = store(instruction.op) {
            let hi = offset.wrapping_add(width.cast_signed());
            if !reads.iter().any(|&(a, b)| offset < b && a < hi) {
                instruction.op = Op::Nop;
                removed += 1;
                continue;
            }
            if bulk && let Some(from) = source {
                if let Some(at) = fixed_frame(from) {
                    reads.push((at, at.wrapping_add(width.cast_signed())));
                } else if escapes || frame_based(from) {
                    break;
                }
            }
            continue;
        }
        if simple_write(instruction.op).is_none() {
            break;
        }
        let mut stop = false;
        let is_store = matches!(instruction.op, Op::Scalar(ScalarOp::Store(..)));
        let is_lea = lea(instruction.op).is_some();
        let width = simple_width(instruction.op).cast_signed();
        let mut operand_index = 0;
        operands(&mut instruction.op, |value, _| {
            let skip = is_store && operand_index == 0 || is_lea;
            operand_index += 1;
            if stop || skip || !value.is_memory() {
                return;
            }
            if let Some(at) = fixed_frame(*value) {
                reads.push((at, at.wrapping_add(width)));
            } else {
                stop = escapes || frame_based(*value);
            }
        });
        if stop {
            break;
        }
    }
    removed
}
fn once(function: &mut Function) -> usize {
    let mut writes = None;
    let mut removed = 0;
    let escapes = function.variadic
        || function.frame_policy.records_pushed
        || function.instructions.iter().any(|i| {
            lea(i.op).is_some_and(frame_based)
                || matches!(
                    i.op,
                    Op::Exception(super::exception::ExceptionOp::Push { .. })
                )
        });
    for b in 0..function.blocks.len() {
        let range = function.blocks[b].range.clone();
        let Some(ret) = range.end.checked_sub(1).filter(|_| !range.is_empty()) else {
            continue;
        };
        if !matches!(function.instructions[ret].op, Op::Return(_)) {
            continue;
        }
        let mut indexes = [0_usize; 2];
        let mut count = 0;
        let mut start = ret;
        for i in (range.start..ret).rev() {
            let instruction = &function.instructions[i];
            if matches!(instruction.op, Op::Nop) {
                continue;
            }
            let Some((d, _)) = move_parts(instruction.op) else {
                break;
            };
            if count == 2 || !matches!(d, R::RAX | R::RDX) || instruction.volatile {
                break;
            }
            if count == 1
                && let Some((other, source)) = move_parts(function.instructions[indexes[0]].op)
                && (other == d || source.mask() & (1 << d.code()) != 0)
            {
                break;
            }
            indexes[count] = i;
            count += 1;
            start = i;
        }
        let mut now = [function.instructions[ret]; 2];
        let mut changed = false;
        for k in 0..count {
            now[k] = function.instructions[indexes[k]];
            if let Some((_, ty, from)) = load(now[k].op)
                && fixed_frame(from).is_some()
                && (bytes(ty) == 8 || ty == Type::U32)
                && let Some(op) = resolve(function, range.start, start, now[k].op, &mut writes)
            {
                now[k] = Instruction {
                    op,
                    location: None,
                    volatile: false,
                };
                changed = true;
            }
        }
        if !changed {
            continue;
        }
        if count == 2 {
            let (Some((a, asrc)), Some((d, dsrc))) = (move_parts(now[1].op), move_parts(now[0].op))
            else {
                continue;
            };
            if dsrc.mask() & (1 << a.code()) != 0 {
                if asrc.mask() & (1 << d.code()) == 0 {
                    now.swap(0, 1);
                } else if load(function.instructions[indexes[0]].op)
                    .is_some_and(|(_, _, s)| fixed_frame(s).is_some())
                {
                    now[0] = function.instructions[indexes[0]];
                } else if load(function.instructions[indexes[1]].op)
                    .is_some_and(|(_, _, s)| fixed_frame(s).is_some())
                {
                    now[1] = function.instructions[indexes[1]];
                    now.swap(0, 1);
                } else {
                    continue;
                }
            }
        }
        for k in 0..count {
            function.instructions[indexes[k]] = now[k];
        }
        removed += prune_writes(function, range.start, ret, escapes);
    }
    removed
}
pub(super) fn forward(function: &mut Function) -> usize {
    let removed = once(function);
    removed + if removed == 0 { 0 } else { once(function) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn function(ops: &[Op]) -> Function {
        let p = crate::lowir::test_program(b"function @f() -> void {block ^entry: return void}")
            .unwrap();
        let symbol = p.publications()[0];
        let mut f = super::super::value_selection::select(
            &p,
            symbol,
            p.function(p.symbol(symbol)).unwrap(),
            0,
        )
        .unwrap();
        f.instructions = ops
            .iter()
            .map(|&op| Instruction {
                op,
                location: None,
                volatile: false,
            })
            .collect();
        f.blocks[0].range = 0..ops.len();
        f.stack_size = 32;
        f.frame_policy.omit_pointer = true;
        f
    }
    #[test]
    fn symbolic_return_move_keeps_the_other_frame_load_in_the_suffix() {
        let mut f = function(&[
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I64,
                Operand::Immediate(7),
            )),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
            Op::Return(None),
        ]);
        let symbol = f.symbol;
        let file = crate::lowir::model::NameId::empty();
        f.instructions.insert(
            2,
            Instruction {
                op: Op::SymbolMove(R::RDX, symbol),
                location: Some(crate::lowir::model::DebugLocation {
                    file,
                    line: 47,
                    column: 5,
                }),
                volatile: false,
            },
        );
        f.blocks[0].range.end += 1;
        // Symbolic moves remain a pruning barrier even in an admitted suffix.
        assert_eq!(forward(&mut f), 0);
        assert!(matches!(
            f.instructions[0].op,
            Op::Scalar(ScalarOp::Store(..))
        ));
        assert!(matches!(f.instructions[1].op, Op::Move(R::RAX, 7)));
        assert!(matches!(f.instructions[2].op, Op::SymbolMove(R::RDX, s) if s == symbol));
        assert!(f.instructions[1].location.is_none());
        assert_eq!(
            f.instructions[2]
                .location
                .map(|l| (l.file, l.line, l.column)),
            Some((file, 47, 5)),
        );
    }
    #[test]
    fn floating_frame_reads_preserve_only_their_own_bytes() {
        use super::super::condition_mir::FlagOp;
        use crate::lowir::literals::FloatKind;
        for (kind, bits, removed) in [
            (FloatKind::F32, u128::from(1.0_f32.to_bits()), 1),
            (FloatKind::F64, u128::from(1.0_f64.to_bits()), 0),
        ] {
            let mut f = function(&[
                Op::Scalar(ScalarOp::Store(
                    Operand::Frame(-8),
                    Type::U32,
                    Operand::Immediate(1),
                )),
                Op::Scalar(ScalarOp::Store(
                    Operand::Frame(-4),
                    Type::U32,
                    Operand::Immediate(2),
                )),
                Op::Scalar(ScalarOp::Condition(FlagOp::CompareFloatLiteral(
                    kind,
                    Operand::Frame(-8),
                    bits,
                ))),
                Op::Return(None),
            ]);
            assert_eq!(prune_writes(&mut f, 0, 3, false), removed);
            assert!(matches!(
                f.instructions[0].op,
                Op::Scalar(ScalarOp::Store(..))
            ));
            assert_eq!(matches!(f.instructions[1].op, Op::Nop), removed == 1);
        }
    }
    #[test]
    fn a_frame_mask_read_preserves_return_padding_provenance() {
        let mut f = function(&[
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-16),
                Type::U8,
                Operand::Immediate(0xab),
            )),
            Op::Scalar(ScalarOp::Condition(
                super::super::condition_mir::FlagOp::MaskTest(
                    Type::I64,
                    Operand::Frame(-16),
                    Operand::Immediate(1),
                ),
            )),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-16))),
            Op::Return(None),
        ]);
        forward(&mut f);
        assert!(matches!(f.instructions[2].op, Op::Move(R::RAX, 0xab)));
        assert!(matches!(
            f.instructions[0].op,
            Op::Scalar(ScalarOp::Store(..))
        ));
    }
    #[test]
    fn partial_return_bytes_use_unwritten_padding_and_release_the_frame() {
        let mut f = function(&[
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-16),
                Type::U8,
                Operand::Immediate(0xab),
            )),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-16))),
            Op::Return(None),
        ]);
        assert_eq!(forward(&mut f), 1);
        assert!(matches!(f.instructions[0].op, Op::Nop));
        assert!(matches!(f.instructions[1].op, Op::Move(R::RAX, 0xab)));
        assert!(super::super::frame_lifetimes::drop_unused(&mut f));
        assert_eq!(f.stack_size, 0);
    }
    #[test]
    fn parallel_return_cycle_keeps_one_load_and_volatile_access_stops_forwarding() {
        let mut f = function(&[
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-16),
                Type::I64,
                Operand::Register(R::RDX),
            )),
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I64,
                Operand::Register(R::RAX),
            )),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-16))),
            Op::Scalar(ScalarOp::Load(R::RDX, Type::I64, Operand::Frame(-8))),
            Op::Return(None),
        ]);
        assert_eq!(forward(&mut f), 1);
        assert!(matches!(
            f.instructions[2].op,
            Op::Scalar(ScalarOp::Move(R::RAX, Operand::Register(R::RDX)))
        ));
        assert!(matches!(
            f.instructions[3].op,
            Op::Scalar(ScalarOp::Load(R::RDX, _, Operand::Frame(-8)))
        ));
        let mut f = function(&[
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(-16),
                Type::I64,
                Operand::Immediate(7),
            )),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-16))),
            Op::Return(None),
        ]);
        f.instructions[0].volatile = true;
        assert_eq!(forward(&mut f), 0);
        f.instructions[0].volatile = false;
        f.instructions[1].volatile = true;
        assert_eq!(forward(&mut f), 0);
    }
}
