// Translated from sema/analyzer_access.cpp's accessible_declarations and
// accessible/access_walk/accessible_below. See NOTICE.
//! Access to one canonical member through declarations of different access.
use super::{
    analyzer::Analyzer,
    graph::{Access, BindingId, DataMemberKind, EntityId, Graph, ScopeId, ScopeKind},
    member_expressions::AccessFailure,
};
use std::collections::{HashMap, HashSet};

enum GrantedAccess {
    Unrestricted,
    Protected(EntityId),
}

fn stricter(first: Access, second: Access) -> Access {
    match (first, second) {
        (Access::Private, _) | (_, Access::Private) => Access::Private,
        (Access::Protected, _) | (_, Access::Protected) => Access::Protected,
        _ => Access::Public,
    }
}

// A complete target's effective member access, shared by all paths to a
// class in this query. Absence means that no member reaches that class.
struct MemberPaths<'a> {
    declaring: EntityId,
    blocked: &'a HashSet<EntityId>,
    memo: HashMap<EntityId, Option<Access>>,
    work: Vec<(EntityId, usize, Option<Access>)>,
}
impl<'a> MemberPaths<'a> {
    fn new(declaring: EntityId, declared: Access, blocked: &'a HashSet<EntityId>) -> Self {
        Self {
            declaring,
            blocked,
            memo: HashMap::from([(declaring, Some(declared))]),
            work: Vec::new(),
        }
    }

    fn access(&mut self, graph: &Graph, naming: EntityId) -> Option<Access> {
        if let Some(&access) = self.memo.get(&naming) {
            return access;
        }
        self.work.push((naming, 0, None));
        while let Some((class, next, best)) = self.work.last_mut() {
            if self.blocked.contains(class) {
                self.memo.insert(*class, None);
                self.work.pop();
                continue;
            }
            if let Some(edge) = graph.entity(*class).bases.get(*next) {
                let Some(&access) = self.memo.get(&edge.class) else {
                    self.work.push((edge.class, 0, None));
                    continue;
                };
                if let Some(access) = access.filter(|&access| access != Access::Private) {
                    let access = stricter(access, edge.access);
                    *best = Some(best.map_or(access, |best| match (best, access) {
                        (Access::Public, _) | (_, Access::Public) => Access::Public,
                        (Access::Protected, _) | (_, Access::Protected) => Access::Protected,
                        _ => Access::Private,
                    }));
                }
                *next += 1;
            } else {
                self.memo.insert(*class, *best);
                self.work.pop();
            }
        }
        self.memo.get(&naming).copied().flatten()
    }
}

impl Analyzer<'_> {
    // Every branch stops at the first declaration of the requested entity.
    // These are lookup subobjects' declarations, not every using of the name
    // anywhere in the unit. Bases are pushed in source order as in C++.
    fn member_declaration_frontier(
        &self,
        naming: EntityId,
        binding: BindingId,
    ) -> Vec<(EntityId, BindingId)> {
        let mut declarations = Vec::new();
        let mut seen = HashSet::new();
        let mut work = vec![naming];
        while let Some(class) = work.pop() {
            if !seen.insert(class) {
                continue;
            }
            if let Some(declaration) = self
                .graph
                .entity(class)
                .scope
                .and_then(|scope| self.graph.own_member_declaration(scope, binding))
            {
                declarations.push((class, declaration));
            } else {
                work.extend(self.graph.entity(class).bases.iter().map(|edge| edge.class));
            }
        }
        declarations
    }

    fn imported_base_accessible(
        &self,
        class: EntityId,
        access: Access,
        scope: ScopeId,
        cache: &mut HashMap<EntityId, bool>,
    ) -> bool {
        if access == Access::Public || self.member_or_friend(scope, class) {
            return true;
        }
        if access == Access::Private {
            return false;
        }
        if let Some(&found) = cache.get(&class) {
            return found;
        }
        // Base accessibility is the invented protected member query (11.2p4).
        // Other declarations block only a query for the selected member's
        // declaring class; they cannot block this different target.
        let empty = HashSet::new();
        let mut paths = MemberPaths::new(class, Access::Protected, &empty);
        let found = self
            .access_contexts(scope)
            .any(|context| paths.access(&self.graph, context).is_some());
        cache.insert(class, found);
        found
    }

    // An iterative form of accessible_below: evaluate bullets 1-3 at each
    // class before bullet 4 visits its accessible bases left to right. The
    // effective-member memo and failure set depend on one complete target.
    fn imported_declaration_access(
        &self,
        naming: EntityId,
        scope: ScopeId,
        paths: &mut MemberPaths<'_>,
    ) -> Option<GrantedAccess> {
        let mut work = vec![naming];
        let mut seen = HashSet::new();
        let mut protected_bases = HashMap::new();
        while let Some(class) = work.pop() {
            if !seen.insert(class) || (class != paths.declaring && paths.blocked.contains(&class)) {
                continue;
            }
            let access = paths.access(&self.graph, class);
            if access == Some(Access::Public)
                || (access.is_some() && self.member_or_friend(scope, class))
            {
                return Some(GrantedAccess::Unrestricted);
            }
            if access == Some(Access::Protected)
                && let Some(context) = self.access_contexts(scope).find(|&context| {
                    self.graph.is_base_of(class, context)
                        && paths.access(&self.graph, context).is_some()
                })
            {
                return Some(GrantedAccess::Protected(context));
            }
            if class == paths.declaring {
                continue;
            }
            for edge in self.graph.entity(class).bases.iter().rev() {
                if self.imported_base_accessible(class, edge.access, scope, &mut protected_bases) {
                    work.push(edge.class);
                }
            }
        }
        None
    }

    pub(super) fn imported_member_access_failure(
        &self,
        binding: BindingId,
        naming: EntityId,
        object: Option<EntityId>,
        scope: ScopeId,
    ) -> Option<AccessFailure> {
        let home = self.graph.binding_scope(binding);
        if self.graph.scope(home).kind != ScopeKind::Class {
            return None;
        }
        let owner = self.graph.scope(home).owner?;
        if naming == owner && self.graph.binding_access(binding) == Access::Public {
            return None;
        }
        let declarations = self.member_declaration_frontier(naming, binding);
        let mut blocked = HashSet::new();
        if declarations.len() > 1 {
            blocked.extend(declarations.iter().map(|&(class, _)| class));
        }
        let mut reached = None;
        if declarations.is_empty() {
            let mut paths = MemberPaths::new(owner, self.graph.binding_access(binding), &blocked);
            reached = self.imported_declaration_access(naming, scope, &mut paths);
        } else {
            for (class, declaration) in declarations {
                let mut paths =
                    MemberPaths::new(class, self.graph.binding_access(declaration), &blocked);
                reached = self.imported_declaration_access(naming, scope, &mut paths);
                if reached.is_some() {
                    break;
                }
            }
        }
        let Some(via) = reached else {
            return Some(AccessFailure::Inaccessible);
        };
        let member = self.graph.entity(self.graph.binding(binding).entity);
        let nonstatic = member.data_member == Some(DataMemberKind::Instance)
            || self.member_function_owner(binding).is_some();
        if nonstatic
            && let GrantedAccess::Protected(via) = via
            && object.is_some_and(|object| object != via && !self.graph.is_base_of(via, object))
        {
            return Some(AccessFailure::ProtectedObject);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{layout::LayoutState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn shared_declarations_keep_access_frontiers_and_protected_object_context() {
        let source = SourceBuffer::new(
            br"
struct V { protected: typedef char m; };
struct B : virtual V { public: using V::m; };
struct C : virtual V {};
struct Left : C,B {}; struct Right : B,C {};
struct Blocked : private B,C {};
struct F { protected: int f(); };
struct U : F { protected: using F::f; }; struct D : U {};
struct P : U { public: using U::f; };
struct S { protected: static int f(); };
struct SU : S { protected: using S::f; }; struct SD : SU {};
"
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"shared-access").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            "V", "B", "Left", "Right", "Blocked", "F", "U", "D", "P", "SU", "SD",
        ]
        .map(|name| cursor.intern_name(name.as_bytes()).unwrap());
        let type_key = cursor.intern_name(b"m").unwrap();
        let function_key = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer.set_lowering();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let expressions = analyzer.tree.expressions.mark();
        let classes = names.map(|name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity
        });
        let [
            virtual_class,
            importer,
            left,
            right,
            blocked,
            object,
            using_function,
            derived,
            public,
            static_using,
            static_derived,
        ] = classes;
        let scope = |class| analyzer.graph.entity(class).scope.unwrap();
        for class in [left, right] {
            let binding = analyzer
                .class_lookup(scope(class), type_key.into(), Filter::Prefix)
                .unwrap()
                .binding
                .unwrap();
            assert!(
                analyzer
                    .member_access_failure(binding, class, None, global)
                    .is_none()
            );
        }
        let binding = analyzer
            .class_lookup(scope(left), type_key.into(), Filter::Prefix)
            .unwrap()
            .binding
            .unwrap();
        assert_eq!(analyzer.graph.binding_scope(binding), scope(virtual_class));
        let frontier = analyzer.member_declaration_frontier(left, binding);
        assert_eq!(
            frontier.iter().map(|&(class, _)| class).collect::<Vec<_>>(),
            [importer, virtual_class]
        );
        let blocked_binding = analyzer
            .class_lookup(scope(blocked), type_key.into(), Filter::Prefix)
            .unwrap()
            .binding
            .unwrap();
        assert!(matches!(
            analyzer.member_access_failure(blocked_binding, blocked, None, global),
            Some(AccessFailure::Inaccessible)
        ));
        let binding = analyzer
            .class_lookup(scope(using_function), function_key.into(), Filter::Ordinary)
            .unwrap()
            .binding
            .unwrap();
        assert!(
            analyzer
                .member_access_failure(binding, using_function, Some(derived), scope(derived))
                .is_none()
        );
        assert!(matches!(
            analyzer.member_access_failure(binding, using_function, Some(object), scope(derived)),
            Some(AccessFailure::ProtectedObject)
        ));
        let binding = analyzer
            .class_lookup(scope(public), function_key.into(), Filter::Ordinary)
            .unwrap()
            .binding
            .unwrap();
        assert!(
            analyzer
                .member_access_failure(binding, public, Some(public), global)
                .is_none()
        );
        let binding = analyzer
            .class_lookup(scope(static_using), function_key.into(), Filter::Ordinary)
            .unwrap()
            .binding
            .unwrap();
        assert!(
            analyzer
                .member_access_failure(binding, static_using, Some(object), scope(static_derived))
                .is_none()
        );
        assert_eq!(expressions, analyzer.tree.expressions.mark());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(
            classes.into_iter().all(|class| matches!(
                analyzer.graph.entity(class).layout,
                LayoutState::NotStarted
            ))
        );
    }
}
