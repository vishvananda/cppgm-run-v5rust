// Translated from sema/analyzer_expressions.cpp::arrow_operator/member_access.
// See NOTICE.
//! Bounded member-only call chains retain selected entities without rewriting syntax.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ObjectArgument, ObjectSource, ValueCategory},
    graph::{BindingId, BindingKind, FunctionAvailability, ScopeId},
    inheritance::BasePathId,
    lookup::Filter,
    procedural_calls::Candidates,
    types::TypeKind,
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast};

#[derive(Clone, Copy)]
pub(super) struct ArrowStep {
    pub binding: BindingId,
    pub result: super::types::TypeId,
    pub category: ValueCategory,
    pub path: Option<BasePathId>,
}
impl Analyzer<'_> {
    pub(super) fn arrow_calls(
        &mut self,
        member: ast::ExprId,
        base: ast::ExprId,
        mut object: ExprFacts,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(member).location;
        let mut calls = Vec::new();
        while let TypeKind::Class(class) = self.graph.types.get(object.ty).kind {
            if calls.len() == 64 {
                return Err(Error::at(location, "operator-> chain too long"));
            }
            let home = self
                .graph
                .entity(class)
                .scope
                .filter(|_| self.graph.entity(class).complete)
                .ok_or(Error::at(
                    location,
                    "-> on an object of incomplete class type",
                ))?;
            if object.category == ValueCategory::Prvalue {
                self.class_parameter_destruction(object.ty, scope, location)?;
            }
            self.lookup_location = location;
            let found = self
                .qualified_result(
                    home,
                    super::operator_candidates::operator_key(P::Arrow)?,
                    Filter::Ordinary,
                )
                .map_err(|error| match error {
                    Error::Diagnostic {
                        message: "ambiguous name",
                        ..
                    } => Error::at(location, "ambiguous member name"),
                    other => other,
                })?;
            let binding = found
                .binding
                .filter(|&binding| self.graph.binding(binding).kind == BindingKind::Function)
                .ok_or(Error::at(
                    location,
                    "-> requires a pointer or an object of a class with operator->",
                ))?;
            let designation = self.function_designation(binding, found.functions.as_slice())?;
            let set = designation
                .functions
                .ok_or(Error::Invariant("arrow designation without candidates"))?;
            let receiver = ObjectArgument {
                source: ObjectSource::Explicit {
                    expression: base,
                    arrow: false,
                },
                ty: object.ty,
                category: object.category,
            };
            let mut scratch = std::mem::take(&mut self.call_scratch);
            let selected = self.resolve_call(
                location,
                Candidates::Functions(set),
                &[],
                Some(receiver),
                &mut scratch,
            );
            scratch.clear();
            self.call_scratch = scratch;
            let binding = selected?.binding;
            let entity = self.graph.binding(binding).entity;
            if self.graph.entity(entity).availability == FunctionAvailability::Deleted {
                return Err(Error::at(location, "use of a deleted function"));
            }
            self.check_named_member_access(binding, class, class, scope, location)?;
            let owner = self
                .member_function_owner(binding)
                .ok_or(Error::Invariant("static arrow operator"))?;
            let owner = self
                .graph
                .class_of(owner)
                .ok_or(Error::Invariant("arrow owner without class"))?;
            if self.graph.base_count(class, owner) > 1 {
                return Err(Error::at(
                    location,
                    "a non-static member of an ambiguous base class subobject",
                ));
            }
            let conversion =
                self.object_sequence(receiver, binding, false)?
                    .ok_or(Error::Invariant(
                        "selected arrow receiver without conversion",
                    ))?;
            let path = self.conversion_base_path(object, conversion, scope, location, false)?;
            self.check_specialization_body(binding)?;
            self.demand_function(binding, true);
            let function = self
                .graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("arrow without type"))?;
            object = self.call_result(function)?;
            if object.category == ValueCategory::Prvalue {
                self.class_parameter_destruction(object.ty, scope, location)?;
                if self.lowering()
                    && self.unevaluated == 0
                    && let TypeKind::Class(result) = self.graph.types.get(object.ty).kind
                {
                    self.request_storage_layout(result, location)?;
                }
            }
            calls.push(ArrowStep {
                binding,
                result: object.ty,
                category: object.category,
                path,
            });
        }
        self.graph.expression_facts.set_arrow_calls(base, calls);
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{
            SyntaxUnit,
            ast::{Body, DeclKind, ExprKind, StmtKind},
        },
    };

    #[test]
    fn chains_keep_original_syntax_and_source_type_with_distinct_selected_calls() {
        let source = SourceBuffer::new(b"struct C{int n;};struct P{C*operator->()const;};struct Q{P operator->()const;};int f(const Q&q){return q->n;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"arrow-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), count);
        let DeclKind::Function { body, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("function")
        };
        let Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body")
        };
        let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound")
        };
        let StmtKind::Return(Some(member)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return")
        };
        let ExprKind::Member { base, .. } = tree.expressions.get(member).kind else {
            panic!("member")
        };
        let source = graph.expression_facts.get(base).unwrap();
        assert!(matches!(
            graph.types.get(source.ty).kind,
            TypeKind::Class(_)
        ));
        assert!(source.conversion.is_none());
        let calls = graph.expression_facts.arrow_calls(base);
        assert_eq!(calls.len(), 2);
        assert_ne!(calls[0].binding, calls[1].binding);
        assert!(matches!(
            graph.types.get(calls[0].result).kind,
            TypeKind::Class(_)
        ));
        assert!(matches!(
            graph.types.get(calls[1].result).kind,
            TypeKind::Pointer(_)
        ));
        assert!(
            graph
                .expression_facts
                .get(member)
                .unwrap()
                .conversion
                .is_some()
        );
        assert_eq!(graph.root_demands.as_slice().len(), 2);
    }
}
