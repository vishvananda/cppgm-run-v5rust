// Translated from analyzer.h and instantiation_context.h. See NOTICE.
//! Ambient substitution facts outlive borrowing analyzer facades and local memos.
use super::{Error, analyzer::Analyzer, graph::EntityId};
use crate::support::source::SourceOffset;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub(crate) location: SourceOffset,
    pub(crate) message: &'static str,
    pub(crate) transient: bool,
}
#[derive(Clone, Copy, Default)]
pub(super) struct Ambient {
    pub soft: u32,
    pub owner: Option<EntityId>,
    pub failure: Option<Failure>,
}
#[derive(Clone, Default)]
pub(super) struct State(Rc<Store>);
struct Store {
    ambient: Cell<Ambient>,
    alias_location: Cell<SourceOffset>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            ambient: Cell::new(Ambient::default()),
            alias_location: Cell::new(SourceOffset(0)),
        }
    }
}
impl State {
    pub(super) fn current(&self) -> Ambient {
        self.0.ambient.get()
    }
    pub(super) fn replace(&self, state: Ambient) -> Ambient {
        self.0.ambient.replace(state)
    }
    pub(super) fn alias_location(&self) -> SourceOffset {
        self.0.alias_location.get()
    }
    pub(super) fn replace_alias_location(&self, location: SourceOffset) -> SourceOffset {
        self.0.alias_location.replace(location)
    }
    pub(super) fn hard_error(&self, location: SourceOffset, message: &'static str) -> Error {
        Error::UnconditionalDiagnostic {
            location: if location.0 == 0 {
                self.alias_location()
            } else {
                location
            },
            message,
        }
    }
    pub(super) fn emit(&self, failure: Failure) -> Result<Failure, Error> {
        if self.current().soft == 0 {
            return Err(self.hard_error(failure.location, failure.message));
        }
        self.record(failure);
        Ok(self.current().failure.unwrap_or(failure))
    }
    pub(super) fn quiet<T>(
        &self,
        location: SourceOffset,
        message: &'static str,
    ) -> Result<Option<T>, Error> {
        if self.current().soft == 0 {
            Err(self.hard_error(location, message))
        } else {
            Ok(None)
        }
    }
    fn record(&self, failure: Failure) {
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
    pub(super) fn resolve_hard_error(&self, error: Error) -> Error {
        match error {
            Error::UnconditionalDiagnostic {
                location: SourceOffset(0),
                message,
            } => self
                .graph
                .analysis
                .substitution
                .hard_error(SourceOffset(0), message),
            Error::InstantiationDiagnostic {
                location: SourceOffset(0),
                message,
            } => Error::InstantiationDiagnostic {
                location: self.graph.analysis.substitution.alias_location(),
                message,
            },
            other => other,
        }
    }
    /// Adapt a source emit-and-null branch whose Rust result has no null value.
    pub(super) fn reject_substitution<T>(
        &self,
        location: SourceOffset,
        message: &'static str,
        transient: bool,
    ) -> Result<T, Error> {
        let failure = self.graph.analysis.substitution.emit(Failure {
            location,
            message,
            transient,
        })?;
        Err(Error::SubstitutionRejected(failure))
    }
    pub(super) fn with_alias_substitution_location<T>(
        &mut self,
        location: SourceOffset,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let state = self.graph.analysis.substitution.clone();
        let outer = state.alias_location();
        if location.0 != 0 {
            state.replace_alias_location(location);
        }
        let result = action(self);
        state.replace_alias_location(outer);
        result
    }
    // Pass the retained current owner to inherit it; None explicitly clears it.
    pub(super) fn with_substitution_soft<T>(
        &mut self,
        owner: Option<EntityId>,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.substitution_soft_scope(owner, true, action)
    }
    pub(super) fn with_raw_substitution_soft<T>(
        &mut self,
        owner: Option<EntityId>,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.substitution_soft_scope(owner, false, action)
    }
    fn substitution_soft_scope<T>(
        &mut self,
        owner: Option<EntityId>,
        count_failure: bool,
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
        // The owner's scope ends before SoftSubstitution's destructor charges
        // this lifetime's current failure, then consumes it and restores outer F.
        let inner = state.current();
        state.replace(Ambient {
            owner: outer.owner,
            ..inner
        });
        if count_failure {
            self.graph.work.totals.substitution_failures += usize::from(inner.failure.is_some());
        }
        state.replace(outer);
        result
    }
    pub(super) fn substitution_trial<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<Option<T>, Error> {
        let owner = self.graph.analysis.substitution.current().owner;
        self.with_substitution_soft(owner, |analyzer| {
            let result = action(analyzer);
            match result {
                Ok(value)
                    if analyzer
                        .graph
                        .analysis
                        .substitution
                        .current()
                        .failure
                        .is_none() =>
                {
                    Ok(Some(value))
                }
                Ok(_) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => Ok(None),
                Err(error) => Err(error),
            }
        })
    }
    pub(super) fn with_substitution_owner<T>(
        &mut self,
        owner: Option<EntityId>,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let state = self.graph.analysis.substitution.clone();
        let outer = state.current().owner;
        state.replace(Ambient {
            owner,
            ..state.current()
        });
        let result = action(self);
        state.replace(Ambient {
            owner: outer,
            ..state.current()
        });
        result
    }
    /// A required noexcept expression or lambda body only suspends soft mode;
    /// unlike instantiation entry, it neither clears nor consumes failure.
    pub(super) fn without_soft_substitution<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let state = self.graph.analysis.substitution.clone();
        let soft = state.current().soft;
        state.replace(Ambient {
            soft: 0,
            ..state.current()
        });
        let result = action(self);
        state.replace(Ambient {
            soft,
            ..state.current()
        });
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
    fn alias_sites_resolve_hard_errors_but_soft_failure_keeps_its_raw_origin() {
        use crate::sema::{lookup::Filter, types::RefKind};
        let source = SourceBuffer::new(b"template<class T>using A=T*;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"alias-rejection-site").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"A").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
        let alias = analyzer.graph.binding(binding).entity;
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
        let state = analyzer.graph.analysis.substitution.clone();
        state.replace_alias_location(SourceOffset(3));
        for (location, expected) in [(21, 21), (0, 3)] {
            assert!(
                matches!(analyzer.alias_specialization(alias, vec![reference], SourceOffset(location)),
                Err(Error::UnconditionalDiagnostic { location, message: "pointer to reference" }) if location == SourceOffset(expected))
            );
            assert!(state.current().failure.is_none());
            assert_eq!(state.alias_location(), SourceOffset(3));
        }
        let result = analyzer.with_substitution_soft(None, |analyzer| {
            analyzer.alias_specialization(alias, vec![reference], SourceOffset(21))
        });
        assert!(matches!(
            result,
            Err(Error::SubstitutionRejected(Failure {
                location: SourceOffset(0),
                message: "pointer to reference",
                transient: false
            }))
        ));
        assert!(state.current().failure.is_none());
        assert_eq!(state.alias_location(), SourceOffset(3));
    }

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
                    state.record(Failure {
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
            state.record(Failure {
                location: SourceOffset(11),
                message: "preserved",
                transient: false,
            });
            state.record(Failure {
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
        state.replace(Ambient {
            failure: None,
            ..state.current()
        });
        let result: Result<(), Error> = analyzer.without_soft_substitution(|_| {
            assert_eq!(state.current().soft, 0);
            assert_eq!(state.current().owner, Some(owner));
            state.record(Failure {
                location: SourceOffset(15),
                message: "required expression",
                transient: false,
            });
            Err(Error::Invariant("required expression test"))
        });
        assert!(result.is_err());
        assert_eq!(state.current().soft, 2);
        assert_eq!(state.current().owner, Some(owner));
        assert_eq!(
            state.current().failure.unwrap().message,
            "required expression"
        );
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
        store.record(Failure {
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
    fn a_class_default_root_rejects_in_the_live_scope_before_type_dispatch() {
        use crate::sema::lookup::Filter;
        let source = SourceBuffer::new(b"template<class T>struct A;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"class-default-root").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"A").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
        let primary = analyzer.graph.binding(binding).entity;
        let mut environment = analyzer.substitution(0, &[]);
        let state = analyzer.graph.analysis.substitution.clone();
        // The environment predates its soft scope. A missing argument rejects
        // without entering either subtype or pack-list substitution.
        analyzer
            .with_substitution_soft(Some(primary), |analyzer| {
                assert!(state.current().failure.is_none());
                let result = analyzer.with_substitution_owner(None, |analyzer| {
                    assert!(
                        analyzer
                            .substitute_class_template_defaults(
                                primary,
                                Vec::new(),
                                &mut environment
                            )?
                            .is_none()
                    );
                    Err::<(), _>(Error::Invariant("owner scope test"))
                });
                assert!(result.is_err());
                assert_eq!(state.current().owner, Some(primary));
                assert_eq!(state.current().soft, 1);
                let failure = state.current().failure.unwrap();
                assert_eq!(failure.message, "too few template arguments");
                assert_eq!(failure.location, SourceOffset(0));
                Ok(())
            })
            .unwrap();
        assert!(state.current().failure.is_none());
        assert_eq!(
            environment.failure.map(|failure| failure.message),
            Some("too few template arguments")
        );
    }

    #[test]
    fn a_repeated_type_demand_emits_rejection_in_each_new_soft_scope() {
        use crate::sema::{
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
        let mut environment = analyzer.substitution(depth, &arguments);
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
