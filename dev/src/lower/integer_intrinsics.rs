// Translated from lower/intrinsic_lowering.cpp::{gnu_intrinsic,bit_count}. See NOTICE.
//! Preserve the frozen straight-line bit-count expansion and instruction order.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I},
        vocabulary::{Binary as B, Compare, Convert, Type, Unary},
    },
    sema::lowering::{BitOperation as Bit, TypeId},
    syntax::{ast, storage::List},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn integer_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        kind: Bit,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let arguments = self.owned_expression_ids(arguments);
        let arguments = arguments.as_slice();
        let mut x = self.operand(arguments[0], Context::Argument)?;
        if kind == Bit::ByteSwap {
            return self.emit(I::Unary(Unary::ByteSwap, x.ty, x.value), source);
        }
        let given = x;
        let bits = x.ty.bits();
        if bits < 32 {
            x = self.emit(
                I::Convert(Convert::ZeroExtend, Type::U32, x.ty, x.value),
                x.source,
            )?;
        }
        let mut count = if bits == 128 {
            self.wide_bit_count(kind, x, source)?
        } else {
            self.bit_count(kind, x, source)?
        };
        if kind == Bit::LeadingZeros && bits < 32 {
            let extra = self.bit_integer(Type::I32, u128::from(32 - bits), source)?;
            count = self.emit(
                I::Binary(
                    B::Sub,
                    Facts::default(),
                    Type::I32,
                    count.value,
                    extra.value,
                ),
                source,
            )?;
        }
        if arguments.len() == 2 {
            let zero = self.bit_integer(given.ty, 0, given.source)?;
            let nonzero = self.emit(
                I::Compare(Compare::Ne, given.ty, given.value, zero.value),
                source,
            )?;
            let zero = self.operand(arguments[1], Context::Argument)?;
            count = self.emit(
                I::Select(Type::I32, nonzero.value, count.value, zero.value),
                source,
            )?;
        }
        Ok(count)
    }

    fn wide_bit_count(&mut self, kind: Bit, x: RValue, source: TypeId) -> Result<RValue, Error> {
        let low = self.emit(
            I::Convert(Convert::Truncate, Type::I64, x.ty, x.value),
            x.source,
        )?;
        let shift = self.bit_integer(x.ty, 64, x.source)?;
        let high = self.bit_binary(B::UShr, x, shift, x.source)?;
        let high = self.emit(
            I::Convert(Convert::Truncate, Type::I64, high.ty, high.value),
            x.source,
        )?;
        let low_count = self.bit_count(kind, low, source)?;
        let high_count = self.bit_count(kind, high, source)?;
        if kind == Bit::Population {
            return self.bit_binary(B::Add, low_count, high_count, source);
        }
        let leading = kind == Bit::LeadingZeros;
        let first = if leading { high_count } else { low_count };
        let sixty_four = self.bit_integer(Type::I32, 64, source)?;
        let second = self.bit_binary(
            B::Add,
            sixty_four,
            if leading { low_count } else { high_count },
            source,
        )?;
        let zero = self.bit_integer(Type::I64, 0, x.source)?;
        let half = if leading { high } else { low };
        let empty = self.emit(
            I::Compare(Compare::Eq, Type::I64, half.value, zero.value),
            source,
        )?;
        let empty = self.emit(
            I::Convert(Convert::Truncate, Type::I32, empty.ty, empty.value),
            source,
        )?;
        let delta = self.bit_binary(B::Sub, second, first, source)?;
        let selected = self.bit_binary(B::Mul, empty, delta, source)?;
        self.bit_binary(B::Add, first, selected, source)
    }

    fn bit_count(&mut self, kind: Bit, mut x: RValue, source: TypeId) -> Result<RValue, Error> {
        let ty = x.ty;
        let width = if ty == Type::I64 { 64 } else { 32 };
        let count = if kind == Bit::Population {
            let one = self.bit_integer(ty, 1, x.source)?;
            let shifted = self.bit_binary(B::UShr, x, one, x.source)?;
            let mask = self.bit_integer(ty, 0x5555_5555_5555_5555, x.source)?;
            let pairs = self.bit_binary(B::And, shifted, mask, x.source)?;
            x = self.bit_binary(B::Sub, x, pairs, x.source)?;
            let two = self.bit_integer(ty, 2, x.source)?;
            let shifted = self.bit_binary(B::UShr, x, two, x.source)?;
            let mask = self.bit_integer(ty, 0x3333_3333_3333_3333, x.source)?;
            let high_pairs = self.bit_binary(B::And, shifted, mask, x.source)?;
            let low_pairs = self.bit_binary(B::And, x, mask, x.source)?;
            x = self.bit_binary(B::Add, low_pairs, high_pairs, x.source)?;
            let four = self.bit_integer(ty, 4, x.source)?;
            let shifted = self.bit_binary(B::UShr, x, four, x.source)?;
            let sum = self.bit_binary(B::Add, x, shifted, x.source)?;
            let mask = self.bit_integer(ty, 0x0f0f_0f0f_0f0f_0f0f, x.source)?;
            x = self.bit_binary(B::And, sum, mask, x.source)?;
            let gather = self.bit_integer(ty, 0x0101_0101_0101_0101, x.source)?;
            let product = self.bit_binary(B::Mul, x, gather, x.source)?;
            let shift = self.bit_integer(ty, width - 8, x.source)?;
            self.bit_binary(B::UShr, product, shift, x.source)?
        } else {
            self.emit(
                I::Unary(
                    if kind == Bit::LeadingZeros {
                        Unary::Clz
                    } else {
                        Unary::Ctz
                    },
                    ty,
                    x.value,
                ),
                source,
            )?
        };
        if width != 32 {
            self.emit(
                I::Convert(Convert::Truncate, Type::I32, count.ty, count.value),
                source,
            )
        } else if count.ty == Type::I32
            || self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .object_mode()
        {
            Ok(RValue { source, ..count })
        } else {
            self.emit(I::Copy(Type::I32, count.value), source)
        }
    }

    fn bit_integer(&mut self, ty: Type, bits: u128, source: TypeId) -> Result<RValue, Error> {
        Ok(RValue {
            value: self.builder.integer(bits)?,
            ty,
            source,
            literal: false,
            bits: None,
        })
    }
    fn bit_binary(&mut self, op: B, a: RValue, b: RValue, source: TypeId) -> Result<RValue, Error> {
        self.emit(
            I::Binary(op, Facts::default(), a.ty, a.value, b.value),
            source,
        )
    }
}
