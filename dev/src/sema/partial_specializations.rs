// Translated from sema/template_specializations.cpp::check_partial_arguments
// and deduced_parameters. See NOTICE.
//! Formation rules for partial argument patterns, before selection or demand.
use super::{
    Error,
    analyzer::Analyzer,
    graph::EntityId,
    templates::HeadId,
    types::{Cv, TypeId, TypeKind},
};
use crate::support::source::SourceOffset;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum IndexArgument {
    Type(TypeId),
    Template(EntityId),
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct PartialIndex(pub u8, pub IndexArgument);

/// C++ `partial_index_`: unindexed lists append; indexed lists prepend. Positions
/// limit probes to lists selected by these arguments, never unrelated templates.
#[derive(Default)]
pub(super) struct PartialLists {
    primaries: HashMap<EntityId, PrimaryLists>,
}
#[derive(Default)]
struct PrimaryLists {
    unindexed: Vec<usize>,
    indexed: HashMap<PartialIndex, Vec<usize>>,
    positions: u64,
}
impl PartialLists {
    pub(super) fn clear(&mut self, primary: EntityId) {
        self.primaries.remove(&primary);
    }
    pub(super) fn add(&mut self, primary: EntityId, key: Option<PartialIndex>, index: usize) {
        let lists = self.primaries.entry(primary).or_default();
        if let Some(key) = key {
            lists.positions |= 1u64 << key.0;
            lists.indexed.entry(key).or_default().push(index);
        } else {
            lists.unindexed.push(index);
        }
    }
    pub(super) fn positions(&self, primary: EntityId) -> u64 {
        self.primaries
            .get(&primary)
            .map_or(0, |lists| lists.positions)
    }
    pub(super) fn candidates(&self, primary: EntityId, keys: &[PartialIndex]) -> Vec<usize> {
        let Some(lists) = self.primaries.get(&primary) else {
            return Vec::new();
        };
        let mut result = lists.unindexed.clone();
        for &key in keys {
            if let Some(indices) = lists.indexed.get(&key) {
                result.extend(indices.iter().rev().copied());
            }
        }
        result
    }
}

impl Analyzer<'_> {
    pub(super) fn partial_index_key(
        &self,
        head: HeadId,
        arguments: &[TypeId],
    ) -> Option<PartialIndex> {
        for (position, &argument) in (0u8..64).zip(arguments) {
            let Some(&parameter) = self
                .graph
                .templates
                .parameters(head)
                .get(usize::from(position))
            else {
                continue;
            };
            if self
                .graph
                .entity(parameter)
                .template_parameter
                .is_some_and(super::templates::ParameterInfo::is_pack)
            {
                continue;
            }
            let indexable = match self.graph.types.get(argument).kind {
                TypeKind::TemplateParameter { .. } => false,
                TypeKind::Value { ty, .. } | TypeKind::Address { ty, .. } => self
                    .graph
                    .entity(parameter)
                    .template_parameter
                    .and_then(|info| info.value_type)
                    .is_some_and(|declared| declared == ty),
                _ => true,
            };
            if indexable && !self.graph.dependent_type(argument) {
                return Some(PartialIndex(position, IndexArgument::Type(argument)));
            }
        }
        for (position, &argument) in (0u8..64).zip(arguments) {
            let argument = self.graph.unqualified_aliased_type(argument);
            if self.graph.types.get(argument).cv.0 != 0 {
                continue;
            }
            let TypeKind::Class(class) = self.graph.types.get(argument).kind else {
                continue;
            };
            let Some(instance) = self
                .graph
                .class_templates
                .instance(class)
                .filter(|i| i.dependent)
            else {
                continue;
            };
            if self
                .graph
                .class_templates
                .template_entities
                .name(instance.primary)
                .is_some()
                || self
                    .graph
                    .class_templates
                    .dependent_names
                    .get(instance.primary)
                    .is_some()
            {
                continue;
            }
            if let Some(home) = self.graph.entity(instance.primary).home
                && self.dependent_class_scope(home)
            {
                continue;
            }
            return Some(PartialIndex(
                position,
                IndexArgument::Template(instance.primary),
            ));
        }
        None
    }
    pub(super) fn variable_partial_candidates(
        &self,
        primary: EntityId,
        arguments: &[TypeId],
    ) -> Vec<(Vec<TypeId>, EntityId)> {
        let lists = &self.graph.variable_instances.partial_lists;
        let keys = self.partial_candidate_keys(lists.positions(primary), arguments);
        let all = self.graph.variable_instances.partials(primary);
        lists
            .candidates(primary, &keys)
            .into_iter()
            .map(|index| all[index].clone())
            .collect()
    }
    pub(super) fn partial_candidate_keys(
        &self,
        positions: u64,
        arguments: &[TypeId],
    ) -> Vec<PartialIndex> {
        let mut keys = Vec::new();
        for (position, &argument) in (0u8..64).zip(arguments) {
            if positions >> position & 1 == 0 {
                continue;
            }
            keys.push(PartialIndex(position, IndexArgument::Type(argument)));
            if self.graph.types.get(argument).cv.0 == 0
                && let TypeKind::Class(class) = self.graph.types.get(argument).kind
                && let Some(instance) = self.graph.class_templates.instance(class)
            {
                keys.push(PartialIndex(
                    position,
                    IndexArgument::Template(instance.primary),
                ));
            }
        }
        keys
    }
    pub(super) fn matched_partial(
        &mut self,
        primary: EntityId,
        arguments: &[TypeId],
        candidates: Vec<(Vec<TypeId>, EntityId)>,
        location: SourceOffset,
    ) -> Result<(EntityId, Vec<TypeId>), Error> {
        self.matched_enclosing_partial(primary, arguments, candidates, location, None)
    }

    pub(super) fn matched_enclosing_partial(
        &mut self,
        primary: EntityId,
        arguments: &[TypeId],
        candidates: Vec<(Vec<TypeId>, EntityId)>,
        location: SourceOffset,
        enclosing: Option<EntityId>,
    ) -> Result<(EntityId, Vec<TypeId>), Error> {
        let mut matches: Vec<(Vec<TypeId>, EntityId)> = Vec::new();
        let mut best = None;
        for (pattern, candidate) in candidates {
            let head = self
                .graph
                .entity(candidate)
                .template_head
                .ok_or(Error::Invariant("partial without head"))?;
            // Only class candidates install their primary as the raw owner.
            // This includes enclosing-frame substitution before deduction.
            let context = self
                .graph
                .class_templates
                .partials
                .primary(candidate)
                .map(|_| primary);
            let deduced = if let Some(owner) = context {
                self.graph.work.totals.partial_candidates += 1;
                self.with_substitution_owner(Some(owner), |analyzer| {
                    analyzer.deduce_enclosing_partial(head, &pattern, arguments, context, enclosing)
                })?
            } else {
                self.deduce_enclosing_partial(head, &pattern, arguments, context, enclosing)?
            };
            let Some(deduced) = deduced else {
                continue;
            };
            if let Some((_, previous)) = best {
                let previous_pattern = &matches[previous];
                if self.partial_arguments_ordered(candidate, &pattern, previous_pattern)?
                    && !self.partial_arguments_ordered(
                        previous_pattern.1,
                        &previous_pattern.0,
                        &(pattern.clone(), candidate),
                    )?
                {
                    best = Some((deduced, matches.len()));
                }
            } else {
                best = Some((deduced, matches.len()));
            }
            matches.push((pattern, candidate));
        }
        let Some((deduced, index)) = best else {
            return Ok((primary, arguments.to_vec()));
        };
        let winner = &matches[index];
        for other in &matches {
            if other.1 != winner.1
                && (!self.partial_arguments_ordered(winner.1, &winner.0, other)?
                    || self.partial_arguments_ordered(other.1, &other.0, winner)?)
            {
                return Err(Error::at(
                    location,
                    "a specialization matches several partial specializations, none more specialized",
                ));
            }
        }
        Ok((winner.1, deduced))
    }
    fn deduce_enclosing_partial(
        &mut self,
        head: HeadId,
        pattern: &[TypeId],
        arguments: &[TypeId],
        context: Option<EntityId>,
        enclosing: Option<EntityId>,
    ) -> Result<Option<Vec<TypeId>>, Error> {
        let substituted;
        let pattern = if let Some(enclosing) = enclosing {
            substituted = self.enclosing_class_arguments(pattern, enclosing)?;
            &substituted
        } else {
            pattern
        };
        self.deduce_partial_arguments(head, pattern, arguments, context)
    }

    pub(super) fn deduce_partial_arguments(
        &mut self,
        head: HeadId,
        pattern: &[TypeId],
        actual: &[TypeId],
        context: Option<EntityId>,
    ) -> Result<Option<Vec<TypeId>>, Error> {
        if pattern.len() != actual.len() {
            return Ok(None);
        }
        let depth = self.graph.templates.depth(head);
        let mut deduced = vec![None; self.graph.templates.parameters(head).len()];
        for (&pattern, &actual) in pattern.iter().zip(actual) {
            if !self.deduce_signature_type(pattern, actual, depth, &mut deduced, false)? {
                return Ok(None);
            }
        }
        for (index, argument) in deduced.iter_mut().enumerate() {
            if argument.is_none() && self.argument_parameter_pack(head, index) {
                *argument = Some(
                    self.graph
                        .types
                        .intern(TypeKind::Pack(Vec::new()), Cv::default())?,
                );
            }
        }
        let Some(mut arguments) = deduced.into_iter().collect::<Option<Vec<_>>>() else {
            return Ok(None);
        };
        match self.match_argument_list(head, &mut arguments, SourceOffset(0)) {
            Ok(()) => (),
            Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => return Ok(None),
            Err(error) => return Err(error),
        }
        let given: Vec<_> = arguments.iter().copied().map(Some).collect();
        let mut substitution = self.substitution(depth, &given);
        substitution.context = context;
        for (&pattern, &actual) in pattern.iter().zip(actual) {
            let owner = self.graph.analysis.substitution.current().owner;
            let matched = self.with_substitution_soft(owner, |analyzer| {
                let result = analyzer.substitute_signature_type(pattern, &mut substitution);
                let substituted = match result {
                    Ok(Some(_))
                        if analyzer
                            .graph
                            .analysis
                            .substitution
                            .current()
                            .failure
                            .is_some() =>
                    {
                        analyzer.graph.work.totals.substitution_failures += 1;
                        return Ok(false);
                    }
                    Ok(Some(ty)) => analyzer.graph.types.canonical_function(ty)?,
                    Ok(None) => {
                        analyzer.graph.work.totals.substitution_failures +=
                            usize::from(substitution.failure.is_some());
                        return Ok(false);
                    }
                    Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => {
                        analyzer.graph.work.totals.substitution_failures += 1;
                        return Ok(false);
                    }
                    Err(error) => return Err(error),
                };
                Ok(substituted == analyzer.graph.types.canonical_function(actual)?)
            })?;
            if !matched {
                return Ok(None);
            }
        }
        Ok(Some(arguments))
    }
    pub(super) fn partial_arguments_ordered(
        &mut self,
        first: EntityId,
        pattern: &[TypeId],
        other: &(Vec<TypeId>, EntityId),
    ) -> Result<bool, Error> {
        if let Some(ordered) = self.graph.templates.partial_ordering(first, other.1) {
            return Ok(ordered);
        }
        let ordered = self.compute_partial_ordering(first, pattern, other)?;
        self.graph
            .templates
            .record_partial_ordering(first, other.1, ordered);
        Ok(ordered)
    }
    fn compute_partial_ordering(
        &mut self,
        first: EntityId,
        pattern: &[TypeId],
        other: &(Vec<TypeId>, EntityId),
    ) -> Result<bool, Error> {
        let Some(head) = self.graph.entity(first).template_head else {
            return Err(Error::Invariant("ordering without head"));
        };
        let Some(second) = self.graph.entity(other.1).template_head else {
            return Err(Error::Invariant("ordering without head"));
        };
        if pattern.len() != other.0.len() {
            return Ok(false);
        }
        let mut unique = self.unique_partial_arguments(head)?;
        for (index, argument) in unique.iter_mut().enumerate() {
            if self.argument_parameter_pack(head, index) {
                let ty = argument.ok_or(Error::Invariant("ordering without unique argument"))?;
                let expansion = self
                    .graph
                    .types
                    .intern(TypeKind::Expansion(ty), Cv::default())?;
                *argument = Some(
                    self.graph
                        .types
                        .intern(TypeKind::Pack(vec![expansion]), Cv::default())?,
                );
            }
        }
        let mut substitution = self.substitution(self.graph.templates.depth(head), &unique);
        let mut deduced = vec![None; self.graph.templates.parameters(second).len()];
        for (&pattern, &other) in pattern.iter().zip(&other.0) {
            let Some(actual) = self.substitute_signature_type(pattern, &mut substitution)? else {
                return Ok(false);
            };
            if !self.deduce_signature_type(
                other,
                actual,
                self.graph.templates.depth(second),
                &mut deduced,
                false,
            )? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn check_type_partial_arguments(
        &mut self,
        head: HeadId,
        arguments: &[TypeId],
        location: SourceOffset,
    ) -> Result<(), Error> {
        // Defaults are prohibited even if the argument list would also be
        // the primary's own list. Earlier argument formation errors win.
        let count = self.graph.templates.parameters(head).len();
        for index in 0..count {
            if self.graph.templates.default_argument(head, index).is_some() {
                return Err(Error::at(
                    location,
                    "a default template argument in a partial specialization's template parameter list",
                ));
            }
        }
        let mut own = arguments.len() == count;
        for (index, &argument) in arguments.iter().enumerate().take(count) {
            let parameter = self.graph.templates.parameters(head)[index];
            let mut ty = self
                .graph
                .entity(parameter)
                .ty
                .ok_or(Error::Invariant("partial parameter without type"))?;
            if self.argument_parameter_pack(head, index) {
                let expansion = self
                    .graph
                    .types
                    .intern(TypeKind::Expansion(ty), Cv::default())?;
                ty = self
                    .graph
                    .types
                    .intern(TypeKind::Pack(vec![expansion]), Cv::default())?;
            }
            own &= argument == self.graph.types.canonical_function(ty)?;
        }
        if own {
            return Err(Error::at(
                location,
                "a partial specialization whose arguments are its template's own parameters",
            ));
        }
        // A primary's pack is one canonical argument partition, but each of
        // its elements is an argument for the formation rules of 14.5.5p8.
        for (index, &argument) in arguments.iter().enumerate() {
            let elements = match &self.graph.types.get(argument).kind {
                TypeKind::Pack(elements) => elements.as_slice(),
                _ => std::slice::from_ref(&argument),
            };
            for (position, &element) in elements.iter().enumerate() {
                if matches!(self.graph.types.get(element).kind, TypeKind::Expansion(_)) {
                    if index + 1 != arguments.len() || position + 1 != elements.len() {
                        return Err(Error::at(
                            location,
                            "a pack expansion other than a partial specialization's last argument",
                        ));
                    }
                } else if self.has_unexpanded_type_pack(element, true)? {
                    return Err(Error::at(
                        location,
                        "a partial specialization's argument names an unexpanded parameter pack",
                    ));
                }
            }
        }
        let depth = self.graph.templates.depth(head);
        let deduced = self.type_pattern_parameters(arguments, depth);
        if (0..count).any(|index| !deduced.contains(&index)) {
            return Err(Error::at(
                location,
                "a partial specialization's template parameter that its arguments do not deduce",
            ));
        }
        Ok(())
    }

    /// A specialized value cannot fix a parameter whose substituted type
    /// still depends on this partial's parameters (14.5.5p8).
    pub(super) fn check_partial_value_types(
        &mut self,
        primary: HeadId,
        head: HeadId,
        arguments: &[TypeId],
        location: SourceOffset,
    ) -> Result<(), Error> {
        let depth = self.graph.templates.depth(head);
        let given: Vec<_> = arguments.iter().copied().map(Some).collect();
        let mut substitution = self.substitution(self.graph.templates.depth(primary), &given);
        for (index, &argument) in arguments.iter().enumerate() {
            let Some(&parameter) = self.graph.templates.parameters(primary).get(index) else {
                break;
            };
            let Some(info) = self.graph.entity(parameter).template_parameter else {
                continue;
            };
            let Some(ty) = info.value_type.filter(|&ty| self.graph.dependent_type(ty)) else {
                continue;
            };
            if info.is_pack() {
                continue;
            }
            if let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(argument).kind
                && (self
                    .graph
                    .entity(entity)
                    .template_parameter
                    .is_some_and(|info| info.index(depth).is_some())
                    || self.named_bound_parameter(argument, depth).is_some())
            {
                continue;
            }
            let Some(ty) = self.substitute_signature_type(ty, &mut substitution)? else {
                return Err(Error::Invariant("partial value type substitution failed"));
            };
            if depth == 0 && self.graph.dependent_type(ty)
                || depth != 0 && !self.type_pattern_parameters(&[ty], depth).is_empty()
            {
                return Err(Error::at(
                    location,
                    "a specialized non-type argument for a parameter whose type depends on the partial specialization's parameters",
                ));
            }
        }
        Ok(())
    }

    /// Visit a type DAG once. Nested-name specifiers and decltype expressions are
    /// non-deduced contexts; their leading parameter's mere presence does not
    /// make it deducible. Function/array/member-pointer components do.
    fn type_pattern_parameters(&self, arguments: &[TypeId], depth: u32) -> HashSet<usize> {
        let mut work = Vec::new();
        self.deducible_pattern_list(arguments, true, &mut work);
        let mut visited = HashSet::new();
        let mut deduced = HashSet::new();
        while let Some(ty) = work.pop() {
            if !visited.insert(ty) {
                continue;
            }
            match &self.graph.types.get(ty).kind {
                TypeKind::TemplateParameter { entity, .. } => {
                    if let Some(index) = self
                        .graph
                        .entity(*entity)
                        .template_parameter
                        .and_then(|parameter| parameter.index(depth))
                    {
                        deduced.insert(index);
                    }
                    if let Some(index) = self.named_bound_parameter(ty, depth) {
                        deduced.insert(index);
                    }
                }
                TypeKind::Class(entity) => {
                    if let Some(aliased) = self.graph.class_templates.aliases.aliased(*entity) {
                        work.push(aliased);
                        continue;
                    }
                    if let Some(instance) = self.graph.class_templates.instance(*entity)
                        && instance.dependent
                        && self
                            .graph
                            .class_templates
                            .dependent_names
                            .get(instance.primary)
                            .is_none()
                    {
                        if let Some(name) = self
                            .graph
                            .class_templates
                            .template_entities
                            .name(instance.primary)
                        {
                            work.push(name);
                        }
                        self.deducible_pattern_list(&instance.arguments, true, &mut work);
                    }
                }
                TypeKind::Pointer(element)
                | TypeKind::Complex(element)
                | TypeKind::Reference { element, .. } => {
                    work.push(*element);
                }
                TypeKind::Array {
                    element, operand, ..
                }
                | TypeKind::Vector {
                    element, operand, ..
                } => {
                    work.push(*element);
                    if let Some(index) =
                        operand.and_then(|ty| self.named_bound_parameter(ty, depth))
                    {
                        deduced.insert(index);
                    }
                }
                TypeKind::MemberPointer { owner, element } => work.extend([*owner, *element]),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    self.deducible_pattern_list(parameters, false, &mut work);
                }
                TypeKind::Pack(elements) => self.deducible_pattern_list(elements, true, &mut work),
                TypeKind::Expansion(pattern) => work.push(*pattern),
                TypeKind::BoundPack { .. }
                | TypeKind::Auto(_)
                | TypeKind::Scalar(_)
                | TypeKind::Template(_)
                | TypeKind::Enum(_)
                | TypeKind::Value { .. }
                | TypeKind::Address { .. }
                | TypeKind::BlockPointer(_) => (),
            }
        }
        deduced
    }

    /// A non-trailing expansion makes a template argument list wholly
    /// non-deduced; in a function parameter list only that expansion is skipped.
    fn deducible_pattern_list(
        &self,
        types: &[TypeId],
        template_arguments: bool,
        work: &mut Vec<TypeId>,
    ) {
        let start = work.len();
        let mut elements = types
            .iter()
            .flat_map(|ty| match &self.graph.types.get(*ty).kind {
                TypeKind::Pack(pack) => pack.as_slice(),
                _ => std::slice::from_ref(ty),
            })
            .peekable();
        while let Some(&ty) = elements.next() {
            if elements.peek().is_some()
                && matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_))
            {
                if template_arguments {
                    work.truncate(start);
                    return;
                }
            } else {
                work.push(ty);
            }
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
    fn deducibility_walk_handles_deep_shared_type_dags() {
        let source = SourceBuffer::new(b"template<class T>extern int value;".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"partial-dag").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"value").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let binding = analyzer
            .graph
            .own(
                analyzer.graph.global,
                name,
                super::super::lookup::Filter::Ordinary,
            )
            .unwrap();
        let entity = analyzer.graph.binding(binding).entity;
        let head = analyzer.graph.entity(entity).template_head.unwrap();
        let parameter = analyzer.graph.templates.parameters(head)[0];
        let mut ty = analyzer.graph.entity(parameter).ty.unwrap();
        for _ in 0..2048 {
            ty = analyzer
                .graph
                .types
                .intern(
                    TypeKind::Function {
                        result: ty,
                        parameters: vec![ty, ty],
                        variadic: false,
                        cv: super::super::types::Cv::default(),
                        reference: None,
                    },
                    super::super::types::Cv::default(),
                )
                .unwrap();
        }
        assert_eq!(
            analyzer.type_pattern_parameters(&[ty, ty], analyzer.graph.templates.depth(head)),
            HashSet::from([0])
        );
    }

    #[test]
    fn non_trailing_expansions_obey_their_list_context() {
        let source = SourceBuffer::new(b"template<class T,class... Ts>struct P;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"deducible-lists").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"P").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer
            .graph
            .own(global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let head = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity)
            .template_head
            .unwrap();
        let parameters = analyzer.graph.templates.parameters(head);
        let first = analyzer.graph.entity(parameters[0]).ty.unwrap();
        let pack = analyzer.graph.entity(parameters[1]).ty.unwrap();
        let expansion = analyzer
            .graph
            .types
            .intern(TypeKind::Expansion(pack), Cv::default())
            .unwrap();
        let depth = analyzer.graph.templates.depth(head);
        assert!(
            analyzer
                .type_pattern_parameters(&[expansion, first], depth)
                .is_empty()
        );
        assert_eq!(
            analyzer.type_pattern_parameters(&[first, expansion], depth),
            HashSet::from([0, 1])
        );
        let function = analyzer
            .graph
            .types
            .intern(
                TypeKind::Function {
                    result: first,
                    parameters: vec![expansion, first],
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )
            .unwrap();
        assert_eq!(
            analyzer.type_pattern_parameters(&[function], depth),
            HashSet::from([0])
        );
    }
}
