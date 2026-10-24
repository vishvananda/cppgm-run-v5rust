//! Typed floating arithmetic and x87 stack effects. See NOTICE.
use super::{
    image::Image,
    scalar_mir::Operand,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    literals::FloatKind,
    model::Program,
    vocabulary::{Binary, Compare, Type},
};
use std::io::Write;

#[derive(Clone, Copy)]
pub(super) enum Arithmetic {
    Binary(Binary),
    Negate,
}
impl Arithmetic {
    fn name(self) -> &'static str {
        match self {
            Self::Binary(Binary::Add) => "fadd",
            Self::Binary(Binary::Sub) => "fsub",
            Self::Binary(Binary::Mul) => "fmul",
            Self::Binary(_) => "fdiv",
            Self::Negate => "fneg",
        }
    }
    fn commutative(self) -> bool {
        matches!(self, Self::Binary(Binary::Add | Binary::Mul))
    }
}
#[derive(Clone, Copy)]
pub(super) enum FloatingOp {
    Compare {
        ty: Type,
        left: Operand,
        right: Operand,
    },
    Set {
        predicate: Compare,
        ty: Type,
        destination: R,
        left: Operand,
        right: Operand,
    },
    Arithmetic {
        operation: Arithmetic,
        ty: Type,
        destination: Operand,
        left: Operand,
        right: Option<Operand>,
    },
    Return(Operand),
    ReturnPair(Operand, Operand),
    Pop(Option<Operand>),
}
pub(super) fn write_operand(
    program: &Program,
    out: &mut impl Write,
    ty: Type,
    value: Operand,
) -> std::io::Result<()> {
    if matches!(value, Operand::Immediate(_)) {
        let Operand::Immediate(bits) = value else {
            return Err(std::io::Error::other("floating literal"));
        };
        let kind =
            FloatKind::from_type(ty).ok_or_else(|| std::io::Error::other("floating type"))?;
        let mut scanner = crate::types::floating::FloatingScanner::default();
        write!(
            out,
            "{}",
            crate::lowir::literals::mir_text(&mut scanner, kind, bits)
                .map_err(|_| std::io::Error::other("floating bits"))?
        )
    } else {
        value.write(program, out)
    }
}
pub(super) fn source(value: Operand, ty: Type, image: &mut Image) -> Result<Rm, Error> {
    if let Operand::Immediate(bits) = value {
        super::emission::constant(image, FloatKind::from_type(ty).ok_or(Error::Limit)?, bits)
    } else {
        value.rm()
    }
}
pub(super) fn x87_load(
    a: &mut Assembler,
    image: &mut Image,
    value: Operand,
    ty: Type,
) -> Result<(), Error> {
    let from = source(value, ty, image)?;
    let (opcode, digit) = match ty {
        Type::F80 => (0xdb, 5),
        Type::F64 => (0xdd, 0),
        _ => (0xd9, 0),
    };
    a.x87(opcode, digit, from)
}
impl FloatingOp {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::Compare { ty, left, right } => {
                write!(out, "    fcmp.{ty} ")?;
                write_operand(program, out, ty, left)?;
                write!(out, ", ")?;
                write_operand(program, out, ty, right)
            }
            Self::Set {
                predicate,
                ty,
                destination,
                left,
                right,
            } => {
                write!(
                    out,
                    "    f{}.{ty} {}, ",
                    predicate.spelling(),
                    destination.spelling()
                )?;
                write_operand(program, out, ty, left)?;
                write!(out, ", ")?;
                write_operand(program, out, ty, right)
            }
            Self::Arithmetic {
                operation,
                ty,
                destination,
                left,
                right,
            } => {
                write!(out, "    {}.{ty} ", operation.name())?;
                destination.write(program, out)?;
                write!(out, ", ")?;
                write_operand(program, out, ty, left)?;
                if let Some(right) = right {
                    write!(out, ", ")?;
                    write_operand(program, out, ty, right)?;
                }
                Ok(())
            }
            Self::ReturnPair(real, imaginary) => {
                write!(out, "    fret.f80 ")?;
                write_operand(program, out, Type::F80, real)?;
                write!(out, ", ")?;
                write_operand(program, out, Type::F80, imaginary)
            }
            Self::Return(value) => {
                write!(out, "    fret.f80 ")?;
                write_operand(program, out, Type::F80, value)
            }
            Self::Pop(value) => {
                write!(out, "    fstp.f80")?;
                if let Some(value) = value {
                    write!(out, " ")?;
                    value.write(program, out)?;
                }
                Ok(())
            }
        }
    }
    pub(super) fn rebase(self, begin: i32, bytes: u32) -> Self {
        let rebase = |v: Operand| v.rebase(begin, bytes);
        match self {
            Self::Compare { ty, left, right } => Self::Compare {
                ty,
                left: rebase(left),
                right: rebase(right),
            },
            Self::Set {
                predicate,
                ty,
                destination,
                left,
                right,
            } => Self::Set {
                predicate,
                ty,
                destination,
                left: rebase(left),
                right: rebase(right),
            },
            Self::Arithmetic {
                operation,
                ty,
                destination,
                left,
                right,
            } => Self::Arithmetic {
                operation,
                ty,
                destination: rebase(destination),
                left: rebase(left),
                right: right.map(rebase),
            },
            Self::Return(v) => Self::Return(rebase(v)),
            Self::ReturnPair(real, imaginary) => Self::ReturnPair(rebase(real), rebase(imaginary)),
            Self::Pop(v) => Self::Pop(v.map(rebase)),
        }
    }
    pub(super) fn emit(self, a: &mut Assembler, image: &mut Image) -> Result<(), Error> {
        match self {
            Self::Compare { ty, left, right } => compare(a, image, ty, left, right),
            Self::Set {
                predicate,
                ty,
                destination,
                left,
                right,
            } => set(a, image, predicate, ty, destination, left, right),
            Self::ReturnPair(real, imaginary) => {
                x87_load(a, image, imaginary, Type::F80)?;
                x87_load(a, image, real, Type::F80)
            }
            Self::Return(v) => x87_load(a, image, v, Type::F80),
            Self::Pop(Some(v)) => a.x87(0xdb, 7, v.rm()?),
            Self::Pop(None) => {
                a.byte(0xdd);
                a.byte(0xd8);
                Ok(())
            }
            Self::Arithmetic {
                operation,
                ty,
                destination,
                left,
                right,
            } => arithmetic(a, image, operation, ty, destination, left, right),
        }
    }
}
fn compare(
    a: &mut Assembler,
    image: &mut Image,
    ty: Type,
    left: Operand,
    right: Operand,
) -> Result<(), Error> {
    if ty == Type::F80 {
        x87_load(a, image, right, ty)?;
        x87_load(a, image, left, ty)?;
        a.byte(0xdf);
        a.byte(0xe9);
        a.byte(0xdd);
        a.byte(0xd8);
        return Ok(());
    }
    let reg = if let Some(r) = left.register() {
        r
    } else {
        let from = source(left, ty, image)?;
        a.sse(
            if ty == Type::F32 { 0xf3 } else { 0xf2 },
            0x10,
            R::XMM15,
            from,
        )?;
        R::XMM15
    };
    let from = source(right, ty, image)?;
    a.sse(if ty == Type::F32 { 0 } else { 0x66 }, 0x2e, reg, from)
}
fn set(
    a: &mut Assembler,
    image: &mut Image,
    predicate: Compare,
    ty: Type,
    d: R,
    mut left: Operand,
    mut right: Operand,
) -> Result<(), Error> {
    use super::{condition::Condition as C, condition_mir::FlagOp, scalar_mir::ScalarOp};
    if matches!(
        predicate,
        Compare::Lt | Compare::Le | Compare::Ugt | Compare::Uge
    ) {
        std::mem::swap(&mut left, &mut right);
    }
    compare(a, image, ty, left, right)?;
    let scratch = if d == R::R11 { R::R10 } else { R::R11 };
    match predicate {
        Compare::Eq | Compare::Ne => {
            FlagOp::Set(
                if predicate == Compare::Eq {
                    C::Equal
                } else {
                    C::NotEqual
                },
                d,
            )
            .emit(a)?;
            FlagOp::Set(
                if predicate == Compare::Eq {
                    C::NotParity
                } else {
                    C::Parity
                },
                scratch,
            )
            .emit(a)?;
            a.alu_rr(
                if predicate == Compare::Eq { 4 } else { 1 },
                d,
                Rm::Reg(scratch),
                1,
            )?;
        }
        _ => FlagOp::Set(
            match predicate {
                Compare::Gt | Compare::Lt => C::Above,
                Compare::Ge | Compare::Le => C::AboveEqual,
                Compare::Ult | Compare::Ugt => C::Below,
                _ => C::BelowEqual,
            },
            d,
        )
        .emit(a)?,
    }
    ScalarOp::Movzx(d).emit(a, false)
}
fn arithmetic(
    a: &mut Assembler,
    image: &mut Image,
    operation: Arithmetic,
    ty: Type,
    destination: Operand,
    left: Operand,
    right: Option<Operand>,
) -> Result<(), Error> {
    if ty == Type::F80 {
        x87_load(a, image, left, ty)?;
        if let Some(right) = right {
            x87_load(a, image, right, ty)?;
            a.byte(0xde);
            a.byte(match operation {
                Arithmetic::Binary(Binary::Add) => 0xc1,
                Arithmetic::Binary(Binary::Sub) => 0xe9,
                Arithmetic::Binary(Binary::Mul) => 0xc9,
                _ => 0xf9,
            });
        } else {
            a.byte(0xd9);
            a.byte(0xe0);
        }
        return a.x87(0xdb, 7, destination.rm()?);
    }
    let prefix = if ty == Type::F32 { 0xf3 } else { 0xf2 };
    if ty == Type::F64
        && let Some(right) = right
    {
        let memory = |a: &mut Assembler, image: &mut Image, v: Operand, below| {
            if let Some(reg) = v.register() {
                let memory = Rm::mem(R::RSP, below);
                a.sse(prefix, 0x11, reg, memory)?;
                Ok(memory)
            } else {
                source(v, ty, image)
            }
        };
        let from = memory(a, image, left, -8)?;
        a.x87(0xdd, 0, from)?;
        let from = memory(a, image, right, -16)?;
        a.x87(
            0xdc,
            match operation {
                Arithmetic::Binary(Binary::Add) => 0,
                Arithmetic::Binary(Binary::Mul) => 1,
                Arithmetic::Binary(Binary::Sub) => 4,
                _ => 6,
            },
            from,
        )?;
        if let Some(reg) = destination.register() {
            a.x87(0xdd, 3, Rm::mem(R::RSP, -8))?;
            a.sse(prefix, 0x10, reg, Rm::mem(R::RSP, -8))?;
        } else {
            a.x87(0xdd, 3, destination.rm()?)?;
        }
        return Ok(());
    }
    let d = destination.register().ok_or(Error::Limit)?;
    if matches!(operation, Arithmetic::Negate) {
        if left.register() != Some(d) {
            if let Some(reg) = left.register() {
                a.sse(0, 0x28, d, Rm::Reg(reg))?;
            } else {
                let from = source(left, ty, image)?;
                a.sse(prefix, 0x10, d, from)?;
            }
        }
        let mut mask = [0; 16];
        mask[if ty == Type::F32 { 3 } else { 7 }] = 0x80;
        let symbol = image.constant(&mask, 16)?;
        return a.sse(0, 0x57, d, Rm::rip(symbol, 0));
    }
    let right = right.ok_or(Error::Limit)?;
    let opcode = match operation {
        Arithmetic::Binary(Binary::Add) => 0x58,
        Arithmetic::Binary(Binary::Sub) => 0x5c,
        Arithmetic::Binary(Binary::Mul) => 0x59,
        _ => 0x5e,
    };
    if left.register() == Some(d) {
        let from = source(right, ty, image)?;
        return a.sse(prefix, opcode, d, from);
    }
    if right.register() == Some(d) {
        if operation.commutative() {
            let from = source(left, ty, image)?;
            return a.sse(prefix, opcode, d, from);
        }
        move_source(a, image, prefix, R::XMM15, left, ty)?;
        a.sse(prefix, opcode, R::XMM15, Rm::Reg(d))?;
        return a.sse(0, 0x28, d, Rm::Reg(R::XMM15));
    }
    move_source(a, image, prefix, d, left, ty)?;
    let from = source(right, ty, image)?;
    a.sse(prefix, opcode, d, from)
}
fn move_source(
    a: &mut Assembler,
    image: &mut Image,
    prefix: u8,
    d: R,
    value: Operand,
    ty: Type,
) -> Result<(), Error> {
    if let Some(reg) = value.register() {
        a.sse(0, 0x28, d, Rm::Reg(reg))
    } else {
        let from = source(value, ty, image)?;
        a.sse(prefix, 0x10, d, from)
    }
}
