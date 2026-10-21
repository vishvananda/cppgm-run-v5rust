//! System V variadic register-save areas and `va_list` initialization. See NOTICE.
use super::{
    abi::Layout,
    float_transport::FloatCopy,
    mir::{Binding, BindingKind, Instruction, Op},
    placement::Placement,
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    x86::{Assembler, Register as R},
};
use crate::lowir::{
    Error,
    model::{Program, Value},
    vocabulary::Type,
};
use std::io::Write;
#[derive(Clone, Copy)]
pub(super) struct SaveArea {
    offset: i32,
    gp: u32,
    fp: u32,
    overflow: i32,
}
#[derive(Clone, Copy)]
pub(super) struct Start {
    list: Operand,
    save: Operand,
    gp: u32,
    fp: u32,
    overflow: i32,
}
impl Start {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(out, "    va_start ")?;
        self.list.write(program, out)?;
        write!(out, ", ")?;
        self.save.write(program, out)
    }
    pub(super) fn emit(self, a: &mut Assembler, begin: i32, bytes: u32) -> Result<(), Error> {
        let list = self.list.rebase(begin, bytes);
        let save = self.save.rebase(begin, bytes);
        let scratch = [R::R11, R::R10]
            .into_iter()
            .find(|r| (list.mask() | save.mask()) & (1 << r.code()) == 0)
            .ok_or(Error::Limit)?;
        let list = list.rm()?;
        a.store_imm(list, i64::from(self.gp), 4)?;
        a.store_imm(list.displaced(4), i64::from(self.fp), 4)?;
        a.lea(scratch, Operand::Frame(self.overflow).rm()?)?;
        a.store(list.displaced(8), scratch, 8)?;
        a.lea(scratch, save.rm()?)?;
        a.store(list.displaced(16), scratch, 8)
    }
}
impl SaveArea {
    pub(super) fn allocate(placement: &mut Placement, layout: &Layout) -> Result<Self, Error> {
        Ok(Self {
            offset: placement.frame_alloc(176, 16)?,
            gp: u32::try_from(layout.gpr_count()).map_err(|_| Error::Limit)? * 8,
            fp: 48 + 16 * u32::from(layout.vector_count()),
            overflow: 16_i32.wrapping_add(layout.stack.cast_signed()),
        })
    }
    pub(super) fn entry(self, mir: &mut super::mir::Function) -> Result<(), Error> {
        mir.bindings.push(Binding {
            name: None,
            ty: Type::Object {
                bytes: 176,
                align: 4,
            },
            offset: self.offset,
            kind: BindingKind::Temp,
        });
        for (index, register) in [R::RDI, R::RSI, R::RDX, R::RCX, R::R8, R::R9]
            .into_iter()
            .enumerate()
        {
            mir.instructions.push(Instruction {
                op: Op::Scalar(ScalarOp::Store(
                    Operand::Frame(
                        self.offset
                            .wrapping_add(8 * i32::try_from(index).map_err(|_| Error::Limit)?),
                    ),
                    Type::I64,
                    Operand::Register(register),
                )),
                location: None,
            });
        }
        for index in 0_u8..8 {
            let register = R::xmm(index)?;
            mir.instructions.push(Instruction {
                op: Op::FloatCopy(FloatCopy {
                    ty: Type::I128,
                    destination: Operand::Frame(
                        self.offset.wrapping_add(48 + 16 * i32::from(index)),
                    ),
                    source: Operand::Register(register),
                }),
                location: None,
            });
        }
        Ok(())
    }
}
impl Selection<'_> {
    pub(super) fn va_start(&mut self, list: Value) -> Result<(), Error> {
        let save = self
            .placement
            .variadic
            .ok_or_else(|| Error::Construction("va_start outside a variadic function".into()))?;
        let list = self.address(list, R::R10)?;
        self.mir.instructions.push(Instruction {
            op: Op::VaStart(Start {
                list,
                save: Operand::Frame(save.offset),
                gp: save.gp,
                fp: save.fp,
                overflow: save.overflow,
            }),
            location: self.location,
        });
        Ok(())
    }
}
