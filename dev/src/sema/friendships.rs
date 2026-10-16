// Translated from sema/analyzer_access.cpp and name_lookup.cpp. See NOTICE.
//! Unit-owned friendship edges and hidden-friend indexes use canonical identities.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::ValueKey,
    graph::{BindingId, EntityId, ScopeId, ScopeKind},
    types::TypeKind,
};
use crate::support::source::SourceOffset;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Friendships {
    edges: HashSet<(EntityId, EntityId)>,
    granting: HashMap<EntityId, Vec<EntityId>>,
    hidden: HashSet<BindingId>,
    functions: HashMap<(EntityId, ValueKey), Vec<BindingId>>,
}
impl Friendships {
    pub(super) fn record(&mut self, class: EntityId, entity: EntityId) {
        if self.edges.insert((class, entity)) {
            self.granting.entry(entity).or_default().push(class);
        }
    }
    pub(super) fn grants(&self, class: EntityId, entity: EntityId) -> bool {
        self.edges.contains(&(class, entity))
    }
    pub(super) fn granting(&self, entity: EntityId) -> &[EntityId] {
        self.granting.get(&entity).map_or(&[], Vec::as_slice)
    }
    pub(super) fn hide(&mut self, binding: BindingId) {
        self.hidden.insert(binding);
    }
    pub(super) fn hidden(&self, binding: BindingId) -> bool {
        self.hidden.contains(&binding)
    }
    pub(super) fn index(&mut self, class: EntityId, key: ValueKey, binding: BindingId) {
        self.functions
            .entry((class, key))
            .or_default()
            .push(binding);
    }
    pub(super) fn functions(&self, class: EntityId, key: ValueKey) -> &[BindingId] {
        self.functions.get(&(class, key)).map_or(&[], Vec::as_slice)
    }
}
impl Analyzer<'_> {
    pub(super) fn function_template_id(
        &self,
        info: &super::declarators::DeclaratorInfo,
    ) -> Option<crate::syntax::ast::NameComponent> {
        let name = self.tree.names.get(info.syntax_name?);
        self.tree
            .name_lists
            .get(name.components)
            .last()
            .copied()
            .filter(|component| component.arguments.is_some())
    }

    pub(super) fn templated_friend_class(&self, scope: ScopeId) -> bool {
        std::iter::successors(Some(self.declaration_scope(scope)), |&id| {
            self.graph.scope(id).parent
        })
        .take_while(|&id| self.graph.scope(id).kind == ScopeKind::Class)
        .any(|id| {
            self.graph
                .scope(id)
                .owner
                .is_some_and(|owner| self.graph.entity(owner).template_head.is_some())
        })
    }

    // analyzer_declarations.cpp::friend_specialization. A friend reference
    // has no namespace/class lookup or ADL publication and demands no body.
    pub(super) fn declare_friend_specialization(
        &mut self,
        info: &super::declarators::DeclaratorInfo,
        specs: super::specifiers::Specifiers,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        if self.function_template_id(info).is_some()
            && (specs.inline == super::specifiers::InlinePreference::Inline
                || info.parameters.is_some_and(|clause| {
                    self.tree
                        .parameters
                        .get(clause.parameters)
                        .iter()
                        .any(|parameter| parameter.default.is_some())
                }))
        {
            return Err(Error::at(
                location,
                "a default argument or inline in a friend declaration of a specialization",
            ));
        }
        let class = self.friend_owner(scope, location)?;
        // A class template retains its friend pattern; each instance resolves
        // it in its own frame. Do not grant the primary or select a signature.
        if self.templated_friend_class(scope) {
            let name = self.tree.names.get(
                info.syntax_name
                    .ok_or(Error::Invariant("friend pattern without a name"))?,
            );
            let component = self
                .tree
                .name_lists
                .get(name.components)
                .last()
                .ok_or(Error::Invariant("empty friend pattern name"))?;
            let key = Self::expression_name(component.kind)?;
            let home = info.home.unwrap_or(self.enclosing_namespace(scope));
            let entity = self.graph.add_entity(
                Some(info.ty),
                info.name,
                crate::lex::vocabulary::Keyword::Int,
            )?;
            self.graph.entity_mut(entity).home = Some(home);
            self.graph.entity_mut(entity).function = Some(super::graph::FunctionProperties {
                no_inline: false,
                internal_linkage: false,
                language: super::linkage::Language::Cpp,
                denotes: None,
                is_static: false,
                special: key.special(),
                allocation: None,
            });
            return self.graph.add_hidden_binding_in(
                home,
                super::graph::Binding {
                    kind: super::graph::BindingKind::Function,
                    entity,
                    ty: Some(info.ty),
                    name: key.identifier(),
                    key: crate::lex::vocabulary::Keyword::Int,
                },
            );
        }
        let binding = self
            .declared_function_specialization(info, scope, location)?
            .ok_or_else(|| {
                Error::at(
                    location,
                    "a friend declaration names no function template specialization",
                )
            })?;
        self.graph
            .friendships
            .record(class, self.graph.binding(binding).entity);
        self.graph
            .add_hidden_function(binding, self.graph.binding(binding))
    }

    pub(super) fn friend_environment(
        &self,
        specs: super::specifiers::Specifiers,
        scope: ScopeId,
        environment: &mut super::declarators::DeclaratorEnvironment,
    ) {
        environment.friendship = specs.friendship;
        if specs.friendship == super::specifiers::Friendship::Friend
            && environment
                .home
                .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Namespace)
        {
            environment.lookup = scope;
        }
    }
    pub(super) fn elaborated_friend_type(
        &mut self,
        key: crate::lex::vocabulary::Keyword,
        name: crate::syntax::ast::Name,
        scope: ScopeId,
    ) -> Result<super::types::TypeId, Error> {
        let class = self.friend_owner(scope, self.tree.names.get(name).location)?;
        let mut home = self.declaration_scope(scope);
        while !matches!(
            self.graph.scope(home).kind,
            ScopeKind::Namespace | ScopeKind::Block
        ) {
            home = self
                .graph
                .scope(home)
                .parent
                .ok_or(Error::Invariant("friend class without enclosing scope"))?;
        }
        let prefix = self.prefix(name, scope)?;
        let last = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()
            .ok_or(Error::Invariant("empty friend class name"))?;
        let named = Self::component_name(last.kind)?;
        if prefix.is_none() {
            self.check_template_parameter_name(scope, named, self.tree.names.get(name).location)?;
        }
        self.check_friend_template_defaults(scope)?;
        let previous = if let Some(prefix) = prefix {
            self.qualified(prefix, named, super::lookup::Filter::Elaborated)?
        } else {
            let mut current = Some(scope);
            let mut found = None;
            while let Some(owner) = current {
                let node = self.graph.scope(owner);
                if matches!(
                    node.kind,
                    ScopeKind::Class | ScopeKind::Namespace | ScopeKind::Block
                ) {
                    found = self
                        .graph
                        .declared_name(owner, named, super::lookup::Filter::Elaborated)
                        .filter(|&binding| {
                            node.kind != ScopeKind::Class
                                || Some(self.graph.binding(binding).entity) != node.owner
                        });
                    if found.is_some()
                        || matches!(node.kind, ScopeKind::Namespace | ScopeKind::Block)
                    {
                        break;
                    }
                }
                current = node.parent;
            }
            found
        };
        let ty = if let Some(binding) = previous {
            let entity = self.elaborated_class_entity(
                binding,
                key,
                self.tree.names.get(name).location,
                false,
            )?;
            self.graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("friend class without type"))?
        } else {
            let ty = self.elaborated_type(key, name, home, true)?;
            let binding = self
                .graph
                .declared_name(home, named, super::lookup::Filter::Elaborated)
                .ok_or(Error::Invariant("friend class declaration without binding"))?;
            self.graph.friendships.hide(binding);
            ty
        };
        if let TypeKind::Class(entity) = self.graph.types.get(ty).kind {
            if self.active_template_head(scope).is_some() {
                self.class_template_head(
                    entity,
                    scope,
                    previous.is_some(),
                    self.tree.names.get(name).location,
                )?;
            }
            self.graph.friendships.record(class, entity);
        }
        Ok(ty)
    }
    pub(super) fn friend_owner(
        &self,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<EntityId, Error> {
        let scope = self.declaration_scope(scope);
        if self.graph.scope(scope).kind != ScopeKind::Class {
            return Err(Error::at(location, "friend declaration outside a class"));
        }
        self.graph
            .scope(scope)
            .owner
            .ok_or(Error::Invariant("friend without class"))
    }
    pub(super) fn friend_type(
        &mut self,
        ty: super::types::TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let class = self.friend_owner(scope, location)?;
        if let TypeKind::Class(entity) = self.graph.types.get(ty).kind {
            self.graph.friendships.record(class, entity);
        } else if matches!(self.graph.types.get(ty).kind, TypeKind::Auto) {
            return Err(Error::at(location, "auto in a friend declaration"));
        }
        Ok(())
    }
    // Settlement checks an out-of-class signature in its selected member's
    // lexical class context, including enclosing classes and their grants.
    fn access_scopes(&self, scope: ScopeId) -> impl Iterator<Item = ScopeId> + '_ {
        let scope = self
            .declaration_access
            .substitution
            .filter(|&(root, _)| self.graph.encloses(root, scope))
            .map_or(scope, |(_, point)| point);
        self.declaration_access
            .class
            .or_else(|| {
                self.declaration_access
                    .function
                    .and_then(|function| self.graph.entity(function).home)
            })
            .filter(|&home| self.graph.scope(home).kind == ScopeKind::Class)
            .into_iter()
            .flat_map(|home| std::iter::successors(Some(home), |&id| self.graph.scope(id).parent))
            .chain(std::iter::successors(Some(scope), |&id| {
                self.graph.scope(id).parent
            }))
    }
    pub(super) fn member_or_friend(&self, scope: ScopeId, class: EntityId) -> bool {
        if self
            .declaration_access
            .function
            .is_some_and(|function| self.graph.friendships.grants(class, function))
        {
            return true;
        }
        self.access_scopes(scope).any(|scope| {
            let current = self.graph.scope(scope);
            matches!(current.kind, ScopeKind::Class | ScopeKind::Function)
                && current.owner.is_some_and(|entity| {
                    current.kind == ScopeKind::Class && entity == class
                        || self.graph.friendships.grants(class, entity)
                })
        })
    }
    pub(super) fn access_contexts(&self, scope: ScopeId) -> impl Iterator<Item = EntityId> + '_ {
        self.declaration_access
            .function
            .into_iter()
            .flat_map(|function| self.graph.friendships.granting(function).iter().copied())
            .chain(
                self.access_scopes(scope)
                    .filter_map(|scope| {
                        let current = self.graph.scope(scope);
                        matches!(current.kind, ScopeKind::Class | ScopeKind::Function)
                            .then_some((current.kind, current.owner?))
                    })
                    .flat_map(|(kind, entity)| {
                        (kind == ScopeKind::Class)
                            .then_some(entity)
                            .into_iter()
                            .chain(self.graph.friendships.granting(entity).iter().copied())
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{
            Error,
            analyzer::Analyzer,
            graph::{BindingId, Graph},
            lookup::Filter,
            types::TypeKind,
        },
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    fn friend_bindings(
        tree: &ast::Tree,
        graph: &Graph,
        declaration: ast::DeclId,
    ) -> Vec<BindingId> {
        let ast::DeclKind::Class(specifier) = tree.declarations.get(declaration).kind else {
            panic!("expected class declaration");
        };
        let ast::SpecifierKind::Class(class) = specifier.kind else {
            panic!("expected class syntax");
        };
        tree.decl_lists
            .get(tree.classes.get(class).members)
            .iter()
            .filter_map(|&member| {
                let ast::DeclKind::Simple { declarators, .. } = tree.declarations.get(member).kind
                else {
                    return None;
                };
                Some(tree.init_declarators.get(declarators))
            })
            .flatten()
            .map(|init| graph.declared_bindings[init.declarator.index()].unwrap())
            .collect()
    }

    #[test]
    fn specialization_friendship_reuses_identity_without_lookup_or_body_demand() {
        let source = SourceBuffer::new(
            b"template<class T>int f(T,int=sizeof(T));\
              class A{friend int f<>(int,int);friend int f<int>(int,int);\
                      friend int f<char>(char,int);};\
              class B{friend int f<>(int,int);};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"friend-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let a = cursor.intern_name(b"A").unwrap();
        let b = cursor.intern_name(b"B").unwrap();
        let expressions = tree.expressions.mark();
        let syntax_declarations = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let graph = &analyzer.graph;
        let primaries: Vec<_> = graph.functions_at(global, f.into()).collect();
        assert_eq!(primaries.len(), 1);
        let primary = graph.binding(primaries[0]).entity;
        let statements = tree.decl_lists.get(declarations);
        let a_friends = friend_bindings(tree, graph, statements[1]);
        let b_friends = friend_bindings(tree, graph, statements[2]);
        assert_ne!(a_friends[0], a_friends[1]);
        let int_function = graph.binding(a_friends[0]).entity;
        let char_function = graph.binding(a_friends[2]).entity;
        assert_eq!(int_function, graph.binding(a_friends[1]).entity);
        assert_eq!(int_function, graph.binding(b_friends[0]).entity);
        assert_ne!(int_function, char_function);
        for (name, char_granted) in [(a, true), (b, false)] {
            let class = graph
                .binding(graph.own(global, name, Filter::Elaborated).unwrap())
                .entity;
            assert!(graph.friendships.grants(class, int_function));
            assert_eq!(graph.friendships.grants(class, char_function), char_granted);
            assert!(!graph.friendships.grants(class, primary));
            assert!(graph.friendships.functions(class, f.into()).is_empty());
            let home = graph.entity(class).scope.unwrap();
            assert!(graph.own(home, f, Filter::Ordinary).is_none());
            assert!(graph.functions_at(home, f.into()).next().is_none());
        }
        for binding in a_friends.into_iter().chain(b_friends) {
            let function = graph.binding(binding).entity;
            assert_eq!(
                graph.function_templates.primary(function),
                Some(primaries[0])
            );
            assert!(!graph.entity(function).defined);
            assert!(graph.entity(function).scope.is_none());
            assert!(graph.entity(function).function_definition.is_none());
            assert!(graph.entity(function).defaults.is_empty());
            assert!(graph.entity(function).demands.as_slice().is_empty());
        }
        let default = graph.entity(primary).defaults[1].unwrap().expression;
        assert!(graph.expression_facts.get(default).is_err());
        assert!(!graph.function_templates.body_demanded(primary));
        assert!(graph.root_demands.as_slice().is_empty());
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), syntax_declarations);
    }

    #[test]
    fn rejected_specialization_friend_does_not_grant_or_publish_a_function() {
        for (source, message) in [
            (
                "template<class T>long f(T);class A{friend int f<>(int);};",
                "a friend declaration names no function template specialization",
            ),
            (
                "template<class T>int f(T&);template<class T>int f(const T&);\
                 class A{friend int f<>(const int&);};",
                "a declaration matches several function templates",
            ),
            (
                "class A{friend inline int absent<>(int);};",
                "a default argument or inline in a friend declaration of a specialization",
            ),
        ] {
            let source = SourceBuffer::new(source.as_bytes().to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"friend-failure").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let a = cursor.intern_name(b"A").unwrap();
            let f = cursor.intern_name(b"f").unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            assert!(
                matches!(analyzer.declarations(declarations, global), Err(Error::Diagnostic { message: actual, .. }) if actual == message)
            );
            let class = analyzer
                .graph
                .binding(analyzer.graph.own(global, a, Filter::Elaborated).unwrap())
                .entity;
            for binding in analyzer.graph.functions_at(global, f.into()) {
                let entity = analyzer.graph.binding(binding).entity;
                assert!(analyzer.graph.entity(entity).template_head.is_some());
                assert!(!analyzer.graph.friendships.grants(class, entity));
            }
            assert!(analyzer.graph.friendships.edges.is_empty());
            assert!(
                analyzer
                    .graph
                    .friendships
                    .functions(class, f.into())
                    .is_empty()
            );
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
            assert_eq!(analyzer.template_depth, 0);
            assert!(analyzer.template_head.is_none());
            assert!(analyzer.graph.lookup_limit.is_none());
        }
    }

    #[test]
    fn templated_class_friend_pattern_neither_resolves_nor_grants_a_specialization() {
        let source = SourceBuffer::new(
            b"namespace N{int h(int);}\
              template<class T>struct A{friend int f<>(T);friend int N::h(T);\
                                       struct B{friend int g<>(T);};};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"friend-pattern").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let g = cursor.intern_name(b"g").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert!(analyzer.graph.own(global, f, Filter::Ordinary).is_none());
        assert!(analyzer.graph.own(global, g, Filter::Ordinary).is_none());
        assert!(analyzer.graph.friendships.edges.is_empty());
        assert!(analyzer.graph.friendships.functions.is_empty());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        let ast::DeclKind::Template { declaration, .. } = analyzer
            .tree
            .declarations
            .get(analyzer.tree.decl_lists.get(declarations)[1])
            .kind
        else {
            panic!("expected template");
        };
        let binding = friend_bindings(analyzer.tree, &analyzer.graph, declaration)[0];
        let entity = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity);
        let TypeKind::Function { .. } = analyzer.graph.types.get(entity.ty.unwrap()).kind else {
            panic!("expected function type");
        };
        assert!(entity.template_head.is_none());
        assert!(entity.scope.is_none());
        assert!(entity.function_parameters.is_none());
        assert!(entity.function_definition.is_none());
        assert!(
            analyzer
                .graph
                .function_templates
                .primary(analyzer.graph.binding(binding).entity)
                .is_none()
        );
    }
}
