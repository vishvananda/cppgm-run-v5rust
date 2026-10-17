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
    // aggregates.h::UnionListMember: an empty list prefers the variant
    // initialized by a brace-or-equal-initializer, including anonymous storage.
    pub(super) fn union_list_member(&self, class: EntityId, empty: bool) -> Option<EntityId> {
        let mut first = None;
        for &member in &self.entity(class).members {
            if self.unnamed_bit_field(member) {
                continue;
            }
            if !empty {
                return Some(member);
            }
            let entity = self.entity(member);
            if entity.member_initializer.is_some()
                || entity
                    .ty
                    .and_then(|ty| self.class_of(ty))
                    .is_some_and(|child| {
                        self.anonymous_member_storage(child).is_some()
                            && self.entity(child).initialized_members != 0
                    })
            {
                return Some(member);
            }
            first.get_or_insert(member);
        }
        first
    }
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
        let selected = union
            .then(|| {
                self.graph
                    .union_list_member(class, *next >= self.tree.expr_lists.get(elements).len())
            })
            .flatten();
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if self.graph.unnamed_bit_field(member)
                || selected.is_some_and(|selected| member != selected)
            {
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
                // Value-initializing an omitted aggregate element calls its
                // constructor, even for an anonymous member object. A written
                // enclosing constructor instead executes the nested plan.
                if self.unevaluated == 0 && self.graph.anonymous_member_storage(class).is_some() {
                    if self.body_context.is_none()
                        && self.graph.constructors.get(constructor).nontrivial()
                        && let Some(entity) = self.initializing
                        && self.graph.entity(entity).data_member
                            != Some(super::graph::DataMemberKind::Instance)
                    {
                        self.graph
                            .initializer_constructors
                            .note(entity, constructor);
                    }
                    self.record_demand(super::function_demands::Demand::Constructor(constructor));
                }
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

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{
            Analyzer, constructor_initialization::ConstructorArguments, lookup::Filter,
            types::TypeKind,
        },
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{
            SyntaxUnit,
            ast::{DeclKind, ExprKind, InitializerKind},
        },
    };

    #[test]
    fn aggregate_conversions_preserve_source_and_constructor_identities() {
        let source = SourceBuffer::new(
            b"struct A{int n;A(int k):n(k){}};struct B{A a,b;};struct Unused{};B value{1,A(2)};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"aggregate-conversions").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(expressions, tree.expressions.mark());
        let declaration = *tree.decl_lists.get(declarations).last().unwrap();
        let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind else {
            panic!("object declaration");
        };
        let InitializerKind::Braced(list) = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap()
            .kind
        else {
            panic!("aggregate list");
        };
        let ExprKind::Braced { elements, .. } = tree.expressions.get(list).kind else {
            panic!("list");
        };
        let values = tree.expr_lists.get(elements);
        let first = graph.expression_facts.get(values[0]).unwrap();
        let second = graph.expression_facts.get(values[1]).unwrap();
        assert!(matches!(
            graph.types.get(first.ty).kind,
            TypeKind::Scalar(_)
        ));
        assert!(first.constructor.is_none());
        assert!(first.class_initialization.is_none());
        let (constructor, ConstructorArguments::Single(original)) =
            graph.expression_facts.class_conversion(values[0]).unwrap()
        else {
            panic!("retained converting argument");
        };
        assert_eq!(original, values[0]);
        assert_eq!(Some(constructor), second.constructor);
        assert!(graph.expression_facts.class_conversion(values[1]).is_none());
        assert!(second.class_copy.is_some());
        let unused = cursor.intern_name(b"Unused").unwrap();
        let unused = graph
            .binding(graph.own(graph.global, unused, Filter::Prefix).unwrap())
            .entity;
        assert!(matches!(
            graph.entity(unused).layout,
            crate::sema::layout::LayoutState::NotStarted
        ));
    }
}
