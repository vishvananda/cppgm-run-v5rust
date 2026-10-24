// Translated from lower/soft_float_lowering.cpp's static conversion queries. See NOTICE.
//! Static values are computed while analysis owns their typed scalar operands.
use super::{
    Error,
    analyzer::Analyzer,
    static_initialization::Value,
    types::{Scalar, TypeId, TypeKind},
};
use crate::types::interchange;
impl Analyzer<'_> {
    pub(super) fn static_soft_conversion(
        &self,
        value: Value,
        source: TypeId,
        target: TypeId,
    ) -> Result<Option<Value>, Error> {
        if let Some(kind) = self.integral_kind(target) {
            let Value::Floating(ty, bits) = value else {
                return Ok(None);
            };
            let Some(bits) = interchange::to_integer(ty, bits, kind.bits, kind.signed) else {
                return Ok(None);
            };
            return if kind.bits > 64 {
                Ok(Some(Value::WideInteger(bits)))
            } else {
                let low = u64::try_from(bits & u128::from(u64::MAX))
                    .map_err(|_| Error::Invariant("soft integral low bits"))?;
                Ok(Some(Value::Integer(kind.convert(low))))
            };
        }
        let TypeKind::Scalar(Scalar::Basic(to)) = self.graph.types.get(target).kind else {
            return Ok(None);
        };
        if to.floating_rank() == 0 {
            return Ok(None);
        }
        let bits = match value {
            Value::Floating(from, bits) => interchange::convert(from, bits, to),
            Value::Integer(bits) => {
                let signed = self
                    .integral_kind(source)
                    .is_some_and(|kind| kind.signed && kind.bits > 1);
                let wide = if signed {
                    i128::from(bits.cast_signed()).cast_unsigned()
                } else {
                    u128::from(bits)
                };
                interchange::from_integer(wide, signed, to)
            }
            Value::WideInteger(bits) => interchange::from_integer(
                bits,
                self.integral_kind(source).is_some_and(|kind| kind.signed),
                to,
            ),
            _ => return Ok(None),
        }
        .map_err(|_| Error::Limit("soft static conversion"))?;
        Ok(Some(Value::Floating(to, bits)))
    }
}
