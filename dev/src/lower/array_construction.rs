// Translated from lower/allocation_lowering.cpp::construct_new_elements. See NOTICE.
//! Each constructor attempt owns its default temporaries and an unwind prefix.
use super::{Error, construction::Variant, function::Function, types};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Binary, Compare, Type},
    },
    sema::lowering::{ConstructorId, EntityId, TypeId},
    syntax::ast,
};

#[derive(Clone, Copy)]
pub(super) struct Construction {
    pub element: TypeId,
    pub elements: Value,
    pub count: Value,
    pub raw: Value,
    pub size: Value,
    pub constructor: ConstructorId,
    pub location: crate::support::source::SourceOffset,
    pub deallocator: Option<EntityId>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn array_constructor_inert(
        &self,
        constructor: ConstructorId,
    ) -> Result<bool, Error> {
        let Some(entity) = self.graph.lower_constructor_entity(constructor) else {
            return Ok(false);
        };
        let Some(body) = self.graph.lower_function_body(entity) else {
            return Ok(false);
        };
        let ast::Body::Parsed(body) = self.tree.bodies.get(body) else {
            return Err(crate::sema::Error::Invariant("unparsed array constructor").into());
        };
        if !body.initializers.is_empty()
            || !matches!(self.tree.statements.get(body.compound).kind,
            ast::StmtKind::Compound(items) if items.is_empty())
        {
            return Ok(false);
        }
        let plan = self.graph.lower_constructor(constructor);
        Ok(plan.delegation.is_none()
            && plan.members.iter().all(|(_, init)| init.is_none())
            && plan.bases.iter().all(|base| {
                base.initializer.is_none()
                    && base
                        .constructor
                        .is_none_or(|id| self.graph.lower_constructor_trivial(id))
            })
            && plan
                .subobjects
                .iter()
                .flatten()
                .all(|child| self.graph.lower_constructor_trivial(child.constructor)))
    }
    pub(super) fn construct_allocated_elements(
        &mut self,
        construction: Construction,
        placement: &[Value],
    ) -> Result<(), Error> {
        let Construction {
            element,
            elements,
            count,
            raw,
            size,
            constructor,
            location,
            deallocator,
        } = construction;
        let slot = self.generated_slot("array_new_index", Type::I64)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, zero, slot))?;
        let condition = self.new_block("array_new_ctor_cond")?;
        let body = self.new_block("array_new_ctor_body")?;
        let end = self.new_block("array_new_ctor_end")?;
        let effects = self.constructor_element_effects(constructor)?;
        let guarded = effects.raises;
        let temporaries = effects.made;
        let cleanup = if guarded {
            Some(self.new_block("array_new_ctor_cleanup")?)
        } else {
            None
        };
        let after = self.new_block("array_new_ctor_cont")?;
        self.jump(condition)?;
        self.place(condition)?;
        let current = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, slot),
                element,
            )?
            .value;
        let test = self
            .emit(I::Compare(Compare::Ult, Type::I64, current, count), element)?
            .value;
        self.effect(I::Branch(test, body, end))?;
        self.place(body)?;
        let object = self.allocated_element_address(element, elements, current)?;
        let transient = guarded
            && !self.lifetime.full_expression
            && !self.lifetime.construction
            && !self.lifetime.region;
        let mut attempt = None;
        if let Some(cleanup) = cleanup {
            self.before_lifetime_call(true)?;
            attempt = self.landing_try();
            self.effect(I::Runtime(R::EhTry(cleanup)))?;
        }
        let scope = self.begin_copy_temporaries(temporaries);
        self.call_constructor(
            constructor,
            Variant::Complete,
            object,
            super::construction::ConstructorInput {
                arguments: None,
                location,
            },
        )?;
        self.end_copy_temporaries(scope)?;
        if guarded {
            self.effect(I::Runtime(R::EhEnd))?;
        }
        if transient {
            self.close_lifetime_region()?;
        }
        let one = self.builder.integer(1)?;
        let next = self
            .emit(
                I::Binary(Binary::Add, Facts::default(), Type::I64, current, one),
                element,
            )?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, next, slot))?;
        self.jump(condition)?;
        self.place(end)?;
        self.jump(after)?;
        if let Some(cleanup) = cleanup {
            self.place(cleanup)?;
            if let Some(attempt) = attempt {
                self.landing_dispatch(attempt)?;
            }
            let constructed = self
                .emit(
                    I::Load(Facts::memory(false, true), Type::I64, slot),
                    element,
                )?
                .value;
            if self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .class_destructor(element)?
                .is_some()
            {
                self.destroy_allocated_prefix(element, elements, constructed)?;
            }
            if let Some(deallocator) = deallocator {
                let arguments =
                    self.array_deallocation_arguments(deallocator, raw, size, placement)?;
                self.deallocate_storage(deallocator, raw, size, Some(arguments), false)?;
            }
            if let Some(attempt) = attempt {
                self.landing_exit(attempt)?;
            } else {
                self.handler_resume()?;
            }
        }
        self.place(after)
    }
    pub(super) fn allocated_element_address(
        &mut self,
        element: TypeId,
        base: Value,
        index: Value,
    ) -> Result<Value, Error> {
        let size = self
            .builder
            .integer(u128::from(types::layout(&*self.graph, element)?.0))?;
        let offset = self
            .emit(
                I::Binary(Binary::Mul, Facts::default(), Type::I64, index, size),
                element,
            )?
            .value;
        Ok(self
            .emit(I::Index(Type::I8, None, base, offset), element)?
            .value)
    }
    fn destroy_allocated_prefix(
        &mut self,
        element: TypeId,
        elements: Value,
        count: Value,
    ) -> Result<(), Error> {
        let slot = self.generated_slot("array_dtor_index", Type::I64)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, count, slot))?;
        self.destroy_allocated_elements(
            element,
            elements,
            slot,
            super::array_deletion::Destruction::ConstructionPrefix,
        )
    }
}
