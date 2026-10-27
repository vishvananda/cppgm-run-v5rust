//! Primitive floating conversion selection and implicit width changes. See NOTICE.
use super::{
    conversion::{Conversion, Kind},
    mir::{Instruction, Op},
    scalar_mir::{Operand, flip_sign},
    scalar_selection::Selection,
    value_selection::unsupported,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    model::{TempId, Value},
    vocabulary::{Convert, Type},
};
fn bits(ty: Type) -> u32 {
    if ty == Type::Ptr { 64 } else { ty.bits() }
}
impl Selection<'_> {
    pub(super) fn conversion(
        &mut self,
        kind: Kind,
        from: Type,
        to: Type,
        destination: Operand,
        source: Operand,
    ) {
        if from == Type::F80 || to == Type::F80 {
            self.mir.scratch_bytes = 48;
        }
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::Convert(Conversion {
                kind,
                from,
                to,
                destination,
                source,
            }),
            location: self.location,
        });
    }
    pub(super) fn width_conversion(
        &mut self,
        from: Type,
        to: Type,
        destination: Operand,
        source: Operand,
    ) {
        self.mir.scratch_bytes = 48;
        let kind = if to.float_rank() > from.float_rank() {
            Kind::Extend
        } else {
            Kind::Truncate
        };
        self.conversion(kind, from, to, destination, source);
    }
    fn wide_float_convert(
        &mut self,
        result: TempId,
        operation: Convert,
        to: Type,
        from: Type,
        value: Value,
    ) -> Result<(), Error> {
        let kind = match operation {
            Convert::SignedToFloat => Kind::SignedToFloat,
            Convert::UnsignedToFloat => Kind::UnsignedToFloat,
            Convert::FloatToSigned => Kind::FloatToSigned,
            Convert::FloatToUnsigned => Kind::FloatToUnsigned,
            Convert::FloatExtend => Kind::Extend,
            Convert::FloatTruncate => Kind::Truncate,
            _ => return Err(unsupported()),
        };
        let source = self.operand(value, from, R::R10)?;
        let memory_result = match operation {
            Convert::SignedToFloat
            | Convert::UnsignedToFloat
            | Convert::FloatExtend
            | Convert::FloatTruncate => to == Type::F80,
            _ => to == Type::I128,
        };
        let destination = if memory_result {
            Operand::Frame(self.placement.home(self.body, result, &mut self.mir)?)
        } else {
            Operand::Register(self.result(result, [None, None], 0))
        };
        self.conversion(kind, from, to, destination, source);
        if let Some(reg) = destination.register() {
            self.finish(result, reg)?;
        }
        Ok(())
    }
    pub(super) fn float_convert(
        &mut self,
        result: TempId,
        operation: Convert,
        to: Type,
        from: Type,
        value: Value,
    ) -> Result<(), Error> {
        if from == Type::I128 || to == Type::I128 {
            return self.wide_float_convert(result, operation, to, from, value);
        }
        match operation {
            Convert::SignedToFloat | Convert::UnsignedToFloat => {
                let mut source = self.operand(value, from, R::R11)?;
                let adjust = operation == Convert::SignedToFloat
                    && matches!(from, Type::U8 | Type::U16 | Type::U32)
                    || operation == Convert::UnsignedToFloat
                        && matches!(from, Type::I8 | Type::I16 | Type::I32);
                if source.register().is_none() || adjust {
                    if from.float_rank() != 0 && matches!(source, Operand::Immediate(_)) {
                        self.float_copy(from, Operand::Register(R::R11), source);
                    } else {
                        self.load(
                            R::R11,
                            source,
                            if source.is_memory() { from } else { Type::I64 },
                        );
                    }
                    if adjust {
                        self.normalize(R::R11, flip_sign(from));
                    }
                    source = Operand::Register(R::R11);
                }
                let kind = if operation == Convert::UnsignedToFloat && bits(from) == 64 {
                    Kind::UnsignedToFloat
                } else {
                    Kind::SignedToFloat
                };
                let destination = self.float_destination(result, to, [None, None])?;
                self.conversion(kind, from, to, destination, source);
                if let Some(r) = destination.register() {
                    self.finish(result, r)?;
                }
            }
            Convert::FloatToSigned | Convert::FloatToUnsigned => {
                let source = self.operand(value, from, R::R10)?;
                let destination = self.result(result, [None, None], 0);
                let kind = if operation == Convert::FloatToUnsigned && bits(to) == 64 {
                    Kind::FloatToUnsigned
                } else {
                    Kind::FloatToSigned
                };
                self.conversion(kind, from, to, Operand::Register(destination), source);
                self.normalize(destination, to);
                self.finish(result, destination)?;
            }
            Convert::FloatExtend | Convert::FloatTruncate => {
                let source = self.operand(value, from, R::R10)?;
                let prefer = if from == Type::F80 {
                    None
                } else {
                    self.dying(source, false)
                };
                let destination = self.float_destination(result, to, [prefer, None])?;
                let kind = if operation == Convert::FloatExtend {
                    Kind::Extend
                } else {
                    Kind::Truncate
                };
                self.conversion(kind, from, to, destination, source);
                if let Some(r) = destination.register() {
                    self.finish(result, r)?;
                }
            }
            _ => return Err(unsupported()),
        }
        Ok(())
    }
}
