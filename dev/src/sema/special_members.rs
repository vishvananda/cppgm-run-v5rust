// Translated from sema/analyzer_declarations.cpp's special_member,
// declare_function, redeclare_function and default_definition. See NOTICE.
//! Special-member identity uses class, kind, canonical type and template shape.
//! Source labels borrow the original name and never participate in lookup.
use super::{
    Error,
    analyzer::{Analyzer, DeferredBody},
    declarators::DeclaratorInfo,
    graph::{Binding, BindingId, BindingKind, EntityId, MemberKey, MemberKind, ScopeId, ScopeKind},
    specifiers::{Specifiers, Virtuality},
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, InitializerKind, NameKind},
};

#[derive(Clone, Copy)]
pub(super) struct MemberDeclaration {
    pub location: SourceOffset,
    pub name: ast::Name,
    pub declarator: ast::DeclaratorId,
    pub initializer: Option<ast::Initializer>,
    pub body: Option<ast::BodyId>,
    pub implicit_type: bool,
}
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
        let prefix = self.prefix(name, scope)?;
        let owner = prefix.unwrap_or(self.declaration_scope(scope));
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
        let last = *self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("special member without name"))?;
        let written = if matches!(last.kind, NameKind::Conversion(_)) {
            None
        } else {
            Some(self.special_result(last.kind, class, scope, location)?)
        };
        let environment = self.declarator_environment(Some(declarator), scope)?;
        let (kind, result) = if let Some(written) = written {
            written
        } else {
            self.special_result(last.kind, class, environment.lookup, location)?
        };
        let mut info = self.declarator_in(Some(declarator), result, environment)?;
        if kind == MemberKind::Conversion {
            let TypeKind::Function {
                parameters,
                variadic,
                ..
            } = &self.graph.types.get(info.ty).kind
            else {
                return Err(Error::Invariant("conversion without function type"));
            };
            if !parameters.is_empty() || *variadic {
                return Err(Error::at(location, "a conversion function with parameters"));
            }
            self.function_specifier_type(specs, &mut info, scope)?;
        }
        if prefix.is_some()
            && self.graph.scope(self.declaration_scope(scope)).kind != ScopeKind::Namespace
        {
            return Err(Error::at(
                location,
                "qualified name in a declaration outside namespace scope",
            ));
        }
        self.complete_special_member(
            class,
            kind,
            &info,
            specs,
            scope,
            MemberDeclaration {
                location,
                name,
                declarator,
                initializer,
                body,
                implicit_type: true,
            },
        )
    }

    pub(super) fn complete_special_member(
        &mut self,
        class: EntityId,
        kind: MemberKind,
        info: &DeclaratorInfo,
        specs: Specifiers,
        scope: ScopeId,
        declaration: MemberDeclaration,
    ) -> Result<(), Error> {
        let MemberDeclaration {
            location,
            name,
            declarator,
            initializer,
            body,
            implicit_type,
        } = declaration;
        let limit = self.graph.binding_limit();
        let binding = self.declare_special_member(class, kind, info, specs, scope, declaration)?;
        super::record(
            &mut self.graph.declared_bindings,
            declarator.index(),
            binding,
        );
        let function = self.graph.binding(binding).entity;
        if kind != MemberKind::Constructor {
            self.graph.special_labels.insert(binding, name);
            if self.graph.entity(function).special_name.is_none() {
                self.graph.entity_mut(function).special_name = Some(name);
            }
        }
        self.record_function_exceptions(function, info, info.lookup, location, limit)?;
        if initializer.is_some_and(|init| matches!(init.kind, InitializerKind::Default)) {
            self.default_special_member(class, kind, info, location)?;
            self.graph.entity_mut(function).defined = true;
        }
        if kind == MemberKind::Constructor && implicit_type {
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
            self.graph.entity_mut(class).user_constructor = Some(binding);
        }
        if let Some(body) = body {
            let deferred = DeferredBody {
                entity: function,
                name: (kind == MemberKind::Constructor)
                    .then_some(self.graph.entity(class).name)
                    .flatten(),
                ty: info.ty,
                parameters: info
                    .parameters
                    .ok_or(Error::Invariant("special member without parameters"))?,
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

    pub(super) fn declared_member(
        &self,
        info: &DeclaratorInfo,
        scope: ScopeId,
    ) -> Result<Option<(EntityId, MemberKind)>, Error> {
        if !matches!(
            self.graph.types.get(info.ty).kind,
            TypeKind::Function { .. }
        ) {
            return Ok(None);
        }
        let Some(name) = info.syntax_name else {
            return Ok(None);
        };
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        if self.graph.scope(home).kind != ScopeKind::Class {
            return Ok(None);
        }
        let class = self
            .graph
            .scope(home)
            .owner
            .ok_or(Error::Invariant("class without owner"))?;
        let last = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()
            .ok_or(Error::Invariant("empty declaration name"))?;
        let kind = match last.kind {
            NameKind::Destructor(_) | NameKind::DestructorDecltype(_) => MemberKind::Destructor,
            NameKind::Identifier(id) if Some(id) == self.graph.entity(class).name => {
                MemberKind::Constructor
            }
            _ => return Ok(None),
        };
        Ok(Some((class, kind)))
    }

    fn special_result(
        &mut self,
        name: NameKind,
        class: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(MemberKind, TypeId), Error> {
        let kind = match name {
            NameKind::Identifier(label) => {
                if Some(label) != self.graph.entity(class).name {
                    return Err(Error::at(
                        location,
                        "constructor name does not match its class",
                    ));
                }
                MemberKind::Constructor
            }
            NameKind::Destructor(label) => {
                if Some(label) != self.graph.entity(class).name {
                    return Err(Error::at(
                        location,
                        "destructor name does not match its class",
                    ));
                }
                MemberKind::Destructor
            }
            NameKind::DestructorDecltype(_) => {
                return Err(Error::at(
                    location,
                    "destructor name does not match its class",
                ));
            }
            NameKind::Conversion(ty) => {
                return Ok((MemberKind::Conversion, self.type_syntax(ty, scope)?));
            }
            _ => return Err(Error::Invariant("ordinary name at special member boundary")),
        };
        Ok((kind, self.graph.types.scalar(Scalar::Void)?))
    }

    fn declare_special_member(
        &mut self,
        class: EntityId,
        kind: MemberKind,
        info: &DeclaratorInfo,
        specs: Specifiers,
        scope: ScopeId,
        declaration: MemberDeclaration,
    ) -> Result<BindingId, Error> {
        let MemberDeclaration {
            initializer, body, ..
        } = declaration;
        let location = declaration.location;
        let owner = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("special member owner without scope"))?;
        if info.home.is_some()
            && self.graph.scope(self.declaration_scope(scope)).kind != ScopeKind::Namespace
        {
            return Err(Error::at(
                location,
                "qualified name in a declaration outside namespace scope",
            ));
        }
        if info.home.is_some() && specs.is_static {
            return Err(Error::at(
                location,
                "static on a member function's declaration outside its class",
            ));
        }
        let head = self.active_template_head(scope);
        if head.is_some() && specs.virtuality == Virtuality::Virtual {
            return Err(Error::at(location, "a virtual member function template"));
        }
        if head.is_some() && kind == MemberKind::Destructor {
            return Err(Error::at(location, "a destructor declared as a template"));
        }
        let mut signature = self.graph.types.canonical_function(info.ty)?;
        if head.is_none() && kind != MemberKind::Conversion {
            let TypeKind::Function {
                result,
                parameters,
                variadic,
                cv,
                reference,
            } = &self.graph.types.get(signature).kind
            else {
                return Err(Error::Invariant("special member without function type"));
            };
            if !matches!(
                self.graph.types.get(*result).kind,
                TypeKind::Scalar(Scalar::Void)
            ) {
                let parameters = parameters.clone();
                let (variadic, cv, reference) = (*variadic, *cv, *reference);
                let result = self.graph.types.scalar(Scalar::Void)?;
                signature = self.graph.types.intern(
                    TypeKind::Function {
                        result,
                        parameters,
                        variadic,
                        cv,
                        reference,
                    },
                    Cv::default(),
                )?;
            }
        }
        let key = MemberKey {
            class,
            kind,
            signature,
            shape: head.map(|id| self.graph.templates.shape(id)),
        };
        let earlier = if kind == MemberKind::Constructor && head.is_none() {
            self.graph.constructor(class, signature)
        } else {
            self.graph.member_function(key)
        };
        let deleted = initializer.is_some_and(|init| matches!(init.kind, InitializerKind::Delete));
        let TypeKind::Function { cv, reference, .. } = self.graph.types.get(signature).kind else {
            return Err(Error::Invariant("special member without function type"));
        };
        if kind != MemberKind::Conversion || specs.is_static {
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
        }
        if earlier.is_none() && head.is_none() && kind == MemberKind::Conversion {
            self.check_conversion_overload(key, specs.is_static, location)?;
        }
        let function = if let Some(binding) = earlier {
            let entity = self.graph.binding(binding).entity;
            let old = self
                .graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("function without type"))?;
            self.check_function_type_agreement(old, info.ty, location)?;
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
            if body.is_some() && self.graph.entity(entity).defined {
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
            self.graph.add_entity(
                Some(info.ty),
                if kind == MemberKind::Constructor {
                    self.graph.entity(class).name
                } else {
                    None
                },
                Keyword::Int,
            )?
        };
        if let Some(head) = head {
            self.merge_template_head(function, head, location)?;
        }
        self.graph.entity_mut(function).defined |= body.is_some() || deleted;
        self.graph.entity_mut(function).constexpr = specs.constexpr;
        self.graph.entity_mut(function).function = Some(super::graph::FunctionProperties {
            internal_linkage: false,
            is_static: specs.is_static,
            special: None,
            allocation: None,
        });
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
        self.graph.record_member_function(key, binding);
        Ok(binding)
    }

    fn check_conversion_overload(
        &self,
        key: MemberKey,
        is_static: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Function {
            reference: current_ref,
            ..
        } = self.graph.types.get(key.signature).kind
        else {
            return Err(Error::Invariant("conversion overload without function"));
        };
        for cv in 0..=3 {
            for reference in [None, Some(RefKind::Lvalue), Some(RefKind::Rvalue)] {
                let Some(signature) =
                    self.graph
                        .types
                        .function_variant(key.signature, Cv(cv), reference)?
                else {
                    continue;
                };
                let Some(binding) = self.graph.member_function(MemberKey { signature, ..key })
                else {
                    continue;
                };
                let other_static = self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .function
                    .is_some_and(|facts| facts.is_static);
                if other_static != is_static {
                    return Err(Error::at(
                        location,
                        "a static and a non-static member function with the same parameters",
                    ));
                }
                if current_ref.is_some() != reference.is_some() {
                    return Err(Error::at(
                        location,
                        "member functions with the same parameters, some with a ref-qualifier and some without",
                    ));
                }
            }
        }
        Ok(())
    }

    fn default_special_member(
        &self,
        class: EntityId,
        kind: MemberKind,
        info: &DeclaratorInfo,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let count = self.graph.types.function_parameter_count(info.ty)?;
        let special = match kind {
            MemberKind::Destructor => true,
            MemberKind::Constructor if count == 0 => true,
            MemberKind::Constructor if count == 1 => {
                let parameter = self.graph.types.function_parameter(info.ty, 0)?;
                matches!(self.graph.types.get(parameter).kind,
                    TypeKind::Reference { element, .. }
                    if matches!(self.graph.types.get(element).kind, TypeKind::Class(entity) if entity == class))
            }
            MemberKind::Constructor | MemberKind::Conversion => false,
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
