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
use crate::{support::source::SourceOffset, syntax::ast};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

#[derive(Clone, PartialEq, Eq, Hash)]
struct SpecializationKey {
    primary: EntityId,
    arguments: Vec<TypeId>,
}

#[derive(Clone, Copy)]
pub(super) enum TemplateOrdering {
    FunctionType,
    Call(usize),
    Operator(usize),
}

/// A borrowed function parameter sequence with an optional implicit object.
/// The published signature itself never acquires a synthetic parameter.
#[derive(Clone, Copy)]
struct OrderingParameters {
    function: TypeId,
    object: Option<TypeId>,
}

#[derive(Default)]
pub(super) struct FunctionTemplates {
    pub(super) conditions: super::explicit_conditions::Conditions,
    pub(super) specializations: super::explicit_specializations::Specializations,
    signatures: HashMap<SpecializationKey, BindingId>,
    active_signatures: HashSet<(SpecializationKey, usize)>,
    primaries: HashMap<EntityId, BindingId>,
    pub(super) bodies: super::template_bodies::Bodies,
    pub(super) argument_defaults: super::template_defaults::Defaults,
    pub(super) instantiations: super::explicit_instantiations::Instantiations,
    pub(super) arguments: HashMap<EntityId, Vec<TypeId>>,
    explicit: HashMap<BindingId, Vec<Option<TypeId>>>,
    defaults: HashMap<(HeadId, usize, Vec<TypeId>), TypeId>,
    demanded_bodies: HashSet<EntityId>,
    trailing_results: HashSet<EntityId>,
}
impl FunctionTemplates {
    pub(super) fn trailing_result(&self, entity: EntityId) -> bool {
        self.trailing_results.contains(&entity)
    }
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
pub(super) struct PackSelection<'a> {
    pub elements: &'a [Option<usize>],
    pub bound_element: Option<usize>,
}
pub(super) struct Substitution<'a> {
    pub(super) depth: u32,
    pub(super) arguments: &'a [Option<TypeId>],
    memo: HashMap<TypeId, Option<TypeId>>,
    pub(super) context: Option<EntityId>,
    pub(super) selection: Option<&'a PackSelection<'a>>,
    pub(super) failure: Option<&'static str>,
    pub(super) failure_location: SourceOffset,
}
impl<'a> Substitution<'a> {
    pub(super) fn new(depth: u32, arguments: &'a [Option<TypeId>]) -> Self {
        Self {
            depth,
            arguments,
            memo: HashMap::new(),
            context: None,
            selection: None,
            failure: None,
            failure_location: SourceOffset(0),
        }
    }
    pub(super) fn reject<T>(&mut self, message: &'static str) -> Option<T> {
        self.reject_at(SourceOffset(0), message)
    }
    pub(super) fn reject_at<T>(
        &mut self,
        location: SourceOffset,
        message: &'static str,
    ) -> Option<T> {
        if self.failure.is_none() {
            self.failure = Some(message);
            self.failure_location = location;
        }
        None
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
        let parameters = self
            .substitute_pack_list(&parameters, &mut environment)?
            .ok_or(Error::Missing("dependent exception prototype parameter"))?;
        let mut objects = Vec::with_capacity(parameters.len());
        for parameter in parameters {
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

    pub(super) fn function_template_head(
        &self,
        binding: BindingId,
    ) -> Result<Option<HeadId>, Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        let Some(head) = entity.template_head else {
            return Ok(None);
        };
        if entity
            .function
            .and_then(|properties| properties.special)
            .is_some_and(|name| {
                !matches!(
                    name,
                    super::function_names::SpecialName::Operator { .. }
                        | super::function_names::SpecialName::Literal(_)
                        | super::function_names::SpecialName::Conversion
                )
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
            if found.binding.is_none() {
                return Err(Error::at(component.location, "undeclared name"));
            }
            return Err(Error::Missing(
                "class or alias template specialization lookup",
            ));
        }
        let head = self.sole_template_head(found.functions.as_slice())?;
        let given = self.explicit_function_arguments(syntax, scope, component.location, head)?;
        self.specialize_explicit_functions(component, found, &given, call)
    }
    pub(super) fn explicit_function_arguments(
        &mut self,
        syntax: crate::syntax::storage::List<ast::TemplateArgument>,
        scope: ScopeId,
        location: SourceOffset,
        head: Option<HeadId>,
    ) -> Result<Vec<TypeId>, Error> {
        let mut given = Vec::new();
        for syntax_index in 0..self.tree.template_arguments.get(syntax).len() {
            let syntax_item = self.tree.template_arguments.get(syntax)[syntax_index];
            let argument = &syntax_item;
            if argument.pack {
                return Err(Error::Missing("function template argument expansion"));
            }
            let target = if let Some(head) = head
                && syntax_index < self.graph.templates.parameters(head).len()
            {
                self.argument_target(head, syntax_index, &given)?.known()
            } else {
                None
            };
            let template = head
                .and_then(|head| self.template_argument_parameter(head, syntax_index))
                .map(|info| info.inner.is_some());
            let unknown = target.is_none()
                && head
                    .and_then(|head| self.template_argument_parameter(head, syntax_index))
                    .is_none_or(|info| info.value_type.is_some());
            let ty =
                self.template_argument_at(*argument, scope, location, target, template, unknown)?;
            given.push(self.graph.types.canonical_function(ty)?);
        }
        Ok(given)
    }
    pub(super) fn sole_template_head(
        &self,
        functions: &[BindingId],
    ) -> Result<Option<HeadId>, Error> {
        let mut found = None;
        for &binding in functions {
            let Some(head) = self
                .graph
                .entity(self.graph.binding(binding).entity)
                .template_head
            else {
                continue;
            };
            if let Some(previous) = found {
                let first = self.graph.templates.parameters(previous);
                let second = self.graph.templates.parameters(head);
                for (&a, &b) in first.iter().zip(second) {
                    let a = self
                        .graph
                        .entity(a)
                        .template_parameter
                        .ok_or(Error::Invariant("head without parameter"))?;
                    let b = self
                        .graph
                        .entity(b)
                        .template_parameter
                        .ok_or(Error::Invariant("head without parameter"))?;
                    if !a.same_argument_kind(b) {
                        return Ok(None);
                    }
                }
                if second.len() <= first.len() {
                    continue;
                }
            }
            found = Some(head);
        }
        Ok(found)
    }
    pub(super) fn specialize_explicit_functions(
        &mut self,
        component: ast::NameComponent,
        found: &mut LookupResult,
        given: &[TypeId],
        call: bool,
    ) -> Result<(), Error> {
        let mut specializations = FunctionCandidates::default();
        for &primary in found.functions.as_slice() {
            let Some(head) = self.function_template_head(primary)? else {
                continue;
            };
            let count = self.graph.templates.parameters(head).len();
            if given.len() > count
                && !(0..count).any(|index| self.argument_parameter_pack(head, index))
            {
                continue;
            }
            let Some(converted) =
                self.match_explicit_template_arguments(head, given, component.location)?
            else {
                continue;
            };
            let mut arguments = vec![None; count];
            for (slot, &argument) in arguments.iter_mut().zip(&converted) {
                *slot = Some(argument);
            }
            let partial = (converted.len()..count)
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
        let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(ty).kind else {
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
                let mut argument = *argument;
                if let Some(Some(position)) = environment
                    .selection
                    .and_then(|selection| selection.elements.get(index))
                    && let TypeKind::Pack(elements) = &self.graph.types.get(argument).kind
                {
                    argument = *elements
                        .get(*position)
                        .ok_or(Error::Invariant("pack element outside argument"))?;
                }
                Some(self.graph.types.qualify(argument, cv)?)
            } else {
                Some(ty)
            };
            environment.memo.insert(ty, result);
            return Ok(result);
        }
        let incomplete = self.incomplete_class_observations;
        let kind = self.graph.types.get(ty).kind.clone();
        let result = match kind {
            TypeKind::Pack(elements) => self
                .substitute_pack_list(&elements, environment)?
                .map(|elements| self.graph.types.intern(TypeKind::Pack(elements), cv))
                .transpose()?,
            TypeKind::Expansion(pattern) => self
                .substitute_signature_type(pattern, environment)?
                .map(|pattern| self.graph.types.intern(TypeKind::Expansion(pattern), cv))
                .transpose()?,
            TypeKind::BoundPack { arguments, .. } => {
                if let Some(position) = environment
                    .selection
                    .and_then(|selection| selection.bound_element)
                {
                    let TypeKind::Pack(elements) = &self.graph.types.get(arguments).kind else {
                        return Err(Error::Invariant("bound pack without arguments"));
                    };
                    if let Some(&element) = elements.get(position) {
                        Some(self.graph.types.qualify(element, cv)?)
                    } else {
                        Some(ty)
                    }
                } else {
                    Some(ty)
                }
            }
            TypeKind::TemplateParameter {
                entity,
                template: true,
            } if self
                .graph
                .class_templates
                .dependent_names
                .get(entity)
                .is_some() =>
            {
                let name = self
                    .graph
                    .class_templates
                    .dependent_names
                    .get(entity)
                    .ok_or(Error::Invariant("lost template name"))?;
                self.substitute_template_name(name, environment)?
                    .map(|primary| {
                        let kind = if self
                            .graph
                            .class_templates
                            .dependent_names
                            .get(primary)
                            .is_some()
                        {
                            TypeKind::TemplateParameter {
                                entity: primary,
                                template: true,
                            }
                        } else {
                            TypeKind::Template(primary)
                        };
                        self.graph.types.intern(kind, cv)
                    })
                    .transpose()?
            }
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
            TypeKind::Class(entity)
                if self
                    .graph
                    .class_templates
                    .instance(entity)
                    .is_some_and(|instance| {
                        self.graph
                            .class_templates
                            .aliases
                            .owner(instance.primary)
                            .is_some()
                    }) =>
            {
                self.substitute_alias_type(entity, environment)?
                    .map(|ty| self.graph.types.qualify(ty, cv))
                    .transpose()?
            }
            TypeKind::Class(entity) if self.graph.class_templates.instance(entity).is_some() => {
                self.substitute_class_type(entity, environment)?
                    .map(|ty| self.graph.types.qualify(ty, cv))
                    .transpose()?
            }
            TypeKind::Vector { .. } => self.substitute_vector_type(ty, environment)?,
            TypeKind::Complex(element) => self
                .substitute_signature_type(element, environment)?
                .map(|element| self.graph.types.intern(TypeKind::Complex(element), cv))
                .transpose()?,
            TypeKind::Pointer(element) => {
                let Some(element) = self.substitute_signature_type(element, environment)? else {
                    return Ok(None);
                };
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Reference { .. }
                ) {
                    environment.reject("pointer to reference")
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
                    environment.reject("reference to void")
                } else {
                    Some(self.graph.types.reference(element, kind)?)
                }
            }
            TypeKind::MemberPointer { owner, element } => {
                let Some(owner) = self.substitute_signature_type(owner, environment)? else {
                    return Ok(None);
                };
                let owner = self.bare_type(owner)?;
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
                    environment.reject("an invalid pointer to member type")
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
                let failure = match self.graph.types.get(element).kind {
                    TypeKind::Reference { .. } => Some("array of references"),
                    TypeKind::Function { .. } => Some("array of functions"),
                    TypeKind::Scalar(Scalar::Void) => Some("array of void"),
                    TypeKind::Array {
                        bound: 0,
                        operand: None,
                        ..
                    } => Some("array of arrays of unknown bound"),
                    TypeKind::Class(class)
                        if !self.exception_dependent_type(element)
                            && self.graph.entity(class).complete
                            && self.graph.virtuals.abstract_class(class) =>
                    {
                        Some("an object of an abstract class type")
                    }
                    _ => None,
                };
                if let Some(message) = failure {
                    environment.reject(message)
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
        if self.incomplete_class_observations == incomplete {
            environment.memo.insert(ty, result);
        }
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
        let Some(parameters) = self.substitute_pack_list(&parameters, environment)? else {
            return Ok(None);
        };
        let mut substituted = Vec::with_capacity(parameters.len());
        for parameter in parameters {
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
            return Ok(environment.reject("a function type returning an array or a function, or with a parameter of type void"));
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

    pub(super) fn complete_signature(
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
        let mut earlier = Vec::with_capacity(arguments.len());
        for index in 0..arguments.len() {
            let ty = if let Some(ty) = arguments[index] {
                ty
            } else if self.argument_parameter_pack(head, index) {
                self.graph
                    .types
                    .intern(TypeKind::Pack(Vec::new()), Cv::default())?
            } else {
                let Some(default) = self.graph.templates.default_argument(head, index) else {
                    return Ok(None);
                };
                let key = (head, index, earlier.clone());
                if let Some(&ty) = self.graph.function_templates.defaults.get(&key) {
                    ty
                } else {
                    let pattern = if let Some(bound) = default.bound {
                        bound
                    } else {
                        let copy = self.copy_template_pattern(default.syntax)?;
                        // Form a dependent default's pattern in its declaration
                        // context before substituting this candidate's arguments.
                        self.template_depth += 1;
                        let saved = self.graph.lookup_limit.replace(default.limit);
                        let template = self
                            .template_argument_parameter(head, index)
                            .map(|info| info.inner.is_some());
                        let result = self.template_argument_at(
                            copy,
                            default.scope,
                            self.default_location(default),
                            None,
                            template,
                            false,
                        );
                        self.graph.lookup_limit = saved;
                        self.template_depth -= 1;
                        result?
                    };
                    let mut environment = Substitution::new(depth, &arguments[..index]);
                    environment.context =
                        check_access.then_some(self.graph.binding(primary).entity);
                    let Some(ty) = self.substitute_signature_type(pattern, &mut environment)?
                    else {
                        return Ok(None);
                    };
                    let ty = self.graph.types.canonical_function(ty)?;
                    self.graph.function_templates.defaults.insert(key, ty);
                    ty
                }
            };
            arguments[index] = Some(ty);
            earlier.push(ty);
        }
        // Frozen complete_deduction keeps newly supplied defaults in source
        // types while later defaults form; specialization converts them after.
        let Some(converted) =
            self.match_explicit_template_arguments(head, &earlier, SourceOffset(0))?
        else {
            return Ok(None);
        };
        for (slot, &ty) in arguments.iter_mut().zip(&converted) {
            *slot = Some(ty);
        }
        let canonical: Option<Vec<_>> = arguments.iter().copied().collect();
        let key = SpecializationKey {
            primary: self.graph.binding(primary).entity,
            arguments: canonical.ok_or(Error::Invariant("incomplete signature arguments"))?,
        };
        if let Some(&binding) = self.graph.function_templates.signatures.get(&key) {
            return Ok(Some(binding));
        }
        let pattern = self
            .graph
            .binding(primary)
            .ty
            .ok_or(Error::Invariant("template without function type"))?;
        // Re-entry from a class definition started by this substitution sees
        // that class's members so far. Only the same class-definition depth
        // makes an active signature circular (C++ ActiveSpecialization).
        let active = (key, self.graph.class_templates.definitions.active_depth());
        if !self
            .graph
            .function_templates
            .active_signatures
            .insert(active.clone())
        {
            return Ok(None);
        }
        let mut environment = Substitution::new(depth, arguments);
        environment.context = check_access.then_some(self.graph.binding(primary).entity);
        let leading = !self
            .graph
            .function_templates
            .trailing_results
            .contains(&self.graph.binding(primary).entity);
        let result = self.substitute_function_signature(pattern, &mut environment, leading);
        self.graph
            .function_templates
            .active_signatures
            .remove(&active);
        let key = active.0;
        let ty = match result {
            Ok(ty) => ty,
            Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => None,
            Err(error) => return Err(error),
        };
        let Some(ty) = ty else {
            return Ok(None);
        };
        let ty = self.graph.types.canonical_function(ty)?;
        // A nested formation may already have published this specialization.
        // Keep its identity, parameters and declaration-time facts.
        if let Some(&binding) = self.graph.function_templates.signatures.get(&key) {
            return Ok(Some(binding));
        }
        let explicitness = match self.specialized_explicitness(primary, depth, &key.arguments) {
            Ok(value) => value,
            Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => return Ok(None),
            Err(error) => return Err(error),
        };
        let origin = self.graph.binding(primary);
        let entity = self.graph.add_entity(Some(ty), origin.name, origin.key)?;
        self.graph
            .instantiation_levels
            .record(entity, self.instantiation_depth);
        let old = self.graph.entity(origin.entity);
        let properties = (
            old.function,
            old.availability,
            old.constexpr,
            old.home,
            old.scope,
        );
        let old_inline = old.inline;
        let special_name = old.special_name;
        let parameters = old.function_parameters;
        let member = old.member_function.map(|key| super::graph::MemberKey {
            signature: ty,
            shape: None,
            ..key
        });
        let special_definition = old.special_definition;
        let exceptions = old.exceptions.clone();
        let specialized = self.graph.entity_mut(entity);
        specialized.function = properties.0;
        specialized.availability = properties.1;
        specialized.constexpr = properties.2;
        specialized.home = properties.3;
        specialized.scope = properties.4;
        specialized.inline = old_inline;
        specialized.special_name = special_name;
        specialized.member_function = member;
        specialized.special_definition = special_definition;
        specialized.explicitness = explicitness;
        // Declaration facts belong to the specialization's formation point.
        // A later primary redeclaration must not rename its prototype; a
        // demanded body installs its own copied parameters separately.
        specialized.function_parameters = parameters;
        specialized.exceptions = exceptions;
        specialized.definition_emission = DefinitionEmission::Demanded;
        self.specialization_defaults(entity, primary, pattern, ty)?;
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
            .insert(key, binding);
        self.graph.work.specializations += 1;
        Ok(Some(binding))
    }

    pub(super) fn deduce_signature_type(
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
        if matches!(
            self.graph.types.get(pattern).kind,
            TypeKind::BoundPack { .. }
        ) {
            return self.deduce_bound_pack(pattern, actual, depth, arguments, extra_cv);
        }
        if let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(pattern).kind
            && (self.graph.template_operands.pattern(entity).is_some()
                || self
                    .graph
                    .class_templates
                    .dependent_names
                    .get(entity)
                    .is_some()
                || self.graph.dependent_members.get(entity).is_some())
        {
            return Ok(true); // Dependent operands and qualified members are non-deduced.
        }
        // Array qualifiers live on their elements. Structural matching reaches
        // those elements below; a parameter deducing an entire array instead
        // observes the element qualifiers, as C++ without_cv does.
        let pcv = self.graph.types.get(pattern).cv.0;
        let acv = self.graph.types.get(actual).cv.0;
        let parameter = if matches!(self.graph.types.get(pattern).kind, TypeKind::Auto) {
            Some(0)
        } else {
            self.template_parameter_index(pattern, depth)
        };
        if let Some(index) = parameter {
            if self.template_argument_kind(pattern) && !self.template_argument_kind(actual) {
                return Ok(false);
            }
            let acv = self.type_cv(actual).0;
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
        let aliased_pattern = self.aliased_type(pattern)?;
        let aliased_actual = self.aliased_type(actual)?;
        if aliased_pattern != pattern || aliased_actual != actual {
            return self.deduce_signature_type(
                aliased_pattern,
                aliased_actual,
                depth,
                arguments,
                extra_cv,
            );
        }
        if pcv != acv && !(extra_cv && acv & !pcv == 0) {
            return Ok(false);
        }
        match (
            self.graph.types.get(pattern).kind.clone(),
            self.graph.types.get(actual).kind.clone(),
        ) {
            (TypeKind::Pack(pattern), TypeKind::Pack(actual)) => {
                self.deduce_type_list(&pattern, &actual, depth, arguments)
            }
            (TypeKind::Class(p), TypeKind::Class(a)) => {
                if p == a {
                    return Ok(true);
                }
                if let Some(instance) = self.graph.class_templates.instance(p)
                    && let Some(name) = self
                        .graph
                        .class_templates
                        .template_entities
                        .name(instance.primary)
                {
                    let patterns = instance.arguments.clone();
                    return self.deduce_template_id(name, &patterns, a, depth, arguments);
                }
                match (
                    self.graph.class_templates.instance(p),
                    self.graph.class_templates.instance(a),
                ) {
                    (Some(p), Some(a)) if p.primary == a.primary => {
                        let patterns = p.arguments.clone();
                        let actual = a.arguments.clone();
                        self.deduce_type_list(&patterns, &actual, depth, arguments)
                    }
                    _ => Ok(pattern == actual),
                }
            }
            (TypeKind::Vector { .. }, TypeKind::Vector { .. }) => {
                self.deduce_vector_type(pattern, actual, depth, arguments)
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
                    operand: actual_operand,
                },
            ) => {
                if let Some(operand) = operand {
                    if actual_operand.is_none()
                        && ab != 0
                        && let Some(index) = self.named_bound_parameter(operand, depth)
                        && let Some(slot) = arguments.get_mut(index)
                    {
                        let ty = self.graph.types.scalar(Scalar::Basic(
                            crate::types::FundamentalType::UnsignedLongInt,
                        ))?;
                        let value = self
                            .graph
                            .types
                            .intern(TypeKind::Value { ty, bits: ab }, Cv::default())?;
                        if slot.is_some_and(|known| !self.same_template_value(known, value)) {
                            return Ok(false);
                        }
                        slot.get_or_insert(value);
                    }
                } else if pb != ab {
                    return Ok(false);
                }
                self.deduce_signature_type(p, a, depth, arguments, extra_cv)
            }
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
                if pv != av || pc != ac || pr != ar {
                    return Ok(false);
                }
                if !self.deduce_function_parameters(&pp, &ap, depth, arguments)? {
                    return Ok(false);
                }
                self.deduce_signature_type(p, a, depth, arguments, false)
            }
            (
                TypeKind::Value { .. } | TypeKind::Address { .. },
                TypeKind::Value { .. } | TypeKind::Address { .. },
            ) => Ok(self.same_template_value(pattern, actual)),
            // A parameter outside this deduction's depth is a fixed identity,
            // as in the early parameter branches of C++ deduce_type.
            (TypeKind::TemplateParameter { .. }, _) => Ok(pattern == actual),
            // Qualification was checked above. Terminal matching compares
            // unqualified identities, including a fixed array element when
            // the bound is the only part that participates in deduction.
            _ => Ok(self.bare_type(pattern)? == self.bare_type(actual)?),
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
        let reference = matches!(
            self.graph.types.get(pattern).kind,
            TypeKind::Reference { .. }
        );
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
        if self.deduce_signature_type(pattern, actual, depth, arguments, true)? {
            return Ok(true);
        }
        if !reference
            && let (TypeKind::Pointer(p), TypeKind::Pointer(a)) = (
                self.graph.types.get(pattern).kind.clone(),
                self.graph.types.get(actual).kind.clone(),
            )
            && self.deduce_from_base(p, a, depth, arguments)?
        {
            return Ok(true);
        }
        self.deduce_from_base(pattern, actual, depth, arguments)
    }

    // function_templates.cpp::deduce_from_base: deduction ignores base access
    // and subobject ambiguity; the selected call's conversion checks those.
    // Each base identity is tried once, and matching branches stop there.
    fn deduce_from_base(
        &mut self,
        pattern: TypeId,
        actual: TypeId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        let Some(class) = self.graph.class_of(actual) else {
            return Ok(false);
        };
        if self.graph.class_of(pattern).is_none()
            || !self.graph.dependent_type(pattern)
            || !self.complete_class(class)?
        {
            return Ok(false);
        }
        let pattern = self.bare_type(pattern)?;
        let mut work = vec![class];
        let mut seen = HashSet::new();
        let mut trial = arguments.to_vec();
        let mut found: Option<Vec<Option<TypeId>>> = None;
        let mut unique = true;
        while let Some(current) = work.pop() {
            for index in 0..self.graph.entity(current).bases.len() {
                let base = self.graph.entity(current).bases[index].class;
                if !seen.insert(base) {
                    continue;
                }
                let ty = self
                    .graph
                    .entity(base)
                    .ty
                    .ok_or(Error::Invariant("base class without type"))?;
                trial.copy_from_slice(arguments);
                if self.deduce_signature_type(pattern, ty, depth, &mut trial, false)? {
                    if let Some(previous) = &mut found {
                        unique &= *previous == trial;
                        previous.copy_from_slice(&trial);
                    } else {
                        found = Some(trial.clone());
                    }
                } else {
                    work.push(base);
                }
            }
        }
        if let Some(found) = found
            && unique
        {
            arguments.copy_from_slice(&found);
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn signature_arguments(
        &self,
        binding: BindingId,
        head: HeadId,
    ) -> Vec<Option<TypeId>> {
        self.graph
            .function_templates
            .explicit
            .get(&binding)
            .cloned()
            .unwrap_or_else(|| vec![None; self.graph.templates.parameters(head).len()])
    }
    pub(super) fn needs_deduction(&mut self, ty: TypeId, depth: u32) -> Result<bool, Error> {
        // deduce_for_call tests the aliased type. A shell retains discarded
        // arguments for substitution/SFINAE, but those arguments cannot make
        // its nondependent target participate in call deduction (CWG 1558).
        let ty = self.aliased_type(ty)?;
        if !self.graph.dependent_type(ty) {
            return Ok(false);
        }
        let ty = self.graph.types.canonical_function(ty)?;
        let mut work = vec![ty];
        let mut seen = HashSet::new();
        while let Some(ty) = work.pop() {
            if !seen.insert(ty) {
                continue;
            }
            if self.template_parameter_index(ty, depth).is_some() {
                return Ok(true);
            }
            match &self.graph.types.get(ty).kind {
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.graph.class_templates.instance(*entity) {
                        // TT<int> deduces TT even though none of its written
                        // arguments names a parameter (C++ Deducible).
                        work.extend(
                            self.graph
                                .class_templates
                                .template_entities
                                .name(instance.primary),
                        );
                        work.extend(&instance.arguments);
                    }
                }
                TypeKind::Pointer(element)
                | TypeKind::Expansion(element)
                | TypeKind::Reference { element, .. } => work.push(*element),
                TypeKind::Array {
                    element, operand, ..
                }
                | TypeKind::Vector {
                    element, operand, ..
                } => {
                    // Deducible includes a bound that directly names a
                    // non-type parameter, even for a fixed element type.
                    // Other bound expressions remain non-deduced contexts.
                    if operand
                        .is_some_and(|bound| self.named_bound_parameter(bound, depth).is_some())
                    {
                        return Ok(true);
                    }
                    work.push(*element);
                }
                TypeKind::Pack(elements) => work.extend(elements.iter().copied()),
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
        Ok(false)
    }
    pub(super) fn deduce_function_call(
        &mut self,
        binding: BindingId,
        args: &[ast::ExprId],
        implicit_postfix: bool,
        prefix: Option<super::expression_facts::ExprFacts>,
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
        let ty = self.graph.types.canonical_function(ty)?;
        let count = self.graph.types.function_parameter_count(ty)?;
        let TypeKind::Function { variadic, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("template is no function"));
        };
        if self
            .graph
            .templates
            .parameters(head)
            .iter()
            .any(|&parameter| {
                self.graph
                    .entity(parameter)
                    .template_parameter
                    .is_some_and(super::templates::ParameterInfo::is_pack)
            })
            || (0..count).any(|index| {
                self.graph
                    .types
                    .function_parameter(ty, index)
                    .is_ok_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)))
            })
        {
            return self.deduce_pack_function_call(binding, head, args, prefix);
        }
        // Postfix operator syntax supplies its final int argument implicitly.
        // Deduce only from the written operand; the operator owner validates
        // that final parameter after forming the specialization.
        let offset = usize::from(prefix.is_some());
        let supplied = args.len() + offset + usize::from(implicit_postfix);
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
        for index in 0..(args.len() + offset).min(count) {
            let pattern = self.graph.types.function_parameter(ty, index)?;
            let mut environment = Substitution::new(depth, &given);
            environment.context = Some(self.graph.binding(binding).entity);
            let Some(pattern) = self.substitute_signature_type(pattern, &mut environment)? else {
                return Ok(None);
            };
            if !self.needs_deduction(pattern, depth)? {
                continue;
            }
            // Only initializer-list parameters deduce from braced elements.
            if index >= offset
                && matches!(
                    self.tree.expressions.get(args[index - offset]).kind,
                    ast::ExprKind::Braced { .. }
                )
            {
                if !self.deduce_list_elements(
                    pattern,
                    args[index - offset],
                    depth,
                    &mut arguments,
                )? {
                    return Ok(None);
                }
                continue;
            }
            let facts = if index < offset {
                prefix.ok_or(Error::Invariant("missing implicit call argument"))?
            } else {
                self.graph.expression_facts.get(args[index - offset])?
            };
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
        if !self.deduced_values_typed(head, ty, &arguments)? {
            return Ok(None);
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
        if !self.deduced_values_typed(head, pattern, &arguments)? {
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
        let Some(converted) =
            self.match_explicit_template_arguments(head, given, SourceOffset(0))?
        else {
            return Ok(None);
        };
        let initial = || {
            let mut arguments = vec![None; count];
            for (slot, &argument) in arguments.iter_mut().zip(&converted) {
                *slot = Some(argument);
            }
            arguments
        };
        let mut arguments = initial();
        // A declaration deduces the whole pack, including any explicitly
        // supplied prefix (14.8.1p9). Verify that prefix after deduction.
        for (index, slot) in arguments.iter_mut().enumerate().take(converted.len()) {
            if self.argument_parameter_pack(head, index)
                && slot.is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Pack(_)))
            {
                *slot = None;
            }
        }
        let depth = self.graph.templates.depth(head);
        let declared = self.graph.types.canonical_function(declared)?;
        let source = self
            .graph
            .binding(primary)
            .ty
            .ok_or(Error::Invariant("template declaration without type"))?;
        let pattern = self.target_deduction_pattern(source, declared, depth)?;
        let mut deduced =
            self.deduce_signature_type(pattern, declared, depth, &mut arguments, false)?;
        for (index, &given) in converted.iter().enumerate() {
            if !deduced || !self.argument_parameter_pack(head, index) {
                continue;
            }
            let TypeKind::Pack(prefix) = &self.graph.types.get(given).kind else {
                continue;
            };
            deduced = arguments[index].is_some_and(|found| {
                self.graph
                    .types
                    .pack_elements(found)
                    .is_ok_and(|elements| elements.starts_with(prefix))
            });
        }
        if !deduced {
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
        if !self.deduced_values_typed(head, source, &arguments)? {
            return Ok(None);
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
            if argument.pack {
                return Err(Error::Missing("function template argument expansion"));
            }
            let ty = self.template_argument(argument, scope, component.location)?;
            given.push(self.graph.types.canonical_function(ty)?);
        }
        self.lookup_location = location;
        let functions = if let Some((class, super::graph::MemberKind::Constructor)) = info
            .home
            .map(|home| self.declared_member(info, home))
            .transpose()?
            .flatten()
        {
            // C++ uses its constructor lookup category. Rust keeps this indexed
            // sequence on the owning class, separate from the injected name.
            self.graph
                .entity(class)
                .declared_constructors
                .iter()
                .rev()
                .copied()
                .collect()
        } else {
            let key = if self.conversion_declarator(info) {
                super::function_names::ValueKey::Special(
                    super::function_names::SpecialName::Conversion,
                )
            } else if component.arguments.is_none() {
                self.declaration_key(info)?
            } else {
                Self::expression_name(component.kind)?
            };
            if let Some(home) = info.home {
                self.qualified_result(home, key, super::lookup::Filter::Ordinary)?
                    .functions
            } else {
                self.unqualified_result(scope, key, super::lookup::Filter::Ordinary)?
                    .functions
            }
        };
        let mut selected = None;
        for &primary in functions.as_slice() {
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
                let count =
                    TemplateOrdering::Call(self.graph.types.function_parameter_count(info.ty)?);
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
            && !self
                .graph
                .function_templates
                .specializations
                .contains(self.graph.binding(binding).entity)
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
        self.graph.function_default(entity, index).is_some()
            || self
                .graph
                .function_templates
                .argument_defaults
                .pattern(entity, index)
                .is_some_and(|default| {
                    default.added_at.is_none_or(|ordinal| {
                        self.graph.lookup_limit.is_none_or(|limit| ordinal < limit)
                    })
                })
    }

    pub(super) fn template_at_least_as_specialized(
        &mut self,
        first: BindingId,
        second: BindingId,
        form: TemplateOrdering,
    ) -> Result<bool, Error> {
        let Some(from_head) = self.function_template_head(first)? else {
            return Ok(false);
        };
        let Some(to_head) = self.function_template_head(second)? else {
            return Ok(false);
        };
        let unique = self.unique_partial_arguments(from_head)?;
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
        match form {
            TemplateOrdering::FunctionType => {
                self.order_function_types(from, to, depth, &mut arguments, &mut environment)
            }
            TemplateOrdering::Call(count) | TemplateOrdering::Operator(count) => {
                let (from, to) = self.ordering_parameters(first, second, form)?;
                self.order_call_parameters(from, to, count, depth, &mut arguments, &mut environment)
            }
        }
    }

    fn ordering_parameters(
        &mut self,
        first: BindingId,
        second: BindingId,
        form: TemplateOrdering,
    ) -> Result<(OrderingParameters, OrderingParameters), Error> {
        let inserting = matches!(form, TemplateOrdering::Operator(_))
            && self.member_function_owner(first).is_some()
                != self.member_function_owner(second).is_some();
        Ok((
            self.ordering_parameter_sequence(first, inserting)?,
            self.ordering_parameter_sequence(second, inserting)?,
        ))
    }

    fn ordering_parameter_sequence(
        &mut self,
        binding: BindingId,
        inserting: bool,
    ) -> Result<OrderingParameters, Error> {
        let function = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("ordering without function type"))?;
        let object = if let Some(owner) = self.member_function_owner(binding).filter(|_| inserting)
        {
            let TypeKind::Function { cv, reference, .. } = self.graph.types.get(function).kind
            else {
                return Err(Error::Invariant("ordering nonfunction"));
            };
            let owner = self.graph.types.qualify(owner, cv)?;
            let kind = reference.unwrap_or(RefKind::Lvalue);
            Some(self.graph.types.reference(owner, kind)?)
        } else {
            None
        };
        Ok(OrderingParameters { function, object })
    }

    fn ordering_parameter(
        &self,
        parameters: OrderingParameters,
        index: usize,
    ) -> Result<TypeId, Error> {
        if let Some(object) = parameters.object {
            if index == 0 {
                return Ok(object);
            }
            return self
                .graph
                .types
                .function_parameter(parameters.function, index - 1);
        }
        self.graph
            .types
            .function_parameter(parameters.function, index)
    }

    fn order_call_parameters(
        &mut self,
        from: OrderingParameters,
        to: OrderingParameters,
        count: usize,
        depth: u32,
        arguments: &mut [Option<TypeId>],
        environment: &mut Substitution<'_>,
    ) -> Result<bool, Error> {
        let from_count = self.graph.types.function_parameter_count(from.function)?
            + usize::from(from.object.is_some());
        let to_count = self.graph.types.function_parameter_count(to.function)?
            + usize::from(to.object.is_some());
        let from_pack = from_count != 0
            && matches!(
                self.graph
                    .types
                    .get(self.ordering_parameter(from, from_count - 1)?)
                    .kind,
                TypeKind::Expansion(_)
            );
        let to_pack = to_count != 0
            && matches!(
                self.graph
                    .types
                    .get(self.ordering_parameter(to, to_count - 1)?)
                    .kind,
                TypeKind::Expansion(_)
            );
        let from_fixed = from_count - usize::from(from_pack);
        let to_fixed = to_count - usize::from(to_pack);
        let mut packed = Vec::new();
        for index in 0..count {
            if index >= from_fixed && !from_pack || index >= to_fixed && !to_pack {
                break;
            }
            let from_expansion = index >= from_fixed;
            let mut parameter =
                self.ordering_parameter(from, if from_expansion { from_fixed } else { index })?;
            if let TypeKind::Expansion(pattern) = self.graph.types.get(parameter).kind {
                parameter = pattern;
            }
            let Some(mut actual) = self.substitute_signature_type(parameter, environment)? else {
                return Ok(false);
            };
            if let TypeKind::Reference { element, .. } = self.graph.types.get(actual).kind {
                actual = element;
            }
            let actual = self.bare_type(actual)?;
            if index >= to_fixed {
                packed.push(actual);
                continue;
            }
            if from_expansion {
                return Ok(false);
            }
            let mut pattern = self.ordering_parameter(to, index)?;
            if let TypeKind::Reference { element, .. } = self.graph.types.get(pattern).kind {
                pattern = element;
            }
            let pattern = self.bare_type(pattern)?;
            if !self.deduce_signature_type(pattern, actual, depth, arguments, false)? {
                return Ok(false);
            }
        }
        if packed.is_empty() {
            return Ok(true);
        }
        let mut pattern = self.ordering_parameter(to, to_count - 1)?;
        if let TypeKind::Expansion(element) = self.graph.types.get(pattern).kind {
            pattern = element;
        }
        if let TypeKind::Reference { element, .. } = self.graph.types.get(pattern).kind {
            pattern = element;
        }
        let pattern = self.bare_type(pattern)?;
        self.deduce_pack_types(pattern, &packed, depth, arguments)
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
        if variadic != to_variadic || cv != to_cv || reference != to_reference {
            return Ok(false);
        }
        let to_pack = to_parameters
            .last()
            .is_some_and(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)));
        let mut given = Vec::with_capacity(from_parameters.len());
        let count = from_parameters.len();
        for (index, parameter) in from_parameters.into_iter().enumerate() {
            let expansion = matches!(self.graph.types.get(parameter).kind, TypeKind::Expansion(_));
            if expansion && index + 1 == count && index >= to_parameters.len() && !to_pack {
                break;
            }
            let pattern = if let TypeKind::Expansion(pattern) = self.graph.types.get(parameter).kind
            {
                pattern
            } else {
                parameter
            };
            let Some(actual) = self.substitute_signature_type(pattern, environment)? else {
                return Ok(false);
            };
            given.push(if expansion {
                self.graph
                    .types
                    .intern(TypeKind::Expansion(actual), Cv::default())?
            } else {
                actual
            });
        }
        if !self.deduce_function_parameters(&to_parameters, &given, depth, arguments)? {
            return Ok(false);
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
                }
                | TypeKind::Vector {
                    element, operand, ..
                } => {
                    if operand.is_some() {
                        return true;
                    }
                    work.push(*element);
                }
                TypeKind::Pack(elements) => work.extend(elements.iter().copied()),
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
        &mut self,
        first: BindingId,
        second: BindingId,
        count: usize,
        form: TemplateOrdering,
    ) -> Result<Ordering, Error> {
        let (a, b) = self.ordering_parameters(first, second, form)?;
        let count = count
            .min(
                self.graph.types.function_parameter_count(a.function)?
                    + usize::from(a.object.is_some()),
            )
            .min(
                self.graph.types.function_parameter_count(b.function)?
                    + usize::from(b.object.is_some()),
            );
        let mut order = Ordering::Equal;
        for index in 0..count {
            let a = self.ordering_parameter(a, index)?;
            let b = self.ordering_parameter(b, index)?;
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

    fn trailing_pack_unused(
        &self,
        primary: BindingId,
        other: BindingId,
        arguments: usize,
    ) -> Result<bool, Error> {
        let ty = self
            .graph
            .binding(primary)
            .ty
            .ok_or(Error::Invariant("pack ordering without type"))?;
        let other = self
            .graph
            .binding(other)
            .ty
            .ok_or(Error::Invariant("pack ordering without type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        let other_count = self.graph.types.function_parameter_count(other)?;
        Ok(count != 0
            && arguments < count
            && other_count < count
            && matches!(
                self.graph
                    .types
                    .get(self.graph.types.function_parameter(ty, count - 1)?)
                    .kind,
                TypeKind::Expansion(_)
            )
            && (other_count == 0
                || !matches!(
                    self.graph
                        .types
                        .get(
                            self.graph
                                .types
                                .function_parameter(other, other_count - 1)?
                        )
                        .kind,
                    TypeKind::Expansion(_)
                )))
    }
    pub(super) fn compare_function_templates(
        &mut self,
        first: BindingId,
        second: BindingId,
        form: TemplateOrdering,
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
        let forward = self.template_at_least_as_specialized(a, b, form)?;
        let backward = self.template_at_least_as_specialized(b, a, form)?;
        if forward != backward || !forward {
            return Ok(forward.cmp(&backward));
        }
        let (TemplateOrdering::Call(count) | TemplateOrdering::Operator(count)) = form else {
            return Ok(Ordering::Equal);
        };
        let references = self.template_reference_order(a, b, count, form)?;
        if references != Ordering::Equal {
            return Ok(references);
        }
        if self.trailing_pack_unused(b, a, count)? {
            return Ok(Ordering::Greater);
        }
        if self.trailing_pack_unused(a, b, count)? {
            return Ok(Ordering::Less);
        }
        Ok(Ordering::Equal)
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
    fn array_bound_deduction_preserves_qualification_and_signature_demand() {
        let source = SourceBuffer::new(
            b"struct A{};enum E{value};\
              template<class H,int N>char f(const H(&)[N]){return H::bad;}\
              template<int N>char c(const char(&)[N]){return N;}\
              template<int N>char v(const volatile int(&)[N]){return N;}\
              template<int N>char p(int*const(&)[N]){return N;}\
              template<int N>char a(const A(&)[N]){return N;}\
              template<int N>char e(const E(&)[N]){return N;}\
              template<int N>char m(const char(&)[N],const char(&)[N]);\
              long m(...);\
              template<int N>char r(char(&)[N]);long r(...);\
              template<int N>char u(unsigned char const(&)[N]);long u(...);\
              char text[3];volatile int ints[3];int*ptrs[3];A objects[3];E enums[3];\
              const char constant[3]={};char different[4];\
              static_assert(sizeof(f(text))==1,\"\");\
              static_assert(sizeof(c(text))==1,\"\");\
              static_assert(sizeof(c(constant))==1,\"\");\
              static_assert(sizeof(v(ints))==1,\"\");\
              static_assert(sizeof(p(ptrs))==1,\"\");\
              static_assert(sizeof(a(objects))==1,\"\");\
              static_assert(sizeof(e(enums))==1,\"\");\
              static_assert(sizeof(m(text,constant))==1,\"\");\
              static_assert(sizeof(m(text,different))==8,\"\");\
              static_assert(sizeof(r(constant))==8,\"\");\
              static_assert(sizeof(u(text))==8,\"\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"array-bound-cv").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        // Unevaluated calls select and memoize signatures without demanding
        // bodies, including f's deliberately invalid dependent body.
        assert_eq!(graph.function_templates.signatures.len(), 7);
        assert!(graph.function_templates.demanded_bodies.is_empty());
        assert!(graph.function_templates.active_signatures.is_empty());
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn default_pack_trials_reject_before_publication_and_reuse_complete_keys() {
        let source = SourceBuffer::new(
            b"template<class...>struct List;template<class,class>struct Pair;\
              template<class...T>struct A{\
                template<class...U,class=List<Pair<T,U>...>>static char f(int);\
                template<class...>static long f(...);};\
              static_assert(sizeof(A<>::f<int>(0))==8,\"\");\
              static_assert(sizeof(A<int>::f<>(0))==8,\"\");\
              static_assert(sizeof(A<int,long>::f<char>(0))==8,\"\");\
              static_assert(sizeof(A<>::f<>(0))==1,\"\");\
              static_assert(sizeof(A<int>::f<long>(0))==1,\"\");\
              static_assert(sizeof(A<>::f<int>(0))==8,\"\");\
              static_assert(sizeof(A<int>::f<long>(0))==1,\"\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-pack-trials").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut constrained = 0;
        for (key, binding) in &graph.function_templates.signatures {
            let primary = graph.entity(key.primary);
            let head = primary.template_head.unwrap();
            if primary.name != Some(f) || graph.templates.parameters(head).len() != 2 {
                continue;
            }
            constrained += 1;
            let owner = graph.scope(primary.home.unwrap()).owner.unwrap();
            let outer = graph.class_templates.instance(owner).unwrap();
            assert_eq!(
                graph.types.pack_elements(outer.arguments[0]).unwrap().len(),
                graph.types.pack_elements(key.arguments[0]).unwrap().len()
            );
            assert_eq!(
                graph.function_templates.arguments[&graph.binding(*binding).entity],
                key.arguments
            );
            let default = graph.templates.default_argument(head, 1).unwrap();
            assert!(default.bound.is_none());
            let ast::TemplateArgumentKind::Type(pattern) = default.syntax else {
                panic!("type default")
            };
            assert!(graph.type_fact(pattern).is_none());
        }
        assert_eq!(constrained, 2);
        assert_eq!(graph.function_templates.defaults.len(), 2);
        assert!(graph.function_templates.active_signatures.is_empty());
        assert!(graph.function_templates.demanded_bodies.is_empty());
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn inserted_object_ordering_preserves_signatures_syntax_and_body_demand() {
        let source = SourceBuffer::new(
            b"struct A{template<class R>int f(R&)const&&{return R::bad;}};\
              template<class R>int g(A const&&,R&){return R::bad;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inserted-object").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax = (tree.expressions.mark(), tree.declarations.mark());
        let a = cursor.intern_name(b"A").unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let g = cursor.intern_name(b"g").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let class = analyzer
            .graph
            .own(global, a, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let class = analyzer.graph.binding(class).entity;
        let scope = analyzer.graph.entity(class).scope.unwrap();
        let member = analyzer.graph.functions_at(scope, f.into()).next().unwrap();
        let free = analyzer
            .graph
            .functions_at(global, g.into())
            .next()
            .unwrap();
        let member_type = analyzer.graph.binding(member).ty.unwrap();
        let free_type = analyzer.graph.binding(free).ty.unwrap();
        let form = TemplateOrdering::Operator(2);
        let (member_parameters, free_parameters) =
            analyzer.ordering_parameters(member, free, form).unwrap();
        let object = member_parameters.object.unwrap();
        assert!(free_parameters.object.is_none());
        let TypeKind::Reference { element, kind } = analyzer.graph.types.get(object).kind else {
            panic!("inserted object without reference");
        };
        assert_eq!(kind, RefKind::Rvalue);
        assert_eq!(analyzer.graph.class_of(element), Some(class));
        assert_eq!(analyzer.type_cv(element), Cv(1));
        for _ in 0..3 {
            assert!(
                analyzer
                    .template_at_least_as_specialized(member, free, form)
                    .unwrap()
            );
            assert!(
                analyzer
                    .template_at_least_as_specialized(free, member, form)
                    .unwrap()
            );
            assert_eq!(
                analyzer
                    .template_reference_order(member, free, 2, form)
                    .unwrap(),
                Ordering::Equal
            );
            let (parameters, _) = analyzer.ordering_parameters(member, free, form).unwrap();
            assert_eq!(parameters.object, Some(object));
        }
        let (member_parameters, _) = analyzer
            .ordering_parameters(member, free, TemplateOrdering::Call(1))
            .unwrap();
        assert!(member_parameters.object.is_none());
        assert_eq!(analyzer.graph.binding(member).ty, Some(member_type));
        assert_eq!(analyzer.graph.binding(free).ty, Some(free_type));
        assert_eq!(
            analyzer
                .graph
                .types
                .function_parameter_count(member_type)
                .unwrap(),
            1
        );
        assert!(analyzer.graph.function_templates.signatures.is_empty());
        assert!(analyzer.graph.function_templates.demanded_bodies.is_empty());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
    }

    #[test]
    fn ordering_keeps_unique_types_and_qualified_members_dependent() {
        let source = SourceBuffer::new(
            b"template<class T>struct Id{typedef T type;};\
              template<class T>int f(T,typename Id<T>::type){return T::bad;}\
              template<class T>int f(T,T){return T::bad;}\
              template<class T>struct Cache{virtual int fill(){return T::bad;}};\
              template<class T>struct Holder{Cache<T> stored;};\
              template<class T>int g(Holder<T>&){return T::bad;}\
              template<class T>int g(Holder<const T>&){return T::bad;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"dependent-ordering").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax = (tree.expressions.mark(), tree.declarations.mark());
        let f = cursor.intern_name(b"f").unwrap();
        let g = cursor.intern_name(b"g").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let f = analyzer
            .graph
            .functions_at(global, f.into())
            .collect::<Vec<_>>();
        let g = analyzer
            .graph
            .functions_at(global, g.into())
            .collect::<Vec<_>>();
        let head = analyzer.function_template_head(f[0]).unwrap().unwrap();
        let unique = analyzer.unique_partial_arguments(head).unwrap();
        let ty = unique[0].unwrap();
        let TypeKind::TemplateParameter {
            entity,
            template: false,
        } = analyzer.graph.types.get(ty).kind
        else {
            panic!("ordering's unique type must stay dependent")
        };
        let info = analyzer.graph.entity(entity).template_parameter.unwrap();
        assert_eq!(info.depth(), 0x7fff);
        assert_eq!(info.argument_index(), 0);
        assert!(info.value_type.is_none());
        assert!(analyzer.graph.entity(entity).scope.is_none());
        assert!(analyzer.graph.dependent_type(ty));
        let const_type = analyzer.graph.types.qualify(ty, Cv(1)).unwrap();
        let depth = analyzer.graph.templates.depth(head);
        let mut deduced = [None];
        // A synthesized ordering parameter belongs to a different depth:
        // permitted qualification at a terminal must not deduce that identity.
        assert!(
            !analyzer
                .deduce_signature_type(const_type, ty, depth, &mut deduced, true)
                .unwrap()
        );
        assert!(
            analyzer
                .deduce_signature_type(const_type, const_type, depth, &mut deduced, true)
                .unwrap()
        );
        assert_eq!(deduced, [None]);
        let parameter = analyzer
            .graph
            .types
            .function_parameter(analyzer.graph.binding(f[0]).ty.unwrap(), 1)
            .unwrap();
        let mut environment = Substitution::new(analyzer.graph.templates.depth(head), &unique);
        let substituted = analyzer
            .substitute_signature_type(parameter, &mut environment)
            .unwrap()
            .unwrap();
        let TypeKind::TemplateParameter { entity: member, .. } =
            analyzer.graph.types.get(substituted).kind
        else {
            panic!("qualified member must stay dependent")
        };
        let qualifier = analyzer
            .graph
            .dependent_members
            .get(member)
            .unwrap()
            .qualifier;
        let TypeKind::Class(shell) = analyzer.graph.types.get(qualifier).kind else {
            panic!("member qualifier must be a class shell")
        };
        assert!(analyzer.graph.entity(shell).scope.is_none());
        for _ in 0..3 {
            for form in [TemplateOrdering::Call(2), TemplateOrdering::FunctionType] {
                assert!(
                    !analyzer
                        .template_at_least_as_specialized(f[0], f[1], form)
                        .unwrap()
                );
                assert!(
                    analyzer
                        .template_at_least_as_specialized(f[1], f[0], form)
                        .unwrap()
                );
            }
            assert!(
                !analyzer
                    .template_at_least_as_specialized(g[0], g[1], TemplateOrdering::Call(1))
                    .unwrap()
            );
            assert!(
                analyzer
                    .template_at_least_as_specialized(g[1], g[0], TemplateOrdering::Call(1))
                    .unwrap()
            );
            assert_eq!(analyzer.unique_partial_arguments(head).unwrap(), unique);
        }
        assert!(analyzer.graph.function_templates.signatures.is_empty());
        assert!(analyzer.graph.function_templates.demanded_bodies.is_empty());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(syntax, (tree.expressions.mark(), tree.declarations.mark()));
    }

    #[test]
    fn class_reentry_publishes_one_signature_without_demanding_its_body() {
        let source = SourceBuffer::new(
            b"template<class T>struct C;\
              template<class T>typename C<T>::type select(T); char select(...);\
              template<class T>struct C{\
                typedef decltype(select(T())) before; typedef long type;\
                typedef decltype(select(T())) after; };\
              template<class T>typename C<T>::type select(T value){return value;}\
              enum { width=sizeof(select(1)), again=sizeof(select(2)) };\
              typedef C<int> instance;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"reentry").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert!(graph.function_templates.active_signatures.is_empty());
        assert_eq!(graph.class_templates.definitions.active_depth(), 0);
        assert_eq!(graph.function_templates.signatures.len(), 1);
        assert_eq!(graph.function_templates.primaries.len(), 1);
        assert!(graph.function_templates.demanded_bodies.is_empty());
        let (&entity, _) = graph.function_templates.primaries.iter().next().unwrap();
        let signature = graph.entity(entity).ty.unwrap();
        let TypeKind::Function { result, .. } = graph.types.get(signature).kind else {
            panic!("specialization without function type");
        };
        assert!(matches!(
            graph.types.get(result).kind,
            TypeKind::Scalar(Scalar::Basic(FundamentalType::LongInt))
        ));
        let name = cursor.intern_name(b"instance").unwrap();
        let binding = graph
            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let TypeKind::Class(class) = graph.types.get(graph.binding(binding).ty.unwrap()).kind
        else {
            panic!("alias without class type");
        };
        let scope = graph.entity(class).scope.unwrap();
        for (spelling, expected) in [
            (b"before".as_slice(), Scalar::Basic(FundamentalType::Char)),
            (b"after", Scalar::Basic(FundamentalType::LongInt)),
        ] {
            let name = cursor.intern_name(spelling).unwrap();
            let binding = graph
                .own(scope, name, super::super::lookup::Filter::Ordinary)
                .unwrap();
            let ty = graph.binding(binding).ty.unwrap();
            assert_eq!(graph.types.get(ty).kind, TypeKind::Scalar(expected));
        }
    }

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
