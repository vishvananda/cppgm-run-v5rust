// Translated from sema/analyzer_builtins.cpp::builtin_invoke. See NOTICE.
//! INVOKE's member forms reuse ordinary member access and call analysis.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::ExprFacts,
    graph::ScopeId,
    types::{TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::SimpleToken},
    syntax::ast,
};

impl Analyzer<'_> {
    pub(super) fn invoke_member(
        &mut self,
        id: ast::ExprId,
        callable: ast::ExprId,
        arguments: &[ast::ExprId],
        owner: TypeId,
        element: TypeId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let data = !matches!(
            self.graph.types.get(element).kind,
            TypeKind::Function { .. }
        );
        let Some((&object, rest)) = arguments
            .split_first()
            .filter(|(_, rest)| !data || rest.is_empty())
        else {
            return Err(Error::at(
                location,
                if data {
                    "__builtin_invoke of a pointer to data member needs one object"
                } else {
                    "__builtin_invoke of a pointer to member function without an object"
                },
            ));
        };
        let target = self.typed_expression(object, scope)?;
        let mut left = object;
        let mut operation = P::DotStar;
        let direct = match (self.graph.class_of(owner), self.graph.class_of(target.ty)) {
            (Some(owner), Some(class)) => class == owner || self.graph.is_base_of(owner, class),
            _ => false,
        };
        if !direct {
            if self.graph.class_of(target.ty).is_none() {
                operation = P::ArrowStar;
            } else {
                left = self
                    .tree
                    .expressions
                    .add(ast::Expression {
                        location: self.tree.expressions.get(object).location,
                        kind: ast::ExprKind::Unary {
                            op: ast::Operator {
                                token: SimpleToken::punctuator(P::Multiply)
                                    .ok_or(Error::Invariant("invoke indirection token"))?,
                                alternative: false,
                            },
                            operand: object,
                        },
                    })
                    .map_err(|_| Error::Limit("invoke syntax limit"))?;
            }
        }
        let access = self
            .tree
            .expressions
            .add(ast::Expression {
                location,
                kind: ast::ExprKind::Binary {
                    op: ast::Operator {
                        token: SimpleToken::punctuator(operation)
                            .ok_or(Error::Invariant("invoke access token"))?,
                        alternative: false,
                    },
                    left,
                    right: callable,
                    assignment: false,
                },
            })
            .map_err(|_| Error::Limit("invoke syntax limit"))?;
        let callee = self
            .tree
            .expressions
            .add(ast::Expression {
                location: self.tree.expressions.get(callable).location,
                // For data the callee stays without facts, as in the oracle's
                // functional-call form. Only the access is evaluated.
                kind: ast::ExprKind::Paren(if data { callable } else { access }),
            })
            .map_err(|_| Error::Limit("invoke syntax limit"))?;
        let start = self.tree.expr_lists.begin();
        if data {
            self.tree.expr_lists.push(access);
        } else {
            for &argument in rest {
                self.tree.expr_lists.push(argument);
            }
        }
        let arguments = self
            .tree
            .expr_lists
            .finish(start)
            .map_err(|_| Error::Limit("invoke argument limit"))?;
        self.tree.expressions.replace(
            id,
            ast::Expression {
                location,
                kind: ast::ExprKind::Call { callee, arguments },
            },
        );
        if !data {
            return self.invoke_call(id, callable, Some(object), scope);
        }
        let saved_callable = self.pretyped_callable.replace(callable);
        let saved_object = self.pretyped_object.replace(object);
        let result = self.typed_expression(access, scope);
        self.pretyped_callable = saved_callable;
        self.pretyped_object = saved_object;
        let access = result?;
        let mut facts = self.new_expression_facts(access.ty, access.category)?;
        facts.functional_call = true;
        self.graph.expression_facts.note_member_data_call_view(id);
        Ok(facts)
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
    fn member_signature_queries_preserve_demand_and_restore_operand_context() {
        let source = SourceBuffer::new(
            b"template<class T>T&& dv();\
            struct R;\
            struct C{R call(int);int data;};\
            struct P{C& operator*()const;};\
            typedef decltype(__builtin_invoke(&C::call,dv<P>(),1)) A;\
            typedef decltype(__builtin_invoke(&C::data,dv<P>())) B;\
            typedef decltype(__builtin_invoke(&C::data,(dv<C>(),dv<C>()))) D;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"invoke-member").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(analyzer.pretyped_callable, None);
        assert_eq!(analyzer.pretyped_object, None);
        assert!(analyzer.body_context.is_none());
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.decltype_call, None);
    }

    #[test]
    fn nested_member_invoke_failure_restores_both_pretyped_operands() {
        for expression in [
            "__builtin_invoke(&C::call,c,__builtin_invoke(&C::call,c,missing))",
            "__builtin_invoke(&C::data,p)",
            "__builtin_invoke(&C::call,c,missing)",
        ] {
            let source = SourceBuffer::new(
                format!("struct C{{int call(int);int data;}};struct P{{}};int f(C&c,P&p){{return {expression};}}")
                    .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"invoke-error").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            assert!(matches!(
                analyzer.declarations(declarations, global),
                Err(Error::Diagnostic { .. })
            ));
            assert_eq!(analyzer.pretyped_callable, None);
            assert_eq!(analyzer.pretyped_object, None);
            assert!(!analyzer.call_scratch.has_candidates());
            assert_eq!(analyzer.unevaluated, 0);
        }
    }
}
