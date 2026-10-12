// Translated from syntax/name_scopes.{h,cpp}. See NOTICE.
//! Parsing categories only; indexed bindings and reversible mutations.
use super::ParseError;
use crate::lex::identifiers::NameId;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ScopeId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category {
    Type,
    Value,
    Namespace,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Binding {
    pub category: Category,
    pub scope: Option<ScopeId>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeKind {
    Ordinary,
    Namespace,
    Block,
    Class(NameId),
    AnonymousClass,
    Enum,
}
struct Scope {
    kind: ScopeKind,
    complete: bool,
    parent: Option<ScopeId>,
    depth: u32,
    edges: Vec<Edge>,
}
#[derive(Clone, Copy)]
struct Edge {
    target: ScopeId,
    at: Option<ScopeId>,
}
#[derive(Clone, Copy)]
struct Pending {
    target: ScopeId,
    depth: u32,
}
impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.depth == other.depth
    }
}
impl Eq for Pending {}
impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Pending {
    fn cmp(&self, other: &Self) -> Ordering {
        self.depth.cmp(&other.depth)
    }
}
enum Change {
    Unnamed(ScopeId, Option<ScopeId>),
    Binding((ScopeId, NameId), Option<Binding>),
    Edge(ScopeId),
    Complete(ScopeId, bool),
    Parent(ScopeId, Option<ScopeId>, u32),
}
#[derive(Clone, Copy)]
pub(super) struct ScopeMark {
    scopes: usize,
    changes: usize,
    current: ScopeId,
}
pub(super) struct Scopes {
    records: Vec<Scope>,
    bindings: HashMap<(ScopeId, NameId), Binding>,
    unnamed: HashMap<ScopeId, ScopeId>,
    changes: Vec<Change>,
    pub current: ScopeId,
    visited: HashSet<ScopeId>,
    pending: BinaryHeap<Pending>,
    edge_targets: HashSet<(ScopeId, ScopeId)>,
    name_marks: Vec<u8>,
    search_work: Vec<ScopeId>,
}
impl Default for Scopes {
    fn default() -> Self {
        Self {
            records: vec![Scope {
                kind: ScopeKind::Namespace,
                complete: false,
                parent: None,
                depth: 0,
                edges: Vec::new(),
            }],
            bindings: HashMap::new(),
            unnamed: HashMap::new(),
            changes: Vec::new(),
            current: ScopeId(0),
            visited: HashSet::new(),
            pending: BinaryHeap::new(),
            edge_targets: HashSet::new(),
            name_marks: Vec::new(),
            search_work: Vec::new(),
        }
    }
}
impl Scopes {
    pub(super) fn global() -> ScopeId {
        ScopeId(0)
    }
    pub(super) fn kind(&self, scope: ScopeId) -> ScopeKind {
        self.records[scope.0 as usize].kind
    }
    pub(super) fn complete(&self, scope: ScopeId) -> bool {
        self.records[scope.0 as usize].complete
    }
    pub(super) fn complete_class(&mut self, scope: ScopeId) {
        let old = self.records[scope.0 as usize].complete;
        self.changes.push(Change::Complete(scope, old));
        self.records[scope.0 as usize].complete = true;
    }
    pub(super) fn reopen(&mut self, scope: ScopeId, parent: ScopeId) -> Result<(), ParseError> {
        let depth = self.records[parent.0 as usize]
            .depth
            .checked_add(1)
            .ok_or(ParseError::Limit("parser scope depth limit"))?;
        let record = &mut self.records[scope.0 as usize];
        self.changes
            .push(Change::Parent(scope, record.parent, record.depth));
        record.parent = Some(parent);
        record.depth = depth;
        Ok(())
    }
    pub(super) fn class_home(&self, mut home: ScopeId, name: NameId) -> ScopeId {
        let original = home;
        // A qualified definition can complete a forward declaration in an
        // owned inline namespace. Using-directives do not own declarations.
        while self.kind(home) == ScopeKind::Namespace {
            if self.own(home, name).is_some_and(|binding| {
                binding.category == Category::Type
                    && binding.scope.is_none_or(|scope| !self.complete(scope))
            }) {
                return home;
            }
            let inner = self.records[home.0 as usize]
                .edges
                .iter()
                .rev()
                .find(|edge| {
                    edge.at.is_none()
                        && self.kind(edge.target) == ScopeKind::Namespace
                        && self.unnamed(home) != Some(edge.target)
                        && self.own(edge.target, name).is_some()
                });
            let Some(inner) = inner else {
                break;
            };
            home = inner.target;
        }
        original
    }
    pub(super) fn push(&mut self) -> Result<ScopeId, ParseError> {
        self.push_kind(ScopeKind::Ordinary)
    }
    pub(super) fn push_kind(&mut self, kind: ScopeKind) -> Result<ScopeId, ParseError> {
        let id = ScopeId(
            u32::try_from(self.records.len())
                .map_err(|_| ParseError::Limit("parser scope limit"))?,
        );
        self.records.push(Scope {
            kind,
            complete: false,
            parent: Some(self.current),
            depth: self.records[self.current.0 as usize]
                .depth
                .checked_add(1)
                .ok_or(ParseError::Limit("parser scope depth limit"))?,
            edges: Vec::new(),
        });
        self.current = id;
        Ok(id)
    }
    pub(super) fn unnamed(&self, scope: ScopeId) -> Option<ScopeId> {
        self.unnamed.get(&scope).copied()
    }
    pub(super) fn set_unnamed(&mut self, parent: ScopeId, scope: ScopeId) {
        let old = self.unnamed.insert(parent, scope);
        self.changes.push(Change::Unnamed(parent, old));
    }
    pub(super) fn enclosing_declaration_scope(&self) -> ScopeId {
        let mut scope = self.current;
        while !matches!(self.kind(scope), ScopeKind::Namespace | ScopeKind::Block) {
            let Some(parent) = self.records[scope.0 as usize].parent else {
                break;
            };
            scope = parent;
        }
        scope
    }
    pub(super) fn own(&self, scope: ScopeId, name: NameId) -> Option<Binding> {
        self.bindings.get(&(scope, name)).copied()
    }
    pub(super) fn declare(&mut self, name: NameId, binding: Binding) {
        self.declare_in(self.current, name, binding);
    }
    pub(super) fn declare_in(&mut self, scope: ScopeId, name: NameId, binding: Binding) {
        // Like the oracle's category-search hints, these bits survive rollback:
        // they can cause a search, but cannot supply an abandoned binding.
        if name.index() >= self.name_marks.len() {
            self.name_marks.resize(name.index() + 1, 0);
        }
        self.name_marks[name.index()] |= if matches!(
            self.kind(scope),
            ScopeKind::Class(_) | ScopeKind::AnonymousClass | ScopeKind::Enum
        ) {
            3
        } else {
            1
        };
        let key = (scope, name);
        let binding = self.own(scope, name).map_or(binding, |previous| {
            if (binding.category == Category::Value && previous.category == Category::Type)
                || (previous.category == Category::Value
                    && binding.category == Category::Type
                    && binding.scope.is_some())
            {
                Binding {
                    category: Category::Value,
                    scope: binding.scope.or(previous.scope),
                }
            } else if binding.category == previous.category && binding.scope.is_none() {
                previous
            } else {
                binding
            }
        });
        let old = self.bindings.insert(key, binding);
        self.changes.push(Change::Binding(key, old));
    }
    pub(super) fn add_edge(&mut self, scope: ScopeId) {
        self.add_edge_at(scope, None);
    }
    pub(super) fn add_using_directive(&mut self, nominated: ScopeId) {
        let mut at = self.current;
        let mut other = nominated;
        while self.records[at.0 as usize].depth > self.records[other.0 as usize].depth {
            at = self.records[at.0 as usize].parent.unwrap_or(Self::global());
        }
        while self.records[other.0 as usize].depth > self.records[at.0 as usize].depth {
            other = self.records[other.0 as usize]
                .parent
                .unwrap_or(Self::global());
        }
        while at != other {
            at = self.records[at.0 as usize].parent.unwrap_or(Self::global());
            other = self.records[other.0 as usize]
                .parent
                .unwrap_or(Self::global());
        }
        while self.kind(at) != ScopeKind::Namespace {
            at = self.records[at.0 as usize].parent.unwrap_or(Self::global());
        }
        self.add_edge_at(nominated, Some(at));
    }
    fn add_edge_at(&mut self, scope: ScopeId, at: Option<ScopeId>) {
        let current = self.current;
        if self.edge_targets.insert((current, scope)) {
            self.records[current.0 as usize]
                .edges
                .push(Edge { target: scope, at });
            self.changes.push(Change::Edge(current));
        }
    }
    pub(super) fn lookup(&mut self, name: NameId) -> Option<Binding> {
        self.lookup_name(name, false)
    }
    pub(super) fn lookup_name(&mut self, name: NameId, prefix_only: bool) -> Option<Binding> {
        if self
            .name_marks
            .get(name.index())
            .is_none_or(|marks| marks & 1 == 0)
        {
            return None;
        }
        self.visited.clear();
        self.pending.clear();
        let mut current = Some(self.current);
        while let Some(scope) = current {
            if let Some(binding) = self.search_level(scope, name, prefix_only) {
                return Some(binding);
            }
            current = self.records[scope.0 as usize].parent;
        }
        None
    }
    pub(super) fn lookup_in_name(
        &mut self,
        scope: ScopeId,
        name: NameId,
        prefix_only: bool,
    ) -> Option<Binding> {
        if self
            .name_marks
            .get(name.index())
            .is_none_or(|marks| marks & 1 == 0)
        {
            return None;
        }
        self.visited.clear();
        self.search(scope, name, prefix_only)
    }
    fn found(binding: Binding, prefix_only: bool) -> bool {
        !prefix_only || binding.category != Category::Value || binding.scope.is_some()
    }
    fn search_level(&mut self, scope: ScopeId, name: NameId, prefix_only: bool) -> Option<Binding> {
        if matches!(
            self.kind(scope),
            ScopeKind::Class(_) | ScopeKind::AnonymousClass
        ) {
            if let Some(binding) = self.search(scope, name, prefix_only) {
                return Some(binding);
            }
        } else if self.visited.insert(scope) {
            if let Some(binding) = self
                .own(scope, name)
                .filter(|binding| Self::found(*binding, prefix_only))
            {
                return Some(binding);
            }
            for index in (0..self.records[scope.0 as usize].edges.len()).rev() {
                let edge = self.records[scope.0 as usize].edges[index];
                if let Some(at) = edge.at {
                    self.pending.push(Pending {
                        target: edge.target,
                        depth: self.records[at.0 as usize].depth,
                    });
                } else if let Some(binding) = self.search(edge.target, name, prefix_only) {
                    return Some(binding);
                }
            }
        }
        while self
            .pending
            .peek()
            .is_some_and(|edge| edge.depth == self.records[scope.0 as usize].depth)
        {
            if let Some(edge) = self.pending.pop()
                && let Some(binding) = self.search(edge.target, name, prefix_only)
            {
                return Some(binding);
            }
        }
        None
    }
    fn search(&mut self, scope: ScopeId, name: NameId, prefix_only: bool) -> Option<Binding> {
        self.search_work.clear();
        self.search_work.push(scope);
        while let Some(scope) = self.search_work.pop() {
            if !self.visited.insert(scope) {
                continue;
            }
            if let Some(binding) = self
                .own(scope, name)
                .filter(|binding| Self::found(*binding, prefix_only))
            {
                return Some(binding);
            }
            if matches!(
                self.kind(scope),
                ScopeKind::Class(_) | ScopeKind::AnonymousClass
            ) && self
                .name_marks
                .get(name.index())
                .is_none_or(|marks| marks & 2 == 0)
            {
                continue;
            }
            self.search_work.extend(
                self.records[scope.0 as usize]
                    .edges
                    .iter()
                    .map(|edge| edge.target),
            );
        }
        None
    }
    pub(super) fn mark(&self) -> ScopeMark {
        ScopeMark {
            scopes: self.records.len(),
            changes: self.changes.len(),
            current: self.current,
        }
    }
    pub(super) fn rewind(&mut self, mark: ScopeMark) {
        while self.changes.len() > mark.changes {
            match self.changes.pop() {
                Some(Change::Unnamed(parent, Some(old))) => {
                    self.unnamed.insert(parent, old);
                }
                Some(Change::Unnamed(parent, None)) => {
                    self.unnamed.remove(&parent);
                }
                Some(Change::Binding(key, Some(old))) => {
                    self.bindings.insert(key, old);
                }
                Some(Change::Binding(key, None)) => {
                    self.bindings.remove(&key);
                }
                Some(Change::Complete(scope, old)) => {
                    self.records[scope.0 as usize].complete = old;
                }
                Some(Change::Parent(scope, parent, depth)) => {
                    self.records[scope.0 as usize].parent = parent;
                    self.records[scope.0 as usize].depth = depth;
                }
                Some(Change::Edge(scope)) => {
                    if let Some(edge) = self.records[scope.0 as usize].edges.pop() {
                        self.edge_targets.remove(&(scope, edge.target));
                    }
                }
                None => break,
            }
        }
        self.records.truncate(mark.scopes);
        self.current = mark.current;
    }
    pub(super) fn settle(&mut self) {
        self.changes.clear();
    }
}
