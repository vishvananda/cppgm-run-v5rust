// Translated from analyzer_virtual.cpp::group_outside/group_dominates. See NOTICE.
//! Final-overrider queries use actual subobject identity. Primary-chain sharing
//! is an ABI projection, so this semantic query needs neither storage nor bodies.
use super::{
    Error,
    function_names::ValueKey,
    graph::{EntityId, Graph, MemberKind},
    types::TypeId,
};
use crate::support::source::SourceOffset;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SubobjectId(u32);
impl SubobjectId {
    fn index(self) -> usize {
        self.0 as usize
    }
}

/// The keys retain both parts of the final-overrider identity. The value is its
/// pure-specifier fact, independent of deletion or a definition being emitted.
type FinalOverriders = HashMap<(SubobjectId, EntityId), bool>;

#[derive(Default)]
pub(super) struct Groups {
    classes: HashMap<EntityId, FinalOverriders>,
}
impl Groups {
    pub(super) fn abstract_class(&self, class: EntityId) -> Option<bool> {
        self.classes
            .get(&class)
            .map(|functions| functions.values().any(|&pure| pure))
    }
    fn contains(&self, class: EntityId) -> bool {
        self.classes.contains_key(&class)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum VirtualSignature {
    Destructor,
    Function(ValueKey, TypeId),
}

type FinalFunction = (SubobjectId, EntityId);
type Query = (SubobjectId, VirtualSignature);
#[derive(Default)]
struct Declarations {
    lookup: HashMap<(EntityId, VirtualSignature), EntityId>,
    ranges: HashMap<EntityId, std::ops::Range<usize>>,
    ordered: Vec<VirtualSignature>,
    seen: HashSet<EntityId>,
}

#[derive(Clone, Copy)]
struct ParentId(u32);
struct Parent {
    object: SubobjectId,
    next: Option<ParentId>,
}

/// Immutable query results share one bulk store; only the reusable merge
/// buffer grows while a query is completed. No vector allocation per query.
#[derive(Default)]
struct Candidates {
    ranges: HashMap<Query, std::ops::Range<usize>>,
    storage: Vec<FinalFunction>,
    scratch: Vec<FinalFunction>,
    dominance: Dominance,
    work: Vec<(SubobjectId, Option<ParentId>)>,
}
impl Candidates {
    fn get(&self, query: Query) -> Result<&[FinalFunction], Error> {
        let range = self
            .ranges
            .get(&query)
            .ok_or(Error::Invariant("unresolved overrider query"))?;
        Ok(&self.storage[range.clone()])
    }
    fn record(&mut self, query: Query) {
        let first = self.storage.len();
        self.storage.append(&mut self.scratch);
        self.ranges.insert(query, first..self.storage.len());
    }
}

struct Subobject {
    class: EntityId,
    first_parent: Option<ParentId>,
}
#[derive(Default)]
struct Subobjects {
    nodes: Vec<Subobject>,
    parents: Vec<Parent>,
}
impl Subobjects {
    fn add(&mut self, class: EntityId) -> Result<SubobjectId, Error> {
        let id = SubobjectId(
            u32::try_from(self.nodes.len()).map_err(|_| Error::Limit("base subobject limit"))?,
        );
        self.nodes.push(Subobject {
            class,
            first_parent: None,
        });
        Ok(id)
    }
    fn build(graph: &Graph, class: EntityId) -> Result<Self, Error> {
        let mut result = Self::default();
        let root = result.add(class)?;
        let mut shared = HashMap::new();
        let mut work = vec![(root, 0)];
        while let Some((parent, next)) = work.last_mut() {
            let cls = result.nodes[parent.index()].class;
            let Some(edge) = graph.entity(cls).bases.get(*next) else {
                work.pop();
                continue;
            };
            *next += 1;
            if !graph.entity(edge.class).polymorphism.is_dynamic() {
                continue;
            }
            let known = edge
                .is_virtual
                .then(|| shared.get(&edge.class).copied())
                .flatten();
            let child = if let Some(id) = known {
                id
            } else {
                let id = result.add(edge.class)?;
                if edge.is_virtual {
                    shared.insert(edge.class, id);
                }
                id
            };
            let next = result.nodes[child.index()].first_parent;
            let edge = ParentId(
                u32::try_from(result.parents.len())
                    .map_err(|_| Error::Limit("base subobject edge limit"))?,
            );
            result.parents.push(Parent {
                object: *parent,
                next,
            });
            result.nodes[child.index()].first_parent = Some(edge);
            if known.is_none() {
                work.push((child, 0));
            }
        }
        Ok(result)
    }
    fn parents(&self, object: SubobjectId) -> impl Iterator<Item = SubobjectId> + '_ {
        std::iter::successors(self.nodes[object.index()].first_parent, |id| {
            self.parents[id.0 as usize].next
        })
        .map(|id| self.parents[id.0 as usize].object)
    }
    fn merge(
        &self,
        candidates: &mut Vec<(SubobjectId, EntityId)>,
        incoming: (SubobjectId, EntityId),
        dominance: &mut Dominance,
    ) {
        // A function in another occurrence of the same class is another
        // candidate, even though its FunctionEntity identity is equal.
        if candidates
            .iter()
            .any(|&(object, _)| dominance.contains(self, object, incoming.0))
        {
            return;
        }
        candidates.retain(|&(object, _)| !dominance.contains(self, incoming.0, object));
        candidates.push(incoming);
    }
    fn resolve(
        &self,
        query: Query,
        declarations: &Declarations,
        memo: &mut Candidates,
        location: SourceOffset,
    ) -> Result<(), Error> {
        // Demand only ancestors of this signature's declaration. In particular,
        // a sibling's unrelated virtual signatures are never queried here.
        let mut work = std::mem::take(&mut memo.work);
        work.clear();
        work.push((query.0, self.nodes[query.0.index()].first_parent));
        while let Some((id, next)) = work.last_mut() {
            let key = (*id, query.1);
            if memo.ranges.contains_key(&key) {
                work.pop();
                continue;
            }
            let object = &self.nodes[id.index()];
            if let Some(edge) = *next {
                let parent = &self.parents[edge.0 as usize];
                *next = parent.next;
                if !memo.ranges.contains_key(&(parent.object, query.1)) {
                    work.push((
                        parent.object,
                        self.nodes[parent.object.index()].first_parent,
                    ));
                }
                continue;
            }
            memo.scratch.clear();
            for parent in self.parents(*id) {
                let range = memo
                    .ranges
                    .get(&(parent, query.1))
                    .ok_or(Error::Invariant("unresolved parent overrider query"))?;
                for &candidate in &memo.storage[range.clone()] {
                    self.merge(&mut memo.scratch, candidate, &mut memo.dominance);
                }
            }
            if let Some(&own) = declarations.lookup.get(&(object.class, query.1)) {
                if memo.scratch.is_empty() {
                    memo.scratch.push((*id, own));
                }
                if memo.scratch.len() != 1 {
                    return Err(Error::at(
                        location,
                        "a virtual function has no unique final overrider",
                    ));
                }
            }
            memo.record(key);
            work.pop();
        }
        memo.work = work;
        Ok(())
    }
}

/// Containment queries share scratch and memoize a complete subobject-pair key.
/// Class identity alone cannot answer dominance in a repeated-base hierarchy.
#[derive(Default)]
struct Dominance {
    known: HashMap<(SubobjectId, SubobjectId), bool>,
    work: Vec<SubobjectId>,
    seen: HashSet<SubobjectId>,
}
impl Dominance {
    fn contains(&mut self, objects: &Subobjects, derived: SubobjectId, base: SubobjectId) -> bool {
        if derived == base {
            return true;
        }
        if let Some(&known) = self.known.get(&(derived, base)) {
            return known;
        }
        self.work.clear();
        self.seen.clear();
        self.work.push(base);
        let mut found = false;
        while let Some(object) = self.work.pop() {
            if object == derived {
                found = true;
                break;
            }
            if self.seen.insert(object) {
                self.work.extend(objects.parents(object));
            }
        }
        self.known.insert((derived, base), found);
        found
    }
}

impl Graph {
    /// A class is grouped only when it has secondary dynamic subobjects or a
    /// virtual base. Ordinary primary chains keep the compact PA13 recipe.
    pub(super) fn needs_final_overriders(&self, class: EntityId) -> bool {
        let bases = &self.entity(class).bases;
        bases
            .iter()
            .any(|base| base.is_virtual || self.virtuals.final_overriders.contains(base.class))
            || bases
                .iter()
                .filter(|base| self.entity(base.class).polymorphism.is_dynamic())
                .count()
                > 1
    }

    pub(super) fn virtual_signature(&self, function: EntityId) -> Result<VirtualSignature, Error> {
        let member = self.entity(function).member_function.map(|key| key.kind);
        if member == Some(MemberKind::Destructor) {
            return Ok(VirtualSignature::Destructor);
        }
        let (key, signature) = self.virtuals.signature(function)?;
        Ok(VirtualSignature::Function(key, signature))
    }

    fn index_virtual_declarations(
        &self,
        class: EntityId,
        declarations: &mut Declarations,
    ) -> Result<(), Error> {
        if declarations.ranges.contains_key(&class) {
            return Ok(());
        }
        let scope = self
            .entity(class)
            .scope
            .ok_or(Error::Invariant("virtual class scope"))?;
        let first = declarations.ordered.len();
        declarations.seen.clear();
        for index in 0..self.virtual_member_count(class)? {
            let Some(function) = self.virtual_member_at(class, index)? else {
                continue;
            };
            if self.entity(function).home == Some(scope)
                && self.virtuals.function(function).is_virtual
                && declarations.seen.insert(function)
            {
                let signature = self.virtual_signature(function)?;
                declarations.lookup.insert((class, signature), function);
                declarations.ordered.push(signature);
            }
        }
        declarations
            .ranges
            .insert(class, first..declarations.ordered.len());
        Ok(())
    }

    fn extend_final_overriders(&mut self, class: EntityId) -> Result<bool, Error> {
        // group_splice's primary-chain case: with one dynamic direct base, the
        // existing subobject relationships stay fixed. Only this class's own
        // declarations can replace a final overrider, uniformly for a signature.
        let mut bases = self
            .entity(class)
            .bases
            .iter()
            .filter(|base| self.entity(base.class).polymorphism.is_dynamic());
        let Some(base) = bases.next().map(|base| base.class) else {
            return Ok(false);
        };
        if bases.next().is_some() {
            return Ok(false);
        }
        let Some(inherited) = self.virtuals.final_overriders.classes.get(&base) else {
            return Ok(false);
        };
        let mut declarations = Declarations::default();
        self.index_virtual_declarations(class, &mut declarations)?;
        let mut signatures = HashMap::new();
        let mut finals = FinalOverriders::new();
        for (&(subobject, function), &pure) in inherited {
            let signature = if let Some(&known) = signatures.get(&function) {
                known
            } else {
                let signature = self.virtual_signature(function)?;
                signatures.insert(function, signature);
                signature
            };
            if let Some(&own) = declarations.lookup.get(&(class, signature)) {
                finals.insert((SubobjectId(0), own), self.virtuals.function(own).pure);
            } else {
                // The derived root is new; inherited occurrence identities are
                // shifted together, without collapsing any repeated subobject.
                let id = SubobjectId(
                    subobject
                        .0
                        .checked_add(1)
                        .ok_or(Error::Limit("base subobject limit"))?,
                );
                finals.insert((id, function), pure);
            }
        }
        for signature in &declarations.ordered {
            let own = declarations.lookup[&(class, *signature)];
            finals.insert((SubobjectId(0), own), self.virtuals.function(own).pure);
        }
        self.virtuals.final_overriders.classes.insert(class, finals);
        Ok(true)
    }

    pub(super) fn resolve_final_overriders(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.extend_final_overriders(class)? {
            return Ok(());
        }
        let objects = Subobjects::build(self, class)?;
        // Index each participating class once. Queries follow subobject and
        // declaration order, never hash iteration. Each (subobject, signature)
        // is resolved at most once within this class-completion query.
        let mut declarations = Declarations::default();
        let mut targets = Vec::new();
        for (index, object) in objects.nodes.iter().enumerate() {
            let id = SubobjectId(
                u32::try_from(index).map_err(|_| Error::Limit("base subobject limit"))?,
            );
            self.index_virtual_declarations(object.class, &mut declarations)?;
            let range = declarations.ranges[&object.class].clone();
            targets.extend(
                declarations.ordered[range]
                    .iter()
                    .map(|&signature| (id, signature)),
            );
        }
        let mut memo = Candidates::default();
        let mut finals = FinalOverriders::new();
        for query in targets {
            objects.resolve(query, &declarations, &mut memo, location)?;
            let chosen = *memo.get(query)?.first().ok_or(Error::Invariant(
                "virtual declaration has no final overrider",
            ))?;
            finals.insert(chosen, self.virtuals.function(chosen.1).pure);
        }
        self.virtuals.final_overriders.classes.insert(class, finals);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, layout::LayoutState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    fn inspect(source: &[u8], check: impl FnOnce(&Graph, EntityId)) {
        let source = SourceBuffer::new(source.to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"overrider-query").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"D").unwrap();
        let expressions = tree.expressions.mark();
        let syntax = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Elaborated).unwrap())
            .entity;
        check(&graph, class);
        assert!(matches!(
            graph.entity(class).layout,
            LayoutState::NotStarted
        ));
        assert!(graph.root_demands.as_slice().is_empty());
        assert_eq!(expressions, tree.expressions.mark());
        assert_eq!(syntax, tree.declarations.mark());
    }

    #[test]
    fn virtual_diamond_keeps_the_dominating_subobject_without_storage_demand() {
        inspect(b"struct R{virtual int f()=0;};struct A:virtual R{int f();};struct B:virtual R{};struct Mid:A,B{};struct D:Mid{};", |graph, class| {
            assert!(!graph.virtuals.abstract_class(class));
            let finals = &graph.virtuals.final_overriders.classes[&class];
            assert_eq!(finals.len(), 1);
            assert!(finals.values().all(|&pure| !pure));
        });
    }

    #[test]
    fn repeated_nonvirtual_bases_keep_distinct_final_overriders() {
        inspect(
            b"struct R{virtual int f()=0;};struct A:R{int f();};struct B:R{};struct D:A,B{};",
            |graph, class| {
                assert!(graph.virtuals.abstract_class(class));
                let finals = &graph.virtuals.final_overriders.classes[&class];
                assert_eq!(finals.len(), 2);
                assert_eq!(finals.values().filter(|&&pure| pure).count(), 1);
            },
        );
        inspect(
            b"struct R{virtual int f();};struct A:R{};struct B:R{};struct D:A,B{};",
            |graph, class| {
                let finals = &graph.virtuals.final_overriders.classes[&class];
                assert_eq!(finals.len(), 2);
                let functions: HashSet<_> = finals.keys().map(|&(_, function)| function).collect();
                assert_eq!(functions.len(), 1);
            },
        );
    }

    #[test]
    fn implicit_destructor_merges_both_roots_but_overloads_remain_distinct() {
        inspect(b"struct A{virtual ~A()=0;virtual void f(int)=0;};struct B{virtual ~B()=0;virtual void f(char)=0;};struct D:A,B{void f(int);void f(char);};", |graph, class| {
            assert!(!graph.virtuals.abstract_class(class));
            let finals = &graph.virtuals.final_overriders.classes[&class];
            assert_eq!(finals.len(), 3);
        });
    }

    #[test]
    fn conversion_targets_are_part_of_the_virtual_signature() {
        inspect(b"struct R{virtual operator int()=0;virtual operator long()=0;};struct A:virtual R{operator int();};struct B:virtual R{operator long();};struct D:A,B{};", |graph, class| {
            assert!(!graph.virtuals.abstract_class(class));
            assert_eq!(graph.virtuals.final_overriders.classes[&class].len(), 2);
        });
    }

    #[test]
    fn implicit_assignment_overrides_a_secondary_roots_virtual() {
        inspect(b"struct D;struct A{virtual int f();};struct B{virtual B&operator=(const D&)=0;};struct D:A,B{};", |graph, class| {
            assert!(!graph.virtuals.abstract_class(class));
            assert_eq!(graph.virtuals.final_overriders.classes[&class].len(), 2);
        });
    }

    #[test]
    fn a_class_relation_does_not_dominate_an_uncontained_occurrence() {
        for source in [
            b"struct R{virtual int f();};struct A:virtual R{int f();};struct B:virtual R{int f();};struct D:A,B{};".as_slice(),
            b"struct R{virtual int f();};struct A:virtual R{int f();};struct B:A{};struct D:A,virtual B{};",
            b"struct R{virtual int f();};struct A:virtual R{int f();};struct B:virtual A{};struct D:B,A{};",
        ] {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"ambiguous-overrider").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_semantics(true);
            assert!(matches!(analyzer.analyze(declarations), Err(Error::Diagnostic { message: "a virtual function has no unique final overrider", .. })));
        }
    }
}
