// Translated from lower/initializer_list_lowering.cpp. See NOTICE.
//! Backing storage and the wrapper are separate objects with explicit owners.
use super::{Error, conversions::Context, function::Function, types};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{EntityId, TypeId, TypeView},
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

    pub(super) fn destroyed_list_array(&self, id: ast::ExprId) -> Result<bool, Error> {
        self.check_stack()?;
        let Some(list) = self.list_object(id)? else {
            return Ok(false);
        };
        let array = self
            .graph
            .lower_initializer_array(list)
            .ok_or(crate::sema::Error::Invariant("list backing type"))?;
        if matches!(self.graph.lower_type(array)?, TypeView::Array { bound, .. } if bound != 0)
            && self.temporary_type_destroyed(array)?
        {
            return Ok(true);
        }
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Err(crate::sema::Error::Invariant("list backing syntax").into());
        };
        for &element in self.tree.expr_lists.get(elements) {
            if self.destroyed_list_array(element)? {
                return Ok(true);
            }
        }
        Ok(false)
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
        owner: Option<EntityId>,
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
            self.list_backing_array(list, array, element, persistent, owner)?
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

    pub(super) fn materialize_list_choice(
        &mut self,
        operand: ast::ExprId,
        ty: TypeId,
        object: Value,
    ) -> Result<bool, Error> {
        let Some(list) = self.list_object(operand)? else {
            return Ok(false);
        };
        let array =
            self.graph
                .lower_initializer_array(list)
                .ok_or(crate::sema::Error::Invariant(
                    "conditional list backing type",
                ))?;
        if !matches!(self.graph.lower_type(array)?, TypeView::Array { bound, .. } if bound != 0) {
            return Ok(false);
        }
        if self.lifetime.tracking.active
            && self.lifetime.full_expression
            && !self.lifetime.region
            && self.handlers.explicit == 0
        {
            self.open_temporary_region()?;
        }
        let slot = self.generated_slot("argobj", types::storage(self.graph, ty)?)?;
        let address = self.emit(I::Addr(slot), ty)?.value;
        self.initialize_list_object(list, ty, address, false, None)?;
        let (size, alignment) = types::layout(self.graph, ty)?;
        self.effect(I::CopyObject(size, alignment, slot, object))?;
        Ok(true)
    }

    fn finish_persistent_list(
        &mut self,
        array: TypeId,
        storage: Value,
        owner: Option<EntityId>,
    ) -> Result<(), Error> {
        if owner.is_none()
            || self
                .static_temporaries
                .as_ref()
                .is_some_and(|local| local.reference)
        {
            return self.persistent_alive(array, storage);
        }
        let Value::Symbol(global) = storage else {
            return Err(crate::sema::Error::Invariant("persistent list storage").into());
        };
        if let Some(finalizer) = super::shutdown::for_type(
            self.builder.program(),
            self.symbols,
            self.graph,
            global,
            array,
            None,
        )? {
            if let Some(local) = self.static_temporaries.as_mut() {
                local.made.push(finalizer);
            } else if let Some(output) = self.initialization_output.as_mut() {
                output.finalizers.push(finalizer);
            } else {
                return Err(crate::sema::Error::Invariant(
                    "persistent list without finalization owner",
                )
                .into());
            }
        }
        Ok(())
    }

    fn list_backing_array(
        &mut self,
        list: ast::ExprId,
        array: TypeId,
        element: TypeId,
        persistent: bool,
        owner: Option<EntityId>,
    ) -> Result<Value, Error> {
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Err(crate::sema::Error::Invariant("list backing syntax").into());
        };
        let class = matches!(self.graph.lower_type(element)?, TypeView::Class(_));
        let guarded = class && self.symbols.storage_destructor(element)?.is_some();
        let destroyed = guarded && !persistent && self.temporary_type_destroyed(element)?;
        let opens = owner.is_none()
            && (destroyed
                || !persistent
                    && self.lifetime.region_start.is_pending()
                    && self.lifetime.guarded_operands == 0
                    && self.lifetime.choice_region != super::lifetime::ChoiceRegion::Reopened)
            && self.lifetime.full_expression
            && !self.lifetime.region;
        // With live objects, a pending segment opens after this array's
        // address unless a throwing call already owns its operand boundary.
        let deferred = opens
            && self.lifetime.region_start.is_pending()
            && !self.lifetime.temporary_segment.is_nothrow()
            && !self.lifetime.cleanups.is_empty()
            && (!self.lifetime.tracking.active || self.reference_lifetimes.has_scope())
            && !self
                .lifetime
                .call_marks
                .iter()
                .any(|mark| !mark.declared && self.builder.same_block(mark.position));
        if opens && !deferred {
            self.open_temporary_region()?;
        }
        let slot = self.temporary_storage(array, persistent, "initlist")?;
        let base = self.emit(I::Addr(slot), array)?.value;
        self.storage_region_event()?;
        if deferred {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        let stride = types::layout(self.graph, element)?.0;
        if class {
            let made = self.initialize_list_elements(
                list,
                super::list_elements::Backing {
                    array,
                    element,
                    base,
                    persistent,
                    owner,
                    storage: slot,
                },
            )?;
            if destroyed && !self.destructor_throws(element)? {
                self.lifetime.list_elements.insert(base, &made)?;
            }
        }
        for (index, &item) in self.tree.expr_lists.get(elements).iter().enumerate() {
            if class {
                break;
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
        if persistent && guarded {
            self.finish_persistent_list(array, slot, owner)?;
        } else if destroyed {
            self.temporary_alive(array, base)?;
            if let Some(owner) = owner {
                self.extend_last_temporary(owner)?;
            }
            if self.lifetime.full_expression && !self.lifetime.region {
                let position = self.builder.position()?;
                self.open_temporary_region_at(Some(position))?;
            }
        } else if !persistent {
            self.finish_inert_temporary(false)?;
        }
        Ok(base)
    }
}
