// Translated from lower/object_lowering.cpp::materialize_prvalue/temporary_object.
// See NOTICE.
//! Original class expressions initialize their destination without an intermediate copy.
use super::{Error, construction::CopyCall, conversions::Context, function::Function, types};
use crate::{
    lowir::model::{InstructionKind as I, Value},
    sema::lowering::{CallTarget, ClassInitialization, CopyAction, EntityId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy)]
pub(super) enum TemporaryUse {
    Argument,
    Reference,
    Designator,
    Discard,
}
impl TemporaryUse {
    fn stem(self) -> &'static str {
        match self {
            Self::Argument => "arg",
            Self::Reference | Self::Designator => "tmpobj",
            Self::Discard => "discard",
        }
    }
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn push_call_arguments(
        &mut self,
        target: crate::sema::lowering::CallTarget,
        parameters: &[TypeId],
        supplied: &[ast::ExprId],
        variadic: bool,
        values: &mut Vec<Value>,
        location: crate::support::source::SourceOffset,
    ) -> Result<Vec<(u32, crate::lowir::vocabulary::ObjectClasses)>, Error> {
        for (index, &parameter) in parameters.iter().enumerate() {
            let argument = supplied
                .get(index)
                .copied()
                .or_else(|| match target {
                    CallTarget::Direct(entity) => self.graph.lower_default(entity, index),
                    CallTarget::Indirect(_, _) => None,
                })
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "call argument missing",
                )))?;
            self.with_source_site((index >= supplied.len()).then_some(location), |lowering| {
                if matches!(
                    lowering.graph.lower_type(parameter)?,
                    TypeView::Reference { .. }
                ) {
                    values.push(lowering.bind_reference(argument, parameter, Context::Argument)?);
                } else if matches!(lowering.graph.lower_type(parameter)?, TypeView::Class(_)) {
                    values.push(lowering.class_argument(argument, parameter)?);
                } else {
                    // argument() applies its recorded standard sequence to the
                    // raw rvalue; operand()'s member-address folding belongs to
                    // expression operands, not this call boundary.
                    let value = if lowering
                        .graph
                        .lower_function_conversion(argument)?
                        .is_some()
                    {
                        lowering.operand(argument, Context::Argument)?
                    } else {
                        let value = lowering.expression(argument, Context::Argument)?;
                        lowering.apply_conversion(argument, value, Context::Argument)?
                    };
                    let value = lowering.fit(value, parameter)?;
                    values.push(value.value);
                }
                Ok(())
            })?;
        }
        let mut argument_classes = Vec::new();
        if variadic {
            for &argument in supplied.iter().skip(parameters.len()) {
                self.push_ellipsis_argument(argument, values, &mut argument_classes)?;
            }
        }
        Ok(argument_classes)
    }

    // expression_lowering.cpp::push_converted's null-parameter class path:
    // The caller owns the copy; host objects pass direct classes as bytes.
    // Classification belongs to this call, including across nested calls.
    pub(super) fn push_ellipsis_argument(
        &mut self,
        id: ast::ExprId,
        values: &mut Vec<Value>,
        classes: &mut Vec<(u32, crate::lowir::vocabulary::ObjectClasses)>,
    ) -> Result<(), Error> {
        let (value, classification) = self.ellipsis_argument(id)?;
        if let Some(classification) = classification {
            let classification = match classification {
                crate::lowir::vocabulary::ObjectClasses::Direct(first, None) => {
                    crate::lowir::vocabulary::ObjectClasses::Direct(
                        first,
                        Some(crate::lowir::vocabulary::EightbyteClass::None),
                    )
                }
                other => other,
            };
            classes.push((
                u32::try_from(values.len()).map_err(|_| crate::lowir::Error::Limit)?,
                classification,
            ));
        }
        values.push(value);
        Ok(())
    }
    fn ellipsis_argument(
        &mut self,
        id: ast::ExprId,
    ) -> Result<(Value, Option<crate::lowir::vocabulary::ObjectClasses>), Error> {
        let facts = self.graph.lower_expression(id)?;
        if self.graph.lower_function_conversion(id)?.is_some()
            || !matches!(self.graph.lower_type(facts.ty)?, TypeView::Class(_))
        {
            let value = self.operand(id, super::conversions::Context::Argument)?;
            let classes = if self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .object_mode()
                && matches!(
                    value.ty,
                    crate::lowir::vocabulary::Type::Object { .. }
                        | crate::lowir::vocabulary::Type::I128
                ) {
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .object_argument_classes(value.source)?
            } else {
                None
            };
            let value = if value.ty == crate::lowir::vocabulary::Type::I128
                && self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .object_mode()
            {
                let ty = crate::lowir::vocabulary::Type::object(
                    16,
                    if self.graph.lower_member_pointer_type(facts.ty).is_some() {
                        3
                    } else {
                        4
                    },
                );
                let slot = self.generated_slot("arg", ty)?;
                let object = self.emit(I::Addr(slot), facts.ty)?.value;
                self.effect(I::Store(
                    crate::lowir::model::Facts::memory(false, true),
                    value.ty,
                    value.value,
                    object,
                ))?;
                slot
            } else {
                value.value
            };
            return Ok((value, classes));
        }
        let TypeView::Class(class) = self.graph.lower_type(facts.ty)? else {
            return Err(crate::sema::Error::Invariant("ellipsis class argument").into());
        };
        let ty = facts.ty;
        let slot = self.generated_slot("arg", types::storage(&*self.graph, ty)?)?;
        let object = self.emit(I::Addr(slot), ty)?.value;
        if facts.is_place {
            let source = self.address(id)?;
            if !self.graph.lower_class_empty(class)? {
                self.copy_object(ty, source, object)?;
            }
        } else {
            self.materialize_class(id, ty, object)?;
        }
        if self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .object_mode()
            && self.graph.lower_direct_class_parameter(class)?
        {
            Ok((
                slot,
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .object_argument_classes(ty)?,
            ))
        } else {
            Ok((object, None))
        }
    }
    pub(super) fn named_elided_list(
        &self,
        initializer: Option<ast::Initializer>,
    ) -> Result<Option<ast::ExprId>, Error> {
        let Some(ast::Initializer {
            kind: ast::InitializerKind::Assign(expr),
            ..
        }) = initializer
        else {
            return Ok(None);
        };
        let expr = self.without_parentheses(expr);
        let ExprKind::FunctionalCast {
            braced: Some(list), ..
        } = self.tree.expressions.get(expr).kind
        else {
            return Ok(None);
        };
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Ok(None);
        };
        if !self.tree.expr_lists.get(elements).is_empty()
            && matches!(
                self.graph.lower_prvalue_initialization(expr)?,
                Some(ClassInitialization::Aggregate(_) | ClassInitialization::FlexibleAggregate(_))
            )
        {
            return Ok(Some(list));
        }
        Ok(None)
    }
    pub(super) fn materialize_member_expression(
        &mut self,
        member: EntityId,
        pointer: TypeId,
        initializer: ast::Initializer,
        union: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<bool, Error> {
        let ty = self.graph.lower_entity_type(member)?;
        if !matches!(self.graph.lower_type(ty)?, TypeView::Class(_)) {
            return Ok(false);
        }
        let expr = match initializer.kind {
            ast::InitializerKind::Assign(expr)
                if !matches!(
                    self.tree.expressions.get(expr).kind,
                    ExprKind::Braced { .. }
                ) =>
            {
                expr
            }
            ast::InitializerKind::Parenthesized(args)
                if self.tree.expr_lists.get(args).len() == 1 =>
            {
                self.tree.expr_lists.get(args)[0]
            }
            _ => return Ok(false),
        };
        if !matches!(
            self.graph
                .lower_type(self.graph.lower_expression(expr)?.ty)?,
            TypeView::Class(_)
        ) {
            return Ok(false);
        }
        // The constructor owner registers this completed member with its
        // subobject cleanup after materialization succeeds.
        let object = self.object_pointer(pointer)?.value;
        let offset = self.builder.integer(u128::from(
            self.member_initialization_offset(member, pointer)?,
        ))?;
        let address = self
            .emit(
                I::Index(
                    crate::lowir::vocabulary::Type::I8,
                    Some(crate::lowir::vocabulary::Projection::Field),
                    object,
                    offset,
                ),
                ty,
            )?
            .value;
        if union {
            self.untyped_places.insert(address);
        }
        self.materialize_class_at(expr, ty, address, false, location)?;
        Ok(true)
    }
    pub(super) fn class_conversion(
        &self,
        id: ast::ExprId,
    ) -> Option<(
        crate::sema::lowering::ConstructorId,
        crate::sema::lowering::ConstructorArguments,
    )> {
        let root = self.without_parentheses(id);
        if self.converting_source == Some(root) {
            None
        } else {
            self.graph.lower_class_conversion(root)
        }
    }
    pub(super) fn call_converting_constructor(
        &mut self,
        source: ast::ExprId,
        constructor: crate::sema::lowering::ConstructorId,
        object: Value,
        arguments: crate::sema::lowering::ConstructorArguments,
    ) -> Result<(), Error> {
        self.call_converting_constructor_at(
            source,
            constructor,
            object,
            arguments,
            self.tree.expressions.get(source).location,
        )
    }
    fn call_converting_constructor_at(
        &mut self,
        source: ast::ExprId,
        constructor: crate::sema::lowering::ConstructorId,
        object: Value,
        arguments: crate::sema::lowering::ConstructorArguments,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let saved = self
            .converting_source
            .replace(self.without_parentheses(source));
        let saved_constructor = self.converting_constructor.replace(constructor);
        let result = self.call_constructor(
            constructor,
            super::construction::Variant::Complete,
            object,
            super::construction::ConstructorInput {
                arguments: Some(arguments),
                location,
            },
        );
        self.converting_source = saved;
        self.converting_constructor = saved_constructor;
        result
    }
    pub(super) fn converting_reference_temporary(
        &mut self,
        source: ast::ExprId,
        ty: TypeId,
        context: super::conversions::Context,
        persistent: bool,
    ) -> Result<Value, Error> {
        let (constructor, arguments) =
            self.class_conversion(source)
                .ok_or(crate::sema::Error::Invariant(
                    "converted reference without constructor",
                ))?;
        let destroyed = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .storage_destructor(ty)?
            .is_some();
        if destroyed
            && !persistent
            && self.lifetime.full_expression
            && !self.lifetime.region
            && !self.storage_region_pending()
        {
            let start = matches!(context, super::conversions::Context::Initialization)
                .then_some(self.lifetime.expression_start)
                .flatten();
            self.open_temporary_region_at(start)?;
        }
        let slot = self.temporary_storage(
            ty,
            persistent,
            if matches!(context, super::conversions::Context::Argument) {
                "arg"
            } else {
                "tmpobj"
            },
        )?;
        let object = self.emit(I::Addr(slot), ty)?.value;
        self.storage_region_event()?;
        self.call_converting_constructor(source, constructor, object, arguments)?;
        if persistent {
            self.persistent_alive(ty, slot)?;
        } else if destroyed {
            self.temporary_alive(ty, object)?;
        }
        Ok(object)
    }
    pub(super) fn class_argument(&mut self, id: ast::ExprId, ty: TypeId) -> Result<Value, Error> {
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("class argument without class").into());
        };
        let direct = self.graph.lower_direct_class_parameter(class)?;
        let destroyed = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .storage_destructor(ty)?
            .is_some();
        if destroyed
            && self.lifetime.full_expression
            && !self.lifetime.region
            && !self.storage_region_pending()
        {
            if self.lifetime.region_start.is_pending()
                && (self.lifetime.temporary_segment.is_nothrow()
                    || self.lifetime.cleanups.is_empty()
                        && self.lifetime.temporary_segment.allows_eager_arguments())
            {
                self.open_temporary_region_at(self.lifetime.expression_start)?;
            } else if self.lifetime.region_start.is_pending()
                || self.lifetime.cleanups.is_empty()
                || self.has_tracked_region()
                || self.aggregation.floor.is_some()
            {
                self.open_temporary_region()?;
            }
        }
        let storage = types::storage(&*self.graph, ty)?;
        let stem = if direct { "argobj" } else { "arg" };
        let slot = if destroyed {
            self.element_argument_slot(storage, stem)?
        } else {
            self.generated_slot(stem, storage)?
        };
        let object = self.emit(I::Addr(slot), ty)?.value;
        self.storage_region_event()?;
        let source = self.without_parentheses(id);
        let functional = matches!(
            self.tree.expressions.get(source).kind,
            ExprKind::FunctionalCast { .. }
        ) || self.graph.lower_functional_call(source)?;
        if self.graph.lower_class_empty(class)?
            && self.graph.lower_class_copy(source)?.is_none()
            && !(self.converting_source == Some(source)
                && self
                    .converting_constructor
                    .is_some_and(|outer| self.graph.lower_parameter_copy(source, class, outer)))
            && functional
            && matches!(self.graph.lower_type(self.graph.lower_expression(source)?.ty)?,
                TypeView::Class(source_class) if source_class == class)
        {
            self.materialize_empty_argument(source, ty, object)?;
        } else {
            let owner = self.reference_lifetimes.aggregate.take();
            let persistent = std::mem::replace(&mut self.persistent_references, false);
            let result = self.materialize_class(id, ty, object);
            self.reference_lifetimes.aggregate = owner;
            self.persistent_references = persistent;
            result?;
        }
        if destroyed {
            let cleanup = if self.sharing_element_arguments() {
                slot
            } else {
                object
            };
            self.temporary_alive(ty, cleanup)?;
        }
        Ok(if direct { slot } else { object })
    }
    pub(super) fn materialize_empty_argument(
        &mut self,
        source: ast::ExprId,
        ty: TypeId,
        object: Value,
    ) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.lower_prvalue_initialization(source)? {
            Some(ClassInitialization::Constructed(
                crate::sema::lowering::ConstructorArguments::Value,
            )) => {
                let constructor = self.graph.lower_list_constructor(source)?.ok_or(
                    crate::sema::Error::Invariant("empty argument without constructor"),
                )?;
                self.construct_at(
                    constructor,
                    ty,
                    object,
                    super::construction::ConstructorInput {
                        arguments: Some(crate::sema::lowering::ConstructorArguments::Value),
                        location: self.tree.expressions.get(source).location,
                    },
                )
            }
            Some(
                ClassInitialization::Aggregate(values)
                | ClassInitialization::FlexibleAggregate(values),
            ) if values.is_empty() => Ok(()),
            Some(ClassInitialization::FromExpression(inner)) => {
                self.materialize_empty_argument(inner, ty, object)
            }
            _ => self.materialize_class(source, ty, object),
        }
    }
    pub(super) fn class_temporary(
        &mut self,
        id: ast::ExprId,
        use_: TemporaryUse,
        persistent: bool,
    ) -> Result<Value, Error> {
        let root = self.without_parentheses(id);
        if let ExprKind::Binary {
            op, left, right, ..
        } = self.tree.expressions.get(root).kind
            && op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Comma)
            && !self.graph.lower_operator_call(root)?
        {
            self.discard(left)?;
            return self.comma_class_temporary(root, right, use_, persistent);
        }
        let persistent = persistent || self.persistent_temporary_path(id);
        let ty = self.graph.lower_expression(id)?.ty;
        let owner = self.extended_temporary_owner(id);
        let destructor = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .storage_destructor(ty)?;
        // A scoped temporary with no earlier cleanup opens its region only
        // after construction, covering the reference's address store.
        let staged = owner.is_some()
            && destructor.is_some()
            && !self.lifetime.region
            && self.lifetime.cleanups.len()
                <= self
                    .handler_floor()
                    .max(self.lifetime.temporary_segment.floor())
            && self.lifetime.full_expression;
        if destructor.is_some()
            && !persistent
            && self.lifetime.guarded_operands != 0
            && !self.lifetime.tracking.active
            && self.lifetime.guard_start.is_none()
        {
            return Err(Error::Missing("guarded full-expression temporary cleanup"));
        }
        let call_result = self.graph.lower_operator_call(root)?
            || self.graph.lower_literal_call(root)?.is_some()
            || matches!(self.tree.expressions.get(root).kind, ExprKind::Call { .. })
                && !self.graph.lower_functional_call(root)?;
        let deferred = matches!(use_, TemporaryUse::Discard)
            && !self.lifetime.cleanups.is_empty()
            && call_result;
        let branched = matches!(
            self.tree.expressions.get(root).kind,
            ExprKind::Conditional { .. }
        );
        // temporary_region(zeroed): hidden segments open a fresh construction
        // region for a functional cast with no selected nontrivial constructor,
        // even after a previous temporary consumed the segment's pending start.
        let zeroed = !matches!(use_, TemporaryUse::Discard)
            && self.lifetime.temporary_segment.allows_eager_arguments()
            && (matches!(
                self.tree.expressions.get(root).kind,
                ExprKind::FunctionalCast { .. }
            ) || self.graph.lower_functional_call(root)?)
            && self
                .graph
                .lower_list_constructor(root)?
                .is_none_or(|constructor| self.graph.lower_constructor_trivial(constructor))
            && !self
                .lifetime
                .temporary_split
                .is_some_and(|position| self.builder.same_block(position));
        // discard_lowering leaves tracked construction to its call, after the
        // discarded object's address. It does not use temporary_object's
        // eager construction region in that case.
        let tracked_discard =
            matches!(use_, TemporaryUse::Discard) && self.lifetime.tracking.active;
        // temporary_region opens a pending nonthrowing segment before the
        // object's address (and before a conditional's condition). Guarded
        // branch calls do not reopen that segment after its terminator.
        if !persistent
            && !staged
            && self.lifetime.full_expression
            && self.lifetime.region_start.is_pending()
            && self.lifetime.temporary_segment.is_nothrow()
            && !self.lifetime.region
            && self.lifetime.guarded_operands == 0
            && self.lifetime.choice_region != super::lifetime::ChoiceRegion::Reopened
            && !tracked_discard
            && (destructor.is_some() || !branched)
            && (owner.is_none() || !self.lifetime.cleanups.is_empty())
        {
            self.open_temporary_region_at(self.lifetime.expression_start)?;
        }
        // An inert call result extended by a local reference opens at the
        // segment's start, before taking the temporary's address.
        if owner.is_some()
            && destructor.is_none()
            && call_result
            && !persistent
            && self.lifetime.region_start.is_pending()
            && !self.lifetime.region
            && !self.lifetime.cleanups.is_empty()
            && self.lifetime.full_expression
            && !self.lifetime.tracking.active
            && self.element_temporaries.depth == 0
            && self.lifetime.guarded_operands == 0
        {
            self.open_temporary_region_at(self.lifetime.expression_start)?;
        }
        let opens = (destructor.is_some()
                || !branched
                    && self.lifetime.region_start.is_pending()
                    && self.lifetime.guarded_operands == 0)
                && !persistent
                && !staged
                && self.lifetime.full_expression
                && !self.lifetime.temporary_segment.is_nothrow()
                && !self.lifetime.region
                && !deferred
                && !tracked_discard
                // A statement block suspends its enclosing construction
                // region, including an empty region's pending cleanup chain.
                // Its opaque body cannot supply operand-temporary facts.
                && (owner.is_none()
                    || matches!(self.tree.expressions.get(root).kind, ExprKind::Statement(_))
                    || self.operand_temporaries(id)?.made)
                && (if branched {
                    self.lifetime.region_start.is_pending()
                        && self.lifetime.guarded_operands == 0
                        && (self.lifetime.cleanups.is_empty() && !self.lifetime.tracking.active
                            || self.lifetime.call_marks.iter().any(|mark| {
                                !mark.declared && self.builder.same_block(mark.position)
                            }))
                } else {
                    self.lifetime.region_start.is_pending()
                        || self.lifetime.cleanups.is_empty()
                        || zeroed
                        || self.aggregate_has_members()
                        || self.lifetime.tracking.active
                        || self.aggregation.call_scope
                            == super::aggregate_cleanup::CallScope::OmittedValue
                });
        let deferred_region = (opens
            && !branched
            && !self.lifetime.cleanups.is_empty()
            && (!self.lifetime.tracking.active || self.reference_lifetimes.has_scope())
            && !self
                .lifetime
                .call_marks
                .iter()
                .any(|mark| !mark.declared && self.builder.same_block(mark.position)))
            || branched
                && matches!(use_, TemporaryUse::Discard)
                && !persistent
                && !staged
                && self.lifetime.full_expression
                && self.lifetime.region_start.is_pending()
                && !self.lifetime.region
                && !self.lifetime.tracking.active
                && !self.lifetime.cleanups.is_empty();
        let pending_storage = opens && !deferred_region && self.pending_storage_call_region()?;
        if opens
            && !deferred_region
            && !pending_storage
            && (!self.storage_region_pending()
                || self.aggregation.call_scope == super::aggregate_cleanup::CallScope::OmittedValue)
        {
            let position =
                if !self.lifetime.region_start.is_pending() && !self.lifetime.tracking.active {
                    Some(self.builder.position()?)
                } else if (branched
                    || self.lifetime.temporary_segment.allows_eager_arguments()
                    || self.lifetime.region_start.is_pending() && self.lifetime.cleanups.is_empty())
                    && !self.lifetime.tracking.active
                {
                    self.lifetime.expression_start
                } else {
                    None
                };
            self.open_temporary_region_at(position)?;
        }
        if destructor.is_some()
            && !persistent
            && self.lifetime.full_expression
            && self.aggregate_has_members()
        {
            self.before_lifetime_call(true)?;
        }
        let slot = self.temporary_storage(ty, persistent, use_.stem())?;
        let object = self.emit(I::Addr(slot), ty)?.value;
        if !persistent && !branched {
            self.storage_region_event()?;
        }
        if deferred_region && !self.lifetime.region {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        let owned_list = if owner.is_some() || persistent {
            self.list_object(id)?
        } else {
            None
        };
        if let Some(list) = owned_list {
            self.initialize_list_object(list, ty, object, persistent, owner)?;
        } else {
            self.materialize_prvalue(
                id,
                ty,
                object,
                false,
                self.tree.expressions.get(id).location,
            )?;
        }
        if persistent && !self.terminated {
            self.persistent_alive(ty, slot)?;
        } else if destructor.is_some() && !self.terminated {
            self.temporary_alive(ty, object)?;
        } else if destructor.is_none() && !persistent && !self.terminated && owned_list.is_none() {
            self.finish_inert_temporary(owner.is_some() || self.list_object(id)?.is_some())?;
        }
        if destructor.is_some()
            && !persistent
            && !self.terminated
            && let Some(owner) = owner
        {
            self.extend_last_temporary(owner)?;
            let subobject = self.reference_lifetimes.root != Some(root);
            if (staged || branched && subobject)
                && !self.lifetime.region
                && self.lifetime.full_expression
            {
                let position = self.builder.position()?;
                self.open_temporary_region_at(Some(position))?;
            }
        }
        Ok(object)
    }
    pub(super) fn materialize_class(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
    ) -> Result<(), Error> {
        self.materialize_class_qualified(id, ty, object, false)
    }
    pub(super) fn materialize_class_qualified(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
        volatile: bool,
    ) -> Result<(), Error> {
        self.materialize_class_at(
            id,
            ty,
            object,
            volatile,
            self.tree.expressions.get(id).location,
        )
    }
    pub(super) fn materialize_class_at(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
        volatile: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        self.symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .storage_destructor(ty)?;
        let id = self.without_parentheses(id);
        if self.converting_source == Some(id)
            && let TypeView::Class(class) = self.graph.lower_type(ty)?
            && self
                .converting_constructor
                .is_some_and(|outer| self.graph.lower_parameter_copy(id, class, outer))
            && let Some(action) = self.graph.lower_demand_internal_copy(
                self.tree,
                self.cursor,
                self.budget,
                self.stack_base,
                (id, class, false),
            )?
        {
            let (reference, projection) =
                self.graph
                    .lower_parameter_binding(id)?
                    .ok_or(crate::sema::Error::Invariant(
                        "internal copy without binding",
                    ))?;
            self.materialize_copy(
                id,
                ty,
                object,
                Some(action),
                CopyCall::InternalParameter {
                    reference,
                    projection,
                },
                location,
            )?;
            return Ok(());
        }
        if let Some(action @ CopyAction::Constructor { constructor, .. }) =
            self.graph.lower_class_copy(id)?
            && matches!(self.graph.lower_type(ty)?, TypeView::Class(class) if class == self.graph.lower_constructor(constructor).class)
            && self.materialize_copy(
                id,
                ty,
                object,
                Some(action),
                CopyCall::Expression,
                location,
            )?
        {
            return Ok(());
        }
        if let Some((constructor, arguments)) = self.class_conversion(id) {
            return self.call_converting_constructor_at(
                id,
                constructor,
                object,
                arguments,
                location,
            );
        }
        if self.initialize_function_conversion(id, ty, object)? {
            return Ok(());
        }
        if self.materialize_copy(
            id,
            ty,
            object,
            self.graph.lower_class_copy(id)?,
            CopyCall::Expression,
            location,
        )? {
            return Ok(());
        }
        self.materialize_prvalue(id, ty, object, volatile, location)
    }
    pub(super) fn materialize_copy(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
        action: Option<CopyAction>,
        call: CopyCall,
        location: crate::support::source::SourceOffset,
    ) -> Result<bool, Error> {
        match action {
            Some(CopyAction::Storage {
                reference,
                skip_empty,
            }) => {
                let TypeView::Class(class) = self.graph.lower_type(ty)? else {
                    return Err(crate::sema::Error::Invariant("storage copy without class").into());
                };
                let empty = self.graph.lower_class_empty(class)?;
                if empty && skip_empty && self.side_effect_free(id, false, 0)? {
                    return Ok(true);
                }
                let source = if let CopyCall::InternalParameter {
                    reference,
                    projection,
                } = call
                {
                    self.bind_internal_copy_source(id, reference, projection)?
                } else {
                    self.bind_reference(id, reference, super::conversions::Context::Argument)?
                };
                if !empty {
                    self.copy_object(ty, source, object)?;
                }
                return Ok(true);
            }
            Some(CopyAction::Constructor {
                constructor,
                elision,
                written_move,
            }) => {
                if !written_move
                    && self.delegate_copy_storage_bound(
                        constructor,
                        object,
                        crate::sema::lowering::ConstructorArguments::Single(id),
                        location,
                        call,
                    )?
                {
                    return Ok(true);
                }
                self.call_constructor_from_expression(
                    constructor,
                    object,
                    id,
                    elision,
                    call,
                    location,
                )?;
                return Ok(true);
            }
            None => (),
        }
        Ok(false)
    }
    // Construct the source expression itself, without replaying the outer
    // initialization's selected copy when a reference materializes its argument.
    pub(super) fn materialize_prvalue(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
        volatile: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        if let ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind
            && let Some(intrinsic) = self.graph.lower_atomic_intrinsic(id)?
        {
            let value = self.atomic_intrinsic(id, arguments, intrinsic)?;
            return self.effect(I::Store(
                crate::lowir::model::Facts::memory(
                    false,
                    value.ty.bits() > 8 || value.ty == crate::lowir::vocabulary::Type::Ptr,
                ),
                value.ty,
                value.value,
                object,
            ));
        }
        if let Some(list) = self.list_object(id)?
            && !matches!(self.graph.lower_type(ty)?, TypeView::Class(class) if self.graph.lower_aggregate_class(class))
        {
            // A directly materialized list expression owns its backing only
            // for this full expression. Extended bindings use the explicit
            // owner path in class_temporary; a surrounding static object
            // must not extend a copied member's functional-cast array.
            return self.initialize_list_object(list, ty, object, false, None);
        }
        match self.graph.lower_prvalue_initialization(id)? {
            Some(ClassInitialization::Constructed(arguments)) => {
                let constructor = self
                    .graph
                    .lower_list_constructor(id)?
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "class prvalue without selected constructor",
                    )))?;
                if self.graph.lower_function_conversion(id)?.is_none()
                    && self.delegate_copy_storage(constructor, object, arguments, location)?
                {
                    return Ok(());
                }
                self.construct_at(
                    constructor,
                    ty,
                    object,
                    super::construction::ConstructorInput {
                        arguments: Some(arguments),
                        location,
                    },
                )
            }
            Some(
                ClassInitialization::Aggregate(arguments)
                | ClassInitialization::FlexibleAggregate(arguments),
            ) => self.aggregate_prvalue(
                ty,
                object,
                &self.owned_expression_ids(arguments),
                self.graph.lower_aggregate_union_member(arguments),
            ),
            Some(ClassInitialization::FromExpression(source)) => {
                self.materialize_class_at(source, ty, object, volatile, location)
            }
            None => match self.tree.expressions.get(id).kind {
                ExprKind::BuiltinValue { .. } => Err(crate::sema::Error::at(
                    self.tree.expressions.get(id).location,
                    "a class prvalue outside the supported construction forms",
                )
                .into()),
                ExprKind::VaArg { operand, .. } => {
                    let value = self.va_arg(id, operand)?;
                    self.effect(I::Store(
                        crate::lowir::model::Facts::default(),
                        value.ty,
                        value.value,
                        object,
                    ))
                }
                ExprKind::Statement(body) => {
                    self.statement_expression(id, body, Some(object))?;
                    Ok(())
                }
                ExprKind::Lambda(_) => self.closure_object(ty, object, location),
                _ if self.graph.lower_operator_call(id)? => {
                    self.operator_call_into(id, Some(object))?;
                    Ok(())
                }
                ExprKind::Literal(_) if self.graph.lower_literal_call(id)?.is_some() => {
                    self.literal_call_into(id, Some(object))?;
                    Ok(())
                }
                ExprKind::Call { callee, arguments } => {
                    self.call_into(id, callee, arguments, Some(object))?;
                    Ok(())
                }
                ExprKind::Binary {
                    op, left, right, ..
                } if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Comma) => {
                    self.discard(left)?;
                    self.materialize_class_at(right, ty, object, volatile, location)
                }
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => self.materialize_conditional(
                    condition,
                    [then_expr, else_expr],
                    ty,
                    object,
                    volatile,
                    location,
                ),
                _ => Err(Error::Missing("class prvalue materialization")),
            },
        }
    }
}
