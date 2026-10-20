// Translated from lower/expression_lowering.cpp::to_base and virtual_base_lowering.cpp. See NOTICE.
//! Runtime projection requests preserve the selected path's canonical endpoints.
use super::{
    Error,
    graph::{EntityId, Graph},
    inheritance::BasePathId,
};

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
