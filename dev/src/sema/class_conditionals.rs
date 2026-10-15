// Translated from analyzer_conversions.cpp::conditional_match/converted_conditional.
// See NOTICE.
//! Conditional class glvalues retain the selected subobject route on the original use.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::ScopeId,
    types::RefKind,
};
use crate::syntax::ast;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Match {
    None,
    Glvalue,
    Prvalue,
}

impl Analyzer<'_> {
    fn conditional_class_match(
        &mut self,
        from: ExprFacts,
        other: ExprFacts,
    ) -> Result<Match, Error> {
        if other.category != V::Prvalue && from.category == other.category {
            let kind = if other.category == V::Lvalue {
                RefKind::Lvalue
            } else {
                RefKind::Rvalue
            };
            let reference = self.graph.types.reference(other.ty, kind)?;
            if self
                .scalar_sequence(from, reference, false)?
                .is_some_and(|binding| !binding.temporary)
            {
                return Ok(Match::Glvalue);
            }
        }
        // A conversion function may supply a reference before the related-class
        // prvalue rule. Keep that responsibility explicit until its owner is ported.
        if let Some(class) = self.graph.class_of(from.ty) {
            let mut work = vec![class];
            let mut seen = std::collections::HashSet::new();
            while let Some(class) = work.pop() {
                if !seen.insert(class) {
                    continue;
                }
                let entity = self.graph.entity(class);
                if !entity.declared_conversions.is_empty() {
                    return Err(Error::Missing("conditional class conversion functions"));
                }
                work.extend(entity.bases.iter().map(|base| base.class));
            }
        }
        if let Some((from_class, to_class)) = self
            .graph
            .class_of(from.ty)
            .zip(self.graph.class_of(other.ty))
            && (from_class == to_class
                || self.graph.is_base_of(from_class, to_class)
                || self.graph.is_base_of(to_class, from_class))
        {
            return Ok(
                if (from_class == to_class || self.graph.is_base_of(to_class, from_class))
                    && self.type_cv(from.ty).0 & !self.type_cv(other.ty).0 == 0
                {
                    Match::Prvalue
                } else {
                    Match::None
                },
            );
        }
        Ok(Match::None)
    }

    pub(super) fn class_conditional(
        &mut self,
        id: ast::ExprId,
        facts: &[ExprFacts; 2],
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let [a, b] = *facts;
        if self.bare_type(a.ty)? == self.bare_type(b.ty)? {
            return Err(Error::Missing("class conditional copy initialization"));
        }
        if !self
            .graph
            .class_of(a.ty)
            .zip(self.graph.class_of(b.ty))
            .is_some_and(|(a, b)| self.graph.is_base_of(a, b) || self.graph.is_base_of(b, a))
        {
            return Err(Error::Missing("conditional unrelated class conversions"));
        }
        let matches = [
            self.conditional_class_match(a, b)?,
            self.conditional_class_match(b, a)?,
        ];
        let location = self.tree.expressions.get(id).location;
        let selected = match matches {
            [Match::None, Match::None] => {
                return Err(Error::at(location, "incompatible operands of ?:"));
            }
            [Match::None, _] => 1,
            [_, Match::None] => 0,
            _ => {
                return Err(Error::at(
                    location,
                    "each operand of ?: converts to the other's type",
                ));
            }
        };
        if matches[selected] == Match::Prvalue {
            return Err(Error::Missing("conditional base class copy initialization"));
        }
        let target = facts[1 - selected];
        let mut paths = [None; 2];
        if let Some((derived, base)) = self
            .graph
            .class_of(facts[selected].ty)
            .zip(self.graph.class_of(target.ty))
            && derived != base
        {
            paths[selected] =
                Some(self.checked_base_path(derived, base, scope, location, true, false)?);
        }
        self.graph.expression_facts.set_conditional_paths(id, paths);
        let mut result = self.new_expression_facts(target.ty, target.category)?;
        result.set_bit_field(a.is_bit_field() || b.is_bit_field());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, expression_facts::ValueCategory, types::Cv},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{
            SyntaxUnit,
            ast::{Body, DeclKind, ExprKind, StmtKind},
        },
    };

    #[test]
    fn conditional_routes_belong_to_the_use_and_keep_original_operand_facts() {
        let source = SourceBuffer::new(
            b"struct B{int n;};struct D:B{};const B& f(bool c,const B&b,D&d){return (c?b:d);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"conditional-routes").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(count, tree.expressions.mark());
        let function = *tree.decl_lists.get(declarations).last().unwrap();
        let DeclKind::Function { body, .. } = tree.declarations.get(function).kind else {
            panic!("function")
        };
        let Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body")
        };
        let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound")
        };
        let StmtKind::Return(Some(paren)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return")
        };
        let ExprKind::Paren(conditional) = tree.expressions.get(paren).kind else {
            panic!("paren")
        };
        let ExprKind::Conditional {
            then_expr,
            else_expr,
            ..
        } = tree.expressions.get(conditional).kind
        else {
            panic!("conditional")
        };
        let facts = graph.expression_facts.get(conditional).unwrap();
        assert_eq!(facts.category, ValueCategory::Lvalue);
        assert_eq!(graph.types.get(facts.ty).cv, Cv::CONST);
        assert_eq!(
            graph.lower_conditional_base_offsets(conditional).unwrap(),
            [None, Some(0)]
        );
        assert_eq!(
            graph.lower_conditional_base_offsets(paren).unwrap(),
            [None, Some(0)]
        );
        let left = graph.expression_facts.get(then_expr).unwrap();
        let right = graph.expression_facts.get(else_expr).unwrap();
        assert_eq!(facts.ty, left.ty);
        assert_ne!(left.ty, right.ty);
        assert!(left.conversion.is_none() && right.conversion.is_none());
        assert!(left.base_path.is_none() && right.base_path.is_none());
        assert_eq!(graph.types.get(right.ty).cv, Cv::default());
    }
}
