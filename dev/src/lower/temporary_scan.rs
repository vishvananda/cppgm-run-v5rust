// Translated from lower/cleanup_lowering.cpp::creates_temporaries. See NOTICE.
//! Full-expression queries use the original operands and selected default arguments.
use super::{Error, function::Function};
use crate::{
    sema::lowering::{CallTarget, ConstructorId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy, Default)]
pub(super) enum CallTemporaries {
    #[default]
    Absent,
    Nothrow,
    ThrowingDestructor,
}
impl CallTemporaries {
    fn include(&mut self, throws: bool) {
        if throws {
            *self = Self::ThrowingDestructor;
        } else if matches!(self, Self::Absent) {
            *self = Self::Nothrow;
        }
    }
    fn present(self) -> bool {
        !matches!(self, Self::Absent)
    }
    fn destructor_throws(self) -> bool {
        matches!(self, Self::ThrowingDestructor)
    }
}
#[derive(Clone, Copy, Default)]
pub(super) struct TemporarySummary {
    pub made: bool,
    pub call_result: CallTemporaries,
    pub caller_destroyed: bool,
}
#[derive(Clone, Copy)]
pub(super) enum ExpressionUse {
    Ordinary,
    Initialization,
    InitializerArgument,
    Value,
    ClassResult,
}
impl Function<'_, '_, '_, '_> {
    // cleanup_lowering.cpp::note_call_arguments: an in-place constructor
    // contributes its operands/defaults, rather than its destination object.
    pub(super) fn note_constructor_temporaries(
        &mut self,
        constructor: ConstructorId,
        written: &[ast::ExprId],
    ) -> Result<(), Error> {
        if !self.lifetime.full_expression
            || !self.lifetime.temporary_segment.allows_eager_arguments()
            || !self.lifetime.region_start.is_pending()
            || self.lifetime.region
        {
            return Ok(());
        }
        let mut made = false;
        for &argument in written {
            let outer_copy = matches!(self.graph.lower_class_copy(argument)?,
                Some(crate::sema::lowering::CopyAction::Constructor { constructor: selected, .. })
                    if selected == constructor);
            made |= if outer_copy {
                // This selected transfer constructs our destination; only
                // evaluating its source can make argument temporaries.
                self.scan_temporaries(argument, true, false)?.made
            } else {
                self.expression_temporaries(argument)?.made
            };
            if made {
                break;
            }
        }
        let function = self.graph.lower_constructor_function(constructor)?;
        if !made
            && let Some(entity) = self.graph.lower_constructor_entity(constructor)
            && let TypeView::Function { parameters, .. } = self.graph.lower_type(function.ty)?
        {
            for index in written.len()..parameters.len() {
                if let Some(argument) = self.graph.lower_default(entity, index) {
                    made |= self.expression_temporaries(argument)?.made;
                }
                if made {
                    break;
                }
            }
        }
        if made {
            self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Eager;
            if !function.nothrow {
                self.open_temporary_region_at(self.lifetime.expression_start)?;
            }
        }
        Ok(())
    }
    pub(super) fn begin_constructor_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: ConstructorId,
    ) -> Result<bool, Error> {
        self.begin_subobject_initializer(initializer, Some(constructor))
    }
    pub(super) fn begin_subobject_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: Option<ConstructorId>,
    ) -> Result<bool, Error> {
        self.begin_initializer(initializer, constructor, false)
    }
    pub(super) fn begin_base_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: Option<ConstructorId>,
    ) -> Result<bool, Error> {
        self.begin_initializer(initializer, constructor, true)
    }
    fn begin_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: Option<ConstructorId>,
        base: bool,
    ) -> Result<bool, Error> {
        if self.lifetime.full_expression {
            return Ok(false);
        }
        let mut caller = false;
        if let Some(constructor) = constructor {
            let function = self.graph.lower_constructor_function(constructor)?;
            let TypeView::Function { parameters, .. } = self.graph.lower_type(function.ty)? else {
                return Err(
                    crate::sema::Error::Invariant("initializer without function type").into(),
                );
            };
            for &parameter in parameters {
                caller |= matches!(self.graph.lower_type(parameter)?, TypeView::Class(_))
                    && self.temporary_type_destroyed(parameter)?;
            }
        }
        let (arguments, root) = match initializer.kind {
            ast::InitializerKind::Assign(root) => {
                (crate::syntax::storage::List::default(), Some(root))
            }
            ast::InitializerKind::Parenthesized(arguments) => (arguments, None),
            ast::InitializerKind::Braced(id) => {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
                    return Err(
                        crate::sema::Error::Invariant("delegating initializer list shape").into(),
                    );
                };
                (elements, Some(id))
            }
            _ => return Err(crate::sema::Error::Invariant("delegating initializer shape").into()),
        };
        let mut made = caller;
        let mut calls = false;
        let mut throwing_destructor = false;
        if let Some(root) = root {
            // A class prvalue with no outer transfer constructs the member
            // itself. Only its operands can create expression temporaries.
            let facts = self.graph.lower_expression(root)?;
            let in_place = !facts.is_place
                && matches!(self.graph.lower_type(facts.ty)?, TypeView::Class(_))
                && self.graph.lower_class_copy(root)?.is_none();
            let summary = if in_place {
                self.operand_temporaries(root)?
            } else {
                self.expression_temporaries(root)?
            };
            made |= summary.made;
            calls |= summary.call_result.present() || summary.caller_destroyed;
            throwing_destructor |= summary.call_result.destructor_throws();
        }
        for &argument in self.tree.expr_lists.get(arguments) {
            let summary = if base {
                // The base's selected transfer initializes the destination;
                // it is not a conversion made while evaluating its argument.
                self.scan_temporaries(argument, true, false)?
            } else {
                self.expression_temporaries(argument)?
            };
            made |= summary.made;
            calls |= summary.call_result.present() || summary.caller_destroyed;
            throwing_destructor |= summary.call_result.destructor_throws();
        }
        if !made && self.lifetime.cleanups.is_empty() {
            return Ok(false);
        }
        // The braced list's root describes the destination, so it is not
        // another temporary. Its selected constructor does contribute to
        // the C++ owner's full-expression throw query for that list.
        let mut throwing = root
            .map(|id| self.expression_throws(id))
            .transpose()?
            .unwrap_or(false);
        for &argument in self.tree.expr_lists.get(arguments) {
            throwing |= self.expression_throws(argument)?;
        }
        if !caller && !throwing && (!calls || throwing_destructor) {
            return Ok(false);
        }
        self.lifetime.full_expression = true;
        self.lifetime.region_start = super::lifetime::RegionStart::Pending;
        self.lifetime.temporary_segment = if caller || made && throwing {
            super::lifetime::TemporarySegment::Eager
        } else if made {
            super::lifetime::TemporarySegment::Nothrow
        } else {
            super::lifetime::TemporarySegment::Empty
        };
        self.ensure_insertion()?;
        self.lifetime.expression_start = Some(self.builder.position()?);
        if let Some(root) = root {
            if caller || throwing {
                self.begin_tracking(root, ExpressionUse::Initialization)?;
            }
        } else if !caller && throwing {
            for &argument in self.tree.expr_lists.get(arguments) {
                self.begin_tracking(argument, ExpressionUse::InitializerArgument)?;
            }
        }
        Ok(true)
    }
    pub(super) fn begin_full_expression(
        &mut self,
        root: Option<ast::ExprId>,
        usage: ExpressionUse,
    ) -> Result<bool, Error> {
        if self.lifetime.full_expression || self.static_initializer {
            return Ok(false);
        }
        let Some(root) = root else { return Ok(false) };
        let summary = if matches!(usage, ExpressionUse::ClassResult) {
            let mut summary = self.operand_temporaries(root)?;
            // record_copy is an initialization conversion even when its
            // source is a named glvalue. It makes the hidden return segment;
            // note_constructor_temporaries separately queries its arguments.
            if let Some(crate::sema::lowering::CopyAction::Constructor { constructor, .. }) =
                self.graph.lower_class_copy(root)?
            {
                summary.made |= self.temporary_type_destroyed(
                    self.graph
                        .lower_entity_type(self.graph.lower_constructor(constructor).class)?,
                )?;
            }
            summary
        } else {
            self.expression_temporaries(root)?
        };
        if !summary.made && self.lifetime.cleanups.is_empty() {
            return Ok(false);
        }
        // class_return supplies a hidden context: once its initialization
        // conversion makes temporaries, call_may_throw brackets even a
        // nonthrowing transfer (cleanup_lowering.cpp).
        let hidden_copy = matches!(usage, ExpressionUse::ClassResult)
            && summary.made
            && matches!(
                self.graph.lower_class_copy(root)?,
                Some(crate::sema::lowering::CopyAction::Constructor { .. })
            );
        let hidden =
            summary.made && matches!(usage, ExpressionUse::Value | ExpressionUse::ClassResult);
        let throwing = hidden_copy || self.expression_throws_in_context(root, hidden)?;
        if (summary.made || !self.lifetime.cleanups.is_empty())
            && (throwing || summary.call_result.present() || summary.caller_destroyed)
        {
            self.lifetime.full_expression = true;
            self.lifetime.region_start = super::lifetime::RegionStart::Pending;
            self.lifetime.temporary_segment = if throwing {
                super::lifetime::TemporarySegment::new(
                    summary.made,
                    matches!(usage, ExpressionUse::Value | ExpressionUse::ClassResult),
                )
            } else {
                super::lifetime::TemporarySegment::Nothrow
            };
            self.ensure_insertion()?;
            self.lifetime.expression_start = Some(self.builder.position()?);
            self.begin_tracking(root, usage)?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn expression_temporaries(
        &mut self,
        root: ast::ExprId,
    ) -> Result<TemporarySummary, Error> {
        let root = self.without_parentheses(root);
        if let Some(&summary) = self.lifetime.temporary_queries.get(&root) {
            return Ok(summary);
        }
        let summary = self.scan_temporaries(root, true, true)?;
        self.lifetime.temporary_queries.insert(root, summary);
        Ok(summary)
    }
    pub(super) fn operand_temporaries(
        &mut self,
        root: ast::ExprId,
    ) -> Result<TemporarySummary, Error> {
        self.scan_temporaries(root, false, true)
    }
    fn scan_temporaries(
        &mut self,
        root: ast::ExprId,
        include_root: bool,
        root_transfer: bool,
    ) -> Result<TemporarySummary, Error> {
        let mut work = std::mem::take(&mut self.lifetime.temporary_work);
        let result = self.scan_temporary_operands(root, include_root, root_transfer, &mut work);
        work.clear();
        self.lifetime.temporary_work = work;
        result
    }
    fn scan_temporary_operands(
        &self,
        root: ast::ExprId,
        include_root: bool,
        root_transfer: bool,
        work: &mut Vec<ast::ExprId>,
    ) -> Result<TemporarySummary, Error> {
        let root = self.without_parentheses(root);
        work.push(root);
        let mut summary = TemporarySummary::default();
        while let Some(id) = work.pop() {
            let id = self.without_parentheses(id);
            let facts = self.graph.lower_expression(id)?;
            let mut step = 0;
            while let Some(call) = self.graph.lower_arrow_call(id, step)? {
                if !call.place && self.temporary_type_destroyed(call.result)? {
                    summary.made = true;
                    summary
                        .call_result
                        .include(self.temporary_destructor_throws(call.result)?);
                }
                step += 1;
            }
            if facts.folded.is_some() {
                continue;
            }
            if (root_transfer || id != root)
                && let Some(crate::sema::lowering::CopyAction::Constructor { constructor, .. }) =
                    self.graph.lower_class_copy(id)?
            {
                let function = self.graph.lower_constructor_function(constructor)?;
                // A recorded transfer is an initialization conversion in
                // C++ creates_temporaries. An operand-only query excludes
                // its destination, including a return's caller-owned result.
                let class = self.graph.lower_constructor(constructor).class;
                if include_root || id != root {
                    summary.made |=
                        self.temporary_type_destroyed(self.graph.lower_entity_type(class)?)?;
                }
                let TypeView::Function { parameters, .. } = self.graph.lower_type(function.ty)?
                else {
                    return Err(crate::sema::Error::Invariant("copy temporary signature").into());
                };
                for &parameter in parameters {
                    let caller = matches!(self.graph.lower_type(parameter)?, TypeView::Class(_))
                        && self.temporary_type_destroyed(parameter)?;
                    summary.made |= caller;
                    summary.caller_destroyed |= caller;
                }
            }
            let kind = self.tree.expressions.get(id).kind;
            let call =
                matches!(kind, ExprKind::Call { .. }) && !self.graph.lower_functional_call(id)?;
            let operator = self.graph.lower_operator_call(id)?;
            if (include_root || id != root)
                && !facts.is_place
                && self.temporary_type_destroyed(facts.ty)?
            {
                summary.made = true;
                if call || operator || self.graph.lower_literal_call(id)?.is_some() {
                    summary
                        .call_result
                        .include(self.temporary_destructor_throws(facts.ty)?);
                }
            }
            if (include_root || id != root)
                && let Some((constructor, _)) = self.graph.lower_class_conversion(id)
            {
                let ty = self
                    .graph
                    .lower_entity_type(self.graph.lower_constructor(constructor).class)?;
                summary.made |= self.temporary_type_destroyed(ty)?;
            }
            if operator {
                self.call_temporary_operands(id, usize::MAX, work, &mut summary)?;
            }
            match kind {
                ExprKind::Call { callee, arguments } => {
                    work.extend(self.tree.expr_lists.get(arguments));
                    if call {
                        self.call_temporary_operands(
                            id,
                            self.tree.expr_lists.get(arguments).len(),
                            work,
                            &mut summary,
                        )?;
                        if self.graph.lower_floating_intrinsic(id)?.is_none() {
                            work.push(callee);
                        }
                    }
                }
                ExprKind::Binary { left, right, .. }
                | ExprKind::Subscript {
                    base: left,
                    index: right,
                } => {
                    work.extend([left, right]);
                }
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    work.extend([condition, then_expr, else_expr]);
                }
                ExprKind::Unary { operand, .. }
                | ExprKind::Postfix { operand, .. }
                | ExprKind::Cast { operand, .. }
                | ExprKind::Delete { operand, .. }
                | ExprKind::Member { base: operand, .. } => work.push(operand),
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    work.extend(self.tree.expr_lists.get(arguments));
                    work.extend(braced);
                }
                ExprKind::Braced { elements, .. } => {
                    work.extend(self.tree.expr_lists.get(elements));
                }
                ExprKind::New {
                    placement,
                    initializer,
                    ..
                } => {
                    work.extend(self.tree.expr_lists.get(placement.unwrap_or_default()));
                    if let Some(initializer) = initializer {
                        match initializer.kind {
                            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                                work.push(id);
                            }
                            ast::InitializerKind::Parenthesized(arguments) => {
                                work.extend(self.tree.expr_lists.get(arguments));
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(summary)
    }
    fn call_temporary_operands(
        &self,
        id: ast::ExprId,
        given: usize,
        work: &mut Vec<ast::ExprId>,
        summary: &mut TemporarySummary,
    ) -> Result<(), Error> {
        if self.graph.lower_floating_intrinsic(id)?.is_some()
            || self.graph.lower_destructor_evaluation_only(id)?
        {
            return Ok(());
        }
        let (ty, entity) = match self.graph.lower_call(id)? {
            CallTarget::Direct(entity) => (self.graph.lower_function(entity)?.ty, Some(entity)),
            CallTarget::Indirect(ty, _) => (ty, None),
        };
        let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("temporary call without function").into());
        };
        for (index, &parameter) in parameters.iter().enumerate() {
            let caller = matches!(self.graph.lower_type(parameter)?, TypeView::Class(_))
                && self.temporary_type_destroyed(parameter)?;
            summary.made |= caller;
            summary.caller_destroyed |= caller;
            if index >= given
                && let Some(entity) = entity
            {
                work.extend(self.graph.lower_default(entity, index));
            }
        }
        Ok(())
    }
    fn temporary_type_destroyed(&self, mut ty: TypeId) -> Result<bool, Error> {
        while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            ty = element;
        }
        Ok(matches!(self.graph.lower_type(ty)?, TypeView::Class(class)
            if self.graph.lower_complete_object(ty)? && self.graph.lower_nontrivial_destructor(class)?))
    }
    fn temporary_destructor_throws(&self, mut ty: TypeId) -> Result<bool, Error> {
        while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            ty = element;
        }
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Ok(false);
        };
        self.graph
            .lower_destructor_entity(class)
            .map_or(Ok(false), |entity| {
                Ok(!self.graph.lower_function(entity)?.nothrow)
            })
    }
    pub(super) fn constructor_expression(
        &self,
        entity: crate::sema::lowering::EntityId,
        initializer: Option<ast::Initializer>,
        made: bool,
    ) -> Result<bool, Error> {
        let Some(constructor) = self
            .graph
            .lower_construction(entity)
            .filter(|_| initializer.is_some())
        else {
            return Ok(false);
        };
        let function = self.graph.lower_constructor_function(constructor)?;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(function.ty)? else {
            return Err(crate::sema::Error::Invariant(
                "constructor temporary boundary without function",
            )
            .into());
        };
        let mut caller = false;
        for &parameter in parameters {
            caller |= matches!(self.graph.lower_type(parameter)?, TypeView::Class(_))
                && self.temporary_type_destroyed(parameter)?;
        }
        let single = match initializer.map(|initializer| initializer.kind) {
            Some(ast::InitializerKind::Parenthesized(arguments)) => {
                self.tree.expr_lists.get(arguments).len() == 1
            }
            Some(ast::InitializerKind::Assign(_) | ast::InitializerKind::Braced(_)) => true,
            _ => false,
        };
        // local_object first stages a single temporary-making expression;
        // caller-destroyed parameter forcing applies to the other initializer path.
        Ok(caller && !(single && made) || made && !function.nothrow)
    }
    pub(super) fn initializer_temporaries(
        &mut self,
        initializer: Option<ast::Initializer>,
    ) -> Result<bool, Error> {
        let Some(initializer) = initializer else {
            return Ok(false);
        };
        match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                Ok(self.expression_temporaries(id)?.made)
            }
            ast::InitializerKind::Parenthesized(arguments) => {
                for &argument in self.tree.expr_lists.get(arguments) {
                    if self.expression_temporaries(argument)?.made {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}
