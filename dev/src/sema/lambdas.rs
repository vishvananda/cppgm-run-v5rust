// Translated from sema/analyzer_lambdas.cpp. See NOTICE.
//! Closure identity belongs to the typed expression, never its token anchor.
use super::{
    Error,
    analyzer::{Analyzer, DeferredBody},
    expression_facts::{ExprFacts, ValueCategory},
    function_names::SpecialName,
    graph::{Binding, BindingKind, EntityId, FunctionProperties, Graph, ScopeId, ScopeKind},
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator, vocabulary::Keyword},
    syntax::{ast, storage::List},
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(crate) struct Closure {
    pub call: EntityId,
    pub invoker: Option<EntityId>,
    pub conversion: Option<EntityId>,
    pub enclosing: Option<EntityId>,
    pub ordinal: u32,
    pub course_number: u16,
    deduces: bool,
    pub clause_token: u32,
    pub body_token: u32,
    pub this_member: Option<EntityId>,
    pub convertible: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Capture {
    pub entity: EntityId,
    pub member: EntityId,
}
#[derive(Default)]
pub(super) struct Closures {
    pub(super) records: HashMap<EntityId, Closure>,
    numbers: HashMap<(Option<EntityId>, TypeId), u32>,
    invocations: HashMap<ast::ExprId, EntityId>,
    pub(super) captures: HashMap<EntityId, Vec<Capture>>,
}
impl Graph {
    pub(crate) fn lower_closure_captures(&self, class: EntityId) -> &[Capture] {
        self.closures
            .captures
            .get(&class)
            .map_or(&[], Vec::as_slice)
    }
    pub(crate) fn lower_call_closure(&self, function: EntityId) -> Option<EntityId> {
        let class = self
            .entity(function)
            .home
            .and_then(|home| self.scope(home).owner)?;
        (self.closure_call(class) == Some(function)).then_some(class)
    }
    pub(crate) fn lower_closure_invocation(&self, id: ast::ExprId) -> Option<EntityId> {
        self.closures
            .invocations
            .get(&self.expression_facts.root(id))
            .copied()
    }
    pub(crate) fn lower_is_closure_invoker(&self, function: EntityId) -> bool {
        self.entity(function)
            .home
            .and_then(|home| self.scope(home).owner)
            .and_then(|class| self.lower_closure(class))
            .is_some_and(|facts| facts.invoker == Some(function))
    }
    pub(crate) fn lower_closure_converter(&self, function: EntityId) -> Option<EntityId> {
        let facts = self
            .entity(function)
            .home
            .and_then(|home| self.scope(home).owner)
            .and_then(|class| self.lower_closure(class))?;
        (facts.conversion == Some(function))
            .then_some(facts.invoker)
            .flatten()
    }
    /// The invoker executes the operator's syntax in its original lexical scope.
    pub(crate) fn lower_closure_body_owner(&self, function: EntityId) -> EntityId {
        self.entity(function)
            .home
            .and_then(|home| self.scope(home).owner)
            .and_then(|class| self.lower_closure(class))
            .filter(|facts| facts.invoker == Some(function))
            .map_or(function, |facts| facts.call)
    }
    pub(crate) fn lower_closure(&self, class: EntityId) -> Option<Closure> {
        self.closures.records.get(&class).copied()
    }
    pub(super) fn closure_deduces_result(&self, call: EntityId) -> bool {
        self.entity(call)
            .home
            .and_then(|home| self.scope(home).owner)
            .and_then(|class| self.lower_closure(class))
            .is_some_and(|facts| facts.call == call && facts.deduces)
    }
    pub(super) fn closure_call(&self, class: EntityId) -> Option<EntityId> {
        self.lower_closure(class).map(|facts| facts.call)
    }
}
impl Analyzer<'_> {
    pub(super) fn note_closure_invocation(
        &mut self,
        id: ast::ExprId,
        callee: ast::ExprId,
        class: EntityId,
    ) -> Result<(), Error> {
        let root = self.graph.expression_facts.root(callee);
        if !matches!(
            self.tree.expressions.get(root).kind,
            ast::ExprKind::Lambda(_)
        ) {
            return Ok(());
        }
        let Some(invoker) = self.closure_conversion(class, false)? else {
            return Ok(());
        };
        self.graph.closures.invocations.insert(id, invoker);
        Ok(())
    }
    pub(super) fn closure_conversion(
        &mut self,
        class: EntityId,
        conversion_function: bool,
    ) -> Result<Option<EntityId>, Error> {
        let Some(closure) = self.graph.lower_closure(class).filter(|c| c.convertible) else {
            return Ok(None);
        };
        let invoker = if let Some(invoker) = closure.invoker {
            invoker
        } else {
            let call = self.graph.entity(closure.call);
            let home = call.home.ok_or(Error::Invariant("closure invoker home"))?;
            let old = call.ty.ok_or(Error::Invariant("closure invoker type"))?;
            let body = call.analyzed_body;
            let scope = call.scope;
            let parameters = call.function_parameters;
            let exceptions = call.exceptions.clone();
            let TypeKind::Function {
                result,
                parameters: types,
                variadic,
                ..
            } = self.graph.types.get(old).kind.clone()
            else {
                return Err(Error::Invariant("closure invoker signature"));
            };
            let ty = self.graph.types.intern(
                TypeKind::Function {
                    result,
                    parameters: types,
                    variadic,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )?;
            let name = self
                .identifiers
                .intern(b"__invoke")
                .map_err(|_| Error::Limit("closure invoker name"))?;
            let invoker = self.graph.add_entity(Some(ty), Some(name), Keyword::Int)?;
            let function = self.graph.entity_mut(invoker);
            function.home = Some(home);
            function.function = Some(FunctionProperties {
                no_inline: false,
                internal_linkage: false,
                language: super::linkage::Language::Cpp,
                denotes: None,
                is_static: true,
                special: None,
                allocation: None,
            });
            function.inline = super::specifiers::InlinePreference::Inline;
            function.definition_emission = super::function_demands::DefinitionEmission::Demanded;
            function.defined = true;
            function.analyzed_body = body;
            function.scope = scope;
            function.function_parameters = parameters;
            function.exceptions = exceptions;
            self.graph
                .closures
                .records
                .get_mut(&class)
                .ok_or(Error::Invariant("closure invoker owner"))?
                .invoker = Some(invoker);
            invoker
        };
        if conversion_function && closure.conversion.is_none() {
            let home = self
                .graph
                .entity(invoker)
                .home
                .ok_or(Error::Invariant("closure conversion home"))?;
            let function_type = self
                .graph
                .entity(invoker)
                .ty
                .ok_or(Error::Invariant("closure conversion target"))?;
            let pointer = self
                .graph
                .types
                .intern(TypeKind::Pointer(function_type), Cv::default())?;
            let ty = self.graph.types.intern(
                TypeKind::Function {
                    result: pointer,
                    parameters: Vec::new(),
                    variadic: false,
                    cv: Cv::CONST,
                    reference: None,
                },
                Cv::default(),
            )?;
            let conversion = self.graph.add_entity(Some(ty), None, Keyword::Int)?;
            let function = self.graph.entity_mut(conversion);
            function.home = Some(home);
            function.function = Some(FunctionProperties {
                no_inline: false,
                internal_linkage: false,
                language: super::linkage::Language::Cpp,
                denotes: None,
                is_static: false,
                special: None,
                allocation: None,
            });
            function.member_function = Some(super::graph::MemberKey {
                class,
                kind: super::graph::MemberKind::Conversion,
                signature: ty,
                shape: None,
            });
            function.inline = super::specifiers::InlinePreference::Inline;
            function.exceptions =
                super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
            let binding = self.graph.add_binding(
                home,
                Some(home),
                Binding {
                    kind: BindingKind::Function,
                    name: None,
                    entity: conversion,
                    ty: Some(ty),
                    key: Keyword::Int,
                },
            )?;
            self.graph
                .record_special_value(home, SpecialName::Conversion, binding)?;
            self.graph
                .entity_mut(class)
                .declared_conversions
                .push(binding);
            self.graph
                .closures
                .records
                .get_mut(&class)
                .ok_or(Error::Invariant("closure conversion owner"))?
                .conversion = Some(conversion);
        }
        Ok(Some(invoker))
    }
    pub(super) fn lambda_expression(
        &mut self,
        id: ast::ExprId,
        lambda: ast::LambdaId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        if self.unevaluated != 0 {
            return Err(Error::at(
                location,
                "a lambda-expression in an unevaluated operand",
            ));
        }
        if let Some(facts) = self.graph.expression_facts.known(id)
            && self
                .graph
                .class_of(facts.ty)
                .is_some_and(|class| self.graph.closure_call(class).is_some())
        {
            return self.new_expression_facts(facts.ty, ValueCategory::Prvalue);
        }
        let lambda = *self.tree.lambdas.get(lambda);
        let ast::LambdaBody::Parsed(compound) = lambda.body else {
            return Err(Error::Invariant("unparsed lambda body"));
        };
        let home = self.declaration_scope(scope);
        let class = self.new_class(Keyword::Class, None, home)?;
        self.graph.entity_mut(class).home = Some(home);
        self.graph.local_types = self
            .graph
            .local_types
            .checked_add(1)
            .ok_or(Error::Limit("closure ordinal limit"))?;
        let ordinal = self.graph.local_types;
        let class_scope = self.graph.add_scope(ScopeKind::Class, home, None, None)?;
        self.graph.scope_mut(class_scope).owner = Some(class);
        self.graph.entity_mut(class).scope = Some(class_scope);
        self.graph.entity_mut(class).complete = true;
        self.graph.entity_mut(class).defined = true;
        self.graph.entity_mut(class).aggregation =
            super::aggregate_initialization::Aggregation::Ineligible;
        let parameters = lambda.parameters.unwrap_or(ast::ParameterClause {
            location,
            parameters: List::default(),
            ellipsis: false,
            ellipsis_without_comma: false,
        });
        let (call, ty) =
            self.declare_closure_call(class_scope, lambda, parameters, scope, location)?;
        let enclosing = self.body_context.as_ref().map(|body| body.entity);
        let canonical = self.graph.types.canonical_function(ty)?;
        let TypeKind::Function {
            parameters: ref types,
            ..
        } = self.graph.types.get(canonical).kind
        else {
            return Err(Error::Invariant("closure signature"));
        };
        let signature = types.clone();
        let key = self
            .graph
            .types
            .intern(TypeKind::Pack(signature), Cv::default())?;
        let count = self
            .graph
            .closures
            .numbers
            .entry((enclosing, key))
            .or_default();
        let earlier = *count;
        *count = count
            .checked_add(1)
            .ok_or(Error::Limit("closure numbering limit"))?;
        self.graph.closures.records.insert(
            class,
            Closure {
                call,
                invoker: None,
                conversion: None,
                enclosing,
                ordinal,
                course_number: u16::try_from(earlier).unwrap_or(u16::MAX),
                deduces: lambda.trailing.is_none(),
                clause_token: lambda.clause_token,
                body_token: lambda.body_token,
                this_member: None,
                convertible: lambda.capture_default.is_none() && lambda.captures.is_empty(),
            },
        );
        let mut info = super::declarators::DeclaratorInfo {
            ty,
            name: None,
            home: Some(class_scope),
            lookup: scope,
            parameters: lambda.parameters,
            qualifiers: None,
            syntax_name: None,
        };
        if let Some(exception) = lambda.exception {
            let start = self.tree.qualifiers.begin();
            self.tree.qualifiers.push(exception);
            info.qualifiers = Some(
                self.tree
                    .qualifiers
                    .finish(start)
                    .map_err(|_| Error::Limit("closure exception limit"))?,
            );
        }
        self.record_function_exceptions(call, &info, scope, location, self.graph.binding_limit())?;
        self.record_function_defaults(call, parameters, scope, ty, true)?;
        let body = self
            .tree
            .bodies
            .add(ast::Body::Parsed(ast::FunctionBody {
                initializer_location: None,
                initializers: List::default(),
                compound,
                handlers: None,
            }))
            .map_err(|_| Error::Limit("closure body limit"))?;
        let frame = self.explicit_captures(class, call, lambda, scope, location)?;
        let parent = if let Some(pattern) = lambda.pattern_parameters {
            let packs = self
                .tree
                .lambda_parameter_packs
                .get(lambda.parameter_packs)
                .to_vec();
            self.function_pack_scope(
                scope,
                pattern,
                &packs,
                self.graph
                    .entity(call)
                    .ty
                    .ok_or(Error::Invariant("closure pack signature"))?,
            )?
        } else {
            scope
        };
        self.capture_frames.push(frame)?;
        let saved_unevaluated = self.unevaluated;
        let saved_callee = self.callee_name.take();
        self.unevaluated = 0;
        let analyzed = self.function_body(DeferredBody {
            entity: call,
            name: None,
            ty,
            parameters,
            body,
            parent,
            print: class_scope,
            template_depth: 0,
            definition_head: None,
        });
        self.unevaluated = saved_unevaluated;
        self.callee_name = saved_callee;
        let frame = self.capture_frames.pop()?;
        analyzed?;
        self.finish_captures(frame)?;
        self.classify_class_copies(class)?;
        self.complete_destructor(class)?;
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("closure type"))?;
        self.new_expression_facts(ty, ValueCategory::Prvalue)
    }
    fn declare_closure_call(
        &mut self,
        class_scope: ScopeId,
        lambda: ast::Lambda,
        parameters: ast::ParameterClause,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(EntityId, TypeId), Error> {
        let mut qualifiers = Vec::new();
        if !lambda.is_mutable {
            qualifiers.push(ast::Qualifier {
                location,
                kind: ast::QualifierKind::Cv(Keyword::Const),
            });
        }
        if let Some(trailing) = lambda.trailing {
            qualifiers.push(ast::Qualifier {
                location,
                kind: ast::QualifierKind::Trailing(trailing),
            });
        }
        let start = self.tree.qualifiers.begin();
        for qualifier in qualifiers {
            self.tree.qualifiers.push(qualifier);
        }
        let qualifiers = self
            .tree
            .qualifiers
            .finish(start)
            .map_err(|_| Error::Limit("closure qualifier limit"))?;
        let result = if lambda.trailing.is_some() {
            self.graph.types.intern(TypeKind::Auto, Cv::default())?
        } else {
            self.graph.types.scalar(Scalar::Void)?
        };
        let (ty, _) = self.function_type(
            result,
            parameters,
            qualifiers,
            scope,
            location,
            super::declarators::FunctionContext::default(),
        )?;
        let special = SpecialName::Operator {
            token: crate::lex::vocabulary::SimpleToken::punctuator(Punctuator::LeftParen)
                .ok_or(Error::Invariant("call token"))?,
            array: false,
        };
        let call = self.graph.add_entity(Some(ty), None, Keyword::Int)?;
        let function = self.graph.entity_mut(call);
        function.function = Some(FunctionProperties {
            no_inline: false,
            internal_linkage: false,
            language: super::linkage::Language::Cpp,
            denotes: None,
            is_static: false,
            special: Some(special),
            allocation: None,
        });
        function.inline = super::specifiers::InlinePreference::Inline;
        function.definition_emission = super::function_demands::DefinitionEmission::Demanded;
        function.defined = true;
        function.function_parameters = Some(parameters);
        let attributes = self.layout_attributes(lambda.attributes, scope, false)?;
        self.graph.note_no_inline(call, attributes.no_inline)?;
        let binding = self.graph.add_binding(
            class_scope,
            Some(class_scope),
            Binding {
                kind: BindingKind::Function,
                name: None,
                entity: call,
                ty: Some(ty),
                key: Keyword::Int,
            },
        )?;
        self.graph
            .record_special_value(class_scope, special, binding)?;
        Ok((call, ty))
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
    fn instantiated_lambda_bodies_get_private_facts_and_distinct_canonical_classes() {
        let source = SourceBuffer::new(b"template<class T>int f(){auto c=[](T x){return x+1;};return c(3);}int main(){return f<int>()+f<char>();}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"closure-instances").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        assert_eq!(tree.lambdas.mark(), 1);
        let graph = super::super::template_bodies::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(tree.lambdas.mark(), 3);
        let closures: Vec<_> = graph.closures.records.iter().collect();
        assert_eq!(closures.len(), 2);
        assert_ne!(closures[0].0, closures[1].0);
        assert_ne!(closures[0].1.enclosing, closures[1].1.enclosing);
        assert_ne!(
            graph.entity(closures[0].1.call).analyzed_body,
            graph.entity(closures[1].1.call).analyzed_body
        );
        for (_, closure) in closures {
            assert_eq!(closure.course_number, 0);
            assert!(closure.invoker.is_none());
            assert_eq!(graph.entity(closure.call).parameter_entities.len(), 1);
        }
    }

    #[test]
    fn direct_invocation_shares_one_analyzed_body_without_copying_expression_syntax() {
        let source = SourceBuffer::new(b"int n;int main(){[]{n=1;}();return n;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"closure-invoker").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let statements = tree.statements.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.statements.mark(), statements);
        assert_eq!(graph.closures.records.len(), 1);
        let closure = graph.closures.records.values().next().unwrap();
        let invoker = closure.invoker.unwrap();
        assert_eq!(graph.closures.invocations.len(), 1);
        assert_eq!(
            graph.entity(invoker).analyzed_body,
            graph.entity(closure.call).analyzed_body
        );
        assert_eq!(
            graph.entity(invoker).scope,
            graph.entity(closure.call).scope
        );
        assert!(graph.entity(invoker).function.unwrap().is_static);
        assert!(!graph.entity(closure.call).function.unwrap().is_static);
    }

    #[test]
    fn pointer_conversion_reuses_signature_body_and_scope_without_eager_unused_helpers() {
        let source = SourceBuffer::new(
            br"using F=int&(*)(int&);
            int main(){auto unused=[]{return 0;};
            auto f=[](int& x)noexcept->int&{static int n;return x;};
            F p=f;F q=f.operator F();return p==q;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"closure-conversion-ownership").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let statements = tree.statements.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.statements.mark(), statements);
        assert_eq!(graph.closures.records.len(), 2);
        let closure = graph
            .closures
            .records
            .values()
            .find(|c| c.conversion.is_some())
            .copied()
            .unwrap();
        let unused = graph
            .closures
            .records
            .values()
            .find(|c| c.conversion.is_none())
            .unwrap();
        assert!(unused.invoker.is_none());
        let call = graph.entity(closure.call);
        let invoker = closure.invoker.unwrap();
        let function = graph.entity(invoker);
        assert_eq!(function.analyzed_body, call.analyzed_body);
        assert_eq!(function.scope, call.scope);
        assert!(matches!((&function.exceptions, &call.exceptions),
            (super::super::exceptions::ExceptionState::Value(first),
             super::super::exceptions::ExceptionState::Value(second)) if first == second));
        let TypeKind::Function {
            result,
            parameters: ref invocation_parameters,
            cv,
            reference,
            ..
        } = graph.types.get(function.ty.unwrap()).kind
        else {
            panic!("function type")
        };
        assert_eq!(cv, Cv::default());
        assert!(reference.is_none());
        assert!(matches!(
            graph.types.get(result).kind,
            TypeKind::Reference { .. }
        ));
        assert_eq!(invocation_parameters.len(), 1);
        assert!(matches!(
            graph.types.get(invocation_parameters[0]).kind,
            TypeKind::Reference { .. }
        ));
        let converter = closure.conversion.unwrap();
        assert_eq!(graph.lower_closure_converter(converter), Some(invoker));
        let TypeKind::Function {
            result: pointer,
            ref parameters,
            cv,
            ..
        } = graph.types.get(graph.entity(converter).ty.unwrap()).kind
        else {
            panic!("conversion type")
        };
        assert_eq!(cv, Cv::CONST);
        assert!(parameters.is_empty());
        assert!(
            matches!(graph.types.get(pointer).kind, TypeKind::Pointer(ty) if Some(ty)==function.ty)
        );
        let TypeKind::Function {
            result: call_result,
            parameters: ref call_parameters,
            ..
        } = graph.types.get(call.ty.unwrap()).kind
        else {
            panic!("operator signature")
        };
        assert_eq!(call_result, result);
        assert_eq!(call_parameters, invocation_parameters);
        assert_eq!(graph.lower_closure_body_owner(invoker), closure.call);
    }
}
