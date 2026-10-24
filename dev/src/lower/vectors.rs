// Translated from lower/vector_lowering.cpp. See NOTICE.
//! Vector lists use addressed object storage; lanes retain semantic conversions.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{InstructionKind as I, Value},
        vocabulary::{Binary, Convert, EightbyteClass as E, ObjectClasses, Projection, Type},
    },
    sema::{
        Graph,
        lowering::{Scalar, TypeId, TypeView},
    },
    syntax::{ast, storage::List},
};

pub(super) fn classes(
    graph: &Graph,
    ty: TypeId,
    element: TypeId,
    bound: u64,
) -> Result<ObjectClasses, Error> {
    let bytes = types::layout(graph, ty)?.0;
    let floating = matches!(graph.lower_type(element)?, TypeView::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0);
    Ok(if bound == 1 && floating {
        ObjectClasses::Memory
    } else if bytes <= 4 {
        ObjectClasses::Direct(if floating { E::Sse } else { E::Integer }, None)
    } else if bytes == 8 {
        ObjectClasses::Direct(E::Sse, None)
    } else if bytes == 16 {
        ObjectClasses::Direct(E::Sse, Some(E::SseUp))
    } else {
        ObjectClasses::Memory
    })
}
impl Function<'_, '_, '_, '_> {
    fn zeroed_vector(&mut self, ty: TypeId) -> Result<(Value, Value), Error> {
        let TypeView::Vector { bound, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("vector storage type").into());
        };
        if bound > 64 {
            return Err(crate::sema::Error::at(
                crate::support::source::SourceOffset(0),
                "a vector operation on more than 64 lanes",
            )
            .into());
        }
        let storage = types::storage(self.graph, ty)?;
        let (bytes, align) = types::layout(self.graph, ty)?;
        let slot = self.generated_slot("vector", storage)?;
        self.effect(I::ZeroInit(bytes, align, slot))?;
        let address = self.emit(I::Addr(slot), ty)?.value;
        Ok((slot, address))
    }
    pub(in crate::lower) fn vector_list(
        &mut self,
        ty: TypeId,
        elements: List<ast::ExprId>,
    ) -> Result<RValue, Error> {
        let TypeView::Vector { element: lane, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("vector list type").into());
        };
        let (slot, address) = self.zeroed_vector(ty)?;
        let boolean = matches!(self.graph.lower_type(lane)?, TypeView::Scalar(Scalar::Bool));
        let lane_bytes = types::layout(self.graph, lane)?.0;
        for (index, &expression) in self.tree.expr_lists.get(elements).iter().enumerate() {
            let index = u64::try_from(index).map_err(|_| crate::lowir::Error::Limit)?;
            let value = self.operand(expression, Context::Initialization)?;
            let value = self.fit(value, lane)?;
            let offset = self.builder.integer(u128::from(if boolean {
                index / 8
            } else {
                index * lane_bytes
            }))?;
            let place = self
                .emit(
                    I::Index(Type::I8, Some(Projection::Field), address, offset),
                    lane,
                )?
                .value;
            let value = if boolean {
                let bit = if value.ty == Type::U8 {
                    value.value
                } else {
                    self.emit(
                        I::Convert(Convert::Truncate, Type::U8, value.ty, value.value),
                        lane,
                    )?
                    .value
                };
                let bit = if index % 8 == 0 {
                    bit
                } else {
                    let shift = self.builder.integer(u128::from(index % 8))?;
                    self.emit(
                        I::Binary(
                            Binary::Shl,
                            crate::lowir::model::Facts::default(),
                            Type::U8,
                            bit,
                            shift,
                        ),
                        lane,
                    )?
                    .value
                };
                let old = self.read_place(place, lane)?;
                self.emit(
                    I::Binary(
                        Binary::Or,
                        crate::lowir::model::Facts::default(),
                        Type::U8,
                        old.value,
                        bit,
                    ),
                    lane,
                )?
                .value
            } else {
                value.value
            };
            self.write_place(place, lane, value)?;
        }
        self.read_place(slot, ty)
    }
}
