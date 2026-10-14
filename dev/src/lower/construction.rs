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
        lowering::{ConstructorId, EntityId, FunctionFacts, TypeId, TypeView},
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
    pub(super) fn default_construct(
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
        let entry = Entry::Constructor(id, Variant::Complete);
        let facts = entry.facts(self.graph)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(facts.ty)? else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "constructor signature missing",
            )));
        };
        let mut values = vec![object];
        for (index, &parameter) in parameters.iter().enumerate() {
            let function =
                entry
                    .entity(self.graph)
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "implicit constructor parameters",
                    )))?;
            let default = self
                .graph
                .lower_default(function, index)
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
    pub(super) fn initialize_constructor(
        &mut self,
        id: ConstructorId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let plan = self.graph.lower_constructor(id);
        if !plan.bases.is_empty()
            || plan
                .subobjects
                .iter()
                .any(|&(_, child)| !self.graph.lower_constructor_trivial(child))
        {
            return Err(Error::Missing("recursive constructor subobject lowering"));
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
        }
        Ok(())
    }
}
