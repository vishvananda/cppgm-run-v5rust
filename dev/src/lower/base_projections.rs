// Translated from lower/expression_lowering.cpp::to_base/adjusted_pointer. See NOTICE.
//! Mechanical projections consume semantic paths and ready layout offsets.
use super::{
    Error,
    function::{Function, RValue},
};
use crate::{
    lex::{pp_lexer::Punctuator, vocabulary::Keyword},
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Compare, Type},
    },
    sema::lowering::{BaseProjection, TypeId},
    syntax::ast::{self, ExprKind},
};
#[derive(Clone, Copy)]
pub(super) enum BaseAccess {
    Table,
    Complete,
    Nullable,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn resolve_base_projection(
        &mut self,
        projection: BaseProjection,
    ) -> Result<BaseProjection, Error> {
        let BaseProjection::Nonvirtual {
            from, to, reverse, ..
        } = projection
        else {
            return Ok(projection);
        };
        if reverse {
            let offset = self.graph.lower_nonvirtual_offset(from, to)?.ok_or(
                crate::sema::Error::Invariant("downcast without a nonvirtual base offset"),
            )?;
            return Ok(BaseProjection::Fixed(offset.wrapping_neg()));
        }
        let route = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .base_route(from, to)?;
        if route.via.is_some() {
            return Err(
                crate::sema::Error::Invariant("nonvirtual path selected a virtual route").into(),
            );
        }
        Ok(BaseProjection::Fixed(route.offset))
    }
    pub(super) fn nonnull_pointer(&self, id: ast::ExprId) -> Result<bool, Error> {
        let id = self.without_parentheses(id);
        Ok(match self.tree.expressions.get(id).kind {
            ExprKind::Keyword(Keyword::This) => true,
            ExprKind::Unary { op, .. } => {
                op.token.as_punctuator() == Some(Punctuator::BitAnd)
                    && !self.graph.lower_operator_call(id)?
            }
            _ => false,
        })
    }
    pub(super) fn member_base_access(
        &self,
        id: ast::ExprId,
        arrow: bool,
    ) -> Result<BaseAccess, Error> {
        let id = self.without_parentheses(id);
        let facts = self.graph.lower_expression(id)?;
        if arrow {
            return Ok(
                if (self.graph.lower_arrow_count(id) != 0
                    || matches!(
                        self.graph.lower_type(facts.ty)?,
                        crate::sema::lowering::TypeView::Pointer(_)
                    ))
                    && !self.nonnull_pointer(id)?
                {
                    BaseAccess::Nullable
                } else {
                    BaseAccess::Table
                },
            );
        }
        if !matches!(
            self.graph.lower_type(facts.ty)?,
            crate::sema::lowering::TypeView::Class(_)
        ) {
            return Ok(BaseAccess::Table);
        }
        let complete = if let ExprKind::Id(name) = self.tree.expressions.get(id).kind {
            self.graph.lower_object_source(id)?.is_none()
                && self.graph.lower_named_object(name)?
                && matches!(
                    self.graph
                        .lower_type(self.graph.lower_entity_type(self.graph.lower_name(name)?)?)?,
                    crate::sema::lowering::TypeView::Class(_)
                )
        } else {
            !facts.is_place
        };
        Ok(if complete {
            BaseAccess::Complete
        } else {
            BaseAccess::Table
        })
    }
    pub(super) fn base_projection(
        &mut self,
        object: Value,
        source: TypeId,
        offset: impl Into<BaseProjection>,
    ) -> Result<Value, Error> {
        self.project_base_access(object, source, offset.into(), BaseAccess::Table)
    }
    pub(super) fn project_base_access(
        &mut self,
        object: Value,
        source: TypeId,
        projection: BaseProjection,
        access: BaseAccess,
    ) -> Result<Value, Error> {
        let projection = self.resolve_base_projection(projection)?;
        let offset = match projection {
            BaseProjection::Fixed(offset) => offset,
            BaseProjection::Nonvirtual { .. } => {
                return Err(
                    crate::sema::Error::Invariant("unresolved nonvirtual projection").into(),
                );
            }
            BaseProjection::Route { from, to } => {
                let route = self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .base_route(from, to)?;
                let Some(base) = route.via else {
                    return self.base_projection(object, source, route.offset);
                };
                if matches!(access, BaseAccess::Complete) {
                    let offset = self
                        .graph
                        .lower_virtual_bases(from)
                        .iter()
                        .find(|&&(class, _)| class == base)
                        .ok_or(crate::sema::Error::Invariant(
                            "complete object without virtual base",
                        ))?
                        .1;
                    return self.base_projection(object, source, offset.wrapping_add(route.offset));
                }
                if matches!(access, BaseAccess::Nullable) {
                    let value = RValue {
                        value: object,
                        ty: Type::Ptr,
                        source,
                        literal: false,
                        bits: None,
                    };
                    return Ok(self
                        .adjust_base_pointer(value, source, projection, true)?
                        .value);
                }
                let object = self.virtual_base_address(object, source, from, base)?;
                return self.base_projection(object, source, route.offset);
            }
        };
        let offset = self.builder.integer(u128::from_le_bytes(
            i128::from(i64::from_le_bytes(offset.to_le_bytes())).to_le_bytes(),
        ))?;
        Ok(self
            .emit(I::Index(Type::I8, None, object, offset), source)?
            .value)
    }
    pub(super) fn member_base_projection(
        &mut self,
        id: ast::ExprId,
        mut object: Value,
        source: TypeId,
    ) -> Result<Value, Error> {
        let mut access = match self.tree.expressions.get(self.without_parentheses(id)).kind {
            ExprKind::Member { base, op, .. } => {
                self.member_base_access(base, op.token.as_punctuator() == Some(Punctuator::Arrow))?
            }
            _ => BaseAccess::Table,
        };
        for projection in self
            .graph
            .lower_member_projections(id)?
            .into_iter()
            .flatten()
        {
            object = self.project_base_access(object, source, projection, access)?;
            access = BaseAccess::Table;
        }
        Ok(object)
    }
    pub(super) fn convert_base_pointer(
        &mut self,
        value: RValue,
        target: TypeId,
        delta: BaseProjection,
        null_test: bool,
        reserve: bool,
    ) -> Result<RValue, Error> {
        let mut delta = self.resolve_base_projection(delta)?;
        if let BaseProjection::Route { from, to } = delta {
            let route = self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .base_route(from, to)?;
            if route.via.is_none() {
                delta = BaseProjection::Fixed(route.offset);
            }
        }
        let null_test = null_test && !delta.is_zero();
        if null_test && reserve && matches!(delta, BaseProjection::Route { .. }) {
            // pointer_to_base reserves its ordinary conversion slot before
            // the virtual route allocates its own null-tested result.
            self.generated_slot("basecast", Type::Ptr)?;
        }
        self.adjust_base_pointer(value, target, delta, null_test)
    }
    pub(super) fn adjust_base_pointer(
        &mut self,
        value: RValue,
        target: TypeId,
        delta: BaseProjection,
        null_test: bool,
    ) -> Result<RValue, Error> {
        if !null_test {
            return Ok(RValue {
                value: self.base_projection(value.value, target, delta)?,
                source: target,
                bits: None,
                literal: false,
                ..value
            });
        }
        let slot = self.generated_slot("basecast", Type::Ptr)?;
        let null_block = self.new_block("basecast_null")?;
        let adjust = self.new_block("basecast_adjust")?;
        let end = self.new_block("basecast_end")?;
        let zero = self.builder.integer(0)?;
        let condition = self.emit(
            I::Compare(Compare::Eq, Type::Ptr, value.value, zero),
            target,
        )?;
        self.effect(I::Branch(condition.value, null_block, adjust))?;
        self.terminated = true;
        self.place(null_block)?;
        self.effect(I::Store(Facts::memory(false, true), Type::Ptr, zero, slot))?;
        self.jump(end)?;
        self.place(adjust)?;
        let adjusted = self.base_projection(value.value, target, delta)?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            adjusted,
            slot,
        ))?;
        self.jump(end)?;
        self.place(end)?;
        self.emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), target)
    }
}
