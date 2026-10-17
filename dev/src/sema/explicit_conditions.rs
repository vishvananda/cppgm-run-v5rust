// Translated from template_names.cpp::explicit_value/specialized_explicit and
// analyzer_declarations.cpp::function_from_declarator. See NOTICE.
//! Conditions retain their declaration frame; specialization decides before publication.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, EntityId, ScopeId, ScopeKind},
    specifiers::{Explicitness, Specifiers},
    types::{Scalar, TypeId},
};
use crate::syntax::ast::ExprId;
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Pending {
    condition: ExprId,
    scope: ScopeId,
}
#[derive(Default)]
pub(super) struct Conditions(HashMap<EntityId, Pending>);

impl Analyzer<'_> {
    pub(super) fn record_explicit_condition(
        &mut self,
        function: EntityId,
        specs: Specifiers,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if specs.explicit_condition.is_none() && specs.explicitness == Explicitness::Converting {
            return Ok(());
        }
        let explicitness = if let Some(condition) = specs.explicit_condition {
            if self.template_depth == 0 {
                self.explicit_value(condition, scope)?
            } else {
                self.graph
                    .function_templates
                    .conditions
                    .0
                    .insert(function, Pending { condition, scope });
                Explicitness::Converting
            }
        } else {
            specs.explicitness
        };
        self.graph.entity_mut(function).explicitness = explicitness;
        Ok(())
    }

    pub(super) fn specialized_explicitness(
        &mut self,
        primary: BindingId,
        depth: u32,
        arguments: &[TypeId],
    ) -> Result<Explicitness, Error> {
        let entity = self.graph.binding(primary).entity;
        let Some(pending) = self
            .graph
            .function_templates
            .conditions
            .0
            .get(&entity)
            .copied()
        else {
            return Ok(self.graph.entity(entity).explicitness);
        };
        let parent = self
            .graph
            .scope(pending.scope)
            .parent
            .ok_or(Error::Invariant("explicit condition without parent"))?;
        let frame = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, parent, None, None)?;
        for index in 0..self.graph.scope(pending.scope).bindings.len() {
            let source = self.graph.scope(pending.scope).bindings[index];
            let binding = self.graph.binding(source);
            if let Some(position) = self
                .graph
                .entity(binding.entity)
                .template_parameter
                .and_then(|info| info.index(depth))
                && let Some(&ty) = arguments.get(position)
            {
                self.bind_template_argument(frame, binding, ty)?;
            }
        }
        let copy = self.copy_expression_in_frame(pending.condition, frame)?;
        self.instantiation_context(
            self.instantiation_depth,
            self.tree.expressions.get(copy).location,
            self.graph.binding_limit(),
            |analyzer| analyzer.explicit_value(copy, frame),
        )
    }

    fn explicit_value(&mut self, condition: ExprId, scope: ScopeId) -> Result<Explicitness, Error> {
        // Unlike noexcept, this is evaluated: reads retain the oracle's
        // static-member declaration demand. Its converted-constant checks
        // are hard errors, distinct from a failed operand substitution.
        if self.semantic() {
            let facts = self.typed_expression(condition, scope)?;
            self.check_resolved_function(condition)?;
            let location = self.tree.expressions.get(condition).location;
            if self.graph.class_of(facts.ty).is_none() {
                let kind = self.integral_kind(facts.ty).ok_or_else(|| {
                    Error::at(location,
                        "an explicit-specifier's condition of a type no converted constant expression of type bool converts (5.19p3)")
                        .required_instantiation()
                })?;
                let before = self
                    .constant_value(condition, scope, true)
                    .map_err(Error::required_instantiation)?
                    .ok_or(Error::Invariant("explicit condition without constant"))?;
                let boolean = super::integral::IntegralKind::of(Scalar::Bool)
                    .ok_or(Error::Invariant("bool without integral kind"))?;
                if !boolean.represents(before.bits, kind) {
                    return Err(Error::at(
                        location,
                        "an explicit-specifier's condition narrowed by conversion to bool (8.5.4p7)",
                    ).required_instantiation());
                }
            }
            self.convert_bool(condition)?;
        }
        let value = self
            .constant_value(condition, scope, true)
            .map_err(Error::required_instantiation)?
            .ok_or(Error::Invariant("explicit condition without constant"))?;
        Ok(if value.bits == 0 {
            Explicitness::Converting
        } else {
            Explicitness::Explicit
        })
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
    fn conditions_are_isolated_memoized_and_never_publish_failed_signatures() {
        let source = SourceBuffer::new(
            b"struct A{template<class T>explicit(sizeof(T)>4) A(T);};\
              struct Bad{template<class T>explicit(sizeof(T)) Bad(T);};"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"conditions").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            cursor.intern_name(b"A").unwrap(),
            cursor.intern_name(b"Bad").unwrap(),
        ];
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.set_lowering();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let primaries = names.map(|name| {
            let binding = analyzer
                .graph
                .own(global, name, super::super::lookup::Filter::Ordinary)
                .unwrap();
            analyzer
                .graph
                .entity(analyzer.graph.binding(binding).entity)
                .declared_constructors[0]
        });
        let int = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::Int))
            .unwrap();
        let long = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::LongInt))
            .unwrap();
        analyzer.instantiation_depth = 7;
        analyzer.operand_home = Some(global);
        analyzer.unevaluated = 2;
        let implicit = analyzer
            .complete_signature(primaries[0], &mut [Some(int)])
            .unwrap()
            .unwrap();
        let explicit = analyzer
            .complete_signature(primaries[0], &mut [Some(long)])
            .unwrap()
            .unwrap();
        assert_ne!(implicit, explicit);
        assert!(
            analyzer
                .graph
                .entity(analyzer.graph.binding(implicit).entity)
                .explicitness
                == Explicitness::Converting
        );
        assert!(
            analyzer
                .graph
                .entity(analyzer.graph.binding(explicit).entity)
                .explicitness
                == Explicitness::Explicit
        );
        let expressions = analyzer.tree.expressions.mark();
        assert_eq!(
            analyzer
                .complete_signature(primaries[0], &mut [Some(int)])
                .unwrap(),
            Some(implicit)
        );
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        let published = analyzer.graph.function_templates.arguments.len();
        assert!(matches!(
            analyzer.complete_signature(primaries[1], &mut [Some(int)]),
            Err(Error::InstantiationDiagnostic { .. })
        ));
        assert_eq!(analyzer.graph.function_templates.arguments.len(), published);
        assert_eq!(analyzer.instantiation_depth, 7);
        assert_eq!(analyzer.operand_home, Some(global));
        assert_eq!(analyzer.unevaluated, 2);
        for primary in primaries {
            let entity = analyzer.graph.binding(primary).entity;
            let pattern = analyzer.graph.function_templates.conditions.0[&entity].condition;
            assert!(analyzer.graph.expression_facts.get(pattern).is_err());
            assert!(!analyzer.graph.function_templates.body_demanded(entity));
        }
    }
}
