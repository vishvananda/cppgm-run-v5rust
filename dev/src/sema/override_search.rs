// Translated from analyzer_virtual.cpp::push_override_bases/override_facts. See NOTICE.
//! A shared base can be revisited when its primary-table path arrives later.
use super::graph::{EntityId, Graph};
use std::collections::HashMap;

pub(super) struct OverrideSearch {
    pending: Vec<(EntityId, bool)>,
    visits: HashMap<EntityId, u8>,
}
impl OverrideSearch {
    pub(super) fn new(graph: &Graph, class: EntityId, primary: Option<EntityId>) -> Self {
        let mut search = Self {
            pending: Vec::new(),
            visits: HashMap::new(),
        };
        search.descend(graph, class, primary);
        search
    }
    pub(super) fn descend(&mut self, graph: &Graph, class: EntityId, primary: Option<EntityId>) {
        let mark = self.pending.len();
        let mut met = false;
        for edge in &graph.entity(class).bases {
            let on_path = Some(edge.class) == primary;
            met |= on_path;
            self.pending.push((edge.class, on_path));
        }
        self.pending[mark..].reverse();
        if let Some(primary) = primary.filter(|_| !met) {
            self.pending.push((primary, true));
        }
    }
    pub(super) fn next(&mut self, graph: &Graph) -> Option<(EntityId, bool)> {
        while let Some((class, primary)) = self.pending.pop() {
            let visit = if primary { 2 } else { 1 };
            if !graph.lower_polymorphic(class)
                || self.visits.get(&class).is_some_and(|&old| old >= visit)
            {
                continue;
            }
            self.visits.insert(class, visit);
            return Some((class, primary));
        }
        None
    }
    pub(super) fn descend_inherited(&mut self, graph: &Graph, class: EntityId, on_primary: bool) {
        let primary = if on_primary {
            graph
                .virtuals
                .classes
                .get(&class)
                .and_then(|facts| facts.primary)
        } else {
            None
        };
        self.descend(graph, class, primary);
    }
}
