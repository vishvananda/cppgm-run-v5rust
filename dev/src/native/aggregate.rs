//! Memory-class ABI values, result homes and aggregate returns. See NOTICE.
use super::{
    abi,
    bulk::Bulk,
    mir::{Instruction, Op},
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    value_facts::Alias,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{TempId, Value},
    vocabulary::Type,
};

pub(super) fn memory_value(ty: Type) -> bool {
    matches!(ty, Type::Object { .. } | Type::I128)
}
pub(super) fn part_source(source: Operand, part: abi::Part) -> Operand {
    if let Operand::Immediate(bits) = source {
        let bits = if part.offset == 0 { bits } else { bits >> 64 };
        Operand::Immediate(bits & u128::from(u64::MAX))
    } else {
        source.displaced(part.offset.cast_signed())
    }
}
impl Selection<'_> {
    pub(super) fn aggregate_operand(&mut self, value: Value, ty: Type) -> Result<Operand, Error> {
        let source = self.aggregate_source(value)?;
        if matches!(ty, Type::Object { .. })
            && matches!(
                self.facts.resolve(self.body, value),
                Alias::Constant(Constant::Integer(_))
            )
            && let Operand::Immediate(bits) = source
        {
            Ok(Operand::Immediate(
                i128::from(super::mir::low_word(bits)?).cast_unsigned(),
            ))
        } else {
            Ok(source)
        }
    }
    pub(super) fn aggregate_source(&mut self, value: Value) -> Result<Operand, Error> {
        match self.facts.resolve(self.body, value) {
            Alias::Temporary(temp) => self.temp_operand(temp),
            Alias::Slot(slot) => Ok(Operand::Frame(self.placement.slot(slot)?)),
            Alias::Constant(Constant::Integer(bits) | Constant::Float(_, bits)) => {
                Ok(Operand::Immediate(bits))
            }
            Alias::Symbol(_) => Err(super::value_selection::unsupported()),
        }
    }
    pub(super) fn wide_load(&mut self, result: TempId, address: Value) -> Result<(), Error> {
        let source = self.address(address, R::R10)?;
        let destination = Operand::Frame(self.placement.home(self.body, result, &mut self.mir)?);
        self.mir.instructions.push(Instruction {
            op: Op::WideMove {
                destination,
                source,
            },
            location: self.location,
        });
        Ok(())
    }
    pub(super) fn wide_store(&mut self, value: Value, address: Value) -> Result<(), Error> {
        let source = self.aggregate_source(value)?;
        let destination = self.address(address, R::R10)?;
        self.mir.instructions.push(Instruction {
            op: Op::WideMove {
                destination,
                source,
            },
            location: self.location,
        });
        Ok(())
    }
    pub(super) fn wide_copy(&mut self, result: TempId, value: Value) -> Result<(), Error> {
        let source = self.aggregate_source(value)?;
        let destination = Operand::Frame(self.placement.home(self.body, result, &mut self.mir)?);
        self.mir.instructions.push(Instruction {
            op: Op::WideMove {
                destination,
                source,
            },
            location: self.location,
        });
        Ok(())
    }
    pub(super) fn aggregate_return(&mut self, ty: Type, value: Value) -> Result<(), Error> {
        let source = self.aggregate_operand(value, ty)?;
        let bytes = abi::value_bytes(ty)?;
        if self.mir.layout.indirect_result {
            self.emit(ScalarOp::Load(
                R::RAX,
                Type::Ptr,
                Operand::Frame(self.placement.sret_home.ok_or(Error::Limit)?),
            ));
            let copy = Bulk::new(
                Operand::Register(R::RAX),
                Some(source),
                u64::from(bytes),
                abi::value_align(ty)?,
            );
            self.mir.instructions.push(Instruction {
                op: Op::Bulk(copy),
                location: self.location,
            });
            self.mir.instructions.push(Instruction {
                op: Op::Return(Some(R::RAX)),
                location: self.location,
            });
            return Ok(());
        }
        if !self.mir.layout.classified_result
            && let Operand::Immediate(bits) = source
        {
            for (register, bits) in [(R::RAX, bits), (R::RDX, bits >> 64)] {
                self.mir.instructions.push(Instruction {
                    op: Op::Move(register, bits & u128::from(u64::MAX)),
                    location: self.location,
                });
            }
            self.mir.instructions.push(Instruction {
                op: Op::Return(None),
                location: self.location,
            });
            return Ok(());
        }
        for part in self.mir.layout.result.into_iter().flatten() {
            let register = part.register.ok_or(Error::Limit)?;
            let source = part_source(source, part);
            if register.code() >= 16 {
                self.load(register, source, part.move_type());
            } else if source.is_memory() {
                self.emit(ScalarOp::Load(register, Type::I64, source));
            } else {
                self.load(register, source, Type::I64);
            }
        }
        self.mir.instructions.push(Instruction {
            op: Op::Return(None),
            location: self.location,
        });
        Ok(())
    }
    pub(super) fn aggregate_call_result(
        &mut self,
        result: TempId,
        layout: &abi::Layout,
    ) -> Result<(), Error> {
        if layout.indirect_result {
            return Ok(());
        }
        let home = Operand::Frame(self.placement.home(self.body, result, &mut self.mir)?);
        for part in layout.result.iter().flatten() {
            let register = part.register.ok_or(Error::Limit)?;
            self.store_transfer(
                home.displaced(part.offset.cast_signed()),
                part.move_type(),
                Operand::Register(register),
                None,
                false,
            );
        }
        Ok(())
    }
}
