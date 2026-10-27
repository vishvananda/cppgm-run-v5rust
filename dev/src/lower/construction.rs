// Translated from lower/object_lowering.cpp and program_lowering.cpp. See NOTICE.
//! Constructor entries consume canonical selection and original member actions.
use super::{Error, conversions::Context, function::Function};
use crate::{
    lowir::{
        model::{InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::{
        Graph,
        lowering::{
            ConstructorArguments, ConstructorId, EntityId, FunctionFacts, TypeId, TypeView,
        },
    },
    syntax::ast,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CopyCall {
    Expression,
    Direct,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Variant {
    Complete,
    Base,
    Deleting,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Entry {
    Function(EntityId),
    InlineCopy(EntityId),
    Constructor(ConstructorId, Variant),
    DestructorBase(EntityId),
    DestructorDeleting(EntityId),
}
impl Entry {
    pub(super) fn facts(self, graph: &Graph) -> Result<FunctionFacts, Error> {
        self.facts_in_mode(graph, false)
    }
    pub(super) fn facts_in_mode(
        self,
        graph: &Graph,
        object_mode: bool,
    ) -> Result<FunctionFacts, Error> {
        Ok(match self {
            Self::Function(entity)
            | Self::InlineCopy(entity)
            | Self::DestructorBase(entity)
            | Self::DestructorDeleting(entity) => {
                graph.lower_function_in_mode(entity, object_mode)?
            }
            Self::Constructor(id, _) => {
                graph.lower_constructor_function_in_mode(id, object_mode)?
            }
        })
    }
    pub(super) fn hidden_vtt(self, graph: &Graph) -> Result<bool, Error> {
        Ok(matches!(
            self,
            Self::Constructor(_, Variant::Base) | Self::DestructorBase(_)
        ) && self
            .facts(graph)?
            .object
            .is_some_and(|class| !graph.lower_virtual_bases(class).is_empty()))
    }
    pub(super) fn entity(self, graph: &Graph) -> Option<EntityId> {
        match self {
            Self::Function(entity)
            | Self::InlineCopy(entity)
            | Self::DestructorBase(entity)
            | Self::DestructorDeleting(entity) => Some(entity),
            Self::Constructor(id, _) => graph.lower_constructor_entity(id),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct ConstructorInput {
    pub arguments: Option<ConstructorArguments>,
    pub location: crate::support::source::SourceOffset,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn construct_local(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        let id = self
            .graph
            .lower_construction(entity)
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "object without constructor",
            )))?;
        let object = self.storage_address(slot, ty)?;
        if self.lifetime.region_start.is_pending() && !self.lifetime.tracking.active {
            self.lifetime.expression_start = Some(self.builder.position()?);
        }
        let arguments = self.graph.lower_constructor_arguments(entity);
        if self.lifetime.full_expression
            && let Some(ConstructorArguments::Values(values)) = arguments
            && self.tree.expr_lists.get(values).len() > 1
        {
            let saved = self.lifetime.expression_start;
            self.lifetime.expression_start = Some(self.builder.position()?);
            for &id in self.tree.expr_lists.get(values) {
                self.begin_tracking(
                    id,
                    super::temporary_scan::ExpressionUse::InitializerArgument,
                )?;
            }
            if !self.lifetime.tracking.active {
                self.lifetime.expression_start = saved;
            }
        }
        let function = self.graph.lower_constructor_entity(id);
        let moving = function
            .map(|function| self.graph.lower_copy_form(function))
            .transpose()?
            .flatten()
            .is_some_and(|copy| copy.moving);
        let written_move = initializer.is_some_and(|initializer| {
            matches!(initializer.kind, ast::InitializerKind::Parenthesized(_))
        }) && function
            .is_some_and(|function| !self.graph.lower_implicit_copy(function) && moving);
        let location = self
            .graph
            .lower_variable(entity)?
            .definition
            .map(|declarator| self.tree.declarators.get(declarator).location)
            .ok_or(crate::sema::Error::Invariant(
                "constructed variable without source",
            ))?;
        if let Some(arguments) = arguments
            && !written_move
            && self.delegate_copy_storage(id, object, arguments, location)?
        {
            return Ok(());
        }
        let saved = self.lifetime.declared_object.replace(object);
        let result = self.construct_at(
            id,
            ty,
            object,
            ConstructorInput {
                arguments,
                location,
            },
        );
        self.lifetime.declared_object = saved;
        result
    }
    pub(super) fn construct_at(
        &mut self,
        id: ConstructorId,
        ty: TypeId,
        object: Value,
        input: ConstructorInput,
    ) -> Result<(), Error> {
        if matches!(input.arguments, Some(ConstructorArguments::Value)) {
            if self.graph.lower_constructor(id).zero_before_value {
                // Value-initialization zeroes storage before any nontrivial call.
                self.zero_class_object(ty, object, None)?;
            }
            if self.graph.lower_constructor_trivial(id) {
                return Ok(());
            }
        }
        self.call_constructor(id, Variant::Complete, object, input)
    }
    pub(super) fn call_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        object: Value,
        input: ConstructorInput,
    ) -> Result<(), Error> {
        self.call_constructor_with_elision(id, variant, object, input, false, true)
    }
    pub(super) fn call_constructor_from_expression(
        &mut self,
        id: ConstructorId,
        object: Value,
        source: ast::ExprId,
        elision: bool,
        call: CopyCall,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        // object_lowering.cpp::call_constructor demands the selected body.
        // Only call_special publishes the class table at the call site;
        // expression copies let the demanded constructor publish its tables.
        self.call_constructor_with_elision(
            id,
            Variant::Complete,
            object,
            ConstructorInput {
                arguments: Some(ConstructorArguments::Single(source)),
                location,
            },
            elision,
            call == CopyCall::Direct,
        )
    }
    fn call_constructor_with_elision(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        object: Value,
        input: ConstructorInput,
        copy_elision: bool,
        publish_table: bool,
    ) -> Result<(), Error> {
        let ConstructorInput {
            arguments,
            location,
        } = input;
        let single;
        let written = match arguments {
            Some(ConstructorArguments::Values(values)) => self.tree.expr_lists.get(values),
            Some(ConstructorArguments::Single(id)) => {
                single = [id];
                &single
            }
            Some(ConstructorArguments::Value) | None => &[],
        };
        let entry = Entry::Constructor(id, variant);
        let facts = entry.facts(self.graph)?;
        let list_call = matches!(arguments, Some(ConstructorArguments::Single(list))
            if self.graph.lower_initializer_array(list).is_some()
                && self.graph.lower_list_constructor(list)? == Some(id));
        // The first-phase list call uses begin_call directly; only
        // call_special marks a declared object's construction boundary.
        let declared = self.lifetime.declared_object == Some(object) && !list_call;
        if self.lifetime.declared_object == Some(object) {
            self.lifetime.declared_object = None;
        }
        if let [source] = written
            && self.elide_conversion_copy(*source, id, object)?
        {
            return Ok(());
        }
        // C++ call_conversion uses begin_call directly. Its selected
        // conversion constructs this destination; it is not another argument
        // temporary for call_special's note_call_arguments query.
        let converting = written.first().is_some_and(|&source| {
            self.converting_source == Some(self.without_parentheses(source))
                && self
                    .graph
                    .lower_class_conversion(source)
                    .is_some_and(|(selected, _)| selected == id)
        });
        if publish_table && !declared && !converting && !list_call {
            self.note_constructor_temporaries(id, written)?;
        }
        self.begin_lifetime_call(!facts.nothrow)?;
        if !facts.nothrow
            && self.lifetime.full_expression
            && let Some(mark) = self.lifetime.call_marks.last_mut()
        {
            mark.declared = declared;
        }
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "constructor signature missing",
            )));
        };
        let mut values = vec![object];
        let mut argument_classes = Vec::new();
        let hidden_vtt = entry.hidden_vtt(self.graph)?;
        if hidden_vtt && !written.is_empty() {
            values.push(self.hidden_vtt_value()?);
        }
        for index in 0..parameters.len().max(written.len()) {
            let parameter = parameters.get(index).copied();
            if parameter.is_none() {
                self.push_ellipsis_argument(written[index], &mut values, &mut argument_classes)?;
                continue;
            }
            let parameter = parameter.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "constructor argument type",
            )))?;
            let function =
                entry
                    .entity(self.graph)
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "implicit constructor parameters",
                    )))?;
            let default = written
                .get(index)
                .copied()
                .or_else(|| self.graph.lower_default(function, index))
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "default constructor without argument",
                )))?;
            let value =
                self.with_source_site((index >= written.len()).then_some(location), |lowering| {
                    Ok(if index == 0 && list_call {
                        let reference = if let TypeView::Reference { element, .. } =
                            lowering.graph.lower_type(parameter)?
                        {
                            Some(element)
                        } else {
                            None
                        };
                        let ty = reference.unwrap_or(parameter);
                        let TypeView::Class(class) = lowering.graph.lower_type(ty)? else {
                            return Err(crate::sema::Error::Invariant(
                                "list constructor parameter",
                            )
                            .into());
                        };
                        let slot = lowering.generated_slot(
                            if reference.is_some() { "arg" } else { "argobj" },
                            super::types::storage(lowering.graph, ty)?,
                        )?;
                        let address = lowering.emit(I::Addr(slot), ty)?.value;
                        lowering.initialize_list_object(default, ty, address, false, None)?;
                        if reference.is_some()
                            || !lowering.graph.lower_direct_class_parameter(class)?
                        {
                            address
                        } else {
                            slot
                        }
                    } else if matches!(
                        lowering.graph.lower_type(parameter)?,
                        TypeView::Reference { .. }
                    ) {
                        lowering.bind_reference(default, parameter, Context::Argument)?
                    } else if matches!(lowering.graph.lower_type(parameter)?, TypeView::Class(_)) {
                        lowering.class_argument(default, parameter)?
                    } else {
                        let value = lowering.operand(default, Context::Argument)?;
                        lowering.fit(value, parameter)?.value
                    })
                })?;
            values.push(value);
        }
        // call_special evaluates omitted arguments before obtaining the hidden
        // VTT. Written arguments retain the earlier VTT evaluation boundary.
        if hidden_vtt && written.is_empty() {
            let vtt = self.hidden_vtt_value()?;
            values.insert(1, vtt);
        }
        let result = self.emit_constructor_call_with_elision(
            entry,
            facts.nothrow,
            &values,
            copy_elision,
            publish_table,
            argument_classes,
        );
        if !facts.nothrow && self.lifetime.full_expression {
            self.lifetime.call_marks.pop();
        }
        result
    }
    pub(super) fn emit_constructor_call(
        &mut self,
        entry: Entry,
        nothrow: bool,
        values: &[Value],
    ) -> Result<(), Error> {
        self.emit_constructor_call_with_elision(entry, nothrow, values, false, true, Vec::new())
    }
    pub(super) fn emit_address_constructor_call(
        &mut self,
        entry: Entry,
        nothrow: bool,
        values: &[Value],
    ) -> Result<(), Error> {
        // object_lowering.cpp::construct_from demands only the constructor.
        // Its body publishes any vtable it needs when that body is lowered.
        self.emit_constructor_call_with_elision(entry, nothrow, values, false, false, Vec::new())
    }
    fn emit_constructor_call_with_elision(
        &mut self,
        entry: Entry,
        nothrow: bool,
        values: &[Value],
        copy_elision: bool,
        publish_table: bool,
        argument_classes: Vec<(u32, crate::lowir::vocabulary::ObjectClasses)>,
    ) -> Result<(), Error> {
        // An aggregate construction's active segment also brackets a
        // nonthrowing call once prior subobjects need destruction.
        let transient = !nothrow
            && !self.lifetime.full_expression
            && !self.lifetime.construction
            && !self.lifetime.region
            && self.aggregation.floor.is_none()
            && !self.storage_region_pending();
        self.before_lifetime_call(
            !nothrow
                || self.lifetime.full_expression
                    && self.lifetime.temporary_segment.allows_eager_arguments()
                || self.lifetime.full_expression
                    && self.aggregate_has_members()
                    && self.aggregation.call_scope
                        == super::aggregate_cleanup::CallScope::FullExpression,
        )?;
        self.symbols.demand_entry(entry);
        if publish_table
            && !self.symbols.host_objects()
            && let Entry::Constructor(id, _) = entry
        {
            self.symbols.construction_table(
                self.builder.program(),
                self.graph.lower_constructor(id).class,
            )?;
        }
        let symbol = self.symbols.entry_symbol(self.builder.program(), entry)?;
        let mut hidden_values;
        let values = if entry.hidden_vtt(self.graph)? {
            let TypeView::Function { parameters, .. } =
                self.graph.lower_type(entry.facts(self.graph)?.ty)?
            else {
                return Err(crate::sema::Error::Invariant("base entry signature").into());
            };
            if values.len() == parameters.len() + 1 {
                hidden_values = values.to_vec();
                hidden_values.insert(1, self.hidden_vtt_value()?);
                &hidden_values
            } else {
                values
            }
        } else {
            values
        };
        let arguments = self.builder.arguments(values)?;
        self.effect(I::Call(crate::lowir::model::Call {
            result: Type::Void,
            callee: Value::Symbol(symbol),
            arguments,
            signature: None,
            copy_elision,
            argument_classes: argument_classes.into_iter().rev().collect(),
        }))?;
        if transient {
            self.close_lifetime_region()?;
        }
        Ok(())
    }
    pub(super) fn forward_constructor(
        &mut self,
        id: ConstructorId,
        object: Value,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let entry = Entry::Constructor(id, Variant::Base);
        let facts = entry.facts(self.graph)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "forwarding without signature",
            )));
        };
        let function = self
            .entity
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "forwarding outside constructor",
            )))?;
        let own = self.graph.lower_function(function)?;
        let mut values = vec![object];
        for (entity, _, ty) in self.graph.lower_parameters(own.scope) {
            let slot = self.locals.get(&entity).copied().ok_or(Error::Semantic(
                crate::sema::Error::Invariant("forwarded parameter without slot"),
            ))?;
            values.push(
                if matches!(self.graph.lower_type(ty)?, TypeView::Class(_)) {
                    slot
                } else {
                    self.read_place(slot, ty)?.value
                },
            );
        }
        let function =
            entry
                .entity(self.graph)
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "forwarded constructor without entity",
                )))?;
        for (index, &parameter) in parameters.iter().enumerate().skip(values.len() - 1) {
            let expression = self
                .graph
                .lower_default(function, index)
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "forwarded default missing",
                )))?;
            let value = self.with_source_site(Some(location), |lowering| {
                Ok(
                    if matches!(
                        lowering.graph.lower_type(parameter)?,
                        TypeView::Reference { .. }
                    ) {
                        lowering.bind_reference(expression, parameter, Context::Argument)?
                    } else if matches!(lowering.graph.lower_type(parameter)?, TypeView::Class(_)) {
                        lowering.class_argument(expression, parameter)?
                    } else {
                        let value = lowering.operand(expression, Context::Argument)?;
                        lowering.fit(value, parameter)?.value
                    },
                )
            })?;
            values.push(value);
        }
        self.emit_constructor_call(entry, facts.nothrow, &values)
    }
    pub(super) fn zero_class_object(
        &mut self,
        ty: TypeId,
        object: Value,
        span: Option<u64>,
    ) -> Result<(), Error> {
        // Preliminary value-initialization covers the complete storage span,
        // including padding. Volatile scalar member writes remain distinct.
        let (size, alignment) = super::types::layout(self.graph, ty)?;
        if self.empty_storage == super::member_initialization::EmptyStorage::Overlapping
            && let TypeView::Class(class) = self.graph.lower_type(ty)?
            && self.graph.lower_class_empty(class)?
        {
            return Ok(());
        }
        self.effect(I::ZeroInit(span.unwrap_or(size), alignment, object))?;
        self.null_data_members(ty, object, 0)
    }
    pub(super) fn initialize_class_member(
        &mut self,
        pointer: TypeId,
        ty: TypeId,
        offset: u64,
        constructor: ConstructorId,
        input: ConstructorInput,
    ) -> Result<(), Error> {
        let ConstructorInput {
            arguments,
            location,
        } = input;
        let trivial = self.graph.lower_constructor_trivial(constructor);
        if arguments.is_none() && trivial {
            return Ok(());
        }
        if matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) {
            if matches!(arguments, Some(ConstructorArguments::Value))
                && self.graph.lower_constructor(constructor).zero_before_value
            {
                let object = self.object_pointer(pointer)?.value;
                let index = self.builder.integer(u128::from(offset))?;
                let object = self
                    .emit(
                        I::Index(Type::I8, Some(Projection::Field), object, index),
                        ty,
                    )?
                    .value;
                self.zero_class_object(ty, object, None)?;
            }
            if trivial {
                return Ok(());
            }
            return self.construct_class_elements(
                ty,
                super::object_arrays::ArrayBase::Member { pointer, offset },
                constructor,
                location,
            );
        }
        let object = self.object_pointer(pointer)?.value;
        let offset = self.builder.integer(u128::from(offset))?;
        let object = self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), object, offset),
                ty,
            )?
            .value;
        self.construct_at(constructor, ty, object, input)
    }
    pub(super) fn initialize_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let plan = self.graph.lower_constructor(id);
        if let Some(actions) = plan.copy {
            return self.copy_members(id, variant, actions);
        }
        self.construction.detached = self
            .lifetime
            .body
            .map(|body| self.body_unwinds(body, &mut false))
            .transpose()?
            .unwrap_or(false);
        for child in plan.subobjects {
            let ty = self.graph.lower_entity_type(child.member)?;
            if matches!(self.graph.lower_type(ty)?, TypeView::Array { .. })
                && !self.graph.lower_constructor_trivial(child.constructor)
                && !Entry::Constructor(child.constructor, Variant::Complete)
                    .facts(self.graph)?
                    .nothrow
            {
                self.construction.detached = true;
                break;
            }
        }
        self.destruction.class = Some(plan.class);
        self.bit_initialization_order = if self.graph.lower_implicit_assignment_demanded(plan.class)
        {
            super::bit_fields::InitializationOrder::AssignmentDemanded
        } else {
            super::bit_fields::InitializationOrder::Ordinary
        };
        if let Some(delegation) = plan.delegation {
            if self.virtual_context.vtt.is_some() {
                self.virtual_context.hidden = Some(0);
            }
            let first = self.lifetime.cleanups.len();
            let began =
                self.begin_constructor_initializer(delegation.initializer, delegation.constructor)?;
            if began && self.lifetime.temporary_segment.is_eager() && !self.lifetime.tracking.active
            {
                self.open_temporary_region_at(self.lifetime.expression_start)?;
            }
            let object = self.object_pointer(plan.object_pointer()?)?.value;
            if !self.delegate_copy_storage(
                delegation.constructor,
                object,
                delegation.arguments,
                location,
            )? {
                self.call_constructor(
                    delegation.constructor,
                    variant,
                    object,
                    ConstructorInput {
                        arguments: Some(delegation.arguments),
                        location,
                    },
                )?;
            }
            if !began || self.lifetime.temporary_segment.is_nothrow() {
                self.close_lifetime_region()?;
            }
            self.destroy_temporaries(first)?;
            self.end_lifetime_expression(began)?;
            self.delegated_object_constructed(self.graph.lower_entity_type(plan.class)?, variant)?;
            return Ok(());
        }
        self.bit_initialization = super::bit_fields::Initialization::default();
        if variant == Variant::Base && !self.graph.lower_constructor_trivial(id) {
            self.bit_initialization.limit = self
                .graph
                .lower_nonvirtual_bytes(plan.class)?
                .wrapping_mul(8);
        }
        if variant != Variant::Base {
            for &(base, offset) in self.graph.lower_virtual_bases(plan.class) {
                if !self.graph.lower_class_empty(base)? {
                    self.bit_initialization.limit =
                        self.bit_initialization.limit.min(offset.wrapping_mul(8));
                }
            }
        }
        self.virtual_context.pending = !self.graph.lower_virtual_bases(plan.class).is_empty();
        if variant != Variant::Base {
            self.initialize_virtual_bases(id, location)?;
        }
        for index in 0..plan.bases.len() {
            if self
                .graph
                .lower_inheritance_edge(plan.class, index)
                .is_some_and(|(_, shared, _)| !shared)
            {
                self.initialize_base(id, variant, index, location)?;
            }
        }
        self.virtual_context.pending = false;
        if self.virtual_context.vtt.is_none() {
            self.virtual_context.this_values.clear();
        }
        self.store_vptr(plan.class, plan.object_pointer()?)?;
        self.initialize_member_plan(id, plan.object_pointer()?, location)
            .map(|_| ())
    }
}
