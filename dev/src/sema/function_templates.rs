// Translated from sema/function_templates.cpp and analyzer_expressions.cpp. See NOTICE.
//! Signature-only specialization: immutable argument overlays, hidden declarations,
//! structural deduction and unit-owned memoization, separate from body demand.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::ValueCategory,
    function_demands::DefinitionEmission,
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId},
    lookup::{FunctionCandidates, LookupResult},
    templates::HeadId,
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::syntax::{ast, storage::List};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

#[derive(PartialEq, Eq, Hash)]
struct SpecializationKey {
    primary: EntityId,
    arguments: Vec<TypeId>,
}
#[derive(Clone, Copy)]
enum SignatureState {
    InProgress,
    Complete(BindingId),
}
#[derive(Default)]
pub(super) struct FunctionTemplates {
    signatures: HashMap<SpecializationKey, SignatureState>,
    primaries: HashMap<EntityId, BindingId>,
    explicit: HashMap<BindingId, Vec<Option<TypeId>>>,
    defaults: HashMap<(HeadId, usize, Vec<TypeId>), TypeId>,
    unique_types: Vec<TypeId>,
    demanded_bodies: HashSet<EntityId>,
}
impl FunctionTemplates {
    pub(super) fn requires_target(&self, binding: BindingId) -> bool {
        self.explicit.contains_key(&binding)
    }
    pub(super) fn body_demanded(&self, primary: EntityId) -> bool {
        self.demanded_bodies.contains(&primary)
    }
    pub(super) fn primary(&self, entity: EntityId) -> Option<BindingId> {
        self.primaries.get(&entity).copied()
    }
}
struct Substitution<'a> {
    depth: u32,
    arguments: &'a [Option<TypeId>],
    memo: HashMap<TypeId, Option<TypeId>>,
}
impl<'a> Substitution<'a> {
    fn new(depth: u32, arguments: &'a [Option<TypeId>]) -> Self {
        Self {
            depth,
            arguments,
            memo: HashMap::new(),
        }
    }
}
impl Analyzer<'_> {
    fn function_template_head(&self, binding: BindingId) -> Result<Option<HeadId>, Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        let Some(head) = entity.template_head else {
            return Ok(None);
        };
        for &parameter in self.graph.templates.parameters(head) {
            if !self
                .graph
                .entity(parameter)
                .template_parameter
                .is_some_and(super::templates::ParameterInfo::ordinary_type)
            {
                return Err(Error::Missing(
                    "function template packs or template-template arguments",
                ));
            }
        }
        if entity
            .function
            .is_some_and(|properties| properties.special.is_some())
        {
            return Err(Error::Missing(
                "special member function template signatures",
            ));
        }
        Ok(Some(head))
    }

    pub(super) fn explicit_function_specializations(
        &mut self,
        component: ast::NameComponent,
        found: &mut LookupResult,
        scope: ScopeId,
        call: bool,
    ) -> Result<(), Error> {
        let Some(syntax) = component.arguments else {
            return Ok(());
        };
        if found.functions.as_slice().is_empty() {
            return Err(Error::Missing(
                "class or alias template specialization lookup",
            ));
        }
        let mut given = Vec::new();
        for argument in self.tree.template_arguments.get(syntax) {
            let ast::TemplateArgumentKind::Type(ty) = argument.kind else {
                return Err(Error::Missing("non-type function template arguments"));
            };
            if argument.pack {
                return Err(Error::Missing("function template argument expansion"));
            }
            let ty = self.type_syntax(ty, scope)?;
            given.push(self.graph.types.canonical_function(ty)?);
        }
        let mut specializations = FunctionCandidates::default();
        for &primary in found.functions.as_slice() {
            let Some(head) = self.function_template_head(primary)? else {
                continue;
            };
            let count = self.graph.templates.parameters(head).len();
            if given.len() > count {
                continue;
            }
            let mut arguments = vec![None; count];
            for (slot, &argument) in arguments.iter_mut().zip(&given) {
                *slot = Some(argument);
            }
            let partial = (given.len()..count)
                .any(|index| self.graph.templates.default_argument(head, index).is_none());
            let binding = if call || partial {
                let binding = self.graph.binding(primary);
                let hidden = self.graph.add_hidden_function(primary, binding)?;
                self.graph
                    .function_templates
                    .explicit
                    .insert(hidden, arguments);
                hidden
            } else if let Some(binding) = self.complete_signature(primary, &mut arguments)? {
                binding
            } else {
                continue;
            };
            specializations.push(binding);
        }
        if specializations.as_slice().is_empty() {
            return Err(Error::at(
                component.location,
                if call {
                    "no viable function for the call"
                } else {
                    "no function template matches the template arguments"
                },
            ));
        }
        found.binding = specializations.as_slice().first().copied();
        found.functions = specializations;
        Ok(())
    }

    fn template_parameter_index(&self, ty: TypeId, depth: u32) -> Option<usize> {
        let TypeKind::TemplateParameter {
            entity,
            template: false,
        } = self.graph.types.get(ty).kind
        else {
            return None;
        };
        self.graph.entity(entity).template_parameter?.index(depth)
    }

    fn substitute_signature_type(
        &mut self,
        ty: TypeId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        self.check_stack()?;
        if let Some(&result) = environment.memo.get(&ty) {
            return Ok(result);
        }
        let cv = self.graph.types.get(ty).cv;
        if let Some(index) = self.template_parameter_index(ty, environment.depth) {
            let result = if let Some(Some(argument)) = environment.arguments.get(index) {
                Some(self.graph.types.qualify(*argument, cv)?)
            } else {
                Some(ty)
            };
            environment.memo.insert(ty, result);
            return Ok(result);
        }
        let kind = self.graph.types.get(ty).kind.clone();
        let result = match kind {
            TypeKind::Pointer(element) => {
                let Some(element) = self.substitute_signature_type(element, environment)? else {
                    return Ok(None);
                };
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Reference { .. }
                ) {
                    None
                } else {
                    Some(self.graph.types.intern(TypeKind::Pointer(element), cv)?)
                }
            }
            TypeKind::Reference { element, kind } => {
                let Some(element) = self.substitute_signature_type(element, environment)? else {
                    return Ok(None);
                };
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Scalar(Scalar::Void)
                ) {
                    None
                } else {
                    Some(self.graph.types.reference(element, kind)?)
                }
            }
            TypeKind::MemberPointer { owner, element } => {
                let Some(owner) = self.substitute_signature_type(owner, environment)? else {
                    return Ok(None);
                };
                let Some(element) = self.substitute_signature_type(element, environment)? else {
                    return Ok(None);
                };
                if !matches!(
                    self.graph.types.get(owner).kind,
                    TypeKind::Class(_) | TypeKind::TemplateParameter { .. }
                ) || matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Reference { .. } | TypeKind::Scalar(Scalar::Void)
                ) {
                    None
                } else {
                    Some(
                        self.graph
                            .types
                            .intern(TypeKind::MemberPointer { owner, element }, cv)?,
                    )
                }
            }
            TypeKind::Array { element, bound } => {
                let Some(element) = self.substitute_signature_type(element, environment)? else {
                    return Ok(None);
                };
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Reference { .. }
                        | TypeKind::Function { .. }
                        | TypeKind::Scalar(Scalar::Void)
                ) {
                    None
                } else {
                    Some(
                        self.graph
                            .types
                            .intern(TypeKind::Array { element, bound }, cv)?,
                    )
                }
            }
            TypeKind::Function {
                result,
                parameters,
                variadic,
                cv: function_cv,
                reference,
            } => {
                let Some(result) = self.substitute_signature_type(result, environment)? else {
                    return Ok(None);
                };
                if matches!(
                    self.graph.types.get(result).kind,
                    TypeKind::Array { .. } | TypeKind::Function { .. }
                ) {
                    return Ok(None);
                }
                let mut substituted = Vec::with_capacity(parameters.len());
                for parameter in parameters {
                    let Some(parameter) = self.substitute_signature_type(parameter, environment)?
                    else {
                        return Ok(None);
                    };
                    if matches!(
                        self.graph.types.get(parameter).kind,
                        TypeKind::Scalar(Scalar::Void)
                    ) {
                        return Ok(None);
                    }
                    substituted.push(self.graph.types.parameter(parameter)?);
                }
                Some(self.graph.types.intern(
                    TypeKind::Function {
                        result,
                        parameters: substituted,
                        variadic,
                        cv: function_cv,
                        reference,
                    },
                    cv,
                )?)
            }
            _ => Some(ty),
        };
        environment.memo.insert(ty, result);
        Ok(result)
    }

    fn complete_signature(
        &mut self,
        primary: BindingId,
        arguments: &mut [Option<TypeId>],
    ) -> Result<Option<BindingId>, Error> {
        let primary = self
            .graph
            .function_origin(self.graph.binding(primary).entity)?;
        let Some(head) = self.function_template_head(primary)? else {
            return Ok(Some(primary));
        };
        let depth = self.graph.templates.depth(head);
        for index in 0..arguments.len() {
            if arguments[index].is_some() {
                continue;
            }
            let Some(default) = self.graph.templates.default_argument(head, index) else {
                return Ok(None);
            };
            let prefix: Option<Vec<_>> = arguments[..index].iter().copied().collect();
            let prefix = prefix.ok_or(Error::Invariant("incomplete default argument prefix"))?;
            let key = (head, index, prefix);
            let ty = if let Some(&ty) = self.graph.function_templates.defaults.get(&key) {
                ty
            } else {
                let pattern = self.type_syntax(default.syntax, default.scope)?;
                let mut environment = Substitution::new(depth, arguments);
                let Some(ty) = self.substitute_signature_type(pattern, &mut environment)? else {
                    return Ok(None);
                };
                let ty = self.graph.types.canonical_function(ty)?;
                self.graph.function_templates.defaults.insert(key, ty);
                ty
            };
            arguments[index] = Some(ty);
        }
        let canonical: Option<Vec<_>> = arguments.iter().copied().collect();
        let key = SpecializationKey {
            primary: self.graph.binding(primary).entity,
            arguments: canonical.ok_or(Error::Invariant("incomplete signature arguments"))?,
        };
        if let Some(&state) = self.graph.function_templates.signatures.get(&key) {
            return Ok(match state {
                SignatureState::Complete(binding) => Some(binding),
                SignatureState::InProgress => None,
            });
        }
        let pattern = self
            .graph
            .binding(primary)
            .ty
            .ok_or(Error::Invariant("template without function type"))?;
        self.graph
            .function_templates
            .signatures
            .insert(key, SignatureState::InProgress);
        let mut environment = Substitution::new(depth, arguments);
        let result = self.substitute_signature_type(pattern, &mut environment);
        let key = SpecializationKey {
            primary: self.graph.binding(primary).entity,
            arguments: arguments
                .iter()
                .copied()
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::Invariant("lost signature arguments"))?,
        };
        let ty = match result {
            Ok(Some(ty)) => self.graph.types.canonical_function(ty)?,
            Ok(None) => {
                self.graph.function_templates.signatures.remove(&key);
                return Ok(None);
            }
            Err(error) => {
                self.graph.function_templates.signatures.remove(&key);
                return Err(error);
            }
        };
        let origin = self.graph.binding(primary);
        let entity = self.graph.add_entity(Some(ty), origin.name, origin.key)?;
        let old = self.graph.entity(origin.entity);
        let properties = (
            old.function,
            old.availability,
            old.constexpr,
            old.home,
            old.scope,
        );
        let specialized = self.graph.entity_mut(entity);
        specialized.function = properties.0;
        specialized.availability = properties.1;
        specialized.constexpr = properties.2;
        specialized.home = properties.3;
        specialized.scope = properties.4;
        specialized.definition_emission = DefinitionEmission::Demanded;
        let binding = self.graph.add_hidden_function(
            primary,
            Binding {
                kind: BindingKind::Function,
                name: origin.name,
                entity,
                ty: Some(ty),
                key: origin.key,
            },
        )?;
        self.graph
            .function_templates
            .primaries
            .insert(entity, primary);
        self.graph
            .function_templates
            .signatures
            .insert(key, SignatureState::Complete(binding));
        Ok(Some(binding))
    }

    fn deduce_signature_type(
        &mut self,
        pattern: TypeId,
        actual: TypeId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
        extra_cv: bool,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let pattern = self.graph.types.canonical_function(pattern)?;
        let actual = self.graph.types.canonical_function(actual)?;
        let pcv = self.type_cv(pattern).0;
        let acv = self.type_cv(actual).0;
        if let Some(index) = self.template_parameter_index(pattern, depth) {
            if pcv & !acv != 0 && !extra_cv {
                return Ok(false);
            }
            let bare = self.bare_type(actual)?;
            let deduced = self.graph.types.qualify(bare, Cv(acv & !pcv))?;
            let Some(slot) = arguments.get_mut(index) else {
                return Ok(pattern == actual);
            };
            if let Some(known) = *slot {
                return Ok(known == deduced);
            }
            *slot = Some(deduced);
            return Ok(true);
        }
        if pcv != acv && !(extra_cv && acv & !pcv == 0) {
            return Ok(false);
        }
        match (
            self.graph.types.get(pattern).kind.clone(),
            self.graph.types.get(actual).kind.clone(),
        ) {
            (TypeKind::Pointer(p), TypeKind::Pointer(a)) => {
                self.deduce_signature_type(p, a, depth, arguments, extra_cv)
            }
            (
                TypeKind::Reference {
                    element: p,
                    kind: pk,
                },
                TypeKind::Reference {
                    element: a,
                    kind: ak,
                },
            ) => Ok(pk == ak && self.deduce_signature_type(p, a, depth, arguments, false)?),
            (
                TypeKind::MemberPointer {
                    owner: po,
                    element: p,
                },
                TypeKind::MemberPointer {
                    owner: ao,
                    element: a,
                },
            ) => Ok(self.deduce_signature_type(po, ao, depth, arguments, false)?
                && self.deduce_signature_type(p, a, depth, arguments, extra_cv)?),
            (
                TypeKind::Array {
                    element: p,
                    bound: pb,
                },
                TypeKind::Array {
                    element: a,
                    bound: ab,
                },
            ) => Ok(pb == ab && self.deduce_signature_type(p, a, depth, arguments, extra_cv)?),
            (
                TypeKind::Function {
                    result: p,
                    parameters: pp,
                    variadic: pv,
                    cv: pc,
                    reference: pr,
                },
                TypeKind::Function {
                    result: a,
                    parameters: ap,
                    variadic: av,
                    cv: ac,
                    reference: ar,
                },
            ) => {
                if pp.len() != ap.len() || pv != av || pc != ac || pr != ar {
                    return Ok(false);
                }
                for (p, a) in pp.into_iter().zip(ap) {
                    if !self.deduce_signature_type(p, a, depth, arguments, false)? {
                        return Ok(false);
                    }
                }
                self.deduce_signature_type(p, a, depth, arguments, false)
            }
            _ => Ok(pattern == actual),
        }
    }

    fn deduce_call_argument(
        &mut self,
        mut pattern: TypeId,
        mut actual: TypeId,
        category: ValueCategory,
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        if let TypeKind::Reference { element, kind } = self.graph.types.get(pattern).kind {
            if kind == RefKind::Rvalue
                && self.type_cv(element).0 == 0
                && self.template_parameter_index(element, depth).is_some()
                && category == ValueCategory::Lvalue
            {
                actual = self.graph.types.reference(actual, RefKind::Lvalue)?;
            }
            pattern = element;
        } else {
            pattern = self.bare_type(pattern)?;
            actual = self.decayed_type(actual)?;
        }
        self.deduce_signature_type(pattern, actual, depth, arguments, true)
    }

    fn signature_arguments(&self, binding: BindingId, head: HeadId) -> Vec<Option<TypeId>> {
        self.graph
            .function_templates
            .explicit
            .get(&binding)
            .cloned()
            .unwrap_or_else(|| vec![None; self.graph.templates.parameters(head).len()])
    }
    fn needs_deduction(&self, ty: TypeId, depth: u32) -> bool {
        let mut work = vec![ty];
        let mut seen = HashSet::new();
        while let Some(ty) = work.pop() {
            if !seen.insert(ty) {
                continue;
            }
            if self.template_parameter_index(ty, depth).is_some() {
                return true;
            }
            match &self.graph.types.get(ty).kind {
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Array { element, .. } => work.push(*element),
                TypeKind::MemberPointer { owner, element } => work.extend([*owner, *element]),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters.iter().copied());
                }
                _ => (),
            }
        }
        false
    }
    pub(super) fn deduce_function_call(
        &mut self,
        binding: BindingId,
        call_arguments: List<ast::ExprId>,
    ) -> Result<Option<BindingId>, Error> {
        let Some(head) = self.function_template_head(binding)? else {
            return Ok(Some(binding));
        };
        let depth = self.graph.templates.depth(head);
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("template call without type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        let args = self.tree.expr_lists.get(call_arguments);
        let TypeKind::Function { variadic, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("template is no function"));
        };
        if args.len() > count && !variadic
            || (args.len()..count)
                .any(|index| !self.function_has_default(self.graph.binding(binding).entity, index))
        {
            return Ok(None);
        }
        let mut arguments = self.signature_arguments(binding, head);
        let given = arguments.clone();
        for (index, &argument) in args.iter().enumerate().take(count) {
            let pattern = self.graph.types.function_parameter(ty, index)?;
            let mut environment = Substitution::new(depth, &given);
            let Some(pattern) = self.substitute_signature_type(pattern, &mut environment)? else {
                return Ok(None);
            };
            if !self.needs_deduction(pattern, depth) {
                continue;
            }
            let facts = self.graph.expression_facts.get(argument)?;
            if let Some(set) = facts.functions {
                self.deduce_overloaded_argument(pattern, set, depth, &mut arguments)?;
            } else if !self.deduce_call_argument(
                pattern,
                facts.ty,
                facts.category,
                depth,
                &mut arguments,
            )? {
                return Ok(None);
            }
        }
        self.complete_signature(binding, &mut arguments)
    }

    fn deduce_overloaded_argument(
        &mut self,
        pattern: TypeId,
        set: super::function_resolution::OverloadId,
        depth: u32,
        arguments: &mut Vec<Option<TypeId>>,
    ) -> Result<(), Error> {
        let mut found = None;
        for index in 0..self.graph.overloads.get(set).len() {
            let function = self.graph.overloads.get(set)[index];
            if self
                .graph
                .entity(self.graph.binding(function).entity)
                .template_head
                .is_some()
            {
                return Ok(());
            }
            let mut trial = arguments.clone();
            let mut actual = self
                .graph
                .binding(function)
                .ty
                .ok_or(Error::Invariant("overloaded argument without type"))?;
            if let Some(owner) = self.member_function_owner(function) {
                actual = self.graph.types.member_pointer(owner, actual)?;
            }
            if !self.deduce_call_argument(
                pattern,
                actual,
                ValueCategory::Lvalue,
                depth,
                &mut trial,
            )? {
                continue;
            }
            if found.is_some() {
                return Ok(());
            }
            found = Some(trial);
        }
        if let Some(found) = found {
            *arguments = found;
        }
        Ok(())
    }

    pub(super) fn deduce_function_target(
        &mut self,
        binding: BindingId,
        target: TypeId,
    ) -> Result<Option<BindingId>, Error> {
        let Some(head) = self.function_template_head(binding)? else {
            return Ok(None);
        };
        let mut arguments = self.signature_arguments(binding, head);
        let pattern = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("template target without type"))?;
        let depth = self.graph.templates.depth(head);
        let mut environment = Substitution::new(depth, &arguments);
        let Some(pattern) = self.substitute_signature_type(pattern, &mut environment)? else {
            return Ok(None);
        };
        let pattern = self.target_deduction_pattern(pattern, target, depth)?;
        if !self.deduce_signature_type(pattern, target, depth, &mut arguments, false)? {
            return Ok(None);
        }
        self.complete_signature(binding, &mut arguments)
    }

    fn target_deduction_pattern(
        &mut self,
        pattern: TypeId,
        target: TypeId,
        depth: u32,
    ) -> Result<TypeId, Error> {
        let TypeKind::Function {
            result,
            mut parameters,
            variadic,
            cv,
            reference,
        } = self.graph.types.get(pattern).kind.clone()
        else {
            return Ok(pattern);
        };
        let count = self.graph.types.function_parameter_count(target)?;
        for (index, parameter) in parameters.iter_mut().enumerate().take(count) {
            let TypeKind::Reference {
                element,
                kind: RefKind::Rvalue,
            } = self.graph.types.get(*parameter).kind
            else {
                continue;
            };
            let actual = self.graph.types.function_parameter(target, index)?;
            if self.type_cv(element).0 == 0
                && self.template_parameter_index(element, depth).is_some()
                && matches!(
                    self.graph.types.get(actual).kind,
                    TypeKind::Reference {
                        kind: RefKind::Lvalue,
                        ..
                    }
                )
            {
                *parameter = element;
            }
        }
        self.graph.types.intern(
            TypeKind::Function {
                result,
                parameters,
                variadic,
                cv,
                reference,
            },
            Cv::default(),
        )
    }

    pub(super) fn check_specialization_body(&mut self, binding: BindingId) -> Result<(), Error> {
        if self.unevaluated == 0
            && self.template_depth == 0
            && let Some(primary) = self
                .graph
                .function_templates
                .primary(self.graph.binding(binding).entity)
        {
            let primary = self.graph.binding(primary).entity;
            self.graph
                .function_templates
                .demanded_bodies
                .insert(primary);
            if self.graph.entity(primary).defined {
                return Err(Error::Missing(
                    "demanded function template body instantiation",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn function_has_default(&self, entity: EntityId, index: usize) -> bool {
        if self.graph.function_default(entity, index).is_some() {
            return true;
        }
        let primary = self
            .graph
            .function_templates
            .primary(entity)
            .map_or(entity, |binding| self.graph.binding(binding).entity);
        let primary = self.graph.entity(primary);
        primary.template_head.is_some()
            && primary
                .function_parameters
                .and_then(|clause| self.tree.parameters.get(clause.parameters).get(index))
                .is_some_and(|parameter| parameter.default.is_some())
    }

    pub(super) fn check_template_call_defaults(
        &self,
        binding: BindingId,
        arguments: List<ast::ExprId>,
    ) -> Result<(), Error> {
        let entity = self.graph.binding(binding).entity;
        let primary = self
            .graph
            .function_templates
            .primary(entity)
            .map_or(entity, |binding| self.graph.binding(binding).entity);
        let primary = self.graph.entity(primary);
        if primary.template_head.is_none() {
            return Ok(());
        }
        if let Some(clause) = primary.function_parameters {
            let parameters = self.tree.parameters.get(clause.parameters);
            let count = self.tree.expr_lists.get(arguments).len();
            if count < parameters.len()
                && parameters[count..]
                    .iter()
                    .all(|parameter| parameter.default.is_some())
            {
                return Err(Error::Missing(
                    "demanded function template default argument instantiation",
                ));
            }
        }
        Ok(())
    }

    fn template_at_least_as_specialized(
        &mut self,
        first: BindingId,
        second: BindingId,
        call_count: Option<usize>,
    ) -> Result<bool, Error> {
        let Some(from_head) = self.function_template_head(first)? else {
            return Ok(false);
        };
        let Some(to_head) = self.function_template_head(second)? else {
            return Ok(false);
        };
        let count = self.graph.templates.parameters(from_head).len();
        while self.graph.function_templates.unique_types.len() < count {
            let entity =
                self.graph
                    .add_entity(None, None, crate::lex::vocabulary::Keyword::Struct)?;
            let ty = self
                .graph
                .types
                .intern(TypeKind::Class(entity), Cv::default())?;
            self.graph.function_templates.unique_types.push(ty);
        }
        let unique: Vec<_> = self.graph.function_templates.unique_types[..count]
            .iter()
            .copied()
            .map(Some)
            .collect();
        let mut environment = Substitution::new(self.graph.templates.depth(from_head), &unique);
        let from = self
            .graph
            .binding(first)
            .ty
            .ok_or(Error::Invariant("ordering without type"))?;
        let Some(from) = self.substitute_signature_type(from, &mut environment)? else {
            return Ok(false);
        };
        let to = self
            .graph
            .binding(second)
            .ty
            .ok_or(Error::Invariant("ordering without pattern"))?;
        let depth = self.graph.templates.depth(to_head);
        let mut arguments = vec![None; self.graph.templates.parameters(to_head).len()];
        if let Some(count) = call_count {
            let a_count = self.graph.types.function_parameter_count(from)?;
            let b_count = self.graph.types.function_parameter_count(to)?;
            for index in 0..count.min(a_count).min(b_count) {
                let a = self.graph.types.function_parameter(from, index)?;
                let p = self.graph.types.function_parameter(to, index)?;
                let a = match self.graph.types.get(a).kind {
                    TypeKind::Reference { element, .. } => element,
                    _ => a,
                };
                let p = match self.graph.types.get(p).kind {
                    TypeKind::Reference { element, .. } => element,
                    _ => p,
                };
                let a = self.bare_type(a)?;
                let p = self.bare_type(p)?;
                if !self.deduce_signature_type(p, a, depth, &mut arguments, false)? {
                    return Ok(false);
                }
            }
            Ok(true)
        } else {
            self.deduce_signature_type(to, from, depth, &mut arguments, false)
        }
    }

    fn template_reference_order(
        &self,
        first: BindingId,
        second: BindingId,
        count: usize,
    ) -> Result<Ordering, Error> {
        let a = self
            .graph
            .binding(first)
            .ty
            .ok_or(Error::Invariant("reference ordering without type"))?;
        let b = self
            .graph
            .binding(second)
            .ty
            .ok_or(Error::Invariant("reference ordering without type"))?;
        let count = count
            .min(self.graph.types.function_parameter_count(a)?)
            .min(self.graph.types.function_parameter_count(b)?);
        let mut order = Ordering::Equal;
        for index in 0..count {
            let a = self.graph.types.function_parameter(a, index)?;
            let b = self.graph.types.function_parameter(b, index)?;
            let (
                TypeKind::Reference {
                    element: a,
                    kind: ak,
                },
                TypeKind::Reference {
                    element: b,
                    kind: bk,
                },
            ) = (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
            else {
                continue;
            };
            let acv = self.type_cv(*a).0;
            let bcv = self.type_cv(*b).0;
            let step = if ak != bk {
                if *ak == RefKind::Lvalue {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            } else if acv != bcv && acv & bcv == bcv {
                Ordering::Greater
            } else if acv != bcv && acv & bcv == acv {
                Ordering::Less
            } else {
                Ordering::Equal
            };
            if step != Ordering::Equal && order != Ordering::Equal && step != order {
                return Ok(Ordering::Equal);
            }
            if step != Ordering::Equal {
                order = step;
            }
        }
        Ok(order)
    }

    pub(super) fn compare_function_templates(
        &mut self,
        first: BindingId,
        second: BindingId,
        call_count: Option<usize>,
    ) -> Result<Ordering, Error> {
        let a = self
            .graph
            .function_templates
            .primary(self.graph.binding(first).entity);
        let b = self
            .graph
            .function_templates
            .primary(self.graph.binding(second).entity);
        let (Some(a), Some(b)) = (a, b) else {
            return Ok(match (a.is_some(), b.is_some()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            });
        };
        if self.graph.binding(a).entity == self.graph.binding(b).entity {
            return Ok(Ordering::Equal);
        }
        let forward = self.template_at_least_as_specialized(a, b, call_count)?;
        let backward = self.template_at_least_as_specialized(b, a, call_count)?;
        if forward != backward || !forward || call_count.is_none() {
            return Ok(forward.cmp(&backward));
        }
        self.template_reference_order(a, b, call_count.unwrap_or(0))
    }
}
