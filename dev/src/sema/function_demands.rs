// Translated from analyzer_expressions.cpp::odr_use and semantics_dump.cpp::trailing. See NOTICE.
//! Function-owned demand edges, separate from declaration and body analysis order.
use super::{
    analyzer::Analyzer,
    construction::ConstructorId,
    graph::{BindingId, EntityId, ScopeKind},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum DefinitionEmission {
    #[default]
    Always,
    Demanded,
}

#[derive(Clone, Copy)]
pub(super) enum Demand {
    Constructor(ConstructorId),
    Function { binding: BindingId, call: bool },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum DemandKey {
    Constructor(ConstructorId),
    Function(EntityId),
}

#[derive(Default)]
pub(super) struct DemandList {
    records: Vec<Demand>,
    index: std::collections::HashMap<DemandKey, usize>,
}
impl DemandList {
    fn record(&mut self, key: DemandKey, demand: Demand) {
        if let Some(&index) = self.index.get(&key) {
            if let (Demand::Function { call: old_call, .. }, Demand::Function { call, .. }) =
                (&mut self.records[index], demand)
            {
                *old_call |= call;
            }
        } else {
            self.index.insert(key, self.records.len());
            self.records.push(demand);
        }
    }
    fn as_slice(&self) -> &[Demand] {
        &self.records
    }
}

impl Analyzer<'_> {
    pub(super) fn demand_function(&mut self, binding: BindingId, call: bool) {
        if self.unevaluated != 0 || self.template_depth != 0 {
            return;
        }
        self.record_demand(Demand::Function { binding, call });
    }

    pub(super) fn record_demand(&mut self, demand: Demand) {
        // Ordinary definitions contribute roots. Inline/constexpr functions retain
        // their dependencies until that member itself is demanded for output.
        let owner = self
            .body_context
            .as_ref()
            .map(|body| body.entity)
            .filter(|&entity| {
                self.graph.entity(entity).definition_emission == DefinitionEmission::Demanded
            });
        let key = match demand {
            Demand::Constructor(id) => DemandKey::Constructor(id),
            Demand::Function { binding, .. } => {
                DemandKey::Function(self.graph.binding(binding).entity)
            }
        };
        if let Some(owner) = owner {
            self.graph.entity_mut(owner).demands.record(key, demand);
        } else {
            self.graph.root_demands.record(key, demand);
        }
    }
}

/// Deduplicate output demand by entity, retaining first-use order and the
/// oracle's separate implicit/member queues. This is a presentation query;
/// it never analyzes a body or looks up a name.
pub(super) fn trailing(graph: &super::graph::Graph) -> Vec<Demand> {
    use std::collections::{HashSet, VecDeque};
    let mut constructors = VecDeque::new();
    let mut members = VecDeque::new();
    let mut seen_constructors = HashSet::new();
    let mut seen_members: HashSet<EntityId> = HashSet::new();
    let mut output = Vec::new();
    let mut schedule =
        |demands: &[Demand], constructors: &mut VecDeque<_>, members: &mut VecDeque<_>| {
            for &demand in demands {
                match demand {
                    Demand::Constructor(id) => {
                        if seen_constructors.insert(id) {
                            constructors.push_back(demand);
                        }
                    }
                    Demand::Function { binding, call } => {
                        let entity = graph.entity(graph.binding(binding).entity);
                        if (entity.definition_emission == DefinitionEmission::Demanded
                            || call
                                && !entity.defined
                                && entity
                                    .home
                                    .is_some_and(|home| graph.scope(home).kind == ScopeKind::Class))
                            && seen_members.insert(graph.binding(binding).entity)
                        {
                            members.push_back(demand);
                        }
                    }
                }
            }
        };
    schedule(
        graph.root_demands.as_slice(),
        &mut constructors,
        &mut members,
    );
    while !constructors.is_empty() || !members.is_empty() {
        while let Some(demand) = constructors.pop_front() {
            output.push(demand);
        }
        while let Some(demand) = members.pop_front() {
            output.push(demand);
            if let Demand::Function { binding, .. } = demand {
                schedule(
                    graph
                        .entity(graph.binding(binding).entity)
                        .demands
                        .as_slice(),
                    &mut constructors,
                    &mut members,
                );
            }
        }
    }
    output
}
