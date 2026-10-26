//! Frame-copy forwarding and byte liveness from frozen `frame_layout/mir_cleanup`. See NOTICE.
use super::{
    condition_mir::FlagOp,
    frame_layout::{Index, Object, operands},
    memory::Base,
    mir::{Function, Instruction, Op},
    mir_cleanup::{SimpleWrite, simple_write},
    scalar_mir::{Operand, ScalarOp, bytes},
    x86::Register as R,
};
use crate::lowir::{Error, vocabulary::Type};

pub(super) fn frame_based(value: Operand) -> bool {
    value
        .memory()
        .is_some_and(|m| matches!(m.base, Base::Frame | Base::Register(R::RSP)))
}
pub(super) fn fixed_frame(value: Operand) -> Option<i32> {
    value.memory().and_then(super::memory::Memory::frame_offset)
}
fn plain_access(op: Op) -> bool {
    matches!(
        op,
        Op::Scalar(
            ScalarOp::Load(..)
                | ScalarOp::Store(..)
                | ScalarOp::CompareZero(..)
                | ScalarOp::Condition(
                    FlagOp::Compare(..)
                        | FlagOp::CompareFloatLiteral(..)
                        | FlagOp::Test(..)
                        | FlagOp::MaskTest(..)
                )
        ) | Op::FloatCopy(_)
            | Op::Bulk(_)
    )
}
fn tracked(function: &Function, index: &Index<'_>) -> Vec<bool> {
    let mut tracked = vec![true; index.objects.len()];
    let mut untrack = |offset| {
        if let Some(i) = index.find(offset) {
            tracked[i] = false;
        }
    };
    if function.scratch_bytes != 0 {
        untrack(function.scratch_offset);
    }
    if function.eh.slot < 0 {
        untrack(function.eh.slot);
    }
    for instruction in &function.instructions {
        let mut op = instruction.op;
        operands(&mut op, |value, width| {
            let Some(m) = value
                .memory()
                .filter(|m| m.base == Base::Frame && m.displacement < 0)
            else {
                return;
            };
            if let Some(i) = index.find(m.displacement)
                && (m.index.is_some()
                    || !plain_access(instruction.op)
                    || i64::from(m.displacement) + i64::from(width)
                        > i64::from(index.objects[i].offset) + i64::from(index.objects[i].bytes))
            {
                tracked[i] = false;
            }
        });
    }
    tracked
}
fn admitted(function: &Function, objects: &[Object]) -> bool {
    !objects.is_empty()
        && function.stack_size != 0
        && !function.variadic
        && !function.frame_policy.records_pushed
        && function.eh.landings.is_empty()
}
fn destination(op: Op) -> Option<(Operand, u32)> {
    match op {
        Op::Scalar(ScalarOp::Store(to, ty, _) | ScalarOp::MemoryBinary(_, ty, to, _)) => {
            Some((to, u32::from(bytes(ty))))
        }
        Op::FloatCopy(c) if c.destination.is_memory() => Some((
            c.destination,
            if c.ty == Type::F80 {
                10
            } else {
                u32::from(bytes(c.ty))
            },
        )),
        Op::Bulk(b) => Some((b.destination, b.bytes())),
        _ => None,
    }
}
fn direct(index: &Index<'_>, value: Operand) -> Option<usize> {
    fixed_frame(value).and_then(|d| index.find(d))
}
#[derive(Clone, Copy)]
struct Copied {
    object: usize,
    lo: i32,
    hi: i32,
    source: Operand,
    registers: u32,
}
const COPY_CLOBBERS: u32 = (1 << R::R10.code())
    | (1 << R::R11.code())
    | (1 << R::RDI.code())
    | (1 << R::RSI.code())
    | (1 << R::RCX.code());
fn memory_registers(source: Operand) -> u32 {
    source.mask() & 0xffff
}
fn source_memory(value: Operand) -> bool {
    value
        .memory()
        .is_some_and(|m| !matches!(m.base, Base::Symbol(_)))
}
pub(super) fn forward_copies(function: &mut Function, objects: &[Object]) -> usize {
    if !admitted(function, objects) {
        return 0;
    }
    let index = Index::new(objects);
    let tracked = tracked(function, &index);
    let mut copied: Vec<Copied> = Vec::new();
    let mut rewritten = 0;
    for block in &function.blocks {
        copied.clear();
        for i in block.range.clone() {
            let mut op = function.instructions[i].op;
            if matches!(op, Op::Nop | Op::Jump(_) | Op::Branch(..)) {
                continue;
            }
            if let Op::Scalar(ScalarOp::Load(r, ty, from)) = op
                && let Some(o) = direct(&index, from)
                && let Some(at) = fixed_frame(from)
                && let Some(c) = copied.iter().find(|c| {
                    c.object == o && c.lo <= at && at.wrapping_add(i32::from(bytes(ty))) <= c.hi
                })
            {
                op = Op::Scalar(ScalarOp::Load(
                    r,
                    ty,
                    c.source.displaced(at.wrapping_sub(c.lo)),
                ));
                function.instructions[i].op = op;
                rewritten += 1;
            }
            let bulk = matches!(op, Op::Bulk(_));
            let simple = simple_write(op);
            if simple.is_none() && !bulk {
                copied.clear();
                continue;
            }
            if let Some((to, width)) = destination(op) {
                if let Some(o) = direct(&index, to) {
                    let lo = fixed_frame(to).unwrap_or(0);
                    let hi = lo.wrapping_add(width.cast_signed());
                    copied.retain(|c| {
                        let src_object = direct(&index, c.source);
                        let src_lo = c.source.memory().map_or(0, |m| m.displacement);
                        let src_hi = src_lo.wrapping_add(c.hi.wrapping_sub(c.lo));
                        !((c.object == o && c.lo < hi && lo < c.hi)
                            || (src_object == Some(o) && src_lo < hi && lo < src_hi)
                            || (src_object.is_none() && !tracked[o]))
                    });
                } else {
                    copied.clear();
                }
            }
            let clobbered = if bulk {
                COPY_CLOBBERS
            } else if let Some(SimpleWrite::Register(r)) = simple {
                1 << r.code()
            } else {
                0
            };
            copied.retain(|c| c.registers & clobbered == 0);
            if let Op::Bulk(b) = op
                && b.bytes() <= 16
                && let Some(from) = b
                    .source
                    .filter(|&v| source_memory(v) && memory_registers(v) & COPY_CLOBBERS == 0)
                && let Some(o) = direct(&index, b.destination)
                && let Some(lo) = fixed_frame(b.destination)
            {
                copied.push(Copied {
                    object: o,
                    lo,
                    hi: lo.wrapping_add(b.bytes().cast_signed()),
                    source: from,
                    registers: memory_registers(from),
                });
            }
            if let Op::Scalar(ScalarOp::Store(to, ty, Operand::Register(R::R11))) = op
                && i > block.range.start
                && let Some(o) = direct(&index, to)
                && let Some(lo) = fixed_frame(to)
                && let Op::Scalar(ScalarOp::Load(R::R11, load_ty, from)) =
                    function.instructions[i - 1].op
                && load_ty == ty
                && source_memory(from)
                && memory_registers(from) & (1 << R::R11.code()) == 0
            {
                copied.push(Copied {
                    object: o,
                    lo,
                    hi: lo.wrapping_add(i32::from(bytes(ty))),
                    source: from,
                    registers: memory_registers(from),
                });
            }
        }
    }
    rewritten
}
struct Bytes<'a> {
    index: Index<'a>,
    tracked: Vec<bool>,
    base: Vec<usize>,
    words: usize,
}
impl<'a> Bytes<'a> {
    fn new(function: &Function, objects: &'a [Object]) -> Result<Self, Error> {
        let index = Index::new(objects);
        let tracked = tracked(function, &index);
        let mut count = 0_usize;
        let mut base = vec![0; objects.len()];
        for (i, object) in objects.iter().enumerate() {
            if tracked[i] {
                base[i] = count;
                count = count
                    .checked_add(usize::try_from(object.bytes).map_err(|_| Error::Limit)?)
                    .ok_or(Error::Limit)?;
            }
        }
        Ok(Self {
            index,
            tracked,
            base,
            words: count.div_ceil(64),
        })
    }
    fn range(&self, value: Operand, count: u32) -> Option<std::ops::Range<usize>> {
        let d = fixed_frame(value).filter(|&d| d < 0)?;
        let o = self.index.find(d).filter(|&o| self.tracked[o])?;
        let offset = usize::try_from(d.wrapping_sub(self.index.objects[o].offset)).ok()?;
        let start = self.base[o].checked_add(offset)?;
        Some(start..start.checked_add(usize::try_from(count).ok()?)?)
    }
    // Dead writes do not make their sources live, including during convergence.
    fn step(&self, instruction: &Instruction, live: &mut [u64]) -> bool {
        if matches!(instruction.op, Op::Nop) {
            return false;
        }
        let write = destination(instruction.op);
        if let Some((to, width)) = write
            && let Some(range) = self.range(to, width)
        {
            if !instruction.volatile && !range.clone().any(|i| live[i / 64] & (1 << (i % 64)) != 0)
            {
                return true;
            }
            for i in range {
                live[i / 64] &= !(1 << (i % 64));
            }
        }
        let stores = write.is_some_and(|(to, _)| to.is_memory());
        let mut first = true;
        let mut op = instruction.op;
        operands(&mut op, |value, width| {
            let skip = stores
                && first
                && !matches!(instruction.op, Op::Scalar(ScalarOp::MemoryBinary(..)));
            first = false;
            if !skip && let Some(range) = self.range(*value, width) {
                for i in range {
                    live[i / 64] |= 1 << (i % 64);
                }
            }
        });
        false
    }
}
pub(super) fn successors(function: &Function, block: usize, mut visit: impl FnMut(usize)) {
    for instruction in &function.instructions[function.blocks[block].range.clone()] {
        match instruction.op {
            Op::Jump(to) | Op::Branch(_, to) => visit(to.index()),
            Op::WideCompare(c) => {
                if let super::wide_compare::Destination::Branch { yes: a, no: b } = c.destination {
                    visit(a.index());
                    visit(b.index());
                }
            }
            _ => {}
        }
    }
}
pub(super) fn drop_stores(function: &mut Function, objects: &[Object]) -> Result<usize, Error> {
    if !admitted(function, objects) {
        return Ok(0);
    }
    let bytes = Bytes::new(function, objects)?;
    if bytes.words == 0 {
        return Ok(0);
    }
    let blocks = function.blocks.len();
    let cells = blocks.checked_mul(bytes.words).ok_or(Error::Limit)?;
    let mut incoming = vec![0_u64; cells];
    let mut live = vec![0_u64; bytes.words];
    let outgoing = |b, incoming: &[u64], live: &mut [u64]| {
        live.fill(0);
        successors(function, b, |s| {
            if s < blocks {
                for (to, &from) in live
                    .iter_mut()
                    .zip(&incoming[s * bytes.words..(s + 1) * bytes.words])
                {
                    *to |= from;
                }
            }
        });
    };
    loop {
        let mut changed = false;
        for b in (0..blocks).rev() {
            outgoing(b, &incoming, &mut live);
            for i in function.blocks[b].range.clone().rev() {
                bytes.step(&function.instructions[i], &mut live);
            }
            let previous = &mut incoming[b * bytes.words..(b + 1) * bytes.words];
            if previous != live {
                previous.copy_from_slice(&live);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut removed = vec![false; function.instructions.len()];
    for b in 0..blocks {
        outgoing(b, &incoming, &mut live);
        for i in function.blocks[b].range.clone().rev() {
            if bytes.step(&function.instructions[i], &mut live) {
                removed[i] = true;
            }
        }
    }
    let count = removed.iter().filter(|&&b| b).count();
    super::mir_cleanup::erase(function, &removed);
    Ok(count)
}

pub(super) fn drop_unused(function: &mut Function) -> bool {
    use super::{condition_mir::FlagOp, floating_mir::FloatingOp};
    if !function.frame_policy.omit_pointer
        || function.preserve != 0
        || function.stack_size == 0
        || function.scratch_bytes != 0
        || function.variadic
        || function.frame_policy.records_pushed
        || !function.eh.landings.is_empty()
    {
        return false;
    }
    for instruction in &function.instructions {
        let mut op = instruction.op;
        let allowed = match op {
            Op::Nop
            | Op::Move(..)
            | Op::SymbolMove(..)
            | Op::Jump(_)
            | Op::Branch(..)
            | Op::Return(_)
            | Op::Trap
            | Op::Bulk(_)
            | Op::DynamicCopy
            | Op::FloatCopy(_)
            | Op::Convert(_)
            | Op::Floating(
                FloatingOp::Arithmetic { .. } | FloatingOp::Compare { .. } | FloatingOp::Set { .. },
            ) => true,
            Op::Scalar(s) => !matches!(
                s,
                ScalarOp::Scan(..) | ScalarOp::Condition(FlagOp::Move(..))
            ),
            _ => false,
        };
        if !allowed || super::frame_encoding::extra_rsp(op) {
            return false;
        }
        let mut frame = false;
        operands(&mut op, |value, _| {
            frame |= frame_based(*value) || value.register() == Some(R::RSP);
        });
        if frame {
            return false;
        }
    }
    function.stack_size = 0;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn function(ops: &[Op]) -> Function {
        let program =
            crate::lowir::test_program(b"function @f() -> void {block ^entry: return void}")
                .unwrap();
        let symbol = program.publications()[0];
        let mut function = super::super::value_selection::select(
            &program,
            symbol,
            program.function(program.symbol(symbol)).unwrap(),
            0,
        )
        .unwrap();
        function.stack_size = 64;
        function.instructions = ops
            .iter()
            .map(|&op| Instruction {
                op,
                location: None,
                volatile: false,
            })
            .collect();
        function.blocks[0].range = 0..ops.len();
        function
    }
    #[test]
    fn loop_byte_liveness_keeps_the_read_store_and_drops_a_dead_copy_source() {
        use super::super::mir::{Block, BlockId};
        let write = |at, value| {
            Op::Scalar(ScalarOp::Store(
                Operand::Frame(at),
                Type::I64,
                Operand::Immediate(value),
            ))
        };
        let mut function = function(&[
            write(-16, 7),
            write(-64, 9),
            Op::Bulk(super::super::bulk::Bulk::new(
                Operand::Frame(-32),
                Some(Operand::Frame(-64)),
                8,
                8,
            )),
            Op::Jump(BlockId(1)),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::U8, Operand::Frame(-16))),
            Op::Branch(super::super::condition::Condition::Equal, BlockId(1)),
            Op::Jump(BlockId(2)),
            Op::Return(None),
        ]);
        let name = function.blocks[0].name;
        function.blocks = vec![
            Block {
                name,
                range: 0..4,
                landing: false,
            },
            Block {
                name,
                range: 4..7,
                landing: false,
            },
            Block {
                name,
                range: 7..8,
                landing: false,
            },
        ];
        let objects = [Object::new(-32, 32, 8), Object::new(-64, 32, 8)];
        assert_eq!(drop_stores(&mut function, &objects).unwrap(), 2);
        assert!(matches!(
            function.instructions[0].op,
            Op::Scalar(ScalarOp::Store(..))
        ));
        assert!(matches!(function.instructions[1].op, Op::Nop));
        assert!(matches!(function.instructions[2].op, Op::Nop));
        function.instructions[1] = Instruction {
            op: write(-64, 9),
            location: None,
            volatile: true,
        };
        assert_eq!(drop_stores(&mut function, &objects).unwrap(), 0);
        assert!(function.instructions[1].volatile);
    }
    #[test]
    fn copy_forwarding_observes_address_clobbers_and_original_adjacency() {
        let from = Operand::Memory(super::super::memory::Memory::new(Base::Register(R::R8), 0));
        let mut function = function(&[
            Op::Bulk(super::super::bulk::Bulk::new(
                Operand::Frame(-16),
                Some(from),
                16,
                8,
            )),
            Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
            Op::Move(R::R8, 0),
            Op::Scalar(ScalarOp::Load(R::RDX, Type::I64, Operand::Frame(-16))),
            Op::Return(None),
        ]);
        let objects = [Object::new(-32, 32, 8)];
        assert_eq!(forward_copies(&mut function, &objects), 1);
        assert!(
            matches!(function.instructions[1].op,Op::Scalar(ScalarOp::Load(_,_,source)) if source==from.displaced(8))
        );
        assert!(matches!(
            function.instructions[3].op,
            Op::Scalar(ScalarOp::Load(_, _, Operand::Frame(-16)))
        ));
        function.instructions = vec![
            Instruction {
                op: Op::Scalar(ScalarOp::Load(R::R11, Type::I64, from)),
                location: None,
                volatile: false,
            },
            Instruction {
                op: Op::Nop,
                location: None,
                volatile: false,
            },
            Instruction {
                op: Op::Scalar(ScalarOp::Store(
                    Operand::Frame(-16),
                    Type::I64,
                    Operand::Register(R::R11),
                )),
                location: None,
                volatile: false,
            },
            Instruction {
                op: Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-16))),
                location: None,
                volatile: false,
            },
        ];
        function.blocks[0].range = 0..4;
        assert_eq!(forward_copies(&mut function, &objects), 0);
    }
}
