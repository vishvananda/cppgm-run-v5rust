// Translated from analyzer_expressions.cpp::odr_use and semantics_dump.cpp::trailing. See NOTICE.
//! Function-owned demand edges, separate from declaration and body analysis order.
use super::{
    Error,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DemandEpoch(u32);
impl DemandEpoch {
    const ROOT: Self = Self(1);
}
#[derive(Clone, Copy)]
struct DemandCursor {
    epoch: DemandEpoch,
    owner: Option<EntityId>,
    index: usize,
}
pub(super) struct DemandMarks {
    next_epoch: u32,
    latest: std::collections::HashMap<DemandKey, DemandCursor>,
}
impl Default for DemandMarks {
    fn default() -> Self {
        Self {
            next_epoch: DemandEpoch::ROOT.0,
            latest: std::collections::HashMap::new(),
        }
    }
}
impl DemandMarks {
    pub(super) fn fresh_epoch(&mut self) -> DemandEpoch {
        // C++'s unsigned epoch counter wraps deliberately at its target width.
        self.next_epoch = self.next_epoch.wrapping_add(1);
        DemandEpoch(self.next_epoch)
    }
}

#[derive(Default)]
pub(super) struct DemandList {
    records: Vec<Demand>,
    pub(super) names_enclosing_statics: bool,
}
impl DemandList {
    fn merge_call(&mut self, index: usize, demand: Demand) {
        if let Demand::Function { call: old_call, .. } = &mut self.records[index] {
            *old_call |= match demand {
                Demand::Constructor(_) => true,
                Demand::Function { call, .. } => call,
            };
        }
    }
    fn append(&mut self, demand: Demand) -> usize {
        let index = self.records.len();
        self.records.push(demand);
        index
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
    pub(super) fn demand_function(&mut self, binding: BindingId, call: bool) -> Result<(), Error> {
        if self.deferring_type_query() {
            self.type_queries
                .uses
                .push(super::type_queries::DeferredUse::Function(binding, call));
        }
        if self.unevaluated != 0 || self.template_depth != 0 {
            return Ok(());
        }
        // odr_use requires a pending specification before recording this edge.
        // Its own calls therefore retain their earlier semantic demand order.
        let entity = self.graph.binding(binding).entity;
        if matches!(
            self.graph.entity(entity).exceptions,
            super::exceptions::ExceptionState::Pending(_)
        ) || self.graph.function_templates.primary(entity).is_some()
            && matches!(
                self.graph.entity(entity).exceptions,
                super::exceptions::ExceptionState::Template(_)
            )
        {
            self.function_may_throw(entity)?;
        }
        self.record_demand(Demand::Function { binding, call });
        Ok(())
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
        let active = self
            .body_context
            .as_ref()
            .and_then(|body| body.demand_epoch.map(|epoch| (body.entity, epoch)));
        let (owner, epoch) = active.map_or((None, DemandEpoch::ROOT), |(owner, epoch)| {
            (Some(owner), epoch)
        });
        let key = match demand {
            Demand::Constructor(id) => self
                .graph
                .constructors
                .get(id)
                .binding
                .map_or(DemandKey::Constructor(id), |binding| {
                    DemandKey::Function(self.graph.binding(binding).entity)
                }),
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
        if let Some(previous) = self.graph.demand_marks.latest.get(&key).copied()
            && previous.epoch == epoch
        {
            let list = if let Some(owner) = previous.owner {
                &mut self.graph.entity_mut(owner).demands
            } else {
                &mut self.graph.root_demands
            };
            list.merge_call(previous.index, demand);
            return;
        }
        let index = if let Some(owner) = owner {
            self.graph.entity_mut(owner).demands.append(demand)
        } else {
            self.graph.root_demands.append(demand)
        };
        self.graph.demand_marks.latest.insert(
            key,
            DemandCursor {
                epoch,
                owner,
                index,
            },
        );
        self.graph.work.totals.demands += 1;
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
                            && entity.copy_origin != super::copy_construction::Origin::Implicit
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
    fn nested_inline_use_replaces_the_latest_root_demand_mark() {
        let source = SourceBuffer::new(
            b"void target(); void root(){ auto a=&target; target(); auto nested=[]{auto b=&target; target();}; auto c=&target; target(); }".to_vec(),
        ).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"demand-epochs").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"target").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let binding = graph.own(graph.global, name, Filter::Ordinary).unwrap();
        let target = graph.binding(binding).entity;
        let uses: Vec<_> = graph
            .root_demands
            .as_slice()
            .iter()
            .filter_map(|demand| match demand {
                super::Demand::Function { binding, call }
                    if graph.binding(*binding).entity == target =>
                {
                    Some(*call)
                }
                _ => None,
            })
            .collect();
        assert_eq!(uses, [true, true]);
    }

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
