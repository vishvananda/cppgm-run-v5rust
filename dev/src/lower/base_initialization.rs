// Translated from lower/object_lowering.cpp::construct_base/call_special. See NOTICE.
//! Base entries consume selected transfers and own each initializer's temporaries.
use super::{Error, construction::Variant, conversions::Context, function::Function, types};
use crate::support::source::SourceOffset;
use crate::{
    lowir::model::{InstructionKind as I, Value},
    sema::lowering::{ConstructorArguments, ConstructorId, CopyAction, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy)]
enum SharedPlan {
    Direct(usize),
    Indirect(usize),
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn initialize_virtual_bases(
        &mut self,
        owner: ConstructorId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        // Plan order governs tail occupancy; construction order is a separate
        // projection. Index each shared occurrence once for this ABI entry.
        let mut selected = crate::support::hash::HashMap::default();
        for (index, base) in self.graph.lower_constructor(owner).bases.iter().enumerate() {
            if self
                .graph
                .lower_inheritance_edge(self.graph.lower_constructor(owner).class, index)
                .is_some_and(|(_, shared, _)| shared)
            {
                selected.insert(base.class, SharedPlan::Direct(index));
            }
        }
        for (index, base) in self
            .graph
            .lower_constructor(owner)
            .indirect_bases
            .iter()
            .enumerate()
        {
            selected.insert(base.class, SharedPlan::Indirect(index));
        }
        for base in self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .virtual_construction_order(self.graph.lower_constructor(owner).class)
        {
            match selected.get(&base).copied() {
                Some(SharedPlan::Direct(index)) => {
                    self.initialize_base(owner, Variant::Complete, index, location)?;
                }
                Some(SharedPlan::Indirect(index)) => {
                    self.initialize_indirect_base(owner, index, location)?;
                }
                None => {}
            }
        }
        Ok(())
    }

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
        location: SourceOffset,
    ) -> Result<(), Error> {
        let saved = self.virtual_context.hidden;
        let result = self.initialize_base_plan(owner, variant, index, false, location);
        self.virtual_context.hidden = saved;
        result
    }
    pub(super) fn initialize_indirect_base(
        &mut self,
        owner: ConstructorId,
        index: usize,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let saved = self.virtual_context.hidden;
        let result = self.initialize_base_plan(owner, Variant::Complete, index, true, location);
        self.virtual_context.hidden = saved;
        result
    }
    fn initialize_base_plan(
        &mut self,
        owner: ConstructorId,
        variant: Variant,
        index: usize,
        indirect: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let base = if indirect {
            self.graph.lower_constructor(owner).indirect_bases[index]
        } else {
            self.graph.lower_constructor(owner).bases[index]
        };
        let shared = indirect
            || self
                .graph
                .lower_inheritance_edge(self.graph.lower_constructor(owner).class, index)
                .is_some_and(|(_, shared, _)| shared);
        let identity = Some(if shared {
            super::destruction::BaseSubobject::Virtual(base.class)
        } else {
            super::destruction::BaseSubobject::Direct(
                u32::try_from(index).map_err(|_| crate::lowir::Error::Limit)?,
            )
        });
        if variant == Variant::Base && shared {
            return Ok(());
        }
        let (offset, empty, span) = if indirect {
            self.indirect_initialization_span(owner, index)?
        } else {
            self.graph.lower_base_initialization_span(
                self.graph.lower_constructor(owner).class,
                index,
                variant == Variant::Base && !self.graph.lower_constructor_trivial(owner),
            )?
        };
        if !self.graph.lower_virtual_bases(base.class).is_empty() {
            self.virtual_context.hidden = Some(if shared {
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .vtt_virtual_index(self.graph.lower_constructor(owner).class, base.class)?
            } else {
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .vtt_base_index(self.graph.lower_constructor(owner).class, index)?
            });
        }
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
            return self.subobject_constructed(ty, offset, identity);
        }
        self.bit_initialization.high = self.bit_initialization.high.max(
            offset
                .wrapping_add(self.graph.lower_nonvirtual_bytes(base.class)?)
                .wrapping_mul(8),
        );
        if base.forwards {
            let object = self
                .object_pointer(self.graph.lower_constructor(owner).object_pointer()?)?
                .value;
            let object = self.base_projection(object, ty, offset)?;
            self.forward_constructor(
                base.constructor.ok_or(crate::sema::Error::Invariant(
                    "forwarded base without constructor",
                ))?,
                object,
                location,
            )?;
            return self.subobject_constructed(ty, offset, identity);
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
        let object = self
            .object_pointer(self.graph.lower_constructor(owner).object_pointer()?)?
            .value;
        let object = self.base_projection(object, ty, offset)?;
        if let Some(constructor) = base.constructor {
            if value && !empty && self.graph.lower_constructor(constructor).zero_before_value {
                self.zero_class_object(ty, object, Some(span))?;
            }
            if !value || !trivial {
                self.call_constructor(
                    constructor,
                    Variant::Base,
                    object,
                    super::construction::ConstructorInput {
                        arguments: base.arguments,
                        location,
                    },
                )?;
            }
        } else if let Some(source) = source.filter(|_| copy.is_some()) {
            self.initialize_base_copy(source, ty, object, copy, span, location)?;
        } else if let Some(source) = source.filter(|_| prvalue) {
            let (size, alignment) = types::layout(&*self.graph, ty)?;
            if span < size
                && !empty
                && self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .storage_destructor(ty)?
                    .is_none()
            {
                let slot = self.generated_slot("baseobj", types::storage(&*self.graph, ty)?)?;
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
        if !self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .host_objects()
            && (self
                .graph
                .lower_polymorphic(self.graph.lower_constructor(owner).class)
                && (!self.graph.lower_polymorphic(base.class)
                    || self.graph.lower_virtual_destructor(base.class))
                || !self
                    .graph
                    .lower_virtual_bases(self.graph.lower_constructor(owner).class)
                    .is_empty())
            && let Some(constructor) =
                constructor.filter(|id| self.graph.lower_user_provided_constructor(*id))
        {
            self.symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .demand_entry(super::construction::Entry::Constructor(
                    constructor,
                    Variant::Complete,
                ));
        }
        self.subobject_constructed(ty, offset, identity)
    }

    fn indirect_initialization_span(
        &self,
        owner: ConstructorId,
        index: usize,
    ) -> Result<(u64, bool, u64), Error> {
        let base = self.graph.lower_constructor(owner).indirect_bases[index].class;
        let (offset, empty, size) = self
            .graph
            .lower_indirect_base_layout(self.graph.lower_constructor(owner).class, base)?;
        let data = self.graph.lower_nonvirtual_bytes(base)?;
        let begin = offset.wrapping_add(data);
        let end = offset.wrapping_add(size);
        let mut occupied = !self.graph.lower_virtual_bases(base).is_empty();
        // The C++ plan places indirect virtual entries after direct bases
        // and members. Occupancy is plan order, independently of execution.
        for i in 0..self.graph.lower_constructor(owner).bases.len() {
            let (class, _, from) = self
                .graph
                .lower_inheritance_edge(self.graph.lower_constructor(owner).class, i)
                .ok_or(crate::sema::Error::Invariant(
                    "constructor plan without direct base",
                ))?;
            let from = from.ok_or(crate::sema::Error::Invariant(
                "constructor base before layout",
            ))?;
            let to = from.wrapping_add(self.graph.lower_nonvirtual_bytes(class)?);
            occupied |= from < end && to > begin;
        }
        for &(member, _) in self.graph.lower_constructor(owner).members {
            let (from, to) = if let Some(field) = self.graph.lower_bit_field(member)? {
                (
                    field.bit / 8,
                    field.bit.wrapping_add(field.width).wrapping_add(7) / 8,
                )
            } else {
                let from = self.graph.lower_member_offset(member)?;
                let size = types::layout(&*self.graph, self.graph.lower_entity_type(member)?)?.0;
                (from, from.wrapping_add(size))
            };
            occupied |= from < end && to > begin;
        }
        for prior in &self.graph.lower_constructor(owner).indirect_bases[..index] {
            let (from, _, _) = self.graph.lower_indirect_base_layout(
                self.graph.lower_constructor(owner).class,
                prior.class,
            )?;
            let to = from.wrapping_add(self.graph.lower_nonvirtual_bytes(prior.class)?);
            occupied |= from < end && to > begin;
        }
        Ok((offset, empty, if occupied { data } else { size }))
    }

    fn initialize_base_copy(
        &mut self,
        source: ast::ExprId,
        ty: TypeId,
        object: Value,
        copy: Option<CopyAction>,
        span: u64,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let reference = match copy {
            Some(CopyAction::Constructor { constructor, .. }) => {
                if !self.graph.lower_constructor_trivial(constructor)
                    || self
                        .graph
                        .lower_class_empty(self.graph.lower_constructor(constructor).class)?
                        && self
                            .symbols
                            .view(&*self.graph, &*self.tree, self.cursor)
                            .storage_destructor(ty)?
                            .is_some()
                {
                    return self.call_constructor(
                        constructor,
                        Variant::Base,
                        object,
                        super::construction::ConstructorInput {
                            arguments: Some(ConstructorArguments::Single(source)),
                            location,
                        },
                    );
                }
                let signature = self.graph.lower_constructor_function(constructor)?.ty;
                let TypeView::Function { parameters, .. } = self.graph.lower_type(signature)?
                else {
                    return Err(crate::sema::Error::Invariant("base copy signature").into());
                };
                let parameters = parameters.to_vec();
                let parameters = parameters.as_slice();
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
                types::layout(&*self.graph, ty)?.1,
                source,
                object,
            ))?;
        }
        Ok(())
    }
}
