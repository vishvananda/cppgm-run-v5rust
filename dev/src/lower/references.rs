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
        model::{DataItem, Facts, Global, GlobalInit, InstructionKind as I, Metadata, Value},
        vocabulary::{Binding, Type},
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn evaluated_reference_place(
        &mut self,
        value: crate::sema::lowering::StaticValue,
        source: TypeId,
    ) -> Result<Value, Error> {
        let (symbol, offset) = self.static_data_address(value)?;
        let place = Value::Symbol(symbol);
        if offset == 0 {
            return Ok(place);
        }
        let address = self.emit(I::Addr(place), source)?.value;
        let offset = self
            .builder
            .integer(u128::from(u64::from_le_bytes(offset.to_le_bytes())))?;
        Ok(self
            .emit(I::Index(Type::I8, None, address, offset), source)?
            .value)
    }
    pub(super) fn storage_address(&mut self, storage: Value, ty: TypeId) -> Result<Value, Error> {
        if matches!(storage, Value::Slot(_) | Value::Symbol(_)) {
            Ok(self.emit(I::Addr(storage), ty)?.value)
        } else {
            Ok(storage)
        }
    }
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
        let persistent = self.persistent_references
            && matches!(
                context,
                Context::Initialization | Context::MemberInitialization
            );
        self.bind_reference_with_persistence(id, reference, context, persistent)
    }
    fn bind_reference_with_persistence(
        &mut self,
        id: ast::ExprId,
        reference: TypeId,
        context: Context,
        persistent: bool,
    ) -> Result<Value, Error> {
        self.check_stack()?;
        let TypeView::Reference { element, .. } = self.graph.lower_type(reference)? else {
            return Err(crate::sema::Error::Invariant("binding without reference type").into());
        };
        let facts = self.graph.lower_expression(id)?;
        if self.class_conversion(id).is_some() {
            return self.converting_reference_temporary(id, element, context, persistent);
        }
        if let Some((call, _)) = self.graph.lower_function_conversion(id)? {
            return self.bind_function_conversion(id, call, element, persistent);
        }
        if matches!(self.graph.lower_type(element)?, TypeView::Array { .. })
            && !facts.is_place
            && matches!(self.graph.lower_type(facts.ty)?, TypeView::Array { .. })
        {
            return self.bound_array_temporary(
                id,
                if matches!(context, Context::Argument) {
                    "argarr"
                } else {
                    "arraytmp"
                },
                persistent,
            );
        }
        if !facts.is_place && matches!(self.graph.lower_type(facts.ty)?, TypeView::Class(_)) {
            let address = self.class_temporary(
                id,
                if matches!(context, Context::Argument) {
                    super::materialization::TemporaryUse::Argument
                } else {
                    super::materialization::TemporaryUse::Reference
                },
                persistent,
            )?;
            if let Some(offset) = self.graph.lower_base_conversion(id)? {
                return self.project_base_access(
                    address,
                    facts.ty,
                    offset,
                    super::base_projections::BaseAccess::Complete,
                );
            }
            return Ok(address);
        }
        if facts.temporary.is_some()
            || (!facts.is_place
                && !matches!(self.graph.lower_type(facts.ty)?, TypeView::Function { .. }))
        {
            if matches!(
                self.graph.lower_type(element)?,
                TypeView::Array { .. } | TypeView::Class(_) | TypeView::Unsupported
            ) && self.graph.lower_member_pointer_type(element).is_none()
            {
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
                if let Some(adjustment) = self.graph.lower_member_pointer_conversion(id)? {
                    self.convert_member_pointer(value, target, adjustment)?
                } else if context != Context::Cast
                    && let Some(offset) = self.graph.lower_base_conversion(id)?
                {
                    self.convert_base_pointer(value, target, offset, !offset.is_zero())?
                } else {
                    self.convert(value, target, context)?
                }
            } else {
                value
            };
            let value = self.fit(value, target)?;
            let storage = types::storage(self.graph, target)?;
            let slot = if !persistent
                && let Some(slot) = chosen
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
                self.temporary_storage(target, persistent, stem)?
            };
            self.effect(I::Store(
                Facts::memory(false, !matches!(storage, Type::I8 | Type::U8)),
                storage,
                value.value,
                slot,
            ))?;
            return Ok(self.emit(I::Addr(slot), target)?.value);
        }
        // bind_converted forwards the outer lifetime to the temporary made
        // by a reference cast's binding, independently of its Cast context.
        let root = self.without_parentheses(id);
        let address = if let Some(reference) = facts.reference_cast
            && self.graph.lower_runtime_cast(root)?.is_none()
            && let ExprKind::Cast { operand, .. } = self.tree.expressions.get(root).kind
            && (self.graph.lower_expression(operand)?.temporary.is_some()
                || self.class_conversion(operand).is_some()
                || self.graph.lower_function_conversion(operand)?.is_some())
        {
            self.bind_reference_with_persistence(operand, reference, Context::Cast, persistent)?
        } else {
            self.address(id)?
        };
        if let Some(offset) = self.graph.lower_base_conversion(id)? {
            self.project_base_access(
                address,
                facts.ty,
                offset,
                self.member_base_access(id, false)?,
            )
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
        if self.graph.lower_runtime_cast(id)?.is_some() {
            return Ok(self.dynamic_cast_value(id, operand)?.value);
        }
        if let Some(reference) = facts.reference_cast
            && (self.graph.lower_expression(operand)?.temporary.is_some()
                || self.class_conversion(operand).is_some()
                || self.graph.lower_function_conversion(operand)?.is_some()
                || self.graph.lower_base_conversion(operand)?.is_some())
        {
            return self.bind_reference(operand, reference, Context::Cast);
        }
        if let Some((delta, project_zero)) = self.graph.lower_cast_base_adjustment(id)?
            && (!delta.is_zero() || project_zero)
        {
            let address = self.address(operand)?;
            return self.project_base_access(
                address,
                facts.ty,
                delta,
                self.member_base_access(operand, false)?,
            );
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
    pub(super) fn temporary_storage(
        &mut self,
        ty: TypeId,
        persistent: bool,
        stem: &'static str,
    ) -> Result<Value, Error> {
        if !persistent {
            return self.generated_slot(stem, types::storage(self.graph, ty)?);
        }
        // A temporary's global has its own identity and sequence under the
        // initializing variable. It is never shared by value with another one.
        let (symbol, object) = self
            .symbols
            .dynamic_temporary_symbol(self.builder.program(), ty)?;
        let (storage, init) = if matches!(
            self.graph.lower_type(ty)?,
            TypeView::Class(_) | TypeView::Array { .. }
        ) {
            (
                None,
                GlobalInit::Data(vec![DataItem::Zero(
                    i64::try_from(types::layout(self.graph, ty)?.0)
                        .map_err(|_| crate::lowir::Error::Limit)?,
                )]),
            )
        } else {
            (Some(types::storage(self.graph, ty)?), GlobalInit::Zero)
        };
        let storage = if let GlobalInit::Data(items) = &init {
            super::globals::structured_type(self.graph, ty, items, self.symbols.object_mode())?
        } else {
            storage
        };
        let program = self.builder.program();
        program.add_global(
            symbol,
            Global {
                name: program.symbol(symbol).name,
                ty: storage,
                metadata: Metadata {
                    binding: Some(Binding::Internal),
                    object,
                    ..Metadata::default()
                },
                init,
            },
        )?;
        Ok(Value::Symbol(symbol))
    }
}
