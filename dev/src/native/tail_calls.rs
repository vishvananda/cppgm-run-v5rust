//! Host sibling-call admission over final native block ranges. See NOTICE.
use super::{
    memory::Base,
    mir::{Function, Op},
    scalar_mir::ScalarOp,
    x86::Register as R,
};

// branch_layout.cpp::form_tail_jumps scans the complete instruction inventory,
// including instructions outside kept ranges. A frame address may have escaped
// before the final call; ordinary loads/stores of frame homes do not escape it.
fn exposes_frame(op: Op) -> bool {
    if matches!(op, Op::Nop) {
        return false;
    }
    let lea = matches!(op, Op::Scalar(ScalarOp::Lea(..) | ScalarOp::Lea32(..)));
    let mut exposed = !matches!(op, Op::Return(_)) && super::frame_encoding::extra_rsp(op);
    let mut copied = op;
    super::frame_layout::operands(&mut copied, |value, _| {
        exposed |= value.register() == Some(R::RSP);
        if lea && let Some(memory) = value.memory() {
            exposed |= matches!(memory.base, Base::Frame | Base::Register(R::RSP))
                || memory.index.is_some_and(|r| r == R::RSP);
        }
    });
    exposed
}

pub(super) fn form(function: &mut Function) {
    if function.variadic
        || function.frame_policy.records_pushed
        || function.instructions.iter().any(|i| exposes_frame(i.op))
    {
        return;
    }
    for block in 0..function.blocks.len() {
        let range = function.blocks[block].range.clone();
        let Some(last) = range.end.checked_sub(1).filter(|&i| i >= range.start) else {
            continue;
        };
        let pair = if range.len() >= 2
            && matches!(function.instructions[last].op, Op::Return(_))
            && matches!(function.instructions[last - 1].op, Op::Call(_))
        {
            Some((last - 1, last))
        } else if matches!(function.instructions[last].op, Op::Call(_)) {
            function.blocks.get(block + 1).and_then(|next| {
                (!next.range.is_empty()
                    && matches!(function.instructions[next.range.start].op, Op::Return(_)))
                .then_some((last, next.range.start))
            })
        } else {
            None
        };
        let Some((call, ret)) = pair else {
            continue;
        };
        let Op::Call(site) = function.instructions[call].op else {
            continue;
        };
        let Op::Return(result) = function.instructions[ret].op else {
            continue;
        };
        if result.is_some_and(|r| r != R::RAX && r != R::XMM0)
            || site.landing.is_some()
            || site.stack != 0
            || site.no_return
        {
            continue;
        }
        function.instructions[call].op = Op::TailJump(site);
        if ret == last {
            function.blocks[block].range.end -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        memory::Memory,
        mir::{Block, CallSite, Instruction},
        scalar_mir::Operand,
    };
    use super::*;
    use crate::lowir::{Error, model::Program, vocabulary::Type};

    fn function() -> Result<Function, Error> {
        let program: Program =
            crate::lowir::test_program(b"function @f() -> void { block ^entry: return void }")?;
        let symbol = program.publications()[0];
        super::super::value_selection::select_for_object(
            &program,
            symbol,
            program
                .function(program.symbol(symbol))
                .ok_or(Error::Limit)?,
            0,
            super::super::object_compile::ObjectModel::Host,
        )
    }
    fn instruction(op: Op) -> Instruction {
        Instruction {
            op,
            volatile: false,
            location: None,
        }
    }
    fn call() -> CallSite {
        CallSite {
            target: Operand::Register(R::R12),
            arguments: 0,
            stack: 0,
            variadic: false,
            no_unwind: false,
            no_return: false,
            landing: None,
        }
    }
    #[test]
    fn sibling_calls_consume_only_their_own_return_and_keep_fallthrough_returns()
    -> Result<(), Error> {
        let mut f = function()?;
        f.instructions = vec![
            instruction(Op::Call(call())),
            instruction(Op::Return(Some(R::RAX))),
        ];
        f.blocks[0].range = 0..2;
        form(&mut f);
        assert!(matches!(f.instructions[0].op, Op::TailJump(_)));
        assert_eq!(f.blocks[0].range, 0..1);
        let name = f.blocks[0].name;
        f.instructions[0].op = Op::Call(call());
        f.blocks.push(Block {
            name,
            range: 1..2,
            landing: false,
        });
        form(&mut f);
        assert!(matches!(f.instructions[0].op, Op::TailJump(_)));
        assert_eq!(f.blocks[1].range, 1..2);
        // A nop breaks adjacency even when no code is emitted for it.
        f.blocks.truncate(1);
        f.instructions = vec![
            instruction(Op::Call(call())),
            instruction(Op::Nop),
            instruction(Op::Return(None)),
        ];
        f.blocks[0].range = 0..3;
        form(&mut f);
        assert!(matches!(f.instructions[0].op, Op::Call(_)));
        Ok(())
    }
    #[test]
    fn escaping_frames_stack_arguments_and_noncanonical_returns_keep_calls() -> Result<(), Error> {
        for guard in 0..10 {
            let mut f = function()?;
            let mut site = call();
            let mut result = Some(R::RAX);
            let prefix = match guard {
                0 => {
                    f.variadic = true;
                    Op::Nop
                }
                1 => {
                    f.frame_policy.records_pushed = true;
                    Op::Nop
                }
                2 => {
                    site.stack = 8;
                    Op::Nop
                }
                3 => {
                    site.no_return = true;
                    Op::Nop
                }
                4 => {
                    result = Some(R::RDX);
                    Op::Nop
                }
                5 => Op::Scalar(ScalarOp::Lea(R::RDI, Operand::Frame(-8))),
                6 => Op::Scalar(ScalarOp::Lea(
                    R::RDI,
                    Operand::Memory(Memory::new(Base::Register(R::RSP), 0)),
                )),
                7 => Op::Scalar(ScalarOp::Move(R::RSP, Operand::Register(R::RAX))),
                8 => Op::Scalar(ScalarOp::Store(
                    Operand::Frame(-8),
                    Type::I64,
                    Operand::Register(R::RSP),
                )),
                _ => {
                    site.landing = Some(super::super::region_flow::LandingId::new(0)?);
                    Op::Nop
                }
            };
            f.instructions = vec![
                instruction(prefix),
                instruction(Op::Call(site)),
                instruction(Op::Return(result)),
            ];
            f.blocks[0].range = 1..3;
            form(&mut f);
            assert!(matches!(f.instructions[1].op, Op::Call(_)), "guard {guard}");
        }
        // Frame homes and incoming stack loads do not expose an address.
        let mut f = function()?;
        f.instructions = vec![
            instruction(Op::Scalar(ScalarOp::Load(
                R::RDI,
                Type::I64,
                Operand::Frame(16),
            ))),
            instruction(Op::Call(call())),
            instruction(Op::Return(Some(R::XMM0))),
        ];
        f.blocks[0].range = 0..3;
        form(&mut f);
        assert!(matches!(f.instructions[1].op, Op::TailJump(_)));
        Ok(())
    }
}
