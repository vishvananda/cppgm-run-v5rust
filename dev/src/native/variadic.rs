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
    model::{Program, TempId, Value},
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
#[derive(Clone, Copy)]
pub(super) struct Argument {
    ty: Type,
    destination: R,
    list: Operand,
}
impl Argument {
    pub(super) fn map_list(&mut self, mut visit: impl FnMut(&mut Operand, u32)) {
        visit(&mut self.list, u32::from(super::scalar_mir::bytes(self.ty)));
    }
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(
            out,
            "    va_arg.{} {}, ",
            self.ty,
            self.destination.spelling()
        )?;
        self.list.write(program, out)
    }
    pub(super) fn emit(self, a: &mut Assembler, begin: i32, bytes: u32) -> Result<(), Error> {
        use super::x86::Rm;
        let floating = self.ty == Type::F64;
        let scratch_result = !floating && matches!(self.destination, R::R10 | R::R11);
        let address = if floating {
            R::R10
        } else if scratch_result {
            R::RAX
        } else {
            self.destination
        };
        let offset = if floating { R::RAX } else { R::R10 };
        a.lea(R::R11, self.list.rebase(begin, bytes).rm()?)?;
        if floating || scratch_result {
            a.push(R::RAX);
        }
        let (field, limit, step) = if floating { (4, 176, 16) } else { (0, 48, 8) };
        let stack = a.label()?;
        let load = a.label()?;
        a.load(offset, Rm::mem(R::R11, field), 4, false)?;
        a.alu_ri(7, Rm::Reg(offset), limit, 4)?;
        a.branch(Some(3), stack, true);
        a.load(address, Rm::mem(R::R11, 16), 8, false)?;
        a.alu_rr(0, address, Rm::Reg(offset), 8)?;
        a.alu_ri(0, Rm::Reg(offset), step, 4)?;
        a.store(Rm::mem(R::R11, field), offset, 4)?;
        a.branch(None, load, true);
        a.bind(stack);
        a.load(address, Rm::mem(R::R11, 8), 8, false)?;
        a.lea(offset, Rm::mem(address, 8))?;
        a.store(Rm::mem(R::R11, 8), offset, 8)?;
        a.bind(load);
        if floating {
            a.pop(R::RAX);
            return a.sse(0xf2, 0x10, self.destination, Rm::mem(address, 0));
        }
        a.load(
            self.destination,
            Rm::mem(address, 0),
            super::scalar_mir::bytes(self.ty),
            super::scalar_mir::signed(self.ty) || self.ty == Type::Ptr,
        )?;
        if scratch_result {
            a.pop(R::RAX);
        }
        Ok(())
    }
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
    pub(super) fn va_arg(&mut self, result: TempId, ty: Type, list: Value) -> Result<(), Error> {
        let list = self.address(list, R::R10)?;
        let destination = self.result(result, [None, None], list.mask());
        self.mir.instructions.push(Instruction {
            op: Op::VaArg(Argument {
                ty,
                destination,
                list,
            }),
            location: self.location,
        });
        self.finish(result, destination)
    }
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
