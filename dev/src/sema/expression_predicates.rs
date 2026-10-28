// Translated from sema/analyzer_traits.cpp's expression-viability queries. See NOTICE.
//! A query owns implicit declval operands and reuses ordinary expression facts. Its
//! result is memoized per syntax use, environment and canonical operand types;
//! unrelated uses never share access or declaration-visibility decisions.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{CallForm, ExpressionOrigin, ValueCategory},
    function_resolution::CallTarget,
    graph::ScopeId,
    predicates::PredicateTypes,
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator, predicates::TypePredicate as T, vocabulary::SimpleToken},
    support::source::SourceOffset,
    syntax::ast,
};
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    expression: ast::ExprId,
    scope: ScopeId,
    types: PredicateTypes,
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
            .cloned()
            .collect();
        assert_eq!(keys.len(), 12);
        for _ in 0..3 {
            for key in &keys {
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

    #[test]
    fn invocation_queries_preserve_demand_and_cache_without_emitting_call_bodies() {
        let source = SourceBuffer::new(
            br#"
            struct C { int field; int call(int) noexcept; };
            struct Wrapper { C& operator*() const noexcept; };
            struct F { int operator()(int) const noexcept; };
            struct Hidden { private: int operator()(int) const; };
            struct Dormant { template<class T> int operator()(T) const noexcept { T::missing(); } };
            struct Incomplete;
            typedef int (*Fn)(int);
            static_assert(__is_invocable(Fn,int), "function");
            static_assert(__is_nothrow_invocable(F,int), "functor");
            static_assert(__is_nothrow_invocable(Dormant,int), "signature only");
            static_assert(__is_invocable(int C::*,C&), "member");
            static_assert(__is_nothrow_invocable(int C::*,Wrapper), "indirection");
            static_assert(!__is_invocable(Hidden,int), "access");
            static_assert(!__is_invocable(int C::*,Incomplete,int), "arity before object demand");
            static_assert(!__is_invocable(void,int), "void callable");
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"invocation-query").unwrap());
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
            .cloned()
            .collect();
        assert!(!keys.is_empty());
        for key in &keys {
            let ast::ExprKind::TypePredicate { predicate, types } =
                analyzer.tree.expressions.get(key.expression).kind
            else {
                panic!("retained query");
            };
            analyzer
                .predicate_constant(key.expression, predicate, types, key.scope, SourceOffset(0))
                .unwrap();
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.graph.expression_predicates.operands.is_empty());
        assert!(
            analyzer
                .graph
                .expression_predicates
                .operand_order
                .is_empty()
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.body_context.is_none());
        assert!(analyzer.pretyped_callable.is_none());
        assert!(analyzer.pretyped_object.is_none());
        assert_eq!(analyzer.unevaluated, 0);
    }

    #[test]
    fn construction_queries_retain_expanded_types_and_reuse_undemanded_selections() {
        let source = SourceBuffer::new(
            br#"
            struct Dormant {
                template<class T> explicit Dormant(T) noexcept { T::missing(); }
            };
            struct Many { Many(int,long,double,char,short) noexcept; };
            struct Hidden { private: ~Hidden(); };
            struct ThrowCopy { ThrowCopy(const ThrowCopy&) noexcept(false); };
            struct Take { Take(ThrowCopy) noexcept; };
            template<class T,class...A>struct Query {
                static const bool value=__is_nothrow_constructible(T,A...);
            };
            static_assert(Query<Dormant,int>::value,"signature only");
            static_assert(Query<Many,int,long,double,char,short>::value,"expanded pack");
            static_assert(__is_trivially_constructible(int[4]),"bounded array");
            static_assert(!__is_constructible(Hidden),"destructor access");
            static_assert(__is_constructible(Hidden&,Hidden&),"no destruction");
            static_assert(__reference_constructs_from_temporary(const int&,double),"temporary");
            static_assert(!__reference_converts_from_temporary(int&,int&),"direct binding");
            static_assert(!__is_nothrow_constructible(Take,ThrowCopy&),"parameter copy");
            "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"construction-query").unwrap());
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
            .cloned()
            .collect();
        assert!(keys.iter().any(|key| key.types.as_slice().len() == 6));
        for key in &keys {
            let ast::ExprKind::TypePredicate { predicate, types } =
                analyzer.tree.expressions.get(key.expression).kind
            else {
                panic!("retained query");
            };
            analyzer
                .predicate_constant(key.expression, predicate, types, key.scope, SourceOffset(0))
                .unwrap();
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert_eq!(
            analyzer.graph.expression_predicates.expressions.len(),
            keys.len()
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.body_context.is_none());
        assert!(analyzer.initializing.is_none());
        assert_eq!(analyzer.unevaluated, 0);
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
    operands: HashMap<ast::ExprId, (TypeId, ValueCategory)>,
    operand_order: Vec<ast::ExprId>,
}
impl Queries {
    fn finish_operands(&mut self, mark: usize) {
        for id in self.operand_order.drain(mark..) {
            self.operands.remove(&id);
        }
    }
}

impl Analyzer<'_> {
    fn invocation_query(
        &mut self,
        types: &[TypeId],
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        for (index, &ty) in types.iter().enumerate() {
            if matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            ) || index != 0 && matches!(self.graph.types.get(ty).kind, TypeKind::Function { .. })
            {
                return Ok(Properties::default());
            }
        }
        let first = types[0];
        if matches!(self.graph.types.get(first).kind, TypeKind::Function { cv, reference, .. } if cv.0 != 0 || reference.is_some())
        {
            return Ok(Properties::default());
        }
        if let TypeKind::Class(class) = self.graph.types.get(first).kind
            && !self.complete_class(class)?
        {
            return Err(Error::at(
                location,
                "a type trait of an incomplete class type",
            ));
        }
        let mark = self.graph.expression_predicates.operand_order.len();
        let result = self.invocation_call_query(types, location, scope);
        self.graph.expression_predicates.finish_operands(mark);
        result
    }

    fn invocation_call_query(
        &mut self,
        types: &[TypeId],
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        let start = self.tree.expr_lists.begin();
        for &ty in types {
            let operand = self.defer_trait_declval(ty, location)?;
            self.tree.expr_lists.push(operand);
        }
        let arguments = self
            .tree
            .expr_lists
            .finish(start)
            .map_err(|_| Error::Limit("invocation query argument limit"))?;
        let callable = self.tree.expr_lists.get(arguments)[0];
        let call = self
            .tree
            .expressions
            .add(ast::Expression {
                location,
                // builtin_invoke replaces this placeholder callee before typing it.
                kind: ast::ExprKind::Call {
                    callee: callable,
                    arguments,
                },
            })
            .map_err(|_| Error::Limit("invocation query syntax limit"))?;
        // Call facts own temporary/throwing information by ExprId, so this
        // unevaluated query needs no synthetic function or body identity.
        let facts = match self.substitution_trial(|analyzer| {
            analyzer.trait_expression_context(|analyzer| {
                analyzer.builtin_invoke(call, arguments, scope)
            })
        }) {
            Ok(Some(facts)) => facts,
            Ok(None) => return Ok(Properties::default()),
            Err(Error::Diagnostic { .. }) => {
                self.graph.work.totals.substitution_failures += 1;
                return Ok(Properties::default());
            }
            Err(error) => return Err(error),
        };
        self.graph.expression_facts.set(call, facts);
        super::record(&mut self.graph.expression_types, call.index(), facts.ty);
        super::record(&mut self.graph.expression_scopes, call.index(), scope);
        Ok(Properties {
            viable: true,
            nothrow: !self.expression_may_throw(call)?,
            trivial: false,
        })
    }

    fn type_construction_operands(
        &mut self,
        operands: crate::syntax::storage::List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        for index in 0..self.tree.expr_lists.get(operands).len() {
            self.typed_expression(self.tree.expr_lists.get(operands)[index], scope)?;
            if self.graph.analysis.substitution.current().failure.is_some() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn construction_query(
        &mut self,
        target: TypeId,
        arguments: &[TypeId],
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        let mark = self.graph.expression_predicates.operand_order.len();
        let result = self.construction_query_in(target, arguments, location, scope);
        self.graph.expression_predicates.finish_operands(mark);
        result
    }
    fn construction_query_in(
        &mut self,
        mut target: TypeId,
        arguments: &[TypeId],
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        for &argument in arguments {
            if matches!(
                self.graph.types.get(argument).kind,
                TypeKind::Scalar(Scalar::Void)
            ) || matches!(self.graph.types.get(argument).kind, TypeKind::Function { cv, reference, .. } if cv.0 != 0 || reference.is_some())
            {
                return Ok(Properties::default());
            }
        }
        // Known-bound arrays have only value initialization here. Peel all
        // extents as the frozen recursive query does, without allocating elements.
        if let TypeKind::Array { bound, .. } = self.graph.types.get(target).kind {
            if bound == 0 || !arguments.is_empty() {
                return Ok(Properties::default());
            }
            while let TypeKind::Array { element, .. } = self.graph.types.get(target).kind {
                target = element;
            }
        }
        let reference = matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference { .. }
        );
        let class = self.graph.class_of(target);
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Void) | TypeKind::Function { .. }
        ) || reference && arguments.len() != 1
            || !reference && class.is_none() && arguments.len() > 1
        {
            return Ok(Properties::default());
        }
        if let Some(class) = class
            && (!self.complete_class(class)?
                || self.graph.entity(class).scope.is_none()
                || self.graph.virtuals.abstract_class(class))
        {
            return Ok(Properties::default());
        }
        let start = self.tree.expr_lists.begin();
        for &argument in arguments {
            let operand = self.defer_trait_declval(argument, location)?;
            self.tree.expr_lists.push(operand);
        }
        let operands = self
            .tree
            .expr_lists
            .finish(start)
            .map_err(|_| Error::Limit("construction query argument limit"))?;
        let initializer = ast::Initializer {
            location,
            kind: ast::InitializerKind::Parenthesized(operands),
        };
        let selection = self.substitution_trial(|analyzer| {
            analyzer.trait_expression_context(|analyzer| {
                let constructor = if class.is_some() {
                    if !analyzer.type_construction_operands(operands, scope)? {
                        return Ok((None, None));
                    }
                    analyzer
                        .initialize_class_object(target, initializer, scope, location)?
                        .map(|(constructor, _)| constructor)
                } else {
                    analyzer.object_initializer(initializer, scope, target, location)?;
                    None
                };
                let destroyed = if reference {
                    analyzer
                        .graph
                        .expression_facts
                        .known(analyzer.tree.expr_lists.get(operands)[0])
                        .and_then(|facts| facts.conversion)
                        .filter(|binding| {
                            binding.temporary
                                && matches!(
                                    analyzer.graph.types.get(binding.target).kind,
                                    TypeKind::Reference { .. }
                                )
                        })
                        .map(|binding| binding.result)
                } else {
                    Some(target)
                };
                if let Some(destroyed) = destroyed
                    && analyzer
                        .graph
                        .analysis
                        .substitution
                        .current()
                        .failure
                        .is_none()
                {
                    analyzer.check_class_destruction(destroyed, scope, location)?;
                }
                Ok((constructor, destroyed))
            })
        });
        let (constructor, destroyed) = match selection {
            Ok(Some(selection)) => selection,
            Ok(None) => return Ok(Properties::default()),
            Err(Error::Diagnostic { .. }) => {
                self.graph.work.totals.substitution_failures += 1;
                return Ok(Properties::default());
            }
            Err(error) => return Err(error),
        };
        self.construction_properties(target, operands, constructor, destroyed)
    }

    fn construction_properties(
        &mut self,
        target: TypeId,
        operands: crate::syntax::storage::List<ast::ExprId>,
        constructor: Option<super::construction::ConstructorId>,
        destroyed: Option<TypeId>,
    ) -> Result<Properties, Error> {
        let class = self.graph.class_of(target);
        let count = self.tree.expr_lists.get(operands).len();
        let mut properties = Properties {
            viable: true,
            nothrow: true,
            trivial: true,
        };
        if let Some(constructor) = constructor {
            properties.nothrow = !self.constructor_function_may_throw(constructor)?;
            properties.trivial = !self.graph.constructors.get(constructor).nontrivial();
            if let Some(binding) = self.graph.constructors.get(constructor).binding {
                let function = self.graph.binding(binding).entity;
                for index in count..self.graph.entity(function).defaults.len() {
                    if properties.nothrow
                        && let Some(default) = self.graph.entity(function).defaults[index]
                    {
                        properties.nothrow = !self.expression_may_throw(default.expression)?;
                    }
                }
            }
        } else if let Some(class) = class
            && let [operand] = self.tree.expr_lists.get(operands)
        {
            let facts = self.graph.expression_facts.get(*operand)?;
            if let Some(copy) = facts.class_copy {
                let facts = self.selected_copy_properties(class, copy)?;
                properties.nothrow = facts.nothrow;
                properties.trivial = facts.trivial;
            }
        }
        if let Some(destroyed) = destroyed
            && let Some(class) = self.graph.class_of(destroyed)
        {
            properties.nothrow &= self.available_destructor_nothrow(class)?;
        }
        if let Some(class) = class {
            if self.graph.has_nontrivial_destructor(class) {
                properties.trivial = false;
            } else if count == 0 {
                properties.trivial = self.property_default_trivial(target)?;
            }
        }
        // The frozen query reads each operand's outer user conversion. Standard
        // arithmetic and reference sequences stay trivial; user calls do not.
        for index in 0..self.tree.expr_lists.get(operands).len() {
            let operand = self.tree.expr_lists.get(operands)[index];
            if let Some(nothrow) = self.construction_conversion_nothrow(
                operand,
                constructor.is_some(),
                properties.nothrow,
            )? {
                properties.trivial = false;
                properties.nothrow &= nothrow;
            }
        }
        Ok(properties)
    }

    fn construction_conversion_nothrow(
        &mut self,
        operand: ast::ExprId,
        parameter: bool,
        query_nothrow: bool,
    ) -> Result<Option<bool>, Error> {
        if let Some(conversion) = self.graph.expression_facts.function_conversion(operand) {
            if !query_nothrow {
                return Ok(Some(true));
            }
            return self
                .function_may_throw(self.graph.binding(conversion.binding).entity)
                .map(|throws| Some(!throws));
        }
        if let Some((constructor, _)) = self.graph.expression_facts.class_conversion(operand) {
            if !query_nothrow {
                return Ok(Some(true));
            }
            return self
                .constructor_function_may_throw(constructor)
                .map(|throws| Some(!throws));
        }
        // initialize_object returns a same-class copy as the definition's
        // constructor. A copy into a different constructor's by-value parameter
        // instead belongs to that operand's outer conversion.
        if parameter && let Some(copy) = self.graph.expression_facts.get(operand)?.class_copy {
            if !query_nothrow {
                return Ok(Some(true));
            }
            let class = self
                .graph
                .class_of(copy.binding.result)
                .ok_or(Error::Invariant("copy conversion without class"))?;
            return self
                .selected_copy_properties(class, copy)
                .map(|facts| Some(facts.nothrow));
        }
        Ok(None)
    }

    fn temporary_binding_query(
        &mut self,
        predicate: T,
        target: TypeId,
        mut source: TypeId,
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        if !matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference { .. }
        ) || matches!(
            self.graph.types.get(source).kind,
            TypeKind::Scalar(Scalar::Void) | TypeKind::Function { .. }
        ) {
            return Ok(Properties::default());
        }
        let prvalue = !matches!(
            self.graph.types.get(source).kind,
            TypeKind::Reference { .. }
        );
        if prvalue
            && !matches!(
                self.graph.types.get(source).kind,
                TypeKind::Array { .. } | TypeKind::Class(_)
            )
        {
            source = self.bare_type(source)?;
        }
        let operand = self.trait_operand(source, location, scope, prvalue)?;
        let kind = if predicate == T::ReferenceConvertsFromTemporary {
            ast::InitializerKind::Assign(operand)
        } else {
            let start = self.tree.expr_lists.begin();
            self.tree.expr_lists.push(operand);
            ast::InitializerKind::Parenthesized(
                self.tree
                    .expr_lists
                    .finish(start)
                    .map_err(|_| Error::Limit("temporary query argument limit"))?,
            )
        };
        match self.substitution_trial(|analyzer| {
            analyzer.trait_expression_context(|analyzer| {
                analyzer.object_initializer(
                    ast::Initializer { location, kind },
                    scope,
                    target,
                    location,
                )
            })
        }) {
            Ok(Some(_)) => (),
            Ok(None) => return Ok(Properties::default()),
            Err(Error::Diagnostic { .. }) => {
                self.graph.work.totals.substitution_failures += 1;
                return Ok(Properties::default());
            }
            Err(error) => return Err(error),
        }
        let binding = self.graph.expression_facts.get(operand)?.conversion;
        let direct_prvalue = prvalue
            && matches!(
                self.graph.types.get(source).kind,
                TypeKind::Array { .. } | TypeKind::Class(_)
            );
        Ok(Properties {
            viable: binding.is_some_and(|binding| {
                binding.temporary
                    || direct_prvalue
                        && !matches!(
                            self.graph.types.get(binding.result).kind,
                            TypeKind::Function { .. }
                        )
            }),
            ..Properties::default()
        })
    }

    pub(super) fn expression_predicate(
        &mut self,
        id: ast::ExprId,
        predicate: T,
        types: &PredicateTypes,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let location = self.tree.expressions.get(id).location;
        let key = Key {
            expression: id,
            scope,
            types: types.clone(),
        };
        let properties = match self.graph.expression_predicates.expressions.get(&key) {
            Some(Answer::Ready(properties)) => *properties,
            Some(Answer::Active) => return Err(Error::Invariant("cyclic expression predicate")),
            None => {
                self.graph
                    .expression_predicates
                    .expressions
                    .insert(key.clone(), Answer::Active);
                let operands = types.as_slice();
                let first = operands[0];
                let result = (|| {
                    if predicate.is_invocation() {
                        self.invocation_query(operands, location, scope)
                    } else if predicate.is_construction() {
                        self.construction_query(first, &operands[1..], location, scope)
                    } else {
                        let second = operands
                            .get(1)
                            .copied()
                            .ok_or(Error::Invariant("binary predicate without second type"))?;
                        if predicate.is_temporary_binding() {
                            self.temporary_binding_query(predicate, first, second, location, scope)
                        } else if predicate.is_assignment() {
                            self.assignment_query(first, second, location, scope)
                        } else {
                            self.conversion_query(first, second, location, scope)
                        }
                    }
                })();
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
                T::IsNothrowAssignable
                | T::IsNothrowConvertible
                | T::IsNothrowConstructible
                | T::IsNothrowInvocable => properties.nothrow,
                T::IsTriviallyAssignable | T::IsTriviallyConstructible => properties.trivial,
                _ => true,
            })
    }

    fn trait_expression_context<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let inserted_body = self.body_context.is_none();
        if inserted_body {
            let result = self.graph.types.scalar(Scalar::Void)?;
            self.body_context = Some(super::procedural_statements::BodyContext::new(result, None));
        }
        self.unevaluated += 1;
        let result = action(self);
        self.unevaluated -= 1;
        if inserted_body {
            self.body_context = None;
        }
        result
    }
    fn conversion_query(
        &mut self,
        from: TypeId,
        to: TypeId,
        location: SourceOffset,
        scope: ScopeId,
    ) -> Result<Properties, Error> {
        let mark = self.graph.expression_predicates.operand_order.len();
        let result = self.conversion_query_in(from, to, location, scope);
        self.graph.expression_predicates.finish_operands(mark);
        result
    }
    fn conversion_query_in(
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
        let operand = self.defer_trait_declval(from, location)?;
        let initializer = ast::Initializer {
            location,
            kind: ast::InitializerKind::Assign(operand),
        };
        let owner = self.graph.analysis.substitution.current().owner;
        let selection = self.with_substitution_soft(owner, |analyzer| {
            let (constructor, mut nothrow) = analyzer.trait_expression_context(|analyzer| {
                // The source initializes first, then checks destruction only
                // without failure, but asks an existing destructor's noexcept
                // even when initialization left a soft failure.
                let initialization = (|| {
                    if analyzer.graph.class_of(to).is_some() {
                        analyzer
                            .initialize_class_object(to, initializer, scope, location)
                            .map(|selected| selected.map(|(constructor, _)| constructor))
                    } else {
                        analyzer.object_initializer(initializer, scope, to, location)?;
                        Ok(None)
                    }
                })();
                let constructor = match initialization {
                    Ok(constructor) => constructor,
                    Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => None,
                    Err(error) => return Err(error),
                };
                if analyzer
                    .graph
                    .analysis
                    .substitution
                    .current()
                    .failure
                    .is_none()
                {
                    match analyzer.check_class_destruction(to, scope, location) {
                        Ok(()) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => {}
                        Err(error) => return Err(error),
                    }
                }
                let nothrow = if let Some(class) = analyzer.graph.class_of(to) {
                    analyzer.available_destructor_nothrow(class)?
                } else {
                    true
                };
                Ok((constructor, nothrow))
            })?;
            if analyzer
                .graph
                .analysis
                .substitution
                .current()
                .failure
                .is_some()
            {
                return Ok(None);
            }
            if nothrow && let Some(constructor) = constructor {
                nothrow = !analyzer.constructor_function_may_throw(constructor)?;
            }
            Ok(Some(nothrow))
        });
        let nothrow = match selection {
            Ok(Some(nothrow)) => nothrow,
            Ok(None) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => {
                return Ok(Properties::default());
            }
            Err(Error::Diagnostic { .. }) => {
                self.graph.work.totals.substitution_failures += 1;
                return Ok(Properties::default());
            }
            Err(error) => return Err(error),
        };
        Ok(Properties {
            viable: true,
            nothrow: nothrow && !self.expression_may_throw(operand)?,
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
        let facts = match self.substitution_trial(|analyzer| {
            analyzer
                .trait_expression_context(|analyzer| analyzer.typed_expression(assignment, scope))
        }) {
            Ok(Some(facts)) => facts,
            Ok(None) => return Ok(Properties::default()),
            Err(Error::Diagnostic { .. }) => {
                self.graph.work.totals.substitution_failures += 1;
                return Ok(Properties::default());
            }
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
        self.trait_operand(ty, location, scope, false)
    }

    fn trait_operand_shape(&self, ty: TypeId, prvalue: bool) -> (TypeId, ValueCategory) {
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
            _ if prvalue => ValueCategory::Prvalue,
            _ => ValueCategory::Xvalue,
        };
        let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        (ty, category)
    }

    fn trait_operand_syntax(&mut self, location: SourceOffset) -> Result<ast::ExprId, Error> {
        let label = self
            .identifiers
            .intern(b"<trait declval>")
            .map_err(|_| Error::Limit("assignment query name limit"))?;
        let name = self
            .tree
            .implicit_name(label, location)
            .map_err(|_| Error::Limit("assignment query name limit"))?;
        self.tree
            .expressions
            .add(ast::Expression {
                location,
                kind: ast::ExprKind::Id(name),
            })
            .map_err(|_| Error::Limit("assignment query syntax limit"))
    }

    fn publish_trait_operand(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        category: ValueCategory,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let mut facts = self.new_expression_facts(ty, category)?;
        facts.origin = ExpressionOrigin::Implicit;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), ty);
        super::record(&mut self.graph.expression_scopes, id.index(), scope);
        Ok(())
    }

    fn defer_trait_declval(
        &mut self,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<ast::ExprId, Error> {
        let shape = self.trait_operand_shape(ty, false);
        let id = self.trait_operand_syntax(location)?;
        self.graph.expression_predicates.operands.insert(id, shape);
        self.graph.expression_predicates.operand_order.push(id);
        Ok(id)
    }

    pub(super) fn demand_trait_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let Some((ty, category)) = self.graph.expression_predicates.operands.remove(&id) else {
            return Ok(false);
        };
        self.publish_trait_operand(id, ty, category, scope)?;
        Ok(true)
    }

    fn trait_operand(
        &mut self,
        ty: TypeId,
        location: SourceOffset,
        scope: ScopeId,
        prvalue: bool,
    ) -> Result<ast::ExprId, Error> {
        let (ty, category) = self.trait_operand_shape(ty, prvalue);
        let id = self.trait_operand_syntax(location)?;
        self.publish_trait_operand(id, ty, category, scope)?;
        Ok(id)
    }
}
