// Translated from sema/constant_memo.cpp and constant_memo.h. See NOTICE.
//! One logical evaluation owns scalar-call keys, body costs and purity proofs.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::Address,
    constants::Constant,
    constexpr_calls::Parameter,
    expression_facts::CallForm,
    function_resolution::CallTarget,
    graph::{BindingKind, EntityId, Graph, ScopeKind},
    types::TypeKind,
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    support::{source::SourceOffset, stack::StackBudget},
    syntax::ast::{self, ExprKind, StmtKind, Tree},
};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
pub(super) struct Key {
    pub function: EntityId,
    pub words: Vec<u64>,
}
#[derive(Clone, Copy)]
pub(super) struct ResultValue {
    pub value: Constant,
    pub steps: u64,
    pub depth: usize,
}
#[derive(Clone, Copy)]
enum Purity {
    Checking,
    Eligible,
    Ineligible,
}
#[derive(Default)]
pub(super) struct Memo {
    calls: HashMap<Key, ResultValue>,
    argument_words: usize,
    purity: HashMap<EntityId, Purity>,
}
impl Memo {
    pub(super) fn get(&self, key: &Key) -> Option<ResultValue> {
        self.calls.get(key).copied()
    }
    pub(super) fn remember(&mut self, key: Key, result: ResultValue) -> bool {
        if self.calls.len() >= 4096 || key.words.len() > 65536 - self.argument_words {
            return false;
        }
        self.argument_words += key.words.len();
        self.calls.insert(key, result);
        true
    }
    pub(super) fn eligible(
        &mut self,
        graph: &Graph,
        tree: &Tree,
        function: EntityId,
        stack: (StackBudget, usize),
    ) -> Result<bool, Error> {
        Proof {
            graph,
            tree,
            memo: self,
            stack,
        }
        .eligible(function)
    }
}

impl Analyzer<'_> {
    pub(super) fn constant_call_key(
        &mut self,
        function: EntityId,
        object: Option<Address>,
        bound: &[Parameter],
        location: SourceOffset,
    ) -> Result<Option<Key>, Error> {
        if object.is_some() || !scalar_signature(&self.graph, function) {
            return Ok(None);
        }
        let query = self
            .constant_query
            .as_mut()
            .ok_or(Error::Invariant("call memo outside a logical query"))?;
        let memo = query.memo.get_or_insert_with(Box::default);
        if !memo.eligible(
            &self.graph,
            self.tree,
            function,
            (self.stack_budget, self.stack_base),
        )? {
            return Ok(None);
        }
        let count = bound
            .len()
            .checked_mul(2)
            .ok_or(Error::Limit("call memo argument count"))?;
        u32::try_from(count).map_err(|_| Error::Limit("call memo argument count"))?;
        let mut words = Vec::with_capacity(count);
        for parameter in bound {
            let value = self
                .constexpr_frames
                .bound_integer(parameter.place, location)?;
            words.push(value.bits);
            words.push(value.high);
        }
        Ok(Some(Key { function, words }))
    }
}

pub(super) fn scalar_signature(graph: &Graph, function: EntityId) -> bool {
    let entity = graph.entity(function);
    if !entity.constexpr
        || entity.builtin.is_some()
        || entity.member_function.is_some()
        || entity
            .home
            .is_some_and(|home| graph.scope(home).kind == ScopeKind::Class)
    {
        return false;
    }
    let Some(ty) = entity.ty else {
        return false;
    };
    let TypeKind::Function {
        result,
        parameters,
        variadic,
        ..
    } = &graph.types.get(ty).kind
    else {
        return false;
    };
    !variadic
        && graph.integral_kind(*result).is_some()
        && parameters
            .iter()
            .all(|&ty| graph.integral_kind(ty).is_some())
}

struct Proof<'a> {
    graph: &'a Graph,
    tree: &'a Tree,
    memo: &'a mut Memo,
    stack: (StackBudget, usize),
}
impl Proof<'_> {
    fn check_stack(&self) -> Result<(), Error> {
        let marker = 0_u8;
        if self
            .stack
            .0
            .exceeded(self.stack.1, std::ptr::from_ref(&marker).addr())
        {
            return Err(Error::Limit("constructs nested too deeply"));
        }
        Ok(())
    }
    fn eligible(&mut self, function: EntityId) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(state) = self.memo.purity.get(&function) {
            return Ok(matches!(state, Purity::Eligible));
        }
        self.memo.purity.insert(function, Purity::Checking);
        let pure = self.function_body(function)?;
        self.memo.purity.insert(
            function,
            if pure {
                Purity::Eligible
            } else {
                Purity::Ineligible
            },
        );
        Ok(pure)
    }
    fn function_body(&mut self, function: EntityId) -> Result<bool, Error> {
        if !scalar_signature(self.graph, function) {
            return Ok(false);
        }
        let Some(body) = self.graph.lower_function_body(function) else {
            return Ok(false);
        };
        let ast::Body::Parsed(body) = *self.tree.bodies.get(body) else {
            return Ok(false);
        };
        if body.handlers.is_some() {
            return Ok(false);
        }
        let StmtKind::Compound(items) = self.tree.statements.get(body.compound).kind else {
            return Ok(false);
        };
        let mut returned = None;
        for &statement in self.tree.stmt_lists.get(items) {
            match self.tree.statements.get(statement).kind {
                StmtKind::Expression(None) => {}
                StmtKind::Return(value) if returned.is_none() => returned = value,
                _ => return Ok(false),
            }
        }
        self.expression(returned, function, 0)
    }
    fn expression(
        &mut self,
        expression: Option<ast::ExprId>,
        function: EntityId,
        depth: u32,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let Some(id) = expression.filter(|_| depth <= 200) else {
            return Ok(false);
        };
        let Some(facts) = self.graph.expression_facts.known(id) else {
            return Ok(false);
        };
        if self
            .graph
            .expression_facts
            .function_conversion(id)
            .is_some()
            || self.graph.expression_facts.class_conversion(id).is_some()
            || facts.call_form != CallForm::Ordinary
            || facts.dispatch.is_virtual()
            || facts.literal_call.is_some()
            || self.graph.integral_kind(facts.ty).is_none()
        {
            return Ok(false);
        }
        let child_depth = depth + 1;
        match self.tree.expressions.get(id).kind {
            ExprKind::Literal(_) | ExprKind::Keyword(_) => Ok(true),
            ExprKind::Id(name) => Ok(self.graph.resolved_name(name).is_some_and(|binding| {
                let binding = self.graph.binding(binding);
                binding.kind == BindingKind::Parameter
                    && self.graph.entity(binding.entity).home == self.graph.entity(function).scope
            })),
            ExprKind::Paren(inner) | ExprKind::Cast { operand: inner, .. } => {
                self.expression(Some(inner), function, child_depth)
            }
            ExprKind::Binary {
                left,
                right,
                assignment: false,
                ..
            } => Ok(self.expression(Some(left), function, child_depth)?
                && self.expression(Some(right), function, child_depth)?),
            ExprKind::Unary { op, operand } => {
                Ok(matches!(
                    op.token.as_punctuator(),
                    Some(P::Add | P::Subtract | P::LogicalNot | P::Complement)
                ) && self.expression(Some(operand), function, child_depth)?)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => Ok(self.expression(Some(condition), function, child_depth)?
                && self.expression(Some(then_expr), function, child_depth)?
                && self.expression(Some(else_expr), function, child_depth)?),
            ExprKind::Call {
                mut callee,
                arguments,
            } => {
                while let ExprKind::Paren(inner) = self.tree.expressions.get(callee).kind {
                    callee = inner;
                }
                if !matches!(self.tree.expressions.get(callee).kind, ExprKind::Id(_)) {
                    return Ok(false);
                }
                let Some(CallTarget::Direct(binding)) = facts.call else {
                    return Ok(false);
                };
                let binding = self.graph.binding(binding);
                if binding.kind != BindingKind::Function {
                    return Ok(false);
                }
                let callee = binding.entity;
                let Some(ty) = self.graph.entity(callee).ty else {
                    return Ok(false);
                };
                let TypeKind::Function { parameters, .. } = &self.graph.types.get(ty).kind else {
                    return Ok(false);
                };
                let arguments = self.tree.expr_lists.get(arguments);
                if arguments.len() != parameters.len()
                    || callee != function && !self.eligible(callee)?
                {
                    return Ok(false);
                }
                for &argument in arguments {
                    if !self.expression(Some(argument), function, child_depth)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lex::vocabulary::Keyword,
        preprocess::unit::Preprocessor,
        sema::{lookup::Filter, types::Scalar},
        support::source::SourceBuffer,
        syntax::SyntaxUnit,
    };

    #[test]
    fn saturation_preserves_existing_hits_and_both_argument_words() {
        let mut graph = Graph::new();
        let ty = graph.types.scalar(Scalar::Bool).unwrap();
        let function = graph.add_entity(Some(ty), None, Keyword::Bool).unwrap();
        let result = ResultValue {
            value: Constant {
                bits: 1,
                high: 9,
                ty,
                state: super::super::constant_addresses::ScalarState::Value,
            },
            steps: 17,
            depth: 3,
        };
        let mut memo = Memo::default();
        for high in 0..4096 {
            assert!(memo.remember(
                Key {
                    function,
                    words: vec![1, high]
                },
                result
            ));
        }
        assert!(!memo.remember(
            Key {
                function,
                words: vec![1, 4096]
            },
            result
        ));
        assert_eq!(
            memo.get(&Key {
                function,
                words: vec![1, 9]
            })
            .unwrap()
            .value
            .high,
            9
        );
        assert!(
            memo.get(&Key {
                function,
                words: vec![9, 1]
            })
            .is_none()
        );
        let mut memo = Memo::default();
        assert!(memo.remember(
            Key {
                function,
                words: vec![0; 65536]
            },
            result
        ));
        assert!(!memo.remember(
            Key {
                function,
                words: vec![1]
            },
            result
        ));
        assert!(memo.remember(
            Key {
                function,
                words: Vec::new()
            },
            result
        ));
        assert!(
            memo.get(&Key {
                function,
                words: vec![0; 65536]
            })
            .is_some()
        );
    }

    #[test]
    fn repeated_recursive_calls_reuse_only_within_each_logical_query() {
        let source = SourceBuffer::new(b"constexpr int f(int n){return n<2?n:f(n-1)+f(n-2);}constexpr int a=f(7);constexpr int b=f(7);".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"memo-fibonacci").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        assert!(analyzer.constant_query.is_none());
        #[cfg(feature = "telemetry")]
        {
            assert_eq!(analyzer.graph.work.totals.constant_call_builds, 16);
            assert_eq!(analyzer.graph.work.totals.constant_call_reuses, 10);
        }
    }

    #[test]
    fn completed_keys_use_converted_values_and_keep_high_words() {
        let cases: &[(&[u8], usize, usize, usize, u64)] = &[
            (b"using I=unsigned __int128;constexpr I id(I x){return x;}constexpr bool v=id(I(1)<<100)==id(I(1)<<100)&&id(I(2)<<100)!=id(I(1)<<100);", 2, 2, 45, 1),
            (b"constexpr int f(unsigned char x){return x;}constexpr int v=f(1)+f(257);", 1, 1, 17, 2),
        ];
        for &(source, builds, reuses, steps, bits) in cases {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"memo-key-values").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let name = cursor.intern_name(b"v").unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_semantics(true);
            let marker = 0_u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let scope = analyzer.graph.global;
            analyzer.declarations(declarations, scope).unwrap();
            let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            assert!(matches!(analyzer.graph.entity(entity).constant,
                super::super::constants::ConstantState::Value(value) if value.bits == bits));
            #[cfg(feature = "telemetry")]
            {
                assert_eq!(analyzer.graph.work.totals.constant_call_builds, builds);
                assert_eq!(analyzer.graph.work.totals.constant_call_reuses, reuses);
                assert_eq!(analyzer.graph.work.totals.constant_steps, steps);
            }
            #[cfg(not(feature = "telemetry"))]
            let _ = (builds, reuses, steps);
        }
    }

    #[test]
    fn independent_failures_restore_outer_memo_and_incomplete_queries_keep_old_hits() {
        use super::super::{
            constant_addresses::{ScalarState, TemporaryValue as Value},
            constexpr_calls::ConstantArguments,
        };
        let source =
            SourceBuffer::new(b"constexpr int identity(int n){return n;}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"memo-lifetime").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"identity").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
        let function = analyzer.graph.binding(binding).entity;
        let TypeKind::Function { result: ty, .. } = analyzer
            .graph
            .types
            .get(analyzer.graph.entity(function).ty.unwrap())
            .kind
        else {
            panic!("function type")
        };
        let call = |analyzer: &mut Analyzer<'_>, bits| {
            let argument = Value::Integer(Constant {
                bits,
                high: 0,
                ty,
                state: ScalarState::Value,
            });
            analyzer.invoke_constant_function_into(
                function,
                (ConstantArguments::Values(&[argument]), false),
                (None, None),
                scope,
                SourceOffset(0),
            )
        };
        analyzer
            .with_fresh_constant_query(|analyzer| {
                call(analyzer, 1)?;
                call(analyzer, 1)?;
                let failed: Result<(), Error> = analyzer.with_fresh_constant_query(|analyzer| {
                    call(analyzer, 1)?;
                    Err(Error::Invariant("independent failure"))
                });
                assert!(failed.is_err());
                call(analyzer, 1)?;
                analyzer.note_incomplete_constant_layout();
                call(analyzer, 2)?;
                call(analyzer, 2)?;
                call(analyzer, 1)?;
                Ok(())
            })
            .unwrap();
        analyzer
            .with_fresh_constant_query(|analyzer| call(analyzer, 2))
            .unwrap();
        assert!(analyzer.constant_query.is_none());
        #[cfg(feature = "telemetry")]
        {
            assert_eq!(analyzer.graph.work.totals.constant_call_builds, 3);
            assert_eq!(analyzer.graph.work.totals.constant_call_reuses, 3);
        }
    }

    #[test]
    fn purity_checks_original_syntax_all_branches_and_dependency_cycles() {
        let source = SourceBuffer::new(
            br"
            constexpr int leaf(int n){return n+1;}
            constexpr int pure(int n){;return (n?leaf(n):pure(n+1));;}
            constexpr int global=1;
            constexpr int read(int n){return n?global:0;}
            constexpr int size(int n){return n?sizeof(n):0;}
            constexpr int comma(int n){return (0,leaf)(n);}
            constexpr int defaulted(int n,int v=1){return n+v;}
            constexpr int implicit(int n){return defaulted(n);}
            constexpr int odd(int);
            constexpr int even(int n){return n?odd(n-1):1;}
            constexpr int odd(int n){return n?even(n-1):0;}
            struct S{static constexpr int method(int n){return n;}};
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"memo-purity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            b"leaf".as_slice(),
            b"pure",
            b"read",
            b"size",
            b"comma",
            b"implicit",
            b"even",
            b"odd",
        ]
        .map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let mut memo = Memo::default();
        for (index, name) in names.into_iter().enumerate() {
            let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
            let function = analyzer.graph.binding(binding).entity;
            assert_eq!(
                memo.eligible(
                    &analyzer.graph,
                    analyzer.tree,
                    function,
                    (analyzer.stack_budget, analyzer.stack_base)
                )
                .unwrap(),
                index < 2
            );
        }
    }
}
