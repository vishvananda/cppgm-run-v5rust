// Translated from sema/analyzer_bodies.cpp. See NOTICE.
//! Function-local return and procedural control contexts, released with each body.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::ScopeId,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::ast::{self, Condition},
};
pub(super) struct BodyContext {
    pub result: TypeId,
    pub loops: usize,
    pub switches: Vec<SwitchContext>,
}
pub(super) struct SwitchContext {
    ty: TypeId,
    values: Vec<u64>,
    has_default: bool,
    initialized: u32,
}
impl BodyContext {
    pub(super) fn new(result: TypeId) -> Self {
        Self {
            result,
            loops: 0,
            switches: Vec::new(),
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
    pub(super) fn note_local_initializer(&mut self) {
        if let Some(context) = self
            .body_context
            .as_mut()
            .and_then(|body| body.switches.last_mut())
        {
            context.initialized = context.initialized.wrapping_add(1);
        }
    }
    pub(super) fn semantic_return(
        &mut self,
        stmt: ast::StmtId,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
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
        let facts = self.typed_expression(id, scope)?;
        if void {
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
            self.convert_scalar(id, result, false)
        }
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
                if context.initialized != 0 {
                    return Err(Error::at(
                        stmt.location,
                        "default label bypasses a variable's initialization",
                    ));
                }
                if context.has_default {
                    return Err(Error::at(stmt.location, "duplicate default label"));
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
        self.typed_expression(value, scope)?;
        self.convert_scalar(value, ty, false)?;
        let constant = self
            .constant_value(value, scope, true)?
            .ok_or(Error::Invariant("case without constant"))?;
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
        switch: Option<TypeId>,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() {
            return self.substatement(id, scope);
        }
        let body = self
            .body_context
            .as_mut()
            .ok_or(Error::Invariant("loop without body"))?;
        if let Some(ty) = switch {
            body.switches.push(SwitchContext {
                ty,
                values: Vec::new(),
                has_default: false,
                initialized: 0,
            });
        } else {
            body.loops += 1;
        }
        let result = self.substatement(id, scope);
        let body = self
            .body_context
            .as_mut()
            .ok_or(Error::Invariant("loop context disappeared"))?;
        if switch.is_some() {
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
        if !self.semantic() {
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
            if self.integral_kind(facts.ty).is_none() {
                return Err(Error::at(
                    location,
                    "switch condition is not of integral or enumeration type",
                ));
            }
            let ty = self.promoted_type(facts.ty)?;
            if let Some(expr) = expr {
                self.convert_operand_scalar(expr, ty, false)?;
            }
            Ok(Some(ty))
        } else {
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            if let Some(expr) = expr {
                self.convert_bool(expr)?;
            } else if self.standard_conversion(facts, ty, true)?.is_none() {
                return Err(Error::at(
                    location,
                    "condition is not contextually convertible to bool",
                ));
            }
            Ok(None)
        }
    }
    pub(super) fn reject_extended_statement(&self, id: ast::StmtId) -> Result<(), Error> {
        if self.semantic()
            && matches!(
                self.tree.statements.get(id).kind,
                ast::StmtKind::RangeFor { .. }
                    | ast::StmtKind::Try { .. }
                    | ast::StmtKind::Throw(_)
                    | ast::StmtKind::Goto(_)
                    | ast::StmtKind::Label { .. }
            )
        {
            return Err(Error::Missing("extended semantic statements"));
        }
        Ok(())
    }
    pub(super) fn is_switch(keyword: Keyword) -> bool {
        keyword == Keyword::Switch
    }
}
