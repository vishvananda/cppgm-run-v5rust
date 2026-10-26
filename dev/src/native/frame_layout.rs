//! Access-density ordering of the selected function's owned frame objects. See NOTICE.
use super::{
    atomic::Atomic,
    condition_mir::FlagOp,
    exception::ExceptionOp,
    floating_mir::FloatingOp,
    memory::Base,
    mir::{Function, Op},
    scalar_mir::{Operand, ScalarOp},
};
use crate::lowir::vocabulary::Type;

pub(super) struct Object {
    pub(super) offset: i32,
    pub(super) bytes: u32,
    align: u32,
}
// Immutable containment queries share one function-owned sorted inventory.
pub(super) struct Index<'a> {
    pub(super) objects: &'a [Object],
    order: Vec<usize>,
}
impl<'a> Index<'a> {
    pub(super) fn new(objects: &'a [Object]) -> Self {
        let mut order: Vec<_> = (0..objects.len()).collect();
        order.sort_unstable_by_key(|&i| objects[i].offset);
        Self { objects, order }
    }
    pub(super) fn find(&self, offset: i32) -> Option<usize> {
        let past = self
            .order
            .partition_point(|&i| self.objects[i].offset <= offset);
        past.checked_sub(1).map(|i| self.order[i]).filter(|&i| {
            i64::from(offset) < i64::from(self.objects[i].offset) + i64::from(self.objects[i].bytes)
        })
    }
}
impl Object {
    pub(super) fn new(offset: i32, bytes: u32, align: u32) -> Self {
        Self {
            offset,
            bytes,
            align,
        }
    }
}
fn access_bytes(ty: Type) -> u32 {
    match ty {
        Type::Void => 1,
        Type::F80 => 10,
        _ => u32::from(super::scalar_mir::bytes(ty)),
    }
}
// Visit the actual MIR operands; implicit scratch/frame metadata is relocated
// separately. False is the frozen VaStart/EhPush prohibition on frame ordering.
pub(super) fn operands(op: &mut Op, mut visit: impl FnMut(&mut Operand, u32)) -> bool {
    match op {
        Op::Scalar(scalar) => match scalar {
            ScalarOp::Load(_, ty, source)
            | ScalarOp::Binary(_, ty, _, source)
            | ScalarOp::Scan(_, ty, _, source)
            | ScalarOp::CompareZero(ty, source) => visit(source, access_bytes(*ty)),
            ScalarOp::Store(destination, ty, source) => {
                visit(destination, access_bytes(*ty));
                visit(source, access_bytes(*ty));
            }
            ScalarOp::Condition(flag) => match flag {
                FlagOp::Compare(ty, left, right) | FlagOp::MaskTest(ty, left, right) => {
                    visit(left, access_bytes(*ty));
                    visit(right, access_bytes(*ty));
                }
                FlagOp::CompareFloatLiteral(kind, left, _) => visit(left, access_bytes(kind.ty())),
                FlagOp::Move(_, _, source) | FlagOp::BitTest(_, _, source) => visit(source, 1),
                FlagOp::Test(..) | FlagOp::Set(..) => {}
            },
            ScalarOp::Move(_, source)
            | ScalarOp::Lea(_, source)
            | ScalarOp::Lea32(_, source)
            | ScalarOp::Divide { source, .. } => visit(source, 1),
            ScalarOp::Shift(_, ty, _, count) => visit(count, access_bytes(*ty)),
            ScalarOp::SignDividend
            | ScalarOp::MultiplyWide { .. }
            | ScalarOp::Unary(..)
            | ScalarOp::Extend(..)
            | ScalarOp::ExtendMove(..)
            | ScalarOp::SetEqual(..)
            | ScalarOp::Movzx(..) => {}
        },
        Op::FloatCopy(copy) => {
            visit(&mut copy.destination, access_bytes(copy.ty));
            visit(&mut copy.source, access_bytes(copy.ty));
        }
        Op::Floating(float) => match float {
            FloatingOp::Compare { ty, left, right }
            | FloatingOp::Set {
                ty, left, right, ..
            } => {
                visit(left, access_bytes(*ty));
                visit(right, access_bytes(*ty));
            }
            FloatingOp::Arithmetic {
                ty,
                destination,
                left,
                right,
                ..
            } => {
                visit(destination, access_bytes(*ty));
                visit(left, access_bytes(*ty));
                if let Some(right) = right {
                    visit(right, access_bytes(*ty));
                }
            }
            FloatingOp::ReturnPair(real, imaginary) => {
                visit(real, 10);
                visit(imaginary, 10);
            }
            FloatingOp::Return(source) => visit(source, 10),
            FloatingOp::Pop(destination) => {
                if let Some(destination) = destination {
                    visit(destination, 10);
                }
            }
        },
        Op::Bulk(bulk) => {
            let bytes = bulk.bytes();
            visit(&mut bulk.destination, bytes);
            if let Some(source) = &mut bulk.source {
                visit(source, bytes);
            }
        }
        Op::Wide(wide) => {
            visit(&mut wide.destination, 8);
            visit(&mut wide.left, 8);
            if let Some(right) = &mut wide.right {
                visit(right, 8);
            }
        }
        Op::WideMove {
            destination,
            source,
        } => {
            visit(destination, 8);
            visit(source, 8);
        }
        Op::WideExtend { destination, .. } => visit(destination, 8),
        Op::WideCompare(compare) => {
            visit(&mut compare.left, 8);
            visit(&mut compare.right, 8);
        }
        Op::Convert(convert) => {
            visit(&mut convert.destination, access_bytes(convert.from));
            visit(&mut convert.source, access_bytes(convert.from));
        }
        Op::Call(call) | Op::TailJump(call) => visit(&mut call.target, 1),
        Op::Atomic(atomic) => match atomic {
            Atomic::Exchange(_, ty, target, _) => visit(target, access_bytes(*ty)),
            Atomic::LoadWide {
                destination,
                source,
            } => {
                visit(destination, 8);
                visit(source, 8);
            }
            Atomic::CompareWide {
                target,
                expected,
                desired,
                ..
            } => {
                visit(target, 8);
                visit(expected, 8);
                visit(desired, 8);
            }
            Atomic::Fence => {}
        },
        Op::VaArg(argument) => argument.map_list(visit),
        Op::Exception(ExceptionOp::Throw(source)) => visit(source, 1),
        Op::VaStart(start) => {
            start.map_operands(visit);
            return false;
        }
        Op::Exception(ExceptionOp::Push { .. }) | Op::HostEh(_) => return false,
        Op::Nop
        | Op::Jump(_)
        | Op::Branch(..)
        | Op::Move(..)
        | Op::SymbolMove(..)
        | Op::Return(_)
        | Op::Trap
        | Op::DynamicCopy
        | Op::StackAlloc(_)
        | Op::Exception(_) => {}
    }
    true
}
fn displacement(operand: &mut Operand) -> Option<&mut i32> {
    match operand {
        Operand::Frame(offset) if *offset < 0 => Some(offset),
        Operand::Memory(memory) if memory.base == Base::Frame && memory.displacement < 0 => {
            Some(&mut memory.displacement)
        }
        _ => None,
    }
}
pub(super) fn order(function: &mut Function, objects: &[Object]) {
    if objects.len() < 2
        || function.stack_size == 0
        || function.variadic
        || function.aligned_base != 0
        || function.frame_pinned
        || function.frame_policy.records_pushed
    {
        return;
    }
    let index = Index::new(objects);
    let find = |offset| index.find(offset);
    let mut weights = vec![0_u64; objects.len()];
    let mut valid = true;
    let frequency = super::block_placement::Flow::new(function).map(|flow| flow.frequencies);
    for b in 0..function.blocks.len() {
        let weight = frequency.as_ref().map_or(1, |f| f[b]);
        for i in function.blocks[b].range.clone() {
            let permitted = operands(&mut function.instructions[i].op, |operand, bytes| {
                let indexed = operand.memory().is_some_and(|m| m.index.is_some());
                let Some(offset) = displacement(operand) else {
                    return;
                };
                if let Some(i) = find(*offset) {
                    valid &= indexed
                        || i64::from(*offset) + i64::from(bytes)
                            <= i64::from(objects[i].offset) + i64::from(objects[i].bytes);
                    weights[i] = weights[i].wrapping_add(weight);
                } else {
                    valid = false;
                }
            });
            valid &= permitted;
        }
    }
    if !valid {
        return;
    }
    let mut order: Vec<_> = (0..objects.len()).collect();
    order.sort_by(|&x, &y| {
        let dx = u128::from(weights[x]) * u128::from(objects[y].bytes);
        let dy = u128::from(weights[y]) * u128::from(objects[x].bytes);
        if function.frame_policy.omit_pointer {
            dx.cmp(&dy)
        } else {
            dy.cmp(&dx)
        }
    });
    let mut moved = vec![0_i32; objects.len()];
    let mut size = 0_u32;
    for i in order {
        let align = objects[i].align.max(1);
        size = size.wrapping_add(objects[i].bytes).wrapping_add(align - 1) & !(align - 1);
        moved[i] = size.cast_signed().wrapping_neg();
    }
    if objects
        .iter()
        .zip(&moved)
        .all(|(object, &offset)| object.offset == offset)
    {
        return;
    }
    let relocate = |offset: &mut i32| {
        if let Some(i) = find(*offset) {
            *offset = offset.wrapping_add(moved[i].wrapping_sub(objects[i].offset));
        }
    };
    for inst in &mut function.instructions {
        operands(&mut inst.op, |operand, _| {
            if let Some(offset) = displacement(operand) {
                relocate(offset);
            }
        });
    }
    for binding in &mut function.bindings {
        relocate(&mut binding.offset);
    }
    if function.scratch_bytes != 0 {
        relocate(&mut function.scratch_offset);
    }
    if function.preserve != 0 {
        size = size
            .wrapping_add(8 * function.preserve.count_ones())
            .wrapping_add(15)
            & !15;
    }
    function.stack_size = size.wrapping_add(15) & !15;
    function.ordered_frames += 1;
}
