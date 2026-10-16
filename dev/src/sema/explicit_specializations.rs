// Translated from sema/template_specializations.cpp. See NOTICE.
//! Explicit function definitions replace implicit body demand for the same key.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    function_demands::DefinitionEmission,
    function_names::Definition,
    graph::{BindingId, EntityId, FunctionAvailability, ScopeId, ScopeKind},
    specifiers::{InlinePreference, Specifiers},
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct Specializations {
    entities: HashSet<EntityId>,
    instantiation_roots: HashSet<EntityId>,
}
impl Specializations {
    pub(super) fn contains(&self, entity: EntityId) -> bool {
        self.entities.contains(&entity)
    }
    pub(super) fn note_instantiation(&mut self, entity: EntityId, is_extern: bool) {
        if !is_extern {
            self.instantiation_roots.insert(entity);
        }
    }
    pub(super) fn instantiation_root(&self, entity: EntityId) -> bool {
        self.instantiation_roots.contains(&entity)
    }
}

impl Analyzer<'_> {
    pub(super) fn explicit_specialization(
        &mut self,
        declaration: ast::DeclId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.scope(self.declaration_scope(scope)).kind != ScopeKind::Namespace {
            return Err(Error::at(
                location,
                "an explicit specialization outside namespace scope",
            ));
        }
        if self.template_depth != 0 {
            return Err(Error::at(
                location,
                "an explicit specialization in a template is not supported",
            ));
        }
        let saved = self.specialization_levels;
        self.specialization_levels = saved
            .checked_add(1)
            .ok_or(Error::Limit("specialization nesting limit"))?;
        let result = self.declaration(declaration, scope);
        self.specialization_levels = saved;
        result
    }

    pub(super) fn explicit_function_specialization(
        &mut self,
        info: &DeclaratorInfo,
        specs: Specifiers,
        scope: ScopeId,
        location: SourceOffset,
        definition: Definition,
    ) -> Result<BindingId, Error> {
        let levels = std::mem::take(&mut self.specialization_levels);
        if info
            .home
            .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class)
        {
            return Err(Error::Missing("explicit member function specialization"));
        }
        let binding = self
            .declared_function_specialization(info, scope, location)?
            .ok_or_else(|| {
                Error::at(
                    location,
                    "an explicit specialization matches no function template",
                )
            })?;
        let entity = self.graph.binding(binding).entity;
        if levels != 1 {
            return Err(Error::at(
                location,
                "an explicit specialization of a function template with the wrong number of template<>",
            ));
        }
        let explicit = self
            .graph
            .function_templates
            .specializations
            .contains(entity);
        if if explicit {
            definition.defines() && self.graph.entity(entity).defined
        } else {
            self.graph.function_templates.bodies.started(entity)
        } {
            return Err(Error::at(
                location,
                "an explicit specialization after the specialization's instantiation or definition",
            ));
        }
        if self.semantic() {
            self.resolve_specialization_exception(entity)?;
            self.record_function_exceptions(
                entity,
                info,
                info.lookup,
                location,
                self.graph.binding_limit(),
            )?;
        }
        if let Some(parameters) = info.parameters {
            for parameter in self.tree.parameters.get(parameters.parameters) {
                if parameter.default.is_some() {
                    return Err(Error::at(
                        location,
                        "a default argument in an explicit specialization",
                    ));
                }
            }
        }
        let function = self.graph.entity_mut(entity);
        if !explicit {
            function.availability = FunctionAvailability::Callable;
            if let Some(properties) = function.function.as_mut() {
                properties.no_inline = false;
            }
        }
        if definition == Definition::Deleted {
            function.availability = FunctionAvailability::Deleted;
        }
        function.inline = if specs.constexpr {
            InlinePreference::Inline
        } else {
            specs.inline
        };
        function.constexpr = specs.constexpr;
        function.definition_emission = if function.inline == InlinePreference::Inline {
            DefinitionEmission::Demanded
        } else {
            DefinitionEmission::Always
        };
        if info.parameters.is_some() {
            function.function_parameters = info.parameters;
        }
        self.graph
            .function_templates
            .specializations
            .entities
            .insert(entity);
        Ok(binding)
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
    fn own_definition_keeps_the_signature_key_without_instantiating_the_primary() {
        let source = SourceBuffer::new(
            b"template<class T>inline int f(T primary){return sizeof(typename T::missing);}\
              template<>int f<int>(int declared);\
              template<>int f(int renamed){return renamed;}\
              int main(){return f(2)+f<int>(3);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"own-specialization").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(graph.function_templates.specializations.entities.len(), 1);
        let &entity = graph
            .function_templates
            .specializations
            .entities
            .iter()
            .next()
            .unwrap();
        let function = graph.entity(entity);
        let primary = graph.function_templates.primary(entity).unwrap();
        assert_ne!(entity, graph.binding(primary).entity);
        assert!(function.inline == InlinePreference::Unspecified);
        assert_eq!(function.definition_emission, DefinitionEmission::Always);
        assert!(function.defined);
        assert!(!graph.function_templates.bodies.started(entity));
        assert_ne!(
            tree.parameters
                .get(function.function_parameters.unwrap().parameters)[0]
                .location,
            graph
                .entity(graph.binding(primary).entity)
                .function_parameters
                .unwrap()
                .location
        );
        // No implicit body copy is made, even though both explicit and deduced
        // calls use the canonical signature that preceded this definition.
        assert_eq!(tree.expressions.mark(), expressions);
    }

    #[test]
    fn rejection_restores_the_specialization_introducer() {
        let source =
            SourceBuffer::new(b"template<class T>int f(T);template<>int f<int>(int x=1);".to_vec())
                .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"specialization-error").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let declarations = analyzer.tree.decl_lists.get(declarations).to_vec();
        let global = analyzer.graph.global;
        analyzer.declaration(declarations[0], global).unwrap();
        assert!(matches!(
            analyzer.declaration(declarations[1], global),
            Err(Error::Diagnostic {
                message: "a default argument in an explicit specialization",
                ..
            })
        ));
        assert_eq!(analyzer.specialization_levels, 0);
        assert_eq!(analyzer.template_depth, 0);
        assert!(
            analyzer
                .graph
                .function_templates
                .specializations
                .entities
                .is_empty()
        );
    }

    #[test]
    fn explicit_instantiation_of_an_own_definition_only_marks_noninline_roots() {
        let source = SourceBuffer::new(
            b"template<class T>int f(T){return 1;}\
              template<class T>int g(T){return 1;}\
              template<class T>int h(T){return 1;}\
              template int f<int>(int);\
              template int h<int>(int);\
              template<>int f<int>(int){return 2;}\
              template<>inline int h<int>(int){return 4;}\
              template<>inline int g<int>(int){return 3;}\
              template int g<int>(int);template int g<int>(int);\
              extern template int g<int>(int);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"specialization-roots").unwrap());
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
        let roots: Vec<_> = graph.lower_explicit_functions().collect();
        assert_eq!(roots.len(), 2);
        assert!(graph.lower_explicit_function_root(roots[0]));
        assert!(!graph.lower_explicit_function_root(roots[1]));
        assert_eq!(graph.function_templates.specializations.entities.len(), 3);
        for &entity in &graph.function_templates.specializations.entities {
            assert!(!graph.function_templates.bodies.started(entity));
            assert!(!graph.function_instantiation_suppressed(entity));
            assert_eq!(
                graph.lower_explicit_function_root(entity),
                entity == roots[0]
            );
        }
    }
}
