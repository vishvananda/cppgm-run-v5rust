// Translated from lower/expression_lowering.cpp::variable_place. See NOTICE.
//! Alias places consume the analyzed object/member/path, with no synthesized expressions.
use super::{Error, function::Function, types};
use crate::{
    lowir::{
        model::{InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{BindingAlias, BindingTarget, TypeId, TypeView},
    support::source::SourceOffset,
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn binding_object(
        &mut self,
        alias: BindingAlias,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        let place = self.entity_place(alias.object, alias.object_type, location)?;
        self.storage_address(place, alias.object_type)
    }
    pub(super) fn binding_alias_place(
        &mut self,
        alias: BindingAlias,
        source: TypeId,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        let object = self.binding_object(alias, location)?;
        match alias.target {
            BindingTarget::Member { member, path } => {
                let object = if let Some(path) = path {
                    self.base_projection(object, source, self.graph.lower_path_projection(path)?)?
                } else {
                    object
                };
                let offset = self
                    .builder
                    .integer(u128::from(self.graph.lower_member_offset(member)?))?;
                let place = self
                    .emit(
                        I::Index(Type::I8, Some(Projection::Field), object, offset),
                        source,
                    )?
                    .value;
                self.reference_holder(place, self.graph.lower_entity_type(member)?)
            }
            BindingTarget::Element(index) => {
                let (ty, offset) = if matches!(
                    self.graph.lower_type(source)?,
                    TypeView::Class(_) | TypeView::Array { .. }
                ) {
                    (
                        Type::I8,
                        index.wrapping_mul(types::layout(self.graph, source)?.0),
                    )
                } else {
                    (types::storage(self.graph, source)?, index)
                };
                let offset = self.builder.integer(u128::from(offset))?;
                Ok(self
                    .emit(
                        I::Index(ty, Some(Projection::ArrayElement), object, offset),
                        source,
                    )?
                    .value)
            }
            BindingTarget::Dependent => {
                Err(crate::sema::Error::Invariant("lowering dependent binding").into())
            }
        }
    }
}
