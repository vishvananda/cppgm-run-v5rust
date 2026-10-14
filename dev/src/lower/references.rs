// Translated from lower/expression_lowering.cpp::bind_converted and glvalue. See NOTICE.
//! References store addresses; scalar binding temporaries belong to one function.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::Type,
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn initializer_expression(
        &self,
        initializer: ast::Initializer,
    ) -> Result<ast::ExprId, Error> {
        let id = match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => id,
            ast::InitializerKind::Parenthesized(args) => *self
                .tree
                .expr_lists
                .get(args)
                .first()
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "empty reference initializer",
                )))?,
            _ => return Err(Error::Missing("special reference initializer")),
        };
        if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind
            && self.graph.lower_expression(id)?.is_place
        {
            return self
                .tree
                .expr_lists
                .get(elements)
                .first()
                .copied()
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "empty glvalue list",
                )));
        }
        Ok(id)
    }
    pub(super) fn address(&mut self, id: ast::ExprId) -> Result<Value, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let place = self.scalar_place(id)?;
        if matches!(place, Value::Symbol(_) | Value::Slot(_)) {
            Ok(self.emit(I::Addr(place), source)?.value)
        } else {
            Ok(place)
        }
    }
    pub(super) fn bind_reference(
        &mut self,
        id: ast::ExprId,
        reference: TypeId,
        context: Context,
    ) -> Result<Value, Error> {
        self.check_stack()?;
        let TypeView::Reference { element, .. } = self.graph.lower_type(reference)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "binding without reference type",
            )));
        };
        let facts = self.graph.lower_expression(id)?;
        if matches!(self.graph.lower_type(element)?, TypeView::Array { .. })
            && !facts.is_place
            && matches!(self.graph.lower_type(facts.ty)?, TypeView::Array { .. })
        {
            return self.array_temporary(
                id,
                if matches!(context, Context::Argument) {
                    "argarr"
                } else {
                    "arraytmp"
                },
            );
        }
        if facts.temporary.is_some()
            || (!facts.is_place
                && !matches!(self.graph.lower_type(facts.ty)?, TypeView::Function { .. }))
        {
            if matches!(
                self.graph.lower_type(element)?,
                TypeView::Array { .. } | TypeView::Class(_) | TypeView::Unsupported
            ) {
                return Err(Error::Missing("aggregate reference temporary lowering"));
            }
            self.conditional_slot = None;
            let value = self.expression(id, Context::Operand)?;
            let chosen = self.conditional_slot;
            let target = facts.temporary.unwrap_or(element);
            let value = if facts.temporary.is_some() {
                // bind_converted uses convert_implicit: this is not a literal
                // written in the outer binding's conversion context.
                let value = RValue {
                    literal: false,
                    ..value
                };
                if context != Context::Cast
                    && let Some(offset) = self.graph.lower_base_conversion_offset(id)?
                {
                    self.adjust_base_pointer(value, target, offset, offset != 0)?
                } else {
                    self.convert(value, target, context)?
                }
            } else {
                value
            };
            let value = self.fit(value, target)?;
            let storage = types::storage(self.graph, target)?;
            let slot = if let Some(slot) = chosen
                && matches!(
                    self.tree.expressions.get(self.without_parentheses(id)).kind,
                    ExprKind::Conditional { .. }
                )
                && types::storage(self.graph, facts.ty)? == storage
            {
                slot
            } else {
                let stem = match context {
                    Context::Argument => "refarg",
                    Context::Return => "retref",
                    _ => "tmpref",
                };
                self.generated_slot(stem, storage)?
            };
            self.effect(I::Store(
                Facts::memory(false, !matches!(storage, Type::I8 | Type::U8)),
                storage,
                value.value,
                slot,
            ))?;
            return Ok(self.emit(I::Addr(slot), target)?.value);
        }
        let address = self.address(id)?;
        if let Some(offset) = self.graph.lower_base_conversion_offset(id)? {
            self.base_projection(address, facts.ty, offset)
        } else {
            Ok(address)
        }
    }
    pub(super) fn reference_cast_place(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
    ) -> Result<Value, Error> {
        let facts = self.graph.lower_expression(id)?;
        if let Some(reference) = facts.reference_cast
            && self.graph.lower_expression(operand)?.temporary.is_some()
        {
            return self.bind_reference(operand, reference, Context::Cast);
        }
        if let Some((delta, project_zero)) = self.graph.lower_cast_base_adjustment(id)?
            && (delta != 0 || project_zero)
        {
            let address = self.address(operand)?;
            return self.base_projection(address, facts.ty, delta);
        }
        self.scalar_place(operand)
    }
    pub(super) fn reference_holder(&mut self, place: Value, ty: TypeId) -> Result<Value, Error> {
        if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
            Ok(self
                .emit(I::Load(self.place_memory(place, ty)?, Type::Ptr, place), ty)?
                .value)
        } else {
            Ok(place)
        }
    }
    pub(super) fn generated_slot(&mut self, stem: &'static str, ty: Type) -> Result<Value, Error> {
        Ok(self.builder.generated_slot(stem, ty)?)
    }
}
