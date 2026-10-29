// Translated from template_packs.cpp::FrameExpander/pack_element_name. See NOTICE.
//! A syntax-copy query borrows the interner and indexes only enclosing pack frames.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, Graph, ScopeId, ScopeKind},
    types::TypeKind,
};
use crate::support::hash::HashMap;
use crate::{
    lex::identifiers::{Identifiers, NameId},
    syntax::{
        ParseError, ast,
        cloning::{Cloner, PackExpander},
    },
};

struct FrameMap {
    lengths: HashMap<NameId, usize>,
    substitution: super::substitution_state::State,
}
impl FrameMap {
    fn new(graph: &Graph, scope: ScopeId) -> Self {
        let mut lengths = HashMap::default();
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
            lengths,
            substitution: graph.analysis.substitution.clone(),
        }
    }
}
pub(super) struct CopySession {
    frames: FrameMap,
    syntax: crate::syntax::cloning::CopyState,
}
impl CopySession {
    fn new(graph: &Graph, scope: ScopeId) -> Self {
        Self {
            frames: FrameMap::new(graph, scope),
            syntax: crate::syntax::cloning::CopyState::default(),
        }
    }
}
enum FrameStore<'a> {
    Owned(FrameMap),
    Borrowed(&'a FrameMap),
}
pub(super) struct FramePacks<'a> {
    identifiers: &'a mut Identifiers,
    frames: FrameStore<'a>,
    abort: Option<Error>,
}
impl<'a> FramePacks<'a> {
    pub(super) fn new(graph: &Graph, scope: ScopeId, identifiers: &'a mut Identifiers) -> Self {
        Self {
            identifiers,
            frames: FrameStore::Owned(FrameMap::new(graph, scope)),
            abort: None,
        }
    }
    fn borrowing(identifiers: &'a mut Identifiers, frames: &'a FrameMap) -> Self {
        Self {
            identifiers,
            frames: FrameStore::Borrowed(frames),
            abort: None,
        }
    }
    fn frames(&self) -> &FrameMap {
        match &self.frames {
            FrameStore::Owned(frames) => frames,
            FrameStore::Borrowed(frames) => frames,
        }
    }
}
impl FramePacks<'_> {
    pub(super) fn enabled(&self) -> bool {
        !self.frames().lengths.is_empty()
    }
    pub(super) fn cloner<'copy>(
        &'copy mut self,
        tree: &'copy mut ast::Tree,
        budget: crate::support::stack::StackBudget,
        base: usize,
    ) -> Cloner<'copy> {
        let cloner = Cloner::new(tree, budget, base);
        if self.enabled() {
            cloner.with_expander(self)
        } else {
            cloner
        }
    }
    fn session_cloner<'copy>(
        &'copy mut self,
        tree: &'copy mut ast::Tree,
        budget: crate::support::stack::StackBudget,
        base: usize,
        state: &'copy mut crate::syntax::cloning::CopyState,
    ) -> Cloner<'copy> {
        let cloner = Cloner::new(tree, budget, base);
        if self.enabled() {
            cloner.with_expander_state(self, state)
        } else {
            cloner
        }
    }
    pub(super) fn finish<T>(self, result: Result<T, ParseError>) -> Result<T, Error> {
        match (result, self.abort) {
            (Err(ParseError::CloneAbort), Some(error)) => Err(error),
            (Err(ParseError::CloneAbort), None) | (_, Some(_)) => {
                Err(Error::Invariant("invalid pack copy abort transport"))
            }
            (result, None) => result.map_err(|error| super::template_bodies::copy_error(&error)),
        }
    }
}
impl PackExpander for FramePacks<'_> {
    fn mismatch(
        &mut self,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), ParseError> {
        match self
            .frames()
            .substitution
            .emit(super::substitution_state::Failure {
                location,
                message: "a pack expansion of parameter packs of different lengths",
                transient: false,
            }) {
            Ok(_) => Ok(()),
            Err(error) => {
                self.abort = Some(error);
                Err(ParseError::CloneAbort)
            }
        }
    }

    fn pack_length(&self, name: NameId) -> Option<usize> {
        self.frames().lengths.get(&name).copied()
    }
    fn element_name(&mut self, name: NameId, index: usize) -> Result<NameId, ParseError> {
        element_name(self.identifiers, name, index).map_err(ParseError::Limit)
    }
}

pub(super) fn element_name(
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
            Ok(None) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => {
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
        renames: &[(NameId, NameId)],
    ) -> Result<ast::ExprId, Error> {
        let mut packs = FramePacks::new(&self.graph, scope, self.identifiers);
        let copied = {
            let mut cloner = packs.cloner(self.tree, self.stack_budget, self.stack_base);
            for &(name, element) in renames {
                cloner.rename_pack(name, element);
            }
            cloner.expression(expression)
        };
        packs.finish(copied)
    }
    pub(super) fn copy_expression_in_frame(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ast::ExprId, Error> {
        let mut packs = FramePacks::new(&self.graph, scope, self.identifiers);
        let result = packs
            .cloner(self.tree, self.stack_budget, self.stack_base)
            .expression(expression);
        packs.finish(result)
    }
    pub(super) fn prepare_initializer_copy(
        &mut self,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
    ) -> Result<(Option<ast::Initializer>, CopySession), Error> {
        let mut session = CopySession::new(&self.graph, scope);
        let mut packs = FramePacks::borrowing(self.identifiers, &session.frames);
        let result = {
            let mut cloner = packs.session_cloner(
                self.tree,
                self.stack_budget,
                self.stack_base,
                &mut session.syntax,
            );
            initializer
                .map(|initializer| cloner.initializer(initializer))
                .transpose()
        };
        let initializer = packs.finish(result)?;
        Ok((initializer, session))
    }
    pub(super) fn copy_expression_with_session(
        &mut self,
        expression: ast::ExprId,
        session: &mut CopySession,
    ) -> Result<ast::ExprId, Error> {
        let mut packs = FramePacks::borrowing(self.identifiers, &session.frames);
        let result = packs
            .session_cloner(
                self.tree,
                self.stack_budget,
                self.stack_base,
                &mut session.syntax,
            )
            .expression(expression);
        packs.finish(result)
    }
    pub(super) fn copy_initializer_in_frame(
        &mut self,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<ast::Initializer, Error> {
        let mut packs = FramePacks::new(&self.graph, scope, self.identifiers);
        let result = packs
            .cloner(self.tree, self.stack_budget, self.stack_base)
            .initializer(initializer);
        packs.finish(result)
    }
}
