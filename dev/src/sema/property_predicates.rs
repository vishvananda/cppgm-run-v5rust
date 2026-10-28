// Translated from sema/analyzer_traits.cpp's declaration-property queries. See NOTICE.
//! Complete-class answers are owned by the unit and keyed by canonical class and question.
//! Incomplete classes never publish answers; querying constructors leaves emission undemanded.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, FunctionAvailability, ScopeId},
    special_members::SpecialDefinition,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{lex::predicates::TypePredicate as T, support::source::SourceOffset};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Question {
    DefaultShape,
    DefaultLegacy,
    Literal,
    StandardLayout,
    HasData,
}
#[derive(Clone, Copy)]
enum Answer {
    Active,
    Ready(bool),
}
#[derive(Default)]
pub(super) struct Properties {
    classes: HashMap<(EntityId, Question), Answer>,
}
impl Analyzer<'_> {
    pub(super) fn property_predicate(
        &mut self,
        predicate: T,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        self.property_predicate_value(predicate, ty, scope, location)
    }

    fn property_predicate_value(
        &mut self,
        predicate: T,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let object = self.property_element(ty);
        let unknown = matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Array {
                bound: 0,
                operand: None,
                ..
            }
        );
        let destruction = matches!(
            predicate,
            T::IsDestructible | T::IsNothrowDestructible | T::IsTriviallyDestructible
        );
        if !(unknown && destruction)
            && let Some(class) = self.graph.class_of(object)
            && !self.complete_class(class)?
        {
            return Err(Error::at(
                location,
                "a type trait of an incomplete class type",
            ));
        }
        match predicate {
            T::IsTriviallyCopyable => self.property_copyable(ty),
            T::IsTrivial => Ok(self.property_copyable(ty)? && self.property_default_trivial(ty)?),
            T::IsStandardLayout => self.property_standard_layout(ty),
            T::IsPod => Ok(self.property_copyable(ty)?
                && self.property_default_trivial(ty)?
                && self.property_standard_layout(ty)?),
            T::IsLiteralType => self.property_literal(ty),
            T::HasTrivialDestructor => {
                if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                    || self.property_copyable(ty)? && self.property_default_trivial(ty)?
                {
                    return Ok(true);
                }
                self.graph
                    .class_of(object)
                    .map_or(Ok(false), |class| self.graph.destructor_trivial(class))
            }
            T::IsDestructible | T::IsNothrowDestructible | T::IsTriviallyDestructible => {
                self.property_destructible(predicate, ty, scope, location)
            }
            T::HasTrivialConstructor
            | T::HasTrivialCopy
            | T::HasTrivialAssign
            | T::HasNothrowConstructor
            | T::HasNothrowCopy
            | T::HasNothrowAssign => self.property_legacy(predicate, ty),
            _ => Err(Error::Invariant(
                "nonproperty predicate in property dispatch",
            )),
        }
    }

    fn property_element(&self, mut ty: TypeId) -> TypeId {
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        ty
    }

    fn property_scalar(&self, ty: TypeId) -> bool {
        matches!(self.graph.types.get(ty).kind,
            TypeKind::Scalar(scalar) if scalar != Scalar::Void)
            || matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Enum(_)
                    | TypeKind::Pointer(_)
                    | TypeKind::MemberPointer { .. }
                    | TypeKind::Complex(_)
                    | TypeKind::Vector { .. }
            )
    }

    pub(super) fn property_copyable(&mut self, ty: TypeId) -> Result<bool, Error> {
        let ty = self.property_element(ty);
        if self.property_scalar(ty) {
            return Ok(true);
        }
        self.graph
            .class_of(ty)
            .map_or(Ok(false), |class| self.class_trivially_copyable(class))
    }

    pub(super) fn property_default_trivial(&mut self, ty: TypeId) -> Result<bool, Error> {
        let ty = self.property_element(ty);
        if self.property_scalar(ty) {
            return Ok(true);
        }
        let Some(class) = self.graph.class_of(ty) else {
            return Ok(false);
        };
        Ok(self
            .queried_default_properties(class)?
            .is_some_and(|facts| !facts.deleted && facts.trivial))
    }

    fn property_legacy(&mut self, predicate: T, ty: TypeId) -> Result<bool, Error> {
        let assignment = matches!(predicate, T::HasTrivialAssign | T::HasNothrowAssign);
        let copy = matches!(predicate, T::HasTrivialCopy | T::HasNothrowCopy);
        let nothrow = matches!(
            predicate,
            T::HasNothrowConstructor | T::HasNothrowCopy | T::HasNothrowAssign
        );
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        let object = self.property_element(ty);
        if assignment && (reference || self.type_cv(object).0 & 1 != 0) {
            return Ok(false);
        }
        if copy && reference {
            return Ok(true);
        }
        if self.property_copyable(ty)?
            && (assignment || copy || self.property_default_trivial(ty)?)
        {
            return Ok(true);
        }
        let Some(class) = self
            .graph
            .class_of(if assignment || copy { ty } else { object })
        else {
            return Ok(false);
        };
        if assignment || copy {
            return self.legacy_copy_trait(class, assignment, nothrow);
        }
        let Some(facts) = self.queried_default_properties(class)? else {
            return Ok(false);
        };
        if self.property_class_question(class, Question::DefaultLegacy)? {
            return Ok(true);
        }
        if !nothrow {
            return Ok(false);
        }
        Ok(facts.nothrow)
    }

    fn property_destructible(
        &mut self,
        predicate: T,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            return Ok(true);
        }
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(Scalar::Void)
                | TypeKind::Function { .. }
                | TypeKind::Array {
                    bound: 0,
                    operand: None,
                    ..
                }
        ) {
            return Ok(false);
        }
        let object = self.property_element(ty);
        let Some(class) = self.graph.class_of(object) else {
            return Ok(true);
        };
        if !self.complete_class(class)? || self.graph.entity(class).scope.is_none() {
            return Ok(false);
        }
        if self
            .substitution_trial(|analyzer| {
                analyzer.check_class_destruction(object, scope, location)
            })?
            .is_none()
        {
            return Ok(false);
        }
        let nothrow = self.available_destructor_nothrow(class)?;
        let trivial = !self.graph.has_nontrivial_destructor(class);
        Ok(match predicate {
            T::IsNothrowDestructible => nothrow,
            T::IsTriviallyDestructible => trivial,
            _ => true,
        })
    }

    fn property_literal(&mut self, ty: TypeId) -> Result<bool, Error> {
        if matches!(self.graph.types.get(ty).kind, TypeKind::Function { .. }) {
            return Ok(false);
        }
        if self.constant_literal_type(ty)? {
            return Ok(true);
        }
        let ty = self.property_element(ty);
        self.graph.class_of(ty).map_or(Ok(false), |class| {
            self.property_class_question(class, Question::Literal)
        })
    }

    fn property_standard_layout(&mut self, ty: TypeId) -> Result<bool, Error> {
        let ty = self.property_element(ty);
        if self.property_scalar(ty) {
            return Ok(true);
        }
        self.graph.class_of(ty).map_or(Ok(false), |class| {
            self.property_class_question(class, Question::StandardLayout)
        })
    }

    fn property_class_question(
        &mut self,
        class: EntityId,
        question: Question,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let key = (class, question);
        if let Some(answer) = self.graph.property_predicates.classes.get(&key) {
            return match *answer {
                Answer::Ready(value) => Ok(value),
                Answer::Active => Err(Error::Invariant("cyclic class property query")),
            };
        }
        // The caller has completed this class. Completion freezes the base and
        // member declarations; these answers never depend on later body demands.
        self.graph
            .property_predicates
            .classes
            .insert(key, Answer::Active);
        let result = match question {
            Question::DefaultShape => self.property_default_shape(class),
            Question::DefaultLegacy => self.property_default_legacy(class),
            Question::Literal => self.property_literal_class(class),
            Question::StandardLayout => self.property_standard_class(class),
            Question::HasData => self.property_has_data(class),
        };
        if let Ok(value) = result {
            self.graph
                .property_predicates
                .classes
                .insert(key, Answer::Ready(value));
        } else {
            self.graph.property_predicates.classes.remove(&key);
        }
        result
    }

    fn property_default_shape(&mut self, class: EntityId) -> Result<bool, Error> {
        let entity = self.graph.entity(class);
        if entity.polymorphism.is_dynamic() || entity.bases.iter().any(|base| base.is_virtual) {
            return Ok(false);
        }
        let mut declared = false;
        let mut found = false;
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let function = self.graph.binding(binding).entity;
            let entity = self.graph.entity(function);
            if self.graph.inherited_constructors.source(function).is_some() {
                continue;
            }
            declared = true;
            let count = self
                .graph
                .types
                .function_parameter_count(self.graph.lower_entity_type(function)?)?;
            if entity.template_head.is_some()
                || (0..count).any(|i| entity.defaults.get(i).is_none_or(Option::is_none))
            {
                continue;
            }
            if entity.availability != FunctionAvailability::Deleted
                && !matches!(
                    entity.special_definition,
                    Some(SpecialDefinition::Defaulted { first: true, .. })
                )
            {
                return Ok(false);
            }
            found = true;
        }
        if declared && !found {
            return Ok(false);
        }
        let count = self.graph.entity(class).bases.len() + self.graph.entity(class).members.len();
        for index in 0..count {
            if let Some((child, member, _)) = self.graph.copy_child(class, index)? {
                if member
                    .is_some_and(|member| self.graph.entity(member).member_initializer.is_some())
                    || !self.property_class_question(child, Question::DefaultShape)?
                {
                    return Ok(false);
                }
            } else if index >= self.graph.entity(class).bases.len()
                && self
                    .graph
                    .entity(
                        self.graph.entity(class).members
                            [index - self.graph.entity(class).bases.len()],
                    )
                    .member_initializer
                    .is_some()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn property_default_legacy(&mut self, class: EntityId) -> Result<bool, Error> {
        let Some(facts) = self.queried_default_properties(class)? else {
            return Ok(false);
        };
        if facts.trivial {
            return Ok(true);
        }
        if !facts.deleted
            || self.graph.entity(class).polymorphism.is_dynamic()
            || self
                .graph
                .entity(class)
                .bases
                .iter()
                .any(|base| base.is_virtual)
        {
            return Ok(false);
        }
        if let Some(binding) = self.declared_default_constructor(class)? {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.availability != FunctionAvailability::Deleted
                && !matches!(
                    entity.special_definition,
                    Some(SpecialDefinition::Defaulted { first: true, .. })
                )
            {
                return Ok(false);
            }
        }
        for index in 0..self.graph.entity(class).bases.len() {
            let child = self.graph.entity(class).bases[index].class;
            if !self.property_legacy(
                T::HasTrivialConstructor,
                self.graph.lower_entity_type(child)?,
            )? {
                return Ok(false);
            }
        }
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if self.graph.entity(member).member_initializer.is_some() {
                return Ok(false);
            }
            let ty = self.property_element(self.graph.lower_entity_type(member)?);
            if self.graph.class_of(ty).is_some()
                && !self.property_legacy(T::HasTrivialConstructor, ty)?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn property_literal_class(&mut self, class: EntityId) -> Result<bool, Error> {
        if !self.graph.destructor_trivial(class)?
            || !self.property_class_question(class, Question::DefaultShape)?
        {
            return Ok(false);
        }
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index];
            if base.is_virtual
                || !self.property_literal(self.graph.lower_entity_type(base.class)?)?
            {
                return Ok(false);
            }
        }
        for index in 0..self.graph.entity(class).members.len() {
            let ty = self
                .graph
                .lower_entity_type(self.graph.entity(class).members[index])?;
            if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                continue;
            }
            if !self.property_literal(ty)? || self.type_cv(self.property_element(ty)).0 & 2 != 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn property_standard_class(&mut self, class: EntityId) -> Result<bool, Error> {
        let entity = self.graph.entity(class);
        if entity.polymorphism.is_dynamic() || entity.bases.iter().any(|base| base.is_virtual) {
            return Ok(false);
        }
        let first = entity.members.first().copied();
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if first.is_some_and(|first| {
                self.graph.entity(first).member_access != self.graph.entity(member).member_access
            }) || !self.property_standard_layout(self.graph.lower_entity_type(member)?)?
            {
                return Ok(false);
            }
        }
        let mut base_has_data = false;
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            if !self.property_class_question(base, Question::StandardLayout)? {
                return Ok(false);
            }
            if self.property_class_question(base, Question::HasData)? {
                if base_has_data || first.is_some() {
                    return Ok(false);
                }
                base_has_data = true;
            }
            if let Some(first) = first
                && self
                    .graph
                    .class_of(self.property_element(self.graph.lower_entity_type(first)?))
                    == Some(base)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn property_has_data(&mut self, class: EntityId) -> Result<bool, Error> {
        if !self.graph.entity(class).members.is_empty() {
            return Ok(true);
        }
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            if self.property_class_question(base, Question::HasData)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn class_properties_reuse_answers_without_syntax_layout_or_emission_demand() {
        let source = SourceBuffer::new(
            b"struct Leaf {const int n;}; struct Pair {Leaf a,b; template<class T>void unused(){T::missing();}}; struct Hidden {private: ~Hidden();}; struct Throwing {~Throwing() noexcept(false);};"
                .to_vec(),
        ).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"property-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"Leaf".as_slice(), b"Pair", b"Hidden", b"Throwing"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let types: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer
                    .graph
                    .own(scope, name, super::super::lookup::Filter::Elaborated)
                    .unwrap();
                analyzer.graph.binding(binding).ty.unwrap()
            })
            .collect();
        let syntax = analyzer.tree.expressions.mark();
        let demands = analyzer.graph.root_demands.as_slice().len();
        let queries = [
            (T::IsLiteralType, types[1], true),
            (T::IsTrivial, types[0], false),
            (T::HasTrivialConstructor, types[0], true),
            (T::HasTrivialConstructor, types[1], true),
            (T::HasNothrowConstructor, types[1], true),
            (T::IsStandardLayout, types[1], true),
            (T::HasNothrowCopy, types[1], true),
            (T::IsDestructible, types[2], false),
            (T::IsNothrowDestructible, types[3], false),
        ];
        let mut memo = None;
        for _ in 0..3 {
            for (predicate, ty, expected) in queries {
                assert_eq!(
                    analyzer
                        .property_predicate(predicate, ty, scope, SourceOffset(0))
                        .unwrap(),
                    expected
                );
                assert_eq!(analyzer.unevaluated, 0);
            }
            let count = analyzer.graph.property_predicates.classes.len();
            assert_eq!(*memo.get_or_insert(count), count);
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax);
        assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
        assert!(analyzer.graph.storage_layouts.is_empty());
        for ty in types {
            let class = analyzer.graph.class_of(ty).unwrap();
            assert!(matches!(
                analyzer.graph.entity(class).layout,
                super::super::layout::LayoutState::NotStarted
            ));
        }
    }
}
