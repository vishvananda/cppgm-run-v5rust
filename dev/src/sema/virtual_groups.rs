// Translated from sema/analyzer_virtual.cpp and primary_bases.cpp. See NOTICE.
//! Virtual-primary recipes and shared segment projection; no body instantiation.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, Graph},
    layout::LayoutState,
    virtual_members::SlotKind,
    vtable_groups::{GroupEntry, SegmentId, VtableGroup},
};
use crate::support::source::SourceOffset;
use std::collections::{HashMap, HashSet};

impl Analyzer<'_> {
    pub(super) fn virtual_table_primary(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<Option<EntityId>, Error> {
        if let Some(edge) = self
            .graph
            .entity(class)
            .bases
            .iter()
            .find(|edge| !edge.is_virtual && self.graph.lower_dynamic(edge.class))
        {
            return Ok(Some(edge.class));
        }
        let bases = self.graph.virtual_base_order(class);
        if bases.is_empty() {
            return Ok(None);
        }
        let mut indirect = HashSet::new();
        let mut expanded = HashSet::new();
        let mut work: Vec<_> = self
            .graph
            .entity(class)
            .bases
            .iter()
            .map(|edge| edge.class)
            .collect();
        while let Some(base) = work.pop() {
            if !expanded.insert(base) {
                continue;
            }
            if self.graph.lower_dynamic(base) {
                let facts = self.graph.lower_virtual_class(base)?;
                if facts.has_virtual_bases && self.graph.nonvirtual_table_primary(base)?.is_none() {
                    indirect.extend(facts.primary);
                }
            }
            work.extend(self.graph.entity(base).bases.iter().map(|edge| edge.class));
        }
        let mut first = None;
        for base in bases {
            if !self.graph.lower_dynamic(base) {
                continue;
            }
            self.class_layout(base, location)?;
            if !matches!(
                self.graph.entity(base).layout,
                LayoutState::Value {
                    non_virtual_size: 8,
                    ..
                }
            ) {
                continue;
            }
            if !indirect.contains(&base) {
                return Ok(Some(base));
            }
            first.get_or_insert(base);
        }
        Ok(first)
    }
}
impl Graph {
    pub(crate) fn lower_dynamic(&self, class: EntityId) -> bool {
        self.lower_polymorphic(class) || self.virtuals.classes.contains_key(&class)
    }
    pub(super) fn nonvirtual_table_primary(
        &self,
        class: EntityId,
    ) -> Result<Option<EntityId>, Error> {
        Ok(self.lower_virtual_class(class)?.primary.filter(|&primary| {
            self.entity(class)
                .bases
                .iter()
                .any(|edge| !edge.is_virtual && edge.class == primary)
        }))
    }
    pub(super) fn has_virtual_primary_below(&self, class: EntityId) -> bool {
        self.virtuals
            .classes
            .get(&class)
            .is_some_and(|facts| facts.has_virtual_primary)
    }
    pub(super) fn complete_virtual_group(&mut self, class: EntityId) -> Result<(), Error> {
        let primary = self.nonvirtual_table_primary(class)?;
        self.demand_virtual_slots(class)?;
        for index in 0..self.entity(class).bases.len() {
            let edge = self.entity(class).bases[index];
            if !edge.is_virtual && Some(edge.class) != primary && self.lower_dynamic(edge.class) {
                self.demand_virtual_chain(edge.class, true)?;
            }
        }
        let bases = self.virtual_base_order(class);
        for &base in &bases {
            if self.lower_dynamic(base) {
                self.demand_virtual_chain(base, true)?;
            }
        }
        let mut group = VtableGroup::default();
        let root = group.push(class, SegmentId(0), None, false)?;
        group.slots(self, Some(class), class, root)?;
        for (index, edge) in self.entity(class).bases.iter().enumerate() {
            if edge.is_virtual || !self.lower_dynamic(edge.class) {
                continue;
            }
            if Some(edge.class) == primary {
                group.splice(self, Some(class), edge.class, None)?;
            } else {
                let id = group.push(
                    edge.class,
                    root,
                    Some((
                        class,
                        u32::try_from(index).map_err(|_| Error::Limit("base edge limit"))?,
                    )),
                    false,
                )?;
                group.slots(self, Some(class), edge.class, id)?;
                group.below(self, Some(class), edge.class, id)?;
            }
        }
        for base in bases {
            if !self.lower_dynamic(base) {
                continue;
            }
            let start = group.segments.len();
            let id = group.push(base, root, None, false)?;
            group.slots(self, None, base, id)?;
            group.below(self, None, base, id)?;
            for segment in &mut group.segments[start..] {
                segment.within = Some(id);
            }
        }
        group.outside(self, class)?;
        if self.has_virtual_primary_below(class) {
            group.share_primaries(self, class)?;
        }
        self.virtuals.groups.insert(class, group);
        self.virtuals.grouped_classes.push(class);
        Ok(())
    }
    fn demand_slot_levels(&mut self, root: EntityId) -> Result<(), Error> {
        let mut chain = Vec::new();
        let mut next = Some(root);
        while let Some(class) = next {
            if self.virtuals.slot_levels.contains_key(&class) {
                break;
            }
            chain.push(class);
            next = self.lower_virtual_class(class)?.primary;
        }
        for class in chain.into_iter().rev() {
            let primary = self.lower_virtual_class(class)?.primary;
            let virtual_edge = primary.is_some() && self.nonvirtual_table_primary(class)?.is_none();
            if !virtual_edge
                && primary.is_none_or(|base| self.virtuals.slot_levels[&base].is_empty())
            {
                self.virtuals.slot_levels.insert(class, Vec::new());
                continue;
            }
            self.demand_virtual_slots(class)?;
            let mut levels = Vec::new();
            let inherited =
                primary.map_or(&[][..], |base| self.virtuals.slot_levels[&base].as_slice());
            let virtual_level = primary.filter(|_| virtual_edge);
            for (index, slot) in self.lower_virtual_slots(class)?.iter().enumerate() {
                let slot = slot.ok_or(Error::Invariant("unfilled level slot"))?;
                let base_slot =
                    index - usize::from(matches!(slot.kind, SlotKind::DeletingDestructor));
                let own = self.entity(slot.function).home == self.entity(class).scope
                    && self
                        .lower_virtual_slot(slot.function)
                        .is_some_and(|slot| slot.index().is_ok_and(|index| index == base_slot));
                let level = if own {
                    None
                } else {
                    inherited.get(index).copied().flatten().or(virtual_level)
                };
                levels.push(level);
            }
            self.virtuals.slot_levels.insert(class, levels);
        }
        Ok(())
    }
}
impl VtableGroup {
    fn outside(&mut self, graph: &mut Graph, class: EntityId) -> Result<(), Error> {
        // Each virtual base lists only the subobjects that have it. The chain
        // candidate is memoized per head, function and virtual base.
        let mut holders: HashMap<EntityId, Vec<SegmentId>> = HashMap::new();
        for (index, segment) in self.segments.iter().enumerate() {
            for base in graph.virtual_base_order(segment.class) {
                holders
                    .entry(base)
                    .or_default()
                    .push(SegmentId::new(index)?);
            }
        }
        for index in 1..self.segments.len() {
            let Some(within) = self.segments[index].within else {
                continue;
            };
            let base = self.segments[within.index()].class;
            let Some(holders) = holders.get(&base) else {
                continue;
            };
            for slot in self.segments[index].entries.clone() {
                let mut candidates = Vec::new();
                let entry = self.entries[slot];
                for &holder in holders {
                    if self.segments[holder.index()].within == Some(within) {
                        continue;
                    }
                    let head = self.segments[holder.index()].class;
                    let found = if holder.index() == 0 {
                        graph.chain_overrider(head, entry.function, base)?
                    } else {
                        graph.declared_virtual_overrider(head, entry.function)?
                    };
                    if let Some(function) = found {
                        candidates.push(GroupEntry {
                            function,
                            kind: entry.kind,
                            overrider: holder,
                        });
                    }
                }
                let mut chosen: Option<GroupEntry> = None;
                for &candidate in &candidates {
                    let mut dominated = false;
                    for &other in &candidates {
                        dominated |= self.dominates(graph, class, other, candidate)?;
                    }
                    if dominated {
                        continue;
                    }
                    match chosen {
                        Some(old)
                            if old.function != candidate.function
                                || old.overrider != candidate.overrider =>
                        {
                            return Err(Error::Invariant(
                                "ambiguous projection after final-overrider analysis",
                            ));
                        }
                        Some(_) => {}
                        None => chosen = Some(candidate),
                    }
                }
                if let Some(chosen) = chosen {
                    self.entries[slot] = chosen;
                }
            }
        }
        Ok(())
    }
    fn dominates(
        &self,
        graph: &Graph,
        class: EntityId,
        over: GroupEntry,
        under: GroupEntry,
    ) -> Result<bool, Error> {
        let owner = |function| {
            graph
                .lower_function(function)?
                .object
                .ok_or(Error::Invariant("virtual without class"))
        };
        let derived = owner(over.function)?;
        let base = owner(under.function)?;
        if derived == base || !graph.is_base_of(base, derived) {
            return Ok(false);
        }
        if let Some(within) = self.segments[under.overrider.index()].within
            && graph
                .virtuals
                .base_memberships
                .contains(&(derived, self.segments[within.index()].class))
        {
            return Ok(true);
        }
        if under.overrider == over.overrider {
            return Ok(true);
        }
        let mut id = under.overrider;
        while id.index() != 0 && self.segments[id.index()].within != Some(id) {
            let segment = &self.segments[id.index()];
            let parent = segment.parent;
            if parent == over.overrider {
                if parent.index() != 0 {
                    return Ok(true);
                }
                let edge_owner = segment
                    .edge
                    .ok_or(Error::Invariant("chain without edge"))?
                    .0;
                let mut link = Some(class);
                while let Some(current) = link {
                    if current == derived {
                        return Ok(true);
                    }
                    if current == edge_owner {
                        return Ok(false);
                    }
                    link = graph.nonvirtual_table_primary(current)?;
                }
                return Err(Error::Invariant("unmapped primary-chain depth"));
            }
            id = parent;
        }
        Ok(false)
    }
    fn share_primaries(&mut self, graph: &mut Graph, class: EntityId) -> Result<(), Error> {
        let mut offsets = vec![0u64];
        for (index, segment) in self.segments.iter().enumerate().skip(1) {
            let offset = if segment.within == Some(SegmentId::new(index)?) {
                graph
                    .lower_virtual_base_offset(class, segment.class)
                    .ok_or(Error::Invariant("sharing before virtual layout"))?
            } else {
                let (owner, edge) = segment
                    .edge
                    .ok_or(Error::Invariant("sharing without edge"))?;
                offsets[segment.parent.index()].wrapping_add(
                    graph.entity(owner).bases[edge as usize]
                        .offset
                        .ok_or(Error::Invariant("sharing before base layout"))?,
                )
            };
            offsets.push(offset);
        }
        let mut claimants: HashMap<(EntityId, u64), SegmentId> = HashMap::new();
        for (index, segment) in self.segments.iter().enumerate().skip(1).rev() {
            if graph.nonvirtual_table_primary(segment.class)?.is_none()
                && let Some(primary) = graph.lower_virtual_class(segment.class)?.primary
            {
                claimants.insert((primary, offsets[index]), SegmentId::new(index)?);
            }
        }
        let mut chained = HashSet::new();
        let mut link = Some(class);
        while let Some(class) = link {
            if graph.nonvirtual_table_primary(class)?.is_none() {
                chained.extend(graph.lower_virtual_class(class)?.primary);
            }
            link = graph.nonvirtual_table_primary(class)?;
        }
        let mut levels = HashMap::new();
        for (index, &offset) in offsets.iter().enumerate().skip(1) {
            let id = SegmentId::new(index)?;
            let segment = &mut self.segments[index];
            if segment.within != Some(id) {
                continue;
            }
            levels.insert(segment.class, id);
            let parent = claimants
                .get(&(segment.class, offset))
                .copied()
                .filter(|&parent| parent != id)
                .or_else(|| {
                    (offset == 0 && chained.contains(&segment.class)).then_some(SegmentId(0))
                });
            if let Some(parent) = parent {
                segment.shared = true;
                segment.parent = parent;
            }
        }
        for index in 0..self.segments.len() {
            let class = self.segments[index].class;
            graph.demand_slot_levels(class)?;
            for (slot, level) in graph.virtuals.slot_levels[&class].iter().enumerate() {
                if let Some(level) = level.and_then(|class| levels.get(&class).copied())
                    && level.index() != index
                {
                    self.entries[self.segments[index].entries.start + slot] =
                        self.entries[self.segments[level.index()].entries.start + slot];
                }
            }
        }
        Ok(())
    }
}
impl Graph {
    fn chain_overrider(
        &mut self,
        head: EntityId,
        function: EntityId,
        base: EntityId,
    ) -> Result<Option<EntityId>, Error> {
        // Completed classes cannot add an override. This complete typed key is
        // unit-owned, so extending a group reuses every earlier chain suffix.
        let mut walk = Vec::new();
        let mut next = Some(head);
        let mut found = None;
        while let Some(class) = next {
            if let Some(&known) = self.virtuals.chain_overriders.get(&(class, function, base)) {
                found = known;
                break;
            }
            if class != head && !self.virtuals.base_memberships.contains(&(class, base)) {
                break;
            }
            walk.push(class);
            found = self.declared_virtual_overrider(class, function)?;
            if found.is_some() {
                break;
            }
            next = self.nonvirtual_table_primary(class)?;
        }
        for class in walk {
            self.virtuals
                .chain_overriders
                .insert((class, function, base), found);
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };
    fn inspect(source: &[u8], check: impl FnOnce(&Graph, EntityId)) {
        let source = SourceBuffer::new(source.to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"virtual-primary").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"D").unwrap();
        let expression_mark = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Elaborated).unwrap())
            .entity;
        check(&graph, class);
        assert_eq!(tree.expressions.mark(), expression_mark);
        assert!(graph.root_demands.as_slice().is_empty());
    }
    #[test]
    fn virtual_primary_preserves_slots_and_destructor_pair_across_a_nonvirtual_chain() {
        inspect(b"struct R{virtual R*copy();virtual ~R();};struct A:R{virtual void side();};struct D:virtual A{virtual int first();virtual int second();};", |graph,class| {
            let slots = graph.lower_virtual_slots(class).unwrap();
            assert_eq!(slots.len(), 6);
            assert!(matches!(slots[1].unwrap().kind, SlotKind::CompleteDestructor));
            assert!(matches!(slots[2].unwrap().kind, SlotKind::DeletingDestructor));
            assert_eq!(slots[1].unwrap().function,slots[2].unwrap().function);
            assert_eq!(slots[5].unwrap().slot.byte_offset(),40);
            let group = graph.lower_vtable_group(class).unwrap();
            assert_eq!(group.segments.len(),3);
            assert!(group.segments[1].shared);
            assert!(group.segments[2].shared);
            assert_eq!(group.segments[1].within,Some(SegmentId(1)));
            assert_eq!(group.segments[2].within,Some(SegmentId(1)));
            assert_eq!(group.segments[2].parent,SegmentId(1));
            assert_eq!(graph.lower_segment_offsets(class).unwrap(),[0,0,0]);
            assert_eq!(graph.lower_virtual_class(class).unwrap().primary,Some(group.segments[1].class));
        });
    }
    #[test]
    fn shared_virtual_slots_take_the_outside_overriders_actual_subobject() {
        inspect(b"struct R{virtual int f();int n;};struct A:virtual R{int f();};struct B:virtual R{};struct D:B,A{};", |graph,class| {
            let group = graph.lower_vtable_group(class).unwrap();
            let shared = group.segments.iter().enumerate().find(|(i,s)|s.within.is_some_and(|id| id.index()==*i)).unwrap();
            let entry = group.entries[shared.1.entries.start];
            assert_eq!(entry.overrider,SegmentId(1));
            assert_eq!(graph.lower_function(entry.function).unwrap().object,Some(group.segments[1].class));
            assert!(matches!(graph.entity(class).layout,LayoutState::NotStarted));
        });
    }
    #[test]
    fn external_primary_ancestors_keep_only_the_demanded_group_recipe() {
        inspect(b"struct R{virtual int f();};struct A:R{virtual int g();};struct B:A{virtual int h();};struct V{};struct D:B,virtual V{};", |graph,class| {
            assert_eq!(graph.lower_virtual_slots(class).unwrap().len(),3);
            let mut next = graph.lower_virtual_class(class).unwrap().primary;
            while let Some(base) = next {
                assert!(graph.lower_virtual_slots(base).is_err());
                next = graph.lower_virtual_class(base).unwrap().primary;
            }
            assert!(matches!(graph.entity(class).layout,LayoutState::NotStarted));
        });
    }
    #[test]
    fn projected_template_slots_leave_unreferenced_member_bodies_uninstantiated() {
        inspect(b"template<class T>struct R{virtual int f(){return T::missing();}};struct D:virtual R<int>{virtual int g(){return 2;}};", |graph,class| {
            let root = graph.lower_virtual_slots(class).unwrap()[0].unwrap().function;
            assert!(!graph.function_templates.bodies.started(root));
            assert_eq!(graph.lower_vtable_group(class).unwrap().segments.len(),2);
            assert!(matches!(graph.entity(class).layout,LayoutState::Value{..}));
        });
    }
}
