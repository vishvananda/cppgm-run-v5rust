//! Typed machine addressing modes and their exact MIR spelling. See NOTICE.
use super::{
    image::ImageSymbol,
    x86::{Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::{Program, SymbolId},
};
use std::io::Write;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Base {
    Absolute,
    Register(R),
    Frame,
    Symbol(SymbolId),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Memory {
    pub(super) base: Base,
    pub(super) index: Option<R>,
    pub(super) scale: u8,
    pub(super) displacement: i32,
}
impl Memory {
    pub(super) fn new(base: Base, displacement: i32) -> Self {
        Self {
            base,
            index: None,
            scale: 1,
            displacement,
        }
    }
    pub(super) fn frame(d: i32) -> Self {
        Self::new(Base::Frame, d)
    }
    pub(super) fn frame_offset(self) -> Option<i32> {
        matches!(self.base, Base::Frame)
            .then_some(self.displacement)
            .filter(|_| self.index.is_none())
    }
    pub(super) fn mask(self) -> u32 {
        let base = match self.base {
            Base::Register(r) => 1 << r.code(),
            _ => 0,
        };
        base | self.index.map_or(0, |r| 1 << r.code())
    }
    pub(super) fn rebased(mut self, begin: i32, bytes: u32) -> Self {
        if matches!(self.base, Base::Frame)
            && self.displacement >= begin
            && i64::from(self.displacement) < i64::from(begin) + i64::from(bytes)
        {
            self.base = Base::Register(R::R12);
            self.displacement = self.displacement.wrapping_sub(begin);
        }
        self
    }
    pub(super) fn rm(self) -> Rm {
        if let Base::Symbol(id) = self.base {
            return Rm::rip(ImageSymbol::Program(id), self.displacement);
        }
        Rm::Memory {
            base: match self.base {
                Base::Frame => Some(R::RBP),
                Base::Register(r) => Some(r),
                _ => None,
            },
            index: self.index,
            scale: self.scale,
            disp: self.displacement,
            target: None,
        }
    }
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        if let Base::Symbol(id) = self.base {
            write!(out, "@{}", program.names.get(program.symbol(id).name))?;
            if self.displacement != 0 {
                write!(out, "{:+}", self.displacement)?;
            }
            return Ok(());
        }
        write!(out, "[")?;
        let reg = match self.base {
            Base::Frame => Some(R::RBP),
            Base::Register(r) => Some(r),
            _ => None,
        };
        if let Some(r) = reg {
            write!(out, "{}", r.spelling())?;
        }
        if let Some(r) = self.index {
            if reg.is_some() {
                write!(out, "+")?;
            }
            write!(out, "{}", r.spelling())?;
            if self.scale != 1 {
                write!(out, "*{}", self.scale)?;
            }
        }
        if reg.is_none() && self.index.is_none() {
            write!(out, "{}", self.displacement)?;
        } else if self.displacement != 0 {
            write!(out, "{:+}", self.displacement)?;
        }
        write!(out, "]")
    }
}
pub(super) fn scalar(ty: crate::lowir::vocabulary::Type) -> Result<(), Error> {
    if ty.float_rank() != 0
        || matches!(
            ty,
            crate::lowir::vocabulary::Type::I128 | crate::lowir::vocabulary::Type::Object { .. }
        )
    {
        Err(super::value_selection::unsupported())
    } else {
        Ok(())
    }
}
