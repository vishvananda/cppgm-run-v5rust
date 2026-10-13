// Translated from sema/analyzer_calls.cpp's argument-dependent lookup. See NOTICE.
//! Association walks only argument types and indexed namespace edges.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, ScopeId, ScopeKind},
    lookup::LookupResult,
    types::TypeKind,
};
use crate::{
    lex::identifiers::NameId,
    syntax::{ast, storage::List},
};
use std::collections::HashSet;
impl Analyzer<'_> {
    pub(super) fn argument_dependent_lookup(
        &self,
        name: NameId,
        arguments: List<ast::ExprId>,
        found: &mut LookupResult,
    ) -> Result<(), Error> {
        if let Some(binding) = found.binding {
            if self.graph.binding(binding).kind != BindingKind::Function {
                return Ok(());
            }
            let origin = self
                .graph
                .function_origin(self.graph.binding(binding).entity)?;
            if self.graph.scope(self.graph.binding_scope(origin)).kind != ScopeKind::Namespace {
                return Ok(());
            }
        }
        let mut types = Vec::new();
        for &argument in self.tree.expr_lists.get(arguments) {
            let facts = self.graph.expression_facts.get(argument)?;
            if facts.functions.is_none()
                && !matches!(self.graph.types.get(facts.ty).kind, TypeKind::Scalar(_))
            {
                types.push((facts.ty, 0u32));
            }
        }
        let mut seen_types = HashSet::new();
        let mut namespaces = Vec::new();
        while let Some((ty, depth)) = types.pop() {
            if depth > 64 || !seen_types.insert(ty) {
                continue;
            }
            match &self.graph.types.get(ty).kind {
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Array { element, .. } => types.push((*element, depth + 1)),
                TypeKind::MemberPointer { owner, element } => {
                    types.push((*element, depth + 1));
                    types.push((*owner, depth + 1));
                }
                TypeKind::Function {
                    parameters, result, ..
                } => {
                    types.extend(parameters.iter().map(|&ty| (ty, depth + 1)));
                    types.push((*result, depth + 1));
                }
                TypeKind::Class(entity) | TypeKind::Enum(entity) => {
                    let entity = self.graph.entity(*entity);
                    if entity.complete {
                        for base in &entity.bases {
                            types.push((
                                self.graph
                                    .entity(base.class)
                                    .ty
                                    .ok_or(Error::Invariant("ADL base without type"))?,
                                depth,
                            ));
                        }
                    }
                    let owner = if let Some(enumeration) = entity.enumeration {
                        Some(enumeration.owner)
                    } else {
                        entity.home
                    };
                    if let Some(owner) = owner {
                        namespaces.push(self.associated_namespace(owner));
                    }
                }
                _ => (),
            }
        }
        if namespaces.is_empty() {
            return Ok(());
        }
        let mut seen_namespaces = HashSet::new();
        let mut seen_functions: HashSet<_> = found
            .functions
            .iter()
            .map(|&id| self.graph.binding(id).entity)
            .collect();
        while let Some(scope) = namespaces.pop() {
            if !seen_namespaces.insert(scope) {
                continue;
            }
            namespaces.extend(self.graph.visible_edges(scope, true));
            let node = self.graph.scope(scope);
            if node.is_inline
                && let Some(parent) = node.parent
            {
                namespaces.push(parent);
            }
            for binding in self.graph.functions_at(scope, name.into()) {
                if seen_functions.insert(self.graph.binding(binding).entity) {
                    found.functions.push(binding);
                    if found.binding.is_none() {
                        found.binding = Some(binding);
                    }
                }
            }
        }
        Ok(())
    }
    fn associated_namespace(&self, mut scope: ScopeId) -> ScopeId {
        while self.graph.scope(scope).kind != ScopeKind::Namespace {
            let Some(parent) = self.graph.scope(scope).parent else {
                break;
            };
            scope = parent;
        }
        scope
    }
}
