// Translated from analyzer_conversions.cpp::visible_conversions/user_conversion.
// See NOTICE.
//! Conversion sets preserve path hiding; selection records one call on the original operand.
use super::{
    Error,
    analyzer::Analyzer,
    conversion_templates::ConversionDeduction,
    expression_facts::{
        Conversion, ConversionRank, ExprFacts, ObjectArgument, ObjectSource, ValueCategory,
        ValueConversion,
    },
    graph::{BindingId, EntityId, ScopeId},
    inheritance::BasePathId,
    procedural_calls::{ArgumentSequence, UserConversion},
    specifiers::Explicitness,
    types::{TypeId, TypeKind},
    user_conversions::{ConversionSite, UserCandidate},
};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use crate::{support::source::SourceOffset, syntax::ast};

/// Completed declarations have a unit-owned immutable set. A class still being
/// defined has a query-owned snapshot, which nested lookup cannot overwrite.
pub(super) struct VisibleConversions(ConversionSet);
enum ConversionSet {
    Complete(EntityId),
    Current(Vec<BindingId>),
}
impl VisibleConversions {
    pub(super) fn as_slice<'a>(&'a self, graph: &'a super::graph::Graph) -> &'a [BindingId] {
        match &self.0 {
            ConversionSet::Complete(class) => &graph.conversion_sets[class],
            ConversionSet::Current(bindings) => bindings,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConversionUse {
    Scalar,
    Contextual,
    Builtin,
}

/// Application of a user conversion has an early, void-returning subobject
/// failure; the caller still owns its outer conversion publication.
pub(super) struct SelectedFunctionConversion {
    pub conversion: FunctionConversion,
    available: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum UserConversionUse {
    Scalar,
    Contextual,
}

enum ConversionReceiver {
    Ready(Option<BasePathId>),
    Unavailable,
}

#[derive(Clone, Copy)]
pub(super) enum ConversionTarget {
    Integral,
    IntegralOrEnumeration,
    ObjectPointer,
}

#[derive(Clone, Copy)]
pub(super) struct FunctionConversion {
    pub binding: BindingId,
    pub result: TypeId,
    pub category: ValueCategory,
    pub path: Option<BasePathId>,
    pub after: Conversion,
    pub after_path: Option<BasePathId>,
    pub usage: ConversionUse,
    pub copy: Option<super::copy_initialization::InternalCopy>,
}

impl Analyzer<'_> {
    pub(super) fn using_conversion(
        &mut self,
        prefix: ScopeId,
        target: ast::TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.lookup_location = location;
        // Unlike conversion-function-id expressions, a using-declaration
        // first looks up the shared name and only then filters its result.
        let key = super::function_names::ValueKey::Special(
            super::function_names::SpecialName::Conversion,
        );
        if let Some(class) = self.graph.scope(prefix).owner {
            self.closure_conversion(class, true)?;
        }
        let found = self.using_lookup(prefix, key, location)?;
        // The frozen using-declaration lookup rejects an absent shared name
        // before analyzing the conversion target or introducing any binding.
        if found.binding.is_none() {
            return Err(Error::at(
                location,
                "undeclared name in a using-declaration",
            ));
        }
        let target = self.type_syntax(target, scope)?;
        let mut selected = super::lookup::FunctionCandidates::default();
        for &binding in found.functions.as_slice() {
            let ty = self
                .graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("conversion using without type"))?;
            if matches!(self.graph.types.get(ty).kind, TypeKind::Function { result, .. } if result == target)
            {
                selected.push(binding);
            }
        }
        if selected.as_slice().is_empty() {
            return Err(Error::at(
                location,
                "undeclared name in a using-declaration",
            ));
        }
        let naming = self
            .graph
            .scope(prefix)
            .owner
            .ok_or(Error::Invariant("conversion using without class"))?;
        if !self.using_access_dependent(scope) {
            for &binding in selected.as_slice() {
                self.validate_member_access(binding, naming, None, scope, location)?;
            }
        }
        for &binding in selected.as_slice().iter().rev() {
            let ty = self
                .graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("conversion using without type"))?;
            let signature = self.graph.types.canonical_function(ty)?;
            let mut hidden = false;
            let mut next = 0;
            while let Some(earlier) = self.graph.next_function(scope, key, &mut next) {
                let entity = self.graph.entity(self.graph.binding(earlier).entity);
                if entity.template_head.is_none()
                    && entity
                        .member_function
                        .is_some_and(|key| key.signature == signature)
                {
                    if self.graph.scope(scope).kind == super::graph::ScopeKind::Class
                        && entity.home == Some(scope)
                    {
                        hidden = true;
                    } else if self.graph.binding(earlier).entity
                        != self.graph.binding(binding).entity
                    {
                        return Err(Error::at(
                            location,
                            "using-declaration conflicts with a function of the same signature",
                        ));
                    }
                }
            }
            if !hidden {
                self.graph.import_conversion(scope, binding)?;
            }
        }
        Ok(())
    }

    pub(super) fn named_conversion_lookup(
        &mut self,
        class: EntityId,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<super::lookup::LookupResult, Error> {
        // 12.3.2, 10.2: a conversion-function-id uses canonical result
        // identity, independent of the tokens (including aliases) naming it.
        // Hiding precedes filtering and ordinary object overload resolution.
        let visible = self.visible_conversion_functions(class)?;
        let count = visible.as_slice(&self.graph).len();
        let mut found = super::lookup::LookupResult::default();
        let mut declared = None;
        for index in 0..count {
            let binding = visible.as_slice(&self.graph)[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("conversion without type"))?;
            let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
                return Err(Error::Invariant("conversion without signature"));
            };
            if result != target {
                continue;
            }
            let home = self.graph.binding_scope(binding);
            if declared.is_some_and(|previous| previous != home) {
                return Err(Error::at(
                    location,
                    "ambiguous lookup of a conversion-function-id",
                ));
            }
            declared = Some(home);
            found.binding.get_or_insert(binding);
            found.functions.push(binding);
        }
        if found.binding.is_none() {
            let mut specializations = Vec::new();
            for index in 0..count {
                let binding = visible.as_slice(&self.graph)[index];
                if self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .template_head
                    .is_none()
                {
                    continue;
                }
                if let Some(binding) =
                    self.deduce_conversion_template(binding, target, ConversionDeduction::Named)?
                {
                    let ty = self
                        .graph
                        .binding(binding)
                        .ty
                        .ok_or(Error::Invariant("conversion specialization without type"))?;
                    if matches!(self.graph.types.get(ty).kind, TypeKind::Function { result, .. } if result == target)
                    {
                        specializations.push(binding);
                    }
                }
            }
            for index in 0..specializations.len() {
                if !self.named_conversion_dominated(&specializations, index)? {
                    let binding = specializations[index];
                    found.binding.get_or_insert(binding);
                    found.functions.push(binding);
                }
            }
        }
        if found.binding.is_none() {
            return Err(Error::at(
                location,
                "no conversion function to the named type",
            ));
        }
        Ok(found)
    }

    pub(super) fn conversion_name_lookup(
        &mut self,
        name: ast::Name,
        target: ast::TypeId,
        scope: ScopeId,
    ) -> Result<super::lookup::LookupResult, Error> {
        let location = self.tree.names.get(name).location;
        let prefix = self.expression_prefix(name, scope)?;
        let mut home = prefix.or(Some(scope));
        if prefix.is_none() {
            while let Some(current) = home {
                if self.graph.scope(current).kind == super::graph::ScopeKind::Class {
                    break;
                }
                home = self.graph.scope(current).parent;
            }
        }
        let class = home
            .filter(|&home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
            .and_then(|home| self.graph.scope(home).owner)
            .ok_or_else(|| {
                Error::at(
                    location,
                    "a conversion-function-id that names no class member",
                )
            })?;
        let target = self.type_syntax(target, scope)?;
        let mut found = self.named_conversion_lookup(class, target, location)?;
        found.qualifier = prefix;
        if let Some(binding) = found.binding {
            super::record(&mut self.graph.resolved_names, name.index(), binding);
        }
        Ok(found)
    }

    pub(super) fn unique_conversion_function(
        &mut self,
        class: EntityId,
        target: ConversionTarget,
    ) -> Result<Option<BindingId>, Error> {
        let visible = self.visible_conversion_functions(class)?;
        let count = visible.as_slice(&self.graph).len();
        let mut found = None;
        for index in 0..count {
            let binding = visible.as_slice(&self.graph)[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.explicitness == Explicitness::Explicit || entity.template_head.is_some() {
                continue;
            }
            let signature = entity.ty.ok_or(Error::Invariant("conversion signature"))?;
            let result = self.call_result_view(signature)?.ty;
            let accepts = match target {
                ConversionTarget::Integral => self.is_integral_operand(result),
                ConversionTarget::IntegralOrEnumeration => self.integral_kind(result).is_some(),
                ConversionTarget::ObjectPointer => matches!(self.graph.types.get(result).kind,
                    TypeKind::Pointer(element) if !matches!(self.graph.types.get(element).kind,
                        TypeKind::Function { .. } | TypeKind::Scalar(super::types::Scalar::Void))),
            };
            if accepts {
                if found.is_some() {
                    return Ok(None);
                }
                found = Some(binding);
            }
        }
        Ok(found)
    }

    pub(super) fn convert_unique_class_operand(
        &mut self,
        id: ast::ExprId,
        target: ConversionTarget,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let source = self.graph.expression_facts.get(id)?;
        let Some(class) = self.graph.class_of(source.ty) else {
            return Ok(source);
        };
        let Some(binding) = self.unique_conversion_function(class, target)? else {
            return Ok(source);
        };
        self.convert_class_operand(id, binding, scope)
    }

    // The frozen unique-operand and surrogate paths select declarations without
    // an implicit-object viability or deletion check, then apply access/demand.
    pub(super) fn convert_class_operand(
        &mut self,
        id: ast::ExprId,
        binding: BindingId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        self.builtin_function_conversion(id, binding, None, scope)
    }

    pub(super) fn builtin_function_conversion(
        &mut self,
        id: ast::ExprId,
        binding: BindingId,
        after: Option<Conversion>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let source = self.graph.expression_facts.get(id)?;
        let location = self.tree.expressions.get(id).location;
        let signature = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("conversion signature"))?;
        // convert_class_operand publishes its counted result before checking
        // the receiver. It omits the ordinary deleted-use check.
        let produced = self.call_result(signature)?;
        let after = if let Some(after) = after {
            after
        } else {
            let target = self.decayed_type(produced.ty)?;
            self.standard_conversion(produced, target, false)?
                .ok_or(Error::Invariant("converted operand identity"))?
        };
        let mut selected = FunctionConversion {
            binding,
            result: produced.ty,
            category: produced.category,
            path: None,
            after,
            after_path: None,
            usage: ConversionUse::Builtin,
            copy: None,
        };
        self.graph
            .expression_facts
            .set_function_conversion(id, selected);
        if let ConversionReceiver::Ready(path) =
            self.conversion_function_receiver(source, binding, scope, location)?
        {
            selected.path = path;
            self.graph
                .expression_facts
                .set_function_conversion(id, selected);
        }
        // Unlike apply_user_conversion, a soft nonunique receiver still
        // reaches this use and its throwing event in convert_class_operand.
        self.use_conversion_function(binding)?;
        Ok(produced)
    }

    pub(super) fn visible_conversion_functions(
        &mut self,
        class: EntityId,
    ) -> Result<VisibleConversions, Error> {
        let complete = self.graph.entity(class).complete;
        if !complete || !self.graph.conversion_sets.contains_key(&class) {
            let mut out = Vec::new();
            let mut hiding = HashMap::<(TypeId, bool), u32>::default();
            let mut seen = HashMap::<EntityId, u8>::default();
            let mut functions = HashSet::default();
            let mut work = vec![(class, false)];
            while let Some((current, leaving)) = work.pop() {
                if !leaving {
                    let visits = seen.entry(current).or_default();
                    if *visits == 2 {
                        continue;
                    }
                    *visits += 1;
                    self.closure_conversion(current, true)?;
                }
                for index in (0..self.graph.entity(current).declared_conversions.len()).rev() {
                    let binding = self.graph.entity(current).declared_conversions[index];
                    let key = self.conversion_hiding_key(binding)?;
                    let count = hiding.entry(key).or_default();
                    if leaving {
                        *count -= 1;
                    } else if *count == 0 && functions.insert(self.graph.binding(binding).entity) {
                        out.push(binding);
                    }
                }
                if leaving {
                    continue;
                }
                // Own declarations hide all corresponding base declarations, even
                // when they were themselves hidden by a more derived declaration.
                for index in (0..self.graph.entity(current).declared_conversions.len()).rev() {
                    let binding = self.graph.entity(current).declared_conversions[index];
                    *hiding
                        .entry(self.conversion_hiding_key(binding)?)
                        .or_default() += 1;
                }
                work.push((current, true));
                for base in self.graph.entity(current).bases.iter().rev() {
                    if self.graph.entity(base.class).complete {
                        work.push((base.class, false));
                    }
                }
            }
            if !complete {
                return Ok(VisibleConversions(ConversionSet::Current(out)));
            }
            self.graph.conversion_sets.insert(class, out);
        }
        Ok(VisibleConversions(ConversionSet::Complete(class)))
    }

    fn conversion_hiding_key(&mut self, binding: BindingId) -> Result<(TypeId, bool), Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        let template = entity.template_head.is_some();
        let ty = entity
            .ty
            .ok_or(Error::Invariant("conversion without signature"))?;
        let ty = self.graph.types.canonical_function(ty)?;
        let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("conversion signature"));
        };
        Ok((result, template))
    }

    pub(super) fn conversion_function_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<ArgumentSequence>, Error> {
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference { .. }
        ) {
            return Ok(None);
        }
        self.scalar_conversion_function_sequence(source, target, direct)
    }

    pub(super) fn scalar_conversion_function_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<ArgumentSequence>, Error> {
        // Class value results have additional constructor competition rules.
        let object = match self.graph.types.get(target).kind {
            TypeKind::Reference { element, .. } => element,
            _ => target,
        };
        if self.graph.class_of(object).is_some() {
            return Ok(None);
        }
        let mut candidates = Vec::new();
        self.conversion_function_candidates(source, target, direct, false, &mut candidates)?;
        let Some(best) = self.best_user_candidate(&candidates)? else {
            return Ok(None);
        };
        let after = best
            .after
            .ok_or(Error::Invariant("function candidate second sequence"))?;
        let mut outer = after;
        outer.rank = ConversionRank::UserDefined;
        Ok(Some(ArgumentSequence {
            conversion: outer,
            application: Some(after),
            function: None,
            user_conversion: Some(UserConversion::Function(best.binding)),
        }))
    }

    pub(super) fn conversion_function_candidates(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
        direct_lvalue: bool,
        candidates: &mut Vec<UserCandidate>,
    ) -> Result<(), Error> {
        let Some(class) = self.graph.class_of(source.ty) else {
            return Ok(());
        };
        // user_conversion/direct_reference_conversion require a complete
        // source before admitting its functions as implicit candidates. Named
        // conversion lookup inside the class still uses its current snapshot.
        if !self.complete_class(class)? || self.graph.entity(class).scope.is_none() {
            return Ok(());
        }
        let object = match self.graph.types.get(target).kind {
            TypeKind::Reference { element, .. } => element,
            _ => target,
        };
        let destination = self.graph.class_of(object);
        let required = if destination.is_some() && object == target {
            self.bare_type(object)?
        } else {
            target
        };
        let receiver = ObjectArgument {
            source: ObjectSource::Implicit,
            ty: source.ty,
            category: source.category,
        };
        let visible = self.visible_conversion_functions(class)?;
        let count = visible.as_slice(&self.graph).len();
        for index in 0..count {
            let binding = visible.as_slice(&self.graph)[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.explicitness == Explicitness::Explicit && !direct {
                continue;
            }
            let Some(binding) =
                self.deduce_conversion_template(binding, required, ConversionDeduction::Implicit)?
            else {
                continue;
            };
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            let explicit = entity.explicitness == Explicitness::Explicit;
            if explicit && !direct {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("conversion without signature"))?;
            let produced = self.call_result_view(ty)?;
            if direct_lvalue && produced.category != ValueCategory::Lvalue {
                continue;
            }
            if let Some(destination) = destination
                && object == target
                && self.graph.class_of(produced.ty).is_none_or(|made| {
                    made != destination && !self.graph.is_base_of(destination, made)
                })
            {
                continue;
            }
            let Some(first) = self.object_sequence(receiver, binding, true)? else {
                continue;
            };
            let Some(after) = self.scalar_sequence(produced, target, false)? else {
                continue;
            };
            if direct_lvalue && after.temporary
                || explicit && destination.is_none() && after.second != ValueConversion::None
            {
                continue;
            }
            candidates.push(UserCandidate {
                binding,
                first,
                after: Some(after),
            });
        }
        Ok(())
    }

    pub(super) fn select_conversion_function(
        &mut self,
        source: ExprFacts,
        sequence: ArgumentSequence,
        scope: ScopeId,
        location: SourceOffset,
        usage: UserConversionUse,
    ) -> Result<SelectedFunctionConversion, Error> {
        let Some(UserConversion::Function(binding)) = sequence.user_conversion else {
            return Err(Error::Invariant("conversion function without selection"));
        };
        let entity = self.graph.binding(binding).entity;
        self.check_function_not_deleted(entity, location)?;
        let receiver = self.conversion_function_receiver(source, binding, scope, location)?;
        let (available, path) = match receiver {
            ConversionReceiver::Ready(path) => (true, path),
            ConversionReceiver::Unavailable => (false, None),
        };
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("conversion without signature"))?;
        let produced = self.call_result_view(ty)?;
        let after = sequence.application.ok_or(Error::Invariant(
            "conversion function without second sequence",
        ))?;
        let after_path = if available {
            self.conversion_projection_path(produced, after, location)?
        } else {
            None
        };
        let defer_demand = usage == UserConversionUse::Scalar
            && matches!(
                self.graph.types.get(sequence.conversion.target).kind,
                TypeKind::Class(_)
            );
        if available && !defer_demand {
            self.use_conversion_function(binding)?;
        }
        Ok(SelectedFunctionConversion {
            available,
            conversion: FunctionConversion {
                binding,
                result: produced.ty,
                category: produced.category,
                path,
                usage: match usage {
                    UserConversionUse::Scalar => ConversionUse::Scalar,
                    UserConversionUse::Contextual => ConversionUse::Contextual,
                },
                after_path,
                after,
                copy: None,
            },
        })
    }

    fn conversion_function_receiver(
        &mut self,
        source: ExprFacts,
        binding: BindingId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<ConversionReceiver, Error> {
        let class = self
            .graph
            .class_of(source.ty)
            .ok_or(Error::Invariant("conversion function without class"))?;
        let owner = self
            .member_function_owner(binding)
            .and_then(|ty| self.graph.class_of(ty))
            .ok_or(Error::Invariant("conversion function without owner"))?;
        if self.graph.base_count(class, owner) > 1 {
            self.emit_substitution_failure(
                location,
                "a conversion function of an ambiguous base class subobject",
                false,
            )?;
            return Ok(ConversionReceiver::Unavailable);
        }
        self.check_named_member_access(binding, class, class, scope, location)?;
        let path = if class == owner {
            None
        } else {
            // Access belongs to the introducing conversion declaration. A
            // public using-declaration can expose a private base's function.
            let path = self.graph.retain_base_path(class, owner)?;
            if self.lowering() {
                self.demand_storage_layout(class, location)?;
            }
            Some(path)
        };
        Ok(ConversionReceiver::Ready(path))
    }

    fn use_conversion_function(&mut self, binding: BindingId) -> Result<(), Error> {
        self.demand_specialization_body(binding);
        self.demand_function(binding, true)?;
        self.note_throwing_function(self.graph.binding(binding).entity);
        Ok(())
    }

    pub(super) fn cast_conversion_function(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
        allowed: bool,
    ) -> Result<ExprFacts, Error> {
        let source = self.graph.expression_facts.get(operand)?;
        if allowed
            && let Some(sequence) = self.conversion_function_sequence(source, target, true)?
        {
            self.apply_function_sequence_at(
                operand,
                sequence,
                scope,
                self.tree.expressions.get(id).location,
                ConversionSite::Cast(self.tree.expressions.get(id).location),
            )?;
            return self.new_expression_facts(target, ValueCategory::Prvalue);
        }
        Err(Error::at(
            self.tree.expressions.get(id).location,
            "invalid explicit conversion",
        ))
    }
    fn check_function_binding_temporary(
        &mut self,
        conversion: Conversion,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        // The outer binding owns this check. A selected user function prevents
        // the ordinary fallback that destroys the original class prvalue.
        if conversion.temporary
            && self.graph.class_of(conversion.result).is_some()
            && matches!(
                self.graph.types.get(conversion.target).kind,
                TypeKind::Reference { .. }
            )
        {
            self.check_binding_temporary(
                ExprFacts::new(conversion.result, ValueCategory::Prvalue),
                conversion,
                scope,
                location,
            )?;
        }
        Ok(())
    }

    pub(super) fn apply_function_sequence_at(
        &mut self,
        id: ast::ExprId,
        sequence: ArgumentSequence,
        scope: ScopeId,
        location: SourceOffset,
        site: ConversionSite,
    ) -> Result<(), Error> {
        let source = self.graph.expression_facts.get(id)?;
        if site == ConversionSite::Parameter {
            self.check_function_binding_temporary(sequence.conversion, scope, location)?;
        }
        let selected = self.select_conversion_function(
            source,
            sequence,
            scope,
            location,
            UserConversionUse::Scalar,
        )?;
        let available = selected.available;
        let mut selected = selected.conversion;
        if available && let Some(class) = self.graph.class_of(sequence.conversion.target) {
            let produced = ExprFacts::new(selected.result, selected.category);
            if let Some(copy) = self.operand_constructor(
                class,
                selected.result,
                selected.category,
                true,
                scope,
                location,
            )? {
                selected.after_path =
                    self.conversion_projection_path(produced, copy.binding, location)?;
                selected.copy = Some(super::copy_initialization::InternalCopy::pending(
                    class, copy, location,
                ));
            }
        }
        if available
            && matches!(
                self.graph.types.get(sequence.conversion.target).kind,
                TypeKind::Class(_)
            )
        {
            // The selected result copy precedes use of the conversion function.
            self.use_conversion_function(selected.binding)?;
        }
        if matches!(site, ConversionSite::Call | ConversionSite::Operator) {
            self.check_function_binding_temporary(sequence.conversion, scope, location)?;
        }
        if matches!(site, ConversionSite::Cast(_))
            && matches!(
                self.graph.types.get(sequence.conversion.target).kind,
                TypeKind::Reference { .. }
            )
            && self.graph.analysis.substitution.current().failure.is_some()
        {
            return Err(Error::SubstitutionNull);
        }
        self.graph
            .expression_facts
            .set_function_conversion(id, selected);
        let mut facts = source;
        self.record_conversion(&mut facts, sequence.conversion);
        self.graph.expression_facts.set(id, facts);
        Ok(())
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
    fn ambiguous_receiver_keeps_scalar_and_builtin_continuations_distinct() {
        let source = SourceBuffer::new(
            b"struct B{operator int();};struct L:B{};struct R:B{};struct D:L,R{};\
              extern D value;int scalar=value;int builtin=+value;int condition=value?1:0;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"conversion-receiver-continuation").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let declarations = tree.decl_lists.get(declarations).to_vec();
        let base = cursor.intern_name(b"B").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        for (index, &declaration) in declarations.iter().enumerate() {
            if index < 5 {
                analyzer.declaration(declaration, global).unwrap();
            } else {
                let failure = analyzer
                    .with_substitution_soft(None, |analyzer| {
                        analyzer.declaration(declaration, global)?;
                        Ok(analyzer.graph.analysis.substitution.current().failure)
                    })
                    .unwrap();
                assert_eq!(
                    failure.unwrap().message,
                    "a conversion function of an ambiguous base class subobject"
                );
                let ast::DeclKind::Simple { declarators, .. } =
                    analyzer.tree.declarations.get(declaration).kind
                else {
                    panic!("initialized declaration");
                };
                let initializer = analyzer.tree.init_declarators.get(declarators)[0]
                    .initializer
                    .unwrap();
                let ast::InitializerKind::Assign(mut expression) = initializer.kind else {
                    panic!("initialized expression");
                };
                if let ast::ExprKind::Unary { operand, .. } =
                    analyzer.tree.expressions.get(expression).kind
                {
                    expression = operand;
                }
                if let ast::ExprKind::Conditional { condition, .. } =
                    analyzer.tree.expressions.get(expression).kind
                {
                    expression = condition;
                }
                let selected = analyzer
                    .graph
                    .expression_facts
                    .function_conversion(expression)
                    .unwrap();
                assert!(selected.path.is_none());
                assert_eq!(analyzer.graph.root_demands.as_slice().iter().any(|demand| matches!(
                    demand,
                    super::super::function_demands::Demand::Function { binding, .. } if *binding == selected.binding
                )), index >= 6);
                assert_eq!(
                    analyzer.graph.analysis.potentially_throwing,
                    u32::try_from(index - 5).unwrap()
                );
            }
        }
        let binding = analyzer
            .graph
            .own(global, base, super::super::lookup::Filter::Prefix)
            .unwrap();
        let base = analyzer.graph.binding(binding).entity;
        let conversion = analyzer.graph.entity(base).declared_conversions[0];
        assert!(analyzer.graph.root_demands.as_slice().iter().any(|demand| matches!(
            demand,
            super::super::function_demands::Demand::Function { binding, .. } if *binding == conversion
        )));
    }

    #[test]
    fn result_copy_is_demanded_before_its_conversion_function() {
        let source = SourceBuffer::new(
            b"struct A{int n;A(const A&,int n=3):n(n){}};\
              struct B{operator A&();};void q(A);void f(B&b){q(b);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"conversion-demand-order").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let mut output = Vec::new();
        graph
            .print_semantics(tree, cursor, declarations, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        let copy = output.find("function-definition A::A ").unwrap();
        let conversion = output.find("function-declaration B::operatorA& ").unwrap();
        assert!(copy < conversion);
    }

    #[test]
    fn class_results_retain_selected_transfers_without_replacing_or_reparsing_receivers() {
        use super::super::lookup::Filter;
        let source = SourceBuffer::new(
            br"
            struct A{int n;A();A(const A&)=delete;};
            struct B{int n;B();B(const B&);~B();};
            struct D:B{int m;};
            struct Own{operator A();};
            struct Ref{operator B&();};
            struct Derived{operator D();};
            A own(Own&s){return ((s));}
            B reference(Ref&s){return ((s));}
            B derived(Derived&s){return ((s));}
            B again(Derived&s){return s;}
            struct Unused{B b;};
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"conversion-result-ownership").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expression_count = tree.expressions.mark();
        let declaration_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expression_count);
        assert_eq!(tree.declarations.mark(), declaration_count);
        let mut shared_copy = None;
        for (spelling, source_name, copied, projected) in [
            (b"own".as_slice(), b"Own".as_slice(), false, false),
            (b"reference", b"Ref", true, false),
            (b"derived", b"Derived", true, true),
            (b"again", b"Derived", true, true),
        ] {
            let name = cursor.intern_name(spelling).unwrap();
            let function = graph
                .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
                .entity;
            let definition = graph.entity(function).function_definition.unwrap();
            let ast::DeclKind::Function { body, .. } = tree.declarations.get(definition).kind
            else {
                panic!("function")
            };
            let ast::Body::Parsed(body) = tree.bodies.get(body) else {
                panic!("body")
            };
            let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
                panic!("compound")
            };
            let ast::StmtKind::Return(Some(operand)) =
                tree.statements.get(tree.stmt_lists.get(items)[0]).kind
            else {
                panic!("return")
            };
            let facts = graph.expression_facts.get(operand).unwrap();
            let name = cursor.intern_name(source_name).unwrap();
            let source = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            assert_eq!(graph.class_of(facts.ty), Some(source));
            assert_eq!(facts.category, ValueCategory::Lvalue);
            assert!(facts.class_copy.is_none());
            let step = graph.expression_facts.function_conversion(operand).unwrap();
            assert_eq!(step.after_path.is_some(), projected);
            assert_eq!(step.copy.is_some(), copied);
            if let Some(copy) = step.copy {
                assert!(copy.action.is_none());
                let function = graph
                    .selected_copy_function(copy.class, copy.selection)
                    .unwrap();
                assert_eq!(*shared_copy.get_or_insert(function), function);
            }
            assert!(graph.lower_class_copy(operand).unwrap().is_none());
            assert_eq!(
                graph.expression_type(graph.expression_facts.root(operand)),
                Some(facts.ty)
            );
        }
        for spelling in [b"A".as_slice(), b"Unused".as_slice()] {
            let name = cursor.intern_name(spelling).unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            if spelling == b"Unused" {
                assert!(graph.entity(class).implicit_copies.is_none());
            }
            for &binding in &graph.entity(class).declared_constructors {
                assert!(
                    graph
                        .entity(graph.binding(binding).entity)
                        .constructor
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn named_calls_and_addresses_share_typed_identity_without_demanding_losing_imports() {
        let source = SourceBuffer::new(
            br"typedef int Alias;
            struct Base { protected: operator int() const &; operator int() &&; operator long(); };
            struct Derived:Base { using Base::operator Alias; explicit operator int() const &; };
            int(Derived::*address())()const& { return &Derived::operator Alias; }
            int call(const Derived& d) { return ((d)).operator Alias(); }"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"named-conversions").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(expressions, tree.expressions.mark());
        assert_eq!(nodes, tree.declarations.mark());
        let mut class = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Prefix)
                        .unwrap(),
                )
                .entity
        };
        let base = class(b"Base");
        let derived = class(b"Derived");
        let scope = graph.entity(derived).scope.unwrap();
        let conversions = &graph.entity(derived).declared_conversions;
        assert_eq!(conversions.len(), 2);
        let imported = conversions
            .iter()
            .copied()
            .find(|&binding| graph.entity(graph.binding(binding).entity).home != Some(scope))
            .unwrap();
        let own = conversions
            .iter()
            .copied()
            .find(|&binding| graph.entity(graph.binding(binding).entity).home == Some(scope))
            .unwrap();
        assert_eq!(
            graph
                .lower_conversion_overloads(derived)
                .collect::<Vec<_>>(),
            vec![graph.binding(own).entity]
        );
        assert!(graph.binding_access(imported) == super::super::graph::Access::Public);
        let origin = graph
            .function_origin(graph.binding(imported).entity)
            .unwrap();
        assert!(graph.binding_access(origin) == super::super::graph::Access::Protected);
        let key = super::super::function_names::ValueKey::Special(
            super::super::function_names::SpecialName::Conversion,
        );
        assert_eq!(graph.visible_functions_at(scope, key).count(), 2);
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
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
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        assert_eq!(
            graph.binding(graph.call_binding(call).unwrap()).entity,
            graph.binding(own).entity
        );
        let ast::ExprKind::Call { callee, .. } = tree.expressions.get(call).kind else {
            panic!("call");
        };
        let facts = graph.expression_facts.get(callee).unwrap();
        assert_eq!(facts.member_naming, Some(derived));
        let object = facts.object.unwrap();
        assert_eq!(graph.class_of(object.ty), Some(derived));
        assert_eq!(object.category, ValueCategory::Lvalue);
        let demanded = |binding| {
            graph.root_demands.as_slice().iter().any(|d| {
                matches!(d, super::super::function_demands::Demand::Function { binding: found, .. }
                if graph.binding(*found).entity == graph.binding(binding).entity)
            })
        };
        assert!(demanded(own));
        assert!(!demanded(imported));
        assert!(
            graph
                .entity(base)
                .declared_conversions
                .iter()
                .all(|&binding| !demanded(binding))
        );
    }

    #[test]
    fn unique_operands_keep_original_facts_and_only_demand_the_discovered_functions() {
        let source = SourceBuffer::new(
            b"struct S{operator volatile short&()&&;operator double();};struct X:S{};\
              struct P{operator int* const&();explicit operator long*();};struct Unused{};\
              void f(const X&x,P&p){switch(((x))){default:break;}new int[x];delete ((p));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"unique-operands").unwrap());
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
            .get(*tree.decl_lists.get(declarations).last().unwrap())
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
        let items = tree.stmt_lists.get(items);
        let ast::StmtKind::ConditionalLoop {
            keyword: crate::lex::vocabulary::Keyword::Switch,
            condition: ast::Condition::Expression(switch),
            ..
        } = tree.statements.get(items[0]).kind
        else {
            panic!("switch");
        };
        let ast::StmtKind::Expression(Some(new)) = tree.statements.get(items[1]).kind else {
            panic!("new");
        };
        let count = graph
            .expression_facts
            .allocation(new)
            .unwrap()
            .array
            .unwrap()
            .count;
        let ast::StmtKind::Expression(Some(delete)) = tree.statements.get(items[2]).kind else {
            panic!("delete");
        };
        let ast::ExprKind::Delete {
            operand: pointer, ..
        } = tree.expressions.get(delete).kind
        else {
            panic!("operand");
        };
        let mut selected = Vec::new();
        for id in [switch, count, pointer] {
            let facts = graph.expression_facts.get(id).unwrap();
            let step = graph.expression_facts.function_conversion(id).unwrap();
            assert!(graph.class_of(facts.ty).is_some());
            assert_eq!(facts.category, ValueCategory::Lvalue);
            assert_eq!(step.category, ValueCategory::Lvalue);
            assert!(step.usage == ConversionUse::Builtin);
            assert!(!facts.conversion.unwrap().initialization);
            assert_eq!(step.after.result, facts.conversion.unwrap().result);
            assert_eq!(step.path.is_some(), id != pointer);
            assert_eq!(
                graph
                    .expression_facts
                    .function_conversion(graph.expression_facts.root(id))
                    .unwrap()
                    .binding,
                step.binding
            );
            selected.push(step.binding);
        }
        assert_eq!(selected[0], selected[1]);
        assert_ne!(switch, graph.expression_facts.root(switch));
        for name in [b"S".as_slice(), b"P"] {
            let name = cursor.intern_name(name).unwrap();
            let class = graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Prefix)
                        .unwrap(),
                )
                .entity;
            for &binding in &graph.entity(class).declared_conversions {
                let demanded = graph.root_demands.as_slice().iter().any(|demand|
                    matches!(demand, super::super::function_demands::Demand::Function { binding: found, .. }
                        if *found == binding));
                assert_eq!(demanded, selected.contains(&binding));
            }
        }
        let unused = cursor.intern_name(b"Unused").unwrap();
        let class = graph
            .binding(
                graph
                    .own(graph.global, unused, super::super::lookup::Filter::Prefix)
                    .unwrap(),
            )
            .entity;
        assert!(matches!(
            graph.entity(class).layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }

    #[test]
    fn unevaluated_result_projection_keeps_layout_and_emission_undemanded() {
        let source = SourceBuffer::new(
            b"struct B{int n;};struct D:B{int m;};struct X{operator D*()noexcept;};\
              void take(B*)noexcept;bool f(X&x){return noexcept(take(x));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"unevaluated-result-path").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert!(super::super::function_demands::trailing(&graph).is_empty());
        for name in [b"B", b"D"] {
            let name = cursor.intern_name(name).unwrap();
            let binding = graph
                .own(graph.global, name, super::super::lookup::Filter::Prefix)
                .unwrap();
            assert!(matches!(
                graph.entity(graph.binding(binding).entity).layout,
                super::super::layout::LayoutState::NotStarted
            ));
        }
    }

    #[test]
    fn scalar_result_and_receiver_keep_separate_base_paths() {
        let source = SourceBuffer::new(
            b"struct B{int n;};struct D:B{int m;};struct S{operator D*&();};\
              struct X:S{};int take(B*);int f(X&x){return take(((x)));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"scalar-result-path").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
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
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Call { arguments, .. } = tree.expressions.get(call).kind else {
            panic!("call");
        };
        let operand = tree.expr_lists.get(arguments)[0];
        let selected = graph.expression_facts.function_conversion(operand).unwrap();
        assert!(selected.usage == ConversionUse::Scalar);
        assert!(selected.path.is_some());
        assert!(selected.after_path.is_some());
        assert_ne!(selected.path, selected.after_path);
        assert_eq!(selected.category, ValueCategory::Lvalue);
        assert_eq!(selected.after.second, ValueConversion::DerivedToBase);
        let facts = graph.expression_facts.get(operand).unwrap();
        assert!(graph.class_of(facts.ty).is_some());
        assert!(facts.base_path.is_none());
        assert_eq!(
            graph
                .lower_base_conversion(operand)
                .unwrap()
                .and_then(crate::sema::lowering::BaseProjection::known_offset),
            Some(0)
        );
        assert_ne!(operand, graph.expression_facts.root(operand));
    }

    #[test]
    fn selected_conversion_keeps_source_aliases_and_does_not_demand_hidden_or_losing_members() {
        let source = SourceBuffer::new(
            br"
            struct Base { operator int() const; operator long() const; };
            struct Box:Base { operator int() const; };
            int choose(int); int choose(char*);
            int f(const Box& b) { return choose(((b))); }
            struct Unused { operator int(); };
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"conversion-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
        let mut class = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Prefix)
                        .unwrap(),
                )
                .entity
        };
        let base = class(b"Base");
        let boxed = class(b"Box");
        let unused = class(b"Unused");
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[4])
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Call { arguments, .. } = tree.expressions.get(call).kind else {
            panic!("call");
        };
        let original = tree.expr_lists.get(arguments)[0];
        let root = graph.expression_facts.root(original);
        assert_ne!(original, root);
        let source = graph.expression_facts.get(original).unwrap();
        assert_eq!(graph.class_of(source.ty), Some(boxed));
        assert_eq!(source.category, ValueCategory::Lvalue);
        assert_eq!(graph.expression_type(root), Some(source.ty));
        let selected = graph
            .expression_facts
            .function_conversion(original)
            .unwrap();
        assert_eq!(
            selected.binding,
            graph.entity(boxed).declared_conversions[0]
        );
        assert_eq!(
            graph
                .expression_facts
                .function_conversion(root)
                .unwrap()
                .binding,
            selected.binding
        );
        assert_eq!(source.conversion.unwrap().rank, ConversionRank::UserDefined);
        assert_eq!(selected.after.result, selected.result);
        let demanded = |binding| {
            graph.root_demands.as_slice().iter().any(|d| matches!(d, super::super::function_demands::Demand::Function { binding: found, .. } if *found == binding))
        };
        assert!(demanded(selected.binding));
        for &binding in graph
            .entity(base)
            .declared_conversions
            .iter()
            .chain(&graph.entity(unused).declared_conversions)
        {
            assert!(!demanded(binding));
        }
    }
}
