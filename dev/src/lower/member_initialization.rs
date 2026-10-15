// Translated from lower/object_lowering.cpp::initialize_member/initialize_scalar_member. See NOTICE.
//! Anonymous plans execute in their enclosing constructor's frame and storage.
use super::{Error, conversions::Context, function::Function};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{ConstructorId, EntityId, TypeId, TypeView},
    syntax::ast,
};
impl Function<'_, '_, '_, '_> {
    fn member_initialization_class(&self, pointer: TypeId) -> Result<EntityId, Error> {
        let TypeView::Pointer(object) = self.graph.lower_type(pointer)? else {
            return Err(crate::sema::Error::Invariant(
                "member initialization without object pointer",
            )
            .into());
        };
        let TypeView::Class(class) = self.graph.lower_type(object)? else {
            return Err(
                crate::sema::Error::Invariant("member initialization without class").into(),
            );
        };
        Ok(class)
    }
    fn member_initialization_origin(
        &self,
        member: EntityId,
        pointer: TypeId,
    ) -> Result<u64, Error> {
        Ok(self.graph.lower_anonymous_member_offset(
            member,
            Some(self.member_initialization_class(pointer)?),
        )?)
    }
    pub(super) fn member_initialization_offset(
        &self,
        member: EntityId,
        pointer: TypeId,
    ) -> Result<u64, Error> {
        Ok(self
            .graph
            .lower_member_offset(member)?
            .wrapping_add(self.member_initialization_origin(member, pointer)?))
    }
    pub(super) fn initialize_member_plan(
        &mut self,
        id: ConstructorId,
        pointer: TypeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let plan = self.graph.lower_constructor(id);
        let within = self.member_initialization_class(pointer)?;
        let union = self.graph.lower_union_class(within);
        let nested = plan.class != within;
        let mut wrote = false;
        let mut children = plan.subobjects.iter().peekable();
        for &(member, initializer) in plan.members {
            let ty = self.graph.lower_entity_type(member)?;
            if !matches!(
                self.graph.lower_type(ty)?,
                TypeView::Class(_) | TypeView::Array { .. }
            ) {
                if let Some(initializer) = initializer {
                    self.initialize_scalar_member(member, pointer, initializer, union)?;
                }
                wrote |= initializer.is_some();
                continue;
            }
            let local_offset = self.graph.lower_member_offset(member)?;
            let offset = self.member_initialization_offset(member, pointer)?;
            let child = if children.peek().is_some_and(|child| child.member == member) {
                children.next()
            } else {
                None
            };
            if let Some(child) = child {
                if self.graph.lower_anonymous_constructor(child.constructor) {
                    let high = self.bit_initialization.high;
                    self.bit_initialization.unit = 0;
                    self.bit_initialization.high = 0;
                    let initialized =
                        self.initialize_member_plan(child.constructor, pointer, location)?;
                    self.bit_initialization.unit = 0;
                    self.bit_initialization.high = high;
                    if initialized {
                        wrote = true;
                        let end = local_offset
                            .wrapping_add(super::types::layout(self.graph, ty)?.0)
                            .wrapping_mul(8);
                        self.bit_initialization.high = self.bit_initialization.high.max(end);
                    }
                    continue;
                }
                wrote |= initializer.is_some()
                    || !self.graph.lower_constructor_trivial(child.constructor);
                let first = self.lifetime.cleanups.len();
                let region = if let Some(initializer) = initializer {
                    self.begin_subobject_initializer(initializer, Some(child.constructor))?
                } else {
                    false
                };
                self.initialize_class_member(
                    pointer,
                    ty,
                    offset,
                    child.constructor,
                    child.arguments,
                )?;
                self.destroy_temporaries(first)?;
                self.end_lifetime_expression(region)?;
                if !plan.union && !nested {
                    self.subobject_constructed(ty, offset, true)?;
                }
                self.bit_initialization.unit = 0;
                self.bit_initialization.high = self.bit_initialization.high.max(
                    local_offset
                        .wrapping_add(super::types::layout(self.graph, ty)?.0)
                        .wrapping_mul(8),
                );
                continue;
            }
            let Some(initializer) = initializer else {
                if !plan.union && !nested {
                    self.subobject_constructed(ty, offset, true)?;
                }
                continue;
            };
            wrote = true;
            let first = self.lifetime.cleanups.len();
            let region = self.begin_subobject_initializer(initializer, None)?;
            self.constructor_aggregate_member(member, pointer, initializer, union, location)?;
            self.destroy_temporaries(first)?;
            self.end_lifetime_expression(region)?;
            if !plan.union && !nested {
                self.subobject_constructed(ty, offset, true)?;
            }
            self.bit_initialization.unit = 0;
            self.bit_initialization.high = self.bit_initialization.high.max(
                self.graph
                    .lower_member_offset(member)?
                    .wrapping_add(super::types::layout(self.graph, ty)?.0)
                    .wrapping_mul(8),
            );
        }
        Ok(wrote)
    }
    fn initialize_scalar_member(
        &mut self,
        member: EntityId,
        pointer: TypeId,
        initializer: ast::Initializer,
        union: bool,
    ) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(member)?;
        let single = match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                if let ast::ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                    if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
                        Some(self.initializer_expression(initializer)?)
                    } else {
                        self.tree.expr_lists.get(elements).first().copied()
                    }
                } else {
                    Some(id)
                }
            }
            ast::InitializerKind::Parenthesized(arguments) => {
                self.tree.expr_lists.get(arguments).first().copied()
            }
            _ => {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "constructor initializer action",
                )));
            }
        };
        if self.graph.lower_bit_field(member)?.is_some() {
            let field = self.field(member)?;
            let value = if let Some(single) = single {
                let value = self.operand(single, Context::MemberInitialization)?;
                self.fit(value, ty)?
            } else {
                self.zero(ty)?
            };
            self.initialize_field_at(
                field,
                value,
                self.graph.lower_cv(ty) & 2 != 0,
                self.bit_initialization_order
                    == super::bit_fields::InitializationOrder::AssignmentDemanded,
                |function| function.member_field_address(member, pointer, field.offset),
            )?;
            let layout = self
                .graph
                .lower_bit_field(member)?
                .ok_or(Error::Missing("constructor field layout"))?;
            self.bit_initialization.high = self
                .bit_initialization
                .high
                .max(layout.bit.wrapping_add(layout.width));
            return Ok(());
        }
        self.bit_initialization.unit = 0;
        let value = if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
            self.bind_reference(
                single.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "reference member without source",
                )))?,
                ty,
                Context::Initialization,
            )?
        } else if let Some(single) = single {
            let value = self.operand(single, Context::Initialization)?;
            self.fit(value, ty)?.value
        } else {
            self.zero(ty)?.value
        };
        let object = self.object_pointer(pointer)?.value;
        let offset = self.builder.integer(u128::from(
            self.member_initialization_offset(member, pointer)?,
        ))?;
        let address = self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), object, offset),
                ty,
            )?
            .value;
        self.effect(I::Store(
            Facts::memory(
                self.graph.lower_cv(ty) & 2 != 0,
                self.graph.lower_cv(ty) & 2 == 0
                    && !union
                    && !matches!(super::types::storage(self.graph, ty)?, Type::I8 | Type::U8),
            ),
            super::types::storage(self.graph, ty)?,
            value,
            address,
        ))?;
        self.bit_initialization.high = self.bit_initialization.high.max(
            self.graph
                .lower_member_offset(member)?
                .wrapping_add(super::types::layout(self.graph, ty)?.0)
                .wrapping_mul(8),
        );
        Ok(())
    }
    fn member_field_address(
        &mut self,
        member: EntityId,
        pointer: TypeId,
        offset: u64,
    ) -> Result<crate::lowir::model::Value, Error> {
        let ty = self.graph.lower_entity_type(member)?;
        let object = self.object_pointer(pointer)?.value;
        let origin = self.member_initialization_origin(member, pointer)?;
        let object = if origin == 0 {
            object
        } else {
            let origin = self.builder.integer(u128::from(origin))?;
            self.emit(
                I::Index(Type::I8, Some(Projection::Field), object, origin),
                ty,
            )?
            .value
        };
        let offset = self.builder.integer(u128::from(offset))?;
        Ok(self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), object, offset),
                ty,
            )?
            .value)
    }
}
