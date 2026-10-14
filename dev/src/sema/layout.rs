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
        contains_volatile_or_union: bool,
    },
    Unsupported,
}
fn round_up(value: u64, alignment: u64) -> u64 {
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
        if !self.graph.entity(entity).bases.is_empty() {
            return Err(Error::Missing("nonvirtual base subobject layout"));
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
            if let Some(&member) = self.graph.entity(entity).members.get(index) {
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
        let union = self.graph.entity(entity).key == Keyword::Union;
        let mut size = 0u64;
        let mut contains_volatile_or_union = union;
        let mut alignment = 1;
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
            if let TypeKind::Class(child) = self.graph.types.get(scalar).kind
                && let LayoutState::Value {
                    contains_volatile_or_union: child,
                    ..
                } = self.graph.entity(child).layout
            {
                contains_volatile_or_union |= child;
            }
            let offset = if union { 0 } else { round_up(size, natural) };
            self.graph.entity_mut(member).member_offset = Some(offset);
            size = if union {
                size.max(bytes)
            } else {
                offset.wrapping_add(bytes)
            };
            alignment = alignment.max(natural);
            if (bytes | size) >> 60 != 0 {
                return Err(Error::at(
                    location,
                    "a class too large (its size reaches 2^60 bytes)",
                ));
            }
        }
        self.graph.entity_mut(entity).layout = LayoutState::Value {
            contains_volatile_or_union,
            size: round_up(size.max(1), alignment),
            alignment,
        };
        Ok(())
    }
}
