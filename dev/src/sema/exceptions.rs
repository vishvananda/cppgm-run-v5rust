// Translated from sema/analyzer_exceptions.cpp and analyzer_exception_sets.cpp. See NOTICE.
//! Exception specifications belong to declarations, never canonical function types.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    graph::{EntityId, ScopeId},
    types::{RefKind, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind, Qualifier, QualifierKind},
};

#[derive(Clone, Copy, Debug)]
pub(super) struct SpecificationId(u32);
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ExceptionSet {
    All,
    None,
    Dynamic(Vec<TypeId>),
}
#[derive(Clone, Debug)]
pub(super) enum ExceptionState {
    Absent,
    Pending(SpecificationId),
    Evaluating,
    Value(ExceptionSet),
}
#[derive(Clone, Copy)]
pub(super) struct PendingSpecification {
    entity: EntityId,
    qualifier: Option<Qualifier>,
    scope: ScopeId,
    limit: usize,
    location: SourceOffset,
    first: bool,
    done: bool,
}

impl Analyzer<'_> {
    pub(super) fn declarator_specifies_exceptions(
        &self,
        mut id: Option<ast::DeclaratorId>,
    ) -> bool {
        while let Some(current) = id {
            let node = self.tree.declarators.get(current);
            for suffix in self.tree.suffixes.get(node.suffixes) {
                if let ast::Suffix::Function { qualifiers, .. } = suffix
                    && self.tree.qualifiers.get(*qualifiers).iter().any(|q| {
                        matches!(q.kind, QualifierKind::Throw(_) | QualifierKind::Noexcept(_))
                    })
                {
                    return true;
                }
            }
            id = node.nested;
        }
        false
    }
    pub(super) fn record_function_exceptions(
        &mut self,
        entity: EntityId,
        info: &DeclaratorInfo,
        mut scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let qualifier =
            info.qualifiers.and_then(|list| {
                self.tree.qualifiers.get(list).iter().copied().find(|q| {
                    matches!(q.kind, QualifierKind::Throw(_) | QualifierKind::Noexcept(_))
                })
            });
        let first = matches!(self.graph.entity(entity).exceptions, ExceptionState::Absent);
        let operand = qualifier.and_then(|q| {
            if let QualifierKind::Noexcept(expr) = q.kind {
                expr
            } else {
                None
            }
        });
        if !first
            && self.class_depth != 0
            && (operand.is_some()
                || matches!(
                    self.graph.entity(entity).exceptions,
                    ExceptionState::Pending(_)
                ))
        {
            // C++ defers comparison when a member's operand needs its class.
            return Ok(());
        }
        if operand.is_some()
            && let Some(clause) = info.parameters
        {
            let prototype =
                self.graph
                    .add_scope(super::graph::ScopeKind::Function, scope, None, None)?;
            let TypeKind::Function { parameters, .. } = self.graph.types.get(info.ty).kind.clone()
            else {
                return Err(Error::Invariant("exception specification on no function"));
            };
            for (&parameter, ty) in self
                .tree
                .parameters
                .get(clause.parameters)
                .iter()
                .zip(parameters)
            {
                self.declare_parameter_object(parameter, ty, prototype)?;
            }
            scope = prototype;
        }
        let id = SpecificationId(
            u32::try_from(self.specifications.len())
                .map_err(|_| Error::Limit("exception specification limit"))?,
        );
        self.specifications.push(PendingSpecification {
            entity,
            qualifier,
            scope,
            limit: self.graph.binding_limit(),
            location,
            first,
            done: false,
        });
        if first {
            self.graph.entity_mut(entity).exceptions = ExceptionState::Pending(id);
        }
        let member_throw = self.class_depth != 0
            && qualifier.is_some_and(
                |q| matches!(q.kind, QualifierKind::Throw(types) if !types.is_empty()),
            );
        if member_throw {
            self.pending_throws.push(id);
        }
        let literal_true = operand.is_some_and(|mut expr| {
            while let ExprKind::Paren(inner) = self.tree.expressions.get(expr).kind {
                expr = inner;
            }
            matches!(
                self.tree.expressions.get(expr).kind,
                ExprKind::Keyword(Keyword::True)
            )
        });
        if !first || (!member_throw && (operand.is_none() || literal_true)) {
            self.resolve_specification(id)?;
        }
        Ok(())
    }
    fn resolve_specification(&mut self, id: SpecificationId) -> Result<(), Error> {
        let pending = self.specifications[id.0 as usize];
        if pending.done {
            return Ok(());
        }
        if pending.first {
            self.graph.entity_mut(pending.entity).exceptions = ExceptionState::Evaluating;
        } else if let ExceptionState::Pending(first) = self.graph.entity(pending.entity).exceptions
        {
            self.resolve_specification(first)?;
        }
        let saved_limit = self.graph.lookup_limit;
        self.graph.lookup_limit = Some(pending.limit);
        let result = self.exception_set(pending.qualifier, pending.scope);
        self.graph.lookup_limit = saved_limit;
        let result = result?;
        if pending.first {
            self.graph.entity_mut(pending.entity).exceptions = ExceptionState::Value(result);
        } else {
            let ExceptionState::Value(first) = &self.graph.entity(pending.entity).exceptions else {
                return Err(Error::Invariant("unresolved first exception specification"));
            };
            if *first != result {
                return Err(Error::at(
                    pending.location,
                    "redeclaration with an incompatible exception-specification",
                ));
            }
        }
        self.specifications[id.0 as usize].done = true;
        Ok(())
    }
    fn exception_set(
        &mut self,
        qualifier: Option<Qualifier>,
        scope: ScopeId,
    ) -> Result<ExceptionSet, Error> {
        match qualifier.map(|q| q.kind) {
            None => Ok(ExceptionSet::All),
            Some(QualifierKind::Noexcept(operand)) => {
                let nothrow = if let Some(operand) = operand {
                    let saved = self.evaluating_noexcept;
                    self.evaluating_noexcept = true;
                    let result = self.constant_value(operand, scope, true);
                    self.evaluating_noexcept = saved;
                    let value = result?.ok_or(Error::Invariant("missing noexcept value"))?;
                    if self.scoped_enum(value.ty) {
                        return Err(Error::at(
                            self.tree.expressions.get(operand).location,
                            "invalid implicit conversion",
                        ));
                    }
                    value.bits != 0
                } else {
                    true
                };
                Ok(if nothrow {
                    ExceptionSet::None
                } else {
                    ExceptionSet::All
                })
            }
            Some(QualifierKind::Throw(types)) => {
                let mut adjusted = Vec::new();
                for &syntax in self.tree.type_lists.get(types) {
                    let ty = self.type_syntax(syntax, scope)?;
                    let location = self.tree.types.get(syntax).location;
                    if matches!(
                        self.graph.types.get(ty).kind,
                        TypeKind::Reference {
                            kind: RefKind::Rvalue,
                            ..
                        }
                    ) {
                        return Err(Error::at(
                            location,
                            "an rvalue reference type in an exception-specification",
                        ));
                    }
                    let ty = self.graph.types.parameter(ty)?;
                    self.check_complete_throw_type(ty, location)?;
                    adjusted.push(self.graph.types.canonical_function(ty)?);
                }
                adjusted.sort_unstable();
                adjusted.dedup();
                Ok(if adjusted.is_empty() {
                    ExceptionSet::None
                } else {
                    ExceptionSet::Dynamic(adjusted)
                })
            }
            _ => Err(Error::Invariant("invalid exception qualifier")),
        }
    }
    fn check_complete_throw_type(
        &self,
        mut ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let pointer = !reference && matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_));
        if pointer && let TypeKind::Pointer(element) = self.graph.types.get(ty).kind {
            ty = element;
        }
        match self.graph.types.get(ty).kind {
            TypeKind::Scalar(super::types::Scalar::Void) if !reference && !pointer => Err(
                Error::at(location, "void in an exception-specification (15.4p2)"),
            ),
            TypeKind::Class(entity) if !self.graph.entity(entity).complete => Err(Error::at(
                location,
                "an incomplete class type in an exception-specification (15.4p2)",
            )),
            _ => Ok(()),
        }
    }
    pub(super) fn finish_class_exceptions(&mut self) -> Result<(), Error> {
        let pending = std::mem::take(&mut self.pending_throws);
        for id in pending {
            self.resolve_specification(id)?;
        }
        Ok(())
    }
    pub(super) fn finish_unit_exceptions(&mut self) -> Result<(), Error> {
        for index in 0..self.specifications.len() {
            self.resolve_specification(SpecificationId(
                u32::try_from(index).map_err(|_| Error::Limit("exception specification limit"))?,
            ))?;
        }
        Ok(())
    }
}
