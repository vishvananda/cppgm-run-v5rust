// Translated from constant_evaluator.cpp and ConstantInterpreter::provisional_value.
// See NOTICE.
//! Pending declarations retain requests; each read owns a fresh value snapshot.
use super::{
    Error,
    analyzer::{Analyzer, DeferredMember},
    constant_addresses::{Address, TemporaryValue, nonconstant},
    constants::ConstantState,
    constexpr_frames::Frozen,
    graph::{DataMemberKind, EntityId, ScopeId, ScopeKind},
    interpreter_queries::Observed,
    types::TypeKind,
};
use crate::{support::source::SourceOffset, syntax::ast};

#[derive(Clone, Copy)]
pub(super) struct Initialization {
    pub initializer: Option<ast::Initializer>,
    pub scope: ScopeId,
}
pub(super) enum Snapshot {
    Scalar(TemporaryValue),
    Reference(Address),
    Object(Frozen),
}

impl Analyzer<'_> {
    pub(super) fn defer_observed_initializer<T>(
        &mut self,
        entity: EntityId,
        initialization: Initialization,
        outcome: &Observed<T>,
    ) -> bool {
        if !outcome.observations.met_incomplete_layout
            || !(outcome.result.is_ok() || matches!(outcome.result, Err(Error::NonConstant { .. })))
            || self.class_depth == 0
        {
            return false;
        }
        let variable = self.graph.entity(entity);
        if !variable.constexpr
            || variable.data_member == Some(DataMemberKind::Instance)
            || !variable
                .home
                .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class)
        {
            return false;
        }
        let Some(mut held) = variable.ty else {
            return false;
        };
        loop {
            held = match self.graph.types.get(held).kind {
                TypeKind::Reference { element, .. } | TypeKind::Array { element, .. } => element,
                TypeKind::Class(_) | TypeKind::MemberPointer { .. } => break,
                _ => return false,
            };
        }
        self.graph.entity_mut(entity).constant = ConstantState::None;
        self.graph.constant_addresses.remove(entity);
        self.graph.aggregate_constants.defer(entity, initialization);
        self.deferred
            .push(DeferredMember::ConstantValue(entity, initialization));
        true
    }

    pub(super) fn finish_deferred_constant(
        &mut self,
        entity: EntityId,
        initialization: Initialization,
    ) -> Result<(), Error> {
        self.graph.aggregate_constants.clear_deferred(entity);
        let ty = self.graph.lower_entity_type(entity)?;
        match self.graph.types.get(ty).kind {
            TypeKind::Class(_) | TypeKind::Array { .. } => self.record_aggregate_constant(
                entity,
                initialization.initializer,
                initialization.scope,
                true,
            ),
            _ => self.record_constant_initializer(
                entity,
                initialization
                    .initializer
                    .ok_or(Error::Invariant("deferred scalar without initializer"))?,
                initialization.scope,
            ),
        }
    }

    pub(super) fn deferred_constant_snapshot(
        &mut self,
        entity: EntityId,
    ) -> Result<Option<Snapshot>, Error> {
        if !matches!(self.graph.entity(entity).constant, ConstantState::None) {
            return Ok(None);
        }
        let Some(initialization) = self.graph.aggregate_constants.deferred(entity) else {
            return Ok(None);
        };
        self.graph.entity_mut(entity).constant = ConstantState::Evaluating;
        let result = self.with_fresh_constant_query(|analyzer| {
            analyzer.initialize_deferred_snapshot(entity, initialization)
        });
        self.graph.entity_mut(entity).constant = ConstantState::None;
        match result {
            Ok(value) => {
                // This propagation belongs to a successful provisional read,
                // independently of the child's own incomplete-layout observation.
                self.note_incomplete_constant_layout();
                Ok(Some(value))
            }
            Err(Error::NonConstant { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn initialize_deferred_snapshot(
        &mut self,
        entity: EntityId,
        initialization: Initialization,
    ) -> Result<Snapshot, Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        let location = initialization
            .initializer
            .map_or(SourceOffset(0), |value| value.location);
        match self.graph.types.get(ty).kind {
            TypeKind::Reference { .. } => {
                let initializer = initialization
                    .initializer
                    .ok_or_else(|| nonconstant(location, "a reference without an initializer"))?;
                let value =
                    self.evaluate_address_initializer(entity, initializer, initialization.scope)?;
                if !self.address_publishable(value) {
                    return Err(nonconstant(
                        location,
                        "a constant designates an object without static storage duration",
                    ));
                }
                Ok(Snapshot::Reference(value.pointer()))
            }
            TypeKind::MemberPointer { .. } => {
                let initializer = initialization.initializer.ok_or(Error::Invariant(
                    "deferred member pointer without initializer",
                ))?;
                self.initialized_scalar_value(ty, initializer, initialization.scope)
                    .map(Snapshot::Scalar)
            }
            TypeKind::Class(_) | TypeKind::Array { .. } => self
                .evaluate_aggregate_initializer(
                    entity,
                    initialization.initializer,
                    initialization.scope,
                    location,
                )
                .map(Snapshot::Object),
            _ => Err(Error::Invariant("unsupported deferred initializer type")),
        }
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
    fn each_deferred_read_evaluates_a_fresh_snapshot_even_when_provenance_is_erased() {
        let cases: &[(&[u8], usize, usize)] = &[
            (b"struct B{int m;};struct D:B{static constexpr int D::*p=&B::m;static_assert(p==p,\"same\");};", 5, 17),
            (b"struct B{int m;};struct D:B{static constexpr int D::*q=static_cast<int D::*>(&B::m)==nullptr?nullptr:nullptr;static_assert(q==nullptr,\"null\");};", 4, 28),
            (b"struct B{int m;};struct H{bool value;};struct D:B{static constexpr H h={static_cast<int D::*>(&B::m)!=nullptr};static_assert(h.value,\"true\");};", 4, 24),
            (b"struct B{int m;};struct H{bool value;};struct D:B{static constexpr H a={static_cast<int D::*>(&B::m)!=nullptr};static constexpr H b=a;static_assert(b.value,\"true\");};", 8, 43),
            (b"struct B{int m;};struct H{_Complex int z;bool value;};struct D:B{static constexpr H h={{3,4},static_cast<int D::*>(&B::m)!=nullptr};static constexpr _Complex int c=h.z;static_assert(__real__ c==3 && __imag__ c==4,\"pair\");};", 5, 53),
        ];
        for &(source, evaluations, steps) in cases {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"deferred-reads").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_semantics(true);
            let marker = 0_u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let scope = analyzer.graph.global;
            analyzer.declarations(declarations, scope).unwrap();
            assert!(analyzer.deferred.is_empty());
            assert!(analyzer.constant_query.is_none());
            #[cfg(feature = "telemetry")]
            {
                assert_eq!(analyzer.graph.work.totals.constant_evaluations, evaluations);
                assert_eq!(analyzer.graph.work.totals.constant_steps, steps);
            }
            #[cfg(not(feature = "telemetry"))]
            let _ = (evaluations, steps);
        }
    }

    #[test]
    fn a_deferred_reference_is_unknown_to_direct_reference_designation() {
        let source = SourceBuffer::new(b"struct B{int m;};struct H{bool value;};struct D:B{static constexpr const H&r=H{static_cast<int D::*>(&B::m)!=nullptr};static_assert(r.value,\"true\");};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"deferred-reference").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        assert!(matches!(
            analyzer.declarations(declarations, scope),
            Err(Error::UnconditionalDiagnostic { message, .. })
                if message.contains("a reference is not usable in a constant expression")
        ));
    }

    #[test]
    fn failed_complex_snapshot_is_not_retried_and_volatile_views_do_not_demand_it() {
        use super::super::{constant_addresses::Root, lookup::Filter, types::Cv};
        let source = SourceBuffer::new(b"struct B{int m;};struct H{_Complex int z;};struct D:B{static constexpr H h=static_cast<int D::*>(&B::m)!=nullptr?H{{1/0,4}}:H{{0,0}};static_assert(false,\"stop before retry\");};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"failed-complex-read").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let class_name = cursor.intern_name(b"D").unwrap();
        let member_name = cursor.intern_name(b"h").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        assert!(analyzer.declarations(declarations, scope).is_err());
        let class = analyzer
            .graph
            .binding(
                analyzer
                    .graph
                    .own(scope, class_name, Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        let members = analyzer.graph.entity(class).scope.unwrap();
        let entity = analyzer
            .graph
            .binding(
                analyzer
                    .graph
                    .own(members, member_name, Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        assert!(
            analyzer
                .graph
                .aggregate_constants
                .deferred(entity)
                .is_some()
        );
        let integer = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        let complex = analyzer
            .graph
            .types
            .intern(TypeKind::Complex(integer), Cv::default())
            .unwrap();
        for (cv, expected_queries) in [(Cv::CONST, 2_usize), (Cv(3), 1)] {
            let ty = analyzer.graph.types.qualify(complex, cv).unwrap();
            let place = Address::new(Root::Variable(entity), ty);
            #[cfg(feature = "telemetry")]
            let before = analyzer.graph.work.totals.constant_evaluations;
            let result = analyzer.with_fresh_constant_query(|analyzer| {
                analyzer.bound_constant_complex_parts(place, complex, SourceOffset(0))
            });
            assert!(matches!(result, Err(Error::NonConstant { message, .. })
                if message == if cv.0 & 2 != 0 { "a read of a volatile glvalue" }
                    else { "a variable is not usable in a constant expression" }));
            #[cfg(feature = "telemetry")]
            assert_eq!(
                analyzer.graph.work.totals.constant_evaluations,
                before + expected_queries
            );
            #[cfg(not(feature = "telemetry"))]
            let _ = expected_queries;
            assert!(matches!(
                analyzer.graph.entity(entity).constant,
                ConstantState::None
            ));
        }
    }
}
