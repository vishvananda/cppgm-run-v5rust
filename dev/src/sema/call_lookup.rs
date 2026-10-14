// Translated from sema/analyzer_calls.cpp's argument-dependent lookup. See NOTICE.
//! Association walks only argument types and indexed namespace edges.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::ValueKey,
    graph::{BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::LookupResult,
    types::{TypeId, TypeKind},
};
use crate::syntax::{ast, storage::List};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct AssociationScratch {
    types: Vec<(TypeId, u32)>,
    seen_types: HashMap<TypeId, u32>,
    namespaces: Vec<ScopeId>,
    classes: Vec<EntityId>,
    seen_classes: HashSet<EntityId>,
    enclosing: Vec<EntityId>,
    seen_namespaces: HashSet<ScopeId>,
    seen_functions: HashSet<EntityId>,
}
impl AssociationScratch {
    fn clear(&mut self) {
        self.types.clear();
        self.seen_types.clear();
        self.namespaces.clear();
        self.classes.clear();
        self.seen_classes.clear();
        self.enclosing.clear();
        self.seen_namespaces.clear();
        self.seen_functions.clear();
    }
}
impl Analyzer<'_> {
    pub(super) fn argument_dependent_lookup(
        &mut self,
        name: ValueKey,
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
        self.associated_functions(name, self.tree.expr_lists.get(arguments), found)
    }
    pub(super) fn associated_functions(
        &mut self,
        name: ValueKey,
        arguments: &[ast::ExprId],
        found: &mut LookupResult,
    ) -> Result<(), Error> {
        let mut scratch = std::mem::take(&mut self.association_scratch);
        let result = self.associated_query(name, arguments, found, &mut scratch);
        scratch.clear();
        self.association_scratch = scratch;
        result
    }
    fn associated_query(
        &self,
        name: ValueKey,
        arguments: &[ast::ExprId],
        found: &mut LookupResult,
        scratch: &mut AssociationScratch,
    ) -> Result<(), Error> {
        let AssociationScratch {
            types,
            seen_types,
            namespaces,
            classes,
            seen_classes,
            enclosing,
            seen_namespaces,
            seen_functions,
        } = scratch;
        for &argument in arguments {
            if matches!(
                self.tree.expressions.get(argument).kind,
                ast::ExprKind::Braced { .. }
            ) {
                continue;
            }
            let facts = self.graph.expression_facts.get(argument)?;
            if (facts.functions.is_none() || self.ordinary_function_operand(facts).is_some())
                && !matches!(self.graph.types.get(facts.ty).kind, TypeKind::Scalar(_))
            {
                types.push((facts.ty, 0u32));
            }
        }
        // The same type may be reached with more association depth remaining.
        // Revisit only for a shorter path; every successful revisit decreases
        // its depth, bounded by the inherited 64-level type budget.
        while let Some((ty, depth)) = types.pop() {
            if depth > 64 || seen_types.get(&ty).is_some_and(|&earlier| earlier <= depth) {
                continue;
            }
            seen_types.insert(ty, depth);
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
                    let identity = *entity;
                    let entity = self.graph.entity(identity);
                    if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_))
                        && seen_classes.insert(identity)
                    {
                        classes.push(identity);
                    }
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
                        if self.graph.scope(owner).kind == ScopeKind::Class
                            && let Some(class) = self.graph.scope(owner).owner
                        {
                            enclosing.push(class);
                        }
                    }
                }
                _ => (),
            }
        }
        for class in enclosing.drain(..) {
            if seen_classes.insert(class) {
                classes.push(class);
            }
        }
        if namespaces.is_empty() {
            return Ok(());
        }
        seen_functions.extend(
            found
                .functions
                .iter()
                .map(|&id| self.graph.binding(id).entity),
        );
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
            for binding in self.graph.visible_functions_at(scope, name) {
                if seen_functions.insert(self.graph.binding(binding).entity) {
                    found.functions.push(binding);
                    if found.binding.is_none() {
                        found.binding = Some(binding);
                    }
                }
            }
        }
        for class in classes.drain(..) {
            for &binding in self.graph.friendships.functions(class, name) {
                if self.graph.binding_visible_at_limit(binding)
                    && seen_functions.insert(self.graph.binding(binding).entity)
                {
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
