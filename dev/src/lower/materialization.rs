// Translated from lower/object_lowering.cpp::materialize_prvalue/temporary_object.
// See NOTICE.
//! Original class expressions initialize their destination without an intermediate copy.
use super::{Error, function::Function, types};
use crate::{
    lowir::model::{InstructionKind as I, Value},
    sema::lowering::{ClassInitialization, CopyAction, EntityId, TypeId, TypeView},
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
    // expression_lowering.cpp::push_converted's null-parameter class path:
    // course LowIR passes the address of a caller-owned byte copy. Unlike a
    // declared class parameter, this path neither selects a copy constructor
    // nor registers destruction of the copied argument.
    pub(super) fn ellipsis_argument(&mut self, id: ast::ExprId) -> Result<Value, Error> {
        let facts = self.graph.lower_expression(id)?;
        let TypeView::Class(class) = self.graph.lower_type(facts.ty)? else {
            return Ok(self
                .operand(id, super::conversions::Context::Argument)?
                .value);
        };
        let ty = facts.ty;
        let slot = self.generated_slot("arg", types::storage(self.graph, ty)?)?;
        let object = self.emit(I::Addr(slot), ty)?.value;
        if facts.is_place {
            let source = self.address(id)?;
            if !self.graph.lower_class_empty(class)? {
                let (size, alignment) = types::layout(self.graph, ty)?;
                let size = size.min(self.graph.lower_nonvirtual_bytes(class)?);
                self.effect(I::CopyObject(size, alignment, source, object))?;
            }
        } else {
            self.materialize_class(id, ty, object)?;
        }
        Ok(object)
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
                Some(ClassInitialization::Aggregate(_))
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
        self.materialize_class(expr, ty, address)?;
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
        let saved = self
            .converting_source
            .replace(self.without_parentheses(source));
        let result = self.call_constructor(
            constructor,
            super::construction::Variant::Complete,
            object,
            Some(arguments),
        );
        self.converting_source = saved;
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
        let destroyed = self.symbols.storage_destructor(ty)?.is_some();
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
        let destroyed = self.symbols.storage_destructor(ty)?.is_some();
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
        let slot = self.generated_slot(
            if direct { "argobj" } else { "arg" },
            types::storage(self.graph, ty)?,
        )?;
        let object = self.emit(I::Addr(slot), ty)?.value;
        self.storage_region_event()?;
        let source = self.without_parentheses(id);
        let functional = matches!(
            self.tree.expressions.get(source).kind,
            ExprKind::FunctionalCast { .. }
        ) || self.graph.lower_functional_call(source)?;
        if self.graph.lower_class_empty(class)?
            && self.graph.lower_class_copy(source)?.is_none()
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
            self.temporary_alive(ty, object)?;
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
                if !self.graph.lower_constructor_trivial(constructor) {
                    self.call_constructor(
                        constructor,
                        super::construction::Variant::Complete,
                        object,
                        None,
                    )?;
                }
                Ok(())
            }
            Some(ClassInitialization::Aggregate(values)) if values.is_empty() => Ok(()),
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
        let destructor = self.symbols.storage_destructor(ty)?;
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
        let opens =
            (destructor.is_some()
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
                && (owner.is_none() || self.operand_temporaries(id)?.made)
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
        if opens
            && !deferred_region
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
        self.storage_region_event()?;
        if deferred_region {
            let position = self.builder.position()?;
            self.open_temporary_region_at(Some(position))?;
        }
        self.materialize_prvalue(id, ty, object, false)?;
        if persistent && !self.terminated {
            self.persistent_alive(ty, slot)?;
        } else if destructor.is_some() && !self.terminated {
            self.temporary_alive(ty, object)?;
        } else if destructor.is_none() && !persistent && !self.terminated {
            self.finish_inert_temporary(owner.is_some())?;
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
        self.check_stack()?;
        self.symbols.storage_destructor(ty)?;
        let id = self.without_parentheses(id);
        if let Some(action @ CopyAction::Constructor { constructor, .. }) =
            self.graph.lower_class_copy(id)?
            && matches!(self.graph.lower_type(ty)?, TypeView::Class(class) if class == self.graph.lower_constructor(constructor).class)
            && self.materialize_copy(id, ty, object, Some(action))?
        {
            return Ok(());
        }
        if let Some((constructor, arguments)) = self.class_conversion(id) {
            return self.call_converting_constructor(id, constructor, object, arguments);
        }
        if self.initialize_function_conversion(id, ty, object)? {
            return Ok(());
        }
        if self.materialize_copy(id, ty, object, self.graph.lower_class_copy(id)?)? {
            return Ok(());
        }
        self.materialize_prvalue(id, ty, object, volatile)
    }
    pub(super) fn materialize_copy(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
        action: Option<CopyAction>,
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
                let source =
                    self.bind_reference(id, reference, super::conversions::Context::Argument)?;
                if !empty {
                    let (size, alignment) = types::layout(self.graph, ty)?;
                    let size = size.min(self.graph.lower_nonvirtual_bytes(class)?);
                    self.effect(I::CopyObject(size, alignment, source, object))?;
                }
                return Ok(true);
            }
            Some(CopyAction::Constructor {
                constructor,
                elision,
                written_move,
            }) => {
                if !written_move
                    && self.delegate_copy_storage(
                        constructor,
                        object,
                        crate::sema::lowering::ConstructorArguments::Single(id),
                    )?
                {
                    return Ok(true);
                }
                self.call_constructor_from_expression(constructor, object, id, elision)?;
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
    ) -> Result<(), Error> {
        self.check_stack()?;
        let id = self.without_parentheses(id);
        match self.graph.lower_prvalue_initialization(id)? {
            Some(ClassInitialization::Constructed(arguments)) => {
                let constructor = self
                    .graph
                    .lower_list_constructor(id)?
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "class prvalue without selected constructor",
                    )))?;
                if self.graph.lower_function_conversion(id)?.is_none()
                    && self.delegate_copy_storage(constructor, object, arguments)?
                {
                    return Ok(());
                }
                self.construct_at_qualified(constructor, ty, object, Some(arguments), volatile)
            }
            Some(ClassInitialization::Aggregate(arguments)) => {
                self.aggregate_prvalue(ty, object, self.tree.expr_lists.get(arguments))
            }
            Some(ClassInitialization::FromExpression(source)) => {
                self.materialize_class_qualified(source, ty, object, volatile)
            }
            None => match self.tree.expressions.get(id).kind {
                ExprKind::Lambda(_) => self.closure_object(ty, object),
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
                    self.materialize_class_qualified(right, ty, object, volatile)
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
                ),
                _ => Err(Error::Missing("class prvalue materialization")),
            },
        }
    }
}
