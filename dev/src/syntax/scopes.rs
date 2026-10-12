// Translated from syntax/name_scopes.{h,cpp}. See NOTICE.
//! Parsing categories only; indexed bindings and reversible mutations.
use super::ParseError;
use crate::lex::identifiers::NameId;
use std::collections::{HashMap, HashSet};
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
struct Scope {
    parent: Option<ScopeId>,
    edges: Vec<ScopeId>,
}
enum Change {
    Unnamed(ScopeId, Option<ScopeId>),
    Binding((ScopeId, NameId), Option<Binding>),
    Edge(ScopeId),
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
}
impl Default for Scopes {
    fn default() -> Self {
        Self {
            records: vec![Scope {
                parent: None,
                edges: Vec::new(),
            }],
            bindings: HashMap::new(),
            unnamed: HashMap::new(),
            changes: Vec::new(),
            current: ScopeId(0),
            visited: HashSet::new(),
        }
    }
}
impl Scopes {
    pub(super) fn global() -> ScopeId {
        ScopeId(0)
    }
    pub(super) fn push(&mut self) -> Result<ScopeId, ParseError> {
        let id = ScopeId(
            u32::try_from(self.records.len())
                .map_err(|_| ParseError::Limit("parser scope limit"))?,
        );
        self.records.push(Scope {
            parent: Some(self.current),
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
    pub(super) fn own(&self, scope: ScopeId, name: NameId) -> Option<Binding> {
        self.bindings.get(&(scope, name)).copied()
    }
    pub(super) fn declare(&mut self, name: NameId, binding: Binding) {
        self.declare_in(self.current, name, binding);
    }
    pub(super) fn declare_in(&mut self, scope: ScopeId, name: NameId, binding: Binding) {
        let key = (scope, name);
        let old = self.bindings.insert(key, binding);
        self.changes.push(Change::Binding(key, old));
    }
    pub(super) fn add_edge(&mut self, scope: ScopeId) {
        let current = self.current;
        if !self.records[current.0 as usize].edges.contains(&scope) {
            self.records[current.0 as usize].edges.push(scope);
            self.changes.push(Change::Edge(current));
        }
    }
    pub(super) fn lookup(&mut self, name: NameId) -> Option<Binding> {
        self.visited.clear();
        let mut current = Some(self.current);
        while let Some(scope) = current {
            if let Some(binding) = self.search(scope, name) {
                return Some(binding);
            }
            current = self.records[scope.0 as usize].parent;
        }
        None
    }
    pub(super) fn lookup_in(&mut self, scope: ScopeId, name: NameId) -> Option<Binding> {
        self.visited.clear();
        self.search(scope, name)
    }
    fn search(&mut self, scope: ScopeId, name: NameId) -> Option<Binding> {
        if !self.visited.insert(scope) {
            return None;
        }
        if let Some(binding) = self.own(scope, name) {
            return Some(binding);
        }
        for index in 0..self.records[scope.0 as usize].edges.len() {
            let edge = self.records[scope.0 as usize].edges[index];
            if let Some(binding) = self.search(edge, name) {
                return Some(binding);
            }
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
                Some(Change::Edge(scope)) => {
                    self.records[scope.0 as usize].edges.pop();
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
