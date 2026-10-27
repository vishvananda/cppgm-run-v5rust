// Translated from native/select/emit_calls.cpp::emit_va_arg_object and
// native/x86/emitter_memory.cpp::va_arg_object/va_copy_bytes. See NOTICE.
//! Aggregate varargs preserve the frozen eightbyte selection and copy order.
use super::{
    scalar_mir::Operand,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    vocabulary::{EightbyteClass as Class, ObjectClasses, Type},
};
#[derive(Clone, Copy)]
pub(super) struct ObjectArgument {
    pub(super) bytes: u32,
    align: u32,
    classes: ObjectClasses,
}
impl ObjectArgument {
    pub(super) fn new(ty: Type, classes: Option<ObjectClasses>) -> Result<Self, Error> {
        let bytes = super::abi::value_bytes(ty)?;
        let align = if ty == Type::I128 && classes.is_some() {
            8
        } else {
            super::abi::value_align(ty)?.max(8)
        };
        let classes = if ty == Type::F80
            || bytes > 16
            || classes.is_some_and(ObjectClasses::argument_in_memory)
        {
            ObjectClasses::Memory
        } else {
            classes.unwrap_or(ObjectClasses::Direct(
                Class::Integer,
                (bytes > 8).then_some(Class::Integer),
            ))
        };
        Ok(Self {
            bytes,
            align,
            classes,
        })
    }
    fn class(self, index: u32) -> Class {
        self.classes.eightbyte(index)
    }
    pub(super) fn emit(
        self,
        a: &mut Assembler,
        destination: Operand,
        list: Operand,
    ) -> Result<(), Error> {
        let home = destination.rm()?;
        a.lea(R::R11, list.rm()?)?;
        let stack = a.label()?;
        let end = a.label()?;
        if !matches!(self.classes, ObjectClasses::Memory) {
            let eightbytes = self.bytes.div_ceil(8);
            let mut gprs = 0;
            let mut xmms = 0;
            for index in 0..eightbytes {
                gprs += i64::from(self.class(index) == Class::Integer);
                xmms += i64::from(self.class(index) == Class::Sse);
            }
            if gprs != 0 {
                a.load(R::R10, Rm::mem(R::R11, 0), 4, false)?;
                a.alu_ri(7, Rm::Reg(R::R10), 48 - 8 * gprs, 4)?;
                a.branch(Some(7), stack, false);
            }
            if xmms != 0 {
                a.load(R::R10, Rm::mem(R::R11, 4), 4, false)?;
                a.alu_ri(7, Rm::Reg(R::R10), 176 - 16 * xmms, 4)?;
                a.branch(Some(7), stack, false);
            }
            for index in 0..eightbytes {
                let class = self.class(index);
                if class == Class::None {
                    continue;
                }
                let field = if class == Class::Sse { 4 } else { 0 };
                let part = (self.bytes - 8 * index).min(8);
                a.load(R::R10, Rm::mem(R::R11, field), 4, false)?;
                a.alu_rr(0, R::R10, Rm::mem(R::R11, 16), 8)?;
                a.alu_ri(
                    0,
                    Rm::mem(R::R11, field),
                    if class == Class::Sse { 16 } else { 8 },
                    4,
                )?;
                copy_bytes(a, home, 0, 8 * index, part)?;
            }
            a.branch(None, end, false);
        }
        a.bind(stack);
        let rounded = i32::try_from(self.bytes.div_ceil(8) * 8).map_err(|_| Error::Limit)?;
        a.load(R::R10, Rm::mem(R::R11, 8), 8, false)?;
        if self.align > 8 {
            a.alu_ri(0, Rm::Reg(R::R10), i64::from(self.align) - 1, 8)?;
            a.alu_ri(4, Rm::Reg(R::R10), -i64::from(self.align), 8)?;
        }
        a.alu_ri(0, Rm::Reg(R::R10), i64::from(rounded), 8)?;
        a.store(Rm::mem(R::R11, 8), R::R10, 8)?;
        copy_bytes(a, home, -rounded, 0, self.bytes)?;
        a.bind(end);
        Ok(())
    }
}
fn copy_bytes(
    a: &mut Assembler,
    home: Rm,
    source: i32,
    offset: u32,
    bytes: u32,
) -> Result<(), Error> {
    let xmm = R::xmm(15)?;
    let mut done = 0;
    while bytes - done >= 16 {
        a.sse(
            0,
            0x10,
            xmm,
            Rm::mem(R::R10, source.wrapping_add(done.cast_signed())),
        )?;
        a.sse(
            0,
            0x11,
            xmm,
            home.displaced(offset.wrapping_add(done).cast_signed()),
        )?;
        done += 16;
    }
    if bytes - done >= 8 {
        a.sse(
            0xf2,
            0x10,
            xmm,
            Rm::mem(R::R10, source.wrapping_add(done.cast_signed())),
        )?;
        a.sse(
            0xf2,
            0x11,
            xmm,
            home.displaced(offset.wrapping_add(done).cast_signed()),
        )?;
        done += 8;
    }
    if done == bytes {
        return Ok(());
    }
    a.load(
        R::R10,
        Rm::mem(R::R10, source.wrapping_add(done.cast_signed())),
        8,
        false,
    )?;
    for width in [4u8, 2, 1] {
        if bytes - done < u32::from(width) {
            continue;
        }
        a.store(
            home.displaced(offset.wrapping_add(done).cast_signed()),
            R::R10,
            width,
        )?;
        done += u32::from(width);
        if done != bytes {
            let count = width * 8;
            a.op(0, true, 0xc1, 1, 5, Rm::Reg(R::R10), false, 1)?;
            a.imm(i64::from(count), 1);
        }
    }
    Ok(())
}
