// Translated from template_specializations.cpp::specialized_class,
// class_explicit_specialization and analyzer_classes.cpp::explicit_member_class.
// See NOTICE.
//! Explicit class definitions own their members, independently of the primary.
use super::{
    Error,
    analyzer::{Analyzer, SpecializationMode},
    graph::{BindingKind, EntityId, ScopeId},
    lookup::Filter,
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

impl Analyzer<'_> {
    pub(super) fn specialized_class(
        &mut self,
        name: ast::Name,
        key: Keyword,
        scope: ScopeId,
    ) -> Result<EntityId, Error> {
        self.specialization_mode = SpecializationMode::Ordinary;
        let location = self.tree.names.get(name).location;
        let access = self.begin_declaration_access(scope, true);
        let found = self.lookup(name, scope, Filter::Elaborated);
        self.finish_declaration_access(access);
        let binding = self.graph.binding(found?);
        if binding.kind != BindingKind::Type
            || !binding
                .ty
                .is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)))
        {
            return Err(Error::at(
                location,
                "an explicit specialization names no class template specialization",
            ));
        }
        let entity = binding.entity;
        let Some(instance) = self.graph.class_templates.instance(entity) else {
            return Err(Error::at(
                location,
                "an explicit specialization names no class template specialization",
            ));
        };
        if instance.dependent {
            return Err(Error::at(
                location,
                "an explicit specialization names no class template specialization",
            ));
        }
        let definitions = &self.graph.class_templates.definitions;
        if !definitions.explicit(entity) && definitions.started(entity) {
            return Err(Error::at(
                location,
                "an explicit specialization after the specialization's instantiation",
            ));
        }
        if (key == Keyword::Union) != (self.graph.entity(entity).key == Keyword::Union) {
            return Err(Error::at(
                location,
                "class-key does not agree with the template's",
            ));
        }
        let home = self
            .graph
            .entity(instance.primary)
            .home
            .ok_or(Error::Invariant("class template without namespace"))?;
        if !self.graph.encloses(self.declaration_scope(scope), home) {
            return Err(Error::at(
                location,
                "an explicit specialization outside its template's namespace",
            ));
        }
        // Even a declaration prevents subsequent primary-definition demand.
        // A later explicit definition completes this same canonical shell.
        self.graph.class_templates.definitions.specialize(entity);
        Ok(entity)
    }

    pub(super) fn class_explicit_specialization(
        &mut self,
        node: ast::ClassSyntax,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let entity = self.specialized_class(name, node.key, scope)?;
        if self.graph.entity(entity).scope.is_some() {
            return Err(Error::at(
                self.tree.names.get(name).location,
                "class redefinition",
            ));
        }
        let home = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("class specialization without owner"))?;
        self.class_body(node, entity, home, Some(scope))
    }

    pub(super) fn explicit_member_class(
        &mut self,
        entity: EntityId,
        owner: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.specialization_mode = SpecializationMode::Ordinary;
        if !self.graph.within_class_instance(owner)
            || self
                .graph
                .class_templates
                .definitions
                .source(entity)
                .is_none()
        {
            return Err(Error::at(
                location,
                "an explicit specialization names no member class of a class template specialization",
            ));
        }
        if self.graph.class_templates.definitions.started(entity)
            || self.graph.entity(entity).scope.is_some()
        {
            return Err(Error::at(
                location,
                "an explicit specialization after the member class's instantiation",
            ));
        }
        self.graph.class_templates.definitions.specialize(entity);
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

    fn alias_class(
        graph: &super::super::graph::Graph,
        binding: super::super::graph::BindingId,
    ) -> EntityId {
        let TypeKind::Class(class) = graph.types.get(graph.binding(binding).ty.unwrap()).kind
        else {
            panic!("alias lost its class identity");
        };
        class
    }

    #[test]
    fn explicit_declarations_keep_shell_identity_and_block_primary_definition_demand() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{typename T::missing bad;};\
              using I=A<int>;template<>struct A<int>;\
              template<>struct A<char>;using C=A<char>;\
              template<>struct A<int>{struct Nested{int x;};int f(){return 3;}};\
              using J=A<int>;int size=sizeof(J::Nested);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"explicit-shell").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"I".as_slice(), b"J", b"C", b"Nested", b"f"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let original_bodies = tree.bodies.mark();
        let original_expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert!(!analyzer.specializing());
        assert_eq!(analyzer.specialization_levels, 0);
        let class = |index| {
            alias_class(
                &analyzer.graph,
                analyzer
                    .graph
                    .own(global, names[index], Filter::Ordinary)
                    .unwrap(),
            )
        };
        let (i, j, c) = (class(0), class(1), class(2));
        assert_eq!(i, j);
        assert_ne!(i, c);
        assert!(analyzer.complete_class(i).unwrap());
        assert!(!analyzer.complete_class(c).unwrap());
        assert!(analyzer.graph.entity(c).scope.is_none());
        assert!(analyzer.graph.class_templates.definitions.explicit(c));
        let scope = analyzer.graph.entity(i).scope.unwrap();
        assert!(!analyzer.graph.within_class_instance(scope));
        let nested = analyzer
            .graph
            .binding(
                analyzer
                    .graph
                    .own(scope, names[3], Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        let function = analyzer
            .graph
            .binding(
                analyzer
                    .graph
                    .own(scope, names[4], Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        assert!(
            analyzer
                .graph
                .class_templates
                .definitions
                .source(nested)
                .is_none()
        );
        assert!(analyzer.graph.entity(nested).complete);
        assert!(!analyzer.graph.function_templates.bodies.member(function));
        assert!(!analyzer.graph.function_templates.bodies.started(function));
        assert_eq!(analyzer.tree.bodies.mark(), original_bodies);
        assert_eq!(analyzer.tree.expressions.mark(), original_expressions);
    }

    #[test]
    fn member_specializations_own_their_definitions_and_nested_members() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{struct B{typename T::missing bad;};\
                void unused(){T::missing;}};\
              template<>struct A<int>::B{struct C{int x;};int f(){return 4;}};\
              using I=A<int>::B;int size=sizeof(I::C);"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"own-member").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"I").unwrap();
        let nested_name = cursor.intern_name(b"C").unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let member = alias_class(
            &graph,
            graph.own(graph.global, name, Filter::Ordinary).unwrap(),
        );
        assert!(graph.class_templates.definitions.explicit(member));
        assert!(graph.class_templates.definitions.source(member).is_some());
        assert!(graph.entity(member).complete);
        let scope = graph.entity(member).scope.unwrap();
        assert!(!graph.within_class_instance(scope));
        let nested = graph
            .binding(graph.own(scope, nested_name, Filter::Ordinary).unwrap())
            .entity;
        assert!(graph.class_templates.definitions.source(nested).is_none());
        assert!(graph.entity(nested).complete);
        // Completing A<int> to name B does not demand unused's invalid body.
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn failed_class_specialization_restores_its_introducer_context() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{int x;};A<int>a;template<>struct A<int>{};".to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"late-class").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        let result = analyzer.declarations(declarations, global);
        assert!(matches!(
            result,
            Err(Error::Diagnostic {
                message: "an explicit specialization after the specialization's instantiation",
                ..
            })
        ));
        assert!(!analyzer.specializing());
        assert_eq!(analyzer.specialization_levels, 0);
        assert_eq!(analyzer.template_depth, 0);
    }
}
