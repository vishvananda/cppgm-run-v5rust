// Translated from sema/layout.cpp's base placement and primary_bases.cpp. See NOTICE.
//! Query-owned placement of nonvirtual bases and the complete object's shared virtual bases.
use super::{
    Error,
    analyzer::Analyzer,
    empty_layout::EmptyPlacement,
    graph::EntityId,
    layout::{LayoutState, round_up},
    virtual_layout::{Claims, Resolver, VirtualLayout},
};
use crate::support::source::SourceOffset;

#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent ABI emptiness, volatility and null-member-pointer containment facts"
)]
pub(super) struct BasePlacement {
    pub size: u64,
    pub data_size: u64,
    pub alignment: u64,
    pub empty: bool,
    pub contains_empty: bool,
    pub contains_volatile_or_union: bool,
    pub contains_data_member_pointer: bool,
    pub dynamic: bool,
    pub placement: EmptyPlacement,
    pub layout: VirtualLayout,
    claims: Claims,
}
impl Analyzer<'_> {
    pub(super) fn place_class_bases(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<BasePlacement, Error> {
        let layout = self.graph.virtual_layout_plan(class)?;
        let dynamic = self.graph.lower_polymorphic(class) || !layout.bases.is_empty();
        let primary = self.graph.nonvirtual_primary(class);
        let mut placement = EmptyPlacement::new(&self.graph, class, &layout.bases)?;
        let claims = Claims::plan(&self.graph, class, &layout, &mut placement, location)?;
        let mut state = BasePlacement {
            size: 0,
            data_size: 0,
            alignment: 1,
            empty: !dynamic
                && self.graph.entity(class).members.iter().all(|&member| {
                    let member = self.graph.entity(member);
                    member.name.is_none()
                        && member
                            .bit_field
                            .is_some_and(|id| self.graph.bit_fields.width(id) == 0)
                }),
            contains_empty: false,
            contains_volatile_or_union: false,
            contains_data_member_pointer: false,
            dynamic,
            placement,
            layout,
            claims,
        };
        for edge in &self.graph.entity(class).bases {
            let LayoutState::Value {
                empty,
                contains_empty,
                contains_volatile_or_union,
                contains_data_member_pointer,
                ..
            } = self.graph.entity(edge.class).layout
            else {
                return Err(Error::Invariant("base facts before layout"));
            };
            state.empty &= empty;
            state.contains_empty |= contains_empty;
            state.contains_volatile_or_union |= contains_volatile_or_union;
            state.contains_data_member_pointer |= contains_data_member_pointer;
        }
        if let Some(virtual_primary) = state.layout.primary {
            let index = state
                .layout
                .bases
                .iter()
                .position(|&(base, _)| base == virtual_primary)
                .ok_or(Error::Invariant("virtual primary without identity"))?;
            let LayoutState::Value {
                non_virtual_size,
                non_virtual_alignment,
                ..
            } = self.graph.entity(virtual_primary).layout
            else {
                return Err(Error::Invariant("primary before layout"));
            };
            state.size = non_virtual_size;
            state.data_size = non_virtual_size;
            state.alignment = non_virtual_alignment;
            state
                .placement
                .record_base(&self.graph, virtual_primary, 0, 0, true, location)?;
            state.claims.record(
                &self.graph,
                &mut state.layout,
                Resolver::Virtual(index),
                0,
                &mut state.placement,
                0,
                false,
                location,
            )?;
        } else if dynamic && primary.is_none() {
            state.size = 8;
            state.data_size = 8;
            state.alignment = 8;
        }
        let pack = u64::from(self.graph.entity(class).class_packing);
        for index in primary.into_iter().chain(
            (0..self.graph.entity(class).bases.len()).filter(|index| Some(*index) != primary),
        ) {
            let edge = self.graph.entity(class).bases[index];
            if edge.is_virtual {
                continue;
            }
            let offset = state.place_base(
                &self.graph,
                edge.class,
                Resolver::Direct(index),
                pack,
                location,
            )?;
            self.graph.entity_mut(class).bases[index].offset = Some(offset);
        }
        state.contains_empty |= state.empty;
        Ok(state)
    }
    pub(super) fn finish_virtual_layout(
        &mut self,
        class: EntityId,
        state: &mut BasePlacement,
        location: SourceOffset,
    ) -> Result<(), Error> {
        for index in 0..state.layout.bases.len() {
            let base = state.layout.bases[index].0;
            if Some(base) == state.layout.primary || state.claims.indirect.contains(&base) {
                continue;
            }
            state.place_base(&self.graph, base, Resolver::Virtual(index), 0, location)?;
        }
        let virtual_offsets: std::collections::HashMap<_, _> =
            state.layout.bases.iter().copied().collect();
        for edge in &mut self.graph.entity_mut(class).bases {
            if edge.is_virtual {
                edge.offset = Some(
                    *virtual_offsets
                        .get(&edge.class)
                        .ok_or(Error::Invariant("direct virtual base without identity"))?,
                );
            }
        }
        Ok(())
    }
    pub(super) fn publish_virtual_layout(&mut self, class: EntityId, state: BasePlacement) {
        if !state.layout.bases.is_empty() {
            self.graph
                .virtuals
                .storage_layouts
                .insert(class, state.layout);
        }
    }
}
impl BasePlacement {
    fn place_base(
        &mut self,
        graph: &super::graph::Graph,
        base: EntityId,
        resolver: Resolver,
        pack: u64,
        location: SourceOffset,
    ) -> Result<u64, Error> {
        let LayoutState::Value {
            size,
            alignment,
            non_virtual_size,
            non_virtual_alignment,
            empty,
            ..
        } = graph.entity(base).layout
        else {
            return Err(Error::Invariant("base placement before layout"));
        };
        let natural = if empty {
            alignment
        } else {
            non_virtual_alignment
        };
        let capped = if empty || pack == 0 {
            natural
        } else {
            natural.min(pack)
        };
        let mut offset = if empty {
            0
        } else {
            round_up(self.data_size, capped)
        };
        if self.conflict(graph, base, resolver, offset, empty, location)? {
            if empty {
                offset = round_up(self.data_size, natural);
            }
            while self.conflict(graph, base, resolver, offset, empty, location)? {
                offset = offset.wrapping_add(natural);
            }
        }
        if empty {
            self.size = self.size.max(offset.wrapping_add(size));
        } else {
            self.data_size = offset.wrapping_add(non_virtual_size);
            self.size = self.size.max(self.data_size);
        }
        if self.size >> 60 != 0 {
            return Err(Error::at(
                location,
                "a class too large (its size reaches 2^60 bytes)",
            ));
        }
        self.alignment = self.alignment.max(capped);
        if let Resolver::Virtual(index) = resolver {
            self.layout.bases[index].1 = offset;
        }
        // Virtual allocation records all empties, whereas a nonvirtual base
        // needs only the window reachable by later placements at offset zero.
        let reachable = !empty && matches!(resolver, Resolver::Direct(_));
        self.placement
            .record_base(graph, base, offset, self.data_size, !reachable, location)?;
        self.claims.record(
            graph,
            &mut self.layout,
            resolver,
            offset,
            &mut self.placement,
            self.data_size,
            reachable,
            location,
        )?;
        Ok(offset)
    }
    fn conflict(
        &mut self,
        graph: &super::graph::Graph,
        base: EntityId,
        resolver: Resolver,
        offset: u64,
        empty: bool,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        Ok(self.placement.conflict(graph, base, offset, location)?
            || !empty
                && self.claims.conflict(
                    graph,
                    &self.layout,
                    resolver,
                    offset,
                    &mut self.placement,
                    location,
                )?)
    }
}
