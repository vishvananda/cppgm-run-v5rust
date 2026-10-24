// Translated from template_packs.cpp::substitute_list/bound_expansion. See NOTICE.
//! Type-list expansion keeps argument partitions and per-element memos separate.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::{PackSelection, Substitution},
    types::{Cv, TypeId, TypeKind},
};

impl Analyzer<'_> {
    #[inline]
    pub(super) fn substitute_pack_list(
        &mut self,
        patterns: &[TypeId],
        environment: &mut Substitution<'_>,
    ) -> Result<Option<Vec<TypeId>>, Error> {
        if patterns
            .iter()
            .any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)))
        {
            return self.substitute_expansion_list(patterns, environment);
        }
        let mut result = Vec::with_capacity(patterns.len());
        let mut valid = true;
        for &pattern in patterns {
            if let Some(ty) = self.substitute_signature_type(pattern, environment)? {
                result.push(ty);
            } else {
                valid = false;
            }
        }
        Ok(valid.then_some(result))
    }

    #[inline(never)]
    fn substitute_expansion_list(
        &mut self,
        patterns: &[TypeId],
        environment: &mut Substitution<'_>,
    ) -> Result<Option<Vec<TypeId>>, Error> {
        self.check_stack()?;
        let mut result = Vec::new();
        let mut valid = true;
        for &pattern in patterns {
            let TypeKind::Expansion(element) = self.graph.types.get(pattern).kind else {
                if let Some(ty) = self.substitute_signature_type(pattern, environment)? {
                    result.push(ty);
                } else {
                    valid = false;
                }
                continue;
            };
            let packs = self.type_pack_pattern(element, true)?;
            let has_parameters = !packs.parameters.is_empty();
            let mut indices = Vec::new();
            let mut unknown = false;
            let mut length = None;
            let mut mismatch = false;
            for parameter in packs.parameters {
                let info = self
                    .graph
                    .entity(parameter)
                    .template_parameter
                    .ok_or(Error::Invariant("expansion without pack parameter"))?;
                let Some(index) = info.index(environment.depth) else {
                    continue;
                };
                let Some(Some(argument)) = environment.arguments.get(index) else {
                    unknown = true;
                    break;
                };
                let TypeKind::Pack(elements) = &self.graph.types.get(*argument).kind else {
                    unknown = true;
                    break;
                };
                mismatch |= length.is_some_and(|known| known != elements.len());
                length = Some(elements.len());
                indices.push(index);
            }
            unknown |= has_parameters && indices.is_empty();
            for &bound in packs.bound.iter().filter(|_| !unknown) {
                let TypeKind::BoundPack { arguments, .. } = self.graph.types.get(bound).kind else {
                    return Err(Error::Invariant("expansion lost bound pack"));
                };
                let TypeKind::Pack(elements) = &self.graph.types.get(arguments).kind else {
                    return Err(Error::Invariant("bound pack without argument pack"));
                };
                mismatch |= length.is_some_and(|known| known != elements.len());
                length = Some(elements.len());
            }
            if mismatch {
                return Ok(
                    environment.reject("a pack expansion of parameter packs of different lengths")
                );
            }
            if unknown || length.is_none() {
                let mut whole = Substitution::new(environment.depth, environment.arguments);
                whole.context = environment.context;
                let selection = PackSelection {
                    elements: environment
                        .selection
                        .map_or(&[], |selection| selection.elements),
                    bound_element: None,
                };
                whole.selection = Some(&selection);
                let Some(ty) = self.substitute_signature_type(element, &mut whole)? else {
                    if environment.failure.is_none() {
                        environment.failure = whole.failure;
                        environment.failure_location = whole.failure_location;
                    }
                    valid = false;
                    continue;
                };
                if !has_parameters
                    && packs.bound.is_empty()
                    && matches!(self.graph.types.get(ty).kind, TypeKind::Value { .. })
                {
                    match self.integer_pack_values(
                        ty,
                        crate::support::source::SourceOffset(0),
                        &mut result,
                    ) {
                        Ok(()) => continue,
                        Err(Error::Diagnostic { location, message }) => {
                            return Ok(environment.reject_at(location, message));
                        }
                        Err(error) => return Err(error),
                    }
                }
                result.push(
                    self.graph
                        .types
                        .intern(TypeKind::Expansion(ty), Cv::default())?,
                );
                continue;
            }
            let mut elements = environment
                .selection
                .map_or_else(Vec::new, |selection| selection.elements.to_vec());
            elements.resize(environment.arguments.len(), None);
            for position in 0..length.unwrap_or_default() {
                for &index in &indices {
                    elements[index] = Some(position);
                }
                let selection = PackSelection {
                    elements: &elements,
                    bound_element: (!packs.bound.is_empty()).then_some(position),
                };
                let mut inner = Substitution::new(environment.depth, environment.arguments);
                inner.context = environment.context;
                inner.selection = Some(&selection);
                if let Some(ty) = self.substitute_signature_type(element, &mut inner)? {
                    result.push(ty);
                } else {
                    if environment.failure.is_none() {
                        environment.failure = inner.failure;
                        environment.failure_location = inner.failure_location;
                    }
                    valid = false;
                }
            }
        }
        Ok(valid.then_some(result))
    }

    pub(super) fn bound_argument_expansion(
        &mut self,
        expansion: TypeId,
    ) -> Result<Vec<TypeId>, Error> {
        let TypeKind::Expansion(pattern) = self.graph.types.get(expansion).kind else {
            return Ok(vec![expansion]);
        };
        let packs = self.type_pack_pattern(pattern, true)?;
        if !packs.parameters.is_empty() || packs.bound.is_empty() || packs.expression {
            return Ok(vec![expansion]);
        }
        let mut environment = Substitution::new(u32::MAX, &[]);
        // This is semantic bound expansion, not a syntax-copy mismatch.
        // Preserve substitute_list's location-zero failure at the hard boundary.
        self.substitute_pack_list(&[expansion], &mut environment)?
            .ok_or_else(|| {
                environment.failure.map_or(
                    Error::Invariant("bound argument expansion failed"),
                    |message| Error::at(environment.failure_location, message),
                )
            })
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
    fn element_substitutions_keep_distinct_memos_and_empty_expansions_disappear() {
        let source = SourceBuffer::new(b"template<class... T>struct Box;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"pack-substitution").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Box").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer
            .graph
            .own(global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let primary = analyzer.graph.binding(binding).entity;
        let head = analyzer.graph.entity(primary).template_head.unwrap();
        let parameter = analyzer.graph.templates.parameters(head)[0];
        let ty = analyzer.graph.entity(parameter).ty.unwrap();
        let pointer = analyzer.graph.types.pointer(ty).unwrap();
        let expansion = analyzer
            .graph
            .types
            .intern(TypeKind::Expansion(pointer), Cv::default())
            .unwrap();
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        let long = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::LongInt,
            ))
            .unwrap();
        let pack = analyzer
            .graph
            .types
            .intern(TypeKind::Pack(vec![int, long]), Cv::default())
            .unwrap();
        let given = [Some(pack)];
        let mut environment = Substitution::new(analyzer.graph.templates.depth(head), &given);
        let result = analyzer
            .substitute_pack_list(&[expansion, expansion], &mut environment)
            .unwrap()
            .unwrap();
        let pi = analyzer.graph.types.pointer(int).unwrap();
        let pl = analyzer.graph.types.pointer(long).unwrap();
        assert_eq!(result, [pi, pl, pi, pl]);
        assert_ne!(pi, pl);
        assert_eq!(environment.arguments, &given);
        assert!(environment.selection.is_none());
        let empty = analyzer
            .graph
            .types
            .intern(TypeKind::Pack(vec![]), Cv::default())
            .unwrap();
        let given = [Some(empty)];
        let mut environment = Substitution::new(analyzer.graph.templates.depth(head), &given);
        assert!(
            analyzer
                .substitute_pack_list(&[expansion], &mut environment)
                .unwrap()
                .unwrap()
                .is_empty()
        );
    }
}
