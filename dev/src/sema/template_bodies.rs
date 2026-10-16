// Translated from class_templates.cpp::instantiate_definition and analyzer.cpp. See NOTICE.
//! Unit-tail body worklist, independent of signature and emission demand.
use super::{
    Analyzer, Error, Graph,
    analyzer::DeferredBody,
    graph::{Binding, BindingKind, EntityId, ScopeKind},
    special_members::SpecialDefinition,
};
use crate::{
    support::stack::StackBudget,
    syntax::{ParseError, ast, cloning::Cloner, cursor::Cursor, storage::List},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct Pattern {
    body: DeferredBody,
    head: super::templates::HeadId,
    limit: usize,
}
#[derive(Clone, Copy)]
enum State {
    InProgress,
    Unavailable,
    Complete(ast::BodyId),
}
#[derive(Default)]
pub(super) struct Bodies {
    patterns: HashMap<EntityId, Pattern>,
    members: HashMap<EntityId, (DeferredBody, usize)>,
    member_links: HashMap<EntityId, EntityId>,
    member_definitions: HashMap<EntityId, (DeferredBody, usize)>,
    states: HashMap<EntityId, State>,
    demands: Vec<EntityId>,
    seen: HashSet<EntityId>,
}
impl Bodies {
    pub(super) fn source(&self, entity: EntityId) -> Option<EntityId> {
        self.member_links.get(&entity).copied()
    }
    pub(super) fn defined_member(&self, entity: EntityId) -> bool {
        self.members.contains_key(&entity)
            || self
                .member_links
                .get(&entity)
                .is_some_and(|source| self.member_definitions.contains_key(source))
    }
    pub(super) fn pending_member(&self, entity: EntityId) -> bool {
        self.member(entity) && !self.states.contains_key(&entity)
    }
    pub(super) fn member(&self, entity: EntityId) -> bool {
        self.members.contains_key(&entity) || self.member_links.contains_key(&entity)
    }
    pub(super) fn link_member(&mut self, source: EntityId, target: EntityId) {
        self.member_links.insert(target, source);
    }
    pub(super) fn record_member_definition(&mut self, body: DeferredBody, limit: usize) {
        self.member_definitions.insert(body.entity, (body, limit));
    }
    pub(super) fn demand(&mut self, entity: EntityId) {
        if self.seen.insert(entity) {
            self.demands.push(entity);
        }
    }
    pub(super) fn body(&self, entity: EntityId) -> Option<ast::BodyId> {
        match self.states.get(&entity) {
            Some(State::Complete(body)) => Some(*body),
            _ => None,
        }
    }
}
impl super::function_templates::FunctionTemplates {
    pub(super) fn record_pattern(
        &mut self,
        body: DeferredBody,
        head: super::templates::HeadId,
        limit: usize,
    ) {
        self.bodies
            .patterns
            .insert(body.entity, Pattern { body, head, limit });
    }
}

/// Semantic analysis exclusively borrows the unit syntax: defaults append at
/// selected calls, while this driver drains unit-tail body demands. Both paths
/// read ordinary slab IDs directly, with no pattern redirection or overlay map.
pub(crate) fn analyze_unit(
    tree: &mut ast::Tree,
    cursor: &mut Cursor<'_>,
    declarations: List<ast::DeclId>,
    budget: StackBudget,
    semantics: bool,
    lowering: bool,
) -> Result<Graph, Error> {
    let mut analyzer = Analyzer::new(tree, cursor, budget)?;
    analyzer.set_semantics(semantics);
    if lowering {
        analyzer.set_lowering();
    }
    let mut graph = if semantics {
        analyzer.analyze_declarations(declarations, false)?
    } else {
        return analyzer.analyze(declarations);
    };
    let marker = 0u8;
    let base = std::ptr::from_ref(&marker).addr();
    let mut next = 0;
    let mut named = 0;
    // Instantiating one demanded definition can append more demands. Each
    // canonical entity enters this queue once, in first-use order.
    loop {
        let definition = graph
            .function_templates
            .instantiations
            .definitions()
            .get(named)
            .copied();
        let entity = if let Some(definition) = definition {
            named += 1;
            graph.binding(definition.binding).entity
        } else if let Some(&entity) = graph.function_templates.bodies.demands.get(next) {
            next += 1;
            entity
        } else {
            if lowering {
                let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(graph))?;
                analyzer.set_lowering();
                analyzer.stack_base = base;
                analyzer.finish_lowering_boundaries()?;
                graph = analyzer.finish_analysis(false)?;
                if next < graph.function_templates.bodies.demands.len() {
                    continue;
                }
            }
            break;
        };
        if graph.function_instantiation_suppressed(entity) {
            continue;
        }
        if graph.function_templates.instantiations.defined(entity)
            && graph.function_templates.bodies.pending_member(entity)
            && matches!(
                graph.entity(entity).special_definition,
                Some(SpecialDefinition::Defaulted { .. })
            )
        {
            let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(graph))?;
            analyzer.set_semantics(true);
            if lowering {
                analyzer.set_lowering();
            }
            analyzer.stack_base = base;
            analyzer.required_instantiation(
                analyzer.graph.instantiation_levels.get(entity)?,
                crate::support::source::SourceOffset(0),
                analyzer.graph.binding_limit(),
                |analyzer| analyzer.prepare_explicit_defaulted(entity),
            )?;
            graph = analyzer.finish_analysis(false)?;
        }
        if graph
            .function_templates
            .bodies
            .member_links
            .contains_key(&entity)
            && !graph
                .function_templates
                .bodies
                .members
                .contains_key(&entity)
            && !graph.function_templates.bodies.states.contains_key(&entity)
        {
            let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(graph))?;
            analyzer.set_semantics(true);
            if lowering {
                analyzer.set_lowering();
            }
            analyzer.stack_base = base;
            analyzer.prepare_member_definition(entity)?;
            graph = analyzer.finish_analysis(false)?;
        }
        if let Some(&(pattern, limit)) = graph.function_templates.bodies.members.get(&entity) {
            if graph.function_templates.bodies.states.contains_key(&entity) {
                continue;
            }
            graph
                .function_templates
                .bodies
                .states
                .insert(entity, State::InProgress);
            let mut cloner = Cloner::new(tree, budget, base);
            let parameters = cloner
                .parameters(pattern.parameters)
                .map_err(|error| copy_error(&error))?;
            let body = cloner
                .body(pattern.body)
                .map_err(|error| copy_error(&error))?;
            let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(graph))?;
            analyzer.set_semantics(true);
            if lowering {
                analyzer.set_lowering();
            }
            analyzer.stack_base = base;
            let special = analyzer.graph.entity(entity).special_definition;
            analyzer.graph.entity_mut(entity).special_definition = match special {
                Some(SpecialDefinition::Body(_)) => Some(SpecialDefinition::Body(body)),
                Some(SpecialDefinition::Defaulted {
                    first,
                    body: Some(_),
                }) => Some(SpecialDefinition::Defaulted {
                    first,
                    body: Some(body),
                }),
                other => other,
            };
            let location = match analyzer.tree.bodies.get(body) {
                ast::Body::Parsed(body) => analyzer.tree.statements.get(body.compound).location,
                ast::Body::Deferred => return Err(Error::Invariant("unparsed instance body")),
            };
            analyzer.required_instantiation(
                analyzer.graph.instantiation_levels.get(entity)?,
                location,
                limit,
                |analyzer| {
                    analyzer.function_body(DeferredBody {
                        parameters,
                        body,
                        ..pattern
                    })
                },
            )?;
            graph = analyzer.finish_analysis(false)?;
            graph
                .function_templates
                .bodies
                .states
                .insert(entity, State::Complete(body));
            continue;
        }
        if matches!(
            graph.function_templates.bodies.states.get(&entity),
            Some(State::Unavailable)
        ) {
            continue;
        }
        let primary = graph
            .function_templates
            .primary(entity)
            .ok_or(Error::Invariant("body demand without template"))?;
        let primary = graph.binding(primary).entity;
        let Some(pattern) = graph
            .function_templates
            .bodies
            .patterns
            .get(&primary)
            .copied()
        else {
            if let Some(definition) = definition
                && graph.entity(primary).availability != super::graph::FunctionAvailability::Deleted
            {
                return Err(Error::at(
                    definition.location,
                    "an explicit instantiation definition of a function whose definition is not available",
                ));
            }
            // The oracle retains a declaration when the template has no
            // definition in this unit. This demand does not synthesize a body.
            continue;
        };
        if graph.function_templates.bodies.states.contains_key(&entity) {
            continue;
        }
        graph
            .function_templates
            .bodies
            .states
            .insert(entity, State::InProgress);
        let mut cloner = Cloner::new(tree, budget, base);
        let parameters = cloner
            .parameters(pattern.body.parameters)
            .map_err(|error| copy_error(&error))?;
        let body = cloner
            .body(pattern.body.body)
            .map_err(|error| copy_error(&error))?;
        let exception = if let super::exceptions::ExceptionState::Template(specification) =
            graph.entity(entity).exceptions
        {
            Some((
                specification,
                cloner
                    .expression(specification.operand)
                    .map_err(|error| copy_error(&error))?,
            ))
        } else {
            None
        };
        let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(graph))?;
        analyzer.set_semantics(true);
        if lowering {
            analyzer.set_lowering();
        }
        analyzer.stack_base = base;
        let location = match analyzer.tree.bodies.get(body) {
            ast::Body::Parsed(body) => analyzer.tree.statements.get(body.compound).location,
            ast::Body::Deferred => return Err(Error::Invariant("unparsed instance body")),
        };
        analyzer.instantiation_context(
            analyzer.graph.instantiation_levels.get(entity)?,
            location,
            pattern.limit,
            |analyzer| {
                analyzer.instantiate_body(entity, primary, pattern, parameters, body, exception)
            },
        )?;
        graph = analyzer.finish_analysis(false)?;
        graph
            .function_templates
            .bodies
            .states
            .insert(entity, State::Complete(body));
    }
    Ok(graph)
}

pub(super) fn copy_error(error: &ParseError) -> Error {
    match error {
        ParseError::Limit(message) => Error::Limit(message),
        ParseError::NotImplemented(group) => Error::Missing(group),
        _ => Error::Invariant("invalid parsed template body copy"),
    }
}
impl Analyzer<'_> {
    fn prepare_explicit_defaulted(&mut self, entity: EntityId) -> Result<(), Error> {
        let Some(member) = self.graph.entity(entity).member_function else {
            return Ok(());
        };
        let binding = self.graph.function_origin(entity)?;
        let ty = self.graph.lower_entity_type(entity)?;
        let first = matches!(
            self.graph.entity(entity).special_definition,
            Some(SpecialDefinition::Defaulted { first: true, .. })
        );
        if first && member.kind == super::graph::MemberKind::Destructor && !self.lowering() {
            // The semantic view only records this root. Emission preparation
            // checks its defaulted definition even if class completion has
            // already marked the destructor unavailable.
            return self.prepare_destruction(member.class);
        }
        if first
            && member.kind == super::graph::MemberKind::Constructor
            && self.graph.types.function_parameter_count(ty)? != 0
        {
            // An explicit root emits the synthesized copy, including its
            // deleted-property result; it is not an odr-use of that function.
            self.prepare_copy_properties(member.class)?;
            self.declared_copy_plan(member.class, binding)?;
            return Ok(());
        }
        if member.kind == super::graph::MemberKind::Constructor
            && self.graph.types.function_parameter_count(ty)? == 0
            && first
        {
            // The frozen explicit-member root bypasses instantiate_definition
            // for an in-class defaulted default constructor. Only an ordinary
            // initialization use demands its subobject/exception plan. Preserve
            // an existing plan; an otherwise unused root has a declaration body.
            if self.graph.entity(entity).constructor.is_none() {
                self.constructor_declaration_plan(binding, member.class)?;
            }
            return Ok(());
        }
        self.defaulted_constructor_definition(
            entity,
            member.class,
            member.kind,
            crate::support::source::SourceOffset(0),
        )
    }
    fn prepare_member_definition(&mut self, entity: EntityId) -> Result<(), Error> {
        let source = *self
            .graph
            .function_templates
            .bodies
            .member_links
            .get(&entity)
            .ok_or(Error::Invariant("member demand without pattern link"))?;
        let Some(&(pattern, limit)) = self
            .graph
            .function_templates
            .bodies
            .member_definitions
            .get(&source)
        else {
            self.graph
                .function_templates
                .bodies
                .states
                .insert(entity, State::Unavailable);
            return Ok(());
        };
        let parent = self.outside_member_frame(entity, pattern)?;
        let owner = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("member without owner"))?;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("member without type"))?;
        let definition = DeferredBody {
            entity,
            ty,
            parent,
            print: owner,
            template_depth: 0,
            definition_head: None,
            ..pattern
        };
        let from = self.graph.entity(source);
        let (special, syntax) = (from.special_definition, from.function_definition);
        let target = self.graph.entity_mut(entity);
        target.defined = true;
        target.special_definition = special;
        target.function_definition = syntax;
        target.definition_emission = super::function_demands::DefinitionEmission::Demanded;
        self.graph
            .function_templates
            .bodies
            .members
            .insert(entity, (definition, limit));
        Ok(())
    }

    pub(super) fn defer_instance_member(&mut self, body: DeferredBody) -> bool {
        if body.template_depth == 0
            && !self
                .graph
                .function_templates
                .bodies
                .states
                .contains_key(&body.entity)
            && self.graph.within_class_instance(body.print)
        {
            let limit = self
                .graph
                .lookup_limit
                .unwrap_or_else(|| self.graph.binding_limit());
            self.graph
                .function_templates
                .bodies
                .members
                .insert(body.entity, (body, limit));
            self.graph
                .instantiation_levels
                .record(body.entity, self.instantiation_depth);
            return true;
        }
        false
    }

    fn instantiate_body(
        &mut self,
        entity: EntityId,
        primary: EntityId,
        pattern: Pattern,
        parameters: ast::ParameterClause,
        body: ast::BodyId,
        exception: Option<(super::template_exceptions::TemplateException, ast::ExprId)>,
    ) -> Result<(), Error> {
        let definition = pattern.body;
        let frame = self.template_body_frame(entity, pattern.head, definition.parent)?;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("instance without type"))?;
        self.graph.entity_mut(entity).function_definition =
            self.graph.entity(primary).function_definition;
        self.graph.entity_mut(entity).defined = true;
        self.graph.entity_mut(entity).function_parameters = Some(parameters);
        self.graph.lookup_limit = Some(pattern.limit);
        if let Some((specification, operand)) = exception {
            let prototype_type = self.specialization_object_signature(entity, specification.ty)?;
            let parent = self.template_body_frame(
                entity,
                self.graph
                    .entity(primary)
                    .template_head
                    .ok_or(Error::Invariant("exception pattern without head"))?,
                specification.scope,
            )?;
            let prototype =
                self.exception_prototype(super::template_exceptions::TemplateException {
                    operand,
                    scope: parent,
                    ty: prototype_type,
                    ..specification
                })?;
            let saved = self.evaluating_noexcept;
            self.evaluating_noexcept = true;
            let result = (|| {
                let ty = self.exception_operand_type(operand, prototype)?;
                self.exception_bool(ty, self.tree.expressions.get(operand).location)?;
                self.constant_value(operand, prototype, true)
            })();
            self.evaluating_noexcept = saved;
            let value = result?.ok_or(Error::Invariant(
                "constant exception specification without value",
            ))?;
            self.graph.entity_mut(entity).exceptions =
                super::exceptions::ExceptionState::Value(if value.bits == 0 {
                    super::exceptions::ExceptionSet::All
                } else {
                    super::exceptions::ExceptionSet::None
                });
        }
        let result = self.function_body(DeferredBody {
            entity,
            ty,
            body,
            parameters,
            parent: frame,
            print: frame,
            template_depth: 0,
            ..definition
        });
        self.graph.lookup_limit = None;
        result?;
        Ok(())
    }
    pub(super) fn template_body_frame(
        &mut self,
        entity: EntityId,
        head: super::templates::HeadId,
        parameter_scope: super::graph::ScopeId,
    ) -> Result<super::graph::ScopeId, Error> {
        let parent = self
            .graph
            .scope(parameter_scope)
            .parent
            .ok_or(Error::Invariant(
                "template definition without enclosing scope",
            ))?;
        let frame = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, parent, None, None)?;
        let depth = self.graph.templates.depth(head);
        let count = self.graph.templates.parameters(head).len();
        for ordinal in 0..count {
            let parameter = self.graph.templates.parameters(head)[ordinal];
            let Some(name) = self.graph.entity(parameter).name else {
                continue;
            };
            let Some(source) =
                self.graph
                    .own(parameter_scope, name, super::lookup::Filter::Ordinary)
            else {
                continue;
            };
            let binding = self.graph.binding(source);
            let Some(parameter) = self.graph.entity(binding.entity).template_parameter else {
                continue;
            };
            let index = parameter
                .index(depth)
                .ok_or(Error::Invariant("body parameter depth"))?;
            let ty = *self
                .graph
                .function_templates
                .arguments
                .get(&entity)
                .and_then(|arguments| arguments.get(index))
                .ok_or(Error::Invariant("template body without argument"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor, support::source::SourceBuffer, syntax::SyntaxUnit,
    };

    #[test]
    fn friends_keep_namespace_identity_and_copy_only_demanded_instance_bodies() {
        let source = SourceBuffer::new(
            b"template<class T>struct Owner{\
                friend int read(Owner){return sizeof(T);}\
                friend int unused(Owner){return T::missing;}\
            };int read(Owner<int>);Owner<char> global;\
            int main(){Owner<int>a;Owner<char>b;return read(a)+read(b)+read(a);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"friend-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let read = cursor.intern_name(b"read").unwrap();
        let global = cursor.intern_name(b"global").unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let bodies = &graph.function_templates.bodies;
        assert_eq!(bodies.members.len(), 4);
        assert_eq!(bodies.member_links.len(), 4);
        let mut completed = HashSet::new();
        for (&entity, &(pattern, _)) in &bodies.members {
            assert_eq!(graph.entity(entity).home, Some(graph.global));
            assert!(graph.within_class_instance(pattern.print));
            if let Some(body) = bodies.body(entity) {
                assert_ne!(body, pattern.body);
                assert!(completed.insert(body));
            } else {
                assert!(graph.entity(entity).scope.is_none());
            }
        }
        assert_eq!(completed.len(), 2);
        let functions: HashSet<_> = graph
            .functions_at(graph.global, read.into())
            .map(|binding| graph.binding(binding).entity)
            .collect();
        assert_eq!(functions.len(), 2);
        // Patterns are source-visible only, with no namespace overload or ADL
        // publication. Each instance links back to those original entities.
        for &source in bodies.member_links.values() {
            assert!(!functions.contains(&source));
            assert!(graph.entity(source).scope.is_some());
        }
        let declarations = &graph.scope(graph.global).bindings;
        let variable_position = declarations
            .iter()
            .position(|&id| graph.binding(id).name == Some(global))
            .unwrap();
        let variable_type = graph.binding(declarations[variable_position]).ty.unwrap();
        let friend_position = declarations
            .iter()
            .position(|&id| {
                let binding = graph.binding(id);
                binding.name == Some(read)
                    && binding.ty.is_some_and(|ty| {
                        graph.types.function_parameter(ty, 0).unwrap() == variable_type
                    })
            })
            .unwrap();
        assert!(friend_position < variable_position);
    }

    #[test]
    fn outside_members_share_pattern_links_but_own_only_demanded_copies() {
        let source = SourceBuffer::new(
            b"template<class Original>struct Owner{int get();int bad();};\
              template<class Renamed>int Owner<Renamed>::get(){return sizeof(Renamed);}\
              template<class U>int Owner<U>::bad(){return U::missing;}\
              int main(){Owner<int>a;Owner<char>b;return a.get()+a.get()+b.get();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"outside-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let bodies = &graph.function_templates.bodies;
        assert_eq!(bodies.member_links.len(), 4);
        assert_eq!(bodies.member_definitions.len(), 2);
        assert_eq!(bodies.members.len(), 2);
        assert_eq!(bodies.states.len(), 2);
        let mut copies = HashSet::new();
        for (&entity, &(definition, _)) in &bodies.members {
            let source = bodies.member_links[&entity];
            let original = bodies.member_definitions[&source].0;
            let copy = bodies.body(entity).unwrap();
            assert!(copies.insert(copy));
            assert_ne!(copy, original.body);
            assert!(definition.definition_head.is_none());
            assert_eq!(definition.template_depth, 0);
            assert_eq!(
                graph.scope(definition.parent).kind,
                ScopeKind::TemplateParameters
            );
            assert!(
                graph.entity(entity).inline
                    == super::super::specifiers::InlinePreference::Unspecified
            );
        }
    }

    #[test]
    fn class_member_bodies_copy_only_on_demand_with_distinct_facts() {
        let source = SourceBuffer::new(
            b"template<class U>struct Box{U value;U get(){return value;}void bad(){U::missing;}};\
            int main(){Box<int> a;Box<char> b;a.value=2;b.value=3;return a.get()+a.get()+b.get();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-body-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let original_bodies = tree.bodies.mark();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let bodies = &graph.function_templates.bodies;
        assert_eq!(bodies.members.len(), 4);
        assert_eq!(bodies.demands.len(), 2);
        assert_eq!(bodies.states.len(), 2);
        assert_eq!(tree.bodies.mark(), original_bodies + 2);
        let mut seen = HashSet::new();
        for (&entity, &(pattern, _)) in &bodies.members {
            if let Some(body) = bodies.body(entity) {
                assert_ne!(pattern.body, body);
                assert!(seen.insert(body));
                assert!(graph.entity(entity).scope.is_some());
                assert!(graph.entity(entity).template_head.is_none());
            } else {
                assert!(graph.entity(entity).scope.is_none());
            }
        }
    }

    #[test]
    fn vtable_body_demand_resumes_only_new_boundaries_and_body_dependencies() {
        let source = SourceBuffer::new(b"template<class T>int helper(T){return sizeof(T);}\
            template<class T>struct Box{virtual int get(){return helper(T());}virtual int extra(){return helper(T());}};\
            template<class T>struct Unused{virtual int bad(){return T::missing;}};\
            int main(){Box<int>b;Box<int>*p=&b;return p->get();}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"virtual-member-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(graph.function_templates.bodies.members.len(), 2);
        assert_eq!(graph.function_templates.bodies.states.len(), 3);
        for &member in graph.function_templates.bodies.members.keys() {
            assert!(graph.function_templates.bodies.body(member).is_some());
        }
        let before = (tree.expressions.mark(), tree.bodies.mark());
        let mut analyzer =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(graph)).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.finish_lowering_boundaries().unwrap();
        assert_eq!(
            before,
            (
                analyzer.tree.expressions.mark(),
                analyzer.tree.bodies.mark()
            )
        );
        assert_eq!(analyzer.graph.function_templates.bodies.states.len(), 3);
        assert!(analyzer.graph.boundary_roots.is_empty());
    }

    #[test]
    fn explicit_definitions_share_signatures_but_not_body_or_emission_demand() {
        let source = SourceBuffer::new(
            b"\
            template<class T>int f(T);\
            extern template int f(int);template int f(int);\
            template<class U>int f(U x){return x;}\
            template<class T>int suppressed(T x){return T::missing;}\
            int main(){return suppressed(2)+f(1)+f(2);}\
            extern template int suppressed(int);extern template int suppressed(int);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"explicit-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expression_mark = tree.expressions.mark();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let instantiations = &graph.function_templates.instantiations;
        assert_eq!(instantiations.definitions().len(), 1);
        let definition = instantiations.definitions()[0];
        let entity = graph.binding(definition.binding).entity;
        assert!(instantiations.defined(entity));
        assert!(!graph.function_instantiation_suppressed(entity));
        assert!(graph.function_templates.bodies.body(entity).is_some());
        let suppressed: Vec<_> = graph
            .function_templates
            .bodies
            .demands
            .iter()
            .copied()
            .filter(|&id| graph.function_instantiation_suppressed(id))
            .collect();
        assert_eq!(suppressed.len(), 1);
        assert!(
            graph
                .function_templates
                .bodies
                .body(suppressed[0])
                .is_none()
        );
        assert!(!graph.entity(suppressed[0]).defined);
        assert_eq!(
            graph.lower_explicit_functions().collect::<Vec<_>>(),
            [entity]
        );
        assert_eq!(
            graph.lower_explicit_function_root(entity),
            graph.entity(entity).defined
        );
        assert!(tree.expressions.mark() > expression_mark);
    }

    #[test]
    fn explicit_roots_instantiate_once_without_demanding_unused_defaults() {
        let source = SourceBuffer::new(
            b"\
            template<class T>int f(T x,int y=T::missing){return x+y;}\
            template int f(int,int);template int f(char,int);\
            template<class T>int unused(T x){return T::missing;}\
            int main(){return f(1,2);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"explicit-roots").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let definitions = graph.function_templates.instantiations.definitions();
        assert_eq!(definitions.len(), 2);
        let first = graph.binding(definitions[0].binding).entity;
        let second = graph.binding(definitions[1].binding).entity;
        assert_ne!(first, second);
        assert_ne!(
            graph.function_templates.bodies.body(first),
            graph.function_templates.bodies.body(second)
        );
        for entity in [first, second] {
            assert!(graph.lower_explicit_function_root(entity));
            let primary = graph.function_templates.primary(entity).unwrap();
            let default = graph.entity(graph.binding(primary).entity).defaults[1].unwrap();
            assert!(graph.expression_facts.get(default.expression).is_err());
        }
        assert_eq!(graph.function_templates.bodies.states.len(), 2);
    }

    #[test]
    fn operator_candidates_demand_only_the_selected_specialization() {
        for (source, count) in [
            (
                "enum E{a,b};template<class T>bool operator==(T,T){return T::missing;}int main(){return a==b;}",
                0,
            ),
            (
                "struct A{operator int(){return 1;}};template<class T>int operator+(int,T){return T::missing;}int main(){A a;return a+1;}",
                0,
            ),
            (
                "struct A{int x;};template<class T>int operator++(T& a,int){return a.x++;}int main(){A a={1};return a++;}",
                1,
            ),
            (
                "struct A{};template<class T>int operator+(T,int x){return x;}int main(){A a;return (a+1)+(a+2);}",
                1,
            ),
        ] {
            let source = SourceBuffer::new(source.as_bytes().to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"operator-demands").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let graph = analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                true,
                true,
            )
            .unwrap();
            let bodies = &graph.function_templates.bodies;
            assert_eq!(bodies.demands.len(), count);
            assert_eq!(bodies.states.len(), count);
            for &entity in &bodies.demands {
                assert!(bodies.body(entity).is_some());
                assert!(graph.function_templates.primary(entity).is_some());
                assert!(graph.entity(entity).template_head.is_none());
            }
        }
    }

    #[test]
    fn exception_prototype_keeps_declared_const_independent_of_call_signature() {
        for qualifier in ["", "const "] {
            let text = format!(
                "template<class T>int f({qualifier}T x) noexcept(noexcept(++x)){{return x;}}int main(){{return f(1);}}"
            );
            let source = SourceBuffer::new(text.into_bytes()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"prototype-cv").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let result = analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                true,
                true,
            );
            if qualifier.is_empty() {
                let graph = result.unwrap();
                let entity = graph.function_templates.bodies.demands[0];
                let ty = graph.entity(entity).ty.unwrap();
                let parameter = graph.types.function_parameter(ty, 0).unwrap();
                assert_eq!(graph.types.get(parameter).cv.0, 0);
                assert!(matches!(
                    graph.entity(entity).exceptions,
                    super::super::exceptions::ExceptionState::Value(
                        super::super::exceptions::ExceptionSet::None
                    )
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(Error::Diagnostic {
                        message: "increment or decrement of something that is not a modifiable lvalue",
                        ..
                    })
                ));
            }
        }
    }

    #[test]
    fn demanded_copies_are_unique_and_keep_definition_context_and_ownership() {
        let source = SourceBuffer::new(
            b"\
            int g(long){return 1;}\
            template<class T>int f(T);\
            template<class U>int f(U x){U y=x;return g(y);}\
            int g(int){return 2;}\
            template<class T>int unused(T x){return x.missing();}\
            int main(){return f(1)+f(2)+f('a');}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"body-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let statements = tree.statements.mark();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let bodies = &graph.function_templates.bodies;
        assert_eq!(bodies.patterns.len(), 2);
        assert_eq!(bodies.demands.len(), 2);
        assert_eq!(bodies.states.len(), 2);
        let mut roots = HashSet::new();
        let mut scopes = HashSet::new();
        for &entity in &bodies.demands {
            let body = bodies.body(entity).unwrap();
            let ast::Body::Parsed(body) = *tree.bodies.get(body) else {
                panic!("missing parsed body")
            };
            assert!(body.compound.index() >= statements);
            assert!(roots.insert(body.compound));
            let function = graph.entity(entity);
            assert!(scopes.insert(function.scope.unwrap()));
            assert!(function.defined);
            assert_eq!(
                function.definition_emission,
                super::super::function_demands::DefinitionEmission::Demanded
            );
            let frame = graph.scope(function.scope.unwrap()).parent.unwrap();
            assert_eq!(graph.scope(frame).kind, ScopeKind::TemplateParameters);
            assert!(graph.scope(frame).bindings.is_empty());
            assert_eq!(graph.scope(frame).index.len(), 1);
            let primary = graph
                .binding(graph.function_templates.primary(entity).unwrap())
                .entity;
            let original = bodies.patterns[&primary].body.body;
            let ast::Body::Parsed(original) = *tree.bodies.get(original) else {
                panic!("missing pattern")
            };
            assert!(original.compound.index() < statements);
            for &statement in tree
                .stmt_lists
                .get(match tree.statements.get(body.compound).kind {
                    ast::StmtKind::Compound(list) => list,
                    _ => panic!("missing compound"),
                })
            {
                if let ast::StmtKind::Return(Some(id)) = tree.statements.get(statement).kind {
                    assert!(id.index() >= expressions);
                    let ast::ExprKind::Call { callee, .. } = tree.expressions.get(id).kind else {
                        panic!("missing call")
                    };
                    let binding = graph
                        .expression_facts
                        .get(callee)
                        .unwrap()
                        .function
                        .unwrap();
                    let ty = graph.binding(binding).ty.unwrap();
                    let parameter = graph.types.function_parameter(ty, 0).unwrap();
                    assert_eq!(
                        graph.types.get(parameter).kind,
                        super::super::types::TypeKind::Scalar(super::super::types::Scalar::Basic(
                            crate::types::FundamentalType::LongInt
                        ))
                    );
                }
            }
        }
        assert!(graph.lookup_limit.is_none());
        assert!(graph.boundary_roots.is_empty());
        assert!(graph.storage_layouts.is_empty());
    }

    #[test]
    fn unused_inline_body_still_demands_definitions_but_signature_uses_do_not() {
        for (source, count) in [
            (
                "template<class T>int f(T x){return x;}inline int g(){return f(1);}int main(){return 0;}",
                1,
            ),
            (
                "template<class T>int f(T x){return x;}int main(){return sizeof(f(1));}",
                0,
            ),
            (
                "template<class T>int f(T x){return x;}struct A{friend int f<>(int);};int main(){return 0;}",
                0,
            ),
            ("template<class T>int f(T);int main(){return f(1);}", 0),
            (
                "namespace N{template<class T>int f(T);}template<class U>int N::f(U x){return sizeof(U)+x;}int main(){return N::f(2);}",
                1,
            ),
        ] {
            let source = SourceBuffer::new(source.as_bytes().to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"body-demands").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let expressions = tree.expressions.mark();
            let graph = analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                true,
                true,
            )
            .unwrap();
            assert_eq!(graph.function_templates.bodies.states.len(), count);
            if count == 0 {
                assert_eq!(tree.expressions.mark(), expressions);
            }
        }
    }
}
