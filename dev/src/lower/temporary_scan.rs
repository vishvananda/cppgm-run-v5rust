// Translated from lower/cleanup_lowering.cpp::creates_temporaries. See NOTICE.
//! Full-expression queries use the original operands and selected default arguments.
use super::{Error, function::Function};
use crate::{
    sema::lowering::{CallTarget, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy, Default)]
pub(super) struct TemporarySummary {
    pub made: bool,
    pub call_result: bool,
    pub caller_destroyed: bool,
}
#[derive(Clone, Copy)]
pub(super) enum ExpressionUse {
    Ordinary,
    Value,
    ClassResult,
}
impl Function<'_, '_, '_, '_> {
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
            self.operand_temporaries(root)?
        } else {
            self.expression_temporaries(root)?
        };
        if !summary.made && self.lifetime.cleanups.is_empty() {
            return Ok(false);
        }
        let throwing = self.expression_throws(root)?;
        if (summary.made || !self.lifetime.cleanups.is_empty())
            && (throwing || summary.call_result || summary.caller_destroyed)
        {
            self.lifetime.full_expression = true;
            self.lifetime.temporary_segment = super::lifetime::TemporarySegment::new(
                summary.made,
                !matches!(usage, ExpressionUse::Ordinary) && throwing,
            );
            self.ensure_insertion()?;
            self.lifetime.expression_start = Some(self.builder.position()?);
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
        let summary = self.scan_temporaries(root, true)?;
        self.lifetime.temporary_queries.insert(root, summary);
        Ok(summary)
    }
    pub(super) fn operand_temporaries(
        &mut self,
        root: ast::ExprId,
    ) -> Result<TemporarySummary, Error> {
        self.scan_temporaries(root, false)
    }
    fn scan_temporaries(
        &mut self,
        root: ast::ExprId,
        include_root: bool,
    ) -> Result<TemporarySummary, Error> {
        let mut work = std::mem::take(&mut self.lifetime.temporary_work);
        let result = self.scan_temporary_operands(root, include_root, &mut work);
        work.clear();
        self.lifetime.temporary_work = work;
        result
    }
    fn scan_temporary_operands(
        &self,
        root: ast::ExprId,
        include_root: bool,
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
                    summary.call_result = true;
                }
                step += 1;
            }
            if facts.folded.is_some() {
                continue;
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
                summary.call_result |=
                    call || operator || self.graph.lower_literal_call(id)?.is_some();
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
