// Translated from analyzer_expressions.cpp and conversions.cpp. See NOTICE.
//! Function designations, target-directed selection and standard sequence ordering.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, ValueCategory as V, ValueConversion as C,
    },
    graph::BindingId,
    types::{RefKind, TypeId, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
pub(super) struct OverloadId(u32);
#[derive(Clone, Copy, Debug)]
pub(super) enum CallTarget {
    Direct(BindingId),
    Indirect(TypeId),
}
#[derive(Default)]
pub(super) struct FunctionSets {
    bindings: Vec<BindingId>,
    ranges: Vec<(u32, u32)>,
}
impl FunctionSets {
    pub(super) fn add(&mut self, bindings: &[BindingId]) -> Result<OverloadId, Error> {
        let id =
            u32::try_from(self.ranges.len()).map_err(|_| Error::Limit("overload set limit"))?;
        let begin = u32::try_from(self.bindings.len())
            .map_err(|_| Error::Limit("overload binding limit"))?;
        let count =
            u32::try_from(bindings.len()).map_err(|_| Error::Limit("overload binding limit"))?;
        let end = begin
            .checked_add(count)
            .ok_or(Error::Limit("overload binding limit"))?;
        self.bindings.extend_from_slice(bindings);
        self.ranges.push((begin, end));
        Ok(OverloadId(id))
    }
    pub(super) fn get(&self, id: OverloadId) -> &[BindingId] {
        let (begin, end) = self.ranges[id.0 as usize];
        &self.bindings[begin as usize..end as usize]
    }
}
impl Analyzer<'_> {
    pub(super) fn function_designation(
        &mut self,
        binding: BindingId,
        functions: &[BindingId],
    ) -> Result<ExprFacts, Error> {
        if functions.is_empty() {
            return Err(Error::Invariant("empty function lookup"));
        }
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("function without type"))?;
        let mut facts = self.new_expression_facts(ty, V::Lvalue)?;
        facts.functions = Some(self.graph.overloads.add(functions)?);
        Ok(facts)
    }
    pub(super) fn check_resolved_function(&mut self, id: ast::ExprId) -> Result<(), Error> {
        let facts = self.graph.expression_facts.get(id)?;
        if let Some(set) = facts.functions {
            let bindings = self.graph.overloads.get(set);
            if bindings.len() != 1
                || self
                    .graph
                    .entity(self.graph.binding(bindings[0]).entity)
                    .template_head
                    .is_some()
            {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "overloaded function name without a target type",
                ));
            }
            let binding = bindings[0];
            self.select_function_designation(id, binding)?;
        }
        Ok(())
    }
    pub(super) fn matching_function(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
    ) -> Result<Option<BindingId>, Error> {
        let Some(set) = facts.functions else {
            return Ok(None);
        };
        let target = match self.graph.types.get(target).kind {
            TypeKind::Reference { element, .. } => element,
            _ => target,
        };
        let target = match self.graph.types.get(target).kind {
            TypeKind::Pointer(element) => element,
            _ => target,
        };
        if !matches!(self.graph.types.get(target).kind, TypeKind::Function { .. }) {
            return Ok(None);
        }
        let target = self.graph.types.canonical_function(target)?;
        let mut found = None;
        for index in 0..self.graph.overloads.get(set).len() {
            let binding = self.graph.overloads.get(set)[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() {
                return Err(Error::Missing(
                    "target-directed function template deduction",
                ));
            }
            let ty = entity.ty.ok_or(Error::Invariant("function without type"))?;
            if self.graph.types.canonical_function(ty)? == target {
                if found.is_some() {
                    return Ok(None);
                }
                found = Some(binding);
            }
        }
        Ok(found)
    }
    pub(super) fn select_function_designation(
        &mut self,
        id: ast::ExprId,
        binding: BindingId,
    ) -> Result<(), Error> {
        if self
            .graph
            .entity(self.graph.binding(binding).entity)
            .availability
            == super::graph::FunctionAvailability::Deleted
        {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "use of a deleted function",
            ));
        }
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("function without type"))?;
        let ty = self.graph.types.canonical_function(ty)?;
        let root = self.graph.expression_facts.root(id);
        let inner = match self.tree.expressions.get(root).kind {
            ast::ExprKind::Unary { op, operand }
                if matches!(op.token.as_punctuator(), Some(P::BitAnd | P::Multiply)) =>
            {
                Some(operand)
            }
            _ => None,
        };
        // resolve_overloaded publishes only the designation and its immediate
        // unary operand. Deeper unary nodes retain their independently typed facts.
        for current in std::iter::once(root).chain(inner) {
            let current = self.graph.expression_facts.root(current);
            let mut facts = self.graph.expression_facts.get(current)?;
            facts.ty = if current == root && inner.is_some() {
                self.graph.types.pointer(ty)?
            } else {
                ty
            };
            facts.functions = None;
            facts.function = Some(
                self.graph
                    .function_origin(self.graph.binding(binding).entity)?,
            );
            self.graph.expression_facts.set(current, facts);
            super::record(&mut self.graph.expression_types, current.index(), facts.ty);
        }
        Ok(())
    }
    pub(super) fn compare_sequences(
        &mut self,
        a: Conversion,
        b: Conversion,
    ) -> Result<Ordering, Error> {
        if a.rank == R::Ellipsis || b.rank == R::Ellipsis {
            return Ok(match (a.rank == R::Ellipsis, b.rank == R::Ellipsis) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            });
        }
        let ai = a.second == C::None && !a.qualification;
        let bi = b.second == C::None && !b.qualification;
        if ai != bi {
            return Ok(ai.cmp(&bi));
        }
        if a.second == b.second && a.qualification != b.qualification {
            return Ok(b.qualification.cmp(&a.qualification));
        }
        if a.rank != b.rank {
            return Ok(b.rank.cmp(&a.rank));
        }
        let ab = a.second == C::PointerBoolean;
        let bb = b.second == C::PointerBoolean;
        if ab != bb {
            return Ok(bb.cmp(&ab));
        }
        let ar = matches!(
            self.graph.types.get(a.target).kind,
            TypeKind::Reference { .. }
        );
        let br = matches!(
            self.graph.types.get(b.target).kind,
            TypeKind::Reference { .. }
        );
        if a.second == b.second
            && a.qualification
            && b.qualification
            && !ar
            && !br
            && a.result != b.result
        {
            if self.pointer_qualification(a.result, b.result)? {
                return Ok(Ordering::Greater);
            }
            if self.pointer_qualification(b.result, a.result)? {
                return Ok(Ordering::Less);
            }
        }
        if let (
            TypeKind::Reference {
                element: ae,
                kind: ak,
            },
            TypeKind::Reference {
                element: be,
                kind: bk,
            },
        ) = (
            self.graph.types.get(a.target).kind.clone(),
            self.graph.types.get(b.target).kind.clone(),
        ) {
            let af = matches!(self.graph.types.get(ae).kind, TypeKind::Function { .. });
            let bf = matches!(self.graph.types.get(be).kind, TypeKind::Function { .. });
            if ak != bk && af == bf {
                return Ok(if (ak == RefKind::Rvalue) == af {
                    Ordering::Less
                } else {
                    Ordering::Greater
                });
            }
            let ac = self.type_cv(ae).0;
            let bc = self.type_cv(be).0;
            if ac != bc && self.bare_type(ae)? == self.bare_type(be)? {
                if ac & !bc == 0 {
                    return Ok(Ordering::Greater);
                }
                if bc & !ac == 0 {
                    return Ok(Ordering::Less);
                }
            }
        }
        Ok(Ordering::Equal)
    }
}
