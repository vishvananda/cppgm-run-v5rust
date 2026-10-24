// Translated from sema/analyzer_bodies.cpp. See NOTICE.
//! Function-local return and procedural control contexts, released with each body.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{BindingKind, ScopeId, ScopeKind},
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::ast::{self, Condition},
};
pub(super) struct BodyContext {
    pub return_deduction: super::return_deduction::ReturnDeduction,
    pub placeholder: Option<TypeId>,
    pub undeduced: bool,
    pub reserved_pending: bool,
    pub entity: super::graph::EntityId,
    pub result: TypeId,
    pub loops: usize,
    pub switches: Vec<SwitchContext>,
    pub transfers: super::jumps::Transfers,
    pub try_parent: Option<ScopeId>,
    pub handled: u32,
    pub constructor_handlers: bool,
}
pub(super) struct SwitchContext {
    ty: Option<TypeId>,
    values: Vec<u64>,
    has_default: bool,
    initialized: u32,
    handled: u32,
}
#[derive(Clone, Copy)]
pub(super) enum Control {
    Loop,
    Switch(Option<TypeId>),
}
impl BodyContext {
    pub(super) fn new(result: TypeId, entity: super::graph::EntityId) -> Self {
        Self {
            return_deduction: super::return_deduction::ReturnDeduction::Fixed,
            placeholder: None,
            undeduced: false,
            reserved_pending: false,
            entity,
            result,
            loops: 0,
            switches: Vec::new(),
            transfers: super::jumps::Transfers::default(),
            try_parent: None,
            handled: 0,
            constructor_handlers: false,
        }
    }
}
impl Analyzer<'_> {
    pub(super) fn switch_initialized(&self) -> Option<u32> {
        self.body_context
            .as_ref()?
            .switches
            .last()
            .map(|context| context.initialized)
    }
    pub(super) fn restore_switch_initialized(&mut self, count: Option<u32>) {
        if let Some(count) = count
            && let Some(context) = self
                .body_context
                .as_mut()
                .and_then(|body| body.switches.last_mut())
        {
            context.initialized = count;
        }
    }
    pub(super) fn note_local_initializer(&mut self, scope: ScopeId) -> Result<(), Error> {
        let initialized = self.graph.scope(scope).initialized;
        self.graph.scope_mut(scope).initialized = initialized
            .checked_add(1)
            .ok_or(Error::Limit("local initialization count limit"))?;
        if let Some(context) = self
            .body_context
            .as_mut()
            .and_then(|body| body.switches.last_mut())
        {
            context.initialized = context
                .initialized
                .checked_add(1)
                .ok_or(Error::Limit("switch initialization count limit"))?;
        }
        Ok(())
    }
    pub(super) fn semantic_return(
        &mut self,
        stmt: ast::StmtId,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if self
            .body_context
            .as_ref()
            .is_some_and(|body| body.constructor_handlers)
        {
            return Err(Error::at(
                self.tree.statements.get(stmt).location,
                "a return statement in a handler of a constructor's function-try-block (15.3p14)",
            ));
        }
        let typed = self.deduce_return(stmt, operand, scope)?;
        let result = self
            .body_context
            .as_ref()
            .ok_or(Error::Invariant("return without body context"))?
            .result;
        let void = matches!(
            self.graph.types.get(result).kind,
            TypeKind::Scalar(Scalar::Void)
        );
        let location = self.tree.statements.get(stmt).location;
        let Some(id) = operand else {
            return if void {
                Ok(())
            } else {
                Err(Error::at(
                    location,
                    "return without a value in a function returning a value",
                ))
            };
        };
        if matches!(
            self.tree.expressions.get(id).kind,
            ast::ExprKind::Braced { .. }
        ) {
            if void || matches!(self.graph.types.get(result).kind, TypeKind::Array { .. }) {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "returning a braced-init-list for this result",
                ));
            }
            self.list_initialize(id, result, scope)?;
            return Ok(());
        }
        let facts = if let Some(facts) = typed {
            facts
        } else {
            self.typed_expression(id, scope)?
        };
        if void {
            self.check_resolved_function(id)?;
            if matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Scalar(Scalar::Void)
            ) {
                Ok(())
            } else {
                Err(Error::at(
                    location,
                    "return with a value in a function returning void",
                ))
            }
        } else {
            let slicing = self
                .graph
                .class_of(result)
                .zip(self.graph.class_of(facts.ty))
                .is_some_and(|(target, source)| {
                    target != source && self.graph.is_base_of(target, source)
                });
            if !slicing {
                self.convert_scalar(id, result, false)?;
            }
            self.record_class_copy(
                id,
                result,
                scope,
                super::copy_initialization::CopyUse::Return {
                    implicit_move: self.graph.class_of(result).is_some()
                        && self.names_automatic_object(id)?,
                },
                location,
            )
        }
    }
    pub(super) fn names_automatic_object(&self, id: ast::ExprId) -> Result<bool, Error> {
        let root = self.graph.expression_facts.root(id);
        let ast::ExprKind::Id(name) = self.tree.expressions.get(root).kind else {
            return Ok(false);
        };
        let Some(binding) = self.graph.resolved_name(name) else {
            return Ok(false);
        };
        let binding = self.graph.binding(binding);
        let entity = self.graph.entity(binding.entity);
        let Some(ty) = entity.ty else {
            return Ok(false);
        };
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            || self.graph.types.get(ty).cv.0 & 2 != 0
        {
            return Ok(false);
        }
        let function = self
            .body_context
            .as_ref()
            .ok_or(Error::Invariant("return eligibility without body context"))?
            .entity;
        let Some(function_scope) = self.graph.entity(function).scope else {
            return Err(Error::Invariant(
                "return eligibility without function scope",
            ));
        };
        let Some(home) = entity.home else {
            return Ok(false);
        };
        if !self.graph.encloses(function_scope, home) {
            return Ok(false);
        }
        Ok(binding.kind == BindingKind::Parameter
            || binding.kind == BindingKind::Variable
                && self.graph.scope(home).kind == ScopeKind::Block
                && entity.variable.is_some_and(|variable| variable.automatic))
    }
    pub(super) fn semantic_jump(&mut self, id: ast::StmtId) -> Result<(), Error> {
        let body = self
            .body_context
            .as_mut()
            .ok_or(Error::Invariant("jump without function"))?;
        let stmt = self.tree.statements.get(id);
        match stmt.kind {
            ast::StmtKind::Break if body.loops == 0 && body.switches.is_empty() => Err(Error::at(
                stmt.location,
                "break outside a loop or switch statement",
            )),
            ast::StmtKind::Continue if body.loops == 0 => {
                Err(Error::at(stmt.location, "continue outside a loop"))
            }
            ast::StmtKind::Default(_) => {
                let context = body.switches.last_mut().ok_or_else(|| {
                    Error::at(stmt.location, "default label outside a switch statement")
                })?;
                if context.has_default {
                    return Err(Error::at(stmt.location, "duplicate default label"));
                }
                if context.initialized != 0 {
                    return Err(Error::at(
                        stmt.location,
                        "default label bypasses a variable's initialization",
                    ));
                }
                if context.handled != body.handled {
                    return Err(Error::at(
                        stmt.location,
                        "default label inside a try block, handler (15p2) or statement expression the switch encloses",
                    ));
                }
                context.has_default = true;
                Ok(())
            }
            _ => Ok(()),
        }
    }
    pub(super) fn semantic_case(
        &mut self,
        id: ast::StmtId,
        value: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ty = self
            .body_context
            .as_ref()
            .and_then(|body| body.switches.last())
            .map(|context| context.ty)
            .ok_or_else(|| {
                Error::at(
                    self.tree.statements.get(id).location,
                    "case label outside a switch statement",
                )
            })?;
        if self.switch_initialized().is_some_and(|count| count != 0) {
            return Err(Error::at(
                self.tree.statements.get(id).location,
                "case label bypasses a variable's initialization",
            ));
        }
        let body = self
            .body_context
            .as_ref()
            .ok_or(Error::Invariant("case without body"))?;
        if body
            .switches
            .last()
            .is_some_and(|context| context.handled != body.handled)
        {
            return Err(Error::at(
                self.tree.statements.get(id).location,
                "case label inside a try block, handler (15p2) or statement expression the switch encloses",
            ));
        }
        let Some(ty) = ty else {
            return self.expression(value, scope);
        };
        self.integral_constant_operand(value, scope, Some(ty))?;
        let constant = self
            .constant_value(value, scope, true)?
            .ok_or(Error::Invariant("case without constant"))?;
        self.convert_operand_scalar(value, ty, false)?;
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("nonintegral switch"))?;
        let from = self
            .integral_kind(constant.ty)
            .ok_or(Error::Invariant("nonintegral case"))?;
        if !matches!(self.graph.types.get(constant.ty).kind, TypeKind::Enum(_))
            && !kind.represents(constant.bits, from)
        {
            return Err(Error::at(
                self.tree.expressions.get(value).location,
                "case value narrowed by conversion to the switch condition's type",
            ));
        }
        let bits = kind.convert(constant.bits);
        super::record(&mut self.graph.case_values, id.index(), bits);
        self.body_context
            .as_mut()
            .and_then(|body| body.switches.last_mut())
            .ok_or(Error::Invariant("switch disappeared"))?
            .values
            .push(bits);
        Ok(())
    }
    pub(super) fn control_substatement(
        &mut self,
        id: ast::StmtId,
        scope: ScopeId,
        control: Control,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() {
            return self.substatement(id, scope);
        }
        let body = self
            .body_context
            .as_mut()
            .ok_or(Error::Invariant("loop without body"))?;
        if let Control::Switch(ty) = control {
            body.switches.push(SwitchContext {
                ty,
                values: Vec::new(),
                has_default: false,
                initialized: 0,
                handled: body.handled,
            });
        } else {
            body.loops += 1;
        }
        let result = self.substatement(id, scope);
        let body = self
            .body_context
            .as_mut()
            .ok_or(Error::Invariant("loop context disappeared"))?;
        if matches!(control, Control::Switch(_)) {
            let mut context = body
                .switches
                .pop()
                .ok_or(Error::Invariant("switch context disappeared"))?;
            result?;
            context.values.sort_unstable();
            if context
                .values
                .windows(2)
                .any(|values| values[0] == values[1])
            {
                return Err(Error::at(location, "duplicate case value"));
            }
        } else {
            body.loops -= 1;
            result?;
        }
        Ok(())
    }
    pub(super) fn condition_conversion(
        &mut self,
        condition: Condition,
        switch: bool,
    ) -> Result<Option<TypeId>, Error> {
        if !self.semantic() || self.template_depth != 0 || self.reserved_context.pattern {
            return Ok(None);
        }
        if let Condition::Expression(id) = condition
            && self.graph.reserved_deferrals.expression(id)
        {
            return Ok(None);
        }
        let (facts, expr, location) = match condition {
            Condition::Expression(id) => (
                self.graph.expression_facts.get(id)?,
                Some(id),
                self.tree.expressions.get(id).location,
            ),
            Condition::Declaration { declarator, .. } => {
                let binding = self
                    .graph
                    .declared_bindings
                    .get(declarator.index())
                    .copied()
                    .flatten()
                    .ok_or(Error::Invariant("condition without binding"))?;
                let ty = self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .ty
                    .ok_or(Error::Invariant("condition without type"))?;
                let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind
                {
                    element
                } else {
                    ty
                };
                (
                    ExprFacts::new(ty, V::Lvalue),
                    None,
                    self.tree.declarators.get(declarator).location,
                )
            }
        };
        if switch {
            self.switch_condition_conversion(condition, facts, location)
                .map(Some)
        } else {
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            if let Some(expr) = expr {
                self.convert_bool(expr)?;
            } else if let Condition::Declaration { declarator, .. } = condition {
                let conversion = if let Some(conversion) =
                    self.standard_conversion(facts, ty, true)?
                {
                    conversion
                } else if let Some(sequence) = self.conversion_function_sequence(facts, ty, true)? {
                    let binding = self.graph.declared_bindings[declarator.index()]
                        .ok_or(Error::Invariant("condition without binding"))?;
                    let scope = self.graph.binding_scope(binding);
                    let selected = self.select_conversion_function(
                        facts,
                        sequence,
                        scope,
                        location,
                        super::conversion_functions::ConversionUse::Contextual,
                    )?;
                    self.graph.record_condition_function(declarator, selected);
                    sequence.conversion
                } else {
                    return Err(Error::at(
                        location,
                        "condition is not contextually convertible to bool",
                    ));
                };
                self.graph
                    .record_condition_conversion(declarator, conversion);
            }
            Ok(None)
        }
    }

    fn switch_condition_conversion(
        &mut self,
        condition: Condition,
        facts: ExprFacts,
        location: crate::support::source::SourceOffset,
    ) -> Result<TypeId, Error> {
        use super::conversion_functions::{ConversionTarget, ConversionUse};
        let diagnostic = "switch condition is not of integral or enumeration type";
        let (source, scope) = match condition {
            Condition::Expression(id) => {
                let scope = self.graph.expression_scopes
                    [self.graph.expression_facts.root(id).index()]
                .ok_or(Error::Invariant("condition without expression scope"))?;
                let produced = self.convert_unique_class_operand(
                    id,
                    ConversionTarget::IntegralOrEnumeration,
                    scope,
                )?;
                (produced.ty, scope)
            }
            Condition::Declaration { declarator, .. } => {
                let binding = self.graph.declared_bindings[declarator.index()]
                    .ok_or(Error::Invariant("condition without binding"))?;
                let mut source = facts.ty;
                if let Some(class) = self.graph.class_of(source)
                    && let Some(converter) = self.unique_conversion_function(
                        class,
                        ConversionTarget::IntegralOrEnumeration,
                    )?
                {
                    let signature = self
                        .graph
                        .binding(converter)
                        .ty
                        .ok_or(Error::Invariant("conversion signature"))?;
                    source = self.call_result(signature)?.ty;
                }
                (source, self.graph.binding_scope(binding))
            }
        };
        if self.integral_kind(source).is_none() {
            return Err(Error::at(location, diagnostic));
        }
        let source = self.bare_type(source)?;
        let target = self.promoted_type(source)?;
        match condition {
            Condition::Expression(id) => self.convert_operand_scalar(id, target, false)?,
            Condition::Declaration { declarator, .. } => {
                let conversion =
                    if let Some(conversion) = self.standard_conversion(facts, target, false)? {
                        conversion
                    } else if let Some(sequence) =
                        self.conversion_function_sequence(facts, target, false)?
                    {
                        let selected = self.select_conversion_function(
                            facts,
                            sequence,
                            scope,
                            location,
                            ConversionUse::Contextual,
                        )?;
                        self.graph.record_condition_function(declarator, selected);
                        sequence.conversion
                    } else {
                        return Err(Error::at(location, diagnostic));
                    };
                self.graph
                    .record_condition_conversion(declarator, conversion);
            }
        }
        Ok(target)
    }
    pub(super) fn is_switch(keyword: Keyword) -> bool {
        keyword == Keyword::Switch
    }
}
