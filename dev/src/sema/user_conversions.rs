// Translated from analyzer_conversions.cpp::user_conversion/apply_user_conversion.
// See NOTICE.
//! Converting constructors rank without body demand; only the chosen use owns construction.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, LvalueTransform, ReferenceSource,
        ValueConversion,
    },
    graph::{BindingId, ScopeId},
    procedural_calls::{ArgumentSequence, ConversionConstructor},
    specifiers::Explicitness,
    types::{RefKind, TypeId, TypeKind},
};
use crate::syntax::ast;
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConversionSite {
    Call,
    Operator,
}
#[derive(Default)]
pub(super) struct UserScratch {
    candidates: Vec<(BindingId, Conversion)>,
}
impl Analyzer<'_> {
    pub(super) fn converting_constructor_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
    ) -> Result<Option<ArgumentSequence>, Error> {
        if matches!(
            self.graph.types.get(source.ty).kind,
            TypeKind::Scalar(super::types::Scalar::Void)
        ) {
            return Ok(None);
        }
        let object = match self.graph.types.get(target).kind {
            TypeKind::Reference { element, kind } => {
                if kind == RefKind::Lvalue && self.type_cv(element).0 & 1 == 0 {
                    return Ok(None);
                }
                element
            }
            _ => target,
        };
        let Some(class) = self.graph.class_of(object) else {
            return Ok(None);
        };
        if !self.graph.entity(class).complete
            || self
                .graph
                .class_of(source.ty)
                .is_some_and(|from| from == class || self.graph.is_base_of(class, from))
        {
            return Ok(None);
        }
        let mut scratch = std::mem::take(&mut self.user_conversion_scratch);
        let selected = self.select_converting_constructor(source, class, &mut scratch.candidates);
        scratch.candidates.clear();
        self.user_conversion_scratch = scratch;
        let Some((binding, first)) = selected? else {
            return Ok(None);
        };
        Ok(Some(ArgumentSequence {
            conversion: Conversion {
                target,
                result: if target == object {
                    self.bare_type(object)?
                } else {
                    object
                },
                first: LvalueTransform::None,
                second: ValueConversion::None,
                rank: R::UserDefined,
                qualification: false,
                initialization: true,
                temporary: target != object,
                reference_source: ReferenceSource::Other,
            },
            application: Some(first),
            function: None,
            user_constructor: Some(ConversionConstructor::Declared(binding)),
        }))
    }
    fn select_converting_constructor(
        &mut self,
        source: ExprFacts,
        class: super::graph::EntityId,
        candidates: &mut Vec<(BindingId, Conversion)>,
    ) -> Result<Option<(BindingId, Conversion)>, Error> {
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.explicitness == Explicitness::Explicit || entity.template_head.is_some() {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("converting constructor without type"))?;
            let TypeKind::Function {
                parameters,
                variadic,
                ..
            } = &self.graph.types.get(ty).kind
            else {
                return Err(Error::Invariant("converting constructor signature"));
            };
            if parameters.is_empty() {
                if *variadic {
                    let target = self.decayed_type(source.ty)?;
                    let mut first = self
                        .scalar_sequence(source, target, false)?
                        .ok_or(Error::Missing("converting constructor class ellipsis"))?;
                    first.rank = R::Ellipsis;
                    candidates.push((binding, first));
                }
                continue;
            }
            let count = parameters.len();
            let parameter = parameters[0];
            if (1..count)
                .any(|index| !self.function_has_default(self.graph.binding(binding).entity, index))
            {
                continue;
            }
            // 13.3.3.1p4: this first step must be standard, so no recursive
            // constructor queries and no second user-defined conversion.
            if let Some(first) = self.scalar_sequence(source, parameter, false)? {
                candidates.push((binding, first));
            }
        }
        let Some(mut best) = candidates.first().copied() else {
            return Ok(None);
        };
        for &candidate in candidates.iter().skip(1) {
            if self.compare_sequences(candidate.1, best.1)? == Ordering::Greater {
                best = candidate;
            }
        }
        for &candidate in candidates.iter() {
            if candidate.0 != best.0
                && self.compare_sequences(best.1, candidate.1)? != Ordering::Greater
            {
                return Ok(None);
            }
        }
        Ok(Some(best))
    }
    pub(super) fn compare_argument_sequences(
        &mut self,
        a: ArgumentSequence,
        b: ArgumentSequence,
    ) -> Result<Ordering, Error> {
        if a.conversion.rank == R::UserDefined
            && b.conversion.rank == R::UserDefined
            && a.user_constructor != b.user_constructor
        {
            return Ok(Ordering::Equal);
        }
        self.compare_sequences(a.conversion, b.conversion)
    }
    pub(super) fn apply_constructor_sequence(
        &mut self,
        id: ast::ExprId,
        sequence: ArgumentSequence,
        scope: ScopeId,
        site: ConversionSite,
    ) -> Result<bool, Error> {
        let Some(constructor) = sequence.user_constructor else {
            return Ok(false);
        };
        let ConversionConstructor::Declared(binding) = constructor else {
            return Err(Error::Invariant(
                "implicit constructor outside list argument",
            ));
        };
        let location = self.tree.expressions.get(id).location;
        // Calls check the parameter/binding destruction before applying the
        // user conversion; overloaded operators do so after the callable checks.
        if site == ConversionSite::Call {
            self.class_parameter_destruction(sequence.conversion.result, scope, location)?;
        }
        let constructor = self.selected_constructor_plan(binding, location)?;
        self.validate_constructor_use(constructor, scope, location)?;
        if matches!(
            self.graph.types.get(sequence.conversion.target).kind,
            TypeKind::Class(_)
        ) {
            self.check_converting_final_copy(
                self.graph.constructors.get(constructor).class,
                scope,
                location,
            )?;
        }
        if site == ConversionSite::Operator {
            self.class_parameter_destruction(sequence.conversion.result, scope, location)?;
        }
        if let Some(function) = sequence.function {
            self.select_function_designation(id, function)?;
        }
        let first = sequence
            .application
            .ok_or(Error::Invariant("constructor without first conversion"))?;
        self.check_bit_field_binding(id, first)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        // The frozen constructor-conversion owner retains this standard route
        // without the access check performed for an outer base conversion.
        facts.base_path = self.conversion_base_path(facts, first, scope, location, false)?;
        facts.conversion = Some(first);
        self.odr_use_constants(id, Some(first))?;
        self.graph.expression_facts.set(id, facts);
        self.class_parameter_destruction(first.target, scope, location)?;
        self.record_class_copy(
            id,
            first.target,
            scope,
            super::copy_initialization::CopyUse::Value { in_place: false },
            location,
        )?;
        self.graph
            .expression_facts
            .set_user_conversion(id, constructor, sequence.conversion);
        self.demand_constructor_use(constructor);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn constructor_ranking_keeps_source_aliases_and_demands_only_selected_use() {
        let source = SourceBuffer::new(b"struct X{int n;X(int);X(long);};void use(const X&);void use(int);int n;const X &r=((n));void g(){use(3);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"user-construction-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), count);
        let ast::DeclKind::Simple { declarators, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[4])
            .kind
        else {
            panic!("reference declaration");
        };
        let ast::InitializerKind::Assign(id) = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap()
            .kind
        else {
            panic!("reference initializer");
        };
        let root = graph.expression_facts.root(id);
        assert_ne!(id, root);
        let facts = graph.expression_facts.get(id).unwrap();
        assert_eq!(
            facts.category,
            super::super::expression_facts::ValueCategory::Lvalue
        );
        assert_eq!(graph.expression_type(root), Some(facts.ty));
        let first = facts.conversion.unwrap();
        assert_eq!(first.rank, R::Exact);
        assert_eq!(first.target, facts.ty);
        let outer = graph.expression_facts.user_conversion(id).unwrap();
        assert_eq!(outer.rank, R::UserDefined);
        assert!(outer.temporary);
        let (constructor, arguments) = graph.expression_facts.class_conversion(id).unwrap();
        assert!(
            matches!(arguments, super::super::constructor_initialization::ConstructorArguments::Single(original) if original == id)
        );
        assert_eq!(
            graph.expression_facts.class_conversion(root).unwrap().0,
            constructor
        );
        let name = cursor.intern_name(b"X").unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        let offered = &graph.entity(class).declared_constructors;
        assert_eq!(offered.len(), 2);
        assert_eq!(
            graph.constructors.get(constructor).binding,
            Some(offered[0])
        );
        assert!(
            graph
                .entity(graph.binding(offered[1]).entity)
                .constructor
                .is_none()
        );
        let demands = graph.root_demands.as_slice();
        assert_eq!(demands.iter().filter(|d| matches!(d, super::super::function_demands::Demand::Function {binding, ..} if offered.contains(binding))).count(), 1);
    }
}
