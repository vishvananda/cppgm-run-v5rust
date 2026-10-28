// Translated from ConstantInterpreter construction/destruction. See NOTICE.
//! Logical queries suspend interpreter control while sharing physical scratch.
use super::{Error, analyzer::Analyzer};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum QueryKind {
    Value,
    Type,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Observations {
    pub met_incomplete_layout: bool,
}
pub(super) struct Query {
    pub kind: QueryKind,
    pub observations: Observations,
    pub memo: Option<Box<super::interpreter_memo::Memo>>,
}
pub(super) struct Observed<T> {
    pub result: Result<T, Error>,
    pub observations: Observations,
}

impl Analyzer<'_> {
    pub(super) fn with_fresh_constant_query<T>(
        &mut self,
        evaluate: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.with_observed_constant_query(evaluate).result
    }

    pub(super) fn with_constant_type_query<T>(
        &mut self,
        evaluate: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.with_constant_query(QueryKind::Type, evaluate).result
    }

    pub(super) fn note_incomplete_constant_layout(&mut self) {
        if let Some(query) = &mut self.constant_query
            && query.kind == QueryKind::Value
        {
            query.observations.met_incomplete_layout = true;
        }
    }
    pub(super) fn with_observed_constant_query<T>(
        &mut self,
        evaluate: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Observed<T> {
        self.with_constant_query(QueryKind::Value, evaluate)
    }

    fn with_constant_query<T>(
        &mut self,
        kind: QueryKind,
        evaluate: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Observed<T> {
        let mut observations = Observations::default();
        let result = self.with_constant_storage(|analyzer| {
            let query = analyzer.constant_query.replace(Query {
                kind,
                observations: Observations::default(),
                memo: None,
            });
            let frames = analyzer.constexpr_frames.enter_evaluation();
            let construction = analyzer.graph.aggregate_constants.suspend_construction();
            let source_site = analyzer.source_site.take();
            if matches!(kind, QueryKind::Value) {
                analyzer.graph.work.totals.constant_evaluations += 1;
            }
            let result = evaluate(analyzer);
            let steps = analyzer.constexpr_frames.leave_evaluation(frames);
            analyzer
                .graph
                .aggregate_constants
                .restore_construction(construction);
            analyzer.source_site = source_site;
            let child = std::mem::replace(&mut analyzer.constant_query, query);
            observations = child
                .ok_or(Error::Invariant("constant query lost its state"))?
                .observations;
            analyzer.graph.work.totals.constant_steps +=
                usize::try_from(steps).map_err(|_| Error::Limit("constant step count limit"))?;
            result
        });
        Observed {
            result,
            observations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{
            source::{SourceBuffer, SourceOffset},
            stack::StackBudget,
        },
        syntax::SyntaxUnit,
    };

    #[test]
    fn observations_survive_failure_without_implicit_propagation_to_the_parent() {
        let source = SourceBuffer::new(Vec::new()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"observations").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, _) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer
            .with_fresh_constant_query(|analyzer| {
                let child: Observed<()> = analyzer.with_observed_constant_query(|analyzer| {
                    analyzer.note_incomplete_constant_layout();
                    Err(Error::Invariant("child failure"))
                });
                assert!(child.result.is_err());
                assert!(child.observations.met_incomplete_layout);
                assert!(
                    !analyzer
                        .constant_query
                        .as_ref()
                        .unwrap()
                        .observations
                        .met_incomplete_layout
                );
                analyzer.note_incomplete_constant_layout();
                let child = analyzer.with_observed_constant_query(|_| Ok(()));
                assert!(!child.observations.met_incomplete_layout);
                assert!(
                    analyzer
                        .constant_query
                        .as_ref()
                        .unwrap()
                        .observations
                        .met_incomplete_layout
                );
                Ok(())
            })
            .unwrap();
        assert!(analyzer.constant_query.is_none());
    }

    #[test]
    fn nested_success_and_failure_restore_source_site_and_outer_query() {
        let source = SourceBuffer::new(b"int value;".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"queries").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, _) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .with_fresh_constant_query(|analyzer| {
                analyzer.source_site = Some(SourceOffset(7));
                for fail in [false, true] {
                    let result = analyzer.with_fresh_constant_query(|analyzer| {
                        assert!(analyzer.source_site.is_none());
                        analyzer.source_site = Some(SourceOffset(9));
                        analyzer.constant_step(SourceOffset(1))?;
                        if fail {
                            Err(Error::Invariant("query failure"))
                        } else {
                            Ok(())
                        }
                    });
                    assert_eq!(result.is_err(), fail);
                    assert_eq!(analyzer.source_site, Some(SourceOffset(7)));
                    assert!(matches!(
                        analyzer.constant_query.as_ref().map(|query| query.kind),
                        Some(QueryKind::Value)
                    ));
                    analyzer.with_constant_type_query(|analyzer| {
                        assert!(analyzer.source_site.is_none());
                        assert!(matches!(
                            analyzer.constant_query.as_ref().map(|query| query.kind),
                            Some(QueryKind::Type)
                        ));
                        analyzer.in_constant_evaluation(|_| Ok(()))
                    })?;
                }
                Ok(())
            })
            .unwrap();
        assert!(analyzer.source_site.is_none());
        assert!(analyzer.constant_query.is_none());
        assert!(analyzer.constant_evaluation.is_none());
        #[cfg(feature = "telemetry")]
        {
            assert_eq!(analyzer.graph.work.totals.constant_evaluations, 3);
            assert_eq!(analyzer.graph.work.totals.constant_steps, 2);
        }
    }
}
