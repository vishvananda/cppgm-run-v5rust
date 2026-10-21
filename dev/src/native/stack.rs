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
            // Selection has checked the native nonnegative adjustment range.
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
            let source = self.size.register().ok_or_else(|| {
                Error::Construction("stack allocation size is not a register".into())
            })?;
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
        // Read the size at its own storage width before choosing the result.
        let mut size = if let super::value_facts::Alias::Temporary(temp) =
            self.facts.resolve(self.body, value)
            && self.body.temp(temp).ty == Some(Type::I128)
        {
            self.temp_operand(temp)?
        } else {
            self.operand(value, Type::I64, R::R10)?
        };
        if let Operand::Immediate(bits) = size {
            let bytes = low_word(bits)?;
            if !(0..=0x7fff_fff0).contains(&bytes) {
                return Err(Error::Diagnostic(
                    "constant stack_alloc size exceeds the native stack adjustment range".into(),
                ));
            }
        } else if size.register().is_none() {
            self.load(R::R10, size, Type::I64);
            size = Operand::Register(R::R10);
        }
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
