// Translated from sema/analyzer_expressions.cpp's member_access,
// member_qualifier and member_of_object, and analyzer_access.cpp. See NOTICE.
//! Member designation retains lookup, receiver type/category and naming-class facts.
use super::{
    Error,
    analyzer::Analyzer,
    constants::ConstantState,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{Access, BindingId, BindingKind, DataMemberKind, EntityId, Mutability, ScopeId},
    lookup::Filter,
    types::{Cv, TypeKind},
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    support::source::SourceOffset,
    syntax::ast::{self, NameKind},
};

impl Analyzer<'_> {
    pub(super) fn current_object_type(&mut self) -> Result<Option<super::types::TypeId>, Error> {
        if self.default_argument_scope.is_some() {
            return Ok(None);
        }
        if let Some(class) = self.member_initializer_class {
            return Ok(Some(self.graph.lower_entity_type(class)?));
        }
        let Some(body) = &self.body_context else {
            return Ok(None);
        };
        let entity = self.graph.entity(body.entity);
        if entity.function.is_some_and(|function| function.is_static) {
            return Ok(None);
        }
        let Some(class) = entity
            .home
            .and_then(|home| self.graph.scope(home).owner)
            .filter(|&class| {
                self.graph
                    .entity(class)
                    .ty
                    .is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)))
            })
        else {
            return Ok(None);
        };
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("object class without type"))?;
        let function = entity
            .ty
            .ok_or(Error::Invariant("member function without type"))?;
        let TypeKind::Function { cv, .. } = self.graph.types.get(function).kind else {
            return Err(Error::Invariant("member without function type"));
        };
        Ok(Some(self.graph.types.qualify(ty, cv)?))
    }

    pub(super) fn implicit_member_object(
        &mut self,
        binding: BindingId,
    ) -> Result<Option<super::expression_facts::ObjectArgument>, Error> {
        let Some(ty) = self.current_object_type()? else {
            return Ok(None);
        };
        let Some(home) = self
            .graph
            .entity(self.graph.binding(binding).entity)
            .home
            .and_then(|home| self.graph.scope(home).owner)
        else {
            return Ok(None);
        };
        let Some(class) = self.graph.class_of(ty) else {
            return Ok(None);
        };
        Ok(
            (home == class || self.graph.is_base_of(home, class)).then_some(
                super::expression_facts::ObjectArgument {
                    source: super::expression_facts::ObjectSource::Implicit,
                    ty,
                    category: V::Lvalue,
                },
            ),
        )
    }

    pub(super) fn data_member_expression(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        op: ast::Operator,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let object = self.typed_expression(base, scope)?;
        self.check_resolved_function(base)?;
        let location = self.tree.expressions.get(id).location;
        let arrow = op.token.as_punctuator() == Some(P::Arrow);
        let mut ty = object.ty;
        if arrow {
            if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                return Err(Error::Missing("class operator-> calls"));
            }
            let pointer = self.decayed_type(ty)?;
            let TypeKind::Pointer(element) = self.graph.types.get(pointer).kind else {
                return Err(Error::at(location, "-> requires a pointer to a class"));
            };
            self.convert_operand_scalar(base, pointer, false)?;
            ty = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Err(Error::at(location, "member access on a non-class"));
        };
        let class_scope = self.graph.entity(class).scope;
        if class_scope.is_none()
            || !self.graph.entity(class).complete
                && !class_scope.is_some_and(|class_scope| self.graph.encloses(class_scope, scope))
        {
            return Err(Error::at(location, "member access on an incomplete class"));
        }
        let node = *self.tree.names.get(name);
        let last = *self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty member name"))?;
        let NameKind::Identifier(key) = last.kind else {
            return Err(Error::Missing("operator and special member access"));
        };
        if last.arguments.is_some() {
            return Err(Error::Missing("member template specialization lookup"));
        }
        let search = self.data_member_qualifier(name, class, scope)?;
        self.lookup_location = node.location;
        let found = self
            .qualified_result(search, key.into(), Filter::Ordinary)
            .map_err(|error| match error {
                Error::Diagnostic {
                    message: "ambiguous name",
                    ..
                } => Error::at(node.location, "ambiguous member name"),
                other => other,
            })?;
        let binding = found
            .binding
            .ok_or_else(|| Error::at(node.location, "no member of that name"))?;
        let naming = self
            .graph
            .scope(search)
            .owner
            .ok_or(Error::Invariant("member search without class"))?;
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        if entity.data_member == Some(DataMemberKind::Instance)
            || self.member_function_owner(binding).is_some()
        {
            let home = entity
                .home
                .and_then(|home| self.graph.scope(home).owner)
                .ok_or(Error::Invariant("member without class"))?;
            if self.graph.base_count(class, naming) > 1 || self.graph.base_count(naming, home) > 1 {
                return Err(Error::at(
                    node.location,
                    "a non-static member of an ambiguous base class subobject",
                ));
            }
            if class != naming {
                let path = self.graph.retain_base_path(class, naming)?;
                self.check_base_path_access(path, scope, node.location)?;
            }
        }
        if self.graph.binding(binding).kind == BindingKind::Function {
            let mut facts = self.function_designation(binding, found.functions.as_slice())?;
            facts.category = if found.functions.as_slice().len() == 1
                && self.member_function_owner(binding).is_none()
            {
                V::Lvalue
            } else {
                V::Prvalue
            };
            facts.object = Some(super::expression_facts::ObjectArgument {
                source: super::expression_facts::ObjectSource::Explicit {
                    expression: base,
                    arrow,
                },
                ty,
                category: if arrow { V::Lvalue } else { object.category },
            });
            facts.member_naming = Some(naming);
            super::record(&mut self.graph.resolved_names, name.index(), binding);
            return Ok(facts);
        }
        self.check_named_member_access(binding, naming, class, scope, node.location)?;
        if self.lowering()
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .data_member
                == Some(DataMemberKind::Instance)
        {
            self.class_layout(class, location)?;
        }
        super::record(&mut self.graph.resolved_names, name.index(), binding);
        let member = self.graph.binding(binding);
        let entity = self.graph.entity(member.entity);
        let mut facts = match member.kind {
            BindingKind::Variable => {
                let mut member_ty = entity.ty.ok_or(Error::Invariant("member without type"))?;
                let mut category = if arrow || object.category == V::Lvalue {
                    V::Lvalue
                } else {
                    V::Xvalue
                };
                if let TypeKind::Reference { element, .. } = self.graph.types.get(member_ty).kind {
                    member_ty = element;
                    category = V::Lvalue;
                } else if entity.data_member == Some(DataMemberKind::Instance) {
                    let cv = self.graph.types.get(ty).cv;
                    let cv = Cv(cv.0
                        & if entity.mutability == Mutability::Mutable {
                            !Cv::CONST.0
                        } else {
                            u8::MAX
                        });
                    member_ty = self.graph.types.qualify(member_ty, cv)?;
                }
                if self.graph.entity(member.entity).data_member == Some(DataMemberKind::Static) {
                    category = V::Lvalue;
                }
                self.new_expression_facts(member_ty, category)?
            }
            BindingKind::Enumerator => {
                let ConstantState::Value(value) = entity.constant else {
                    return Err(Error::Invariant("enumerator member without value"));
                };
                let mut facts = self.new_expression_facts(value.ty, V::Prvalue)?;
                facts.folded = Some(value.bits);
                facts
            }
            _ => {
                return Err(Error::at(
                    location,
                    "member name does not denote a data member or member function",
                ));
            }
        };
        facts.member = Some(binding);
        self.static_member_constant(&mut facts, member.entity);
        facts.member_naming = Some(naming);
        facts.member_path = self.data_member_path(binding, naming, location)?;
        if class != naming
            && self.graph.entity(member.entity).data_member == Some(DataMemberKind::Instance)
        {
            facts.member_naming_path = Some(self.graph.retain_base_path(class, naming)?);
        }
        Ok(facts)
    }

    pub(super) fn data_member_path(
        &mut self,
        binding: BindingId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<Option<super::inheritance::BasePathId>, Error> {
        let member = self.graph.entity(self.graph.binding(binding).entity);
        if member.data_member != Some(DataMemberKind::Instance) {
            return Ok(None);
        }
        let holder = member
            .home
            .and_then(|home| self.graph.scope(home).owner)
            .ok_or(Error::Invariant("member without holder"))?;
        // Anonymous injection owns a separate subobject route, not a base edge.
        if self.graph.entity(holder).anonymous_storage.is_some() {
            return Ok(None);
        }
        if class == holder {
            return Ok(None);
        }
        let path = self.graph.retain_base_path(class, holder)?;
        if self.lowering() {
            self.request_storage_layout(class, location)?;
        }
        Ok(Some(path))
    }

    fn data_member_qualifier(
        &mut self,
        name: ast::Name,
        class: EntityId,
        scope: ScopeId,
    ) -> Result<ScopeId, Error> {
        let class_scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("member class without scope"))?;
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if !node.global && components.len() == 1 {
            return Ok(class_scope);
        }
        let mut prefix = node.global.then_some(self.graph.global);
        for component in &components[..components.len() - 1] {
            if component.arguments.is_some() {
                return Err(Error::Missing(
                    "template specialization in a member qualifier",
                ));
            }
            let NameKind::Identifier(key) = component.kind else {
                return Err(Error::at(
                    node.location,
                    "unsupported qualified member name",
                ));
            };
            self.lookup_location = node.location;
            let found = if let Some(prefix) = prefix {
                self.qualified(prefix, key, Filter::Prefix)?
            } else if let Some(binding) = self.qualified(class_scope, key, Filter::Prefix)? {
                self.check_named_member_access(binding, class, class, scope, node.location)?;
                Some(binding)
            } else {
                self.unqualified(scope, key, Filter::Prefix)?
            };
            let binding = found
                .ok_or_else(|| Error::at(node.location, "undeclared name in a member access"))?;
            let binding = self.graph.binding(binding);
            prefix = self.graph.entity(binding.entity).scope.or_else(|| {
                binding
                    .ty
                    .and_then(|ty| match self.graph.types.get(ty).kind {
                        TypeKind::Class(class) | TypeKind::Enum(class) => {
                            self.graph.entity(class).scope
                        }
                        _ => None,
                    })
            });
            if prefix.is_none() {
                return Err(Error::at(
                    node.location,
                    "name before :: is not a class, namespace or enumeration",
                ));
            }
        }
        if !prefix
            .and_then(|scope| self.graph.scope(scope).owner)
            .is_some_and(|base| base == class || self.graph.is_base_of(base, class))
        {
            return Err(Error::at(
                node.location,
                "qualified member name does not name a base class",
            ));
        }
        prefix.ok_or(Error::Invariant("member qualifier without scope"))
    }

    pub(super) fn check_direct_member_access(
        &self,
        binding: BindingId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.direct_member_accessible(binding, scope) {
            return Err(Error::at(location, "inaccessible member"));
        }
        Ok(())
    }

    pub(super) fn check_special_member_access(
        &self,
        binding: BindingId,
        object: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_direct_member_access(binding, scope, location)?;
        let owner = self
            .graph
            .scope(self.graph.binding_scope(binding))
            .owner
            .ok_or(Error::Invariant("special member without class"))?;
        if self.graph.binding_access(binding) == Access::Protected
            && !self.protected_object_accessible(scope, owner, object)
        {
            return Err(Error::at(
                location,
                "protected member accessed through an object of another class",
            ));
        }
        Ok(())
    }

    pub(super) fn direct_member_accessible(&self, binding: BindingId, scope: ScopeId) -> bool {
        let home = self.graph.binding_scope(binding);
        let owner = self.graph.scope(home).owner;
        self.graph.binding_access(binding) == Access::Public
            || owner.is_some_and(|class| self.member_or_friend(scope, class))
            || self.graph.binding_access(binding) == Access::Protected
                && owner.is_some_and(|class| self.class_context(scope, class))
    }

    pub(super) fn check_named_member_access(
        &self,
        binding: BindingId,
        naming: EntityId,
        object: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.validate_member_access(binding, naming, Some(object), scope, location)
    }
    pub(super) fn validate_member_access(
        &self,
        binding: BindingId,
        naming: EntityId,
        object: Option<EntityId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_direct_member_access(binding, scope, location)?;
        let home = self.graph.binding_scope(binding);
        let Some(owner) = self.graph.scope(home).owner else {
            return Ok(());
        };
        if naming != owner && !self.accessible_base_name(naming, owner, scope) {
            return Err(Error::at(location, "inaccessible member"));
        }
        if self.graph.binding_access(binding) == Access::Protected
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .data_member
                == Some(DataMemberKind::Instance)
            && object.is_some_and(|object| !self.protected_object_accessible(scope, owner, object))
        {
            return Err(Error::at(
                location,
                "protected member accessed through an object of another class",
            ));
        }
        Ok(())
    }
}
