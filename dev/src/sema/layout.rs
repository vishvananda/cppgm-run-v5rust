// Translated from sema/layout.cpp's type_size, lay_out and ordinary member placement. See NOTICE.
//! Demand-only layout memos and an explicit dependency worklist for plain classes/unions.
use super::{Error, analyzer::Analyzer, graph::EntityId, types::TypeKind};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

#[derive(Clone, Copy, Debug)]
pub(super) enum LayoutState {
    NotStarted,
    InProgress,
    Value { size: u64, alignment: u64 },
    Unsupported,
}
fn round_up(value: u64, alignment: u64) -> u64 {
    value.wrapping_add(alignment - 1) / alignment * alignment
}
impl Analyzer<'_> {
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
        let LayoutState::Value { size, alignment } = self.graph.entity(root).layout else {
            return Err(Error::Invariant("uncompleted layout"));
        };
        Ok((size, alignment))
    }
    fn place_members(&mut self, entity: EntityId, location: SourceOffset) -> Result<(), Error> {
        let union = self.graph.entity(entity).key == Keyword::Union;
        let mut size = 0u64;
        let mut alignment = 1;
        for index in 0..self.graph.entity(entity).members.len() {
            let member = self.graph.entity(entity).members[index];
            let ty = self
                .graph
                .entity(member)
                .ty
                .ok_or(Error::Invariant("member without type"))?;
            let (bytes, natural) = self.type_size(ty, location)?;
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
            size: round_up(size.max(1), alignment),
            alignment,
        };
        Ok(())
    }
}
