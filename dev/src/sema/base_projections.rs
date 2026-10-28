// Translated from lower/expression_lowering.cpp::to_base and virtual_base_lowering.cpp. See NOTICE.
//! Runtime projection requests preserve the selected path's canonical endpoints.
use super::{
    Error,
    graph::{EntityId, Graph},
    inheritance::BasePathId,
};

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
pub(crate) struct BaseRoute {
    pub via: Option<EntityId>,
    pub offset: u64,
}
/// Query-owner storage over completed layouts. Canonical class-pair keys are
/// immutable after layout publication; no emission or semantic body demand.
#[derive(Default)]
pub(crate) struct BaseRoutes {
    selected: HashMap<(EntityId, EntityId), BaseRoute>,
    holders: HashMap<EntityId, HashMap<EntityId, EntityId>>,
}
impl BaseRoutes {
    pub(crate) fn get(
        &mut self,
        graph: &Graph,
        from: EntityId,
        to: EntityId,
    ) -> Result<BaseRoute, Error> {
        // ProgramLowering keeps routes only for classes with virtual bases.
        // Ordinary projections perform their nonvirtual search on each demand.
        if graph.lower_virtual_bases(from).is_empty() {
            return graph
                .lower_nonvirtual_offset(from, to)?
                .map(|offset| BaseRoute { via: None, offset })
                .ok_or(Error::Invariant(
                    "nonvirtual projection without a base route",
                ));
        }
        if let Some(&route) = self.selected.get(&(from, to)) {
            return Ok(route);
        }
        let route = if let Some(offset) = graph.lower_nonvirtual_offset(from, to)? {
            BaseRoute { via: None, offset }
        } else {
            self.virtual_holders(graph, from);
            let via = self.holders[&from]
                .get(&to)
                .copied()
                .ok_or(Error::Invariant("virtual projection without a base route"))?;
            let offset = graph
                .lower_nonvirtual_offset(via, to)?
                .ok_or(Error::Invariant("base outside its virtual holder"))?;
            BaseRoute {
                via: Some(via),
                offset,
            }
        };
        self.selected.insert((from, to), route);
        Ok(route)
    }
    fn virtual_holders(&mut self, graph: &Graph, from: EntityId) {
        let std::collections::hash_map::Entry::Vacant(entry) = self.holders.entry(from) else {
            return;
        };
        let mut holders = HashMap::new();
        let mut work = Vec::new();
        for &(base, _) in graph.lower_virtual_bases(from) {
            work.push(base);
            while let Some(class) = work.pop() {
                if holders.contains_key(&class) {
                    continue;
                }
                holders.insert(class, base);
                work.extend(graph.lower_nonvirtual_bases(class));
            }
        }
        entry.insert(holders);
    }
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.selected.len()
    }
}

/// Checked canonical result identities; lowering selects their ABI route.
pub(crate) struct CovariantResult {
    pub from: EntityId,
    pub to: EntityId,
    pub through: Option<EntityId>,
    pub reference: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BaseProjection {
    Fixed(u64),
    Nonvirtual {
        from: EntityId,
        to: EntityId,
        offset: u64,
        reverse: bool,
    },
    Route {
        from: EntityId,
        to: EntityId,
    },
}
impl BaseProjection {
    pub(crate) fn known_offset(self) -> Option<u64> {
        match self {
            Self::Fixed(offset) => Some(offset),
            Self::Nonvirtual {
                offset, reverse, ..
            } => Some(if reverse {
                offset.wrapping_neg()
            } else {
                offset
            }),
            Self::Route { .. } => None,
        }
    }
    pub(crate) fn is_zero(self) -> bool {
        self.known_offset() == Some(0)
    }
    pub(super) fn negated(self) -> Result<Self, Error> {
        match self {
            Self::Fixed(offset) => Ok(Self::Fixed(offset.wrapping_neg())),
            Self::Nonvirtual {
                from,
                to,
                offset,
                reverse,
            } => Ok(Self::Nonvirtual {
                from,
                to,
                offset,
                reverse: !reverse,
            }),
            Self::Route { .. } => Err(Error::Invariant("downcast through a virtual base")),
        }
    }
}
impl From<u64> for BaseProjection {
    fn from(offset: u64) -> Self {
        Self::Fixed(offset)
    }
}
#[derive(Clone, Copy)]
pub(super) enum OffsetSearch {
    Nonvirtual,
    Constant,
    VirtualSlot,
}
/// A single target's failed branch roots, shared by provisional sibling searches.
pub(super) struct BaseOffsetQuery {
    base: EntityId,
    misses: HashSet<EntityId>,
}
impl BaseOffsetQuery {
    pub(super) fn new(base: EntityId) -> Self {
        Self {
            base,
            misses: HashSet::new(),
        }
    }
}
#[derive(Clone, Copy)]
enum LeafBranches {
    Skip,
    Visit,
}
struct OffsetFrame {
    root: EntityId,
    class: EntityId,
    total: u64,
    entry_offset: Option<u64>,
    next: usize,
    entered: bool,
}
impl OffsetFrame {
    fn new(class: EntityId, entry_offset: Option<u64>) -> Self {
        Self {
            root: class,
            class,
            total: 0,
            entry_offset,
            next: 0,
            entered: false,
        }
    }
}
enum OffsetStep {
    Continue,
    Miss,
    Found(u64),
    Descend(EntityId, Option<u64>),
}
fn found_offset(work: &[OffsetFrame], extra: u64) -> Result<u64, Error> {
    work.iter().try_fold(extra, |total, frame| {
        let entry = frame
            .entry_offset
            .ok_or(Error::Invariant("covariant result before layout"))?;
        Ok(total.wrapping_add(frame.total).wrapping_add(entry))
    })
}
impl Graph {
    pub(crate) fn lower_path_projection(&self, path: BasePathId) -> Result<BaseProjection, Error> {
        let edges = self.base_paths.get(path);
        if edges
            .iter()
            .any(|&(owner, index)| self.entity(owner).bases[index].is_virtual)
        {
            let &(from, _) = edges
                .first()
                .ok_or(Error::Invariant("empty virtual path"))?;
            let &(owner, index) = edges.last().ok_or(Error::Invariant("empty virtual path"))?;
            Ok(BaseProjection::Route {
                from,
                to: self.entity(owner).bases[index].class,
            })
        } else {
            let Some(&(from, _)) = edges.first() else {
                return Ok(BaseProjection::Fixed(0));
            };
            let &(owner, index) = edges.last().ok_or(Error::Invariant("nonempty base path"))?;
            Ok(BaseProjection::Nonvirtual {
                from,
                to: self.entity(owner).bases[index].class,
                offset: self.lower_path_offset(path)?,
                reverse: false,
            })
        }
    }
    /// Member access performs `ProgramLowering::base_route` even for a fixed,
    /// zero-offset base. Keep endpoints until that query is demanded.
    pub(super) fn lower_member_route_projection(
        &self,
        path: BasePathId,
    ) -> Result<BaseProjection, Error> {
        let edges = self.base_paths.get(path);
        let &(from, _) = edges
            .first()
            .ok_or(Error::Invariant("empty member base path"))?;
        let &(owner, index) = edges
            .last()
            .ok_or(Error::Invariant("empty member base path"))?;
        Ok(BaseProjection::Route {
            from,
            to: self.entity(owner).bases[index].class,
        })
    }
    /// Projection queries run after their storage-layout boundary; an absent
    /// sparse layout then denotes a class with no virtual bases.
    pub(crate) fn lower_virtual_bases(&self, class: EntityId) -> &[(EntityId, u64)] {
        self.virtuals
            .storage_layouts
            .get(&class)
            .map_or(&[], |layout| layout.bases.as_slice())
    }
    pub(crate) fn lower_itanium_primary(&self, class: EntityId) -> Option<(EntityId, bool)> {
        if let Some(facts) = self.virtuals.classes.get(&class) {
            return facts.primary.map(|primary| {
                (
                    primary,
                    !self
                        .entity(class)
                        .bases
                        .iter()
                        .any(|edge| !edge.is_virtual && edge.class == primary),
                )
            });
        }
        self.nonvirtual_primary(class)
            .map(|index| (self.entity(class).bases[index].class, false))
            .or_else(|| {
                self.virtuals
                    .storage_layouts
                    .get(&class)?
                    .primary
                    .map(|base| (base, true))
            })
    }
    pub(crate) fn lower_nonvirtual_bases(
        &self,
        class: EntityId,
    ) -> impl DoubleEndedIterator<Item = EntityId> + '_ {
        self.entity(class)
            .bases
            .iter()
            .filter(|edge| !edge.is_virtual)
            .map(|edge| edge.class)
    }
    pub(crate) fn lower_nonvirtual_base_offsets(
        &self,
        class: EntityId,
    ) -> impl DoubleEndedIterator<Item = (EntityId, Result<u64, Error>)> + '_ {
        self.entity(class)
            .bases
            .iter()
            .filter(|edge| !edge.is_virtual)
            .map(|edge| {
                (
                    edge.class,
                    edge.offset
                        .ok_or(Error::Invariant("base route before layout")),
                )
            })
    }
    pub(super) fn nonvirtual_result_offset(
        &self,
        from: EntityId,
        to: EntityId,
    ) -> Result<Option<u64>, Error> {
        // Analyzer::result_needs_adjustment uses NonVirtualBaseOffset, whose
        // shape query is outside the lowering/interpreter base-step census.
        self.base_search_offset(from, to, OffsetSearch::VirtualSlot)
    }
    pub(super) fn base_search_offset(
        &self,
        derived: EntityId,
        base: EntityId,
        search: OffsetSearch,
    ) -> Result<Option<u64>, Error> {
        match search {
            OffsetSearch::VirtualSlot => self.base_shape_offset(derived, base),
            OffsetSearch::Nonvirtual => self.counted_base_offset(
                derived,
                &mut BaseOffsetQuery::new(base),
                LeafBranches::Skip,
            ),
            OffsetSearch::Constant => {
                self.constant_base_offset(derived, &mut BaseOffsetQuery::new(base))
            }
        }
    }
    pub(super) fn constant_base_offset(
        &self,
        derived: EntityId,
        query: &mut BaseOffsetQuery,
    ) -> Result<Option<u64>, Error> {
        self.counted_base_offset(derived, query, LeafBranches::Visit)
    }
    fn counted_base_offset(
        &self,
        derived: EntityId,
        query: &mut BaseOffsetQuery,
        leaves: LeafBranches,
    ) -> Result<Option<u64>, Error> {
        let mut work = vec![OffsetFrame::new(derived, Some(0))];
        while let Some(frame) = work.last_mut() {
            match self.counted_offset_step(frame, query.base, leaves, &query.misses)? {
                OffsetStep::Continue => {}
                OffsetStep::Found(extra) => return Ok(Some(found_offset(&work, extra)?)),
                OffsetStep::Descend(class, offset) => work.push(OffsetFrame::new(class, offset)),
                OffsetStep::Miss => {
                    let root = frame.root;
                    work.pop();
                    // A failed recursive branch root is remembered. Classes
                    // followed within its single-base chain are not memoized.
                    if !work.is_empty() {
                        query.misses.insert(root);
                    }
                }
            }
        }
        Ok(None)
    }
    fn counted_offset_step(
        &self,
        frame: &mut OffsetFrame,
        base: EntityId,
        leaves: LeafBranches,
        misses: &HashSet<EntityId>,
    ) -> Result<OffsetStep, Error> {
        if frame.class == base {
            return Ok(OffsetStep::Found(0));
        }
        let edges = &self.entity(frame.class).bases;
        if !frame.entered {
            self.work.base_steps(1);
            frame.entered = true;
            match edges.as_slice() {
                [] => return Ok(OffsetStep::Miss),
                [edge] => {
                    if edge.is_virtual {
                        return Ok(OffsetStep::Miss);
                    }
                    let offset = edge
                        .offset
                        .ok_or(Error::Invariant("base search before layout"))?;
                    frame.total = frame.total.wrapping_add(offset);
                    frame.class = edge.class;
                    frame.entered = false;
                    return Ok(OffsetStep::Continue);
                }
                _ => {
                    if let Some(edge) = edges
                        .iter()
                        .find(|edge| !edge.is_virtual && edge.class == base)
                    {
                        return Ok(OffsetStep::Found(
                            edge.offset
                                .ok_or(Error::Invariant("base search before layout"))?,
                        ));
                    }
                }
            }
        }
        let Some(edge) = edges.get(frame.next) else {
            return Ok(OffsetStep::Miss);
        };
        frame.next += 1;
        if edge.is_virtual
            || misses.contains(&edge.class)
            || matches!(leaves, LeafBranches::Skip) && self.entity(edge.class).bases.is_empty()
        {
            return Ok(OffsetStep::Continue);
        }
        Ok(OffsetStep::Descend(edge.class, edge.offset))
    }
    fn base_shape_offset(&self, derived: EntityId, base: EntityId) -> Result<Option<u64>, Error> {
        // Relative offsets are memoized by class, so repeated subobjects do
        // not repeat a search. The first nonvirtual path in declaration order
        // is the route chosen by the C++ ABI query, after semantic covariance.
        let mut known = HashMap::new();
        known.insert(base, Some(0_u64));
        let mut work = vec![(derived, 0_usize)];
        while let Some((class, next)) = work.last_mut() {
            if known.contains_key(class) {
                work.pop();
                continue;
            }
            let edges = &self.entity(*class).bases;
            // Both offset owners prefer a direct nonvirtual base before
            // searching the other bases of a branching class.
            if *next == 0
                && edges.len() > 1
                && let Some(edge) = edges
                    .iter()
                    .find(|edge| !edge.is_virtual && edge.class == base)
            {
                known.insert(
                    *class,
                    Some(
                        edge.offset
                            .ok_or(Error::Invariant("base search before layout"))?,
                    ),
                );
                work.pop();
                continue;
            }
            let Some(edge) = edges.get(*next) else {
                known.insert(*class, None);
                work.pop();
                continue;
            };
            if edge.is_virtual {
                *next += 1;
                continue;
            }
            if let Some(&below) = known.get(&edge.class) {
                if let Some(below) = below {
                    let offset = edge
                        .offset
                        .ok_or(Error::Invariant("covariant result before layout"))?;
                    known.insert(*class, Some(offset.wrapping_add(below)));
                    work.pop();
                } else {
                    *next += 1;
                }
            } else if edges.len() > 1
                && edge.class != base
                && self.entity(edge.class).bases.is_empty()
            {
                *next += 1;
            } else {
                work.push((edge.class, 0));
            }
        }
        Ok(known[&derived])
    }
    pub(crate) fn lower_nonvirtual_offset(
        &self,
        from: EntityId,
        to: EntityId,
    ) -> Result<Option<u64>, Error> {
        self.base_search_offset(from, to, OffsetSearch::Nonvirtual)
    }
    pub(crate) fn lower_vcall_functions(&self, class: EntityId) -> Result<Vec<EntityId>, Error> {
        if !self.lower_polymorphic(class) {
            return Ok(Vec::new());
        }
        let facts = self.lower_virtual_class(class)?;
        let mut entries: Vec<_> = facts
            .entries
            .iter()
            .filter(|entry| {
                !matches!(
                    entry.kind,
                    super::virtual_members::SlotKind::DeletingDestructor
                )
            })
            .collect();
        entries.sort_unstable_by_key(|entry| {
            self.virtuals
                .vcall_orders
                .get(&entry.function)
                .copied()
                .unwrap_or(entry.slot.byte_offset())
        });
        Ok(entries.into_iter().map(|entry| entry.function).collect())
    }
    pub(crate) fn lower_vcall_signature(
        &self,
        function: EntityId,
    ) -> Result<VcallSignature, Error> {
        self.virtual_signature(function).map(VcallSignature)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct VcallSignature(super::final_overriders::VirtualSignature);
