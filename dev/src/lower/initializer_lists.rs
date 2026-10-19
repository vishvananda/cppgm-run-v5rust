// Translated from lower/initializer_list_lowering.cpp. See NOTICE.
//! Backing storage and the wrapper are separate objects with explicit owners.
use super::{Error, conversions::Context, function::Function, types};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn list_object(&self, id: ast::ExprId) -> Result<Option<ast::ExprId>, Error> {
        let id = self.without_parentheses(id);
        let list = match self.tree.expressions.get(id).kind {
            ExprKind::Braced { .. } => id,
            ExprKind::FunctionalCast {
                braced: Some(list), ..
            } => list,
            _ => return Ok(None),
        };
        if self.graph.lower_list_constructor(list)?.is_some() {
            return Ok(None);
        }
        Ok(self.graph.lower_initializer_array(list).map(|_| list))
    }

    pub(super) fn list_variable(
        &self,
        initializer: Option<ast::Initializer>,
    ) -> Result<Option<ast::ExprId>, Error> {
        let Some(initializer) = initializer else {
            return Ok(None);
        };
        let id = match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => id,
            ast::InitializerKind::Parenthesized(args)
                if self.tree.expr_lists.get(args).len() == 1 =>
            {
                self.tree.expr_lists.get(args)[0]
            }
            _ => return Ok(None),
        };
        self.list_object(id)
    }

    pub(super) fn initialize_list_object(
        &mut self,
        list: ast::ExprId,
        ty: TypeId,
        object: Value,
        persistent: bool,
    ) -> Result<(), Error> {
        let array = self
            .graph
            .lower_initializer_array(list)
            .ok_or(crate::sema::Error::Invariant("list backing type"))?;
        let TypeView::Array { element, bound } = self.graph.lower_type(array)? else {
            return Err(crate::sema::Error::Invariant("list backing array").into());
        };
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("list wrapper class").into());
        };
        let members = self.graph.lower_class_member_entities(class);
        let [first, second, ..] = members else {
            return Err(crate::sema::Error::at(
                self.tree.expressions.get(list).location,
                "a std::initializer_list without its array's address and size members",
            )
            .into());
        };
        let offsets = [
            self.graph.lower_member_offset(*first)?,
            self.graph.lower_member_offset(*second)?,
        ];
        let elements = if bound == 0 {
            self.builder.integer(0)?
        } else {
            self.list_backing_array(list, array, element, persistent)?
        };
        let size = self.builder.integer(u128::from(bound))?;
        for (offset, value, storage) in [
            (offsets[0], elements, Type::Ptr),
            (offsets[1], size, Type::I64),
        ] {
            let address = if offset == 0 {
                object
            } else {
                let offset = self.builder.integer(u128::from(offset))?;
                self.emit(I::Index(Type::I8, None, object, offset), ty)?
                    .value
            };
            self.effect(I::Store(
                Facts::memory(false, true),
                storage,
                value,
                address,
            ))?;
        }
        Ok(())
    }

    fn list_backing_array(
        &mut self,
        list: ast::ExprId,
        array: TypeId,
        element: TypeId,
        persistent: bool,
    ) -> Result<Value, Error> {
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Err(crate::sema::Error::Invariant("list backing syntax").into());
        };
        let class = matches!(self.graph.lower_type(element)?, TypeView::Class(_));
        if class && self.symbols.storage_destructor(element)?.is_some() {
            return Err(Error::Missing(
                "initializer-list class backing array lifetime",
            ));
        }
        let slot = self.temporary_storage(array, persistent, "initlist")?;
        let base = self.emit(I::Addr(slot), array)?.value;
        self.storage_region_event()?;
        let stride = types::layout(self.graph, element)?.0;
        for (index, &item) in self.tree.expr_lists.get(elements).iter().enumerate() {
            if class {
                let address = if index == 0 {
                    base
                } else {
                    let offset = u64::try_from(index)
                        .map_err(|_| crate::lowir::Error::Limit)?
                        .wrapping_mul(stride);
                    let offset = self.builder.integer(u128::from(offset))?;
                    self.emit(
                        I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                        element,
                    )?
                    .value
                };
                self.materialize_class(item, element, address)?;
                continue;
            }
            let source = match self.tree.expressions.get(item).kind {
                ExprKind::Braced { elements, .. } => {
                    self.tree.expr_lists.get(elements).first().copied()
                }
                _ => Some(item),
            };
            let value = if let Some(source) = source {
                let value = self.operand(source, Context::ArrayElement)?;
                self.fit(value, element)?.value
            } else {
                self.zero(element)?.value
            };
            let address = if index == 0 {
                base
            } else {
                let offset = u64::try_from(index)
                    .map_err(|_| crate::lowir::Error::Limit)?
                    .wrapping_mul(stride);
                let offset = self.builder.integer(u128::from(offset))?;
                self.emit(
                    I::Index(Type::I8, Some(Projection::ArrayElement), base, offset),
                    element,
                )?
                .value
            };
            self.effect(I::Store(
                types::memory(self.graph, element)?,
                types::storage(self.graph, element)?,
                value,
                address,
            ))?;
        }
        if !persistent {
            self.finish_inert_temporary(false)?;
        }
        Ok(base)
    }
}
