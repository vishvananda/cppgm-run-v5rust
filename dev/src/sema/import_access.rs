// Translated from sema/analyzer_access.cpp's accessible_declarations and
// accessible/access_walk/accessible_below. See NOTICE.
//! Access to one canonical member through declarations of different access.
use super::{
    analyzer::Analyzer,
    friendships::AccessContext,
    graph::{Access, BindingId, DataMemberKind, EntityId, Graph, ScopeId, ScopeKind},
    member_expressions::AccessFailure,
};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;

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
    declared: Access,
    blocked: &'a HashSet<EntityId>,
    memo: HashMap<EntityId, Option<Access>>,
    work: Vec<AccessFrame>,
}
struct AccessFrame {
    top: EntityId,
    bottom: EntityId,
    next: usize,
    path: Access,
    best: Option<Access>,
}
enum Chain {
    Known(Option<Access>),
    Pending,
}
fn chain_member(top: EntityId, bottom: EntityId, path: Access, access: Access) -> Option<Access> {
    (top == bottom || access != Access::Private).then(|| stricter(path, access))
}
impl<'a> MemberPaths<'a> {
    fn new(declaring: EntityId, declared: Access, blocked: &'a HashSet<EntityId>) -> Self {
        Self {
            declaring,
            declared,
            blocked,
            memo: HashMap::default(),
            work: Vec::new(),
        }
    }
    fn chain(&mut self, graph: &Graph, top: EntityId) -> Chain {
        let mut class = top;
        let mut path = Access::Public;
        while class != self.declaring {
            graph.work.base_steps(1);
            let bases = &graph.entity(class).bases;
            let Some(base) = bases.first() else {
                return Chain::Known(None);
            };
            if bases.len() > 1 {
                let direct = bases.iter().find(|base| base.class == self.declaring);
                if let Some(direct) = direct.filter(|direct| {
                    direct.access == Access::Public
                        || !direct.is_virtual && graph.simple_base_hierarchy(class)
                }) {
                    return Chain::Known(chain_member(
                        top,
                        class,
                        path,
                        stricter(self.declared, direct.access),
                    ));
                }
                self.work.push(AccessFrame {
                    top,
                    bottom: class,
                    next: 0,
                    path,
                    best: None,
                });
                return Chain::Pending;
            }
            if class != top && base.access == Access::Private {
                return Chain::Known(None);
            }
            path = stricter(path, base.access);
            class = base.class;
            if class != self.declaring && self.blocked.contains(&class) {
                return Chain::Known(None);
            }
        }
        Chain::Known(chain_member(top, class, path, self.declared))
    }
    fn access(&mut self, graph: &Graph, naming: EntityId) -> Option<Access> {
        if self.declared == Access::Private && naming != self.declaring {
            return None;
        }
        self.work.clear();
        if let Chain::Known(access) = self.chain(graph, naming) {
            return access;
        }
        let mut known = None;
        while let Some(frame) = self.work.last() {
            let Some(base) = graph.entity(frame.bottom).bases.get(frame.next).copied() else {
                let done = self.work.pop()?;
                let access = done
                    .best
                    .and_then(|best| chain_member(done.top, done.bottom, done.path, best));
                if self.work.is_empty() {
                    return access;
                }
                self.memo.insert(done.top, access);
                known = Some(access);
                continue;
            };
            if known.is_none() {
                known = if base.class == self.declaring {
                    Some(Some(self.declared))
                } else if graph.entity(base.class).bases.is_empty()
                    || self.blocked.contains(&base.class)
                {
                    Some(None)
                } else if let Some(&access) = self.memo.get(&base.class) {
                    Some(access)
                } else {
                    match self.chain(graph, base.class) {
                        Chain::Known(access) => {
                            self.memo.insert(base.class, access);
                            Some(access)
                        }
                        Chain::Pending => continue,
                    }
                };
            }
            let frame = self.work.last_mut()?;
            if let Some(access) = known.flatten().filter(|&access| access != Access::Private) {
                let access = stricter(access, base.access);
                frame.best = Some(frame.best.map_or(access, |best| match (best, access) {
                    (Access::Public, _) | (_, Access::Public) => Access::Public,
                    (Access::Protected, _) | (_, Access::Protected) => Access::Protected,
                    _ => Access::Private,
                }));
            }
            frame.next += 1;
            known = None;
        }
        None
    }
}
enum AccessTask {
    Visit(EntityId),
    Bases(EntityId, usize),
    Unreached(EntityId),
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
        let mut seen = HashSet::default();
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
        context: AccessContext,
    ) -> bool {
        if access == Access::Public || self.member_or_friend_at(scope, class, context) {
            return true;
        }
        if access == Access::Private {
            return false;
        }
        // Each invented-member query owns its own complete target and memo.
        let empty = HashSet::default();
        self.access_contexts_at(scope, context).any(|context| {
            MemberPaths::new(class, Access::Protected, &empty)
                .access(&self.graph, context)
                .is_some()
        })
    }

    // Summarize single-base chains without recursively walking each suffix.
    // Branch tasks resume only after a failed child; no later branch is
    // queried after an earlier one grants access.
    fn imported_declaration_access(
        &self,
        naming: EntityId,
        scope: ScopeId,
        paths: &mut MemberPaths<'_>,
        context: AccessContext,
    ) -> Option<GrantedAccess> {
        let mut work = vec![AccessTask::Visit(naming)];
        let mut unreached = HashSet::default();
        let mut leads = HashMap::default();
        'query: while let Some(task) = work.pop() {
            let naming = match task {
                AccessTask::Unreached(class) => {
                    unreached.insert(class);
                    continue;
                }
                AccessTask::Bases(class, index) => {
                    let Some(base) = self.graph.entity(class).bases.get(index) else {
                        continue;
                    };
                    work.push(AccessTask::Bases(class, index + 1));
                    if base.class != paths.declaring
                        && (paths.blocked.contains(&base.class)
                            || unreached.contains(&base.class)
                            || !super::base_facts::leads_to_base(
                                &self.graph,
                                base.class,
                                paths.declaring,
                                &mut leads,
                            ))
                    {
                        continue;
                    }
                    if self.imported_base_accessible(class, base.access, scope, context) {
                        if base.class != paths.declaring {
                            work.push(AccessTask::Unreached(base.class));
                        }
                        work.push(AccessTask::Visit(base.class));
                    }
                    continue;
                }
                AccessTask::Visit(class) => class,
            };
            let mut bottom = naming;
            let (mut edges, mut private, mut restricted) = (0usize, 0usize, 0usize);
            while bottom != paths.declaring {
                let [base] = self.graph.entity(bottom).bases.as_slice() else {
                    break;
                };
                edges += 1;
                if base.access == Access::Private {
                    private = edges;
                }
                if base.access != Access::Public {
                    restricted = edges;
                }
                bottom = base.class;
                if bottom != paths.declaring && paths.blocked.contains(&bottom) {
                    continue 'query;
                }
            }
            let bottom_access = if bottom == paths.declaring {
                Some(paths.declared)
            } else {
                paths.access(&self.graph, bottom)
            };
            let mut class = naming;
            for level in 0..=edges {
                let access = bottom_access
                    .filter(|&access| {
                        level == edges || access != Access::Private && private <= level + 1
                    })
                    .map(|access| {
                        stricter(
                            access,
                            if private > level {
                                Access::Private
                            } else if restricted > level {
                                Access::Protected
                            } else {
                                Access::Public
                            },
                        )
                    });
                if access == Some(Access::Public)
                    || access.is_some() && self.member_or_friend_at(scope, class, context)
                {
                    return Some(GrantedAccess::Unrestricted);
                }
                if access == Some(Access::Protected)
                    && let Some(context) =
                        self.access_contexts_at(scope, context).find(|&context| {
                            self.graph.is_base_of(class, context)
                                && paths.access(&self.graph, context).is_some()
                        })
                {
                    return Some(GrantedAccess::Protected(context));
                }
                if level != edges {
                    let base = self.graph.entity(class).bases.first()?;
                    if !self.imported_base_accessible(class, base.access, scope, context) {
                        continue 'query;
                    }
                    class = base.class;
                }
            }
            if bottom != paths.declaring {
                work.push(AccessTask::Bases(bottom, 0));
            }
        }
        None
    }

    // analyzer_virtual.cpp's nonfatal deallocation query calls accessible
    // directly for every matching declaration, without import-frontier checks,
    // protected-object restrictions, diagnostic emission or access deferral.
    pub(super) fn declaration_member_accessible(
        &self,
        binding: BindingId,
        naming: EntityId,
        scope: ScopeId,
    ) -> bool {
        self.scoped_declaration_access(binding, naming, scope)
            .is_some()
    }

    fn scoped_declaration_access(
        &self,
        binding: BindingId,
        naming: EntityId,
        scope: ScopeId,
    ) -> Option<GrantedAccess> {
        let declaring = self.graph.scope(self.graph.binding_scope(binding)).owner?;
        let blocked = HashSet::default();
        let mut paths = MemberPaths::new(declaring, self.graph.binding_access(binding), &blocked);
        self.imported_declaration_access(naming, scope, &mut paths, AccessContext::Scope)
    }

    /// Raw lexical destructor accessibility also retains the protected object rule.
    pub(super) fn subobject_destructor_accessible(
        &self,
        binding: BindingId,
        class: EntityId,
        object: EntityId,
        scope: ScopeId,
    ) -> bool {
        match self.scoped_declaration_access(binding, class, scope) {
            None => false,
            Some(GrantedAccess::Protected(via)) => {
                object == via || self.graph.is_base_of(via, object)
            }
            Some(GrantedAccess::Unrestricted) => true,
        }
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
        let declarations = if self.graph.has_member_import(binding) {
            self.member_declaration_frontier(naming, binding)
        } else {
            Vec::new()
        };
        let mut blocked = HashSet::default();
        if declarations.len() > 1 {
            blocked.extend(declarations.iter().map(|&(class, _)| class));
        }
        let mut reached = None;
        if declarations.is_empty() {
            let mut paths = MemberPaths::new(owner, self.graph.binding_access(binding), &blocked);
            reached = self.imported_declaration_access(
                naming,
                scope,
                &mut paths,
                AccessContext::Declaration,
            );
        } else {
            for (class, declaration) in declarations {
                let mut paths =
                    MemberPaths::new(class, self.graph.binding_access(declaration), &blocked);
                reached = self.imported_declaration_access(
                    naming,
                    scope,
                    &mut paths,
                    AccessContext::Declaration,
                );
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
    fn raw_class_access_does_not_inherit_unrelated_declaration_privileges() {
        let source = SourceBuffer::new(
            b"struct C;struct B{friend struct C;private:static void operator delete(void*);};struct C{void f();};struct D:B{};".to_vec(),
        ).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"raw-class-access").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            cursor.intern_name(b"B").unwrap(),
            cursor.intern_name(b"C").unwrap(),
            cursor.intern_name(b"D").unwrap(),
        ];
        let f = cursor.intern_name(b"f").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let [base, friend, derived] = names.map(|name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(scope, name, Filter::Prefix).unwrap())
                .entity
        });
        let friend_scope = analyzer.graph.entity(friend).scope.unwrap();
        let derived_scope = analyzer.graph.entity(derived).scope.unwrap();
        let binding = analyzer
            .graph
            .functions_at(
                analyzer.graph.entity(base).scope.unwrap(),
                super::super::allocation::key(crate::lex::vocabulary::Keyword::Delete, false),
            )
            .next()
            .unwrap();
        let function = analyzer
            .graph
            .binding(
                analyzer
                    .graph
                    .own(friend_scope, f, Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        for context in 0..3 {
            analyzer.declaration_access.class = (context == 0).then_some(friend_scope);
            analyzer.declaration_access.function = (context == 1).then_some(function);
            analyzer.declaration_access.substitution =
                (context == 2).then_some((derived_scope, friend_scope));
            assert!(analyzer.member_or_friend(derived_scope, base));
            assert!(!analyzer.declaration_member_accessible(binding, derived, derived_scope));
            assert!(analyzer.declaration_member_accessible(binding, derived, friend_scope));
        }
        assert!(
            analyzer
                .graph
                .analysis
                .substitution
                .current()
                .failure
                .is_none()
        );
    }

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
