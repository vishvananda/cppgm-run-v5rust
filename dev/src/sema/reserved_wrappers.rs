// Translated from analyzer_declarations.cpp::wrapper_body and analyzer_bodies.cpp.
// See NOTICE.
//! Reserved calls suspend their owning full-expression, retaining name checks
//! and the ordinary typing of neighboring statements. No synthetic value escapes.
use super::{
    Error,
    analyzer::{Analyzer, DeferredBody},
    graph::{EntityId, ScopeId},
    lookup::Filter,
};
use crate::{lex::identifiers::NameId, syntax::ast};

#[derive(Clone, Copy, Default)]
pub(super) struct Context {
    pub deferring: bool,
    pub pattern: bool,
}

#[derive(Default)]
pub(super) struct Deferrals {
    bodies: crate::support::hash::HashSet<EntityId>,
    undeduced: crate::support::hash::HashSet<EntityId>,
    initializers: crate::support::hash::HashSet<EntityId>,
    expressions: crate::support::hash::HashSet<ast::ExprId>,
}
impl Deferrals {
    pub(super) fn note_undeduced(&mut self, entity: EntityId) {
        self.undeduced.insert(entity);
    }
    pub(super) fn undeduced(&self, entity: EntityId) -> bool {
        self.undeduced.contains(&entity)
    }
    pub(super) fn note_body(&mut self, entity: EntityId) {
        self.bodies.insert(entity);
    }
    pub(super) fn body(&self, entity: EntityId) -> bool {
        self.bodies.contains(&entity)
    }
    pub(super) fn note_initializer(&mut self, entity: EntityId) {
        self.initializers.insert(entity);
    }
    pub(super) fn initializer(&self, entity: EntityId) -> bool {
        self.initializers.contains(&entity)
    }
    pub(super) fn note_expression(&mut self, id: ast::ExprId) {
        self.expressions.insert(id);
    }
    pub(super) fn expression(&self, id: ast::ExprId) -> bool {
        self.expressions.contains(&id)
    }
}

impl Analyzer<'_> {
    pub(super) fn reserved_wrapper(&mut self, body: DeferredBody) -> Result<bool, Error> {
        let entity = self.graph.entity(body.entity);
        let closure = entity
            .home
            .and_then(|home| self.graph.scope(home).owner)
            .is_some_and(|class| self.graph.closure_call(class) == Some(body.entity));
        if closure {
            return Ok(self.reserved_context.deferring);
        }
        if !self.semantic()
            || body.template_depth == 0
                && !self
                    .graph
                    .function_templates
                    .specializations
                    .contains(body.entity)
                && (self.graph.function_templates.primary(body.entity).is_some()
                    || self.graph.function_templates.bodies.member(body.entity))
            || body.template_depth != 0 && entity.member_function.is_none()
            || self.graph.entity(body.entity).inline != super::specifiers::InlinePreference::Inline
        {
            return Ok(false);
        }
        let ast::Body::Parsed(parsed) = self.tree.bodies.get(body.body) else {
            return Ok(false);
        };
        let Some(identifier) = parsed.reserved_call else {
            return Ok(false);
        };
        Ok(self
            .unqualified(body.parent, identifier, Filter::Ordinary)?
            .is_none())
    }

    pub(super) fn deferred_builtin_name(&self, name: ast::Name) -> Option<NameId> {
        let node = self.tree.names.get(name);
        if node.global {
            return None;
        }
        let [component] = self.tree.name_lists.get(node.components) else {
            return None;
        };
        if component.arguments.is_some() {
            return None;
        }
        let ast::NameKind::Identifier(identifier) = component.kind else {
            return None;
        };
        self.identifiers
            .bytes(identifier)
            .starts_with(b"__builtin_")
            .then_some(identifier)
    }

    pub(super) fn reserved_constructor_plan(
        &mut self,
        entity: EntityId,
        parsed: &ast::FunctionBody,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let unevaluated = self.unevaluated;
        let initializing = self.initializing;
        let declarator = self.initializing_declarator;
        let base = self.base_object;
        let member = self.member_initializer_class;
        match self.written_constructor_plan(entity, parsed, scope) {
            Err(Error::DeferredBuiltin) if self.reserved_context.deferring => {
                self.unevaluated = unevaluated;
                self.initializing = initializing;
                self.initializing_declarator = declarator;
                self.base_object = base;
                self.member_initializer_class = member;
                self.mark_reserved_body();
                let saved = self.reserved_context.pattern;
                self.reserved_context.pattern = true;
                let result = (|| {
                    let ty = self
                        .graph
                        .entity(entity)
                        .ty
                        .ok_or(Error::Invariant("constructor type"))?;
                    for index in 0..self.tree.mem_initializers.get(parsed.initializers).len() {
                        let initializer =
                            self.tree.mem_initializers.get(parsed.initializers)[index];
                        self.initializer_names(initializer.initializer, scope, ty)?;
                    }
                    Ok(())
                })();
                self.reserved_context.pattern = saved;
                result.map(|()| false)
            }
            result => result.map(|()| true),
        }
    }

    pub(super) fn note_deferred_initializer(&mut self, initializer: ast::Initializer) {
        match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                self.graph.reserved_deferrals.note_expression(id);
            }
            ast::InitializerKind::Parenthesized(arguments) => {
                for &id in self.tree.expr_lists.get(arguments) {
                    self.graph.reserved_deferrals.note_expression(id);
                }
            }
            ast::InitializerKind::Default | ast::InitializerKind::Delete => {}
        }
    }

    pub(super) fn check_deferred_constant(&self, id: ast::ExprId) -> Result<(), Error> {
        let untyped_body_expression = self.graph.expression_facts.known(id).is_none()
            && matches!(self.constexpr_frames.current_call().and_then(|frame| frame.source_function),
                Some(super::predefined_names::SourceFunction::Function(function))
                    if self.graph.reserved_deferrals.body(function));
        if self.graph.reserved_deferrals.expression(id) || untyped_body_expression {
            let mut operand = id;
            while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(operand).kind {
                operand = inner;
            }
            return Err(super::constant_addresses::nonconstant(
                self.tree.expressions.get(operand).location,
                "an expression outside constant evaluation",
            ));
        }
        Ok(())
    }

    pub(super) fn mark_reserved_body(&mut self) {
        if let Some(function) = self.body_context.as_ref().and_then(|body| body.function) {
            self.graph.reserved_deferrals.note_body(function);
        }
    }

    pub(super) fn reserved_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let unevaluated = self.unevaluated;
        match self.expression(id, scope) {
            Err(Error::DeferredBuiltin) if self.reserved_context.deferring => {
                self.unevaluated = unevaluated;
                self.mark_reserved_body();
                self.graph.reserved_deferrals.note_expression(id);
                let saved = self.reserved_context.pattern;
                self.reserved_context.pattern = true;
                let result = self.expression(id, scope);
                self.reserved_context.pattern = saved;
                result.map(|()| false)
            }
            result => result.map(|()| true),
        }
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
    fn deferral_keeps_syntax_and_typed_neighbors_without_a_fabricated_value() {
        let source = SourceBuffer::new(
            b"inline int f(){int x=__builtin_unprovided();return x;}int g(){return 1;}".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"reserved-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let marks = (
            tree.bodies.mark(),
            tree.expressions.mark(),
            tree.statements.mark(),
        );
        let f = cursor.intern_name(b"f").unwrap();
        let g = cursor.intern_name(b"g").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        for &declaration in &analyzer.tree.decl_lists.get(declarations).to_vec() {
            analyzer
                .declaration(declaration, analyzer.graph.global)
                .unwrap();
        }
        let f = analyzer
            .graph
            .own(analyzer.graph.global, f, Filter::Ordinary)
            .unwrap();
        let g = analyzer
            .graph
            .own(analyzer.graph.global, g, Filter::Ordinary)
            .unwrap();
        assert!(
            analyzer
                .graph
                .reserved_deferrals
                .body(analyzer.graph.binding(f).entity)
        );
        assert!(
            !analyzer
                .graph
                .reserved_deferrals
                .body(analyzer.graph.binding(g).entity)
        );
        for &id in &analyzer.graph.reserved_deferrals.expressions {
            assert!(analyzer.graph.expression_facts.known(id).is_none());
        }
        assert_eq!(
            marks,
            (
                analyzer.tree.bodies.mark(),
                analyzer.tree.expressions.mark(),
                analyzer.tree.statements.mark()
            )
        );
        assert!(analyzer.body_context.is_none());
        assert!(!analyzer.reserved_context.deferring);
        assert!(!analyzer.reserved_context.pattern);
        assert!(analyzer.expression_work.is_empty());
        assert!(analyzer.binary_spine.is_empty());
    }

    #[test]
    fn a_neighbor_error_restores_the_enclosing_analysis_context() {
        let source =
            SourceBuffer::new(b"inline void f(){__builtin_unprovided();missing();}".to_vec())
                .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"reserved-error").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let declaration = analyzer.tree.decl_lists.get(declarations)[0];
        assert!(matches!(
            analyzer.declaration(declaration, analyzer.graph.global),
            Err(Error::Diagnostic {
                message: "undeclared function name",
                ..
            })
        ));
        assert!(analyzer.body_context.is_none());
        assert!(!analyzer.reserved_context.deferring);
        assert!(!analyzer.reserved_context.pattern);
        assert_eq!(analyzer.unevaluated, 0);
        assert!(analyzer.expression_work.is_empty());
        assert!(analyzer.binary_spine.is_empty());
    }
}
