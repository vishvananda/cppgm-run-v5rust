// Translated from lower/conversion_lowering.cpp. See NOTICE.
//! Context controls the oracle's literal retyping; all target truncation is explicit.
use super::{
    Error,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::InstructionKind as I,
        vocabulary::{Compare, Convert, Type},
    },
    sema::lowering::TypeId,
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Context {
    Operand,
    Initialization,
    Argument,
    Return,
    Additive,
    Comparison,
    BesideSize,
    Unary,
    Assignment,
    BitFieldAssignment,
    CompoundAssignment,
    Cast,
    ArrayElement,
    MemberInitialization,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn fit(&mut self, value: RValue, target: TypeId) -> Result<RValue, Error> {
        if types::boolean(self.graph, target)? && value.ty == Type::I64 {
            return self.emit(
                I::Convert(Convert::Truncate, Type::U8, Type::I64, value.value),
                target,
            );
        }
        Ok(value)
    }
    pub(super) fn convert(
        &mut self,
        value: RValue,
        target: TypeId,
        context: Context,
    ) -> Result<RValue, Error> {
        if value.source == target
            || matches!((self.graph.lower_type(value.source)?, self.graph.lower_type(target)?),
                (crate::sema::lowering::TypeView::Complex(a), crate::sema::lowering::TypeView::Complex(b)) if a == b)
            || matches!(
                (self.graph.lower_type(value.source)?, self.graph.lower_type(target)?),
                (crate::sema::lowering::TypeView::Scalar(from), crate::sema::lowering::TypeView::Scalar(to))
                    if from == to
            )
        {
            // Scalar conversion compares unqualified identities. In particular,
            // binding a const nullptr_t temporary must not retype its ptr value.
            return Ok(RValue {
                source: target,
                ..value
            });
        }
        if matches!((self.graph.lower_type(value.source)?, self.graph.lower_type(target)?),
            (crate::sema::lowering::TypeView::Vector { element: a, bound: n }, crate::sema::lowering::TypeView::Vector { element: b, bound: m }) if a == b && n == m)
        {
            return Ok(RValue {
                source: target,
                ..value
            });
        }
        let to = types::storage(self.graph, target)?;
        if to == Type::Void {
            return Ok(value);
        }
        if types::boolean(self.graph, target)? {
            if super::complex::part(self.graph, value.source)?.is_some() {
                return self.complex_truth(value, target);
            }
            if self.graph.lower_member_pointer_type(value.source).is_some() {
                let truth = self.member_pointer_truth(value)?;
                let zero = self.builder.integer(0)?;
                return self.emit(I::Compare(Compare::Ne, truth.ty, truth.value, zero), target);
            }
            if types::boolean(self.graph, value.source)? {
                return Ok(value);
            }
            if let Some(kind) = super::soft_float::kind(self.graph, value.source)? {
                let zero = self.soft_constant(kind, 0, value.source)?;
                return self.soft_compare(Compare::Ne, kind, value, zero, target);
            }
            let zero = self.zero(value.source)?;
            return self.emit(
                I::Compare(Compare::Ne, value.ty, value.value, zero.value),
                target,
            );
        }
        if super::complex::part(self.graph, target)?.is_some() {
            return self.complex_convert(value, target, context);
        }
        if super::complex::part(self.graph, value.source)?.is_some() {
            let [real, _] = self.complex_parts(value)?;
            return self.convert(real, target, context);
        }
        if super::soft_float::kind(self.graph, value.source)?.is_some()
            || super::soft_float::kind(self.graph, target)?.is_some()
        {
            return self.soft_convert(value, target);
        }
        let from_signed = types::signed(self.graph, value.source)?;
        let to_signed = types::signed(self.graph, target)?;
        let operation = if to.float_rank() != 0 {
            if value.ty.float_rank() != 0 {
                if to == value.ty {
                    return Ok(RValue {
                        source: target,
                        ..value
                    });
                }
                if to.float_rank() > value.ty.float_rank() {
                    Convert::FloatExtend
                } else {
                    Convert::FloatTruncate
                }
            } else if from_signed {
                Convert::SignedToFloat
            } else {
                Convert::UnsignedToFloat
            }
        } else if value.ty.float_rank() != 0 {
            if to_signed {
                Convert::FloatToSigned
            } else {
                Convert::FloatToUnsigned
            }
        } else if to == Type::Ptr {
            if value.ty == Type::Ptr
                && !matches!(
                    self.graph.lower_type(value.source)?,
                    crate::sema::lowering::TypeView::Scalar(crate::sema::lowering::Scalar::Nullptr)
                )
            {
                return Ok(RValue {
                    source: target,
                    ..value
                });
            }
            return self.emit(I::Copy(to, value.value), target);
        } else if value.ty == Type::Ptr {
            if to.bits() == 64 {
                return self.emit(I::Copy(to, value.value), target);
            }
            Convert::Truncate
        } else {
            return self.convert_integer(value, target, to, to_signed, context);
        };
        self.emit(I::Convert(operation, to, value.ty, value.value), target)
    }
    pub(super) fn convert_integer(
        &mut self,
        value: RValue,
        target: TypeId,
        to: Type,
        to_signed: bool,
        context: Context,
    ) -> Result<RValue, Error> {
        let from_signed = types::signed(self.graph, value.source)?;
        let operation = {
            let sign_change =
                !types::boolean(self.graph, value.source)? && from_signed != to_signed;
            if to == value.ty && !sign_change {
                return Ok(RValue {
                    source: target,
                    ..value
                });
            }
            if to == Type::I128 && value.ty == Type::I128 && value.bits.is_some() {
                return Ok(RValue {
                    source: target,
                    ..value
                });
            }
            let from_bits = value.ty.bits();
            let to_bits = to.bits();
            let bool_source = types::boolean(self.graph, value.source)?;
            let widening = value.literal
                && to_bits > from_bits
                && if from_signed && to_signed {
                    !matches!(
                        context,
                        Context::Operand
                            | Context::Assignment
                            | Context::CompoundAssignment
                            | Context::ArrayElement
                    )
                } else {
                    match context {
                        Context::BesideSize => true,
                        Context::Additive | Context::Unary => {
                            bool_source || !from_signed && !to_signed
                        }
                        Context::Comparison => bool_source,
                        Context::Return => !from_signed || to_signed,
                        _ => false,
                    }
                };
            if let Some(bits) = value.bits
                && ((to_bits <= from_bits
                    && (!matches!(context, Context::ArrayElement | Context::BitFieldAssignment)
                        || to_bits == from_bits)
                    && (context != Context::MemberInitialization
                        || to_bits == from_bits
                        || from_signed && to_signed))
                    || widening)
            {
                let width = to_bits.min(from_bits);
                let mask = u64::MAX >> (64 - width.min(64));
                let mut folded = bits & mask;
                let extend_sign = if to_bits <= from_bits {
                    to_signed
                } else {
                    from_signed
                };
                if extend_sign && width < 64 && folded & (1 << (width - 1)) != 0 {
                    folded |= !mask;
                }
                let literal_bits = if to_bits != 128 || extend_sign {
                    i128::from(folded.cast_signed()).cast_unsigned()
                } else {
                    u128::from(folded)
                };
                let literal = self.builder.integer(literal_bits)?;
                return Ok(RValue {
                    value: literal,
                    ty: to,
                    source: target,
                    literal: value.literal,
                    bits: Some(folded),
                });
            }
            if to_bits == from_bits {
                return self.emit(I::Copy(to, value.value), target);
            }
            if to_bits < from_bits {
                Convert::Truncate
            } else if from_signed {
                Convert::SignExtend
            } else {
                Convert::ZeroExtend
            }
        };
        self.emit(I::Convert(operation, to, value.ty, value.value), target)
    }
}
