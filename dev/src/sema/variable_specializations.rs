// Translated from sema/variable_templates.cpp::variable_specialization,
// matched_partial and instantiate_variable_specialization. See NOTICE.
//! Hidden canonical declarations, definition demand and storage remain separate.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    types::TypeId,
    variable_instances::Definition,
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
struct Key {
    primary: EntityId,
    arguments: Vec<TypeId>,
}
struct Instance {
    primary: EntityId,
    arguments: Vec<TypeId>,
    binding: BindingId,
    dependent: bool,
    explicit: bool,
    location: SourceOffset,
}
#[derive(Clone, Copy)]
struct Declaration {
    ty: TypeId,
    constexpr: bool,
}
#[derive(Default)]
pub(super) struct Specializations {
    keys: HashMap<Key, EntityId>,
    instances: HashMap<EntityId, Instance>,
}
impl Specializations {
    pub(super) fn contains(&self, entity: EntityId) -> bool {
        self.instances.contains_key(&entity)
    }
    pub(super) fn concrete(&self, entity: EntityId) -> bool {
        self.instances.get(&entity).is_some_and(|i| !i.dependent)
    }
    pub(super) fn explicit(&self, entity: EntityId) -> bool {
        self.instances.get(&entity).is_some_and(|i| i.explicit)
    }
    pub(super) fn arguments(&self, entity: EntityId) -> Option<&[TypeId]> {
        self.instances.get(&entity).map(|i| i.arguments.as_slice())
    }
    pub(super) fn location(&self, entity: EntityId) -> Option<SourceOffset> {
        self.instances.get(&entity).map(|i| i.location)
    }
}
impl Analyzer<'_> {
    pub(super) fn variable_template_binding(
        &mut self,
        found: BindingId,
        component: ast::NameComponent,
        scope: ScopeId,
    ) -> Result<Option<BindingId>, Error> {
        let Some(primary) = self.variable_template_primary(found) else {
            return Ok(None);
        };
        let head = self
            .graph
            .entity(primary)
            .template_head
            .ok_or(Error::Invariant("variable template without head"))?;
        self.supported_fixed_template_head(head)?;
        let mut arguments = self.form_template_arguments(component, scope, Some(head))?;
        self.complete_template_arguments(head, &mut arguments, component.location)?;
        let binding = self.variable_specialization(primary, arguments, component.location, None)?;
        if self.graph.binding_scope(found) == self.graph.binding_scope(binding) {
            self.graph.inherit_binding_access(binding, found);
        }
        Ok(Some(binding))
    }
    fn variable_specialization(
        &mut self,
        primary: EntityId,
        arguments: Vec<TypeId>,
        location: SourceOffset,
        declared: Option<Declaration>,
    ) -> Result<BindingId, Error> {
        self.check_stack()?;
        let key = Key { primary, arguments };
        if let Some(&entity) = self.graph.variable_instances.templates.keys.get(&key) {
            if declared.is_none()
                && self
                    .graph
                    .entity(entity)
                    .ty
                    .is_some_and(|ty| self.holds_auto(ty))
            {
                return Err(Error::at(
                    location,
                    "an auto variable template's specialization named in its own initializer",
                ));
            }
            return Ok(self.graph.variable_instances.templates.instances[&entity].binding);
        }
        let dependent = key
            .arguments
            .iter()
            .any(|&ty| self.graph.dependent_type(ty));
        let (source, deduced) = if !dependent && declared.is_none() {
            self.matched_variable_partial(primary, &key.arguments, location)?
        } else {
            (primary, key.arguments.clone())
        };
        let head = self
            .graph
            .entity(source)
            .template_head
            .ok_or(Error::Invariant("variable pattern without head"))?;
        let given: Vec<_> = deduced.into_iter().map(Some).collect();
        let mut environment = Substitution::new(self.graph.templates.depth(head), &given);
        let pattern_ty = self
            .graph
            .entity(source)
            .ty
            .ok_or(Error::Invariant("variable template without type"))?;
        let pattern_ty = self
            .graph
            .variable_instances
            .pattern(source)
            .and_then(|p| p.auto_type)
            .unwrap_or(pattern_ty);
        let ty = if let Some(declaration) = declared {
            declaration.ty
        } else {
            self.substitute_signature_type(pattern_ty, &mut environment)?
                .ok_or(Error::at(
                    location,
                    "a substituted variable template type is invalid",
                ))?
        };
        let template = self.graph.entity(primary);
        let (name, home, variable, data_member, key_word) = (
            template.name,
            template.home,
            template.variable,
            template.data_member,
            template.key,
        );
        // A shell keeps its first declaration's type and constexpr fact, even
        // when a subsequent explicit declaration supplies the definition.
        let constexpr = declared.map_or(self.graph.entity(source).constexpr, |d| d.constexpr);
        let entity = self.graph.add_entity(Some(ty), name, key_word)?;
        let node = self.graph.entity_mut(entity);
        node.home = home;
        node.variable = variable;
        node.data_member = data_member;
        node.constexpr = constexpr;
        let binding = self.graph.add_hidden_binding_in(
            home.ok_or(Error::Invariant("variable template without owner"))?,
            Binding {
                kind: BindingKind::Variable,
                name,
                entity,
                ty: Some(ty),
                key: key_word,
            },
        )?;
        self.graph
            .instantiation_levels
            .record(entity, self.instantiation_depth);
        self.graph.set_binding_dependence(binding, dependent);
        self.graph.variable_instances.templates.instances.insert(
            entity,
            Instance {
                primary,
                arguments: key.arguments.clone(),
                binding,
                dependent,
                explicit: false,
                location,
            },
        );
        self.graph
            .variable_instances
            .templates
            .keys
            .insert(key, entity);
        if !dependent && declared.is_none() && self.holds_auto(ty) {
            self.instantiate_variable_specialization(entity)?;
            if self
                .graph
                .entity(entity)
                .ty
                .is_none_or(|ty| self.holds_auto(ty))
            {
                return Err(Error::at(
                    location,
                    "an auto variable template's specialization without an initializer to deduce from",
                ));
            }
        }
        Ok(binding)
    }
    fn matched_variable_partial(
        &mut self,
        primary: EntityId,
        arguments: &[TypeId],
        location: SourceOffset,
    ) -> Result<(EntityId, Vec<TypeId>), Error> {
        let candidates = self.variable_partial_candidates(primary, arguments);
        self.matched_partial(primary, arguments, candidates, location)
    }
    pub(super) fn instantiate_variable_specialization(
        &mut self,
        entity: EntityId,
    ) -> Result<(), Error> {
        let instance = &self.graph.variable_instances.templates.instances[&entity];
        if instance.dependent || instance.explicit {
            return Ok(());
        }
        let (primary, arguments, binding, location) = (
            instance.primary,
            instance.arguments.clone(),
            instance.binding,
            instance.location,
        );
        let (mut source, deduced) = self.matched_variable_partial(primary, &arguments, location)?;
        let mut pattern = self.graph.variable_instances.pattern(source);
        // Member links always point to the earlier declaration pattern. A
        // class's copied member template can acquire its outside definition
        // through that chain without revisiting any unrelated declaration.
        if source == primary {
            while pattern.is_none_or(|p| p.initializer.is_none())
                && !self.graph.entity(source).defined
            {
                let Some(next) = self.graph.variable_instances.member_source(source) else {
                    break;
                };
                source = next;
                pattern = self.graph.variable_instances.pattern(source);
            }
        }
        let Some(pattern) = pattern else {
            return Ok(());
        };
        if !self.graph.entity(source).defined && pattern.initializer.is_none() {
            return Ok(());
        }
        let head = pattern
            .head
            .ok_or(Error::Invariant("variable definition without head"))?;
        let owner = self
            .graph
            .entity(primary)
            .home
            .ok_or(Error::Invariant("variable template without owner"))?;
        let frame = if source != primary || pattern.qualified {
            self.outside_template_frame(owner, head, &deduced, pattern.scope)?
        } else {
            let parameters = self.graph.templates.parameter_scope(head);
            let parent = self
                .graph
                .scope(parameters)
                .parent
                .ok_or(Error::Invariant("variable head without parent"))?;
            let frame = self
                .graph
                .add_scope(ScopeKind::TemplateParameters, parent, None, None)?;
            let depth = self.graph.templates.depth(head);
            for index in 0..self.graph.scope(parameters).bindings.len() {
                let old = self
                    .graph
                    .binding(self.graph.scope(parameters).bindings[index]);
                if let Some(index) = self
                    .graph
                    .entity(old.entity)
                    .template_parameter
                    .and_then(|p| p.index(depth))
                {
                    self.bind_template_argument(frame, old, deduced[index])?;
                }
            }
            frame
        };
        self.graph.variable_instances.begin(entity);
        let result = self.required_instantiation(
            self.graph.instantiation_levels.get(entity)?,
            pattern.location,
            pattern.limit,
            |analyzer| {
                let initializer = pattern
                    .initializer
                    .map(|init| analyzer.copy_initializer_in_frame(init, frame))
                    .transpose()?;
                analyzer.initialize_variable_specialization(
                    binding,
                    initializer,
                    frame,
                    pattern.location,
                    pattern.auto_cv,
                )?;
                analyzer.graph.entity_mut(entity).variable_definition = Some(pattern.declarator);
                Ok(Definition { initializer })
            },
        );
        self.graph.variable_instances.finish(entity, result)
    }
    pub(super) fn initialize_variable_specialization(
        &mut self,
        binding: BindingId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        location: SourceOffset,
        auto_cv: super::types::Cv,
    ) -> Result<(), Error> {
        self.deduce_variable_specialization_type(binding, initializer, scope, location, auto_cv)?;
        let entity = self.graph.binding(binding).entity;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("variable instance without type"))?;
        let ty = self.complete_initializer_array(ty, initializer)?;
        self.graph.complete_variable_type(binding, ty)?;
        self.graph.entity_mut(entity).defined = true;
        if let Some(init) = initializer {
            self.initialize_variable(binding, init, scope, location)?;
            self.record_constant_initializer(entity, init, scope)?;
        } else {
            self.default_initialize_declaration(binding, location)?;
        }
        self.variable_destruction(binding, scope, location, initializer.is_some())?;
        if self.lowering() {
            let home = self
                .graph
                .entity(entity)
                .home
                .ok_or(Error::Invariant("variable instance without home"))?;
            self.request_static_initializer(entity, initializer, scope, home)?;
            self.request_storage_layout(entity, location)?;
        }
        Ok(())
    }
    // Auto deduction belongs to the declaration, including in types mode;
    // initializer checking and storage demand are separate semantic facts.
    fn deduce_variable_specialization_type(
        &mut self,
        binding: BindingId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        location: SourceOffset,
        auto_cv: super::types::Cv,
    ) -> Result<(), Error> {
        let entity = self.graph.binding(binding).entity;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("variable instance without type"))?;
        if self.holds_auto(ty) {
            let mut known = None;
            let ty =
                self.deduce_object_type(ty, auto_cv, initializer, scope, location, &mut known)?;
            let ty = if self.graph.entity(entity).constexpr {
                self.graph.types.qualify(ty, super::types::Cv::CONST)?
            } else {
                ty
            };
            self.graph.complete_variable_type(binding, ty)?;
        }
        Ok(())
    }
    pub(super) fn explicit_variable_template(
        &mut self,
        specs: super::specifiers::Specifiers,
        init: ast::InitDeclarator,
        scope: ScopeId,
        info: &super::declarators::DeclaratorInfo,
        last: ast::NameComponent,
    ) -> Result<(), Error> {
        let name = info
            .syntax_name
            .ok_or(Error::Invariant("variable specialization without name"))?;
        let location = self.tree.names.get(name).location;
        self.specialization_mode = super::analyzer::SpecializationMode::Ordinary;
        let found = self
            .lookup_result_in(name, scope, super::lookup::Filter::Ordinary, false)?
            .binding
            .ok_or(Error::at(location, "undeclared name"))?;
        let primary = self.variable_template_primary(found).ok_or(Error::at(
            location,
            "a template-id declarator of no variable template",
        ))?;
        let head = self
            .graph
            .entity(primary)
            .template_head
            .ok_or(Error::Invariant("variable template without head"))?;
        let mut arguments = self.form_template_arguments(last, scope, Some(head))?;
        self.complete_template_arguments(head, &mut arguments, location)?;
        let ty = if specs.constexpr {
            self.graph.types.qualify(info.ty, super::types::Cv::CONST)?
        } else {
            info.ty
        };
        let binding = self.variable_specialization(
            primary,
            arguments,
            location,
            Some(Declaration {
                ty,
                constexpr: specs.constexpr,
            }),
        )?;
        let entity = self.graph.binding(binding).entity;
        self.deduce_variable_specialization_type(
            binding,
            init.initializer,
            scope,
            location,
            specs.auto_cv,
        )?;
        let member = self
            .graph
            .entity(primary)
            .home
            .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class);
        let declared = self.graph.variable_instances.started(entity)
            && !self.graph.entity(entity).defined
            && member;
        if self.graph.variable_instances.started(entity) && !declared {
            return Err(Error::at(
                location,
                "an explicit specialization after the specialization's instantiation or definition",
            ));
        }
        super::record(
            &mut self.graph.declared_bindings,
            init.declarator.index(),
            binding,
        );
        if declared && init.initializer.is_none() {
            return Ok(());
        }
        self.graph
            .variable_instances
            .templates
            .instances
            .get_mut(&entity)
            .ok_or(Error::Invariant("lost variable specialization"))?
            .explicit = true;
        self.graph.variable_instances.begin(entity);
        // The frozen explicit-specialization path initializes only a supplied
        // initializer. An omitted initializer still defines a namespace object,
        // but does not demand default construction or destruction checks.
        if self.semantic() && init.initializer.is_some() {
            self.initialize_variable_specialization(
                binding,
                init.initializer,
                scope,
                location,
                specs.auto_cv,
            )?;
        }
        self.graph.entity_mut(entity).defined = init.initializer.is_some() || !member;
        self.graph.entity_mut(entity).variable_definition =
            self.graph.entity(entity).defined.then_some(init.declarator);
        self.graph.variable_instances.finish(
            entity,
            Ok(Definition {
                initializer: init.initializer,
            }),
        )
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
    fn fixed_variable_types_do_not_demand_initializers_in_semantics_mode() {
        let source = SourceBuffer::new(
            b"template<class T>int make(){return T::missing;}\
              template<class T>int v=make<T>();int main(){return v<int>;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"variable-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let graph = &analyzer.graph;
        assert_eq!(graph.variable_instances.templates.instances.len(), 1);
        let (&entity, _) = graph
            .variable_instances
            .templates
            .instances
            .iter()
            .next()
            .unwrap();
        assert!(!graph.variable_instances.started(entity));
        assert!(!graph.entity(entity).defined);
        assert!(graph.root_demands.as_slice().is_empty());
        assert!(graph.variable_instances.demands().is_empty());
    }

    #[test]
    fn explicit_auto_type_is_available_without_semantic_initializer_demand() {
        let source = SourceBuffer::new(
            b"template<class T>int v=2;template<>auto v<int> =3L;\
              using X=decltype(v<int>);using Y=decltype(v<int>);"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"explicit-auto").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let graph = &analyzer.graph;
        assert_eq!(graph.variable_instances.templates.instances.len(), 1);
        let (&entity, _) = graph
            .variable_instances
            .templates
            .instances
            .iter()
            .next()
            .unwrap();
        let ty = graph.entity(entity).ty.unwrap();
        assert_eq!(
            graph.types.get(ty).kind,
            super::super::types::TypeKind::Scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::LongInt,
            ))
        );
        for name in [b"X", b"Y"] {
            let name = cursor.intern_name(name).unwrap();
            let alias = graph
                .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                .unwrap();
            assert_eq!(graph.binding(alias).ty, Some(ty));
        }
        assert!(graph.variable_instances.demands().is_empty());
        assert!(graph.entity(entity).static_initializer.is_none());
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn shells_defaults_and_definition_facts_have_independent_identity() {
        let source = SourceBuffer::new(
            b"template<class T=int>int v=sizeof(T);\
            using A=decltype(v<>);using B=decltype(v<int>);\
            int main(){return v<>+v<char>+v<int>;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"variable-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.set_lowering();
        let ids = analyzer.tree.decl_lists.get(declarations).to_vec();
        let global = analyzer.graph.global;
        for &id in &ids[..3] {
            analyzer.declaration(id, global).unwrap();
        }
        let store = &analyzer.graph.variable_instances;
        assert_eq!(store.templates.instances.len(), 1);
        let (&entity, instance) = store.templates.instances.iter().next().unwrap();
        let pattern = store.pattern(instance.primary).unwrap();
        assert!(!store.started(entity));
        assert!(!analyzer.graph.entity(entity).defined);
        let original = pattern.initializer.unwrap();
        let ast::InitializerKind::Assign(original) = original.kind else {
            panic!("initializer kind");
        };
        assert!(analyzer.graph.expression_facts.get(original).is_err());
        analyzer.declaration(ids[3], global).unwrap();
        analyzer.finish_instance_variables().unwrap();
        let store = &analyzer.graph.variable_instances;
        assert_eq!(store.templates.instances.len(), 2);
        assert!(store.demands().is_empty());
        let mut copies = Vec::new();
        for &entity in store.templates.instances.keys() {
            let definition = store.initializer(entity).unwrap();
            let ast::InitializerKind::Assign(copy) = definition.initializer.unwrap().kind else {
                panic!("initializer kind");
            };
            assert_ne!(copy, original);
            assert!(analyzer.graph.expression_facts.get(copy).is_ok());
            assert!(analyzer.graph.entity(entity).defined);
            copies.push(copy);
        }
        assert_ne!(copies[0], copies[1]);
        assert!(analyzer.graph.expression_facts.get(original).is_err());
    }

    #[test]
    fn outside_member_templates_bind_enclosing_and_own_definition_heads_once() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{template<class U>static int v;\
            int unused(){typename T::missing x;return 0;}};\
            template<class X>template<class Y>int A<X>::v=sizeof(X)+sizeof(Y);\
            int main(){return A<int>::v<char>+A<char>::v<char>+A<int>::v<char>;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"outside-variable").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let store = &graph.variable_instances;
        assert_eq!(store.templates.instances.len(), 2);
        let mut primaries = Vec::new();
        let mut values = Vec::new();
        let mut copies = Vec::new();
        for (&entity, instance) in &store.templates.instances {
            primaries.push(instance.primary);
            assert_eq!(instance.arguments.len(), 1);
            let Some(super::super::static_initialization::Value::Integer(value)) =
                graph.entity(entity).static_initializer
            else {
                panic!("initializer was not evaluated");
            };
            values.push(value);
            let ast::InitializerKind::Assign(copy) =
                store.initializer(entity).unwrap().initializer.unwrap().kind
            else {
                panic!("initializer kind");
            };
            copies.push(copy);
            assert!(graph.expression_facts.get(copy).is_ok());
        }
        values.sort_unstable();
        assert_eq!(values, [2, 5]);
        assert_ne!(primaries[0], primaries[1]);
        assert_ne!(copies[0], copies[1]);
    }

    #[test]
    fn indexed_partials_visit_only_selected_lists_and_memoize_orderings() {
        let source = SourceBuffer::new(
            b"template<class A,class B>int v=0;\
            template<class T>int v<int,T> =1;\
            template<class T>int v<char,T> =2;\
            template<class T,class U>int v<T*,U> =3;\
            template<class T>int v<T*,int> =4;\
            int main(){return v<int,int>+v<char,char>+v<int*,int>;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"partial-index").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"v").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.set_lowering();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let binding = analyzer
            .graph
            .own(
                analyzer.graph.global,
                name,
                super::super::lookup::Filter::Ordinary,
            )
            .unwrap();
        let primary = analyzer.graph.binding(binding).entity;
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        let char = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Char,
            ))
            .unwrap();
        assert_eq!(
            analyzer
                .variable_partial_candidates(primary, &[char, char])
                .len(),
            2
        );
        assert_eq!(
            analyzer
                .variable_partial_candidates(primary, &[int, int])
                .len(),
            3
        );
        let pointer = analyzer.graph.types.pointer(int).unwrap();
        let (chosen, _) = analyzer
            .matched_variable_partial(primary, &[pointer, int], SourceOffset(0))
            .unwrap();
        assert_eq!(
            chosen,
            analyzer.graph.variable_instances.partials(primary)[3].1
        );
        let expressions = analyzer.tree.expressions.mark();
        let entities = analyzer.graph.binding_limit();
        let repeated = analyzer
            .matched_variable_partial(primary, &[pointer, int], SourceOffset(0))
            .unwrap();
        assert_eq!(repeated.0, chosen);
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.graph.binding_limit(), entities);
    }
}
