// Translated from lower/object_lowering.cpp::construct_base/call_special. See NOTICE.
//! Base entries consume selected transfers and own each initializer's temporaries.
use super::{Error, construction::Variant, conversions::Context, function::Function, types};
use crate::{
    lowir::model::{InstructionKind as I, Value},
    sema::lowering::{ConstructorArguments, ConstructorId, CopyAction, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
    fn base_source(&self, initializer: ast::Initializer) -> Option<ast::ExprId> {
        let values = match initializer.kind {
            ast::InitializerKind::Parenthesized(values) => values,
            ast::InitializerKind::Braced(id) => {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
                    return None;
                };
                elements
            }
            ast::InitializerKind::Assign(id) => return Some(id),
            _ => return None,
        };
        if let [source] = self.tree.expr_lists.get(values) {
            Some(self.without_parentheses(*source))
        } else {
            None
        }
    }

    pub(super) fn initialize_base(
        &mut self,
        owner: ConstructorId,
        variant: Variant,
        index: usize,
    ) -> Result<(), Error> {
        let plan = self.graph.lower_constructor(owner);
        let base = plan.bases[index];
        let (offset, empty, span) = self.graph.lower_base_initialization_span(
            plan.class,
            index,
            variant == Variant::Base && !self.graph.lower_constructor_trivial(owner),
        )?;
        let ty = self.graph.lower_entity_type(base.class)?;
        let trivial = base
            .constructor
            .is_none_or(|id| self.graph.lower_constructor_trivial(id));
        let value = matches!(base.arguments, Some(ConstructorArguments::Value))
            || base.initializer.is_some_and(|initializer| {
                if let ast::InitializerKind::Braced(id) = initializer.kind
                    && let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind
                {
                    return self.tree.expr_lists.get(elements).is_empty();
                }
                false
            });
        if !base.forwards && (base.initializer.is_none() && trivial || empty && value && trivial) {
            return self.subobject_constructed(ty, offset, false);
        }
        self.bit_initialization.high = self.bit_initialization.high.max(
            offset
                .wrapping_add(self.graph.lower_nonvirtual_bytes(base.class)?)
                .wrapping_mul(8),
        );
        if base.forwards {
            let object = self.object_pointer(plan.object_pointer)?.value;
            let object = self.base_projection(object, ty, offset)?;
            self.forward_constructor(
                base.constructor.ok_or(crate::sema::Error::Invariant(
                    "forwarded base without constructor",
                ))?,
                object,
            )?;
            return self.subobject_constructed(ty, offset, false);
        }
        let source = base
            .initializer
            .and_then(|written| self.base_source(written));
        let prvalue = if let Some(source) = source {
            let facts = self.graph.lower_expression(source)?;
            !facts.is_place
                && matches!(self.graph.lower_type(facts.ty)?,
                TypeView::Class(class) if class == base.class)
        } else {
            false
        };
        let copy = source
            .map(|source| self.graph.lower_class_copy(source))
            .transpose()?
            .flatten();
        let constructor = base.constructor.or(match copy {
            Some(CopyAction::Constructor { constructor, .. }) => Some(constructor),
            _ => None,
        });
        let first = self.lifetime.cleanups.len();
        let region = if let Some(written) = base.initializer {
            self.begin_subobject_initializer(written, constructor)?
        } else {
            false
        };
        let comma = if let Some(source) = source {
            matches!(self.tree.expressions.get(source).kind,
                ExprKind::Binary { op, .. }
                if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Comma))
                && !self.graph.lower_operator_call(source)?
        } else {
            false
        };
        if region && self.lifetime.temporary_segment.is_eager() && !self.lifetime.tracking.active {
            if prvalue && copy.is_none() && !comma {
                // The prvalue constructs the base itself. Its own call's
                // operands decide whether an eager region is needed. A
                // comma's left effects already belong to the initializer.
                self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Hidden;
            } else {
                self.open_temporary_region_at(self.lifetime.expression_start)?;
            }
        }
        let object = self.object_pointer(plan.object_pointer)?.value;
        let object = self.base_projection(object, ty, offset)?;
        if let Some(constructor) = base.constructor {
            if value && !empty && self.graph.lower_constructor(constructor).zero_before_value {
                self.zero_class_object(ty, object, Some(span))?;
            }
            if !value || !trivial {
                self.call_constructor(constructor, Variant::Base, object, base.arguments)?;
            }
        } else if let Some(source) = source.filter(|_| copy.is_some()) {
            self.initialize_base_copy(source, ty, object, copy, span)?;
        } else if let Some(source) = source.filter(|_| prvalue) {
            let (size, alignment) = types::layout(self.graph, ty)?;
            if span < size && !empty && self.symbols.storage_destructor(ty)?.is_none() {
                let slot = self.generated_slot("baseobj", types::storage(self.graph, ty)?)?;
                let made = self.emit(I::Addr(slot), ty)?.value;
                self.materialize_class(source, ty, made)?;
                self.effect(I::CopyObject(span, alignment, made, object))?;
            } else {
                self.materialize_class(source, ty, object)?;
            }
        } else {
            self.constructor_aggregate_base(
                ty,
                object,
                base.initializer.ok_or(crate::sema::Error::Invariant(
                    "aggregate base without initializer",
                ))?,
            )?;
        }
        if !region || self.lifetime.temporary_segment.is_nothrow() {
            self.close_lifetime_region()?;
        }
        self.destroy_temporaries(first)?;
        self.end_lifetime_expression(region)?;
        // object_lowering.cpp::construct_base's course entry demand.
        if (self.graph.lower_polymorphic(plan.class)
            && (!self.graph.lower_polymorphic(base.class)
                || self.graph.lower_virtual_destructor(base.class))
            || !self.graph.lower_virtual_bases(plan.class).is_empty())
            && let Some(constructor) =
                constructor.filter(|id| self.graph.lower_user_provided_constructor(*id))
        {
            self.symbols
                .demand_entry(super::construction::Entry::Constructor(
                    constructor,
                    Variant::Complete,
                ));
        }
        self.subobject_constructed(ty, offset, false)
    }

    fn initialize_base_copy(
        &mut self,
        source: ast::ExprId,
        ty: TypeId,
        object: Value,
        copy: Option<CopyAction>,
        span: u64,
    ) -> Result<(), Error> {
        let reference = match copy {
            Some(CopyAction::Constructor { constructor, .. }) => {
                let plan = self.graph.lower_constructor(constructor);
                if !self.graph.lower_constructor_trivial(constructor)
                    || self.graph.lower_class_empty(plan.class)?
                        && self.symbols.storage_destructor(ty)?.is_some()
                {
                    return self.call_constructor(
                        constructor,
                        Variant::Base,
                        object,
                        Some(ConstructorArguments::Single(source)),
                    );
                }
                let signature = self.graph.lower_constructor_function(constructor)?.ty;
                let TypeView::Function { parameters, .. } = self.graph.lower_type(signature)?
                else {
                    return Err(crate::sema::Error::Invariant("base copy signature").into());
                };
                parameters[0]
            }
            Some(CopyAction::Storage { reference, .. }) => reference,
            None => return Err(crate::sema::Error::Invariant("base copy selection missing").into()),
        };
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("base copy without class").into());
        };
        let empty = self.graph.lower_class_empty(class)?;
        if empty && self.side_effect_free(source, false, 0)? {
            return Ok(());
        }
        let source = self.bind_reference(source, reference, Context::Argument)?;
        if !empty {
            self.effect(I::CopyObject(
                span.min(self.graph.lower_nonvirtual_bytes(class)?),
                types::layout(self.graph, ty)?.1,
                source,
                object,
            ))?;
        }
        Ok(())
    }
}
