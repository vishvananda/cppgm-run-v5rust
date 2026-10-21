//! Scalar machine operations shared by the byte encoder and MIR view. See NOTICE.
use super::{
    image::ImageSymbol,
    mir::low_word,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::Program,
    vocabulary::{Binary, Type, Unary},
};
use std::io::Write;
// MIR carries the scalar kind independently from a LowIR object's full size.
pub(super) struct Kind(pub(super) Type);
impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Type::Object { .. } => f.write_str("obj"),
            ty => std::fmt::Display::fmt(&ty, f),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Operand {
    Memory(super::memory::Memory),
    SymbolOffset(crate::lowir::model::SymbolId, i32),
    Register(R),
    Immediate(u128),
    Frame(i32),
    Symbol(crate::lowir::model::SymbolId),
}
impl Operand {
    pub(super) fn displaced(self, offset: i32) -> Self {
        self.memory().map_or(self, |mut memory| {
            memory.displacement = memory.displacement.wrapping_add(offset);
            Self::Memory(memory)
        })
    }
    pub(super) fn register(self) -> Option<R> {
        if let Self::Register(reg) = self {
            Some(reg)
        } else {
            None
        }
    }
    pub(super) fn mask(self) -> u32 {
        if let Self::Memory(m) = self {
            m.mask()
        } else {
            self.register().map_or(0, |r| 1 << r.code())
        }
    }
    pub(super) fn is_memory(self) -> bool {
        matches!(self, Self::Frame(_) | Self::Memory(_))
    }
    pub(super) fn memory(self) -> Option<super::memory::Memory> {
        match self {
            Self::Frame(d) => Some(super::memory::Memory::frame(d)),
            Self::Memory(m) => Some(m),
            _ => None,
        }
    }
    pub(super) fn frame_offset(self) -> Option<i32> {
        self.memory().and_then(super::memory::Memory::frame_offset)
    }
    pub(super) fn rebase(self, begin: i32, bytes: u32) -> Self {
        self.memory()
            .map_or(self, |m| Self::Memory(m.rebased(begin, bytes)))
    }
    pub(super) fn rm(self) -> Result<Rm, Error> {
        match self {
            Self::Register(r) => Ok(Rm::Reg(r)),
            Self::Frame(d) => Ok(Rm::mem(R::RBP, d)),
            Self::Memory(m) => Ok(m.rm()),
            _ => Err(Error::Construction(
                "scalar operand is not register/memory".into(),
            )),
        }
    }
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::Register(r) => write!(out, "{}", r.spelling()),
            Self::Frame(d) => write!(out, "[rbp{d:+}]"),
            Self::Memory(m) => m.write(program, out),
            Self::SymbolOffset(id, d) => {
                write!(out, "@{}", program.names.get(program.symbol(id).name))?;
                if d != 0 {
                    write!(out, "{d:+}")?;
                }
                Ok(())
            }
            Self::Symbol(id) => write!(out, "@{}", program.names.get(program.symbol(id).name)),
            Self::Immediate(bits) => {
                let low = low_word(bits)
                    .map_err(|_| std::io::Error::other("invalid scalar immediate"))?;
                if bits >> 64 == if low < 0 { u128::from(u64::MAX) } else { 0 } {
                    write!(out, "{low}")
                } else {
                    write!(out, "0x{bits:032x}")
                }
            }
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum ScalarOp {
    SignDividend,
    MultiplyWide { unsigned: bool, source: R },
    Divide { unsigned: bool, source: Operand },
    Condition(super::condition_mir::FlagOp),
    Move(R, Operand),
    Lea(R, Operand),
    Lea32(R, Operand),
    Load(R, Type, Operand),
    Store(Operand, Type, Operand),
    Binary(Binary, Type, R, Operand),
    Unary(Unary, Type, R),
    Extend(Type, R),
    ExtendMove(Type, R, R),
    Scan(Unary, Type, R, Operand),
    CompareZero(Type, Operand),
    SetEqual(R),
    Movzx(R),
    Shift(Binary, R, Operand),
}
pub(super) fn signed(ty: Type) -> bool {
    matches!(
        ty,
        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128
    )
}
pub(super) fn flip_sign(ty: Type) -> Type {
    match ty {
        Type::I8 => Type::U8,
        Type::I16 => Type::U16,
        Type::I32 => Type::U32,
        Type::U8 => Type::I8,
        Type::U16 => Type::I16,
        Type::U32 => Type::I32,
        _ => ty,
    }
}
pub(super) fn bytes(ty: Type) -> u8 {
    match ty {
        Type::I1 | Type::I8 | Type::U8 => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 | Type::F32 => 4,
        _ => 8,
    }
}
impl ScalarOp {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        let mut operand = None;
        match self {
            Self::MultiplyWide { unsigned, source } => write!(
                out,
                "{} {}",
                if unsigned { "mul" } else { "imul" },
                source.spelling()
            )?,
            Self::SignDividend => write!(out, "cqo")?,
            Self::Divide { unsigned, source } => {
                write!(out, "{} ", if unsigned { "div" } else { "idiv" })?;
                operand = Some(source);
            }
            Self::Condition(op) => op.write(program, out)?,
            Self::Lea32(r, src) => {
                write!(out, "lea.i32 {}, ", r.spelling())?;
                operand = Some(src);
            }
            Self::Lea(r, src) => {
                write!(out, "lea {}, ", r.spelling())?;
                operand = Some(src);
            }
            Self::Move(r, src) => {
                write!(out, "mov {}, ", r.spelling())?;
                operand = Some(src);
            }
            Self::Load(r, ty, d) => {
                write!(out, "load.{} {}, ", Kind(ty), r.spelling())?;
                operand = Some(d);
            }
            Self::Store(d, ty, r) => {
                write!(out, "store.{ty} ")?;
                d.write(program, out)?;
                write!(out, ", ")?;
                r.write(program, out)?;
            }
            Self::Binary(op, ty, r, src) => {
                write!(
                    out,
                    "{}",
                    if op == Binary::Mul {
                        "imul"
                    } else {
                        op.spelling()
                    }
                )?;
                if !matches!(ty, Type::I64 | Type::Ptr) {
                    write!(out, ".{ty}")?;
                }
                write!(out, " {}, ", r.spelling())?;
                operand = Some(src);
            }
            Self::Unary(op, ty, r) => {
                write!(
                    out,
                    "{}",
                    match op {
                        Unary::Neg => "neg",
                        Unary::BitNot => "not",
                        _ => "bswap",
                    }
                )?;
                if op == Unary::ByteSwap && ty != Type::I64 {
                    write!(out, ".{ty}")?;
                }
                write!(out, " {}", r.spelling())?;
            }
            Self::ExtendMove(ty, r, source) => {
                write!(
                    out,
                    "{}.i{} {}, {}",
                    if signed(ty) { "sext" } else { "zext" },
                    bytes(ty) * 8,
                    r.spelling(),
                    source.spelling()
                )?;
            }
            Self::Extend(ty, r) => {
                write!(
                    out,
                    "{}.{} {}",
                    if signed(ty) { "sext" } else { "zext" },
                    match bytes(ty) {
                        1 => "i8",
                        2 => "i16",
                        _ => "i32",
                    },
                    r.spelling()
                )?;
            }
            Self::Scan(op, ty, r, src) => {
                write!(out, "{}", if op == Unary::Ctz { "bsf" } else { "bsr" })?;
                if ty != Type::I64 {
                    write!(out, ".{ty}")?;
                }
                write!(out, " {}, ", r.spelling())?;
                operand = Some(src);
            }
            Self::CompareZero(ty, src) => {
                write!(out, "cmp.{ty} ")?;
                src.write(program, out)?;
                write!(out, ", 0")?;
            }
            Self::SetEqual(r) => write!(out, "sete {}", r.spelling())?,
            Self::Movzx(r) => write!(out, "movzx {}, {}", r.spelling(), r.spelling())?,
            Self::Shift(op, r, count) => {
                write!(
                    out,
                    "{} {}, ",
                    match op {
                        Binary::Shl => "shl",
                        Binary::Shr => "sar",
                        _ => "shr",
                    },
                    r.spelling()
                )?;
                operand = Some(count);
            }
        }
        if let Some(src) = operand {
            src.write(program, out)?;
        }
        Ok(())
    }
    pub(super) fn rebase(self, begin: i32, bytes: u32) -> Self {
        match self {
            Self::Divide { unsigned, source } => Self::Divide {
                unsigned,
                source: source.rebase(begin, bytes),
            },
            Self::Condition(op) => Self::Condition(op.rebase(begin, bytes)),
            Self::Load(r, t, m) => Self::Load(r, t, m.rebase(begin, bytes)),
            Self::Store(m, t, s) => Self::Store(m.rebase(begin, bytes), t, s.rebase(begin, bytes)),
            Self::Lea(r, m) => Self::Lea(r, m.rebase(begin, bytes)),
            Self::Lea32(r, m) => Self::Lea32(r, m.rebase(begin, bytes)),
            Self::Move(r, m) => Self::Move(r, m.rebase(begin, bytes)),
            Self::Binary(op, t, r, m) => Self::Binary(op, t, r, m.rebase(begin, bytes)),
            Self::Scan(op, t, r, m) => Self::Scan(op, t, r, m.rebase(begin, bytes)),
            Self::CompareZero(t, m) => Self::CompareZero(t, m.rebase(begin, bytes)),
            other => other,
        }
    }
    pub(super) fn emit(self, a: &mut Assembler, keep_flags: bool) -> Result<(), Error> {
        match self {
            Self::MultiplyWide { unsigned, source } => a.op(
                0,
                true,
                0xf7,
                1,
                if unsigned { 4 } else { 5 },
                Rm::Reg(source),
                false,
                0,
            ),
            Self::SignDividend => {
                a.byte(0x48);
                a.byte(0x99);
                Ok(())
            }
            Self::Divide { unsigned, source } => a.op(
                0,
                true,
                0xf7,
                1,
                if unsigned { 6 } else { 7 },
                source.rm()?,
                false,
                0,
            ),
            Self::Condition(op) => op.emit(a),
            Self::Move(r, src) => match src {
                Operand::Register(s) => a.mov_rr(r, s),
                Operand::Immediate(v) => a.mov_ri_flags(r, low_word(v)?, keep_flags),
                Operand::Symbol(s) => a.lea(r, Rm::rip(ImageSymbol::Program(s), 0)),
                Operand::Frame(d) => a.load(r, Rm::mem(R::RBP, d), 8, false),
                Operand::Memory(m) => a.load(r, m.rm(), 8, false),
                Operand::SymbolOffset(s, d) => a.lea(r, Rm::rip(ImageSymbol::Program(s), d)),
            },
            Self::Load(r, ty, d) => a.load(r, d.rm()?, bytes(ty), signed(ty)),
            Self::Store(d, ty, src) => {
                if let Operand::Register(r) = src {
                    a.store(d.rm()?, r, bytes(ty))
                } else if let Operand::Immediate(v) = src {
                    let low = low_word(v)?;
                    if bytes(ty) < 8 || i32::try_from(low).is_ok() {
                        a.store_imm(d.rm()?, low, bytes(ty))
                    } else {
                        let scratch = if d.mask() & (1 << R::R11.code()) == 0 {
                            R::R11
                        } else {
                            R::R10
                        };
                        a.mov_ri(scratch, low)?;
                        a.store(d.rm()?, scratch, 8)
                    }
                } else {
                    Err(Error::Construction("invalid store source".into()))
                }
            }
            Self::Lea(r, src) => a.lea(r, src.rm()?),
            Self::Lea32(r, src) => a.op(0, false, 0x8d, 1, r.code(), src.rm()?, false, 0),
            Self::ExtendMove(ty, r, source) => a.load(r, Rm::Reg(source), bytes(ty), signed(ty)),
            Self::Extend(ty, r) => a.load(r, Rm::Reg(r), bytes(ty), signed(ty)),
            Self::Binary(op, ty, r, src) => binary(a, op, ty, r, src),
            Self::Unary(op, ty, r) => match op {
                Unary::Neg | Unary::BitNot => a.op(
                    0,
                    !matches!(ty, Type::I32 | Type::U32),
                    0xf7,
                    1,
                    if op == Unary::Neg { 3 } else { 2 },
                    Rm::Reg(r),
                    false,
                    0,
                ),
                _ => {
                    if bytes(ty) == 2 {
                        shift_immediate(a, 0, r, 8, 2)
                    } else {
                        let wide = bytes(ty) == 8;
                        if wide || r.code() & 8 != 0 {
                            a.byte(0x40 | if wide { 8 } else { 0 } | (r.code() >> 3));
                        }
                        a.byte(0x0f);
                        a.byte(0xc8 + (r.code() & 7));
                        Ok(())
                    }
                }
            },
            Self::Scan(op, ty, r, src) => a.op(
                0,
                bytes(ty) == 8,
                if op == Unary::Ctz { 0x0fbc } else { 0x0fbd },
                2,
                r.code(),
                src.rm()?,
                false,
                0,
            ),
            Self::CompareZero(ty, src) => {
                super::condition_mir::FlagOp::Compare(ty, src, Operand::Immediate(0)).emit(a)
            }
            Self::SetEqual(r) => a.op(0, false, 0x0f94, 2, 0, Rm::Reg(r), true, 0),
            Self::Movzx(r) => a.op(0, false, 0x0fb6, 2, r.code(), Rm::Reg(r), true, 0),
            Self::Shift(op, r, count) => {
                let digit = match op {
                    Binary::Shl => 4,
                    Binary::Shr => 7,
                    _ => 5,
                };
                if let Operand::Immediate(v) = count {
                    shift_immediate(a, digit, r, low_word(v)?, 8)
                } else {
                    a.shift_cl(digit, Rm::Reg(r), 8)
                }
            }
        }
    }
}
fn shift_immediate(a: &mut Assembler, digit: u8, r: R, count: i64, bytes: u8) -> Result<(), Error> {
    let count = count & (i64::from(bytes) * 8 - 1);
    if count == 0 {
        return Ok(());
    }
    a.op(
        if bytes == 2 { 0x66 } else { 0 },
        bytes == 8,
        if count == 1 { 0xd1 } else { 0xc1 },
        1,
        digit,
        Rm::Reg(r),
        false,
        u8::from(count != 1),
    )?;
    if count != 1 {
        a.imm(count, 1);
    }
    Ok(())
}
fn binary(a: &mut Assembler, op: Binary, ty: Type, r: R, src: Operand) -> Result<(), Error> {
    let width = bytes(ty);
    if op == Binary::Mul {
        if let Operand::Immediate(v) = src {
            let v = low_word(v)?;
            if v > 0 && width == 8 {
                let shift = v.trailing_zeros();
                let odd = v >> shift;
                if matches!(odd, 1 | 3 | 5 | 9) {
                    if odd != 1 {
                        let rm = Rm::Memory {
                            base: Some(r),
                            index: Some(r),
                            scale: u8::try_from(odd - 1).map_err(|_| Error::Limit)?,
                            disp: 0,
                            target: None,
                        };
                        a.lea(r, rm)?;
                    }
                    if shift != 0 {
                        shift_immediate(a, 4, r, i64::from(shift), 8)?;
                    }
                    return Ok(());
                }
            }
            let small = i8::try_from(v).is_ok();
            a.op(
                0,
                width == 8,
                if small { 0x6b } else { 0x69 },
                1,
                r.code(),
                Rm::Reg(r),
                false,
                if small { 1 } else { 4 },
            )?;
            a.imm(v, if small { 1 } else { 4 });
            return Ok(());
        }
        return a.op(0, width == 8, 0x0faf, 2, r.code(), src.rm()?, false, 0);
    }
    let digit = match op {
        Binary::Add => 0,
        Binary::Sub => 5,
        Binary::And => 4,
        Binary::Or => 1,
        Binary::Xor => 6,
        _ => return Err(Error::Construction("non-ALU scalar operation".into())),
    };
    if let Operand::Immediate(v) = src {
        let v = low_word(v)?;
        if digit == 4 && width == 8 && (0..=0xffff_ffff).contains(&v) {
            return a.alu_ri(4, Rm::Reg(r), v, 4);
        }
        if width < 8 || i32::try_from(v).is_ok() {
            return a.alu_ri(digit, Rm::Reg(r), v, width);
        }
        let scratch = if r == R::R11 { R::R10 } else { R::R11 };
        a.mov_ri(scratch, v)?;
        return a.alu_rr(digit, r, Rm::Reg(scratch), width);
    }
    a.alu_rr(digit, r, src.rm()?, width)
}
