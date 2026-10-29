// Translated from lower/symbol_lowering.cpp::unnamed_ordinal/type_unqualified_name
// and local_mangling.cpp::anonymous_name. See NOTICE.
//! Unit-owned ABI identities number a class's unnamed member types once.
use super::Error;
use crate::support::hash::HashMap;
use crate::{
    abi::graph::{Graph as AbiGraph, Node, NodeId},
    sema::{
        Graph,
        lowering::{EntityId, ScopeId, UnnamedNumbering},
    },
};
#[derive(Default)]
pub(super) struct Names {
    numbered: HashMap<EntityId, u32>,
    private: HashMap<EntityId, u32>,
    next_private: u32,
    function_private: HashMap<EntityId, u32>,
}
impl Names {
    pub(super) fn scope_terminal(
        &mut self,
        graph: &Graph,
        abi: &mut AbiGraph,
        scope: ScopeId,
    ) -> Result<Option<NodeId>, Error> {
        let Some((class, _, _)) = graph.lower_anonymous_scope(scope)? else {
            return Ok(None);
        };
        Ok(Some(self.terminal(graph, abi, class)?))
    }
    pub(super) fn terminal(
        &mut self,
        graph: &Graph,
        abi: &mut AbiGraph,
        entity: EntityId,
    ) -> Result<NodeId, Error> {
        let context = match graph.lower_unnamed_numbering(entity)? {
            UnnamedNumbering::Members(home) => {
                if !self.numbered.contains_key(&entity) {
                    let mut next = 0u32;
                    for member in graph.lower_unnamed_scope_types(home) {
                        if let std::collections::hash_map::Entry::Vacant(entry) =
                            self.numbered.entry(member)
                        {
                            entry.insert(next);
                            next = next.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
                        }
                    }
                }
                let ordinal = *self
                    .numbered
                    .get(&entity)
                    .ok_or(crate::sema::Error::Invariant(
                        "unnamed type missing from declarations",
                    ))?;
                return Ok(abi.intern(Node::UnnamedType(ordinal), &[])?);
            }
            UnnamedNumbering::Local(ordinal) => {
                return Ok(abi.intern(Node::UnnamedType(ordinal), &[])?);
            }
            UnnamedNumbering::Private(context) => context,
        };
        let ordinal = match self.private.entry(entity) {
            std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let ordinal = if let Some(function) = context {
                    let next = self.function_private.entry(function).or_default();
                    let ordinal = *next;
                    *next = ordinal.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
                    ordinal
                } else {
                    self.next_private
                };
                // The unit total includes private names allocated in functions.
                self.next_private = self
                    .next_private
                    .checked_add(1)
                    .ok_or(crate::lowir::Error::Limit)?;
                entry.insert(ordinal);
                ordinal
            }
        };
        Ok(abi.string(format!("$_{ordinal}").as_bytes())?)
    }
}

// Clang reserves names for written in-class bodies after the complete
// outer declaration; this names scopes without demanding their functions.
#[derive(Default)]
struct Deferred {
    work: Vec<(crate::syntax::ast::DeclId, bool)>,
    named: crate::support::hash::HashSet<ScopeId>,
    scopes: Vec<ScopeId>,
}
impl Deferred {
    fn scopes(
        &mut self,
        graph: &Graph,
        tree: &crate::syntax::ast::Tree,
        declaration: crate::syntax::ast::DeclId,
    ) -> Result<&[ScopeId], Error> {
        use crate::syntax::ast::{DeclKind, SpecifierKind};
        self.work.clear();
        self.work.push((declaration, false));
        self.named.clear();
        self.scopes.clear();
        while let Some((id, member)) = self.work.pop() {
            match tree.declarations.get(id).kind {
                DeclKind::Namespace { declarations, .. }
                | DeclKind::Linkage { declarations, .. } => {
                    self.work.extend(
                        tree.decl_lists
                            .get(declarations)
                            .iter()
                            .rev()
                            .map(|&id| (id, false)),
                    );
                }
                DeclKind::Template {
                    parameters,
                    declaration,
                } if parameters.parameters.is_empty() => {
                    self.work.push((declaration, member));
                }
                DeclKind::Class(specifier) => {
                    if let SpecifierKind::Class(class) = specifier.kind {
                        self.work.extend(
                            tree.decl_lists
                                .get(tree.classes.get(class).members)
                                .iter()
                                .rev()
                                .map(|&id| (id, true)),
                        );
                    }
                }
                DeclKind::Simple { specifiers, .. } => {
                    for specifier in tree.specs.get(specifiers).iter().rev() {
                        if let SpecifierKind::Class(class) = specifier.kind {
                            self.work.extend(
                                tree.decl_lists
                                    .get(tree.classes.get(class).members)
                                    .iter()
                                    .rev()
                                    .map(|&id| (id, true)),
                            );
                        }
                    }
                }
                DeclKind::Function { declarator, .. }
                | DeclKind::SpecialMember {
                    declarator,
                    body: Some(_),
                    ..
                } if member => {
                    let function = graph.lower_declared(declarator)?;
                    if let Some(scope) = graph.lower_unnamed_member_owner(function)
                        && self.named.insert(scope)
                    {
                        self.scopes.push(scope);
                    }
                }
                _ => {}
            }
        }
        Ok(&self.scopes)
    }
}

// The finish event belongs to the outermost declaration, including a whole
// namespace/linkage block. Nested declarations emit before that event.
pub(super) struct Declarations {
    deferred: Deferred,
    work: Vec<(crate::syntax::ast::DeclId, bool)>,
}
impl Declarations {
    pub(super) fn new(
        tree: &crate::syntax::ast::Tree,
        declarations: crate::syntax::storage::List<crate::syntax::ast::DeclId>,
    ) -> Self {
        let mut work = Vec::new();
        for &id in tree.decl_lists.get(declarations).iter().rev() {
            work.push((id, true));
            work.push((id, false));
        }
        Self {
            work,
            deferred: Deferred::default(),
        }
    }
    pub(super) fn next(
        &mut self,
        tree: &crate::syntax::ast::Tree,
        graph: &Graph,
        symbols: &mut super::symbols::Symbols<'_, '_>,
    ) -> Result<Option<(crate::syntax::ast::DeclId, bool)>, Error> {
        use crate::syntax::ast::DeclKind;
        while let Some((id, finish)) = self.work.pop() {
            if finish {
                if graph.lower_has_unnamed_member_definitions() {
                    symbols.name_scopes(self.deferred.scopes(graph, tree, id)?)?;
                }
                continue;
            }
            match tree.declarations.get(id).kind {
                DeclKind::Namespace { declarations, .. }
                | DeclKind::Linkage { declarations, .. } => {
                    self.work.extend(
                        tree.decl_lists
                            .get(declarations)
                            .iter()
                            .rev()
                            .map(|&id| (id, false)),
                    );
                }
                DeclKind::Template {
                    parameters,
                    declaration,
                } if parameters.parameters.is_empty() => {
                    if matches!(
                        tree.declarations.get(declaration).kind,
                        DeclKind::Simple { .. }
                    ) {
                        return Ok(Some((declaration, true)));
                    }
                    self.work.push((declaration, false));
                }
                _ => return Ok(Some((id, false))),
            }
        }
        Ok(None)
    }
}

pub(super) fn decomposition_spelling(
    graph: &Graph,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    entity: EntityId,
) -> Result<Option<String>, Error> {
    let Some(bindings) = graph.lower_decomposition(entity) else {
        return Ok(None);
    };
    let mut spelling = String::from("__binding");
    for &binding in bindings {
        let name = graph.lower_variable(binding)?.named()?;
        spelling.push_str("__");
        spelling.push_str(
            std::str::from_utf8(cursor.name(name))
                .map_err(|_| Error::Missing("local name spelling"))?,
        );
    }
    Ok(Some(spelling))
}
pub(super) fn variable_terminal(
    graph: &Graph,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    abi: &mut AbiGraph,
    entity: EntityId,
) -> Result<NodeId, Error> {
    if let Some(bindings) = graph.lower_decomposition(entity) {
        let mut names = Vec::with_capacity(bindings.len());
        for &binding in bindings {
            let name = graph.lower_variable(binding)?.named()?;
            names.push(abi.string(cursor.name(name))?);
        }
        return Ok(abi.intern(Node::DecompositionName, &names)?);
    }
    let facts = graph.lower_variable(entity)?;
    let identifier = facts
        .name
        .or_else(|| graph.lower_first_named_member(facts.ty))
        .ok_or(crate::sema::Error::Invariant(
            "anonymous object without named member",
        ))?;
    Ok(abi.string(cursor.name(identifier))?)
}
