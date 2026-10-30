// Translated from analyzer_declarations.cpp's ABI tag owners. See NOTICE.
//! Explicit tags belong to canonical entities or inline namespace scopes.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, BindingKind, EntityId, Graph, ScopeId},
};
use crate::support::hash::HashMap;
use crate::{
    lex::identifiers::NameId,
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Owner {
    Entity(EntityId),
    Namespace(ScopeId),
}

#[derive(Default)]
pub(super) struct AbiTags {
    values: HashMap<Owner, Box<[NameId]>>,
}

impl Analyzer<'_> {
    fn collect_abi_tags(
        &mut self,
        list: List<ast::Attribute>,
        on_class: bool,
        tags: &mut Vec<NameId>,
        implied: Option<NameId>,
    ) -> Result<(), Error> {
        let mut skipped = false;
        for attribute in self.tree.attributes.get(list) {
            if attribute.kind != ast::AttributeKind::AbiTag
                || (attribute.site == ast::AttributeSite::Class) != on_class
            {
                continue;
            }
            if !attribute.continued {
                skipped = !tags.is_empty();
            }
            if skipped {
                continue;
            }
            let Some(expression) = attribute.expression else {
                let name = implied.ok_or_else(|| {
                    Error::at(
                        attribute.location,
                        "an ABI tag attribute without arguments outside an inline namespace",
                    )
                })?;
                tags.push(name);
                continue;
            };
            let ast::ExprKind::Literal(literal) = self.tree.expressions.get(expression).kind else {
                return Err(Error::Invariant("ABI tag without literal"));
            };
            let tag = tag_name(self.literal_bytes.get(literal.spelling(self.literal_bytes)))
                .ok_or_else(|| Error::at(attribute.location, "an ABI tag that is no identifier"))?;
            tags.push(self.identifiers.intern(&tag).map_err(Error::Limit)?);
        }
        Ok(())
    }

    fn declare_abi_tags(
        &mut self,
        owner: Owner,
        mut tags: Vec<NameId>,
        redeclaration: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        tags.sort_unstable_by(|a, b| self.identifiers.bytes(*a).cmp(self.identifiers.bytes(*b)));
        tags.dedup();
        let known = self.graph.abi_tags.values.get(&owner);
        if redeclaration {
            for tag in &tags {
                if known.is_none_or(|known| {
                    known
                        .binary_search_by(|known| {
                            self.identifiers
                                .bytes(*known)
                                .cmp(self.identifiers.bytes(*tag))
                        })
                        .is_err()
                }) {
                    return Err(Error::at(location, "a redeclaration adding an ABI tag"));
                }
            }
            return Ok(());
        }
        if let Some(known) = known {
            tags.extend_from_slice(known);
            tags.sort_unstable_by(|a, b| {
                self.identifiers.bytes(*a).cmp(self.identifiers.bytes(*b))
            });
            tags.dedup();
        }
        self.graph
            .abi_tags
            .values
            .insert(owner, tags.into_boxed_slice());
        Ok(())
    }

    pub(super) fn class_abi_tags(
        &mut self,
        entity: EntityId,
        list: List<ast::Attribute>,
        on_class: bool,
        redeclaration: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let mut tags = Vec::new();
        self.collect_abi_tags(list, on_class, &mut tags, None)?;
        let defined = self.graph.entity(entity).enumerated().enumeration.is_none()
            && self.graph.entity(entity).context().scope.is_some();
        if !(tags.is_empty() || on_class && defined) {
            self.declare_abi_tags(Owner::Entity(entity), tags, redeclaration, location)?;
        }
        Ok(())
    }

    pub(super) fn namespace_abi_tags(
        &mut self,
        scope: ScopeId,
        list: List<ast::Attribute>,
        name: Option<NameId>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.scope(scope).is_inline {
            let mut tags = Vec::new();
            self.collect_abi_tags(list, false, &mut tags, name)?;
            if !tags.is_empty() {
                self.declare_abi_tags(Owner::Namespace(scope), tags, false, location)?;
            }
        }
        Ok(())
    }

    pub(super) fn declarator_abi_tags(
        &mut self,
        binding: BindingId,
        declarator: ast::DeclaratorId,
        leading: List<ast::Attribute>,
    ) -> Result<(), Error> {
        let redeclaration = self.graph.binding_redeclared(binding);
        let binding = self.graph.binding(binding);
        let (entity, kind) = (binding.entity, binding.kind);
        let mut tags = Vec::new();
        self.collect_abi_tags(leading, false, &mut tags, None)?;
        let mut part = Some(declarator);
        while let Some(id) = part {
            let node = *self.tree.declarators.get(id);
            self.collect_abi_tags(node.attributes, false, &mut tags, None)?;
            part = node.nested;
        }
        if tags.is_empty() {
            return Ok(());
        }
        let location = self.tree.declarators.get(declarator).location;
        if !matches!(kind, BindingKind::Function | BindingKind::Variable)
            || self.graph.entity(entity).object().data_member
                == Some(super::graph::DataMemberKind::Instance)
        {
            return Err(Error::at(
                location,
                "an ABI tag on a declaration of no function, variable or class",
            ));
        }
        self.declare_abi_tags(Owner::Entity(entity), tags, redeclaration, location)
    }
}

fn tag_name(spelling: &[u8]) -> Option<Vec<u8>> {
    let mut inside = false;
    let mut name = Vec::new();
    let mut index = 0;
    while let Some(&byte) = spelling.get(index) {
        if byte == b'"' {
            inside = !inside;
        } else if inside
            && (byte.is_ascii_alphabetic()
                || byte == b'_'
                || byte.is_ascii_digit() && !name.is_empty())
        {
            name.push(byte);
        } else if !inside && spelling.get(index..index + 3) == Some(b"u8\"") {
            index += 1;
        } else if inside || !matches!(byte, b' ' | b'\t' | b'\n') {
            return None;
        }
        index += 1;
    }
    (!name.is_empty()).then_some(name)
}

impl Graph {
    #[must_use]
    pub fn lower_class_abi_tag_owner(&self, entity: EntityId) -> EntityId {
        self.class_templates
            .instance(entity)
            .map(|instance| instance.primary)
            .or_else(|| self.class_templates.definitions.source(entity))
            .unwrap_or(entity)
    }
    #[inline]
    #[must_use]
    pub fn lower_function_abi_tag_owner(&self, entity: EntityId) -> EntityId {
        self.function_templates
            .primary(entity)
            .map_or(entity, |binding| self.binding(binding).entity)
    }
    #[inline]
    #[must_use]
    pub fn lower_variable_abi_tag_owner(&self, entity: EntityId) -> EntityId {
        self.variable_instances
            .templates
            .primary(entity)
            .unwrap_or(entity)
    }
    #[inline]
    #[must_use]
    pub fn lower_has_abi_tags(&self) -> bool {
        !self.abi_tags.values.is_empty()
    }
    #[inline]
    pub fn lower_abi_tags(&self, entity: EntityId) -> &[NameId] {
        self.abi_tags
            .values
            .get(&Owner::Entity(entity))
            .map_or(&[], AsRef::as_ref)
    }
    #[inline]
    pub fn lower_namespace_abi_tags(&self, scope: ScopeId) -> &[NameId] {
        self.abi_tags
            .values
            .get(&Owner::Namespace(scope))
            .map_or(&[], AsRef::as_ref)
    }
}
