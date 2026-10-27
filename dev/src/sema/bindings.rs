// Translated from sema/analyzer_bindings.cpp. See NOTICE.
//! Declarations retain a hidden storage object and typed aliases with no storage of their own.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, EntityId, Graph, Mutability, ScopeId},
    lookup::Filter,
    specifiers::Specifiers,
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
pub(crate) enum BindingTarget {
    Element(u64),
    Member {
        member: EntityId,
        path: Option<super::inheritance::BasePathId>,
    },
    Dependent,
}
#[derive(Clone, Copy)]
pub(crate) struct BindingAlias {
    pub object: EntityId,
    pub object_type: TypeId,
    pub target: BindingTarget,
}
#[derive(Default)]
pub(super) struct Bindings {
    aliases: HashMap<EntityId, BindingAlias>,
    objects: HashMap<EntityId, Vec<EntityId>>,
    local_counts: HashMap<(EntityId, crate::lex::identifiers::NameId), u32>,
    local_discriminators: HashMap<EntityId, u16>,
}
impl Graph {
    pub(crate) fn lower_binding_alias(&self, entity: EntityId) -> Option<BindingAlias> {
        self.decompositions.aliases.get(&entity).copied()
    }
    pub(crate) fn lower_decomposition(&self, entity: EntityId) -> Option<&[EntityId]> {
        self.decompositions.objects.get(&entity).map(Vec::as_slice)
    }
    pub(crate) fn lower_decomposition_discriminator(&self, entity: EntityId) -> u32 {
        self.decompositions
            .local_discriminators
            .get(&entity)
            .copied()
            .map_or(0, u32::from)
    }
}
impl Analyzer<'_> {
    pub(super) fn structured_binding_declaration(
        &mut self,
        specifiers: Specifiers,
        declarators: List<ast::InitDeclarator>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        use super::specifiers::{
            DeclarationRole, Explicitness, Friendship, InlinePreference, Virtuality,
        };
        if !matches!(self.graph.types.get(specifiers.ty).kind, TypeKind::Auto(_))
            || specifiers.role == DeclarationRole::Typedef
            || specifiers.constexpr
            || specifiers.inline == InlinePreference::Inline
            || specifiers.is_extern
            || specifiers.friendship == Friendship::Friend
            || specifiers.virtuality == Virtuality::Virtual
            || specifiers.explicitness == Explicitness::Explicit
            || specifiers.mutability == Mutability::Mutable
        {
            return Err(Error::at(
                location,
                "a structured binding declaration's decl-specifiers are auto and cv-qualifiers (11.5p1)",
            ));
        }
        if self.graph.scope(scope).kind == super::graph::ScopeKind::TemplateParameters {
            return Err(Error::at(
                location,
                "a structured binding declaration as a template declaration",
            ));
        }
        let object = *self
            .tree
            .init_declarators
            .get(declarators)
            .first()
            .ok_or(Error::Invariant("decomposition without object"))?;
        let binding = if object.initializer.is_some() {
            self.declare_init(specifiers, object, scope, location, &mut None)?;
            self.graph.declared_bindings[object.declarator.index()]
                .ok_or(Error::Invariant("decomposition object binding"))?
        } else {
            let name = self
                .simple_declarator_name(object.declarator)?
                .ok_or(Error::Invariant("range binding name"))?;
            let found = self
                .unqualified(scope, name, Filter::Ordinary)?
                .ok_or(Error::at(
                    location,
                    "a range-based for's structured binding without its object",
                ))?;
            super::record(
                &mut self.graph.declared_bindings,
                object.declarator.index(),
                found,
            );
            found
        };
        let entity = self.graph.binding(binding).entity;
        let ty = self.graph.lower_entity_type(entity)?;
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        if !reference
            && let Some(single) = object
                .initializer
                .and_then(|init| self.single_initializer(init))
            && self
                .graph
                .expression_facts
                .known(single)
                .is_some_and(|facts| {
                    matches!(self.graph.types.get(facts.ty).kind, TypeKind::Array { .. })
                })
        {
            return Err(Error::at(
                location,
                "a structured binding copy of an array is not supported (bind by reference)",
            ));
        }
        // The frozen declaration owner numbers static objects before their
        // synthetic names are cleared. Even an object never emitted counts.
        if (specifiers.is_static
            || specifiers.thread_storage != super::specifiers::ThreadStorage::Ordinary)
            && self
                .graph
                .entity(entity)
                .home
                .is_some_and(|home| self.graph.scope(home).kind == super::graph::ScopeKind::Block)
            && let Some(function) = self.body_context.as_ref().and_then(|body| body.function)
            && let Some(name) = self.graph.entity(entity).name
        {
            let count = self
                .graph
                .decompositions
                .local_counts
                .entry((function, name))
                .or_default();
            let earlier = u16::try_from(*count).unwrap_or(u16::MAX);
            *count = count
                .checked_add(1)
                .ok_or(Error::Limit("local binding object count"))?;
            self.graph
                .decompositions
                .local_discriminators
                .insert(entity, earlier);
        }
        self.graph.entity_mut(entity).name = None;
        self.graph.decompositions.objects.insert(entity, Vec::new());
        let count = self.tree.init_declarators.get(declarators).len() - 1;
        if self.template_depth != 0 && self.graph.dependent_type(ty) {
            for index in 0..count {
                self.declare_binding(
                    declarators,
                    index,
                    ty,
                    BindingAlias {
                        object: entity,
                        object_type: ty,
                        target: BindingTarget::Dependent,
                    },
                    scope,
                )?;
            }
            return Ok(());
        }
        if let TypeKind::Array { element, bound, .. } = self.graph.types.get(ty).kind {
            if u64::try_from(count).ok() != Some(bound) {
                return Err(Error::at(
                    location,
                    "a structured binding of an array with as many names as elements (11.5p2)",
                ));
            }
            for index in 0..count {
                let ordinal =
                    u64::try_from(index).map_err(|_| Error::Limit("structured binding index"))?;
                self.declare_binding(
                    declarators,
                    index,
                    element,
                    BindingAlias {
                        object: entity,
                        object_type: ty,
                        target: BindingTarget::Element(ordinal),
                    },
                    scope,
                )?;
            }
            return Ok(());
        }
        self.bind_members(declarators, entity, ty, scope, location)
    }
    fn declare_binding(
        &mut self,
        declarators: List<ast::InitDeclarator>,
        index: usize,
        ty: TypeId,
        alias: BindingAlias,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let item = self.tree.init_declarators.get(declarators)[index + 1];
        let location = self.tree.declarators.get(item.declarator).location;
        let name = self
            .simple_declarator_name(item.declarator)?
            .ok_or(Error::Invariant("structured binding without name"))?;
        let home = self.declaration_scope(scope);
        self.check_template_parameter_name(scope, name, location)?;
        self.check_construct_redeclaration(home, name, location)?;
        if self
            .graph
            .scope(home)
            .index
            .get(&name)
            .and_then(|entry| entry.value)
            .is_some_and(|binding| !self.graph.friendships.hidden(binding))
        {
            return Err(Error::at(
                location,
                "a structured binding redeclares a name of its scope",
            ));
        }
        let entity = self.graph.add_entity(Some(ty), Some(name), Keyword::Auto)?;
        let properties = self.graph.entity(alias.object).variable;
        let variable = self.graph.entity_mut(entity);
        variable.defined = true;
        variable.home = Some(home);
        variable.variable = properties;
        let binding = self.graph.add_binding(
            scope,
            Some(home),
            Binding {
                kind: BindingKind::Variable,
                name: Some(name),
                entity,
                ty: Some(ty),
                key: Keyword::Auto,
            },
        )?;
        self.graph
            .set_binding_dependence(binding, self.graph.dependent_type(ty));
        super::record(
            &mut self.graph.declared_bindings,
            item.declarator.index(),
            binding,
        );
        self.graph.decompositions.aliases.insert(entity, alias);
        self.graph
            .decompositions
            .objects
            .get_mut(&alias.object)
            .ok_or(Error::Invariant("binding object not registered"))?
            .push(entity);
        Ok(())
    }
    fn bind_members(
        &mut self,
        declarators: List<ast::InitDeclarator>,
        object: EntityId,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Err(Error::at(
                location,
                "a structured binding of a type that is no array, tuple-like type or complete class (11.5)",
            ));
        };
        if !self.complete_class(class)? {
            return Err(Error::at(
                location,
                "a structured binding of a type that is no array, tuple-like type or complete class (11.5)",
            ));
        }
        if self.graph.entity(class).key == Keyword::Union {
            return Err(Error::at(
                location,
                "a structured binding of a union (11.5p4)",
            ));
        }
        let mut holder = None;
        let mut pending = vec![class];
        let mut seen = HashSet::new();
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            if self.graph.entity(current).members.iter().any(|&member| {
                let member = self.graph.entity(member);
                member.name.is_some() || member.bit_field.is_none()
            }) {
                if holder.is_some_and(|holder| holder != current) {
                    return Err(Error::at(
                        location,
                        "a structured binding of a class whose non-static data members are of two classes (11.5p4)",
                    ));
                }
                holder = Some(current);
            }
            pending.extend(
                self.graph
                    .entity(current)
                    .bases
                    .iter()
                    .map(|base| base.class),
            );
        }
        let holder = holder.unwrap_or(class);
        let path = if holder == class {
            None
        } else {
            if self.graph.base_count(class, holder) > 1 {
                return Err(Error::at(
                    location,
                    "a structured binding of the members of an ambiguous base class (11.5p4)",
                ));
            }
            Some(self.checked_base_path(class, holder, scope, location, true, false)?)
        };
        let mut count = 0;
        for &member in &self.graph.entity(holder).members {
            let member = self.graph.entity(member);
            if member.name.is_some() {
                count += 1;
            } else if member.bit_field.is_none() {
                return Err(Error::at(
                    location,
                    "a structured binding of a class with an anonymous union member (11.5p4)",
                ));
            }
        }
        if count + 1 != self.tree.init_declarators.get(declarators).len() {
            return Err(Error::at(
                location,
                "a structured binding of a class with as many names as non-static data members (11.5p4)",
            ));
        }
        let mut index = 0;
        for ordinal in 0..self.graph.entity(holder).members.len() {
            let member = self.graph.entity(holder).members[ordinal];
            let variable = self.graph.entity(member);
            let Some(name) = variable.name else {
                continue;
            };
            let home = self
                .graph
                .entity(holder)
                .scope
                .ok_or(Error::Invariant("member holder without scope"))?;
            if let Some(binding) = self.graph.own(home, name, Filter::Ordinary) {
                self.validate_member_access(binding, class, Some(class), scope, location)?;
            }
            let declared = self.graph.lower_entity_type(member)?;
            let cv = self.graph.types.get(ty).cv.0
                & if self.graph.entity(member).mutability == Mutability::Mutable {
                    !1
                } else {
                    u8::MAX
                };
            let referenced = if matches!(
                self.graph.types.get(declared).kind,
                TypeKind::Reference { .. }
            ) {
                declared
            } else {
                self.graph.types.qualify(declared, Cv(cv))?
            };
            self.declare_binding(
                declarators,
                index,
                referenced,
                BindingAlias {
                    object,
                    object_type: ty,
                    target: BindingTarget::Member { member, path },
                },
                scope,
            )?;
            index += 1;
        }
        self.class_layout(class, location)?;
        Ok(())
    }
}

impl Analyzer<'_> {
    pub(super) fn constant_binding_place(
        &mut self,
        alias: BindingAlias,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<super::constant_addresses::Address, Error> {
        use super::constant_addresses::{Address, Root};
        let mut object = if let Some(place) = self.constexpr_frames.local(alias.object) {
            place
        } else if matches!(
            self.graph
                .types
                .get(self.graph.lower_entity_type(alias.object)?)
                .kind,
            TypeKind::Reference { .. }
        ) {
            self.instantiate_variable_value(alias.object)?;
            self.graph
                .constant_addresses
                .value(alias.object)
                .ok_or_else(|| {
                    super::constant_addresses::nonconstant(
                        location,
                        "a reference is not usable in a constant expression",
                    )
                })?
        } else {
            Address::new(Root::Variable(alias.object), alias.object_type)
        };
        object.designated = alias.object_type;
        match alias.target {
            BindingTarget::Member { member, path } => {
                object = self.constant_project_path(object, path, location)?;
                self.constant_member_designation(object, member, ty, location)
            }
            BindingTarget::Element(index) => {
                let (size, _) = self.type_size(ty, location)?;
                let offset = index.wrapping_mul(size);
                object.offset = object
                    .offset
                    .wrapping_add(i64::from_le_bytes(offset.to_le_bytes()));
                object.designated = ty;
                Ok(object)
            }
            BindingTarget::Dependent => Err(Error::Invariant("constant dependent binding")),
        }
    }
}
