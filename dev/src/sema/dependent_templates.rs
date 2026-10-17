// Translated from template_names.cpp::dependent_template_name,
// stand_in_template, specialize_template and substitute_stand_in. See NOTICE.
//! Canonical dependent template names own undefined stand-in class identities.
//! Their argument lists use the ordinary specialization store; no definition,
//! member scope or default list is invented for an unknown template.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::{BindingId, EntityId, ScopeId},
    lookup::Filter,
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    syntax::ast,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Name {
    pub qualifier: TypeId,
    pub name: NameId,
}

#[derive(Default)]
pub(super) struct Names {
    templates: HashMap<Name, EntityId>,
    names: HashMap<EntityId, Name>,
}
impl Names {
    pub(super) fn get(&self, primary: EntityId) -> Option<Name> {
        self.names.get(&primary).copied()
    }
}
impl Analyzer<'_> {
    fn dependent_template(&mut self, qualifier: TypeId, name: NameId) -> Result<EntityId, Error> {
        let qualifier = self.bare_type(qualifier)?;
        let qualifier = self.graph.types.canonical_function(qualifier)?;
        let key = Name { qualifier, name };
        if let Some(&entity) = self
            .graph
            .class_templates
            .dependent_names
            .templates
            .get(&key)
        {
            return Ok(entity);
        }
        let entity = self.graph.add_entity(None, Some(name), Keyword::Struct)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Class(entity), Cv::default())?;
        self.graph.entity_mut(entity).ty = Some(ty);
        self.graph
            .class_templates
            .dependent_names
            .templates
            .insert(key, entity);
        self.graph
            .class_templates
            .dependent_names
            .names
            .insert(entity, key);
        Ok(entity)
    }

    pub(super) fn dependent_template_id(
        &mut self,
        qualifier: TypeId,
        component: ast::NameComponent,
        scope: ScopeId,
    ) -> Result<BindingId, Error> {
        if !component.template_keyword {
            return Err(Error::at(
                component.location,
                "a dependent member template-id needs template",
            ));
        }
        let ast::NameKind::Identifier(name) = component.kind else {
            return Err(Error::Missing("dependent special member template name"));
        };
        let arguments = self.form_template_arguments(component, scope, None)?;
        let primary = self.dependent_template(qualifier, name)?;
        let entity = self.class_specialization(primary, arguments)?;
        self.graph.class_templates.binding(entity)
    }

    pub(super) fn substitute_dependent_template(
        &mut self,
        name: Name,
        patterns: Vec<TypeId>,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<EntityId>, Error> {
        // The C++ owner resolves the template-name before substituting its
        // written arguments. Keep definition demand and diagnostic order alike.
        let primary = self.substitute_template_name(name, environment)?;
        let mut arguments = Vec::with_capacity(patterns.len());
        let mut valid = true;
        for pattern in patterns {
            if let Some(ty) = self.substitute_signature_type(pattern, environment)? {
                arguments.push(self.graph.types.canonical_function(ty)?);
            } else {
                // substitute_list visits the rest even after a soft failure:
                // a later argument may require an ill-formed instantiation.
                valid = false;
            }
        }
        let Some(primary) = primary.filter(|_| valid) else {
            return Ok(None);
        };
        if self
            .graph
            .class_templates
            .dependent_names
            .get(primary)
            .is_some()
        {
            return self.class_specialization(primary, arguments).map(Some);
        }
        self.substitute_class_template_defaults(primary, arguments)
    }

    fn substitute_template_name(
        &mut self,
        name: Name,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<EntityId>, Error> {
        let Some(qualifier) = self.substitute_signature_type(name.qualifier, environment)? else {
            return Ok(None);
        };
        if self.graph.dependent_type(qualifier) {
            return self.dependent_template(qualifier, name.name).map(Some);
        }
        let TypeKind::Class(class) = self.graph.types.get(qualifier).kind else {
            return Ok(None);
        };
        if !self.complete_class(class)? {
            return Ok(None);
        }
        let scope = self.graph.entity(class).scope.ok_or(Error::Invariant(
            "complete template qualifier without scope",
        ))?;
        let found = self.qualified_declaration_result(scope, name.name.into(), Filter::Ordinary)?;
        // substitute_template_name consumes LookupResult::binding even when
        // lookup records ambiguity. Preserve the frozen owner's selection.
        let Some(binding) = found.binding else {
            return Ok(None);
        };
        self.class_template_primary(binding)
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
    fn renamed_names_share_undefined_shells_and_substitute_only_demanded_arguments() {
        // A dependent default exercises instantiation limits. A class member
        // template's fixed default is bound at declaration and needs no demand.
        let source = SourceBuffer::new(
            b"template<class T>typename T::template item<int> f(T);\
              template<class U>typename U::template item<int> f(U);\
              template<class T>typename T::template item<char> g(T);\
              struct Q { template<class X,class Y=X>struct item {typename X::unused field;}; };"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"dependent-templates").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let q = cursor.intern_name(b"Q").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        for index in 0..analyzer.tree.decl_lists.get(declarations).len() {
            let declaration = analyzer.tree.decl_lists.get(declarations)[index];
            analyzer.declaration(declaration, global).unwrap();
        }
        let names = &analyzer.graph.class_templates.dependent_names;
        assert_eq!(names.templates.len(), 1);
        let primary = *names.names.keys().next().unwrap();
        let name = names.get(primary).unwrap();
        assert!(analyzer.graph.entity(primary).home.is_none());
        assert!(analyzer.graph.entity(primary).scope.is_none());
        assert!(analyzer.graph.entity(primary).template_head.is_none());
        assert!(!analyzer.graph.entity(primary).complete);
        assert!(!analyzer.complete_class(primary).unwrap());
        let binding_limit = analyzer.graph.binding_limit();
        let mut shells = Vec::new();
        for kind in [
            crate::types::FundamentalType::Int,
            crate::types::FundamentalType::Char,
        ] {
            let argument = analyzer
                .graph
                .types
                .scalar(super::super::types::Scalar::Basic(kind))
                .unwrap();
            let entity = analyzer
                .class_specialization(primary, vec![argument])
                .unwrap();
            shells.push(analyzer.graph.entity(entity).ty.unwrap());
        }
        assert_eq!(binding_limit, analyzer.graph.binding_limit());
        assert_ne!(shells[0], shells[1]);
        let bindings_before_reuse = analyzer.graph.binding_limit();
        let unchanged = [Some(name.qualifier)];
        let mut partial = Substitution::new(0, &unchanged);
        assert_eq!(
            analyzer
                .substitute_signature_type(shells[0], &mut partial)
                .unwrap(),
            Some(shells[0])
        );
        assert_eq!(analyzer.graph.binding_limit(), bindings_before_reuse);
        for &ty in &shells {
            let TypeKind::Class(entity) = analyzer.graph.types.get(ty).kind else {
                panic!("stand-in class")
            };
            let instance = analyzer.graph.class_templates.instance(entity).unwrap();
            assert_eq!(instance.primary, primary);
            assert!(instance.dependent);
            assert_eq!(instance.arguments.len(), 1);
            assert!(!analyzer.complete_class(entity).unwrap());
        }
        let binding = analyzer.graph.own(global, q, Filter::Ordinary).unwrap();
        let q = analyzer.graph.binding(binding).ty.unwrap();
        let arguments = [Some(q)];
        let mut environment = Substitution::new(0, &arguments);
        analyzer.instantiation_depth = 1024;
        assert!(matches!(
            analyzer.substitute_signature_type(shells[0], &mut environment),
            Err(Error::InstantiationDiagnostic {
                message: "template instantiation nested too deeply",
                ..
            })
        ));
        assert_eq!(analyzer.instantiation_depth, 1024);
        analyzer.instantiation_depth = 0;
        let syntax_mark = analyzer.tree.expressions.mark();
        let result = analyzer
            .substitute_signature_type(shells[0], &mut environment)
            .unwrap()
            .unwrap();
        assert_eq!(
            analyzer
                .substitute_signature_type(shells[0], &mut environment)
                .unwrap(),
            Some(result)
        );
        let TypeKind::Class(entity) = analyzer.graph.types.get(result).kind else {
            panic!("concrete specialization")
        };
        let instance = analyzer.graph.class_templates.instance(entity).unwrap();
        assert!(!instance.dependent);
        assert_eq!(instance.arguments.len(), 2);
        assert_eq!(instance.arguments[0], instance.arguments[1]);
        assert!(analyzer.graph.entity(entity).scope.is_none());
        assert!(!analyzer.graph.entity(entity).complete);
        assert_eq!(syntax_mark, analyzer.tree.expressions.mark());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn incomplete_qualifier_is_not_cached_and_later_definition_supplies_the_template() {
        let source = SourceBuffer::new(
            b"struct Q; template<class T>typename T::template item<int> f(T*);\
              struct Q { template<class X>struct item{typename X::unused field;}; };"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"template-query-retry").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let q = cursor.intern_name(b"Q").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        for index in 0..2 {
            let declaration = analyzer.tree.decl_lists.get(declarations)[index];
            analyzer.declaration(declaration, global).unwrap();
        }
        let (&primary, &name) = analyzer
            .graph
            .class_templates
            .dependent_names
            .names
            .iter()
            .next()
            .unwrap();
        let argument = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        let shell = analyzer
            .class_specialization(primary, vec![argument])
            .unwrap();
        let pattern = analyzer.graph.entity(shell).ty.unwrap();
        let binding = analyzer.graph.own(global, q, Filter::Ordinary).unwrap();
        let q = analyzer.graph.binding(binding).ty.unwrap();
        let given = [Some(q)];
        let mut environment = Substitution::new(0, &given);
        let observations = analyzer.incomplete_class_observations;
        assert_eq!(
            analyzer
                .substitute_signature_type(pattern, &mut environment)
                .unwrap(),
            None
        );
        assert!(analyzer.incomplete_class_observations > observations);
        let declaration = analyzer.tree.decl_lists.get(declarations)[2];
        analyzer.declaration(declaration, global).unwrap();
        let result = analyzer
            .substitute_signature_type(pattern, &mut environment)
            .unwrap()
            .unwrap();
        assert_ne!(result, pattern);
        assert!(analyzer.graph.dependent_type(pattern));
        assert!(!analyzer.graph.dependent_type(result));
        assert_eq!(
            analyzer.graph.class_templates.dependent_names.get(primary),
            Some(name)
        );
        let binding_limit = analyzer.graph.binding_limit();
        assert_eq!(
            analyzer
                .substitute_signature_type(pattern, &mut environment)
                .unwrap(),
            Some(result)
        );
        assert_eq!(analyzer.graph.binding_limit(), binding_limit);
    }
}
