// Translated from sema/template_specializations.cpp::check_partial_arguments
// and deduced_parameters. See NOTICE.
//! Formation rules for partial argument patterns, before selection or demand.
use super::{
    Error,
    analyzer::Analyzer,
    templates::HeadId,
    types::{TypeId, TypeKind},
};
use crate::support::source::SourceOffset;
use std::collections::HashSet;

impl Analyzer<'_> {
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
        // Pack expansions and non-type parameters have distinct deduction
        // rules. Their formation remains an explicit implementation boundary.
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
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Array { element, .. } => work.push(*element),
                TypeKind::MemberPointer { owner, element } => work.extend([*owner, *element]),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters.iter().copied());
                }
                TypeKind::Auto
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
