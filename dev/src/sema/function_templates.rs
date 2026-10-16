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
use crate::syntax::ast;
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
    pub(super) bodies: super::template_bodies::Bodies,
    pub(super) argument_defaults: super::template_defaults::Defaults,
    pub(super) instantiations: super::explicit_instantiations::Instantiations,
    pub(super) arguments: HashMap<EntityId, Vec<TypeId>>,
    explicit: HashMap<BindingId, Vec<Option<TypeId>>>,
    defaults: HashMap<(HeadId, usize, Vec<TypeId>), TypeId>,
    unique_types: Vec<TypeId>,
    demanded_bodies: HashSet<EntityId>,
    trailing_results: HashSet<EntityId>,
}
impl FunctionTemplates {
    pub(super) fn requires_target(&self, binding: BindingId) -> bool {
        self.explicit.contains_key(&binding)
    }
    #[cfg(test)]
    pub(super) fn body_demanded(&self, primary: EntityId) -> bool {
        self.demanded_bodies.contains(&primary)
    }
    pub(super) fn primary(&self, entity: EntityId) -> Option<BindingId> {
        self.primaries.get(&entity).copied()
    }
}
pub(super) struct Substitution<'a> {
    pub(super) depth: u32,
    pub(super) arguments: &'a [Option<TypeId>],
    memo: HashMap<TypeId, Option<TypeId>>,
    pub(super) context: Option<EntityId>,
}
impl<'a> Substitution<'a> {
    pub(super) fn new(depth: u32, arguments: &'a [Option<TypeId>]) -> Self {
        Self {
            depth,
            arguments,
            memo: HashMap::new(),
            context: None,
        }
    }
}
impl Analyzer<'_> {
    pub(super) fn record_template_result_order(
        &mut self,
        entity: EntityId,
        info: &super::declarators::DeclaratorInfo,
    ) {
        if info.qualifiers.is_some_and(|qualifiers| {
            self.tree
                .qualifiers
                .get(qualifiers)
                .iter()
                .any(|qualifier| matches!(qualifier.kind, ast::QualifierKind::Trailing(_)))
        }) {
            self.graph
                .function_templates
                .trailing_results
                .insert(entity);
        }
    }

    pub(super) fn specialization_object_signature(
        &mut self,
        entity: EntityId,
        pattern: TypeId,
    ) -> Result<TypeId, Error> {
        let primary = self
            .graph
            .function_templates
            .primary(entity)
            .ok_or(Error::Invariant("prototype without function template"))?;
        let head = self
            .function_template_head(primary)?
            .ok_or(Error::Invariant("prototype without template head"))?;
        let arguments: Vec<_> = self
            .graph
            .function_templates
            .arguments
            .get(&entity)
            .ok_or(Error::Invariant("prototype without template arguments"))?
            .iter()
            .copied()
            .map(Some)
            .collect();
        let mut environment = Substitution::new(self.graph.templates.depth(head), &arguments);
        environment.context = Some(self.graph.binding(primary).entity);
        let TypeKind::Function {
            result,
            parameters,
            variadic,
            cv,
            reference,
        } = self.graph.types.get(pattern).kind.clone()
        else {
            return Err(Error::Invariant("prototype without function type"));
        };
        let result = self
            .substitute_signature_type(result, &mut environment)?
            .ok_or(Error::Missing("dependent exception prototype result"))?;
        let mut objects = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let parameter = self
                .substitute_signature_type(parameter, &mut environment)?
                .ok_or(Error::Missing("dependent exception prototype parameter"))?;
            // Prototype objects keep declared cv; canonical call signatures
            // erase it. Arrays and functions still adjust to pointer objects.
            objects.push(self.graph.types.parameter_object(parameter)?);
        }
        self.graph.types.intern(
            TypeKind::Function {
                result,
                parameters: objects,
                variadic,
                cv,
                reference,
            },
            Cv::default(),
        )
    }

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
            .and_then(|properties| properties.special)
            .is_some_and(|name| {
                !matches!(name, super::function_names::SpecialName::Operator { .. })
            })
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
        for syntax_index in 0..self.tree.template_arguments.get(syntax).len() {
            let syntax_item = self.tree.template_arguments.get(syntax)[syntax_index];
            let argument = &syntax_item;
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

    pub(super) fn substitute_signature_type(
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
            TypeKind::TemplateParameter { entity, .. }
                if self.graph.dependent_members.get(entity).is_some() =>
            {
                let member = self
                    .graph
                    .dependent_members
                    .get(entity)
                    .ok_or(Error::Invariant("lost dependent member"))?;
                self.substitute_member_type(ty, member, environment)?
            }
            TypeKind::TemplateParameter { entity, .. }
                if self.graph.template_operands.pattern(entity).is_some() =>
            {
                self.substitute_operand(ty, entity, environment)?
            }
            TypeKind::Class(entity) if self.graph.class_templates.instance(entity).is_some() => {
                self.substitute_class_type(entity, environment)?
                    .map(|entity| self.graph.types.intern(TypeKind::Class(entity), cv))
                    .transpose()?
            }
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
            TypeKind::Array {
                element,
                mut bound,
                mut operand,
            } => {
                let Some(element) = self.substitute_signature_type(element, environment)? else {
                    return Ok(None);
                };
                if let Some(pattern) = operand
                    && environment.arguments.iter().all(|argument| {
                        argument.is_some_and(|ty| !self.exception_dependent_type(ty))
                    })
                {
                    let Some(count) = self.substitute_array_bound(pattern, environment)? else {
                        return Ok(None);
                    };
                    bound = count;
                    operand = None;
                }
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Reference { .. }
                        | TypeKind::Function { .. }
                        | TypeKind::Scalar(Scalar::Void)
                ) || !self.exception_dependent_type(element)
                    && matches!(self.graph.types.get(element).kind, TypeKind::Class(class)
                        if self.graph.entity(class).complete
                            && self.graph.virtuals.abstract_class(class))
                {
                    None
                } else {
                    Some(self.graph.types.intern(
                        TypeKind::Array {
                            element,
                            bound,
                            operand,
                        },
                        cv,
                    )?)
                }
            }
            TypeKind::Function { .. } => {
                self.substitute_function_signature(ty, environment, true)?
            }
            _ => Some(ty),
        };
        environment.memo.insert(ty, result);
        Ok(result)
    }

    fn substitute_function_signature(
        &mut self,
        ty: TypeId,
        environment: &mut Substitution<'_>,
        leading: bool,
    ) -> Result<Option<TypeId>, Error> {
        let TypeKind::Function {
            result,
            parameters,
            variadic,
            cv,
            reference,
        } = self.graph.types.get(ty).kind.clone()
        else {
            return Err(Error::Invariant("substituting a nonfunction signature"));
        };
        let mut substituted_result = None;
        if leading {
            substituted_result = self.substitute_signature_type(result, environment)?;
            if substituted_result.is_none() {
                return Ok(None);
            }
        }
        let mut substituted = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let Some(parameter) = self.substitute_signature_type(parameter, environment)? else {
                return Ok(None);
            };
            substituted.push(self.graph.types.parameter(parameter)?);
        }
        let Some(result) = (if leading {
            substituted_result
        } else {
            self.substitute_signature_type(result, environment)?
        }) else {
            return Ok(None);
        };
        // Formation validity follows lexical substitution, as in C++'s
        // substitute_function; an earlier failed operand already returned.
        if matches!(
            self.graph.types.get(result).kind,
            TypeKind::Array { .. } | TypeKind::Function { .. }
        ) || substituted.iter().any(|&parameter| {
            matches!(
                self.graph.types.get(parameter).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        }) {
            return Ok(None);
        }
        self.graph
            .types
            .intern(
                TypeKind::Function {
                    result,
                    parameters: substituted,
                    variadic,
                    cv,
                    reference,
                },
                self.graph.types.get(ty).cv,
            )
            .map(Some)
    }

    fn complete_signature(
        &mut self,
        primary: BindingId,
        arguments: &mut [Option<TypeId>],
    ) -> Result<Option<BindingId>, Error> {
        self.complete_signature_in(primary, arguments, true)
    }
    fn complete_signature_in(
        &mut self,
        primary: BindingId,
        arguments: &mut [Option<TypeId>],
        check_access: bool,
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
                let mut environment = Substitution::new(depth, &arguments[..index]);
                environment.context = check_access.then_some(self.graph.binding(primary).entity);
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
        environment.context = check_access.then_some(self.graph.binding(primary).entity);
        let leading = !self
            .graph
            .function_templates
            .trailing_results
            .contains(&self.graph.binding(primary).entity);
        let result = self.substitute_function_signature(pattern, &mut environment, leading);
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
        let old_inline = old.inline;
        let parameters = old.function_parameters;
        let exceptions = old.exceptions.clone();
        let specialized = self.graph.entity_mut(entity);
        specialized.function = properties.0;
        specialized.availability = properties.1;
        specialized.constexpr = properties.2;
        specialized.home = properties.3;
        specialized.scope = properties.4;
        specialized.inline = old_inline;
        // Declaration facts belong to the specialization's formation point.
        // A later primary redeclaration must not rename its prototype; a
        // demanded body installs its own copied parameters separately.
        specialized.function_parameters = parameters;
        specialized.exceptions = exceptions;
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
            .arguments
            .insert(entity, key.arguments.clone());
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
        if let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(pattern).kind
            && (self.graph.template_operands.pattern(entity).is_some()
                || self.graph.dependent_members.get(entity).is_some())
        {
            return Ok(true); // Dependent operands and qualified members are non-deduced.
        }
        let pcv = self.type_cv(pattern).0;
        let acv = self.type_cv(actual).0;
        let parameter = if matches!(self.graph.types.get(pattern).kind, TypeKind::Auto) {
            Some(0)
        } else {
            self.template_parameter_index(pattern, depth)
        };
        if let Some(index) = parameter {
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
            (TypeKind::Class(p), TypeKind::Class(a)) => {
                match (
                    self.graph.class_templates.instance(p),
                    self.graph.class_templates.instance(a),
                ) {
                    (Some(p), Some(a))
                        if p.primary == a.primary && p.arguments.len() == a.arguments.len() =>
                    {
                        let pairs: Vec<_> = p
                            .arguments
                            .iter()
                            .copied()
                            .zip(a.arguments.iter().copied())
                            .collect();
                        for (p, a) in pairs {
                            if !self.deduce_signature_type(p, a, depth, arguments, false)? {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                    _ => Ok(pattern == actual),
                }
            }
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
                    operand,
                },
                TypeKind::Array {
                    element: a,
                    bound: ab,
                    ..
                },
            ) => Ok((operand.is_some() || pb == ab)
                && self.deduce_signature_type(p, a, depth, arguments, extra_cv)?),
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

    pub(super) fn deduce_call_argument(
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
                && (self.template_parameter_index(element, depth).is_some()
                    || matches!(self.graph.types.get(element).kind, TypeKind::Auto))
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
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.graph.class_templates.instance(*entity) {
                        work.extend(&instance.arguments);
                    }
                }
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
        args: &[ast::ExprId],
        implicit_postfix: bool,
    ) -> Result<Option<BindingId>, Error> {
        if self
            .graph
            .ignored_assignment(self.graph.binding(binding).entity)
        {
            return Ok(None);
        }
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
        let TypeKind::Function { variadic, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("template is no function"));
        };
        // Postfix operator syntax supplies its final int argument implicitly.
        // Deduce only from the written operand; the operator owner validates
        // that final parameter after forming the specialization.
        let supplied = args.len() + usize::from(implicit_postfix);
        if supplied > count && !variadic
            || (supplied..count)
                .any(|index| !self.function_has_default(self.graph.binding(binding).entity, index))
        {
            return Ok(None);
        }
        let mut arguments = self.signature_arguments(binding, head);
        let given = arguments.clone();
        // Explicit arguments substitute a leading result before parameter
        // deduction. A trailing result waits until signature completion.
        if self.graph.function_templates.requires_target(binding)
            && !self
                .graph
                .function_templates
                .trailing_results
                .contains(&self.graph.binding(binding).entity)
        {
            let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
                return Err(Error::Invariant("template call without function type"));
            };
            let mut environment = Substitution::new(depth, &given);
            environment.context = Some(self.graph.binding(binding).entity);
            if self
                .substitute_signature_type(result, &mut environment)?
                .is_none()
            {
                return Ok(None);
            }
        }
        for (index, &argument) in args.iter().enumerate().take(count) {
            let pattern = self.graph.types.function_parameter(ty, index)?;
            let mut environment = Substitution::new(depth, &given);
            environment.context = Some(self.graph.binding(binding).entity);
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

    pub(super) fn deduce_overloaded_argument(
        &mut self,
        pattern: TypeId,
        set: super::function_resolution::OverloadId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
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
            let mut trial = arguments.to_vec();
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
            arguments.copy_from_slice(&found);
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
        environment.context = Some(self.graph.binding(binding).entity);
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

    // template_specializations.cpp::deduce_for_declaration: unlike call
    // deduction, every part of the declared type (including its result)
    // participates, and the completed specialization must have that type.
    fn deduce_function_declaration(
        &mut self,
        primary: BindingId,
        given: &[TypeId],
        declared: TypeId,
    ) -> Result<Option<BindingId>, Error> {
        let Some(head) = self.function_template_head(primary)? else {
            return Ok(None);
        };
        let count = self.graph.templates.parameters(head).len();
        if given.len() > count {
            return Ok(None);
        }
        let initial = || {
            let mut arguments = vec![None; count];
            for (slot, &argument) in arguments.iter_mut().zip(given) {
                *slot = Some(argument);
            }
            arguments
        };
        let mut arguments = initial();
        let depth = self.graph.templates.depth(head);
        let declared = self.graph.types.canonical_function(declared)?;
        let source = self
            .graph
            .binding(primary)
            .ty
            .ok_or(Error::Invariant("template declaration without type"))?;
        let pattern = self.target_deduction_pattern(source, declared, depth)?;
        if !self.deduce_signature_type(pattern, declared, depth, &mut arguments, false)? {
            if given.is_empty() {
                return Ok(None);
            }
            arguments = initial();
            let mut environment = Substitution::new(depth, &arguments);
            let Some(pattern) = self.substitute_signature_type(source, &mut environment)? else {
                return Ok(None);
            };
            let pattern = self.target_deduction_pattern(pattern, declared, depth)?;
            if !self.deduce_signature_type(pattern, declared, depth, &mut arguments, false)? {
                return Ok(None);
            }
        }
        let Some(binding) = self.complete_signature_in(primary, &mut arguments, false)? else {
            return Ok(None);
        };
        Ok((self.graph.binding(binding).ty == Some(declared)).then_some(binding))
    }

    pub(super) fn declared_function_specialization(
        &mut self,
        info: &super::declarators::DeclaratorInfo,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<BindingId>, Error> {
        let component = self
            .tree
            .name_lists
            .get(
                self.tree
                    .names
                    .get(
                        info.syntax_name
                            .ok_or(Error::Invariant("specialization declaration without name"))?,
                    )
                    .components,
            )
            .last()
            .copied()
            .ok_or(Error::Invariant("empty specialization name"))?;
        let mut given = Vec::new();
        for index in 0..self
            .tree
            .template_arguments
            .get(component.arguments.unwrap_or_default())
            .len()
        {
            let argument = self
                .tree
                .template_arguments
                .get(component.arguments.unwrap_or_default())[index];
            let ast::TemplateArgumentKind::Type(ty) = argument.kind else {
                return Err(Error::Missing("non-type function template arguments"));
            };
            if argument.pack {
                return Err(Error::Missing("function template argument expansion"));
            }
            let ty = self.type_syntax(ty, scope)?;
            given.push(self.graph.types.canonical_function(ty)?);
        }
        let key = Self::expression_name(component.kind)?;
        self.lookup_location = location;
        let found = if let Some(home) = info.home {
            self.qualified_result(home, key, super::lookup::Filter::Ordinary)?
        } else {
            self.unqualified_result(scope, key, super::lookup::Filter::Ordinary)?
        };
        let mut selected = None;
        for &primary in found.functions.as_slice() {
            let Some(candidate) = self.deduce_function_declaration(primary, &given, info.ty)?
            else {
                continue;
            };
            if selected == Some(candidate) {
                continue;
            }
            if let Some(previous) = selected {
                let previous = self
                    .graph
                    .function_templates
                    .primary(self.graph.binding(previous).entity)
                    .ok_or(Error::Invariant("declared specialization without primary"))?;
                let count = Some(self.graph.types.function_parameter_count(info.ty)?);
                let forward = self.template_at_least_as_specialized(primary, previous, count)?;
                let backward = self.template_at_least_as_specialized(previous, primary, count)?;
                if forward == backward {
                    return Err(Error::at(
                        location,
                        "a declaration matches several function templates",
                    ));
                }
                if !forward {
                    continue;
                }
            }
            selected = Some(candidate);
        }
        Ok(selected)
    }

    pub(super) fn demand_specialization_body(&mut self, binding: BindingId) {
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
            self.graph
                .function_templates
                .bodies
                .demand(self.graph.binding(binding).entity);
        }
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
        self.graph.entity(primary).template_head.is_some()
            && self.graph.function_default(primary, index).is_some()
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
                let Some(a) = self.substitute_signature_type(a, &mut environment)? else {
                    return Ok(false);
                };
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
            self.order_function_types(from, to, depth, &mut arguments, &mut environment)
        }
    }

    fn order_function_types(
        &mut self,
        from: TypeId,
        to: TypeId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
        environment: &mut Substitution<'_>,
    ) -> Result<bool, Error> {
        let TypeKind::Function {
            result: from_result,
            parameters: from_parameters,
            variadic,
            cv,
            reference,
        } = self.graph.types.get(from).kind.clone()
        else {
            return Err(Error::Invariant("ordering nonfunction"));
        };
        let TypeKind::Function {
            result: to_result,
            parameters: to_parameters,
            variadic: to_variadic,
            cv: to_cv,
            reference: to_reference,
        } = self.graph.types.get(to).kind.clone()
        else {
            return Err(Error::Invariant("ordering nonfunction"));
        };
        if from_parameters.len() != to_parameters.len()
            || variadic != to_variadic
            || cv != to_cv
            || reference != to_reference
        {
            return Ok(false);
        }
        for (from, to) in from_parameters.into_iter().zip(to_parameters) {
            let Some(from) = self.substitute_signature_type(from, environment)? else {
                return Ok(false);
            };
            if !self.deduce_signature_type(to, from, depth, arguments, false)? {
                return Ok(false);
            }
        }
        if self.names_operand(from_result) || self.names_operand(to_result) {
            return Ok(true);
        }
        let Some(from) = self.substitute_signature_type(from_result, environment)? else {
            return Ok(false);
        };
        self.deduce_signature_type(to_result, from, depth, arguments, false)
    }

    fn names_operand(&self, root: TypeId) -> bool {
        let mut work = vec![root];
        let mut visited = HashSet::new();
        while let Some(ty) = work.pop() {
            if !visited.insert(ty) {
                continue;
            }
            match &self.graph.types.get(ty).kind {
                TypeKind::TemplateParameter { entity, .. }
                    if self.graph.template_operands.pattern(*entity).is_some()
                        || self.graph.dependent_members.get(*entity).is_some() =>
                {
                    return true;
                }
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.graph.class_templates.instance(*entity) {
                        work.extend(&instance.arguments);
                    }
                }
                TypeKind::Pointer(ty) | TypeKind::Reference { element: ty, .. } => work.push(*ty),
                TypeKind::Array {
                    element, operand, ..
                } => {
                    if operand.is_some() {
                        return true;
                    }
                    work.push(*element);
                }
                TypeKind::MemberPointer { owner, element } => work.extend([*owner, *element]),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters);
                }
                _ => (),
            }
        }
        false
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
    fn operand_search_handles_shared_function_type_dags() {
        let source =
            SourceBuffer::new(b"template<class T>auto f(T x)->decltype(x+1);".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"operand-dag").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let binding = analyzer
            .graph
            .functions_at(analyzer.graph.global, name.into())
            .next()
            .unwrap();
        let pattern = analyzer.graph.binding(binding).ty.unwrap();
        assert!(analyzer.names_operand(pattern));
        let mut ty = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Int))
            .unwrap();
        // The unfolded tree has 3^48 leaves; the owned DAG has 97 nodes.
        for _ in 0..48 {
            ty = analyzer
                .graph
                .types
                .intern(
                    TypeKind::Function {
                        result: ty,
                        parameters: vec![ty, ty],
                        variadic: false,
                        cv: Cv::default(),
                        reference: None,
                    },
                    Cv::default(),
                )
                .unwrap();
            ty = analyzer
                .graph
                .types
                .intern(TypeKind::Pointer(ty), Cv::default())
                .unwrap();
        }
        assert!(!analyzer.names_operand(ty));
        let combined = analyzer
            .graph
            .types
            .intern(
                TypeKind::Function {
                    result: pattern,
                    parameters: vec![ty, ty],
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )
            .unwrap();
        assert!(analyzer.names_operand(combined));
    }
}
