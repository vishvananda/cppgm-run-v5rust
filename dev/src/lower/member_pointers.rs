// Translated from lower/member_pointer_lowering.cpp. See NOTICE.
//! Itanium data-member offsets, null sentinel and field projections.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Compare, Projection, Type},
    },
    sema::lowering::TypeId,
    syntax::ast,
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn converted_member_address(
        &mut self,
        id: ast::ExprId,
    ) -> Result<Option<RValue>, Error> {
        let Some(adjustment) = self.graph.lower_member_pointer_conversion(id)? else {
            return Ok(None);
        };
        let Some(offset) = self.graph.lower_data_member_address(id)? else {
            return Ok(None);
        };
        let target =
            self.graph
                .lower_expression(id)?
                .target
                .ok_or(crate::sema::Error::Invariant(
                    "member address conversion target",
                ))?;
        let offset = self
            .builder
            .integer(u128::from(offset.wrapping_add(adjustment)))?;
        self.emit(I::Const(Type::I64, offset), target).map(Some)
    }
    pub(super) fn convert_member_pointer(
        &mut self,
        value: RValue,
        target: TypeId,
        adjustment: u64,
    ) -> Result<RValue, Error> {
        if adjustment == 0 {
            return Ok(RValue {
                source: target,
                ..value
            });
        }
        if !self.data_member_pointer(value.source) {
            return Err(Error::Missing("member function pointer conversion"));
        }
        let null = self.builder.integer(u128::MAX)?;
        let present = self.emit(
            I::Compare(Compare::Ne, Type::I64, value.value, null),
            value.source,
        )?;
        let adjustment = self.builder.integer(u128::from_le_bytes(
            i128::from(i64::from_le_bytes(adjustment.to_le_bytes())).to_le_bytes(),
        ))?;
        let scaled = self.emit(
            I::Binary(
                Binary::Mul,
                Facts::default(),
                Type::I64,
                present.value,
                adjustment,
            ),
            value.source,
        )?;
        self.emit(
            I::Binary(
                Binary::Add,
                Facts::default(),
                Type::I64,
                value.value,
                scaled.value,
            ),
            target,
        )
    }
    pub(super) fn member_pointer_place(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        arrow: bool,
    ) -> Result<Value, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let mut object = if arrow {
            self.operand(left, Context::Operand)?.value
        } else {
            self.address(left)?
        };
        if let Some(offset) = self.graph.lower_member_pointer_object_offset(id)?
            && offset != 0
        {
            object = self.base_projection(object, source, offset)?;
        }
        let offset = self.operand(right, Context::Operand)?;
        let place = self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), object, offset.value),
                source,
            )?
            .value;
        if self
            .graph
            .lower_member_pointer_union(self.graph.lower_expression(left)?.ty, arrow)
        {
            self.untyped_places.insert(place);
        }
        Ok(place)
    }

    pub(super) fn data_member_pointer(&self, source: TypeId) -> bool {
        self.graph
            .lower_member_pointer_type(source)
            .is_some_and(|(_, element)| {
                !matches!(
                    self.graph.lower_type(element),
                    Ok(crate::sema::lowering::TypeView::Function { .. })
                )
            })
    }

    pub(super) fn member_pointer_truth(&mut self, value: RValue) -> Result<RValue, Error> {
        let one = self.builder.integer(1)?;
        self.emit(
            I::Binary(Binary::Add, Facts::default(), Type::I64, value.value, one),
            value.source,
        )
    }
}
