// Translated from sema/class_templates.cpp::class_specialization/template_id_binding.
// See NOTICE.
//! Canonical class shells and argument defaults, independent of definition demand.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    templates::HeadId,
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::{ast, cloning::Cloner},
};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
struct Key {
    primary: EntityId,
    arguments: Vec<TypeId>,
}

pub(super) struct Instance {
    pub primary: EntityId,
    pub arguments: Vec<TypeId>,
    pub dependent: bool,
    binding: BindingId,
}
#[derive(Default)]
pub(super) struct ClassTemplates {
    pub(super) definitions: super::instance_definitions::Definitions,
    pub(super) enumerations: super::enum_instances::Enumerations,
    specializations: HashMap<Key, EntityId>,
    instances: HashMap<EntityId, Instance>,
    defaults: HashMap<Key, BindingId>,
    argument_defaults: HashMap<(HeadId, usize, Vec<TypeId>), TypeId>,
}
impl ClassTemplates {
    pub(super) fn instance(&self, entity: EntityId) -> Option<&Instance> {
        self.instances.get(&entity)
    }
}
impl super::graph::Graph {
    pub(super) fn same_template_injected_names(&self, a: BindingId, b: BindingId) -> bool {
        let primary = |id: BindingId| {
            let binding = self.binding(id);
            if binding.kind != BindingKind::Type {
                return None;
            }
            let class = self.entity(binding.entity);
            if class.scope != Some(self.binding_scope(id)) {
                return None;
            }
            self.class_templates
                .instance(binding.entity)
                .map(|instance| instance.primary)
        };
        primary(a).is_some_and(|a| Some(a) == primary(b))
    }
}
impl Analyzer<'_> {
    /// An elaborated class template-id names its specialization, without
    /// demanding the specialization's definition. Alias templates are excluded
    /// here even when their result would be a class (7.1.6.3p2).
    pub(super) fn elaborated_specialization(
        &mut self,
        key: crate::lex::vocabulary::Keyword,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let location = self.tree.names.get(name).location;
        let found = self
            .lookup_result_in(name, scope, super::lookup::Filter::Elaborated, false)?
            .binding
            .ok_or_else(|| Error::at(location, "undeclared name"))?;
        let binding = self.graph.binding(found);
        if binding.kind == BindingKind::Alias
            && self.graph.entity(binding.entity).template_head.is_some()
        {
            return Err(Error::at(
                location,
                "an elaborated-type-specifier names an alias template specialization",
            ));
        }
        let component = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()
            .copied()
            .ok_or(Error::Invariant("empty elaborated template name"))?;
        let specialized = self
            .class_template_id(found, component, scope)?
            .ok_or_else(|| {
                Error::at(
                    location,
                    "a template-id that names no class or variable template",
                )
            })?;
        let entity = self.elaborated_class_entity(specialized, key, location, false)?;
        super::record(&mut self.graph.resolved_names, name.index(), specialized);
        self.graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("class specialization without type"))
    }

    pub(super) fn class_template_id(
        &mut self,
        found: BindingId,
        component: ast::NameComponent,
        scope: ScopeId,
    ) -> Result<Option<BindingId>, Error> {
        self.check_stack()?;
        let binding = self.graph.binding(found);
        if binding.kind == BindingKind::Alias
            && self.graph.entity(binding.entity).template_head.is_some()
        {
            return Err(Error::Missing("alias template specialization"));
        }
        let primary = if binding.kind == BindingKind::Type {
            self.graph
                .class_templates
                .instance(binding.entity)
                .map_or(binding.entity, |instance| instance.primary)
        } else {
            binding.entity
        };
        let Some(head) = self.graph.entity(primary).template_head else {
            return Ok(None);
        };
        if binding.kind != BindingKind::Type {
            return Ok(None);
        }
        self.supported_class_head(head)?;
        let syntax = component
            .arguments
            .ok_or(Error::Invariant("template-id without arguments"))?;
        let mut arguments = Vec::new();
        for index in 0..self.tree.template_arguments.get(syntax).len() {
            let argument = self.tree.template_arguments.get(syntax)[index];
            if argument.pack {
                return Err(Error::Missing("class template argument expansion"));
            }
            let ty = self
                .template_type_argument(argument, scope, component.location)?
                .ok_or(Error::Missing("non-type class template arguments"))?;
            arguments.push(self.graph.types.canonical_function(ty)?);
        }
        let count = self.graph.templates.parameters(head).len();
        if arguments.len() > count {
            return Err(Error::at(component.location, "too many template arguments"));
        }
        let omitted = arguments.len() < count;
        let key = Key { primary, arguments };
        let result =
            if omitted && let Some(&binding) = self.graph.class_templates.defaults.get(&key) {
                binding
            } else {
                let mut full = key.arguments.clone();
                self.class_template_defaults(head, &mut full, component.location)?;
                let entity = self.class_specialization(primary, full)?;
                let binding = self
                    .graph
                    .class_templates
                    .instance(entity)
                    .ok_or(Error::Invariant("class shell lost its instance"))?
                    .binding;
                if omitted {
                    self.graph.class_templates.defaults.insert(key, binding);
                }
                binding
            };
        // Only the template's own arguments in its definition denote the
        // current instantiation. Elsewhere the same list denotes a shell.
        let instance = self
            .graph
            .class_templates
            .instance(self.graph.binding(result).entity)
            .ok_or(Error::Invariant("template-id without instance"))?;
        let arguments = instance.arguments.clone();
        let mut own = arguments.len() == count;
        for (index, &argument) in arguments.iter().enumerate().take(count) {
            let parameter = self.graph.templates.parameters(head)[index];
            let pattern = self
                .graph
                .entity(parameter)
                .ty
                .ok_or(Error::Invariant("class parameter without type"))?;
            own &= self.graph.types.canonical_function(pattern)? == argument;
        }
        if own
            && self.graph.entity(primary).scope.is_some_and(|class| {
                self.graph.encloses(class, scope)
                    || self.template_owner.is_some_and(|owner| {
                        owner == primary
                            || self
                                .graph
                                .entity(owner)
                                .home
                                .is_some_and(|home| self.graph.encloses(class, home))
                    })
            })
        {
            return Ok(Some(found));
        }
        Ok(Some(result))
    }

    fn supported_class_head(&self, head: HeadId) -> Result<(), Error> {
        for &parameter in self.graph.templates.parameters(head) {
            if !self
                .graph
                .entity(parameter)
                .template_parameter
                .is_some_and(super::templates::ParameterInfo::ordinary_type)
            {
                return Err(Error::Missing(
                    "class template packs or template parameters",
                ));
            }
        }
        Ok(())
    }

    fn class_template_defaults(
        &mut self,
        head: HeadId,
        arguments: &mut Vec<TypeId>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let count = self.graph.templates.parameters(head).len();
        for index in arguments.len()..count {
            let default = self
                .graph
                .templates
                .default_argument(head, index)
                .ok_or_else(|| Error::at(location, "too few template arguments"))?;
            let key = (head, index, arguments.clone());
            let ty = if let Some(&ty) = self.graph.class_templates.argument_defaults.get(&key) {
                ty
            } else {
                let ty = self.class_default_type(default, head, arguments)?;
                let ty = self.graph.types.canonical_function(ty)?;
                self.graph.class_templates.argument_defaults.insert(key, ty);
                ty
            };
            arguments.push(ty);
        }
        Ok(())
    }

    fn class_default_type(
        &mut self,
        default: super::templates::DefaultArgument,
        head: HeadId,
        arguments: &[TypeId],
    ) -> Result<TypeId, Error> {
        self.check_stack()?;
        let depth = self.graph.templates.depth(head);
        if self.template_depth != 0 && arguments.iter().any(|&ty| self.graph.dependent_type(ty)) {
            let pattern = self.type_syntax(default.syntax, default.scope)?;
            let given: Vec<_> = arguments.iter().copied().map(Some).collect();
            let mut environment = Substitution::new(depth, &given);
            return self
                .substitute_signature_type(pattern, &mut environment)?
                .ok_or(Error::Missing(
                    "dependent class template default substitution",
                ));
        }
        let parent = self
            .graph
            .scope(default.scope)
            .parent
            .ok_or(Error::Invariant("class default without parent"))?;
        let frame = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, parent, None, None)?;
        for index in 0..self.graph.scope(default.scope).bindings.len() {
            let source = self.graph.scope(default.scope).bindings[index];
            let binding = self.graph.binding(source);
            let Some(position) = self
                .graph
                .entity(binding.entity)
                .template_parameter
                .and_then(|info| info.index(depth))
            else {
                continue;
            };
            if let Some(&ty) = arguments.get(position) {
                self.graph.add_binding(
                    frame,
                    None,
                    Binding {
                        kind: BindingKind::Alias,
                        ty: Some(ty),
                        ..binding
                    },
                )?;
            }
        }
        let copy = Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .ty(default.syntax)
            .map_err(|error| super::template_bodies::copy_error(&error))?;
        self.required_instantiation(
            self.instantiation_depth,
            self.tree.types.get(default.syntax).location,
            default.limit,
            |analyzer| analyzer.type_syntax(copy, frame),
        )
    }

    pub(super) fn class_specialization(
        &mut self,
        primary: EntityId,
        arguments: Vec<TypeId>,
    ) -> Result<EntityId, Error> {
        self.check_stack()?;
        let key = Key { primary, arguments };
        if let Some(&entity) = self.graph.class_templates.specializations.get(&key) {
            return Ok(entity);
        }
        let template = self.graph.entity(primary);
        let (name, class_key, home) = (
            template.name,
            template.key,
            template.home.unwrap_or(self.graph.global),
        );
        let mut dependent = key
            .arguments
            .iter()
            .any(|&ty| self.graph.dependent_type(ty));
        let mut enclosing = Some(home);
        while let Some(scope) = enclosing {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::Namespace {
                break;
            }
            if let Some(owner) = node.owner {
                dependent |= self.graph.entity(owner).template_head.is_some()
                    || self
                        .graph
                        .class_templates
                        .instance(owner)
                        .is_some_and(|instance| instance.dependent);
            }
            enclosing = node.parent;
        }
        let entity = self.graph.add_entity(None, name, class_key)?;
        self.graph
            .instantiation_levels
            .record(entity, self.instantiation_depth);
        let ty = self
            .graph
            .types
            .intern(TypeKind::Class(entity), Cv::default())?;
        self.graph.entity_mut(entity).ty = Some(ty);
        let binding = self.graph.add_hidden_binding_in(
            home,
            Binding {
                kind: BindingKind::Type,
                name,
                entity,
                ty: Some(ty),
                key: class_key,
            },
        )?;
        self.graph.class_templates.instances.insert(
            entity,
            Instance {
                primary,
                arguments: key.arguments.clone(),
                dependent,
                binding,
            },
        );
        self.graph
            .class_templates
            .specializations
            .insert(key, entity);
        Ok(entity)
    }

    pub(super) fn substitute_class_type(
        &mut self,
        entity: EntityId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<EntityId>, Error> {
        let instance = self
            .graph
            .class_templates
            .instance(entity)
            .ok_or(Error::Invariant("substitution without class instance"))?;
        let primary = instance.primary;
        let patterns = instance.arguments.clone();
        let mut arguments = Vec::with_capacity(patterns.len());
        for pattern in patterns {
            let Some(ty) = self.substitute_signature_type(pattern, environment)? else {
                return Ok(None);
            };
            arguments.push(self.graph.types.canonical_function(ty)?);
        }
        self.class_specialization(primary, arguments).map(Some)
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
    fn semantic_type_arguments_and_elaborated_names_reuse_undemanded_shells() {
        let source = SourceBuffer::new(
            b"struct Source{typedef int lower;}; Source source;\
              template<class T,class U=T*>struct Holder{typename T::missing unused;};\
              using A=Holder<decltype(source)::lower>;\
              typedef struct Holder<int,int*> B;\
              struct Holder<decltype(source)::lower>* first;\
              struct Holder<int>* second;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"type-arguments").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        let a = cursor.intern_name(b"A").unwrap();
        let b = cursor.intern_name(b"B").unwrap();
        let lookup = |name| {
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                        .unwrap(),
                )
                .ty
                .unwrap()
        };
        let ty = lookup(a);
        assert_eq!(ty, lookup(b));
        let TypeKind::Class(class) = graph.types.get(ty).kind else {
            panic!("specialization type")
        };
        assert!(graph.entity(class).scope.is_none());
        let instance = graph.class_templates.instance(class).unwrap();
        assert_eq!(instance.arguments.len(), 2);
        assert!(!instance.dependent);
        assert!(graph.root_demands.as_slice().is_empty());
        assert!(
            !graph
                .scope(graph.global)
                .bindings
                .contains(&instance.binding)
        );
        assert_eq!(
            graph
                .class_templates
                .instances
                .values()
                .filter(|instance| !instance.dependent)
                .count(),
            1
        );
        let ast::DeclKind::Alias { ty, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[3])
            .kind
        else {
            panic!("alias syntax")
        };
        let ast::SpecifierKind::Named(name) = tree.specs.get(tree.types.get(ty).specifiers)[0].kind
        else {
            panic!("template-id syntax")
        };
        let arguments = tree.name_lists.get(tree.names.get(name).components)[0]
            .arguments
            .unwrap();
        let ast::TemplateArgumentKind::Expression(expression) =
            tree.template_arguments.get(arguments)[0].kind
        else {
            panic!("retained expression reading")
        };
        let ast::ExprKind::Id(name) = tree.expressions.get(expression).kind else {
            panic!("retained id-expression")
        };
        let binding = graph.resolved_names[name.index()].unwrap();
        assert!(matches!(graph.binding(binding).kind, BindingKind::Alias));
    }

    #[test]
    fn elaborated_friends_grant_only_the_named_specialization_without_definition_demand() {
        let source = SourceBuffer::new(
            b"template<class T>struct Friend;\
              struct Owner{friend struct Friend<int>;};\
              typedef struct Friend<int> A;typedef struct Friend<char> B;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"friend-specialization").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut entity = |name| {
            let name = cursor.intern_name(name).unwrap();
            let binding = graph.binding(
                graph
                    .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                    .unwrap(),
            );
            let TypeKind::Class(class) = graph.types.get(binding.ty.unwrap()).kind else {
                panic!("class type")
            };
            class
        };
        let owner = entity(b"Owner");
        let selected = entity(b"A");
        let other = entity(b"B");
        let primary = graph.class_templates.instance(selected).unwrap().primary;
        assert!(graph.friendships.grants(owner, selected));
        assert!(!graph.friendships.grants(owner, other));
        assert!(!graph.friendships.grants(owner, primary));
        assert!(graph.entity(selected).scope.is_none());
        assert!(graph.entity(other).scope.is_none());
    }

    #[test]
    fn canonical_shells_are_hidden_and_never_demand_definitions() {
        let source = SourceBuffer::new(
            b"template<class T,class U=T*>struct Box;\
              using A=Box<int>;using A=Box<int,int*>;\
              template<class V,class W>struct Box{V field;};\
              using B=Box<char>;using B=Box<char,char*>;\
              template<class T>int f(Box<T>*);\
              template<class U>int f(Box<U>*);"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"class-shells").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.class_templates.instances.len(), 3);
        assert_eq!(graph.class_templates.defaults.len(), 3);
        assert_eq!(graph.class_templates.specializations.len(), 3);
        let mut primaries = std::collections::HashSet::new();
        let mut dependent = 0;
        for (&entity, instance) in &graph.class_templates.instances {
            primaries.insert(instance.primary);
            dependent += usize::from(instance.dependent);
            let shell = graph.entity(entity);
            assert!(shell.scope.is_none());
            assert!(!shell.defined && !shell.complete);
            assert!(matches!(
                shell.layout,
                super::super::layout::LayoutState::NotStarted
            ));
            assert!(
                !graph
                    .scope(graph.global)
                    .bindings
                    .contains(&instance.binding)
            );
            assert_eq!(shell.home, graph.entity(instance.primary).home);
        }
        assert_eq!(primaries.len(), 1);
        assert_eq!(dependent, 1);
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn invalid_default_restores_the_callers_context() {
        let source =
            SourceBuffer::new(b"template<class T,class U=T*>struct Box;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-context").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Box").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer
            .graph
            .own(global, name, super::super::lookup::Filter::Elaborated)
            .unwrap();
        let entity = analyzer.graph.binding(binding).entity;
        let head = analyzer.graph.entity(entity).template_head.unwrap();
        let default = analyzer.graph.templates.default_argument(head, 1).unwrap();
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        let reference = analyzer
            .graph
            .types
            .reference(int, super::super::types::RefKind::Lvalue)
            .unwrap();
        analyzer.template_depth = 4;
        analyzer.class_depth = 2;
        analyzer.unevaluated = 3;
        analyzer.template_head = Some(head);
        analyzer.default_argument_scope = Some(global);
        analyzer.initializing = Some(entity);
        analyzer.declaration_access.function = Some(entity);
        assert!(matches!(
            analyzer.class_default_type(default, head, &[reference]),
            Err(Error::InstantiationDiagnostic { .. })
        ));
        assert_eq!(analyzer.template_depth, 4);
        assert_eq!(analyzer.class_depth, 2);
        assert_eq!(analyzer.unevaluated, 3);
        assert_eq!(analyzer.template_head, Some(head));
        assert_eq!(analyzer.default_argument_scope, Some(global));
        assert_eq!(analyzer.initializing, Some(entity));
        assert_eq!(analyzer.declaration_access.function, Some(entity));
        assert!(analyzer.graph.lookup_limit.is_none());
        assert!(analyzer.graph.class_templates.instances.is_empty());
        assert!(analyzer.graph.class_templates.argument_defaults.is_empty());
    }
}
