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
use crate::support::hash::BuildHasher as Fingerprints;
use crate::support::hash::HashMap;
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    syntax::ast,
};
use std::hash::BuildHasher;

#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent facts retained by the frozen placeholder owner"
)]
#[derive(Clone, Copy)]
pub(super) struct Pattern {
    pub value: bool,
    // argument_target supplies only a concrete address target. It belongs to
    // placeholder identity as well as later overload selection.
    value_target: Option<TypeId>,
    fixed: bool,
    fixed_value: Option<FixedValue>,
    pub expression: ast::ExprId,
    pub scope: ScopeId,
    pub(super) object: Option<TypeId>,
    pub key_dependent: bool,
    pub pack_names: Option<TypeId>,
    pub sizeof_packs: bool,
    pub nested_packs: Option<TypeId>,
    home: ScopeId,
    next: Option<EntityId>,
    limit: usize,
    // Known positions below 63, matching the frozen owner's conservative mask.
    // Unknown keys and higher positions retain ordinary substitution.
    parameters: Option<u64>,
    head_depth: HeadDepth,
    named_value_parameter: Option<EntityId>,
}

#[derive(Clone, Copy)]
enum HeadDepth {
    None,
    One(u32),
    Several,
}
#[derive(Clone, Copy)]
struct FixedValue {
    target: Option<TypeId>,
    value: TypeId,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    entity: EntityId,
    context: Option<EntityId>,
    depth: u32,
    arguments: Vec<TypeId>,
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
    Value,
}
#[derive(Clone, Copy)]
enum Evaluation {
    Type(TypeId),
}
#[derive(Clone, Copy)]
enum CachedOperand {
    Valid(TypeId),
    Invalid,
}
#[derive(Clone, Copy)]
struct OperandInput<'a> {
    arguments: &'a [Option<TypeId>],
    elements: Option<&'a [Option<usize>]>,
    access: Option<EntityId>,
}
enum OperandOutcome {
    EarlyFailure(Option<super::substitution_state::Failure>),
    Evaluated {
        value: Option<Evaluation>,
        invalid: Option<super::substitution_state::Failure>,
    },
}
#[derive(Clone, Copy)]
enum OperandKind {
    Type {
        fixed: bool,
    },
    Value {
        target: Option<TypeId>,
        unknown: bool,
    },
}
#[derive(Default)]
pub(super) struct Operands {
    index: HashMap<u64, EntityId>,
    words: HashMap<EntityId, Vec<KeyWord>>,
    fingerprints: Fingerprints,
    patterns: HashMap<EntityId, Pattern>,
    equivalents: HashMap<(ScopeId, Vec<KeyWord>, bool, Option<TypeId>), EntityId>,
    substitutions: HashMap<Key, CachedOperand>,
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
        self.dependent_operand_in(expression, scope, OperandKind::Type { fixed })
    }
    pub(super) fn dependent_value_operand(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        target: Option<TypeId>,
        unknown: bool,
    ) -> Result<TypeId, Error> {
        self.dependent_operand_in(expression, scope, OperandKind::Value { target, unknown })
    }
    fn dependent_operand_in(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        kind: OperandKind,
    ) -> Result<TypeId, Error> {
        let (fixed, value, target, unknown) = match kind {
            OperandKind::Type { fixed } => (fixed, false, None, false),
            OperandKind::Value { target, unknown } => (false, true, target, unknown),
        };
        let value_target = target.filter(|&ty| self.graph.address_parameter_type(ty));
        self.expression(expression, scope)?;
        // placeholder_words leaves a function body's operands private, even
        // when their written form is nondependent. Each demanded body copy
        // analyzes them in its own environment; they are never signature keys.
        let enclosing_sizeof =
            std::mem::replace(&mut self.graph.analysis.keyed_sizeof_packs, false);
        let keyed = if self.operand_in_body(scope) {
            Ok(None)
        } else {
            self.operand_key(expression, scope)
        };
        let sizeof_packs = self.graph.analysis.keyed_sizeof_packs || matches!(keyed, Ok(None));
        self.graph.analysis.keyed_sizeof_packs = enclosing_sizeof || sizeof_packs;
        let key = keyed?;
        let is_fixed = (fixed || value && self.semantic())
            && key.as_ref().is_some_and(|key| !key.dependent)
            && !self.type_dependent_expression(expression, scope)?;
        // Fixed decltypes are ordinary types. Fixed values retain their
        // placeholder's last target/value, as later structural keying may
        // reuse an object designation formed for an unknown parameter.
        if is_fixed && !value {
            let saved = std::mem::replace(&mut self.template_depth, 0);
            let result = self.decltype_type(expression, scope);
            self.template_depth = saved;
            return result;
        }
        // After semantic fixed-decltype typing, the frozen types-only key
        // rule also collapses a transform with a concrete type operand.
        if !value
            && let Some(key) = key.as_ref().filter(|key| !key.dependent)
            && let [
                KeyWord::Transform(transform),
                KeyWord::Count(1),
                KeyWord::Type(ty),
            ] = key.words.as_slice()
        {
            return self.transformed_type(
                *transform,
                *ty,
                self.tree.expressions.get(expression).location,
            );
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
                .hash_one((home, words, value, value_target))
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
                && pattern.value_target == value_target
                && pattern.home == home
                && self.graph.template_operands.words.get(&entity) == words.as_ref()
            {
                self.graph.analysis.keyed_sizeof_packs = enclosing_sizeof || pattern.sizeof_packs;
                if pattern.fixed {
                    return self.fixed_value_operand(entity, expression, scope, target, unknown);
                }
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
        let (pack_names, nested_packs) = self.operand_pack_names(expression, scope)?;
        let sizeof_packs = sizeof_packs || nested_packs.is_some();
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
        let object = self.current_object_type()?;
        let named_value_parameter = self.operand_named_value_parameter(expression, scope)?;
        self.graph.template_operands.patterns.insert(
            entity,
            Pattern {
                value,
                value_target,
                fixed: is_fixed,
                fixed_value: None,
                expression,
                scope,
                object,
                key_dependent,
                pack_names,
                sizeof_packs,
                nested_packs,
                home,
                next: head,
                limit: self.graph.binding_limit(),
                parameters,
                head_depth,
                named_value_parameter,
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
                    .entry((home, equivalent, value, value_target))
                    .or_insert(entity);
                if earlier != entity {
                    self.graph
                        .types
                        .record_operand_representative(entity, earlier);
                }
            }
            self.graph.template_operands.words.insert(entity, words);
        }
        if is_fixed {
            self.fixed_value_operand(entity, expression, scope, target, unknown)
        } else {
            Ok(ty)
        }
    }

    fn operand_named_value_parameter(
        &self,
        mut expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<EntityId>, Error> {
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(expression).kind {
            expression = inner;
        }
        let ast::ExprKind::Id(name) = self.tree.expressions.get(expression).kind else {
            return Ok(None);
        };
        let name = self.tree.names.get(name);
        let components = self.tree.name_lists.get(name.components);
        let [component] = components else {
            return Ok(None);
        };
        if name.global || component.arguments.is_some() {
            return Ok(None);
        }
        let ast::NameKind::Identifier(name) = component.kind else {
            return Ok(None);
        };
        let found = self.unqualified_declaration_result(
            scope,
            name.into(),
            super::lookup::Filter::Ordinary,
        )?;
        let Some(binding) = found.binding.filter(|_| !found.ambiguous) else {
            return Ok(None);
        };
        let binding = self.graph.binding(binding);
        if binding.kind != BindingKind::TemplateValue {
            return Ok(None);
        }
        let ty = self
            .graph
            .entity(binding.entity)
            .ty
            .ok_or(Error::Invariant("named template value without type"))?;
        let TypeKind::TemplateParameter { entity, .. } = self
            .graph
            .types
            .get(self.graph.types.canonical_view(ty))
            .kind
        else {
            return Err(Error::Invariant("named template value without parameter"));
        };
        Ok(Some(entity))
    }

    fn operand_in_body(&self, mut scope: ScopeId) -> bool {
        loop {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::TemplateParameters {
                return false;
            }
            if node.kind == ScopeKind::Block
                || node.kind == ScopeKind::Function && node.owner.is_some()
            {
                return true;
            }
            let Some(parent) = node.parent else {
                return false;
            };
            scope = parent;
        }
    }

    fn fixed_value_operand(
        &mut self,
        entity: EntityId,
        expression: ast::ExprId,
        scope: ScopeId,
        target: Option<TypeId>,
        unknown: bool,
    ) -> Result<TypeId, Error> {
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("fixed value without pattern"))?;
        if !unknown && let Some(value) = pattern.fixed_value.filter(|value| value.target == target)
        {
            return Ok(value.value);
        }
        let saved = std::mem::replace(&mut self.template_depth, 0);
        let result = self.template_value(expression, scope, target, unknown);
        self.template_depth = saved;
        let value = result?;
        self.graph
            .template_operands
            .patterns
            .get_mut(&entity)
            .ok_or(Error::Invariant("fixed value lost its pattern"))?
            .fixed_value = Some(FixedValue { target, value });
        Ok(value)
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
        if environment.arguments.iter().any(Option::is_none) {
            return Ok(Some(ty));
        }
        let effective = self.canonical_operand_arguments(environment)?;
        if effective.iter().enumerate().all(|(index, &argument)| {
            self.own_operand_parameter(argument, environment.depth, index)
        }) && effective
            .iter()
            .any(|&ty| self.exception_dependent_type(ty))
        {
            return Ok(Some(ty));
        }
        match self.cached_operand_with_arguments(entity, environment, effective)? {
            Some(Evaluation::Type(result)) => self
                .graph
                .types
                .qualify(result, self.graph.types.get(ty).cv)
                .map(Some),
            None => Ok(None),
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
            .context()
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
                        .context()
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
                    let Some(info) = self
                        .graph
                        .entity(binding.entity)
                        .context()
                        .template_parameter
                    else {
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
                    let Some(info) = self
                        .graph
                        .entity(binding.entity)
                        .context()
                        .template_parameter
                    else {
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
            if !self.type_pack_pattern(argument, true).parameters.is_empty() {
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
        if let Some(packs) = pattern.pack_names {
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
                    .context()
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
        let pack_names = if named.is_empty() {
            None
        } else {
            Some(
                self.graph
                    .types
                    .intern(TypeKind::Pack(named), Cv::default())?,
            )
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
                    pattern.value_target,
                ));
                made_pattern.next = self.graph.template_operands.index.insert(fingerprint, made);
            }
            Some(key.words)
        } else {
            // A pending substitution owns its recorded levels, not the
            // original operand's structural key. C++ pending_value leaves
            // key unset unless equivalent_placeholder produced a key for
            // this first dependent level. Later levels cannot inherit it.
            made_pattern.key_dependent = false;
            None
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
        if pattern.object.is_some()
            || matches!(pattern.head_depth, HeadDepth::Several)
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
        let owner = self.graph.analysis.substitution.current().owner;
        // This is the reference's raw speculative scope, with NO direct soft
        // failure charge. Nested ordinary scopes retain their own accounting.
        let result = self.with_raw_substitution_soft(owner, |analyzer| {
            let scope = analyzer.operand_scope(
                pattern.scope,
                &mut analyzer.substitution(depth, &arguments),
                pattern.parameters,
            )?;
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
            let copy = analyzer.copy_expression_in_frame(pattern.expression, scope)?;
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
            let key = analyzer.operand_key(copy, scope)?;
            Ok(
                if analyzer
                    .graph
                    .analysis
                    .substitution
                    .current()
                    .failure
                    .is_some()
                {
                    None
                } else {
                    key
                },
            )
        });
        self.graph.lookup_limit = saved_limit;
        self.body_context = saved_body;
        self.template_depth = saved_depth;
        self.dependent_uses.set(saved_uses);
        let key = match result {
            Ok(Some(key)) => key,
            Ok(None) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let fingerprint = self.graph.template_operands.fingerprints.hash_one((
            pattern.home,
            &key.words,
            pattern.value,
            pattern.value_target,
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
                && other.value_target == pattern.value_target
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
            TypeKind::TemplateParameter { entity, .. } => self
                .graph
                .entity(*entity)
                .context()
                .template_parameter
                .is_none(),
            TypeKind::Pointer(element)
            | TypeKind::Reference { element, .. }
            | TypeKind::Expansion(element) => self.operand_names_placeholder(*element, level + 1),
            TypeKind::Array {
                element, operand, ..
            }
            | TypeKind::Vector {
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
                let result = self.graph.types.canonical_function(result)?;
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
                    self.canonical_operand_arguments(environment)?,
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
            let mut inner = self.substitution(depth, &arguments);
            inner.context = environment.context;
            let Some(Evaluation::Type(value)) = self.cached_operand(keyed, &mut inner)? else {
                environment.carry(inner.failure);
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

    fn operand_access_context(&self) -> Option<EntityId> {
        let state = self.graph.analysis.substitution.current();
        if state.soft != 0 { state.owner } else { None }
    }

    fn cached_operand(
        &mut self,
        entity: EntityId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<Evaluation>, Error> {
        let canonical = self.canonical_operand_arguments(environment)?;
        self.cached_operand_with_arguments(entity, environment, canonical)
    }

    fn canonical_operand_arguments(
        &mut self,
        environment: &Substitution<'_>,
    ) -> Result<Vec<TypeId>, Error> {
        self.effective_operand_arguments(environment)?
            .into_iter()
            .map(|ty| self.graph.types.canonical_function(ty))
            .collect()
    }

    fn cached_operand_with_arguments(
        &mut self,
        entity: EntityId,
        environment: &mut Substitution<'_>,
        canonical: Vec<TypeId>,
    ) -> Result<Option<Evaluation>, Error> {
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("missing cached operand"))?;
        let dependent = canonical
            .iter()
            .any(|&ty| self.exception_dependent_type(ty));
        let mut arguments = canonical.clone();
        let mut selected_packs = false;
        if pattern.sizeof_packs
            && !dependent
            && let Some(selection) = environment.selection
        {
            for (index, argument) in environment.arguments.iter().enumerate() {
                if selection.elements.get(index).is_some_and(Option::is_some)
                    && let Some(argument) = argument
                    && matches!(self.graph.types.get(*argument).kind, TypeKind::Pack(_))
                {
                    arguments.push(self.graph.types.canonical_function(*argument)?);
                    selected_packs = true;
                }
            }
        }
        let key = Key {
            entity,
            context: self.operand_access_context(),
            depth: environment.depth,
            arguments,
        };
        if let Some(&known) = self.graph.template_operands.substitutions.get(&key) {
            return match known {
                CachedOperand::Valid(ty) => Ok(Some(Evaluation::Type(ty))),
                CachedOperand::Invalid => environment.reject_at(
                    self.tree.expressions.get(pattern.expression).location,
                    "an invalid expression in a substitution",
                ),
            };
        }
        let incomplete = self.incomplete_class_observations;
        let result = (|| {
            let outcome = if dependent {
                OperandOutcome::Evaluated {
                    value: None,
                    invalid: None,
                }
            } else {
                let effective = canonical.iter().copied().map(Some).collect::<Vec<_>>();
                let mut scoped = effective.clone();
                let elements = environment
                    .selection
                    .filter(|_| selected_packs)
                    .map(|selection| selection.elements);
                if let Some(elements) = elements {
                    scoped = self.whole_operand_arguments(
                        environment,
                        elements,
                        &key.arguments,
                        canonical.len(),
                    );
                }
                let mut inner = self.substitution(environment.depth, &effective);
                inner.context = environment.context;
                let result = self.evaluate_operand(
                    entity,
                    &mut inner,
                    if pattern.value {
                        Query::Value
                    } else {
                        Query::Type
                    },
                    OperandInput {
                        arguments: &scoped,
                        elements,
                        access: key.context,
                    },
                );
                environment.carry(inner.failure);
                result?
            };
            let memoized = self.incomplete_class_observations == incomplete;
            let outcome = if let OperandOutcome::Evaluated {
                value: None,
                invalid: None,
            } = outcome
            {
                let value = self.pending_operand(entity, environment.depth, canonical)?;
                OperandOutcome::Evaluated {
                    value: Some(Evaluation::Type(value)),
                    invalid: None,
                }
            } else {
                outcome
            };
            Ok((outcome, memoized))
        })();
        match result {
            Ok((
                OperandOutcome::Evaluated {
                    value: Some(Evaluation::Type(value)),
                    invalid: None,
                },
                memoized,
            )) => {
                if memoized {
                    self.graph
                        .template_operands
                        .substitutions
                        .insert(key, CachedOperand::Valid(value));
                }
                Ok(Some(Evaluation::Type(value)))
            }
            Ok((
                OperandOutcome::Evaluated {
                    invalid: Some(failure),
                    ..
                },
                memoized,
            )) => {
                if memoized && !failure.transient {
                    self.graph
                        .template_operands
                        .substitutions
                        .insert(key, CachedOperand::Invalid);
                }
                environment.carry(Some(failure));
                Ok(None)
            }
            Ok((OperandOutcome::EarlyFailure(failure), _)) => {
                environment.carry(failure);
                Ok(None)
            }
            Err(Error::SubstitutionRejected(failure)) => {
                environment.carry(Some(failure));
                Ok(None)
            }
            Err(Error::SubstitutionNull) => Ok(None),
            Ok((
                OperandOutcome::Evaluated {
                    value: None,
                    invalid: None,
                },
                _,
            )) => Err(Error::Invariant("operand fallback without a value")),
            Err(error) => Err(error),
        }
    }

    fn whole_operand_arguments(
        &self,
        environment: &Substitution<'_>,
        elements: &[Option<usize>],
        arguments: &[TypeId],
        count: usize,
    ) -> Vec<Option<TypeId>> {
        let mut scoped = arguments[..count]
            .iter()
            .copied()
            .map(Some)
            .collect::<Vec<_>>();
        let mut next = count;
        for (index, argument) in environment.arguments.iter().enumerate() {
            if elements.get(index).is_some_and(Option::is_some)
                && argument
                    .is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Pack(_)))
            {
                scoped[index] = Some(arguments[next]);
                next += 1;
            }
        }
        scoped
    }

    fn nested_operand_pack(
        &self,
        nested: TypeId,
        parameter: TypeId,
        depth: u32,
        index: usize,
    ) -> Result<bool, Error> {
        for &named in self.graph.types.pack_elements(nested)? {
            if named == parameter {
                return Ok(true);
            }
            if !matches!(
                self.graph.types.get(named).kind,
                TypeKind::TemplateParameter { .. }
            ) && self
                .type_pack_pattern(named, true)
                .parameters
                .iter()
                .any(|&entity| {
                    self.graph
                        .entity(entity)
                        .context()
                        .template_parameter
                        .is_some_and(|info| info.index(depth) == Some(index))
                })
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn operand_element_renames(
        &mut self,
        pattern: Pattern,
        depth: u32,
        elements: &[Option<usize>],
        count: usize,
    ) -> Result<(bool, Vec<(NameId, NameId)>), Error> {
        let mut scope = Some(pattern.scope);
        while let Some(current) = scope {
            if self.operand_declares_depth(current, depth) {
                break;
            }
            scope = self.graph.scope(current).parent;
        }
        let mut whole = false;
        let mut renames = Vec::new();
        if let Some(scope) = scope {
            for ordinal in 0..self.graph.scope(scope).bindings.len() {
                let binding = self
                    .graph
                    .binding(self.graph.scope(scope).bindings[ordinal]);
                if !matches!(binding.kind, BindingKind::Type | BindingKind::TemplateValue) {
                    continue;
                }
                let Some(info) = self
                    .graph
                    .entity(binding.entity)
                    .context()
                    .template_parameter
                else {
                    continue;
                };
                let Some(index) = info.index(depth).filter(|&index| index < count) else {
                    continue;
                };
                let Some(element) = elements.get(index).copied().flatten() else {
                    continue;
                };
                let Some(name) = binding.name else {
                    continue;
                };
                let element_name =
                    super::pack_frames::element_name(self.identifiers, name, element)
                        .map_err(Error::Limit)?;
                renames.push((name, element_name));
                if !whole && let Some(nested) = pattern.nested_packs {
                    let parameter = self
                        .graph
                        .entity(binding.entity)
                        .ty
                        .ok_or(Error::Invariant("renamed pack without parameter type"))?;
                    let parameter = self.graph.types.canonical_view(parameter);
                    whole = self.nested_operand_pack(nested, parameter, depth, index)?;
                }
            }
        }
        Ok((whole, renames))
    }

    pub(super) fn substitute_bound_type(
        &mut self,
        ty: TypeId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let effective = environment
            .arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let Some(ty) = *argument else {
                    return Ok(None);
                };
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
                        .map(Some)
                        .ok_or(Error::Invariant("bound pack element outside pack"));
                }
                Ok(Some(ty))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut inner = self.substitution(environment.depth, &effective);
        inner.context = environment.context;
        let result = self.substitute_bound_type_in(ty, &mut inner);
        environment.carry(inner.failure);
        result
    }

    fn substitute_bound_type_in(
        &mut self,
        ty: TypeId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let (element, operand, vector) = match self.graph.types.get(ty).kind {
            TypeKind::Array {
                element,
                operand: Some(operand),
                ..
            } => (element, operand, None),
            TypeKind::Vector {
                element,
                operand: Some(operand),
                bytes,
                ..
            } => (element, operand, Some(bytes)),
            _ => return Err(Error::Invariant("bound type without operand")),
        };
        let cv = self.graph.types.get(ty).cv;
        let Some(mut element) = self.substitute_signature_type(element, environment)? else {
            return Ok(None);
        };
        let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(operand).kind else {
            return Err(Error::Invariant("bound type without placeholder"));
        };
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("bound type without pattern"))?;
        let deferred = environment
            .arguments
            .iter()
            .any(|slot| slot.is_none_or(|ty| self.graph.dependent_type(ty)))
            || vector.is_none()
                && pattern.named_value_parameter.is_some_and(|named| {
                    self.graph
                        .entity(named)
                        .context()
                        .template_parameter
                        .is_some_and(|parameter| parameter.depth() != environment.depth)
                });
        if deferred {
            let kind = if let Some(bytes) = vector {
                TypeKind::Vector {
                    element,
                    bound: 0,
                    operand: Some(operand),
                    bytes,
                }
            } else {
                TypeKind::Array {
                    element,
                    bound: 0,
                    operand: Some(operand),
                }
            };
            return self.graph.types.intern(kind, cv).map(Some);
        }
        let Some((count, location)) = self.substitute_array_bound(operand, environment)? else {
            return Ok(None);
        };
        if let Some(bytes) = vector {
            element = self.bare_type(element)?;
            let bound = match self.checked_vector_bound(element, count, bytes) {
                Ok(bound) => bound,
                Err(message) => return environment.reject_at(location, message),
            };
            self.graph
                .types
                .intern(
                    TypeKind::Vector {
                        element,
                        bound,
                        operand: None,
                        bytes: false,
                    },
                    cv,
                )
                .map(Some)
        } else {
            self.checked_substituted_array(element, count, cv, location, environment)
        }
    }

    pub(super) fn checked_substituted_array(
        &mut self,
        element: TypeId,
        bound: u64,
        cv: Cv,
        location: crate::support::source::SourceOffset,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let failure = match self.graph.types.get(element).kind {
            TypeKind::Reference { .. } => Some("array of references"),
            TypeKind::Function { .. } => Some("array of functions"),
            TypeKind::Scalar(super::types::Scalar::Void) => Some("array of void"),
            TypeKind::Array {
                bound: 0,
                operand: None,
                ..
            } => Some("array of arrays of unknown bound"),
            _ => None,
        };
        if let Some(message) = failure {
            environment.reject_at(location, message)
        } else {
            if self.semantic() && self.template_depth == 0 {
                let mut object = element;
                while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
                    object = element;
                }
                if let TypeKind::Class(class) = self.graph.types.get(object).kind
                    && self.graph.entity(class).complete
                    && self.graph.virtuals.abstract_class(class)
                {
                    return environment.reject_at(location, "an object of an abstract class type");
                }
            }
            self.graph
                .types
                .intern(
                    TypeKind::Array {
                        element,
                        bound,
                        operand: None,
                    },
                    cv,
                )
                .map(Some)
        }
    }

    pub(super) fn substitute_array_bound(
        &mut self,
        operand: TypeId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<(u64, crate::support::source::SourceOffset)>, Error> {
        let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(operand).kind else {
            return Err(Error::Invariant("array bound without operand placeholder"));
        };
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("missing bound operand"))?;
        let scope = self.operand_scope(pattern.scope, environment, pattern.parameters)?;
        let copy = self.copy_expression_in_frame(pattern.expression, scope)?;
        if let Some(failure) = self.graph.analysis.substitution.current().failure {
            environment.carry(Some(failure));
            return Ok(None);
        }
        let location = self.tree.expressions.get(copy).location;
        self.instantiation_context_in(
            self.instantiation_depth,
            location,
            pattern.limit,
            super::template_context::InstantiationMode::Substitution,
            |analyzer| {
                let result = analyzer.checked_array_bound(copy, scope);
                let failure = analyzer.graph.analysis.substitution.current().failure;
                environment.carry(failure);
                match result {
                    Ok(value) if failure.is_none() => Ok(Some((value, location))),
                    Ok(_) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => {
                        Ok(None)
                    }
                    Err(error) => Err(error),
                }
            },
        )
    }

    fn evaluate_operand(
        &mut self,
        entity: EntityId,
        environment: &mut Substitution<'_>,
        query: Query,
        input: OperandInput<'_>,
    ) -> Result<OperandOutcome, Error> {
        self.check_stack()?;
        let pattern = self
            .graph
            .template_operands
            .pattern(entity)
            .ok_or(Error::Invariant("missing operand"))?;
        let (whole_names, renames) = if let Some(elements) = input.elements {
            self.operand_element_renames(
                pattern,
                environment.depth,
                elements,
                environment.arguments.len(),
            )?
        } else {
            (false, Vec::new())
        };
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
                self.operand_scope(scope, &mut self.substitution(depth, &arguments), parameters)?;
            parameters = None;
            earlier = next.and_then(|id| self.graph.template_operands.pending.get(&id));
        }
        let mut scoped = self.substitution(environment.depth, input.arguments);
        scoped.context = environment.context;
        let selection = super::function_templates::PackSelection {
            elements: input.elements.unwrap_or(&[]),
            bound_element: None,
        };
        if input.elements.is_some() && !whole_names {
            scoped.selection = Some(&selection);
        }
        let result = self.operand_scope(scope, &mut scoped, parameters);
        environment.carry(scoped.failure);
        let scope = result?;
        let open = self.operand_open_depth(entity, environment.depth);
        let copy = self.copy_selected_operand(pattern.expression, scope, &renames)?;
        // Scope/copy rejection belongs to the inherited substitution and must
        // be observed before instantiation entry clears its failure record.
        if let Some(failure) = self.graph.analysis.substitution.current().failure {
            return Ok(OperandOutcome::EarlyFailure(Some(failure)));
        }
        let access = input
            .access
            .map(|entity| self.substitution_scope(entity))
            .transpose()?;
        self.instantiation_context_in(
            self.instantiation_depth,
            self.tree.expressions.get(copy).location,
            pattern.limit,
            super::template_context::InstantiationMode::Substitution,
            |analyzer| {
                analyzer.declaration_access.substitution = access.map(|point| (scope, point));
                // This local object stands for the operand's BodyContext. The
                // common instantiation state does not own signature_object.
                let saved_object = analyzer.signature_object.take();
                let result = (|| {
                    let object = pattern
                        .object
                        .map(|ty| analyzer.substitute_signature_type(ty, environment))
                        .transpose()?;
                    if object == Some(None) {
                        return Ok(OperandOutcome::EarlyFailure(
                            analyzer.graph.analysis.substitution.current().failure,
                        ));
                    }
                    analyzer.signature_object = object.flatten();
                    let result = analyzer.evaluate_operand_query(
                        copy,
                        scope,
                        pattern.value_target,
                        query,
                        open,
                    );
                    let value = match result {
                        Ok(value) => value,
                        Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => None,
                        Err(error) => return Err(error),
                    };
                    Ok(OperandOutcome::Evaluated {
                        value,
                        invalid: analyzer.graph.analysis.substitution.current().failure,
                    })
                })();
                analyzer.signature_object = saved_object;
                result
            },
        )
    }

    fn evaluate_operand_query(
        &mut self,
        copy: ast::ExprId,
        scope: ScopeId,
        target: Option<TypeId>,
        query: Query,
        open: bool,
    ) -> Result<Option<Evaluation>, Error> {
        let result = match query {
            Query::Type if open => Ok(None),
            Query::Type => self
                .decltype_type(copy, scope)
                .map(|ty| Some(Evaluation::Type(ty))),
            Query::Value if open => {
                let saved_depth = self.template_depth;
                self.template_depth += 1;
                let result = self.constant_value(copy, scope, false).and_then(|value| {
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
                });
                self.template_depth = saved_depth;
                result
            }
            Query::Value => self
                .template_value(copy, scope, target, false)
                .map(|ty| Some(Evaluation::Type(ty))),
        };
        if !open
            && query == Query::Value
            && (result.is_ok()
                || matches!(
                    result,
                    Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull)
                ))
        {
            self.graph.work.totals.value_substitutions += 1;
        }
        result
    }

    fn bind_operand_argument(
        &mut self,
        frame: ScopeId,
        binding: Binding,
        ty: TypeId,
        index: usize,
        environment: &Substitution<'_>,
    ) -> Result<(), Error> {
        if matches!(self.graph.types.get(ty).kind, TypeKind::Pack(_)) {
            let element = environment
                .selection
                .and_then(|selection| selection.elements.get(index))
                .copied()
                .flatten();
            self.bind_argument_pack_element(frame, binding, ty, element)
        } else {
            self.bind_template_argument(frame, binding, ty)
        }
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
        let substitution = self.graph.analysis.substitution.clone();
        let saved_failure = substitution.current().failure;
        let saved_transport = environment.failure;
        for index in 0..self.graph.scope(scope).bindings.len() {
            let source = self.graph.scope(scope).bindings[index];
            let binding = self.graph.binding(source);
            let result = (|| {
                if kind == ScopeKind::TemplateParameters {
                    if let Some(parameter) = self
                        .graph
                        .entity(binding.entity)
                        .context()
                        .template_parameter
                        && let Some(index) = parameter.index(environment.depth)
                        && let Some(Some(ty)) = environment.arguments.get(index)
                    {
                        self.bind_operand_argument(frame, binding, *ty, index, environment)?;
                    }
                } else if binding.kind == BindingKind::Parameter {
                    let skipped = binding.ty.is_some_and(|ty| self.graph.dependent_type(ty))
                        && parameters.is_some_and(|mask| {
                            self.graph
                                .prototype_parameter(binding.entity)
                                .is_some_and(|index| index < 63 && mask & (1u64 << index) == 0)
                        });
                    if !skipped
                        && self.substitute_prototype_pack(
                            frame,
                            &mut packs,
                            binding,
                            environment,
                        )?
                    {
                        return Ok(());
                    }
                    let ty = if let Some(ty) = binding.ty.filter(|_| !skipped) {
                        match self.substitute_signature_type(ty, environment) {
                            Ok(ty) => ty,
                            Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => None,
                            Err(error) => return Err(error),
                        }
                    } else {
                        None
                    };
                    let entity = self.graph.add_entity(ty, binding.name, binding.key)?;
                    let declared = self.graph.add_binding(
                        frame,
                        None,
                        Binding {
                            entity,
                            ty,
                            ..binding
                        },
                    )?;
                    // Definition-time dependence survives concrete substitution:
                    // ADL in the copied operand may see later associated operators.
                    self.graph.set_binding_dependence(
                        declared,
                        binding.ty.is_none_or(|ty| self.graph.dependent_type(ty)),
                    );
                }
                Ok(())
            })();
            if kind == ScopeKind::Function {
                substitution.replace(super::substitution_state::Ambient {
                    failure: saved_failure,
                    ..substitution.current()
                });
                environment.failure = saved_transport;
            }
            result?;
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
                .context()
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
                if matches!(self.graph.types.get(*argument).kind, TypeKind::Pack(_)) {
                    let element = environment
                        .selection
                        .and_then(|selection| selection.elements.get(position))
                        .copied()
                        .flatten();
                    self.bind_argument_pack_element(frame, binding, *argument, element)?;
                } else {
                    self.bind_template_argument(frame, binding, *argument)?;
                }
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
        let mut visited = crate::support::hash::HashSet::default();
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
    fn incomplete_nested_qualifier_rejects_transiently_then_retries_after_completion() {
        let source = SourceBuffer::new(b"struct Q;struct Wrap{using type=Q;};template<class T>auto f(T)->decltype(T::type::value);char f(...);static_assert(sizeof(f(Wrap{}))==1,\"before\");struct Q{static long value;};static_assert(sizeof(f(Wrap{}))==sizeof(long),\"after\");".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"transient-nested-qualifier").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.work.totals.substitution_failures, 1);
        assert!(graph.analysis.substitution.current().failure.is_none());
    }

    #[test]
    fn body_decltypes_are_private_until_each_specialization_is_demanded() {
        let source = SourceBuffer::new(
            b"template<class T>void f(){\
            decltype(1)a;decltype(1)b;for(auto n:{1,2L}){}}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"body-operands").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let operands = &analyzer.graph.template_operands;
        assert_eq!(operands.patterns.len(), 4);
        assert!(operands.patterns.values().all(|pattern| !pattern.fixed));
        assert!(operands.words.is_empty());
        assert!(operands.substitutions.is_empty());
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn trailing_objects_keep_cv_identity_without_body_or_layout_demand() {
        let source = SourceBuffer::new(
            b"template<class X>struct A{\
              template<class T>auto f(T)->decltype(this){return T::missing;}\
              template<class T>auto f(T)const->decltype(this){return T::missing;}};\
              using I=A<int>;\
              static_assert(sizeof(((I*)0)->f(0))==8,\"\");\
              static_assert(sizeof(((const I*)0)->f(0))==8,\"\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"trailing-object").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let name = cursor.intern_name(b"I").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert!(analyzer.signature_object.is_none());
        assert!(analyzer.body_context.is_none());
        let mut objects = Vec::new();
        for pattern in analyzer.graph.template_operands.patterns.values() {
            if let Some(object) = pattern.object {
                objects.push(object);
            }
        }
        assert!(
            objects
                .iter()
                .any(|&ty| analyzer.graph.types.get(ty).cv == Cv::CONST)
        );
        assert!(
            objects
                .iter()
                .any(|&ty| analyzer.graph.types.get(ty).cv == Cv::default())
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        let binding = analyzer
            .graph
            .own(global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let class = analyzer
            .graph
            .class_of(analyzer.graph.binding(binding).ty.unwrap())
            .unwrap();
        assert!(matches!(
            analyzer.graph.entity(class).class().layout,
            super::super::layout::LayoutState::NotStarted
        ));
        let home = analyzer.graph.entity(class).context().scope.unwrap();
        for binding in analyzer.graph.functions_at(home, f.into()) {
            assert!(
                !analyzer
                    .graph
                    .function_templates
                    .body_demanded(analyzer.graph.binding(binding).entity)
            );
        }
        let syntax = analyzer.tree.expressions.mark();
        let count = analyzer.graph.template_operands.substitutions.len();
        assert!(analyzer.complete_class(class).unwrap());
        assert_eq!(analyzer.tree.expressions.mark(), syntax);
        assert_eq!(analyzer.graph.template_operands.substitutions.len(), count);
    }

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
            let home = graph
                .entity(graph.binding(binding).entity)
                .context()
                .scope
                .unwrap();
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
            .context()
            .scope
            .unwrap();
        let binding = analyzer.graph.functions_at(home, f.into()).next().unwrap();
        let entity = analyzer.graph.binding(binding).entity;
        let head = analyzer
            .graph
            .entity(entity)
            .context()
            .template_head
            .unwrap();
        assert!(matches!(
            analyzer.declaration(declarations[1], global),
            Err(Error::Diagnostic {
                message: "a default template argument on a member defined outside its class template",
                ..
            })
        ));
        assert_eq!(
            analyzer.graph.entity(entity).context().template_head,
            Some(head)
        );
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
    fn successful_bound_queries_repeat_copies_and_keep_type_queries_separate() {
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
        let mut environment = analyzer.substitution(0, &args);
        assert_eq!(
            analyzer
                .substitute_array_bound(ty, &mut environment)
                .unwrap()
                .map(|(value, _)| value),
            Some(4)
        );
        let count = analyzer.tree.expressions.mark();
        for _ in 0..4 {
            assert_eq!(
                analyzer
                    .substitute_array_bound(ty, &mut environment)
                    .unwrap()
                    .map(|(value, _)| value),
                Some(4)
            );
        }
        assert!(analyzer.tree.expressions.mark() > count);
        assert!(analyzer.graph.template_operands.substitutions.is_empty());
        let result = analyzer
            .substitute_operand(ty, entity, &mut environment)
            .unwrap()
            .unwrap();
        assert_eq!(analyzer.integral_kind(result).unwrap().bits, 64);
        assert_eq!(analyzer.graph.template_operands.substitutions.len(), 1);
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
        assert_eq!(tree.expressions.mark(), count + 6);
        let pattern = graph.template_operands.patterns.values().next().unwrap();
        assert!(graph.expression_facts.get(pattern.expression).is_err());
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn invalid_operand_replay_keeps_caller_state_and_bounds_uncached() {
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
        analyzer.signature_object = Some(argument);
        analyzer.dependent_uses.set(17);
        let args = [Some(argument)];
        let mut environment = analyzer.substitution(0, &args);
        assert_eq!(
            analyzer
                .with_substitution_soft(None, |analyzer| analyzer.substitute_operand(
                    ty,
                    entity,
                    &mut environment
                ))
                .unwrap(),
            None
        );
        let copied = analyzer.tree.expressions.mark();
        assert_eq!(
            analyzer
                .with_substitution_soft(None, |analyzer| {
                    let result = analyzer.substitute_operand(ty, entity, &mut environment)?;
                    assert_eq!(
                        analyzer
                            .graph
                            .analysis
                            .substitution
                            .current()
                            .failure
                            .unwrap()
                            .message,
                        "an invalid expression in a substitution"
                    );
                    Ok(result)
                })
                .unwrap(),
            None
        );
        assert_eq!(analyzer.tree.expressions.mark(), copied);
        let original_location = analyzer
            .tree
            .expressions
            .get(
                analyzer
                    .graph
                    .template_operands
                    .pattern(entity)
                    .unwrap()
                    .expression,
            )
            .location;
        assert!(
            matches!(analyzer.substitute_operand(ty, entity, &mut environment),
            Err(Error::UnconditionalDiagnostic { location, message: "an invalid expression in a substitution" }) if location == original_location)
        );
        assert_eq!(analyzer.tree.expressions.mark(), copied);
        assert_eq!(
            analyzer
                .with_substitution_soft(None, |analyzer| analyzer
                    .substitute_array_bound(ty, &mut environment))
                .unwrap(),
            None
        );
        assert_eq!(analyzer.graph.template_operands.substitutions.len(), 1);
        assert_eq!(analyzer.dependent_uses.get(), 17);
        assert_eq!(analyzer.template_depth, 4);
        assert_eq!(analyzer.class_depth, 2);
        assert_eq!(analyzer.unevaluated, 3);
        assert_eq!(analyzer.default_argument_scope, Some(scope));
        assert_eq!(analyzer.initializing, Some(entity));
        assert_eq!(analyzer.base_object, Some(entity));
        assert_eq!(analyzer.signature_object, Some(argument));
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
    fn deferred_addresses_keep_target_identity_and_memoize_selection() {
        let source = SourceBuffer::new(
            b"struct O{static int f(int);static long f(long);};\
            template<int(*)(int)>struct I;template<long(*)(long)>struct L;\
            template<class T>I<&T::f>* a();template<class T>L<&T::f>* b();\
            template<class U>I<&U::f>* c();"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"address-targets").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let owner = cursor.intern_name(b"O").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let owner = analyzer
            .graph
            .own(global, owner, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let owner = analyzer.graph.binding(owner).ty.unwrap();
        let patterns = analyzer
            .graph
            .template_operands
            .patterns
            .iter()
            .filter_map(|(&entity, pattern)| pattern.value_target.map(|target| (entity, target)))
            .collect::<Vec<_>>();
        // Renaming T to U shares an operand; changing its target does not.
        assert_eq!(patterns.len(), 2);
        assert_ne!(patterns[0].1, patterns[1].1);
        let arguments = [Some(owner)];
        let mut selected = Vec::new();
        for (entity, target) in patterns {
            let ty = analyzer.graph.entity(entity).ty.unwrap();
            let result = analyzer
                .substitute_operand(ty, entity, &mut analyzer.substitution(0, &arguments))
                .unwrap()
                .unwrap();
            let TypeKind::Address {
                ty: address_type,
                entity: function,
            } = analyzer.graph.types.get(result).kind
            else {
                panic!("selected function address")
            };
            assert_eq!(address_type, target);
            selected.push(function);
            let expressions = analyzer.tree.expressions.mark();
            assert_eq!(
                analyzer
                    .substitute_operand(ty, entity, &mut analyzer.substitution(0, &arguments))
                    .unwrap(),
                Some(result)
            );
            assert_eq!(analyzer.tree.expressions.mark(), expressions);
        }
        assert_ne!(selected[0], selected[1]);
    }

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
            let mut environment = analyzer.substitution(0, &arguments);
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
            let mut environment = analyzer.substitution(0, &arguments);
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
    fn bound_pack_size_keeps_formation_separate_from_structural_identity() {
        let source = SourceBuffer::new(
            b"template<bool>struct C;template<class...T>struct S{\
              template<class...U>using Q=C<(sizeof...(U)==sizeof...(T))>;};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"bound-size-key").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(analyzer.graph.template_operands.patterns.len(), 1);
        let entity = *analyzer
            .graph
            .template_operands
            .patterns
            .keys()
            .next()
            .unwrap();
        let pattern = analyzer.graph.template_operands.pattern(entity).unwrap();
        let original_key = analyzer
            .graph
            .template_operands
            .words(entity)
            .unwrap()
            .to_vec();
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let pack = analyzer
            .graph
            .types
            .intern(TypeKind::Pack(vec![int]), Cv::default())
            .unwrap();
        let outer = [Some(pack)];
        let scope = analyzer
            .operand_scope(
                pattern.scope,
                &mut analyzer.substitution(0, &outer),
                pattern.parameters,
            )
            .unwrap();
        assert!(
            analyzer
                .operand_key(pattern.expression, scope)
                .unwrap()
                .is_none()
        );
        let unknown = analyzer
            .dependent_value_operand(pattern.expression, scope, None, false)
            .unwrap();
        let TypeKind::TemplateParameter {
            entity: unknown_entity,
            ..
        } = analyzer.graph.types.get(unknown).kind
        else {
            panic!("expected an unkeyed operand");
        };
        assert_ne!(unknown_entity, entity);
        assert!(
            analyzer
                .graph
                .template_operands
                .words(unknown_entity)
                .is_none()
        );
        assert_eq!(
            analyzer.graph.lower_operand_key_dependent(unknown),
            Some(false)
        );
        assert!(
            analyzer
                .graph
                .lower_template_operand(unknown)
                .unwrap()
                .1
                .is_empty()
        );
        let inner = [Some(pack)];
        let result = analyzer
            .substitute_operand(
                unknown,
                unknown_entity,
                &mut analyzer.substitution(1, &inner),
            )
            .unwrap()
            .unwrap();
        assert!(matches!(
            analyzer.graph.types.get(result).kind,
            TypeKind::Value { bits: 1, .. }
        ));
        let expressions = analyzer.tree.expressions.mark();
        assert_eq!(
            analyzer
                .substitute_operand(
                    unknown,
                    unknown_entity,
                    &mut analyzer.substitution(1, &inner),
                )
                .unwrap(),
            Some(result)
        );
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(
            analyzer.graph.template_operands.words(entity),
            Some(original_key.as_slice())
        );
        assert!(
            analyzer
                .graph
                .expression_facts
                .get(pattern.expression)
                .is_err()
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

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
            .context()
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
        let mut environment = analyzer.substitution(0, &args);
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
        assert_eq!(saved, [analyzer.graph.types.canonical_view(parameter), one]);
        // This first dependent level has one head and can be re-keyed as
        // N-1. It owns that key rather than inheriting the A-B pattern.
        assert!(analyzer.graph.lower_operand_key_dependent(pending).unwrap());
        assert_ne!(
            analyzer.graph.template_operands.words(pending_entity),
            analyzer.graph.template_operands.words(entity)
        );
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
            let mut environment = analyzer.substitution(0, &args);
            let result = analyzer
                .substitute_operand(pending, pending_entity, &mut environment)
                .unwrap()
                .unwrap();
            assert!(
                matches!(analyzer.graph.types.get(result).kind, TypeKind::Value { ty, bits } if ty == int && bits == value-1)
            );
            let expressions = analyzer.tree.expressions.mark();
            for _ in 0..3 {
                let mut environment = analyzer.substitution(0, &args);
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
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn open_heads_keep_unkeyed_pending_operands_and_evaluate_each_level_once() {
        for value in [true, false] {
            let source = if value {
                b"template<int>struct I;template<int A>struct Outer{\
                  template<int B>using sub=I<A-B>;};"
                    .as_slice()
            } else {
                b"template<class A>struct Outer{\
                  template<class B>using sub=decltype(A()+B());};"
                    .as_slice()
            };
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"open-heads").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            analyzer.declarations(declarations, global).unwrap();
            assert_eq!(analyzer.graph.template_operands.patterns.len(), 1);
            let entity = *analyzer
                .graph
                .template_operands
                .patterns
                .keys()
                .next()
                .unwrap();
            let ty = analyzer.graph.entity(entity).ty.unwrap();
            let pattern = analyzer.graph.template_operands.pattern(entity).unwrap();
            let words = analyzer
                .graph
                .template_operands
                .words(entity)
                .unwrap()
                .to_vec();
            assert!(matches!(pattern.head_depth, HeadDepth::Several));
            let int = analyzer
                .graph
                .types
                .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
                .unwrap();
            let long = analyzer
                .graph
                .types
                .scalar(super::super::types::Scalar::Basic(FundamentalType::LongInt))
                .unwrap();
            let mut arguments = [int, long];
            if value {
                for (argument, bits) in arguments.iter_mut().zip([7, 3]) {
                    *argument = analyzer
                        .graph
                        .types
                        .intern(TypeKind::Value { ty: int, bits }, Cv::default())
                        .unwrap();
                }
            }
            let outer = [Some(arguments[0])];
            let pending = analyzer
                .substitute_operand(ty, entity, &mut analyzer.substitution(0, &outer))
                .unwrap()
                .unwrap();
            let TypeKind::TemplateParameter {
                entity: pending_entity,
                ..
            } = analyzer.graph.types.get(pending).kind
            else {
                panic!("expected an open pending operand");
            };
            assert!(
                analyzer
                    .graph
                    .template_operands
                    .words(pending_entity)
                    .is_none()
            );
            assert_eq!(
                analyzer.graph.lower_operand_key_dependent(pending),
                Some(false)
            );
            let (expression, key) = analyzer.graph.lower_template_operand(pending).unwrap();
            assert_eq!(expression, pattern.expression);
            assert!(key.is_empty());
            let recorded = &analyzer.graph.template_operands.pending[&pending_entity];
            assert_eq!(recorded.placeholder, entity);
            assert_eq!(recorded.arguments, [arguments[0]]);
            assert!(recorded.earlier.is_none());
            let inner = [Some(arguments[1])];
            let result = analyzer
                .substitute_operand(
                    pending,
                    pending_entity,
                    &mut analyzer.substitution(1, &inner),
                )
                .unwrap()
                .unwrap();
            if value {
                assert!(matches!(analyzer.graph.types.get(result).kind,
                    TypeKind::Value { ty, bits: 4 } if ty == int));
            } else {
                assert_eq!(result, long);
            }
            let expressions = analyzer.tree.expressions.mark();
            for _ in 0..3 {
                assert_eq!(
                    analyzer
                        .substitute_operand(
                            pending,
                            pending_entity,
                            &mut analyzer.substitution(1, &inner),
                        )
                        .unwrap(),
                    Some(result)
                );
                assert_eq!(analyzer.tree.expressions.mark(), expressions);
            }
            assert_eq!(
                analyzer.graph.template_operands.words(entity),
                Some(words.as_slice())
            );
            assert!(
                analyzer
                    .graph
                    .expression_facts
                    .get(pattern.expression)
                    .is_err()
            );
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
        }
    }
}
