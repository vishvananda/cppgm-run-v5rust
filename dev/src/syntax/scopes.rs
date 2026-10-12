// Translated from syntax/name_scopes.{h,cpp}. See NOTICE.
//! Parsing categories only; indexed bindings and reversible mutations.
use super::ParseError;
use crate::lex::identifiers::NameId;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ScopeId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Category {
    Value = 1,
    Type = 2,
    TypeTemplate = 3,
    FunctionTemplate = 4,
    Namespace = 5,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Categories(u8);
impl Categories {
    pub(super) fn one(category: Category) -> Self {
        Self(1 << category as u8)
    }
    pub(super) fn contains(self, category: Category) -> bool {
        self.0 & Self::one(category).0 != 0
    }
    pub(super) fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    pub(super) fn without(self, category: Category) -> Self {
        Self(self.0 & !Self::one(category).0)
    }
    pub(super) fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub(super) fn multiple(self) -> bool {
        self.0.count_ones() > 1
    }
    pub(super) fn has_type(self) -> bool {
        self.contains(Category::Type) || self.contains(Category::TypeTemplate)
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Guesses {
    pub approximate: bool,
    pub dependent: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Binding {
    pub pack: bool,
    pub category: Category,
    pub parameter: bool,
    pub scope: Option<ScopeId>,
    pub specialization: u8,
    pub alternatives: Categories,
    pub guesses: Guesses,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ScopeIdentity(u64);
#[derive(Clone, Copy, Debug)]
pub(super) struct ParameterBase {
    pub index: u32,
    pub pack: bool,
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
    identity: ScopeIdentity,
    specialization: bool,
    memo: bool,
    graph_revision: u64,
    kind: ScopeKind,
    complete: bool,
    parent: Option<ScopeId>,
    member_of: Option<ScopeId>,
    depth: u32,
    edges: Vec<Edge>,
    parameter_bases: Vec<ParameterBase>,
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
    Complete(ScopeId, bool, bool),
    Member(ScopeId, Option<ScopeId>),
    Parent(ScopeId, Option<ScopeId>, u32),
    ParameterBase(ScopeId, Option<ParameterBase>),
    Specialization(ScopeId, bool),
}
#[derive(Clone, Copy)]
pub(super) struct ScopeMark {
    scopes: usize,
    changes: usize,
    current: ScopeId,
    environment: u64,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SearchKey {
    scope: ScopeIdentity,
    name: NameId,
    prefix_only: bool,
}
#[derive(Clone, Copy)]
struct KeptSearch {
    result: Option<Binding>,
    name_revision: u64,
    graph_revision: u64,
}
enum SearchDecision {
    Finished(Option<Binding>),
    Edges,
}
struct SearchFrame {
    scope: ScopeId,
    remaining: usize,
}
pub(super) struct Scopes {
    records: Vec<Scope>,
    // Tombstones retain the oracle binding census; identities never recycle.
    bindings: HashMap<(ScopeIdentity, NameId), Option<Binding>>,
    // Each oracle query identity holds both base-demand contexts. The slot
    // choice is part of the complete key, without changing the census limit.
    searches: HashMap<SearchKey, [Option<KeptSearch>; 2]>,
    name_revisions: Vec<u64>,
    incoming: HashMap<ScopeId, HashSet<ScopeId>>,
    invalidation_work: Vec<ScopeId>,
    invalidation_seen: HashSet<ScopeId>,
    unnamed: HashMap<ScopeId, ScopeId>,
    changes: Vec<Change>,
    pub current: ScopeId,
    environment: u64,
    epoch: u64,
    visited: HashSet<ScopeId>,
    pending: BinaryHeap<Pending>,
    edge_targets: HashSet<(ScopeId, ScopeId, bool)>,
    name_marks: Vec<u8>,
    search_work: Vec<SearchFrame>,
    #[cfg(test)]
    search_visits: u64,
    next_identity: u64,
    categories: HashMap<(ScopeIdentity, NameId, NameId), Categories>,
    conflicts: bool,
    member_conflicts: bool,
    uncertain: bool,
}
impl Default for Scopes {
    fn default() -> Self {
        Self {
            records: vec![Scope {
                identity: ScopeIdentity(0),
                specialization: false,
                memo: false,
                graph_revision: 0,
                kind: ScopeKind::Namespace,
                complete: false,
                parent: None,
                member_of: None,
                depth: 0,
                edges: Vec::new(),
                parameter_bases: Vec::new(),
            }],
            bindings: HashMap::new(),
            searches: HashMap::new(),
            name_revisions: Vec::new(),
            incoming: HashMap::new(),
            invalidation_work: Vec::new(),
            invalidation_seen: HashSet::new(),
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
            #[cfg(test)]
            search_visits: 0,
            next_identity: 1,
            categories: HashMap::new(),
            conflicts: false,
            member_conflicts: false,
            uncertain: false,
        }
    }
}
impl Scopes {
    fn template_home(&self, scope: ScopeId) -> ScopeId {
        let mut home = self.records[scope.0 as usize]
            .parent
            .unwrap_or(Self::global());
        while self.kind(home) == ScopeKind::Template {
            home = self.records[home.0 as usize]
                .parent
                .unwrap_or(Self::global());
        }
        home
    }
    fn category_key(
        &self,
        scope: ScopeId,
        name: NameId,
    ) -> Option<(ScopeIdentity, NameId, NameId)> {
        let ScopeKind::Class(template) = self.kind(scope) else {
            return None;
        };
        Some((
            self.records[self.template_home(scope).0 as usize].identity,
            template,
            name,
        ))
    }
    fn mark_name(&mut self, name: NameId, bits: u8) {
        if name.index() >= self.name_marks.len() {
            self.name_marks.resize(name.index() + 1, 0);
        }
        self.name_marks[name.index()] |= bits;
        self.member_conflicts |= bits & 8 != 0;
    }
    fn record_member_category(&mut self, scope: ScopeId, name: NameId, category: Category) {
        let Some(key) = self.category_key(scope, name) else {
            return;
        };
        let primary = self
            .own(self.template_home(scope), key.1)
            .and_then(|binding| binding.scope);
        let initial = primary
            .and_then(|scope| self.own(scope, name))
            .map_or(Categories::default(), |binding| {
                Categories::one(binding.category)
            });
        let bits = self.categories.entry(key).or_insert(initial);
        let before = *bits;
        *bits = bits.union(Categories::one(category));
        if !before.multiple() && bits.multiple() {
            self.conflicts = true;
            self.mark_name(key.1, 8);
        }
    }
    fn specialization_alternatives(
        &mut self,
        scope: ScopeId,
        name: NameId,
        prefix_only: bool,
        found: Category,
    ) -> Categories {
        let Some(key) = self.category_key(scope, name) else {
            return Categories::default();
        };
        let marks = self.name_marks.get(key.1.index()).copied().unwrap_or(0);
        if !self.conflicts && marks & 32 == 0 {
            return Categories::default();
        }
        let bits = self.categories.get(&key).copied();
        let based = marks & 16 != 0;
        let primary = if bits.is_some() || based {
            self.own(scope, name)
        } else {
            None
        };
        if bits.is_none() && primary.is_none() {
            return Categories::default();
        }
        let mut all = bits.unwrap_or_default();
        if let Some(primary) = primary {
            all = all.union(Categories::one(primary.category));
        }
        if based {
            for index in (0..self.records[scope.0 as usize].edges.len()).rev() {
                let target = self.records[scope.0 as usize].edges[index].target;
                if self.records[target.0 as usize].specialization
                    && !self.records[target.0 as usize].edges.is_empty()
                {
                    self.visited.clear();
                    if let Some(binding) = self.search_with_bases(target, name, prefix_only, true) {
                        all = all.union(Categories::one(binding.category));
                    }
                }
            }
        }
        all.without(found)
    }
    pub(super) fn has_conflicts(&self) -> bool {
        self.conflicts
    }
    pub(super) fn has_category_conflicts(&self) -> bool {
        self.conflicts || self.uncertain
    }
    pub(super) fn uncertain(&self) -> bool {
        self.uncertain
    }
    pub(super) fn any_member_conflicts(&self) -> bool {
        self.member_conflicts
    }
    pub(super) fn member_conflicts(&self, name: NameId) -> bool {
        self.name_marks
            .get(name.index())
            .is_some_and(|bits| bits & 8 != 0)
    }
    pub(super) fn note_uncertain(&mut self) {
        self.uncertain = true;
    }
    pub(super) fn member_specialized(&self, scope: ScopeId, name: NameId) -> bool {
        self.category_key(scope, name)
            .is_some_and(|key| self.categories.contains_key(&key))
    }
    pub(super) fn member_categories_differ(&self, scope: ScopeId, name: NameId) -> bool {
        self.category_key(scope, name)
            .and_then(|key| self.categories.get(&key))
            .is_some_and(|bits| {
                self.own(scope, name)
                    .map_or(*bits, |binding| {
                        bits.union(Categories::one(binding.category))
                    })
                    .multiple()
            })
    }
    pub(super) fn specialization(&self, scope: ScopeId) -> bool {
        self.records[scope.0 as usize].specialization
    }
    pub(super) fn add_specialization(&mut self, primary: ScopeId, specialization: ScopeId) {
        let saved = self.current;
        self.current = primary;
        self.add_edge(specialization);
        self.current = saved;
        let old = self.records[specialization.0 as usize].specialization;
        self.changes
            .push(Change::Specialization(specialization, old));
        self.records[specialization.0 as usize].specialization = true;
    }
    pub(super) fn note_late_primary(&mut self, scope: ScopeId) {
        if let ScopeKind::Class(name) = self.kind(scope)
            && !self.records[scope.0 as usize].edges.is_empty()
        {
            self.mark_name(name, 32 | 8);
            self.note_uncertain();
        }
    }
    pub(super) fn note_specialization_base(&mut self, scope: ScopeId) {
        if self.specialization(scope)
            && let ScopeKind::Class(name) = self.kind(scope)
        {
            self.mark_name(name, 16 | 32 | 8);
            self.note_uncertain();
        }
    }
    pub(super) fn current_instantiation(&self, template: ScopeId) -> bool {
        let mut scope = Some(self.current);
        while let Some(current) = scope {
            if current == template {
                return true;
            }
            let mut member = self.records[current.0 as usize].member_of;
            while let Some(class) = member.filter(|scope| {
                matches!(
                    self.kind(*scope),
                    ScopeKind::Class(_) | ScopeKind::AnonymousClass
                )
            }) {
                if class == template {
                    return true;
                }
                member = self.records[class.0 as usize]
                    .member_of
                    .or(self.records[class.0 as usize].parent);
            }
            scope = self.records[current.0 as usize].parent;
        }
        false
    }
    pub(super) fn in_template_definition(&self) -> bool {
        let mut scope = Some(self.current);
        while let Some(current) = scope {
            if self.kind(current) == ScopeKind::Template {
                return true;
            }
            scope = self.records[current.0 as usize].parent;
        }
        false
    }
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
    pub(super) fn complete_class(&mut self, scope: ScopeId, templated: bool) {
        let record = &self.records[scope.0 as usize];
        let memo = !templated
            && record.edges.iter().all(|edge| {
                let target = &self.records[edge.target.0 as usize];
                matches!(target.kind, ScopeKind::Class(_) | ScopeKind::AnonymousClass)
                    && target.memo
                    && !edge.dependent
            });
        self.changes
            .push(Change::Complete(scope, record.complete, record.memo));
        self.records[scope.0 as usize].complete = true;
        self.records[scope.0 as usize].memo = memo;
        self.invalidate_class_graph(scope);
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
        if !self.class_forward(self.own(home, name), name) {
            // Specialization edges are prepended by C++; visiting every edge
            // and retaining the last forward selects the earliest unfinished
            // member. The primary's own forward always wins.
            let mut selected = None;
            for edge in self.records[home.0 as usize].edges.iter().rev() {
                if self.specialization(edge.target)
                    && self.class_forward(self.own(edge.target, name), name)
                {
                    selected = Some(edge.target);
                }
            }
            if let Some(selected) = selected {
                return selected;
            }
        }
        // A qualified definition can complete a forward declaration in an
        // owned inline namespace. Using-directives do not own declarations.
        while self.kind(home) == ScopeKind::Namespace {
            if self.class_forward(self.own(home, name), name) {
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
    fn class_forward(&self, binding: Option<Binding>, name: NameId) -> bool {
        binding.is_some_and(|binding| {
            matches!(binding.category, Category::Type | Category::TypeTemplate)
                && binding.scope.is_none_or(|scope| {
                    self.kind(scope) == ScopeKind::Class(name)
                        && !self.specialization(scope)
                        && self.own(scope, name).is_none()
                })
        })
    }
    pub(super) fn push(&mut self) -> Result<ScopeId, ParseError> {
        self.push_kind(ScopeKind::Ordinary)
    }
    pub(super) fn push_kind(&mut self, kind: ScopeKind) -> Result<ScopeId, ParseError> {
        let id = ScopeId(
            u32::try_from(self.records.len())
                .map_err(|_| ParseError::Limit("parser scope limit"))?,
        );
        let identity = ScopeIdentity(self.next_identity);
        self.next_identity = self
            .next_identity
            .checked_add(1)
            .ok_or(ParseError::Limit("parser scope identity limit"))?;
        self.records.push(Scope {
            identity,
            specialization: false,
            memo: false,
            graph_revision: 0,
            kind,
            complete: false,
            parent: Some(self.current),
            member_of: None,
            depth: self.records[self.current.0 as usize]
                .depth
                .checked_add(1)
                .ok_or(ParseError::Limit("parser scope depth limit"))?,
            edges: Vec::new(),
            parameter_bases: Vec::new(),
        });
        self.current = id;
        self.changed();
        Ok(id)
    }
    pub(super) fn unnamed(&self, scope: ScopeId) -> Option<ScopeId> {
        self.unnamed.get(&scope).copied()
    }
    pub(super) fn parameter_bases(&self, scope: ScopeId) -> &[ParameterBase] {
        &self.records[scope.0 as usize].parameter_bases
    }
    pub(super) fn add_parameter_base(&mut self, scope: ScopeId, base: ParameterBase) {
        let bases = &mut self.records[scope.0 as usize].parameter_bases;
        if bases
            .iter()
            .any(|old| old.index == base.index && old.pack == base.pack)
        {
            return;
        }
        bases.push(base);
        self.changes.push(Change::ParameterBase(scope, None));
        self.changed();
    }
    pub(super) fn pending_class(&self, scope: ScopeId) -> bool {
        matches!(self.kind(scope), ScopeKind::Class(name)
            if !self.specialization(scope) && self.records[scope.0 as usize].edges.is_empty() && self.own(scope, name).is_none())
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
        self.bindings
            .get(&(self.records[scope.0 as usize].identity, name))
            .copied()
            .flatten()
    }
    pub(super) fn declare(&mut self, name: NameId, binding: Binding) {
        self.declare_in(self.current, name, binding);
    }
    pub(super) fn declare_in(&mut self, scope: ScopeId, name: NameId, binding: Binding) {
        let category = binding.category;
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
                    pack: false,
                    parameter: false,
                    category: Category::Value,
                    scope: if binding.category == Category::Value {
                        previous.scope
                    } else {
                        binding.scope
                    },
                    specialization: 0,
                    alternatives: Categories::default(),
                    guesses: Guesses::default(),
                }
            } else if binding.category == previous.category && binding.scope.is_none() {
                previous
            } else {
                binding
            }
        });
        let old = self
            .bindings
            .insert(
                (self.records[scope.0 as usize].identity, name),
                Some(binding),
            )
            .flatten();
        self.changes.push(Change::Binding(key, old));
        if matches!(
            self.kind(scope),
            ScopeKind::Class(_) | ScopeKind::AnonymousClass | ScopeKind::Enum
        ) {
            self.touch_name(name);
        }
        self.changed();
        if self.records[scope.0 as usize].specialization {
            self.record_member_category(scope, name, category);
        }
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
            self.incoming.entry(scope).or_default().insert(current);
            self.invalidate_class_graph(current);
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
        let mut result = self.search_with_bases(scope, name, prefix_only, specialization)?;
        if specialization {
            result.alternatives =
                self.specialization_alternatives(scope, name, prefix_only, result.category);
            if !result.alternatives.is_empty() {
                result.specialization = 0;
            }
        }
        Some(result)
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
        if let SearchDecision::Finished(result) =
            self.begin_search(scope, name, prefix_only, specialization)
        {
            return result;
        }
        self.search_work.push(SearchFrame {
            scope,
            remaining: self.records[scope.0 as usize].edges.len(),
        });
        while let Some(frame) = self.search_work.last_mut() {
            if frame.remaining == 0 {
                let scope = frame.scope;
                self.search_work.pop();
                self.keep_search(scope, name, prefix_only, specialization, None);
                continue;
            }
            frame.remaining -= 1;
            let edge = self.records[frame.scope.0 as usize].edges[frame.remaining];
            if edge.dependent && !specialization {
                continue;
            }
            if let SearchDecision::Finished(result) =
                self.begin_search(edge.target, name, prefix_only, specialization)
            {
                if let Some(binding) = result {
                    while let Some(frame) = self.search_work.pop() {
                        self.keep_search(
                            frame.scope,
                            name,
                            prefix_only,
                            specialization,
                            Some(binding),
                        );
                    }
                    return Some(binding);
                }
            } else {
                self.search_work.push(SearchFrame {
                    scope: edge.target,
                    remaining: self.records[edge.target.0 as usize].edges.len(),
                });
            }
        }
        None
    }
    // A completed negative search differs from one still needing its edges.
    fn begin_search(
        &mut self,
        scope: ScopeId,
        name: NameId,
        prefix_only: bool,
        specialization: bool,
    ) -> SearchDecision {
        if !self.visited.insert(scope) {
            return SearchDecision::Finished(None);
        }
        #[cfg(test)]
        {
            self.search_visits += 1;
        }
        if let Some(binding) = self
            .own(scope, name)
            .filter(|binding| Self::found(*binding, prefix_only))
        {
            return SearchDecision::Finished(Some(binding));
        }
        let record = &self.records[scope.0 as usize];
        if matches!(record.kind, ScopeKind::Class(_) | ScopeKind::AnonymousClass)
            && self
                .name_marks
                .get(name.index())
                .is_none_or(|marks| marks & 2 == 0)
        {
            return SearchDecision::Finished(None);
        }
        let key = SearchKey {
            scope: record.identity,
            name,
            prefix_only,
        };
        self.searches
            .get(&key)
            .and_then(|kept| kept[usize::from(specialization)])
            .filter(|kept| {
                record.memo
                    && kept.graph_revision == record.graph_revision
                    && kept.name_revision == self.name_revision(name)
            })
            .map_or(SearchDecision::Edges, |kept| {
                SearchDecision::Finished(kept.result)
            })
    }
    fn name_revision(&self, name: NameId) -> u64 {
        self.name_revisions.get(name.index()).copied().unwrap_or(0)
    }
    fn touch_name(&mut self, name: NameId) {
        // Names without kept searches need no epoch storage.
        if self
            .name_marks
            .get(name.index())
            .is_none_or(|marks| marks & 64 == 0)
        {
            return;
        }
        if name.index() >= self.name_revisions.len() {
            self.name_revisions.resize(name.index() + 1, 0);
        }
        let revision = &mut self.name_revisions[name.index()];
        if let Some(next) = revision.checked_add(1) {
            *revision = next;
        } else {
            // Wrapping may not revive an old result. Only this name is cold.
            *revision = 0;
            self.searches.retain(|key, _| key.name != name);
        }
    }
    fn keep_search(
        &mut self,
        scope: ScopeId,
        name: NameId,
        prefix_only: bool,
        specialization: bool,
        result: Option<Binding>,
    ) {
        let record = &self.records[scope.0 as usize];
        if !record.memo {
            return;
        }
        let key = SearchKey {
            scope: record.identity,
            name,
            prefix_only,
        };
        // A full memo renews existing queries but introduces no new ones.
        let limit = self.bindings.len().saturating_mul(4).max(1 << 16);
        if self.searches.len() >= limit && !self.searches.contains_key(&key) {
            return;
        }
        let kept = KeptSearch {
            result,
            name_revision: self.name_revision(name),
            graph_revision: record.graph_revision,
        };
        self.searches.entry(key).or_insert([None; 2])[usize::from(specialization)] = Some(kept);
        self.mark_name(name, 64);
    }
    fn invalidate_class_graph(&mut self, scope: ScopeId) {
        // Reverse base edges invalidate this entity and its actual dependents,
        // rather than making unrelated translation-unit searches cold.
        self.invalidation_work.clear();
        self.invalidation_seen.clear();
        self.invalidation_work.push(scope);
        while let Some(scope) = self.invalidation_work.pop() {
            if !self.invalidation_seen.insert(scope) {
                continue;
            }
            let record = &mut self.records[scope.0 as usize];
            if let Some(next) = record.graph_revision.checked_add(1) {
                record.graph_revision = next;
            } else {
                record.graph_revision = 0;
                let identity = record.identity;
                self.searches.retain(|key, _| key.scope != identity);
            }
            if let Some(incoming) = self.incoming.get(&scope) {
                self.invalidation_work.extend(incoming);
            }
        }
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
            Change::Specialization(scope, value) => Change::Specialization(
                scope,
                std::mem::replace(&mut self.records[scope.0 as usize].specialization, value),
            ),
            Change::ParameterBase(scope, Some(base)) => {
                self.records[scope.0 as usize].parameter_bases.push(base);
                Change::ParameterBase(scope, None)
            }
            Change::ParameterBase(scope, None) => {
                Change::ParameterBase(scope, self.records[scope.0 as usize].parameter_bases.pop())
            }
            Change::Unnamed(parent, value) => {
                let old = if let Some(value) = value {
                    self.unnamed.insert(parent, value)
                } else {
                    self.unnamed.remove(&parent)
                };
                Change::Unnamed(parent, old)
            }
            Change::Binding(key, value) => {
                self.touch_name(key.1);
                let identity = self.records[key.0.0 as usize].identity;
                let old = self.bindings.insert((identity, key.1), value).flatten();
                Change::Binding(key, old)
            }
            Change::Member(scope, value) => Change::Member(
                scope,
                std::mem::replace(&mut self.records[scope.0 as usize].member_of, value),
            ),
            Change::Complete(scope, value, memo) => {
                self.invalidate_class_graph(scope);
                let record = &mut self.records[scope.0 as usize];
                Change::Complete(
                    scope,
                    std::mem::replace(&mut record.complete, value),
                    std::mem::replace(&mut record.memo, memo),
                )
            }
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
                self.incoming.entry(edge.target).or_default().insert(scope);
                self.invalidate_class_graph(scope);
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
                if let Some(edge) = edge
                    && !self.records[scope.0 as usize]
                        .edges
                        .iter()
                        .any(|remaining| remaining.target == edge.target)
                    && let Some(incoming) = self.incoming.get_mut(&edge.target)
                {
                    incoming.remove(&scope);
                }
                self.invalidate_class_graph(scope);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::identifiers::Identifiers;

    fn binding(category: Category, scope: Option<ScopeId>) -> Binding {
        Binding {
            category,
            scope,
            pack: false,
            parameter: false,
            specialization: 0,
            alternatives: Categories::default(),
            guesses: Guesses::default(),
        }
    }
    fn class(scopes: &mut Scopes, name: NameId, base: Option<ScopeId>) -> ScopeId {
        scopes.current = Scopes::global();
        let scope = scopes.push_kind(ScopeKind::Class(name)).unwrap();
        if let Some(base) = base {
            scopes.add_base(base, false);
        }
        scopes.complete_class(scope, false);
        scope
    }
    #[test]
    fn inherited_searches_scale_and_unrelated_mutations_keep_them_warm() {
        let mut names = Identifiers::default();
        let class_name = names.intern(b"C").unwrap();
        let word = names.intern(b"word").unwrap();
        let other = names.intern(b"other").unwrap();
        let mut scopes = Scopes::default();
        let base = class(&mut scopes, class_name, None);
        scopes.declare_in(base, word, binding(Category::Type, None));
        let mut last = base;
        for _ in 0..1024 {
            last = class(&mut scopes, class_name, Some(last));
            assert_eq!(
                scopes
                    .lookup_specialization(last, word, false, false)
                    .unwrap()
                    .category,
                Category::Type
            );
        }
        assert!(scopes.search_visits <= 2048);
        let before = scopes.search_visits;
        scopes.declare_in(base, other, binding(Category::Value, None));
        scopes.declare_in(Scopes::global(), word, binding(Category::Value, None));
        assert_eq!(
            scopes
                .lookup_specialization(last, word, false, false)
                .unwrap()
                .category,
            Category::Type
        );
        assert_eq!(scopes.search_visits - before, 1);
    }
    #[test]
    fn negative_prefix_and_specialization_queries_have_complete_keys() {
        let mut names = Identifiers::default();
        let class_name = names.intern(b"C").unwrap();
        let word = names.intern(b"word").unwrap();
        let mut scopes = Scopes::default();
        let empty = class(&mut scopes, class_name, None);
        let other = class(&mut scopes, class_name, None);
        scopes.declare_in(other, word, binding(Category::Type, Some(other)));
        let child = class(&mut scopes, class_name, Some(empty));
        let root = class(&mut scopes, class_name, Some(child));
        assert!(
            scopes
                .lookup_specialization(root, word, true, false)
                .is_none()
        );
        let before = scopes.search_visits;
        assert!(
            scopes
                .lookup_specialization(root, word, true, false)
                .is_none()
        );
        assert_eq!(scopes.search_visits - before, 1);
        scopes.declare_in(empty, word, binding(Category::Value, None));
        assert_eq!(
            scopes
                .lookup_specialization(root, word, false, false)
                .unwrap()
                .category,
            Category::Value
        );
        assert!(
            scopes
                .lookup_specialization(root, word, true, false)
                .is_none()
        );
        scopes.current = child;
        scopes.add_base(other, true);
        assert!(
            scopes
                .lookup_specialization(root, word, true, false)
                .is_none()
        );
        assert_eq!(
            scopes
                .lookup_specialization(root, word, true, true)
                .unwrap()
                .scope,
            Some(other)
        );
        assert!(
            scopes
                .lookup_specialization(root, word, true, false)
                .is_none()
        );
    }
    #[test]
    fn rewound_memos_never_return_discarded_scope_handles() {
        let mut names = Identifiers::default();
        let class_name = names.intern(b"C").unwrap();
        let word = names.intern(b"word").unwrap();
        let mut scopes = Scopes::default();
        let base = class(&mut scopes, class_name, None);
        scopes.declare_in(base, word, binding(Category::Type, Some(base)));
        let root = class(&mut scopes, class_name, Some(base));
        assert_eq!(
            scopes
                .lookup_specialization(root, word, false, false)
                .unwrap()
                .scope,
            Some(base)
        );
        scopes.settle();
        let mark = scopes.mark();
        let temporary = class(&mut scopes, class_name, None);
        scopes.declare_in(base, word, binding(Category::Type, Some(temporary)));
        assert_eq!(
            scopes
                .lookup_specialization(root, word, false, false)
                .unwrap()
                .scope,
            Some(temporary)
        );
        scopes.current = root;
        scopes.add_base(temporary, false);
        scopes.rewind(mark);
        let recycled = class(&mut scopes, class_name, None);
        assert_eq!(temporary, recycled);
        assert_eq!(
            scopes
                .lookup_specialization(root, word, false, false)
                .unwrap()
                .scope,
            Some(base)
        );
        assert!(scopes.incoming.get(&recycled).is_none_or(HashSet::is_empty));
        assert!(
            scopes
                .lookup_specialization(recycled, word, false, false)
                .is_none()
        );
    }
    #[test]
    fn full_search_store_renews_existing_results_and_declines_new_slots() {
        let mut names = Identifiers::default();
        let class_name = names.intern(b"C").unwrap();
        let word = names.intern(b"word").unwrap();
        let mut scopes = Scopes::default();
        let base = class(&mut scopes, class_name, None);
        scopes.declare_in(base, word, binding(Category::Type, None));
        let mut first = None;
        let mut last = base;
        for _ in 0..=1 << 16 {
            last = class(&mut scopes, class_name, Some(base));
            first.get_or_insert(last);
            assert_eq!(
                scopes
                    .lookup_specialization(last, word, false, false)
                    .unwrap()
                    .category,
                Category::Type
            );
        }
        assert_eq!(scopes.searches.len(), 1 << 16);
        scopes.declare_in(base, word, binding(Category::Value, None));
        for scope in [first.unwrap(), last] {
            assert_eq!(
                scopes
                    .lookup_specialization(scope, word, false, false)
                    .unwrap()
                    .category,
                Category::Value
            );
        }
        assert_eq!(scopes.searches.len(), 1 << 16);
    }
}
