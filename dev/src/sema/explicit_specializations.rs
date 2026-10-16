// Translated from sema/template_specializations.cpp. See NOTICE.
//! Explicit function definitions replace implicit body demand for the same key.
use super::{
    Error,
    analyzer::{Analyzer, SpecializationMode},
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
        let mode = self.specialization_mode;
        self.specialization_levels = saved
            .checked_add(1)
            .ok_or(Error::Limit("specialization nesting limit"))?;
        self.specialization_mode = SpecializationMode::Explicit;
        let result = self.declaration(declaration, scope);
        self.specialization_levels = saved;
        self.specialization_mode = mode;
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
        self.specialization_mode = SpecializationMode::Ordinary;
        // C++ passes deletion separately from a body/defaulted definition.
        // A copied member may still carry its primary's defined flag after
        // an explicit declaration; deleting it does not define another body.
        let defines_body = matches!(definition, Definition::Body | Definition::Defaulted);
        let member = self.declared_instance_member(info)?;
        let of_template = member.is_none();
        let binding = if let Some(member) = member {
            self.specialized_member_binding(
                member,
                info.home
                    .ok_or(Error::Invariant("member specialization without owner"))?,
                location,
            )?
        } else {
            self.declared_function_specialization(info, scope, location)?
                .ok_or_else(|| {
                    Error::at(
                        location,
                        "an explicit specialization matches no function template",
                    )
                })?
        };
        let entity = self.graph.binding(binding).entity;
        if of_template && self.specialization_levels != self.specialized_enclosing(info.home) + 1 {
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
        if of_template
            && if explicit {
                defines_body && self.graph.entity(entity).defined
            } else {
                self.graph.function_templates.bodies.started(entity)
            }
        {
            return Err(Error::at(
                location,
                "an explicit specialization after the specialization's instantiation or definition",
            ));
        }
        if defines_body && self.graph.entity(entity).defined && explicit {
            return Err(Error::at(location, "redefinition of a function"));
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
            if !of_template {
                // Copied declarations retain the pattern's availability, but
                // its deferred body is not this specialization's definition.
                if definition.defines() {
                    function.defined = false;
                }
                function.special_definition = None;
                function.function_definition = None;
            }
            function.availability = FunctionAvailability::Callable;
            if of_template && let Some(properties) = function.function.as_mut() {
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
        if self.semantic() {
            self.check_operator_declaration(binding, location)?;
        }
        Ok(binding)
    }

    // template_specializations.cpp::SpecializedEnclosing: count only class
    // template specializations, walking through copied ordinary member classes.
    fn specialized_enclosing(&self, mut scope: Option<ScopeId>) -> u32 {
        let mut count = 0;
        while let Some(home) = scope {
            let node = self.graph.scope(home);
            if node.kind != ScopeKind::Class {
                break;
            }
            let Some(owner) = node.owner else { break };
            if let Some(instance) = self.graph.class_templates.instance(owner) {
                count += u32::from(!self.graph.class_templates.definitions.explicit(owner));
                scope = self.graph.entity(instance.primary).home;
            } else if self
                .graph
                .class_templates
                .definitions
                .source(owner)
                .is_some()
            {
                scope = self.graph.entity(owner).home;
            } else {
                break;
            }
        }
        count
    }

    pub(super) fn declared_instance_member(
        &mut self,
        info: &DeclaratorInfo,
    ) -> Result<Option<BindingId>, Error> {
        let Some(home) = info
            .home
            .filter(|&home| self.graph.within_class_instance(home))
        else {
            return Ok(None);
        };
        if self.function_template_id(info).is_some() {
            return Ok(None);
        }
        let special = self
            .declared_member(info, home)?
            .map(|(_, kind)| kind)
            .or_else(|| {
                self.conversion_declarator(info)
                    .then_some(super::graph::MemberKind::Conversion)
            });
        if let Some(kind) = special {
            let class = self
                .graph
                .scope(home)
                .owner
                .ok_or(Error::Invariant("specialization without class"))?;
            let signature = self.graph.types.canonical_function(info.ty)?;
            let binding = if kind == super::graph::MemberKind::Constructor {
                self.graph.constructor(class, signature)
            } else {
                self.graph.member_function(super::graph::MemberKey {
                    class,
                    kind,
                    signature,
                    shape: None,
                })
            };
            return Ok(binding);
        }
        let key = self.declaration_key(info)?;
        // Implicit assignment declarations are materialized by their owning
        // class, just as for an ordinary qualified operator lookup.
        self.prepare_assignment_lookup(home, key, false)?;
        let mut next = 0;
        while let Some(binding) = self.graph.next_function(home, key, &mut next) {
            let candidate = self.graph.binding(binding);
            if self.graph.entity(candidate.entity).home != Some(home)
                || self.graph.entity(candidate.entity).template_head.is_some()
                || !self.same_function_parameters(
                    candidate
                        .ty
                        .ok_or(Error::Invariant("member without type"))?,
                    info.ty,
                )?
            {
                continue;
            }
            return Ok(Some(binding));
        }
        Ok(None)
    }

    fn specialized_member_binding(
        &mut self,
        binding: BindingId,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        let candidate = self.graph.binding(binding);
        if self.specialization_levels != self.specialized_enclosing(Some(home)) {
            return Err(Error::at(
                location,
                "an explicit specialization of a member with the wrong number of template<>",
            ));
        }
        if !self
            .graph
            .function_templates
            .specializations
            .contains(candidate.entity)
            && (!self
                .graph
                .function_templates
                .bodies
                .member(candidate.entity)
                || self
                    .graph
                    .function_templates
                    .bodies
                    .started(candidate.entity))
        {
            return Err(Error::at(
                location,
                "an explicit specialization after the member's instantiation",
            ));
        }
        self.graph.add_hidden_function(binding, candidate)
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
    fn member_definition_keeps_identity_without_copying_the_primary_body() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{int f(){return sizeof(typename T::missing);}};\
              int size=sizeof(A<int>);template<>int A<int>::f(){return 4;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"own-member-body").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let ids = analyzer.tree.decl_lists.get(declarations).to_vec();
        let global = analyzer.graph.global;
        analyzer.declaration(ids[0], global).unwrap();
        analyzer.declaration(ids[1], global).unwrap();
        let bodies = analyzer.tree.bodies.mark();
        let expressions = analyzer.tree.expressions.mark();
        analyzer.declaration(ids[2], global).unwrap();
        let &entity = analyzer
            .graph
            .function_templates
            .specializations
            .entities
            .iter()
            .next()
            .unwrap();
        assert!(analyzer.graph.function_templates.bodies.member(entity));
        assert!(!analyzer.graph.function_templates.bodies.started(entity));
        assert!(analyzer.graph.entity(entity).scope.is_some());
        assert!(analyzer.graph.entity(entity).function_definition.is_some());
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        analyzer.finish_lowering_boundaries().unwrap();
        assert!(!analyzer.graph.function_templates.bodies.started(entity));
    }

    #[test]
    fn member_forward_keeps_redeclaration_state_without_a_lowerable_primary_body() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{int f(){return 1;}};\
              template<>int A<int>::f();template<>int A<int>::f(){return 4;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-forward").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let ids = analyzer.tree.decl_lists.get(declarations).to_vec();
        let global = analyzer.graph.global;
        analyzer.declaration(ids[0], global).unwrap();
        analyzer.declaration(ids[1], global).unwrap();
        let &entity = analyzer
            .graph
            .function_templates
            .specializations
            .entities
            .iter()
            .next()
            .unwrap();
        // C++ retains the copied declaration's defined flag for redeclaration
        // diagnostics, independently of own-body availability and emission.
        assert!(analyzer.graph.entity(entity).defined);
        assert!(!analyzer.graph.lower_function_defined(entity));
        assert!(analyzer.graph.entity(entity).function_definition.is_none());
        assert!(matches!(
            analyzer.declaration(ids[2], global),
            Err(Error::Diagnostic {
                message: "redefinition of a function",
                ..
            })
        ));
        assert!(!analyzer.specializing());
        assert_eq!(analyzer.specialization_levels, 0);
    }

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
