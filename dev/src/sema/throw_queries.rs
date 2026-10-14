// Translated from analyzer_exceptions.cpp::noexcept_expression/may_throw. See NOTICE.
//! Unevaluated procedural throw queries consume selected calls and original AST facts.
use super::{
    Error,
    analyzer::Analyzer,
    constants::Constant,
    exceptions::TargetUnwind,
    expression_facts::{ExprFacts, ValueCategory},
    function_resolution::CallTarget,
    graph::ScopeId,
    types::{Scalar, TypeKind},
};
use crate::syntax::ast::{self, ExprKind};

impl Analyzer<'_> {
    fn constructor_may_throw(
        &mut self,
        id: super::construction::ConstructorId,
        expression: ast::ExprId,
        depth: u16,
    ) -> Result<bool, Error> {
        let plan = self.graph.constructors.get(id);
        if let Some(binding) = plan.binding {
            let entity = self.graph.binding(binding).entity;
            if self.function_may_throw(entity)? {
                return Ok(true);
            }
            let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expression).kind
            else {
                return Err(Error::Invariant("constructor throw query without list"));
            };
            let written = self.tree.expr_lists.get(elements).len();
            for default in self
                .graph
                .entity(entity)
                .defaults
                .iter()
                .skip(written)
                .rev()
                .flatten()
            {
                self.throw_work.push((default.expression, depth + 1));
            }
            Ok(false)
        } else {
            Ok(!plan.nothrow)
        }
    }
    pub(super) fn initializer_may_throw(
        &mut self,
        initializer: ast::Initializer,
    ) -> Result<bool, Error> {
        match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                self.expression_may_throw(id)
            }
            ast::InitializerKind::Parenthesized(arguments) => {
                for &id in self.tree.expr_lists.get(arguments) {
                    if self.expression_may_throw(id)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn noexcept_expression(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        if let Ok(facts) = self.graph.expression_facts.get(id) {
            return Ok(facts);
        }
        self.unevaluated += 1;
        let result = self.typed_expression(operand, scope);
        self.unevaluated -= 1;
        result?;
        let bits = u64::from(!self.expression_may_throw(operand)?);
        let ty = self.graph.types.scalar(Scalar::Bool)?;
        let mut facts = self.new_expression_facts(ty, ValueCategory::Prvalue)?;
        facts.folded = Some(bits);
        self.graph.expression_facts.set(id, facts);
        super::record(
            &mut self.graph.constant_facts,
            id.index(),
            Constant { bits, ty },
        );
        Ok(facts)
    }

    pub(super) fn expression_may_throw(&mut self, root: ast::ExprId) -> Result<bool, Error> {
        // The C++ query's semantic depth bound is independent of stack exhaustion.
        // Nested pending specifications share capacity while retaining separate bases.
        // Only capacity survives a query; the analyzer releases it before lowering.
        let base = self.throw_work.len();
        self.throw_work.push((root, 0_u16));
        let result = self.walk_throw_query(base);
        self.throw_work.truncate(base);
        result
    }

    fn aggregate_destruction_may_throw(
        &mut self,
        class: super::graph::EntityId,
    ) -> Result<bool, Error> {
        let mut work = vec![class];
        let mut seen = std::collections::HashSet::new();
        while let Some(class) = work.pop() {
            if !seen.insert(class) {
                continue;
            }
            if let Some(binding) = self.graph.entity(class).declared_destructor {
                if self.function_may_throw(self.graph.binding(binding).entity)? {
                    return Ok(true);
                }
                continue;
            }
            work.extend(self.graph.entity(class).bases.iter().map(|base| base.class));
            for &member in &self.graph.entity(class).members {
                let mut ty = self
                    .graph
                    .entity(member)
                    .ty
                    .ok_or(Error::Invariant("member without type"))?;
                while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                if let TypeKind::Class(class) = self.graph.types.get(ty).kind {
                    work.push(class);
                }
            }
        }
        Ok(false)
    }

    fn walk_throw_query(&mut self, base: usize) -> Result<bool, Error> {
        while self.throw_work.len() > base {
            let (id, depth) = self
                .throw_work
                .pop()
                .ok_or(Error::Invariant("missing throw-query frame"))?;
            let id = self.graph.expression_facts.root(id);
            let Ok(facts) = self.graph.expression_facts.get(id) else {
                continue;
            };
            if depth > 256 {
                continue;
            }
            self.check_stack()?;
            let kind = self.tree.expressions.get(id).kind;
            if facts.folded.is_some() && !matches!(kind, ExprKind::Member { .. }) {
                continue;
            }
            if let TypeKind::Class(class) = self.graph.types.get(facts.ty).kind
                && facts.category == ValueCategory::Prvalue
            {
                if let Some(constructor) = facts.constructor {
                    if self.constructor_may_throw(constructor, id, depth)? {
                        return Ok(true);
                    }
                } else if !matches!(kind, ExprKind::Braced { .. })
                    || !self.graph.aggregate_class(class)
                {
                    return Err(Error::Missing("class temporary exception queries"));
                }
                if self.aggregate_destruction_may_throw(class)? {
                    return Ok(true);
                }
            }
            if facts.call_form == super::expression_facts::CallForm::Operator
                && let Some(CallTarget::Direct(binding)) = facts.call
                && self.function_may_throw(self.graph.binding(binding).entity)?
            {
                return Ok(true);
            }
            let next = depth + 1;
            match kind {
                ExprKind::Throw(_) => return Ok(true),
                ExprKind::Call { callee, arguments } if !facts.functional_call => {
                    if matches!(
                        facts.destructor,
                        Some(super::expression_facts::DestructorTarget::Scalar)
                    ) {
                        self.throw_work.push((callee, next));
                        continue;
                    }
                    match facts
                        .call
                        .ok_or(Error::Invariant("throw query call without target"))?
                    {
                        CallTarget::Direct(binding) => {
                            let entity = self.graph.binding(binding).entity;
                            if self.function_may_throw(entity)? {
                                return Ok(true);
                            }
                            let defaults = &self.graph.entity(entity).defaults;
                            let count = self.tree.expr_lists.get(arguments).len();
                            for default in defaults.iter().skip(count).rev().flatten() {
                                self.throw_work.push((default.expression, next));
                            }
                        }
                        CallTarget::Indirect(_, entity) => {
                            if !entity.is_some_and(|entity| {
                                self.graph.entity(entity).target_unwind == TargetUnwind::NoThrow
                            }) {
                                return Ok(true);
                            }
                        }
                    }
                    self.throw_work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                    self.throw_work.push((callee, next));
                }
                ExprKind::Call { arguments, .. } => {
                    self.throw_work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                }
                ExprKind::Paren(inner)
                | ExprKind::Unary { operand: inner, .. }
                | ExprKind::Postfix { operand: inner, .. }
                | ExprKind::Cast { operand: inner, .. }
                | ExprKind::Member { base: inner, .. } => self.throw_work.push((inner, next)),
                ExprKind::Binary { left, right, .. }
                | ExprKind::Subscript {
                    base: left,
                    index: right,
                } => {
                    self.throw_work.extend([(right, next), (left, next)]);
                }
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    self.throw_work.extend([
                        (else_expr, next),
                        (then_expr, next),
                        (condition, next),
                    ]);
                }
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    let elements = if let Some(braced) = braced {
                        let ExprKind::Braced { elements, .. } =
                            self.tree.expressions.get(braced).kind
                        else {
                            return Err(Error::Invariant("functional cast without brace elements"));
                        };
                        // A scalar cast records facts on its elements and result,
                        // not on the intervening initializer-list syntax node.
                        elements
                    } else {
                        arguments
                    };
                    self.throw_work.extend(
                        self.tree
                            .expr_lists
                            .get(elements)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                }
                ExprKind::Braced { elements, .. } => {
                    self.throw_work.extend(
                        self.tree
                            .expr_lists
                            .get(elements)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                }
                ExprKind::New { .. } | ExprKind::Delete { .. } => {
                    return Err(Error::Missing("allocation exception queries"));
                }
                // sizeof, traits (including nested noexcept) and lambdas
                // have no potentially evaluated child in this query.
                _ => (),
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lex::vocabulary::Keyword,
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    fn return_expression(
        tree: &ast::Tree,
        declarations: crate::syntax::storage::List<ast::DeclId>,
    ) -> ast::ExprId {
        let declaration = *tree.decl_lists.get(declarations).last().unwrap();
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(declaration).kind else {
            panic!("missing function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("unparsed body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("missing compound");
        };
        let ast::StmtKind::Return(Some(expr)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("missing return");
        };
        expr
    }

    #[test]
    fn folded_queries_preserve_selected_targets_without_emission_demand() {
        let source = SourceBuffer::new(b"inline int f(int)noexcept{return 1;} inline int f(double){return 2;} int(*p)(int)noexcept; int g(int=1)noexcept; int h()noexcept(noexcept(g())){return 3;} int main(){return noexcept(f(1))+noexcept(f(1.0))+noexcept(p(2));}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"throw-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), count);
        assert!(super::super::function_demands::trailing(&graph).is_empty());
        let mut values = Vec::new();
        let mut calls = Vec::new();
        let mut work = vec![return_expression(tree, declarations)];
        while let Some(id) = work.pop() {
            let node = tree.expressions.get(id);
            if let ExprKind::Binary { left, right, .. } = node.kind {
                work.extend([right, left]);
            }
            if let ExprKind::TypeTrait {
                keyword: Keyword::Noexcept,
                operand: Some(operand),
                ..
            } = node.kind
            {
                values.push(graph.expression_facts.get(id).unwrap().folded.unwrap());
                if let ExprKind::Call { .. } = tree.expressions.get(operand).kind {
                    calls.push(graph.expression_facts.get(operand).unwrap().call.unwrap());
                }
            }
        }
        assert_eq!(values, [1, 0, 1]);
        assert_eq!(calls.len(), 3);
        let (CallTarget::Direct(first), CallTarget::Direct(second)) = (calls[0], calls[1]) else {
            panic!("overload queries lost their targets");
        };
        assert_ne!(graph.binding(first).entity, graph.binding(second).entity);
        assert!(matches!(calls[2], CallTarget::Indirect(_, Some(_))));
    }

    #[test]
    fn invalid_unevaluated_operand_restores_demand_context() {
        let source = SourceBuffer::new(
            b"int f(){return 1;} int main(){return noexcept(f()+missing());}".to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"throw-error").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let id = return_expression(tree, declarations);
        let ExprKind::TypeTrait {
            operand: Some(operand),
            ..
        } = tree.expressions.get(id).kind
        else {
            panic!("return is not a noexcept query");
        };
        assert!(
            analyzer
                .noexcept_expression(id, operand, analyzer.graph.global)
                .is_err()
        );
        assert_eq!(analyzer.unevaluated, 0);
        assert!(analyzer.graph.expression_facts.get(id).is_err());
    }
}
