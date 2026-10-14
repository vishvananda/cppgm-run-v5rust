// Translated from sema/analyzer_access.cpp's declaration access deferral. See NOTICE.
//! Failed signature checks wait only for their declaration's canonical identity.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
};
use crate::support::source::SourceOffset;

#[derive(Clone, Copy)]
struct Pending {
    binding: BindingId,
    naming: EntityId,
    object: Option<EntityId>,
    scope: ScopeId,
    location: SourceOffset,
}
#[derive(Clone, Copy, Default)]
pub(super) struct Mark {
    scope: Option<ScopeId>,
    first: usize,
    keep: usize,
    length: usize,
}
#[derive(Default)]
pub(super) struct DeclarationAccess {
    context: Mark,
    pending: Vec<Pending>,
    pub function: Option<EntityId>,
}
impl Analyzer<'_> {
    pub(super) fn begin_declaration_access(&mut self, scope: ScopeId, enabled: bool) -> Mark {
        let mark = Mark {
            length: self.declaration_access.pending.len(),
            ..self.declaration_access.context
        };
        self.declaration_access.context = Mark {
            scope: (enabled
                && self.semantic()
                && self.template_depth == 0
                && self.graph.scope(scope).kind == ScopeKind::Namespace)
                .then_some(scope),
            first: mark.length,
            keep: mark.length,
            length: mark.length,
        };
        mark
    }
    pub(super) fn finish_declaration_access(&mut self, mark: Mark) {
        self.declaration_access.pending.truncate(mark.length);
        self.declaration_access.context = mark;
    }
    pub(super) fn resume_declaration_access(&mut self, scope: ScopeId) {
        self.declaration_access.context.scope = (self.semantic()
            && self.template_depth == 0
            && self.graph.scope(scope).kind == ScopeKind::Namespace)
            .then_some(scope);
    }
    pub(super) fn keep_specifier_access(&mut self) {
        self.declaration_access.context.keep = self.declaration_access.pending.len();
    }
    pub(super) fn check_declaration_member_access(
        &mut self,
        binding: BindingId,
        naming: EntityId,
        object: Option<EntityId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let result = self.validate_member_access(binding, naming, object, scope, location);
        if result.is_ok() {
            return result;
        }
        let mut owner = scope;
        while self.graph.scope(owner).kind == ScopeKind::Function
            && self.graph.scope(owner).owner.is_none()
        {
            owner = self
                .graph
                .scope(owner)
                .parent
                .ok_or(Error::Invariant("prototype without parent"))?;
        }
        if self.declaration_access.context.scope == Some(owner) {
            self.declaration_access.pending.push(Pending {
                binding,
                naming,
                object,
                scope,
                location,
            });
            return Ok(());
        }
        result
    }
    pub(super) fn check_declaration_direct_access(
        &mut self,
        binding: BindingId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let home = self.graph.binding_scope(binding);
        if self.graph.scope(home).kind == ScopeKind::Class
            && let Some(class) = self.graph.scope(home).owner
        {
            let naming = std::iter::successors(Some(scope), |&id| self.graph.scope(id).parent)
                .find_map(|id| {
                    let owner = self.graph.scope(id);
                    (owner.kind == ScopeKind::Class)
                        .then_some(owner.owner)
                        .flatten()
                        .filter(|&owner| owner == class || self.graph.is_base_of(class, owner))
                })
                .unwrap_or(class);
            self.check_declaration_member_access(binding, naming, None, scope, location)
        } else {
            Ok(())
        }
    }
    pub(super) fn settle_declaration_access(
        &mut self,
        binding: Option<BindingId>,
    ) -> Result<(), Error> {
        let function = binding
            .filter(|&id| self.graph.binding(id).kind == BindingKind::Function)
            .map(|id| self.graph.binding(id).entity);
        let saved = self.declaration_access.function;
        self.declaration_access.function = function;
        let result = (self.declaration_access.context.first..self.declaration_access.pending.len())
            .try_for_each(|index| {
                let pending = self.declaration_access.pending[index];
                if function.is_none() {
                    return Err(Error::at(pending.location, "inaccessible member"));
                }
                self.validate_member_access(
                    pending.binding,
                    pending.naming,
                    pending.object,
                    pending.scope,
                    pending.location,
                )
            });
        self.declaration_access.function = saved;
        self.declaration_access.context.scope = None;
        self.declaration_access
            .pending
            .truncate(self.declaration_access.context.keep);
        result
    }
}
