// Translated from template_packs.cpp::deduce_elements/deduce_list. See NOTICE.
//! Deduction collects canonical pack elements without changing a pattern or frame.
use super::{
    Error,
    analyzer::Analyzer,
    types::{Cv, TypeId, TypeKind},
};

impl Analyzer<'_> {
    #[inline(never)]
    pub(super) fn deduce_bound_pack(
        &mut self,
        pattern: TypeId,
        actual: TypeId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
        extra_cv: bool,
    ) -> Result<bool, Error> {
        let TypeKind::BoundPack {
            arguments: pack, ..
        } = self.graph.types.get(pattern).kind
        else {
            return Err(Error::Invariant("deducing non-bound pack"));
        };
        let element = self.deduced_pack_element.and_then(|index| {
            self.graph
                .types
                .pack_elements(pack)
                .ok()?
                .get(index)
                .copied()
        });
        if let Some(element) = element {
            let element = self.graph.types.qualify(element, self.type_cv(pattern))?;
            self.deduce_signature_type(element, actual, depth, arguments, extra_cv)
        } else {
            Ok(pattern == actual)
        }
    }
    #[inline(never)]
    pub(super) fn deduce_pack_elements(
        &mut self,
        pattern: TypeId,
        count: usize,
        counted: bool,
        depth: u32,
        arguments: &mut [Option<TypeId>],
        mut deduce: impl FnMut(&mut Self, usize, &mut [Option<TypeId>]) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        let packs = self.type_pack_pattern(pattern, true);
        let indices: Vec<_> = packs
            .parameters
            .iter()
            .filter_map(|&entity| {
                self.graph
                    .entity(entity)
                    .context()
                    .template_parameter
                    .and_then(|info| info.index(depth))
            })
            .collect();
        if indices.is_empty() {
            return Ok(true);
        }
        if counted
            && packs.bound.iter().any(|&bound| {
                let TypeKind::BoundPack { arguments, .. } = self.graph.types.get(bound).kind else {
                    return true;
                };
                self.graph
                    .types
                    .pack_elements(arguments)
                    .map_or(true, |elements| elements.len() != count)
            })
        {
            return Ok(false);
        }
        let mut collected = vec![Vec::with_capacity(count); indices.len()];
        let mut trial = arguments.to_vec();
        for position in 0..count {
            trial.copy_from_slice(arguments);
            for &index in &indices {
                trial[index] = arguments[index].and_then(|pack| {
                    self.graph
                        .types
                        .pack_elements(pack)
                        .ok()?
                        .get(position)
                        .copied()
                });
            }
            let saved = std::mem::replace(
                &mut self.deduced_pack_element,
                (!packs.bound.is_empty()).then_some(position),
            );
            let result = deduce(self, position, &mut trial);
            self.deduced_pack_element = saved;
            if !result? {
                return Ok(false);
            }
            for (&index, elements) in indices.iter().zip(&mut collected) {
                let Some(element) = trial[index] else {
                    return Ok(false);
                };
                elements.push(self.graph.types.canonical_function(element)?);
            }
            for (index, &argument) in trial.iter().enumerate() {
                if !indices.contains(&index) && argument.is_some() {
                    arguments[index] = argument;
                }
            }
        }
        for (index, elements) in indices.into_iter().zip(collected) {
            let pack = self
                .graph
                .types
                .intern(TypeKind::Pack(elements), Cv::default())?;
            if arguments[index].is_some_and(|known| known != pack) {
                return Ok(false);
            }
            arguments[index] = Some(pack);
        }
        Ok(true)
    }

    pub(super) fn deduce_function_parameters(
        &mut self,
        patterns: &[TypeId],
        actual: &[TypeId],
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        if patterns
            .last()
            .is_some_and(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)))
        {
            let patterns = self
                .graph
                .types
                .intern(TypeKind::Pack(patterns.to_vec()), Cv::default())?;
            let actual = self
                .graph
                .types
                .intern(TypeKind::Pack(actual.to_vec()), Cv::default())?;
            return self.deduce_signature_type(patterns, actual, depth, arguments, false);
        }
        if patterns.len() != actual.len() {
            return Ok(false);
        }
        for (&pattern, &actual) in patterns.iter().zip(actual) {
            if matches!(self.graph.types.get(actual).kind, TypeKind::Expansion(_))
                || !self.deduce_signature_type(pattern, actual, depth, arguments, false)?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn deduce_type_list(
        &mut self,
        patterns: &[TypeId],
        actual: &[TypeId],
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        let trailing = patterns
            .last()
            .and_then(|&ty| match self.graph.types.get(ty).kind {
                TypeKind::Expansion(pattern) => Some(pattern),
                _ => None,
            });
        let fixed = patterns.len() - usize::from(trailing.is_some());
        if patterns[..fixed]
            .iter()
            .any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)))
        {
            return Ok(true);
        }
        if if trailing.is_some() {
            actual.len() < fixed
        } else {
            actual.len() != fixed
        } {
            return Ok(false);
        }
        for (&pattern, &actual) in patterns[..fixed].iter().zip(actual) {
            if matches!(self.graph.types.get(actual).kind, TypeKind::Expansion(_))
                || !self.deduce_signature_type(pattern, actual, depth, arguments, false)?
            {
                return Ok(false);
            }
        }
        if let Some(pattern) = trailing {
            self.deduce_pack_types(pattern, &actual[fixed..], depth, arguments)
        } else {
            Ok(true)
        }
    }

    #[inline(never)]
    pub(super) fn deduce_pack_types(
        &mut self,
        pattern: TypeId,
        actual: &[TypeId],
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        let counted = !actual
            .iter()
            .any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)));
        self.deduce_pack_elements(
            pattern,
            actual.len(),
            counted,
            depth,
            arguments,
            |this, index, trial| {
                let actual = match this.graph.types.get(actual[index]).kind {
                    TypeKind::Expansion(element) => {
                        this.deduced_pack_element = None;
                        element
                    }
                    _ => actual[index],
                };
                this.deduce_signature_type(pattern, actual, depth, trial, false)
            },
        )
    }
}
