// Translated from lower/copy_lowering.cpp. See NOTICE.
//! Synthesized constructors consume ordered selections, never rerunning overload resolution.
use super::{
    Error,
    construction::{Entry, Variant},
    function::Function,
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{ConstructorId, CopyTarget, SubobjectCopy, TypeId, TypeView},
};
impl Function<'_, '_, '_, '_> {
    fn copy_offset(
        &self,
        class: crate::sema::lowering::EntityId,
        action: SubobjectCopy,
    ) -> Result<u64, Error> {
        Ok(match action.target {
            CopyTarget::Base(index) => {
                self.graph
                    .lower_base_initialization_span(class, index as usize, false)?
                    .0
            }
            CopyTarget::Member(member) => {
                if let Some(field) = self.graph.lower_bit_field(member)? {
                    field.bit / 8
                } else {
                    self.graph.lower_member_offset(member)?
                }
            }
        })
    }
    pub(super) fn copy_address(
        &mut self,
        object: Value,
        ty: TypeId,
        transfer: super::bit_fields::Transfer,
    ) -> Result<Value, Error> {
        let offset = self.builder.integer(u128::from(transfer.offset))?;
        Ok(self
            .emit(
                I::Index(
                    Type::I8,
                    transfer.member.map(|_| Projection::Field),
                    object,
                    offset,
                ),
                ty,
            )?
            .value)
    }
    pub(super) fn copy_members(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        actions: &[SubobjectCopy],
    ) -> Result<(), Error> {
        let plan = self.graph.lower_constructor(id);
        let class = plan.class;
        let ty = self.graph.lower_entity_type(class)?;
        let function = self
            .graph
            .lower_constructor_entity(id)
            .ok_or(crate::sema::Error::Invariant("copy body function"))?;
        let transfers: Vec<_> = actions
            .iter()
            .map(|&action| {
                Ok(super::bit_fields::Transfer {
                    member: match action.target {
                        CopyTarget::Member(member) => Some(member),
                        CopyTarget::Base(_) => None,
                    },
                    ty: action.ty,
                    offset: self.copy_offset(class, action)?,
                    trivial: !matches!(
                        self.graph.lower_type(action.ty)?,
                        TypeView::Reference { .. }
                    ) && action
                        .constructor
                        .is_none_or(|id| self.graph.lower_constructor_trivial(id)),
                })
            })
            .collect::<Result<_, Error>>()?;
        self.copying.active = true;
        let mut guarded = 0;
        for (index, action) in actions.iter().enumerate().rev() {
            if let Some(constructor) = action.constructor
                && !self.graph.lower_constructor_trivial(constructor)
                && self.default_constructor_throws(constructor)?
            {
                guarded = index;
                break;
            }
        }
        let mut first = transfers.iter().take_while(|action| action.trivial).count();
        let mut field_wise = false;
        for action in &transfers {
            field_wise |= if let Some(member) = action.member {
                self.graph.lower_local_name(member).is_some()
                    && self.graph.lower_bit_field(member)?.is_some()
            } else {
                true
            };
        }
        let limit = if variant == Variant::Base && !self.graph.lower_constructor_trivial(id) {
            self.graph.lower_nonvirtual_bytes(class)?
        } else {
            types::layout(self.graph, ty)?.0
        };
        if first == actions.len() && !field_wise {
            if !self.graph.lower_class_empty(class)? {
                let destination = self.assignment_this(ty)?;
                let source = self.assignment_source(function)?;
                let (size, align) = types::layout(self.graph, ty)?;
                self.effect(I::CopyObject(size.min(limit), align, source, destination))?;
            }
            self.copying.active = false;
            return Ok(());
        }
        let prefix = if first < transfers.len() {
            transfers[first].offset
        } else {
            0
        };
        if first == actions.len() {
            first = 0;
        }
        if prefix != 0 {
            let destination = self.assignment_this(ty)?;
            let source = self.assignment_source(function)?;
            self.effect(I::CopyObject(
                prefix,
                types::layout(self.graph, ty)?.1,
                source,
                destination,
            ))?;
            for &transfer in transfers.iter().take(first.min(guarded)) {
                self.note_copied_member(class, ty, transfer, None)?;
            }
        } else {
            first = 0;
        }
        while first < actions.len() {
            let action = actions[first];
            if let CopyTarget::Member(member) = action.target
                && self.graph.lower_bit_field(member)?.is_some()
            {
                if self.graph.lower_local_name(member).is_some() {
                    first = self.transfer_bit_fields(function, class, &transfers, first, limit)?;
                }
            } else {
                let address = self.copy_subobject(function, ty, action, transfers[first], limit)?;
                if first < guarded {
                    self.note_copied_member(class, ty, transfers[first], address)?;
                }
            }
            first += 1;
        }
        self.copying.active = false;
        self.copy_truncate(0);
        Ok(())
    }
    fn note_copied_member(
        &mut self,
        class: crate::sema::lowering::EntityId,
        ty: TypeId,
        transfer: super::bit_fields::Transfer,
        address: Option<Value>,
    ) -> Result<(), Error> {
        if self.graph.lower_union_class(class)
            || self.symbols.class_destructor(transfer.ty)?.is_none()
        {
            return Ok(());
        }
        if transfer
            .member
            .is_some_and(|member| self.graph.lower_local_name(member).is_none())
        {
            let mut element = transfer.ty;
            while let TypeView::Array { element: child, .. } = self.graph.lower_type(element)? {
                element = child;
            }
            if let TypeView::Class(child) = self.graph.lower_type(element)?
                && self.graph.lower_union_class(child)
            {
                return Ok(());
            }
        }
        let address = if let Some(address) = address {
            address
        } else {
            let object = self.assignment_this(ty)?;
            self.copy_address(object, transfer.ty, transfer)?
        };
        self.copied_subobject(transfer.ty, address);
        Ok(())
    }
    fn copy_subobject(
        &mut self,
        function: crate::sema::lowering::EntityId,
        ty: TypeId,
        action: SubobjectCopy,
        transfer: super::bit_fields::Transfer,
        limit: u64,
    ) -> Result<Option<Value>, Error> {
        let mut element = action.ty;
        while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
            element = inner;
        }
        let cls = if let TypeView::Class(class) = self.graph.lower_type(element)? {
            Some(class)
        } else {
            None
        };
        if let Some(class) = cls
            && element == action.ty
            && self.graph.lower_class_empty(class)?
            && transfer.trivial
        {
            if self.symbols.class_destructor(action.ty)?.is_some() {
                let destination = self.assignment_this(ty)?;
                let destination = self.copy_address(destination, action.ty, transfer)?;
                let source = self.assignment_source(function)?;
                self.copy_address(source, action.ty, transfer)?;
                return Ok(Some(destination));
            }
            return Ok(None);
        }
        let reference = matches!(
            self.graph.lower_type(action.ty)?,
            TypeView::Reference { .. }
        );
        if cls.is_none() && element == action.ty {
            let storage = types::storage(self.graph, action.ty)?;
            let destination = if reference {
                None
            } else {
                let object = self.assignment_this(ty)?;
                Some(self.copy_address(object, action.ty, transfer)?)
            };
            let source = self.assignment_source(function)?;
            let source = self.copy_address(source, action.ty, transfer)?;
            let value = self
                .emit(
                    I::Load(Facts::memory(false, storage.bits() != 8), storage, source),
                    action.ty,
                )?
                .value;
            let destination = if let Some(destination) = destination {
                destination
            } else {
                let object = self.assignment_this(ty)?;
                self.copy_address(object, action.ty, transfer)?
            };
            self.effect(I::Store(
                Facts::memory(false, storage.bits() != 8),
                storage,
                value,
                destination,
            ))?;
            return Ok(None);
        }
        let destination = self.assignment_this(ty)?;
        let destination = self.copy_address(destination, action.ty, transfer)?;
        if element != action.ty && !transfer.trivial {
            self.copy_elements(function, action, transfer, destination)?;
            return Ok(Some(destination));
        }
        let source = self.assignment_source(function)?;
        let source = self.copy_address(source, action.ty, transfer)?;
        if transfer.trivial {
            let (size, align) = types::layout(self.graph, action.ty)?;
            let size = if transfer.member.is_none() && transfer.offset.wrapping_add(size) > limit {
                self.graph.lower_nonvirtual_bytes(
                    cls.ok_or(crate::sema::Error::Invariant("copy base class"))?,
                )?
            } else {
                size
            };
            self.effect(I::CopyObject(size, align, source, destination))?;
            return Ok(if element == action.ty {
                Some(destination)
            } else {
                None
            });
        }
        let id = action
            .constructor
            .ok_or(crate::sema::Error::Invariant("nontrivial copy constructor"))?;
        self.call_copy_constructor(
            id,
            if transfer.member.is_none() {
                Variant::Base
            } else {
                Variant::Complete
            },
            destination,
            source,
        )?;
        Ok(Some(destination))
    }
    pub(super) fn delegate_copy_storage(
        &mut self,
        id: ConstructorId,
        object: Value,
        written: crate::sema::lowering::ConstructorArguments,
    ) -> Result<bool, Error> {
        let plan = self.graph.lower_constructor(id);
        if plan.copy.is_none() || !self.graph.lower_constructor_trivial(id) {
            return Ok(false);
        }
        let single = match written {
            crate::sema::lowering::ConstructorArguments::Single(id) => Some(id),
            crate::sema::lowering::ConstructorArguments::Values(values) => {
                if let [id] = self.tree.expr_lists.get(values) {
                    Some(*id)
                } else {
                    None
                }
            }
            crate::sema::lowering::ConstructorArguments::Value => None,
        };
        let Some(source) = single else {
            return Ok(false);
        };
        let empty = self.graph.lower_class_empty(plan.class)?;
        let ty = self.graph.lower_entity_type(plan.class)?;
        if empty && self.symbols.class_destructor(ty)?.is_some() {
            return Ok(false);
        }
        if empty && self.side_effect_free(source, false, 0)? {
            return Ok(true);
        }
        let facts = self.graph.lower_expression(source)?;
        if facts.is_place {
            let signature = self.graph.lower_constructor_function(id)?.ty;
            let TypeView::Function { parameters, .. } = self.graph.lower_type(signature)? else {
                return Err(crate::sema::Error::Invariant("delegated copy signature").into());
            };
            let source =
                self.bind_reference(source, parameters[0], super::conversions::Context::Argument)?;
            if !empty {
                let (size, alignment) = types::layout(self.graph, ty)?;
                self.effect(I::CopyObject(size, alignment, source, object))?;
            }
        } else {
            self.materialize_class(source, ty, object)?;
        }
        Ok(true)
    }
    pub(super) fn call_copy_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        destination: Value,
        source: Value,
    ) -> Result<(), Error> {
        let entry = Entry::Constructor(id, variant);
        let facts = entry.facts(self.graph)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(crate::sema::Error::Invariant("copy call type").into());
        };
        let throws = self.default_constructor_throws(id)?;
        let chained = if parameters.len() > 1 {
            self.begin_copy_call(throws)?
        } else {
            false
        };
        let made = if parameters.len() > 1 {
            self.copy_defaults_temporaries(id)?
        } else {
            false
        };
        let scope = self.begin_copy_temporaries(chained && made);
        let began = parameters.len() > 1
            && !self.lifetime.full_expression
            && (made || !self.lifetime.cleanups.is_empty())
            && throws;
        if began {
            self.lifetime.full_expression = true;
            self.lifetime.region_start = super::lifetime::RegionStart::Pending;
            self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Eager;
            self.ensure_insertion()?;
            self.lifetime.expression_start = Some(self.builder.position()?);
        }
        let first = self.lifetime.cleanups.len();
        let mut values = vec![destination, source];
        for (index, &parameter) in parameters.iter().enumerate().skip(1) {
            let function = self
                .graph
                .lower_constructor_entity(id)
                .ok_or(crate::sema::Error::Invariant("copy defaults owner"))?;
            let expression = self
                .graph
                .lower_default(function, index)
                .ok_or(crate::sema::Error::Invariant("copy default argument"))?;
            let value = if matches!(
                self.graph.lower_type(parameter)?,
                TypeView::Reference { .. }
            ) {
                self.bind_reference(expression, parameter, super::conversions::Context::Argument)?
            } else if matches!(self.graph.lower_type(parameter)?, TypeView::Class(_)) {
                self.class_argument(expression, parameter)?
            } else {
                let value = self.operand(expression, super::conversions::Context::Argument)?;
                self.fit(value, parameter)?.value
            };
            values.push(value);
        }
        let chained = if parameters.len() == 1 {
            self.begin_copy_call(throws)?
        } else {
            chained
        };
        self.emit_constructor_call(entry, facts.nothrow, &values)?;
        if self.element_temporaries.depth == 0 {
            let mut throwing_destructor = false;
            for index in first..self.lifetime.cleanups.len() {
                throwing_destructor |= self.destructor_throws(self.lifetime.cleanups[index].ty)?;
            }
            if throwing_destructor {
                self.close_lifetime_region()?;
            }
            self.destroy_temporaries(first)?;
        }
        self.end_lifetime_expression(began)?;
        self.end_copy_temporaries(scope)?;
        self.end_copy_call(chained)
    }
}
