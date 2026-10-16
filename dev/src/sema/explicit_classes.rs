// Translated from class_templates.cpp::explicit_instantiation and
// explicitly_instantiate_members. See NOTICE.
//! Class definitions select only their own, already-defined member patterns.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, EntityId, Graph, ScopeId, ScopeKind},
    lookup::Filter,
    types::TypeKind,
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

impl Analyzer<'_> {
    pub(super) fn explicit_class_instantiation(
        &mut self,
        specifier: ast::Specifier,
        is_extern: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let ast::SpecifierKind::Elaborated { key, name, .. } = specifier.kind else {
            return Err(Error::at(
                location,
                "an explicit instantiation names a class template specialization",
            ));
        };
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let qualified = node.global || components.len() > 1;
        let arguments = components.last().is_some_and(|c| c.arguments.is_some());
        if !arguments && !qualified {
            return Err(Error::at(
                location,
                "an explicit instantiation names a class template specialization",
            ));
        }
        let access = self.begin_declaration_access(scope, true);
        let found = self.lookup(name, scope, Filter::Elaborated);
        self.finish_declaration_access(access);
        let binding = self.graph.binding(found?);
        let entity = binding.entity;
        if binding.kind != BindingKind::Type
            || !binding
                .ty
                .is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)))
        {
            return Err(Error::at(
                node.location,
                "an explicit instantiation names a class template specialization",
            ));
        }
        let source = if arguments {
            self.graph.class_templates.instance(entity).map(|i| i.primary)
        } else {
            self.graph.class_templates.definitions.source(entity)
        }
        .ok_or(Error::at(
            node.location,
            "an explicit instantiation names a class template specialization or a member class of one",
        ))?;
        if (key == Keyword::Union) != (self.graph.entity(entity).key == Keyword::Union) {
            return Err(Error::at(
                node.location,
                "class-key does not agree with the template's",
            ));
        }
        let here = self.declaration_scope(scope);
        let home = self
            .graph
            .entity(source)
            .home
            .ok_or(Error::Invariant("class pattern without owner"))?;
        let mut placed = if qualified {
            self.graph.encloses(here, home)
        } else {
            here == home
        };
        let mut current = home;
        while !placed && !qualified && self.graph.scope(current).is_inline {
            let Some(parent) = self.graph.scope(current).parent else {
                break;
            };
            placed = parent == here;
            current = parent;
        }
        if !placed {
            return Err(Error::at(
                node.location,
                "an explicit instantiation outside its template's namespace",
            ));
        }
        if !self.complete_class(entity)? {
            return Err(Error::at(
                node.location,
                "explicit instantiation of an undefined class template",
            ));
        }
        self.graph.function_templates.instantiations.record_class(
            entity,
            is_extern,
            node.location,
        )?;
        if !is_extern {
            self.explicitly_instantiate_members(entity, node.location)?;
        }
        Ok(())
    }

    fn explicitly_instantiate_members(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        self.graph
            .function_templates
            .instantiations
            .demand_class(class);
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("complete class without scope"))?;
        // The binding sequence is source order, independent of the lookup index.
        for index in 0..self.graph.scope(scope).bindings.len() {
            let id = self.graph.scope(scope).bindings[index];
            let binding = self.graph.binding(id);
            let entity = binding.entity;
            if entity == class || self.graph.entity(entity).home != Some(scope) {
                continue;
            }
            match binding.kind {
                BindingKind::Function => {
                    let special = self.graph.entity(entity).special_definition.or_else(|| {
                        self.graph
                            .function_templates
                            .bodies
                            .source(entity)
                            .and_then(|source| self.graph.entity(source).special_definition)
                    });
                    if !(self.graph.function_templates.bodies.defined_member(entity)
                        || self.graph.function_templates.bodies.member(entity)
                            && matches!(
                                special,
                                Some(
                                    super::special_members::SpecialDefinition::Defaulted { .. }
                                        | super::special_members::SpecialDefinition::Deleted
                                )
                            ))
                        || self.graph.function_templates.instantiations.defined(entity)
                        || self.graph.instance_member_policy(entity).excluded
                    {
                        continue;
                    }
                    self.graph
                        .function_templates
                        .instantiations
                        .record(id, entity, false, location)?;
                }
                BindingKind::Variable => self.explicit_class_variable(entity)?,
                BindingKind::Type
                    if self
                        .graph
                        .class_templates
                        .definitions
                        .source(entity)
                        .is_some()
                        && self.complete_class(entity)? =>
                {
                    self.explicitly_instantiate_members(entity, location)?;
                }
                _ => (),
            }
        }
        Ok(())
    }
}

impl Graph {
    pub(super) fn instance_member_policy(
        &self,
        entity: EntityId,
    ) -> super::explicit_instantiations::MemberPolicy {
        let store = &self.function_templates.instantiations;
        let own = store.policy(entity);
        let source = self.function_templates.bodies.source(entity).or_else(|| {
            self.function_templates
                .primary(entity)
                .map(|id| self.binding(id).entity)
        });
        let pattern = source.map_or_default(|source| store.policy(source));
        super::explicit_instantiations::MemberPolicy {
            excluded: own.excluded || pattern.excluded,
            always_inline: own.always_inline || pattern.always_inline,
        }
    }
    pub(super) fn class_instantiation_suppressed(&self, entity: EntityId) -> bool {
        self.entity(entity)
            .home
            .filter(|&home| self.scope(home).kind == ScopeKind::Class)
            .and_then(|home| self.scope(home).owner)
            .is_some_and(|class| self.function_templates.instantiations.declared(class))
    }
    pub(super) fn function_instantiation_suppressed(&self, entity: EntityId) -> bool {
        let record = self.entity(entity);
        let store = &self.function_templates.instantiations;
        let policy = self.instance_member_policy(entity);
        if self.function_templates.specializations.contains(entity)
            || record.inline == super::specifiers::InlinePreference::Inline
            || store.defined(entity)
            || policy.excluded
            || policy.always_inline
        {
            return false;
        }
        store.declared(entity)
            || self.function_templates.bodies.member(entity)
                && self.class_instantiation_suppressed(entity)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn explicit_deleted_copy_keeps_its_property_and_subobject_identity() {
        let source = SourceBuffer::new(
            b"struct M{int n;M(const M&)=delete;};\
              template<class T>struct Box{T value;Box(const Box&)=default;};\
              template struct Box<M>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"explicit-copy").unwrap());
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
        assert_eq!(roots.len(), 1);
        let entity = graph.entity(roots[0]);
        let properties = entity.copy_properties.unwrap();
        let properties = graph.copy_properties.get(properties);
        assert!(properties.deleted);
        let super::super::construction::ConstructorState::Ready(plan) = properties.plan else {
            panic!("unpublished copy plan");
        };
        assert_eq!(entity.constructor, Some(plan));
        let plan = graph.constructors.get(plan);
        assert!(plan.deleted);
        let actions = plan.copy.as_ref().unwrap();
        assert_eq!(actions.len(), 1);
        assert!(actions[0].constructor.is_none());
        let super::super::copy_construction::CopyTarget::Member(member) = actions[0].target else {
            panic!("lost member identity");
        };
        assert_eq!(Some(member), properties.subobjects[0].0);
        assert_eq!(properties.subobjects.len(), 1);
        assert!(properties.subobjects[0].2.is_none());
        assert!(graph.function_templates.bodies.body(roots[0]).is_none());
    }

    #[test]
    fn explicit_members_keep_definition_point_and_independent_demands() {
        let source = SourceBuffer::new(
            b"template<class T>struct B{static int n;int f(){return sizeof(T);}int later();\
            int bad() __attribute__((exclude_from_explicit_instantiation)){return T::bad;}\
            struct I{static int n;int g(){return sizeof(T);}};};\
            template<class T>int B<T>::n=sizeof(T);template<class T>int B<T>::I::n=sizeof(T)+1;\
            template struct B<int>;template<class T>int B<T>::later(){return T::bad;}\
            int main(){return B<int>::n+B<int>::n;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"explicit-members").unwrap());
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
        assert_eq!(graph.lower_explicit_classes().len(), 2);
        assert_eq!(graph.lower_demanded_variables().len(), 2);
        let first = graph.function_templates.bodies.body(roots[0]).unwrap();
        let second = graph.function_templates.bodies.body(roots[1]).unwrap();
        assert_ne!(first, second);
        assert!(
            roots
                .iter()
                .all(|&id| graph.function_templates.instantiations.defined(id))
        );
        assert!(
            graph
                .lower_demanded_variables()
                .iter()
                .all(|&id| graph.lower_instance_initializer(id).is_some())
        );
    }
}
