//! Integer operation selection from typed values and separate placement facts. See NOTICE.
use super::{
    mir::{Function as MirFunction, Instruction, Op},
    placement::Placement,
    scalar_mir::{Operand, ScalarOp, signed},
    value_facts::{Alias, Facts, normalize},
    value_selection::unsupported,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{Body, DebugLocation, TempId, Value},
    vocabulary::{Binary, Type, Unary},
};
pub(super) struct Selection<'a> {
    pub(super) phis: super::control_selection::PhiInputs,
    pub(super) edge_moves: Vec<super::parallel_transfer::Transfer>,
    pub(super) parallel: super::parallel_transfer::Workspace,
    pub(super) outgoing: super::call_selection::Staging,
    pub(super) addresses: super::address_facts::Addresses,
    pub(super) body: &'a Body,
    pub(super) facts: Facts,
    pub(super) placement: Placement,
    pub(super) mir: MirFunction,
    pub(super) location: Option<DebugLocation>,
    pub(super) pos: usize,
}
impl Selection<'_> {
    pub(super) fn emit(&mut self, op: ScalarOp) {
        self.mir.instructions.push(Instruction {
            op: Op::Scalar(op),
            location: self.location,
        });
    }
    pub(super) fn operand(&mut self, value: Value, ty: Type, scratch: R) -> Result<Operand, Error> {
        match self.facts.resolve(self.body, value) {
            Alias::Constant(Constant::Integer(bits)) => Ok(Operand::Immediate(
                i128::from(super::mir::low_word(normalize(bits, ty))?).cast_unsigned(),
            )),
            Alias::Constant(Constant::Float(_, bits)) => Ok(Operand::Immediate(bits)),
            Alias::Symbol(id) => Ok(Operand::Symbol(id)),
            Alias::Slot(slot) => Ok(Operand::Frame(self.placement.slot(slot)?)),
            Alias::Temporary(temp) => {
                let op = if let Some(address) = self.addresses.folded[temp.index()] {
                    self.address(address, scratch)?
                } else if self.addresses.lazy[temp.index()] {
                    self.materialize(
                        scratch,
                        self.addresses.expressions[temp.index()].ok_or(Error::Limit)?,
                        false,
                    )?;
                    Operand::Register(scratch)
                } else {
                    self.temp_operand(temp)?
                };
                let own = self.body.temp(temp).ty.ok_or(Error::Limit)?;
                if op.is_memory() && super::abi::value_bytes(own)? != super::abi::value_bytes(ty)? {
                    self.load(scratch, op, own);
                    Ok(Operand::Register(scratch))
                } else {
                    Ok(op)
                }
            }
        }
    }
    pub(super) fn temp_operand(&mut self, temp: TempId) -> Result<Operand, Error> {
        if let Ok(operand) = self.placement.operand(temp) {
            return Ok(operand);
        }
        if self.facts.definitions[temp.index()]
            .is_some_and(|p| self.facts.flow.reachable[self.facts.flow.blocks[p].index()])
            || temp.index() < self.body.parameter_count
        {
            return Ok(Operand::Frame(self.placement.home(
                self.body,
                temp,
                &mut self.mir,
            )?));
        }
        Ok(Operand::Immediate(0))
    }
    pub(super) fn load(&mut self, r: R, source: Operand, ty: Type) {
        match source {
            Operand::Register(s) if s == r => {}
            Operand::Frame(d) => self.emit(ScalarOp::Load(r, ty, Operand::Frame(d))),
            Operand::Memory(m) => self.emit(ScalarOp::Load(r, ty, Operand::Memory(m))),
            _ => self.emit(ScalarOp::Move(r, source)),
        }
    }
    pub(super) fn result(&mut self, temp: TempId, prefs: [Option<R>; 2], exclude: u32) -> R {
        self.placement
            .result(&self.facts, temp, self.pos, prefs, exclude, &mut self.mir)
    }
    pub(super) fn finish(&mut self, temp: TempId, r: R) -> Result<(), Error> {
        self.placement.finish(
            self.body,
            &self.facts,
            temp,
            r,
            &mut self.mir,
            self.location,
        )
    }
    pub(super) fn dying(&self, op: Operand, skip_params: bool) -> Option<R> {
        self.placement
            .dying(&self.facts, op, self.pos, self.body, skip_params)
    }
    pub(super) fn normalize(&mut self, r: R, ty: Type) {
        match ty {
            Type::I1 => self.emit(ScalarOp::Binary(
                Binary::And,
                Type::I64,
                r,
                Operand::Immediate(1),
            )),
            Type::I8 | Type::I16 | Type::I32 | Type::U8 | Type::U16 | Type::U32 => {
                self.emit(ScalarOp::Extend(ty, r));
            }
            _ => {}
        }
    }
    pub(super) fn copy(&mut self, temp: TempId, ty: Type, value: Value) -> Result<(), Error> {
        if ty.float_rank() != 0 || matches!(ty, Type::I128 | Type::Object { .. }) {
            return Err(unsupported());
        }
        let own = if let Value::Temp(id) = value {
            self.body.temp(id).ty.ok_or(Error::Limit)?
        } else {
            Type::Ptr
        };
        let src = self.operand(value, own, R::R10)?;
        let r = self.result(temp, [self.dying(src, false), None], 0);
        if let Operand::Frame(d) = src {
            self.emit(ScalarOp::Load(r, ty, Operand::Frame(d)));
        } else {
            self.load(r, src, Type::I64);
            self.normalize(r, ty);
        }
        self.finish(temp, r)
    }
    pub(super) fn binary(
        &mut self,
        temp: TempId,
        op: Binary,
        ty: Type,
        left: Value,
        right: Value,
    ) -> Result<(), Error> {
        if ty.float_rank() != 0 || matches!(ty, Type::I128 | Type::Object { .. }) {
            return Err(unsupported());
        }
        if matches!(op, Binary::Shl | Binary::Shr | Binary::UShr) {
            return self.shift(temp, op, ty, left, right);
        }
        if matches!(op, Binary::Div | Binary::Mod | Binary::UDiv | Binary::UMod) {
            return self.divide(temp, op, ty, left, right);
        }
        if !matches!(
            op,
            Binary::Add | Binary::Sub | Binary::Mul | Binary::And | Binary::Or | Binary::Xor
        ) {
            return Err(unsupported());
        }
        let mut a = self.operand(left, ty, R::R10)?;
        let mut b = self.operand(right, ty, R::R11)?;
        if matches!(b, Operand::Symbol(_)) {
            self.load(R::R11, b, Type::Ptr);
            b = Operand::Register(R::R11);
        }
        if matches!(a, Operand::Symbol(_)) {
            self.load(R::R10, a, Type::Ptr);
            a = Operand::Register(R::R10);
        }
        self.right_register(op, ty, a, &mut b);
        let commutative = op != Binary::Sub;
        if commutative && a.register().is_none() && b.register().is_some() {
            std::mem::swap(&mut a, &mut b);
            self.right_register(op, ty, a, &mut b);
        }
        let dying_a = self.dying(a, true);
        let dying_b = if commutative {
            self.dying(b, true)
        } else {
            None
        };
        let exclude = (b.mask() & !dying_b.map_or(0, |r| 1 << r.code()))
            | self.placement.parameter_mask(self.body, [a, b]);
        let r = self.result(temp, [dying_a, dying_b], exclude);
        let width = if b.is_memory() { ty } else { Type::I64 };
        if a.register() == Some(r) {
            self.emit(ScalarOp::Binary(op, width, r, b));
        } else if commutative && b.register() == Some(r) {
            self.emit(ScalarOp::Binary(op, width, r, a));
        } else {
            self.load(r, a, if a.is_memory() { ty } else { Type::I64 });
            self.emit(ScalarOp::Binary(op, width, r, b));
        }
        if ty.bits() > 0
            && ty.bits() < 64
            && (width == Type::I64 && !matches!(op, Binary::And | Binary::Or | Binary::Xor)
                || width != Type::I64 && ty == Type::I32)
        {
            self.normalize(r, ty);
        }
        self.finish(temp, r)
    }
    fn right_register(&mut self, op: Binary, ty: Type, a: Operand, b: &mut Operand) {
        let wide_mem = matches!(ty, Type::I64 | Type::Ptr | Type::I32 | Type::U32);
        let large = matches!(*b,Operand::Immediate(v) if super::mir::low_word(v).is_ok_and(|v|i32::try_from(v).is_err()));
        if b.is_memory() && !wide_mem || op == Binary::Mul && large {
            let scratch = if a.register() == Some(R::R11) {
                R::R10
            } else {
                R::R11
            };
            self.load(scratch, *b, if b.is_memory() { ty } else { Type::I64 });
            *b = Operand::Register(scratch);
        }
    }
    pub(super) fn unary(
        &mut self,
        temp: TempId,
        op: Unary,
        ty: Type,
        value: Value,
    ) -> Result<(), Error> {
        if ty.float_rank() != 0 || matches!(ty, Type::I128 | Type::Object { .. }) {
            return Err(unsupported());
        }
        let mut source = self.operand(value, ty, R::R10)?;
        if op == Unary::Not {
            if matches!(source, Operand::Immediate(_)) {
                self.load(R::R10, source, Type::I64);
                source = Operand::Register(R::R10);
            }
            let r = self.result(temp, [None, None], 0);
            self.emit(ScalarOp::CompareZero(ty, source));
            self.emit(ScalarOp::SetEqual(r));
            self.emit(ScalarOp::Movzx(r));
            return self.finish(temp, r);
        }
        let r = self.result(temp, [self.dying(source, false), None], 0);
        if matches!(op, Unary::Ctz | Unary::Clz) {
            if matches!(source, Operand::Immediate(_)) {
                self.load(R::R10, source, ty);
                source = Operand::Register(R::R10);
            }
            self.emit(ScalarOp::Scan(op, ty, r, source));
            if op == Unary::Clz {
                self.emit(ScalarOp::Binary(
                    Binary::Xor,
                    Type::I64,
                    r,
                    Operand::Immediate(if super::scalar_mir::bytes(ty) == 8 {
                        63
                    } else {
                        31
                    }),
                ));
            }
        } else {
            self.load(r, source, ty);
            self.emit(ScalarOp::Unary(op, ty, r));
            if op != Unary::BitNot || !signed(ty) {
                self.normalize(r, ty);
            }
        }
        self.finish(temp, r)
    }
    fn shift(
        &mut self,
        temp: TempId,
        op: Binary,
        ty: Type,
        left: Value,
        right: Value,
    ) -> Result<(), Error> {
        let a = self.operand(left, ty, R::R10)?;
        let count = if let Alias::Constant(Constant::Integer(bits)) =
            self.facts.resolve(self.body, right)
        {
            Operand::Immediate(bits & 63)
        } else {
            let own = if let Value::Temp(t) = right {
                self.body.temp(t).ty.ok_or(Error::Limit)?
            } else {
                Type::Ptr
            };
            let source = self.operand(right, own, R::R11)?;
            self.load(R::RCX, source, Type::I64);
            Operand::Register(R::RCX)
        };
        let r = self.result(temp, [self.dying(a, false), None], 1 << R::RCX.code());
        self.load(r, a, if a.is_memory() { ty } else { Type::I64 });
        let unsigned = match ty {
            Type::I8 => Type::U8,
            Type::I16 => Type::U16,
            Type::I32 => Type::U32,
            other => other,
        };
        let signed_type = match ty {
            Type::U8 => Type::I8,
            Type::U16 => Type::I16,
            Type::U32 => Type::I32,
            other => other,
        };
        match op {
            Binary::Shl => {
                self.emit(ScalarOp::Shift(op, r, count));
                self.normalize(r, ty);
            }
            Binary::UShr => {
                if signed(ty) && ty != Type::I64 {
                    self.normalize(r, unsigned);
                }
                self.emit(ScalarOp::Shift(op, r, count));
                if signed(ty) && ty != Type::I64 {
                    self.normalize(r, ty);
                }
            }
            _ => {
                let narrow_unsigned = ty.bits() > 0 && ty.bits() < 64 && !signed(ty);
                if narrow_unsigned {
                    self.normalize(r, signed_type);
                }
                self.emit(ScalarOp::Shift(op, r, count));
                if narrow_unsigned {
                    self.normalize(r, ty);
                }
            }
        }
        self.finish(temp, r)
    }
}
