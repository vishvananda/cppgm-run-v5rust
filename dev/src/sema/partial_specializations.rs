// Translated from sema/template_specializations.cpp::check_partial_arguments
// and deduced_parameters. See NOTICE.
//! Formation rules for partial argument patterns, before selection or demand.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::EntityId,
    templates::HeadId,
    types::{TypeId, TypeKind},
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
    unindexed: HashMap<EntityId, Vec<usize>>,
    indexed: HashMap<(EntityId, PartialIndex), Vec<usize>>,
    positions: HashMap<EntityId, u64>,
    orderings: HashMap<(EntityId, EntityId), bool>,
}
impl PartialLists {
    pub(super) fn add(&mut self, primary: EntityId, key: Option<PartialIndex>, index: usize) {
        if let Some(key) = key {
            *self.positions.entry(primary).or_default() |= 1u64 << key.0;
            self.indexed.entry((primary, key)).or_default().push(index);
        } else {
            self.unindexed.entry(primary).or_default().push(index);
        }
    }
    pub(super) fn positions(&self, primary: EntityId) -> u64 {
        self.positions.get(&primary).copied().unwrap_or_default()
    }
    pub(super) fn candidates(&self, primary: EntityId, keys: &[PartialIndex]) -> Vec<usize> {
        let mut result = self.unindexed.get(&primary).cloned().unwrap_or_default();
        for &key in keys {
            if let Some(indices) = self.indexed.get(&(primary, key)) {
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
                TypeKind::Value { ty, .. } => self
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
        let positions = lists.positions(primary);
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
        let all = self.graph.variable_instances.partials(primary);
        lists
            .candidates(primary, &keys)
            .into_iter()
            .map(|index| all[index].clone())
            .collect()
    }
    pub(super) fn deduce_partial_arguments(
        &mut self,
        head: HeadId,
        pattern: &[TypeId],
        actual: &[TypeId],
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
        let Some(mut arguments) = deduced.into_iter().collect::<Option<Vec<_>>>() else {
            return Ok(None);
        };
        for index in 0..arguments.len() {
            arguments[index] = match self.match_template_argument(
                head,
                index,
                arguments[index],
                &arguments[..index],
                SourceOffset(0),
            ) {
                Ok(ty) => ty,
                Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => return Ok(None),
                Err(error) => return Err(error),
            };
        }
        let given: Vec<_> = arguments.iter().copied().map(Some).collect();
        let mut substitution = Substitution::new(depth, &given);
        for (&pattern, &actual) in pattern.iter().zip(actual) {
            let result = self.substitute_signature_type(pattern, &mut substitution);
            let substituted = match result {
                Ok(Some(ty)) => self.graph.types.canonical_function(ty)?,
                Ok(None) | Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => {
                    return Ok(None);
                }
                Err(error) => return Err(error),
            };
            if substituted != self.graph.types.canonical_function(actual)? {
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
        let key = (first, other.1);
        if let Some(&ordered) = self
            .graph
            .variable_instances
            .partial_lists
            .orderings
            .get(&key)
        {
            return Ok(ordered);
        }
        let ordered = self.compute_partial_ordering(first, pattern, other)?;
        self.graph
            .variable_instances
            .partial_lists
            .orderings
            .insert(key, ordered);
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
        let unique = self.unique_partial_arguments(head)?;
        let mut substitution = Substitution::new(self.graph.templates.depth(head), &unique);
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
        self.supported_fixed_template_head(head)?;
        let mut own = arguments.len() == count;
        for (index, &argument) in arguments.iter().enumerate().take(count) {
            let parameter = self.graph.templates.parameters(head)[index];
            let ty = self
                .graph
                .entity(parameter)
                .ty
                .ok_or(Error::Invariant("partial parameter without type"))?;
            own &= argument == self.graph.types.canonical_function(ty)?;
        }
        if own {
            return Err(Error::at(
                location,
                "a partial specialization whose arguments are its template's own parameters",
            ));
        }
        // Even ordinary heads can name an enclosing head's unexpanded pack.
        for &argument in arguments {
            if self.has_unexpanded_type_pack(argument, true)? {
                return Err(Error::at(
                    location,
                    "a partial specialization's argument names an unexpanded parameter pack",
                ));
            }
        }
        // Ordinary fixed parameters contribute through deducible type/value
        // positions; pack expansion formation remains a separate boundary.
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

    /// Visit a type DAG once. Nested-name specifiers and decltype expressions are
    /// non-deduced contexts; their leading parameter's mere presence does not
    /// make it deducible. Function/array/member-pointer components do.
    fn type_pattern_parameters(&self, arguments: &[TypeId], depth: u32) -> HashSet<usize> {
        let mut work = arguments.to_vec();
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
                }
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.graph.class_templates.instance(*entity)
                        && instance.dependent
                        && self
                            .graph
                            .class_templates
                            .dependent_names
                            .get(instance.primary)
                            .is_none()
                    {
                        work.extend(instance.arguments.iter().copied());
                    }
                }
                TypeKind::Pointer(element) | TypeKind::Reference { element, .. } => {
                    work.push(*element);
                }
                TypeKind::Array {
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
                    work.extend(parameters.iter().copied());
                }
                TypeKind::Pack(elements) => work.extend(elements.iter().copied()),
                TypeKind::Expansion(pattern) => work.push(*pattern),
                TypeKind::BoundPack { .. }
                | TypeKind::Auto
                | TypeKind::Scalar(_)
                | TypeKind::Enum(_)
                | TypeKind::Value { .. } => (),
            }
        }
        deduced
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
}
