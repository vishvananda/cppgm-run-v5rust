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
}
impl Entry {
    pub(super) fn facts(self, graph: &Graph) -> Result<FunctionFacts, Error> {
        Ok(match self {
            Self::Function(entity) => graph.lower_function(entity)?,
            Self::Constructor(id, _) => graph.lower_constructor_function(id)?,
        })
    }
    pub(super) fn entity(self, graph: &Graph) -> Option<EntityId> {
        match self {
            Self::Function(entity) => Some(entity),
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
        let object = self.emit(I::Addr(slot), ty)?.value;
        let arguments = self.graph.lower_constructor_arguments(entity);
        if matches!(arguments, Some(ConstructorArguments::Value)) {
            if self.graph.lower_constructor(id).zero_before_value {
                // Value-initialization zeroes storage before any nontrivial call.
                self.zero_class_object(ty, object, None)?;
            }
            if self.graph.lower_constructor_trivial(id) {
                return Ok(());
            }
        }
        self.call_constructor(id, Variant::Complete, object, arguments)
    }
    fn call_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        object: Value,
        arguments: Option<ConstructorArguments>,
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
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "constructor signature missing",
            )));
        };
        let mut values = vec![object];
        for index in 0..parameters.len().max(written.len()) {
            let parameter = parameters.get(index).copied();
            if parameter.is_none() {
                let value = self.operand(written[index], Context::Argument)?;
                values.push(value.value);
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
            } else {
                let value = self.operand(default, Context::Argument)?;
                self.fit(value, parameter)?.value
            };
            values.push(value);
        }
        let symbol = self.symbols.entry_symbol(self.builder.program(), entry)?;
        self.symbols.demand_entry(entry);
        let arguments = self.builder.arguments(&values)?;
        self.effect(I::Call(crate::lowir::model::Call {
            result: Type::Void,
            callee: Value::Symbol(symbol),
            arguments,
            signature: None,
            copy_elision: false,
        }))?;
        Ok(())
    }
    fn zero_class_object(
        &mut self,
        ty: TypeId,
        object: Value,
        span: Option<u64>,
    ) -> Result<(), Error> {
        let (size, alignment) = super::types::layout(self.graph, ty)?;
        let size = span.unwrap_or(size);
        if self.graph.lower_contiguous_zero(ty)? {
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
    pub(super) fn initialize_constructor(
        &mut self,
        id: ConstructorId,
        variant: Variant,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let plan = self.graph.lower_constructor(id);
        if plan
            .subobjects
            .iter()
            .any(|&(_, child)| !self.graph.lower_constructor_trivial(child))
        {
            return Err(Error::Missing("recursive constructor subobject lowering"));
        }
        self.bit_initialization = super::bit_fields::Initialization::default();
        if variant == Variant::Base && !self.graph.lower_constructor_trivial(id) {
            self.bit_initialization.limit = self
                .graph
                .lower_nonvirtual_bytes(plan.class)?
                .wrapping_mul(8);
        }
        for (index, base) in plan.bases.iter().enumerate() {
            let (offset, empty, span) = self.graph.lower_base_initialization_span(
                plan.class,
                index,
                variant == Variant::Base,
            )?;
            let trivial = base
                .constructor
                .is_none_or(|id| self.graph.lower_constructor_trivial(id));
            let value = matches!(base.arguments, Some(ConstructorArguments::Value))
                || base.initializer.is_some_and(|initializer| {
                    if let ast::InitializerKind::Braced(id) = initializer.kind
                        && let ast::ExprKind::Braced { elements, .. } =
                            self.tree.expressions.get(id).kind
                    {
                        return self.tree.expr_lists.get(elements).is_empty();
                    }
                    false
                });
            if base.initializer.is_none() && trivial || empty && value && trivial {
                continue;
            }
            self.bit_initialization.high = self.bit_initialization.high.max(
                offset
                    .wrapping_add(self.graph.lower_nonvirtual_bytes(base.class)?)
                    .wrapping_mul(8),
            );
            let ty = self.graph.lower_entity_type(base.class)?;
            let object = self.object_pointer(plan.object_pointer)?.value;
            let object = self.base_projection(object, ty, offset)?;
            if let Some(constructor) = base.constructor {
                if value {
                    if !empty && self.graph.lower_constructor(constructor).zero_before_value {
                        self.zero_class_object(ty, object, Some(span))?;
                    }
                    if trivial {
                        continue;
                    }
                }
                self.call_constructor(constructor, Variant::Base, object, base.arguments)?;
            } else {
                let initializer =
                    base.initializer
                        .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                            "aggregate base without initializer",
                        )))?;
                self.constructor_aggregate_base(ty, object, initializer)?;
            }
        }
        for &(member, initializer) in plan.members {
            let Some(initializer) = initializer else {
                continue;
            };
            let ty = self.graph.lower_entity_type(member)?;
            if matches!(
                self.graph.lower_type(ty)?,
                TypeView::Class(_) | TypeView::Array { .. }
            ) {
                self.constructor_aggregate_member(
                    member,
                    plan.object_pointer,
                    initializer,
                    plan.union,
                    location,
                )?;
                self.bit_initialization.unit = 0;
                self.bit_initialization.high = self.bit_initialization.high.max(
                    self.graph
                        .lower_member_offset(member)?
                        .wrapping_add(super::types::layout(self.graph, ty)?.0)
                        .wrapping_mul(8),
                );
                continue;
            }
            let single = match initializer.kind {
                ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                    if let ast::ExprKind::Braced { elements, .. } =
                        self.tree.expressions.get(id).kind
                    {
                        if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
                            Some(self.initializer_expression(initializer)?)
                        } else {
                            self.tree.expr_lists.get(elements).first().copied()
                        }
                    } else {
                        Some(id)
                    }
                }
                ast::InitializerKind::Parenthesized(arguments) => {
                    self.tree.expr_lists.get(arguments).first().copied()
                }
                _ => {
                    return Err(Error::Semantic(crate::sema::Error::Invariant(
                        "constructor initializer action",
                    )));
                }
            };
            if self.graph.lower_bit_field(member)?.is_some() {
                let field = self.field(member)?;
                let value = if let Some(single) = single {
                    let value = self.operand(single, Context::MemberInitialization)?;
                    self.fit(value, ty)?
                } else {
                    self.zero(ty)?
                };
                let object = self.object_pointer(plan.object_pointer)?.value;
                let offset = self.builder.integer(u128::from(field.offset))?;
                let address = self
                    .emit(
                        I::Index(Type::I8, Some(Projection::Field), object, offset),
                        ty,
                    )?
                    .value;
                self.initialize_field_value(
                    field,
                    super::bit_fields::Unit {
                        address,
                        volatile: self.graph.lower_cv(ty) & 2 != 0,
                    },
                    value,
                )?;
                let layout = self
                    .graph
                    .lower_bit_field(member)?
                    .ok_or(Error::Missing("constructor field layout"))?;
                self.bit_initialization.high = self
                    .bit_initialization
                    .high
                    .max(layout.bit.wrapping_add(layout.width));
                continue;
            }
            self.bit_initialization.unit = 0;
            let value = if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
                self.bind_reference(
                    single.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "reference member without source",
                    )))?,
                    ty,
                    Context::Initialization,
                )?
            } else if let Some(single) = single {
                let value = self.operand(single, Context::Initialization)?;
                self.fit(value, ty)?.value
            } else {
                self.zero(ty)?.value
            };
            let object = self.object_pointer(plan.object_pointer)?.value;
            let offset = self
                .builder
                .integer(u128::from(self.graph.lower_member_offset(member)?))?;
            let address = self
                .emit(
                    I::Index(Type::I8, Some(Projection::Field), object, offset),
                    ty,
                )?
                .value;
            self.effect(I::Store(
                Facts::memory(
                    self.graph.lower_cv(ty) & 2 != 0,
                    self.graph.lower_cv(ty) & 2 == 0
                        && !plan.union
                        && !matches!(super::types::storage(self.graph, ty)?, Type::I8 | Type::U8),
                ),
                super::types::storage(self.graph, ty)?,
                value,
                address,
            ))?;
            self.bit_initialization.high = self.bit_initialization.high.max(
                self.graph
                    .lower_member_offset(member)?
                    .wrapping_add(super::types::layout(self.graph, ty)?.0)
                    .wrapping_mul(8),
            );
        }
        Ok(())
    }
}
