// Translated from class_templates.cpp::link_member/instantiate_variable and
// analyzer_declarations.cpp::CompatibleVariableTypes/declare_variable. See NOTICE.
//! Static member patterns, initializer facts and evaluated storage demands have
//! independent unit-owned identities. Missing definitions can arrive later.
use super::{
    Error,
    analyzer::Analyzer,
    constants::ConstantState,
    declarators::DeclaratorInfo,
    graph::{BindingId, EntityId, ScopeId},
    templates::HeadId,
    types::{TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::{ast, cloning::Cloner},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct Pattern {
    declarator: ast::DeclaratorId,
    initializer: Option<ast::Initializer>,
    ty: TypeId,
    head: Option<HeadId>,
    scope: ScopeId,
    location: SourceOffset,
    limit: usize,
}
#[derive(Clone, Copy)]
pub(crate) struct Definition {
    pub initializer: Option<ast::Initializer>,
}
#[derive(Clone, Copy)]
enum State {
    InProgress,
    Complete(Definition),
}
#[derive(Default)]
pub(super) struct Variables {
    patterns: HashMap<EntityId, Pattern>,
    // Declaration order is independent of any specialization's later demand.
    partials: HashMap<EntityId, Vec<(Vec<TypeId>, EntityId)>>,
    partial_declarations: Vec<Option<EntityId>>,
    deferred_declarations: HashSet<ast::DeclaratorId>,
    links: HashMap<EntityId, EntityId>,
    states: HashMap<EntityId, State>,
    demands: Vec<EntityId>,
    seen: HashSet<EntityId>,
    next: usize,
}
impl Variables {
    pub(super) fn defer_declaration(&mut self, declarator: ast::DeclaratorId) {
        self.deferred_declarations.insert(declarator);
    }
    pub(super) fn nonobject_declaration(&self, declarator: ast::DeclaratorId) -> bool {
        self.partial_declaration(declarator).is_some()
            || self.deferred_declarations.contains(&declarator)
    }
    pub(super) fn record_partial(
        &mut self,
        primary: EntityId,
        arguments: Vec<TypeId>,
        partial: EntityId,
        declarator: ast::DeclaratorId,
    ) {
        self.partials
            .entry(primary)
            .or_default()
            .push((arguments, partial));
        super::record(&mut self.partial_declarations, declarator.index(), partial);
    }
    pub(super) fn partial_declaration(&self, declarator: ast::DeclaratorId) -> Option<EntityId> {
        self.partial_declarations
            .get(declarator.index())
            .copied()
            .flatten()
    }
    pub(super) fn link(&mut self, source: EntityId, target: EntityId) {
        self.links.insert(target, source);
    }
    pub(super) fn instance(&self, entity: EntityId) -> bool {
        self.links.contains_key(&entity)
    }
    pub(super) fn demands(&self) -> &[EntityId] {
        &self.demands
    }
    pub(super) fn initializer(&self, entity: EntityId) -> Option<Definition> {
        match self.states.get(&entity) {
            Some(State::Complete(initializer)) => Some(*initializer),
            _ => None,
        }
    }
}
impl Analyzer<'_> {
    pub(super) fn retain_variable_template_pattern(
        &mut self,
        entity: EntityId,
        init: ast::InitDeclarator,
        info: &DeclaratorInfo,
        scope: ScopeId,
        location: SourceOffset,
    ) {
        // A later declaration without an initializer keeps the definition's
        // parameter names, lookup scope and definition point.
        if init.initializer.is_none()
            && self.graph.variable_instances.patterns.contains_key(&entity)
        {
            return;
        }
        self.graph.variable_instances.patterns.insert(
            entity,
            Pattern {
                declarator: init.declarator,
                initializer: init.initializer,
                ty: info.ty,
                head: self.active_template_head(scope),
                scope: info.lookup,
                location,
                limit: self.graph.binding_limit(),
            },
        );
    }
    pub(super) fn explicit_class_variable(&mut self, entity: EntityId) -> Result<(), Error> {
        let variables = &self.graph.variable_instances;
        if !variables
            .links
            .get(&entity)
            .is_some_and(|source| variables.patterns.contains_key(source))
        {
            return Ok(());
        }
        self.instantiate_variable(entity)?;
        if self.graph.variable_instances.seen.insert(entity) {
            self.graph.variable_instances.demands.push(entity);
        }
        Ok(())
    }
    pub(super) fn compatible_variable_type(
        &mut self,
        old: TypeId,
        new: TypeId,
    ) -> Result<Option<TypeId>, Error> {
        let canonical_old = self.graph.types.canonical_function(old)?;
        let canonical_new = self.graph.types.canonical_function(new)?;
        let compatible = canonical_old == canonical_new
            || matches!((&self.graph.types.get(canonical_old).kind, &self.graph.types.get(canonical_new).kind),
                (TypeKind::Array { element: old_element, bound: old_bound, .. },
                 TypeKind::Array { element: new_element, bound: new_bound, .. })
                    if old_element == new_element && (*old_bound == 0 || *new_bound == 0));
        Ok(compatible.then_some(
            if matches!(
                self.graph.types.get(old).kind,
                TypeKind::Array { bound: 0, .. }
            ) {
                new
            } else {
                old
            },
        ))
    }
    pub(super) fn record_variable_pattern(
        &mut self,
        binding: BindingId,
        init: ast::InitDeclarator,
        info: &DeclaratorInfo,
        definition: bool,
        ty: TypeId,
        location: SourceOffset,
    ) {
        let entity = self.graph.binding(binding).entity;
        if self.template_depth == 0
            || !definition
            || info.home.is_none()
            || self.graph.entity(entity).data_member != Some(super::graph::DataMemberKind::Static)
        {
            return;
        }
        self.graph.variable_instances.patterns.insert(
            entity,
            Pattern {
                declarator: init.declarator,
                initializer: init.initializer,
                ty,
                head: self.template_head,
                scope: info.lookup,
                location,
                limit: self.graph.binding_limit(),
            },
        );
    }
    pub(super) fn use_instance_variable(&mut self, entity: EntityId) -> Result<(), Error> {
        if self.template_depth != 0 || !self.graph.variable_instances.instance(entity) {
            return Ok(());
        }
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("static member without type"))?;
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Array {
                bound: 0,
                operand: None,
                ..
            }
        ) || matches!(self.graph.entity(entity).constant, ConstantState::None)
            && self.integral_kind(ty).is_some()
            && self.graph.lower_cv(ty) & 1 != 0
        {
            self.instantiate_variable(entity)?;
        }
        if self.unevaluated == 0 && self.graph.variable_instances.seen.insert(entity) {
            self.graph.variable_instances.demands.push(entity);
        }
        Ok(())
    }
    pub(super) fn finish_instance_variables(&mut self) -> Result<(), Error> {
        while let Some(&entity) = self
            .graph
            .variable_instances
            .demands
            .get(self.graph.variable_instances.next)
        {
            self.graph.variable_instances.next += 1;
            self.instantiate_variable(entity)?;
        }
        Ok(())
    }
    fn instance_array_type(
        &mut self,
        ty: TypeId,
        declared: TypeId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let TypeKind::Array {
            element,
            bound: 0,
            operand: None,
        } = self.graph.types.get(ty).kind
        else {
            return Ok(ty);
        };
        let TypeKind::Array { bound, operand, .. } = self.graph.types.get(declared).kind else {
            return Ok(ty);
        };
        let bound = if let Some(operand) = operand {
            let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(operand).kind
            else {
                return Err(Error::Invariant("bound operand without entity"));
            };
            let pattern = self
                .graph
                .template_operands
                .pattern(entity)
                .ok_or(Error::Invariant("array bound without syntax"))?;
            let expression = Cloner::new(self.tree, self.stack_budget, self.stack_base)
                .expression(pattern.expression)
                .map_err(|error| super::template_bodies::copy_error(&error))?;
            self.array_bound(expression, scope)?
        } else {
            bound
        };
        if bound == 0 {
            return Ok(ty);
        }
        self.graph.types.intern(
            TypeKind::Array {
                element,
                bound,
                operand: None,
            },
            super::types::Cv::default(),
        )
    }
    pub(super) fn explicit_variable_instantiation(
        &mut self,
        info: &DeclaratorInfo,
        specs: super::specifiers::Specifiers,
        is_extern: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let name = info
            .syntax_name
            .ok_or(Error::Invariant("explicit variable without name"))?;
        let binding = self.lookup(name, scope, super::lookup::Filter::Ordinary)?;
        let binding = self.graph.binding(binding);
        let entity = binding.entity;
        if binding.kind != super::graph::BindingKind::Variable
            || !self.graph.variable_instances.instance(entity)
        {
            return Err(Error::at(
                location,
                "an explicit instantiation names no static data member or variable template specialization",
            ));
        }
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("instance variable without type"))?;
        if self.graph.types.canonical_function(ty)?
            != self.graph.types.canonical_function(info.ty)?
        {
            return Err(Error::at(
                location,
                "an explicit instantiation's type differs from the variable's",
            ));
        }
        if specs.inline == super::specifiers::InlinePreference::Inline
            || specs.constexpr
            || specs.is_static
            || specs.is_extern
            || specs.thread_storage != super::specifiers::ThreadStorage::Ordinary
        {
            return Err(Error::at(
                location,
                "a specifier an explicit instantiation may not have",
            ));
        }
        let home = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("instance variable without owner"))?;
        if !self.graph.encloses(
            self.declaration_scope(scope),
            self.enclosing_namespace(home),
        ) {
            return Err(Error::at(
                location,
                "an explicit instantiation outside its template's namespace",
            ));
        }
        if is_extern {
            return Ok(());
        }
        self.instantiate_variable(entity)?;
        if self.graph.variable_instances.initializer(entity).is_none() {
            return Err(Error::at(
                location,
                "an explicit instantiation of a variable whose template's definition is not available",
            ));
        }
        if self.graph.variable_instances.seen.insert(entity) {
            self.graph.variable_instances.demands.push(entity);
        }
        Ok(())
    }
    fn instantiate_variable(&mut self, entity: EntityId) -> Result<(), Error> {
        if self.graph.variable_instances.states.contains_key(&entity) {
            return Ok(());
        }
        let Some(source) = self.graph.variable_instances.links.get(&entity).copied() else {
            return Ok(());
        };
        let Some(pattern) = self.graph.variable_instances.patterns.get(&source).copied() else {
            return Ok(());
        };
        let owner = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("instance variable without owner"))?;
        let frame = self.outside_class_frame(owner, pattern.head, pattern.scope)?;
        self.graph
            .variable_instances
            .states
            .insert(entity, State::InProgress);
        let result = self.required_instantiation(
            self.graph.instantiation_levels.get(entity)?,
            pattern.location,
            pattern.limit,
            |analyzer| {
                let initializer = pattern
                    .initializer
                    .map(|init| {
                        Cloner::new(analyzer.tree, analyzer.stack_budget, analyzer.stack_base)
                            .initializer(init)
                            .map_err(|e| super::template_bodies::copy_error(&e))
                    })
                    .transpose()?;
                let binding = analyzer
                    .graph
                    .own(
                        owner,
                        analyzer
                            .graph
                            .entity(entity)
                            .name
                            .ok_or(Error::Invariant("unnamed static member"))?,
                        super::lookup::Filter::Ordinary,
                    )
                    .ok_or(Error::Invariant("static member without binding"))?;
                analyzer.graph.entity_mut(entity).defined = true;
                analyzer.graph.entity_mut(entity).variable_definition = Some(pattern.declarator);
                let mut ty = analyzer
                    .graph
                    .entity(entity)
                    .ty
                    .ok_or(Error::Invariant("static member without type"))?;
                ty = analyzer.instance_array_type(ty, pattern.ty, frame)?;
                ty = analyzer.complete_initializer_array(ty, initializer)?;
                analyzer.graph.complete_variable_type(binding, ty)?;
                let effective = initializer.or(analyzer.graph.entity(entity).member_initializer);
                if let Some(initializer) = initializer {
                    analyzer.initialize_variable(binding, initializer, frame, pattern.location)?;
                    analyzer.record_constant_initializer(entity, initializer, frame)?;
                } else if effective.is_none()
                    && !matches!(
                        analyzer.graph.entity(entity).constant,
                        ConstantState::Value(_)
                    )
                    && !analyzer.graph.entity(entity).constexpr
                {
                    analyzer.default_initialize_declaration(binding, pattern.location)?;
                }
                analyzer.variable_destruction(
                    binding,
                    frame,
                    pattern.location,
                    effective.is_some(),
                )?;
                if analyzer.lowering() {
                    analyzer.request_static_initializer(entity, effective, frame, owner)?;
                    analyzer.request_storage_layout(entity, pattern.location)?;
                }
                Ok(effective)
            },
        );
        match result {
            Ok(initializer) => {
                self.graph
                    .variable_instances
                    .states
                    .insert(entity, State::Complete(Definition { initializer }));
            }
            Err(error) => {
                self.graph.variable_instances.states.remove(&entity);
                return Err(error);
            }
        }
        Ok(())
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
    fn variable_template_partials_preserve_primary_and_definition_ownership() {
        let source = SourceBuffer::new(
            b"template<class T,class U,class=void>extern const bool v;\
            template<class A,class B,class C>const bool v=false;\
            template<class X,class Y,class Z>extern const bool v;\
            template<class P>const bool v<P,P> =true;\
            template<class Q>int v<Q*,Q*> =sizeof(typename Q::missing);\
            template<class T>void unused(){int ignored<T>;}\
            template<class T>struct A{int ignored<T>;};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"variable-patterns").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"v").unwrap();
        let initializer_count = tree.expressions.mark();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let binding = graph
            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let primary = graph.binding(binding).entity;
        let variables = &graph.variable_instances;
        let partials = &variables.partials[&primary];
        assert_eq!(partials.len(), 2);
        assert_ne!(partials[0].1, partials[1].1);
        assert_ne!(partials[0].1, primary);
        assert_eq!(variables.patterns.len(), 3);
        assert!(variables.demands.is_empty());
        assert!(variables.links.is_empty());
        assert!(variables.states.is_empty());
        assert_eq!(variables.deferred_declarations.len(), 2);
        assert!(graph.entity(primary).defined);
        let pattern = variables.patterns[&primary];
        assert_ne!(pattern.head, graph.entity(primary).template_head);
        assert_ne!(pattern.head, graph.entity(partials[0].1).template_head);
        // Heads of identical shape have canonical parameter positions, but
        // each definition retains its actual scope and initializer syntax.
        assert_ne!(pattern.scope, variables.patterns[&partials[0].1].scope);
        for &entity in &[primary, partials[0].1, partials[1].1] {
            let pattern = variables.patterns[&entity];
            let ast::InitializerKind::Assign(expression) = pattern.initializer.unwrap().kind else {
                panic!("pattern lost initializer");
            };
            assert!(graph.expression_facts.get(expression).is_err());
        }
        assert_eq!(tree.expressions.mark(), initializer_count);
        assert!(partials.iter().all(|(arguments, _)| arguments.len() == 3));
    }

    #[test]
    fn invalid_partial_does_not_replace_or_register_the_primary() {
        for (tail, message) in [
            (
                "template<class U>int v<U>;",
                "a partial specialization whose arguments are its template's own parameters",
            ),
            (
                "template<class U=int>int v<U*>;",
                "a default template argument in a partial specialization's template parameter list",
            ),
            (
                "template<class U>int v<typename U::missing>;",
                "a partial specialization's template parameter that its arguments do not deduce",
            ),
        ] {
            let source =
                SourceBuffer::new(format!("template<class T>extern int v;{tail}").into_bytes())
                    .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"partial-publication").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let name = cursor.intern_name(b"v").unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let result = analyzer.declarations(declarations, analyzer.graph.global);
            assert!(
                matches!(result, Err(Error::Diagnostic{message: actual,..}) if actual==message)
            );
            assert_eq!(analyzer.template_depth, 0);
            assert!(analyzer.template_head.is_none());
            let binding = analyzer
                .graph
                .own(
                    analyzer.graph.global,
                    name,
                    super::super::lookup::Filter::Ordinary,
                )
                .unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            assert!(analyzer.graph.entity(entity).template_head.is_some());
            assert!(!analyzer.graph.entity(entity).defined);
            let variables = &analyzer.graph.variable_instances;
            assert!(variables.partials.is_empty());
            assert_eq!(variables.patterns.len(), 1);
            assert!(variables.demands.is_empty());
        }
    }

    #[test]
    fn declarations_initializers_and_storage_are_independent_memoized_facts() {
        let source = SourceBuffer::new(b"template<class Original>struct Box{static Original value;static int bad;};\
            template<class Renamed>Renamed Box<Renamed>::value=sizeof(Renamed);\
            template<class T>int Box<T>::bad=T::missing;\
            int main(){return sizeof(Box<long>::bad)+Box<int>::value+Box<char>::value+Box<int>::value;}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"static-facts").unwrap());
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
        let variables = &graph.variable_instances;
        assert_eq!(variables.patterns.len(), 2);
        assert_eq!(variables.links.len(), 6);
        assert_eq!(variables.demands.len(), 2);
        assert_eq!(variables.states.len(), 2);
        let mut copies = Vec::new();
        for &entity in &variables.demands {
            let definition = variables.initializer(entity).unwrap();
            let initializer = definition.initializer.unwrap();
            let original = variables
                .patterns
                .get(&variables.links[&entity])
                .unwrap()
                .initializer
                .unwrap();
            let ast::InitializerKind::Assign(copy) = initializer.kind else {
                panic!("copy lost initializer");
            };
            let ast::InitializerKind::Assign(pattern) = original.kind else {
                panic!("pattern lost initializer");
            };
            assert_ne!(copy, pattern);
            assert!(graph.expression_facts.get(copy).is_ok());
            assert!(graph.expression_facts.get(pattern).is_err());
            copies.push(copy);
        }
        assert_ne!(copies[0], copies[1]);
    }
}
