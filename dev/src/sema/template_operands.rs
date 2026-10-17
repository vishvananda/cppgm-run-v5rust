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
use crate::{lex::vocabulary::Keyword, syntax::ast};
use std::{
    collections::{HashMap, HashSet, hash_map::RandomState},
    hash::BuildHasher,
};

#[derive(Clone, Copy)]
pub(super) struct Pattern {
    pub value: bool,
    pub expression: ast::ExprId,
    pub scope: ScopeId,
    pub key_dependent: bool,
    pub pack_names: super::pack_names::PackNames,
    home: ScopeId,
    next: Option<EntityId>,
    limit: usize,
    // Known positions below 63, matching the frozen owner's conservative mask.
    // Unknown keys and higher positions retain ordinary substitution.
    parameters: Option<u64>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    entity: EntityId,
    context: Option<EntityId>,
    query: Query,
    depth: u32,
    arguments: Vec<TypeId>,
    selection: Option<Box<ElementSelection>>,
}
#[derive(Clone, PartialEq, Eq, Hash)]
struct ElementSelection {
    elements: Vec<Option<usize>>,
    bound_element: Option<usize>,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Query {
    Type,
    Bound,
    Value,
}
#[derive(Clone, Copy)]
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
    equivalents: HashMap<(ScopeId, Vec<KeyWord>, bool), EntityId>,
    substitutions: HashMap<Key, Evaluation>,
    active: HashSet<(Key, usize)>,
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
        self.dependent_operand_in(expression, scope, fixed, false)
    }
    pub(super) fn dependent_value_operand(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        self.dependent_operand_in(expression, scope, false, true)
    }
    fn dependent_operand_in(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        fixed: bool,
        value: bool,
    ) -> Result<TypeId, Error> {
        self.expression(expression, scope)?;
        let key = self.operand_key(expression, scope)?;
        // Non-dependent operands in semantic mode have their ordinary type.
        if (fixed || value)
            && key.as_ref().is_some_and(|key| !key.dependent)
            && !self.type_dependent_expression(expression, scope)?
        {
            let saved = std::mem::replace(&mut self.template_depth, 0);
            let result = if value {
                self.template_value(expression, scope)
            } else {
                self.decltype_type(expression, scope)
            };
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
                .hash_one((home, words, value))
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
            if pattern.value == value
                && pattern.home == home
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
        // C++ records expression_packs at placeholder formation. Later pack
        // checks consume this definition-owned fact, never a new name lookup.
        let pack_names = self.expression_pack_names(expression, scope)?;
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
                value,
                expression,
                scope,
                key_dependent,
                pack_names,
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
                    .entry((home, equivalent, value))
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
        let query = if self
            .graph
            .template_operands
            .pattern(entity)
            .is_some_and(|pattern| pattern.value)
        {
            Query::Value
        } else {
            Query::Type
        };
        match self.cached_operand(entity, environment, arguments, query)? {
            Some(Evaluation::Type(result)) => self
                .graph
                .types
                .qualify(result, self.graph.types.get(ty).cv)
                .map(Some),
            None => Ok(None),
            Some(Evaluation::Bound(_)) => {
                Err(Error::Invariant("operand type query returned a bound"))
            }
        }
    }

    fn cached_operand(
        &mut self,
        entity: EntityId,
        environment: &mut Substitution<'_>,
        arguments: Vec<TypeId>,
        query: Query,
    ) -> Result<Option<Evaluation>, Error> {
        let key = Key {
            entity,
            context: environment.context,
            query,
            depth: environment.depth,
            arguments,
            selection: environment
                .selection
                .filter(|selection| {
                    selection.bound_element.is_some()
                        || selection.elements.iter().any(Option::is_some)
                })
                .map(|selection| {
                    Box::new(ElementSelection {
                        elements: selection.elements.to_vec(),
                        bound_element: selection.bound_element,
                    })
                }),
        };
        if let Some(&result) = self.graph.template_operands.substitutions.get(&key) {
            return Ok(Some(result));
        }
        // A class definition may re-enter the operand with additional member
        // declarations available, just as it may re-enter a function signature.
        let active = (key, self.graph.class_templates.definitions.active_depth());
        if !self.graph.template_operands.active.insert(active.clone()) {
            return Ok(None);
        }
        let incomplete = self.incomplete_class_observations;
        let result = self.evaluate_operand(entity, environment, query);
        self.graph.template_operands.active.remove(&active);
        let key = active.0;
        match result {
            Ok(result) => {
                if self.incomplete_class_observations == incomplete {
                    self.graph
                        .template_operands
                        .substitutions
                        .insert(key, result);
                }
                Ok(Some(result))
            }
            Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => {
                // Failed lookups may name a class completed later. Keep only
                // successful facts; a failed trial cannot poison later demand.
                Ok(None)
            }
            Err(error) => Err(error),
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
        let arguments = environment
            .arguments
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .ok_or(Error::Invariant("bound query without concrete arguments"))?;
        match self.cached_operand(entity, environment, arguments, Query::Bound)? {
            Some(Evaluation::Bound(bound)) => Ok(Some(bound)),
            Some(Evaluation::Type(_)) => Err(Error::Invariant("bound query returned a type")),
            None => Ok(None),
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
        let copy = self.copy_selected_operand(pattern.expression, scope, environment)?;
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
        let saved_arguments = std::mem::replace(&mut self.template_arguments, 0);
        let saved_default = self.default_argument_scope.take();
        let saved_head = self.template_head.take();
        let saved_friend = self.namespace_friend_class.take();
        let saved_class = std::mem::replace(&mut self.class_depth, 0);
        let saved_initializing = self.initializing.take();
        let saved_base = self.base_object.take();
        let saved_member = self.member_initializer_class.take();
        // Operand substitution is an independent instantiation query. Its
        // pattern names must not mark the expression demanding it dependent.
        let saved_uses = self.dependent_uses.replace(0);
        let result = match query {
            Query::Type => self.decltype_type(copy, scope).map(Evaluation::Type),
            Query::Bound => self.array_bound(copy, scope).map(Evaluation::Bound),
            Query::Value => self.template_value(copy, scope).map(Evaluation::Type),
        };
        self.declaration_access.substitution = saved_access;
        self.declaration_access.function = saved_function;
        self.declaration_access.class = saved_access_class;
        self.body_context = saved_body;
        self.graph.lookup_limit = saved_limit;
        self.template_depth = saved_depth;
        self.unevaluated = saved_unevaluated;
        self.template_arguments = saved_arguments;
        self.default_argument_scope = saved_default;
        self.template_head = saved_head;
        self.namespace_friend_class = saved_friend;
        self.class_depth = saved_class;
        self.initializing = saved_initializing;
        self.base_object = saved_base;
        self.member_initializer_class = saved_member;
        self.dependent_uses.set(saved_uses);
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
        let mut packs = None;
        for index in 0..self.graph.scope(scope).bindings.len() {
            let source = self.graph.scope(scope).bindings[index];
            let binding = self.graph.binding(source);
            if kind == ScopeKind::TemplateParameters {
                if let Some(parameter) = self.graph.entity(binding.entity).template_parameter
                    && let Some(index) = parameter.index(environment.depth)
                    && let Some(Some(ty)) = environment.arguments.get(index)
                {
                    self.bind_template_argument(frame, binding, *ty)?;
                }
            } else if binding.kind == BindingKind::Parameter {
                let skipped = binding.ty.is_some_and(|ty| self.graph.dependent_type(ty))
                    && parameters.is_some_and(|mask| {
                        self.graph
                            .prototype_parameter(binding.entity)
                            .is_some_and(|index| index < 63 && mask & (1u64 << index) == 0)
                    });
                if !skipped
                    && self.substitute_prototype_pack(frame, &mut packs, binding, environment)?
                {
                    continue;
                }
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
        Ok(packs.unwrap_or(frame))
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
    fn successful_bound_queries_reuse_copies_and_keep_type_queries_separate() {
        let source =
            SourceBuffer::new(b"template<class T>int f(int(&)[sizeof(T)]);".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"bound-cache").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let entity = *analyzer
            .graph
            .template_operands
            .patterns
            .keys()
            .next()
            .unwrap();
        let ty = analyzer.graph.entity(entity).ty.unwrap();
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let args = [Some(int)];
        let mut environment = Substitution::new(0, &args);
        assert_eq!(
            analyzer
                .substitute_array_bound(ty, &mut environment)
                .unwrap(),
            Some(4)
        );
        let count = analyzer.tree.expressions.mark();
        for _ in 0..4 {
            assert_eq!(
                analyzer
                    .substitute_array_bound(ty, &mut environment)
                    .unwrap(),
                Some(4)
            );
        }
        assert_eq!(analyzer.tree.expressions.mark(), count);
        let result = analyzer
            .substitute_operand(ty, entity, &mut environment)
            .unwrap()
            .unwrap();
        assert_eq!(analyzer.integral_kind(result).unwrap().bits, 64);
        assert_eq!(analyzer.graph.template_operands.substitutions.len(), 2);
        assert!(analyzer.tree.expressions.mark() > count);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
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
        assert!(graph.template_operands.active.is_empty());
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
        assert!(graph.template_operands.active.is_empty());
        assert_eq!(tree.expressions.mark(), count + 6);
        let pattern = graph.template_operands.patterns.values().next().unwrap();
        assert!(graph.expression_facts.get(pattern.expression).is_err());
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn invalid_operand_removes_in_progress_state_and_restores_caller() {
        let source =
            SourceBuffer::new(b"template<class T>auto f(T x)->decltype(T(),x.missing());".to_vec())
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
        analyzer.dependent_uses.set(17);
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
        assert!(analyzer.graph.template_operands.active.is_empty());
        assert_eq!(analyzer.dependent_uses.get(), 17);
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

#[cfg(test)]
mod value_tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
        types::FundamentalType,
    };

    #[test]
    fn same_operand_has_separate_type_and_value_queries() {
        let source = SourceBuffer::new(b"template<int N>struct A;template<int N>auto f()->A<N+1>*;template<int N>auto g()->decltype(N+1);".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"operand-modes").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let patterns = analyzer
            .graph
            .template_operands
            .patterns
            .iter()
            .map(|(&id, pattern)| (id, pattern.value))
            .collect::<Vec<_>>();
        assert_eq!(patterns.len(), 2);
        assert_ne!(patterns[0].1, patterns[1].1);
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let value = analyzer
            .graph
            .types
            .intern(TypeKind::Value { ty: int, bits: 3 }, Cv::default())
            .unwrap();
        let arguments = [Some(value)];
        for &(entity, is_value) in &patterns {
            let ty = analyzer.graph.entity(entity).ty.unwrap();
            let mut environment = Substitution::new(0, &arguments);
            let result = analyzer
                .substitute_operand(ty, entity, &mut environment)
                .unwrap()
                .unwrap();
            if is_value {
                assert!(
                    matches!(analyzer.graph.types.get(result).kind, TypeKind::Value { ty, bits: 4 } if ty == int)
                );
            } else {
                assert_eq!(result, int);
            }
            let count = analyzer.tree.expressions.mark();
            let mut environment = Substitution::new(0, &arguments);
            assert_eq!(
                analyzer
                    .substitute_operand(ty, entity, &mut environment)
                    .unwrap(),
                Some(result)
            );
            assert_eq!(analyzer.tree.expressions.mark(), count);
            let pattern = analyzer.graph.template_operands.pattern(entity).unwrap();
            assert!(
                analyzer
                    .graph
                    .expression_facts
                    .get(pattern.expression)
                    .is_err()
            );
        }
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.graph.template_operands.active.is_empty());
    }
}
