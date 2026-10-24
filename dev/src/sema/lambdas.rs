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
pub(crate) enum ClosureContext {
    None,
    MemberSpecification(EntityId),
    StaticMemberInitializer(EntityId),
}
/// Itanium object naming keeps each member initializer and default parameter
/// separate. Course `LowIR` retains its historical class-wide numbering.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClosureNamingContext {
    DataMember(EntityId),
    DefaultArgument { function: EntityId, index: u32 },
}
#[derive(Clone, Copy)]
pub(crate) struct Closure {
    pub call: EntityId,
    pub invoker: Option<EntityId>,
    pub conversion: Option<EntityId>,
    pub enclosing: Option<EntityId>,
    pub context: ClosureContext,
    pub ordinal: u32,
    pub course_number: u16,
    pub object_number: u16,
    pub naming_context: Option<ClosureNamingContext>,
    deduces: bool,
    pub clause_token: u32,
    pub body_token: u32,
    pub this_member: Option<EntityId>,
    pub convertible: bool,
    pub template: Option<ast::LambdaId>,
}
#[derive(Clone, Copy)]
pub(crate) struct Capture {
    pub entity: EntityId,
    pub member: EntityId,
    pub constructor: Option<super::construction::ConstructorId>,
}
#[derive(Default)]
pub(super) struct Closures {
    pub(super) records: HashMap<EntityId, Closure>,
    numbers: HashMap<(Option<EntityId>, TypeId), u32>,
    context_numbers: HashMap<(ClosureNamingContext, TypeId, bool), u32>,
    invocations: HashMap<ast::ExprId, EntityId>,
    pub(super) captures: HashMap<EntityId, Vec<Capture>>,
}
impl Graph {
    pub(super) fn closure_member_context(&self, class: EntityId) -> bool {
        self.lower_closure(class).is_some_and(|closure| {
            matches!(closure.context, ClosureContext::MemberSpecification(owner)
                if self.class_templates.instance(owner).is_none())
        })
    }
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
        let primary = self
            .function_templates
            .primary(function)
            .map_or(function, |binding| self.binding(binding).entity);
        (self.closure_call(class) == Some(primary)).then_some(class)
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
                effects: super::graph::FunctionEffect::Default,
                no_inline: false,
                returns: super::graph::FunctionReturn::MayReturn,
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
                .source_exceptions
                .inherit_specification(invoker, closure.call);
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
                effects: super::graph::FunctionEffect::Default,
                no_inline: false,
                returns: super::graph::FunctionReturn::MayReturn,
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
        let lambda_id = lambda;
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
        let (signature_scope, head) = if let Some(parameters) = lambda.template_parameters {
            if parameters.parameters.is_empty() {
                return Err(Error::at(
                    location,
                    "an explicit specialization outside namespace scope",
                ));
            }
            let parent =
                self.graph
                    .add_scope(ScopeKind::TemplateParameters, scope, Some(scope), None)?;
            let head = self.template_parameters(parameters, parent, true)?;
            (parent, Some(head))
        } else {
            (scope, None)
        };
        let saved_depth = self.template_depth;
        let saved_head = self.template_head;
        if head.is_some() {
            self.template_depth += 1;
            self.template_head = head;
        }
        let declared =
            self.declare_closure_call(class_scope, lambda, parameters, signature_scope, location);
        self.template_depth = saved_depth;
        self.template_head = saved_head;
        let (call, ty) = declared?;
        let enclosing = self.body_context.as_ref().map(|body| body.entity);
        let context = self.closure_context(home);
        let naming_context = self.closure_naming_context(home);
        let (course_number, object_number) =
            self.number_closure(ty, enclosing, context, naming_context)?;
        self.graph.closures.records.insert(
            class,
            Closure {
                call,
                invoker: None,
                conversion: None,
                enclosing,
                context,
                ordinal,
                course_number,
                object_number,
                naming_context,
                deduces: lambda.trailing.is_none(),
                clause_token: lambda.clause_token,
                body_token: lambda.body_token,
                this_member: None,
                convertible: head.is_none()
                    && lambda.capture_default.is_none()
                    && lambda.captures.is_empty(),
                template: head.map(|_| lambda_id),
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
            asm_label: None,
        };
        if lambda.exception.is_some() || lambda.trailing.is_some() {
            let start = self.tree.qualifiers.begin();
            if let Some(exception) = lambda.exception {
                self.tree.qualifiers.push(exception);
            }
            if let Some(trailing) = lambda.trailing {
                self.tree.qualifiers.push(ast::Qualifier {
                    location,
                    kind: ast::QualifierKind::Trailing(trailing),
                });
            }
            info.qualifiers = Some(
                self.tree
                    .qualifiers
                    .finish(start)
                    .map_err(|_| Error::Limit("closure exception limit"))?,
            );
        }
        let saved_depth = self.template_depth;
        let saved_head = self.template_head;
        if head.is_some() {
            self.template_depth += 1;
            self.template_head = head;
        }
        let signature = (|| {
            self.record_function_exceptions(
                call,
                &info,
                signature_scope,
                location,
                self.graph.binding_limit(),
            )?;
            if head.is_some() {
                self.record_template_result_order(call, &info);
                self.record_template_function_defaults(call, parameters, signature_scope, ty)
            } else {
                self.record_function_defaults(call, parameters, scope, ty, true)
            }
        })();
        self.template_depth = saved_depth;
        self.template_head = saved_head;
        signature?;
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
            signature_scope
        };
        self.capture_frames.push(frame)?;
        let saved_unevaluated = self.unevaluated;
        let saved_callee = self.callee_name.take();
        self.unevaluated = 0;
        let saved_scan = self.capture_scan;
        if head.is_some() {
            self.capture_scan = Some(call);
        }
        let analyzed = self.function_body(DeferredBody {
            entity: call,
            name: None,
            ty,
            parameters,
            body,
            parent,
            print: class_scope,
            template_depth: usize::from(head.is_some()),
            definition_head: head,
        });
        self.capture_scan = saved_scan;
        self.unevaluated = saved_unevaluated;
        self.callee_name = saved_callee;
        let frame = self.capture_frames.pop()?;
        analyzed?;
        self.finish_captures(frame)?;
        self.classify_class_copies(class)?;
        self.complete_destructor(class)?;
        // Class completion declares the implicit destructor; its body and
        // emission stay demanded by the lifetime that materializes the closure.
        let _destructor = self.destructor_callable(class)?;
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("closure type"))?;
        self.new_expression_facts(ty, ValueCategory::Prvalue)
    }

    fn closure_context(&self, home: ScopeId) -> ClosureContext {
        if self.body_context.is_some() || self.graph.scope(home).kind != ScopeKind::Class {
            return ClosureContext::None;
        }
        let Some(class) = self.graph.scope(home).owner else {
            return ClosureContext::None;
        };
        match self.initializing {
            Some(variable)
                if self.graph.entity(variable).data_member
                    == Some(super::graph::DataMemberKind::Instance) =>
            {
                ClosureContext::MemberSpecification(class)
            }
            Some(variable) if self.graph.variable_instances.instance(variable) => {
                ClosureContext::StaticMemberInitializer(variable)
            }
            None => ClosureContext::MemberSpecification(class),
            _ => ClosureContext::None,
        }
    }

    fn closure_naming_context(&self, home: ScopeId) -> Option<ClosureNamingContext> {
        if self.body_context.is_some() || self.graph.scope(home).kind != ScopeKind::Class {
            return None;
        }
        if let Some((function, index)) = self.closure_default_argument {
            return Some(ClosureNamingContext::DefaultArgument { function, index });
        }
        self.initializing
            .filter(|&variable| {
                self.graph.entity(variable).data_member
                    == Some(super::graph::DataMemberKind::Instance)
                    || self.graph.variable_instances.instance(variable)
            })
            .map(ClosureNamingContext::DataMember)
    }

    fn number_closure(
        &mut self,
        ty: TypeId,
        enclosing: Option<EntityId>,
        context: ClosureContext,
        naming_context: Option<ClosureNamingContext>,
    ) -> Result<(u16, u16), Error> {
        let canonical = self.graph.types.canonical_function(ty)?;
        let TypeKind::Function {
            parameters,
            variadic,
            ..
        } = &self.graph.types.get(canonical).kind
        else {
            return Err(Error::Invariant("closure signature"));
        };
        let variadic = *variadic;
        let mut signature = parameters.clone();
        let context_key = if let Some(context) = naming_context {
            let parameters = self
                .graph
                .types
                .intern(TypeKind::Pack(signature.clone()), Cv::default())?;
            Some((context, parameters, variadic))
        } else {
            None
        };
        if let ClosureContext::MemberSpecification(class) = context
            && self.graph.class_templates.instance(class).is_none()
        {
            signature.insert(0, self.graph.lower_entity_type(class)?);
        }
        let key = self
            .graph
            .types
            .intern(TypeKind::Pack(signature.clone()), Cv::default())?;
        signature.push(self.graph.types.scalar(Scalar::Void)?);
        let variadic_key = self
            .graph
            .types
            .intern(TypeKind::Pack(signature), Cv::default())?;
        let course_number = next_closure_number(
            self.graph
                .closures
                .numbers
                .entry((enclosing, key))
                .or_default(),
        )?;
        let earlier_variadic = self
            .graph
            .closures
            .numbers
            .entry((enclosing, variadic_key))
            .or_default();
        let object_number = if variadic {
            next_closure_number(earlier_variadic)?
        } else {
            u16::try_from(u32::from(course_number).saturating_sub(*earlier_variadic))
                .map_err(|_| Error::Invariant("closure number range"))?
        };
        let object_number = if let Some(key) = context_key {
            next_closure_number(self.graph.closures.context_numbers.entry(key).or_default())?
        } else {
            object_number
        };
        Ok((course_number, object_number))
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
        let result = if lambda.trailing.is_some() || lambda.template_parameters.is_some() {
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
            super::declarators::FunctionContext {
                placeholder_result: lambda.template_parameters.is_some(),
                ..super::declarators::FunctionContext::default()
            },
        )?;
        let special = SpecialName::Operator {
            token: crate::lex::vocabulary::SimpleToken::punctuator(Punctuator::LeftParen)
                .ok_or(Error::Invariant("call token"))?,
            array: false,
        };
        let call = self.graph.add_entity(Some(ty), None, Keyword::Int)?;
        let function = self.graph.entity_mut(call);
        function.function = Some(FunctionProperties {
            effects: super::graph::FunctionEffect::Default,
            no_inline: false,
            returns: super::graph::FunctionReturn::MayReturn,
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
        self.graph.note_effects(call, attributes.effects)?;
        if lambda.template_parameters.is_some() {
            let head = self
                .template_head
                .ok_or(Error::Invariant("closure template head"))?;
            self.check_function_packs(ty, head, location)?;
            self.merge_template_head(call, head, location)?;
        }
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

fn next_closure_number(count: &mut u32) -> Result<u16, Error> {
    let earlier = u16::try_from(*count).unwrap_or(u16::MAX);
    *count = count
        .checked_add(1)
        .ok_or(Error::Limit("closure numbering limit"))?;
    Ok(earlier)
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
    fn member_template_calls_share_fixed_captures_but_own_their_body_facts() {
        let source = SourceBuffer::new(
            b"int main(){const int n=4;int x=1;auto f=[=]<class T>(T t,int y=3){return x+n+t+y;};return f(1)+f('a');}".to_vec()
        ).unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"templated-closure-ownership").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let pattern_count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.closures.records.len(), 1);
        let (&class, closure) = graph.closures.records.iter().next().unwrap();
        assert!(closure.template.is_some());
        assert!(!closure.convertible);
        assert!(graph.entity(closure.call).template_head.is_some());
        let captures = graph.lower_closure_captures(class);
        assert_eq!(captures.len(), 2);
        assert_ne!(captures[0].entity, captures[1].entity);
        let mut bodies = Vec::new();
        for &entity in graph.function_templates.arguments.keys() {
            if graph.lower_call_closure(entity) == Some(class) {
                bodies.push(graph.entity(entity).analyzed_body.unwrap());
            }
        }
        assert_eq!(bodies.len(), 2);
        assert_ne!(bodies[0], bodies[1]);
        assert!(tree.expressions.mark() > pattern_count);
    }

    #[test]
    fn closure_numbers_separate_object_signatures_and_member_contexts() {
        let source = SourceBuffer::new(
            b"inline int f(){auto a=[](...){return 1;};auto b=[]{return 2;};\
              auto c=[](...){return 3;};auto d=[]{return 4;};return a()+b()+c()+d();}\
              struct A{int m=[]{return 1;}();int n=[]{return 2;}();\
              int g(int x=[]{return 3;}()){return x;}\
              int h(int x=[]{return 4;}()){return x;}};\
              int main(){A a;return f()+a.g()+a.h()+a.m+a.n;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"closure-number-contexts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut local: Vec<_> = graph
            .closures
            .records
            .values()
            .filter(|closure| closure.enclosing.is_some())
            .collect();
        local.sort_unstable_by_key(|closure| closure.ordinal);
        assert_eq!(local.len(), 4);
        assert_eq!(
            local
                .iter()
                .map(|closure| closure.course_number)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        assert_eq!(
            local
                .iter()
                .map(|closure| closure.object_number)
                .collect::<Vec<_>>(),
            [0, 0, 1, 1]
        );
        assert!(local.iter().all(|closure| closure.naming_context.is_none()));
        let members: Vec<_> = graph
            .closures
            .records
            .values()
            .filter(|closure| closure.enclosing.is_none())
            .collect();
        assert_eq!(members.len(), 4);
        let mut numbers: Vec<_> = members
            .iter()
            .map(|closure| closure.course_number)
            .collect();
        numbers.sort_unstable();
        assert_eq!(numbers, [0, 1, 2, 3]);
        let contexts: std::collections::HashSet<_> = members
            .iter()
            .map(|closure| closure.naming_context.unwrap())
            .collect();
        assert_eq!(contexts.len(), 4);
        assert_eq!(
            contexts
                .iter()
                .filter(|context| matches!(context, ClosureNamingContext::DataMember(_)))
                .count(),
            2
        );
        assert!(members.iter().all(|closure| closure.object_number == 0));
    }

    #[test]
    fn nested_member_initializers_mask_the_surrounding_default_context() {
        let source = SourceBuffer::new(
            b"struct A{int f(int v=[]{struct B{int n=[]{return 1;}();\
              int g(int n=[]{return 2;}()){return n;}};B b;return b.n+b.g();}())\
              {return v;}};int main(){A a;return a.f();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"nested-closure-contexts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.closures.records.len(), 3);
        let contexts: std::collections::HashSet<_> = graph
            .closures
            .records
            .values()
            .map(|closure| closure.naming_context.unwrap())
            .collect();
        assert_eq!(contexts.len(), 3);
        assert_eq!(
            contexts
                .iter()
                .filter(|context| matches!(context, ClosureNamingContext::DataMember(_)))
                .count(),
            1
        );
        assert!(
            graph
                .closures
                .records
                .values()
                .all(|closure| closure.object_number == 0)
        );
    }

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
