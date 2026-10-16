// Translated from class_templates.cpp::complete_class/instantiate_class and
// analyzer_classes.cpp::define_class/class_body. See NOTICE.
//! Definition demand is separate from canonical class identity and layout.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, EntityId, ScopeId, ScopeKind},
    templates::HeadId,
};
use crate::syntax::{ast, cloning::Cloner};
use std::collections::HashMap;

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
#[derive(Default)]
pub(super) struct Definitions {
    patterns: HashMap<EntityId, Pattern>,
    states: HashMap<EntityId, State>,
    members: HashMap<EntityId, Pattern>,
}
impl Analyzer<'_> {
    pub(super) fn record_class_pattern(
        &mut self,
        entity: EntityId,
        syntax: ast::ClassId,
        parent: ScopeId,
    ) {
        if self.graph.entity(entity).template_head.is_some()
            && let Some(head) = self.template_head
        {
            self.graph.class_templates.definitions.patterns.insert(
                entity,
                Pattern {
                    syntax,
                    head: Some(head),
                    parent,
                    limit: self.graph.binding_limit(),
                },
            );
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
        let (pattern, arguments) = if let Some(instance) =
            self.graph.class_templates.instance(entity)
        {
            if instance.dependent {
                return Ok(false);
            }
            let Some(pattern) = self
                .graph
                .class_templates
                .definitions
                .patterns
                .get(&instance.primary)
                .copied()
            else {
                // A later definition can satisfy a later demand. Do not memoize
                // incompleteness when the primary is still only declared.
                return Ok(false);
            };
            (pattern, instance.arguments.clone())
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
        let result = self.required_instantiation(
            self.graph.instantiation_levels.get(entity)?,
            self.tree.classes.get(pattern.syntax).location,
            pattern.limit,
            |analyzer| {
                let frame = analyzer.class_definition_frame(pattern, &arguments)?;
                let copy = Cloner::new(analyzer.tree, analyzer.stack_budget, analyzer.stack_base)
                    .class_definition(pattern.syntax)
                    .map_err(|error| super::template_bodies::copy_error(&error))?;
                let node = *analyzer.tree.classes.get(copy);
                analyzer.class_body(node, entity, frame, None).map(|_| ())
            },
        );
        if result.is_ok() {
            self.graph
                .class_templates
                .definitions
                .states
                .insert(entity, State::Complete);
        } else {
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
        Ok(frame)
    }
}

impl super::graph::Graph {
    pub(super) fn within_class_instance(&self, mut scope: ScopeId) -> bool {
        loop {
            let node = self.scope(scope);
            if node.kind != ScopeKind::Class {
                return false;
            }
            let Some(owner) = node.owner else {
                return false;
            };
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
        let mut scopes = std::collections::HashSet::new();
        let mut fields = std::collections::HashSet::new();
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
