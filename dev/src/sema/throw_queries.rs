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
    fn constructor_function_may_throw(
        &mut self,
        id: super::construction::ConstructorId,
    ) -> Result<bool, Error> {
        let plan = self.graph.constructors.get(id);
        if let Some(binding) = plan.binding {
            self.function_may_throw(self.graph.binding(binding).entity)
        } else {
            Ok(!plan.nothrow)
        }
    }
    fn constructor_may_throw(
        &mut self,
        id: super::construction::ConstructorId,
        arguments: super::constructor_initialization::ConstructorArguments,
        depth: u16,
    ) -> Result<bool, Error> {
        let plan = self.graph.constructors.get(id);
        if let Some(binding) = plan.binding {
            let entity = self.graph.binding(binding).entity;
            if self.function_may_throw(entity)? {
                return Ok(true);
            }
            let written = match arguments {
                super::constructor_initialization::ConstructorArguments::Values(values) => {
                    self.tree.expr_lists.get(values).len()
                }
                super::constructor_initialization::ConstructorArguments::Single(_) => 1,
                super::constructor_initialization::ConstructorArguments::Value => 0,
            };
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
                for syntax_index in 0..self.tree.expr_lists.get(arguments).len() {
                    let id = self.tree.expr_lists.get(arguments)[syntax_index];
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

    fn class_temporary_may_throw(&mut self, facts: ExprFacts, depth: u16) -> Result<bool, Error> {
        let TypeKind::Class(class) = self.graph.types.get(facts.ty).kind else {
            return Ok(false);
        };
        if facts.category != ValueCategory::Prvalue {
            return Ok(false);
        }
        if let Some(constructor) = facts.constructor {
            let Some(super::expression_facts::ClassInitialization::Constructed(arguments)) =
                facts.class_initialization
            else {
                return Err(Error::Invariant(
                    "constructor throw query without arguments",
                ));
            };
            if self.constructor_may_throw(constructor, arguments, depth)? {
                return Ok(true);
            }
        } else if let Some(super::expression_facts::ClassInitialization::FromExpression(source)) =
            facts.class_initialization
        {
            // This is written class construction whose selected copy
            // lives on its source use, rather than a separate plan on
            // the class prvalue. Its defaults still belong to this call.
            if let Some(super::copy_initialization::CopyAction::Constructor { constructor, .. }) =
                self.graph.expression_facts.copy_action(source)
                && self.constructor_may_throw(
                    constructor,
                    super::constructor_initialization::ConstructorArguments::Single(source),
                    depth,
                )?
            {
                return Ok(true);
            }
        }
        // Calls, conditionals and member projections can produce a class
        // prvalue without a construction plan on that expression. Their
        // selected calls/operands are visited by the syntax walk; the result
        // itself contributes only its destructor (temporary_may_throw).
        self.aggregate_destruction_may_throw(class)
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
            if let Some(selected) = self.graph.expression_facts.function_conversion(id) {
                if self.function_may_throw(self.graph.binding(selected.binding).entity)? {
                    return Ok(true);
                }
                if let Some(copy) = selected.copy {
                    let throws = match (copy.selection.origin, copy.action) {
                        (super::copy_initialization::CopyOrigin::Declared(binding), _) => {
                            self.function_may_throw(self.graph.binding(binding).entity)?
                        }
                        (
                            _,
                            super::copy_initialization::CopyAction::Constructor {
                                constructor, ..
                            },
                        ) => self.constructor_function_may_throw(constructor)?,
                        (_, super::copy_initialization::CopyAction::Storage { .. }) => false,
                    };
                    if throws {
                        return Ok(true);
                    }
                }
                if selected.usage != super::conversion_functions::ConversionUse::Scalar {
                    // kUserConverted is terminal in the frozen may_throw query:
                    // only the converter and original prvalue's destruction
                    // count, not the original expression's construction/children.
                    if facts.category == ValueCategory::Prvalue
                        && let Some(class) = self.graph.class_of(facts.ty)
                        && self.aggregate_destruction_may_throw(class)?
                    {
                        return Ok(true);
                    }
                    continue;
                }
            }
            for index in 0..self.graph.expression_facts.arrow_calls(id).len() {
                let step = self.graph.expression_facts.arrow_calls(id)[index];
                if self.function_may_throw(self.graph.binding(step.binding).entity)? {
                    return Ok(true);
                }
                if step.category == ValueCategory::Prvalue
                    && let TypeKind::Class(class) = self.graph.types.get(step.result).kind
                    && self.aggregate_destruction_may_throw(class)?
                {
                    return Ok(true);
                }
            }
            let kind = self.tree.expressions.get(id).kind;
            if facts.folded.is_some() && !matches!(kind, ExprKind::Member { .. }) {
                continue;
            }
            // A selected value transfer belongs to the original operand,
            // including an unevaluated implicit copy. conversion_may_throw
            // queries only the callable; written construction separately
            // queries its defaults through constructor_may_throw below.
            match self.graph.expression_facts.copy_action(id) {
                Some(super::copy_initialization::CopyAction::Constructor {
                    constructor, ..
                }) => {
                    if self.constructor_function_may_throw(constructor)? {
                        return Ok(true);
                    }
                }
                Some(super::copy_initialization::CopyAction::Storage { .. }) => {
                    if let Some(super::copy_initialization::CopySelection {
                        origin: super::copy_initialization::CopyOrigin::Declared(binding),
                        ..
                    }) = facts.class_copy
                        && self.function_may_throw(self.graph.binding(binding).entity)?
                    {
                        return Ok(true);
                    }
                }
                None => (),
            }
            if let Some((constructor, _)) = self.graph.expression_facts.class_conversion(id) {
                let plan = self.graph.constructors.get(constructor);
                let throws = if let Some(binding) = plan.binding {
                    self.function_may_throw(self.graph.binding(binding).entity)?
                } else {
                    !plan.nothrow
                };
                if throws {
                    return Ok(true);
                }
            }
            if self.class_temporary_may_throw(facts, depth)? {
                return Ok(true);
            }
            if (facts.call_form == super::expression_facts::CallForm::Operator
                || facts.literal_call.is_some())
                && let Some(CallTarget::Direct(binding)) = facts.call
                && self.function_may_throw(self.graph.binding(binding).entity)?
            {
                return Ok(true);
            }
            let next = depth + 1;
            match kind {
                ExprKind::Throw(_) | ExprKind::Statement(_) => return Ok(true),
                ExprKind::Call { arguments, .. } if facts.functional_call => {
                    self.throw_work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                }
                ExprKind::Call { callee, arguments } if !facts.functional_call => {
                    if matches!(
                        facts.destructor,
                        Some(super::expression_facts::DestructorTarget::Scalar)
                    ) || facts.intrinsic.is_some()
                    {
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
                ExprKind::New { .. } => {
                    if self.allocation_may_throw(id, next)? {
                        return Ok(true);
                    }
                }
                ExprKind::Delete { operand, .. } => {
                    if self.deletion_may_throw(id)? {
                        return Ok(true);
                    }
                    self.throw_work.push((operand, next));
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
    fn class_call_query_retains_the_result_without_inventing_a_constructor() {
        for (specification, bits) in [("noexcept", 1), ("noexcept(false)", 0)] {
            let source = SourceBuffer::new(format!(
                "struct X{{int n;~X(){specification};}};X make()noexcept;bool f(){{return noexcept(make());}}"
            ).into_bytes()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"result-unwind").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let expressions = tree.expressions.mark();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let graph = analyzer.analyze(declarations).unwrap();
            let query = return_expression(tree, declarations);
            assert_eq!(
                graph.expression_facts.get(query).unwrap().folded,
                Some(bits)
            );
            let ExprKind::TypeTrait {
                operand: Some(call),
                ..
            } = tree.expressions.get(query).kind
            else {
                panic!("missing query operand");
            };
            let facts = graph.expression_facts.get(call).unwrap();
            assert!(facts.constructor.is_none() && facts.class_initialization.is_none());
            assert!(matches!(facts.call, Some(CallTarget::Direct(_))));
            let TypeKind::Class(class) = graph.types.get(facts.ty).kind else {
                panic!("missing class result");
            };
            assert!(matches!(
                graph.entity(class).layout,
                super::super::layout::LayoutState::NotStarted
            ));
            assert!(super::super::function_demands::trailing(&graph).is_empty());
            assert_eq!(tree.expressions.mark(), expressions);
        }
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
    fn converted_query_distinguishes_contextual_operands_from_scalar_initialization() {
        for (expression, bits) in [("+X()", 1), ("!X()", 1), ("static_cast<int>(X())", 0)] {
            let source = SourceBuffer::new(
                format!("struct X{{X();~X()noexcept;operator int()noexcept;}};bool f(){{return noexcept({expression});}}")
                    .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"converted-query").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let expressions = tree.expressions.mark();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let graph = analyzer.analyze(declarations).unwrap();
            assert_eq!(tree.expressions.mark(), expressions);
            assert!(super::super::function_demands::trailing(&graph).is_empty());
            let query = return_expression(tree, declarations);
            assert_eq!(
                graph.expression_facts.get(query).unwrap().folded,
                Some(bits)
            );
        }
    }

    #[test]
    fn unevaluated_copy_query_keeps_selection_without_emission_or_syntax_growth() {
        let source = SourceBuffer::new(b"struct D{D();D(const D&);};struct E{D d;};extern E e;bool f(){return noexcept(E(e));}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"copy-throw-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), count);
        assert!(super::super::function_demands::trailing(&graph).is_empty());
        let query = return_expression(tree, declarations);
        assert_eq!(graph.expression_facts.get(query).unwrap().folded, Some(0));
        let ExprKind::TypeTrait {
            operand: Some(cast),
            ..
        } = tree.expressions.get(query).kind
        else {
            panic!("query");
        };
        let ExprKind::FunctionalCast { arguments, .. } = tree.expressions.get(cast).kind else {
            panic!("class cast");
        };
        let source = tree.expr_lists.get(arguments)[0];
        let facts = graph.expression_facts.get(source).unwrap();
        assert_eq!(facts.category, ValueCategory::Lvalue);
        assert!(facts.class_copy.is_some());
        assert!(matches!(
            graph.expression_facts.copy_action(source),
            Some(super::super::copy_initialization::CopyAction::Constructor { .. })
        ));
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
        let id = return_expression(analyzer.tree, declarations);
        let ExprKind::TypeTrait {
            operand: Some(operand),
            ..
        } = analyzer.tree.expressions.get(id).kind
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
