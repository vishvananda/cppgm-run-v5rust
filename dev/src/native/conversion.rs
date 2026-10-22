//! Typed primitive floating conversions and their exact SSE/x87 forms. See NOTICE.
use super::{
    floating_mir::{self, source, x87_load},
    image::Image,
    scalar_mir::Operand,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{Error, model::Program, vocabulary::Type};
use std::io::Write;
#[derive(Clone, Copy)]
pub(super) enum Kind {
    SignedToFloat,
    UnsignedToFloat,
    FloatToSigned,
    FloatToUnsigned,
    Extend,
    Truncate,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::SignedToFloat => "sitofp",
            Self::UnsignedToFloat => "uitofp",
            Self::FloatToSigned => "fptosi",
            Self::FloatToUnsigned => "fptoui",
            Self::Extend => "fpext",
            Self::Truncate => "fptrunc",
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Conversion {
    pub(super) kind: Kind,
    pub(super) from: Type,
    pub(super) to: Type,
    pub(super) destination: Operand,
    pub(super) source: Operand,
}
fn prefix(ty: Type) -> u8 {
    if ty == Type::F32 { 0xf3 } else { 0xf2 }
}
impl Conversion {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        write!(out, "    {}.{}.{} ", self.kind.name(), self.from, self.to)?;
        self.destination.write(program, out)?;
        write!(out, ", ")?;
        if self.from.float_rank() != 0 {
            floating_mir::write_operand(program, out, self.from, self.source)
        } else {
            self.source.write(program, out)
        }
    }
    pub(super) fn rebase(mut self, begin: i32, bytes: u32) -> Self {
        self.source = self.source.rebase(begin, bytes);
        self.destination = self.destination.rebase(begin, bytes);
        self
    }
    pub(super) fn emit(
        self,
        a: &mut Assembler,
        image: &mut Image,
        scratch: i32,
        runtime: Option<super::runtime::Symbols>,
    ) -> Result<(), Error> {
        if self.from == Type::I128 || self.to == Type::I128 {
            return self.emit_wide(a, image, runtime.ok_or(Error::Limit)?);
        }
        let scratch = Rm::frame(scratch);
        match self.kind {
            Kind::SignedToFloat => {
                let s = self.source.register().ok_or(Error::Limit)?;
                if self.to == Type::F80 {
                    a.store(scratch, s, 8)?;
                    a.x87(0xdf, 5, scratch)?;
                    a.x87(0xdb, 7, self.destination.rm()?)
                } else {
                    let d = self.destination.register().ok_or(Error::Limit)?;
                    a.sse(0, 0x57, d, Rm::Reg(d))?;
                    a.sse_wide(prefix(self.to), 0x2a, d, Rm::Reg(s))
                }
            }
            Kind::UnsignedToFloat => self.unsigned_to_float(a, image, scratch),
            Kind::FloatToSigned => {
                let d = self.destination.register().ok_or(Error::Limit)?;
                if self.from == Type::F80 {
                    x87_load(a, image, self.source, self.from)?;
                    a.x87(0xdd, 1, scratch)?;
                    a.load(d, scratch, 8, false)
                } else {
                    let from = source(self.source, self.from, image)?;
                    a.sse_wide(prefix(self.from), 0x2c, d, from)
                }
            }
            Kind::FloatToUnsigned => self.float_to_unsigned(a, image, scratch),
            Kind::Extend => {
                if self.to == Type::F80 {
                    if let Some(s) = self.source.register() {
                        a.sse(prefix(self.from), 0x11, s, scratch)?;
                        a.x87(if self.from == Type::F64 { 0xdd } else { 0xd9 }, 0, scratch)?;
                    } else {
                        x87_load(a, image, self.source, self.from)?;
                    }
                    a.x87(0xdb, 7, self.destination.rm()?)
                } else {
                    let from = source(self.source, self.from, image)?;
                    a.sse(
                        0xf3,
                        0x5a,
                        self.destination.register().ok_or(Error::Limit)?,
                        from,
                    )
                }
            }
            Kind::Truncate => {
                // The frozen same-width f80 truncation keeps a memory MIR
                // destination, but SSE takes its raw kFrame field & 15.
                let d = if self.to == Type::F80 && self.destination.is_memory() {
                    R::xmm(0xfd & 15)?
                } else {
                    self.destination.register().ok_or(Error::Limit)?
                };
                if self.from == Type::F80 {
                    x87_load(a, image, self.source, self.from)?;
                    a.x87(if self.to == Type::F64 { 0xdd } else { 0xd9 }, 3, scratch)?;
                    a.sse(prefix(self.to), 0x10, d, scratch)
                } else {
                    let from = source(self.source, self.from, image)?;
                    a.sse(0xf2, 0x5a, d, from)
                }
            }
        }
    }
    fn emit_wide(
        self,
        a: &mut Assembler,
        image: &mut Image,
        runtime: super::runtime::Symbols,
    ) -> Result<(), Error> {
        let to_float = self.from == Type::I128;
        let floating = if to_float { self.to } else { self.from };
        let routine = (if to_float { 0 } else { 6 })
            + (if matches!(self.kind, Kind::UnsignedToFloat | Kind::FloatToUnsigned) {
                3
            } else {
                0
            })
            + match floating {
                Type::F32 => 0,
                Type::F64 => 1,
                Type::F80 => 2,
                _ => return Err(Error::Limit),
            };
        let call = runtime.float_conversion[routine];
        if to_float {
            super::wide_mir::half(a, R::RDI, self.source, false)?;
            super::wide_mir::half(a, R::RSI, self.source, true)?;
            a.call_symbol(call)?;
            if self.to == Type::F80 {
                a.x87(0xdb, 7, self.destination.rm()?)?;
            } else {
                let dst = self.destination.register().ok_or(Error::Limit)?;
                let result = R::xmm(0)?;
                if dst != result {
                    a.sse(0, 0x28, dst, Rm::Reg(result))?;
                }
            }
            return Ok(());
        }
        if self.from == Type::F80 {
            x87_load(a, image, self.source, self.from)?;
            a.alu_ri(5, Rm::Reg(R::RSP), 16, 8)?;
            a.x87(0xdb, 7, Rm::mem(R::RSP, 0))?;
            a.call_symbol(call)?;
            a.alu_ri(0, Rm::Reg(R::RSP), 16, 8)?;
        } else {
            let input = R::xmm(0)?;
            if self.source.register() != Some(input) {
                a.sse(
                    prefix(self.from),
                    0x10,
                    input,
                    source(self.source, self.from, image)?,
                )?;
            }
            a.call_symbol(call)?;
        }
        a.store(self.destination.rm()?, R::RAX, 8)?;
        a.store(self.destination.displaced(8).rm()?, R::RDX, 8)
    }
    fn unsigned_to_float(
        self,
        a: &mut Assembler,
        image: &mut Image,
        scratch: Rm,
    ) -> Result<(), Error> {
        let s = self.source.register().ok_or(Error::Limit)?;
        if self.to == Type::F80 {
            a.store(scratch, s, 8)?;
            a.x87(0xdf, 5, scratch)?;
            a.op(0, true, 0x85, 1, s.code(), Rm::Reg(s), false, 0)?;
            let done = a.label()?;
            a.branch(Some(9), done, true);
            let symbol = image.constant(&0x5f80_0000_u32.to_le_bytes(), 4)?;
            a.x87(0xd8, 0, Rm::rip(symbol, 0))?;
            a.bind(done);
            return a.x87(0xdb, 7, self.destination.rm()?);
        }
        let d = self.destination.register().ok_or(Error::Limit)?;
        let prefix = prefix(self.to);
        let negative = a.label()?;
        let done = a.label()?;
        a.op(0, true, 0x85, 1, s.code(), Rm::Reg(s), false, 0)?;
        a.branch(Some(8), negative, true);
        a.sse(0, 0x57, d, Rm::Reg(d))?;
        a.sse_wide(prefix, 0x2a, d, Rm::Reg(s))?;
        a.branch(None, done, true);
        a.bind(negative);
        let (t1, t2) = if s == R::R11 {
            (R::R10, R::R11)
        } else {
            (R::R11, R::R10)
        };
        a.mov_rr(t1, s)?;
        a.op(0, true, 0xd1, 1, 5, Rm::Reg(t1), false, 0)?;
        if t2 != s {
            a.mov_rr(t2, s)?;
        }
        a.alu_ri(4, Rm::Reg(t2), 1, 4)?;
        a.alu_rr(1, t1, Rm::Reg(t2), 8)?;
        a.sse(0, 0x57, d, Rm::Reg(d))?;
        a.sse_wide(prefix, 0x2a, d, Rm::Reg(t1))?;
        a.sse(prefix, 0x58, d, Rm::Reg(d))?;
        a.bind(done);
        Ok(())
    }
    fn float_to_unsigned(
        self,
        a: &mut Assembler,
        image: &mut Image,
        scratch: Rm,
    ) -> Result<(), Error> {
        let d = self.destination.register().ok_or(Error::Limit)?;
        let small = a.label()?;
        let done = a.label()?;
        if self.from == Type::F80 {
            let symbol = image.constant(&0x5f00_0000_u32.to_le_bytes(), 4)?;
            let limit = Rm::rip(symbol, 0);
            x87_load(a, image, self.source, self.from)?;
            a.x87(0xd9, 0, limit)?;
            a.byte(0xdf);
            a.byte(0xe9);
            a.branch(Some(7), small, true);
            a.x87(0xd8, 4, limit)?;
            a.x87(0xdd, 1, scratch)?;
            a.load(d, scratch, 8, false)?;
            toggle_top_bit(a, d)?;
            a.branch(None, done, true);
            a.bind(small);
            a.x87(0xdd, 1, scratch)?;
            a.load(d, scratch, 8, false)?;
            a.bind(done);
            return Ok(());
        }
        let prefix = prefix(self.from);
        let x = if let Some(r) = self.source.register() {
            r
        } else {
            let from = source(self.source, self.from, image)?;
            a.sse(prefix, 0x10, R::XMM15, from)?;
            R::XMM15
        };
        let limit = if self.from == Type::F32 {
            image.constant(&0x5f00_0000_u32.to_le_bytes(), 4)?
        } else {
            image.constant(&0x43e0_0000_0000_0000_u64.to_le_bytes(), 8)?
        };
        a.sse(prefix, 0x10, R::XMM14, Rm::rip(limit, 0))?;
        a.sse(
            if self.from == Type::F32 { 0 } else { 0x66 },
            0x2e,
            x,
            Rm::Reg(R::XMM14),
        )?;
        a.branch(Some(2), small, true);
        if x != R::XMM15 {
            a.sse(0, 0x28, R::XMM15, Rm::Reg(x))?;
        }
        a.sse(prefix, 0x5c, R::XMM15, Rm::Reg(R::XMM14))?;
        a.sse_wide(prefix, 0x2c, d, Rm::Reg(R::XMM15))?;
        toggle_top_bit(a, d)?;
        a.branch(None, done, true);
        a.bind(small);
        a.sse_wide(prefix, 0x2c, d, Rm::Reg(x))?;
        a.bind(done);
        Ok(())
    }
}
fn toggle_top_bit(a: &mut Assembler, d: R) -> Result<(), Error> {
    a.op(0, true, 0x0fba, 2, 7, Rm::Reg(d), false, 1)?;
    a.imm(63, 1);
    Ok(())
}
