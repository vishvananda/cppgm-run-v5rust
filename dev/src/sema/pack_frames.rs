// Translated from template_packs.cpp::FrameExpander/pack_element_name. See NOTICE.
//! A syntax-copy query borrows the interner and indexes only enclosing pack frames.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, Graph, ScopeId, ScopeKind},
    types::TypeKind,
};
use crate::{
    lex::identifiers::{Identifiers, NameId},
    syntax::{
        ParseError, ast,
        cloning::{Cloner, PackExpander},
    },
};
use std::collections::HashMap;

pub(super) struct FramePacks<'a> {
    identifiers: &'a mut Identifiers,
    lengths: HashMap<NameId, usize>,
}
impl<'a> FramePacks<'a> {
    pub(super) fn new(graph: &Graph, scope: ScopeId, identifiers: &'a mut Identifiers) -> Self {
        let mut lengths = HashMap::new();
        let mut scope = Some(scope);
        while let Some(current) = scope {
            let node = graph.scope(current);
            if node.kind == ScopeKind::TemplateParameters {
                // Hidden frame aliases are in lookup, independently of the
                // source-facing declaration list used by the dump tools.
                for id in node.index.values().filter_map(|entry| entry.ty) {
                    let binding = graph.binding(id);
                    if binding.kind != BindingKind::Alias {
                        continue;
                    }
                    let Some(name) = binding.name else {
                        continue;
                    };
                    let Some(mut ty) = binding.ty else {
                        continue;
                    };
                    if let TypeKind::BoundPack { arguments, .. } = graph.types.get(ty).kind {
                        ty = arguments;
                    }
                    if let TypeKind::Pack(elements) = &graph.types.get(ty).kind {
                        lengths.entry(name).or_insert(elements.len());
                    }
                }
            }
            scope = node.parent;
        }
        Self {
            identifiers,
            lengths,
        }
    }
}
impl PackExpander for FramePacks<'_> {
    fn pack_length(&self, name: NameId) -> Option<usize> {
        self.lengths.get(&name).copied()
    }
    fn element_name(&mut self, name: NameId, index: usize) -> Result<NameId, ParseError> {
        let mut spelling = self.identifiers.bytes(name).to_vec();
        spelling.extend_from_slice(format!("__pack{index}").as_bytes());
        self.identifiers
            .intern(&spelling)
            .map_err(ParseError::Limit)
    }
}

impl Analyzer<'_> {
    pub(super) fn copy_expression_in_frame(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ast::ExprId, Error> {
        let mut packs = FramePacks::new(&self.graph, scope, self.identifiers);
        Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .with_expander(&mut packs)
            .expression(expression)
            .map_err(|error| super::template_bodies::copy_error(&error))
    }
    pub(super) fn copy_initializer_in_frame(
        &mut self,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<ast::Initializer, Error> {
        let mut packs = FramePacks::new(&self.graph, scope, self.identifiers);
        Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .with_expander(&mut packs)
            .initializer(initializer)
            .map_err(|error| super::template_bodies::copy_error(&error))
    }
}
