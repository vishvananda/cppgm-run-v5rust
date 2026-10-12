// Translated from sema/analyzer_declarations.cpp's special_member,
// declare_function, redeclare_function and default_definition. See NOTICE.
//! Explicit constructor declarations have a class-owned signature index,
//! separate from the injected class name and source-facing binding order.
use super::{
    Error,
    analyzer::{Analyzer, DeferredBody},
    declarators::DeclaratorInfo,
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    specifiers::Specifiers,
    types::{Scalar, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, InitializerKind, NameKind},
};

impl Analyzer<'_> {
    pub(super) fn special_member(&mut self, id: ast::DeclId, scope: ScopeId) -> Result<(), Error> {
        let declaration = *self.tree.declarations.get(id);
        let ast::DeclKind::SpecialMember {
            specifiers,
            name,
            declarator,
            initializer,
            body,
        } = declaration.kind
        else {
            return Err(Error::Invariant(
                "ordinary declaration at special member boundary",
            ));
        };
        let location = declaration.location;
        let (specs, written_type) = self.special_specifiers(specifiers, scope)?;
        if written_type.is_some() {
            return Err(Error::at(
                location,
                "constructor, destructor or conversion function with a type",
            ));
        }
        let node = self.tree.names.get(name);
        let owner = self
            .prefix(name, scope)?
            .unwrap_or(self.declaration_scope(scope));
        if self.graph.scope(owner).kind != ScopeKind::Class {
            return Err(Error::at(
                location,
                "constructor, destructor or conversion function outside its class",
            ));
        }
        let class = self
            .graph
            .scope(owner)
            .owner
            .ok_or(Error::Invariant("class without owner"))?;
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("special member without name"))?;
        let NameKind::Identifier(label) = last.kind else {
            return Err(Error::Missing(
                "destructor and conversion function declarations",
            ));
        };
        if Some(label) != self.graph.entity(class).name {
            return Err(Error::at(
                location,
                "constructor name does not match its class",
            ));
        }
        if self.active_template_head(scope).is_some() {
            return Err(Error::Missing("constructor template declaration matching"));
        }
        if self.prefix(name, scope)?.is_some()
            && self.graph.scope(self.declaration_scope(scope)).kind != ScopeKind::Namespace
        {
            return Err(Error::at(
                location,
                "qualified name in a declaration outside namespace scope",
            ));
        }
        let result = self.graph.types.scalar(Scalar::Void)?;
        let info = self.declarator(Some(declarator), result, scope)?;
        let limit = self.graph.binding_limit();
        let binding = self.declare_constructor(class, &info, specs, scope, declaration)?;
        super::record(
            &mut self.graph.declared_bindings,
            declarator.index(),
            binding,
        );
        let function = self.graph.binding(binding).entity;
        self.record_function_exceptions(function, &info, info.lookup, location, limit)?;
        if initializer.is_some_and(|init| matches!(init.kind, InitializerKind::Default)) {
            self.default_constructor(class, &info, location)?;
            self.graph.entity_mut(function).defined = true;
        }
        // Types mode does not record/evaluate default arguments. In this mode
        // only a sole by-value class parameter triggers the copy diagnostic.
        let signature = self.graph.types.canonical_function(info.ty)?;
        if self.graph.types.function_parameter_count(signature)? == 1 {
            let parameter = self.graph.types.function_parameter(signature, 0)?;
            if matches!(self.graph.types.get(parameter).kind, TypeKind::Class(entity) if entity == class)
            {
                return Err(Error::at(
                    location,
                    "a constructor taking its own class by value",
                ));
            }
        }
        if let Some(body) = body {
            let deferred = DeferredBody {
                entity: function,
                name: label,
                ty: info.ty,
                parameters: info
                    .parameters
                    .ok_or(Error::Invariant("constructor without parameters"))?,
                body,
                parent: info.lookup,
                print: info.home.unwrap_or(scope),
                template_depth: self.template_depth,
            };
            if self.class_depth == 0 {
                self.function_body(deferred)?;
            } else {
                self.deferred.push(deferred);
            }
        }
        Ok(())
    }

    fn declare_constructor(
        &mut self,
        class: EntityId,
        info: &DeclaratorInfo,
        specs: Specifiers,
        scope: ScopeId,
        declaration: ast::Declaration,
    ) -> Result<BindingId, Error> {
        let ast::DeclKind::SpecialMember {
            initializer, body, ..
        } = declaration.kind
        else {
            return Err(Error::Invariant(
                "ordinary declaration at constructor boundary",
            ));
        };
        let definition = body.is_some();
        let location = declaration.location;
        let owner = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("constructor owner without scope"))?;
        let signature = self.graph.types.canonical_function(info.ty)?;
        let earlier = self.graph.constructor(class, signature);
        let deleted = initializer.is_some_and(|init| matches!(init.kind, InitializerKind::Delete));
        if info.home.is_some() && specs.is_static {
            return Err(Error::at(
                location,
                "static on a member function's declaration outside its class",
            ));
        }
        let TypeKind::Function { cv, reference, .. } = self.graph.types.get(signature).kind else {
            return Err(Error::Invariant("constructor without function type"));
        };
        if reference.is_some() {
            return Err(Error::at(
                location,
                "a ref-qualifier on a function that is not a non-static member function",
            ));
        }
        if cv.0 != 0 {
            return Err(Error::at(
                location,
                "a cv-qualifier on a function that is not a non-static member function",
            ));
        }
        let function = if let Some(binding) = earlier {
            let entity = self.graph.binding(binding).entity;
            if info.home.is_none() {
                return Err(Error::at(
                    location,
                    "member function redeclared in its class",
                ));
            }
            if deleted {
                return Err(Error::at(
                    location,
                    "deleted definition of a function declared before",
                ));
            }
            if definition && self.graph.entity(entity).defined {
                return Err(Error::at(location, "redefinition of a function"));
            }
            if self.graph.entity(entity).constexpr != specs.constexpr {
                return Err(Error::at(
                    location,
                    "declarations of a function that disagree on constexpr",
                ));
            }
            entity
        } else {
            if info.home.is_some() {
                return Err(Error::at(
                    location,
                    "qualified definition does not match a declaration",
                ));
            }
            self.graph
                .add_entity(Some(info.ty), self.graph.entity(class).name, Keyword::Int)?
        };
        self.graph.entity_mut(function).defined |= definition || deleted;
        self.graph.entity_mut(function).constexpr = specs.constexpr;
        let binding = self.graph.add_binding(
            owner,
            Some(info.home.unwrap_or(scope)),
            Binding {
                kind: BindingKind::Function,
                name: None,
                entity: function,
                ty: Some(info.ty),
                key: Keyword::Int,
            },
        )?;
        self.graph.record_constructor(class, signature, binding);
        Ok(binding)
    }

    fn default_constructor(
        &self,
        class: EntityId,
        info: &DeclaratorInfo,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let count = self.graph.types.function_parameter_count(info.ty)?;
        let special = if count == 0 {
            true
        } else if count == 1 {
            let parameter = self.graph.types.function_parameter(info.ty, 0)?;
            matches!(self.graph.types.get(parameter).kind,
                TypeKind::Reference { element, .. }
                if matches!(self.graph.types.get(element).kind, TypeKind::Class(entity) if entity == class))
        } else {
            false
        };
        if !special {
            return Err(Error::at(
                location,
                "only special member functions can be defaulted",
            ));
        }
        Ok(())
    }
}
