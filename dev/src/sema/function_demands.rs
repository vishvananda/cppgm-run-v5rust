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
    pub(super) fn as_slice(&self) -> &[Demand] {
        &self.records
    }
}

impl Analyzer<'_> {
    pub(super) fn demand_instance_member(&mut self, entity: EntityId) {
        if self.graph.function_templates.bodies.member(entity) {
            self.graph.function_templates.bodies.demand(entity);
        }
    }
    pub(super) fn demand_function(&mut self, binding: BindingId, call: bool) {
        if self.unevaluated != 0 || self.template_depth != 0 {
            return;
        }
        self.record_demand(Demand::Function { binding, call });
    }

    pub(super) fn record_demand(&mut self, demand: Demand) {
        let function = match demand {
            Demand::Function { binding, .. } => Some(binding),
            Demand::Constructor(id) => self.graph.constructors.get(id).binding,
        };
        if let Some(binding) = function {
            let entity = self.graph.binding(binding).entity;
            if self.graph.function_templates.bodies.member(entity) {
                self.graph.function_templates.bodies.semantic_demand(entity);
            }
        }
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
                let function = self.graph.binding(binding).entity;
                let entity = self.graph.entity(function);
                // Like C++ demand_mark, this records a potentially evaluated
                // use even inside an inline body that is never emitted. Only
                // implicit assignment declarations affect constructor order.
                if entity.assignment_origin == super::assignment::Origin::Implicit
                    && let Some(class) = entity.home.and_then(|home| self.graph.scope(home).owner)
                {
                    self.graph.entity_mut(class).implicit_assignment_use =
                        super::assignment::ImplicitUse::Used;
                }
                DemandKey::Function(function)
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
                        let function = graph.binding(binding).entity;
                        let entity = graph.entity(function);
                        // C++ marks inheriting constructors implicit. Only a
                        // parameterless implicit definition appears in this view.
                        if graph.inherited_constructors.source(function).is_some() {
                            if entity.ty.is_some_and(|ty| {
                                matches!(&graph.types.get(ty).kind,
                                    super::types::TypeKind::Function { parameters, .. }
                                        if parameters.is_empty())
                            }) && let Some(id) = entity.constructor
                                && seen_constructors.insert(id)
                            {
                                constructors.push_back(Demand::Constructor(id));
                            }
                            continue;
                        }
                        if entity.assignment_origin != super::assignment::Origin::Implicit
                            && !graph.function_templates.bodies.synthesized(function)
                            && (entity.definition_emission == DefinitionEmission::Demanded
                                || graph.function_templates.primary(function).is_some()
                                || call
                                    && !entity.defined
                                    && entity.home.is_some_and(|home| {
                                        graph.scope(home).kind == ScopeKind::Class
                                    }))
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

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{analyzer::Analyzer, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn implicit_assignment_use_is_class_owned_and_independent_of_emission() {
        let source = SourceBuffer::new(
            b"struct Used{unsigned n:2;};struct Unused{unsigned n:2;};\
              struct Unevaluated{unsigned n:2;};\
              struct Defaulted{unsigned n:2;Defaulted&operator=(const Defaulted&)=default;};\
              struct Hidden{void hidden(Used&a,Used&b){a=b;a=b;}};\
              void f(Unevaluated&a,Unevaluated&b,Defaulted&c,Defaulted&d){\
                (void)sizeof(a=b);c=d;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"assignment-use").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"Used".as_slice(), b"Unused", b"Unevaluated", b"Defaulted"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let expression_count = tree.expressions.mark();
        let declaration_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        analyzer.finish_lowering_boundaries().unwrap();
        for (index, name) in names.into_iter().enumerate() {
            let binding = analyzer
                .graph
                .own(global, name, Filter::Elaborated)
                .unwrap();
            let class = analyzer.graph.binding(binding).entity;
            assert_eq!(
                analyzer.graph.lower_implicit_assignment_demanded(class),
                index == 0
            );
        }
        // The unused inline body supplied the demand fact, but is not an
        // emission root and must not force its assignment body into output.
        assert!(!analyzer.graph.root_demands.as_slice().iter().any(|demand| {
            matches!(demand, super::Demand::Function { binding, .. }
                if analyzer.graph.lower_implicit_assignment(analyzer.graph.binding(*binding).entity))
        }));
        assert_eq!(tree.expressions.mark(), expression_count);
        assert_eq!(tree.declarations.mark(), declaration_count);
    }
}
