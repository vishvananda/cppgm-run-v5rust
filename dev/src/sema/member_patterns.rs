// Translated from class_templates.cpp::qualifying_template/link_member_patterns,
// template_names.cpp::outside_frames and analyzer_types.cpp::qualified_lookup_scope.
// See NOTICE.
//! Declaration-owned bridges and positional links from copied instance members.
use super::{
    Error,
    analyzer::{Analyzer, DeferredBody},
    graph::{Binding, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::Filter,
    templates::HeadId,
    types::TypeId,
};
use crate::syntax::ast;

impl Analyzer<'_> {
    fn declared_syntax_name(&self, declaration: ast::DeclId) -> Option<ast::Name> {
        let declarator = match self.tree.declarations.get(declaration).kind {
            ast::DeclKind::Function { declarator, .. }
            | ast::DeclKind::SpecialMember { declarator, .. } => Some(declarator),
            ast::DeclKind::Simple { declarators, .. } => self
                .tree
                .init_declarators
                .get(declarators)
                .first()
                .map(|node| node.declarator),
            ast::DeclKind::Class(specifier) => {
                return match specifier.kind {
                    ast::SpecifierKind::Class(class) => self.tree.classes.get(class).name,
                    ast::SpecifierKind::Elaborated { name, .. } => Some(name),
                    _ => None,
                };
            }
            ast::DeclKind::Template { declaration, .. } => {
                return self.declared_syntax_name(declaration);
            }
            ast::DeclKind::Enum { name, .. } => return name,
            _ => None,
        };
        let mut current = declarator;
        while let Some(id) = current {
            let node = self.tree.declarators.get(id);
            if node.name.is_some() {
                return node.name;
            }
            current = node.nested;
        }
        None
    }

    pub(super) fn qualifying_template(
        &mut self,
        declaration: ast::DeclId,
        scope: ScopeId,
    ) -> Result<Option<EntityId>, Error> {
        let Some(name) = self.declared_syntax_name(declaration) else {
            return Ok(None);
        };
        let node = *self.tree.names.get(name);
        let count = self
            .tree
            .name_lists
            .get(node.components)
            .len()
            .saturating_sub(1);
        let mut prefix = node.global.then_some(self.graph.global);
        let mut owner = None;
        // Like the oracle's preliminary probe, this discovers the template
        // owner without instantiating its definition or checking its arguments.
        for index in 0..count {
            let component = self.tree.name_lists.get(node.components)[index];
            let ast::NameKind::Identifier(key) = component.kind else {
                break;
            };
            let found = if let Some(prefix) = prefix {
                self.qualified(prefix, key, Filter::Prefix)
            } else {
                self.unqualified(scope, key, Filter::Prefix)
            };
            let found = match found {
                Ok(found) => found,
                Err(Error::Diagnostic {
                    message: "ambiguous name",
                    ..
                }) => break,
                Err(error) => return Err(error),
            };
            let Some(found) = found else { break };
            let binding = self.graph.binding(found);
            if component.arguments.is_some()
                && binding.kind == BindingKind::Type
                && self.graph.entity(binding.entity).template_head.is_some()
            {
                owner = Some(binding.entity);
            }
            let Some(next) = self.graph.entity(binding.entity).scope else {
                break;
            };
            prefix = Some(next);
        }
        Ok(owner)
    }

    fn enclosing_template_lists(&self, mut scope: ScopeId) -> usize {
        let mut count = 0;
        loop {
            let node = self.graph.scope(scope);
            if !matches!(node.kind, ScopeKind::Class | ScopeKind::TemplateParameters) {
                break;
            }
            if node.kind == ScopeKind::Class
                && node
                    .owner
                    .is_some_and(|owner| self.graph.entity(owner).template_head.is_some())
            {
                count += 1;
            }
            let Some(parent) = node.parent else { break };
            scope = parent;
        }
        count
    }

    pub(super) fn member_template_head(
        &self,
        scope: ScopeId,
        home: Option<ScopeId>,
    ) -> Option<HeadId> {
        let head = self.active_template_head(scope)?;
        if home.is_some_and(|home| self.graph.encloses(home, scope)) {
            return Some(head);
        }
        let enclosing = home.map_or(0, |home| self.enclosing_template_lists(home));
        let mut lists = 0;
        let mut current = Some(scope);
        while let Some(scope) = current {
            let node = self.graph.scope(scope);
            if node.kind != ScopeKind::TemplateParameters {
                break;
            }
            lists += 1;
            current = node.parent;
        }
        (lists > enclosing).then_some(head)
    }

    fn member_hides_parameter(
        &self,
        mut scope: ScopeId,
        name: crate::lex::identifiers::NameId,
    ) -> Result<bool, Error> {
        loop {
            let node = self.graph.scope(scope);
            if !matches!(node.kind, ScopeKind::Class | ScopeKind::TemplateParameters) {
                return Ok(false);
            }
            if node.kind == ScopeKind::Class
                && self
                    .class_lookup(scope, name.into(), Filter::Ordinary)?
                    .binding
                    .is_some()
            {
                return Ok(true);
            }
            let Some(parent) = node.parent else {
                return Ok(false);
            };
            scope = parent;
        }
    }

    pub(super) fn member_lookup_bridge(
        &mut self,
        scope: ScopeId,
        home: ScopeId,
    ) -> Result<ScopeId, Error> {
        if self.graph.scope(scope).kind != ScopeKind::TemplateParameters {
            let mut current = Some(home);
            let mut frames = false;
            while let Some(scope) = current {
                let node = self.graph.scope(scope);
                if node.kind == ScopeKind::Namespace {
                    break;
                }
                frames |= node.kind == ScopeKind::TemplateParameters;
                current = node.parent;
            }
            if !frames {
                return Ok(home);
            }
        }
        let bridge = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, home, None, None)?;
        self.graph.scope_mut(bridge).template_bridge = true;
        let mut headers = Vec::new();
        let mut current = scope;
        while self.graph.scope(current).kind == ScopeKind::TemplateParameters {
            headers.push(current);
            let Some(parent) = self.graph.scope(current).parent else {
                break;
            };
            current = parent;
        }
        let enclosing = self.enclosing_template_lists(home);
        for (level, &header) in headers.iter().enumerate() {
            let own = level + enclosing < headers.len();
            for index in 0..self.graph.scope(header).bindings.len() {
                let binding = self.graph.binding(self.graph.scope(header).bindings[index]);
                let Some(name) = binding.name else { continue };
                if self
                    .graph
                    .entity(binding.entity)
                    .template_parameter
                    .is_none()
                    || self.graph.own(bridge, name, Filter::Ordinary).is_some()
                    || !own && self.member_hides_parameter(home, name)?
                {
                    continue;
                }
                self.graph.add_binding(bridge, None, binding)?;
            }
        }
        Ok(bridge)
    }

    pub(super) fn outside_member_frame(
        &mut self,
        entity: EntityId,
        pattern: DeferredBody,
    ) -> Result<ScopeId, Error> {
        let owner = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("member without owner"))?;
        self.outside_class_frame(owner, pattern.definition_head, pattern.parent)
    }

    pub(super) fn outside_class_frame(
        &mut self,
        owner: ScopeId,
        head: Option<HeadId>,
        bridge: ScopeId,
    ) -> Result<ScopeId, Error> {
        self.outside_lookup_frame(owner, owner, head, bridge)
    }

    pub(super) fn outside_lookup_frame(
        &mut self,
        owner: ScopeId,
        lookup: ScopeId,
        head: Option<HeadId>,
        bridge: ScopeId,
    ) -> Result<ScopeId, Error> {
        let mut enclosing = Vec::new();
        let mut current = Some(owner);
        while let Some(scope) = current {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::Namespace {
                break;
            }
            if let Some(class) = node.owner
                && let Some(instance) = self.graph.class_templates.instance(class)
            {
                enclosing.push((class, instance.arguments.clone()));
            }
            current = node.parent;
        }
        let mut header = head.map(|head| self.graph.templates.parameter_scope(head));
        let mut headers = Vec::new();
        while let Some(scope) = header {
            let node = self.graph.scope(scope);
            if node.kind != ScopeKind::TemplateParameters {
                break;
            }
            headers.push(scope);
            header = node.parent;
        }
        let mut frame = lookup;
        for index in (0..headers.len().min(enclosing.len())).rev() {
            frame = self.bound_member_frame(headers[index], &enclosing[index].1, frame, bridge)?;
        }
        Ok(frame)
    }

    fn bound_member_frame(
        &mut self,
        header: ScopeId,
        arguments: &[TypeId],
        parent: ScopeId,
        bridge: ScopeId,
    ) -> Result<ScopeId, Error> {
        let frame = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, parent, None, None)?;
        self.graph.scope_mut(frame).template_bridge = true;
        for index in 0..self.graph.scope(header).bindings.len() {
            let binding = self.graph.binding(self.graph.scope(header).bindings[index]);
            let Some(name) = binding.name else { continue };
            if self.graph.own(bridge, name, Filter::Ordinary).is_none() {
                continue;
            }
            let Some(info) = self.graph.entity(binding.entity).template_parameter else {
                continue;
            };
            let Some(position) = info.ordinal() else {
                continue;
            };
            let ty = *arguments
                .get(position)
                .ok_or(Error::Invariant("outside frame without argument"))?;
            self.graph.add_binding(
                frame,
                None,
                Binding {
                    kind: BindingKind::Alias,
                    ty: Some(ty),
                    ..binding
                },
            )?;
        }
        Ok(frame)
    }

    pub(super) fn link_member_patterns(&mut self, pattern: ast::ClassId, copy: ast::ClassId) {
        let a = self.tree.classes.get(pattern).members;
        let b = self.tree.classes.get(copy).members;
        for index in 0..self.tree.decl_lists.get(a).len() {
            self.link_member_declarations(
                self.tree.decl_lists.get(a)[index],
                self.tree.decl_lists.get(b)[index],
            );
        }
    }
    fn link_member_declarations(&mut self, from: ast::DeclId, to: ast::DeclId) {
        match (
            self.tree.declarations.get(from).kind,
            self.tree.declarations.get(to).kind,
        ) {
            (
                ast::DeclKind::Function { declarator: a, .. }
                | ast::DeclKind::SpecialMember { declarator: a, .. },
                ast::DeclKind::Function { declarator: b, .. }
                | ast::DeclKind::SpecialMember { declarator: b, .. },
            ) => self.link_member_declarators(a, b),
            (
                ast::DeclKind::Simple { declarators: a, .. },
                ast::DeclKind::Simple { declarators: b, .. },
            ) => {
                for index in 0..self.tree.init_declarators.get(a).len() {
                    self.link_member_declarators(
                        self.tree.init_declarators.get(a)[index].declarator,
                        self.tree.init_declarators.get(b)[index].declarator,
                    );
                }
            }
            _ => (),
        }
    }
    fn link_member_declarators(&mut self, from: ast::DeclaratorId, to: ast::DeclaratorId) {
        let source = self
            .graph
            .declared_bindings
            .get(from.index())
            .copied()
            .flatten();
        let target = self
            .graph
            .declared_bindings
            .get(to.index())
            .copied()
            .flatten();
        if let (Some(source), Some(target)) = (source, target) {
            let source = self.graph.binding(source);
            let target = self.graph.binding(target);
            if source.kind == BindingKind::Variable
                && target.kind == BindingKind::Variable
                && self.graph.entity(target.entity).data_member
                    == Some(super::graph::DataMemberKind::Static)
            {
                self.graph
                    .variable_instances
                    .link(source.entity, target.entity);
                self.graph
                    .instantiation_levels
                    .record(target.entity, self.instantiation_depth);
            }
            if source.kind == BindingKind::Function
                && target.kind == BindingKind::Function
                && self
                    .graph
                    .function_templates
                    .primary(target.entity)
                    .is_none()
                && self.graph.entity(target.entity).template_head.is_none()
                && (self
                    .graph
                    .entity(target.entity)
                    .home
                    .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class)
                    || self.graph.function_templates.bodies.member(target.entity))
            {
                self.graph
                    .function_templates
                    .bodies
                    .link_member(source.entity, target.entity);
                self.graph.entity_mut(target.entity).definition_emission =
                    super::function_demands::DefinitionEmission::Demanded;
                self.graph
                    .instantiation_levels
                    .record(target.entity, self.instantiation_depth);
            }
        }
    }
}
