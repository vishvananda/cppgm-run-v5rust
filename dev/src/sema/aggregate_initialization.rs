// Translated from sema/analyzer_initialization.cpp::aggregate_initialize/members. See NOTICE.
//! Plain aggregate lists initialize original member/element nodes in declaration order.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ClassInitialization, ValueCategory},
    graph::{EntityId, Graph, ScopeId},
    types::{TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ArrayMemberKind {
    #[default]
    Ordinary,
    ZeroLength,
}
// One variable initializer owns permission to extend static storage. Nested
// aggregate lists and temporaries cannot inherit that permission.
pub(super) struct FlexibleInitialization {
    pub list: ast::ExprId,
    pub elements: List<ast::ExprId>,
    pub class: EntityId,
    pub given: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Aggregation {
    Eligible,
    Ineligible,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AggregateInitialization {
    pub elements: List<ast::ExprId>,
    pub union_member: Option<EntityId>,
}
impl From<List<ast::ExprId>> for AggregateInitialization {
    fn from(elements: List<ast::ExprId>) -> Self {
        Self {
            elements,
            union_member: None,
        }
    }
}
impl Graph {
    pub(crate) fn lower_list_union_member(&self, id: ast::ExprId) -> Option<EntityId> {
        match self
            .expression_facts
            .get(id)
            .ok()?
            .class_initialization(&self.expression_facts)?
        {
            super::expression_facts::ClassInitialization::Aggregate(elements)
            | super::expression_facts::ClassInitialization::FlexibleAggregate(elements) => {
                self.lower_aggregate_union_member(elements)
            }
            _ => None,
        }
    }
    pub(crate) fn lower_aggregate_union_member(
        &self,
        elements: List<ast::ExprId>,
    ) -> Option<EntityId> {
        self.expression_facts.union_list_member(elements)
    }
    pub(crate) fn lower_selected_union_member(
        &self,
        class: EntityId,
        selected: Option<EntityId>,
    ) -> Option<EntityId> {
        selected.filter(|&member| self.entity(member).home == self.entity(class).scope)
    }

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
    // aggregates.h::InitializesSubaggregate: a class expression initializes
    // that class, and a string literal initializes an array as one element.
    pub(super) fn initializes_subaggregate(
        &self,
        target: TypeId,
        expr: ast::ExprId,
    ) -> Result<bool, Error> {
        let facts = self.graph.expression_facts.get(expr)?;
        Ok(match self.graph.types.get(target).kind {
            TypeKind::Array { .. } => {
                self.string_literal(expr)
                    && matches!(self.graph.types.get(facts.ty).kind, TypeKind::Array { .. })
            }
            _ => {
                self.graph.class_of(facts.ty).is_some()
                    && self.graph.class_of(facts.ty) == self.graph.class_of(target)
            }
        })
    }

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
        if !self.complete_class(class)? {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "initialization of an incomplete class object",
            ));
        }
        let ast::ExprKind::Braced { .. } = self.tree.expressions.get(id).kind else {
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
        let plan = self
            .designated_list(id, class, Some(scope))?
            .ok_or(Error::Invariant("checked designated list"))?;
        let elements = plan.elements;
        if let Some(context) = &mut self.flexible_initialization
            && context.list == id
        {
            context.elements = elements;
        }
        if let ast::ExprKind::Braced {
            designators,
            trailing_comma,
            ..
        } = self.tree.expressions.get(id).kind
            && !designators.is_empty()
        {
            self.tree.expressions.replace(
                id,
                ast::Expression {
                    location,
                    kind: ast::ExprKind::Braced {
                        elements,
                        designators: List::default(),
                        trailing_comma,
                    },
                },
            );
        }
        let mut next = 0;
        self.aggregate_members(
            elements,
            &mut next,
            class,
            scope,
            location,
            plan.union_member,
        )?;
        if let Some(&extra) = self.tree.expr_lists.get(elements).get(next) {
            return Err(Error::at(
                self.tree.expressions.get(extra).location,
                "too many initializers for the aggregate",
            ));
        }
        let bare = self.bare_type(target)?;
        self.publish_list(id, bare, ValueCategory::Prvalue)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        if let Some(member) = plan.union_member {
            self.graph
                .expression_facts
                .set_union_list_member(elements, member);
        }
        facts.set_class_initialization(
            Some(
                if let Some(context) = &mut self.flexible_initialization
                    && context.list == id
                    && context.given
                {
                    context.given = false;
                    ClassInitialization::FlexibleAggregate(elements)
                } else {
                    ClassInitialization::Aggregate(elements)
                },
            ),
            &mut self.graph.expression_facts,
        )?;
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
        selection: Option<EntityId>,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if !self.graph.aggregate_class(class) {
            return Err(Error::Missing("nonaggregate subobject list initialization"));
        }
        let union = self.graph.entity(class).key == Keyword::Union;
        let selected = union
            .then(|| {
                self.graph
                    .lower_selected_union_member(class, selection)
                    .or_else(|| {
                        self.graph.union_list_member(
                            class,
                            *next >= self.tree.expr_lists.get(elements).len(),
                        )
                    })
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
            if !union
                && self.hosted()
                && self.template_depth == 0
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Array {
                        bound: 0,
                        operand: None,
                        ..
                    }
                )
                && let Some(&element) = self.tree.expr_lists.get(elements).get(*next)
                && !matches!(self.tree.expressions.get(element).kind,
                    ast::ExprKind::Braced { elements, .. } if elements.is_empty())
            {
                let at = self.tree.expressions.get(element).location;
                if self.graph.entity(member).array_member == ArrayMemberKind::ZeroLength {
                    return Err(Error::at(
                        at,
                        "a zero-length array member initialized with elements",
                    ));
                }
                let context = self.flexible_initialization.as_mut().filter(|context| {
                    context.class == class && context.elements == elements
                        && index + 1 == self.graph.entity(class).members.len()
                }).ok_or_else(|| Error::at(at,
                    "a flexible array member initialized outside the initializer of an object of static storage duration"))?;
                context.given = true;
            }
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
                self.validate_constructor_use(constructor, class, scope, location)?;
                self.demand_constructor_use(constructor)?;
                // Value-initializing an omitted aggregate element calls its
                // constructor, even for an anonymous member object. A written
                // enclosing constructor instead executes the nested plan.
                if self.unevaluated == 0 && self.graph.anonymous_member_storage(class).is_some() {
                    self.note_initializer_constructor(constructor);
                    self.record_demand(super::function_demands::Demand::Constructor(constructor));
                }
                Ok(())
            }
            TypeKind::Class(class) => {
                // Reuse one borrowed empty child slice; no synthetic syntax is published.
                let empty = List::default();
                self.aggregate_members(empty, &mut 0, class, scope, SourceOffset(0), None)
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
        assert!(first.constructor(&graph.expression_facts).is_none());
        assert!(
            first
                .class_initialization(&graph.expression_facts)
                .is_none()
        );
        let (constructor, ConstructorArguments::Single(original)) =
            graph.expression_facts.class_conversion(values[0]).unwrap()
        else {
            panic!("retained converting argument");
        };
        assert_eq!(original, values[0]);
        assert_eq!(
            Some(constructor),
            second.constructor(&graph.expression_facts)
        );
        assert!(graph.expression_facts.class_conversion(values[1]).is_none());
        assert!(second.class_copy(&graph.expression_facts).is_some());
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
