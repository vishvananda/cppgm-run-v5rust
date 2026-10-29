// Translated from template_specializations.cpp::partial_specialization,
// class_partial_specialization/matching_partial. See NOTICE.
//! Partial declarations own patterns; concrete shells retain primary identity.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId},
    lookup::Filter,
    partial_specializations::PartialLists,
    types::{Cv, TypeId, TypeKind},
};
use crate::support::hash::HashMap;
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

#[derive(Default)]
pub(super) struct ClassPartials {
    identities: HashMap<EntityId, HashMap<Vec<TypeId>, EntityId>>,
    patterns: HashMap<EntityId, Vec<(Vec<TypeId>, EntityId)>>,
    primaries: HashMap<EntityId, EntityId>,
    lists: PartialLists,
    bindings: HashMap<EntityId, BindingId>,
    selections: HashMap<EntityId, (EntityId, Vec<TypeId>)>,
}
impl ClassPartials {
    pub(super) fn has_partials(&self, primary: EntityId) -> bool {
        self.patterns.contains_key(&primary)
    }
    pub(super) fn replace_primary(&mut self, entity: EntityId) {
        self.patterns.remove(&entity);
        self.identities.remove(&entity);
        self.lists.clear(entity);
    }
    pub(super) fn primary(&self, entity: EntityId) -> Option<EntityId> {
        self.primaries.get(&entity).copied()
    }
    pub(super) fn selected(&self, entity: EntityId) -> Option<(EntityId, &[TypeId])> {
        self.selections
            .get(&entity)
            .map(|(pattern, arguments)| (*pattern, arguments.as_slice()))
    }
    pub(super) fn select(&mut self, entity: EntityId, pattern: EntityId, arguments: Vec<TypeId>) {
        self.selections.insert(entity, (pattern, arguments));
    }
    pub(super) fn forget_selection(&mut self, entity: EntityId) {
        self.selections.remove(&entity);
    }
    pub(super) fn current(
        &self,
        primary: EntityId,
        arguments: &[TypeId],
    ) -> Option<(EntityId, BindingId)> {
        let entity = *self.identities.get(&primary)?.get(arguments)?;
        self.bindings.get(&entity).map(|&binding| (entity, binding))
    }
}
impl Analyzer<'_> {
    pub(super) fn partial_class(
        &mut self,
        name: ast::Name,
        key: Keyword,
        scope: ScopeId,
    ) -> Result<EntityId, Error> {
        let node = *self.tree.names.get(name);
        let found = self
            .lookup_result_in(
                name,
                scope,
                Filter::Elaborated,
                super::lookup::Specialization::None,
            )?
            .binding
            .ok_or_else(|| Error::at(node.location, "undeclared name"))?;
        let primary = self.class_template_primary(found).ok_or_else(|| {
            Error::at(
                node.location,
                "a partial specialization of no class template",
            )
        })?;
        if (key == Keyword::Union) != (self.graph.entity(primary).key == Keyword::Union) {
            return Err(Error::at(
                node.location,
                "class-key does not agree with the template's",
            ));
        }
        let home = self
            .graph
            .entity(primary)
            .home
            .ok_or(Error::Invariant("partial without owner"))?;
        if !self.graph.encloses(self.declaration_scope(scope), home) {
            return Err(Error::at(
                node.location,
                "a partial specialization outside its template's scope",
            ));
        }
        let head = self
            .active_template_head(scope)
            .ok_or(Error::Invariant("partial without head"))?;
        let primary_head = self
            .graph
            .entity(primary)
            .context()
            .template_head
            .ok_or(Error::Invariant("primary without head"))?;
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .copied()
            .ok_or(Error::Invariant("empty partial name"))?;
        let mut arguments = self.form_template_arguments(last, scope, Some(primary_head))?;
        self.fill_template_arguments_in(primary_head, &mut arguments, node.location, None)?;
        self.match_declaration_arguments(primary_head, &mut arguments, node.location)?;
        if let Some(&earlier) = self
            .graph
            .class_templates
            .partials
            .identities
            .get(&primary)
            .and_then(|known| known.get(&arguments))
        {
            self.merge_template_head(earlier, head, node.location)?;
            self.check_type_partial_arguments(head, &arguments, node.location)?;
            self.check_partial_value_types(primary_head, head, &arguments, node.location)?;
            return Ok(earlier);
        }
        self.check_type_partial_arguments(head, &arguments, node.location)?;
        self.check_partial_value_types(primary_head, head, &arguments, node.location)?;
        let template = self.graph.entity(primary);
        let (name, key) = (template.name, template.key);
        let entity = self.graph.add_entity(None, name, key)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Class(entity), Cv::default())?;
        let mut partial = self.graph.entity_mut(entity);
        partial.ty = Some(ty);
        partial.home = Some(home);
        partial.context_mut().template_head = Some(head);
        let index = self.partial_index_key(primary_head, &arguments);
        let store = &mut self.graph.class_templates.partials;
        let patterns = store.patterns.entry(primary).or_default();
        store.lists.add(primary, index, patterns.len());
        patterns.push((arguments.clone(), entity));
        store
            .identities
            .entry(primary)
            .or_default()
            .insert(arguments, entity);
        store.primaries.insert(entity, primary);
        Ok(entity)
    }

    pub(super) fn class_partial_definition(
        &mut self,
        syntax: ast::ClassId,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let node = *self.tree.classes.get(syntax);
        if self.specializing() {
            self.specialization_mode = super::analyzer::SpecializationMode::Ordinary;
            let access = self.begin_declaration_access(scope, true);
            let qualifier = self.prefix(name, scope);
            self.finish_declaration_access(access);
            let qualifier = qualifier?.filter(|&scope| {
                self.graph.scope(scope).owner.is_some_and(|owner| {
                    self.graph.class_templates.instance(owner).is_some()
                        || self.graph.class_templates.definitions.source(owner).is_some()
                })
            }).ok_or_else(|| Error::at(self.tree.names.get(name).location,
                "a partial specialization under template<> of no member template of a class template specialization"))?;
            if self.specialization_levels != self.specialized_enclosing(Some(qualifier)) {
                return Err(Error::at(
                    self.tree.names.get(name).location,
                    "a member template's partial specialization with the wrong number of template<>",
                ));
            }
        }
        let entity = self.partial_class(name, node.key, scope)?;
        if self.graph.entity(entity).defined {
            return Err(Error::at(
                self.tree.names.get(name).location,
                "class redefinition",
            ));
        }
        let home = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("partial without home"))?;
        let parent = if self.prefix(name, scope)?.is_some() {
            self.member_lookup_bridge(scope, home)?
        } else {
            scope
        };
        let binding = self.graph.add_hidden_binding_in(
            home,
            Binding {
                kind: BindingKind::Type,
                name: self.graph.entity(entity).name,
                entity,
                ty: self.graph.entity(entity).ty,
                key: node.key,
            },
        )?;
        self.graph
            .class_templates
            .partials
            .bindings
            .insert(entity, binding);
        let ty = self.class_body(node, entity, parent, Some(scope))?;
        self.record_class_pattern(entity, syntax, parent);
        Ok(ty)
    }

    pub(super) fn named_class_partial(
        &mut self,
        primary: EntityId,
        component: ast::NameComponent,
        scope: ScopeId,
    ) -> Result<Option<EntityId>, Error> {
        if !self
            .graph
            .class_templates
            .partials
            .patterns
            .contains_key(&primary)
        {
            return Ok(None);
        }
        let head = self
            .graph
            .entity(primary)
            .context()
            .template_head
            .ok_or(Error::Invariant("partial primary lost head"))?;
        self.template_depth += 1;
        let result = self
            .form_template_arguments(component, scope, Some(head))
            .and_then(|mut arguments| {
                self.fill_template_arguments_in(head, &mut arguments, component.location, None)?;
                Ok(arguments)
            });
        self.template_depth -= 1;
        let mut arguments = result?;
        let context = super::pack_arguments::MatchContext {
            mode: super::pack_arguments::MatchMode::Nonfatal,
            location: component.location,
        };
        if self
            .match_argument_list_in(head, &mut arguments, context)?
            .is_none()
        {
            return Ok(None);
        }
        Ok(self
            .graph
            .class_templates
            .partials
            .identities
            .get(&primary)
            .and_then(|known| known.get(&arguments))
            .copied())
    }

    pub(super) fn matched_class_partial(
        &mut self,
        primary: EntityId,
        arguments: &[TypeId],
    ) -> Result<(EntityId, Vec<TypeId>), Error> {
        self.matched_member_class_partial(primary, arguments, None)
    }

    pub(super) fn matched_member_class_partial(
        &mut self,
        primary: EntityId,
        arguments: &[TypeId],
        enclosing: Option<EntityId>,
    ) -> Result<(EntityId, Vec<TypeId>), Error> {
        let store = &self.graph.class_templates.partials;
        let Some(patterns) = store.patterns.get(&primary) else {
            return Ok((primary, arguments.to_vec()));
        };
        let keys = self.partial_candidate_keys(store.lists.positions(primary), arguments);
        let candidates = store
            .lists
            .candidates(primary, &keys)
            .into_iter()
            .map(|index| patterns[index].clone())
            .collect();
        self.matched_enclosing_partial(primary, arguments, candidates, SourceOffset(0), enclosing)
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
    fn selected_function_packs_keep_partition_identity_and_cached_demand() {
        let source = SourceBuffer::new(
            b"template<class>struct P;\
              template<class R,class... A>struct P<R(A...)>{static const int n=sizeof...(A);};\
              using Empty=P<void()>;using Two=P<int(char,long)>;\
              int e=sizeof(Empty);int t=sizeof(Two);"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"partial-packs").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"Empty".as_slice(), b"Two"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let mut selected = Vec::new();
        for (name, count) in names.into_iter().zip([0, 2]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            let TypeKind::Class(entity) = analyzer
                .graph
                .types
                .get(analyzer.graph.binding(binding).ty.unwrap())
                .kind
            else {
                panic!("alias lost class identity");
            };
            let (partial, arguments) = analyzer
                .graph
                .class_templates
                .partials
                .selected(entity)
                .unwrap();
            assert_eq!(arguments.len(), 2);
            assert_eq!(
                analyzer
                    .graph
                    .types
                    .pack_elements(arguments[1])
                    .unwrap()
                    .len(),
                count
            );
            assert!(analyzer.graph.entity(entity).complete);
            selected.push((entity, partial, arguments.to_vec()));
        }
        assert_eq!(selected[0].1, selected[1].1);
        assert_ne!(selected[0].2[1], selected[1].2[1]);
        let syntax = (
            analyzer.tree.expressions.mark(),
            analyzer.tree.declarations.mark(),
            analyzer.tree.bodies.mark(),
        );
        for _ in 0..8 {
            for (entity, partial, arguments) in &selected {
                assert!(analyzer.complete_class(*entity).unwrap());
                assert_eq!(
                    analyzer.graph.class_templates.partials.selected(*entity),
                    Some((*partial, arguments.as_slice()))
                );
            }
        }
        assert_eq!(
            syntax,
            (
                analyzer.tree.expressions.mark(),
                analyzer.tree.declarations.mark(),
                analyzer.tree.bodies.mark()
            )
        );
    }

    #[test]
    fn partial_patterns_keep_shell_identity_and_independent_definition_demand() {
        let source = SourceBuffer::new(
            b"template<class T>struct Box{typename T::missing bad;};\
              using I=Box<int*>;using L=Box<long*>;\
              template<class T>struct Box<T*>;\
              template<class U>struct Box<U*>{U value;U f(){return value;}};\
              using J=Box<int*>;int size=sizeof(J);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"partial-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"Box".as_slice(), b"I", b"J", b"L"]
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
        let entities: Vec<_> = names
            .iter()
            .map(|&name| {
                let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
                let TypeKind::Class(class) = analyzer
                    .graph
                    .types
                    .get(analyzer.graph.binding(binding).ty.unwrap())
                    .kind
                else {
                    panic!("class alias lost identity");
                };
                class
            })
            .collect();
        let [primary, first, reused, undemanded] = entities[..] else {
            panic!("missing aliases");
        };
        assert_eq!(first, reused);
        let partials = &analyzer.graph.class_templates.partials;
        let patterns = &partials.patterns[&primary];
        assert_eq!(patterns.len(), 1);
        let partial = patterns[0].1;
        assert_ne!(partial, primary);
        assert_eq!(partials.primary(partial), Some(primary));
        assert_eq!(
            analyzer
                .graph
                .class_templates
                .instance(first)
                .unwrap()
                .primary,
            primary
        );
        assert!(analyzer.graph.entity(first).complete);
        let (selected, arguments) = partials.selected(first).unwrap();
        assert_eq!(selected, partial);
        assert_eq!(arguments.len(), 1);
        assert!(matches!(
            analyzer.graph.types.get(arguments[0]).kind,
            TypeKind::Scalar(_)
        ));
        assert!(analyzer.graph.entity(undemanded).context().scope.is_none());
        assert!(
            !analyzer
                .graph
                .class_templates
                .definitions
                .started(undemanded)
        );
        assert_ne!(
            analyzer.graph.entity(first).context().scope,
            analyzer.graph.entity(partial).context().scope
        );
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        let syntax = (
            analyzer.tree.expressions.mark(),
            analyzer.tree.declarations.mark(),
        );
        for _ in 0..8 {
            assert!(analyzer.complete_class(first).unwrap());
        }
        assert_eq!(
            syntax,
            (
                analyzer.tree.expressions.mark(),
                analyzer.tree.declarations.mark()
            )
        );
    }
}
