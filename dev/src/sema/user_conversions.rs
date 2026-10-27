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
    AggregateElement,
    Cast(crate::support::source::SourceOffset),
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
    pub(super) fn class_initializer_conversion(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        context: super::constructor_initialization::ClassContext,
        location: crate::support::source::SourceOffset,
    ) -> Result<ArgumentSequence, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        self.user_conversion_sequence(facts, target, false)?
            .ok_or_else(|| {
                Error::at(
                    location,
                    if context == super::constructor_initialization::ClassContext::AggregateMember {
                        "no conversion to the member's class"
                    } else {
                        "no unambiguous user-defined conversion initializes the class object"
                    },
                )
            })
    }
    pub(super) fn user_conversion_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<ArgumentSequence>, Error> {
        self.user_conversion_query(source, target, direct)
    }
    pub(super) fn conditional_user_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
    ) -> Result<Option<ArgumentSequence>, Error> {
        self.user_conversion_query(source, target, false)
    }
    fn user_conversion_query(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
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
            if reference == Some(RefKind::Lvalue) {
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
                && self.complete_class(class)?
                && !(reference == Some(RefKind::Lvalue) && self.type_cv(object).0 & 1 == 0)
            {
                self.converting_constructor_candidates(
                    source,
                    class,
                    direct,
                    &mut scratch.candidates,
                )?;
            }
            self.conversion_function_candidates(
                source,
                target,
                direct,
                false,
                &mut scratch.candidates,
            )?;
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
            list_initialization: crate::sema::expression_facts::ListInitialization::Ordinary,
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
            if entity.explicitness == Explicitness::Explicit && !direct {
                continue;
            }
            let Some(binding) = self.deduce_function_call(binding, &[], false, Some(source))?
            else {
                continue;
            };
            let binding = self.inherited_constructor_specialization(class, binding, 1)?;
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.explicitness == Explicitness::Explicit && !direct {
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
        let mut order = self.compare_sequences(a.first, b.first)?;
        if order == Ordering::Equal
            && let Some((a, b)) = a.after.zip(b.after)
        {
            order = self.compare_sequences(a, b)?;
        }
        if order != Ordering::Equal {
            return Ok(order);
        }
        let ap = self
            .graph
            .function_templates
            .primary(self.graph.binding(a.binding).entity);
        let bp = self
            .graph
            .function_templates
            .primary(self.graph.binding(b.binding).entity);
        if ap.is_some() != bp.is_some() {
            return Ok(bp.is_some().cmp(&ap.is_some()));
        }
        if let Some((ap, bp)) = ap.zip(bp)
            && self.graph.binding(ap).entity != self.graph.binding(bp).entity
            && a.after.is_some() == b.after.is_some()
        {
            return if a.after.is_some() {
                self.conversion_template_order(ap, bp)
            } else {
                let form = super::function_templates::TemplateOrdering::Call(1);
                let forward = self.template_at_least_as_specialized(ap, bp, form)?;
                let backward = self.template_at_least_as_specialized(bp, ap, form)?;
                Ok(forward.cmp(&backward))
            };
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
            && a.conversion.list_initialization == b.conversion.list_initialization
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
            if let ConversionSite::Cast(location) = site {
                self.apply_function_sequence_at(id, sequence, scope, location)?;
            } else {
                self.apply_function_sequence(id, sequence, scope)?;
            }
            return Ok(true);
        }
        let UserConversion::Constructor(binding) = constructor else {
            return Err(Error::Invariant(
                "implicit constructor outside list argument",
            ));
        };
        let location = if let ConversionSite::Cast(location) = site {
            location
        } else {
            self.tree.expressions.get(id).location
        };
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
        self.record_conversion(&mut facts, first);
        self.odr_use_constants(id, Some(first))?;
        self.graph.expression_facts.set(id, facts);
        if first.rank == R::Ellipsis {
            // The frozen implicit constructor conversion retains its first
            // sequence without ordinary variadic promotions. Only class
            // storage demand is shared with a written ellipsis argument.
            self.demand_variadic_class_storage(first.target);
        } else {
            self.class_parameter_destruction(first.target, scope, location)?;
            self.record_class_copy(
                id,
                first.target,
                scope,
                super::copy_initialization::CopyUse::Value { in_place: false },
                location,
            )?;
        }
        self.graph
            .expression_facts
            .set_user_conversion(id, constructor, sequence.conversion);
        self.demand_constructor_use(constructor)?;
        self.request_constructor_defaults(binding, 1)?;
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
    fn ordinary_queries_reuse_scratch_without_demand_and_prioritize_direct_references() {
        use super::super::{expression_facts::ValueCategory, types::Cv};
        let source = SourceBuffer::new(
            b"struct Target;struct Source{operator const Target&()const;operator int();};\
              struct Target{Target(Source&);};struct Unused{Unused(int);};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"ordinary-query-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let source_name = cursor.intern_name(b"Source").unwrap();
        let target_name = cursor.intern_name(b"Target").unwrap();
        let unused_name = cursor.intern_name(b"Unused").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let class = |name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity
        };
        let source_class = class(source_name);
        let target_class = class(target_name);
        let unused = class(unused_name);
        let source_type = analyzer.graph.entity(source_class).ty.unwrap();
        let target = analyzer.graph.entity(target_class).ty.unwrap();
        let constant = analyzer.graph.types.qualify(target, Cv::CONST).unwrap();
        let reference = analyzer
            .graph
            .types
            .reference(constant, RefKind::Lvalue)
            .unwrap();
        let facts = ExprFacts::new(source_type, ValueCategory::Lvalue);
        let constructor = analyzer.graph.entity(target_class).declared_constructors[0];
        let function = analyzer.graph.entity(source_class).declared_conversions[0];
        let demands = analyzer.graph.root_demands.as_slice().len();
        for _ in 0..3 {
            let value = analyzer
                .user_conversion_sequence(facts, target, false)
                .unwrap()
                .unwrap();
            assert!(
                matches!(value.user_conversion, Some(UserConversion::Constructor(binding)) if binding == constructor)
            );
            let bound = analyzer
                .user_conversion_sequence(facts, reference, false)
                .unwrap()
                .unwrap();
            assert!(
                matches!(bound.user_conversion, Some(UserConversion::Function(binding)) if binding == function)
            );
            assert!(!bound.conversion.temporary);
            assert!(analyzer.user_conversion_scratch.candidates.is_empty());
        }
        assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
        assert!(
            analyzer
                .graph
                .entity(analyzer.graph.binding(constructor).entity)
                .constructor
                .is_none()
        );
        for class in [source_class, target_class, unused] {
            assert!(analyzer.graph.entity(class).implicit_copies.is_none());
            assert!(matches!(
                analyzer.graph.entity(class).layout,
                super::super::layout::LayoutState::NotStarted
            ));
        }
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
    }

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
