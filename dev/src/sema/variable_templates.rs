// Translated from sema/variable_templates.cpp::declare_variable_template,
// named_variable_template/variable_partial_specialization and
// analyzer_declarations.cpp::declare_declarator/declare_variable. See NOTICE.
//! Declaration patterns own their heads and initializers; a partial never
//! redeclares its primary's ordinary binding or demands specialization storage.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    graph::{BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::Filter,
    specifiers::{DeclarationRole, Specifiers},
    types::{Cv, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

impl Analyzer<'_> {
    pub(super) fn variable_template_primary(&self, binding: BindingId) -> Option<EntityId> {
        let binding = self.graph.binding(binding);
        (binding.kind == BindingKind::Variable
            && self.graph.entity(binding.entity).template_head.is_some())
        .then_some(binding.entity)
    }

    /// This dispatch precedes ordinary variable-specifier and object checks,
    /// exactly as in C++. A partial declares a pattern, not an object.
    pub(super) fn variable_template_declarator(
        &mut self,
        specs: Specifiers,
        init: ast::InitDeclarator,
        scope: ScopeId,
        info: &DeclaratorInfo,
    ) -> Result<bool, Error> {
        if specs.role == DeclarationRole::Typedef
            || matches!(
                self.graph.types.get(info.ty).kind,
                TypeKind::Function { .. }
            )
        {
            return Ok(false);
        }
        let Some(name) = info.syntax_name else {
            return Ok(false);
        };
        let node = *self.tree.names.get(name);
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .copied()
            .ok_or(Error::Invariant("empty variable declarator name"))?;
        if last.arguments.is_none() {
            return Ok(false);
        }
        if self.template_depth == 0 {
            return Err(Error::at(
                node.location,
                "a template-id declarator outside a specialization's declaration",
            ));
        }
        let Some(head) = self.active_template_head(scope) else {
            self.graph
                .variable_instances
                .defer_declaration(init.declarator);
            return Ok(true);
        };
        // Suppress specialization of the last component, retaining ordinary
        // qualified lookup of its owner and all normal ambiguity checks.
        let found = self
            .lookup_result_in(name, scope, Filter::Ordinary, false)?
            .binding
            .ok_or_else(|| Error::at(node.location, "undeclared name"))?;
        let primary = self.variable_template_primary(found).ok_or_else(|| {
            Error::at(
                node.location,
                "a template-id declarator of no variable template",
            )
        })?;
        let primary_head = self
            .graph
            .entity(primary)
            .template_head
            .ok_or(Error::Invariant("variable template lost head"))?;
        self.supported_type_template_head(primary_head)?;
        let mut arguments = self.type_template_arguments(last, scope)?;
        self.complete_type_template_arguments(primary_head, &mut arguments, node.location)?;
        self.check_type_partial_arguments(head, &arguments, node.location)?;
        let ty = if specs.constexpr {
            self.graph.types.qualify(info.ty, Cv::CONST)?
        } else {
            info.ty
        };
        let template = self.graph.entity(primary);
        let partial = self
            .graph
            .add_entity(Some(ty), template.name, Keyword::Int)?;
        let home = self.graph.entity(primary).home;
        let entity = self.graph.entity_mut(partial);
        entity.home = home;
        entity.template_head = Some(head);
        entity.defined = init.initializer.is_some();
        entity.constexpr = specs.constexpr;
        entity.variable_definition = init.initializer.map(|_| init.declarator);
        self.retain_variable_template_pattern(partial, init, info, scope, node.location);
        self.graph
            .variable_instances
            .record_partial(primary, arguments, partial, init.declarator);
        if let Some(initializer) = init.initializer {
            self.initializer_names(initializer, scope, ty)?;
        }
        Ok(true)
    }

    pub(super) fn declare_variable_template(
        &mut self,
        binding: BindingId,
        init: ast::InitDeclarator,
        info: &DeclaratorInfo,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let Some(head) = self.active_template_head(scope) else {
            return Ok(());
        };
        let entity = self.graph.binding(binding).entity;
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        let primary = if info.home.is_some() {
            self.variable_template_primary(binding).is_some()
        } else {
            self.graph.scope(home).kind == ScopeKind::Namespace
                || self.graph.entity(entity).data_member
                    == Some(super::graph::DataMemberKind::Static)
        };
        if primary {
            self.merge_template_head(entity, head, location)?;
            self.retain_variable_template_pattern(entity, init, info, scope, location);
        }
        Ok(())
    }
}
