// Translated from analyzer_expressions.cpp and conversions.cpp. See NOTICE.
//! Function designations, target-directed selection and standard sequence ordering.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, ReferenceSource, ValueCategory as V,
        ValueConversion as C,
    },
    function_templates::TemplateOrdering,
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
        if self.graph.binding(binding).kind != BindingKind::Function {
            return None;
        }
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
        let syntax_item = *self.tree.names.get(name);
        let node = &syntax_item;
        if !node.global && self.tree.name_lists.get(node.components).len() == 1 {
            return Ok(None);
        }
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .copied()
            .ok_or(Error::Invariant("empty address name"))?;
        let location = node.location;
        let prefix = self
            .prefix(name, scope)?
            .ok_or(Error::Invariant("qualified address without prefix"))?;
        // Only a class-qualified operand has the pointer-to-member form.
        // Other prefixes belong to the ordinary operand's single lookup.
        if self.graph.scope(prefix).kind != ScopeKind::Class {
            return Ok(None);
        }
        self.lookup_location = location;
        let mut found = if let ast::NameKind::Conversion(target) = last.kind {
            self.conversion_name_lookup(name, target, scope)?
        } else {
            let key = Self::expression_name(last.kind)?;
            self.prepare_assignment_lookup(prefix, key, false)?;
            self.qualified_result(prefix, key, Filter::Ordinary)
                .map_err(|error| match error {
                    Error::Diagnostic {
                        message: "ambiguous name",
                        ..
                    } if self.graph.scope(prefix).kind == ScopeKind::Class => {
                        Error::at(location, "ambiguous member")
                    }
                    other => other,
                })?
        };
        let address_functions = (!found.functions.as_slice().is_empty())
            .then(|| self.graph.overloads.add(found.functions.as_slice()))
            .transpose()?;
        if last.arguments.is_some() {
            self.explicit_function_specializations(last, &mut found, scope, false)?;
        }
        let binding = found
            .binding
            .ok_or_else(|| Error::at(location, "undeclared name"))?;
        super::record(&mut self.graph.resolved_names, name.index(), binding);
        if self.graph.scope(prefix).kind == ScopeKind::Class
            && let Some(naming) = self.graph.scope(prefix).owner
            && let Some(facts) = self.qualified_data_address(operand, binding, naming, scope)?
        {
            return Ok(Some(facts));
        }
        if self.graph.binding(binding).kind != BindingKind::Function
            && let Some(naming) = self.graph.scope(prefix).owner
            && self.graph.scope(prefix).kind == ScopeKind::Class
            && self.graph.binding_scope(binding) != prefix
        {
            self.check_named_member_access(binding, naming, naming, scope, location)?;
        }
        if self.graph.binding(binding).kind != BindingKind::Function {
            return Ok(None);
        }
        if found.functions.as_slice().len() == 1 {
            self.check_intrinsic_designation(binding, node.location)?;
        }
        let concrete = found.functions.as_slice().len() == 1
            && !self.graph.function_templates.requires_target(binding)
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .template_head
                .is_none();
        if concrete && last.arguments.is_none() {
            let function = self.graph.binding(binding).entity;
            self.check_function_not_deleted(function, node.location)?;
            self.require_deduced_result(function, node.location)?;
            let naming = self
                .graph
                .scope(prefix)
                .owner
                .ok_or(Error::Invariant("class address without naming class"))?;
            self.check_named_member_access(binding, naming, naming, scope, node.location)?;
            if let Some(failure) = self.graph.analysis.substitution.current().failure {
                return Err(Error::SubstitutionRejected(failure));
            }
            let signature = self.graph.types.canonical_function(
                self.graph
                    .entity(function)
                    .ty
                    .ok_or(Error::Invariant("function address without type"))?,
            )?;
            let origin = self.graph.function_origin(function)?;
            let member = self.member_function_owner(binding).is_some();
            let mut inner = self.new_expression_facts(signature, V::Lvalue)?;
            inner.function = Some(origin);
            self.graph.expression_facts.set(operand, inner);
            super::record(&mut self.graph.expression_types, operand.index(), signature);
            if !member {
                self.demand_function(binding, false)?;
                self.demand_specialization_body(binding);
                let pointer = self.graph.types.pointer(signature)?;
                let mut facts = self.new_expression_facts(pointer, V::Prvalue)?;
                facts.function = Some(origin);
                return Ok(Some(facts));
            }
            let functions =
                address_functions.ok_or(Error::Invariant("member address without declarations"))?;
            self.graph
                .expression_facts
                .set_function_declarations(operand, functions);
            self.graph
                .expression_facts
                .set_function_declarations(id, functions);
            let pointer = self.function_address_type(binding, signature)?;
            let mut facts = self.new_expression_facts(pointer, V::Prvalue)?;
            facts.function = Some(origin);
            self.graph.expression_facts.set(id, facts);
            super::record(&mut self.graph.expression_types, id.index(), pointer);
            self.demand_function(binding, false)?;
            self.demand_specialization_body(binding);
            return Ok(Some(facts));
        }
        if concrete {
            // The explicit template-id first forms its operand; unlike the
            // direct ordinary branch, its access check follows that demand.
            let function = self.graph.binding(binding).entity;
            let origin = self.graph.function_origin(function)?;
            let mut inner = self.function_designation(binding, found.functions.as_slice())?;
            self.concrete_function_designation(operand, binding, &mut inner)?;
            self.graph.expression_facts.set(operand, inner);
            super::record(&mut self.graph.expression_types, operand.index(), inner.ty);
            if self.callee_name != Some(operand) {
                self.require_deduced_result(function, self.tree.expressions.get(operand).location)?;
            }
            let signature = self.graph.types.canonical_function(
                self.graph
                    .entity(function)
                    .ty
                    .ok_or(Error::Invariant("template address without function type"))?,
            )?;
            inner.ty = signature;
            self.graph.expression_facts.set(operand, inner);
            super::record(&mut self.graph.expression_types, operand.index(), signature);
            self.check_function_not_deleted(function, self.tree.expressions.get(operand).location)?;
            self.demand_function(binding, false)?;
            self.demand_specialization_body(binding);
            let naming = self
                .graph
                .scope(prefix)
                .owner
                .ok_or(Error::Invariant("template address without naming class"))?;
            if let Some(declaration) = self.selected_function_declaration(
                binding,
                self.graph.overloads.get(
                    address_functions
                        .ok_or(Error::Invariant("template address without declarations"))?,
                ),
            ) {
                self.check_named_member_access(
                    declaration,
                    naming,
                    naming,
                    scope,
                    self.tree.expressions.get(id).location,
                )?;
            }
            if let Some(failure) = self.graph.analysis.substitution.current().failure {
                return Err(Error::SubstitutionRejected(failure));
            }
            let pointer = self.function_address_type(binding, signature)?;
            let mut facts = self.new_expression_facts(pointer, V::Prvalue)?;
            facts.function = Some(origin);
            return Ok(Some(facts));
        }
        let address = self.graph.expression_facts.add_member_address(
            super::expression_facts::MemberAddress {
                naming: (self.graph.scope(prefix).kind == ScopeKind::Class)
                    .then_some(self.graph.scope(prefix).owner)
                    .flatten(),
                scope,
                functions: address_functions
                    .ok_or(Error::Invariant("function address without lookup"))?,
                location: None,
            },
        )?;
        let mut inner = self.function_designation(binding, found.functions.as_slice())?;
        inner.member_address = Some(address);
        self.graph.expression_facts.set(operand, inner);
        super::record(&mut self.graph.expression_types, operand.index(), inner.ty);
        // Pending sets have no pointer type until target selection.
        let mut facts = self.new_expression_facts(inner.ty, V::Prvalue)?;
        facts.functions = inner.functions;
        facts.member_address = Some(address);
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
            .entity(self.graph.binding(binding).entity)
            .ty
            .ok_or(Error::Invariant("function without type"))?;
        let mut facts = self.new_expression_facts(ty, V::Lvalue)?;
        facts.functions = Some(self.graph.overloads.add(functions)?);
        Ok(facts)
    }
    pub(super) fn concrete_function_designation(
        &mut self,
        id: ast::ExprId,
        binding: BindingId,
        facts: &mut ExprFacts,
    ) -> Result<(), Error> {
        if let Some(functions) = facts.functions {
            self.graph
                .expression_facts
                .set_function_declarations(id, functions);
        }
        facts.functions = None;
        facts.function = Some(
            self.graph
                .function_origin(self.graph.binding(binding).entity)?,
        );
        Ok(())
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
        self.check_bound_function(id)
    }
    pub(super) fn check_bound_function(&self, id: ast::ExprId) -> Result<(), Error> {
        let facts = self.graph.expression_facts.get(id)?;
        if facts.object.is_some()
            && (facts.functions.is_some()
                || facts.function.is_some()
                || facts.destructor.is_some()
                || matches!(
                    self.graph.types.get(facts.ty).kind,
                    TypeKind::Function { .. }
                ))
        {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "a member function must be called",
            ));
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
            && (self.member_function_owner(*binding).is_none() || facts.member_address.is_some()))
        .then_some(*binding)
    }
    pub(super) fn matching_function(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
    ) -> Result<Option<BindingId>, Error> {
        self.matching_function_cast(facts, target, false)
    }
    pub(super) fn matching_function_cast(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
        explicit: bool,
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
            let candidate_owner = self.member_function_owner(binding);
            if candidate_owner != owner {
                let Some((candidate, owner)) = candidate_owner.zip(owner) else {
                    continue;
                };
                if !(self.derived_class_conversion(candidate, owner)?
                    || explicit && self.derived_class_conversion(owner, candidate)?)
                {
                    continue;
                }
            }
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some()
                || self.graph.function_templates.requires_target(binding)
            {
                template = true;
                continue;
            }
            let ty = entity.ty.ok_or(Error::Invariant("function without type"))?;
            if self.graph.types.canonical_function(ty)? == target {
                if found.is_some_and(|old| {
                    self.graph.binding(old).entity != self.graph.binding(binding).entity
                }) {
                    return Ok(None);
                }
                found = Some(binding);
            }
        }
        if found.is_none() && template {
            let count = self.graph.types.function_parameter_count(target)?;
            let mut candidates = Vec::new();
            for index in 0..self.graph.overloads.get(set).len() {
                let binding = self.graph.overloads.get(set)[index];
                if self.member_function_owner(binding).is_some() != owner.is_some() {
                    continue;
                }
                if let Some(binding) = self.deduce_function_target(binding, target)?
                    && self.graph.binding(binding).ty == Some(target)
                    && !candidates.iter().any(|&old| {
                        self.graph.binding(old).entity == self.graph.binding(binding).entity
                    })
                {
                    candidates.push(binding);
                }
            }
            for &binding in &candidates {
                if let Some(old) = found {
                    if self.more_specialized_function_target(binding, old, count)? {
                        found = Some(binding);
                    }
                } else {
                    found = Some(binding);
                }
            }
            if let Some(best) = found {
                for binding in candidates {
                    if binding != best
                        && !self.more_specialized_function_target(best, binding, count)?
                    {
                        return Ok(None);
                    }
                }
            }
        }
        Ok(found)
    }
    fn more_specialized_function_target(
        &mut self,
        first: super::graph::BindingId,
        second: super::graph::BindingId,
        count: usize,
    ) -> Result<bool, Error> {
        let first = self
            .graph
            .function_templates
            .primary(self.graph.binding(first).entity)
            .ok_or(Error::Invariant("target specialization without primary"))?;
        let second = self
            .graph
            .function_templates
            .primary(self.graph.binding(second).entity)
            .ok_or(Error::Invariant("target specialization without primary"))?;
        // Address selection asks the reverse direction only after a successful
        // forward query. Ordinary call ordering retains its two-query policy.
        Ok(self.template_at_least_as_specialized(
            first,
            second,
            TemplateOrdering::FunctionType(count),
        )? && !self.template_at_least_as_specialized(
            second,
            first,
            TemplateOrdering::FunctionType(count),
        )?)
    }
    pub(super) fn select_function_designation(
        &mut self,
        id: ast::ExprId,
        binding: BindingId,
    ) -> Result<(), Error> {
        let function = self.graph.binding(binding).entity;
        self.check_function_not_deleted(function, self.tree.expressions.get(id).location)?;
        let ty = self.graph.types.canonical_function(
            self.graph
                .entity(function)
                .ty
                .ok_or(Error::Invariant("function without type"))?,
        )?;
        let root = self.graph.expression_facts.root(id);
        if self.graph.expression_facts.get(root)?.functions.is_none() {
            self.demand_function(binding, false)?;
            self.demand_specialization_body(binding);
            return Ok(());
        }
        let address = self.graph.expression_facts.get(root)?.member_address;
        let inner = match self.tree.expressions.get(root).kind {
            ast::ExprKind::Unary { op, operand }
                if matches!(op.token.as_punctuator(), Some(P::BitAnd | P::Multiply)) =>
            {
                Some(operand)
            }
            _ => None,
        };
        if let Some(inner) = inner {
            let mut facts = self.graph.expression_facts.get(inner)?;
            facts.ty = ty;
            self.concrete_function_designation(inner, binding, &mut facts)?;
            self.graph.expression_facts.set(inner, facts);
            super::record(&mut self.graph.expression_types, inner.index(), ty);
        }
        let mut facts = self.graph.expression_facts.get(root)?;
        facts.ty = if inner.is_some() {
            if address.is_some() {
                self.function_address_type(binding, ty)?
            } else {
                self.graph.types.pointer(ty)?
            }
        } else {
            ty
        };
        self.graph.expression_facts.set(root, facts);
        super::record(&mut self.graph.expression_types, root.index(), facts.ty);
        if inner.is_some()
            && let Some(address) = address
        {
            let address = self.graph.expression_facts.member_address(address);
            if let Some(naming) = address.naming
                && let Some(declaration) = self.selected_function_declaration(
                    binding,
                    self.graph.overloads.get(address.functions),
                )
            {
                self.check_named_member_access(
                    declaration,
                    naming,
                    naming,
                    address.scope,
                    address
                        .location
                        .unwrap_or(self.tree.expressions.get(id).location),
                )?;
            }
        }
        let mut facts = self.graph.expression_facts.get(root)?;
        self.concrete_function_designation(root, binding, &mut facts)?;
        self.graph.expression_facts.set(root, facts);
        self.demand_function(binding, false)?;
        self.demand_specialization_body(binding);
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
        if (a.rank == R::UserDefined) != (b.rank == R::UserDefined) {
            return Ok(b.rank.cmp(&a.rank));
        }
        if a.list_initialization != b.list_initialization {
            return Ok(a.list_initialization.cmp(&b.list_initialization));
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
        if a.second == C::MemberToDerived && b.second == C::MemberToDerived {
            let owner = |ty| match self.graph.types.get(ty).kind {
                TypeKind::MemberPointer { owner, .. } => self.graph.class_of(owner),
                _ => None,
            };
            if let (Some(a), Some(b)) = (owner(a.result), owner(b.result)) {
                if self.graph.is_base_of(a, b) {
                    return Ok(Ordering::Greater);
                }
                if self.graph.is_base_of(b, a) {
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
            let unqualified_object =
                matches!(a.reference_source, ReferenceSource::UnqualifiedObject)
                    || matches!(b.reference_source, ReferenceSource::UnqualifiedObject);
            if ak != bk && af == bf && (af || !unqualified_object) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn failed_explicit_argument_stops_later_demands_and_callee_publication() {
        for suffix in ["", ",int[1/0]"] {
            let source = SourceBuffer::new(
                format!(
                    "struct Key{{private:template<class>struct Hidden;}};\
                 template<template<class>class C,class U=int>void only();\
                 int result=only<Key::template Hidden{suffix}>();"
                )
                .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"failed-call-argument").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_semantics(true);
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let scope = analyzer.graph.global;
            let declarations = analyzer.tree.decl_lists.get(declarations).to_vec();
            for &declaration in &declarations[..2] {
                analyzer.declaration(declaration, scope).unwrap();
            }
            let ast::DeclKind::Simple { declarators, .. } =
                analyzer.tree.declarations.get(declarations[2]).kind
            else {
                panic!("initializer declaration");
            };
            let initializer = analyzer.tree.init_declarators.get(declarators)[0]
                .initializer
                .unwrap();
            let ast::InitializerKind::Assign(call) = initializer.kind else {
                panic!("call initializer");
            };
            let ast::ExprKind::Call { callee, .. } = analyzer.tree.expressions.get(call).kind
            else {
                panic!("call syntax");
            };
            let expressions = analyzer.graph.work.totals.expressions;
            let error = analyzer
                .with_substitution_soft(None, |analyzer| analyzer.typed_expression(call, scope))
                .unwrap_err();
            assert!(
                matches!(error, Error::SubstitutionRejected(failure) if failure.message == "inaccessible member")
            );
            assert_eq!(analyzer.graph.work.totals.expressions, expressions);
            assert!(analyzer.graph.expression_facts.known(callee).is_none());
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
            assert!(
                analyzer
                    .graph
                    .analysis
                    .substitution
                    .current()
                    .failure
                    .is_none()
            );
        }
    }

    #[test]
    fn target_selection_counts_functions_rather_than_binding_handles() {
        let source = SourceBuffer::new(b"int f(int);int other(int);".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"target-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"f".as_slice(), b"other"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let [first, other] =
            names.map(|name| analyzer.graph.own(scope, name, Filter::Ordinary).unwrap());
        let entity = analyzer.graph.binding(first).entity;
        let ty = analyzer.graph.binding(first).ty.unwrap();
        let handle = analyzer.graph.function_handle(first, entity, ty).unwrap();
        assert_ne!(first, handle);
        for (bindings, ambiguous) in [
            (&[first, handle][..], false),
            (&[first, handle, other][..], true),
        ] {
            let mut facts = ExprFacts::new(ty, V::Lvalue);
            facts.functions = Some(analyzer.graph.overloads.add(bindings).unwrap());
            let selected = analyzer.matching_function(facts, ty).unwrap();
            if ambiguous {
                assert!(selected.is_none());
            } else {
                assert_eq!(analyzer.graph.binding(selected.unwrap()).entity, entity);
            }
        }
    }
}
