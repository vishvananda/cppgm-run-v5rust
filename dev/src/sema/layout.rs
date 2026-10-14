// Translated from sema/layout.cpp's type_size, lay_out and ordinary member placement. See NOTICE.
//! Demand-only layout memos and an explicit dependency worklist for plain classes/unions.
use super::{Error, analyzer::Analyzer, graph::EntityId, types::TypeKind};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

#[derive(Clone, Copy, Debug)]
pub(super) enum LayoutState {
    NotStarted,
    InProgress,
    Value {
        size: u64,
        alignment: u64,
        non_virtual_size: u64,
        empty: bool,
        contains_empty: bool,
        contains_volatile_or_union: bool,
    },
    Unsupported,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LayoutCategory {
    Pod,
    NonPod,
}
pub(super) fn round_up(value: u64, alignment: u64) -> u64 {
    value.wrapping_add(alignment - 1) / alignment * alignment
}
impl Analyzer<'_> {
    pub(super) fn request_storage_layout(
        &mut self,
        entity: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        let mut object = ty;
        while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
            object = element;
        }
        if matches!(self.graph.types.get(object).kind, TypeKind::Class(_)) {
            self.storage_layouts.push((
                self.body_context.as_ref().map(|body| body.entity),
                ty,
                location,
            ));
        }
        Ok(())
    }
    pub(super) fn finish_lowering_boundaries(&mut self) -> Result<(), Error> {
        use super::function_demands::Demand;
        let mut work = std::mem::take(&mut self.boundary_roots);
        for demand in self.graph.root_demands.as_slice() {
            if let Demand::Function { binding, .. } = demand {
                work.push(self.graph.binding(*binding).entity);
            }
        }
        let mut seen = std::collections::HashSet::new();
        while let Some(entity) = work.pop() {
            if !seen.insert(entity) {
                continue;
            }
            for demand in self.graph.entity(entity).demands.as_slice() {
                if let Demand::Function { binding, .. } = demand {
                    work.push(self.graph.binding(*binding).entity);
                }
            }
            if let Some(class) = self.graph.lower_function(entity)?.object {
                self.class_layout(class, SourceOffset(0))?;
            }
            let ty = self.graph.lower_entity_type(entity)?;
            let count = self.graph.types.function_parameter_count(ty)?;
            for index in 0..count {
                let parameter = self.graph.types.function_parameter(ty, index)?;
                if let TypeKind::Reference { element, .. } = self.graph.types.get(parameter).kind {
                    if matches!(self.graph.types.get(element).kind, TypeKind::Array { bound, .. } if bound != 0)
                    {
                        self.type_size(element, SourceOffset(0))?;
                        continue;
                    }
                    let mut object = element;
                    let mut bounded = true;
                    while let TypeKind::Array { element, bound } = self.graph.types.get(object).kind
                    {
                        bounded &= bound != 0;
                        object = element;
                    }
                    if bounded
                        && let TypeKind::Class(class) = self.graph.types.get(object).kind
                        && self.graph.entity(class).complete
                    {
                        self.class_layout(class, SourceOffset(0))?;
                    }
                }
            }
        }
        for (owner, ty, location) in std::mem::take(&mut self.storage_layouts) {
            if owner.is_none_or(|owner| seen.contains(&owner)) {
                self.type_size(ty, location)?;
            }
        }
        Ok(())
    }
    pub(super) fn class_layout(
        &mut self,
        root: EntityId,
        location: SourceOffset,
    ) -> Result<(u64, u64), Error> {
        let base = self.layout_work.len();
        let result = self.finish_layout(root, location);
        if result.is_err() {
            for &(entity, _) in &self.layout_work[base..] {
                self.graph.entity_mut(entity).layout = LayoutState::NotStarted;
            }
        }
        self.layout_work.truncate(base);
        result
    }
    fn start_layout(&mut self, entity: EntityId, location: SourceOffset) -> Result<bool, Error> {
        if !self.graph.entity(entity).complete {
            return Err(Error::at(location, "size of an incomplete class"));
        }
        match self.graph.entity(entity).layout {
            LayoutState::Value { .. } => Ok(false),
            LayoutState::InProgress => Err(Error::at(location, "class contains itself")),
            LayoutState::Unsupported => Err(Error::Missing(
                "class layout attributes and virtual members",
            )),
            LayoutState::NotStarted => {
                self.graph.entity_mut(entity).layout = LayoutState::InProgress;
                self.layout_work.push((entity, 0));
                Ok(true)
            }
        }
    }
    fn finish_layout(
        &mut self,
        root: EntityId,
        location: SourceOffset,
    ) -> Result<(u64, u64), Error> {
        let base = self.layout_work.len();
        self.start_layout(root, location)?;
        while self.layout_work.len() > base {
            let &(entity, index) = self
                .layout_work
                .last()
                .ok_or(Error::Invariant("missing layout frame"))?;
            let bases = self.graph.entity(entity).bases.len();
            if let Some(dependency) = self
                .graph
                .entity(entity)
                .bases
                .get(index)
                .map(|edge| edge.class)
            {
                self.layout_work
                    .last_mut()
                    .ok_or(Error::Invariant("missing layout frame"))?
                    .1 += 1;
                self.start_layout(dependency, location)?;
            } else if let Some(&member) = self.graph.entity(entity).members.get(index - bases) {
                self.layout_work
                    .last_mut()
                    .ok_or(Error::Invariant("missing layout frame"))?
                    .1 += 1;
                let mut ty = self
                    .graph
                    .entity(member)
                    .ty
                    .ok_or(Error::Invariant("member without type"))?;
                while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                if let TypeKind::Class(dependency) = self.graph.types.get(ty).kind {
                    self.start_layout(dependency, location)?;
                }
            } else {
                self.place_members(entity, location)?;
                self.layout_work.pop();
            }
        }
        let LayoutState::Value {
            size, alignment, ..
        } = self.graph.entity(root).layout
        else {
            return Err(Error::Invariant("uncompleted layout"));
        };
        Ok((size, alignment))
    }
    fn place_members(&mut self, entity: EntityId, location: SourceOffset) -> Result<(), Error> {
        let mut bits = std::mem::take(&mut self.bit_layout);
        let result = self.place_class_members(entity, location, &mut bits);
        bits.clear();
        self.bit_layout = bits;
        result
    }
    fn place_class_members(
        &mut self,
        entity: EntityId,
        location: SourceOffset,
        bits: &mut super::bit_layout::BitLayout,
    ) -> Result<(), Error> {
        let union = self.graph.entity(entity).key == Keyword::Union;
        let mut size = 0u64;
        let mut data_size = 0u64;
        let mut contains_volatile_or_union = union;
        let mut empty = self.graph.entity(entity).members.iter().all(|&member| {
            let member = self.graph.entity(member);
            member.name.is_none()
                && member
                    .bit_field
                    .is_some_and(|id| self.graph.bit_fields.width(id) == 0)
        });
        let mut contains_empty = false;
        let mut alignment = 1;
        let attributes = self.graph.entity(entity).storage_attributes;
        let pack = u64::from(self.graph.entity(entity).class_packing);
        let mut non_pod = self.graph.entity(entity).layout_category == LayoutCategory::NonPod
            || !self.graph.entity(entity).bases.is_empty();
        let mut packs_all = true;
        let mut placement = super::empty_layout::EmptyPlacement::new(&self.graph, entity)?;
        for index in 0..self.graph.entity(entity).bases.len() {
            let base = self.graph.entity(entity).bases[index].class;
            let LayoutState::Value {
                size: bytes,
                alignment: natural,
                non_virtual_size,
                empty: base_empty,
                contains_empty: child_empty,
                contains_volatile_or_union: child_volatile,
            } = self.graph.entity(base).layout
            else {
                return Err(Error::Invariant("base without completed layout"));
            };
            empty &= base_empty;
            contains_empty |= child_empty;
            contains_volatile_or_union |= child_volatile;
            let base_alignment = if base_empty || pack == 0 {
                natural
            } else {
                natural.min(pack)
            };
            let mut offset = if base_empty {
                0
            } else {
                round_up(data_size, base_alignment)
            };
            if placement.conflict(&self.graph, base, offset, location)? {
                if base_empty {
                    offset = round_up(data_size, natural);
                }
                while placement.conflict(&self.graph, base, offset, location)? {
                    offset = offset.wrapping_add(natural);
                }
            }
            if base_empty {
                size = size.max(offset.wrapping_add(bytes));
            } else {
                data_size = offset.wrapping_add(non_virtual_size);
                size = size.max(data_size);
            }
            if size >> 60 != 0 {
                return Err(Error::at(
                    location,
                    "a class too large (its size reaches 2^60 bytes)",
                ));
            }
            alignment = alignment.max(base_alignment);
            self.graph.entity_mut(entity).bases[index].offset = Some(offset);
            placement.record_base(&self.graph, base, offset, data_size, base_empty, location)?;
        }
        contains_empty |= empty;
        let mut data_bits = data_size * 8;
        for index in 0..self.graph.entity(entity).members.len() {
            let member = self.graph.entity(entity).members[index];
            let ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            let (bytes, natural) = self.type_size(ty, location)?;
            let mut scalar = ty;
            while let TypeKind::Array { element, .. } = self.graph.types.get(scalar).kind {
                scalar = element;
            }
            contains_volatile_or_union |= self.graph.types.get(scalar).cv.0 & 2 != 0;
            let mut packable = true;
            if let TypeKind::Class(child) = self.graph.types.get(scalar).kind {
                let child = self.graph.entity(child);
                non_pod |= child.layout_category == LayoutCategory::NonPod;
                packable =
                    child.layout_category == LayoutCategory::Pod || child.storage_attributes.packed;
                if let LayoutState::Value {
                    contains_volatile_or_union: child,
                    contains_empty: child_empty,
                    ..
                } = child.layout
                {
                    contains_volatile_or_union |= child;
                    contains_empty |= child_empty
                        && super::empty_layout::class_elements(&self.graph, ty)
                            .is_some_and(|(_, count)| count != 0);
                }
            }
            packs_all &= packable;
            let member_attributes = self.graph.entity(member).storage_attributes;
            let requested = u64::from(member_attributes.alignment);
            let mut member_alignment = if member_attributes.packed || attributes.packed && packable
            {
                requested.max(1)
            } else {
                natural.max(requested)
            };
            if pack != 0 {
                member_alignment = member_alignment.min(pack);
            }
            if let Some(id) = self.graph.entity(member).bit_field {
                let named = self.graph.entity(member).name.is_some();
                let placed = bits.place_member(
                    &mut self.graph.bit_fields,
                    id,
                    super::bit_layout::Placement {
                        data_bits,
                        natural: bytes,
                        requested,
                        packed: member_attributes.packed || attributes.packed,
                        pack,
                        union,
                        named,
                        location,
                    },
                )?;
                data_bits = placed.data_bits;
                size = size.max(placed.size);
                alignment = alignment.max(placed.alignment);
                self.graph.entity_mut(member).member_offset = None;
                continue;
            }
            data_size = round_up(data_bits, 8) / 8;
            let mut offset = if union {
                0
            } else {
                round_up(data_size, member_alignment)
            };
            if !union {
                while placement.member_conflict(&self.graph, ty, offset, location)? {
                    offset = round_up(offset.wrapping_add(natural), member_alignment);
                }
                data_size = offset.wrapping_add(bytes);
                placement.record_member(&self.graph, ty, offset, data_size, location)?;
            }
            self.graph.entity_mut(member).member_offset = Some(offset);
            bits.end_run(offset);
            data_bits = data_size * 8;
            size = if union {
                size.max(bytes)
            } else {
                size.max(data_size)
            };
            alignment = alignment.max(member_alignment);
            if (bytes | size) >> 60 != 0 {
                return Err(Error::at(
                    location,
                    "a class too large (its size reaches 2^60 bytes)",
                ));
            }
        }
        let requested = u64::from(attributes.alignment);
        if requested != 0 {
            if requested >= alignment {
                alignment = requested;
            } else if attributes.standard_alignment {
                return Err(Error::at(
                    location,
                    "a class alignment-specifier weaker than its natural alignment",
                ));
            }
        }
        self.graph.entity_mut(entity).layout_category = if non_pod {
            LayoutCategory::NonPod
        } else {
            LayoutCategory::Pod
        };
        self.graph.entity_mut(entity).storage_attributes.packed &= packs_all;
        let full_size = round_up(size.max(1), alignment);
        bits.finish(
            &mut self.graph.bit_fields,
            if non_pod {
                round_up(data_bits, 8) / 8
            } else {
                full_size
            },
            location,
        )?;
        self.graph.entity_mut(entity).layout = LayoutState::Value {
            contains_volatile_or_union,
            empty,
            contains_empty,
            non_virtual_size: if non_pod { size } else { full_size },
            size: full_size,
            alignment,
        };
        Ok(())
    }
}
