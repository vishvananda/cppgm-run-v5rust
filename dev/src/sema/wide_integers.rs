// Translated from sema/integral_arithmetic.cpp and constant_expressions.cpp. See NOTICE.
//! Full-width interpreter values and the checked projection into integral constants.
use super::{
    Error, analyzer::Analyzer, constants::Constant, integral::IntegralKind, types::TypeId,
};
use crate::{lex::pp_lexer::Punctuator as P, support::source::SourceOffset, syntax::ast};

impl Constant {
    pub(super) fn truth(self) -> bool {
        self.bits != 0 || self.high != 0
    }
    pub(super) fn wide(self) -> u128 {
        (u128::from(self.high) << 64) | u128::from(self.bits)
    }
    pub(super) fn from_wide(ty: TypeId, bits: u128) -> Self {
        let bytes = bits.to_le_bytes();
        let mut low = [0; 8];
        let mut high = [0; 8];
        low.copy_from_slice(&bytes[..8]);
        high.copy_from_slice(&bytes[8..]);
        Self {
            ty,
            bits: u64::from_le_bytes(low),
            high: u64::from_le_bytes(high),
        }
    }
}

impl Analyzer<'_> {
    pub(super) fn integral_value(&self, ty: TypeId, bits: u64) -> Constant {
        Constant {
            ty,
            bits,
            high: if self
                .integral_kind(ty)
                .is_some_and(|kind| kind.bits > 64 && kind.signed)
                && bits >> 63 != 0
            {
                u64::MAX
            } else {
                0
            },
        }
    }
    pub(super) fn held_integral_constant(&self, value: Constant) -> bool {
        value.high == self.integral_value(value.ty, value.bits).high
    }
    pub(super) fn wide_integer(&self, value: Constant) -> Result<u128, Error> {
        let kind = self
            .integral_kind(value.ty)
            .ok_or(Error::Invariant("nonintegral full-width value"))?;
        Ok(if kind.bits > 64 {
            value.wide()
        } else if kind.signed {
            i128::from(super::integral::signed(value.bits)).cast_unsigned()
        } else {
            u128::from(value.bits)
        })
    }
    pub(super) fn convert_integer_constant(
        &self,
        value: Constant,
        ty: TypeId,
    ) -> Result<Constant, Error> {
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("nonintegral conversion target"))?;
        Ok(if kind.bits > 64 {
            Constant::from_wide(ty, self.wide_integer(value)?)
        } else {
            self.integral_value(
                ty,
                if kind.bits == 1 {
                    u64::from(value.truth())
                } else {
                    kind.convert(value.bits)
                },
            )
        })
    }
    pub(super) fn wide_arithmetic(
        &mut self,
        op: P,
        left: Constant,
        right: Constant,
        types: (TypeId, TypeId),
        skipping: bool,
        id: ast::ExprId,
    ) -> Result<Constant, Error> {
        let (ty, right_ty) = types;
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("wide operation type"))?;
        let a = self.wide_integer(left)?;
        let b = self.wide_integer(right)?;
        if let Some(result) = compare(op, kind.signed, a, b) {
            return self.boolean(result);
        }
        let shift = matches!(op, P::ShiftLeft | P::ShiftRight);
        let (bits, valid) = if shift {
            let count_kind = self
                .integral_kind(right_ty)
                .ok_or(Error::Invariant("wide shift count type"))?;
            let (bits, valid) = shift_value(op, kind.signed, a, right.bits, count_kind);
            (bits, valid && right.high == 0)
        } else {
            operation(op, kind.signed, a, b).ok_or(Error::Invariant("wide arithmetic operator"))?
        };
        if !valid && !skipping {
            return Err(super::constant_addresses::nonconstant(
                self.tree.expressions.get(id).location,
                if shift {
                    "a shift outside its defined range"
                } else {
                    "an arithmetic operation with an undefined result"
                },
            ));
        }
        Ok(Constant::from_wide(ty, bits))
    }
    pub(super) fn wide_unary(
        &self,
        op: Option<P>,
        value: Constant,
        ty: TypeId,
        skipping: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let bits = self.wide_integer(value)?;
        let result = match op {
            Some(P::Subtract) => {
                let signed = self.integral_kind(ty).is_some_and(|kind| kind.signed);
                let (result, valid) = operation(P::Subtract, signed, 0, bits)
                    .ok_or(Error::Invariant("wide negation operator"))?;
                if !valid && !skipping {
                    return Err(super::constant_addresses::nonconstant(
                        location,
                        "a negation that overflows",
                    ));
                }
                result
            }
            Some(P::Complement) => !bits,
            _ => bits,
        };
        Ok(Constant::from_wide(ty, result))
    }
}

pub(super) fn operation(op: P, signed: bool, a: u128, b: u128) -> Option<(u128, bool)> {
    let sa = a.cast_signed();
    let sb = b.cast_signed();
    let (bits, overflow) = match op {
        P::Add => {
            let (_, overflow) = sa.overflowing_add(sb);
            (a.wrapping_add(b), overflow)
        }
        P::Subtract => {
            let (_, overflow) = sa.overflowing_sub(sb);
            (a.wrapping_sub(b), overflow)
        }
        P::Multiply => {
            let (_, overflow) = sa.overflowing_mul(sb);
            (a.wrapping_mul(b), overflow)
        }
        P::Divide | P::Remainder => {
            if b == 0 || signed && sa == i128::MIN && sb == -1 {
                return Some((0, false));
            }
            return Some((
                if signed {
                    (if op == P::Divide { sa / sb } else { sa % sb }).cast_unsigned()
                } else if op == P::Divide {
                    a / b
                } else {
                    a % b
                },
                true,
            ));
        }
        P::BitAnd => return Some((a & b, true)),
        P::BitOr => return Some((a | b, true)),
        P::Xor => return Some((a ^ b, true)),
        _ => return None,
    };
    Some((bits, !signed || !overflow))
}
pub(super) fn shift_value(
    op: P,
    signed: bool,
    a: u128,
    count: u64,
    kind: IntegralKind,
) -> (u128, bool) {
    let count = kind.convert(count);
    if count >= 128 {
        return (0, false);
    }
    if op == P::ShiftRight {
        return (
            if signed {
                (a.cast_signed() >> count).cast_unsigned()
            } else {
                a >> count
            },
            true,
        );
    }
    (
        a << count,
        !signed || a.cast_signed() >= 0 && a <= u128::MAX >> count,
    )
}
fn compare(op: P, signed: bool, a: u128, b: u128) -> Option<bool> {
    let less = if signed {
        a.cast_signed() < b.cast_signed()
    } else {
        a < b
    };
    Some(match op {
        P::Equal => a == b,
        P::NotEqual => a != b,
        P::Less => less,
        P::Greater => !less && a != b,
        P::LessEqual => less || a == b,
        P::GreaterEqual => !less,
        _ => return None,
    })
}
