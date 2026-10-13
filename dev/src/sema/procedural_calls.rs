// Translated from sema/analyzer_calls.cpp. See NOTICE.
//! Ordinary call resolution; candidate failures stay compact until the final diagnostic.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{Conversion, ConversionRank as R, ExprFacts, ValueCategory as V},
    function_resolution::CallTarget,
    graph::{BindingId, ScopeId},
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    syntax::{ast, storage::List},
    types::FundamentalType as F,
};
use std::cmp::Ordering;

#[derive(Default)]
pub(super) struct CallScratch {
    candidates: Vec<(BindingId, usize)>,
    sequences: Vec<Option<Conversion>>,
}
impl Analyzer<'_> {
    pub(super) fn procedural_call(
        &mut self,
        id: ast::ExprId,
        callee: ast::ExprId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let facts = if let ast::ExprKind::Id(name) = self.tree.expressions.get(callee).kind {
            let mut found = self.lookup_result(name, scope, super::lookup::Filter::Ordinary)?;
            if let Some(facts) = self.builtin_call(id, name, arguments, scope, &mut found)? {
                return Ok(facts);
            }
            if let Some(binding) = found.binding {
                let binding = self.graph.binding(binding);
                if matches!(
                    binding.kind,
                    super::graph::BindingKind::Type | super::graph::BindingKind::Alias
                ) {
                    let target = binding
                        .ty
                        .ok_or(Error::Invariant("cast name without type"))?;
                    let mut facts =
                        self.scalar_functional_cast(id, target, arguments, None, scope)?;
                    facts.functional_call = true;
                    return Ok(facts);
                }
            }
            for &argument in self.tree.expr_lists.get(arguments) {
                self.typed_expression(argument, scope)?;
            }
            let syntax_name = self.tree.names.get(name);
            if !syntax_name.global && self.tree.name_lists.get(syntax_name.components).len() == 1 {
                let component = self.tree.name_lists.get(syntax_name.components)[0];
                if let ast::NameKind::Identifier(key) = component.kind {
                    self.argument_dependent_lookup(key, arguments, &mut found)?;
                }
            }
            if found.binding.is_none() {
                return Err(Error::at(
                    self.tree.expressions.get(callee).location,
                    if self.tree.names.get(name).global
                        || self
                            .tree
                            .name_lists
                            .get(self.tree.names.get(name).components)
                            .len()
                            > 1
                    {
                        "undeclared name"
                    } else {
                        "undeclared function name"
                    },
                ));
            }
            let facts = self.named_expression(callee, &found)?;
            self.graph.expression_facts.set(callee, facts);
            super::record(&mut self.graph.expression_types, callee.index(), facts.ty);
            facts
        } else {
            let facts = self.typed_expression(callee, scope)?;
            for &argument in self.tree.expr_lists.get(arguments) {
                self.typed_expression(argument, scope)?;
            }
            facts
        };
        if let Some(set) = facts.functions {
            let mut designator = callee;
            let mut levels = 0;
            loop {
                designator = self.graph.expression_facts.root(designator);
                let ast::ExprKind::Unary { operand, .. } =
                    self.tree.expressions.get(designator).kind
                else {
                    break;
                };
                levels += 1;
                designator = operand;
            }
            if levels > 1 {
                self.check_resolved_function(callee)?;
            }
            // Nested argument calls finish before this query borrows the reusable scratch.
            let mut scratch = std::mem::take(&mut self.call_scratch);
            let chosen = self.resolve_call(id, set, arguments, &mut scratch);
            self.call_scratch = scratch;
            let binding = chosen?;
            self.select_function_designation(callee, binding)?;
            let ty = self.graph.types.canonical_function(
                self.graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("function without type"))?,
            )?;
            let mut facts = self.call_result(ty)?;
            facts.call = Some(CallTarget::Direct(
                self.graph
                    .function_origin(self.graph.binding(binding).entity)?,
            ));
            self.graph.record_call_binding(id, binding);
            return Ok(facts);
        }
        let target = self.decayed_type(facts.ty)?;
        let function = match self.graph.types.get(target).kind {
            TypeKind::Function { .. } => target,
            TypeKind::Pointer(element)
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) =>
            {
                self.convert_operand_scalar(callee, target, false)?;
                element
            }
            _ => {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "called object is not a function or pointer to function",
                ));
            }
        };
        let TypeKind::Function {
            parameters,
            variadic,
            ..
        } = &self.graph.types.get(function).kind
        else {
            return Err(Error::Invariant("indirect function type"));
        };
        let count = parameters.len();
        let args = self.tree.expr_lists.get(arguments);
        if args.len() < count || args.len() > count && !variadic {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "wrong number of arguments in an indirect call",
            ));
        }
        for (index, &argument) in args.iter().enumerate() {
            if index < count {
                let target = self.graph.types.function_parameter(function, index)?;
                self.convert_scalar(argument, target, false)?;
            } else {
                self.ellipsis_argument(argument)?;
            }
        }
        let mut facts = self.call_result(function)?;
        facts.call = Some(CallTarget::Indirect(function));
        Ok(facts)
    }
    fn argument_sequence(
        &mut self,
        mut facts: ExprFacts,
        target: TypeId,
    ) -> Result<Option<Conversion>, Error> {
        if facts.functions.is_some() {
            let Some(binding) = self.matching_function(facts, target)? else {
                return Ok(None);
            };
            let ty = self.graph.types.canonical_function(
                self.graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("function argument without type"))?,
            )?;
            facts.ty = if matches!(self.graph.types.get(facts.ty).kind, TypeKind::Pointer(_)) {
                self.graph.types.pointer(ty)?
            } else {
                ty
            };
            facts.functions = None;
        }
        self.scalar_sequence(facts, target, false)
    }
    fn resolve_call(
        &mut self,
        id: ast::ExprId,
        set: super::function_resolution::OverloadId,
        arguments: List<ast::ExprId>,
        scratch: &mut CallScratch,
    ) -> Result<BindingId, Error> {
        scratch.candidates.clear();
        scratch.sequences.clear();
        let args = self.tree.expr_lists.get(arguments);
        for index in 0..self.graph.overloads.get(set).len() {
            let binding = self.graph.overloads.get(set)[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() {
                return Err(Error::Missing("function template call deduction"));
            }
            let ty = entity.ty.ok_or(Error::Invariant("function without type"))?;
            let ty = self.graph.types.canonical_function(ty)?;
            let TypeKind::Function {
                parameters,
                variadic,
                ..
            } = &self.graph.types.get(ty).kind
            else {
                return Err(Error::Invariant("candidate without function type"));
            };
            let count = parameters.len();
            if args.len() > count && !variadic
                || (args.len()..count).any(|index| {
                    self.graph
                        .function_default(self.graph.binding(binding).entity, index)
                        .is_none()
                })
            {
                continue;
            }
            let first = scratch.sequences.len();
            let mut viable = true;
            for (index, &arg) in args.iter().enumerate() {
                let facts = self.graph.expression_facts.get(arg)?;
                let sequence = if index < count {
                    let target = self.graph.types.function_parameter(ty, index)?;
                    let Some(conversion) = self.argument_sequence(facts, target)? else {
                        viable = false;
                        break;
                    };
                    Some(conversion)
                } else {
                    if facts.functions.is_some()
                        || matches!(
                            self.graph.types.get(facts.ty).kind,
                            TypeKind::Scalar(Scalar::Void)
                        )
                    {
                        viable = false;
                        break;
                    }
                    None
                };
                scratch.sequences.push(sequence);
            }
            if viable {
                scratch.candidates.push((binding, first));
            } else {
                scratch.sequences.truncate(first);
            }
        }
        let location = self.tree.expressions.get(id).location;
        if scratch.candidates.is_empty() {
            return Err(Error::at(location, "no viable function for the call"));
        }
        let mut best = 0;
        for index in 1..scratch.candidates.len() {
            if self.compare_candidates(scratch, index, best, args.len())? == Ordering::Greater {
                best = index;
            }
        }
        for index in 0..scratch.candidates.len() {
            if index != best
                && self.compare_candidates(scratch, best, index, args.len())? != Ordering::Greater
            {
                return Err(Error::at(location, "ambiguous call to overloaded function"));
            }
        }
        let (binding, first) = scratch.candidates[best];
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("selected function without type"))?;
        let ty = self.graph.types.canonical_function(ty)?;
        for (index, &arg) in args.iter().enumerate() {
            if let Some(conversion) = scratch.sequences[first + index] {
                let mut facts = self.graph.expression_facts.get(arg)?;
                if facts.functions.is_some() {
                    let target = self.graph.types.function_parameter(ty, index)?;
                    let binding = self
                        .matching_function(facts, target)?
                        .ok_or(Error::Invariant("selected function argument disappeared"))?;
                    self.select_function_designation(arg, binding)?;
                    facts = self.graph.expression_facts.get(arg)?;
                }
                facts.conversion = Some(conversion);
                self.graph.expression_facts.set(arg, facts);
            } else {
                self.ellipsis_argument(arg)?;
            }
        }
        Ok(binding)
    }
    fn compare_candidates(
        &mut self,
        scratch: &CallScratch,
        a: usize,
        b: usize,
        count: usize,
    ) -> Result<Ordering, Error> {
        let (_, af) = scratch.candidates[a];
        let (_, bf) = scratch.candidates[b];
        let mut better = false;
        let mut worse = false;
        for index in 0..count {
            let order = match (scratch.sequences[af + index], scratch.sequences[bf + index]) {
                (Some(a), Some(b)) => self.compare_sequences(a, b)?,
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            };
            better |= order == Ordering::Greater;
            worse |= order == Ordering::Less;
        }
        Ok(match (better, worse) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => Ordering::Equal,
        })
    }
    fn ellipsis_argument(&mut self, id: ast::ExprId) -> Result<(), Error> {
        self.check_resolved_function(id)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        if matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "argument of type void",
            ));
        }
        let mut target = self.decayed_type(facts.ty)?;
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Basic(F::Float))
        ) {
            target = self.graph.types.scalar(Scalar::Basic(F::Double))?;
        } else if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Nullptr)
        ) {
            let void = self.graph.types.scalar(Scalar::Void)?;
            target = self.graph.types.pointer(void)?;
        } else if self.is_integral_operand(target) {
            target = self.promoted_type(target)?;
        }
        let mut conversion = self
            .scalar_sequence(facts, target, false)?
            .ok_or(Error::Missing("class ellipsis argument"))?;
        conversion.rank = R::Ellipsis;
        facts.conversion = Some(conversion);
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }
    fn call_result(&mut self, function: TypeId) -> Result<ExprFacts, Error> {
        let TypeKind::Function { result, .. } = self.graph.types.get(function).kind else {
            return Err(Error::Invariant("call without function type"));
        };
        if let TypeKind::Reference { element, kind } = self.graph.types.get(result).kind {
            let category = if kind == RefKind::Lvalue
                || matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) {
                V::Lvalue
            } else {
                V::Xvalue
            };
            self.new_expression_facts(element, category)
        } else {
            let result = if matches!(self.graph.types.get(result).kind, TypeKind::Class(_)) {
                result
            } else {
                self.bare_type(result)?
            };
            self.new_expression_facts(result, V::Prvalue)
        }
    }
}
