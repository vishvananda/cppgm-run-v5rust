// Translated from class_templates.cpp::instantiate_definition and analyzer.cpp. See NOTICE.
//! Unit-tail body worklist, independent of signature and emission demand.
use super::{
    Analyzer, Error, Graph,
    analyzer::DeferredBody,
    graph::{Binding, BindingKind, EntityId, ScopeKind},
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
    Complete(ast::BodyId),
}
#[derive(Default)]
pub(super) struct Bodies {
    patterns: HashMap<EntityId, Pattern>,
    states: HashMap<EntityId, State>,
    demands: Vec<EntityId>,
    seen: HashSet<EntityId>,
}
impl Bodies {
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

/// The driver gives semantic analysis exclusive access to append instantiated
/// nodes between analysis sessions. Sessions read ordinary slab IDs directly;
/// no expression read redirects through a template pattern or an overlay map.
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
    // Instantiating one demanded definition can append more demands. Each
    // canonical entity enters this queue once, in first-use order.
    while let Some(&entity) = graph.function_templates.bodies.demands.get(next) {
        next += 1;
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
        analyzer.instantiate_body(entity, primary, pattern, parameters, body, exception)?;
        graph = analyzer.finish_analysis(false)?;
        graph
            .function_templates
            .bodies
            .states
            .insert(entity, State::Complete(body));
    }
    // Boundary work from every session stays on the unit and is visited once,
    // after the worklist has installed all demanded body facts.
    if lowering {
        let mut analyzer = Analyzer::with_graph(tree, cursor, budget, Some(graph))?;
        analyzer.set_lowering();
        analyzer.stack_base = base;
        return analyzer.finish_analysis(true);
    }
    Ok(graph)
}

fn copy_error(error: &ParseError) -> Error {
    match error {
        ParseError::Limit(message) => Error::Limit(message),
        ParseError::NotImplemented(group) => Error::Missing(group),
        _ => Error::Invariant("invalid parsed template body copy"),
    }
}
impl Analyzer<'_> {
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
    fn template_body_frame(
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
