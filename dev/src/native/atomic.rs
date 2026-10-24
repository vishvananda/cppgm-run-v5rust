//! Atomic MIR, TSO selection and fixed-register encodings. See NOTICE.
use super::{
    mir::{Instruction, Op},
    scalar_mir::{Operand, ScalarOp, bytes},
    scalar_selection::Selection,
    value_facts::Alias,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::{Program, RuntimeInstruction as Runtime, TempId, Value},
    vocabulary::{AtomicUpdate, Binary, Fence, MemoryOrder, Type},
};
use std::io::Write;

#[derive(Clone, Copy)]
pub(super) enum Exchange {
    Swap,
    Add,
    Compare,
}
#[derive(Clone, Copy)]
pub(super) enum Atomic {
    Fence,
    Exchange(Exchange, Type, Operand, R),
    LoadWide {
        destination: Operand,
        source: Operand,
    },
    CompareWide {
        target: Operand,
        expected: Operand,
        desired: Operand,
        result: R,
    },
}
impl Atomic {
    pub(super) fn write(self, program: &Program, out: &mut impl Write) -> std::io::Result<()> {
        let (name, ty, operands) = match self {
            Self::Fence => return write!(out, "    mfence"),
            Self::Exchange(op, ty, target, value) => (
                match op {
                    Exchange::Swap => "xchg",
                    Exchange::Add => "lock_xadd",
                    Exchange::Compare => "lock_cmpxchg",
                },
                ty,
                [Some(target), Some(Operand::Register(value)), None],
            ),
            Self::LoadWide {
                destination,
                source,
            } => (
                "atomic_load",
                Type::I128,
                [Some(destination), Some(source), None],
            ),
            Self::CompareWide {
                target,
                expected,
                desired,
                result,
            } => {
                write!(out, "    lock_cmpxchg16b.i128 {}, ", result.spelling())?;
                target.write(program, out)?;
                write!(out, ", ")?;
                expected.write(program, out)?;
                write!(out, ", ")?;
                return desired.write(program, out);
            }
        };
        write!(out, "    {name}.{ty} ")?;
        for (index, operand) in operands.into_iter().flatten().enumerate() {
            if index != 0 {
                write!(out, ", ")?;
            }
            operand.write(program, out)?;
        }
        Ok(())
    }
    pub(super) fn rebase(self, begin: i32, bytes: u32) -> Self {
        match self {
            Self::Exchange(op, ty, target, r) => {
                Self::Exchange(op, ty, target.rebase(begin, bytes), r)
            }
            Self::LoadWide {
                destination,
                source,
            } => Self::LoadWide {
                destination: destination.rebase(begin, bytes),
                source: source.rebase(begin, bytes),
            },
            Self::CompareWide {
                target,
                expected,
                desired,
                result,
            } => Self::CompareWide {
                target: target.rebase(begin, bytes),
                expected: expected.rebase(begin, bytes),
                desired: desired.rebase(begin, bytes),
                result,
            },
            Self::Fence => self,
        }
    }
    pub(super) fn emit(self, a: &mut Assembler, scratch: i32) -> Result<(), Error> {
        match self {
            Self::Fence => {
                a.byte(0x0f);
                a.byte(0xae);
                a.byte(0xf0);
                Ok(())
            }
            Self::Exchange(op, ty, target, value) => {
                let width = bytes(ty);
                if !matches!(op, Exchange::Swap) {
                    a.byte(0xf0);
                }
                let code = match (op, width == 1) {
                    (Exchange::Swap, true) => 0x86,
                    (Exchange::Swap, false) => 0x87,
                    (Exchange::Add, true) => 0x0fc0,
                    (Exchange::Add, false) => 0x0fc1,
                    (Exchange::Compare, true) => 0x0fb0,
                    (Exchange::Compare, false) => 0x0fb1,
                };
                a.op(
                    if width == 2 { 0x66 } else { 0 },
                    width == 8,
                    code,
                    if code > 0xff { 2 } else { 1 },
                    value.code(),
                    target.rm()?,
                    width == 1,
                    0,
                )
            }
            Self::LoadWide {
                destination,
                source,
            } => {
                a.lea(R::R11, source.rm()?)?;
                a.store(Rm::frame(scratch), R::RBX, 8)?;
                for r in [R::RAX, R::RDX, R::RCX, R::RBX] {
                    a.op(0, false, 0x33, 1, r.code(), Rm::Reg(r), false, 0)?;
                }
                compare16(a)?;
                a.load(R::RBX, Rm::frame(scratch), 8, false)?;
                a.store(destination.rm()?, R::RAX, 8)?;
                a.store(destination.displaced(8).rm()?, R::RDX, 8)
            }
            Self::CompareWide {
                target,
                expected,
                desired,
                result,
            } => {
                address_into(a, R::RCX, expected)?;
                address_into(a, R::R11, target)?;
                a.mov_rr(R::R10, R::RCX)?;
                a.store(Rm::frame(scratch), R::RBX, 8)?;
                a.load(R::RAX, Rm::mem(R::R10, 0), 8, false)?;
                a.load(R::RDX, Rm::mem(R::R10, 8), 8, false)?;
                super::wide_mir::half(a, R::RBX, desired, false)?;
                super::wide_mir::half(a, R::RCX, desired, true)?;
                compare16(a)?;
                a.store(Rm::mem(R::R10, 0), R::RAX, 8)?;
                a.store(Rm::mem(R::R10, 8), R::RDX, 8)?;
                a.load(R::RBX, Rm::frame(scratch), 8, false)?;
                ScalarOp::SetEqual(result).emit(a, true)?;
                ScalarOp::Movzx(result).emit(a, true)
            }
        }
    }
}
fn address_into(a: &mut Assembler, r: R, operand: Operand) -> Result<(), Error> {
    if let Some(source) = operand.register() {
        a.mov_rr(r, source)
    } else {
        a.lea(r, operand.rm()?)
    }
}
fn compare16(a: &mut Assembler) -> Result<(), Error> {
    a.byte(0xf0);
    a.op(0, true, 0x0fc7, 2, 1, Rm::mem(R::R11, 0), false, 0)
}
impl Selection<'_> {
    fn emit_atomic(&mut self, op: Atomic) {
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::Atomic(op),
            location: self.location,
        });
    }
    pub(super) fn atomic(&mut self, result: Option<TempId>, inst: &Runtime) -> Result<(), Error> {
        match *inst {
            Runtime::Fence(Fence::Thread, MemoryOrder::SequentiallyConsistent) => {
                self.emit_atomic(Atomic::Fence);
            }
            Runtime::Fence(..) => {}
            Runtime::AtomicLoad(Type::I128, address, _, _) => {
                self.mir.scratch_bytes = 48;
                let source = self.address(address, R::R10)?;
                let destination = Operand::Frame(self.placement.home(
                    self.body,
                    result.ok_or(Error::Limit)?,
                    &mut self.mir,
                )?);
                self.emit_atomic(Atomic::LoadWide {
                    destination,
                    source,
                });
            }
            Runtime::AtomicLoad(ty, address, _, _) => {
                self.read_memory(result.ok_or(Error::Limit)?, ty, address)?;
            }
            Runtime::AtomicStore(Type::I128, ..) => {
                return Err(Error::Diagnostic(
                    "atomic i128 store is not supported".into(),
                ));
            }
            Runtime::AtomicStore(ty, value, address, order) => {
                let target = self.address(address, R::R10)?;
                let mut value = self.operand(value, ty, R::R11)?;
                let sequential = matches!(order, MemoryOrder::SequentiallyConsistent);
                if sequential || value.is_memory() || value.is_symbol() {
                    self.load(R::R11, value, ty);
                    value = Operand::Register(R::R11);
                }
                if sequential {
                    self.emit_atomic(Atomic::Exchange(Exchange::Swap, ty, target, R::R11));
                } else {
                    self.emit(ScalarOp::Store(target, ty, value));
                }
            }
            Runtime::AtomicUpdate(op, ty, address, value, _) => {
                self.atomic_update(result.ok_or(Error::Limit)?, op, ty, address, value)?;
            }
            Runtime::AtomicCompareExchange {
                ty,
                address,
                expected,
                desired,
                ..
            } => {
                self.atomic_compare(result.ok_or(Error::Limit)?, ty, address, expected, desired)?;
            }
            _ => return Err(Error::Construction("non-atomic runtime instruction".into())),
        }
        Ok(())
    }
    fn atomic_update(
        &mut self,
        temp: TempId,
        op: AtomicUpdate,
        ty: Type,
        address: Value,
        delta: Value,
    ) -> Result<(), Error> {
        let target = self.address(address, R::R10)?;
        let mut value = if ty == Type::I128 {
            self.wide_operand(delta, R::R11)?
        } else {
            self.operand(delta, ty, R::R11)?
        };
        if value.is_memory() || value.is_symbol() {
            self.load(R::R11, value, ty);
            value = Operand::Register(R::R11);
        }
        let r = self.result(temp, [None, None], target.mask() | value.mask());
        self.load(r, value, Type::I64);
        self.emit_atomic(Atomic::Exchange(
            if op == AtomicUpdate::Exchange {
                Exchange::Swap
            } else {
                Exchange::Add
            },
            ty,
            target,
            r,
        ));
        self.normalize(r, ty);
        if op == AtomicUpdate::AddFetch {
            if value.register() == Some(r) {
                let scratch = if r == R::R11 { R::R10 } else { R::R11 };
                value = if ty == Type::I128 {
                    self.wide_operand(delta, scratch)?
                } else {
                    self.operand(delta, ty, scratch)?
                };
                if value.is_memory() || value.is_symbol() {
                    self.load(scratch, value, ty);
                    value = Operand::Register(scratch);
                }
            }
            self.emit(ScalarOp::Binary(Binary::Add, Type::I64, r, value));
            self.normalize(r, ty);
        }
        self.finish(temp, r)
    }
    fn atomic_compare(
        &mut self,
        temp: TempId,
        ty: Type,
        address: Value,
        expected: Value,
        desired: Value,
    ) -> Result<(), Error> {
        let target = self.address(address, R::R10)?;
        let expected = self.address(expected, R::R11)?;
        if ty == Type::I128 {
            self.mir.scratch_bytes = 48;
            let desired = self.wide_operand(desired, R::R10)?;
            let result = self.result(temp, [None, None], 0);
            self.emit_atomic(Atomic::CompareWide {
                target,
                expected,
                desired,
                result,
            });
            return self.finish(temp, result);
        }
        let source = self.facts.resolve(self.body, desired);
        let slot_address = matches!(source, Alias::Slot(_));
        // Desired values retain their own-width home until a staging register is chosen.
        let mut value = match source {
            Alias::Slot(slot) => Operand::Frame(self.placement.slot(slot)?),
            Alias::Temporary(t) => self.temp_operand(t)?,
            _ => self.operand(desired, ty, R::R10)?,
        };
        let mut borrowed = None;
        if value.register().is_none() {
            let (r, save) = self.placement.atomic_staging(
                target.mask() | expected.mask() | (1 << R::RAX.code()),
                &self.mir,
            )?;
            if let Some(home) = save {
                self.emit(ScalarOp::Store(
                    Operand::Frame(home),
                    Type::I64,
                    Operand::Register(r),
                ));
                borrowed = Some((r, home));
            }
            if slot_address {
                self.emit(ScalarOp::Lea(r, value));
            } else {
                let own = if value.is_memory() {
                    if let Alias::Temporary(t) = source {
                        self.body.temp(t).ty.ok_or(Error::Limit)?
                    } else {
                        ty
                    }
                } else {
                    ty
                };
                self.load(r, value, own);
            }
            value = Operand::Register(r);
        }
        self.emit(ScalarOp::Load(R::RAX, ty, expected));
        self.emit_atomic(Atomic::Exchange(
            Exchange::Compare,
            ty,
            target,
            value.register().ok_or(Error::Limit)?,
        ));
        self.emit(ScalarOp::Store(expected, ty, Operand::Register(R::RAX)));
        if let Some((r, home)) = borrowed {
            self.emit(ScalarOp::Load(r, Type::I64, Operand::Frame(home)));
        }
        let r = self.result(temp, [None, None], 0);
        self.emit(ScalarOp::SetEqual(r));
        self.emit(ScalarOp::Movzx(r));
        self.finish(temp, r)
    }
}
