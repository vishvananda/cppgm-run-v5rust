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
    head_depth: HeadDepth,
}

#[derive(Clone, Copy)]
enum HeadDepth {
    None,
    One(u32),
    Several,
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
// Each pending entity owns one immutable level; earlier levels are shared by
// EntityId. No visible environment or expression tree is copied to retain it.
#[derive(Clone)]
struct Pending {
    placeholder: EntityId,
    depth: u32,
    arguments: Vec<TypeId>,
    earlier: Option<EntityId>,
}
struct EquivalentOperand {
    words: Vec<KeyWord>,
    dependent: bool,
    shared: bool,
    found: Option<TypeId>,
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
    pending: HashMap<EntityId, Pending>,
}
impl Operands {
    pub(super) fn count(&self) -> usize {
        // C++ placeholders_made_ counts formation, not pending_value records.
        // Parameter clauses use it to retain the operand's prototype scope.
        self.patterns.len() - self.pending.len()
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
        if fixed && !value && self.graph.class_templates.definitions.active_depth() != 0 {
            self.check_fixed_decltype_operands(expression, scope, 0)?;
        }
        let key_dependent = key.as_ref().is_some_and(|key| key.dependent);
        let words = key.map(|key| key.words);
        let parameters = words
            .as_deref()
            .and_then(|words| self.operand_parameters(words));
        let mut home = self.operand_home.unwrap_or(scope);
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
        let head_depth = self.operand_head_depth(scope);
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
                head_depth,
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
        if self.graph.template_operands.pending.contains_key(&entity) {
            return self.substitute_pending_operand(ty, entity, environment);
        }
        let Some(arguments) = environment
            .arguments
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(Some(ty));
        };
        let effective = self.effective_operand_arguments(environment)?;
        if effective.iter().enumerate().all(|(index, &argument)| {
            self.own_operand_parameter(argument, environment.depth, index)
        }) && effective
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

    fn effective_operand_arguments(
        &self,
        environment: &Substitution<'_>,
    ) -> Result<Vec<TypeId>, Error> {
        environment
            .arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let ty = argument.ok_or(Error::Invariant("operand without known argument"))?;
                if let Some(position) = environment
                    .selection
                    .and_then(|selection| selection.elements.get(index))
                    .copied()
                    .flatten()
                    && let TypeKind::Pack(elements) = &self.graph.types.get(ty).kind
                {
                    return elements
                        .get(position)
                        .copied()
                        .ok_or(Error::Invariant("operand pack element outside pack"));
                }
                Ok(ty)
            })
            .collect()
    }

    fn own_operand_parameter(&self, mut argument: TypeId, depth: u32, index: usize) -> bool {
        let mut expansion = false;
        if let TypeKind::Pack(elements) = &self.graph.types.get(argument).kind
            && let [element] = elements.as_slice()
            && let TypeKind::Expansion(element) = self.graph.types.get(*element).kind
        {
            expansion = true;
            argument = element;
        }
        let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(argument).kind else {
            return false;
        };
        self.graph
            .entity(entity)
            .template_parameter
            .is_some_and(|info| info.index(depth) == Some(index) && info.is_pack() == expansion)
    }

    fn operand_declares_depth(&self, scope: ScopeId, depth: u32) -> bool {
        self.graph.scope(scope).kind == ScopeKind::TemplateParameters
            && self.graph.scope(scope).bindings.iter().any(|&id| {
                let binding = self.graph.binding(id);
                matches!(binding.kind, BindingKind::Type | BindingKind::TemplateValue)
                    && self
                        .graph
                        .entity(binding.entity)
                        .template_parameter
                        .is_some_and(|info| info.depth() == depth)
            })
    }

    fn operand_open_depth(&self, entity: EntityId, depth: u32) -> bool {
        let Some(pattern) = self.graph.template_operands.pattern(entity) else {
            return false;
        };
        match pattern.head_depth {
            HeadDepth::None => return false,
            HeadDepth::One(declared) => {
                let mut covered = declared == depth;
                let mut pending = self.graph.template_operands.pending.get(&entity);
                while let Some(level) = pending.filter(|_| !covered) {
                    covered |= level.depth == declared;
                    pending = level
                        .earlier
                        .and_then(|id| self.graph.template_operands.pending.get(&id));
                }
                return !covered;
            }
            HeadDepth::Several => (),
        }
        let mut scope = self
            .graph
            .template_operands
            .pattern(entity)
            .map(|pattern| pattern.scope);
        while let Some(current) = scope {
            let node = self.graph.scope(current);
            if node.kind == ScopeKind::TemplateParameters {
                for &id in &node.bindings {
                    let binding = self.graph.binding(id);
                    if !matches!(binding.kind, BindingKind::Type | BindingKind::TemplateValue) {
                        continue;
                    }
                    let Some(info) = self.graph.entity(binding.entity).template_parameter else {
                        continue;
                    };
                    let mut covered = info.depth() == depth;
                    let mut pending = self.graph.template_operands.pending.get(&entity);
                    while let Some(level) = pending {
                        covered |= level.depth == info.depth();
                        pending = level
                            .earlier
                            .and_then(|id| self.graph.template_operands.pending.get(&id));
                    }
                    if !covered {
                        return true;
                    }
                }
            }
            scope = node.parent;
        }
        false
    }

    fn operand_head_depth(&self, mut scope: ScopeId) -> HeadDepth {
        let mut result = HeadDepth::None;
        loop {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::TemplateParameters {
                for &id in &node.bindings {
                    let binding = self.graph.binding(id);
                    if !matches!(binding.kind, BindingKind::Type | BindingKind::TemplateValue) {
                        continue;
                    }
                    let Some(info) = self.graph.entity(binding.entity).template_parameter else {
                        continue;
                    };
                    match result {
                        HeadDepth::None => result = HeadDepth::One(info.depth()),
                        HeadDepth::One(depth) if depth == info.depth() => (),
                        HeadDepth::One(_) | HeadDepth::Several => return HeadDepth::Several,
                    }
                }
            }
            let Some(parent) = node.parent else {
                return result;
            };
            scope = parent;
        }
    }

    fn pending_operand(
        &mut self,
        entity: EntityId,
        depth: u32,
        arguments: Vec<TypeId>,
    ) -> Result<TypeId, Error> {
        let earlier = self.graph.template_operands.pending.get(&entity);
        let placeholder = earlier.map_or(entity, |level| level.placeholder);
        let earlier = earlier.map(|_| entity);
        let pattern = self
            .graph
            .template_operands
            .pattern(placeholder)
            .ok_or(Error::Invariant("pending operand without pattern"))?;
        let equivalent = if earlier.is_none()
            && arguments
                .iter()
                .any(|&ty| self.exception_dependent_type(ty))
        {
            self.equivalent_operand(placeholder, depth, &arguments)?
        } else {
            None
        };
        if let Some(ty) = equivalent.as_ref().and_then(|key| key.found) {
            return Ok(ty);
        }
        let mut named = Vec::new();
        for &argument in &arguments {
            if !self
                .type_pack_pattern(argument, true)?
                .parameters
                .is_empty()
            {
                let ty = if let TypeKind::Pack(elements) = &self.graph.types.get(argument).kind
                    && let [element] = elements.as_slice()
                    && let TypeKind::Expansion(element) = self.graph.types.get(*element).kind
                {
                    element
                } else {
                    argument
                };
                named.push(ty);
            }
        }
        let pack_names = match pattern.pack_names {
            super::pack_names::PackNames::Known(packs) => {
                if let Some(packs) = packs {
                    let TypeKind::Pack(packs) = &self.graph.types.get(packs).kind else {
                        return Err(Error::Invariant("operand names without pack"));
                    };
                    for &pack in packs {
                        let TypeKind::TemplateParameter {
                            entity: parameter, ..
                        } = self.graph.types.get(pack).kind
                        else {
                            return Err(Error::Invariant("operand names without parameter"));
                        };
                        let info = self
                            .graph
                            .entity(parameter)
                            .template_parameter
                            .ok_or(Error::Invariant("operand pack without position"))?;
                        let mut bound = info.depth() == depth;
                        let mut level =
                            earlier.and_then(|id| self.graph.template_operands.pending.get(&id));
                        while let Some(pending) = level {
                            bound |= pending.depth == info.depth();
                            level = pending
                                .earlier
                                .and_then(|id| self.graph.template_operands.pending.get(&id));
                        }
                        if !bound {
                            named.push(pack);
                        }
                    }
                }
                super::pack_names::PackNames::Known(if named.is_empty() {
                    None
                } else {
                    Some(
                        self.graph
                            .types
                            .intern(TypeKind::Pack(named), Cv::default())?,
                    )
                })
            }
            other @ super::pack_names::PackNames::Unsupported(_) => other,
        };
        let made = self
            .graph
            .add_entity(None, Some(self.decltype_name), Keyword::Typename)?;
        let ty = self.graph.types.intern(
            TypeKind::TemplateParameter {
                entity: made,
                template: false,
            },
            Cv::default(),
        )?;
        self.graph.entity_mut(made).ty = Some(ty);
        let mut made_pattern = Pattern {
            pack_names,
            next: None,
            ..pattern
        };
        let words = if let Some(key) = equivalent {
            made_pattern.key_dependent = key.dependent;
            if key.shared {
                let fingerprint = self.graph.template_operands.fingerprints.hash_one((
                    pattern.home,
                    &key.words,
                    pattern.value,
                ));
                made_pattern.next = self.graph.template_operands.index.insert(fingerprint, made);
            }
            Some(key.words)
        } else {
            self.graph
                .template_operands
                .words
                .get(&placeholder)
                .cloned()
        };
        self.graph
            .template_operands
            .patterns
            .insert(made, made_pattern);
        if let Some(words) = words {
            self.graph.template_operands.words.insert(made, words);
        }
        self.graph.template_operands.pending.insert(
            made,
            Pending {
                placeholder,
                depth,
                arguments,
                earlier,
            },
        );
        Ok(ty)
    }

    fn equivalent_operand(
        &mut self,
        entity: EntityId,
        depth: u32,
        arguments: &[TypeId],
    ) -> Result<Option<EquivalentOperand>, Error> {
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("equivalent operand without pattern"))?;
        // C++ only re-keys operands with at most one head depth and no `this`.
        if matches!(pattern.head_depth, HeadDepth::Several)
            || self
                .graph
                .template_operands
                .words(entity)
                .is_some_and(|words| {
                    words.contains(&KeyWord::Node(super::expression_keys::KeyNode::This))
                })
        {
            return Ok(None);
        }
        let arguments = arguments.iter().copied().map(Some).collect::<Vec<_>>();
        let saved_limit = self.graph.lookup_limit.replace(pattern.limit);
        let saved_body = self.body_context.take();
        let saved_depth = std::mem::replace(&mut self.template_depth, 1);
        let saved_uses = self.dependent_uses.replace(0);
        let result = (|| {
            let scope = self.operand_scope(
                pattern.scope,
                &mut Substitution::new(depth, &arguments),
                pattern.parameters,
            )?;
            let copy = self.copy_expression_in_frame(pattern.expression, scope)?;
            self.operand_key(copy, scope)
        })();
        self.graph.lookup_limit = saved_limit;
        self.body_context = saved_body;
        self.template_depth = saved_depth;
        self.dependent_uses.set(saved_uses);
        let key = match result {
            Ok(Some(key)) => key,
            Ok(None) | Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let fingerprint = self.graph.template_operands.fingerprints.hash_one((
            pattern.home,
            &key.words,
            pattern.value,
        ));
        let mut next = self
            .graph
            .template_operands
            .index
            .get(&fingerprint)
            .copied();
        let mut found = None;
        let mut framed = false;
        while let Some(id) = next {
            let other = self
                .graph
                .template_operands
                .pattern(id)
                .ok_or(Error::Invariant("equivalent index without pattern"))?;
            if other.home == pattern.home
                && other.value == pattern.value
                && self.graph.template_operands.words(id) == Some(key.words.as_slice())
            {
                framed = self.graph.template_operands.pending.contains_key(&id);
                let mut scope = Some(other.scope);
                while let Some(current) = scope.filter(|_| !framed) {
                    let node = self.graph.scope(current);
                    framed |=
                        node.kind == ScopeKind::TemplateParameters && node.template_lists == 0;
                    scope = node.parent;
                }
                if !framed {
                    found = self.graph.entity(id).ty;
                }
                break;
            }
            next = other.next;
        }
        let shared = !framed
            && arguments
                .iter()
                .flatten()
                .all(|&ty| !self.operand_names_placeholder(ty, 0));
        Ok(Some(EquivalentOperand {
            words: key.words,
            dependent: key.dependent,
            shared,
            found,
        }))
    }

    fn operand_names_placeholder(&self, ty: TypeId, level: u32) -> bool {
        // The oracle conservatively stops this structural sharing probe at 16.
        if level > 16 {
            return true;
        }
        match &self.graph.types.get(ty).kind {
            TypeKind::TemplateParameter { entity, .. } => {
                self.graph.entity(*entity).template_parameter.is_none()
            }
            TypeKind::Pointer(element)
            | TypeKind::Reference { element, .. }
            | TypeKind::Expansion(element) => self.operand_names_placeholder(*element, level + 1),
            TypeKind::Array {
                element, operand, ..
            } => operand.is_some() || self.operand_names_placeholder(*element, level + 1),
            TypeKind::MemberPointer { owner, element } => {
                self.operand_names_placeholder(*owner, level + 1)
                    || self.operand_names_placeholder(*element, level + 1)
            }
            TypeKind::Function {
                result, parameters, ..
            } => {
                self.operand_names_placeholder(*result, level + 1)
                    || parameters
                        .iter()
                        .any(|&ty| self.operand_names_placeholder(ty, level + 1))
            }
            TypeKind::Pack(elements) => elements
                .iter()
                .any(|&ty| self.operand_names_placeholder(ty, level + 1)),
            TypeKind::Class(entity) => {
                self.graph
                    .class_templates
                    .instance(*entity)
                    .is_some_and(|instance| {
                        instance.dependent
                            && instance
                                .arguments
                                .iter()
                                .any(|&ty| self.operand_names_placeholder(ty, level + 1))
                    })
            }
            _ => false,
        }
    }

    fn substitute_pending_operand(
        &mut self,
        ty: TypeId,
        entity: EntityId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        self.check_stack()?;
        let mut levels = Vec::new();
        let mut next = Some(entity);
        let mut changed = false;
        let mut covered = false;
        while let Some(id) = next {
            let level = self
                .graph
                .template_operands
                .pending
                .get(&id)
                .ok_or(Error::Invariant("missing pending level"))?;
            let (depth, earlier, placeholder, mut arguments) = (
                level.depth,
                level.earlier,
                level.placeholder,
                level.arguments.clone(),
            );
            // This copy is the result buffer, rewritten in place. The
            // immutable recorded level remains shared by other queries.
            for argument in &mut arguments {
                let Some(result) = self.substitute_signature_type(*argument, environment)? else {
                    return Ok(None);
                };
                changed |= result != *argument;
                *argument = result;
            }
            covered |= depth == environment.depth;
            next = earlier;
            levels.push((depth, arguments, placeholder));
        }
        let placeholder = levels
            .last()
            .ok_or(Error::Invariant("pending without levels"))?
            .2;
        if !covered {
            let mut scope = self
                .graph
                .template_operands
                .pattern(placeholder)
                .map(|pattern| pattern.scope);
            let mut declared = false;
            while let Some(current) = scope {
                declared |= self.operand_declares_depth(current, environment.depth);
                scope = self.graph.scope(current).parent;
            }
            if declared
                && !environment.arguments.is_empty()
                && environment.arguments.iter().all(Option::is_some)
            {
                levels.push((
                    environment.depth,
                    self.effective_operand_arguments(environment)?,
                    placeholder,
                ));
                changed = true;
            }
        }
        if !changed {
            return Ok(Some(ty));
        }
        let mut result = self
            .graph
            .entity(placeholder)
            .ty
            .ok_or(Error::Invariant("operand without type"))?;
        for (depth, arguments, _) in levels.into_iter().rev() {
            let TypeKind::TemplateParameter { entity: keyed, .. } =
                self.graph.types.get(result).kind
            else {
                break;
            };
            if !self.graph.template_operands.patterns.contains_key(&keyed) {
                break;
            }
            let arguments = arguments.into_iter().map(Some).collect::<Vec<_>>();
            let mut inner = Substitution::new(depth, &arguments);
            inner.context = environment.context;
            let query = if self
                .graph
                .template_operands
                .pattern(keyed)
                .is_some_and(|pattern| pattern.value)
            {
                Query::Value
            } else {
                Query::Type
            };
            let canonical = arguments
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::Invariant("pending argument absent"))?;
            let Some(Evaluation::Type(value)) =
                self.cached_operand(keyed, &mut inner, canonical, query)?
            else {
                if environment.failure.is_none() {
                    environment.failure = inner.failure;
                    environment.failure_location = inner.failure_location;
                }
                return Ok(None);
            };
            result = value;
            if !matches!(self.graph.types.get(result).kind, TypeKind::TemplateParameter { entity, .. } if self.graph.template_operands.pending.contains_key(&entity))
            {
                break;
            }
        }
        self.graph
            .types
            .qualify(result, self.graph.types.get(ty).cv)
            .map(Some)
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
        let effective = self.effective_operand_arguments(environment)?;
        let dependent = effective
            .iter()
            .any(|&ty| self.exception_dependent_type(ty));
        let result = if dependent {
            self.pending_operand(entity, environment.depth, effective)
                .map(Evaluation::Type)
        } else {
            self.evaluate_operand(entity, environment, query)
                .and_then(|result| {
                    result.map_or_else(
                        || {
                            self.pending_operand(entity, environment.depth, effective)
                                .map(Evaluation::Type)
                        },
                        Ok,
                    )
                })
        };
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
            Err(
                Error::Diagnostic { location, message } | Error::NonConstant { location, message },
            ) => {
                // Failed lookups may name a class completed later. Keep only
                // successful facts; a failed trial cannot poison later demand.
                if environment.failure.is_none() {
                    environment.failure = Some(message);
                    environment.failure_location = location;
                }
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
    ) -> Result<Option<Evaluation>, Error> {
        self.check_stack()?;
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("missing operand"))?;
        let mut scope = pattern.scope;
        let mut earlier = self.graph.template_operands.pending.get(&entity);
        let mut parameters = pattern.parameters;
        while let Some(level) = earlier {
            let (depth, next) = (level.depth, level.earlier);
            let arguments = level
                .arguments
                .iter()
                .copied()
                .map(Some)
                .collect::<Vec<_>>();
            scope =
                self.operand_scope(scope, &mut Substitution::new(depth, &arguments), parameters)?;
            parameters = None;
            earlier = next.and_then(|id| self.graph.template_operands.pending.get(&id));
        }
        let scope = self.operand_scope(scope, environment, parameters)?;
        let open = self.operand_open_depth(entity, environment.depth);
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
        let saved_depth = std::mem::replace(&mut self.template_depth, usize::from(open));
        // A substituted value/bound is potentially evaluated even when the
        // query arose in an unevaluated operand. decltype_type establishes
        // its own unevaluated context; failed value deduction still retains
        // the functions it used, as InstantiationContext does in C++.
        let saved_unevaluated = std::mem::replace(&mut self.unevaluated, 0);
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
            Query::Type if open => Ok(None),
            Query::Type => self
                .decltype_type(copy, scope)
                .map(|ty| Some(Evaluation::Type(ty))),
            Query::Bound => self
                .array_bound(copy, scope)
                .map(|bound| Some(Evaluation::Bound(bound))),
            Query::Value if open => self.constant_value(copy, scope, false).and_then(|value| {
                value
                    .filter(|value| self.integral_kind(value.ty).is_some())
                    .map(|value| {
                        self.graph
                            .types
                            .intern(
                                TypeKind::Value {
                                    ty: value.ty,
                                    bits: value.bits,
                                },
                                Cv::default(),
                            )
                            .map(Evaluation::Type)
                    })
                    .transpose()
            }),
            Query::Value => self
                .template_value(copy, scope)
                .map(|ty| Some(Evaluation::Type(ty))),
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
        if (node.kind != ScopeKind::Function || node.owner.is_some())
            && !self.operand_declares_depth(scope, environment.depth)
        {
            return self.enclosing_operand_scope(scope, environment);
        }
        let kind = node.kind;
        let parent = node
            .parent
            .ok_or(Error::Invariant("operand frame without parent"))?;
        let parent = if kind == ScopeKind::TemplateParameters {
            parent
        } else {
            self.operand_scope(parent, environment, parameters)?
        };
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

    fn enclosing_operand_scope(
        &mut self,
        scope: ScopeId,
        environment: &Substitution<'_>,
    ) -> Result<ScopeId, Error> {
        let mut parameters = Some(scope);
        while let Some(current) = parameters {
            if self.operand_declares_depth(current, environment.depth) {
                break;
            }
            parameters = self.graph.scope(current).parent;
        }
        let Some(parameters) = parameters else {
            return Ok(scope);
        };
        let frame = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, scope, None, None)?;
        for index in 0..self.graph.scope(parameters).bindings.len() {
            let id = self.graph.scope(parameters).bindings[index];
            let binding = self.graph.binding(id);
            if !matches!(binding.kind, BindingKind::Type | BindingKind::TemplateValue) {
                continue;
            }
            let Some(position) = self
                .graph
                .entity(binding.entity)
                .template_parameter
                .and_then(|info| info.index(environment.depth))
            else {
                continue;
            };
            let Some(Some(argument)) = environment.arguments.get(position) else {
                continue;
            };
            let Some(name) = binding.name else {
                continue;
            };
            let found = self.unqualified_declaration_result(
                scope,
                name.into(),
                super::lookup::Filter::Ordinary,
            )?;
            if found.binding == Some(id) {
                self.bind_template_argument(frame, binding, *argument)?;
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
    fn specialized_member_head_forms_types_at_its_own_parameter_depth() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{template<class U,decltype(sizeof(U)) I>int f();};\
              template<>template<class V,decltype(sizeof(V)) J>int A<int>::f(){return J;}\
              int main(){return A<int>().f<char,3>()-3;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"specialized-home").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        // Original depth-one U and the instance's depth-zero U have distinct
        // operand identities. The replacement V shares only the latter.
        assert_eq!(graph.template_operands.patterns.len(), 2);
        assert!(graph.template_operands.active.is_empty());
    }

    #[test]
    fn outside_member_heads_share_operands_only_in_their_declaration_home() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{\
                template<class U,decltype(sizeof(U)) I>int f();\
                template<class U,U I>int f();};\
              template<class X>template<class V,decltype(sizeof(V)) J>\
                int A<X>::f(){return J;}\
              template<class X>template<class V,V J>int A<X>::f(){return J;}\
              template<class T>struct B{\
                template<class U,decltype(sizeof(U)) I>int f();};\
              template<class X>template<class V,decltype(sizeof(V)) J>\
                int B<X>::f(){return J;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-head-home").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let a = cursor.intern_name(b"A").unwrap();
        let b = cursor.intern_name(b"B").unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let syntax_declarations = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut homes = Vec::new();
        for (name, count) in [(a, 2), (b, 1)] {
            let binding = graph
                .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                .unwrap();
            let home = graph.entity(graph.binding(binding).entity).scope.unwrap();
            homes.push(home);
            let functions: Vec<_> = graph.functions_at(home, f.into()).collect();
            assert_eq!(functions.len(), count);
            for binding in functions {
                let entity = graph.binding(binding).entity;
                assert!(graph.entity(entity).defined);
                assert!(!graph.function_templates.body_demanded(entity));
            }
        }
        assert_eq!(graph.template_operands.patterns.len(), 2);
        for pattern in graph.template_operands.patterns.values() {
            assert!(homes.contains(&pattern.home));
            assert_ne!(pattern.scope, pattern.home);
            assert!(graph.expression_facts.get(pattern.expression).is_err());
        }
        assert_eq!(tree.declarations.mark(), syntax_declarations);
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn outside_default_rejection_restores_context_and_keeps_the_first_head() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{template<class U>int f();};\
              template<class T>template<class U=int>int A<T>::f(){return 1;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-default-state").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let a = cursor.intern_name(b"A").unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let declarations = tree.decl_lists.get(declarations).to_vec();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declaration(declarations[0], global).unwrap();
        let binding = analyzer
            .graph
            .own(global, a, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let home = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity)
            .scope
            .unwrap();
        let binding = analyzer.graph.functions_at(home, f.into()).next().unwrap();
        let entity = analyzer.graph.binding(binding).entity;
        let head = analyzer.graph.entity(entity).template_head.unwrap();
        assert!(matches!(
            analyzer.declaration(declarations[1], global),
            Err(Error::Diagnostic {
                message: "a default template argument on a member defined outside its class template",
                ..
            })
        ));
        assert_eq!(analyzer.graph.entity(entity).template_head, Some(head));
        assert!(analyzer.graph.templates.default_argument(head, 0).is_none());
        assert!(!analyzer.graph.entity(entity).defined);
        assert_eq!(analyzer.graph.functions_at(home, f.into()).count(), 1);
        assert!(analyzer.template_head.is_none());
        assert!(analyzer.template_owner.is_none());
        assert!(analyzer.operand_home.is_none());
        assert_eq!(analyzer.template_depth, 0);
    }

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

#[cfg(test)]
mod pending_tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
        types::FundamentalType,
    };

    #[test]
    fn pending_arguments_rebind_and_memoize_without_mutating_the_pattern() {
        let source = SourceBuffer::new(b"template<int>struct I;template<int A,int B>using sub=I<A-B>;template<int N>struct Other;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"pending-values").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let other = cursor.intern_name(b"Other").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let entity = *analyzer
            .graph
            .template_operands
            .patterns
            .keys()
            .next()
            .unwrap();
        let pattern = analyzer.graph.template_operands.pattern(entity).unwrap();
        let ty = analyzer.graph.entity(entity).ty.unwrap();
        let other = analyzer
            .graph
            .own(global, other, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let head = analyzer
            .graph
            .entity(analyzer.graph.binding(other).entity)
            .template_head
            .unwrap();
        let parameter = analyzer.graph.templates.parameters(head)[0];
        let parameter = analyzer.graph.entity(parameter).ty.unwrap();
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let one = analyzer
            .graph
            .types
            .intern(TypeKind::Value { ty: int, bits: 1 }, Cv::default())
            .unwrap();
        let args = [Some(parameter), Some(one)];
        let mut environment = Substitution::new(0, &args);
        let formed = analyzer.graph.template_operands.count();
        let pending = analyzer
            .substitute_operand(ty, entity, &mut environment)
            .unwrap()
            .unwrap();
        let TypeKind::TemplateParameter {
            entity: pending_entity,
            ..
        } = analyzer.graph.types.get(pending).kind
        else {
            panic!("expected a pending operand");
        };
        let saved = analyzer.graph.template_operands.pending[&pending_entity]
            .arguments
            .clone();
        assert_eq!(analyzer.graph.template_operands.count(), formed);
        assert_eq!(saved, [parameter, one]);
        for value in [3, 4, 3] {
            let argument = analyzer
                .graph
                .types
                .intern(
                    TypeKind::Value {
                        ty: int,
                        bits: value,
                    },
                    Cv::default(),
                )
                .unwrap();
            let args = [Some(argument)];
            let mut environment = Substitution::new(0, &args);
            let result = analyzer
                .substitute_operand(pending, pending_entity, &mut environment)
                .unwrap()
                .unwrap();
            assert!(
                matches!(analyzer.graph.types.get(result).kind, TypeKind::Value { ty, bits } if ty == int && bits == value-1)
            );
            let expressions = analyzer.tree.expressions.mark();
            for _ in 0..3 {
                let mut environment = Substitution::new(0, &args);
                assert_eq!(
                    analyzer
                        .substitute_operand(pending, pending_entity, &mut environment)
                        .unwrap(),
                    Some(result)
                );
                assert_eq!(analyzer.tree.expressions.mark(), expressions);
            }
        }
        assert_eq!(
            analyzer.graph.template_operands.pending[&pending_entity].arguments,
            saved
        );
        assert_eq!(
            analyzer
                .graph
                .template_operands
                .pattern(entity)
                .unwrap()
                .scope,
            pattern.scope
        );
        assert!(
            analyzer
                .graph
                .expression_facts
                .get(pattern.expression)
                .is_err()
        );
        assert!(analyzer.graph.template_operands.active.is_empty());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }
}
