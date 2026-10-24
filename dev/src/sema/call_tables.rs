// Translated from lower/expression_lowering.cpp::call/call_object. See NOTICE.
//! The course's table demand observes the last member receiver evaluated in a
//! call's arguments. Retain that effect on typed occurrences, without demanding
//! bodies from a receiver that a nested call replaces.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::CallForm,
    graph::{BindingId, EntityId, Graph},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    syntax::{
        ast::{self, ExprKind, InitializerKind, Tree},
        storage::List,
    },
};

#[derive(Clone, Copy, Debug)]
pub(super) enum ObjectEffect {
    Preserve,
    Set(Option<EntityId>),
}
enum Step {
    Visit(ast::ExprId),
    Finish(ast::ExprId, usize),
    Set(Option<EntityId>),
}
impl Analyzer<'_> {
    pub(super) fn call_table_class(
        &mut self,
        callee: ast::ExprId,
        binding: BindingId,
        arguments: List<ast::ExprId>,
        callable: bool,
    ) -> Result<Option<EntityId>, Error> {
        let mut class = if callable {
            match self.graph.call_object_effect(self.tree, callee)? {
                ObjectEffect::Set(class) => class,
                ObjectEffect::Preserve => None,
            }
        } else {
            self.graph.lower_receiver_class(callee)?
        };
        let function = self.graph.binding(binding).entity;
        let ty = self.graph.lower_function(function)?.ty;
        let super::lowering::TypeView::Function { parameters, .. } = self.graph.lower_type(ty)?
        else {
            return Err(Error::Invariant("table demand without function type"));
        };
        let supplied = self.tree.expr_lists.get(arguments);
        for index in 0..parameters.len().max(supplied.len()) {
            if let Some(argument) = supplied
                .get(index)
                .copied()
                .or_else(|| self.graph.lower_default(function, index))
                && let ObjectEffect::Set(last) =
                    self.graph.call_object_effect(self.tree, argument)?
            {
                class = last;
            }
        }
        Ok(class)
    }
}
impl Graph {
    fn call_object_effect(
        &mut self,
        tree: &Tree,
        root: ast::ExprId,
    ) -> Result<ObjectEffect, Error> {
        let mut work = vec![Step::Visit(root)];
        let mut values = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                Step::Set(class) => values.push(ObjectEffect::Set(class)),
                Step::Finish(id, first) => {
                    let effect = values[first..]
                        .iter()
                        .rev()
                        .copied()
                        .find(|effect| matches!(effect, ObjectEffect::Set(_)))
                        .unwrap_or(ObjectEffect::Preserve);
                    values.truncate(first);
                    values.push(effect);
                    self.expression_facts.set_call_object_effect(id, effect);
                }
                Step::Visit(id) => {
                    if let Some(effect) = self.expression_facts.call_object_effect(id) {
                        values.push(effect);
                        continue;
                    }
                    work.push(Step::Finish(id, values.len()));
                    // A scalar's braced initializer is syntax only; its
                    // elements own the expression facts and are evaluated.
                    if let ExprKind::Braced { elements, .. } = tree.expressions.get(id).kind {
                        work.extend(
                            tree.expr_lists
                                .get(elements)
                                .iter()
                                .rev()
                                .copied()
                                .map(Step::Visit),
                        );
                        continue;
                    }
                    let facts = self.expression_facts.get(id)?;
                    if facts.folded.is_some() || facts.query_value.is_some() {
                        if let ExprKind::Member { base, .. } = tree.expressions.get(id).kind {
                            work.push(Step::Visit(base));
                        }
                        continue;
                    }
                    self.queue_call_object_effect(tree, id, &mut work)?;
                }
            }
        }
        values
            .pop()
            .ok_or(Error::Invariant("missing call object effect"))
    }
    fn queue_call_object_effect(
        &self,
        tree: &Tree,
        id: ast::ExprId,
        work: &mut Vec<Step>,
    ) -> Result<(), Error> {
        match tree.expressions.get(id).kind {
            ExprKind::Paren(inner)
            | ExprKind::PackExpansion(inner)
            | ExprKind::ComplexPart { operand: inner, .. }
            | ExprKind::Unary { operand: inner, .. }
            | ExprKind::Postfix { operand: inner, .. }
            | ExprKind::VaArg { operand: inner, .. }
            | ExprKind::Cast { operand: inner, .. }
            | ExprKind::Delete { operand: inner, .. }
            | ExprKind::Member { base: inner, .. } => work.push(Step::Visit(inner)),
            ExprKind::Binary {
                left,
                right,
                assignment,
                op,
            } => {
                if assignment && op.token.as_punctuator() == Some(P::Assign) {
                    work.extend([Step::Visit(left), Step::Visit(right)]);
                } else {
                    let truth = self.lower_literal_truth(tree, left)?;
                    let skipped = matches!(
                        (op.token.as_punctuator(), truth),
                        (Some(P::LogicalAnd), Some(false)) | (Some(P::LogicalOr), Some(true))
                    );
                    if !skipped {
                        work.push(Step::Visit(right));
                    }
                    work.push(Step::Visit(left));
                }
            }
            ExprKind::Subscript { base, index } => {
                work.extend([Step::Visit(index), Step::Visit(base)]);
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let selected = if self.class_of(self.expression_facts.get(id)?.ty).is_none() {
                    self.lower_constant_conversion(tree, condition)?
                } else {
                    None
                };
                if let Some(value) = selected {
                    work.push(Step::Visit(if value { then_expr } else { else_expr }));
                } else {
                    work.extend([Step::Visit(else_expr), Step::Visit(then_expr)]);
                }
                work.push(Step::Visit(condition));
            }
            ExprKind::Call { callee, arguments } => {
                if self.lower_destructor_evaluation_only(id)? {
                    let mut callee = callee;
                    while let ExprKind::Paren(inner) = tree.expressions.get(callee).kind {
                        callee = inner;
                    }
                    let ExprKind::Member { base, .. } = tree.expressions.get(callee).kind else {
                        return Err(Error::Invariant("destructor effect without member"));
                    };
                    work.push(Step::Visit(base));
                    return Ok(());
                }
                let supplied = tree.expr_lists.get(arguments);
                if let Some(super::function_resolution::CallTarget::Direct(binding)) =
                    self.expression_facts.get(id)?.call
                {
                    let function = self.binding(binding).entity;
                    let facts = self.lower_function(function)?;
                    let super::lowering::TypeView::Function { parameters, .. } =
                        self.lower_type(facts.ty)?
                    else {
                        return Err(Error::Invariant("call effect without signature"));
                    };
                    for index in (supplied.len()..parameters.len()).rev() {
                        if let Some(argument) = self.lower_default(function, index) {
                            work.push(Step::Visit(argument));
                        }
                    }
                    work.extend(supplied.iter().rev().copied().map(Step::Visit));
                    if facts.object.is_some() && self.lower_closure_invocation(id).is_none() {
                        if self.expression_facts.get(id)?.call_form == CallForm::CallableObject {
                            // call_object clears the receiver before it
                            // evaluates a callable object's expression.
                            work.push(Step::Visit(callee));
                            work.push(Step::Set(None));
                        } else {
                            work.push(Step::Set(self.lower_receiver_class(callee)?));
                            work.push(Step::Visit(callee));
                        }
                    } else {
                        work.push(Step::Visit(callee));
                    }
                } else {
                    // An indirect callee is evaluated after its arguments.
                    // A bound member pointer evaluates its object before the
                    // arguments, then the pointer value after them.
                    if self.lower_bound_member_pointer(callee)? {
                        let mut access = callee;
                        while let ExprKind::Paren(inner) = tree.expressions.get(access).kind {
                            access = inner;
                        }
                        let ExprKind::Binary { left, right, .. } =
                            tree.expressions.get(access).kind
                        else {
                            return Err(Error::Invariant(
                                "bound call effect without member pointer",
                            ));
                        };
                        work.push(Step::Visit(right));
                        work.extend(supplied.iter().rev().copied().map(Step::Visit));
                        work.push(Step::Visit(left));
                    } else {
                        work.push(Step::Visit(callee));
                        work.extend(supplied.iter().rev().copied().map(Step::Visit));
                    }
                }
            }
            ExprKind::Braced { elements, .. } => work.extend(
                tree.expr_lists
                    .get(elements)
                    .iter()
                    .rev()
                    .copied()
                    .map(Step::Visit),
            ),
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if let Some(braced) = braced {
                    work.push(Step::Visit(braced));
                } else {
                    work.extend(
                        tree.expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .copied()
                            .map(Step::Visit),
                    );
                }
            }
            ExprKind::New {
                placement,
                initializer,
                ..
            } => {
                if let Some(initializer) = initializer {
                    match initializer.kind {
                        InitializerKind::Assign(id) | InitializerKind::Braced(id) => {
                            work.push(Step::Visit(id));
                        }
                        InitializerKind::Parenthesized(arguments) => work.extend(
                            tree.expr_lists
                                .get(arguments)
                                .iter()
                                .rev()
                                .copied()
                                .map(Step::Visit),
                        ),
                        InitializerKind::Default | InitializerKind::Delete => {}
                    }
                }
                if let Some(placement) = placement {
                    work.extend(
                        tree.expr_lists
                            .get(placement)
                            .iter()
                            .rev()
                            .copied()
                            .map(Step::Visit),
                    );
                }
                // The array's dynamic size is evaluated before placement
                // operands; a constant size evaluates no count expression.
                if let Some(array) = self.expression_facts.allocation(id)?.array
                    && array.constant.is_none()
                {
                    work.push(Step::Visit(array.count));
                }
            }
            ExprKind::TypeTrait {
                keyword: Keyword::Typeid,
                operand: Some(operand),
                ..
            } if self
                .expression_facts
                .type_query(id)
                .is_some_and(|query| query.dynamic) =>
            {
                work.push(Step::Visit(operand));
            }
            ExprKind::Throw(Some(operand)) => work.push(Step::Visit(operand)),
            ExprKind::Fold { .. } => return Err(Error::Invariant("unexpanded fold call effect")),
            ExprKind::Statement(_)
            | ExprKind::Lambda(_)
            | ExprKind::IntegerPack(_)
            | ExprKind::SizeofPack(_)
            | ExprKind::Id(_)
            | ExprKind::Literal(_)
            | ExprKind::Keyword(_)
            | ExprKind::Sizeof { .. }
            | ExprKind::TypeTrait { .. }
            | ExprKind::Typeof { .. }
            | ExprKind::TypeTransform { .. }
            | ExprKind::TypePredicate { .. }
            | ExprKind::OffsetOf { .. }
            | ExprKind::OffsetOfBase(_)
            | ExprKind::Throw(None) => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };
    #[test]
    fn nested_receiver_does_not_instantiate_an_unemitted_table_body() {
        let source = SourceBuffer::new(b"template<class T>struct R{virtual int extra(){return T::missing();}};struct D:virtual R<int>{virtual int f(int){return 1;}};struct C{int g(){return 2;}};int use(D& p,C& q){return p.f(q.g());}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"nested-call-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"D").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Elaborated).unwrap())
            .entity;
        let extra = graph.lower_virtual_slots(class).unwrap()[0]
            .unwrap()
            .function;
        assert!(!graph.function_templates.bodies.started(extra));
    }
}
