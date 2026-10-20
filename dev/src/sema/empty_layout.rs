// Translated from sema/layout.cpp's empty_subobjects, member_conflict and record_member. See NOTICE.
//! Query-owned placement set and iterative, gap-pruned empty-subobject walks.
use super::{
    Error,
    graph::{EntityId, Graph},
    layout::LayoutState,
    types::{TypeId, TypeKind},
};
use crate::support::source::SourceOffset;
use std::collections::HashSet;

pub(super) fn class_elements(graph: &Graph, mut ty: TypeId) -> Option<(EntityId, u64)> {
    let mut count = 1u64;
    while let TypeKind::Array { element, bound, .. } = graph.types.get(ty).kind {
        count = count.wrapping_mul(bound);
        ty = element;
    }
    graph.class_of(ty).map(|class| (class, count))
}

fn first_outside_gap(start: u64, size: u64, index: u64, begin: u64, end: u64) -> u64 {
    let at = start.wrapping_add(index.wrapping_mul(size));
    if at < begin || end != u64::MAX && at.wrapping_add(size) > end {
        index
    } else if end == u64::MAX {
        u64::MAX
    } else {
        (end - start) / size
    }
}

#[derive(Clone, Copy)]
enum Walk {
    Conflict,
    Record,
    RecordReachable(u64),
}

pub(super) struct EmptyPlacement {
    placed: HashSet<(EntityId, u64)>,
    work: Vec<(EntityId, u64, bool)>,
    maximum: u64,
    record_limit: u64,
    budget: u64,
    hint: Option<(EntityId, EntityId, u64, bool)>,
}
impl EmptyPlacement {
    pub(super) fn new(
        graph: &Graph,
        class: EntityId,
        virtual_bases: &[(EntityId, u64)],
    ) -> Result<Self, Error> {
        let mut record_limit = 0;
        for base in &graph.entity(class).bases {
            let LayoutState::Value { empty, size, .. } = graph.entity(base.class).layout else {
                return Err(Error::Invariant("empty placement before base layout"));
            };
            if empty {
                record_limit = record_limit.max(size);
            }
        }
        for &(base, _) in virtual_bases {
            let LayoutState::Value { empty, size, .. } = graph.entity(base).layout else {
                return Err(Error::Invariant(
                    "empty placement before virtual base layout",
                ));
            };
            if empty {
                record_limit = record_limit.max(size);
            }
        }
        Ok(Self {
            placed: HashSet::new(),
            work: Vec::new(),
            maximum: 0,
            record_limit,
            budget: 1 << 20,
            hint: None,
        })
    }
    pub(super) fn spend(&mut self, location: SourceOffset) -> Result<(), Error> {
        if self.budget == 0 {
            return Err(Error::at(
                location,
                "class layout needs more work than this implementation allows",
            ));
        }
        self.budget -= 1;
        Ok(())
    }
    pub(super) fn conflict(
        &mut self,
        graph: &Graph,
        class: EntityId,
        offset: u64,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        self.conflict_object(graph, class, offset, false, location)
    }
    fn conflict_object(
        &mut self,
        graph: &Graph,
        class: EntityId,
        offset: u64,
        complete: bool,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        if self.placed.is_empty() {
            return Ok(false);
        }
        if let Some((root, child, relative, hint_complete)) = self.hint
            && root == class
            && hint_complete == complete
            && self
                .placed
                .contains(&(child, offset.wrapping_add(relative)))
        {
            return Ok(true);
        }
        self.walk(graph, class, offset, complete, Walk::Conflict, location)
    }
    pub(super) fn record_base(
        &mut self,
        graph: &Graph,
        class: EntityId,
        offset: u64,
        tail: u64,
        empty: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.walk(
            graph,
            class,
            offset,
            false,
            if empty {
                Walk::Record
            } else {
                Walk::RecordReachable(tail)
            },
            location,
        )?;
        Ok(())
    }
    pub(super) fn member_conflict(
        &mut self,
        graph: &Graph,
        ty: TypeId,
        offset: u64,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let Some((class, count)) = class_elements(graph, ty) else {
            return Ok(false);
        };
        let LayoutState::Value {
            contains_empty,
            size,
            ..
        } = graph.entity(class).layout
        else {
            return Err(Error::Invariant("member conflict before layout"));
        };
        if !contains_empty {
            return Ok(false);
        }
        for index in 0..count {
            let at = offset.wrapping_add(index.wrapping_mul(size));
            if at > self.maximum {
                break;
            }
            if self.conflict_object(graph, class, at, true, location)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn record_member(
        &mut self,
        graph: &Graph,
        ty: TypeId,
        offset: u64,
        tail: u64,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let Some((class, count)) = class_elements(graph, ty) else {
            return Ok(());
        };
        let LayoutState::Value {
            contains_empty,
            size,
            ..
        } = graph.entity(class).layout
        else {
            return Err(Error::Invariant("member record before layout"));
        };
        if !contains_empty {
            return Ok(());
        }
        let mut index = first_outside_gap(offset, size, 0, self.record_limit, tail);
        while index < count {
            self.walk(
                graph,
                class,
                offset.wrapping_add(index.wrapping_mul(size)),
                true,
                Walk::RecordReachable(tail),
                location,
            )?;
            index = first_outside_gap(offset, size, index + 1, self.record_limit, tail);
        }
        Ok(())
    }
    fn walk(
        &mut self,
        graph: &Graph,
        root: EntityId,
        offset: u64,
        complete: bool,
        walk: Walk,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let (gap_begin, gap_end) = match walk {
            Walk::Conflict => (self.maximum + 1, u64::MAX),
            Walk::Record => (u64::MAX, u64::MAX),
            Walk::RecordReachable(tail) => (self.record_limit, tail),
        };
        self.work.clear();
        self.work.push((root, offset, complete));
        while let Some((class, at, complete_object)) = self.work.pop() {
            let LayoutState::Value {
                size,
                empty,
                contains_empty,
                ..
            } = graph.entity(class).layout
            else {
                return Err(Error::Invariant("empty walk before layout"));
            };
            if !contains_empty
                || at >= gap_begin && (gap_end == u64::MAX || at.wrapping_add(size) <= gap_end)
            {
                continue;
            }
            if empty {
                if matches!(walk, Walk::Conflict) {
                    if self.placed.contains(&(class, at)) {
                        self.hint = Some((root, class, at - offset, complete));
                        self.work.clear();
                        return Ok(true);
                    }
                } else {
                    self.placed.insert((class, at));
                    self.maximum = self.maximum.max(at);
                }
            }
            for base in &graph.entity(class).bases {
                if base.is_virtual {
                    continue;
                }
                self.spend(location)?;
                self.work.push((
                    base.class,
                    at.wrapping_add(
                        base.offset
                            .ok_or(Error::Invariant("empty walk without base offset"))?,
                    ),
                    false,
                ));
            }
            if complete_object && let Some(layout) = graph.virtuals.storage_layouts.get(&class) {
                for &(base, offset) in &layout.bases {
                    self.spend(location)?;
                    self.work.push((base, at.wrapping_add(offset), false));
                }
            }
            for &member in &graph.entity(class).members {
                let node = graph.entity(member);
                let Some((element, count)) = class_elements(
                    graph,
                    node.ty
                        .ok_or(Error::Invariant("empty walk without member type"))?,
                ) else {
                    continue;
                };
                let LayoutState::Value {
                    size: bytes,
                    contains_empty: child_empty,
                    ..
                } = graph.entity(element).layout
                else {
                    return Err(Error::Invariant("empty walk without member layout"));
                };
                if !child_empty {
                    continue;
                }
                let start = at.wrapping_add(
                    node.member_offset
                        .ok_or(Error::Invariant("empty walk without member offset"))?,
                );
                let mut index = first_outside_gap(start, bytes, 0, gap_begin, gap_end);
                while index < count {
                    self.spend(location)?;
                    self.work
                        .push((element, start.wrapping_add(index.wrapping_mul(bytes)), true));
                    index = first_outside_gap(start, bytes, index + 1, gap_begin, gap_end);
                }
            }
        }
        Ok(false)
    }
}
