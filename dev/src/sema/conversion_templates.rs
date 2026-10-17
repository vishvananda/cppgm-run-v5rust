// Translated from function_templates.cpp::deduce_for_conversion,
// result_at_least_as_specialized and conversion_template_order. See NOTICE.
//! Result-directed deduction forms signatures only; use selection owns body demand.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::BindingId,
    types::{Cv, RefKind, TypeId, TypeKind},
};
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub(super) enum ConversionDeduction {
    Named,
    Implicit,
}

impl Analyzer<'_> {
    pub(super) fn deduce_conversion_template(
        &mut self,
        binding: BindingId,
        required: TypeId,
        form: ConversionDeduction,
    ) -> Result<Option<BindingId>, Error> {
        let Some(head) = self.function_template_head(binding)? else {
            return Ok(Some(binding));
        };
        let signature = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("conversion template without signature"))?;
        let mut pattern = self.conversion_template_result(binding)?;
        let mut actual = required;
        let mut binds = false;
        if matches!(form, ConversionDeduction::Implicit) {
            binds = matches!(
                self.graph.types.get(pattern).kind,
                TypeKind::Reference { .. }
            ) && matches!(
                self.graph.types.get(actual).kind,
                TypeKind::Reference { .. }
            );
            if let TypeKind::Reference { element, .. } = self.graph.types.get(pattern).kind {
                pattern = element;
            }
            if let TypeKind::Reference { element, .. } = self.graph.types.get(actual).kind {
                actual = element;
            }
            if !binds {
                pattern = self.decayed_type(pattern)?;
                actual = self.bare_type(actual)?;
            }
        }
        let depth = self.graph.templates.depth(head);
        let mut arguments = vec![None; self.graph.templates.parameters(head).len()];
        let mut deduced =
            self.deduce_signature_type(pattern, actual, depth, &mut arguments, false)?;
        // The frozen alternatives are ordered, independent deduction attempts.
        // They change A only; the second standard conversion checks the result.
        if !deduced && binds && self.type_cv(actual).0 & !self.type_cv(pattern).0 != 0 {
            arguments.fill(None);
            let bare = self.bare_type(actual)?;
            let qualified = self
                .graph
                .types
                .qualify(bare, Cv(self.type_cv(actual).0 & self.type_cv(pattern).0))?;
            deduced =
                self.deduce_signature_type(pattern, qualified, depth, &mut arguments, false)?;
        }
        if !deduced
            && matches!(form, ConversionDeduction::Implicit)
            && matches!(
                self.graph.types.get(actual).kind,
                TypeKind::Pointer(_) | TypeKind::MemberPointer { .. }
            )
        {
            arguments.fill(None);
            let unqualified = self.conversion_unqualified_levels(actual)?;
            deduced =
                self.deduce_signature_type(pattern, unqualified, depth, &mut arguments, false)?;
        }
        if !deduced || !self.deduced_values_typed(head, signature, &arguments)? {
            return Ok(None);
        }
        self.complete_signature(binding, &mut arguments)
    }

    fn conversion_unqualified_levels(&mut self, mut ty: TypeId) -> Result<TypeId, Error> {
        let mut levels = Vec::new();
        loop {
            ty = self.bare_type(ty)?;
            match self.graph.types.get(ty).kind {
                TypeKind::Pointer(element) => {
                    levels.push(None);
                    ty = element;
                }
                TypeKind::MemberPointer { owner, element } => {
                    levels.push(Some(owner));
                    ty = element;
                }
                _ => break,
            }
        }
        for owner in levels.into_iter().rev() {
            ty = if let Some(owner) = owner {
                self.graph.types.member_pointer(owner, ty)?
            } else {
                self.graph
                    .types
                    .intern(TypeKind::Pointer(ty), Cv::default())?
            };
        }
        Ok(ty)
    }

    fn conversion_template_result(&self, binding: BindingId) -> Result<TypeId, Error> {
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("conversion template without signature"))?;
        let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant(
                "conversion template without function type",
            ));
        };
        Ok(result)
    }

    fn conversion_result_at_least_as_specialized(
        &mut self,
        first: BindingId,
        second: BindingId,
    ) -> Result<bool, Error> {
        let Some(head) = self.function_template_head(first)? else {
            return Ok(false);
        };
        let Some(other) = self.function_template_head(second)? else {
            return Ok(false);
        };
        if self.graph.templates.parameters(other).is_empty() {
            return Ok(false);
        }
        let unique = self.unique_partial_arguments(head)?;
        let mut substitution = Substitution::new(self.graph.templates.depth(head), &unique);
        let from = self.conversion_template_result(first)?;
        let Some(mut actual) = self.substitute_signature_type(from, &mut substitution)? else {
            return Ok(false);
        };
        let mut pattern = self.conversion_template_result(second)?;
        if let TypeKind::Reference { element, .. } = self.graph.types.get(actual).kind {
            actual = element;
        }
        if let TypeKind::Reference { element, .. } = self.graph.types.get(pattern).kind {
            pattern = element;
        }
        let actual = self.bare_type(actual)?;
        let pattern = self.bare_type(pattern)?;
        let mut arguments = vec![None; self.graph.templates.parameters(other).len()];
        self.deduce_signature_type(
            pattern,
            actual,
            self.graph.templates.depth(other),
            &mut arguments,
            false,
        )
    }

    // Both inputs are primary bindings. Unlike ordinary call ordering, only
    // the result participates, with reference/cv ties after mutual deduction.
    pub(super) fn conversion_template_order(
        &mut self,
        first: BindingId,
        second: BindingId,
    ) -> Result<Ordering, Error> {
        let forward = self.conversion_result_at_least_as_specialized(first, second)?;
        let backward = self.conversion_result_at_least_as_specialized(second, first)?;
        if forward != backward || !forward {
            return Ok(forward.cmp(&backward));
        }
        let a = self.conversion_template_result(first)?;
        let b = self.conversion_template_result(second)?;
        let (
            TypeKind::Reference {
                element: a,
                kind: ak,
            },
            TypeKind::Reference {
                element: b,
                kind: bk,
            },
        ) = (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
        else {
            return Ok(Ordering::Equal);
        };
        if ak != bk {
            return Ok(if *ak == RefKind::Lvalue {
                Ordering::Greater
            } else {
                Ordering::Less
            });
        }
        let acv = self.type_cv(*a).0;
        let bcv = self.type_cv(*b).0;
        Ok(if acv != bcv && acv & bcv == bcv {
            Ordering::Greater
        } else if acv != bcv && acv & bcv == acv {
            Ordering::Less
        } else {
            Ordering::Equal
        })
    }

    pub(super) fn named_conversion_dominated(
        &mut self,
        bindings: &[BindingId],
        index: usize,
    ) -> Result<bool, Error> {
        let binding = bindings[index];
        let entity = self.graph.binding(binding).entity;
        let primary = self
            .graph
            .function_templates
            .primary(entity)
            .ok_or(Error::Invariant(
                "conversion specialization without primary",
            ))?;
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("conversion specialization without type"))?;
        let TypeKind::Function { cv, reference, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant(
                "conversion specialization without function type",
            ));
        };
        for (other_index, &other) in bindings.iter().enumerate() {
            let other_entity = self.graph.binding(other).entity;
            if other_entity == entity {
                if other_index < index {
                    return Ok(true);
                }
                continue;
            }
            let other_ty = self
                .graph
                .binding(other)
                .ty
                .ok_or(Error::Invariant("conversion specialization without type"))?;
            if !matches!(self.graph.types.get(other_ty).kind,
                TypeKind::Function { cv: other_cv, reference: other_ref, .. }
                    if cv == other_cv && reference == other_ref)
            {
                continue;
            }
            let other_primary =
                self.graph
                    .function_templates
                    .primary(other_entity)
                    .ok_or(Error::Invariant(
                        "conversion specialization without primary",
                    ))?;
            if self.conversion_template_order(other_primary, primary)? == Ordering::Greater {
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
        sema::{
            expression_facts::{ExprFacts, ValueCategory},
            lookup::Filter,
            types::Scalar,
        },
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
        types::FundamentalType,
    };

    #[test]
    fn result_queries_reuse_signatures_without_body_or_syntax_demand() {
        let source = SourceBuffer::new(
            b"struct A{template<class T>operator T*(){return T::bad;}\
              template<class T>operator T(){return T::other;}};\
              struct R{template<class T>operator T&(){return T::bad;}};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"conversion-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax = (tree.expressions.mark(), tree.declarations.mark());
        let a_name = cursor.intern_name(b"A").unwrap();
        let r_name = cursor.intern_name(b"R").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let a = analyzer
            .graph
            .binding(
                analyzer
                    .graph
                    .own(global, a_name, Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        let r = analyzer
            .graph
            .binding(
                analyzer
                    .graph
                    .own(global, r_name, Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        let pointer_primary = analyzer.graph.entity(a).declared_conversions[0];
        let value_primary = analyzer.graph.entity(a).declared_conversions[1];
        let reference_primary = analyzer.graph.entity(r).declared_conversions[0];
        let published = analyzer.graph.binding(pointer_primary).ty;
        let int = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let pointer = analyzer.graph.types.pointer(int).unwrap();
        let constant = analyzer.graph.types.qualify(int, Cv::CONST).unwrap();
        let reference = analyzer
            .graph
            .types
            .reference(constant, RefKind::Lvalue)
            .unwrap();
        let found = analyzer
            .named_conversion_lookup(a, pointer, crate::support::source::SourceOffset(0))
            .unwrap();
        assert_eq!(found.functions.as_slice().len(), 1);
        let selected = found.binding.unwrap();
        assert_eq!(
            analyzer
                .graph
                .function_templates
                .primary(analyzer.graph.binding(selected).entity),
            Some(pointer_primary)
        );
        assert_eq!(
            analyzer
                .conversion_template_order(pointer_primary, value_primary)
                .unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            analyzer
                .conversion_template_order(value_primary, pointer_primary)
                .unwrap(),
            Ordering::Less
        );
        let named = analyzer
            .deduce_conversion_template(reference_primary, reference, ConversionDeduction::Named)
            .unwrap()
            .unwrap();
        let implicit = analyzer
            .deduce_conversion_template(reference_primary, reference, ConversionDeduction::Implicit)
            .unwrap()
            .unwrap();
        assert_eq!(named, implicit);
        assert_eq!(
            analyzer.graph.function_templates.arguments[&analyzer.graph.binding(named).entity],
            [constant]
        );
        let named_value = analyzer
            .deduce_conversion_template(value_primary, reference, ConversionDeduction::Named)
            .unwrap()
            .unwrap();
        assert_eq!(
            analyzer.graph.function_templates.arguments
                [&analyzer.graph.binding(named_value).entity],
            [reference]
        );
        // A value template's implicit result constructs a const-reference's
        // temporary using T=int, rather than T=const int or T=const int&.
        let value = analyzer
            .deduce_conversion_template(value_primary, reference, ConversionDeduction::Implicit)
            .unwrap()
            .unwrap();
        assert_eq!(
            analyzer.graph.function_templates.arguments[&analyzer.graph.binding(value).entity],
            [int]
        );
        let argument_entries = analyzer.graph.function_templates.arguments.len();
        let facts = ExprFacts::new(analyzer.graph.entity(a).ty.unwrap(), ValueCategory::Lvalue);
        for _ in 0..3 {
            assert_eq!(
                analyzer
                    .named_conversion_lookup(a, pointer, crate::support::source::SourceOffset(0))
                    .unwrap()
                    .binding,
                Some(selected)
            );
            let sequence = analyzer
                .scalar_conversion_function_sequence(facts, pointer, false)
                .unwrap()
                .unwrap();
            assert!(
                matches!(sequence.user_conversion, Some(super::super::procedural_calls::UserConversion::Function(binding)) if binding == selected)
            );
        }
        assert_eq!(
            analyzer.graph.function_templates.arguments.len(),
            argument_entries
        );
        assert_eq!(analyzer.graph.binding(pointer_primary).ty, published);
        for primary in [pointer_primary, value_primary, reference_primary] {
            assert!(
                !analyzer
                    .graph
                    .function_templates
                    .body_demanded(analyzer.graph.binding(primary).entity)
            );
        }
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
    }
}
