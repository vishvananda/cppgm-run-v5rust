// Translated from abi/itanium/abi_graph.cpp's tag queries. See NOTICE.
//! Bounded memoized sets keep shared type DAGs linear; large sets use a query walk.
use super::graph::{Graph, Modifier, Node, NodeId};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Default)]
enum Set {
    #[default]
    Unknown,
    Small {
        start: usize,
        count: usize,
    },
    Large,
}

#[derive(Default)]
pub(super) struct Tags {
    underived: HashMap<NodeId, NodeId>,
    sets: Vec<[Set; 2]>,
    items: Vec<NodeId>,
}
impl Tags {
    pub(super) fn bytes_reserved(&self) -> usize {
        self.underived.capacity() * size_of::<(NodeId, NodeId)>()
            + self.sets.capacity() * size_of::<[Set; 2]>()
            + self.items.capacity() * size_of::<NodeId>()
    }
}

fn extend_small(set: &mut Vec<NodeId>, tags: &[NodeId], limit: usize) {
    for &tag in tags {
        if !set.contains(&tag) && set.len() <= limit {
            set.push(tag);
        }
    }
}

impl Graph {
    pub(crate) fn note_implicit_tags(&mut self, derived: NodeId, underived: NodeId) {
        self.tags.underived.entry(derived).or_insert(underived);
    }

    fn tag_node(&self, node: NodeId) -> NodeId {
        self.tags.underived.get(&node).copied().unwrap_or(node)
    }

    /// Collect every tag, except that a local scope contributes only tags
    /// actually written in its enclosing function's underived encoding.
    pub(crate) fn collect_tags(&mut self, root: NodeId) -> Vec<NodeId> {
        const LIMIT: usize = 8;
        self.tags.sets.resize(self.len(), [Set::Unknown; 2]);
        let root = self.tag_node(root);
        let mut work = vec![(root, false, false)];
        let mut children = Vec::new();
        let mut small = Vec::with_capacity(LIMIT + 1);
        while let Some((id, written, finish)) = work.pop() {
            let mode = usize::from(written);
            if !matches!(self.tags.sets[id.index()][mode], Set::Unknown) {
                continue;
            }
            children.clear();
            let own = self.tag_components(id, written, &mut children);
            let below = written || matches!(self.get(id), Node::FunctionScope { .. });
            if !finish {
                work.push((id, written, true));
                for &child in &children {
                    let child = self.tag_node(child);
                    if matches!(
                        self.tags.sets[child.index()][usize::from(below)],
                        Set::Unknown
                    ) {
                        work.push((child, below, false));
                    }
                }
                continue;
            }
            small.clear();
            if own {
                small.extend(self.items(id).iter().take(LIMIT + 1));
            }
            let mut large = own && self.items(id).len() > LIMIT;
            let mut contributors = 0;
            let mut only = Set::Small { start: 0, count: 0 };
            for &child in &children {
                let set = self.tags.sets[self.tag_node(child).index()][usize::from(below)];
                match set {
                    Set::Large => large = true,
                    Set::Small { start, count } if count != 0 => {
                        only = set;
                        contributors += 1;
                        extend_small(&mut small, &self.tags.items[start..start + count], LIMIT);
                    }
                    _ => (),
                }
            }
            let set = if large || small.len() > LIMIT {
                Set::Large
            } else if !own && contributors == 1 {
                only
            } else {
                small.sort_unstable_by_key(|id| id.index());
                small.dedup();
                let start = self.tags.items.len();
                self.tags.items.extend_from_slice(&small);
                Set::Small {
                    start,
                    count: small.len(),
                }
            };
            self.tags.sets[id.index()][mode] = set;
        }

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut work = vec![(root, false)];
        while let Some((id, written)) = work.pop() {
            let id = self.tag_node(id);
            match self.tags.sets[id.index()][usize::from(written)] {
                Set::Small { start, count } => {
                    result.extend_from_slice(&self.tags.items[start..start + count]);
                }
                Set::Large if visited.insert((id, written)) => {
                    children.clear();
                    if self.tag_components(id, written, &mut children) {
                        result.extend_from_slice(self.items(id));
                    }
                    let below = written || matches!(self.get(id), Node::FunctionScope { .. });
                    work.extend(children.iter().map(|&child| (child, below)));
                }
                _ => (),
            }
        }
        result.sort_unstable_by_key(|id| id.index());
        result.dedup();
        result
    }

    fn tag_components(&self, id: NodeId, written: bool, children: &mut Vec<NodeId>) -> bool {
        let list = match self.get(id) {
            Node::Name {
                parent,
                terminal,
                hidden_tags,
                ..
            } => {
                children.extend(parent);
                children.push(terminal);
                return !written || !hidden_tags;
            }
            Node::Abbreviation(_) => return true,
            Node::FunctionScope { encoding, .. } => {
                children.push(encoding);
                false
            }
            Node::Terminal { inherited, .. } => {
                children.extend(inherited);
                false
            }
            Node::Modified { modifier, operand } => {
                children.push(operand);
                if let Modifier::ExpressionArray(expression) = modifier {
                    children.push(expression);
                }
                false
            }
            Node::Conversion(node)
            | Node::Decltype {
                expression: node, ..
            }
            | Node::Literal { ty: node, .. }
            | Node::UnsignedLiteral { ty: node, .. }
            | Node::NullptrLiteral(node)
            | Node::FloatingLiteral { ty: node, .. }
            | Node::ExpressionArgument(node)
            | Node::EntityArgument { entity: node, .. }
            | Node::EntityReference(node)
            | Node::AlignofType(node)
            | Node::TypeidType(node)
            | Node::SizeofType(node) => {
                children.push(node);
                false
            }
            Node::MemberPointer { owner, member }
            | Node::Cast {
                ty: owner,
                operand: member,
                ..
            } => {
                children.extend([owner, member]);
                false
            }
            Node::FunctionType { result, owner, .. } => {
                children.push(result);
                children.extend(owner);
                true
            }
            Node::Function { name, result, .. } => {
                children.push(name);
                children.extend(result);
                true
            }
            Node::Template(node)
            | Node::VendorType(node)
            | Node::NonTypeParameter(node)
            | Node::ParameterPack(node)
            | Node::UnresolvedName(node)
            | Node::VendorExpression(node)
            | Node::ConversionList(node) => {
                children.push(node);
                true
            }
            Node::TypedArgument {
                declaration,
                argument,
            } => {
                children.extend([declaration, argument]);
                true
            }
            Node::UnresolvedPrefix { root, .. } | Node::BracedList(root) => {
                children.extend(root);
                true
            }
            Node::ScopedMember { owner, .. } => {
                children.push(owner);
                true
            }
            Node::MemberAccess { object, .. } => {
                children.push(object);
                true
            }
            Node::Allocation {
                ty, initializer, ..
            } => {
                children.push(ty);
                children.extend(initializer);
                true
            }
            Node::ClosureName { .. }
            | Node::TemplateParameter
            | Node::ArgumentPack
            | Node::Operation(_)
            | Node::Call
            | Node::Deallocation { .. }
            | Node::ParenthesizedList
            | Node::ParenthesizedCall
            | Node::PrefixStep { .. } => true,
            _ => false,
        };
        if list {
            children.extend_from_slice(self.items(id));
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn name(graph: &mut Graph, parent: Option<NodeId>, text: &[u8], tags: &[NodeId]) -> NodeId {
        let text = graph.string(text).unwrap();
        graph.name(parent, text, tags).unwrap()
    }

    #[test]
    fn shared_dags_memoize_small_sets_and_bound_large_set_storage() {
        let mut graph = Graph::default();
        let mut names = Vec::new();
        let mut tags = Vec::new();
        for i in 0..12 {
            let tag = graph.string(format!("tag{i}").as_bytes()).unwrap();
            tags.push(tag);
            names.push(name(&mut graph, None, format!("T{i}").as_bytes(), &[tag]));
        }
        let template = name(&mut graph, None, b"Pair", &[]);
        let small = graph.intern(Node::Template(template), &names[..2]).unwrap();
        let large = graph.intern(Node::Template(template), &names).unwrap();
        let mut roots = [small, large];
        for root in &mut roots {
            for _ in 0..1024 {
                *root = graph
                    .intern(Node::Template(template), &[*root, *root])
                    .unwrap();
            }
        }
        assert_eq!(graph.collect_tags(roots[0]), tags[..2]);
        let small_storage = graph.tags.items.len();
        assert!(small_storage < 3 * graph.len());
        assert_eq!(graph.collect_tags(roots[1]), tags);
        let stored = graph.tags.items.len();
        assert!(stored <= small_storage + tags.len());
        assert_eq!(graph.collect_tags(roots[1]), tags);
        assert_eq!(graph.tags.items.len(), stored);
    }

    #[test]
    fn local_scopes_collect_written_tags_and_underived_function_names() {
        let mut graph = Graph::default();
        let hidden = graph.string(b"hidden").unwrap();
        let written = graph.string(b"written").unwrap();
        let ns = name(&mut graph, None, b"ns", &[hidden]);
        let Node::Name {
            parent,
            terminal,
            internal,
            data_member,
            occurrence,
            ..
        } = graph.get(ns)
        else {
            panic!("namespace is a name");
        };
        let ns = graph
            .intern(
                Node::Name {
                    parent,
                    terminal,
                    internal,
                    data_member,
                    occurrence,
                    hidden_tags: true,
                },
                &[hidden],
            )
            .unwrap();
        let function = name(&mut graph, Some(ns), b"f", &[written]);
        let derived = graph.tagged(function, &[hidden]).unwrap();
        graph.note_implicit_tags(derived, function);
        let scope = graph
            .intern(
                Node::FunctionScope {
                    encoding: derived,
                    default_argument: 0,
                },
                &[],
            )
            .unwrap();
        let local = name(&mut graph, Some(scope), b"Local", &[]);
        assert_eq!(graph.collect_tags(function), [hidden, written]);
        assert_eq!(graph.collect_tags(local), [written]);
    }
}
