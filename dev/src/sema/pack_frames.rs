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
        element_name(self.identifiers, name, index).map_err(ParseError::Limit)
    }
}

fn element_name(
    identifiers: &mut Identifiers,
    name: NameId,
    index: usize,
) -> Result<NameId, &'static str> {
    let mut spelling = identifiers.bytes(name).to_vec();
    spelling.extend_from_slice(format!("__pack{index}").as_bytes());
    identifiers.intern(&spelling)
}

impl Analyzer<'_> {
    // class_templates.cpp::substituted_scope: a known prototype pack owns
    // ordinary element parameters and a hidden alias for sizeof...(name).
    pub(super) fn substitute_prototype_pack(
        &mut self,
        frame: ScopeId,
        packs: &mut Option<ScopeId>,
        binding: super::graph::Binding,
        environment: &mut super::function_templates::Substitution<'_>,
    ) -> Result<bool, Error> {
        let Some(ty) = binding.ty.filter(|&ty| {
            binding.name.is_some()
                && matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_))
        }) else {
            return Ok(false);
        };
        let elements = match self.substitute_pack_list(&[ty], environment) {
            Ok(Some(elements)) => elements,
            Ok(None) | Err(Error::Diagnostic { .. } | Error::NonConstant { .. }) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        if elements
            .iter()
            .any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)))
        {
            return Ok(false);
        }
        let name = binding
            .name
            .ok_or(Error::Invariant("prototype pack without name"))?;
        for (index, &ty) in elements.iter().enumerate() {
            let name = element_name(self.identifiers, name, index).map_err(Error::Limit)?;
            let entity = self.graph.add_entity(Some(ty), Some(name), binding.key)?;
            let id = self.graph.add_binding(
                frame,
                Some(frame),
                super::graph::Binding {
                    entity,
                    name: Some(name),
                    ty: Some(ty),
                    ..binding
                },
            )?;
            self.graph.set_binding_dependence(id, true);
        }
        let scope = if let Some(scope) = *packs {
            scope
        } else {
            let scope = self
                .graph
                .add_scope(ScopeKind::TemplateParameters, frame, None, None)?;
            *packs = Some(scope);
            scope
        };
        let ty = self
            .graph
            .types
            .intern(TypeKind::Pack(elements), super::types::Cv::default())?;
        let entity = self.graph.add_entity(Some(ty), Some(name), binding.key)?;
        self.graph.add_binding(
            scope,
            None,
            super::graph::Binding {
                kind: BindingKind::Alias,
                entity,
                ty: Some(ty),
                ..binding
            },
        )?;
        Ok(true)
    }

    #[inline(never)]
    pub(super) fn copy_selected_operand(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        environment: &super::function_templates::Substitution<'_>,
    ) -> Result<ast::ExprId, Error> {
        let Some(selection) = environment.selection else {
            return self.copy_expression_in_frame(expression, scope);
        };
        let mut names = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut parent = Some(scope);
        while let Some(current) = parent {
            let node = self.graph.scope(current);
            if node.kind == ScopeKind::TemplateParameters {
                for id in node.index.values().filter_map(|entry| entry.ty) {
                    let binding = self.graph.binding(id);
                    let Some(name) = binding.name.filter(|&name| seen.insert(name)) else {
                        continue;
                    };
                    let Some(ty) = binding.ty else {
                        continue;
                    };
                    let TypeKind::BoundPack { parameter, .. } = self.graph.types.get(ty).kind
                    else {
                        continue;
                    };
                    let element = self
                        .graph
                        .entity(parameter)
                        .template_parameter
                        .and_then(|info| info.index(environment.depth))
                        .and_then(|index| selection.elements.get(index).copied().flatten())
                        .or(selection.bound_element);
                    if let Some(index) = element {
                        names.push((name, index));
                    }
                }
            }
            parent = node.parent;
        }
        let mut packs = FramePacks::new(&self.graph, scope, self.identifiers);
        let mut cloner =
            Cloner::new(self.tree, self.stack_budget, self.stack_base).with_expander(&mut packs);
        for (name, index) in names {
            cloner
                .select_pack(name, index)
                .map_err(|error| super::template_bodies::copy_error(&error))?;
        }
        cloner
            .expression(expression)
            .map_err(|error| super::template_bodies::copy_error(&error))
    }
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
