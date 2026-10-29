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
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use crate::syntax::ast;

#[derive(Clone, Copy)]
pub(super) struct Pattern {
    pub head: HeadId,
    pub scope: ScopeId,
    pub limit: usize,
}
#[derive(Default)]
pub(super) struct Defaults {
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
impl super::graph::Graph {
    pub(crate) fn lower_demand_default_argument(
        &mut self,
        tree: &mut ast::Tree,
        cursor: &mut crate::syntax::cursor::Cursor<'_>,
        budget: crate::support::stack::StackBudget,
        stack_base: usize,
        function: EntityId,
        index: usize,
    ) -> Result<Option<ast::ExprId>, Error> {
        let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(self))?;
        analyzer.set_lowering();
        analyzer.stack_base = stack_base;
        let result = analyzer
            .constant_default_argument(function, index)
            .and_then(|expression| {
                analyzer.finish_lowering_boundaries()?;
                Ok(expression)
            });
        *self = analyzer.into_graph();
        result
    }
}
impl Analyzer<'_> {
    /// A shared plan's selected subobject calls retain their own supplied
    /// counts. Activate their defaults only when that plan is reached.
    pub(super) fn request_subobject_constructor_defaults(
        &mut self,
        id: super::construction::ConstructorId,
    ) -> Result<(), Error> {
        for index in 0..self.graph.constructors.get(id).bases.len() {
            let base = self.graph.constructors.get(id).bases[index];
            if !base.forwards
                && let Some(child) = base.constructor
            {
                self.request_planned_constructor_defaults(child, base.arguments)?;
            }
        }
        for index in 0..self.graph.constructors.get(id).subobjects.len() {
            let Some(member) = self.graph.constructors.get(id).subobjects[index] else {
                continue;
            };
            self.request_planned_constructor_defaults(member.constructor, member.arguments)?;
        }
        if let Some(delegation) = self.graph.constructors.get(id).delegation {
            self.request_planned_constructor_defaults(
                delegation.constructor,
                Some(delegation.arguments),
            )?;
        }
        let count = self
            .graph
            .constructors
            .get(id)
            .copy
            .as_ref()
            .map_or(0, Vec::len);
        for index in 0..count {
            let action = self
                .graph
                .constructors
                .get(id)
                .copy
                .as_ref()
                .ok_or(Error::Invariant("copy defaults without actions"))?[index];
            if let Some(child) = action.constructor
                && let Some(binding) = self.graph.constructors.get(child).binding
            {
                self.request_constructor_defaults(binding, 1)?;
            }
        }
        Ok(())
    }

    fn request_planned_constructor_defaults(
        &mut self,
        id: super::construction::ConstructorId,
        arguments: Option<super::constructor_initialization::ConstructorArguments>,
    ) -> Result<(), Error> {
        use super::constructor_initialization::ConstructorArguments;
        if let Some(binding) = self.graph.constructors.get(id).binding {
            let supplied = match arguments {
                Some(ConstructorArguments::Single(_)) => 1,
                Some(ConstructorArguments::Values(values)) => {
                    self.tree.expr_lists.get(values).len()
                }
                Some(ConstructorArguments::Value) | None => 0,
            };
            self.request_constructor_defaults(binding, supplied)?;
        }
        Ok(())
    }

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
            .and_then(|body| body.function)
            .filter(|&entity| {
                self.graph.entity(entity).callable().definition_emission
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
        if self.graph.entity(primary).callable().defaults.is_empty() {
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
            defaults[front] = self.graph.entity(primary).callable().defaults[front];
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
            defaults[count - back] =
                self.graph.entity(primary).callable().defaults[patterns - back];
            back += 1;
        }
        self.graph
            .entity_mut(entity)
            .callable_mut()
            .defaults
            .clone_from(&defaults);
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
        if self.template_depth != 0 || !self.graph.instance_class(home) {
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

    fn untyped_default_argument(&self, entity: EntityId, index: usize) -> Option<DefaultArgument> {
        self.graph
            .entity(entity)
            .callable()
            .defaults
            .get(index)
            .copied()
            .flatten()
            .filter(|default| {
                self.graph
                    .expression_facts
                    .known(default.expression)
                    .is_none()
            })
    }

    fn instantiate_selected_defaults(
        &mut self,
        binding: BindingId,
        supplied: usize,
        end: Option<usize>,
    ) -> Result<(), Error> {
        let entity = self.graph.binding(binding).entity;
        let Some(_primary) = self.graph.function_templates.primary(entity) else {
            return self.instantiate_member_defaults(entity, supplied, end);
        };
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("default without function type"))?;
        let count = self.graph.entity(entity).callable().defaults.len();
        for index in supplied..end.map_or(count, |end| end.min(count)) {
            let Some(default) = self.untyped_default_argument(entity, index) else {
                continue;
            };
            let pattern = default
                .template
                .ok_or(Error::Invariant("template default without context"))?;
            self.instantiate_default(entity, ty, index, default, pattern)?;
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
        let count = self.graph.entity(entity).callable().defaults.len();
        for index in supplied..end.map_or(count, |end| end.min(count)) {
            let Some(default) = self.untyped_default_argument(entity, index) else {
                continue;
            };
            let target = self.graph.types.function_parameter(ty, index)?;
            self.form_default(default.expression, target, scope, limit, entity, index)?;
            self.graph.work.totals.instantiated_arguments += 1;
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
        self.form_default(copy, target, frame, pattern.limit, entity, index)?;
        let count = self.graph.types.function_parameter_count(ty)?;
        self.graph
            .entity_mut(entity)
            .callable_mut()
            .defaults
            .resize(count, None);
        self.graph.entity_mut(entity).callable_mut().defaults[index] = Some(DefaultArgument {
            expression: copy,
            template: None,
            ..default
        });
        self.graph.work.totals.instantiated_arguments += 1;
        Ok(())
    }

    fn form_default(
        &mut self,
        copy: ast::ExprId,
        target: super::types::TypeId,
        frame: ScopeId,
        limit: usize,
        function: EntityId,
        index: usize,
    ) -> Result<(), Error> {
        let location = self.tree.expressions.get(copy).location;
        self.required_instantiation(self.instantiation_depth, location, limit, |analyzer| {
            analyzer.form_default_expression(copy, target, frame, function, index)
        })
    }

    fn form_default_expression(
        &mut self,
        copy: ast::ExprId,
        target: super::types::TypeId,
        frame: ScopeId,
        function: EntityId,
        index: usize,
    ) -> Result<(), Error> {
        let index = u32::try_from(index).map_err(|_| Error::Limit("default argument index"))?;
        // The default belongs to its declaring scope, with no caller locals or
        // implicit object. Instantiation is evaluated even inside sizeof/noexcept.
        let saved_body = self.body_context.take();
        let saved_context = self.closure_default_argument;
        self.closure_default_argument = self
            .graph
            .entity(function)
            .home
            .filter(|&home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
            .map(|_| (function, index));
        let saved_unevaluated = std::mem::replace(&mut self.unevaluated, 0);
        let saved_depth = std::mem::replace(&mut self.template_depth, 0);
        let saved_class_depth = std::mem::replace(&mut self.class_depth, 0);
        let saved_head = self.template_head.take();
        let saved_initializing = self.initializing.take();
        let saved_declarator = self.initializing_declarator.take();
        let saved_base = self.base_object.take();
        let saved_uses = self.dependent_uses.replace(0);
        let result = (|| {
            let void = self.graph.types.scalar(super::types::Scalar::Void)?;
            self.body_context = Some(super::procedural_statements::BodyContext::new(void, None));
            if matches!(
                self.tree.expressions.get(copy).kind,
                ast::ExprKind::Braced { .. }
            ) {
                self.initialize_list_argument(copy, target, frame, None)
            } else {
                self.typed_expression(copy, frame)?;
                self.convert_scalar(copy, target, false)?;
                if let Some(class) = self.graph.class_of(target)
                    && self.same_class_source(class, copy)?
                {
                    self.record_class_copy(
                        copy,
                        target,
                        frame,
                        super::copy_initialization::CopyUse::Value { in_place: false },
                        self.tree.expressions.get(copy).location,
                    )?;
                }
                Ok(())
            }
        })();
        self.body_context = saved_body;
        self.closure_default_argument = saved_context;
        self.unevaluated = saved_unevaluated;
        self.template_depth = saved_depth;
        self.class_depth = saved_class_depth;
        self.template_head = saved_head;
        self.initializing = saved_initializing;
        self.initializing_declarator = saved_declarator;
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

    fn formed_defaults(
        graph: &super::super::graph::Graph,
    ) -> crate::support::hash::HashSet<(EntityId, usize)> {
        let defaults = &graph.function_templates.argument_defaults;
        defaults
            .patterns
            .keys()
            .chain(defaults.members.keys())
            .flat_map(|&entity| {
                graph
                    .entity(entity)
                    .callable()
                    .defaults
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, default)| {
                        default
                            .filter(|default| {
                                graph.expression_facts.known(default.expression).is_some()
                            })
                            .map(|_| (entity, index))
                    })
            })
            .collect()
    }

    #[test]
    fn cached_default_plans_keep_per_owner_demand_and_memoize_only_reached_defaults() {
        let source = SourceBuffer::new(
            b"struct Cell{template<class T=int>Cell(int=sizeof(T)){}};\
              struct Store{Cell cells[9];};\
              struct Bad{template<class T=int>Bad(int=T::missing){}};\
              struct Unused{Bad cells[2];};\
              inline void unused(){Store s;Unused u;Bad direct;}\
              int main(){Store a;Store b;Bad supplied(3);return 0;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"cached-defaults").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert!(formed_defaults(&analyzer.graph).is_empty());
        analyzer.finish_lowering_boundaries().unwrap();
        let defaults = &analyzer.graph.function_templates.argument_defaults;
        assert_eq!(formed_defaults(&analyzer.graph).len(), 1);
        assert_eq!(
            defaults.constructor_edges.len(),
            1,
            "unused owner retains its edge"
        );
        let expressions = analyzer.tree.expressions.mark();
        analyzer.finish_lowering_boundaries().unwrap();
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
    }

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
            let formed = formed_defaults(&graph);
            assert_eq!(formed.len(), usize::from(lowering));
            if lowering {
                assert_eq!(defaults.constructor_requests.len(), 2);
                assert_eq!(defaults.constructor_edges.len(), 1);
                assert!(defaults.constructor_roots.is_empty());
                let &(entity, index) = formed.iter().next().unwrap();
                let pattern = defaults.pattern(entity, index).unwrap();
                let typed = graph.entity(entity).callable().defaults[index].unwrap();
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
        assert_eq!(formed_defaults(&graph).len(), 3);
        let mut positions = Vec::new();
        let mut copies = crate::support::hash::HashSet::default();
        for (&entity, patterns) in &defaults.patterns {
            let count = patterns.len() - 1;
            positions.push(count);
            assert!(patterns[..count].iter().all(Option::is_none));
            let pattern = patterns[count].unwrap();
            assert!(pattern.template.is_some());
            assert!(graph.expression_facts.get(pattern.expression).is_err());
            let copy = graph.entity(entity).callable().defaults[count].unwrap();
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
        let formed = formed_defaults(&graph);
        assert_eq!(formed.len(), 2);
        let defaults = &graph.function_templates.argument_defaults;
        let mut expressions = crate::support::hash::HashSet::default();
        for &(entity, index) in &formed {
            assert_eq!(index, 1);
            let function = graph.entity(entity);
            assert!(
                graph
                    .expression_facts
                    .get(function.callable().defaults[0].unwrap().expression)
                    .is_err()
            );
            let expression = function.callable().defaults[index].unwrap().expression;
            assert!(expression.index() >= original);
            assert!(expressions.insert(expression));
            assert!(graph.expression_facts.get(expression).is_ok());
            assert!(defaults.members.contains_key(&entity));
        }
        for &entity in defaults.members.keys() {
            let function = graph.entity(entity);
            if function
                .callable()
                .member_function
                .is_some_and(|member| member.kind == super::super::graph::MemberKind::Constructor)
            {
                for default in function.callable().defaults.iter().flatten() {
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
        let formed = formed_defaults(&graph);
        assert_eq!(formed.len(), 2);
        let mut copies = crate::support::hash::HashSet::default();
        let mut primaries = crate::support::hash::HashSet::default();
        for &(entity, index) in &formed {
            assert_eq!(index, 2);
            let function = graph.entity(entity);
            assert!(function.callable().defaults[0].is_none());
            let unused = function.callable().defaults[1].unwrap();
            assert!(graph.expression_facts.known(unused.expression).is_none());
            let copy = function.callable().defaults[2].unwrap().expression;
            assert!(copy.index() >= original_expressions);
            assert!(copies.insert(copy));
            assert!(graph.expression_facts.get(copy).is_ok());
            let primary = graph
                .binding(graph.function_templates.primary(entity).unwrap())
                .entity;
            primaries.insert(primary);
            for pattern in graph.entity(primary).callable().defaults.iter().flatten() {
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
        let state = analyzer.graph.analysis.substitution.clone();
        let failures = analyzer.graph.work.totals.substitution_failures;
        let result = analyzer
            .with_substitution_soft(None, |analyzer| analyzer.declaration(declaration, global));
        assert!(matches!(result, Err(Error::InstantiationDiagnostic { .. })));
        assert_eq!(state.current().soft, 0);
        assert!(state.current().failure.is_none());
        assert_eq!(analyzer.graph.work.totals.substitution_failures, failures);
        assert_eq!(analyzer.graph.lookup_limit, saved_limit);
        assert_eq!(analyzer.dependent_uses.get(), 17);
        assert_eq!(analyzer.template_head, head);
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.template_depth, 0);
        assert!(analyzer.default_argument_scope.is_none());
        assert!(analyzer.initializing.is_none());
        assert!(analyzer.base_object.is_none());
        assert!(formed_defaults(&analyzer.graph).is_empty());
    }
}
