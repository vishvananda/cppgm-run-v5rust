// Translated from analyzer_member_pointers.cpp and analyzer_access.cpp. See NOTICE.
//! Bounded completed-class subobject facts and per-access reachability scratch.
use super::graph::{EntityId, Graph};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use std::cell::RefCell;

#[derive(Clone, Copy, Default)]
pub(super) struct Fact {
    pub(super) count: u8,
    pub(super) virtual_path: bool,
    nonvirtual_path: bool,
    counted_virtual: bool,
}
impl Fact {
    fn one() -> Self {
        Self {
            count: 1,
            nonvirtual_path: true,
            ..Self::default()
        }
    }
    fn receive(&mut self, above: Self, is_virtual: bool) {
        if is_virtual {
            self.count = self
                .count
                .saturating_add(u8::from(!self.counted_virtual))
                .min(2);
            self.counted_virtual = true;
            self.virtual_path = true;
        } else {
            self.count = self.count.saturating_add(above.count).min(2);
            self.nonvirtual_path |= above.nonvirtual_path;
            self.virtual_path |= above.virtual_path;
        }
    }
}
#[derive(Default)]
pub(super) struct Tables {
    cache: RefCell<Cache>,
    simple: RefCell<HashMap<EntityId, bool>>,
}
#[derive(Default)]
struct Cache {
    entries: usize,
    tables: HashMap<EntityId, Option<HashMap<EntityId, Fact>>>,
}
impl Tables {
    // An incomplete class's bases can still change. Only completed owners
    // enter either cache; their base identities and edges are immutable.
    pub(super) fn simple(&self, graph: &Graph, class: EntityId) -> bool {
        if let Some(&simple) = self.simple.borrow().get(&class) {
            return simple;
        }
        let mut seen = HashSet::from_iter([class]);
        let mut pending = vec![class];
        let mut simple = true;
        'walk: while let Some(class) = pending.pop() {
            for edge in &graph.entity(class).class().bases {
                if edge.is_virtual || !seen.insert(edge.class) {
                    simple = false;
                    break 'walk;
                }
                pending.push(edge.class);
            }
        }
        if graph.entity(class).complete {
            self.simple.borrow_mut().insert(class, simple);
        }
        simple
    }
    pub(super) fn get(&self, graph: &Graph, derived: EntityId, base: EntityId) -> Option<Fact> {
        if base == derived || self.simple(graph, derived) {
            return Some(Fact::one());
        }
        let mut bottom = derived;
        while let [edge] = graph.entity(bottom).class().bases.as_slice() {
            if edge.is_virtual {
                break;
            }
            bottom = edge.class;
        }
        if base == bottom {
            return Some(Fact::one());
        }
        if !graph.entity(bottom).complete {
            return None;
        }
        let mut cache = self.cache.borrow_mut();
        if !cache.tables.contains_key(&bottom) {
            let order = postorder(graph, bottom);
            graph.work.base_steps(order.len());
            // Match the C++ retained-fact budget. A miss is remembered so a
            // later query uses bounded per-query scratch rather than retrying.
            if cache.entries + order.len() > 1 << 18 {
                cache.tables.insert(bottom, None);
            } else {
                let mut facts = HashMap::from_iter([(bottom, Fact::one())]);
                for &class in order.iter().rev() {
                    let above = facts[&class];
                    for edge in &graph.entity(class).class().bases {
                        let below = facts.entry(edge.class).or_default();
                        below.receive(above, edge.is_virtual);
                    }
                }
                facts.remove(&bottom);
                cache.entries += facts.len();
                cache.tables.insert(bottom, Some(facts));
            }
        }
        let table = cache.tables.get(&bottom)?.as_ref()?;
        Some(table.get(&base).copied().unwrap_or_else(|| {
            if bottom == derived {
                Fact::default()
            } else {
                Fact::one()
            }
        }))
    }
}
fn postorder(graph: &Graph, root: EntityId) -> Vec<EntityId> {
    let mut order = Vec::new();
    let mut seen = HashSet::from_iter([root]);
    let mut pending = vec![(root, 0usize)];
    while let Some((class, next)) = pending.last_mut() {
        if let Some(edge) = graph.entity(*class).class().bases.get(*next) {
            *next += 1;
            if seen.insert(edge.class) {
                pending.push((edge.class, 0));
            }
        } else {
            order.push(*class);
            pending.pop();
        }
    }
    order
}
struct Frame {
    top: EntityId,
    bottom: EntityId,
    next: usize,
}
pub(super) fn leads_to_base(
    graph: &Graph,
    top: EntityId,
    base: EntityId,
    memo: &mut HashMap<EntityId, bool>,
) -> bool {
    if top == base || graph.entity(top).class().bases.is_empty() {
        return top == base;
    }
    if let Some(&known) = memo.get(&top) {
        return known;
    }
    let mut pending = Vec::<Frame>::new();
    let mut known = leads_chain(graph, top, base, memo, &mut pending);
    loop {
        if known == Some(true) {
            for frame in pending.drain(..) {
                memo.insert(frame.top, true);
                memo.insert(frame.bottom, true);
            }
            return true;
        }
        let Some(frame) = pending.last_mut() else {
            return known.unwrap_or(false);
        };
        if let Some(edge) = graph.entity(frame.bottom).class().bases.get(frame.next) {
            frame.next += 1;
            known = if edge.class == base || graph.entity(edge.class).class().bases.is_empty() {
                Some(edge.class == base)
            } else if let Some(&known) = memo.get(&edge.class) {
                Some(known)
            } else {
                leads_chain(graph, edge.class, base, memo, &mut pending)
            };
        } else {
            memo.insert(frame.top, false);
            memo.insert(frame.bottom, false);
            pending.pop();
            known = Some(false);
        }
    }
}
fn leads_chain(
    graph: &Graph,
    top: EntityId,
    base: EntityId,
    memo: &mut HashMap<EntityId, bool>,
    pending: &mut Vec<Frame>,
) -> Option<bool> {
    let mut class = top;
    while class != base {
        let [edge] = graph.entity(class).class().bases.as_slice() else {
            break;
        };
        graph.work.base_steps(1);
        class = edge.class;
    }
    if class == base || graph.entity(class).class().bases.is_empty() {
        let result = class == base;
        memo.insert(top, result);
        return Some(result);
    }
    if class != top
        && let Some(&known) = memo.get(&class)
    {
        memo.insert(top, known);
        return Some(known);
    }
    pending.push(Frame {
        top,
        bottom: class,
        next: 0,
    });
    None
}
