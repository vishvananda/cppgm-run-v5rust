// Translated from class_templates.cpp::InstantiationContext. See NOTICE.
//! Isolate required instantiation from the expression/declaration demanding it.
use super::{
    Error,
    analyzer::{Analyzer, DeferredMember, SpecializationMode},
    declaration_access::DeclarationAccess,
    graph::{EntityId, ScopeId},
    procedural_statements::BodyContext,
    templates::HeadId,
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

// analyzer.h::kInstantiationDepthLimit. This logical limit is independent of
// the genuine native stack-byte budget (Annex B requires at least 1024).
pub(super) const MAX_INSTANTIATION_DEPTH: u32 = 2048;

/// A substitution query keeps the caller's soft activation and leaves its own
/// rejection recorded. A required instantiation isolates and consumes it.
#[derive(Clone, Copy)]
pub(super) enum InstantiationMode {
    Required,
    Substitution,
}

#[derive(Default)]
pub(super) struct Levels(HashMap<EntityId, u32>);
impl Levels {
    pub(super) fn record(&mut self, entity: EntityId, level: u32) {
        self.0.entry(entity).or_insert(level);
    }
    pub(super) fn get(&self, entity: EntityId) -> Result<u32, Error> {
        self.0
            .get(&entity)
            .copied()
            .ok_or(Error::Invariant("instantiation without formation level"))
    }
}

#[derive(Default)]
struct Context {
    access: DeclarationAccess,
    body: Option<BodyContext>,
    head: Option<HeadId>,
    owner: Option<EntityId>,
    operand_home: Option<ScopeId>,
    default_scope: Option<ScopeId>,
    closure_default_argument: Option<(EntityId, u32)>,
    friend: Option<ScopeId>,
    initializing: Option<EntityId>,
    initializing_declarator: Option<ast::DeclaratorId>,
    base: Option<EntityId>,
    member: Option<EntityId>,
    reserved: super::reserved_wrappers::Context,
    template_depth: usize,
    specialization_levels: u32,
    specialization_mode: SpecializationMode,
    instantiation_depth: u32,
    class_depth: usize,
    unevaluated: u32,
    template_arguments: u32,
    dependent_uses: std::cell::Cell<u64>,
    allocation: super::allocation::GlobalAllocationContext,
    c_language: bool,
    linkage: Option<super::linkage::Specification>,
    suppress_implicit: bool,
    deferred: Vec<DeferredMember>,
}
impl Context {
    fn swap(&mut self, analyzer: &mut Analyzer<'_>) {
        use std::mem::swap;
        swap(&mut self.access, &mut analyzer.declaration_access);
        swap(&mut self.body, &mut analyzer.body_context);
        swap(&mut self.head, &mut analyzer.template_head);
        swap(&mut self.owner, &mut analyzer.template_owner);
        swap(&mut self.operand_home, &mut analyzer.operand_home);
        swap(
            &mut self.default_scope,
            &mut analyzer.default_argument_scope,
        );
        swap(
            &mut self.closure_default_argument,
            &mut analyzer.closure_default_argument,
        );
        swap(&mut self.friend, &mut analyzer.namespace_friend_class);
        swap(&mut self.initializing, &mut analyzer.initializing);
        swap(
            &mut self.initializing_declarator,
            &mut analyzer.initializing_declarator,
        );
        swap(&mut self.base, &mut analyzer.base_object);
        swap(&mut self.member, &mut analyzer.member_initializer_class);
        swap(&mut self.reserved, &mut analyzer.reserved_context);
        swap(&mut self.template_depth, &mut analyzer.template_depth);
        swap(
            &mut self.specialization_mode,
            &mut analyzer.specialization_mode,
        );
        swap(
            &mut self.specialization_levels,
            &mut analyzer.specialization_levels,
        );
        swap(
            &mut self.instantiation_depth,
            &mut analyzer.instantiation_depth,
        );
        swap(&mut self.class_depth, &mut analyzer.class_depth);
        swap(&mut self.unevaluated, &mut analyzer.unevaluated);
        swap(
            &mut self.template_arguments,
            &mut analyzer.template_arguments,
        );
        swap(&mut self.dependent_uses, &mut analyzer.dependent_uses);
        swap(
            &mut self.allocation,
            &mut analyzer.global_allocation_context,
        );
        swap(&mut self.c_language, &mut analyzer.c_language);
        swap(&mut self.linkage, &mut analyzer.linkage_specification);
        swap(&mut self.suppress_implicit, &mut analyzer.suppress_implicit);
        swap(&mut self.deferred, &mut analyzer.deferred);
        // Class completion owns global throw/override suffixes by prefix
        // marks; an instantiation must preserve the enclosing events.
    }
}
impl Analyzer<'_> {
    pub(super) fn required_instantiation<T>(
        &mut self,
        formation_depth: u32,
        location: SourceOffset,
        limit: usize,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.instantiation_context(formation_depth, location, limit, action)
            .map_err(Error::required_instantiation)
    }
    pub(super) fn required_unlimited_instantiation<T>(
        &mut self,
        formation_depth: u32,
        location: SourceOffset,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.with_instantiation_limit(
            formation_depth,
            location,
            None,
            InstantiationMode::Required,
            action,
        )
        .map_err(Error::required_instantiation)
    }
    pub(super) fn instantiation_context<T>(
        &mut self,
        formation_depth: u32,
        location: SourceOffset,
        limit: usize,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.instantiation_context_in(
            formation_depth,
            location,
            limit,
            InstantiationMode::Required,
            action,
        )
    }
    pub(super) fn instantiation_context_in<T>(
        &mut self,
        formation_depth: u32,
        location: SourceOffset,
        limit: usize,
        mode: InstantiationMode,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.with_instantiation_limit(formation_depth, location, Some(limit), mode, action)
    }
    pub(super) fn unlimited_instantiation_context_in<T>(
        &mut self,
        formation_depth: u32,
        location: SourceOffset,
        mode: InstantiationMode,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.with_instantiation_limit(formation_depth, location, None, mode, action)
    }
    fn with_instantiation_limit<T>(
        &mut self,
        formation_depth: u32,
        location: SourceOffset,
        limit: Option<usize>,
        mode: InstantiationMode,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.check_stack()?;
        // The oracle's logical Annex B limit is separate from the native
        // byte-budget guard. A pre-existing shell retains its formation level.
        if formation_depth >= MAX_INSTANTIATION_DEPTH {
            return Err(
                Error::at(location, "template instantiation nested too deeply")
                    .required_instantiation(),
            );
        }
        let mut saved = Context::default();
        saved.swap(self);
        self.instantiation_depth = formation_depth + 1;
        let saved_limit = std::mem::replace(&mut self.graph.lookup_limit, limit);
        let result = self.with_substitution_instantiation(
            matches!(mode, InstantiationMode::Substitution),
            action,
        );
        self.graph.lookup_limit = saved_limit;
        saved.swap(self);
        result
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
    fn logical_limit_is_independent_of_native_recursion_and_restores_context() {
        let source = SourceBuffer::new(b"int value;".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"logical-depth").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let ast::DeclKind::Simple { declarators, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            panic!("object declaration");
        };
        let declarator = tree.init_declarators.get(declarators)[0].declarator;
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.graph.lookup_limit = Some(7);
        let unrestricted = analyzer.required_unlimited_instantiation(0, SourceOffset(0), |inner| {
            assert!(inner.graph.lookup_limit.is_none());
            Err::<(), _>(Error::at(SourceOffset(0), "unrestricted context test"))
        });
        assert!(matches!(
            unrestricted,
            Err(Error::InstantiationDiagnostic {
                message: "unrestricted context test",
                ..
            })
        ));
        assert_eq!(analyzer.graph.lookup_limit, Some(7));
        analyzer.graph.lookup_limit = None;
        analyzer.instantiation_depth = 7;
        analyzer.global_allocation_context =
            super::super::allocation::GlobalAllocationContext::Declaration;
        analyzer.dependent_uses.set(11);
        analyzer.operand_home = Some(analyzer.graph.global);
        analyzer.initializing_declarator = Some(declarator);
        analyzer.declaration_access.before_owner_declarator = true;
        let function = analyzer
            .graph
            .add_entity(None, None, crate::lex::vocabulary::Keyword::Int)
            .unwrap();
        analyzer.closure_default_argument = Some((function, 2));
        let substitution = analyzer.graph.analysis.substitution.clone();
        for mode in [InstantiationMode::Required, InstantiationMode::Substitution] {
            use super::super::substitution_state::{Ambient, Failure};
            substitution.replace(Ambient {
                soft: 2,
                owner: Some(function),
                failure: Some(Failure {
                    location: SourceOffset(3),
                    message: "outer",
                    transient: false,
                }),
            });
            let result = analyzer.instantiation_context_in(9, SourceOffset(0), 5, mode, |inner| {
                let state = substitution.current();
                assert_eq!(
                    state.soft,
                    if matches!(mode, InstantiationMode::Substitution) {
                        2
                    } else {
                        0
                    }
                );
                assert_eq!(state.owner, Some(function));
                assert!(state.failure.is_none());
                assert_eq!(inner.instantiation_depth, 10);
                assert_eq!(inner.graph.lookup_limit, Some(5));
                assert!(inner.initializing_declarator.is_none());
                substitution.replace(Ambient {
                    failure: Some(Failure {
                        location: SourceOffset(4),
                        message: "inner",
                        transient: true,
                    }),
                    ..substitution.current()
                });
                Err::<(), _>(Error::Missing("preserving query test"))
            });
            assert!(result.is_err());
            let restored = substitution.current();
            assert_eq!(restored.soft, 2);
            assert_eq!(restored.owner, Some(function));
            assert_eq!(
                restored.failure.unwrap().message,
                if matches!(mode, InstantiationMode::Substitution) {
                    "inner"
                } else {
                    "outer"
                }
            );
            assert_eq!(analyzer.instantiation_depth, 7);
            assert_eq!(analyzer.graph.lookup_limit, None);
            assert_eq!(analyzer.initializing_declarator, Some(declarator));
        }
        substitution.replace(super::super::substitution_state::Ambient::default());
        let result = analyzer.instantiation_context(2047, SourceOffset(0), 0, |inner| {
            assert!(matches!(
                inner.global_allocation_context,
                super::super::allocation::GlobalAllocationContext::Instantiation
            ));
            assert_eq!(inner.instantiation_depth, 2048);
            assert!(inner.operand_home.is_none());
            assert!(inner.initializing_declarator.is_none());
            assert!(!inner.declaration_access.before_owner_declarator);
            assert!(inner.closure_default_argument.is_none());
            inner.note_dependent_use();
            Ok(())
        });
        assert!(result.is_ok());
        assert!(matches!(
            analyzer.global_allocation_context,
            super::super::allocation::GlobalAllocationContext::Declaration
        ));
        assert_eq!(analyzer.instantiation_depth, 7);
        assert_eq!(analyzer.dependent_uses.get(), 11);
        assert_eq!(analyzer.operand_home, Some(analyzer.graph.global));
        assert_eq!(analyzer.initializing_declarator, Some(declarator));
        assert!(analyzer.declaration_access.before_owner_declarator);
        assert_eq!(analyzer.closure_default_argument, Some((function, 2)));
        let result = analyzer.instantiation_context(0, SourceOffset(0), 0, |inner| {
            assert!(matches!(
                inner.global_allocation_context,
                super::super::allocation::GlobalAllocationContext::Instantiation
            ));
            assert!(inner.initializing_declarator.is_none());
            inner.note_dependent_use();
            Err::<(), _>(Error::Missing("test substitution"))
        });
        assert!(result.is_err());
        assert!(matches!(
            analyzer.global_allocation_context,
            super::super::allocation::GlobalAllocationContext::Declaration
        ));
        assert_eq!(analyzer.dependent_uses.get(), 11);
        assert_eq!(analyzer.initializing_declarator, Some(declarator));
        assert_eq!(analyzer.closure_default_argument, Some((function, 2)));
        let result =
            analyzer.instantiation_context(2048, SourceOffset(0), 0, |_| -> Result<(), Error> {
                panic!("logical limit must precede instantiation");
            });
        assert!(matches!(
            result,
            Err(Error::InstantiationDiagnostic {
                message: "template instantiation nested too deeply",
                ..
            })
        ));
        assert_eq!(analyzer.instantiation_depth, 7);
    }
}
