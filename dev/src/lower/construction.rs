// Translated from lower/object_lowering.cpp and program_lowering.cpp. See NOTICE.
//! Constructor entries consume canonical selection and original member actions.
use super::{Error, conversions::Context, function::Function};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Variant {
    Complete,
    Base,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Entry {
    Function(EntityId),
    Constructor(ConstructorId, Variant),
    DestructorBase(EntityId),
}
impl Entry {
    pub(super) fn facts(self, graph: &Graph) -> Result<FunctionFacts, Error> {
        Ok(match self {
            Self::Function(entity) | Self::DestructorBase(entity) => {
                graph.lower_function(entity)?
            }
            Self::Constructor(id, _) => graph.lower_constructor_function(id)?,
        })
    }
    pub(super) fn entity(self, graph: &Graph) -> Option<EntityId> {
        match self {
            Self::Function(entity) | Self::DestructorBase(entity) => Some(entity),
            Self::Constructor(id, _) => graph.lower_constructor_entity(id),
        }
    }
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn construct_local(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        slot: Value,
    ) -> Result<(), Error> {
        let id = self
            .graph
            .lower_construction(entity)
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "object without constructor",
            )))?;
        let object = self.storage_address(slot, ty)?;
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
        let saved = self.lifetime.declared_object.replace(object);
        let result = self.construct_at(id, ty, object, arguments);
        self.lifetime.declared_object = saved;
        result
    }
    pub(super) fn construct_at(
        &mut self,
        id: ConstructorId,
        ty: TypeId,
        object: Value,
        arguments: Option<ConstructorArguments>,
    ) -> Result<(), Error> {
        self.construct_at_qualified(id, ty, object, arguments, false)
    }
    pub(super) fn construct_at_qualified(
        &mut self,
        id: ConstructorId,
        ty: TypeId,
        object: Value,
        arguments: Option<ConstructorArguments>,
        volatile: bool,
    ) -> Result<(), Error> {
        if matches!(arguments, Some(ConstructorArguments::Value)) {
            if self.graph.lower_constructor(id).zero_before_value {
                // Value-initialization zeroes storage before any nontrivial call.
                self.zero_class_object_qualified(ty, object, None, volatile)?;
            }
            if self.graph.lower_constructor_trivial(id) {
                return Ok(());
            }
        }
        self.call_constructor(id, Variant::Complete, object, arguments)
    }
    pub(super) fn call_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        object: Value,
        arguments: Option<ConstructorArguments>,
    ) -> Result<(), Error> {
        self.call_constructor_with_elision(id, variant, object, arguments, false)
    }
    pub(super) fn call_constructor_from_expression(
        &mut self,
        id: ConstructorId,
        object: Value,
        source: ast::ExprId,
        elision: bool,
    ) -> Result<(), Error> {
        self.call_constructor_with_elision(
            id,
            Variant::Complete,
            object,
            Some(ConstructorArguments::Single(source)),
            elision,
        )
    }
    fn call_constructor_with_elision(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        object: Value,
        arguments: Option<ConstructorArguments>,
        copy_elision: bool,
    ) -> Result<(), Error> {
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
        let declared = self.lifetime.declared_object == Some(object);
        if declared {
            self.lifetime.declared_object = None;
        }
        if !declared {
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
        for index in 0..parameters.len().max(written.len()) {
            let parameter = parameters.get(index).copied();
            if parameter.is_none() {
                values.push(self.ellipsis_argument(written[index])?);
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
            let value = if matches!(
                self.graph.lower_type(parameter)?,
                TypeView::Reference { .. }
            ) {
                self.bind_reference(default, parameter, Context::Argument)?
            } else if matches!(self.graph.lower_type(parameter)?, TypeView::Class(_)) {
                self.class_argument(default, parameter)?
            } else {
                let value = self.operand(default, Context::Argument)?;
                self.fit(value, parameter)?.value
            };
            values.push(value);
        }
        let result =
            self.emit_constructor_call_with_elision(entry, facts.nothrow, &values, copy_elision);
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
        self.emit_constructor_call_with_elision(entry, nothrow, values, false)
    }
    fn emit_constructor_call_with_elision(
        &mut self,
        entry: Entry,
        nothrow: bool,
        values: &[Value],
        copy_elision: bool,
    ) -> Result<(), Error> {
        // An aggregate construction's active segment also brackets a
        // nonthrowing call once prior subobjects need destruction.
        self.before_lifetime_call(
            !nothrow
                || self.lifetime.full_expression
                    && self.lifetime.temporary_segment.allows_eager_arguments()
                || self.lifetime.full_expression
                    && self.aggregate_has_members()
                    && self.aggregation.call_scope
                        == super::aggregate_cleanup::CallScope::FullExpression,
        )?;
        let symbol = self.symbols.entry_symbol(self.builder.program(), entry)?;
        self.symbols.demand_entry(entry);
        let arguments = self.builder.arguments(values)?;
        self.effect(I::Call(crate::lowir::model::Call {
            result: Type::Void,
            callee: Value::Symbol(symbol),
            arguments,
            signature: None,
            copy_elision,
        }))?;
        Ok(())
    }
    pub(super) fn forward_constructor(
        &mut self,
        id: ConstructorId,
        object: Value,
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
            let value = if matches!(
                self.graph.lower_type(parameter)?,
                TypeView::Reference { .. }
            ) {
                self.bind_reference(expression, parameter, Context::Argument)?
            } else if matches!(self.graph.lower_type(parameter)?, TypeView::Class(_)) {
                self.class_argument(expression, parameter)?
            } else {
                let value = self.operand(expression, Context::Argument)?;
                self.fit(value, parameter)?.value
            };
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
        self.zero_class_object_qualified(ty, object, span, false)
    }
    pub(super) fn zero_class_object_qualified(
        &mut self,
        ty: TypeId,
        object: Value,
        span: Option<u64>,
        volatile: bool,
    ) -> Result<(), Error> {
        let (size, alignment) = super::types::layout(self.graph, ty)?;
        let size = span.unwrap_or(size);
        if !volatile && self.graph.lower_contiguous_zero(ty)? {
            return self.effect(I::ZeroInit(size, alignment, object));
        }
        let mut offset = 0;
        while offset < size {
            let left = size - offset;
            let (bytes, unit) = if left >= 8 {
                (8, Type::I64)
            } else if left >= 4 {
                (4, Type::I32)
            } else if left >= 2 {
                (2, Type::I16)
            } else {
                (1, Type::I8)
            };
            let address = if offset == 0 {
                object
            } else {
                let index = self.builder.integer(u128::from(offset))?;
                self.emit(I::Index(Type::I8, None, object, index), ty)?
                    .value
            };
            let zero = self.builder.integer(0)?;
            self.effect(I::Store(
                Facts::memory(false, bytes != 1),
                unit,
                zero,
                address,
            ))?;
            offset += bytes;
        }
        Ok(())
    }
    pub(super) fn initialize_class_member(
        &mut self,
        pointer: TypeId,
        ty: TypeId,
        offset: u64,
        constructor: ConstructorId,
        arguments: Option<ConstructorArguments>,
    ) -> Result<(), Error> {
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
        self.construct_at(constructor, ty, object, arguments)
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
        if let Some(delegation) = plan.delegation {
            let first = self.lifetime.cleanups.len();
            let began =
                self.begin_constructor_initializer(delegation.initializer, delegation.constructor)?;
            if began && self.lifetime.temporary_segment.is_eager() && !self.lifetime.tracking.active
            {
                self.open_temporary_region_at(self.lifetime.expression_start)?;
            }
            let object = self.object_pointer(plan.object_pointer)?.value;
            if !self.delegate_copy_storage(delegation.constructor, object, delegation.arguments)? {
                self.call_constructor(
                    delegation.constructor,
                    variant,
                    object,
                    Some(delegation.arguments),
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
        for index in 0..plan.bases.len() {
            self.initialize_base(id, variant, index)?;
        }
        self.initialize_member_plan(id, plan.object_pointer, location)
            .map(|_| ())
    }
}
