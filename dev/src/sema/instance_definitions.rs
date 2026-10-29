// Translated from class_templates.cpp::complete_class/instantiate_class and
// analyzer_classes.cpp::define_class/class_body. See NOTICE.
//! Definition demand is separate from canonical class identity and layout.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, ScopeId, ScopeKind},
    templates::HeadId,
};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use crate::syntax::ast;

#[derive(Clone, Copy)]
struct Pattern {
    syntax: ast::ClassId,
    head: Option<HeadId>,
    parent: ScopeId,
    limit: usize,
}
#[derive(Clone, Copy)]
enum State {
    InProgress,
    Complete,
}
struct ActiveClass {
    entity: EntityId,
    pattern: ast::ClassId,
    copy: ast::ClassId,
    declared: usize,
    linked: usize,
}
#[derive(Default)]
pub(super) struct Definitions {
    patterns: HashMap<EntityId, Pattern>,
    states: HashMap<EntityId, State>,
    members: HashMap<EntityId, Pattern>,
    sources: HashMap<EntityId, EntityId>,
    origins: HashMap<EntityId, EntityId>,
    active: Vec<ActiveClass>,
    explicit: HashSet<EntityId>,
}
impl Definitions {
    pub(super) fn explicit(&self, entity: EntityId) -> bool {
        self.explicit.contains(&entity)
    }

    pub(super) fn started(&self, entity: EntityId) -> bool {
        self.states.contains_key(&entity)
    }

    pub(super) fn specialize(&mut self, entity: EntityId) {
        self.explicit.insert(entity);
        self.states.insert(entity, State::Complete);
    }

    pub(super) fn active_depth(&self) -> usize {
        self.active.len()
    }

    pub(super) fn source(&self, entity: EntityId) -> Option<EntityId> {
        self.sources.get(&entity).copied()
    }
}
impl Analyzer<'_> {
    // Publish only the already declared prefix. Constant evaluation can ask
    // for a member's pattern before the enclosing instance has completed.
    pub(super) fn link_active_member_patterns(&mut self, owner: EntityId) {
        let Some(record) = self
            .graph
            .class_templates
            .definitions
            .active
            .iter_mut()
            .rev()
            .find(|record| record.entity == owner)
        else {
            return;
        };
        let (pattern, copy, first, end) =
            (record.pattern, record.copy, record.linked, record.declared);
        record.linked = record.linked.max(end);
        self.link_member_pattern_range(pattern, copy, first..end);
    }

    pub(super) fn declare_class_members(
        &mut self,
        entity: EntityId,
        members: crate::syntax::storage::List<ast::DeclId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        for index in 0..self.tree.decl_lists.get(members).len() {
            if let Some(record) = self.graph.class_templates.definitions.active.last_mut()
                && record.entity == entity
            {
                record.declared = index;
            }
            self.declaration(self.tree.decl_lists.get(members)[index], scope)?;
        }
        Ok(())
    }

    // template_names.cpp::pattern_member_template/original_member_template.
    // Follow only lexical owner edges; a copied declaration may predate the
    // original's outside definition, so absence of a definition is not cached.
    fn original_member_class(&self, mut entity: EntityId) -> Option<EntityId> {
        let requested = entity;
        let mut nested = Vec::new();
        loop {
            if self.graph.class_templates.definitions.explicit(entity) {
                return None;
            }
            let node = self.graph.entity(entity);
            let name = node.name?;
            let owner = self.graph.scope(node.home?).owner?;
            let source = self
                .graph
                .class_templates
                .partials
                .selected(owner)
                .map(|(source, _)| source)
                .or_else(|| {
                    self.graph
                        .class_templates
                        .definitions
                        .origins
                        .get(&owner)
                        .copied()
                })
                .or_else(|| {
                    self.graph
                        .class_templates
                        .instance(owner)
                        .map(|i| i.primary)
                })
                .or_else(|| self.graph.class_templates.definitions.source(owner));
            if let Some(source) = source {
                let mut source = self.original_class_member(source, name)?;
                for name in nested.into_iter().rev() {
                    source = self.original_class_member(source, name)?;
                }
                return (source != requested).then_some(source);
            }
            self.graph.entity(owner).template_head?;
            nested.push(name);
            entity = owner;
        }
    }

    fn original_class_member(
        &self,
        source: EntityId,
        name: crate::lex::identifiers::NameId,
    ) -> Option<EntityId> {
        let scope = self.graph.entity(source).scope?;
        let found = self
            .graph
            .own(scope, name, super::lookup::Filter::Elaborated)?;
        let binding = self.graph.binding(found);
        (binding.kind == super::graph::BindingKind::Type
            && self.graph.entity(binding.entity).enumeration.is_none())
        .then_some(binding.entity)
    }

    fn outside_class_definition(&self, mut entity: EntityId) -> Option<(EntityId, Pattern)> {
        // The frozen compiler bounds traversal of original member templates.
        for _ in 0..64 {
            entity = self.original_member_class(entity)?;
            if let Some(&pattern) = self.graph.class_templates.definitions.patterns.get(&entity) {
                return Some((entity, pattern));
            }
        }
        None
    }

    pub(super) fn record_class_pattern(
        &mut self,
        entity: EntityId,
        syntax: ast::ClassId,
        parent: ScopeId,
    ) {
        if self.template_depth != 0 {
            let head = if self.graph.entity(entity).template_head.is_some()
                || self.graph.scope(parent).kind == ScopeKind::TemplateParameters
            {
                self.template_head
            } else {
                None
            };
            self.graph.class_templates.definitions.patterns.insert(
                entity,
                Pattern {
                    syntax,
                    head,
                    parent,
                    limit: self.graph.binding_limit(),
                },
            );
        }
    }

    pub(super) fn record_member_class_source(
        &mut self,
        entity: EntityId,
        name: Option<crate::lex::identifiers::NameId>,
        home: ScopeId,
    ) {
        let Some(name) = name else { return };
        if self.template_depth != 0 || !self.graph.within_class_instance(home) {
            return;
        }
        let Some(owner) = self.graph.scope(home).owner else {
            return;
        };
        let source = self
            .graph
            .class_templates
            .partials
            .selected(owner)
            .map(|(pattern, _)| pattern)
            .or_else(|| {
                self.graph
                    .class_templates
                    .definitions
                    .origins
                    .get(&owner)
                    .copied()
            })
            .or_else(|| {
                self.graph
                    .class_templates
                    .instance(owner)
                    .map(|instance| instance.primary)
            })
            .or_else(|| {
                self.graph
                    .class_templates
                    .definitions
                    .sources
                    .get(&owner)
                    .copied()
            });
        let Some(scope) = source.and_then(|source| self.graph.entity(source).scope) else {
            return;
        };
        if let Some(binding) = self
            .graph
            .own(scope, name, super::lookup::Filter::Elaborated)
        {
            let source = self.graph.binding(binding).entity;
            self.graph
                .class_templates
                .definitions
                .sources
                .insert(entity, source);
            self.graph
                .instantiation_levels
                .record(entity, self.instantiation_depth);
        }
    }

    pub(super) fn defer_class_member(
        &mut self,
        entity: EntityId,
        node: ast::ClassSyntax,
        parent: ScopeId,
    ) -> Result<bool, Error> {
        if self.template_depth != 0
            || self.graph.entity(entity).name.is_none()
            || self.graph.class_templates.definitions.explicit(entity)
            || !self.graph.within_class_instance(parent)
        {
            return Ok(false);
        }
        let syntax = self
            .tree
            .classes
            .add(node)
            .map_err(|error| super::template_bodies::copy_error(&error))?;
        let limit = self
            .graph
            .lookup_limit
            .unwrap_or_else(|| self.graph.binding_limit());
        self.graph.class_templates.definitions.members.insert(
            entity,
            Pattern {
                syntax,
                head: None,
                parent,
                limit,
            },
        );
        self.graph
            .instantiation_levels
            .record(entity, self.instantiation_depth);
        Ok(true)
    }

    pub(super) fn complete_class(&mut self, entity: EntityId) -> Result<bool, Error> {
        let complete = self.complete_class_definition(entity)?;
        if !complete {
            // This observation only prevents the current substitution from
            // publishing a context-dependent result; it invalidates no cache.
            self.incomplete_class_observations = self
                .incomplete_class_observations
                .checked_add(1)
                .ok_or(Error::Invariant(
                    "incomplete class observation count overflow",
                ))?;
        }
        Ok(complete)
    }

    fn selected_class_definition(
        &mut self,
        primary: EntityId,
        arguments: &[super::types::TypeId],
    ) -> Result<(EntityId, Vec<super::types::TypeId>, bool), Error> {
        let (source, deduced) = self.matched_class_partial(primary, arguments)?;
        if source != primary {
            return Ok((source, deduced, false));
        }
        let mut member = self.original_member_class(primary);
        for _ in 0..64 {
            let Some(candidate) = member else { break };
            if self.graph.class_templates.partials.has_partials(candidate) {
                let owner = self
                    .graph
                    .entity(primary)
                    .home
                    .and_then(|scope| self.graph.scope(scope).owner);
                let (partial, deduced) =
                    self.matched_member_class_partial(candidate, arguments, owner)?;
                if partial != candidate {
                    return Ok((partial, deduced, true));
                }
                break;
            }
            member = self.original_member_class(candidate);
        }
        Ok((source, deduced, false))
    }

    fn complete_class_definition(&mut self, entity: EntityId) -> Result<bool, Error> {
        if self.graph.entity(entity).complete {
            return Ok(true);
        }
        if self
            .graph
            .class_templates
            .definitions
            .states
            .contains_key(&entity)
        {
            return Ok(false);
        }
        let mut outside = false;
        let mut origin = None;
        let (pattern, arguments) = if let Some(instance) =
            self.graph.class_templates.instance(entity)
        {
            if instance.dependent {
                return Ok(false);
            }
            let primary = instance.primary;
            let arguments = instance.arguments.clone();
            // Selection can recursively observe this shell. Publish only the
            // in-progress guard until a selected definition actually exists.
            self.graph
                .class_templates
                .definitions
                .states
                .insert(entity, State::InProgress);
            let selected = self.selected_class_definition(primary, &arguments);
            let (source, arguments, selected_outside) = match selected {
                Ok(selected) => selected,
                Err(error) => {
                    self.graph
                        .class_templates
                        .definitions
                        .states
                        .remove(&entity);
                    return Err(error);
                }
            };
            outside = selected_outside;
            let Some(pattern) = self
                .graph
                .class_templates
                .definitions
                .patterns
                .get(&source)
                .copied()
                .or_else(|| {
                    if source != primary {
                        return None;
                    }
                    let (source, pattern) = self.outside_class_definition(source)?;
                    origin = Some(source);
                    outside = true;
                    Some(pattern)
                })
            else {
                self.graph
                    .class_templates
                    .definitions
                    .states
                    .remove(&entity);
                let complete = source == primary && self.complete_builtin_list(entity)?;
                if complete {
                    self.graph
                        .class_templates
                        .definitions
                        .states
                        .insert(entity, State::Complete);
                }
                return Ok(complete);
            };
            if source != primary {
                self.graph
                    .class_templates
                    .partials
                    .select(entity, source, arguments.clone());
            }
            (pattern, arguments)
        } else if let Some(pattern) = self
            .graph
            .class_templates
            .definitions
            .sources
            .get(&entity)
            .and_then(|&source| {
                self.graph
                    .class_templates
                    .definitions
                    .patterns
                    .get(&source)
                    .copied()
                    .or_else(|| {
                        self.outside_class_definition(source)
                            .map(|(source, pattern)| {
                                origin = Some(source);
                                pattern
                            })
                    })
            })
        {
            (pattern, Vec::new())
        } else if let Some(&pattern) = self.graph.class_templates.definitions.members.get(&entity) {
            (pattern, Vec::new())
        } else {
            return Ok(false);
        };
        self.graph
            .class_templates
            .definitions
            .states
            .insert(entity, State::InProgress);
        if let Some(origin) = origin {
            self.graph
                .class_templates
                .definitions
                .origins
                .insert(entity, origin);
        }
        let result = (|| {
            let frame = if self
                .graph
                .class_templates
                .definitions
                .sources
                .contains_key(&entity)
            {
                let owner = self
                    .graph
                    .entity(entity)
                    .home
                    .ok_or(Error::Invariant("member class without owner"))?;
                self.outside_class_frame(owner, pattern.head, pattern.parent)?
            } else if outside {
                let owner = self
                    .graph
                    .entity(entity)
                    .home
                    .ok_or(Error::Invariant("member template without owner"))?;
                let head = pattern
                    .head
                    .ok_or(Error::Invariant("outside template without head"))?;
                self.outside_template_frame(owner, head, &arguments, pattern.parent)?
            } else {
                self.class_definition_frame(pattern, &arguments)?
            };
            let mut packs =
                super::pack_frames::FramePacks::new(&self.graph, frame, self.identifiers);
            let copied = packs
                .cloner(self.tree, self.stack_budget, self.stack_base)
                .class_definition(pattern.syntax);
            let copy = packs.finish(copied)?;
            self.required_instantiation(
                self.graph.instantiation_levels.get(entity)?,
                self.tree.classes.get(pattern.syntax).location,
                pattern.limit,
                |analyzer| {
                    let node = *analyzer.tree.classes.get(copy);
                    analyzer
                        .graph
                        .class_templates
                        .definitions
                        .active
                        .push(ActiveClass {
                            entity,
                            pattern: pattern.syntax,
                            copy,
                            declared: 0,
                            linked: 0,
                        });
                    let result = analyzer.class_body(node, entity, frame, None);
                    analyzer.graph.class_templates.definitions.active.pop();
                    result?;
                    analyzer.link_member_patterns(pattern.syntax, copy);
                    analyzer.finish_member_enums(entity)?;
                    Ok(())
                },
            )
        })();
        if result.is_ok() {
            self.graph.work.totals.instantiated_classes += 1;
            self.graph
                .class_templates
                .definitions
                .states
                .insert(entity, State::Complete);
        } else {
            self.graph
                .class_templates
                .definitions
                .origins
                .remove(&entity);
            self.graph.class_templates.partials.forget_selection(entity);
            self.graph
                .class_templates
                .definitions
                .states
                .remove(&entity);
        }
        result?;
        Ok(self.graph.entity(entity).complete)
    }

    fn class_definition_frame(
        &mut self,
        pattern: Pattern,
        arguments: &[super::types::TypeId],
    ) -> Result<ScopeId, Error> {
        let Some(head) = pattern.head else {
            return Ok(pattern.parent);
        };
        let source = self.graph.templates.parameter_scope(head);
        let parent = self
            .graph
            .scope(pattern.parent)
            .parent
            .ok_or(Error::Invariant("class definition without parent"))?;
        let frame = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, parent, None, None)?;
        let depth = self.graph.templates.depth(head);
        for index in 0..self.graph.scope(source).bindings.len() {
            let binding = self.graph.binding(self.graph.scope(source).bindings[index]);
            let Some(position) = self
                .graph
                .entity(binding.entity)
                .template_parameter
                .and_then(|info| info.index(depth))
            else {
                continue;
            };
            let ty = *arguments
                .get(position)
                .ok_or(Error::Invariant("class argument frame without argument"))?;
            self.bind_template_argument(frame, binding, ty)?;
        }
        Ok(frame)
    }
}

impl super::graph::Graph {
    pub(crate) fn lower_class_instantiated_elsewhere(&self, mut class: EntityId) -> bool {
        // vtable_lowering.cpp::instantiated_elsewhere walks only the lexical
        // class owners carrying template facts. A nearer explicit definition
        // wins over an enclosing extern declaration.
        loop {
            let definitions = &self.class_templates.definitions;
            if self.class_templates.instance(class).is_none()
                && !definitions.sources.contains_key(&class)
                && !definitions.members.contains_key(&class)
                && !definitions.patterns.contains_key(&class)
            {
                return false;
            }
            let instantiations = &self.function_templates.instantiations;
            if instantiations.defined(class) {
                return false;
            }
            if instantiations.declared(class) {
                return true;
            }
            let Some(parent) = self.entity(class).home else {
                return false;
            };
            let scope = self.scope(parent);
            let Some(owner) = scope.owner.filter(|_| scope.kind == ScopeKind::Class) else {
                return false;
            };
            class = owner;
        }
    }

    pub(super) fn instance_class(&self, scope: ScopeId) -> bool {
        let node = self.scope(scope);
        node.kind == ScopeKind::Class
            && node.owner.is_some_and(|class| {
                let definitions = &self.class_templates.definitions;
                !definitions.explicit(class)
                    && (self.class_templates.instance(class).is_some()
                        || definitions.sources.contains_key(&class)
                        || definitions.members.contains_key(&class))
            })
    }

    pub(super) fn within_class_instance(&self, mut scope: ScopeId) -> bool {
        loop {
            let node = self.scope(scope);
            if node.kind != ScopeKind::Class {
                return false;
            }
            let Some(owner) = node.owner else {
                return false;
            };
            if self.class_templates.definitions.explicit(owner) {
                return false;
            }
            if self.class_templates.instance(owner).is_some() {
                return true;
            }
            let Some(parent) = self.entity(owner).home else {
                return false;
            };
            scope = parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{lookup::Filter, types::TypeKind};
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn outside_definitions_retain_origins_and_independent_instance_demand() {
        let source = SourceBuffer::new(
            b"template<class A>struct X{template<class B>struct Y;};\
              using I=X<int>::Y<char>;using L=X<long>::Y<short>;\
              template<class T>template<class U>struct X<T>::Y{\
                T a;U b;struct Nested;void unused(){U::missing;}};\
              template<class T>template<class U>struct X<T>::Y<U>::Nested{T a;U b;};\
              using N=I::Nested;int size=sizeof(I)+sizeof(N);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"outside-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"I".as_slice(), b"L", b"N"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
                let TypeKind::Class(class) = analyzer
                    .graph
                    .types
                    .get(analyzer.graph.binding(binding).ty.unwrap())
                    .kind
                else {
                    panic!("class alias")
                };
                class
            })
            .collect();
        assert!(analyzer.graph.entity(classes[0]).complete);
        assert!(!analyzer.graph.entity(classes[1]).complete);
        assert!(analyzer.graph.entity(classes[1]).scope.is_none());
        assert!(analyzer.graph.entity(classes[2]).complete);
        let definitions = &analyzer.graph.class_templates.definitions;
        let origin = definitions.origins[&classes[0]];
        assert_ne!(origin, classes[0]);
        assert!(definitions.patterns.contains_key(&origin));
        assert!(definitions.sources.contains_key(&classes[2]));
        assert!(definitions.active.is_empty());
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        let before = (
            analyzer.tree.declarations.mark(),
            analyzer.tree.types.mark(),
            analyzer.graph.binding_limit(),
        );
        for _ in 0..8 {
            assert!(analyzer.complete_class(classes[0]).unwrap());
            assert!(analyzer.complete_class(classes[2]).unwrap());
        }
        assert_eq!(
            before,
            (
                analyzer.tree.declarations.mark(),
                analyzer.tree.types.mark(),
                analyzer.graph.binding_limit()
            )
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn failed_outside_partial_selection_publishes_no_definition() {
        let source = SourceBuffer::new(
            b"template<class A>struct X{template<class B,class C>struct Y;};\
              template<class A>template<class B>struct X<A>::Y<B*,int>{};\
              template<class A>template<class C>struct X<A>::Y<int*,C>{};\
              using Bad=X<char>::Y<int*,int>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"outside-trial").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Bad").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
        let TypeKind::Class(class) = analyzer
            .graph
            .types
            .get(analyzer.graph.binding(binding).ty.unwrap())
            .kind
        else {
            panic!("class alias")
        };
        for _ in 0..2 {
            assert!(matches!(
                analyzer.complete_class(class),
                Err(Error::Diagnostic {
                    message: "a specialization matches several partial specializations, none more specialized",
                    ..
                })
            ));
            let definitions = &analyzer.graph.class_templates.definitions;
            assert!(!definitions.started(class));
            assert!(!definitions.origins.contains_key(&class));
            assert!(definitions.active.is_empty());
            assert!(
                analyzer
                    .graph
                    .class_templates
                    .partials
                    .selected(class)
                    .is_none()
            );
            assert!(analyzer.graph.entity(class).scope.is_none());
        }
    }

    #[test]
    fn definitions_and_named_members_have_independent_memoized_demand() {
        let source = SourceBuffer::new(
            b"template<class A>struct Box;\
            template<class U>struct Box{U data;U get(){return data;}void bad(){U::missing;}\
                struct Nested{typename U::missing value;};};\
            using I=Box<int>;using C=Box<char>;using F=Box<long>;\
            int k=sizeof(I)+sizeof(C);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"definition-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"I".as_slice(), b"C", b"F", b"Nested"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let original_bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes: Vec<_> = names[..3]
            .iter()
            .map(|&name| {
                let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
                let TypeKind::Class(class) = analyzer
                    .graph
                    .types
                    .get(analyzer.graph.binding(binding).ty.unwrap())
                    .kind
                else {
                    panic!("alias lost class");
                };
                class
            })
            .collect();
        assert_eq!(analyzer.graph.class_templates.definitions.states.len(), 2);
        assert!(analyzer.graph.entity(classes[2]).scope.is_none());
        let mut scopes = crate::support::hash::HashSet::default();
        let mut fields = crate::support::hash::HashSet::default();
        for &class in &classes[..2] {
            assert!(analyzer.graph.entity(class).complete);
            let scope = analyzer.graph.entity(class).scope.unwrap();
            assert!(scopes.insert(scope));
            let field = analyzer.graph.entity(class).members[0];
            assert!(fields.insert(field));
            assert!(
                !analyzer
                    .graph
                    .dependent_type(analyzer.graph.entity(field).ty.unwrap())
            );
            let binding = analyzer
                .graph
                .own(scope, names[3], Filter::Ordinary)
                .unwrap();
            let nested = analyzer.graph.binding(binding).entity;
            assert!(analyzer.graph.entity(nested).scope.is_none());
            assert!(
                analyzer
                    .graph
                    .class_templates
                    .definitions
                    .members
                    .contains_key(&nested)
            );
        }
        // A definition copies declarations but no member function body. Reusing
        // its completed identity adds no syntax and preserves its scope.
        assert_eq!(analyzer.tree.bodies.mark(), original_bodies);
        let before = (
            analyzer.tree.expressions.mark(),
            analyzer.tree.declarations.mark(),
            analyzer.tree.types.mark(),
        );
        for _ in 0..8 {
            assert!(analyzer.complete_class(classes[0]).unwrap());
        }
        assert_eq!(
            before,
            (
                analyzer.tree.expressions.mark(),
                analyzer.tree.declarations.mark(),
                analyzer.tree.types.mark()
            )
        );
        assert!(analyzer.graph.entity(classes[2]).scope.is_none());
        assert!(analyzer.deferred.is_empty());
    }

    #[test]
    fn failed_required_definition_restores_the_demanding_context() {
        let source = SourceBuffer::new(
            b"template<class T>struct Bad{typename T::missing field;};using B=Bad<int>;".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"failed-definition").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"B").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
        let TypeKind::Class(class) = analyzer
            .graph
            .types
            .get(analyzer.graph.binding(binding).ty.unwrap())
            .kind
        else {
            panic!("alias lost class");
        };
        analyzer.template_depth = 3;
        analyzer.class_depth = 2;
        analyzer.unevaluated = 4;
        analyzer.initializing = Some(class);
        analyzer.declaration_access.function = Some(class);
        analyzer.graph.lookup_limit = Some(0);
        assert!(matches!(
            analyzer.complete_class(class),
            Err(Error::InstantiationDiagnostic { .. })
        ));
        assert_eq!(analyzer.template_depth, 3);
        assert_eq!(analyzer.class_depth, 2);
        assert_eq!(analyzer.unevaluated, 4);
        assert_eq!(analyzer.initializing, Some(class));
        assert_eq!(analyzer.declaration_access.function, Some(class));
        assert_eq!(analyzer.graph.lookup_limit, Some(0));
        assert!(
            !analyzer
                .graph
                .class_templates
                .definitions
                .states
                .contains_key(&class)
        );
    }
}
