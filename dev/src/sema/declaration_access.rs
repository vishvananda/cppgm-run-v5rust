// Translated from sema/analyzer_access.cpp's declaration access deferral. See NOTICE.
//! Failed signature and base-clause checks wait for their owning declaration.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    member_expressions::AccessFailure,
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
    pub class: Option<ScopeId>,
}
impl Analyzer<'_> {
    pub(super) fn declaration_specifiers(
        &mut self,
        specifiers: crate::syntax::storage::List<crate::syntax::ast::Specifier>,
        scope: ScopeId,
        declarator: Option<crate::syntax::ast::DeclaratorId>,
        lone: bool,
    ) -> Result<super::specifiers::Specifiers, Error> {
        let saved = self.declaration_access.class;
        let result = (|| {
            if self.semantic()
                && self.template_depth == 0
                && self.graph.scope(scope).kind == ScopeKind::Namespace
            {
                self.declaration_access.class = self
                    .declarator_environment(declarator, scope)?
                    .home
                    .filter(|&home| self.graph.scope(home).kind == ScopeKind::Class);
            }
            self.specifiers(specifiers, scope, lone)
        })();
        self.declaration_access.class = saved;
        result
    }
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
    pub(super) fn begin_base_access(&mut self, scope: ScopeId) -> Mark {
        // Base names use the enclosing lookup scope, including at block/class
        // scope. Their access waits for the complete base list (11p6-p7).
        let mark = self.begin_declaration_access(scope, false);
        self.declaration_access.context.scope = self.semantic().then_some(scope);
        mark
    }
    pub(super) fn settle_base_access(&self, scope: ScopeId) -> Result<(), Error> {
        for pending in &self.declaration_access.pending[self.declaration_access.context.first..] {
            self.validate_member_access(
                pending.binding,
                pending.naming,
                pending.object,
                scope,
                pending.location,
            )?;
        }
        Ok(())
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
        let failure = match self.member_access_failure(binding, naming, object, scope) {
            None => return Ok(()),
            Some(AccessFailure::ProtectedObject) => {
                return Err(Error::at(
                    location,
                    AccessFailure::ProtectedObject.message(),
                ));
            }
            Some(failure) => failure,
        };
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
        Err(Error::at(location, failure.message()))
    }
    pub(super) fn check_special_use_access(
        &mut self,
        binding: BindingId,
        object: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let owner = self
            .graph
            .scope(self.graph.binding_scope(binding))
            .owner
            .ok_or(Error::Invariant("special member without class"))?;
        self.check_declaration_member_access(binding, owner, Some(object), scope, location)
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

#[cfg(test)]
mod tests {
    use super::Analyzer;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{layout::LayoutState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn base_access_uses_complete_edges_and_releases_declaration_local_checks() {
        for source in [
            "struct A{private:struct B{};friend struct X;};struct X:A::B{};",
            "struct A{protected:struct B{};};struct X:A::B,A{};",
            "struct A{protected:struct B{};};struct X:A,A::B{};",
            "struct A{private:struct B{};friend struct X;};struct X{struct Y:A::B{};};",
            "struct A{private:struct B{};static B make();friend struct X;};struct X:decltype(A::make()){};",
        ] {
            let source = SourceBuffer::new(source.as_bytes().to_vec()).unwrap();
            let mut unit = SyntaxUnit::new(
                Preprocessor::with_name(&source, b"base-access-ownership").unwrap(),
            );
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let expressions = tree.expressions.mark();
            let names = [
                cursor.intern_name(b"A").unwrap(),
                cursor.intern_name(b"X").unwrap(),
            ];
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            for declaration in analyzer.tree.decl_lists.get(declarations).to_vec() {
                analyzer
                    .declaration(declaration, analyzer.graph.global)
                    .unwrap();
                assert!(analyzer.declaration_access.pending.is_empty());
                assert!(analyzer.declaration_access.context.scope.is_none());
            }
            assert_eq!(expressions, analyzer.tree.expressions.mark());
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
            for name in names {
                let binding = analyzer
                    .graph
                    .own(analyzer.graph.global, name, Filter::Elaborated)
                    .unwrap();
                let class = analyzer
                    .graph
                    .entity(analyzer.graph.binding(binding).entity);
                assert!(class.complete);
                assert!(matches!(class.layout, LayoutState::NotStarted));
            }
        }
    }

    #[test]
    fn rejected_base_clause_restores_the_enclosing_access_mark() {
        let source =
            SourceBuffer::new(b"struct A{private:struct B{};};struct X:A::B{};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"base-access-failure").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let declarations = analyzer.tree.decl_lists.get(declarations).to_vec();
        analyzer
            .declaration(declarations[0], analyzer.graph.global)
            .unwrap();
        assert!(
            analyzer
                .declaration(declarations[1], analyzer.graph.global)
                .is_err()
        );
        assert!(analyzer.declaration_access.pending.is_empty());
        assert!(analyzer.declaration_access.context.scope.is_none());
    }
}
