//! Fixed bulk-memory policy, typed MIR, selection and exact O0 encoding. See NOTICE.
use super::{
    memory::Base,
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

pub(super) fn alignment(log: u8) -> u32 {
    1_u32.wrapping_shl(u32::from(log))
}
pub(super) fn direct_copy(bytes: u64, align: u32) -> bool {
    bytes <= 32 || bytes <= 64 && align >= 8
}
pub(super) fn direct_zero(bytes: u64) -> bool {
    matches!(bytes, 1 | 2 | 4 | 8 | 16)
}
pub(super) fn object_clobbers(ty: Type) -> Result<u32, Error> {
    Ok(
        if direct_copy(
            u64::from(super::abi::value_bytes(ty)?),
            super::abi::value_align(ty)?,
        ) {
            0
        } else {
            (1 << R::RDI.code()) | (1 << R::RSI.code()) | (1 << R::RCX.code())
        },
    )
}
fn target_bytes(bytes: u64) -> u32 {
    let [a, b, c, d, ..] = bytes.to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}
#[derive(Clone, Copy)]
pub(super) struct Bulk {
    pub(super) destination: Operand,
    pub(super) source: Option<Operand>,
    bytes: u32,
    align: u32,
}
impl Bulk {
    pub(super) fn bytes(self) -> u32 {
        self.bytes
    }
    pub(super) fn new(
        destination: Operand,
        source: Option<Operand>,
        bytes: u64,
        align: u32,
    ) -> Self {
        Self {
            destination,
            source,
            bytes: target_bytes(bytes),
            align,
        }
    }
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(
            out,
            "    {}_bytes {}x{}, ",
            if self.source.is_some() {
                "copy"
            } else {
                "zero"
            },
            self.bytes,
            self.align
        )?;
        self.destination.write(program, out)?;
        if let Some(source) = self.source {
            write!(out, ", ")?;
            source.write(program, out)?;
        }
        Ok(())
    }
    pub(super) fn rebase(mut self, begin: i32, bytes: u32) -> Self {
        self.destination = self.destination.rebase(begin, bytes);
        self.source = self.source.map(|source| source.rebase(begin, bytes));
        self
    }
    pub(super) fn emit(self, assembler: &mut Assembler) -> Result<(), Error> {
        if let Some(source) = self.source {
            self.copy(assembler, source)
        } else {
            self.zero(assembler)
        }
    }
    fn copy(self, a: &mut Assembler, source: Operand) -> Result<(), Error> {
        if !direct_copy(u64::from(self.bytes), self.align) {
            if source.mask() & (1 << R::RDI.code()) == 0 {
                address_into(a, R::RDI, self.destination)?;
                address_into(a, R::RSI, source)?;
            } else if self.destination.mask() & (1 << R::RSI.code()) == 0 {
                address_into(a, R::RSI, source)?;
                address_into(a, R::RDI, self.destination)?;
            } else {
                address_into(a, R::RCX, self.destination)?;
                address_into(a, R::RSI, source)?;
                a.mov_rr(R::RDI, R::RCX)?;
            }
            a.mov_ri(R::RCX, i64::from(self.bytes))?;
            a.byte(0xf3);
            a.byte(0xa4);
            return Ok(());
        }
        let mut to = location(self.destination)?;
        let mut from = location(source)?;
        let vector = R::xmm(15)?;
        let mut offset = 0_u32;
        while self.bytes - offset >= 16 {
            a.sse(0xf3, 0x6f, vector, from.displaced(offset.cast_signed()))?;
            a.sse(0xf3, 0x7f, vector, to.displaced(offset.cast_signed()))?;
            offset += 16;
        }
        if offset == self.bytes {
            return Ok(());
        }
        let spare = [R::R11, R::R10]
            .into_iter()
            .find(|&reg| !to.uses(reg) && !from.uses(reg));
        let saved = spare.is_none();
        let spare = spare
            .or_else(|| {
                [R::RAX, R::RCX, R::RDX]
                    .into_iter()
                    .find(|&reg| !to.uses(reg) && !from.uses(reg))
            })
            .ok_or(Error::Limit)?;
        if saved {
            a.push(spare);
            for operand in [&mut to, &mut from] {
                if let Rm::Memory {
                    base: Some(R::RSP),
                    disp,
                    ..
                } = operand
                {
                    *disp = disp.wrapping_add(8);
                }
            }
        }
        for width in [8_u8, 4, 2, 1] {
            while self.bytes - offset >= u32::from(width) {
                a.load(spare, from.displaced(offset.cast_signed()), width, false)?;
                a.store(to.displaced(offset.cast_signed()), spare, width)?;
                offset += u32::from(width);
            }
        }
        if saved {
            a.pop(spare);
        }
        Ok(())
    }
    fn zero(self, a: &mut Assembler) -> Result<(), Error> {
        let to = location(self.destination)?;
        if self.bytes == 16 {
            let vector = R::xmm(15)?;
            a.sse(0x66, 0xef, vector, Rm::Reg(vector))?;
            return a.sse(0xf3, 0x7f, vector, to);
        }
        let mut cost = 0_u32;
        let mut offset = 0_u32;
        for width in [8_u8, 4, 2, 1] {
            while self.bytes - offset >= u32::from(width) {
                cost =
                    cost.wrapping_add(zero_store_bytes(to.displaced(offset.cast_signed()), width));
                offset += u32::from(width);
            }
        }
        let setup = if let Some(reg) = self.destination.register() {
            u32::from(reg != R::RDI) * 3
        } else {
            4 + rm_extra(to)
        } + 2
            + 5
            + 2;
        if direct_zero(u64::from(self.bytes)) || cost <= setup {
            let mut offset = 0_u32;
            for width in [8_u8, 4, 2, 1] {
                while self.bytes - offset >= u32::from(width) {
                    a.store_imm(to.displaced(offset.cast_signed()), 0, width)?;
                    offset += u32::from(width);
                }
            }
            return Ok(());
        }
        address_into(a, R::RDI, self.destination)?;
        a.op(0, false, 0x33, 1, R::RAX.code(), Rm::Reg(R::RAX), false, 0)?;
        a.mov_ri(R::RCX, i64::from(self.bytes))?;
        a.byte(0xf3);
        a.byte(0xaa);
        Ok(())
    }
}
fn location(operand: Operand) -> Result<Rm, Error> {
    if let Some(reg) = operand.register() {
        Ok(Rm::mem(reg, 0))
    } else {
        operand.rm()
    }
}
fn address_into(a: &mut Assembler, destination: R, operand: Operand) -> Result<(), Error> {
    if let Some(reg) = operand.register() {
        a.mov_rr(destination, reg)
    } else {
        a.lea(destination, location(operand)?)
    }
}
fn rm_extra(rm: Rm) -> u32 {
    let Rm::Memory {
        base,
        index,
        disp,
        target,
        ..
    } = rm
    else {
        return 0;
    };
    let Some(base) = base.filter(|_| target.is_none()) else {
        return if base.is_none() && target.is_none() {
            5
        } else {
            4
        };
    };
    let extra = u32::from(index.is_some() || base.code() & 7 == 4);
    if disp == 0 && base.code() & 7 != 5 {
        extra
    } else {
        extra + if i8::try_from(disp).is_ok() { 1 } else { 4 }
    }
}
fn zero_store_bytes(rm: Rm, width: u8) -> u32 {
    let rex = matches!(rm, Rm::Memory { base, index, .. } if base.is_some_and(|r|r.code()&8!=0)||index.is_some_and(|r|r.code()&8!=0));
    u32::from(width == 2)
        + u32::from(width == 8 || rex)
        + 2
        + rm_extra(rm)
        + if width == 8 { 4 } else { u32::from(width) }
}
fn bulk_address(operand: Operand) -> Operand {
    if let Some(memory) = operand.memory()
        && memory.index.is_none()
        && memory.displacement == 0
        && let Base::Register(reg) = memory.base
    {
        Operand::Register(reg)
    } else {
        operand
    }
}
impl Selection<'_> {
    fn object_transfer(
        &mut self,
        ty: Type,
        destination: Operand,
        source: Operand,
    ) -> Result<(), Error> {
        self.mir.instructions.push(super::mir::Instruction {
            op: super::mir::Op::Bulk(Bulk::new(
                destination,
                Some(source),
                u64::from(super::abi::value_bytes(ty)?),
                super::abi::value_align(ty)?,
            )),
            location: self.location,
        });
        Ok(())
    }
    pub(super) fn object_load(
        &mut self,
        result: TempId,
        ty: Type,
        address: Value,
    ) -> Result<(), Error> {
        let source = self.address(address, R::R10)?;
        let destination = Operand::Frame(self.placement.home(self.body, result, &mut self.mir)?);
        self.object_transfer(ty, destination, source)
    }
    pub(super) fn object_store(
        &mut self,
        ty: Type,
        value: Value,
        address: Value,
    ) -> Result<(), Error> {
        let source = self.aggregate_operand(value, ty)?;
        if !source.is_memory() && source.register().is_none() {
            return Err(Error::Diagnostic(
                "object store requires a storage operand".into(),
            ));
        }
        let destination = self.address(address, R::R10)?;
        self.object_transfer(ty, destination, source)
    }
    pub(super) fn object_copy(
        &mut self,
        result: TempId,
        ty: Type,
        value: Value,
    ) -> Result<(), Error> {
        let source = self.aggregate_operand(value, ty)?;
        if !source.is_memory() && source.register().is_none() {
            return Err(Error::Diagnostic(
                "object copy requires a storage operand".into(),
            ));
        }
        let destination = Operand::Frame(self.placement.home(self.body, result, &mut self.mir)?);
        self.object_transfer(ty, destination, source)
    }
    pub(super) fn bulk(
        &mut self,
        bytes: u64,
        log: u8,
        source: Option<crate::lowir::model::Value>,
        destination: crate::lowir::model::Value,
    ) -> Result<(), Error> {
        let source = source
            .map(|source| self.address(source, R::R11).map(bulk_address))
            .transpose()?;
        let destination = bulk_address(self.address(destination, R::R10)?);
        if source.is_some_and(|s| {
            s.frame_offset().is_some() && s.frame_offset() == destination.frame_offset()
        }) {
            return Ok(());
        }
        self.mir.instructions.push(super::mir::Instruction {
            op: super::mir::Op::Bulk(Bulk::new(destination, source, bytes, alignment(log))),
            location: self.location,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Facts, InstructionKind as I},
        vocabulary::Type,
    };
    #[test]
    fn object_values_cross_blocks_in_function_owned_homes() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "f", &[("source", Type::Ptr)], Type::Ptr)?;
        let ty = Type::Object {
            bytes: 96,
            align: 3,
        };
        let destination = builder.slot("destination", ty)?;
        builder.block("entry")?;
        let source = builder.parameter(0)?;
        let object = builder.value(I::Load(Facts::default(), ty, source), "object")?;
        let next = builder.block_id("next")?;
        builder.effect(I::Jump(next))?;
        builder.block("next")?;
        builder.effect(I::Store(Facts::default(), ty, object, destination))?;
        let address = builder.value(I::Addr(destination), "address")?;
        builder.effect(I::Return(Type::Ptr, Some(address)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let mir = super::super::value_selection::select(&program, symbol, function, 0)?;
        assert_eq!(mir.homes, 1);
        assert_eq!(mir.global_registers, 0);
        assert_eq!(mir.spills, 0);
        assert_eq!(
            mir.instructions
                .iter()
                .filter(|inst| matches!(inst.op, super::super::mir::Op::Bulk(_)))
                .count(),
            2
        );
        Ok(())
    }
    #[test]
    fn direct_copy_borrows_a_register_and_adjusts_stack_relative_storage() -> Result<(), Error> {
        use super::super::memory::Memory;
        let destination = Operand::Memory(Memory::new(Base::Register(R::RSP), 16));
        let mut source = Memory::new(Base::Register(R::R10), 0);
        source.index = Some(R::R11);
        let mut assembler = Assembler::default();
        Bulk::new(destination, Some(Operand::Memory(source)), 4, 4).emit(&mut assembler)?;
        // push rax; mov eax,[r10+r11]; mov [rsp+24],eax; pop rax.
        assert_eq!(
            assembler.code,
            [0x50, 0x43, 0x8b, 0x04, 0x1a, 0x89, 0x44, 0x24, 0x18, 0x58]
        );
        Ok(())
    }
}
