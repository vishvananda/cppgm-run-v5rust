// Translated from analyzer_expressions.cpp and conversions.cpp. See NOTICE.
//! Function designations, target-directed selection and standard sequence ordering.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, ValueCategory as V, ValueConversion as C,
    },
    graph::{BindingId, BindingKind, ScopeId, ScopeKind},
    lookup::Filter,
    types::{RefKind, TypeId, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
pub(super) struct OverloadId(u32);
#[derive(Clone, Copy, Debug)]
pub(super) enum CallTarget {
    Direct(BindingId),
    Indirect(TypeId, Option<super::graph::EntityId>),
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
    pub(super) fn member_function_owner(&self, binding: BindingId) -> Option<TypeId> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        let home = entity.home?;
        if self.graph.scope(home).kind != ScopeKind::Class
            || entity.function.is_some_and(|props| props.is_static)
        {
            return None;
        }
        self.graph
            .scope(home)
            .owner
            .and_then(|class| self.graph.entity(class).ty)
    }

    pub(super) fn qualified_function_address(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<ExprFacts>, Error> {
        let ast::ExprKind::Id(name) = self.tree.expressions.get(operand).kind else {
            return Ok(None);
        };
        let node = self.tree.names.get(name);
        if !node.global && self.tree.name_lists.get(node.components).len() == 1 {
            return Ok(None);
        }
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty address name"))?;
        let ast::NameKind::Identifier(key) = last.kind else {
            return Err(Error::at(
                node.location,
                "operator and special member names are not supported here",
            ));
        };
        let location = node.location;
        let prefix = self
            .prefix(name, scope)?
            .ok_or(Error::Invariant("qualified address without prefix"))?;
        self.lookup_location = location;
        let mut found = self
            .qualified_result(prefix, key.into(), Filter::Ordinary)
            .map_err(|error| match error {
                Error::Diagnostic {
                    message: "ambiguous name",
                    ..
                } if self.graph.scope(prefix).kind == ScopeKind::Class => {
                    Error::at(location, "ambiguous member")
                }
                other => other,
            })?;
        if last.arguments.is_some() {
            self.explicit_function_specializations(*last, &mut found, scope, false)?;
        }
        let binding = found
            .binding
            .ok_or(Error::Invariant("address lookup without binding"))?;
        super::record(&mut self.graph.resolved_names, name.index(), binding);
        if let Some(naming) = self.graph.scope(prefix).owner
            && self.graph.scope(prefix).kind == ScopeKind::Class
            && self.graph.binding_scope(binding) != prefix
        {
            self.check_named_member_access(binding, naming, naming, scope, location)?;
        }
        if self.graph.binding(binding).kind != BindingKind::Function {
            return Ok(None);
        }
        let mut inner = self.function_designation(binding, found.functions.as_slice())?;
        inner.member_address_scope = Some(scope);
        self.graph.expression_facts.set(operand, inner);
        super::record(&mut self.graph.expression_types, operand.index(), inner.ty);
        let ty = self.function_address_type(binding, inner.ty)?;
        let mut facts = self.new_expression_facts(ty, V::Prvalue)?;
        facts.functions = inner.functions;
        facts.member_address_scope = Some(scope);
        if found.functions.as_slice().len() == 1
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .template_head
                .is_none()
        {
            self.check_direct_member_access(binding, scope, node.location)?;
            if self
                .graph
                .entity(self.graph.binding(binding).entity)
                .availability
                == super::graph::FunctionAvailability::Deleted
            {
                return Err(Error::at(node.location, "use of a deleted function"));
            }
            self.graph.expression_facts.set(id, facts);
            self.select_function_designation(id, binding)?;
            return Ok(Some(self.graph.expression_facts.get(id)?));
        }
        Ok(Some(facts))
    }

    pub(super) fn function_address_type(
        &mut self,
        binding: BindingId,
        ty: TypeId,
    ) -> Result<TypeId, Error> {
        if let Some(owner) = self.member_function_owner(binding) {
            self.graph.types.member_pointer(owner, ty)
        } else {
            self.graph.types.pointer(ty)
        }
    }
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
    pub(super) fn ordinary_function_operand(&self, facts: ExprFacts) -> Option<BindingId> {
        let bindings = self.graph.overloads.get(facts.functions?);
        let [binding] = bindings else {
            return None;
        };
        let entity = self.graph.entity(self.graph.binding(*binding).entity);
        (entity.template_head.is_none()
            && !self.graph.function_templates.requires_target(*binding)
            && (self.member_function_owner(*binding).is_none()
                || facts.member_address_scope.is_some()))
        .then_some(*binding)
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
        let (owner, target) = match self.graph.types.get(target).kind {
            TypeKind::Pointer(element) => (None, element),
            TypeKind::MemberPointer { owner, element } => (Some(owner), element),
            _ => (None, target),
        };
        if !matches!(self.graph.types.get(target).kind, TypeKind::Function { .. }) {
            return Ok(None);
        }
        let target = self.graph.types.canonical_function(target)?;
        let mut found = None;
        let mut template = false;
        for index in 0..self.graph.overloads.get(set).len() {
            let binding = self.graph.overloads.get(set)[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if self.member_function_owner(binding) != owner {
                continue;
            }
            if entity.template_head.is_some() {
                template = true;
                continue;
            }
            let ty = entity.ty.ok_or(Error::Invariant("function without type"))?;
            if self.graph.types.canonical_function(ty)? == target {
                if found.is_some() {
                    return Ok(None);
                }
                found = Some(binding);
            }
        }
        if found.is_none() && template {
            let mut candidates = Vec::new();
            for index in 0..self.graph.overloads.get(set).len() {
                let binding = self.graph.overloads.get(set)[index];
                if self.member_function_owner(binding) != owner {
                    continue;
                }
                if let Some(binding) = self.deduce_function_target(binding, target)?
                    && self.graph.binding(binding).ty == Some(target)
                    && !candidates.contains(&binding)
                {
                    candidates.push(binding);
                }
            }
            for &binding in &candidates {
                if let Some(old) = found {
                    if self.compare_function_templates(binding, old, None)? == Ordering::Greater {
                        found = Some(binding);
                    }
                } else {
                    found = Some(binding);
                }
            }
            if let Some(best) = found {
                for binding in candidates {
                    if binding != best
                        && self.compare_function_templates(best, binding, None)?
                            != Ordering::Greater
                    {
                        return Ok(None);
                    }
                }
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
        self.check_specialization_body(binding)?;
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("function without type"))?;
        let ty = self.graph.types.canonical_function(ty)?;
        let root = self.graph.expression_facts.root(id);
        let address_scope = self.graph.expression_facts.get(root)?.member_address_scope;
        if let Some(scope) = address_scope {
            self.check_direct_member_access(
                binding,
                scope,
                self.tree.expressions.get(root).location,
            )?;
        }
        // Ordinary unary designations are already typed, including their
        // function-to-pointer operand conversions. Only unresolved overload
        // designations use the oracle's immediate-unary rewrite below.
        if self.graph.expression_facts.get(root)?.functions.is_none() {
            if self.unevaluated == 0 {
                self.demand_function(binding, false);
            }
            return Ok(());
        }
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
                if address_scope.is_some() {
                    self.function_address_type(binding, ty)?
                } else {
                    self.graph.types.pointer(ty)?
                }
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
        if self.unevaluated == 0 {
            self.demand_function(binding, false);
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
        if a.second == C::DerivedToBase && b.second == C::Pointer {
            return Ok(Ordering::Greater);
        }
        if b.second == C::DerivedToBase && a.second == C::Pointer {
            return Ok(Ordering::Less);
        }
        if a.second == C::DerivedToBase && b.second == C::DerivedToBase {
            let class = |ty| {
                let ty = match self.graph.types.get(ty).kind {
                    TypeKind::Pointer(element) => element,
                    _ => ty,
                };
                self.graph.class_of(ty)
            };
            if let (Some(a), Some(b)) = (class(a.result), class(b.result)) {
                if self.graph.is_base_of(b, a) {
                    return Ok(Ordering::Greater);
                }
                if self.graph.is_base_of(a, b) {
                    return Ok(Ordering::Less);
                }
            }
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
            let af = a.reference_source.is_function_lvalue();
            let bf = b.reference_source.is_function_lvalue();
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
