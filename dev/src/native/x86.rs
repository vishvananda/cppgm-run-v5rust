//! Direct x86 encoding, local labels and typed image fixups. See NOTICE.
use super::{image::ImageSymbol, symbol_access::SymbolAccess};
use crate::lowir::{Error, model::SymbolId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Register(u8);
impl Register {
    pub(super) const fn code(self) -> u8 {
        self.0
    }
    pub(super) const RAX: Self = Self(0);
    pub(super) const RCX: Self = Self(1);
    pub(super) const RDX: Self = Self(2);
    pub(super) const RBX: Self = Self(3);
    pub(super) const RSP: Self = Self(4);
    pub(super) const RBP: Self = Self(5);
    pub(super) const RSI: Self = Self(6);
    pub(super) const RDI: Self = Self(7);
    pub(super) const R8: Self = Self(8);
    pub(super) const R9: Self = Self(9);
    pub(super) const R10: Self = Self(10);
    pub(super) const R11: Self = Self(11);
    pub(super) const R12: Self = Self(12);
    pub(super) const R13: Self = Self(13);
    pub(super) const R14: Self = Self(14);
    pub(super) const R15: Self = Self(15);
    pub(super) const XMM0: Self = Self(16);
    pub(super) const XMM14: Self = Self(30);
    pub(super) const XMM15: Self = Self(31);
    pub(super) fn xmm(index: u8) -> Result<Self, Error> {
        if index >= 16 {
            return Err(invariant("invalid XMM register"));
        }
        Ok(Self(16 + index))
    }
    pub(super) fn spelling(self) -> &'static str {
        const NAMES: [&str; 32] = [
            "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11",
            "r12", "r13", "r14", "r15", "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6",
            "xmm7", "xmm8", "xmm9", "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
        ];
        NAMES[usize::from(self.0)]
    }
}

#[derive(Clone, Copy)]
pub(super) enum Rm {
    Reg(Register),
    Frame {
        index: Option<Register>,
        scale: u8,
        disp: i32,
        incoming: bool,
    },
    Memory {
        base: Option<Register>,
        index: Option<Register>,
        scale: u8,
        disp: i32,
        target: Option<ImageSymbol>,
    },
}
impl Rm {
    pub(super) fn absolute(disp: i32) -> Self {
        Self::Memory {
            base: None,
            index: None,
            scale: 1,
            disp,
            target: None,
        }
    }
    pub(super) fn displaced(mut self, offset: i32) -> Self {
        if let Self::Memory { disp, .. } | Self::Frame { disp, .. } = &mut self {
            *disp = disp.wrapping_add(offset);
        }
        self
    }
    pub(super) fn uses(self, reg: Register) -> bool {
        match self {
            Self::Reg(value) => value == reg,
            Self::Frame { index, .. } => index == Some(reg),
            Self::Memory { base, index, .. } => base == Some(reg) || index == Some(reg),
        }
    }
    pub(super) fn frame(disp: i32) -> Self {
        Self::Frame {
            index: None,
            scale: 1,
            disp,
            incoming: disp >= 0,
        }
    }
    pub(super) fn mem(base: Register, disp: i32) -> Self {
        Self::Memory {
            base: Some(base),
            index: None,
            scale: 1,
            disp,
            target: None,
        }
    }
    pub(super) fn indexed(base: Register, disp: i32, index: Register) -> Self {
        Self::Memory {
            base: Some(base),
            index: Some(index),
            scale: 1,
            disp,
            target: None,
        }
    }
    pub(super) fn rip(target: ImageSymbol, disp: i32) -> Self {
        Self::Memory {
            base: None,
            index: None,
            scale: 1,
            disp,
            target: Some(target),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Label(u32);
#[derive(Clone, Copy)]
pub(super) enum FixupKind {
    Relative,
    Got,
    TpOff,
    GotTpOff,
}
#[derive(Clone, Copy)]
pub(super) struct CodeFixup {
    pub(super) offset: u32,
    pub(super) target: ImageSymbol,
    pub(super) addend: i32,
    pub(super) end: u8,
    pub(super) branch: bool,
    pub(super) kind: FixupKind,
}
struct LabelReference {
    offset: usize,
    label: Label,
    bytes: u8,
    base: Option<Label>,
}
#[derive(Default)]
pub(super) struct Assembler {
    pub(super) code: Vec<u8>,
    pub(super) fixups: Vec<CodeFixup>,
    labels: Vec<Option<usize>>,
    references: Vec<LabelReference>,
    frame: super::frame_encoding::Addressing,
}
pub(super) fn invariant(message: &str) -> Error {
    Error::Construction(message.into())
}
impl Assembler {
    pub(super) fn frame_addressing(&mut self, frame: super::frame_encoding::Addressing) {
        self.frame = frame;
    }
    pub(super) fn position(&self) -> Result<u32, Error> {
        u32::try_from(self.code.len()).map_err(|_| Error::Limit)
    }
    pub(super) fn label(&mut self) -> Result<Label, Error> {
        let label = Label(u32::try_from(self.labels.len()).map_err(|_| Error::Limit)?);
        self.labels.push(None);
        Ok(label)
    }
    pub(super) fn label_position(&self, label: Label) -> Option<usize> {
        self.labels[label.0 as usize]
    }
    pub(super) fn bind(&mut self, label: Label) {
        self.labels[label.0 as usize] = Some(self.code.len());
    }
    pub(super) fn resolve(&mut self) -> Result<(), Error> {
        for reference in &self.references {
            let target = self.labels[reference.label.0 as usize]
                .ok_or_else(|| invariant("branch out of range"))?;
            let base = if let Some(label) = reference.base {
                self.label_position(label)
                    .ok_or_else(|| invariant("unbound table base"))?
            } else {
                reference.offset + usize::from(reference.bytes)
            };
            let value = i64::try_from(target).map_err(|_| Error::Limit)?
                - i64::try_from(base).map_err(|_| Error::Limit)?;
            if reference.bytes == 4 && i32::try_from(value).is_err() {
                return Err(invariant("label offset out of range"));
            }
            if reference.bytes == 1 && i8::try_from(value).is_err() {
                return Err(invariant("branch out of range"));
            }
            self.code[reference.offset..reference.offset + usize::from(reference.bytes)]
                .copy_from_slice(&value.to_le_bytes()[..usize::from(reference.bytes)]);
        }
        self.references.clear();
        Ok(())
    }
    pub(super) fn label_offset32(&mut self, label: Label, base: Label) {
        self.references.push(LabelReference {
            offset: self.code.len(),
            label,
            bytes: 4,
            base: Some(base),
        });
        self.imm(0, 4);
    }
    pub(super) fn byte(&mut self, byte: u8) {
        self.code.push(byte);
    }
    pub(super) fn imm(&mut self, value: i64, bytes: u8) {
        self.code
            .extend_from_slice(&value.to_le_bytes()[..usize::from(bytes)]);
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "x86 instruction fields are independent encoding inputs"
    )]
    pub(super) fn op(
        &mut self,
        prefix: u8,
        wide: bool,
        opcode: u32,
        opcode_bytes: u8,
        reg: u8,
        rm: Rm,
        byte_regs: bool,
        imm_bytes: u8,
    ) -> Result<(), Error> {
        let rm = self.frame.resolve(rm);
        if prefix != 0 {
            self.byte(prefix);
        }
        let mut rex = 0x40 | if wide { 8 } else { 0 } | if reg & 8 != 0 { 4 } else { 0 };
        let mut force = false;
        match rm {
            Rm::Frame { .. } => return Err(invariant("unresolved virtual frame")),
            Rm::Reg(r) => {
                rex |= u8::from(r.0 & 8 != 0);
                force = byte_regs && (4..=7).contains(&(r.0 & 15)) && r.0 < 16;
            }
            Rm::Memory { base, index, .. } => {
                rex |= u8::from(base.is_some_and(|r| r.0 & 8 != 0));
                rex |= u8::from(index.is_some_and(|r| r.0 & 8 != 0)) * 2;
            }
        }
        let byte_field = byte_regs
            && ((opcode_bytes == 1
                && (matches!(opcode, 0x88 | 0x8a | 0x84 | 0x86)
                    || opcode < 0x40 && matches!(opcode & 7, 0 | 2)))
                || opcode_bytes == 2 && matches!(opcode, 0x0fb0 | 0x0fc0));
        force |= byte_field && (4..=7).contains(&(reg & 15)) && reg < 16;
        if rex != 0x40 || force {
            self.byte(rex);
        }
        for k in (0..opcode_bytes).rev() {
            self.byte(opcode.to_le_bytes()[usize::from(k)]);
        }
        let r = (reg & 7) << 3;
        let Rm::Memory {
            base,
            index,
            scale,
            disp,
            target,
        } = rm
        else {
            if let Rm::Reg(reg) = rm {
                self.byte(0xc0 | r | (reg.0 & 7));
            }
            return Ok(());
        };
        if let Some(target) = target {
            self.byte(5 | r);
            self.fixups.push(CodeFixup {
                offset: self.position()?,
                target,
                addend: disp,
                end: 4 + imm_bytes,
                branch: false,
                kind: FixupKind::Relative,
            });
            self.imm(0, 4);
            return Ok(());
        }
        let scale_bits = match scale {
            2 => 1,
            4 => 2,
            8 => 3,
            _ => 0,
        };
        let has_index = index.is_some();
        let index = index.map_or(4, |r| r.0 & 7);
        let Some(base) = base else {
            self.byte(4 | r);
            self.byte((scale_bits << 6) | (index << 3) | 5);
            self.imm(i64::from(disp), 4);
            return Ok(());
        };
        let base = base.0 & 7;
        let mode = if disp == 0 && base != 5 {
            0
        } else if i8::try_from(disp).is_ok() {
            1
        } else {
            2
        };
        // Index code 4 is absent in the supported GPR addressing vocabulary.
        if has_index || base == 4 {
            self.byte((mode << 6) | r | 4);
            self.byte((scale_bits << 6) | (index << 3) | base);
        } else {
            self.byte((mode << 6) | r | base);
        }
        if mode != 0 {
            self.imm(i64::from(disp), if mode == 1 { 1 } else { 4 });
        }
        Ok(())
    }
    fn plus(&mut self, wide: bool, opcode: u8, reg: Register) {
        if wide || reg.0 & 8 != 0 {
            self.byte(0x40 | if wide { 8 } else { 0 } | u8::from(reg.0 & 8 != 0));
        }
        self.byte(opcode + (reg.0 & 7));
    }
    pub(super) fn mov_rr(&mut self, dst: Register, src: Register) -> Result<(), Error> {
        if dst != src {
            self.op(0, true, 0x8b, 1, dst.0, Rm::Reg(src), false, 0)?;
        }
        Ok(())
    }
    pub(super) fn mov_ri_flags(
        &mut self,
        dst: Register,
        value: i64,
        keep_flags: bool,
    ) -> Result<(), Error> {
        if value == 0 && keep_flags {
            if dst.code() & 8 != 0 {
                self.byte(0x41);
            }
            self.byte(0xb8 + (dst.code() & 7));
            self.imm(0, 4);
            Ok(())
        } else {
            self.mov_ri(dst, value)
        }
    }
    pub(super) fn mov_ri(&mut self, dst: Register, value: i64) -> Result<(), Error> {
        if value == 0 {
            self.op(0, false, 0x33, 1, dst.0, Rm::Reg(dst), false, 0)?;
        } else if value > 0 && value <= i64::from(u32::MAX) {
            self.plus(false, 0xb8, dst);
            self.imm(value, 4);
        } else if i32::try_from(value).is_ok() {
            self.op(0, true, 0xc7, 1, 0, Rm::Reg(dst), false, 4)?;
            self.imm(value, 4);
        } else {
            self.plus(true, 0xb8, dst);
            self.imm(value, 8);
        }
        Ok(())
    }
    pub(super) fn load(
        &mut self,
        dst: Register,
        src: Rm,
        bytes: u8,
        sign: bool,
    ) -> Result<(), Error> {
        let (wide, opcode, count, byte_regs) = match bytes {
            1 => (sign, if sign { 0x0fbe } else { 0x0fb6 }, 2, true),
            2 => (sign, if sign { 0x0fbf } else { 0x0fb7 }, 2, false),
            4 => (sign, if sign { 0x63 } else { 0x8b }, 1, false),
            _ => (true, 0x8b, 1, false),
        };
        self.op(0, wide, opcode, count, dst.0, src, byte_regs, 0)
    }
    pub(super) fn store(&mut self, dst: Rm, src: Register, bytes: u8) -> Result<(), Error> {
        self.op(
            if bytes == 2 { 0x66 } else { 0 },
            bytes == 8,
            if bytes == 1 { 0x88 } else { 0x89 },
            1,
            src.0,
            dst,
            bytes == 1,
            0,
        )
    }
    pub(super) fn store_imm(&mut self, dst: Rm, value: i64, bytes: u8) -> Result<(), Error> {
        let n = if bytes == 8 { 4 } else { bytes };
        self.op(
            if bytes == 2 { 0x66 } else { 0 },
            bytes == 8,
            if bytes == 1 { 0xc6 } else { 0xc7 },
            1,
            0,
            dst,
            bytes == 1,
            n,
        )?;
        self.imm(value, n);
        Ok(())
    }
    pub(super) fn lea(&mut self, dst: Register, src: Rm) -> Result<(), Error> {
        self.op(0, true, 0x8d, 1, dst.0, src, false, 0)
    }
    pub(super) fn lea_label(&mut self, dst: Register, label: Label) {
        self.byte(0x48 | if dst.code() & 8 != 0 { 4 } else { 0 });
        self.byte(0x8d);
        self.byte(5 | ((dst.code() & 7) << 3));
        self.references.push(LabelReference {
            offset: self.code.len(),
            label,
            bytes: 4,
            base: None,
        });
        self.imm(0, 4);
    }
    pub(super) fn alu_rr(
        &mut self,
        digit: u8,
        dst: Register,
        src: Rm,
        bytes: u8,
    ) -> Result<(), Error> {
        if let Rm::Reg(reg) = src {
            return self.op(
                if bytes == 2 { 0x66 } else { 0 },
                bytes == 8,
                u32::from(digit) * 8 + u32::from(bytes != 1),
                1,
                reg.0,
                Rm::Reg(dst),
                bytes == 1,
                0,
            );
        }
        self.op(
            if bytes == 2 { 0x66 } else { 0 },
            bytes == 8,
            u32::from(digit) * 8 + if bytes == 1 { 2 } else { 3 },
            1,
            dst.0,
            src,
            bytes == 1,
            0,
        )
    }
    pub(super) fn alu_ri(
        &mut self,
        digit: u8,
        dst: Rm,
        value: i64,
        bytes: u8,
    ) -> Result<(), Error> {
        let small = i8::try_from(value).is_ok();
        let n = if bytes == 1 || small {
            1
        } else if bytes == 2 {
            2
        } else {
            4
        };
        self.op(
            if bytes == 2 { 0x66 } else { 0 },
            bytes == 8,
            if bytes == 1 {
                0x80
            } else if small {
                0x83
            } else {
                0x81
            },
            1,
            digit,
            dst,
            bytes == 1,
            n,
        )?;
        self.imm(value, n);
        Ok(())
    }
    pub(super) fn shift_cl(&mut self, digit: u8, dst: Rm, bytes: u8) -> Result<(), Error> {
        self.op(
            if bytes == 2 { 0x66 } else { 0 },
            bytes == 8,
            if bytes == 1 { 0xd2 } else { 0xd3 },
            1,
            digit,
            dst,
            bytes == 1,
            0,
        )
    }
    pub(super) fn branch(&mut self, condition: Option<u8>, label: Label, short: bool) {
        if let Some(condition) = condition {
            if short {
                self.byte(0x70 + condition);
            } else {
                self.byte(0x0f);
                self.byte(0x80 + condition);
            }
        } else {
            self.byte(if short { 0xeb } else { 0xe9 });
        }
        let bytes = if short { 1 } else { 4 };
        self.references.push(LabelReference {
            offset: self.code.len(),
            label,
            bytes,
            base: None,
        });
        self.imm(0, bytes);
    }
    pub(super) fn call_symbol(&mut self, target: ImageSymbol) -> Result<(), Error> {
        self.symbol_branch(target, 0xe8)
    }
    pub(super) fn jump_symbol(&mut self, target: ImageSymbol) -> Result<(), Error> {
        self.symbol_branch(target, 0xe9)
    }
    fn symbol_branch(&mut self, target: ImageSymbol, opcode: u8) -> Result<(), Error> {
        self.byte(opcode);
        self.fixups.push(CodeFixup {
            offset: self.position()?,
            target,
            addend: 0,
            end: 4,
            branch: true,
            kind: FixupKind::Relative,
        });
        self.imm(0, 4);
        Ok(())
    }
    pub(super) fn call_rm(&mut self, target: Rm) -> Result<(), Error> {
        self.op(0, false, 0xff, 1, 2, target, false, 0)
    }
    pub(super) fn jump_rm(&mut self, target: Rm) -> Result<(), Error> {
        self.op(0, false, 0xff, 1, 4, target, false, 0)
    }
    pub(super) fn push(&mut self, reg: Register) {
        self.plus(false, 0x50, reg);
    }
    pub(super) fn pop(&mut self, reg: Register) {
        self.plus(false, 0x58, reg);
    }
    pub(super) fn ret(&mut self) {
        self.byte(0xc3);
    }
    pub(super) fn sse(
        &mut self,
        prefix: u8,
        opcode: u8,
        dst: Register,
        src: Rm,
    ) -> Result<(), Error> {
        self.op(
            prefix,
            false,
            0x0f00 | u32::from(opcode),
            2,
            dst.0 & 15,
            src,
            false,
            0,
        )
    }
    pub(super) fn x87(&mut self, opcode: u8, digit: u8, memory: Rm) -> Result<(), Error> {
        self.op(0, false, u32::from(opcode), 1, digit, memory, false, 0)
    }
    pub(super) fn sse_wide(
        &mut self,
        prefix: u8,
        opcode: u8,
        dst: Register,
        src: Rm,
    ) -> Result<(), Error> {
        self.op(
            prefix,
            true,
            0x0f00 | u32::from(opcode),
            2,
            dst.0 & 15,
            src,
            false,
            0,
        )
    }
}

impl Assembler {
    pub(super) fn symbol_address(
        &mut self,
        destination: Register,
        symbol: SymbolId,
        addend: i32,
        access: SymbolAccess,
    ) -> Result<(), Error> {
        let target = ImageSymbol::Program(symbol);
        match access {
            SymbolAccess::Direct => return self.lea(destination, Rm::rip(target, addend)),
            SymbolAccess::TlsLocal => {
                self.op(
                    0x64,
                    true,
                    0x8b,
                    1,
                    destination.code(),
                    Rm::absolute(0),
                    false,
                    0,
                )?;
                self.op(0, true, 0x81, 1, 0, Rm::Reg(destination), false, 4)?;
                self.fixups.push(CodeFixup {
                    offset: self.position()?,
                    target,
                    addend,
                    end: 4,
                    branch: false,
                    kind: FixupKind::TpOff,
                });
                self.imm(0, 4);
                return Ok(());
            }
            SymbolAccess::Got | SymbolAccess::TlsInitial => {
                self.op(
                    0,
                    true,
                    0x8b,
                    1,
                    destination.code(),
                    Rm::rip(target, 0),
                    false,
                    0,
                )?;
                self.fixups.last_mut().ok_or(Error::Limit)?.kind = if access == SymbolAccess::Got {
                    FixupKind::Got
                } else {
                    FixupKind::GotTpOff
                };
                if access == SymbolAccess::TlsInitial {
                    self.op(
                        0x64,
                        true,
                        0x03,
                        1,
                        destination.code(),
                        Rm::absolute(0),
                        false,
                        0,
                    )?;
                }
            }
        }
        if addend != 0 {
            self.lea(destination, Rm::mem(destination, addend))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::image::Image;
    #[test]
    fn immediate_forms_retain_unsigned_and_signed_boundaries() -> Result<(), Error> {
        for (value, expected) in [
            (0, vec![0x33, 0xc0]),
            (-1, vec![0x48, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]),
            (i64::from(u32::MAX), vec![0xb8, 0xff, 0xff, 0xff, 0xff]),
            (1_i64 << 32, vec![0x48, 0xb8, 0, 0, 0, 0, 1, 0, 0, 0]),
        ] {
            let mut a = Assembler::default();
            a.mov_ri(Register::RAX, value)?;
            assert_eq!(a.code, expected);
        }
        Ok(())
    }
    #[test]
    fn indexed_r12_and_rip_immediate_bases_are_distinct() -> Result<(), Error> {
        let mut a = Assembler::default();
        a.lea(Register::RAX, Rm::indexed(Register::RBP, 0, Register::R12))?;
        assert_eq!(a.code, [0x4a, 0x8d, 0x44, 0x25, 0]);
        let mut image = Image::new(0);
        let symbol = image.local()?;
        let mut a = Assembler::default();
        a.store_imm(Rm::rip(symbol, 7), 3, 8)?;
        assert_eq!(a.fixups[0].end, 8);
        assert_eq!(a.fixups[0].offset, 3);
        assert_eq!(a.code.len(), 11);
        Ok(())
    }
    #[test]
    fn labels_resolve_from_the_end_of_the_displacement() -> Result<(), Error> {
        let mut a = Assembler::default();
        let end = a.label()?;
        a.branch(None, end, true);
        a.mov_ri(Register::RAX, 1)?;
        assert!(a.resolve().is_err());
        a.bind(end);
        a.resolve()?;
        assert_eq!(a.code, [0xeb, 5, 0xb8, 1, 0, 0, 0]);
        Ok(())
    }
}
