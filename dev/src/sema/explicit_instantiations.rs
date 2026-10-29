// Translated from template_specializations.cpp::explicit_function_instantiation,
// class_templates.cpp::explicit_instantiation and analyzer.cpp. See NOTICE.
//! Explicit declarations suppress implicit bodies; definitions demand unit-tail
//! analysis and emission independently of uses. Canonical entities own state.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, EntityId, ScopeId, ScopeKind},
    specifiers::{InlinePreference, Specifiers},
    types::{Scalar, TypeKind},
};
use crate::support::hash::HashMap;
use crate::{
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};

#[derive(Clone, Copy, Default)]
pub(super) struct MemberPolicy {
    pub excluded: bool,
    pub always_inline: bool,
    pub gnu_inline: bool,
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{lookup::Filter, types::TypeKind},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn deleted_roots_skip_semantic_synthesis_but_retain_emission_identity() {
        let source = SourceBuffer::new(
            b"struct X{X(const X&)=delete;~X()=delete;};\
              template<class T>struct A{T t;A(const A&)=default;A(A&&)=default;~A()=default;};\
              template A<X>::A(const A<X>&);template A<X>::A(A<X>&&);template A<X>::~A();\
              template<class T>struct B{T t;B&operator=(const B&)=default;};\
              template B<int>& B<int>::operator=(const B<int>&);\
              struct Y{Y(Y&&)=delete;};\
              template B<Y>& B<Y>::operator=(const B<Y>&);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"defaulted-roots").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            false,
        )
        .unwrap();
        let definitions = graph.function_templates.instantiations.definitions();
        assert_eq!(definitions.len(), 5);
        for definition in &definitions[..3] {
            let entity = graph.binding(definition.binding).entity;
            assert_eq!(
                graph.entity(entity).availability,
                super::super::graph::FunctionAvailability::Deleted
            );
            assert!(!graph.function_templates.bodies.started(entity));
            assert!(graph.entity(entity).initialization().constructor.is_none());
        }
        let assignment = graph.binding(definitions[3].binding).entity;
        assert_eq!(graph.lower_explicit_functions().count(), 5);
        assert!(graph.entity(assignment).callable().assignment.is_some());
        assert!(graph.lower_has_synthesized_body(assignment));
        let deleted = graph.binding(definitions[4].binding).entity;
        let facts = graph
            .entity(deleted)
            .callable()
            .assignment
            .as_ref()
            .unwrap();
        assert!(facts.deleted && facts.nothrow);
        assert!(facts.actions[0].function.is_none());
        assert!(!graph.function_templates.bodies.started(deleted));
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn extern_special_members_share_identity_without_body_or_default_demand() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{A(int=T::missing);~A();operator T();};\
              template<class T>A<T>::A(int){T::missing;}\
              template<class T>A<T>::~A(){T::missing;}\
              template<class T>A<T>::operator T(){return T::missing;}\
              using I=A<int>;extern template A<int>::A(int);\
              extern template A<int>::~A();extern template A<int>::operator int();\
              extern template A<int>::A(int);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"extern-special-members").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let alias = cursor.intern_name(b"I").unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let binding = graph.binding(graph.own(graph.global, alias, Filter::Ordinary).unwrap());
        let TypeKind::Class(class) = graph.types.get(binding.ty.unwrap()).kind else {
            panic!("class alias");
        };
        let scope = graph.entity(class).context().scope.unwrap();
        let functions: Vec<_> = graph
            .scope(scope)
            .bindings
            .iter()
            .map(|&id| graph.binding(id).entity)
            .filter(|&entity| graph.entity(entity).callable().member_function.is_some())
            .collect();
        assert_eq!(functions.len(), 3);
        assert_eq!(graph.entity(class).class().declared_constructors.len(), 1);
        assert!(
            graph
                .function_templates
                .instantiations
                .definitions()
                .is_empty()
        );
        assert_eq!(graph.lower_explicit_functions().count(), 0);
        for entity in functions {
            assert!(graph.function_instantiation_suppressed(entity));
            assert!(!graph.function_templates.bodies.started(entity));
            for default in graph.entity(entity).callable().defaults.iter().flatten() {
                assert!(graph.expression_facts.get(default.expression).is_err());
            }
        }
    }
}

#[derive(Clone, Copy)]
enum State {
    Declaration,
    Definition,
}

#[derive(Clone, Copy)]
pub(super) enum DefinitionDemand {
    Named,
    ClassMember,
}

#[derive(Clone, Copy)]
pub(super) struct Definition {
    pub binding: BindingId,
    pub location: SourceOffset,
    pub demand: DefinitionDemand,
}
#[derive(Default)]
pub(super) struct Instantiations {
    states: HashMap<EntityId, State>,
    definitions: Vec<Definition>,
    classes: Vec<EntityId>,
    policies: HashMap<EntityId, MemberPolicy>,
}
impl Instantiations {
    pub(super) fn note_policy(&mut self, entity: EntityId, policy: MemberPolicy) {
        if policy.excluded || policy.always_inline || policy.gnu_inline {
            let old = self.policies.entry(entity).or_default();
            old.excluded |= policy.excluded;
            old.always_inline |= policy.always_inline;
            old.gnu_inline |= policy.gnu_inline;
        }
    }
    pub(super) fn policy(&self, entity: EntityId) -> MemberPolicy {
        self.policies.get(&entity).copied().unwrap_or_default()
    }
    pub(super) fn declared(&self, entity: EntityId) -> bool {
        matches!(self.states.get(&entity), Some(State::Declaration))
    }
    pub(super) fn classes(&self) -> &[EntityId] {
        &self.classes
    }
    pub(super) fn record_class(
        &mut self,
        entity: EntityId,
        is_extern: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.record_state(entity, is_extern, location)
    }
    pub(super) fn demand_class(&mut self, entity: EntityId) {
        self.classes.push(entity);
    }
    pub(super) fn definitions(&self) -> &[Definition] {
        &self.definitions
    }
    pub(super) fn defined(&self, entity: EntityId) -> bool {
        matches!(self.states.get(&entity), Some(State::Definition))
    }
    pub(super) fn note_ordinary_definition(&mut self, entity: EntityId) {
        // An explicit class specialization's members are ordinary entities.
        // Record their root flag without queueing implicit body demand.
        self.states.insert(entity, State::Definition);
    }
    pub(super) fn record(
        &mut self,
        binding: BindingId,
        entity: EntityId,
        is_extern: bool,
        location: SourceOffset,
        demand: DefinitionDemand,
    ) -> Result<(), Error> {
        self.record_state(entity, is_extern, location)?;
        if !is_extern {
            self.definitions.push(Definition {
                binding,
                location,
                demand,
            });
        }
        Ok(())
    }
    fn record_state(
        &mut self,
        entity: EntityId,
        is_extern: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.defined(entity) {
            return Err(Error::at(
                location,
                if is_extern {
                    "an explicit instantiation declaration after the definition"
                } else {
                    "a specialization explicitly instantiated twice"
                },
            ));
        }
        self.states.insert(
            entity,
            if is_extern {
                State::Declaration
            } else {
                State::Definition
            },
        );
        Ok(())
    }
}
impl Analyzer<'_> {
    pub(super) fn explicit_instantiation(
        &mut self,
        declaration: ast::DeclId,
        is_extern: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let inner = *self.tree.declarations.get(declaration);
        if let ast::DeclKind::Simple { declarators, .. } = inner.kind
            && self.tree.init_declarators.get(declarators).len() > 1
        {
            return Err(Error::at(
                location,
                "an explicit instantiation must declare a single entity",
            ));
        }
        if let ast::DeclKind::Class(specifier) = inner.kind {
            return self.explicit_class_instantiation(specifier, is_extern, scope, inner.location);
        }
        // 14.7.2p12: names in the declaration, including explicit arguments,
        // do not impose access checks. Drop the deferred checks on every path.
        let access = self.begin_declaration_access(scope, true);
        let result = (|| {
            let (specs, info) = self.explicit_instantiation_declarator(inner, scope)?;
            if info.syntax_name.is_none() {
                return Err(Error::at(
                    inner.location,
                    "an explicit instantiation of no function or variable",
                ));
            }
            if !matches!(
                self.graph.types.get(info.ty).kind,
                TypeKind::Function { .. }
            ) {
                self.explicit_variable_instantiation(
                    &info,
                    specs,
                    is_extern,
                    scope,
                    inner.location,
                )?;
                return Ok((specs, None));
            }
            let binding = if let Some(member) = self.declared_instance_member(&info)? {
                let entity = self.graph.binding(member).entity;
                let ty = self.graph.types.canonical_function(
                    self.graph
                        .binding(member)
                        .ty
                        .ok_or(Error::Invariant("member without signature"))?,
                )?;
                let declared = self.graph.types.canonical_function(info.ty)?;
                let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
                    return Err(Error::Invariant("member signature without function"));
                };
                if !matches!(self.graph.types.get(declared).kind, TypeKind::Function { result: other, .. } if other == result)
                {
                    return Err(Error::at(
                        inner.location,
                        "an explicit instantiation's return type differs from the member's",
                    ));
                }
                if !self.graph.function_templates.bodies.member(entity)
                    && !self
                        .graph
                        .function_templates
                        .specializations
                        .contains(entity)
                {
                    return Err(Error::at(
                        inner.location,
                        "an explicit instantiation of a member that is not instantiated",
                    ));
                }
                member
            } else {
                self.declared_function_specialization(&info, scope, inner.location)?
                    .ok_or(Error::at(
                        inner.location,
                        "an explicit instantiation matches no function template specialization",
                    ))?
            };
            Ok((specs, Some(binding)))
        })();
        self.finish_declaration_access(access);
        let (specs, binding) = result?;
        let Some(binding) = binding else {
            return Ok(());
        };
        if specs.inline == InlinePreference::Inline
            || specs.constexpr
            || specs.is_static
            || specs.is_extern
        {
            return Err(Error::at(
                inner.location,
                "a specifier an explicit instantiation may not have",
            ));
        }
        let entity = self.graph.binding(binding).entity;
        let home = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("instance without home"))?;
        let here = self.declaration_scope(scope);
        if self.graph.scope(here).kind != ScopeKind::Namespace
            || !self.graph.encloses(here, self.enclosing_namespace(home))
        {
            return Err(Error::at(
                inner.location,
                "an explicit instantiation outside its template's namespace",
            ));
        }
        if self.semantic() {
            self.check_operator_declaration(binding, inner.location)?;
        }
        if self
            .graph
            .function_templates
            .specializations
            .contains(entity)
        {
            return Ok(());
        }
        self.graph.function_templates.instantiations.record(
            binding,
            entity,
            is_extern,
            inner.location,
            DefinitionDemand::Named,
        )
    }

    fn explicit_instantiation_declarator(
        &mut self,
        declaration: ast::Declaration,
        scope: ScopeId,
    ) -> Result<(Specifiers, super::declarators::DeclaratorInfo), Error> {
        let location = declaration.location;
        let (specs, declarator, result) = match declaration.kind {
            ast::DeclKind::Simple {
                specifiers,
                declarators,
                ..
            } => {
                let declarators = self.tree.init_declarators.get(declarators);
                if declarators.len() != 1 || declarators[0].initializer.is_some() {
                    return Err(Error::at(
                        location,
                        "an explicit instantiation declares one function or variable",
                    ));
                }
                let declarator = declarators[0].declarator;
                let specs = self.specifiers(specifiers, scope, false)?;
                (specs, declarator, specs.ty)
            }
            ast::DeclKind::SpecialMember {
                name,
                declarator,
                initializer,
                body,
                ..
            } => {
                if body.is_some() || initializer.is_some() {
                    return Err(Error::at(
                        location,
                        "an explicit instantiation with a definition",
                    ));
                }
                let name_node = self.tree.names.get(name);
                let last = self
                    .tree
                    .name_lists
                    .get(name_node.components)
                    .last()
                    .ok_or(Error::Invariant("explicit special member without name"))?;
                let result = if let ast::NameKind::Conversion(ty) = last.kind {
                    let owner = self.prefix(name, scope)?.unwrap_or(scope);
                    self.type_syntax(ty, owner)?
                } else {
                    self.graph.types.scalar(Scalar::Void)?
                };
                // The frozen explicit-instantiation owner uses empty specifiers
                // for special members; it does not redeclare the member.
                let specs = self.special_specifiers(List::default(), scope)?.0;
                (specs, declarator, result)
            }
            _ => {
                return Err(Error::at(
                    location,
                    "an explicit instantiation of no class, function or variable",
                ));
            }
        };
        let mut environment = self.declarator_environment(Some(declarator), scope)?;
        environment.placement = super::declarators::Placement::ExplicitInstantiation;
        let info = self.declarator_in(Some(declarator), result, environment)?;
        Ok((specs, info))
    }
}
