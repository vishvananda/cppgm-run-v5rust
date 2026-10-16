// Translated from template_placeholders.cpp and class_templates.cpp::substituted_scope.
// See NOTICE.
//! Signature operands retain parsed syntax and definition-owned environments.
use super::{
    Error,
    analyzer::Analyzer,
    expression_keys::KeyWord,
    function_templates::Substitution,
    graph::{Binding, BindingKind, EntityId, ScopeId, ScopeKind},
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::{ast, cloning::Cloner},
};
use std::{
    collections::{HashMap, hash_map::RandomState},
    hash::BuildHasher,
};

#[derive(Clone, Copy)]
pub(super) struct Pattern {
    pub expression: ast::ExprId,
    pub scope: ScopeId,
    pub key_dependent: bool,
    home: ScopeId,
    next: Option<EntityId>,
    limit: usize,
    // Known positions below 63, matching the frozen owner's conservative mask.
    // Unknown keys and higher positions retain ordinary substitution.
    parameters: Option<u64>,
}

#[derive(PartialEq, Eq, Hash)]
struct Key {
    entity: EntityId,
    context: Option<EntityId>,
    depth: u32,
    arguments: Vec<TypeId>,
}
#[derive(Clone, Copy)]
enum State {
    InProgress,
    Complete(TypeId),
}
#[derive(Clone, Copy)]
enum Query {
    Type,
    Bound,
}
enum Evaluation {
    Type(TypeId),
    Bound(u64),
}
#[derive(Default)]
pub(super) struct Operands {
    index: HashMap<u64, EntityId>,
    words: HashMap<EntityId, Vec<KeyWord>>,
    fingerprints: RandomState,
    patterns: HashMap<EntityId, Pattern>,
    equivalents: HashMap<(ScopeId, Vec<KeyWord>), EntityId>,
    substitutions: HashMap<Key, State>,
}
impl Operands {
    pub(super) fn count(&self) -> usize {
        self.patterns.len()
    }
    pub(super) fn words(&self, entity: EntityId) -> Option<&[KeyWord]> {
        self.words.get(&entity).map(Vec::as_slice)
    }
    pub(super) fn pattern(&self, entity: EntityId) -> Option<Pattern> {
        self.patterns.get(&entity).copied()
    }
}
impl Analyzer<'_> {
    pub(super) fn dependent_operand(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        fixed: bool,
    ) -> Result<TypeId, Error> {
        self.expression(expression, scope)?;
        let key = self.operand_key(expression, scope)?;
        // Non-dependent operands in semantic mode have their ordinary type.
        if fixed
            && key.as_ref().is_some_and(|key| !key.dependent)
            && !self.type_dependent_expression(expression, scope)?
        {
            let saved = std::mem::replace(&mut self.template_depth, 0);
            let result = self.decltype_type(expression, scope);
            self.template_depth = saved;
            return result;
        }
        let key_dependent = key.as_ref().is_some_and(|key| key.dependent);
        let words = key.map(|key| key.words);
        let parameters = words
            .as_deref()
            .and_then(|words| self.operand_parameters(words));
        let mut home = scope;
        while matches!(
            self.graph.scope(home).kind,
            ScopeKind::Function | ScopeKind::Block | ScopeKind::TemplateParameters
        ) {
            home = self
                .graph
                .scope(home)
                .parent
                .ok_or(Error::Invariant("operand without home"))?;
        }
        if self.namespace_friend_class == Some(home) {
            while self.graph.scope(home).kind != ScopeKind::Namespace {
                home = self
                    .graph
                    .scope(home)
                    .parent
                    .ok_or(Error::Invariant("friend without namespace"))?;
            }
        }
        let fingerprint = words.as_ref().map(|words| {
            self.graph
                .template_operands
                .fingerprints
                .hash_one((home, words))
        });
        let head = fingerprint.and_then(|fingerprint| {
            self.graph
                .template_operands
                .index
                .get(&fingerprint)
                .copied()
        });
        let mut current = head;
        while let Some(entity) = current {
            let pattern = self
                .graph
                .template_operands
                .pattern(entity)
                .ok_or(Error::Invariant("operand index without pattern"))?;
            if pattern.home == home
                && self.graph.template_operands.words.get(&entity) == words.as_ref()
            {
                return self
                    .graph
                    .entity(entity)
                    .ty
                    .ok_or(Error::Invariant("operand without type"));
            }
            current = pattern.next;
        }
        let name = self.decltype_name;
        let entity = self.graph.add_entity(None, Some(name), Keyword::Typename)?;
        let ty = self.graph.types.intern(
            TypeKind::TemplateParameter {
                entity,
                template: false,
            },
            Cv::default(),
        )?;
        self.graph.entity_mut(entity).ty = Some(ty);
        self.graph.template_operands.patterns.insert(
            entity,
            Pattern {
                expression,
                scope,
                key_dependent,
                home,
                next: head,
                limit: self.graph.binding_limit(),
                parameters,
            },
        );
        if let Some(fingerprint) = fingerprint
            && let Some(words) = words
        {
            self.graph
                .template_operands
                .index
                .insert(fingerprint, entity);
            if words
                .iter()
                .any(|word| matches!(word, KeyWord::DependentCallee(..)))
            {
                let equivalent = words
                    .iter()
                    .map(|word| match *word {
                        KeyWord::DependentCallee(_, name) => KeyWord::FreeName(name),
                        _ => word.clone(),
                    })
                    .collect::<Vec<_>>();
                let earlier = *self
                    .graph
                    .template_operands
                    .equivalents
                    .entry((home, equivalent))
                    .or_insert(entity);
                if earlier != entity {
                    self.graph
                        .types
                        .record_operand_representative(entity, earlier);
                }
            }
            self.graph.template_operands.words.insert(entity, words);
        }
        Ok(ty)
    }

    pub(super) fn substitute_operand(
        &mut self,
        ty: TypeId,
        entity: EntityId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let Some(arguments) = environment
            .arguments
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(Some(ty));
        };
        if arguments
            .iter()
            .any(|&ty| self.exception_dependent_type(ty))
        {
            return Ok(Some(ty));
        }
        let key = Key {
            entity,
            context: environment.context,
            depth: environment.depth,
            arguments,
        };
        match self.graph.template_operands.substitutions.get(&key) {
            Some(State::Complete(result)) => {
                return self
                    .graph
                    .types
                    .qualify(*result, self.graph.types.get(ty).cv)
                    .map(Some);
            }
            Some(State::InProgress) => return Ok(None),
            None => (),
        }
        self.graph
            .template_operands
            .substitutions
            .insert(key, State::InProgress);
        let result = self
            .evaluate_operand(entity, environment, Query::Type)
            .and_then(|result| match result {
                Evaluation::Type(ty) => Ok(ty),
                Evaluation::Bound(_) => {
                    Err(Error::Invariant("operand type query returned a bound"))
                }
            });
        let key = Key {
            entity,
            context: environment.context,
            depth: environment.depth,
            arguments: environment
                .arguments
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::Invariant("lost operand arguments"))?,
        };
        match result {
            Ok(result) => {
                self.graph
                    .template_operands
                    .substitutions
                    .insert(key, State::Complete(result));
                self.graph
                    .types
                    .qualify(result, self.graph.types.get(ty).cv)
                    .map(Some)
            }
            Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => {
                // Failed lookups may name a class completed later. Keep only
                // successful facts; a failed trial cannot poison later demand.
                self.graph.template_operands.substitutions.remove(&key);
                Ok(None)
            }
            Err(error) => {
                self.graph.template_operands.substitutions.remove(&key);
                Err(error)
            }
        }
    }

    pub(super) fn substitute_array_bound(
        &mut self,
        operand: TypeId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<u64>, Error> {
        let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(operand).kind else {
            return Err(Error::Invariant("array bound without operand placeholder"));
        };
        match self.evaluate_operand(entity, environment, Query::Bound) {
            Ok(Evaluation::Bound(bound)) => Ok(Some(bound)),
            Ok(Evaluation::Type(_)) => Err(Error::Invariant("bound query returned a type")),
            Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn evaluate_operand(
        &mut self,
        entity: EntityId,
        environment: &mut Substitution<'_>,
        query: Query,
    ) -> Result<Evaluation, Error> {
        self.check_stack()?;
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("missing operand"))?;
        let scope = self.operand_scope(pattern.scope, environment, pattern.parameters)?;
        let copy = Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .expression(pattern.expression)
            .map_err(|e| super::template_bodies::copy_error(&e))?;
        let access = environment
            .context
            .map(|entity| self.substitution_scope(entity))
            .transpose()?;
        let saved_access = std::mem::replace(
            &mut self.declaration_access.substitution,
            access.map(|point| (scope, point)),
        );
        let saved_function = self.declaration_access.function.take();
        let saved_access_class = self.declaration_access.class.take();
        let saved_body = self.body_context.take();
        let saved_limit = self.graph.lookup_limit.replace(pattern.limit);
        let saved_depth = std::mem::replace(&mut self.template_depth, 0);
        let saved_unevaluated = std::mem::replace(&mut self.unevaluated, 1);
        let saved_default = self.default_argument_scope.take();
        let saved_head = self.template_head.take();
        let saved_friend = self.namespace_friend_class.take();
        let saved_class = std::mem::replace(&mut self.class_depth, 0);
        let saved_initializing = self.initializing.take();
        let saved_base = self.base_object.take();
        let saved_member = self.member_initializer_class.take();
        let result = match query {
            Query::Type => self.decltype_type(copy, scope).map(Evaluation::Type),
            Query::Bound => self.array_bound(copy, scope).map(Evaluation::Bound),
        };
        self.declaration_access.substitution = saved_access;
        self.declaration_access.function = saved_function;
        self.declaration_access.class = saved_access_class;
        self.body_context = saved_body;
        self.graph.lookup_limit = saved_limit;
        self.template_depth = saved_depth;
        self.unevaluated = saved_unevaluated;
        self.default_argument_scope = saved_default;
        self.template_head = saved_head;
        self.namespace_friend_class = saved_friend;
        self.class_depth = saved_class;
        self.initializing = saved_initializing;
        self.base_object = saved_base;
        self.member_initializer_class = saved_member;
        result
    }

    fn operand_scope(
        &mut self,
        scope: ScopeId,
        environment: &mut Substitution<'_>,
        parameters: Option<u64>,
    ) -> Result<ScopeId, Error> {
        self.check_stack()?;
        let node = self.graph.scope(scope);
        if !matches!(
            node.kind,
            ScopeKind::Function | ScopeKind::TemplateParameters
        ) || node.owner.is_some()
        {
            return Ok(scope);
        }
        let kind = node.kind;
        let parent = node
            .parent
            .ok_or(Error::Invariant("operand frame without parent"))?;
        let parent = self.operand_scope(parent, environment, parameters)?;
        let frame = self.graph.add_scope(kind, parent, None, None)?;
        for index in 0..self.graph.scope(scope).bindings.len() {
            let source = self.graph.scope(scope).bindings[index];
            let binding = self.graph.binding(source);
            if kind == ScopeKind::TemplateParameters {
                if let Some(parameter) = self.graph.entity(binding.entity).template_parameter
                    && let Some(index) = parameter.index(environment.depth)
                    && let Some(Some(ty)) = environment.arguments.get(index)
                {
                    self.graph.add_binding(
                        frame,
                        None,
                        Binding {
                            kind: BindingKind::Alias,
                            ty: Some(*ty),
                            ..binding
                        },
                    )?;
                }
            } else if binding.kind == BindingKind::Parameter {
                let skipped = binding.ty.is_some_and(|ty| self.graph.dependent_type(ty))
                    && parameters.is_some_and(|mask| {
                        self.graph
                            .prototype_parameter(binding.entity)
                            .is_some_and(|index| index < 63 && mask & (1u64 << index) == 0)
                    });
                let ty = binding
                    .ty
                    .filter(|_| !skipped)
                    .map(|ty| self.substitute_signature_type(ty, environment))
                    .transpose()?
                    .flatten();
                let entity = self.graph.add_entity(ty, binding.name, binding.key)?;
                self.graph.add_binding(
                    frame,
                    None,
                    Binding {
                        entity,
                        ty,
                        ..binding
                    },
                )?;
            }
        }
        Ok(frame)
    }

    fn operand_parameters(&self, words: &[KeyWord]) -> Option<u64> {
        let mut mask = 0;
        let mut types = Vec::new();
        for word in words {
            match *word {
                KeyWord::Parameter { index, .. } if index < 63 => mask |= 1u64 << index,
                KeyWord::Type(ty) => types.push(ty),
                _ => (),
            }
        }
        // A type may itself name a retained operand. Its named positions also
        // belong to this key; visit shared type DAG nodes only once.
        let mut visited = std::collections::HashSet::new();
        while let Some(ty) = types.pop() {
            if !visited.insert(ty) {
                continue;
            }
            match &self.graph.types.get(ty).kind {
                TypeKind::TemplateParameter { entity, .. } => {
                    if let Some(pattern) = self.graph.template_operands.pattern(*entity) {
                        mask |= pattern.parameters?;
                    }
                    if let Some(member) = self.graph.dependent_members.get(*entity) {
                        types.push(member.qualifier);
                    }
                }
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.graph.class_templates.instance(*entity) {
                        types.extend(&instance.arguments);
                    }
                }
                TypeKind::Pointer(ty) | TypeKind::Reference { element: ty, .. } => types.push(*ty),
                TypeKind::Array {
                    element, operand, ..
                } => {
                    types.push(*element);
                    types.extend(operand);
                }
                TypeKind::MemberPointer { owner, element } => types.extend([*owner, *element]),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    types.push(*result);
                    types.extend(parameters);
                }
                _ => (),
            }
        }
        Some(mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
        types::FundamentalType,
    };

    #[test]
    fn array_bounds_keep_identity_through_canonicalization_and_cv() {
        let source = SourceBuffer::new(
            b"template<class T>int f(const T(&)[sizeof(T)]);\
              template<class U>int f(const U(&)[sizeof(U)]);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"array-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let mut graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.template_operands.patterns.len(), 1);
        let operand = graph
            .template_operands
            .patterns
            .keys()
            .next()
            .copied()
            .unwrap();
        let operand = graph.entity(operand).ty.unwrap();
        let int = graph
            .types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let array = graph
            .types
            .intern(
                TypeKind::Array {
                    element: int,
                    bound: 0,
                    operand: Some(operand),
                },
                Cv::default(),
            )
            .unwrap();
        let unknown = graph
            .types
            .intern(
                TypeKind::Array {
                    element: int,
                    bound: 0,
                    operand: None,
                },
                Cv::default(),
            )
            .unwrap();
        assert_ne!(array, unknown);
        assert_eq!(graph.types.complete_array(array, unknown), None);
        let qualified = graph.types.qualify(array, Cv::CONST).unwrap();
        for ty in [array, qualified] {
            assert!(graph.dependent_type(ty));
            for canonical in [
                graph.types.canonical_function(ty).unwrap(),
                graph.types.declared_type(ty).unwrap(),
            ] {
                assert!(
                    matches!(graph.types.get(canonical).kind, TypeKind::Array { bound: 0, operand: Some(id), .. } if id == operand)
                );
            }
        }
        assert!(graph.template_operands.substitutions.is_empty());
    }

    #[test]
    fn renamed_parameter_types_keep_nested_operand_positions() {
        let source = SourceBuffer::new(
            b"template<class T>auto a(T x,decltype(x+1) y)->decltype(y+1);\
              template<class U>auto a(U value,decltype(value+1) extra)->decltype(extra+1){return value+extra;}\
              int main(){return a(1,2);}".to_vec(),
        ).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"nested-positions").unwrap());
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
        assert_eq!(graph.template_operands.patterns.len(), 2);
        assert_eq!(graph.template_operands.substitutions.len(), 2);
        assert!(
            graph
                .template_operands
                .substitutions
                .values()
                .all(|state| { matches!(state, State::Complete(_)) })
        );
    }

    #[test]
    fn operands_share_identity_but_keep_substitution_facts_separate() {
        let source = SourceBuffer::new(
            b"template<class T>auto f(T x)->decltype(x+1);\
            template<class U>auto f(U y)->decltype(y+1);\
            int main(){return sizeof(f(1))+sizeof(f(2))+sizeof(f(1L));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"operand-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        // Renamed parameters have one canonical structural operand. Its two
        // concrete argument lists each own one copy of its three expressions.
        assert_eq!(graph.template_operands.patterns.len(), 1);
        assert_eq!(graph.template_operands.substitutions.len(), 2);
        assert!(
            graph
                .template_operands
                .substitutions
                .values()
                .all(|state| matches!(state, State::Complete(_)))
        );
        assert_eq!(tree.expressions.mark(), count + 6);
        let pattern = graph.template_operands.patterns.values().next().unwrap();
        assert!(graph.expression_facts.get(pattern.expression).is_err());
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn invalid_operand_removes_in_progress_state_and_restores_caller() {
        let source =
            SourceBuffer::new(b"template<class T>auto f(T x)->decltype(x.missing());".to_vec())
                .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"operand-failure").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        for id in analyzer.tree.decl_lists.get(declarations).to_vec() {
            analyzer.declaration(id, scope).unwrap();
        }
        let entity = *analyzer
            .graph
            .template_operands
            .patterns
            .keys()
            .next()
            .unwrap();
        let ty = analyzer.graph.entity(entity).ty.unwrap();
        let argument = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        analyzer.template_depth = 4;
        analyzer.class_depth = 2;
        analyzer.unevaluated = 3;
        analyzer.default_argument_scope = Some(scope);
        analyzer.initializing = Some(entity);
        analyzer.base_object = Some(entity);
        let args = [Some(argument)];
        let mut environment = Substitution::new(0, &args);
        assert_eq!(
            analyzer
                .substitute_operand(ty, entity, &mut environment)
                .unwrap(),
            None
        );
        assert_eq!(
            analyzer
                .substitute_array_bound(ty, &mut environment)
                .unwrap(),
            None
        );
        assert!(analyzer.graph.template_operands.substitutions.is_empty());
        assert_eq!(analyzer.template_depth, 4);
        assert_eq!(analyzer.class_depth, 2);
        assert_eq!(analyzer.unevaluated, 3);
        assert_eq!(analyzer.default_argument_scope, Some(scope));
        assert_eq!(analyzer.initializing, Some(entity));
        assert_eq!(analyzer.base_object, Some(entity));
        assert_eq!(analyzer.graph.lookup_limit, None);
    }

    #[test]
    fn declaration_equivalence_preserves_each_operands_lookup_identity() {
        let source = SourceBuffer::new(
            b"long g(double);\
            template<class T>auto f(T x)->decltype(g(x));\
            int g(int);template<class U>auto f(U y)->decltype(g(y));"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"operand-equivalence").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let mut graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.template_operands.patterns.len(), 2);
        let types = graph
            .template_operands
            .patterns
            .keys()
            .map(|&entity| graph.entity(entity).ty.unwrap())
            .collect::<Vec<_>>();
        assert_ne!(
            graph.types.canonical_function(types[0]).unwrap(),
            graph.types.canonical_function(types[1]).unwrap()
        );
        assert_eq!(
            graph.types.declared_type(types[0]).unwrap(),
            graph.types.declared_type(types[1]).unwrap()
        );
        let limits = graph
            .template_operands
            .patterns
            .values()
            .map(|pattern| pattern.limit)
            .collect::<Vec<_>>();
        assert_ne!(limits[0], limits[1]);
        assert!(graph.template_operands.substitutions.is_empty());
    }
}
