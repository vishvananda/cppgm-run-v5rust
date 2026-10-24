// Translated from sema/analyzer_traits.cpp::trait_assignable/trait_convertible. See NOTICE.
//! A query owns implicit declval operands and reuses ordinary expression facts. Its
//! result is memoized per syntax use, environment and canonical operand types;
//! unrelated uses never share access or declaration-visibility decisions.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{CallForm, ExpressionOrigin, ValueCategory},
    function_resolution::CallTarget,
    graph::ScopeId,
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator, predicates::TypePredicate as T, vocabulary::SimpleToken},
    support::source::SourceOffset,
    syntax::ast,
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    expression: ast::ExprId,
    scope: ScopeId,
    first: TypeId,
    second: TypeId,
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
    fn expression_queries_reuse_facts_and_restore_context_without_emitting_members() {
        let source = SourceBuffer::new(
            b"struct Plain{int n;};struct Const{const int n;};\
              struct Hidden{private:Hidden&operator=(int)noexcept;};\
              struct Converted{operator int()noexcept;};\
              struct Assign{template<class T>Assign&operator=(T)noexcept{T::missing();}};\
              static_assert(__is_assignable(Assign&,int),\"signature only\");\
              static_assert(__is_nothrow_assignable(Plain&,const Plain&),\"implicit\");\
              static_assert(__is_trivially_assignable(Plain&,const Plain&),\"trivial\");\
              static_assert(!__is_assignable(Const&,Const),\"deleted\");\
              static_assert(!__is_assignable(Hidden&,int),\"private\");\
              static_assert(!__is_assignable(const int&,int),\"const\");\
              static_assert(__is_nothrow_assignable(int&,Converted),\"conversion\");\
              static_assert(!__is_trivially_assignable(int&,Converted),\"conversion\");\
              struct To{To(int)noexcept;};\
              struct Bad{Bad(int)noexcept;private:~Bad();};\
              struct Loud{Loud(int)noexcept;~Loud()noexcept(false);};\
              static_assert(__is_nothrow_convertible(int,To),\"constructor\");\
              static_assert(!__is_convertible(int,Bad),\"destructor access\");\
              static_assert(!__is_nothrow_convertible(int,Loud),\"destructor exception\");\
              static_assert(__is_convertible_to(Converted,int),\"converter\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"assignment-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let expressions = analyzer.tree.expressions.mark();
        let keys: Vec<_> = analyzer
            .graph
            .expression_predicates
            .expressions
            .keys()
            .copied()
            .collect();
        assert_eq!(keys.len(), 12);
        for _ in 0..3 {
            for &key in &keys {
                let ast::ExprKind::TypePredicate { predicate, types } =
                    analyzer.tree.expressions.get(key.expression).kind
                else {
                    panic!("retained query");
                };
                analyzer
                    .predicate_constant(
                        key.expression,
                        predicate,
                        types,
                        key.scope,
                        SourceOffset(0),
                    )
                    .unwrap();
            }
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(
            analyzer.graph.expression_predicates.expressions.len(),
            keys.len()
        );
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.body_context.is_none());
        assert_eq!(analyzer.unevaluated, 0);
        assert!(analyzer.operator_lookup.bindings.is_empty());
        assert!(analyzer.operator_scratch.candidates.is_empty());
        assert!(!analyzer.call_scratch.has_candidates());
    }

    #[test]
    fn incomplete_source_queries_and_in_definition_conversion_sets_are_not_frozen() {
        let source = SourceBuffer::new(
            b"struct Forward;static_assert(!__is_convertible(Forward,int),\"forward\");\
              static_assert(!__is_assignable(Forward&,Forward),\"assignment\");\
              struct Open{static const bool early=__is_convertible(Open&,int);\
                operator int()noexcept;static auto read(Open&value)->decltype(value.operator int());static const bool late=__is_convertible(Open&,int);};\
              static_assert(!Open::early,\"before declaration\");\
              static_assert(!Open::late,\"still incomplete\");\
              static_assert(__is_nothrow_convertible(Open&,int),\"complete class\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"open-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(analyzer.unevaluated, 0);
        assert!(analyzer.body_context.is_none());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn hard_initialization_failure_restores_the_query_and_unevaluated_context() {
        let source = SourceBuffer::new(
            b"struct Incomplete;bool query=__is_convertible(Incomplete,Incomplete);".to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"hard-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        assert!(matches!(
            analyzer.declarations(declarations, global),
            Err(Error::UnconditionalDiagnostic {
                message: "initialization of an incomplete class object",
                ..
            })
        ));
        assert_eq!(analyzer.unevaluated, 0);
        assert!(analyzer.graph.expression_predicates.expressions.is_empty());
        assert!(analyzer.body_context.is_none());
        assert!(analyzer.initializing.is_none());
    }
}
#[derive(Clone, Copy, Default)]
struct Properties {
    viable: bool,
    nothrow: bool,
    trivial: bool,
}
#[derive(Clone, Copy)]
enum Answer {
    Active,
    Ready(Properties),
}
#[derive(Default)]
pub(super) struct Queries {
    // The syntax use fixes its source point and access context; scope fixes
    // the instantiated bindings. Query results do not depend on later body
    // or emission demand. Hard failures remove their active entries.
    expressions: HashMap<Key, Answer>,
}

impl Analyzer<'_> {
    pub(super) fn expression_predicate(
        &mut self,
        id: ast::ExprId,
        predicate: T,
        first: TypeId,
        second: TypeId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let location = self.tree.expressions.get(id).location;
        let key = Key {
            expression: id,
            scope,
            first,
            second,
        };
        let properties = match self.graph.expression_predicates.expressions.get(&key) {
            Some(Answer::Ready(properties)) => *properties,
            Some(Answer::Active) => return Err(Error::Invariant("cyclic expression predicate")),
            None => {
                self.graph
                    .expression_predicates
                    .expressions
                    .insert(key, Answer::Active);
                self.unevaluated += 1;
                let result = if predicate.is_assignment() {
                    self.assignment_query(first, second, location, scope)
                } else {
                    self.conversion_query(first, second, location, scope)
                };
                self.unevaluated -= 1;
                match result {
                    Ok(properties) => {
                        self.graph
                            .expression_predicates
                            .expressions
                            .insert(key, Answer::Ready(properties));
                        properties
                    }
                    Err(error) => {
                        self.graph.expression_predicates.expressions.remove(&key);
                        return Err(error);
                    }
                }
            }
        };
        Ok(properties.viable
            && match predicate {
                T::IsNothrowAssignable | T::IsNothrowConvertible => properties.nothrow,
                T::IsTriviallyAssignable => properties.trivial,
                _ => true,
            })
    }

    fn conversion_query(
        &mut self,
        from: TypeId,
        to: TypeId,
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        let from_kind = &self.graph.types.get(from).kind;
        let to_kind = &self.graph.types.get(to).kind;
        let from_void = matches!(from_kind, TypeKind::Scalar(Scalar::Void));
        let to_void = matches!(to_kind, TypeKind::Scalar(Scalar::Void));
        if from_void || to_void {
            return Ok(Properties {
                viable: from_void && to_void,
                nothrow: true,
                trivial: false,
            });
        }
        if matches!(to_kind, TypeKind::Array { .. } | TypeKind::Function { .. })
            || matches!(from_kind, TypeKind::Function { cv, reference, .. } if cv.0 != 0 || reference.is_some())
        {
            return Ok(Properties::default());
        }
        if let Some(class) = self.graph.class_of(to)
            && self.complete_class(class)?
            && self.graph.virtuals.abstract_class(class)
        {
            return Ok(Properties::default());
        }
        let operand = self.trait_declval(from, location, scope)?;
        let initializer = ast::Initializer {
            location,
            kind: ast::InitializerKind::Assign(operand),
        };
        let selection = (|| {
            // new_facts completes class-template declval operands in C++.
            // Class initialization can reuse their known facts, so perform
            // that demand here before asking for conversion functions.
            self.typed_expression(operand, scope)?;
            let constructor = if self.graph.class_of(to).is_some() {
                self.initialize_class_object(to, initializer, scope, location)?
                    .map(|(constructor, _)| constructor)
            } else {
                self.object_initializer(initializer, scope, to, location)?;
                None
            };
            self.check_class_destruction(to, scope, location)?;
            Ok(constructor)
        })();
        let constructor = match selection {
            Ok(constructor) => constructor,
            Err(Error::Diagnostic { .. }) => return Ok(Properties::default()),
            Err(error) => return Err(error),
        };
        let mut nothrow = !self.expression_may_throw(operand)?;
        if let Some(constructor) = constructor {
            nothrow &= !self.constructor_function_may_throw(constructor)?;
        }
        if let Some(class) = self.graph.class_of(to) {
            nothrow &= !self.aggregate_destruction_may_throw(class)?;
        }
        Ok(Properties {
            viable: true,
            nothrow,
            trivial: false,
        })
    }

    fn assignment_query(
        &mut self,
        target: TypeId,
        source: TypeId,
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Void) | TypeKind::Function { .. }
        ) || matches!(
            self.graph.types.get(source).kind,
            TypeKind::Scalar(Scalar::Void)
        ) || matches!(self.graph.types.get(source).kind, TypeKind::Function { cv, reference, .. } if cv.0 != 0 || reference.is_some())
        {
            return Ok(Properties::default());
        }
        let left = self.trait_declval(target, location, scope)?;
        let right = self.trait_declval(source, location, scope)?;
        let assignment = self
            .tree
            .expressions
            .add(ast::Expression {
                location,
                kind: ast::ExprKind::Binary {
                    op: ast::Operator {
                        token: SimpleToken::punctuator(Punctuator::Assign)
                            .ok_or(Error::Invariant("assignment token"))?,
                        alternative: false,
                    },
                    left,
                    right,
                    assignment: true,
                },
            })
            .map_err(|_| Error::Limit("assignment query syntax limit"))?;
        let facts = match self.typed_expression(assignment, scope) {
            Ok(facts) => facts,
            Err(Error::Diagnostic { .. }) => return Ok(Properties::default()),
            Err(error) => return Err(error),
        };
        let mut trivial = if let Some(CallTarget::Direct(binding)) = facts.call {
            self.graph
                .entity(self.graph.binding(binding).entity)
                .assignment
                .as_ref()
                .is_some_and(|facts| facts.trivial)
        } else {
            facts.call_form != CallForm::Operator
        };
        // A user conversion on the source is a nontrivial operation even if
        // assignment itself selects a trivial member or the builtin operator.
        trivial &= self
            .graph
            .expression_facts
            .function_conversion(right)
            .is_none()
            && self
                .graph
                .expression_facts
                .class_conversion(right)
                .is_none();
        Ok(Properties {
            viable: true,
            nothrow: !self.expression_may_throw(assignment)?,
            trivial,
        })
    }

    fn trait_declval(
        &mut self,
        ty: TypeId,
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<ast::ExprId, Error> {
        let category = match self.graph.types.get(ty).kind {
            TypeKind::Reference {
                kind: RefKind::Lvalue,
                ..
            }
            | TypeKind::Function { .. } => ValueCategory::Lvalue,
            TypeKind::Reference { element, .. }
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) =>
            {
                ValueCategory::Lvalue
            }
            _ => ValueCategory::Xvalue,
        };
        let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        let label = self
            .identifiers
            .intern(b"<trait declval>")
            .map_err(|_| Error::Limit("assignment query name limit"))?;
        let name = self
            .tree
            .implicit_name(label, location)
            .map_err(|_| Error::Limit("assignment query name limit"))?;
        let id = self
            .tree
            .expressions
            .add(ast::Expression {
                location,
                kind: ast::ExprKind::Id(name),
            })
            .map_err(|_| Error::Limit("assignment query syntax limit"))?;
        let mut facts = self.new_expression_facts(ty, category)?;
        facts.origin = ExpressionOrigin::Implicit;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), ty);
        super::record(&mut self.graph.expression_scopes, id.index(), scope);
        Ok(id)
    }
}
