// Translated from sema/analyzer_operators.cpp::class_builtin_candidates. See NOTICE.
//! Query-local converted type alternatives compete without rewriting or reparsing syntax.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::ScopeId,
    operator_resolution::OperatorScratch,
    procedural_calls::ArgumentSequence,
    specifiers::Explicitness,
    types::{RefKind, TypeId, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast};

pub(super) fn converted_builtin(op: P, count: usize) -> bool {
    match op {
        P::LogicalAnd
        | P::LogicalOr
        | P::LogicalNot
        | P::Comma
        | P::Assign
        | P::Arrow
        | P::LeftParen => false,
        P::BitAnd => count == 2,
        _ => true,
    }
}

impl Analyzer<'_> {
    fn builtin_operand_options(
        &mut self,
        facts: ExprFacts,
        options: &mut Vec<TypeId>,
    ) -> Result<(), Error> {
        let begin = options.len();
        if facts.functions.is_some() {
            return Ok(());
        }
        let Some(class) = self.graph.class_of(facts.ty) else {
            options.push(match self.graph.types.get(facts.ty).kind {
                TypeKind::Array { element, .. } => self.graph.types.pointer(element)?,
                _ => facts.ty,
            });
            return Ok(());
        };
        if !self.graph.entity(class).complete {
            return Ok(());
        }
        let count = self.visible_conversion_functions(class)?.len();
        for index in 0..count {
            let binding = self.visible_conversion_functions(class)?[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.explicitness == Explicitness::Explicit {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("conversion option without type"))?;
            let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
                return Err(Error::Invariant("conversion option without signature"));
            };
            let object = match self.graph.types.get(result).kind {
                TypeKind::Reference { element, .. } => element,
                _ => result,
            };
            if self.graph.class_of(object).is_some() {
                continue;
            }
            let result = match self.graph.types.get(object).kind {
                TypeKind::Array { element, .. } => self.graph.types.pointer(element)?,
                TypeKind::Function { .. } if result != object => {
                    self.graph.types.pointer(object)?
                }
                _ if result == object => self.bare_type(result)?,
                _ => result,
            };
            if !options[begin..].contains(&result) {
                options.push(result);
            }
        }
        Ok(())
    }

    pub(super) fn class_builtin_candidates(
        &mut self,
        op: P,
        operands: &[ast::ExprId],
        postfix: bool,
        scratch: &mut OperatorScratch,
    ) -> Result<(), Error> {
        let mut ranges = [0..0, 0..0];
        let mut original = [self.graph.expression_facts.get(operands[0])?; 2];
        for (index, &operand) in operands.iter().enumerate() {
            let facts = self.graph.expression_facts.get(operand)?;
            let begin = scratch.operand_options.len();
            self.builtin_operand_options(facts, &mut scratch.operand_options)?;
            ranges[index] = begin..scratch.operand_options.len();
            if ranges[index].is_empty() {
                return Ok(());
            }
            original[index] = facts;
        }
        let first_candidate = scratch.candidates.len();
        for a in ranges[0].clone() {
            let left = scratch.operand_options[a];
            // A unary query has exactly one combination for each left option.
            let second = if operands.len() == 1 {
                a..a + 1
            } else {
                ranges[1].clone()
            };
            for b in second {
                let right = scratch.operand_options[b];
                let mut views = original;
                for ((facts, view), option) in original.iter().zip(&mut views).zip([left, right]) {
                    let (ty, category) = match self.graph.types.get(option).kind {
                        TypeKind::Reference { element, kind } => (
                            element,
                            if kind == RefKind::Lvalue {
                                V::Lvalue
                            } else {
                                V::Xvalue
                            },
                        ),
                        _ => (
                            option,
                            if facts.ty == option {
                                facts.category
                            } else {
                                V::Prvalue
                            },
                        ),
                    };
                    *view = ExprFacts::new(ty, category);
                    view.null_literal =
                        self.graph.class_of(facts.ty).is_none() && facts.null_literal;
                }
                let Some(parameters) =
                    self.builtin_parameters(op, &views[..operands.len()], postfix, false)?
                else {
                    continue;
                };
                if self.repeated_builtin(
                    parameters,
                    operands.len(),
                    postfix,
                    first_candidate,
                    scratch,
                )? {
                    continue;
                }
                let first = scratch.sequences.len();
                for (&operand, target) in operands.iter().zip(parameters) {
                    let sequence = self.argument_sequence(operand, target)?;
                    let sequence = if sequence.is_none()
                        && matches!(
                            self.graph.types.get(target).kind,
                            TypeKind::Reference { .. }
                        ) {
                        self.scalar_conversion_function_sequence(
                            self.graph.expression_facts.get(operand)?,
                            target,
                            false,
                        )?
                    } else {
                        sequence
                    };
                    let Some(sequence) = sequence else {
                        break;
                    };
                    scratch.sequences.push(sequence);
                }
                if scratch.sequences.len() == first + operands.len() {
                    scratch.candidates.push((None, first));
                } else {
                    scratch.sequences.truncate(first);
                }
            }
        }
        Ok(())
    }

    fn repeated_builtin(
        &self,
        parameters: [TypeId; 2],
        count: usize,
        postfix: bool,
        first_candidate: usize,
        scratch: &OperatorScratch,
    ) -> Result<bool, Error> {
        for &(binding, first) in &scratch.candidates[first_candidate..] {
            if binding.is_none()
                && (0..count)
                    .all(|i| scratch.sequences[first + i].conversion.target == parameters[i])
            {
                return Ok(true);
            }
        }
        for &(binding, _) in &scratch.candidates[..first_candidate] {
            let Some(binding) = binding else {
                continue;
            };
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() || self.member_function_owner(binding).is_some() {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("operator candidate without type"))?;
            let TypeKind::Function {
                parameters: declared,
                ..
            } = &self.graph.types.get(ty).kind
            else {
                return Err(Error::Invariant("operator candidate without signature"));
            };
            if declared.len() == count + usize::from(postfix)
                && declared[..count] == parameters[..count]
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn apply_builtin_conversion(
        &mut self,
        id: ast::ExprId,
        sequence: ArgumentSequence,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let source = self.graph.expression_facts.get(id)?;
        let selected = self.select_conversion_function(
            source,
            sequence,
            scope,
            self.tree.expressions.get(id).location,
            true,
        )?;
        // The ordinary builtin checker applies the final standard conversion
        // to the conversion function's result, preserving the original receiver.
        self.graph
            .expression_facts
            .set_function_conversion(id, selected);
        Ok(())
    }

    pub(super) fn builtin_operand_facts(&self, id: ast::ExprId) -> Result<ExprFacts, Error> {
        if let Some(step) = self.graph.expression_facts.function_conversion(id)
            && step.builtin_operand
        {
            Ok(ExprFacts::new(step.result, step.category))
        } else {
            self.graph.expression_facts.get(id)
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
    fn builtin_selection_retains_original_receiver_and_demands_only_the_winner() {
        let source = SourceBuffer::new(
            b"struct X{operator int();operator long() const;};\
              int operator+(X&,char*);int f(X&x){return +(((x)));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-owner").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body");
        };
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(value)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Unary { operand, .. } = tree.expressions.get(value).kind else {
            panic!("unary");
        };
        let original = graph.expression_facts.get(operand).unwrap();
        let class = graph.class_of(original.ty).unwrap();
        assert_eq!(original.category, V::Lvalue);
        let selection = graph.expression_facts.function_conversion(operand).unwrap();
        assert!(selection.builtin_operand);
        assert!(matches!(
            graph.types.get(selection.result).kind,
            TypeKind::Scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int
            ))
        ));
        let root = graph.expression_facts.root(operand);
        assert_eq!(
            graph
                .expression_facts
                .function_conversion(root)
                .unwrap()
                .binding,
            selection.binding
        );
        let demanded = |binding| {
            graph.root_demands.as_slice().iter().any(|demand| {
                matches!(demand,
            super::super::function_demands::Demand::Function { binding: found, .. } if *found == binding)
            })
        };
        assert!(demanded(selection.binding));
        for &binding in &graph.entity(class).declared_conversions {
            assert_eq!(demanded(binding), binding == selection.binding);
        }
    }

    #[test]
    fn converted_pointer_keeps_result_base_path_separate_from_the_receiver() {
        let source = SourceBuffer::new(
            b"struct B{int n;};struct D:B{};struct X{operator D*();};\
              bool f(X&x,B*b){return x==b;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-result-path").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[3])
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body");
        };
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(value)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Binary { left, .. } = tree.expressions.get(value).kind else {
            panic!("binary");
        };
        let original = graph.expression_facts.get(left).unwrap();
        assert!(graph.class_of(original.ty).is_some());
        assert!(original.base_path.is_none());
        let step = graph.expression_facts.function_conversion(left).unwrap();
        assert!(step.after_path.is_some());
        assert_ne!(step.result, step.after.result);
        assert_eq!(
            step.after.second,
            super::super::expression_facts::ValueConversion::DerivedToBase
        );
        assert_eq!(graph.lower_base_conversion_offset(left).unwrap(), Some(0));
    }

    #[test]
    fn ambiguous_builtin_query_restores_scratch_before_the_next_operator() {
        let source = SourceBuffer::new(
            b"struct X{operator int();operator long();};struct Y{operator int();};\
              int bad(X&x){return x+1;}int good(Y&y){return y+2;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-rollback").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        let declarations = tree.decl_lists.get(declarations);
        for &declaration in &declarations[..2] {
            analyzer.declaration(declaration, scope).unwrap();
        }
        assert!(matches!(
            analyzer.declaration(declarations[2], scope),
            Err(Error::Diagnostic {
                message: "ambiguous overloaded operator",
                ..
            })
        ));
        assert!(analyzer.operator_lookup.bindings.is_empty());
        assert!(analyzer.operator_lookup.seen.is_empty());
        assert!(analyzer.operator_scratch.candidates.is_empty());
        assert!(analyzer.operator_scratch.sequences.is_empty());
        assert!(analyzer.operator_scratch.operand_options.is_empty());
        analyzer.declaration(declarations[3], scope).unwrap();
        assert!(analyzer.operator_scratch.candidates.is_empty());
        assert!(analyzer.operator_scratch.sequences.is_empty());
        assert!(analyzer.operator_scratch.operand_options.is_empty());
        assert_eq!(tree.expressions.mark(), count);
    }
}
