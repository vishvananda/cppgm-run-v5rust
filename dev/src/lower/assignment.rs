// Translated from lower/copy_lowering.cpp. See NOTICE.
//! Assignment bodies consume recorded subobject selections in declaration order.
use super::{Error, function::Function, types};
use crate::{
    lowir::{
        model::{Call, Facts, InstructionKind as I, Value},
        vocabulary::{Projection, Type},
    },
    sema::lowering::{AssignmentAction, EntityId, TypeId, TypeView},
};
pub(super) struct DefinitionOutput<'a> {
    pub program: &'a mut crate::lowir::model::Program,
    pub definitions: &'a mut super::definition_order::Definitions,
    pub emitted: &'a mut std::collections::HashSet<EntityId>,
}
pub(super) fn lower_definitions<'a>(
    output: DefinitionOutput<'_>,
    symbols: &mut super::symbols::Symbols<'a, '_>,
    graph: &'a crate::sema::Graph,
    tree: &'a crate::syntax::ast::Tree,
    declarators: crate::syntax::storage::List<crate::syntax::ast::InitDeclarator>,
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    let DefinitionOutput {
        program,
        definitions,
        emitted,
    } = output;
    for init in tree.init_declarators.get(declarators) {
        if init
            .initializer
            .is_none_or(|i| !matches!(i.kind, crate::syntax::ast::InitializerKind::Default))
        {
            continue;
        }
        let entity = graph.lower_declared(init.declarator)?;
        if graph.lower_assignment(entity).is_some()
            && graph.lower_function(entity)?.emission == crate::sema::lowering::Emission::Always
            && emitted.insert(entity)
        {
            let entry = super::construction::Entry::Function(entity);
            super::function::lower(program, symbols, graph, tree, entry, None, budget)?;
            symbols.stream_definition(program, definitions, entry)?;
        }
    }
    Ok(())
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn assignment_this(&mut self, ty: TypeId) -> Result<Value, Error> {
        let slot = self
            .this_slot
            .ok_or(crate::sema::Error::Invariant("assignment this slot"))?;
        Ok(self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), ty)?
            .value)
    }
    pub(super) fn assignment_source(&mut self, function: EntityId) -> Result<Value, Error> {
        let scope = self.graph.lower_function(function)?.scope;
        let (parameter, _, ty) = self
            .graph
            .lower_parameters(scope)
            .next()
            .ok_or(crate::sema::Error::Invariant("assignment source parameter"))?;
        let slot = self
            .locals
            .get(&parameter)
            .copied()
            .ok_or(crate::sema::Error::Invariant("assignment source slot"))?;
        Ok(self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), ty)?
            .value)
    }
    pub(super) fn assignment_offset(&self, action: AssignmentAction) -> Result<u64, Error> {
        Ok(if let Some(member) = action.member {
            if let Some(field) = self.graph.lower_bit_field(member)? {
                field.bit / 8
            } else {
                self.graph.lower_member_offset(member)?
            }
        } else if let Some((class, index)) = action.base {
            self.graph
                .lower_base_initialization_span(
                    class,
                    usize::try_from(index)
                        .map_err(|_| crate::sema::Error::Limit("assignment base limit"))?,
                    false,
                )?
                .0
        } else {
            return Err(
                crate::sema::Error::Invariant("assignment without subobject identity").into(),
            );
        })
    }
    fn assignment_address(
        &mut self,
        object: Value,
        action: AssignmentAction,
    ) -> Result<Value, Error> {
        let offset = self
            .builder
            .integer(u128::from(self.assignment_offset(action)?))?;
        Ok(self
            .emit(
                I::Index(
                    Type::I8,
                    action.member.map(|_| Projection::Field),
                    object,
                    offset,
                ),
                action.ty,
            )?
            .value)
    }
    fn empty_assignment_member(&self, action: AssignmentAction) -> Result<bool, Error> {
        Ok(action.member.is_some()
            && if let TypeView::Class(class) = self.graph.lower_type(action.ty)? {
                self.graph.lower_class_empty(class)?
            } else {
                false
            })
    }
    pub(super) fn assign_members(&mut self, function: EntityId) -> Result<(), Error> {
        let properties = self
            .graph
            .lower_assignment(function)
            .ok_or(crate::sema::Error::Invariant("assignment properties"))?;
        let facts = self.graph.lower_function(function)?;
        let class = facts
            .object
            .ok_or(crate::sema::Error::Invariant("assignment class"))?;
        let ty = self.graph.lower_entity_type(class)?;
        let actions = &properties.actions;
        let transfers: Vec<_> = actions
            .iter()
            .map(|&action| {
                Ok(super::bit_fields::Transfer {
                    member: action.member,
                    ty: action.ty,
                    offset: self.assignment_offset(action)?,
                    trivial: action
                        .function
                        .is_none_or(|f| self.graph.lower_assignment(f).is_some_and(|p| p.trivial)),
                })
            })
            .collect::<Result<_, Error>>()?;
        let mut first = 0;
        while let Some(&action) = actions.get(first) {
            if self.empty_assignment_member(action)?
                || action
                    .function
                    .is_some_and(|f| !self.graph.lower_assignment(f).is_some_and(|p| p.trivial))
            {
                break;
            }
            first += 1;
        }
        if self.graph.lower_polymorphic(class) {
            first = 0;
        }
        let mut field_wise = self.graph.lower_polymorphic(class);
        for action in actions {
            field_wise |= if let Some(member) = action.member {
                self.graph.lower_bit_field(member)?.is_some()
                    && self.graph.lower_local_name(member).is_some()
            } else {
                true
            };
        }
        if first == actions.len() && !field_wise {
            if !self.graph.lower_class_empty(class)? {
                let destination = self.assignment_this(ty)?;
                let source = self.assignment_source(function)?;
                let (size, alignment) = types::layout(self.graph, ty)?;
                self.effect(I::CopyObject(
                    size.min(self.graph.lower_nonvirtual_bytes(class)?),
                    alignment,
                    source,
                    destination,
                ))?;
            }
        } else {
            let prefix = actions
                .get(first)
                .filter(|_| !self.graph.lower_polymorphic(class))
                .copied()
                .map(|a| self.assignment_offset(a))
                .transpose()?
                .unwrap_or(0);
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
            } else {
                first = 0;
            }
            while first < actions.len() {
                let action = actions[first];
                if let Some(member) = action.member
                    && self.graph.lower_bit_field(member)?.is_some()
                {
                    if self.graph.lower_local_name(member).is_some() {
                        first = self.transfer_bit_fields(
                            function,
                            class,
                            &transfers,
                            first,
                            self.graph.lower_nonvirtual_bytes(class)?,
                        )?;
                    }
                } else {
                    self.assign_subobject(function, ty, action)?;
                }
                first += 1;
            }
        }
        let object = self.assignment_this(ty)?;
        self.effect(I::Return(Type::Ptr, Some(object)))?;
        self.terminated = true;
        Ok(())
    }
    fn assign_subobject(
        &mut self,
        function: EntityId,
        ty: TypeId,
        action: AssignmentAction,
    ) -> Result<(), Error> {
        let Some(selected) = action.function else {
            let source = self.assignment_source(function)?;
            let source = self.assignment_address(source, action)?;
            if matches!(self.graph.lower_type(action.ty)?, TypeView::Array { .. }) {
                let object = self.assignment_this(ty)?;
                let object = self.assignment_address(object, action)?;
                let (bytes, alignment) = types::layout(self.graph, action.ty)?;
                self.effect(I::CopyObject(bytes, alignment, source, object))?;
            } else {
                let storage = types::storage(self.graph, action.ty)?;
                let value = self
                    .emit(
                        I::Load(Facts::memory(false, storage.bits() != 8), storage, source),
                        action.ty,
                    )?
                    .value;
                let object = self.assignment_this(ty)?;
                let object = self.assignment_address(object, action)?;
                self.effect(I::Store(
                    Facts::memory(false, storage.bits() != 8),
                    storage,
                    value,
                    object,
                ))?;
            }
            return Ok(());
        };
        let mut element = action.ty;
        let mut count = 1u64;
        while let TypeView::Array {
            element: inner,
            bound,
        } = self.graph.lower_type(element)?
        {
            element = inner;
            count = count.wrapping_mul(bound);
        }
        let TypeView::Class(class) = self.graph.lower_type(element)? else {
            return Err(crate::sema::Error::Invariant("class assignment subobject").into());
        };
        if element == action.ty
            && self.graph.lower_class_empty(class)?
            && self
                .graph
                .lower_assignment(selected)
                .is_some_and(|p| p.trivial)
        {
            if action.member.is_some() {
                self.symbols.demand(selected);
            }
            return Ok(());
        }
        let destination = self.assignment_this(ty)?;
        let destination = self.assignment_address(destination, action)?;
        if element == action.ty {
            let source = self.assignment_source(function)?;
            let source = self.assignment_address(source, action)?;
            self.call_assignment(action, destination, source, ty)?;
        } else {
            let size = types::layout(self.graph, element)?.0;
            if count > 8 {
                let base = self.assignment_source(function)?;
                let source = self.assignment_address(base, action)?;
                self.assign_array_loop(action, element, count, destination, source)?;
            } else {
                let mut source = None;
                for index in 0..count {
                    let to = self.element_address(destination, index, size, element)?;
                    let base = if let Some(source) = source {
                        source
                    } else {
                        let base = self.assignment_source(function)?;
                        let base = self.assignment_address(base, action)?;
                        source = Some(base);
                        base
                    };
                    let from = self.element_address(base, index, size, element)?;
                    self.call_assignment(action, to, from, ty)?;
                }
            }
        }
        Ok(())
    }
    fn assign_array_loop(
        &mut self,
        action: AssignmentAction,
        element: TypeId,
        count: u64,
        destination: Value,
        source: Value,
    ) -> Result<(), Error> {
        use crate::lowir::vocabulary::{Binary, Compare};
        let size = types::layout(self.graph, element)?.0;
        let slot = self.generated_slot("assign_array_index", Type::I64)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, zero, slot))?;
        let condition = self.new_block("assign_array_cond")?;
        let body = self.new_block("assign_array_body")?;
        let end = self.new_block("assign_array_end")?;
        self.jump(condition)?;
        self.place(condition)?;
        let index = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let count = self.builder.integer(u128::from(count))?;
        let test = self
            .emit(I::Compare(Compare::Ult, Type::I64, index, count), element)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.terminated = true;
        self.place(body)?;
        let size = self.builder.integer(u128::from(size))?;
        let to = self
            .emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                element,
            )?
            .value;
        let to = self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), destination, to),
                element,
            )?
            .value;
        let from = self
            .emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                element,
            )?
            .value;
        let from = self
            .emit(
                I::Index(Type::I8, Some(Projection::ArrayElement), source, from),
                element,
            )?
            .value;
        self.call_assignment(action, to, from, element)?;
        let one = self.builder.integer(1)?;
        let next = self
            .emit(
                I::Binary(Binary::Add, Facts::default(), Type::I64, index, one),
                element,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, next, slot))?;
        self.jump(condition)?;
        self.place(end)
    }
    fn call_assignment(
        &mut self,
        action: AssignmentAction,
        destination: Value,
        source: Value,
        ty: TypeId,
    ) -> Result<(), Error> {
        let selected = action.function.ok_or(crate::sema::Error::Invariant(
            "assignment function selection",
        ))?;
        let facts = self.graph.lower_function(selected)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(crate::sema::Error::Invariant("assignment call signature").into());
        };
        let caller_destroyed = matches!(self.graph.lower_type(parameters[0])?, TypeView::Class(_))
            && self.symbols.storage_destructor(parameters[0])?.is_some();
        let began = caller_destroyed && !self.lifetime.full_expression;
        if began {
            self.lifetime.full_expression = true;
            self.lifetime.region_start = super::lifetime::RegionStart::Pending;
            self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Eager;
            self.ensure_insertion()?;
            self.lifetime.expression_start = Some(self.builder.position()?);
        }
        let first = self.lifetime.cleanups.len();
        if caller_destroyed {
            self.open_temporary_region()?;
        }
        let mut source = source;
        if let TypeView::Class(class) = self.graph.lower_type(parameters[0])? {
            let slot = self.generated_slot("argobj", types::storage(self.graph, parameters[0])?)?;
            let object = self.emit(I::Addr(slot), parameters[0])?.value;
            let constructor = action
                .parameter_constructor
                .ok_or(crate::sema::Error::Invariant(
                    "assignment parameter constructor plan",
                ))?;
            let empty = self.graph.lower_class_empty(class)?;
            if !self.graph.lower_constructor_trivial(constructor) || empty && caller_destroyed {
                self.call_copy_constructor(
                    constructor,
                    super::construction::Variant::Complete,
                    object,
                    source,
                )?;
            } else if !empty {
                let (size, alignment) = types::layout(self.graph, parameters[0])?;
                self.effect(I::CopyObject(size, alignment, source, object))?;
            }
            source = if self.graph.lower_direct_class_parameter(class)? {
                slot
            } else {
                object
            };
            if caller_destroyed {
                self.temporary_alive(parameters[0], source)?;
            }
        }
        self.symbols.demand(selected);
        let callee = self.symbols.callee(selected, &mut self.builder)?;
        let arguments = self.builder.arguments(&[destination, source])?;
        self.before_lifetime_call(!facts.nothrow)?;
        let result = self.emit(
            I::Call(Call {
                result: Type::Ptr,
                callee,
                arguments,
                signature: None,
                copy_elision: false,
            }),
            ty,
        )?;
        self.finish_lifetime_call(result, false)?;
        self.destroy_temporaries(first)?;
        self.end_lifetime_expression(began)
    }
}
