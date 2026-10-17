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
    pub dependency: ast::BaseDependency,
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
    pub(super) fn derived_class_conversion(
        &mut self,
        base: TypeId,
        derived: TypeId,
    ) -> Result<bool, Error> {
        let Some((base, derived)) = self.graph.class_of(base).zip(self.graph.class_of(derived))
        else {
            return Ok(false);
        };
        if base == derived {
            return Ok(false);
        }
        // ConversionRules::is_base_of completes only the queried derived
        // class. Its bases are completed by class_bases; member bodies wait.
        self.complete_class(derived)?;
        Ok(self.graph.is_base_of(base, derived))
    }

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
        let mark = self.begin_base_access(scope);
        let result = self.resolve_class_bases(node, class, scope).and_then(|()| {
            let home = self
                .graph
                .entity(class)
                .scope
                .ok_or(Error::Invariant("base owner without scope"))?;
            self.settle_base_access(home)
        });
        self.finish_declaration_access(mark);
        result
    }

    fn resolve_class_bases(
        &mut self,
        node: ast::ClassSyntax,
        class: EntityId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let mut seen = HashSet::new();
        for syntax_index in 0..self.tree.bases.get(node.bases).len() {
            let syntax_item = self.tree.bases.get(node.bases)[syntax_index];
            let base = &syntax_item;
            if base.pack && self.template_depth == 0 {
                return Err(Error::at(
                    base.location,
                    "a pack expansion outside a template",
                ));
            }
            let name = *self.tree.names.get(base.name);
            let components = self.tree.name_lists.get(name.components);
            let mut access_binding = None;
            let decltype_base = !name.global
                && components.len() == 1
                && matches!(components[0].kind, ast::NameKind::Decltype(_));
            let ty =
                if decltype_base {
                    if base.pack {
                        None
                    } else if let ast::NameKind::Decltype(operand) = components[0].kind {
                        Some(self.decltype_type(operand, scope)?)
                    } else {
                        return Err(Error::Invariant("decltype base without operand"));
                    }
                } else {
                    let found = self.lookup_result(base.name, scope, Filter::Elaborated)?;
                    let binding = found
                        .binding
                        .ok_or_else(|| Error::at(name.location, "undeclared name"))?;
                    access_binding = found.qualifier.map(|qualifier| (binding, qualifier));
                    if base.pack {
                        None
                    } else {
                        Some(self.graph.binding(binding).ty.ok_or_else(|| {
                            Error::at(base.location, "name does not denote a type")
                        })?)
                    }
                };
            if base.pack
                || ty.is_some_and(|ty| self.template_depth != 0 && self.graph.dependent_type(ty))
            {
                self.graph.entity_mut(class).base_dependency = ast::BaseDependency::Dependent;
                self.tree.bases.replace(
                    node.bases,
                    syntax_index,
                    ast::Base {
                        dependency: ast::BaseDependency::Dependent,
                        ..syntax_item
                    },
                );
                continue;
            }
            let ty = ty.ok_or(Error::Invariant("base without type"))?;
            let Some(parent) = self.graph.class_of(ty) else {
                if matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::TemplateParameter { .. }
                ) {
                    return Err(Error::Missing("dependent base class formation"));
                }
                return Err(Error::at(base.location, "base is not a class"));
            };
            if !self.complete_class(parent)? {
                return Err(Error::at(base.location, "base class is incomplete"));
            }
            if self.graph.entity(parent).derivation == Derivation::Final {
                return Err(Error::at(
                    base.location,
                    "a final class used as a base class",
                ));
            }
            // Fixed template bases are checked where the pattern is defined.
            // The ordinary lookup suppresses template access checks; enqueue
            // this one explicitly so the complete base list supplies context.
            if self.semantic()
                && self.template_depth != 0
                && let Some((binding, qualifier)) = access_binding
                && self.graph.scope(qualifier).kind == super::graph::ScopeKind::Class
                && let Some(naming) = self.graph.scope(qualifier).owner
            {
                self.check_declaration_member_access(binding, naming, None, scope, base.location)?;
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
            if self.graph.entity(parent).polymorphism.is_dynamic() {
                self.graph.entity_mut(class).polymorphism =
                    super::virtual_members::Polymorphism::Dynamic;
            }
            self.graph.entity_mut(class).bases.push(BaseClass {
                class: parent,
                access,
                offset: None,
                dependency: base.dependency,
            });
            seen.insert(parent);
        }
        if seen.len() != self.graph.entity(class).bases.len() {
            return Err(Error::at(
                node.location,
                "a class named twice as a direct base class",
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn bound_base_packs_copy_once_and_keep_pattern_and_body_demand_separate() {
        let source = SourceBuffer::new(
            b"struct Fixed{};struct A{};struct B{};\
              template<class... Ts>struct All:Fixed,Ts...{void idle(){}};\
              using X=All<A,B>;using Again=All<A,B>;using Empty=All<>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"base-packs").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"X".as_slice(), b"Again", b"Empty", b"idle"]
            .map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes = names[..3]
            .iter()
            .map(|&name| {
                let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
                analyzer
                    .graph
                    .class_of(analyzer.graph.binding(binding).ty.unwrap())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(classes[0], classes[1]);
        assert!(
            classes
                .iter()
                .all(|&class| analyzer.graph.entity(class).scope.is_none())
        );
        assert!(analyzer.complete_class(classes[0]).unwrap());
        let mark = analyzer.tree.classes.mark();
        assert!(analyzer.complete_class(classes[1]).unwrap());
        assert_eq!(analyzer.tree.classes.mark(), mark);
        let entity = analyzer.graph.entity(classes[0]);
        assert_eq!(entity.bases.len(), 3);
        assert_eq!(entity.bases[0].dependency, ast::BaseDependency::Fixed);
        assert!(
            entity.bases[1..]
                .iter()
                .all(|edge| edge.dependency == ast::BaseDependency::Dependent)
        );
        let home = entity.scope.unwrap();
        let idle = analyzer
            .graph
            .own(home, names[3], Filter::Ordinary)
            .unwrap();
        assert!(
            analyzer
                .graph
                .function_templates
                .bodies
                .body(analyzer.graph.binding(idle).entity)
                .is_none()
        );
        assert!(analyzer.graph.entity(classes[2]).scope.is_none());
        assert!(analyzer.complete_class(classes[2]).unwrap());
        assert_eq!(analyzer.graph.entity(classes[2]).bases.len(), 1);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        let ast::DeclKind::Template { declaration, .. } = analyzer
            .tree
            .declarations
            .get(analyzer.tree.decl_lists.get(declarations)[3])
            .kind
        else {
            panic!("template");
        };
        let ast::DeclKind::Class(specifier) = analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("class declaration");
        };
        let ast::SpecifierKind::Class(pattern) = specifier.kind else {
            panic!("class specifier");
        };
        let bases = analyzer
            .tree
            .bases
            .get(analyzer.tree.classes.get(pattern).bases);
        assert_eq!(bases.len(), 2);
        assert!(bases[1].pack);
        assert_eq!(bases[1].dependency, ast::BaseDependency::Dependent);
    }

    #[test]
    fn copied_bases_keep_provenance_and_imports_keep_canonical_entities() {
        let source = SourceBuffer::new(
            b"struct Fixed{typedef long fixed;};\
              template<class T>struct Base{typedef T type;int idle(){return T::bad;}};\
              template<class T>struct Derived:Base<T>,Fixed{\
                using typename Base<T>::type;type value;};Derived<int> d;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"base-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        let name = cursor.intern_name(b"Derived").unwrap();
        let primary = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        assert_eq!(
            graph.entity(primary).base_dependency,
            ast::BaseDependency::Dependent
        );
        assert_eq!(graph.entity(primary).bases.len(), 1);
        let object = cursor.intern_name(b"d").unwrap();
        let ty = graph
            .binding(graph.own(graph.global, object, Filter::Ordinary).unwrap())
            .ty
            .unwrap();
        let class = graph.class_of(ty).unwrap();
        let bases = &graph.entity(class).bases;
        assert_eq!(bases.len(), 2);
        assert_eq!(bases[0].dependency, ast::BaseDependency::Dependent);
        assert_eq!(bases[1].dependency, ast::BaseDependency::Fixed);
        assert_eq!(
            graph.entity(class).base_dependency,
            ast::BaseDependency::Fixed
        );
        let home = graph.entity(class).scope.unwrap();
        let base_home = graph.entity(bases[0].class).scope.unwrap();
        let name = cursor.intern_name(b"type").unwrap();
        let imported = graph.own(home, name, Filter::Prefix).unwrap();
        let original = graph.own(base_home, name, Filter::Prefix).unwrap();
        assert_eq!(
            graph.binding(imported).entity,
            graph.binding(original).entity
        );
        assert_eq!(graph.binding(imported).ty, graph.binding(original).ty);
        let idle = cursor.intern_name(b"idle").unwrap();
        let idle = graph.own(base_home, idle, Filter::Ordinary).unwrap();
        assert!(
            graph
                .function_templates
                .bodies
                .body(graph.binding(idle).entity)
                .is_none()
        );
        assert!(graph.root_demands.as_slice().is_empty());
        let ast::DeclKind::Template { declaration, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("template");
        };
        let ast::DeclKind::Class(specifier) = tree.declarations.get(declaration).kind else {
            panic!("class declaration");
        };
        let ast::SpecifierKind::Class(pattern) = specifier.kind else {
            panic!("class");
        };
        let syntax_bases = tree.bases.get(tree.classes.get(pattern).bases);
        assert_eq!(syntax_bases[0].dependency, ast::BaseDependency::Dependent);
        assert_eq!(syntax_bases[1].dependency, ast::BaseDependency::Fixed);
    }
}
