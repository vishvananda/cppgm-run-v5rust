// Translated from sema/analyzer_initialization.cpp::aggregate_initialize/members. See NOTICE.
//! Plain aggregate lists initialize original member/element nodes in declaration order.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::ValueCategory,
    graph::{EntityId, Graph, ScopeId},
    types::{TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Aggregation {
    Eligible,
    Ineligible,
}
impl Graph {
    pub(super) fn aggregate_class(&self, class: EntityId) -> bool {
        let entity = self.entity(class);
        entity.aggregation == Aggregation::Eligible && entity.bases.is_empty()
    }
}
impl Analyzer<'_> {
    pub(super) fn class_list_initialize(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        self.check_stack()?;
        let TypeKind::Class(class) = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("aggregate without class"));
        };
        let ast::ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("aggregate without list"));
        };
        if !self.graph.aggregate_class(class) {
            self.initialize_class_object(
                target,
                ast::Initializer {
                    location: self.tree.expressions.get(id).location,
                    kind: ast::InitializerKind::Assign(id),
                },
                scope,
                self.tree.expressions.get(id).location,
            )?;
            return Ok(target);
        }
        let location = self.tree.expressions.get(id).location;
        let mut next = 0;
        self.aggregate_members(elements, &mut next, class, scope, location)?;
        if let Some(&extra) = self.tree.expr_lists.get(elements).get(next) {
            return Err(Error::at(
                self.tree.expressions.get(extra).location,
                "too many initializers for the aggregate",
            ));
        }
        let bare = self.bare_type(target)?;
        self.publish_list(id, bare, ValueCategory::Prvalue)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        facts.class_initialization = Some(super::expression_facts::ClassInitialization::Aggregate(
            elements,
        ));
        self.graph.expression_facts.set(id, facts);
        Ok(bare)
    }
    pub(super) fn aggregate_members(
        &mut self,
        elements: List<ast::ExprId>,
        next: &mut usize,
        class: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if !self.graph.aggregate_class(class) {
            return Err(Error::Missing("nonaggregate subobject list initialization"));
        }
        let union = self.graph.entity(class).key == Keyword::Union;
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if self.graph.unnamed_bit_field(member) {
                continue;
            }
            let ty = self.graph.lower_entity_type(member)?;
            self.aggregate_array_element(elements, next, ty, scope, location)?;
            if union {
                break;
            }
        }
        Ok(())
    }
    pub(super) fn missing_aggregate_element(
        &mut self,
        target: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let mut target = target;
        while let TypeKind::Array { element, .. } = self.graph.types.get(target).kind {
            target = element;
        }
        match self.graph.types.get(target).kind {
            TypeKind::Reference { .. } => Err(Error::at(
                location,
                "a reference member without an initializer",
            )),
            TypeKind::Class(class) if !self.graph.aggregate_class(class) => {
                let constructor = self.default_constructor(class, location)?;
                self.validate_constructor_use(constructor, scope, location)?;
                self.demand_constructor_use(constructor);
                Ok(())
            }
            TypeKind::Class(class) => {
                // Reuse one borrowed empty child slice; no synthetic syntax is published.
                let empty = List::default();
                self.aggregate_members(empty, &mut 0, class, scope, SourceOffset(0))
            }
            _ => Ok(()),
        }
    }
}
