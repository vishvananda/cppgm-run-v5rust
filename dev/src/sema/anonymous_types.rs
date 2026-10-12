// Translated from sema/analyzer_classes.cpp's unnamed type/union declarations. See NOTICE.
//! Declaration-point naming is presentation; injected members share entity identity.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, ScopeId, ScopeKind},
    specifiers::Specifiers,
    types::{TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast,
};

#[derive(Clone, Copy)]
pub(super) enum AnonymousClass {
    Pending(ast::TokenSpan),
    Labeled(ast::TokenSpan),
}
impl AnonymousClass {
    fn tokens(self) -> ast::TokenSpan {
        match self {
            Self::Pending(tokens) | Self::Labeled(tokens) => tokens,
        }
    }
    pub(super) fn label(self) -> Option<ast::TokenSpan> {
        match self {
            Self::Pending(_) => None,
            Self::Labeled(tokens) => Some(tokens),
        }
    }
}

impl Analyzer<'_> {
    pub(super) fn simple_declarator_name(
        &self,
        mut id: ast::DeclaratorId,
    ) -> Result<Option<NameId>, Error> {
        loop {
            let node = self.tree.declarators.get(id);
            if let Some(name) = node.name {
                let name = self.tree.names.get(name);
                let components = self.tree.name_lists.get(name.components);
                return Ok(if !name.global && components.len() == 1 {
                    if let ast::NameKind::Identifier(name) = components[0].kind {
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    None
                });
            }
            id = node
                .nested
                .ok_or(Error::Invariant("declarator without name"))?;
        }
    }

    pub(super) fn name_unnamed_class(&mut self, ty: TypeId, name: Option<NameId>) {
        let TypeKind::Class(entity) = self.graph.types.get(ty).kind else {
            return;
        };
        let Some(anonymous) = self.graph.entity(entity).anonymous_class else {
            return;
        };
        if self.graph.entity(entity).name.is_some() {
            return;
        }
        if let Some(name) = name {
            self.graph.entity_mut(entity).name = Some(name);
            if let Some(scope) = self.graph.entity(entity).scope {
                self.graph.scope_mut(scope).name = Some(name);
            }
        } else {
            self.graph.entity_mut(entity).anonymous_class =
                Some(AnonymousClass::Labeled(anonymous.tokens()));
        }
        // As in name_unnamed_type, the placeholder's printed name changes,
        // without publishing a class-name lookup binding or injected-class-name.
    }

    pub(super) fn anonymous_union(
        &mut self,
        specs: Specifiers,
        scope: ScopeId,
        location: SourceOffset,
        after_semicolon: bool,
    ) -> Result<(), Error> {
        let Some(declared) = specs.declared else {
            return Ok(());
        };
        let TypeKind::Class(entity) = self.graph.types.get(declared).kind else {
            return Ok(());
        };
        let Some(anonymous) = self.graph.entity(entity).anonymous_class else {
            return Ok(());
        };
        if self.graph.entity(entity).name.is_some() {
            return Ok(());
        }
        if specs.virtuality == super::specifiers::Virtuality::Virtual {
            return Err(Error::at(
                location,
                "virtual in a declaration that declares no function",
            ));
        }
        if specs.constexpr {
            return Err(Error::at(
                location,
                "constexpr in a declaration that declares no variable or function",
            ));
        }
        let kind = self.graph.scope(scope).kind;
        if self.graph.entity(entity).key != Keyword::Union && kind != ScopeKind::Class {
            return Err(Error::at(location, "declaration does not declare anything"));
        }
        let unnamed_namespace = kind == ScopeKind::Namespace
            && scope != self.graph.global
            && self.graph.scope(scope).name.is_none();
        if kind == ScopeKind::Namespace && !specs.is_static && !unnamed_namespace {
            return Err(Error::at(
                location,
                "anonymous union in a named or the global namespace must be static",
            ));
        }
        let mut tokens = anonymous.tokens();
        if after_semicolon {
            // The class-key-only declaration's label is assigned after its ';'.
            tokens.end = tokens
                .end
                .checked_add(1)
                .ok_or(Error::Limit("token ordinal limit"))?;
        }
        self.graph.entity_mut(entity).anonymous_class = Some(AnonymousClass::Labeled(tokens));
        let class = self
            .graph
            .entity(entity)
            .scope
            .ok_or(Error::Invariant("anonymous class without scope"))?;
        // Read only this class's source-order bindings. No enclosing-scope scan.
        for index in 0..self.graph.scope(class).bindings.len() {
            let member = self.graph.binding(self.graph.scope(class).bindings[index]);
            if member.kind == BindingKind::Type && self.graph.entity(member.entity).name.is_none() {
                continue;
            }
            if member.kind != BindingKind::Variable {
                return Err(Error::at(
                    location,
                    "an anonymous union can only have non-static data members",
                ));
            }
            if member
                .name
                .is_some_and(|name| self.graph.scope(scope).index.contains_key(&name))
            {
                return Err(Error::at(
                    location,
                    "anonymous union member conflicts with a declaration in the enclosing scope",
                ));
            }
            self.graph.add_binding(scope, Some(scope), member)?;
        }
        if kind == ScopeKind::Class {
            let owner = self
                .graph
                .scope(scope)
                .owner
                .ok_or(Error::Invariant("class without owner"))?;
            let storage = self.graph.add_entity(Some(specs.ty), None, Keyword::Int)?;
            self.graph.entity_mut(owner).members.push(storage);
            self.graph.add_binding(
                scope,
                None,
                Binding {
                    kind: BindingKind::Variable,
                    name: None,
                    entity: storage,
                    ty: Some(specs.ty),
                    key: Keyword::Int,
                },
            )?;
        }
        Ok(())
    }
}
