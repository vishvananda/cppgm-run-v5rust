// Translated from analyzer_virtual.cpp::vtable_group/group_slots/group_below/group_splice.
// See NOTICE.
//! ABI segments preserve each occurrence and its selected overrider.
use super::{
    Error,
    graph::{EntityId, Graph},
    virtual_members::SlotKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SegmentId(pub(super) u32);
impl SegmentId {
    #[inline]
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
    pub(super) fn new(index: usize) -> Result<Self, Error> {
        Ok(Self(
            u32::try_from(index).map_err(|_| Error::Limit("vtable segment limit"))?,
        ))
    }
}
#[derive(Clone, Copy)]
pub struct GroupEntry {
    pub function: EntityId,
    pub kind: SlotKind,
    pub overrider: SegmentId,
}
pub struct Segment {
    pub class: EntityId,
    pub parent: SegmentId,
    // An edge is identified by its owning class and declaration ordinal.
    pub edge: Option<(EntityId, u32)>,
    pub shared: bool,
    pub within: Option<SegmentId>,
    pub entries: std::ops::Range<usize>,
}
#[derive(Default)]
pub struct VtableGroup {
    pub segments: Vec<Segment>,
    pub entries: Vec<GroupEntry>,
}
impl VtableGroup {
    pub(super) fn push(
        &mut self,
        class: EntityId,
        parent: SegmentId,
        edge: Option<(EntityId, u32)>,
        shared: bool,
    ) -> Result<SegmentId, Error> {
        let id = SegmentId::new(self.segments.len())?;
        self.segments.push(Segment {
            class,
            parent,
            edge,
            shared,
            within: None,
            entries: self.entries.len()..self.entries.len(),
        });
        Ok(id)
    }
    pub(super) fn close(&mut self, id: SegmentId) {
        self.segments[id.index()].entries.end = self.entries.len();
    }
    pub(super) fn slots(
        &mut self,
        graph: &Graph,
        top: Option<EntityId>,
        class: EntityId,
        id: SegmentId,
    ) -> Result<(), Error> {
        for slot in graph.lower_virtual_slots(class)? {
            let slot = slot.ok_or(Error::Invariant("unfilled group slot"))?;
            let found = top
                .map(|top| graph.declared_virtual_overrider(top, slot.function))
                .transpose()?
                .flatten();
            self.entries.push(GroupEntry {
                function: found.unwrap_or(slot.function),
                kind: slot.kind,
                overrider: if found.is_some() { SegmentId(0) } else { id },
            });
        }
        self.close(id);
        Ok(())
    }
    pub(super) fn splice(
        &mut self,
        graph: &Graph,
        top: Option<EntityId>,
        class: EntityId,
        chain: Option<&crate::support::hash::HashMap<EntityId, SegmentId>>,
    ) -> Result<(), Error> {
        let Some(group) = graph.virtuals.groups.get(&class) else {
            return Ok(());
        };
        let first = self.segments.len() - 1;
        for from in group
            .segments
            .iter()
            .skip(1)
            .take_while(|part| part.within.is_none())
        {
            let map = |id: SegmentId, owner: EntityId| -> Result<SegmentId, Error> {
                if id.index() != 0 {
                    SegmentId::new(first + id.index())
                } else if let Some(chain) = chain {
                    chain
                        .get(&owner)
                        .copied()
                        .ok_or(Error::Invariant("unmapped primary-chain class"))
                } else {
                    Ok(SegmentId(0))
                }
            };
            let (owner, _) = from
                .edge
                .ok_or(Error::Invariant("segment without base edge"))?;
            let parent = map(from.parent, owner)?;
            let id = self.push(from.class, parent, from.edge, from.shared)?;
            for slot in &group.entries[from.entries.clone()] {
                let found = top
                    .map(|top| graph.declared_virtual_overrider(top, slot.function))
                    .transpose()?
                    .flatten();
                let owner = graph
                    .lower_function(slot.function)?
                    .object
                    .ok_or(Error::Invariant("virtual without owner"))?;
                self.entries.push(GroupEntry {
                    function: found.unwrap_or(slot.function),
                    kind: slot.kind,
                    overrider: if found.is_some() {
                        SegmentId(0)
                    } else {
                        map(slot.overrider, owner)?
                    },
                });
            }
            self.close(id);
        }
        Ok(())
    }
    pub(super) fn below(
        &mut self,
        graph: &Graph,
        top: Option<EntityId>,
        class: EntityId,
        segment: SegmentId,
    ) -> Result<(), Error> {
        let mut chain = crate::support::hash::HashMap::default();
        let mut parent = segment;
        let mut link = class;
        loop {
            chain.insert(link, parent);
            let Some(next) = graph.nonvirtual_table_primary(link)? else {
                break;
            };
            let edge = graph
                .entity(link)
                .class()
                .bases
                .iter()
                .position(|base| !base.is_virtual && base.class == next)
                .ok_or(Error::Invariant("primary without direct edge"))?;
            let id = self.push(
                next,
                parent,
                Some((
                    link,
                    u32::try_from(edge).map_err(|_| Error::Limit("base edge limit"))?,
                )),
                true,
            )?;
            let above = self.segments[parent.index()].entries.start;
            for (index, slot) in graph.lower_virtual_slots(next)?.iter().enumerate() {
                let slot = slot.ok_or(Error::Invariant("unfilled primary slot"))?;
                let mut entry = self.entries[above + index];
                entry.kind = slot.kind;
                if entry.overrider == parent {
                    let found = graph.declared_virtual_overrider(link, slot.function)?;
                    entry = GroupEntry {
                        function: found.unwrap_or(slot.function),
                        kind: slot.kind,
                        overrider: if found.is_some() { parent } else { id },
                    };
                }
                self.entries.push(entry);
            }
            self.close(id);
            link = next;
            parent = id;
        }
        self.splice(graph, top, class, Some(&chain))
    }
}
impl Graph {
    pub(super) fn complete_nonvirtual_group(&mut self, class: EntityId) -> Result<(), Error> {
        // Virtual-primary selection, virtual sharing and outside overriders
        // require the virtual-base layout group. Leave those facts undemanded.
        if self.lower_virtual_class(class)?.has_virtual_bases {
            return Ok(());
        }
        let primary = self.lower_virtual_class(class)?.primary;
        let secondary = self.entity(class).class().bases.iter().any(|edge| {
            self.lower_polymorphic(edge.class)
                && (Some(edge.class) != primary || self.virtuals.groups.contains_key(&edge.class))
        });
        if !secondary {
            return Ok(());
        }
        self.demand_virtual_slots(class)?;
        for index in 0..self.entity(class).class().bases.len() {
            let base = self.entity(class).class().bases[index].class;
            if self.lower_polymorphic(base) {
                self.demand_virtual_chain(base, Some(base) != primary)?;
            }
        }
        let mut group = VtableGroup::default();
        let root = group.push(class, SegmentId(0), None, false)?;
        group.slots(self, Some(class), class, root)?;
        for (index, edge) in self.entity(class).class().bases.iter().enumerate() {
            if !self.lower_polymorphic(edge.class) {
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
        self.virtuals.groups.insert(class, group);
        self.virtuals.grouped_classes.push(class);
        Ok(())
    }
    #[inline]
    #[must_use]
    pub fn lower_grouped_classes(&self) -> &[EntityId] {
        &self.virtuals.grouped_classes
    }
    #[inline]
    #[must_use]
    pub fn lower_vtable_group(&self, class: EntityId) -> Option<&VtableGroup> {
        self.virtuals.groups.get(&class)
    }
    pub fn lower_segment_offsets(&self, class: EntityId) -> Result<Vec<u64>, Error> {
        let Some(group) = self.lower_vtable_group(class) else {
            return Ok(vec![0]);
        };
        let mut offsets = Vec::<u64>::with_capacity(group.segments.len());
        offsets.push(0);
        for (index, segment) in group.segments.iter().enumerate().skip(1) {
            if segment.within == Some(SegmentId::new(index)?) {
                let offset = self
                    .lower_virtual_base_offset(class, segment.class)
                    .ok_or(Error::Invariant("virtual segment before layout"))?;
                offsets.push(offset);
                continue;
            }
            let (owner, index) = segment
                .edge
                .ok_or(Error::Invariant("segment without base edge"))?;
            let offset = self.entity(owner).class().bases[index as usize]
                .offset
                .ok_or(Error::Invariant("segment before layout"))?;
            offsets.push(offsets[segment.parent.index()].wrapping_add(offset));
        }
        Ok(offsets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, layout::LayoutState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    fn inspect(source: &[u8], check: impl FnOnce(&Graph, EntityId)) {
        inspect_with_instances(source, false, check);
    }
    fn inspect_with_instances(
        source: &[u8],
        instantiated: bool,
        check: impl FnOnce(&Graph, EntityId),
    ) {
        let source = SourceBuffer::new(source.to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"vtable-projection").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"D").unwrap();
        let expressions = tree.expressions.mark();
        let syntax = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Elaborated).unwrap())
            .entity;
        check(&graph, class);
        assert!(matches!(
            graph.entity(class).class().layout,
            LayoutState::NotStarted
        ));
        assert!(graph.root_demands.as_slice().is_empty());
        // A template specialization legitimately owns substituted syntax.
        // Ordinary declarations must keep exactly the parsed store marks.
        if !instantiated {
            assert_eq!(expressions, tree.expressions.mark());
            assert_eq!(syntax, tree.declarations.mark());
        }
    }

    #[test]
    fn secondary_override_appends_after_unrelated_primary_slots() {
        inspect(b"struct A{virtual int f();};struct B{virtual int g();};struct I:A,B{};struct D:I{int g();};", |graph, class| {
            let slots = graph.lower_virtual_slots(class).unwrap();
            assert_eq!(slots.len(), 2);
            assert_ne!(graph.lower_function(slots[0].unwrap().function).unwrap().object, Some(class));
            assert_eq!(graph.lower_function(slots[1].unwrap().function).unwrap().object, Some(class));
            let group = graph.lower_vtable_group(class).unwrap();
            assert_eq!(group.segments.len(), 2);
            let entry = group.entries[group.segments[1].entries.start];
            assert_eq!(entry.function, slots[1].unwrap().function);
            assert_eq!(entry.overrider, SegmentId(0));
        });
    }

    #[test]
    fn secondary_primary_chain_preserves_shared_pointer_and_occurrences() {
        inspect(b"struct R{virtual int f();};struct A:R{};struct B:R{};struct M:A,B{};struct X{virtual int x();};struct D:X,M{};", |graph, class| {
            let group = graph.lower_vtable_group(class).unwrap();
            assert_eq!(group.segments.len(), 6);
            assert!(!group.segments[1].shared);
            assert!(group.segments[2].shared);
            assert_eq!(group.segments[2].parent, SegmentId(1));
            assert!(group.segments[3].shared);
            assert_eq!(group.segments[3].parent, SegmentId(2));
            assert!(!group.segments[4].shared);
            assert!(group.segments[5].shared);
            assert_eq!(group.segments[5].parent, SegmentId(4));
            assert_eq!(group.segments[3].class, group.segments[5].class);
            let left = group.entries[group.segments[3].entries.start];
            let right = group.entries[group.segments[5].entries.start];
            assert_eq!(left.function, right.function);
            assert_ne!(left.overrider, right.overrider);
        });
    }

    #[test]
    fn implicit_secondary_destructor_takes_two_slots_after_written_functions() {
        inspect(
            b"struct A{virtual int f();};struct B{virtual ~B();};struct D:A,B{virtual int g();};",
            |graph, class| {
                let slots = graph.lower_virtual_slots(class).unwrap();
                assert_eq!(slots.len(), 4);
                assert!(matches!(
                    slots[2].unwrap().kind,
                    SlotKind::CompleteDestructor
                ));
                assert!(matches!(
                    slots[3].unwrap().kind,
                    SlotKind::DeletingDestructor
                ));
                assert_eq!(slots[2].unwrap().function, slots[3].unwrap().function);
                let group = graph.lower_vtable_group(class).unwrap();
                let entries = &group.entries[group.segments[1].entries.clone()];
                assert_eq!(entries.len(), 2);
                assert!(entries.iter().all(|entry| entry.overrider == SegmentId(0)));
            },
        );
    }
    #[test]
    fn covariant_introducer_keeps_the_old_slots_adjusted_result() {
        inspect(b"struct P{int p;};struct Q{int q;};struct R:P,Q{};struct A{virtual Q* get();};struct B{virtual void f();};struct D:A,B{R* get();};", |graph, class| {
            let slots = graph.lower_virtual_slots(class).unwrap();
            assert_eq!(slots.len(), 2);
            let old = slots[0].unwrap();
            let own = slots[1].unwrap();
            assert_eq!(old.function, own.function);
            assert_ne!(old.introducer, own.introducer);
            assert_ne!(old.adjusted, own.adjusted);
            assert_eq!(own.adjusted, own.function);
            let result = graph.lower_covariant_result(own.function, old.adjusted, class).unwrap().unwrap();
            assert!(!result.reference);
            assert_eq!(graph.lower_nonvirtual_offset(result.from, result.to).unwrap(), Some(4));
        });
    }

    #[test]
    fn unused_secondary_covariant_result_retains_identity_without_layout_demand() {
        inspect(b"struct P{int p;};struct Q{int q;};struct R:P,Q{};struct A{virtual void f();};struct B{virtual Q*get();};struct D:A,B{R*get();};", |graph, class| {
            let entry = graph.lower_virtual_slots(class).unwrap()[1].unwrap();
            let ty = graph.lower_entity_type(entry.function).unwrap();
            let super::super::lowering::TypeView::Function { result, .. } = graph.lower_type(ty).unwrap() else {
                panic!("virtual signature");
            };
            let super::super::lowering::TypeView::Pointer(result) = graph.lower_type(result).unwrap() else {
                panic!("covariant pointer");
            };
            let super::super::lowering::TypeView::Class(result) = graph.lower_type(result).unwrap() else {
                panic!("covariant class");
            };
            assert!(matches!(graph.entity(result).class().layout, LayoutState::NotStarted));
        });
    }

    #[test]
    fn projection_does_not_instantiate_unused_virtual_bodies() {
        inspect_with_instances(b"template<class T>struct A{virtual int f(){return T::missing();}};struct B{virtual int g();};struct D:A<int>,B{};", true, |graph,class| {
            let entry = graph.lower_virtual_slots(class).unwrap()[0].unwrap();
            assert!(!graph.function_templates.bodies.started(entry.function));
            assert!(graph.lower_vtable_group(class).is_some());
        });
    }
    #[test]
    fn cached_group_head_retains_an_external_ancestor_for_secondary_projection() {
        inspect(b"struct R{virtual int f();};struct A:R{};struct B{virtual int g(){return 1;}};struct M:A,B{};struct X{virtual int x();};struct D:X,M{};", |graph,class| {
            let group = graph.lower_vtable_group(class).unwrap();
            assert_eq!(group.segments.len(), 5);
            let external = group.segments[3].class;
            assert!(group.segments[3].shared);
            assert_eq!(graph.lower_virtual_slots(external).unwrap().len(), 1);
            assert!(!graph.lower_emits_vtable(external).unwrap());
            assert!(graph.lower_class_layout(external).is_err());
        });
    }
}
