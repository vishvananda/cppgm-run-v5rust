// Translated from analyzer_construction.cpp::class_prvalue and analyzer_expressions.cpp.
// See NOTICE.
//! Class casts retain cv-qualified results and selected initialization on the original node.
use super::{
    Error,
    analyzer::Analyzer,
    constructor_initialization::{ClassInput, ConstructorArguments},
    expression_facts::{ClassInitialization, ExprFacts, ValueCategory},
    graph::ScopeId,
    types::{TypeId, TypeKind},
};
use crate::syntax::ast;

impl Analyzer<'_> {
    pub(super) fn class_prvalue(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let TypeKind::Class(class) = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("class prvalue without class"));
        };
        if !self.graph.entity(class).complete || self.graph.entity(class).scope.is_none() {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "initialization of an incomplete class object",
            ));
        }
        if let ast::InitializerKind::Parenthesized(arguments) = initializer.kind
            && !arguments.is_empty()
            && let TypeKind::Class(class) = self.graph.types.get(target).kind
            && self.graph.aggregate_class(class)
            && self.parenthesized_aggregate(class, arguments, scope)?
        {
            let saved = std::mem::take(&mut self.parenthesized_elements);
            self.parenthesized_elements
                .extend(self.tree.expr_lists.get(arguments));
            let mut next = 0;
            let result =
                self.aggregate_members(arguments, &mut next, class, scope, initializer.location);
            self.parenthesized_elements = saved;
            result?;
            if let Some(&extra) = self.tree.expr_lists.get(arguments).get(next) {
                return Err(Error::at(
                    self.tree.expressions.get(extra).location,
                    "too many initializers for the aggregate",
                ));
            }
            let mut facts = self.new_expression_facts(target, ValueCategory::Prvalue)?;
            facts.class_initialization = Some(ClassInitialization::Aggregate(arguments));
            return Ok(facts);
        }
        let selected = self.initialize_class_object(
            target,
            initializer,
            scope,
            self.tree.expressions.get(id).location,
        )?;
        let mut facts = self.new_expression_facts(target, ValueCategory::Prvalue)?;
        if let Some((constructor, arguments)) = selected {
            facts.constructor = Some(constructor);
            facts.class_initialization = Some(ClassInitialization::Constructed(arguments));
        } else {
            facts.class_initialization = Some(match initializer.kind {
                ast::InitializerKind::Braced(list) => {
                    let ast::ExprKind::Braced { elements, .. } =
                        self.tree.expressions.get(list).kind
                    else {
                        return Err(Error::Invariant("class prvalue aggregate list"));
                    };
                    ClassInitialization::Aggregate(elements)
                }
                ast::InitializerKind::Parenthesized(arguments) => ClassInitialization::Elided(
                    *self
                        .tree
                        .expr_lists
                        .get(arguments)
                        .first()
                        .ok_or(Error::Invariant("elided class prvalue without source"))?,
                ),
                _ => return Err(Error::Invariant("class prvalue initialization shape")),
            });
        }
        Ok(facts)
    }
    fn parenthesized_aggregate(
        &mut self,
        class: super::graph::EntityId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let args = self.tree.expr_lists.get(arguments);
        if args.len() > 1 {
            return Ok(true);
        }
        let first = args[0];
        self.type_call_argument(first, scope)?;
        if matches!(
            self.tree.expressions.get(first).kind,
            ast::ExprKind::Braced { .. }
        ) {
            return Ok(false);
        }
        let facts = self.graph.expression_facts.get(first)?;
        let Some(source) = self.graph.class_of(facts.ty) else {
            return Ok(true);
        };
        if class == source || self.graph.is_base_of(class, source) {
            return Ok(false);
        }
        if self.graph.entity(source).declared_conversions.is_empty() {
            return Ok(true);
        }
        Err(Error::Missing(
            "parenthesized aggregate user conversion ranking",
        ))
    }
    pub(super) fn explicit_class_prvalue(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let TypeKind::Class(class) = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("class cast without class"));
        };
        let location = self.tree.expressions.get(id).location;
        if !self.graph.entity(class).complete || self.graph.entity(class).scope.is_none() {
            return Err(Error::at(location, "a cast to an incomplete class type"));
        }
        let selected = self.initialize_class_arguments(
            target,
            ClassInput {
                arguments: ConstructorArguments::Single(operand),
                list: None,
                converting: false,
                copy_list: false,
            },
            scope,
            location,
        )?;
        let mut facts = self.new_expression_facts(target, ValueCategory::Prvalue)?;
        if let Some((constructor, arguments)) = selected {
            facts.constructor = Some(constructor);
            facts.class_initialization = Some(ClassInitialization::Constructed(arguments));
        } else {
            facts.class_initialization = Some(ClassInitialization::Elided(operand));
        }
        Ok(facts)
    }
}
