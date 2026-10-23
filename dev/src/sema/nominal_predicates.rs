// Translated from sema/analyzer_traits.cpp and analyzer_classes.cpp. See NOTICE.
//! Complete-class predicates reuse declaration facts without layout or emission demand.
use super::{
    Error,
    analyzer::Analyzer,
    graph::EntityId,
    inheritance::Derivation,
    types::{TypeId, TypeKind},
};
use crate::{
    lex::{predicates::TypePredicate as T, vocabulary::Keyword},
    support::source::SourceOffset,
};

impl Analyzer<'_> {
    pub(super) fn class_predicate_value(
        &mut self,
        predicate: T,
        ty: TypeId,
        second: Option<TypeId>,
        location: SourceOffset,
    ) -> Result<Option<bool>, Error> {
        if predicate == T::IsBaseOf {
            let derived = second.ok_or(Error::Invariant("base-of without derived type"))?;
            return self.base_of_predicate(ty, derived, location).map(Some);
        }
        if !matches!(
            predicate,
            T::IsEmpty
                | T::IsPolymorphic
                | T::IsAbstract
                | T::IsFinal
                | T::IsAggregate
                | T::HasVirtualDestructor
        ) {
            return Ok(None);
        }
        // The five declaration-property predicates leave array elements
        // unasked. HasVirtualDestructor completes the element class even
        // though its answer for an array itself is still false.
        let mut object = ty;
        if predicate == T::HasVirtualDestructor {
            while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
                object = element;
            }
        }
        if let Some(class) = self.graph.class_of(object)
            && !self.complete_class(class)?
        {
            return Err(Error::at(
                location,
                "a type trait of an incomplete class type",
            ));
        }
        // ClassType does not unwrap references or arrays. An array is always
        // an aggregate, even when its element class has never been completed.
        let Some(class) = self.graph.class_of(ty) else {
            return Ok(Some(
                predicate == T::IsAggregate
                    && matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. }),
            ));
        };
        let entity = self.graph.entity(class);
        let value = match predicate {
            T::IsEmpty => entity.key != Keyword::Union && self.predicate_class_empty(class)?,
            T::IsPolymorphic => entity.polymorphism.is_dynamic(),
            T::IsAbstract => self.graph.virtuals.abstract_class(class),
            T::IsFinal => entity.derivation == Derivation::Final,
            T::IsAggregate => {
                self.graph.aggregate_class(class) && !entity.polymorphism.is_dynamic()
            }
            T::HasVirtualDestructor => self.graph.virtuals.destructor(class).is_some(),
            _ => return Err(Error::Invariant("nonclass predicate in class dispatch")),
        };
        Ok(Some(value))
    }

    fn base_of_predicate(
        &mut self,
        base: TypeId,
        derived: TypeId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let Some((base, derived)) = self.graph.class_of(base).zip(self.graph.class_of(derived))
        else {
            return Ok(false);
        };
        if self.graph.entity(base).key == Keyword::Union
            || self.graph.entity(derived).key == Keyword::Union
        {
            return Ok(false);
        }
        if base == derived {
            return Ok(true);
        }
        if !self.complete_class(derived)? {
            return Err(Error::at(
                location,
                "__is_base_of of an incomplete derived class",
            ));
        }
        Ok(self.graph.is_base_of(base, derived))
    }

    fn predicate_class_empty(&mut self, class: EntityId) -> Result<bool, Error> {
        self.check_stack()?;
        // Published only for completed classes: their member/base declarations
        // are immutable. Incomplete substitution observations are never cached.
        if let Some(empty) = self.graph.entity(class).emptiness {
            return Ok(empty);
        }
        let mut empty = !self.graph.entity(class).polymorphism.is_dynamic();
        for index in 0..self.graph.entity(class).members.len() {
            if !empty {
                break;
            }
            let member = self.graph.entity(self.graph.entity(class).members[index]);
            let zero = member.name.is_none()
                && member
                    .bit_field
                    .is_some_and(|id| self.graph.bit_fields.width(id) == 0);
            let overlapping = (member.storage_attributes.overlapping && member.bit_field.is_none())
                .then(|| member.ty.and_then(|ty| self.graph.class_of(ty)))
                .flatten();
            empty = zero
                || if let Some(child) = overlapping {
                    self.predicate_class_empty(child)?
                } else {
                    false
                };
        }
        for index in 0..self.graph.entity(class).bases.len() {
            if !empty {
                break;
            }
            let edge = self.graph.entity(class).bases[index];
            empty = !edge.is_virtual && self.predicate_class_empty(edge.class)?;
        }
        self.graph.entity_mut(class).emptiness = Some(empty);
        Ok(empty)
    }
}

#[cfg(test)]
mod tests {
    use super::super::layout::LayoutState;
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn class_queries_complete_declarations_without_layout_or_member_body_demand() {
        let source = SourceBuffer::new(
            b"struct I;template<class T>struct Box{void unused(){T::missing();}};\
              using Queried=Box<int>;static_assert(__is_empty(Queried),\"complete\");\
              static_assert(__is_empty(Box<int>),\"memo\");\
              static_assert(__is_aggregate(I[]),\"element unasked\");\
              static_assert(__is_base_of(I,I),\"incomplete identity\");\
              using ArrayElement=Box<long>;\
              static_assert(!__has_virtual_destructor(ArrayElement[]),\"element completed\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"class-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let queried = analyzer.identifiers.intern(b"Queried").unwrap();
        let incomplete = analyzer.identifiers.intern(b"I").unwrap();
        let array_element = analyzer.identifiers.intern(b"ArrayElement").unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        // Template instantiation copies its retained parsed class nodes once.
        // Repeated trait evaluation must reuse the resulting canonical class.
        let mut empty_classes = 0;
        for &declaration in tree.decl_lists.get(declarations) {
            let ast::DeclKind::StaticAssert { condition, .. } =
                tree.declarations.get(declaration).kind
            else {
                continue;
            };
            let ast::ExprKind::TypePredicate {
                predicate: T::IsEmpty,
                ..
            } = tree.expressions.get(condition).kind
            else {
                continue;
            };
            let constant = graph.constant_facts[condition.index()].unwrap();
            assert_eq!(constant.bits, 1);
            empty_classes += 1;
        }
        assert_eq!(empty_classes, 2);
        let binding = graph
            .own(
                graph.global,
                queried,
                super::super::lookup::Filter::Ordinary,
            )
            .unwrap();
        let class = graph.class_of(graph.binding(binding).ty.unwrap()).unwrap();
        assert!(graph.entity(class).complete);
        assert_eq!(graph.entity(class).emptiness, Some(true));
        assert!(matches!(
            graph.entity(class).layout,
            LayoutState::NotStarted
        ));
        let binding = graph
            .own(
                graph.global,
                incomplete,
                super::super::lookup::Filter::Elaborated,
            )
            .unwrap();
        let incomplete = graph.binding(binding).entity;
        assert!(!graph.entity(incomplete).complete);
        assert!(graph.entity(incomplete).emptiness.is_none());
        let binding = graph
            .own(
                graph.global,
                array_element,
                super::super::lookup::Filter::Ordinary,
            )
            .unwrap();
        let class = graph.class_of(graph.binding(binding).ty.unwrap()).unwrap();
        assert!(graph.entity(class).complete);
        assert!(graph.entity(class).emptiness.is_none());
        assert!(matches!(
            graph.entity(class).layout,
            LayoutState::NotStarted
        ));
        assert!(graph.storage_layouts.is_empty());
        assert!(graph.root_demands.as_slice().is_empty());
        assert!(super::super::function_demands::trailing(&graph).is_empty());
    }
}
