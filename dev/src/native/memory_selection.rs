//! Typed scalar memory selection and address materialization. See NOTICE.
use super::{
    abi,
    address_facts::{AddressBase, Expression},
    memory::{Base, Memory},
    mir::low_word,
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    value_facts::Alias,
    value_selection::unsupported,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{TempId, Value},
    vocabulary::{Binary, Type},
};
impl Selection<'_> {
    fn temp_register(&mut self, temp: TempId, scratch: R) -> Result<R, Error> {
        let source = self.temp_operand(temp)?;
        if let Some(r) = source.register() {
            return Ok(r);
        }
        let ty = self.body.temp(temp).ty.ok_or(Error::Limit)?;
        self.load(scratch, source, ty);
        Ok(scratch)
    }
    pub(super) fn address(&mut self, value: Value, scratch: R) -> Result<Operand, Error> {
        let mem = match self.facts.resolve(self.body, value) {
            Alias::Slot(slot) => Memory::frame(self.placement.slot(slot)?),
            Alias::Symbol(id) => Memory::new(Base::Symbol(id), 0),
            Alias::Constant(Constant::Integer(bits)) => Memory::new(
                Base::Absolute,
                i32::from_le_bytes(
                    low_word(bits)?.to_le_bytes()[..4]
                        .try_into()
                        .map_err(|_| Error::Limit)?,
                ),
            ),
            Alias::Temporary(temp) => {
                if matches!(self.body.temp(temp).ty, Some(Type::Object { .. })) {
                    return Ok(Operand::Frame(self.placement.home(
                        self.body,
                        temp,
                        &mut self.mir,
                    )?));
                }
                if self.addresses.lazy[temp.index()] {
                    return self.lazy_memory(
                        self.addresses.expressions[temp.index()].ok_or(Error::Limit)?,
                        scratch,
                        None,
                    );
                }
                super::memory::scalar(self.body.temp(temp).ty.ok_or(Error::Limit)?)?;
                Memory::new(Base::Register(self.temp_register(temp, scratch)?), 0)
            }
            Alias::Constant(_) => return Err(unsupported()),
        };
        Ok(Operand::Memory(mem))
    }
    fn lazy_memory(
        &mut self,
        expr: Expression,
        scratch: R,
        spare: Option<R>,
    ) -> Result<Operand, Error> {
        let disp = i32::from_le_bytes(
            expr.displacement.to_le_bytes()[..4]
                .try_into()
                .map_err(|_| Error::Limit)?,
        );
        let mut mem = match expr.base {
            AddressBase::Slot(slot) => Memory::frame(self.placement.slot(slot)?.wrapping_add(disp)),
            AddressBase::Symbol(id) => Memory::new(Base::Symbol(id), disp),
            AddressBase::Absolute => Memory::new(Base::Absolute, disp),
            AddressBase::Home(temp) => Memory::frame(
                self.placement
                    .home(self.body, temp, &mut self.mir)?
                    .wrapping_add(disp),
            ),
            AddressBase::Temporary(temp) => {
                let source = self.temp_operand(temp)?;
                if source.register().is_none()
                    && spare.is_none()
                    && let Some(index) = expr.index
                    && self.temp_operand(index)?.register().is_none()
                {
                    self.temp_register(index, scratch)?;
                    if expr.scale > 1 {
                        self.emit(ScalarOp::Shift(
                            Binary::Shl,
                            scratch,
                            Operand::Immediate(u128::from(expr.scale.trailing_zeros())),
                        ));
                    }
                    self.emit(ScalarOp::Binary(Binary::Add, Type::I64, scratch, source));
                    return Ok(Operand::Memory(Memory::new(Base::Register(scratch), disp)));
                }
                Memory::new(Base::Register(self.temp_register(temp, scratch)?), disp)
            }
        };
        if let Some(index) = expr.index {
            let work = if matches!(mem.base,Base::Register(r) if r==scratch) {
                spare.unwrap_or(scratch)
            } else {
                scratch
            };
            mem.index = Some(self.temp_register(index, work)?);
            mem.scale = expr.scale;
        }
        Ok(Operand::Memory(mem))
    }
    pub(super) fn materialize(
        &mut self,
        destination: R,
        expr: Expression,
        exclusive: bool,
    ) -> Result<(), Error> {
        if expr.index.is_none() {
            match expr.base {
                AddressBase::Symbol(id) => {
                    self.emit(ScalarOp::Move(
                        destination,
                        Operand::SymbolOffset(
                            id,
                            i32::try_from(expr.displacement).map_err(|_| Error::Limit)?,
                        ),
                    ));
                    return Ok(());
                }
                AddressBase::Absolute => {
                    self.emit(ScalarOp::Move(
                        destination,
                        Operand::Immediate(i128::from(expr.displacement).cast_unsigned()),
                    ));
                    return Ok(());
                }
                _ => {}
            }
        }
        let scratch = if matches!(destination, R::R10 | R::R11) {
            destination
        } else {
            R::R10
        };
        let spare = exclusive.then_some(if scratch == R::R10 { R::R11 } else { R::R10 });
        let mem = self.lazy_memory(expr, scratch, spare)?;
        if let Some(memory) = mem.memory()
            && memory.index.is_none()
            && memory.displacement == 0
            && let Base::Register(r) = memory.base
        {
            if r != destination {
                self.emit(ScalarOp::Move(destination, Operand::Register(r)));
            }
            return Ok(());
        }
        self.emit(ScalarOp::Lea(destination, mem));
        Ok(())
    }
    pub(super) fn read_memory(
        &mut self,
        result: TempId,
        ty: Type,
        address: Value,
    ) -> Result<(), Error> {
        super::memory::scalar(ty)?;
        let mem = self.address(address, R::R10)?;
        let prefer = if self.facts.intervals[result.index()].hint.is_none() {
            mem.memory().and_then(|m| {
                if let Base::Register(r) = m.base {
                    self.dying(Operand::Register(r), false)
                } else {
                    None
                }
            })
        } else {
            None
        };
        let r = self.result(result, [prefer, None], 0);
        self.emit(ScalarOp::Load(r, ty, mem));
        self.finish(result, r)
    }
    pub(super) fn write_memory(
        &mut self,
        ty: Type,
        value: Value,
        address: Value,
    ) -> Result<(), Error> {
        super::memory::scalar(ty)?;
        let mut source = self.operand(value, ty, R::R11)?;
        let mem = self.address(address, R::R10)?;
        if source.is_memory() || matches!(source, Operand::Symbol(_) | Operand::SymbolOffset(..)) {
            self.load(R::R11, source, ty);
            source = Operand::Register(R::R11);
        }
        self.emit(ScalarOp::Store(mem, ty, source));
        Ok(())
    }
    pub(super) fn select_address(&mut self, result: TempId) -> Result<(), Error> {
        let expr = self.addresses.expressions[result.index()].ok_or_else(unsupported)?;
        let r = self.result(result, [None, None], 0);
        self.materialize(r, expr, true)?;
        self.finish(result, r)
    }
    pub(super) fn select_index(
        &mut self,
        result: TempId,
        ty: Type,
        base: Value,
        offset: Value,
    ) -> Result<(), Error> {
        if let Some(expr) = self.addresses.expressions[result.index()] {
            let a = match self.facts.resolve(self.body, base) {
                Alias::Temporary(t) => self.placement.operand(t).ok(),
                _ => None,
            };
            let b = match self.facts.resolve(self.body, offset) {
                Alias::Temporary(t) => self.placement.operand(t).ok(),
                _ => None,
            };
            let prefs = [
                a.and_then(|a| self.dying(a, false)),
                b.and_then(|b| self.dying(b, false)),
            ];
            let r = self.result(result, prefs, 0);
            self.materialize(r, expr, true)?;
            return self.finish(result, r);
        }
        let size = abi::value_bytes(ty)?;
        let a = self.operand(base, Type::Ptr, R::R10)?;
        let own = if let Value::Temp(t) = offset {
            self.body.temp(t).ty.ok_or(Error::Limit)?
        } else {
            Type::Ptr
        };
        let b = self.operand(offset, own, R::R11)?;
        let r = self.result(result, [self.dying(b, false), None], a.mask());
        self.load(r, b, if b.is_memory() { own } else { Type::I64 });
        if size != 1 {
            self.emit(ScalarOp::Binary(
                Binary::Mul,
                Type::I64,
                r,
                Operand::Immediate(u128::from(size)),
            ));
        }
        let a = if matches!(a, Operand::Symbol(_) | Operand::SymbolOffset(..)) {
            let scratch = if r == R::R10 { R::R11 } else { R::R10 };
            self.load(scratch, a, Type::Ptr);
            Operand::Register(scratch)
        } else {
            a
        };
        self.emit(ScalarOp::Binary(Binary::Add, Type::I64, r, a));
        self.finish(result, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Facts as LowFacts, InstructionKind as I, Program},
    };
    fn chosen(program: &Program) -> Result<super::super::mir::Function, Error> {
        let f = &program.functions()[0];
        super::super::value_selection::select(
            program,
            program.find_symbol(f.name).ok_or(Error::Limit)?,
            f,
            0,
        )
    }
    #[test]
    fn a_lazy_address_keeps_its_components_live_through_all_readers() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "lazy", &[("p", Type::Ptr)], Type::I64)?;
        builder.block("entry")?;
        let p = builder.parameter(0)?;
        let one = builder.integer(1)?;
        let address = builder.value(I::Index(Type::I64, None, p, one), "address")?;
        let mut values = Vec::new();
        for i in 0..3 {
            values.push(builder.value(
                I::Binary(Binary::Add, LowFacts::default(), Type::I64, one, one),
                &format!("v{i}"),
            )?);
        }
        for value in values {
            builder.effect(I::Store(LowFacts::default(), Type::I64, value, address))?;
        }
        let value = builder.value(I::Load(LowFacts::default(), Type::I64, address), "read")?;
        builder.effect(I::Return(Type::I64, Some(value)))?;
        builder.finish()?;
        let selected = chosen(&program)?;
        let destinations: Vec<_> = selected
            .instructions
            .iter()
            .filter_map(|inst| {
                if let super::super::mir::Op::Scalar(ScalarOp::Binary(_, _, r, _)) = inst.op {
                    Some(r)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(destinations, [R::R8, R::R9, R::RSI]);
        assert_eq!(selected.lazy_addresses, 1);
        Ok(())
    }
    #[test]
    fn only_the_over_aligned_slot_region_uses_the_aligned_base() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "aligned", &[], Type::Ptr)?;
        let buffer = builder.slot(
            "buffer",
            Type::Object {
                bytes: 64,
                align: 6,
            },
        )?;
        let other = builder.slot("other", Type::Object { bytes: 8, align: 3 })?;
        builder.block("entry")?;
        builder.value(I::Addr(other), "dead-address")?;
        let address = builder.value(I::Addr(buffer), "address")?;
        builder.effect(I::Return(Type::Ptr, Some(address)))?;
        builder.finish()?;
        let selected = chosen(&program)?;
        assert_eq!(selected.aligned_base, 64);
        assert_eq!(selected.aligned_disp, -112);
        assert_eq!(selected.aligned_bytes, 64);
        assert_eq!(selected.bindings[1].offset, -136);
        assert_eq!(selected.preserve, 1 << R::R12.code());
        let mut dump = Vec::new();
        selected.write(&program, &mut dump)?;
        let dump = std::str::from_utf8(&dump).map_err(|_| Error::Limit)?;
        assert!(dump.contains("slot $buffer ->"));
        assert!(dump.contains("slot $other ->"));
        let remapped = Memory::frame(-112).rebased(selected.aligned_disp, selected.aligned_bytes);
        assert!(matches!(remapped.base, Base::Register(R::R12)));
        assert_eq!(remapped.displacement, 0);
        assert_eq!(
            Memory::frame(-136)
                .rebased(selected.aligned_disp, selected.aligned_bytes)
                .frame_offset(),
            Some(-136)
        );
        Ok(())
    }
    #[test]
    fn an_unused_volatile_read_remains_observable() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "reads", &[("p", Type::Ptr)], Type::I64)?;
        builder.block("entry")?;
        let p = builder.parameter(0)?;
        builder.value(I::Load(LowFacts::default(), Type::I64, p), "dead")?;
        builder.value(
            I::Load(LowFacts::memory(true, false), Type::I64, p),
            "volatile",
        )?;
        let zero = builder.integer(0)?;
        builder.effect(I::Return(Type::I64, Some(zero)))?;
        builder.finish()?;
        let selected = chosen(&program)?;
        assert_eq!(
            selected
                .instructions
                .iter()
                .filter(|inst| matches!(inst.op, super::super::mir::Op::Scalar(ScalarOp::Load(..))))
                .count(),
            1
        );
        Ok(())
    }
}
