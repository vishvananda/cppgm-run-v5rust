// Translated from analyzer_conversions.cpp::user_conversion/apply_user_conversion.
// See NOTICE.
//! User sequences rank without body demand; only a selected use owns its call or construction.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, LvalueTransform, ReferenceSource,
        ValueConversion,
    },
    graph::{BindingId, ScopeId},
    procedural_calls::{ArgumentSequence, UserConversion},
    specifiers::Explicitness,
    types::{RefKind, TypeId, TypeKind},
};
use crate::syntax::ast;
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConversionSite {
    Call,
    Operator,
    Conditional,
}
#[derive(Clone, Copy)]
pub(super) struct UserCandidate {
    pub binding: BindingId,
    pub first: Conversion,
    pub after: Option<Conversion>,
}
#[derive(Default)]
pub(super) struct UserScratch {
    candidates: Vec<UserCandidate>,
}
impl Analyzer<'_> {
    pub(super) fn user_conversion_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<ArgumentSequence>, Error> {
        if let Some(sequence) = self.conversion_function_sequence(source, target, direct)? {
            return Ok(Some(sequence));
        }
        self.user_conversion_query(source, target, direct, false)
    }
    pub(super) fn conditional_user_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
    ) -> Result<Option<ArgumentSequence>, Error> {
        self.user_conversion_query(source, target, false, true)
    }
    fn user_conversion_query(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
        functions: bool,
    ) -> Result<Option<ArgumentSequence>, Error> {
        if matches!(
            self.graph.types.get(source.ty).kind,
            TypeKind::Scalar(super::types::Scalar::Void)
        ) || source.functions.is_some()
        {
            return Ok(None);
        }
        let (object, reference) = match self.graph.types.get(target).kind {
            TypeKind::Reference { element, kind } => (element, Some(kind)),
            _ => (target, None),
        };
        let class = self.graph.class_of(object);
        if self
            .graph
            .class_of(source.ty)
            .zip(class)
            .is_some_and(|(from, to)| from == to || self.graph.is_base_of(to, from))
        {
            return Ok(None);
        }
        let mut scratch = std::mem::take(&mut self.user_conversion_scratch);
        let selected = (|| {
            // 8.5.3p5: a directly binding lvalue result has priority over
            // constructing a temporary. A failed or ambiguous probe falls through.
            if functions && reference == Some(RefKind::Lvalue) {
                self.conversion_function_candidates(
                    source,
                    target,
                    direct,
                    true,
                    &mut scratch.candidates,
                )?;
                if let Some(best) = self.best_user_candidate(&scratch.candidates)? {
                    return Ok(Some(best));
                }
                scratch.candidates.clear();
            }
            if let Some(class) = class
                && self.graph.entity(class).complete
                && !(reference == Some(RefKind::Lvalue) && self.type_cv(object).0 & 1 == 0)
            {
                self.converting_constructor_candidates(
                    source,
                    class,
                    direct,
                    &mut scratch.candidates,
                )?;
            }
            if functions {
                self.conversion_function_candidates(
                    source,
                    target,
                    direct,
                    false,
                    &mut scratch.candidates,
                )?;
            }
            self.best_user_candidate(&scratch.candidates)
        })();
        scratch.candidates.clear();
        self.user_conversion_scratch = scratch;
        let Some(best) = selected? else {
            return Ok(None);
        };
        let conversion = Conversion {
            target,
            result: if reference.is_some() {
                object
            } else {
                self.bare_type(object)?
            },
            first: LvalueTransform::None,
            second: ValueConversion::None,
            rank: R::UserDefined,
            qualification: false,
            initialization: true,
            temporary: reference.is_some() && best.after.is_none_or(|after| after.temporary),
            reference_source: ReferenceSource::Other,
        };
        Ok(Some(ArgumentSequence {
            conversion,
            application: Some(best.after.unwrap_or(best.first)),
            function: None,
            user_conversion: Some(if best.after.is_some() {
                UserConversion::Function(best.binding)
            } else {
                UserConversion::Constructor(best.binding)
            }),
        }))
    }

    fn converting_constructor_candidates(
        &mut self,
        source: ExprFacts,
        class: super::graph::EntityId,
        direct: bool,
        candidates: &mut Vec<UserCandidate>,
    ) -> Result<(), Error> {
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.explicitness == Explicitness::Explicit && !direct
                || entity.template_head.is_some()
            {
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
                    candidates.push(UserCandidate {
                        binding,
                        first,
                        after: None,
                    });
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
                candidates.push(UserCandidate {
                    binding,
                    first,
                    after: None,
                });
            }
        }
        Ok(())
    }
    pub(super) fn best_user_candidate(
        &mut self,
        candidates: &[UserCandidate],
    ) -> Result<Option<UserCandidate>, Error> {
        let Some(mut best) = candidates.first().copied() else {
            return Ok(None);
        };
        for &candidate in candidates.iter().skip(1) {
            if self.user_candidate_order(candidate, best)? == Ordering::Greater {
                best = candidate;
            }
        }
        for &candidate in candidates {
            if candidate.binding != best.binding
                && self.user_candidate_order(best, candidate)? != Ordering::Greater
            {
                return Ok(None);
            }
        }
        Ok(Some(best))
    }
    fn user_candidate_order(
        &mut self,
        a: UserCandidate,
        b: UserCandidate,
    ) -> Result<Ordering, Error> {
        let order = self.compare_sequences(a.first, b.first)?;
        if order == Ordering::Equal
            && let Some((a, b)) = a.after.zip(b.after)
        {
            return self.compare_sequences(a, b);
        }
        Ok(order)
    }
    pub(super) fn compare_argument_sequences(
        &mut self,
        a: ArgumentSequence,
        b: ArgumentSequence,
    ) -> Result<Ordering, Error> {
        if a.conversion.rank == R::UserDefined
            && b.conversion.rank == R::UserDefined
            && a.user_conversion != b.user_conversion
        {
            return Ok(Ordering::Equal);
        }
        if matches!(a.user_conversion, Some(UserConversion::Function(_)))
            && a.user_conversion == b.user_conversion
        {
            let after_a = a.application.ok_or(Error::Invariant(
                "conversion function without second sequence",
            ))?;
            let after_b = b.application.ok_or(Error::Invariant(
                "conversion function without second sequence",
            ))?;
            return self.compare_sequences(after_a, after_b);
        }
        self.compare_sequences(a.conversion, b.conversion)
    }
    pub(super) fn apply_user_sequence(
        &mut self,
        id: ast::ExprId,
        sequence: ArgumentSequence,
        scope: ScopeId,
        site: ConversionSite,
    ) -> Result<bool, Error> {
        let Some(constructor) = sequence.user_conversion else {
            return Ok(false);
        };
        if matches!(constructor, UserConversion::Function(_)) {
            self.apply_function_sequence(id, sequence, scope)?;
            return Ok(true);
        }
        let UserConversion::Constructor(binding) = constructor else {
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
        self.check_inherited_constructor_access(binding, location)?;
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
