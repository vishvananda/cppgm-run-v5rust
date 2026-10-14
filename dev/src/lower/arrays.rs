// Translated from lower/statement_lowering.cpp's local array initialization. See NOTICE.
//! Array slots remain distinct; only eligible constant images are shared.
use super::{
    Error,
    conversions::Context,
    data,
    function::{Function, RValue},
    types,
};
use crate::{
    lex::PostTokenKind,
    lowir::{
        model::{InstructionKind as I, Value},
        vocabulary::Type,
    },
    sema::lowering::{EntityId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn aggregate_array_argument(
        &mut self,
        ty: TypeId,
        list: &[ast::ExprId],
        next: &mut usize,
    ) -> Result<Value, Error> {
        let slot = self.generated_slot("argarr", types::storage(self.graph, ty)?)?;
        let address = self.emit(I::Addr(slot), ty)?.value;
        self.structured_array_item(ty, address, list, next)?;
        Ok(address)
    }
    pub(super) fn array_temporary(
        &mut self,
        id: ast::ExprId,
        stem: &'static str,
    ) -> Result<Value, Error> {
        let ty = self.graph.lower_expression(id)?.ty;
        let root = self.without_parentheses(id);
        let list = match self.tree.expressions.get(root).kind {
            ExprKind::Braced { elements, .. } => elements,
            ExprKind::FunctionalCast {
                braced: Some(expr), ..
            } => {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expr).kind else {
                    return Err(Error::Semantic(crate::sema::Error::Invariant(
                        "array temporary without list",
                    )));
                };
                elements
            }
            _ => return Err(Error::Missing("non-list array temporary lowering")),
        };
        let slot = self.generated_slot(stem, types::storage(self.graph, ty)?)?;
        let address = self.emit(I::Addr(slot), ty)?.value;
        self.structured_array(ty, address, self.tree.expr_lists.get(list), &mut 0)?;
        Ok(address)
    }
    pub(super) fn literal_array_unit(
        &self,
        literal: ast::Literal,
        index: u64,
        width: usize,
    ) -> Result<u64, Error> {
        let offset = usize::try_from(index)
            .map_err(|_| crate::lowir::Error::Limit)?
            .checked_mul(width)
            .ok_or(crate::lowir::Error::Limit)?;
        let end = offset
            .checked_add(width)
            .ok_or(crate::lowir::Error::Limit)?;
        let mut value = [0u8; 8];
        if let Some(unit) = self.symbols.literal_bytes(literal).get(offset..end) {
            value[..unit.len()].copy_from_slice(unit);
        }
        Ok(u64::from_le_bytes(value))
    }
    pub(super) fn local_array(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        let Some(initializer) = initializer else {
            let mut object = ty;
            while let TypeView::Array { element, .. } = self.graph.lower_type(object)? {
                object = element;
            }
            if matches!(self.graph.lower_type(object)?, TypeView::Class(_)) {
                self.symbols.require_trivial_storage(ty)?;
                if !self.graph.lower_trivial_default_construction(entity)? {
                    return Err(Error::Missing("class array default construction lowering"));
                }
            }
            return Ok(());
        };
        let TypeView::Array { element, bound } = self.graph.lower_type(ty)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "local array without type",
            )));
        };
        if (self.graph.lower_constexpr(entity)
            || !matches!(
                self.graph.lower_type(element)?,
                TypeView::Array { .. } | TypeView::Class(_)
            ))
            && self.graph.lower_cv(element) & 2 == 0
            && let Some(image) = self.graph.lower_array_image(entity)
        {
            let image = data::image(
                self.builder.program(),
                self.symbols,
                self.graph,
                self.tree,
                image,
            )?;
            let (bytes, align) = types::layout(self.graph, ty)?;
            let destination = self.emit(I::Addr(slot), ty)?;
            return self.effect(I::CopyObject(
                bytes,
                align,
                Value::Symbol(image),
                destination.value,
            ));
        }
        let (ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr)) =
            initializer.kind
        else {
            return Err(Error::Missing("array parenthesized initialization"));
        };
        let base = if matches!(slot, Value::Slot(_) | Value::Symbol(_)) {
            self.emit(I::Addr(slot), ty)?.value
        } else {
            slot
        };
        if let TypeView::Class(class) = self.graph.lower_type(element)? {
            self.symbols.require_trivial_storage(ty)?;
            let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expr).kind else {
                return Err(Error::Missing("class array non-list initialization"));
            };
            let list = self.tree.expr_lists.get(elements);
            if self.array_helper_eligible(class, bound, list, entity)? {
                return self.aggregate_array_helpers(class, bound, base, list);
            }
            return self.aggregate_array(ty, base, list);
        }
        if matches!(self.graph.lower_type(element)?, TypeView::Array { .. }) {
            let mut next = 0;
            return self.structured_array_item(ty, base, &[expr], &mut next);
        }
        let mut root = expr;
        while let ExprKind::Paren(inner) = self.tree.expressions.get(root).kind {
            root = inner;
        }
        let list = if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(root).kind {
            self.tree.expr_lists.get(elements)
        } else {
            &[]
        };
        let literal = if list.len() == 1
            && matches!(
                self.graph
                    .lower_type(self.graph.lower_expression(list[0])?.ty)?,
                TypeView::Array { .. }
            ) {
            list[0]
        } else {
            root
        };
        let literal = self.without_parentheses(literal);
        let units = if let ExprKind::Literal(literal) = self.tree.expressions.get(literal).kind
            && let PostTokenKind::String { ty, .. } = literal.kind
        {
            Some((literal, ty.size()))
        } else {
            None
        };
        // The literal byte slice belongs to the syntax cursor; each unit is copied before mutation.
        let size = types::layout(self.graph, element)?.0;
        for index in 0..bound {
            let offset = index.checked_mul(size).ok_or(crate::lowir::Error::Limit)?;
            let place = if offset == 0 {
                base
            } else {
                let offset = self.builder.integer(u128::from(offset))?;
                self.emit(I::Index(Type::I8, None, base, offset), element)?
                    .value
            };
            let value = if let Some((literal, width)) = units {
                let unit = self.literal_array_unit(literal, index, width)?;
                RValue {
                    value: self.builder.integer(u128::from(unit))?,
                    ty: types::storage(self.graph, element)?,
                    source: element,
                    literal: false,
                    bits: None,
                }
            } else if let Some(&expr) =
                list.get(usize::try_from(index).map_err(|_| crate::lowir::Error::Limit)?)
            {
                let value = self.operand(expr, Context::Initialization)?;
                self.fit(value, element)?
            } else {
                self.zero(element)?
            };
            self.effect(I::Store(
                types::memory(self.graph, element)?,
                value.ty,
                value.value,
                place,
            ))?;
        }
        Ok(())
    }
    fn structured_array_item(
        &mut self,
        ty: TypeId,
        address: Value,
        list: &[ast::ExprId],
        next: &mut usize,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let mut expr = list.get(*next).copied();
        if let TypeView::Array { .. } = self.graph.lower_type(ty)? {
            if let Some(expr) = expr {
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expr).kind {
                    *next += 1;
                    return self.structured_array(
                        ty,
                        address,
                        self.tree.expr_lists.get(elements),
                        &mut 0,
                    );
                }
                if matches!(
                    self.graph
                        .lower_type(self.graph.lower_expression(expr)?.ty)?,
                    TypeView::Array { .. }
                ) {
                    *next += 1;
                    return self.structured_array(ty, address, &[expr], &mut 0);
                }
            }
            return self.structured_array(ty, address, list, next);
        }
        if let Some(id) = expr {
            *next += 1;
            if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                expr = self.tree.expr_lists.get(elements).first().copied();
            }
        }
        let value = if let Some(expr) = expr {
            let value = self.operand(expr, Context::ArrayElement)?;
            self.fit(value, ty)?
        } else {
            self.zero(ty)?
        };
        self.effect(I::Store(
            types::memory(self.graph, ty)?,
            types::storage(self.graph, ty)?,
            value.value,
            address,
        ))
    }
    fn structured_array(
        &mut self,
        ty: TypeId,
        address: Value,
        list: &[ast::ExprId],
        next: &mut usize,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let TypeView::Array { element, bound } = self.graph.lower_type(ty)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "structured array without array type",
            )));
        };
        let units = list.get(*next).copied().and_then(|mut id| {
            while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
                id = inner;
            }
            if let ExprKind::Literal(literal) = self.tree.expressions.get(id).kind
                && let PostTokenKind::String { ty, .. } = literal.kind
            {
                Some((literal, ty.size()))
            } else {
                None
            }
        });
        let units = if matches!(self.graph.lower_type(element)?, TypeView::Scalar(_)) {
            units
        } else {
            None
        };
        if units.is_some() {
            *next += 1;
        }
        let size = types::layout(self.graph, element)?.0;
        for index in 0..bound {
            let place = if index == 0 {
                address
            } else {
                let offset = self.builder.integer(u128::from(
                    index.checked_mul(size).ok_or(crate::lowir::Error::Limit)?,
                ))?;
                let projection =
                    if matches!(self.graph.lower_type(element)?, TypeView::Array { .. }) {
                        None
                    } else {
                        Some(crate::lowir::vocabulary::Projection::ArrayElement)
                    };
                self.emit(I::Index(Type::I8, projection, address, offset), element)?
                    .value
            };
            if let Some((literal, width)) = units {
                let unit = self.literal_array_unit(literal, index, width)?;
                let value = self.builder.integer(u128::from(unit))?;
                self.effect(I::Store(
                    types::memory(self.graph, element)?,
                    types::storage(self.graph, element)?,
                    value,
                    place,
                ))?;
            } else {
                self.structured_array_item(element, place, list, next)?;
            }
        }
        Ok(())
    }
}
