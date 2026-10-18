// Translated from sema/template_names.cpp and class_templates.cpp. See NOTICE.
//! Template-name arguments retain declaration identity, independent of class demand.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::{BindingId, BindingKind, EntityId, ScopeId},
    lookup::{Filter, Prefix},
    templates::{HeadId, ParameterInfo},
    types::{Cv, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

/// Immutable canonical-name keys own undefined shells for this semantic unit.
/// Definition/default changes cannot invalidate these name identities.
#[derive(Default)]
pub(super) struct TemplateNames {
    stand_ins: HashMap<TypeId, EntityId>,
    names: HashMap<EntityId, TypeId>,
}
impl TemplateNames {
    pub(super) fn name(&self, primary: EntityId) -> Option<TypeId> {
        self.names.get(&primary).copied()
    }
}
impl Analyzer<'_> {
    pub(super) fn template_name_type(
        &mut self,
        binding: BindingId,
        injected: bool,
    ) -> Result<Option<TypeId>, Error> {
        let value = self.graph.binding(binding);
        if value.kind == BindingKind::Alias {
            if self.graph.entity(value.entity).key == Keyword::Typedef
                && self.graph.entity(value.entity).template_head.is_some()
            {
                return self
                    .graph
                    .types
                    .intern(TypeKind::Template(value.entity), Cv::default())
                    .map(Some);
            }
            return Ok(value.ty.filter(|&ty| {
                matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Template(_) | TypeKind::TemplateParameter { template: true, .. }
                )
            }));
        }
        if value.kind != BindingKind::Type {
            return Ok(None);
        }
        if self
            .graph
            .entity(value.entity)
            .template_parameter
            .is_some_and(|info| info.inner.is_some())
        {
            return Ok(value.ty);
        }
        let Some(primary) = self.class_template_primary(binding) else {
            return Ok(None);
        };
        if !injected
            && (self.graph.class_templates.instance(value.entity).is_some()
                || self.graph.entity(value.entity).scope == Some(self.graph.binding_scope(binding)))
        {
            return Ok(None);
        }
        self.graph
            .types
            .intern(TypeKind::Template(primary), Cv::default())
            .map(Some)
    }

    pub(super) fn plain_template_name(
        &self,
        argument: ast::TemplateArgument,
    ) -> Option<(ast::Name, bool)> {
        let mut expansion = argument.pack;
        let name = match argument.kind {
            ast::TemplateArgumentKind::Expression(expression) => {
                let ast::ExprKind::Id(name) = self.tree.expressions.get(expression).kind else {
                    return None;
                };
                name
            }
            ast::TemplateArgumentKind::Type(ty) => {
                let node = self.tree.types.get(ty);
                let [spec] = self.tree.specs.get(node.specifiers) else {
                    return None;
                };
                let ast::SpecifierKind::Named(name) = spec.kind else {
                    return None;
                };
                if self.tree.names.get(name).typename_keyword {
                    return None;
                }
                if let Some(declarator) = node.declarator {
                    let node = self.tree.declarators.get(declarator);
                    if !node.pointers.is_empty()
                        || node.name.is_some()
                        || node.nested.is_some()
                        || !node.suffixes.is_empty()
                    {
                        return None;
                    }
                    expansion |= node.pack;
                }
                name
            }
        };
        let last = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()?;
        (matches!(last.kind, ast::NameKind::Identifier(_)) && last.arguments.is_none())
            .then_some((name, expansion))
    }

    pub(super) fn template_name_argument(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
        injected: bool,
    ) -> Result<Option<TypeId>, Error> {
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let last = *components
            .last()
            .ok_or(Error::Invariant("empty template name"))?;
        let count = components.len() - 1;
        if let Some(Prefix::Dependent(qualifier)) = self.name_prefix(name, scope, count, true)? {
            if !last.template_keyword {
                return Ok(None);
            }
            let ast::NameKind::Identifier(identifier) = last.kind else {
                return Ok(None);
            };
            let primary = self.dependent_template(qualifier, identifier)?;
            return self
                .graph
                .types
                .intern(
                    TypeKind::TemplateParameter {
                        entity: primary,
                        template: true,
                    },
                    Cv::default(),
                )
                .map(Some);
        }
        let found = self.lookup_result_in(
            name,
            scope,
            Filter::Ordinary,
            super::lookup::Specialization::None,
        )?;
        let Some(binding) = found
            .binding
            .filter(|_| !found.ambiguous || found.injected_template)
        else {
            return Ok(None);
        };
        self.template_name_type(binding, injected || found.injected_template)
    }

    pub(super) fn template_argument_kind(&self, mut ty: TypeId) -> bool {
        if let TypeKind::Expansion(pattern) = self.graph.types.get(ty).kind {
            ty = pattern;
        }
        match self.graph.types.get(ty).kind {
            TypeKind::Template(_) | TypeKind::TemplateParameter { template: true, .. } => true,
            TypeKind::BoundPack { parameter, .. } => self
                .graph
                .entity(parameter)
                .template_parameter
                .is_some_and(|info| info.inner.is_some()),
            _ => false,
        }
    }

    fn template_name_head(&self, ty: TypeId) -> Option<HeadId> {
        match self.graph.types.get(ty).kind {
            TypeKind::Template(entity) => self.graph.entity(entity).template_head,
            TypeKind::TemplateParameter {
                entity,
                template: true,
            } => self
                .graph
                .entity(entity)
                .template_parameter
                .and_then(|info| info.inner),
            _ => None,
        }
    }

    pub(super) fn template_parameters_match(
        &mut self,
        p: ParameterInfo,
        a: ParameterInfo,
    ) -> Result<bool, Error> {
        if p.inner.is_some() != a.inner.is_some()
            || p.value_type.is_some() != a.value_type.is_some()
        {
            return Ok(false);
        }
        if let Some((p, a)) = p.inner.zip(a.inner) {
            return self.template_heads_match(p, a);
        }
        let Some((p_type, a_type)) = p.value_type.zip(a.value_type) else {
            return Ok(true);
        };
        let first = self.graph.types.canonical_function(p_type)?;
        let second = self.graph.types.canonical_function(a_type)?;
        if first == second {
            return Ok(true);
        }
        let own = |ty, depth| {
            let node = self.graph.types.get(ty);
            if let TypeKind::TemplateParameter {
                entity,
                template: false,
            } = node.kind
            {
                node.cv.0 == 0
                    && self
                        .graph
                        .entity(entity)
                        .template_parameter
                        .is_some_and(|info| info.depth() == depth && info.value_type.is_none())
            } else {
                false
            }
        };
        if own(first, p.depth()) != own(second, a.depth()) {
            return Ok(true);
        }
        let (
            TypeKind::TemplateParameter {
                entity: x,
                template: false,
            },
            TypeKind::TemplateParameter {
                entity: y,
                template: false,
            },
        ) = (
            &self.graph.types.get(first).kind,
            &self.graph.types.get(second).kind,
        )
        else {
            return Ok(false);
        };
        let Some((x, y)) = self
            .graph
            .entity(*x)
            .template_parameter
            .zip(self.graph.entity(*y).template_parameter)
        else {
            return Ok(false);
        };
        Ok(
            self.graph.types.get(first).cv == self.graph.types.get(second).cv
                && x.depth() == p.depth()
                && y.depth() == a.depth()
                && x.argument_index() == y.argument_index()
                && x.value_type.is_some() == y.value_type.is_some()
                && x.is_pack() == y.is_pack(),
        )
    }

    fn template_heads_match(&mut self, parameter: HeadId, argument: HeadId) -> Result<bool, Error> {
        self.check_stack()?;
        let count = self.graph.templates.parameters(parameter).len();
        let mut next = 0;
        for index in 0..self.graph.templates.parameters(argument).len() {
            if next == count {
                return Ok(false);
            }
            let p = self.graph.templates.parameters(parameter)[next];
            let a = self.graph.templates.parameters(argument)[index];
            let p = self
                .graph
                .entity(p)
                .template_parameter
                .ok_or(Error::Invariant("template head without parameter"))?;
            let a = self
                .graph
                .entity(a)
                .template_parameter
                .ok_or(Error::Invariant("template head without parameter"))?;
            if !self.template_parameters_match(p, a)? || (a.is_pack() && !p.is_pack()) {
                return Ok(false);
            }
            if !p.is_pack() {
                next += 1;
            }
        }
        Ok(next == count || (next + 1 == count && self.argument_parameter_pack(parameter, next)))
    }

    pub(super) fn match_template_name(
        &mut self,
        parameter: ParameterInfo,
        argument: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let Some((p, a)) = parameter.inner.zip(self.template_name_head(argument))
            && !self.template_heads_match(p, a)?
        {
            return Err(Error::at(
                location,
                "a template template-argument whose template parameters do not match",
            ));
        }
        Ok(())
    }

    pub(super) fn template_parameter_stand_in(&mut self, name: TypeId) -> Result<EntityId, Error> {
        let name = self.graph.types.canonical_function(name)?;
        if let TypeKind::TemplateParameter {
            entity,
            template: true,
        } = self.graph.types.get(name).kind
            && self
                .graph
                .class_templates
                .dependent_names
                .get(entity)
                .is_some()
        {
            return Ok(entity);
        }
        if let Some(&primary) = self
            .graph
            .class_templates
            .template_entities
            .stand_ins
            .get(&name)
        {
            return Ok(primary);
        }
        let primary = self.graph.add_entity(None, None, Keyword::Struct)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Class(primary), Cv::default())?;
        self.graph.entity_mut(primary).ty = Some(ty);
        self.graph
            .class_templates
            .template_entities
            .stand_ins
            .insert(name, primary);
        self.graph
            .class_templates
            .template_entities
            .names
            .insert(primary, name);
        Ok(primary)
    }

    pub(super) fn specialize_template_name(
        &mut self,
        name: TypeId,
        arguments: Vec<TypeId>,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        if let TypeKind::Template(primary) = self.graph.types.get(name).kind {
            if self.graph.entity(primary).key == Keyword::Typedef {
                return self.alias_specialization(primary, arguments, location);
            }
            let binding = self.class_template_with_defaults(primary, arguments, location)?;
            return self
                .graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("class template result without type"));
        }
        let primary = self.template_parameter_stand_in(name)?;
        let entity = self.class_specialization(primary, arguments)?;
        self.graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("stand-in without type"))
    }

    pub(super) fn template_parameter_id(
        &mut self,
        found: BindingId,
        name: TypeId,
        component: ast::NameComponent,
        scope: ScopeId,
    ) -> Result<BindingId, Error> {
        let syntax = component
            .arguments
            .ok_or(Error::Invariant("template-id without arguments"))?;
        let written = component.expanded
            || self
                .tree
                .template_arguments
                .get(syntax)
                .iter()
                .any(|argument| {
                    let type_pack = match argument.kind {
                        ast::TemplateArgumentKind::Type(ty) => self
                            .tree
                            .types
                            .get(ty)
                            .declarator
                            .is_some_and(|d| self.tree.declarators.get(d).pack),
                        ast::TemplateArgumentKind::Expression(_) => false,
                    };
                    argument.pack || type_pack
                });
        // A bound argument keeps the declared parameter entity for its own
        // defaults; positional canonicalization intentionally excludes defaults.
        let own = self
            .graph
            .entity(self.graph.binding(found).entity)
            .template_parameter
            .and_then(|info| info.inner)
            .filter(|_| !written);
        let mut arguments = self.form_template_arguments(
            component,
            scope,
            own.or_else(|| self.template_name_head(name)),
        )?;
        if let Some(own) = own {
            self.complete_template_arguments_in(
                own,
                &mut arguments,
                component.location,
                Some(self.graph.binding_scope(found)),
            )?;
            arguments = self.graph.types.flattened_arguments(&arguments);
        }
        let ty = self.specialize_template_name(name, arguments, component.location)?;
        self.specialization_binding(ty, self.graph.binding(found).entity)
    }

    pub(super) fn deduce_template_id(
        &mut self,
        name: TypeId,
        patterns: &[TypeId],
        actual: EntityId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        let Some(instance) = self.graph.class_templates.instance(actual) else {
            return Ok(false);
        };
        let primary = instance.primary;
        let given = self.graph.types.flattened_arguments(&instance.arguments);
        let actual_name =
            if let Some(name) = self.graph.class_templates.template_entities.name(primary) {
                name
            } else if self
                .graph
                .class_templates
                .dependent_names
                .get(primary)
                .is_some()
            {
                self.graph.types.intern(
                    TypeKind::TemplateParameter {
                        entity: primary,
                        template: true,
                    },
                    Cv::default(),
                )?
            } else {
                self.graph
                    .types
                    .intern(TypeKind::Template(primary), Cv::default())?
            };
        if !self.deduce_signature_type(name, actual_name, depth, arguments, false)? {
            return Ok(false);
        }
        if let Some(head) = self.template_name_head(name) {
            for (index, &argument) in given
                .iter()
                .enumerate()
                .take(self.graph.templates.parameters(head).len())
            {
                if self.argument_parameter_pack(head, index) {
                    break;
                }
                let parameter = self.graph.templates.parameters(head)[index];
                if let Some(ty) = self
                    .graph
                    .entity(parameter)
                    .template_parameter
                    .and_then(|info| info.value_type)
                    && !self.graph.dependent_type(ty)
                    && let TypeKind::Value { ty: actual, .. } | TypeKind::Address { ty: actual, .. } =
                        self.graph.types.get(argument).kind
                    && !self.graph.address_parameter_type(actual)
                {
                    let expected = self.graph.types.canonical_function(ty)?;
                    let expected = self.bare_type(expected)?;
                    if self.graph.types.canonical_function(actual)? != expected {
                        return Ok(false);
                    }
                }
            }
        }
        self.deduce_type_list(patterns, &given, depth, arguments)
    }

    pub(super) fn substitute_template_parameter_id(
        &mut self,
        name: TypeId,
        patterns: &[TypeId],
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let name = self.substitute_signature_type(name, environment)?;
        let arguments = self.substitute_pack_list(patterns, environment)?;
        let Some((name, arguments)) = name.zip(arguments) else {
            return Ok(None);
        };
        match self.specialize_template_name(name, arguments, SourceOffset(0)) {
            Ok(ty) => Ok(Some(ty)),
            Err(
                Error::Diagnostic { location, message } | Error::NonConstant { location, message },
            ) => Ok(environment.reject_at(location, message)),
            Err(error) => Err(error),
        }
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
    fn inner_defaults_bind_earlier_slots_and_keep_outer_frames_distinct() {
        let source = SourceBuffer::new(
            b"template<class A,class B,class C>struct P{void unused(){A::missing();}};\
            template<class Outer,template<class A,class B=Outer,class C=B*>class TT>\
            struct W{using type=TT<char>;};\
            using X=W<int,P>::type;using Y=W<long,P>::type;using Z=W<int,P>::type;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inner-default-frames").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"X".as_slice(), b"Y", b"Z"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let types = names.map(|name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Ordinary).unwrap())
                .ty
                .unwrap()
        });
        assert_eq!(types[0], types[2]);
        assert_ne!(types[0], types[1]);
        for (ty, expected) in types[..2].iter().zip([
            crate::types::FundamentalType::Int,
            crate::types::FundamentalType::LongInt,
        ]) {
            let TypeKind::Class(entity) = analyzer.graph.types.get(*ty).kind else {
                panic!("class result");
            };
            let instance = analyzer.graph.class_templates.instance(entity).unwrap();
            assert!(
                matches!(analyzer.graph.types.get(instance.arguments[1]).kind,
                TypeKind::Scalar(super::super::types::Scalar::Basic(kind)) if kind == expected)
            );
            assert!(
                matches!(analyzer.graph.types.get(instance.arguments[2]).kind,
                TypeKind::Pointer(element) if element == instance.arguments[1])
            );
            assert!(!analyzer.graph.entity(entity).complete);
            assert!(analyzer.graph.entity(entity).scope.is_none());
        }
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.graph.lookup_limit.is_none());
    }

    #[test]
    fn declared_defaults_survive_positional_identity_without_demanding_argument_classes() {
        let source = SourceBuffer::new(
            b"template<class T,class=short>struct P{void unused(){T::missing();}};\
            template<template<class,class=int>class TT>struct W{using type=TT<char>;};\
            template<template<class,class=long>class TT>struct V{using type=TT<char>;};\
            using X=W<P>::type;using Y=V<P>::type;using Z=W<P>::type;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"template-default-ownership").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"X".as_slice(), b"Y", b"Z"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let types = names.map(|name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Ordinary).unwrap())
                .ty
                .unwrap()
        });
        assert_eq!(types[0], types[2]);
        assert_ne!(types[0], types[1]);
        for (ty, expected) in types[..2].iter().zip([
            crate::types::FundamentalType::Int,
            crate::types::FundamentalType::LongInt,
        ]) {
            let TypeKind::Class(entity) = analyzer.graph.types.get(*ty).kind else {
                panic!("class result");
            };
            assert!(!analyzer.graph.entity(entity).complete);
            assert!(analyzer.graph.entity(entity).scope.is_none());
            let instance = analyzer.graph.class_templates.instance(entity).unwrap();
            assert!(
                matches!(analyzer.graph.types.get(instance.arguments[1]).kind,TypeKind::Scalar(super::super::types::Scalar::Basic(kind)) if kind == expected)
            );
        }
        let syntax = analyzer.tree.expressions.mark();
        let bindings = analyzer.graph.binding_limit();
        for ty in types {
            let TypeKind::Class(entity) = analyzer.graph.types.get(ty).kind else {
                panic!("class result");
            };
            let instance = analyzer.graph.class_templates.instance(entity).unwrap();
            let primary = instance.primary;
            let arguments = instance.arguments.clone();
            assert_eq!(
                analyzer.class_specialization(primary, arguments).unwrap(),
                entity
            );
        }
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(bindings, analyzer.graph.binding_limit());
        assert_eq!(syntax, analyzer.tree.expressions.mark());
    }
}
