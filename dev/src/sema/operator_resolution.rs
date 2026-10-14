// Translated from sema/analyzer_operators.cpp::overloaded_operator/operator_conversions. See NOTICE.
//! Operator queries rank operand-aligned sequences and publish facts on the original use.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ObjectArgument, ObjectSource},
    function_resolution::CallTarget,
    graph::{BindingId, FunctionAvailability, ScopeId},
    procedural_calls::ArgumentSequence,
    types::{Scalar, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast, types::FundamentalType as F};
use std::cmp::Ordering;

#[derive(Clone, Copy)]
struct OperatorQuery<'a> {
    id: ast::ExprId,
    op: P,
    operands: &'a [ast::ExprId],
    postfix: bool,
    scope: ScopeId,
    class_operand: bool,
}

#[derive(Default)]
pub(super) struct OperatorScratch {
    candidates: Vec<(Option<BindingId>, usize)>,
    sequences: Vec<ArgumentSequence>,
}
impl OperatorScratch {
    fn clear(&mut self) {
        self.candidates.clear();
        self.sequences.clear();
    }
}
impl Analyzer<'_> {
    pub(super) fn overloaded_operator(
        &mut self,
        id: ast::ExprId,
        op: P,
        operands: &[ast::ExprId],
        postfix: bool,
        scope: ScopeId,
    ) -> Result<Option<ExprFacts>, Error> {
        let mut overloadable = false;
        let mut class_operand = false;
        for &operand in operands {
            if matches!(
                self.tree.expressions.get(operand).kind,
                ast::ExprKind::Braced { .. }
            ) {
                continue;
            }
            let facts = self.graph.expression_facts.get(operand)?;
            if facts.object.is_some() && facts.functions.is_some() {
                return Ok(None);
            }
            if facts.functions.is_some() {
                continue;
            }
            overloadable |= matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Class(_) | TypeKind::Enum(_)
            );
            class_operand |= self.graph.class_of(facts.ty).is_some();
        }
        if !overloadable {
            return Ok(None);
        }
        let set = self.operator_candidates(op, operands, scope)?;
        if set.is_empty() {
            return Ok(None);
        }
        let mut scratch = std::mem::take(&mut self.operator_scratch);
        let result = self.select_operator(
            OperatorQuery {
                id,
                op,
                operands,
                postfix,
                scope,
                class_operand,
            },
            &set,
            &mut scratch,
        );
        scratch.clear();
        self.operator_scratch = scratch;
        result
    }

    fn select_operator(
        &mut self,
        query: OperatorQuery<'_>,
        set: &[BindingId],
        scratch: &mut OperatorScratch,
    ) -> Result<Option<ExprFacts>, Error> {
        let OperatorQuery {
            id,
            op,
            operands,
            postfix,
            scope,
            class_operand,
        } = query;
        let mut same_enum_list = false;
        for &primary in set {
            let member = self.member_function_owner(primary).is_some();
            let args = &operands[usize::from(member)..];
            let Some(binding) = self.deduce_function_call(primary, args)? else {
                continue;
            };
            let ty = self
                .graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("operator without type"))?;
            let TypeKind::Function {
                parameters,
                variadic,
                ..
            } = &self.graph.types.get(ty).kind
            else {
                return Err(Error::Invariant("operator without signature"));
            };
            if *variadic || parameters.len() != args.len() + usize::from(postfix) {
                continue;
            }
            if postfix
                && !parameters.last().is_some_and(|&ty| {
                    matches!(
                        self.graph.types.get(ty).kind,
                        TypeKind::Scalar(Scalar::Basic(F::Int))
                    )
                })
            {
                continue;
            }
            if !class_operand {
                let mut matches_enum = false;
                for (index, &operand) in operands.iter().enumerate() {
                    let facts = self.graph.expression_facts.get(operand)?;
                    if let Some(&parameter) = parameters.get(index) {
                        let parameter = match self.graph.types.get(parameter).kind {
                            TypeKind::Reference { element, .. } => element,
                            _ => parameter,
                        };
                        matches_enum |=
                            matches!(self.graph.types.get(facts.ty).kind, TypeKind::Enum(_))
                                && self.graph.types.get(parameter).kind
                                    == self.graph.types.get(facts.ty).kind;
                    }
                }
                if !matches_enum {
                    continue;
                }
                if operands.len() == 2
                    && self
                        .graph
                        .entity(self.graph.binding(primary).entity)
                        .template_head
                        .is_none()
                {
                    let a = self.graph.expression_facts.get(operands[0])?.ty;
                    let b = self.graph.expression_facts.get(operands[1])?.ty;
                    let (pa, pb) = (parameters[0], parameters[1]);
                    same_enum_list |= pa == self.bare_type(a)? && pb == self.bare_type(b)?;
                }
            }
            let first = scratch.sequences.len();
            for (index, &operand) in operands.iter().enumerate() {
                let sequence = if member && index == 0 {
                    let facts = self.graph.expression_facts.get(operand)?;
                    let object = ObjectArgument {
                        source: ObjectSource::Explicit {
                            expression: operand,
                            arrow: false,
                        },
                        ty: facts.ty,
                        category: facts.category,
                    };
                    self.object_sequence(object, binding, false)?
                        .map(|conversion| ArgumentSequence {
                            conversion,
                            application: Some(conversion),
                            function: None,
                        })
                } else {
                    let target = self
                        .graph
                        .types
                        .function_parameter(ty, index - usize::from(member))?;
                    self.argument_sequence(operand, target)?
                };
                let Some(sequence) = sequence else {
                    break;
                };
                scratch.sequences.push(sequence);
            }
            if scratch.sequences.len() == first + operands.len() {
                scratch.candidates.push((Some(binding), first));
            } else {
                scratch.sequences.truncate(first);
            }
        }
        if scratch.candidates.is_empty() {
            return Ok(None);
        }
        if !class_operand
            && let Some(parameters) =
                self.enum_builtin_parameters(op, operands, postfix, same_enum_list)?
        {
            let first = scratch.sequences.len();
            for (&operand, target) in operands.iter().zip(parameters) {
                let Some(sequence) = self.argument_sequence(operand, target)? else {
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
        let mut best = 0;
        for index in 1..scratch.candidates.len() {
            if self.compare_operator_candidates(scratch, index, best, operands.len())?
                == Ordering::Greater
            {
                best = index;
            }
        }
        for index in 0..scratch.candidates.len() {
            if index != best
                && self.compare_operator_candidates(scratch, best, index, operands.len())?
                    != Ordering::Greater
            {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "ambiguous overloaded operator",
                ));
            }
        }
        let (binding, first) = scratch.candidates[best];
        let Some(binding) = binding else {
            return Ok(None);
        };
        let location = self.tree.expressions.get(id).location;
        if self
            .graph
            .entity(self.graph.binding(binding).entity)
            .availability
            == FunctionAvailability::Deleted
        {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.check_specialization_body(binding)?;
        let member = self.member_function_owner(binding).is_some();
        for (index, &operand) in operands.iter().enumerate() {
            let sequence = scratch.sequences[first + index];
            if matches!(
                self.tree.expressions.get(operand).kind,
                ast::ExprKind::Braced { .. }
            ) {
                self.initialize_list_argument(operand, sequence.conversion.target, scope)?;
                continue;
            }
            if let Some(function) = sequence.function {
                self.select_function_designation(operand, function)?;
            }
            let mut facts = self.graph.expression_facts.get(operand)?;
            let conversion = sequence.application.ok_or(Error::at(
                self.tree.expressions.get(operand).location,
                "invalid implicit conversion",
            ))?;
            if member && index == 0 {
                let class = self
                    .graph
                    .class_of(facts.ty)
                    .ok_or(Error::Invariant("operator receiver without class"))?;
                self.check_named_member_access(binding, class, class, scope, location)?;
            }
            facts.base_path = self.conversion_base_path(
                facts,
                conversion,
                scope,
                location,
                !(member && index == 0),
            )?;
            facts.conversion = Some(conversion);
            self.graph.expression_facts.set(operand, facts);
            if !member || index != 0 {
                self.class_parameter_destruction(conversion.target, scope, location)?;
                self.elide_class_copy(operand, conversion.target, scope, false, false, location)?;
            }
        }
        if self.unevaluated == 0 {
            self.demand_function(binding, true);
        }
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("operator without result"))?;
        let mut facts = self.call_result(ty)?;
        facts.call = Some(CallTarget::Direct(
            self.graph
                .function_origin(self.graph.binding(binding).entity)?,
        ));
        facts.call_form = super::expression_facts::CallForm::Operator;
        self.graph.record_call_binding(id, binding);
        Ok(Some(facts))
    }
    fn compare_operator_candidates(
        &mut self,
        scratch: &OperatorScratch,
        a: usize,
        b: usize,
        count: usize,
    ) -> Result<Ordering, Error> {
        let (ab, af) = scratch.candidates[a];
        let (bb, bf) = scratch.candidates[b];
        let mut better = false;
        let mut worse = false;
        for index in 0..count {
            let order = self.compare_sequences(
                scratch.sequences[af + index].conversion,
                scratch.sequences[bf + index].conversion,
            )?;
            better |= order == Ordering::Greater;
            worse |= order == Ordering::Less;
        }
        Ok(match (better, worse) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => match (ab, bb) {
                (Some(a), Some(b)) => self.compare_function_templates(a, b, Some(count))?,
                (Some(a), None) => {
                    if self
                        .graph
                        .entity(self.graph.binding(a).entity)
                        .template_head
                        .is_some()
                    {
                        Ordering::Less
                    } else {
                        Ordering::Equal
                    }
                }
                (None, Some(b)) => {
                    if self
                        .graph
                        .entity(self.graph.binding(b).entity)
                        .template_head
                        .is_some()
                    {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                }
                _ => Ordering::Equal,
            },
            _ => Ordering::Equal,
        })
    }
}
