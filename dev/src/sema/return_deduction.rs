// Translated from analyzer_lambdas.cpp and analyzer_bodies.cpp. See NOTICE.
//! Placeholder results are published on their canonical function entity once
//! a return determines them. Template definitions retain their placeholders;
//! only a selected specialization's body is demanded to determine its result.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::ExprFacts,
    graph::{EntityId, Graph, ScopeId},
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast};

#[derive(Clone, Copy)]
pub(super) enum ReturnDeduction {
    Fixed,
    Pending,
    Ready(TypeId),
}

impl Graph {
    pub(super) fn set_function_result(
        &mut self,
        function: EntityId,
        result: TypeId,
    ) -> Result<(), Error> {
        let old = self
            .entity(function)
            .ty
            .ok_or(Error::Invariant("function type"))?;
        let TypeKind::Function {
            parameters,
            variadic,
            cv,
            reference,
            ..
        } = self.types.get(old).kind.clone()
        else {
            return Err(Error::Invariant("function result without signature"));
        };
        let ty = self.types.intern(
            TypeKind::Function {
                result,
                parameters,
                variadic,
                cv,
                reference,
            },
            Cv::default(),
        )?;
        self.entity_mut(function).ty = Some(ty);
        let binding = self.function_origin(function)?;
        self.set_function_binding_type(binding, ty);
        Ok(())
    }
}

impl Analyzer<'_> {
    pub(super) fn require_deduced_result(
        &mut self,
        function: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let ty = self
            .graph
            .entity(function)
            .ty
            .ok_or(Error::Invariant("function type"))?;
        let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("result demand without function"));
        };
        if !self.holds_auto(result) {
            return Ok(());
        }
        if (self.graph.function_templates.primary(function).is_some()
            || self.graph.function_templates.bodies.member(function))
            && self.constant_function_definition(function)?
        {
            let ty = self
                .graph
                .entity(function)
                .ty
                .ok_or(Error::Invariant("deduced function type"))?;
            if let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind
                && !self.holds_auto(result)
            {
                return Ok(());
            }
        }
        self.reject_substitution(
            location,
            "a function whose return type is not deduced yet (7.1.6.4p11)",
            false,
        )
    }

    fn return_type(
        &mut self,
        facts: Option<ExprFacts>,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let declared = self
            .body_context
            .as_ref()
            .ok_or(Error::Invariant("return deduction without body"))?
            .placeholder;
        let plain =
            declared.is_none_or(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Auto(_)));
        let Some(facts) = facts else {
            return if plain {
                self.graph.types.scalar(Scalar::Void)
            } else {
                Err(Error::at(
                    location,
                    "a return without a value deduces no reference or pointer to auto",
                ))
            };
        };
        let Some(declared) = declared else {
            return self.decayed_type(facts.ty);
        };
        if matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return if plain {
                self.graph.types.scalar(Scalar::Void)
            } else {
                Err(Error::at(
                    location,
                    "a void return deduces no reference or pointer to auto",
                ))
            };
        }
        let mut arguments = [None];
        if !self.deduce_call_argument(declared, facts.ty, facts.category, 0, &mut arguments)? {
            arguments[0] = None;
        }
        let replacing = arguments[0].ok_or_else(|| {
            Error::at(
                location,
                "the return statement deduces no return type (7.1.6.4p7)",
            )
        })?;
        let ty = self.replace_auto(declared, replacing, Cv::default())?;
        self.graph.types.canonical_function(ty)
    }

    fn publish_return(
        &mut self,
        facts: Option<ExprFacts>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let ty = self.return_type(facts, location)?;
        self.check_abstract_object(ty, location)?;
        let body = self
            .body_context
            .as_mut()
            .ok_or(Error::Invariant("return deduction context"))?;
        if matches!(body.return_deduction, ReturnDeduction::Ready(earlier) if earlier != ty) {
            return Err(Error::at(
                location,
                "return statements deduce different return types (7.1.6.4p9)",
            ));
        }
        if matches!(body.return_deduction, ReturnDeduction::Pending) {
            body.return_deduction = ReturnDeduction::Ready(ty);
            body.result = ty;
            let function = body
                .function
                .ok_or(Error::Invariant("return deduction without body function"))?;
            self.graph.set_function_result(function, ty)?;
        }
        Ok(())
    }

    pub(super) fn deduce_return(
        &mut self,
        stmt: ast::StmtId,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<Option<ExprFacts>, Error> {
        if self
            .body_context
            .as_ref()
            .is_none_or(|body| matches!(body.return_deduction, ReturnDeduction::Fixed))
        {
            return Ok(None);
        }
        let facts = if let Some(operand) = operand {
            if matches!(
                self.tree.expressions.get(operand).kind,
                ast::ExprKind::Braced { .. }
            ) {
                return Err(Error::at(
                    self.tree.expressions.get(operand).location,
                    "a braced-init-list deduces no return type (7.1.6.4p7)",
                ));
            }
            self.typed_expression(operand, scope)?;
            self.check_resolved_function(operand)?;
            Some(self.graph.expression_facts.get(operand)?)
        } else {
            None
        };
        self.publish_return(facts, self.tree.statements.get(stmt).location)?;
        Ok(facts)
    }

    pub(super) fn finish_return_deduction(&mut self, location: SourceOffset) -> Result<(), Error> {
        if self.body_context.as_ref().is_some_and(|body| {
            !body.undeduced
                && body.placeholder.is_some()
                && matches!(body.return_deduction, ReturnDeduction::Pending)
        }) {
            self.publish_return(None, location)?;
        }
        Ok(())
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
    fn redeclarations_display_the_shared_deduced_signature() {
        let source = SourceBuffer::new(
            b"auto f();auto f();auto f(){return 1;}\
              struct C{auto g();};auto C::g(){return 2L;}\
              auto& r();int object;auto& r(){return object;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"deduced-redeclarations").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let mut output = Vec::new();
        graph
            .print_semantics(tree, cursor, declarations, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("function-definition f function of () returning int\n"));
        assert!(output.contains(
            "function-definition C::g function of (pointer to struct C) returning long int\n"
        ));
        assert!(
            output.contains(
                "function-definition r function of () returning lvalue-reference to int\n"
            )
        );
        assert!(!output.contains("typename auto"));
    }

    #[test]
    fn selected_result_demands_reuse_one_body_and_preserve_template_patterns() {
        let source = SourceBuffer::new(
            b"template<class T>auto identity(T value){return value;}\
              template<class T>auto unused(T value){return value.missing;}\
              auto first(){return identity(1);}\
              auto second(){return identity(2);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"result-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let original_bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        assert_eq!(analyzer.graph.function_templates.arguments.len(), 1);
        let selected = *analyzer
            .graph
            .function_templates
            .arguments
            .keys()
            .next()
            .unwrap();
        let primary = analyzer.graph.function_templates.primary(selected).unwrap();
        let primary = analyzer.graph.binding(primary).entity;
        let pattern = analyzer.graph.entity(primary).ty.unwrap();
        let TypeKind::Function { result, .. } = analyzer.graph.types.get(pattern).kind else {
            panic!("function pattern");
        };
        assert!(analyzer.holds_auto(result));
        let concrete = analyzer.graph.entity(selected).ty.unwrap();
        let TypeKind::Function { result, .. } = analyzer.graph.types.get(concrete).kind else {
            panic!("concrete function");
        };
        assert!(!analyzer.holds_auto(result));
        assert_eq!(analyzer.tree.bodies.mark(), original_bodies + 1);
        let body = analyzer.graph.entity(selected).callable().analyzed_body;
        let marks = (
            analyzer.tree.bodies.mark(),
            analyzer.tree.expressions.mark(),
            analyzer.graph.binding_limit(),
        );
        for _ in 0..4 {
            analyzer
                .require_deduced_result(selected, SourceOffset(0))
                .unwrap();
        }
        assert_eq!(analyzer.graph.entity(primary).ty, Some(pattern));
        assert_eq!(analyzer.graph.entity(selected).ty, Some(concrete));
        assert_eq!(
            analyzer.graph.entity(selected).callable().analyzed_body,
            body
        );
        assert_eq!(
            marks,
            (
                analyzer.tree.bodies.mark(),
                analyzer.tree.expressions.mark(),
                analyzer.graph.binding_limit()
            )
        );
        assert!(analyzer.body_context.is_none());
        assert_eq!(analyzer.template_depth, 0);
    }

    #[test]
    fn ordinary_results_publish_immutable_signatures_without_new_syntax() {
        use super::super::{lookup::Filter, types::RefKind};
        let source = SourceBuffer::new(
            b"int object;auto&& reference(){return object;}\
              const auto* pointer(){return &object;}auto empty(){}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"result-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"reference".as_slice(), b"pointer", b"empty"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let marks = (
            tree.bodies.mark(),
            tree.expressions.mark(),
            tree.statements.mark(),
        );
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(
            marks,
            (
                tree.bodies.mark(),
                tree.expressions.mark(),
                tree.statements.mark()
            )
        );
        let results: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = graph.own(graph.global, name, Filter::Ordinary).unwrap();
                let ty = graph.entity(graph.binding(binding).entity).ty.unwrap();
                assert_eq!(graph.binding(binding).ty, Some(ty));
                let TypeKind::Function { result, .. } = graph.types.get(ty).kind else {
                    panic!("function result");
                };
                result
            })
            .collect();
        assert!(matches!(
            graph.types.get(results[0]).kind,
            TypeKind::Reference {
                kind: RefKind::Lvalue,
                ..
            }
        ));
        let TypeKind::Pointer(element) = graph.types.get(results[1]).kind else {
            panic!("pointer result");
        };
        assert_eq!(graph.types.get(element).cv, Cv::CONST);
        assert!(matches!(
            graph.types.get(results[2]).kind,
            TypeKind::Scalar(Scalar::Void)
        ));
    }
}
