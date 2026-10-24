// Translated from sema/analyzer_initialization.cpp::designated_list. See NOTICE.
//! Class-specific list ordering retains source operands and canonical union selections.
use super::{
    Error,
    aggregate_initialization::AggregateInitialization,
    analyzer::Analyzer,
    graph::{EntityId, ScopeId},
    lookup::Filter,
    types::TypeKind,
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::{
        ast::{self, ExprKind},
        storage::List,
    },
};

impl Analyzer<'_> {
    // initialize_class rejects these after aggregate selection, before any
    // initializer-list or ordinary constructor is demanded. Ranked arguments
    // reach the same check only after overload selection.
    pub(super) fn check_constructor_designators(
        &self,
        list: Option<ast::ExprId>,
    ) -> Result<(), Error> {
        if let Some(id) = list
            && let ExprKind::Braced { designators, .. } = self.tree.expressions.get(id).kind
            && !designators.is_empty()
        {
            return Err(Error::UnconditionalDiagnostic {
                location: self.tree.expressions.get(id).location,
                message: "designated initializers of a class that is no aggregate",
            });
        }
        Ok(())
    }

    pub(super) fn designated_list(
        &mut self,
        id: ast::ExprId,
        class: EntityId,
        scope: Option<ScopeId>,
    ) -> Result<Option<AggregateInitialization>, Error> {
        self.check_stack()?;
        if let Some(plan) = self.graph.expression_facts.designated_list(id, class) {
            return Ok(Some(plan));
        }
        let node = *self.tree.expressions.get(id);
        let ExprKind::Braced {
            elements,
            designators,
            ..
        } = node.kind
        else {
            return Err(Error::Invariant("designated initialization without list"));
        };
        if designators.is_empty() {
            return Ok(Some(elements.into()));
        }
        let mut ordered = Vec::new();
        let mut member_index = 0;
        let mut index = 0;
        let mut selection = None;
        let union = self.graph.entity(class).key == Keyword::Union;
        while index < self.tree.expr_lists.get(elements).len() {
            let name = self
                .tree
                .designators
                .get(designators)
                .get(index)
                .copied()
                .flatten();
            let mut element = self.tree.expr_lists.get(elements)[index];
            let location = self.tree.expressions.get(element).location;
            let found = name.and_then(|name| self.designated_member(class, member_index, name));
            let problem = if name.is_none() {
                Some("a designated list with an element without a designator")
            } else if union && index != 0 {
                Some("a designated list of a union with several elements")
            } else if found.is_none() {
                Some(
                    "a designator that names no later direct non-static data member (declaration order)",
                )
            } else {
                None
            };
            if let Some(problem) = problem {
                return if scope.is_some() {
                    Err(Error::at(location, problem))
                } else {
                    Ok(None)
                };
            }
            let (found, anonymous) = found.ok_or(Error::Invariant("designated member"))?;
            let member = self.graph.entity(class).members[found];
            if !union {
                for at in member_index..found {
                    let omitted = self.graph.entity(class).members[at];
                    if !self.graph.unnamed_bit_field(omitted) {
                        ordered.push(self.designated_braces(&[], &[], location)?);
                    }
                }
            }
            if let Some(anonymous) = anonymous {
                let mut end = index + 1;
                while end < self.tree.expr_lists.get(elements).len()
                    && self
                        .tree
                        .designators
                        .get(designators)
                        .get(end)
                        .copied()
                        .flatten()
                        .is_some_and(|name| self.anonymous_holds(anonymous, name))
                {
                    end += 1;
                }
                let elements = elements
                    .range(index..end)
                    .map_err(|_| Error::Invariant("designated member element range"))?;
                let designators = designators
                    .range(index..end)
                    .map_err(|_| Error::Invariant("designated member name range"))?;
                element = self
                    .tree
                    .expressions
                    .add(ast::Expression {
                        location,
                        kind: ExprKind::Braced {
                            elements,
                            designators,
                            trailing_comma: false,
                        },
                    })
                    .map_err(|_| Error::Limit("designated initializer storage"))?;
                index = end - 1;
            } else {
                let ty = self.graph.lower_entity_type(member)?;
                let subaggregate = match self.graph.types.get(ty).kind {
                    TypeKind::Array { .. } => true,
                    TypeKind::Class(child) => self.graph.aggregate_class(child),
                    _ => false,
                };
                if subaggregate
                    && !matches!(
                        self.tree.expressions.get(element).kind,
                        ExprKind::Braced { .. }
                    )
                {
                    if let Some(scope) = scope {
                        self.ensure_typed(element, scope)?;
                    }
                    if !self.initializes_subaggregate(ty, element)? {
                        element = self.designated_braces(&[element], &[], location)?;
                    }
                }
            }
            ordered.push(element);
            if union {
                selection = Some(member);
            }
            member_index = found + 1;
            index += 1;
        }
        let start = self.tree.expr_lists.begin();
        for element in ordered {
            self.tree.expr_lists.push(element);
        }
        let plan = AggregateInitialization {
            elements: self
                .tree
                .expr_lists
                .finish(start)
                .map_err(|_| Error::Limit("designated initializer storage"))?,
            union_member: selection,
        };
        self.graph
            .expression_facts
            .set_designated_list(id, class, plan);
        Ok(Some(plan))
    }

    fn designated_member(
        &self,
        class: EntityId,
        start: usize,
        name: NameId,
    ) -> Option<(usize, Option<EntityId>)> {
        for (index, &member) in self
            .graph
            .entity(class)
            .members
            .iter()
            .enumerate()
            .skip(start)
        {
            if self.graph.entity(member).name == Some(name) {
                return Some((index, None));
            }
            if self.graph.entity(member).name.is_none()
                && !self.graph.unnamed_bit_field(member)
                && let Some(child) = self
                    .graph
                    .entity(member)
                    .ty
                    .and_then(|ty| self.graph.class_of(ty))
                && self.graph.anonymous_member_storage(child).is_some()
                && self.anonymous_holds(child, name)
            {
                return Some((index, Some(child)));
            }
        }
        None
    }
    fn anonymous_holds(&self, class: EntityId, name: NameId) -> bool {
        self.graph
            .entity(class)
            .scope
            .and_then(|scope| self.graph.own(scope, name, Filter::Ordinary))
            .is_some_and(|binding| {
                self.graph
                    .entity(self.graph.binding(binding).entity)
                    .data_member
                    .is_some()
            })
    }
    fn designated_braces(
        &mut self,
        elements: &[ast::ExprId],
        designators: &[Option<NameId>],
        location: SourceOffset,
    ) -> Result<ast::ExprId, Error> {
        let start = self.tree.expr_lists.begin();
        for &element in elements {
            self.tree.expr_lists.push(element);
        }
        let elements = self
            .tree
            .expr_lists
            .finish(start)
            .map_err(|_| Error::Limit("designated initializer storage"))?;
        let start = self.tree.designators.begin();
        for &name in designators {
            self.tree.designators.push(name);
        }
        let designators = if designators.is_empty() {
            List::default()
        } else {
            self.tree
                .designators
                .finish(start)
                .map_err(|_| Error::Limit("designated initializer storage"))?
        };
        self.tree
            .expressions
            .add(ast::Expression {
                location,
                kind: ExprKind::Braced {
                    elements,
                    designators,
                    trailing_comma: false,
                },
            })
            .map_err(|_| Error::Limit("designated initializer storage"))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, expression_facts::ClassInitialization},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{
            SyntaxUnit,
            ast::{DeclKind, ExprKind, InitializerKind},
        },
    };

    #[test]
    fn candidate_lists_keep_source_operands_and_only_the_winner_publishes_initialization() {
        let source = SourceBuffer::new(
            b"struct A{int a,b;};struct B{void* b;};\
            int pick(A);int pick(B);int n=pick({.b=7});"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"designated-candidates").unwrap());
        unit.set_hosted(true);
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let declaration = *tree.decl_lists.get(declarations).last().unwrap();
        let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind else {
            panic!("variable")
        };
        let InitializerKind::Assign(call) = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap()
            .kind
        else {
            panic!("call initializer")
        };
        let ExprKind::Call { arguments, .. } = tree.expressions.get(call).kind else {
            panic!("call")
        };
        let list = tree.expr_lists.get(arguments)[0];
        let ExprKind::Braced {
            elements,
            designators,
            ..
        } = tree.expressions.get(list).kind
        else {
            panic!("list")
        };
        let original = tree.expr_lists.get(elements)[0];
        assert_eq!(tree.designators.get(designators).len(), 1);
        let before = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let ClassInitialization::Aggregate(selected) = graph
            .expression_facts
            .get(list)
            .unwrap()
            .class_initialization
            .unwrap()
        else {
            panic!("selected aggregate")
        };
        let ordered = tree.expr_lists.get(selected);
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[1], original);
        assert!(
            graph
                .expression_facts
                .get(original)
                .unwrap()
                .class_initialization
                .is_none()
        );
        assert_eq!(tree.expressions.mark(), before + 1);
        assert!(
            matches!(tree.expressions.get(ordered[0]).kind, ExprKind::Braced { elements, .. } if elements.is_empty())
        );
    }
}
