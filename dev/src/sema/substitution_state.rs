// Translated from analyzer.h and instantiation_context.h. See NOTICE.
//! Ambient substitution facts outlive borrowing analyzer facades and local memos.
use super::{Error, analyzer::Analyzer, graph::EntityId};
use crate::support::source::SourceOffset;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
pub(super) struct Failure {
    pub location: SourceOffset,
    pub message: &'static str,
    pub transient: bool,
}
#[derive(Clone, Copy, Default)]
pub(super) struct Ambient {
    pub soft: u32,
    pub owner: Option<EntityId>,
    pub failure: Option<Failure>,
}
#[derive(Clone, Default)]
pub(super) struct State(Rc<Cell<Ambient>>);
impl State {
    pub(super) fn current(&self) -> Ambient {
        self.0.get()
    }
    pub(super) fn replace(&self, state: Ambient) -> Ambient {
        self.0.replace(state)
    }
    pub(super) fn reject(&self, failure: Failure) {
        let mut state = self.current();
        state.failure = Some(match state.failure {
            Some(first) => Failure {
                location: first.location,
                message: first.message,
                transient: first.transient || failure.transient,
            },
            None => failure,
        });
        self.replace(state);
    }
}

impl Analyzer<'_> {
    // Counter/admission ownership remains at the translated caller until its
    // complete failure-producing operation has been reconciled. These scopes
    // preserve the ambient facts independently of local Result transport.
    // Pass the retained current owner to inherit it; None explicitly clears it.
    pub(super) fn with_substitution_soft<T>(
        &mut self,
        owner: Option<EntityId>,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let state = self.graph.analysis.substitution.clone();
        let outer = state.current();
        let soft = outer
            .soft
            .checked_add(1)
            .ok_or(Error::Limit("substitution nesting limit"))?;
        state.replace(Ambient {
            soft,
            owner,
            failure: None,
        });
        let result = action(self);
        // The explicit owner scope ends before the soft scope restores its
        // saved failure. No local type memo owns either ambient fact.
        let mut inner = state.current();
        inner.owner = outer.owner;
        state.replace(inner);
        state.replace(outer);
        result
    }
    pub(super) fn with_substitution_instantiation<T>(
        &mut self,
        preserve: bool,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let state = self.graph.analysis.substitution.clone();
        let outer = state.current();
        state.replace(Ambient {
            soft: if preserve { outer.soft } else { 0 },
            failure: None,
            ..outer
        });
        let result = action(self);
        let inner = state.current();
        state.replace(Ambient {
            soft: outer.soft,
            failure: if preserve && inner.failure.is_some() {
                inner.failure
            } else {
                outer.failure
            },
            ..inner
        });
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lex::vocabulary::Keyword,
        preprocess::unit::Preprocessor,
        sema::types::Scalar,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn required_context_keeps_latent_owner_and_scopes_restore_failure_on_error() {
        let source = SourceBuffer::new(Vec::new()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"ambient-scopes").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, _) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let ty = analyzer.graph.types.scalar(Scalar::Bool).unwrap();
        let owner = analyzer
            .graph
            .add_entity(Some(ty), None, Keyword::Bool)
            .unwrap();
        let state = analyzer.graph.analysis.substitution.clone();
        state.replace(Ambient {
            soft: 2,
            owner: Some(owner),
            failure: Some(Failure {
                location: SourceOffset(7),
                message: "outer",
                transient: false,
            }),
        });
        for fail in [false, true] {
            let result = analyzer.with_substitution_instantiation(false, |analyzer| {
                let inner = state.current();
                assert_eq!(inner.soft, 0);
                assert_eq!(inner.owner, Some(owner));
                assert!(inner.failure.is_none());
                let scoped = analyzer.with_substitution_soft(Some(owner), |_| {
                    let inner = state.current();
                    assert_eq!(inner.soft, 1);
                    assert_eq!(inner.owner, Some(owner));
                    state.reject(Failure {
                        location: SourceOffset(9),
                        message: "child",
                        transient: true,
                    });
                    if fail {
                        Err(Error::Invariant("scope failure"))
                    } else {
                        Ok(())
                    }
                });
                assert_eq!(state.current().soft, 0);
                assert!(state.current().failure.is_none());
                scoped
            });
            assert_eq!(result.is_err(), fail);
            let restored = state.current();
            assert_eq!(restored.soft, 2);
            assert_eq!(restored.owner, Some(owner));
            assert_eq!(restored.failure.unwrap().location, SourceOffset(7));
            assert_eq!(restored.failure.unwrap().message, "outer");
        }
        let result: Result<(), Error> = analyzer.with_substitution_instantiation(true, |_| {
            assert_eq!(state.current().soft, 2);
            assert!(state.current().failure.is_none());
            state.reject(Failure {
                location: SourceOffset(11),
                message: "preserved",
                transient: false,
            });
            state.reject(Failure {
                location: SourceOffset(12),
                message: "later",
                transient: true,
            });
            Err(Error::Invariant("preserving failure"))
        });
        assert!(result.is_err());
        let inner = state.current().failure.unwrap();
        assert_eq!(inner.location, SourceOffset(11));
        assert_eq!(inner.message, "preserved");
        assert!(inner.transient);
    }

    #[test]
    fn a_new_analyzer_facade_retains_the_unit_owned_ambient_store() {
        let source = SourceBuffer::new(Vec::new()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"ambient-facade").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let store = analyzer.graph.analysis.substitution.clone();
        store.reject(Failure {
            location: SourceOffset(13),
            message: "retained",
            transient: false,
        });
        let graph = analyzer.analyze(declarations).unwrap();
        let facade =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(graph)).unwrap();
        assert!(Rc::ptr_eq(&store.0, &facade.graph.analysis.substitution.0));
        assert_eq!(
            facade
                .graph
                .analysis
                .substitution
                .current()
                .failure
                .unwrap()
                .message,
            "retained"
        );
    }

    #[test]
    fn a_repeated_type_demand_emits_rejection_in_each_new_soft_scope() {
        use crate::sema::{
            function_templates::Substitution,
            lookup::Filter,
            types::{RefKind, TypeKind},
        };
        let source = SourceBuffer::new(b"template<class T>T* f(T);".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"substitution-reentry").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
        let entity = analyzer.graph.binding(binding).entity;
        let head = analyzer.function_template_head(binding).unwrap().unwrap();
        let depth = analyzer.graph.templates.depth(head);
        let TypeKind::Function { result, .. } = analyzer
            .graph
            .types
            .get(analyzer.graph.binding(binding).ty.unwrap())
            .kind
        else {
            panic!("function type");
        };
        let integer = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::Int))
            .unwrap();
        let reference = analyzer
            .graph
            .types
            .reference(integer, RefKind::Lvalue)
            .unwrap();
        let arguments = [Some(reference)];
        let mut environment = Substitution::new(depth, &arguments);
        let state = analyzer.graph.analysis.substitution.clone();
        for _ in 0..2 {
            analyzer
                .with_substitution_soft(Some(entity), |analyzer| {
                    assert!(state.current().failure.is_none());
                    assert!(
                        analyzer
                            .substitute_signature_type(result, &mut environment)?
                            .is_none()
                    );
                    assert!(state.current().failure.is_some());
                    Ok(())
                })
                .unwrap();
            assert!(state.current().failure.is_none());
        }
    }
}
