// Translated from analyzer_conversions.cpp::conditional_match/converted_conditional.
// See NOTICE.
//! Conditional class glvalues retain the selected subobject route on the original use.
use super::{
    Error,
    analyzer::Analyzer,
    copy_initialization::CopyUse,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::ScopeId,
    procedural_calls::{ArgumentSequence, UserConversion},
    types::{RefKind, TypeKind},
    user_conversions::ConversionSite,
};
use crate::syntax::ast;

#[derive(Clone, Copy)]
enum Match {
    None,
    Glvalue(Option<ArgumentSequence>),
    Prvalue,
    Converted(ArgumentSequence),
}

impl Analyzer<'_> {
    fn conditional_class_match(
        &mut self,
        from: ExprFacts,
        other: ExprFacts,
    ) -> Result<Match, Error> {
        if other.category != V::Prvalue {
            let kind = if other.category == V::Lvalue {
                RefKind::Lvalue
            } else {
                RefKind::Rvalue
            };
            let reference = self.graph.types.reference(other.ty, kind)?;
            if from.category == other.category
                && self
                    .scalar_sequence(from, reference, false)?
                    .is_some_and(|binding| !binding.temporary)
            {
                return Ok(Match::Glvalue(None));
            }
            if self.graph.class_of(from.ty).is_some()
                && let Some(sequence) = self.conditional_user_sequence(from, reference)?
                && let Some(UserConversion::Function(binding)) = sequence.user_conversion
            {
                let signature = self
                    .graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("conditional converter signature"))?;
                if self.call_result_view(signature)?.category == other.category {
                    return Ok(Match::Glvalue(Some(sequence)));
                }
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
        let target = if self.graph.class_of(other.ty).is_some() {
            other.ty
        } else {
            self.decayed_type(other.ty)?
        };
        if let Some(conversion) = self.scalar_sequence(from, target, false)? {
            return Ok(Match::Converted(ArgumentSequence {
                conversion,
                application: None,
                function: None,
                user_conversion: None,
            }));
        }
        Ok(self
            .conditional_user_sequence(from, target)?
            .map_or(Match::None, Match::Converted))
    }

    pub(super) fn class_conditional(
        &mut self,
        id: ast::ExprId,
        operands: [ast::ExprId; 2],
        facts: &[ExprFacts; 2],
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let [a, b] = *facts;
        if self.bare_type(a.ty)? == self.bare_type(b.ty)? {
            let ac = self.type_cv(a.ty).0;
            let bc = self.type_cv(b.ty).0;
            let target = if ac & !bc == 0 { b.ty } else { a.ty };
            return self.class_conditional_value(operands, target, scope);
        }
        let matches = [
            self.conditional_class_match(a, b)?,
            self.conditional_class_match(b, a)?,
        ];
        let location = self.tree.expressions.get(id).location;
        let selected = match matches {
            [Match::None, Match::None] => {
                return self.builtin_conditional(id, operands, facts, scope);
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
        if matches!(matches[selected], Match::Prvalue) {
            return self.class_conditional_value(operands, facts[1 - selected].ty, scope);
        }
        let target = facts[1 - selected];
        let converted = operands[selected];
        let other = operands[1 - selected];
        let Match::Glvalue(sequence) = matches[selected] else {
            let Match::Converted(mut sequence) = matches[selected] else {
                return Err(Error::Invariant("conditional match"));
            };
            let class = self.graph.class_of(target.ty).is_some();
            let ty = if class {
                target.ty
            } else {
                self.decayed_type(target.ty)?
            };
            sequence.conversion.initialization = class;
            if matches!(sequence.user_conversion, Some(UserConversion::Function(_))) && class {
                return Err(Error::Missing(
                    "conditional class-valued conversion final copy",
                ));
            }
            if !self.apply_user_sequence(converted, sequence, scope, ConversionSite::Conditional)? {
                self.convert_operand_scalar(converted, ty, false)?;
            }
            if class {
                self.record_class_copy(
                    other,
                    ty,
                    scope,
                    CopyUse::Value { in_place: true },
                    self.tree.expressions.get(other).location,
                )?;
            } else {
                self.convert_operand_scalar(other, ty, false)?;
            }
            return self.new_expression_facts(ty, V::Prvalue);
        };
        let now = if let Some(sequence) = sequence {
            self.apply_builtin_conversion(converted, sequence, scope)?;
            self.builtin_operand_facts(converted)?
        } else {
            facts[selected]
        };
        let mut paths = [None; 2];
        if let Some((derived, base)) = self
            .graph
            .class_of(now.ty)
            .zip(self.graph.class_of(target.ty))
            && derived != base
        {
            paths[selected] =
                Some(self.checked_base_path(derived, base, scope, location, true, false)?);
        }
        self.graph.expression_facts.set_conditional_paths(id, paths);
        let mut result = self.new_expression_facts(target.ty, target.category)?;
        result.set_bit_field(now.is_bit_field() || target.is_bit_field());
        Ok(result)
    }
    fn builtin_conditional(
        &mut self,
        id: ast::ExprId,
        operands: [ast::ExprId; 2],
        facts: &[ExprFacts; 2],
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        use std::cmp::Ordering;
        let mut options = [Vec::new(), Vec::new()];
        for (facts, options) in facts.iter().zip(&mut options) {
            self.builtin_operand_options(*facts, options)?;
            for ty in options.iter_mut() {
                let object = match self.graph.types.get(*ty).kind {
                    TypeKind::Reference { element, .. } => element,
                    _ => *ty,
                };
                *ty = self.decayed_type(object)?;
            }
        }
        let mut candidates = Vec::new();
        for &left in &options[0] {
            for &right in &options[1] {
                let parameters = if self.is_arithmetic_operand(left)
                    && self.is_arithmetic_operand(right)
                {
                    [self.promoted_type(left)?, self.promoted_type(right)?]
                } else {
                    let mut views = [
                        ExprFacts::new(left, V::Prvalue),
                        ExprFacts::new(right, V::Prvalue),
                    ];
                    for (view, original) in views.iter_mut().zip(facts) {
                        view.set_null_literal(
                            self.graph.class_of(original.ty).is_none() && original.null_literal(),
                        );
                    }
                    let common = if left == right && self.scoped_enum(left) {
                        Some(left)
                    } else {
                        self.composite_pointer(views[0], views[1], false)?
                    };
                    let Some(common) = common else {
                        continue;
                    };
                    [common, common]
                };
                if candidates.iter().any(|(kept, _)| *kept == parameters) {
                    continue;
                }
                let Some(first) = self.argument_sequence(operands[0], parameters[0])? else {
                    continue;
                };
                let Some(second) = self.argument_sequence(operands[1], parameters[1])? else {
                    continue;
                };
                candidates.push((parameters, [first, second]));
            }
        }
        let mut best = 0;
        for index in 1..candidates.len() {
            if self.conditional_candidate_order(candidates[index].1, candidates[best].1)?
                == Ordering::Greater
            {
                best = index;
            }
        }
        let mut unique = !candidates.is_empty();
        for index in 0..candidates.len() {
            if index != best
                && self.conditional_candidate_order(candidates[best].1, candidates[index].1)?
                    != Ordering::Greater
            {
                unique = false;
                break;
            }
        }
        if !unique {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "incompatible operands of ?:",
            ));
        }
        let (parameters, sequences) = candidates[best];
        let ty = if parameters[0] == parameters[1] {
            parameters[0]
        } else {
            self.arithmetic_type(parameters[0], parameters[1])?
        };
        for (operand, sequence) in operands.into_iter().zip(sequences) {
            if let Some(UserConversion::Function(binding)) = sequence.user_conversion {
                self.check_function_not_deleted(
                    self.graph.binding(binding).entity,
                    self.tree.expressions.get(operand).location,
                )?;
                self.apply_builtin_conversion(operand, sequence, scope)?;
            }
            self.convert_operand_scalar(operand, ty, false)?;
        }
        self.new_expression_facts(ty, V::Prvalue)
    }
    fn conditional_candidate_order(
        &mut self,
        a: [ArgumentSequence; 2],
        b: [ArgumentSequence; 2],
    ) -> Result<std::cmp::Ordering, Error> {
        use std::cmp::Ordering;
        let left = self.compare_argument_sequences(a[0], b[0])?;
        let right = self.compare_argument_sequences(a[1], b[1])?;
        Ok(
            if left != Ordering::Less
                && right != Ordering::Less
                && (left == Ordering::Greater || right == Ordering::Greater)
            {
                Ordering::Greater
            } else if left != Ordering::Greater
                && right != Ordering::Greater
                && (left == Ordering::Less || right == Ordering::Less)
            {
                Ordering::Less
            } else {
                Ordering::Equal
            },
        )
    }
    fn class_conditional_value(
        &mut self,
        operands: [ast::ExprId; 2],
        target: super::types::TypeId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        for operand in operands {
            self.record_class_copy(
                operand,
                target,
                scope,
                CopyUse::Value { in_place: true },
                self.tree.expressions.get(operand).location,
            )?;
        }
        self.new_expression_facts(target, V::Prvalue)
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
    fn conversion_matches_keep_branch_facts_and_only_selected_demands() {
        use crate::sema::{lookup::Filter, types::TypeKind};
        let source = SourceBuffer::new(
            br"
            struct A { A(); ~A(); };
            struct B { B(const A&); B(long); B(const B&); ~B(); };
            struct R { operator B&() &; operator B&&() const &&; };
            struct Unused { int n; };
            B value(bool c,A&a,B&b){return c?((a)):((b));}
            B& reference(bool c,R&r,B&b){return c?((r)):((b));}
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"conditional-conversion-ownership").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
        let mut entity = |spelling: &[u8], filter| {
            let name = cursor.intern_name(spelling).unwrap();
            graph
                .binding(graph.own(graph.global, name, filter).unwrap())
                .entity
        };
        for (name, converted) in [(b"value".as_slice(), false), (b"reference", true)] {
            let function = entity(name, Filter::Ordinary);
            let definition = graph
                .entity(function)
                .callable()
                .function_definition
                .unwrap();
            let DeclKind::Function { body, .. } = tree.declarations.get(definition).kind else {
                panic!("function");
            };
            let Body::Parsed(body) = tree.bodies.get(body) else {
                panic!("body");
            };
            let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
                panic!("compound");
            };
            let StmtKind::Return(Some(choice)) =
                tree.statements.get(tree.stmt_lists.get(items)[0]).kind
            else {
                panic!("return");
            };
            let ExprKind::Conditional {
                then_expr,
                else_expr,
                ..
            } = tree.expressions.get(choice).kind
            else {
                panic!("conditional");
            };
            let left = graph.expression_facts.get(then_expr).unwrap();
            let right = graph.expression_facts.get(else_expr).unwrap();
            assert_eq!(left.category, ValueCategory::Lvalue);
            assert_eq!(right.category, ValueCategory::Lvalue);
            assert_ne!(left.ty, right.ty);
            assert_ne!(graph.expression_facts.root(then_expr), then_expr);
            assert_eq!(graph.expression_facts.get(choice).unwrap().ty, right.ty);
            if converted {
                let step = graph
                    .expression_facts
                    .function_conversion(then_expr)
                    .unwrap();
                assert_eq!(step.result, right.ty);
                assert_eq!(step.category, ValueCategory::Lvalue);
                assert!(left.conversion.is_none());
                assert!(left.class_copy(&graph.expression_facts).is_none());
                assert!(
                    graph
                        .expression_facts
                        .function_conversion(else_expr)
                        .is_none()
                );
                let class = entity(b"R", Filter::Prefix);
                assert_eq!(
                    step.binding,
                    graph.entity(class).class().declared_conversions[0]
                );
                assert!(!graph.root_demands.as_slice().iter().any(|d| matches!(d,
                    crate::sema::function_demands::Demand::Function {binding,..}
                    if *binding == graph.entity(class).class().declared_conversions[1])));
            } else {
                let (constructor, arguments) =
                    graph.expression_facts.class_conversion(then_expr).unwrap();
                assert!(
                    matches!(arguments,crate::sema::constructor_initialization::ConstructorArguments::Single(original) if original == then_expr)
                );
                let class = entity(b"B", Filter::Prefix);
                assert_eq!(
                    graph.constructors.get(constructor).binding,
                    Some(graph.entity(class).class().declared_constructors[0])
                );
                assert!(matches!(
                    graph.types.get(left.conversion.unwrap().target).kind,
                    TypeKind::Reference { .. }
                ));
                assert!(
                    graph
                        .entity(
                            graph
                                .binding(graph.entity(class).class().declared_constructors[1])
                                .entity
                        )
                        .initialization()
                        .constructor
                        .is_none()
                );
                assert!(graph.expression_facts.class_conversion(else_expr).is_none());
            }
        }
        let unused = entity(b"Unused", Filter::Prefix);
        assert!(matches!(
            graph.entity(unused).class().layout,
            crate::sema::layout::LayoutState::NotStarted
        ));
        assert!(graph.entity(unused).special().implicit_copies.is_none());
    }

    #[test]
    fn branch_copies_and_outer_elision_preserve_original_facts_and_demand() {
        use crate::sema::copy_initialization::CopyAction;
        let source = SourceBuffer::new(
            b"struct V{V();V(const V&);V(V&&);};struct Unused{Unused(const Unused&);};V a;const V b;V copied=true?a:V();V direct(false?a:V());const V cv=true?a:b;"
                .to_vec(),
        ).unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"conditional-copy-ownership").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(count, tree.expressions.mark());
        let mut conditionals = Vec::new();
        for &declaration in tree.decl_lists.get(declarations) {
            let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind
            else {
                continue;
            };
            for declarator in tree.init_declarators.get(declarators) {
                let Some(initializer) = declarator.initializer else {
                    continue;
                };
                let expression = match initializer.kind {
                    crate::syntax::ast::InitializerKind::Assign(id) => id,
                    crate::syntax::ast::InitializerKind::Parenthesized(arguments) => {
                        tree.expr_lists.get(arguments)[0]
                    }
                    _ => continue,
                };
                if let ExprKind::Conditional {
                    then_expr,
                    else_expr,
                    ..
                } = tree.expressions.get(expression).kind
                {
                    conditionals.push((expression, then_expr, else_expr));
                }
            }
        }
        assert_eq!(conditionals.len(), 3);
        let mut branch_constructor = None;
        for ((expression, left, right), elision) in
            conditionals[..2].iter().copied().zip([true, false])
        {
            let facts = graph.expression_facts.get(expression).unwrap();
            assert_eq!(facts.category, ValueCategory::Prvalue);
            assert!(
                matches!(graph.lower_class_copy(expression).unwrap(), Some(CopyAction::Constructor { elision: permission, .. }) if permission == elision)
            );
            let left_facts = graph.expression_facts.get(left).unwrap();
            assert_eq!(left_facts.category, ValueCategory::Lvalue);
            assert!(left_facts.conversion.is_none());
            let Some(CopyAction::Constructor {
                constructor,
                elision: false,
                ..
            }) = graph.lower_class_copy(left).unwrap()
            else {
                panic!("selected branch copy")
            };
            assert_eq!(*branch_constructor.get_or_insert(constructor), constructor);
            assert_eq!(
                graph.expression_facts.get(right).unwrap().category,
                ValueCategory::Prvalue
            );
            assert!(graph.lower_class_copy(right).unwrap().is_none());
        }
        let (expression, left, right) = conditionals[2];
        let facts = graph.expression_facts.get(expression).unwrap();
        assert_eq!(facts.category, ValueCategory::Lvalue);
        assert_eq!(graph.types.get(facts.ty).cv, Cv::CONST);
        for operand in [left, right] {
            assert!(
                graph
                    .expression_facts
                    .get(operand)
                    .unwrap()
                    .class_copy(&graph.expression_facts)
                    .is_none()
            );
            assert!(graph.lower_class_copy(operand).unwrap().is_none());
        }
        let name = cursor.intern_name(b"Unused").unwrap();
        let unused = graph
            .binding(
                graph
                    .own(graph.global, name, crate::sema::lookup::Filter::Prefix)
                    .unwrap(),
            )
            .entity;
        assert!(graph.entity(unused).special().implicit_copies.is_none());
    }

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
            graph
                .lower_conditional_projections(conditional)
                .unwrap()
                .map(|projection| projection
                    .and_then(crate::sema::lowering::BaseProjection::known_offset)),
            [None, Some(0)]
        );
        assert_eq!(
            graph
                .lower_conditional_projections(paren)
                .unwrap()
                .map(|projection| projection
                    .and_then(crate::sema::lowering::BaseProjection::known_offset)),
            [None, Some(0)]
        );
        let left = graph.expression_facts.get(then_expr).unwrap();
        let right = graph.expression_facts.get(else_expr).unwrap();
        assert_eq!(facts.ty, left.ty);
        assert_ne!(left.ty, right.ty);
        assert!(left.conversion.is_none() && right.conversion.is_none());
        assert!(
            left.base_path(&graph.expression_facts).is_none()
                && right.base_path(&graph.expression_facts).is_none()
        );
        assert_eq!(graph.types.get(right.ty).cv, Cv::default());
    }
}
