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
    support::source::SourceOffset,
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
            CopyTarget::VirtualBase(base) => self
                .graph
                .lower_virtual_base_offset(class, base)
                .ok_or(crate::sema::Error::Invariant("copy virtual base layout"))?,
            CopyTarget::Member(member) => {
                if let Some(field) = self.graph.lower_bit_field(member)? {
                    field.bit / 8
                } else {
                    self.graph.lower_member_offset(member)?
                }
            }
        })
    }
    fn shared_copy_base(
        &self,
        class: crate::sema::lowering::EntityId,
        action: SubobjectCopy,
    ) -> Option<crate::sema::lowering::EntityId> {
        match action.target {
            CopyTarget::VirtualBase(base) => Some(base),
            CopyTarget::Base(index) => self
                .graph
                .lower_inheritance_edge(class, index as usize)
                .and_then(|(base, shared, _)| shared.then_some(base)),
            CopyTarget::Member(_) => None,
        }
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
        let class = self.graph.lower_constructor(id).class;
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
                        CopyTarget::Base(_) | CopyTarget::VirtualBase(_) => None,
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
        for index in (0..actions.len()).rev() {
            if self.copy_action_throws(actions[index])? {
                guarded = index;
                break;
            }
        }
        let mut first = transfers.iter().take_while(|action| action.trivial).count();
        if self.graph.lower_dynamic_class(class) {
            first = 0;
        }
        let mut field_wise = self.graph.lower_dynamic_class(class);
        for action in &transfers {
            field_wise |= if let Some(member) = action.member {
                self.graph.lower_local_name(member).is_some()
                    && self.graph.lower_bit_field(member)?.is_some()
            } else {
                true
            };
        }
        let mut limit = if variant == Variant::Base && !self.graph.lower_constructor_trivial(id) {
            self.graph.lower_nonvirtual_bytes(class)?
        } else {
            types::layout(&*self.graph, ty)?.0
        };
        if variant != Variant::Base {
            for &(base, offset) in self.graph.lower_virtual_bases(class) {
                if !self.graph.lower_class_empty(base)? {
                    limit = limit.min(offset);
                }
            }
        }
        if first == actions.len() && !field_wise {
            if !self.graph.lower_class_empty(class)? {
                let destination = self.assignment_this(ty)?;
                let source = self.assignment_source(function)?;
                self.copy_object(ty, source, destination)?;
            }
            self.copying.active = false;
            return Ok(());
        }
        let prefix = if first < transfers.len() && !self.graph.lower_dynamic_class(class) {
            transfers[first].offset
        } else {
            0
        };
        if first == actions.len() {
            first = 0;
        }
        if variant != Variant::Base {
            let mut planned = std::collections::HashMap::new();
            let mut later = false;
            for (index, &action) in actions.iter().enumerate() {
                if let Some(base) = self.shared_copy_base(class, action) {
                    planned.insert(base, index);
                } else if !later {
                    later = self.copy_action_throws(action)?;
                }
            }
            let order: Vec<_> = self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .virtual_construction_order(class)
                .into_iter()
                .filter_map(|base| planned.get(&base).copied())
                .collect();
            let mut tracked = vec![false; order.len()];
            for (position, &index) in order.iter().enumerate().rev() {
                tracked[position] = later || index < guarded;
                if !later {
                    later = self.copy_action_throws(actions[index])?;
                }
            }
            for (position, &index) in order.iter().enumerate() {
                let address =
                    self.copy_subobject(function, ty, actions[index], transfers[index], limit)?;
                if tracked[position] {
                    self.note_copied_member(class, ty, actions[index], transfers[index], address)?;
                }
            }
        }
        if prefix != 0 {
            let destination = self.assignment_this(ty)?;
            let source = self.assignment_source(function)?;
            self.effect(I::CopyObject(
                prefix,
                types::layout(&*self.graph, ty)?.1,
                source,
                destination,
            ))?;
            for index in 0..first.min(guarded) {
                self.note_copied_member(class, ty, actions[index], transfers[index], None)?;
            }
        } else {
            first = 0;
        }
        while first < actions.len() {
            let action = actions[first];
            if self.shared_copy_base(class, action).is_some() {
                first += 1;
                continue;
            }
            if let CopyTarget::Member(member) = action.target
                && self.graph.lower_bit_field(member)?.is_some()
            {
                if self.graph.lower_local_name(member).is_some() {
                    first = self.transfer_bit_fields(function, class, &transfers, first, limit)?;
                }
            } else {
                let address = self.copy_subobject(function, ty, action, transfers[first], limit)?;
                if first < guarded {
                    self.note_copied_member(class, ty, action, transfers[first], address)?;
                }
            }
            first += 1;
        }
        self.copying.active = false;
        self.copy_truncate(0);
        self.store_vptr(class, self.graph.lower_constructor(id).object_pointer()?)?;
        Ok(())
    }
    fn copy_action_throws(&mut self, action: SubobjectCopy) -> Result<bool, Error> {
        if let Some(constructor) = action.constructor {
            Ok(!self.graph.lower_constructor_trivial(constructor)
                && self.copy_constructor_throws(constructor)?)
        } else {
            Ok(false)
        }
    }
    fn note_copied_member(
        &mut self,
        class: crate::sema::lowering::EntityId,
        ty: TypeId,
        action: SubobjectCopy,
        transfer: super::bit_fields::Transfer,
        address: Option<Value>,
    ) -> Result<(), Error> {
        if self.graph.lower_union_class(class)
            || self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .class_destructor(transfer.ty)?
                .is_none()
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
        let base = match action.target {
            CopyTarget::Member(_) => None,
            CopyTarget::Base(index) if self.shared_copy_base(class, action).is_none() => {
                Some(super::destruction::BaseSubobject::Direct(index))
            }
            _ => Some(super::destruction::BaseSubobject::Virtual(
                self.shared_copy_base(class, action)
                    .ok_or(crate::sema::Error::Invariant("copy cleanup shared base"))?,
            )),
        };
        self.copied_subobject(transfer.ty, address, base);
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
            if self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .class_destructor(action.ty)?
                .is_some()
            {
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
            let storage = types::storage(&*self.graph, action.ty)?;
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
        let owner = self
            .graph
            .lower_function(function)?
            .object
            .ok_or(crate::sema::Error::Invariant("copy owner"))?;
        let shared = self.shared_copy_base(owner, action);
        let source = if let Some(base) = shared {
            self.virtual_base_address(source, action.ty, owner, base)?
        } else {
            self.copy_address(source, action.ty, transfer)?
        };
        if transfer.trivial {
            let (size, align) = types::layout(&*self.graph, action.ty)?;
            if shared.is_none()
                && transfer.member.is_none()
                && transfer.offset.wrapping_add(size) > limit
            {
                let bytes = self.graph.lower_nonvirtual_bytes(
                    cls.ok_or(crate::sema::Error::Invariant("copy base class"))?,
                )?;
                self.effect(I::CopyObject(bytes, align, source, destination))?;
            } else {
                self.copy_object(action.ty, source, destination)?;
            }
            return Ok(if element == action.ty {
                Some(destination)
            } else {
                None
            });
        }
        let id = action
            .constructor
            .ok_or(crate::sema::Error::Invariant("nontrivial copy constructor"))?;
        let saved = self.virtual_context.hidden;
        if transfer.member.is_none()
            && !self
                .graph
                .lower_virtual_bases(cls.ok_or(crate::sema::Error::Invariant("copy base class"))?)
                .is_empty()
        {
            self.virtual_context.hidden = Some(match action.target {
                CopyTarget::Base(index) if shared.is_none() => self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .vtt_base_index(owner, index as usize)?,
                _ => self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .vtt_virtual_index(
                        owner,
                        cls.ok_or(crate::sema::Error::Invariant("shared copy class"))?,
                    )?,
            });
        }
        let result = self.call_subobject_copy_constructor(
            id,
            if transfer.member.is_none() {
                Variant::Base
            } else {
                Variant::Complete
            },
            destination,
            source,
            SourceOffset(0),
        );
        self.virtual_context.hidden = saved;
        result?;
        Ok(Some(destination))
    }
    pub(super) fn delegate_copy_storage(
        &mut self,
        id: ConstructorId,
        object: Value,
        written: crate::sema::lowering::ConstructorArguments,
        location: crate::support::source::SourceOffset,
    ) -> Result<bool, Error> {
        self.delegate_copy_storage_bound(
            id,
            object,
            written,
            location,
            super::construction::CopyCall::Expression,
        )
    }
    pub(super) fn delegate_copy_storage_bound(
        &mut self,
        id: ConstructorId,
        object: Value,
        written: crate::sema::lowering::ConstructorArguments,
        location: SourceOffset,
        call: super::construction::CopyCall,
    ) -> Result<bool, Error> {
        if !self.graph.lower_constructor_copies_storage(id) {
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
        if self.graph.lower_function_conversion(source)?.is_some() {
            return Ok(false);
        }
        let empty = self
            .graph
            .lower_class_empty(self.graph.lower_constructor(id).class)?;
        let ty = self
            .graph
            .lower_entity_type(self.graph.lower_constructor(id).class)?;
        if empty
            && self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .class_destructor(ty)?
                .is_some()
        {
            return Ok(false);
        }
        if empty && self.side_effect_free(source, false, 0)? {
            return Ok(true);
        }
        let facts = self.graph.lower_expression(source)?;
        if !facts.is_place
            && matches!(self.graph.lower_type(facts.ty)?, TypeView::Class(class) if class == self.graph.lower_constructor(id).class)
        {
            self.materialize_prvalue(source, ty, object, false, location)?;
        } else {
            let signature = self.graph.lower_constructor_function(id)?.ty;
            let TypeView::Function { parameters, .. } = self.graph.lower_type(signature)? else {
                return Err(crate::sema::Error::Invariant("delegated copy signature").into());
            };
            let parameters = parameters.to_vec();
            let parameters = parameters.as_slice();
            let source = if let super::construction::CopyCall::InternalParameter {
                reference,
                projection,
            } = call
            {
                self.bind_internal_copy_source(source, reference, projection)?
            } else {
                self.bind_reference(source, parameters[0], super::conversions::Context::Argument)?
            };
            if !empty {
                self.copy_object(ty, source, object)?;
            }
        }
        Ok(true)
    }
    // object_lowering.cpp::construct_from: an address-bound first argument.
    pub(super) fn construct_from(
        &mut self,
        id: ConstructorId,
        destination: Value,
        source: Value,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let class = self.graph.lower_constructor(id).class;
        let ty = self.graph.lower_entity_type(class)?;
        let empty_lifetime = self.graph.lower_constructor_trivial(id)
            && self.graph.lower_class_empty(class)?
            && self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .class_destructor(ty)?
                .is_some();
        if self.graph.lower_constructor_copies_storage(id) && !empty_lifetime {
            if !self.graph.lower_class_empty(class)? {
                self.copy_object(ty, source, destination)?;
            }
            return Ok(());
        }
        let entry = Entry::Constructor(id, Variant::Complete);
        let facts = entry.facts(&*self.graph)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(
                crate::sema::Error::Invariant("address-bound constructor signature").into(),
            );
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let throws = self.entry_call_throws(entry)?;
        self.begin_lifetime_call(throws)?;
        let mut values = vec![destination, source];
        for (index, &parameter) in parameters.iter().enumerate().skip(1) {
            let function = entry
                .entity(&*self.graph)
                .ok_or(crate::sema::Error::Invariant("capture copy defaults owner"))?;
            let expression =
                self.graph
                    .lower_default(function, index)
                    .ok_or(crate::sema::Error::Invariant(
                        "capture copy default argument",
                    ))?;
            values.push(self.constructor_default_value(expression, parameter, location)?);
        }
        let result = self.emit_address_constructor_call(entry, !throws, &values);
        if throws && self.lifetime.full_expression {
            self.lifetime.call_marks.pop();
        }
        result
    }
    pub(super) fn constructor_default_value(
        &mut self,
        expression: crate::syntax::ast::ExprId,
        parameter: TypeId,
        location: SourceOffset,
    ) -> Result<Value, Error> {
        use super::conversions::Context;
        self.with_source_site(Some(location), |lowering| {
            match lowering.graph.lower_type(parameter)? {
                TypeView::Reference { .. } => {
                    lowering.bind_reference(expression, parameter, Context::Argument)
                }
                TypeView::Class(_) => lowering.class_argument(expression, parameter),
                _ => {
                    let value = lowering.operand(expression, Context::Argument)?;
                    Ok(lowering.fit(value, parameter)?.value)
                }
            }
        })
    }
    /// `copy_lowering.cpp::copy_call`: constructor plans have no parameter copy.
    /// Keep this policy above the lower-level `call_copy` entry/default demand.
    pub(super) fn call_subobject_copy_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        destination: Value,
        source: Value,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let signature = self.graph.lower_constructor_type(id)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(signature)? else {
            return Err(crate::sema::Error::Invariant("subobject copy signature").into());
        };
        let parameter = *parameters
            .first()
            .ok_or(crate::sema::Error::Invariant("subobject copy parameter"))?;
        if matches!(
            self.graph.lower_type(parameter)?,
            TypeView::Reference { .. }
        ) {
            return self.call_copy_constructor(id, variant, destination, source, location);
        }
        // Source caller_destroyed observes passed_directly before destruction,
        // even though copied_parameter must subsequently reject the null copy.
        let caller_destroyed = if let TypeView::Class(class) = self.graph.lower_type(parameter)? {
            !self.graph.lower_direct_class_parameter(class)?
                && self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .storage_destructor(parameter)?
                    .is_some()
        } else {
            false
        };
        if caller_destroyed {
            if !self.lifetime.full_expression {
                self.lifetime.full_expression = true;
                self.lifetime.region_start = super::lifetime::RegionStart::Pending;
                self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Eager;
                self.ensure_insertion()?;
                self.lifetime.expression_start = Some(self.builder.position()?);
            }
            self.open_temporary_region()?;
        }
        Err(crate::sema::Error::at(
            location,
            "a by-value assignment parameter without its constructor",
        )
        .into())
    }
    pub(super) fn call_copy_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        destination: Value,
        source: Value,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let entry = Entry::Constructor(id, variant);
        let facts = entry.facts(&*self.graph)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(crate::sema::Error::Invariant("copy call type").into());
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let throws = self.entry_call_throws(entry)?;
        let defaults = self.copy_default_effects(id, location)?;
        let outer_throws = throws || defaults.raises;
        let chained = if parameters.len() > 1 {
            self.begin_copy_call(outer_throws)?
        } else {
            false
        };
        let made = if chained && parameters.len() > 1 {
            self.copy_defaults_temporaries(id)?
        } else {
            false
        };
        let scope = self.begin_copy_temporaries(chained && made);
        let began =
            parameters.len() > 1 && !self.lifetime.full_expression && defaults.made && outer_throws;
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
            let value = self.constructor_default_value(expression, parameter, location)?;
            values.push(value);
        }
        // call_copy forms the hidden context before demanding the entry and
        // opening a plain copy region. This also fixes table emission order.
        if entry.hidden_vtt(&*self.graph)? {
            values.insert(1, self.hidden_vtt_value()?);
        }
        let chained = if parameters.len() == 1 {
            self.begin_copy_call(outer_throws)?
        } else {
            chained
        };
        // copy_lowering.cpp::call_copy demands the selected entry only;
        // the copy body publishes any vtable its own initialization uses.
        self.emit_address_constructor_call(entry, !throws, &values)?;
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
