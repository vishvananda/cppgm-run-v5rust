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
    TypeTemplate,
    FunctionTemplate,
    Value,
    Namespace,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Binding {
    pub category: Category,
    pub parameter: bool,
    pub scope: Option<ScopeId>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeKind {
    Template,
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
    member_of: Option<ScopeId>,
    depth: u32,
    edges: Vec<Edge>,
}
#[derive(Clone, Copy)]
pub(super) struct Edge {
    target: ScopeId,
    at: Option<ScopeId>,
    dependent: bool,
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
#[derive(Clone, Copy)]
pub(super) enum Change {
    Unnamed(ScopeId, Option<ScopeId>),
    Binding((ScopeId, NameId), Option<Binding>),
    Edge(ScopeId, Option<Edge>),
    Complete(ScopeId, bool),
    Member(ScopeId, Option<ScopeId>),
    Parent(ScopeId, Option<ScopeId>, u32),
}
#[derive(Clone, Copy)]
pub(super) struct ScopeMark {
    scopes: usize,
    changes: usize,
    current: ScopeId,
    environment: u64,
}
pub(super) struct Scopes {
    records: Vec<Scope>,
    bindings: HashMap<(ScopeId, NameId), Binding>,
    unnamed: HashMap<ScopeId, ScopeId>,
    changes: Vec<Change>,
    pub current: ScopeId,
    environment: u64,
    epoch: u64,
    visited: HashSet<ScopeId>,
    pending: BinaryHeap<Pending>,
    edge_targets: HashSet<(ScopeId, ScopeId, bool)>,
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
                member_of: None,
                depth: 0,
                edges: Vec::new(),
            }],
            bindings: HashMap::new(),
            unnamed: HashMap::new(),
            changes: Vec::new(),
            current: ScopeId(0),
            environment: 0,
            epoch: 0,
            visited: HashSet::new(),
            pending: BinaryHeap::new(),
            edge_targets: HashSet::new(),
            name_marks: Vec::new(),
            search_work: Vec::new(),
        }
    }
}
impl Scopes {
    pub(super) fn environment(&self) -> u64 {
        self.environment
    }
    fn changed(&mut self) {
        // A transaction restores its environment; new mutations receive a fresh
        // identity, so recycled scope slots cannot reuse failed parse queries.
        self.epoch = self.epoch.wrapping_add(1);
        self.environment = self.epoch;
    }
    pub(super) fn global() -> ScopeId {
        ScopeId(0)
    }
    pub(super) fn kind(&self, scope: ScopeId) -> ScopeKind {
        self.records[scope.0 as usize].kind
    }
    pub(super) fn set_member(&mut self, scope: ScopeId, member: ScopeId) {
        let old = self.records[scope.0 as usize].member_of;
        self.changes.push(Change::Member(scope, old));
        self.records[scope.0 as usize].member_of = Some(member);
        self.changed();
    }
    pub(super) fn member_context(&mut self, member: ScopeId) -> Result<ScopeId, ParseError> {
        let scope = self.push()?;
        self.set_member(scope, member);
        Ok(scope)
    }
    pub(super) fn complete(&self, scope: ScopeId) -> bool {
        self.records[scope.0 as usize].complete
    }
    pub(super) fn complete_class(&mut self, scope: ScopeId) {
        let old = self.records[scope.0 as usize].complete;
        self.changes.push(Change::Complete(scope, old));
        self.records[scope.0 as usize].complete = true;
        self.changed();
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
        self.changed();
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
            member_of: None,
            depth: self.records[self.current.0 as usize]
                .depth
                .checked_add(1)
                .ok_or(ParseError::Limit("parser scope depth limit"))?,
            edges: Vec::new(),
        });
        self.current = id;
        self.changed();
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
    pub(super) fn declaration_scope(&self) -> ScopeId {
        let mut scope = self.current;
        while self.kind(scope) == ScopeKind::Template {
            let Some(parent) = self.records[scope.0 as usize].parent else {
                break;
            };
            scope = parent;
        }
        scope
    }
    pub(super) fn ever_template(&self, name: NameId) -> bool {
        self.name_marks
            .get(name.index())
            .is_some_and(|marks| marks & 4 != 0)
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
        if binding.category == Category::TypeTemplate
            || (binding.category == Category::FunctionTemplate
                && matches!(
                    self.kind(scope),
                    ScopeKind::Class(_) | ScopeKind::AnonymousClass | ScopeKind::Enum
                ))
        {
            self.name_marks[name.index()] |= 4;
        }
        let key = (scope, name);
        let binding = self.own(scope, name).map_or(binding, |previous| {
            if previous.category == Category::FunctionTemplate
                && binding.category == Category::Value
            {
                previous
            } else if (binding.category == Category::Value
                && matches!(previous.category, Category::Type | Category::TypeTemplate))
                || (previous.category == Category::Value
                    && matches!(binding.category, Category::Type | Category::TypeTemplate)
                    && binding.scope.is_some())
            {
                Binding {
                    parameter: false,
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
        self.changed();
    }
    pub(super) fn add_edge(&mut self, scope: ScopeId) {
        self.add_edge_at(scope, None, false);
    }
    pub(super) fn add_base(&mut self, scope: ScopeId, dependent: bool) {
        self.add_edge_at(scope, None, dependent);
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
        self.add_edge_at(nominated, Some(at), false);
    }
    fn add_edge_at(&mut self, scope: ScopeId, at: Option<ScopeId>, dependent: bool) {
        let current = self.current;
        if dependent && self.edge_targets.contains(&(current, scope, false)) {
            return;
        }
        if self.edge_targets.insert((current, scope, dependent)) {
            self.records[current.0 as usize].edges.push(Edge {
                target: scope,
                at,
                dependent,
            });
            self.changes.push(Change::Edge(current, None));
            self.changed();
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
        let mut home: Option<ScopeId> = None;
        while let Some(scope) = current {
            if self.kind(scope) == ScopeKind::Namespace
                && let Some(inner) = home.take()
                && let Some(binding) = self.search_home(inner, scope, name, prefix_only)
            {
                return Some(binding);
            }
            if let Some(binding) = self.search_level(scope, name, prefix_only) {
                return Some(binding);
            }
            if let Some(member) = self.records[scope.0 as usize].member_of {
                let mut member = member;
                loop {
                    if self.kind(member) == ScopeKind::Namespace {
                        home = Some(member);
                        break;
                    }
                    if self.kind(member) != ScopeKind::Template
                        && let Some(binding) = self.search(member, name, prefix_only)
                    {
                        return Some(binding);
                    }
                    let next = self.records[member.0 as usize]
                        .member_of
                        .or(self.records[member.0 as usize].parent);
                    let Some(next) = next else {
                        break;
                    };
                    member = next;
                }
            }
            current = self.records[scope.0 as usize].parent;
        }
        None
    }
    fn search_home(
        &mut self,
        mut inner: ScopeId,
        scope: ScopeId,
        name: NameId,
        prefix_only: bool,
    ) -> Option<Binding> {
        let mut outer = inner;
        while self.records[outer.0 as usize].depth > self.records[scope.0 as usize].depth {
            outer = self.records[outer.0 as usize]
                .parent
                .unwrap_or(Self::global());
        }
        if outer != scope {
            return None;
        }
        while inner != scope {
            if let Some(binding) = self.search_level(inner, name, prefix_only) {
                return Some(binding);
            }
            inner = self.records[inner.0 as usize]
                .parent
                .unwrap_or(Self::global());
        }
        None
    }
    pub(super) fn lookup_specialization(
        &mut self,
        scope: ScopeId,
        name: NameId,
        prefix_only: bool,
        specialization: bool,
    ) -> Option<Binding> {
        if self
            .name_marks
            .get(name.index())
            .is_none_or(|marks| marks & 1 == 0)
        {
            return None;
        }
        self.visited.clear();
        self.search_with_bases(scope, name, prefix_only, specialization)
    }
    fn found(binding: Binding, prefix_only: bool) -> bool {
        !prefix_only
            || matches!(
                binding.category,
                Category::Type | Category::TypeTemplate | Category::Namespace
            )
            || binding.scope.is_some()
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
        self.search_with_bases(scope, name, prefix_only, false)
    }
    fn search_with_bases(
        &mut self,
        scope: ScopeId,
        name: NameId,
        prefix_only: bool,
        specialization: bool,
    ) -> Option<Binding> {
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
                    .filter(|edge| !edge.dependent || specialization)
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
            environment: self.environment,
        }
    }
    // Swap a logged value, returning the inverse. Deferred bodies temporarily
    // detach only changes made after their source position, never whole scopes.
    fn exchange(&mut self, change: Change) -> Change {
        match change {
            Change::Unnamed(parent, value) => {
                let old = if let Some(value) = value {
                    self.unnamed.insert(parent, value)
                } else {
                    self.unnamed.remove(&parent)
                };
                Change::Unnamed(parent, old)
            }
            Change::Binding(key, value) => {
                let old = if let Some(value) = value {
                    self.bindings.insert(key, value)
                } else {
                    self.bindings.remove(&key)
                };
                Change::Binding(key, old)
            }
            Change::Member(scope, value) => Change::Member(
                scope,
                std::mem::replace(&mut self.records[scope.0 as usize].member_of, value),
            ),
            Change::Complete(scope, value) => Change::Complete(
                scope,
                std::mem::replace(&mut self.records[scope.0 as usize].complete, value),
            ),
            Change::Parent(scope, parent, depth) => {
                let record = &mut self.records[scope.0 as usize];
                Change::Parent(
                    scope,
                    std::mem::replace(&mut record.parent, parent),
                    std::mem::replace(&mut record.depth, depth),
                )
            }
            Change::Edge(scope, Some(edge)) => {
                self.records[scope.0 as usize].edges.push(edge);
                self.edge_targets
                    .insert((scope, edge.target, edge.dependent));
                Change::Edge(scope, None)
            }
            Change::Edge(scope, None) => {
                let edge = self.records[scope.0 as usize].edges.pop();
                if let Some(edge) = edge {
                    self.edge_targets
                        .remove(&(scope, edge.target, edge.dependent));
                }
                Change::Edge(scope, edge)
            }
        }
    }
    pub(super) fn log_position(&self) -> usize {
        self.changes.len()
    }
    pub(super) fn detach(&mut self, position: usize, later: &mut Vec<Change>) {
        later.clear();
        while self.changes.len() > position {
            if let Some(change) = self.changes.pop() {
                later.push(self.exchange(change));
            }
        }
        later.reverse();
        self.changed();
    }
    pub(super) fn reattach(&mut self, later: &[Change]) {
        for &change in later {
            let inverse = self.exchange(change);
            self.changes.push(inverse);
        }
        self.changed();
    }
    pub(super) fn rewind(&mut self, mark: ScopeMark) {
        while self.changes.len() > mark.changes {
            if let Some(change) = self.changes.pop() {
                self.exchange(change);
            }
        }
        self.records.truncate(mark.scopes);
        self.current = mark.current;
        self.environment = mark.environment;
    }
    pub(super) fn settle(&mut self) {
        self.changes.clear();
    }
}
