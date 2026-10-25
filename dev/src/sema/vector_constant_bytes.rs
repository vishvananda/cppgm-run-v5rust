// Translated from sema/constant_complex.cpp's vector_reinterpretation. See NOTICE.
//! Bounded little-endian representation casts retain target integer and native-float rules.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, TemporaryValue as Value, nonconstant},
    constant_floats::Floating,
    constants::Constant,
    graph::ScopeId,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::ast,
    types::{
        FundamentalType as F,
        float_value::{BinaryFloat, Format},
    },
};

fn native_format(kind: &TypeKind) -> Option<Format> {
    match kind {
        TypeKind::Scalar(Scalar::Basic(F::Float)) => Some(Format::Single),
        TypeKind::Scalar(Scalar::Basic(F::Double)) => Some(Format::Double),
        _ => None,
    }
}

impl Analyzer<'_> {
    pub(super) fn constant_vector_cast(
        &mut self,
        operand: ast::ExprId,
        to: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let source = self
            .graph
            .expression_facts
            .function_conversion(operand)
            .map_or(self.graph.expression_facts.get(operand)?.ty, |step| {
                step.after.result
            });
        let from = self.bare_type(source)?;
        let to = self.bare_type(to)?;
        if from == to {
            return self.constant_vector_value(operand, scope);
        }
        let mut bytes = [0; 256];
        let size = self.constant_vector_bytes(operand, to, scope, location, &mut bytes)?;
        let Some((lane, bound)) = self.constant_lane_vector(to) else {
            return Err(nonconstant(
                location,
                "a vector reinterpretation outside constant evaluation",
            ));
        };
        let stride = usize::try_from(self.type_size(lane, location)?.0)
            .map_err(|_| Error::Limit("vector lane size"))?;
        if stride == 0
            || bound
                .checked_mul(u64::try_from(stride).map_err(|_| Error::Limit("vector lane size"))?)
                != Some(u64::try_from(size).map_err(|_| Error::Limit("vector cast size"))?)
        {
            return Err(nonconstant(
                location,
                "a vector reinterpretation outside constant evaluation",
            ));
        }
        let place = self.allocate_constant_object(to, None, location)?;
        for (index, bytes) in bytes[..size].chunks_exact(stride).enumerate() {
            let Some(value) = self.vector_bytes_scalar(bytes, lane)? else {
                return Err(nonconstant(
                    location,
                    "a vector reinterpretation making a lane outside constant evaluation",
                ));
            };
            let offset = u64::try_from(
                index
                    .checked_mul(stride)
                    .ok_or(Error::Limit("vector lane offset"))?,
            )
            .map_err(|_| Error::Limit("vector lane offset"))?;
            self.store_constant_value(
                Address {
                    designated: lane,
                    offset: place.offset.wrapping_add(super::integral::signed(offset)),
                    ..place
                },
                value,
                location,
            )?;
        }
        Ok(place)
    }
    pub(super) fn constant_vector_scalar_cast(
        &mut self,
        operand: ast::ExprId,
        to: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        let mut bytes = [0; 256];
        let size = self.constant_vector_bytes(operand, to, scope, location, &mut bytes)?;
        self.vector_bytes_scalar(&bytes[..size], to)?
            .ok_or_else(|| {
                nonconstant(
                    location,
                    "a vector reinterpretation outside constant evaluation",
                )
            })
    }
    fn constant_vector_bytes(
        &mut self,
        operand: ast::ExprId,
        to: TypeId,
        scope: ScopeId,
        location: SourceOffset,
        bytes: &mut [u8; 256],
    ) -> Result<usize, Error> {
        let source = self
            .graph
            .expression_facts
            .function_conversion(operand)
            .map_or(self.graph.expression_facts.get(operand)?.ty, |step| {
                step.after.result
            });
        let from = self.bare_type(source)?;
        // Evaluate the operand before rejecting the byte representation, as convert_scalar does.
        let vector = self.vector_shape(from).is_some();
        let place = if vector {
            Some(self.constant_vector_value(operand, scope)?)
        } else {
            None
        };
        let scalar = if vector {
            None
        } else {
            Some(self.scalar_value(operand, scope)?)
        };
        if scalar.is_some_and(Value::indeterminate) {
            return Err(nonconstant(
                location,
                "a conversion of an indeterminate value",
            ));
        }
        let size = self.type_size(from, location)?.0;
        if size == 0 || size > 256 || size != self.type_size(to, location)?.0 {
            return Err(nonconstant(
                location,
                "a vector reinterpretation outside constant evaluation",
            ));
        }
        let size = usize::try_from(size).map_err(|_| Error::Limit("vector cast size"))?;
        if let Some(place) = place {
            let Some((lane, bound)) = self.constant_lane_vector(from) else {
                return Err(nonconstant(
                    location,
                    "a vector reinterpretation outside constant evaluation",
                ));
            };
            let stride = usize::try_from(self.type_size(lane, location)?.0)
                .map_err(|_| Error::Limit("vector lane size"))?;
            if stride == 0
                || u64::try_from(size / stride).map_err(|_| Error::Limit("vector lane count"))?
                    != bound
                || size % stride != 0
            {
                return Err(nonconstant(
                    location,
                    "a vector reinterpretation outside constant evaluation",
                ));
            }
            for (index, bytes) in bytes[..size].chunks_exact_mut(stride).enumerate() {
                let offset = u64::try_from(
                    index
                        .checked_mul(stride)
                        .ok_or(Error::Limit("vector lane offset"))?,
                )
                .map_err(|_| Error::Limit("vector lane offset"))?;
                let value = self.constexpr_frames.read_value(
                    Address {
                        designated: lane,
                        offset: place.offset.wrapping_add(super::integral::signed(offset)),
                        ..place
                    },
                    location,
                )?;
                if value.is_none_or(|value| !self.vector_scalar_bytes(value, lane, bytes)) {
                    return Err(nonconstant(
                        location,
                        "a vector reinterpretation outside constant evaluation",
                    ));
                }
            }
        } else if !self.vector_scalar_bytes(
            scalar.ok_or(Error::Invariant("vector cast scalar"))?,
            from,
            &mut bytes[..size],
        ) {
            return Err(nonconstant(
                location,
                "a vector reinterpretation outside constant evaluation",
            ));
        }
        Ok(size)
    }
    fn vector_scalar_bytes(&self, value: Value, ty: TypeId, bytes: &mut [u8]) -> bool {
        if bytes.len() > 16 {
            return false;
        }
        let bits = match value {
            Value::Integer(value) => value.wide(),
            Value::Floating(value) => {
                if value.value.is_nan() && value.value.extended_bits() & (1 << 62) == 0 {
                    return false;
                }
                let Some(format) = native_format(&self.graph.types.get(ty).kind) else {
                    return false;
                };
                let Some(bits) = value.value.bits(format) else {
                    return false;
                };
                bits
            }
            _ => return false,
        };
        bytes.copy_from_slice(&bits.to_le_bytes()[..bytes.len()]);
        true
    }
    fn vector_bytes_scalar(&mut self, bytes: &[u8], ty: TypeId) -> Result<Option<Value>, Error> {
        if bytes.len() > 16 {
            return Ok(None);
        }
        let mut buffer = [0; 16];
        buffer[..bytes.len()].copy_from_slice(bytes);
        let bits = u128::from_le_bytes(buffer);
        if self.integral_kind(ty).is_some() {
            let value = Constant {
                state: crate::sema::constant_addresses::ScalarState::Value,
                ty,
                bits: u64::try_from(bits & u128::from(u64::MAX))
                    .map_err(|_| Error::Invariant("vector integer low half"))?,
                high: u64::try_from(bits >> 64)
                    .map_err(|_| Error::Invariant("vector integer high half"))?,
            };
            return self
                .convert_integer_constant(value, ty)
                .map(Value::Integer)
                .map(Some);
        }
        let Some(format) = native_format(&self.graph.types.get(ty).kind) else {
            return Ok(None);
        };
        let Some(value) = BinaryFloat::from_bits(format, bits) else {
            return Ok(None);
        };
        if value.is_nan() && value.extended_bits() & (1 << 62) == 0 {
            return Ok(None);
        }
        Ok(Some(Value::Floating(Floating {
            state: crate::sema::constant_addresses::ScalarState::Value,
            ty,
            value,
        })))
    }
}
