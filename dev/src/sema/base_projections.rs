// Translated from lower/expression_lowering.cpp::to_base and virtual_base_lowering.cpp. See NOTICE.
//! Runtime projection requests preserve the selected path's canonical endpoints.
use super::{
    Error,
    graph::{EntityId, Graph},
    inheritance::BasePathId,
};

use std::collections::HashMap;

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
    holders: HashMap<EntityId, HashMap<EntityId, BaseRoute>>,
}
impl BaseRoutes {
    pub(crate) fn get(
        &mut self,
        graph: &Graph,
        from: EntityId,
        to: EntityId,
    ) -> Result<BaseRoute, Error> {
        if let Some(&route) = self.selected.get(&(from, to)) {
            return Ok(route);
        }
        let route = if let Some(offset) = graph.lower_nonvirtual_offset(from, to)? {
            BaseRoute { via: None, offset }
        } else {
            self.virtual_holders(graph, from)?;
            self.holders[&from]
                .get(&to)
                .copied()
                .ok_or(Error::Invariant("virtual projection without a base route"))?
        };
        self.selected.insert((from, to), route);
        Ok(route)
    }
    fn virtual_holders(&mut self, graph: &Graph, from: EntityId) -> Result<(), Error> {
        let std::collections::hash_map::Entry::Vacant(entry) = self.holders.entry(from) else {
            return Ok(());
        };
        let mut holders = HashMap::new();
        let mut work = Vec::new();
        for &(base, _) in graph.lower_virtual_bases(from) {
            work.push((base, 0_u64));
            while let Some((class, offset)) = work.pop() {
                if holders.contains_key(&class) {
                    continue;
                }
                holders.insert(
                    class,
                    BaseRoute {
                        via: Some(base),
                        offset,
                    },
                );
                for (child, step) in graph.lower_nonvirtual_base_offsets(class).rev() {
                    work.push((child, offset.wrapping_add(step?)));
                }
            }
        }
        entry.insert(holders);
        Ok(())
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
    Route { from: EntityId, to: EntityId },
}
impl BaseProjection {
    pub(crate) fn is_zero(self) -> bool {
        matches!(self, Self::Fixed(0))
    }
    pub(super) fn negated(self) -> Result<Self, Error> {
        match self {
            Self::Fixed(offset) => Ok(Self::Fixed(offset.wrapping_neg())),
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
}
impl Graph {
    pub(super) fn lower_path_projection(&self, path: BasePathId) -> Result<BaseProjection, Error> {
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
            self.lower_path_offset(path).map(BaseProjection::Fixed)
        }
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
        self.base_search_offset(from, to, OffsetSearch::Nonvirtual)
    }
    pub(super) fn base_search_offset(
        &self,
        derived: EntityId,
        base: EntityId,
        search: OffsetSearch,
    ) -> Result<Option<u64>, Error> {
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
            // ConstantInterpreter::base_search recognizes a direct nonvirtual
            // base of a branching class before searching its other bases.
            if matches!(search, OffsetSearch::Constant)
                && *next == 0
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
        self.nonvirtual_result_offset(from, to)
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
