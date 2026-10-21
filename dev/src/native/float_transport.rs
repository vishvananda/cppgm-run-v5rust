//! Bit-preserving floating ABI moves shared by MIR and encoding. See NOTICE.
use super::{
    image::Image,
    scalar_mir::Operand,
    x86::{Assembler, Register as R, Rm},
};
use crate::{
    lowir::{
        Error,
        literals::{self, FloatKind},
        model::Program,
        vocabulary::Type,
    },
    types::floating::FloatingScanner,
};
use std::io::Write;

#[derive(Clone, Copy)]
pub(super) struct FloatCopy {
    pub(super) ty: Type,
    pub(super) destination: Operand,
    pub(super) source: Operand,
}
impl FloatCopy {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(out, "    fmov.{} ", self.ty)?;
        self.destination.write(program, out)?;
        write!(out, ", ")?;
        if let Operand::Immediate(bits) = self.source {
            let kind = FloatKind::from_type(self.ty)
                .ok_or_else(|| std::io::Error::other("floating ABI move type"))?;
            let mut scanner = FloatingScanner::default();
            write!(
                out,
                "{}",
                literals::mir_text(&mut scanner, kind, bits)
                    .map_err(|_| std::io::Error::other("floating ABI bits"))?
            )
        } else {
            self.source.write(program, out)
        }
    }
    pub(super) fn rebase(mut self, begin: i32, bytes: u32) -> Self {
        self.source = self.source.rebase(begin, bytes);
        self.destination = self.destination.rebase(begin, bytes);
        self
    }
    pub(super) fn emit(self, a: &mut Assembler, image: &mut Image) -> Result<(), Error> {
        let prefix = if self.ty == Type::F32 { 0xf3 } else { 0xf2 };
        if let Some(destination) = self.destination.register() {
            if let Some(source) = self.source.register() {
                if destination != source {
                    a.sse(0, 0x28, destination, Rm::Reg(source))?;
                }
            } else if let Operand::Immediate(bits) = self.source {
                if bits & u128::from(u64::MAX) == 0 {
                    a.sse(0, 0x57, destination, Rm::Reg(destination))?;
                } else {
                    let kind = FloatKind::from_type(self.ty).ok_or(Error::Limit)?;
                    let source = super::emission::constant(image, kind, bits)?;
                    a.sse(prefix, 0x10, destination, source)?;
                }
            } else {
                a.sse(prefix, 0x10, destination, self.source.rm()?)?;
            }
            return Ok(());
        }
        let to = self.destination.rm()?;
        if let Some(source) = self.source.register() {
            return a.sse(prefix, 0x11, source, to);
        }
        let width = if self.ty == Type::F32 { 4 } else { 8 };
        let scratch = [R::R11, R::R10]
            .into_iter()
            .find(|r| (self.destination.mask() | self.source.mask()) & (1 << r.code()) == 0)
            .ok_or(Error::Limit)?;
        if let Operand::Immediate(bits) = self.source {
            let value = super::mir::low_word(bits)?;
            if width == 4 || i32::try_from(value).is_ok() {
                a.store_imm(to, value, width)?;
            } else {
                a.mov_ri(scratch, value)?;
                a.store(to, scratch, width)?;
            }
        } else {
            a.load(scratch, self.source.rm()?, width, false)?;
            a.store(to, scratch, width)?;
        }
        Ok(())
    }
}
