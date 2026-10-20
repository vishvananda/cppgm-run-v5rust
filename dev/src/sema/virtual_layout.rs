// Translated from sema/primary_bases.cpp and layout.cpp. See NOTICE.
//! Shared virtual-base identities and query-local primary claims. Published only after layout.
use super::{
    Error,
    graph::{EntityId, Graph},
    layout::LayoutState,
};
use std::collections::{HashMap, HashSet};

pub(super) struct VirtualLayout {
    pub bases: Vec<(EntityId, u64)>,
    positions: HashMap<EntityId, usize>,
    pub primary: Option<EntityId>,
    indirect: HashSet<EntityId>,
}

impl Graph {
    /// The base order is fixed when the plan is published. Placement changes
    /// only offsets, so this class-owned identity index needs no invalidation.
    pub(crate) fn lower_virtual_base_offset(&self, class: EntityId, base: EntityId) -> Option<u64> {
        let layout = self.virtuals.storage_layouts.get(&class)?;
        Some(layout.bases[*layout.positions.get(&base)?].1)
    }
    /// Preorder of inheritance edges, expanding each class once. A virtual
    /// occurrence is recorded even if its class was first reached nonvirtually.
    pub(super) fn virtual_base_order(&self, root: EntityId) -> Vec<EntityId> {
        if let Some(facts) = self.virtuals.classes.get(&root) {
            return facts.bases.clone();
        }
        let mut expanded = HashSet::new();
        let mut shared = HashSet::new();
        let mut result = Vec::new();
        let mut work = vec![(root, false)];
        while let Some((class, virtual_edge)) = work.pop() {
            if virtual_edge && shared.insert(class) {
                result.push(class);
            }
            if !expanded.insert(class) {
                continue;
            }
            work.extend(
                self.entity(class)
                    .bases
                    .iter()
                    .rev()
                    .map(|edge| (edge.class, edge.is_virtual)),
            );
        }
        result
    }
    /// Shared bases not named by a direct virtual edge. Available while the
    /// class completes, before its own virtual/layout facts are published.
    pub(super) fn indirect_virtual_bases(&self, class: EntityId) -> Vec<EntityId> {
        let bases = &self.entity(class).bases;
        if !bases.iter().any(|edge| {
            edge.is_virtual
                || self
                    .virtuals
                    .classes
                    .get(&edge.class)
                    .is_some_and(|facts| facts.has_virtual_bases)
        }) {
            return Vec::new();
        }
        let direct: HashSet<_> = bases
            .iter()
            .filter(|edge| edge.is_virtual)
            .map(|edge| edge.class)
            .collect();
        self.virtual_base_order(class)
            .into_iter()
            .filter(|base| !direct.contains(base))
            .collect()
    }
    pub(super) fn nonvirtual_primary(&self, class: EntityId) -> Option<usize> {
        self.entity(class)
            .bases
            .iter()
            .position(|edge| !edge.is_virtual && self.layout_dynamic(edge.class))
    }
    pub(super) fn layout_dynamic(&self, class: EntityId) -> bool {
        self.lower_dynamic(class) || self.virtuals.storage_layouts.contains_key(&class)
    }
    pub(super) fn virtual_layout_plan(&self, class: EntityId) -> Result<VirtualLayout, Error> {
        let mut bases = Vec::new();
        let mut shared = HashSet::new();
        let mut indirect = HashSet::new();
        for edge in &self.entity(class).bases {
            if edge.is_virtual && shared.insert(edge.class) {
                bases.push(edge.class);
            }
            if let Some(child) = self.virtuals.storage_layouts.get(&edge.class) {
                for &(base, _) in &child.bases {
                    if shared.insert(base) {
                        bases.push(base);
                    }
                }
                indirect.extend(child.indirect.iter().copied());
                indirect.extend(child.primary);
            }
        }
        let mut first = None;
        let mut primary = None;
        if self.nonvirtual_primary(class).is_none() {
            for &base in &bases {
                let LayoutState::Value {
                    non_virtual_size, ..
                } = self.entity(base).layout
                else {
                    return Err(Error::Invariant("virtual primary before base layout"));
                };
                if !self.layout_dynamic(base) || non_virtual_size != 8 {
                    continue;
                }
                if !indirect.contains(&base) {
                    primary = Some(base);
                    break;
                }
                first.get_or_insert(base);
            }
            primary = primary.or(first);
        }
        Ok(VirtualLayout {
            positions: bases
                .iter()
                .enumerate()
                .map(|(index, &base)| (base, index))
                .collect(),
            bases: bases.into_iter().map(|base| (base, 0)).collect(),
            primary,
            indirect,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Resolver {
    Direct(usize),
    Virtual(usize),
}
#[derive(Clone, Copy)]
struct ClaimNodeId(u32);
struct ClaimNode {
    class: EntityId,
    owner: EntityId,
    edge: usize,
    parent: Option<ClaimNodeId>,
}
struct ClaimFrame {
    node: ClaimNodeId,
    next: usize,
    after: Option<EntityId>,
}
struct ClaimEntry {
    base: usize,
    relative: u64,
    next: Option<usize>,
}
/// Only the resolver index survives claim planning; occurrence nodes and
/// traversal scratch die before ordinary member placement begins.
#[derive(Default)]
pub(super) struct Claims {
    by_resolver: HashMap<Resolver, usize>,
    entries: Vec<ClaimEntry>,
    pub indirect: HashSet<EntityId>,
}
impl Claims {
    pub(super) fn plan(
        graph: &Graph,
        root: EntityId,
        layout: &VirtualLayout,
        placement: &mut super::empty_layout::EmptyPlacement,
        location: crate::support::source::SourceOffset,
    ) -> Result<Self, Error> {
        let mut result = Self::default();
        result.indirect.clone_from(&layout.indirect);
        if result.indirect.is_empty() {
            return Ok(result);
        }
        let mut nodes = Vec::<ClaimNode>::new();
        let mut frames = Vec::<ClaimFrame>::new();
        let mut virtual_nodes = HashSet::new();
        let mut claimants = HashMap::<EntityId, ClaimNodeId>::new();
        for direct in 0..graph.entity(root).bases.len() {
            Self::enter(
                graph,
                root,
                direct,
                None,
                &mut nodes,
                &mut frames,
                &mut virtual_nodes,
                &mut claimants,
            )?;
            while let Some(frame) = frames.last_mut() {
                let class = nodes[frame.node.0 as usize].class;
                if frame.next < graph.entity(class).bases.len() {
                    let index = frame.next;
                    frame.next += 1;
                    let parent = frame.node;
                    placement.spend(location)?;
                    Self::enter(
                        graph,
                        class,
                        index,
                        Some(parent),
                        &mut nodes,
                        &mut frames,
                        &mut virtual_nodes,
                        &mut claimants,
                    )?;
                } else {
                    if let Some(primary) = frame.after {
                        claimants.insert(primary, frame.node);
                    }
                    frames.pop();
                }
            }
        }
        if let Some(primary) = layout.primary {
            claimants.remove(&primary);
        }
        let virtual_indices: HashMap<_, _> = layout
            .bases
            .iter()
            .enumerate()
            .map(|(index, &(class, _))| (class, index))
            .collect();
        for (index, &(class, _)) in layout.bases.iter().enumerate() {
            if Some(class) == layout.primary || !result.indirect.contains(&class) {
                continue;
            }
            let Some(&claimant) = claimants.get(&class) else {
                continue;
            };
            let mut node = claimant;
            let mut relative = 0u64;
            let mut resolver = None;
            for _ in 0..=nodes.len() {
                let item = &nodes[node.0 as usize];
                let edge = graph.entity(item.owner).bases[item.edge];
                if !edge.is_virtual {
                    // The root's direct edge has not been placed yet. Its
                    // relative offset is zero; internal edges are completed facts.
                    relative = relative.wrapping_add(if item.parent.is_none() {
                        0
                    } else {
                        edge.offset
                            .ok_or(Error::Invariant("claim before internal base layout"))?
                    });
                    if let Some(parent) = item.parent {
                        node = parent;
                    } else {
                        resolver = Some(Resolver::Direct(item.edge));
                        break;
                    }
                } else if Some(item.class) == layout.primary
                    || !result.indirect.contains(&item.class)
                    || !claimants.contains_key(&item.class)
                {
                    resolver = Some(Resolver::Virtual(
                        *virtual_indices
                            .get(&item.class)
                            .ok_or(Error::Invariant("claim without virtual identity"))?,
                    ));
                    break;
                } else {
                    node = claimants[&item.class];
                }
            }
            if let Some(resolver) = resolver {
                let next = result.by_resolver.insert(resolver, result.entries.len());
                result.entries.push(ClaimEntry {
                    base: index,
                    relative,
                    next,
                });
            }
        }
        Ok(result)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Query-local flat claim stores share one iterative traversal"
    )]
    fn enter(
        graph: &Graph,
        owner: EntityId,
        edge_index: usize,
        parent: Option<ClaimNodeId>,
        nodes: &mut Vec<ClaimNode>,
        frames: &mut Vec<ClaimFrame>,
        virtual_nodes: &mut HashSet<EntityId>,
        claimants: &mut HashMap<EntityId, ClaimNodeId>,
    ) -> Result<(), Error> {
        let edge = graph.entity(owner).bases[edge_index];
        if edge.is_virtual && !virtual_nodes.insert(edge.class) {
            return Ok(());
        }
        let id = ClaimNodeId(
            u32::try_from(nodes.len()).map_err(|_| Error::Limit("layout claim node limit"))?,
        );
        nodes.push(ClaimNode {
            class: edge.class,
            owner,
            edge: edge_index,
            parent,
        });
        let mut after = None;
        if let Some(layout) = graph.virtuals.storage_layouts.get(&edge.class)
            && let Some(primary) = layout.primary
        {
            if virtual_nodes.contains(&primary) {
                claimants.entry(primary).or_insert(id);
            } else {
                after = Some(primary);
            }
        }
        frames.push(ClaimFrame {
            node: id,
            next: 0,
            after,
        });
        Ok(())
    }
    fn resolved_entries(&self, resolver: Resolver) -> impl Iterator<Item = &ClaimEntry> {
        std::iter::successors(self.by_resolver.get(&resolver).copied(), |&index| {
            self.entries[index].next
        })
        .map(|index| &self.entries[index])
    }
    pub(super) fn conflict(
        &self,
        graph: &Graph,
        layout: &VirtualLayout,
        resolver: Resolver,
        offset: u64,
        placement: &mut super::empty_layout::EmptyPlacement,
        location: crate::support::source::SourceOffset,
    ) -> Result<bool, Error> {
        for entry in self.resolved_entries(resolver) {
            if placement.conflict(
                graph,
                layout.bases[entry.base].0,
                offset.wrapping_add(entry.relative),
                location,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Claimed bases use the resolver's placement and recording window"
    )]
    pub(super) fn record(
        &self,
        graph: &Graph,
        layout: &mut VirtualLayout,
        resolver: Resolver,
        offset: u64,
        placement: &mut super::empty_layout::EmptyPlacement,
        tail: u64,
        reachable: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        for entry in self.resolved_entries(resolver) {
            let (class, at) = &mut layout.bases[entry.base];
            *at = offset.wrapping_add(entry.relative);
            placement.record_base(graph, *class, *at, tail, !reachable, location)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, lookup::Filter},
        support::{
            source::{SourceBuffer, SourceOffset},
            stack::StackBudget,
        },
        syntax::SyntaxUnit,
    };

    #[test]
    fn layout_retains_complete_and_nonvirtual_facts_without_emission_demand() {
        let source = SourceBuffer::new(
            br"
            struct E {};
            struct N : E { virtual void f(); };
            struct V : virtual N { int v; };
            struct A : virtual N {};
            struct B : virtual N {};
            struct D : A, B { char c; };
            struct K : virtual N {};
            struct X : N, E, K {};
            struct Tail { Tail(); int n; char c; };
            struct Mark { char c; };
            struct Holder : Tail, virtual Mark {};
            struct Aligned { alignas(32) char c; };
            struct Small : virtual Aligned { char c; };
            struct Outer : Small { char c; };
            struct Object { Small member; E es[2]; };
            struct NonEmpty { virtual void f(); int n; };
            struct Dormant : virtual NonEmpty { void unused(); };
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"virtual-layout").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [
            b"V".as_slice(),
            b"D",
            b"X",
            b"Holder",
            b"Small",
            b"Outer",
            b"Object",
            b"Dormant",
        ]
        .into_iter()
        .map(|name| cursor.intern_name(name).unwrap())
        .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let syntax_mark = analyzer.tree.expressions.mark();
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer
                    .graph
                    .own(analyzer.graph.global, name, Filter::Prefix)
                    .unwrap();
                analyzer.graph.binding(binding).entity
            })
            .collect();
        for (&class, (size, nvsize, nvalign)) in classes.iter().zip([
            (16, 12, 8),
            (24, 17, 8),
            (24, 24, 8),
            (16, 13, 8),
            (64, 9, 8),
            (64, 10, 8),
            (96, 66, 32),
        ]) {
            assert_eq!(
                analyzer.class_layout(class, SourceOffset(0)).unwrap().0,
                size
            );
            let LayoutState::Value {
                non_virtual_size,
                non_virtual_alignment,
                ..
            } = analyzer.graph.entity(class).layout
            else {
                panic!("layout");
            };
            assert_eq!((non_virtual_size, non_virtual_alignment), (nvsize, nvalign));
        }
        let x = classes[2];
        assert_eq!(analyzer.graph.entity(x).bases[2].offset, Some(16));
        let layout = &analyzer.graph.virtuals.storage_layouts[&x];
        assert_eq!(layout.bases[0].1, 16);
        let holder = &analyzer.graph.virtuals.storage_layouts[&classes[3]];
        assert_eq!(holder.bases[0].1, 13);
        assert!(matches!(
            analyzer.graph.entity(classes[7]).layout,
            LayoutState::NotStarted
        ));
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(analyzer.tree.expressions.mark(), syntax_mark);
        // Ready layout queries retain the same store; no duplicate shared bases.
        analyzer.class_layout(x, SourceOffset(0)).unwrap();
        assert_eq!(analyzer.graph.virtuals.storage_layouts[&x].bases.len(), 1);
    }
}
