// Translated from class_templates.cpp::instantiate_default_argument and
// analyzer_calls.cpp::call. See NOTICE.
//! Selected-call default demand, separate from signature, body and emission.
use super::{
    Error,
    analyzer::Analyzer,
    default_arguments::DefaultArgument,
    graph::{BindingId, EntityId, ScopeId},
    templates::HeadId,
};
use crate::syntax::ast;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
pub(super) struct Pattern {
    pub head: HeadId,
    pub scope: ScopeId,
    pub limit: usize,
}
#[derive(Clone, Copy)]
enum State {
    InProgress,
    Complete,
}
#[derive(Default)]
pub(super) struct Defaults {
    states: HashMap<(EntityId, usize), State>,
    members: HashMap<EntityId, (ScopeId, usize)>,
    patterns: HashMap<EntityId, Vec<Option<DefaultArgument>>>,
    constructor_roots: Vec<(BindingId, usize)>,
    constructor_edges: HashMap<EntityId, Vec<(BindingId, usize)>>,
    constructor_requests: HashSet<(Option<EntityId>, BindingId, usize)>,
}
impl Defaults {
    pub(super) fn pattern(&self, entity: EntityId, index: usize) -> Option<DefaultArgument> {
        self.patterns.get(&entity)?.get(index).copied().flatten()
    }
}
impl Analyzer<'_> {
    /// C++ `call_copy` asks for omitted constructor arguments only when emitting
    /// its selected use. Retain that edge on its emitting function instead of
    /// instantiating defaults while checking an unused inline body.
    pub(super) fn request_constructor_defaults(
        &mut self,
        binding: BindingId,
        supplied: usize,
    ) -> Result<(), Error> {
        if !self.lowering() || self.unevaluated != 0 || self.template_depth != 0 {
            return Ok(());
        }
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("constructor defaults without signature"))?;
        if self.graph.types.function_parameter_count(ty)? <= supplied {
            return Ok(());
        }
        let owner = self
            .body_context
            .as_ref()
            .map(|body| body.entity)
            .filter(|&entity| {
                self.graph.entity(entity).definition_emission
                    == super::function_demands::DefinitionEmission::Demanded
                    || self.graph.entity(entity).inline
                        == super::specifiers::InlinePreference::Inline
            });
        let defaults = &mut self.graph.function_templates.argument_defaults;
        if !defaults
            .constructor_requests
            .insert((owner, binding, supplied))
        {
            return Ok(());
        }
        if let Some(owner) = owner {
            defaults
                .constructor_edges
                .entry(owner)
                .or_default()
                .push((binding, supplied));
        } else {
            defaults.constructor_roots.push((binding, supplied));
        }
        Ok(())
    }

    pub(super) fn prepare_constructor_defaults(
        &mut self,
        owner: Option<EntityId>,
    ) -> Result<(), Error> {
        let mut bindings = owner
            .and_then(|entity| {
                self.graph
                    .function_templates
                    .argument_defaults
                    .constructor_edges
                    .remove(&entity)
            })
            .unwrap_or_default();
        loop {
            bindings.extend(std::mem::take(
                &mut self
                    .graph
                    .function_templates
                    .argument_defaults
                    .constructor_roots,
            ));
            if bindings.is_empty() {
                return Ok(());
            }
            for (binding, supplied) in bindings.drain(..) {
                self.instantiate_call_defaults(binding, supplied)?;
            }
        }
    }

    // function_templates.cpp::specialize: only parameters outside the first
    // and last expansion retain defaults. Pack elements never inherit one.
    pub(super) fn specialization_defaults(
        &mut self,
        entity: EntityId,
        primary: BindingId,
        pattern: super::types::TypeId,
        ty: super::types::TypeId,
    ) -> Result<(), Error> {
        let primary = self.graph.binding(primary).entity;
        if self.graph.entity(primary).defaults.is_empty() {
            return Ok(());
        }
        let count = self.graph.types.function_parameter_count(ty)?;
        let patterns = self.graph.types.function_parameter_count(pattern)?;
        let mut defaults = vec![None; count];
        let mut front = 0;
        while front < patterns.min(count)
            && !matches!(
                self.graph
                    .types
                    .get(self.graph.types.function_parameter(pattern, front)?)
                    .kind,
                super::types::TypeKind::Expansion(_)
            )
        {
            defaults[front] = self.graph.entity(primary).defaults[front];
            front += 1;
        }
        let mut back = 1;
        while back <= patterns - front
            && back <= count - front
            && !matches!(
                self.graph
                    .types
                    .get(
                        self.graph
                            .types
                            .function_parameter(pattern, patterns - back)?
                    )
                    .kind,
                super::types::TypeKind::Expansion(_)
            )
        {
            defaults[count - back] = self.graph.entity(primary).defaults[patterns - back];
            back += 1;
        }
        self.graph
            .function_templates
            .argument_defaults
            .patterns
            .insert(entity, defaults);
        Ok(())
    }

    pub(super) fn defer_instance_defaults(&mut self, entity: EntityId) -> bool {
        let Some(home) = self.graph.entity(entity).home else {
            return false;
        };
        if self.template_depth != 0 || !self.graph.within_class_instance(home) {
            return false;
        }
        let limit = self
            .graph
            .lookup_limit
            .unwrap_or_else(|| self.graph.binding_limit());
        self.graph
            .function_templates
            .argument_defaults
            .members
            .entry(entity)
            .or_insert((home, limit));
        true
    }

    pub(super) fn instantiate_call_defaults(
        &mut self,
        binding: BindingId,
        supplied: usize,
    ) -> Result<(), Error> {
        self.instantiate_selected_defaults(binding, supplied, None)
    }

    pub(super) fn constant_default_argument(
        &mut self,
        function: EntityId,
        index: usize,
    ) -> Result<Option<ast::ExprId>, Error> {
        let end = index
            .checked_add(1)
            .ok_or(Error::Limit("default argument index limit"))?;
        let binding = self.graph.function_origin(function)?;
        self.instantiate_selected_defaults(binding, index, Some(end))?;
        Ok(self.graph.function_default(function, index))
    }

    fn instantiate_selected_defaults(
        &mut self,
        binding: BindingId,
        supplied: usize,
        end: Option<usize>,
    ) -> Result<(), Error> {
        let entity = self.graph.binding(binding).entity;
        let Some(_) = self.graph.function_templates.primary(entity) else {
            return self.instantiate_member_defaults(entity, supplied, end);
        };
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("default without function type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        for index in supplied..end.map_or(count, |end| end.min(count)) {
            let key = (entity, index);
            match self
                .graph
                .function_templates
                .argument_defaults
                .states
                .get(&key)
            {
                Some(State::Complete) => continue,
                Some(State::InProgress) => {
                    let default = self
                        .graph
                        .function_templates
                        .argument_defaults
                        .pattern(entity, index)
                        .ok_or(Error::Invariant("recursive default without pattern"))?;
                    return Err(Error::at(
                        self.tree.expressions.get(default.expression).location,
                        "template instantiation nested too deeply",
                    ));
                }
                None => (),
            }
            let Some(default) = self
                .graph
                .function_templates
                .argument_defaults
                .pattern(entity, index)
            else {
                continue;
            };
            let pattern = default
                .template
                .ok_or(Error::Invariant("template default without context"))?;
            self.graph
                .function_templates
                .argument_defaults
                .states
                .insert(key, State::InProgress);
            let result = self.instantiate_default(entity, ty, index, default, pattern);
            if let Err(error) = result {
                self.graph
                    .function_templates
                    .argument_defaults
                    .states
                    .remove(&key);
                return Err(error.required_instantiation());
            }
            self.graph
                .function_templates
                .argument_defaults
                .states
                .insert(key, State::Complete);
        }
        Ok(())
    }

    fn instantiate_member_defaults(
        &mut self,
        entity: EntityId,
        supplied: usize,
        end: Option<usize>,
    ) -> Result<(), Error> {
        let Some(&(scope, limit)) = self
            .graph
            .function_templates
            .argument_defaults
            .members
            .get(&entity)
        else {
            return Ok(());
        };
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("member default without function type"))?;
        let count = self.graph.entity(entity).defaults.len();
        for index in supplied..end.map_or(count, |end| end.min(count)) {
            let Some(default) = self.graph.entity(entity).defaults[index] else {
                continue;
            };
            let key = (entity, index);
            let location = self.tree.expressions.get(default.expression).location;
            match self
                .graph
                .function_templates
                .argument_defaults
                .states
                .get(&key)
            {
                Some(State::Complete) => continue,
                Some(State::InProgress) => {
                    return Err(Error::at(
                        location,
                        "template instantiation nested too deeply",
                    ));
                }
                None => (),
            }
            self.graph
                .function_templates
                .argument_defaults
                .states
                .insert(key, State::InProgress);
            let target = self.graph.types.function_parameter(ty, index)?;
            let result = self.required_instantiation(
                self.instantiation_depth,
                location,
                limit,
                |analyzer| analyzer.form_default(default.expression, target, scope, limit),
            );
            if let Err(error) = result {
                self.graph
                    .function_templates
                    .argument_defaults
                    .states
                    .remove(&key);
                return Err(error);
            }
            self.graph
                .function_templates
                .argument_defaults
                .states
                .insert(key, State::Complete);
        }
        Ok(())
    }

    fn instantiate_default(
        &mut self,
        entity: EntityId,
        ty: super::types::TypeId,
        index: usize,
        default: DefaultArgument,
        pattern: Pattern,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let frame = self.template_body_frame(entity, pattern.head, pattern.scope)?;
        let copy = self.copy_expression_in_frame(default.expression, frame)?;
        let target = self.graph.types.function_parameter(ty, index)?;
        self.form_default(copy, target, frame, pattern.limit)?;
        let count = self.graph.types.function_parameter_count(ty)?;
        self.graph.entity_mut(entity).defaults.resize(count, None);
        self.graph.entity_mut(entity).defaults[index] = Some(DefaultArgument {
            expression: copy,
            template: None,
            ..default
        });
        Ok(())
    }

    fn form_default(
        &mut self,
        copy: ast::ExprId,
        target: super::types::TypeId,
        frame: ScopeId,
        limit: usize,
    ) -> Result<(), Error> {
        // The default belongs to its declaring scope, with no caller locals or
        // implicit object. Instantiation is evaluated even inside sizeof/noexcept.
        let saved_body = self.body_context.take();
        let saved_limit = self.graph.lookup_limit.replace(limit);
        let saved_scope = self.default_argument_scope.replace(frame);
        let saved_unevaluated = std::mem::replace(&mut self.unevaluated, 0);
        let saved_depth = std::mem::replace(&mut self.template_depth, 0);
        let saved_class_depth = std::mem::replace(&mut self.class_depth, 0);
        let saved_head = self.template_head.take();
        let saved_initializing = self.initializing.take();
        let saved_base = self.base_object.take();
        let saved_uses = self.dependent_uses.replace(0);
        let result = (|| {
            if matches!(
                self.tree.expressions.get(copy).kind,
                ast::ExprKind::Braced { .. }
            ) {
                self.initialize_list_argument(copy, target, frame, None)
            } else {
                self.typed_expression(copy, frame)?;
                self.convert_scalar(copy, target, false)?;
                self.record_class_copy(
                    copy,
                    target,
                    frame,
                    super::copy_initialization::CopyUse::Value { in_place: false },
                    self.tree.expressions.get(copy).location,
                )
            }
        })();
        self.body_context = saved_body;
        self.graph.lookup_limit = saved_limit;
        self.default_argument_scope = saved_scope;
        self.unevaluated = saved_unevaluated;
        self.template_depth = saved_depth;
        self.class_depth = saved_class_depth;
        self.template_head = saved_head;
        self.initializing = saved_initializing;
        self.base_object = saved_base;
        self.dependent_uses.set(saved_uses);
        result
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
    fn converting_defaults_follow_emission_edges_and_share_one_typed_copy() {
        for lowering in [false, true] {
            let source = SourceBuffer::new(
                b"struct Deferred{template<class T>Deferred(T,int=T::missing){}};\
                  inline void unused(){Deferred d=1;}\
                  struct Box{int n;template<class T>Box(T v,int x=7):n(v+x){}};\
                  inline int used(){Box a=1;Box b=2;return a.n+b.n;}\
                  int main(){return used()-17;}"
                    .to_vec(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"conversion-defaults").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let graph = super::super::analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                true,
                lowering,
            )
            .unwrap();
            let defaults = &graph.function_templates.argument_defaults;
            assert_eq!(defaults.states.len(), usize::from(lowering));
            if lowering {
                assert_eq!(defaults.constructor_requests.len(), 2);
                assert_eq!(defaults.constructor_edges.len(), 1);
                assert!(defaults.constructor_roots.is_empty());
                let (&(entity, index), &state) = defaults.states.iter().next().unwrap();
                assert!(matches!(state, State::Complete));
                let pattern = defaults.pattern(entity, index).unwrap();
                let typed = graph.entity(entity).defaults[index].unwrap();
                assert_ne!(pattern.expression, typed.expression);
                assert!(graph.expression_facts.get(pattern.expression).is_err());
                assert!(graph.expression_facts.get(typed.expression).is_ok());
            }
        }
    }

    #[test]
    fn expanded_defaults_keep_partition_positions_and_only_demanded_copies() {
        let source = SourceBuffer::new(
            b"template<class...T>int f(T...t,int n=sizeof...(T)){return n;}\
              int main(){return f<>()+f<int>(1)+f<int,long>(1,2)+f<int>(2);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"pack-defaults").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let original = tree.expressions.mark();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let defaults = &graph.function_templates.argument_defaults;
        assert_eq!(defaults.patterns.len(), 3);
        assert_eq!(defaults.states.len(), 3);
        let mut positions = Vec::new();
        let mut copies = std::collections::HashSet::new();
        for (&entity, patterns) in &defaults.patterns {
            let count = patterns.len() - 1;
            positions.push(count);
            assert!(patterns[..count].iter().all(Option::is_none));
            let pattern = patterns[count].unwrap();
            assert!(pattern.template.is_some());
            assert!(graph.expression_facts.get(pattern.expression).is_err());
            assert!(matches!(defaults.states[&(entity, count)], State::Complete));
            let copy = graph.entity(entity).defaults[count].unwrap();
            assert!(copy.template.is_none());
            assert!(copy.expression.index() >= original);
            assert!(copies.insert(copy.expression));
            assert!(graph.expression_facts.get(copy.expression).is_ok());
        }
        positions.sort_unstable();
        assert_eq!(positions, [0, 1, 2]);
    }

    #[test]
    fn member_defaults_keep_class_copies_and_only_selected_defaults_get_facts() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{A(int x=sizeof(T),int y=T::bad){}\
              int g(int x=T::bad,int y=sizeof(T)){return x+y;}};\
              int main(){A<int>a(1,2);A<char>b(1,2);return a.g(2)+a.g(3)+b.g(2);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-default-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let original = tree.expressions.mark();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let defaults = &graph.function_templates.argument_defaults;
        assert_eq!(defaults.states.len(), 2);
        let mut expressions = std::collections::HashSet::new();
        for (&(entity, index), state) in &defaults.states {
            assert_eq!(index, 1);
            assert!(matches!(state, State::Complete));
            let function = graph.entity(entity);
            assert!(
                graph
                    .expression_facts
                    .get(function.defaults[0].unwrap().expression)
                    .is_err()
            );
            let expression = function.defaults[index].unwrap().expression;
            assert!(expression.index() >= original);
            assert!(expressions.insert(expression));
            assert!(graph.expression_facts.get(expression).is_ok());
            assert!(defaults.members.contains_key(&entity));
        }
        for &entity in defaults.members.keys() {
            let function = graph.entity(entity);
            if function
                .member_function
                .is_some_and(|member| member.kind == super::super::graph::MemberKind::Constructor)
            {
                for default in function.defaults.iter().flatten() {
                    assert!(graph.expression_facts.get(default.expression).is_err());
                }
            }
        }
    }

    #[test]
    fn default_copies_belong_to_instances_and_only_demanded_parameters_get_facts() {
        let source = SourceBuffer::new(
            b"template<class T>int f(T,int x=T::missing,int y=sizeof(T)){return x+y;}\
              template<class T>int unused(T,int x=T::missing){return x;}\
              int main(){return f(1,2)+f(2,3)+f('a',2)+f(1,2,4);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let original_expressions = tree.expressions.mark();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let defaults = &graph.function_templates.argument_defaults;
        assert_eq!(defaults.states.len(), 2);
        let mut copies = std::collections::HashSet::new();
        let mut primaries = std::collections::HashSet::new();
        for (&(entity, index), state) in &defaults.states {
            assert_eq!(index, 2);
            assert!(matches!(state, State::Complete));
            let function = graph.entity(entity);
            assert!(function.defaults[0].is_none());
            assert!(function.defaults[1].is_none());
            let copy = function.defaults[2].unwrap().expression;
            assert!(copy.index() >= original_expressions);
            assert!(copies.insert(copy));
            assert!(graph.expression_facts.get(copy).is_ok());
            let primary = graph
                .binding(graph.function_templates.primary(entity).unwrap())
                .entity;
            primaries.insert(primary);
            for pattern in graph.entity(primary).defaults.iter().flatten() {
                assert!(pattern.template.is_some());
                assert!(graph.expression_facts.get(pattern.expression).is_err());
                assert_ne!(pattern.expression, copy);
            }
        }
        assert_eq!(primaries.len(), 1);
        assert!(graph.lookup_limit.is_none());
    }

    #[test]
    fn failed_default_restores_the_caller_context_and_publishes_no_completed_fact() {
        let source = SourceBuffer::new(
            b"template<class T>int f(T,int x=T::missing);int caller(){return f(1);}".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-error-context").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        let declaration = analyzer.tree.decl_lists.get(declarations)[0];
        analyzer.declaration(declaration, global).unwrap();
        let head = analyzer.template_head;
        let saved_limit = analyzer.graph.lookup_limit;
        analyzer.dependent_uses.set(17);
        let declaration = analyzer.tree.decl_lists.get(declarations)[1];
        assert!(matches!(
            analyzer.declaration(declaration, global),
            Err(Error::InstantiationDiagnostic { .. })
        ));
        assert_eq!(analyzer.graph.lookup_limit, saved_limit);
        assert_eq!(analyzer.dependent_uses.get(), 17);
        assert_eq!(analyzer.template_head, head);
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.template_depth, 0);
        assert!(analyzer.default_argument_scope.is_none());
        assert!(analyzer.initializing.is_none());
        assert!(analyzer.base_object.is_none());
        assert!(
            analyzer
                .graph
                .function_templates
                .argument_defaults
                .states
                .is_empty()
        );
    }
}
