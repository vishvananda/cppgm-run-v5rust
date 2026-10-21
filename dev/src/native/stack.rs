//! Dynamic function-stack allocation and target alignment. See NOTICE.
use super::{
    mir::{Instruction, Op, low_word},
    scalar_mir::Operand,
    scalar_selection::Selection,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::{Program, TempId, Value},
    vocabulary::Type,
};
use std::io::Write;

#[derive(Clone, Copy)]
pub(super) struct Allocation {
    destination: R,
    size: Operand,
}
impl Allocation {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(out, "    stack_alloc {}, ", self.destination.spelling())?;
        self.size.write(program, out)
    }
    pub(super) fn emit(self, a: &mut Assembler) -> Result<(), Error> {
        if let Operand::Immediate(bits) = self.size {
            let size = low_word(bits)?.wrapping_add(15) & !15;
            // The frozen encoder rejects non-encodable imm64 ALU operands.
            i32::try_from(size).map_err(|_| {
                Error::Construction("immediate does not fit its instruction encoding".into())
            })?;
            a.alu_ri(5, Rm::Reg(R::RSP), size, 8)?;
        } else {
            let scratch = [R::R11, R::R10]
                .into_iter()
                .find(|r| {
                    (self.size.mask() | (1 << self.destination.code())) & (1 << r.code()) == 0
                })
                .ok_or(Error::Limit)?;
            // emitter.cpp reads Operand::reg even for a memory size. Keep
            // that observable raw-field encoding: kFrame (0xfd) encodes r13,
            // and the absent base (0xff) encodes r15. Do not dereference it.
            let source = match self.size {
                Operand::Register(r) => r,
                Operand::Frame(_) => R::R13,
                Operand::Memory(memory) => match memory.base {
                    super::memory::Base::Register(r) => r,
                    super::memory::Base::Frame => R::R13,
                    _ => R::R15,
                },
                _ => R::R15,
            };
            a.mov_rr(scratch, source)?;
            a.alu_ri(0, Rm::Reg(scratch), 15, 8)?;
            a.alu_ri(4, Rm::Reg(scratch), -16, 8)?;
            a.alu_rr(5, R::RSP, Rm::Reg(scratch), 8)?;
        }
        a.mov_rr(self.destination, R::RSP)
    }
}
impl Selection<'_> {
    pub(super) fn stack_alloc(&mut self, result: TempId, value: Value) -> Result<(), Error> {
        // rm_operand widens GPR homes at their own width; a memory-class
        // i128 keeps its location, including the frozen raw-field behavior.
        let size = if let super::value_facts::Alias::Temporary(temp) =
            self.facts.resolve(self.body, value)
            && self.body.temp(temp).ty == Some(Type::I128)
        {
            self.temp_operand(temp)?
        } else {
            self.operand(value, Type::I64, R::R10)?
        };
        let exclude = if size.register().is_some() {
            0
        } else {
            size.mask()
        };
        let destination = self.result(result, [None, None], exclude);
        self.mir.instructions.push(Instruction {
            op: Op::StackAlloc(Allocation { destination, size }),
            location: self.location,
        });
        self.finish(result, destination)
    }
}
