//! Virtual frame addressing and frozen optimized frame policy. See NOTICE.
use super::{
    atomic::Atomic,
    condition_mir::FlagOp,
    exception::ExceptionOp,
    floating_mir::FloatingOp,
    memory::Base,
    mir::{Function, Op},
    object_compile::ObjectModel,
    scalar_mir::{Operand, ScalarOp},
    wide_compare::Destination,
    wide_mir::Operation,
    x86::{Register as R, Rm},
};
use crate::lowir::vocabulary::{Binary, Type};

pub(super) const SAVES: [R; 6] = [R::RBX, R::R12, R::R13, R::R14, R::R15, R::RBP];

#[derive(Default, Clone, Copy)]
pub(super) struct Addressing {
    omit: bool,
    size: i32,
    incoming: i32,
    shift: i32,
}
impl Addressing {
    pub(super) fn resolve(self, operand: Rm) -> Rm {
        let Rm::Frame {
            index,
            scale,
            disp,
            incoming,
        } = operand
        else {
            return operand;
        };
        Rm::Memory {
            base: Some(if self.omit { R::RSP } else { R::RBP }),
            index,
            scale,
            disp: if self.omit {
                disp.wrapping_add(self.size)
                    .wrapping_sub(if incoming { self.incoming } else { 0 })
                    .wrapping_add(self.shift)
            } else {
                disp
            },
            target: None,
        }
    }
    pub(super) fn adjust(&mut self, delta: i32) {
        self.shift = self.shift.wrapping_add(delta);
    }
    pub(super) fn reset_area(&mut self) -> bool {
        let changed = self.shift != 0;
        self.shift = 0;
        changed
    }
    pub(super) fn area(self) -> u32 {
        self.shift.cast_unsigned()
    }
}
#[derive(Default)]
pub(super) struct Policy {
    pub(super) omit_pointer: bool,
    pub(super) records_pushed: bool,
}
pub(super) struct Plan {
    pub(super) address: Addressing,
    pub(super) omit: bool,
    pub(super) reserve: u32,
    pub(super) dynamic: bool,
}
impl Plan {
    pub(super) fn new(function: &Function) -> Self {
        let dynamic = function.frame_policy.records_pushed
            || function
                .instructions
                .iter()
                .any(|i| matches!(i.op, Op::StackAlloc(_)));
        let omit = function.frame_policy.omit_pointer
            && !dynamic
            && function.object_model == ObjectModel::Host;
        let incoming = if omit && !function.instructions.iter().any(|i| calls_out(i.op)) {
            8
        } else {
            0
        };
        let saved = function.preserve.count_ones();
        let mut reserve = if omit {
            function
                .stack_size
                .wrapping_add(8)
                .wrapping_sub(saved * 8)
                .wrapping_sub(incoming)
        } else {
            0
        };
        if omit && frameless_body(function) {
            reserve = if incoming != 0 || saved % 2 == 1 {
                0
            } else {
                8
            };
        }
        Self {
            address: Addressing {
                omit,
                size: function.stack_size.cast_signed(),
                incoming: incoming.cast_signed(),
                shift: 0,
            },
            omit,
            reserve,
            dynamic,
        }
    }
}
pub(super) fn calls_out(op: Op) -> bool {
    matches!(
        op,
        Op::Call(_)
            | Op::HostEh(super::landing_pad::Effect::Resume)
            | Op::Exception(
                ExceptionOp::Catch { .. } | ExceptionOp::Resume | ExceptionOp::Throw(_)
            )
    ) || matches!(op, Op::Wide(w) if matches!(w.operation, Operation::Binary(Binary::Div | Binary::UDiv | Binary::Mod | Binary::UMod)))
        || matches!(op, Op::Convert(c) if c.from == Type::I128 || c.to == Type::I128)
}
pub(super) fn extra_rsp(op: Op) -> bool {
    let reg = |r: R| r == R::RSP;
    match op {
        Op::Move(r, _)
        | Op::SymbolMove(r, _)
        | Op::Return(Some(r))
        | Op::WideExtend { source: r, .. } => reg(r),
        Op::Scalar(scalar) => match scalar {
            ScalarOp::MultiplyWide { source, .. }
            | ScalarOp::Unary(_, _, source)
            | ScalarOp::Extend(_, source)
            | ScalarOp::SetEqual(source)
            | ScalarOp::Movzx(source) => reg(source),
            ScalarOp::Move(r, _)
            | ScalarOp::Lea(r, _)
            | ScalarOp::Lea32(r, _)
            | ScalarOp::Load(r, _, _)
            | ScalarOp::Scan(_, _, r, _)
            | ScalarOp::Binary(_, _, r, _)
            | ScalarOp::Shift(_, _, r, _) => reg(r),
            ScalarOp::ExtendMove(_, a, b) => reg(a) || reg(b),
            ScalarOp::Condition(FlagOp::Set(_, r) | FlagOp::Test(_, r) | FlagOp::Move(_, r, _)) => {
                reg(r)
            }
            _ => false,
        },
        Op::Floating(FloatingOp::Set { destination, .. })
        | Op::Exception(
            ExceptionOp::Read { destination, .. } | ExceptionOp::Selector(destination),
        ) => reg(destination),
        Op::WideCompare(w) => matches!(w.destination, Destination::Register(r) if reg(r)),
        Op::Atomic(
            Atomic::Exchange(_, _, _, value) | Atomic::CompareWide { result: value, .. },
        ) => reg(value),
        _ => false,
    }
}
fn frame_memory(value: Operand) -> bool {
    value.memory().is_some_and(|m| m.base == Base::Frame)
}
fn accesses_stack(value: Operand) -> bool {
    value.memory().is_some_and(|m| {
        m.base == Base::Frame || m.base == Base::Register(R::RSP) || m.index == Some(R::RSP)
    })
}
// Visit MIR's explicit memory/source operands without materializing a second IR.
// Register destinations omitted by frame ordering are observed by extra_rsp.
fn visit(op: Op, mut operand: impl FnMut(Operand)) {
    let mut copied = op;
    super::frame_layout::operands(&mut copied, |value, _| operand(*value));
}
fn frameless_body(function: &Function) -> bool {
    if function.scratch_bytes != 0 || function.aligned_base != 0 {
        return false;
    }
    for inst in &function.instructions {
        if matches!(inst.op, Op::HostEh(effect) if !matches!(effect, super::landing_pad::Effect::Resume))
        {
            return false;
        }
        let mut frame = false;
        visit(inst.op, |value| frame |= accesses_stack(value));
        if frame {
            return false;
        }
    }
    true
}
pub(super) fn can_omit(function: &Function) -> bool {
    if !function.optimized
        || function.object_model != ObjectModel::Host
        || function.variadic
        || function.frame_policy.records_pushed
        || function.aligned_base != 0
    {
        return false;
    }
    fixed_stack(function)
}
pub(super) fn fixed_stack(function: &Function) -> bool {
    let mut areas = false;
    let mut accesses = 0_u32;
    for inst in &function.instructions {
        if matches!(inst.op, Op::StackAlloc(_) | Op::VaStart(_) | Op::VaArg(_))
            || matches!(inst.op, Op::Convert(c) if c.from == Type::I128 || c.to == Type::I128)
        {
            return false;
        }
        let mut rsp = extra_rsp(inst.op);
        let mut frame = false;
        visit(inst.op, |value| {
            rsp |= value.register() == Some(R::RSP);
            frame |= frame_memory(value);
        });
        if rsp {
            if !matches!(
                inst.op,
                Op::Scalar(ScalarOp::Binary(
                    Binary::Add | Binary::Sub,
                    _,
                    R::RSP,
                    Operand::Immediate(_)
                ))
            ) {
                return false;
            }
            areas = true;
        }
        accesses = accesses.wrapping_add(if let Op::HostEh(effect) = inst.op {
            u32::from(!matches!(effect, super::landing_pad::Effect::Resume))
        } else {
            u32::from(frame)
        });
    }
    !areas || accesses <= 64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::model::Program;
    fn host() -> (Program, Function) {
        let program =
            crate::lowir::test_program(b"function @f() -> void { block ^entry: return void }")
                .unwrap();
        let symbol = program.publications()[0];
        let function = super::super::value_selection::select_for_object(
            &program,
            symbol,
            program.function(program.symbol(symbol)).unwrap(),
            0,
            ObjectModel::Host,
        )
        .unwrap();
        (program, function)
    }
    #[test]
    fn virtual_frames_keep_real_rbp_distinct_and_displacements_keep_their_argument_origin() {
        let address = Addressing {
            omit: true,
            size: 32,
            incoming: 8,
            shift: 16,
        };
        assert!(matches!(
            address.resolve(Rm::frame(-8).displaced(8)),
            Rm::Memory {
                base: Some(R::RSP),
                disp: 48,
                ..
            }
        ));
        assert!(matches!(
            address.resolve(Rm::frame(0)),
            Rm::Memory {
                base: Some(R::RSP),
                disp: 40,
                ..
            }
        ));
        assert!(matches!(
            address.resolve(Rm::mem(R::RBP, 7)),
            Rm::Memory {
                base: Some(R::RBP),
                disp: 7,
                ..
            }
        ));
        let mut assembler = super::super::x86::Assembler::default();
        assembler.frame_addressing(address);
        assembler.lea(R::RAX, Rm::frame(-8)).unwrap();
        assert_eq!(assembler.code, [0x48, 0x8d, 0x44, 0x24, 40]);
    }
    #[test]
    fn fixed_stack_area_cap_counts_mir_instructions_and_rejects_other_rsp_uses() {
        let (_, mut function) = host();
        function.optimized = true;
        function.instructions.clear();
        for _ in 0..64 {
            function.instructions.push(super::super::mir::Instruction {
                volatile: false,
                op: Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
                location: None,
            });
        }
        function.instructions.push(super::super::mir::Instruction {
            volatile: false,
            op: Op::Scalar(ScalarOp::Binary(
                Binary::Sub,
                Type::I64,
                R::RSP,
                Operand::Immediate(16),
            )),
            location: None,
        });
        assert!(can_omit(&function));
        function.instructions.push(super::super::mir::Instruction {
            volatile: false,
            op: Op::Scalar(ScalarOp::Load(R::RAX, Type::I64, Operand::Frame(-8))),
            location: None,
        });
        assert!(!can_omit(&function));
        function.instructions.remove(64);
        assert!(can_omit(&function));
        function.instructions.push(super::super::mir::Instruction {
            volatile: false,
            op: Op::Scalar(ScalarOp::Store(
                Operand::Frame(-8),
                Type::I64,
                Operand::Register(R::RSP),
            )),
            location: None,
        });
        assert!(!can_omit(&function));
    }
    #[test]
    fn leaf_and_calling_reservations_keep_alignment_and_saved_rbp_rules() {
        let (program, mut function) = host();
        function.frame_policy.omit_pointer = true;
        assert_eq!(Plan::new(&function).reserve, 0);
        function.instructions.insert(
            0,
            super::super::mir::Instruction {
                volatile: false,
                op: Op::Call(super::super::mir::CallSite {
                    target: Operand::Symbol(program.publications()[0]),
                    arguments: 0,
                    stack: 0,
                    variadic: false,
                    no_unwind: true,
                    no_return: false,
                    landing: None,
                }),
                location: None,
            },
        );
        assert_eq!(Plan::new(&function).reserve, 8);
        function.preserve = 1 << R::RBP.code();
        function.stack_size = 16;
        assert_eq!(Plan::new(&function).reserve, 0);
    }
}
