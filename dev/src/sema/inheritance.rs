// Translated from analyzer_classes.cpp::base_classes, lookup.cpp::class_members,
// conversions.cpp::is_base_of and analyzer_member_pointers.cpp. See NOTICE.
//! Nonvirtual base edges, query-local graph walks and retained conversion paths.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Access, EntityId, Graph, ScopeId},
    lookup::Filter,
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Derivation {
    #[default]
    Allowed,
    Final,
}

#[derive(Clone, Copy)]
pub(super) struct BaseClass {
    pub class: EntityId,
    pub access: Access,
    pub offset: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BasePathId(u32);

#[derive(Default)]
pub(super) struct BasePaths {
    records: Vec<Vec<(EntityId, usize)>>,
    index: HashMap<(EntityId, EntityId), BasePathId>,
}

impl BasePaths {
    pub(super) fn get(&self, id: BasePathId) -> &[(EntityId, usize)] {
        &self.records[id.0 as usize]
    }
}

impl Graph {
    pub(super) fn class_of(&self, ty: TypeId) -> Option<EntityId> {
        if let TypeKind::Class(class) = self.types.get(ty).kind {
            Some(class)
        } else {
            None
        }
    }

    pub(super) fn is_base_of(&self, base: EntityId, derived: EntityId) -> bool {
        if base == derived || self.entity(derived).bases.is_empty() {
            return false;
        }
        let mut seen = HashSet::new();
        let mut work = vec![derived];
        while let Some(class) = work.pop() {
            if !seen.insert(class) {
                continue;
            }
            for edge in &self.entity(class).bases {
                if edge.class == base {
                    return true;
                }
                work.push(edge.class);
            }
        }
        false
    }

    // Each graph edge is visited once, even for exponentially many subobjects.
    // Saturation at two preserves the only distinction needed by ambiguity checks.
    fn base_counts(&self, derived: EntityId, base: EntityId) -> HashMap<EntityId, u8> {
        let mut counts = HashMap::new();
        counts.insert(base, 1u8);
        let mut work = vec![(derived, 0usize)];
        while let Some((class, next)) = work.last_mut() {
            if counts.contains_key(class) {
                work.pop();
                continue;
            }
            if let Some(edge) = self.entity(*class).bases.get(*next) {
                if counts.contains_key(&edge.class) {
                    *next += 1;
                } else {
                    work.push((edge.class, 0));
                }
            } else {
                let mut count = 0u8;
                for edge in &self.entity(*class).bases {
                    count = count.saturating_add(counts[&edge.class]).min(2);
                }
                counts.insert(*class, count);
                work.pop();
            }
        }
        counts
    }

    pub(super) fn base_count(&self, derived: EntityId, base: EntityId) -> u8 {
        self.base_counts(derived, base)[&derived]
    }

    pub(super) fn retain_base_path(
        &mut self,
        derived: EntityId,
        base: EntityId,
    ) -> Result<BasePathId, Error> {
        if let Some(&id) = self.base_paths.index.get(&(derived, base)) {
            return Ok(id);
        }
        let counts = self.base_counts(derived, base);
        if counts[&derived] != 1 {
            return Err(Error::Invariant("retaining a nonunique base path"));
        }
        let mut class = derived;
        let mut path = Vec::new();
        while class != base {
            let (index, edge) = self
                .entity(class)
                .bases
                .iter()
                .enumerate()
                .find(|(_, edge)| counts.get(&edge.class).copied().unwrap_or(0) != 0)
                .ok_or(Error::Invariant("base path without an edge"))?;
            path.push((class, index));
            class = edge.class;
        }
        let id = BasePathId(
            u32::try_from(self.base_paths.records.len())
                .map_err(|_| Error::Limit("base path limit"))?,
        );
        self.base_paths.records.push(path);
        self.base_paths.index.insert((derived, base), id);
        Ok(id)
    }
}

impl Analyzer<'_> {
    pub(super) fn conversion_base_path(
        &mut self,
        facts: super::expression_facts::ExprFacts,
        conversion: super::expression_facts::Conversion,
        scope: ScopeId,
        location: SourceOffset,
        access: bool,
    ) -> Result<Option<BasePathId>, Error> {
        if conversion.second != super::expression_facts::ValueConversion::DerivedToBase {
            return Ok(None);
        }
        let from = match self.graph.types.get(facts.ty).kind {
            TypeKind::Pointer(element) | TypeKind::Array { element, .. } => element,
            _ => facts.ty,
        };
        let to = match self.graph.types.get(conversion.result).kind {
            TypeKind::Pointer(element) => element,
            _ => conversion.result,
        };
        let derived = self
            .graph
            .class_of(from)
            .ok_or(Error::Invariant("base conversion without source class"))?;
        let base = self
            .graph
            .class_of(to)
            .ok_or(Error::Invariant("base conversion without target class"))?;
        self.checked_base_path(derived, base, scope, location, access, false)
            .map(Some)
    }

    pub(super) fn checked_base_path(
        &mut self,
        derived: EntityId,
        base: EntityId,
        scope: ScopeId,
        location: SourceOffset,
        access: bool,
        downcast: bool,
    ) -> Result<BasePathId, Error> {
        if self.graph.base_count(derived, base) > 1 {
            return Err(Error::at(
                location,
                if downcast {
                    "a cast from an ambiguous base class to its derived class"
                } else {
                    "conversion to an ambiguous base class"
                },
            ));
        }
        let path = self.graph.retain_base_path(derived, base)?;
        if access {
            self.check_base_path_access(path, scope, location)?;
        }
        if self.lowering() && self.unevaluated == 0 {
            self.request_storage_layout(derived, location)?;
        }
        Ok(path)
    }

    pub(super) fn class_bases(
        &mut self,
        node: ast::ClassSyntax,
        class: EntityId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let mut seen = HashSet::new();
        let mut bindings = Vec::new();
        for base in self.tree.bases.get(node.bases) {
            if base.pack {
                return Err(if self.template_depth == 0 {
                    Error::at(base.location, "a pack expansion outside a template")
                } else {
                    Error::Missing("dependent base pack expansion")
                });
            }
            let name = self.tree.names.get(base.name);
            let components = self.tree.name_lists.get(name.components);
            let (ty, binding) = if !name.global
                && components.len() == 1
                && let ast::NameKind::Decltype(operand) = components[0].kind
            {
                (self.decltype_type(operand, scope)?, None)
            } else {
                let binding = self.lookup(base.name, scope, Filter::Elaborated)?;
                (
                    self.graph
                        .binding(binding)
                        .ty
                        .ok_or_else(|| Error::at(base.location, "name does not denote a type"))?,
                    Some(binding),
                )
            };
            let Some(parent) = self.graph.class_of(ty) else {
                if matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::TemplateParameter { .. }
                ) {
                    return Err(Error::Missing("dependent base class formation"));
                }
                return Err(Error::at(base.location, "base is not a class"));
            };
            if !self.graph.entity(parent).complete {
                return Err(Error::at(base.location, "base class is incomplete"));
            }
            if self.graph.entity(parent).derivation == Derivation::Final {
                return Err(Error::at(
                    base.location,
                    "a final class used as a base class",
                ));
            }
            if self.graph.entity(parent).key == Keyword::Union || node.key == Keyword::Union {
                return Err(Error::at(base.location, "unions have no base classes"));
            }
            if base.is_virtual {
                return Err(Error::Missing("virtual base class formation"));
            }
            let access = match base.access {
                Some(Keyword::Private) => Access::Private,
                Some(Keyword::Protected) => Access::Protected,
                None if node.key == Keyword::Class => Access::Private,
                Some(_) | None => Access::Public,
            };
            self.graph.entity_mut(class).bases.push(BaseClass {
                class: parent,
                access,
                offset: None,
            });
            seen.insert(parent);
            if let Some(binding) = binding {
                bindings.push((binding, base.location));
            }
        }
        if seen.len() != self.graph.entity(class).bases.len() {
            return Err(Error::at(
                node.location,
                "a class named twice as a direct base class",
            ));
        }
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("base owner without scope"))?;
        for (binding, location) in bindings {
            self.check_direct_member_access(binding, home, location)?;
        }
        Ok(())
    }

    pub(super) fn class_context(&self, scope: ScopeId, class: EntityId) -> bool {
        self.access_contexts(scope)
            .any(|owner| owner == class || self.graph.is_base_of(class, owner))
    }

    pub(super) fn protected_object_accessible(
        &self,
        scope: ScopeId,
        declaring: EntityId,
        object: EntityId,
    ) -> bool {
        self.member_or_friend(scope, declaring)
            || self.access_contexts(scope).any(|class| {
                (class == declaring || self.graph.is_base_of(declaring, class))
                    && (class == object || self.graph.is_base_of(class, object))
            })
    }

    pub(super) fn accessible_base_name(
        &self,
        derived: EntityId,
        base: EntityId,
        scope: ScopeId,
    ) -> bool {
        // Access is a query over edges, independent of subobject multiplicity.
        // Repeated static members, enumerators and type names may be accessible.
        let mut seen = HashSet::new();
        let mut work = vec![derived];
        while let Some(class) = work.pop() {
            if class == base {
                return true;
            }
            if !seen.insert(class) {
                continue;
            }
            for edge in &self.graph.entity(class).bases {
                let own = self.member_or_friend(scope, class);
                if edge.access == Access::Public
                    || own
                    || edge.access == Access::Protected && self.class_context(scope, class)
                {
                    work.push(edge.class);
                }
            }
        }
        false
    }

    pub(super) fn check_base_path_access(
        &self,
        path: BasePathId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        for &(class, index) in self.graph.base_paths.get(path) {
            let edge = self.graph.entity(class).bases[index];
            if edge.access != Access::Public
                && !self.member_or_friend(scope, class)
                && !(edge.access == Access::Protected && self.class_context(scope, class))
            {
                return Err(Error::at(
                    location,
                    "conversion to an inaccessible base class",
                ));
            }
        }
        Ok(())
    }
}
