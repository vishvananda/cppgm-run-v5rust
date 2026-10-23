// Translated from sema/analyzer_copy.cpp::defaulted_copy_facts. See NOTICE.
//! Callable copy plans are demanded separately from recursive selection properties.
//! Each property record owns one memo; children are completed before their parent.
use super::{
    Error,
    analyzer::Analyzer,
    construction::{Constructor, ConstructorId, ConstructorState, Triviality},
    copy_initialization::{CopyOrigin, CopySelection},
    copy_properties::CopyPropertiesId,
    graph::{Binding, BindingId, BindingKind, EntityId, MemberKey, MemberKind, ScopeKind},
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Origin {
    #[default]
    Declared,
    Implicit,
}
#[derive(Clone, Copy)]
pub(crate) enum CopyTarget {
    Base(u32),
    Member(EntityId),
    VirtualBase(EntityId),
}
#[derive(Clone, Copy)]
pub(crate) struct SubobjectCopy {
    pub target: CopyTarget,
    pub ty: TypeId,
    pub constructor: Option<ConstructorId>,
}
impl super::graph::Graph {
    /// Defaulted constructors select direct bases, members, then indirect
    /// virtual bases. This is plan order; lowering owns construction order.
    pub(super) fn copy_targets(&self, class: EntityId) -> Result<Vec<CopyTarget>, Error> {
        let entity = self.entity(class);
        let mut targets = Vec::with_capacity(entity.bases.len() + entity.members.len());
        for index in 0..entity.bases.len() {
            targets.push(CopyTarget::Base(
                u32::try_from(index).map_err(|_| Error::Limit("copy base index"))?,
            ));
        }
        targets.extend(entity.members.iter().copied().map(CopyTarget::Member));
        targets.extend(
            self.indirect_virtual_bases(class)
                .into_iter()
                .map(CopyTarget::VirtualBase),
        );
        Ok(targets)
    }
    pub(super) fn copy_target_type(
        &self,
        class: EntityId,
        target: CopyTarget,
    ) -> Result<TypeId, Error> {
        let entity = match target {
            CopyTarget::Base(index) => self.entity(class).bases[index as usize].class,
            CopyTarget::Member(member) | CopyTarget::VirtualBase(member) => member,
        };
        self.entity(entity)
            .ty
            .ok_or(Error::Invariant("copy action without type"))
    }
    pub(super) fn copy_target_child(
        &self,
        class: EntityId,
        target: CopyTarget,
    ) -> Result<Option<(EntityId, Option<EntityId>, Cv)>, Error> {
        let mut ty = self.copy_target_type(class, target)?;
        while let TypeKind::Array { element, .. } = self.types.get(ty).kind {
            ty = element;
        }
        Ok(if let TypeKind::Class(child) = self.types.get(ty).kind {
            Some((
                child,
                match target {
                    CopyTarget::Member(member) => Some(member),
                    _ => None,
                },
                self.types.get(ty).cv,
            ))
        } else {
            None
        })
    }
    /// `object_lowering.cpp::copies_storage`: an instantiated out-of-class
    /// defaulted copy stays nontrivial for ABI passing, but transfers storage
    /// when every selected subobject copy is trivial. Synthesis has already
    /// published these facts; lowering performs no definition demand.
    pub(crate) fn lower_constructor_copies_storage(&self, id: ConstructorId) -> bool {
        self.constructors.get(id).copy_transfer == super::construction::CopyTransfer::Storage
    }
    // Publish this per-plan fact once with the selected copy actions, rather
    // than walking all subobjects again at every copy's lowering use.
    pub(super) fn constructor_copies_storage(
        &self,
        plan: &super::construction::Constructor,
    ) -> bool {
        let Some(actions) = plan.copy.as_ref() else {
            return false;
        };
        if !plan.nontrivial() {
            return true;
        }
        plan.binding
            .and_then(|binding| {
                self.function_templates
                    .bodies
                    .source(self.binding(binding).entity)
            })
            .is_some_and(|source| {
                matches!(
                    self.entity(source).special_definition,
                    Some(super::special_members::SpecialDefinition::Defaulted {
                        first: false,
                        body: None,
                    })
                )
            })
            && actions.iter().all(|action| {
                action
                    .constructor
                    .is_none_or(|id| !self.constructors.get(id).nontrivial())
            })
    }
}
impl Analyzer<'_> {
    pub(super) fn build_implicit_copy_plan(
        &mut self,
        class: EntityId,
        kind: RefKind,
    ) -> Result<ConstructorId, Error> {
        self.build_copy_plans(class, CopyOrigin::Implicit { class, kind }, SourceOffset(0))
    }
    pub(super) fn implicit_copy_bindings(
        &mut self,
        class: EntityId,
    ) -> Result<[Option<BindingId>; 2], Error> {
        self.prepare_copy_properties(class)?;
        let copies = self
            .graph
            .entity(class)
            .implicit_copies
            .ok_or(Error::Invariant("copy declarations without properties"))?;
        let mut bindings = [None; 2];
        for (index, (kind, properties)) in [
            (RefKind::Lvalue, copies.copy_properties),
            (RefKind::Rvalue, copies.move_properties),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(properties) = properties {
                bindings[index] = Some(self.implicit_copy_binding(class, kind, properties)?);
            }
        }
        Ok(bindings)
    }
    pub(super) fn declared_copy_plan(
        &mut self,
        class: EntityId,
        binding: BindingId,
    ) -> Result<ConstructorId, Error> {
        self.prepare_copy_children(class)?;
        self.prepare_declared_copy(binding, class)?;
        self.build_copy_plans(class, CopyOrigin::Declared(binding), SourceOffset(0))
    }
    pub(super) fn copy_constructor_plan(
        &mut self,
        class: EntityId,
        selected: CopySelection,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        self.build_copy_plans(class, selected.origin, location)
    }
    fn copy_plan_properties(
        &self,
        class: EntityId,
        origin: CopyOrigin,
    ) -> Result<Option<CopyPropertiesId>, Error> {
        match origin {
            CopyOrigin::Declared(binding) => Ok(self
                .graph
                .entity(self.graph.binding(binding).entity)
                .copy_properties),
            CopyOrigin::Implicit { kind, .. } => {
                let copies = self
                    .graph
                    .entity(class)
                    .implicit_copies
                    .ok_or(Error::Invariant("copy plan without properties"))?;
                Ok(Some(
                    if kind == RefKind::Lvalue {
                        copies.copy_properties
                    } else {
                        copies.move_properties
                    }
                    .ok_or(Error::Invariant("copy plan without selected properties"))?,
                ))
            }
        }
    }
    fn ready_copy_plan(
        &mut self,
        class: EntityId,
        origin: CopyOrigin,
        location: SourceOffset,
    ) -> Result<Option<ConstructorId>, Error> {
        if let Some(properties) = self.copy_plan_properties(class, origin)? {
            Ok(match self.graph.copy_properties.get(properties).plan {
                ConstructorState::Ready(id) => Some(id),
                _ => None,
            })
        } else if let CopyOrigin::Declared(binding) = origin {
            self.selected_constructor_plan(binding, location).map(Some)
        } else {
            Err(Error::Invariant("implicit copy plan without properties"))
        }
    }
    fn build_copy_plans(
        &mut self,
        class: EntityId,
        origin: CopyOrigin,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        if let Some(id) = self.ready_copy_plan(class, origin, location)? {
            return Ok(id);
        }
        let mut work = vec![(class, origin, 0usize)];
        while let Some((class, origin, next)) = work.last_mut() {
            let properties = self
                .copy_plan_properties(*class, *origin)?
                .ok_or(Error::Invariant("copy plan work item"))?;
            let record = self.graph.copy_properties.get(properties);
            let deleted = record.deleted;
            if *next == 0 {
                if matches!(record.plan, ConstructorState::InProgress) {
                    return Err(Error::Invariant("cyclic copy body demand"));
                }
                self.graph.copy_properties.get_mut(properties).plan = ConstructorState::InProgress;
            }
            if let Some(&(_, child, selected)) = self
                .graph
                .copy_properties
                .get(properties)
                .subobjects
                .get(*next)
            {
                *next += 1;
                let Some(selected) = selected else {
                    if deleted {
                        continue;
                    }
                    return Err(Error::Invariant("deleted subobject in copy body"));
                };
                if self
                    .ready_copy_plan(child, selected.origin, location)?
                    .is_none()
                {
                    work.push((child, selected.origin, 0));
                }
                continue;
            }
            let class = *class;
            let origin = *origin;
            work.pop();
            self.publish_copy_plan(class, origin, properties, location)?;
        }
        self.ready_copy_plan(class, origin, location)?
            .ok_or(Error::Invariant("unfinished copy plan"))
    }
    fn publish_copy_plan(
        &mut self,
        class: EntityId,
        origin: CopyOrigin,
        properties: CopyPropertiesId,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        let binding = match origin {
            CopyOrigin::Declared(binding) => binding,
            CopyOrigin::Implicit { kind, .. } => {
                self.implicit_copy_binding(class, kind, properties)?
            }
        };
        self.prepare_copy_parameters(binding)?;
        let targets = self.graph.copy_targets(class)?;
        let mut actions = Vec::with_capacity(targets.len());
        let mut child_index = 0;
        for target in targets {
            let ty = self.graph.copy_target_type(class, target)?;
            let constructor =
                if let Some((child, _, _)) = self.graph.copy_target_child(class, target)? {
                    let &(_, planned_child, selected) = self
                        .graph
                        .copy_properties
                        .get(properties)
                        .subobjects
                        .get(child_index)
                        .ok_or(Error::Invariant("missing copy child selection"))?;
                    child_index += 1;
                    if child != planned_child {
                        return Err(Error::Invariant("copy action child identity"));
                    }
                    if let Some(selected) = selected {
                        self.ready_copy_plan(child, selected.origin, location)?
                    } else if self.graph.copy_properties.get(properties).deleted {
                        // The frozen deleted-property result retains this subobject
                        // without a callable selection; explicit roots still emit it.
                        None
                    } else {
                        return Err(Error::Invariant("deleted copy action"));
                    }
                } else {
                    None
                };
            actions.push(SubobjectCopy {
                target,
                ty,
                constructor,
            });
        }
        let (object_pointer, presentation_type, callable_type) =
            self.constructor_types(class, Some(binding))?;
        let facts = self.graph.copy_properties.get(properties);
        let record = Constructor {
            class,
            binding: Some(binding),
            object_pointer,
            presentation_type,
            callable_type,
            nothrow: facts.nothrow,
            deleted: facts.deleted,
            const_default: true,
            members: Vec::new(),
            written_members: Vec::new(),
            bases: Vec::new(),
            indirect_bases: Vec::new(),
            subobjects: Vec::new(),
            delegation: None,
            copy: Some(actions),
            copy_transfer: super::construction::CopyTransfer::Call,
            triviality: if facts.trivial {
                Triviality::Trivial
            } else {
                Triviality::Nontrivial
            },
        };
        // A declaration plan may already be selected by an earlier use. Keep
        // that identity when the unit-tail template definition supplies facts.
        let id = self.store_constructor(record)?;
        self.graph.copy_properties.get_mut(properties).plan = ConstructorState::Ready(id);
        Ok(id)
    }
    fn implicit_copy_binding(
        &mut self,
        class: EntityId,
        kind: RefKind,
        properties: CopyPropertiesId,
    ) -> Result<BindingId, Error> {
        if let Some(binding) = self.graph.copy_properties.get(properties).callable {
            return Ok(binding);
        }
        let copies = self
            .graph
            .entity(class)
            .implicit_copies
            .ok_or(Error::Invariant("implicit copy signature"))?;
        let parameter = if kind == RefKind::Lvalue {
            copies.copy.map(|(ty, _)| ty)
        } else {
            copies.moving
        }
        .ok_or(Error::Invariant("implicit copy parameter"))?;
        let void = self.graph.types.scalar(Scalar::Void)?;
        let signature = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters: vec![parameter],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("implicit copy home"))?;
        let function = self.graph.add_entity(Some(signature), None, Keyword::Int)?;
        let nothrow = self.graph.copy_properties.get(properties).nothrow;
        let entity = self.graph.entity_mut(function);
        entity.copy_origin = Origin::Implicit;
        entity.copy_kind = Some(kind);
        entity.copy_properties = Some(properties);
        if self.graph.copy_properties.get(properties).deleted {
            self.graph.entity_mut(function).availability =
                super::graph::FunctionAvailability::Deleted;
        }
        let entity = self.graph.entity_mut(function);
        entity.defined = true;
        entity.inline = super::specifiers::InlinePreference::Inline;
        entity.definition_emission = super::function_demands::DefinitionEmission::Demanded;
        entity.exceptions = super::exceptions::ExceptionState::Value(if nothrow {
            super::exceptions::ExceptionSet::None
        } else {
            super::exceptions::ExceptionSet::All
        });
        entity.member_function = Some(MemberKey {
            class,
            kind: MemberKind::Constructor,
            signature,
            shape: None,
        });
        entity.function = Some(super::graph::FunctionProperties {
            effects: super::graph::FunctionEffect::Default,
            no_inline: false,
            returns: super::graph::FunctionReturn::MayReturn,
            internal_linkage: false,
            language: super::linkage::Language::Cpp,
            denotes: None,
            is_static: false,
            special: None,
            allocation: None,
        });
        let binding = self
            .graph
            .add_implicit_copy_binding(home, function, signature)?;
        self.graph.copy_properties.get_mut(properties).callable = Some(binding);
        Ok(binding)
    }
    fn prepare_copy_parameters(&mut self, binding: BindingId) -> Result<(), Error> {
        let function = self.graph.binding(binding).entity;
        if self.graph.entity(function).scope.is_some() {
            return Ok(());
        }
        let home = self
            .graph
            .entity(function)
            .home
            .ok_or(Error::Invariant("copy function home"))?;
        let scope = self
            .graph
            .add_scope(ScopeKind::Function, home, None, None)?;
        self.graph.scope_mut(scope).owner = Some(function);
        self.graph.entity_mut(function).scope = Some(scope);
        let signature = self
            .graph
            .entity(function)
            .ty
            .ok_or(Error::Invariant("copy function type"))?;
        let count = self.graph.types.function_parameter_count(signature)?;
        for index in 0..count {
            let parameter = self.graph.types.function_parameter(signature, index)?;
            // Implicit and explicitly defaulted copy bodies are synthesized.
            // Their parameter objects are unnamed even when the declaration
            // retained a written parameter name for its source-facing clause.
            let name = None;
            let other = self.graph.add_entity(Some(parameter), name, Keyword::Int)?;
            self.graph.add_binding(
                scope,
                Some(scope),
                Binding {
                    kind: BindingKind::Parameter,
                    name,
                    entity: other,
                    ty: Some(parameter),
                    key: Keyword::Int,
                },
            )?;
            self.graph
                .entity_mut(function)
                .parameter_entities
                .push(other);
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
    fn indirect_copy_selections_are_unique_ordered_and_demand_owned() {
        let source = SourceBuffer::new(b"struct V{int n;V(const V&);};struct W{int w;};struct A:virtual V{};struct B:virtual V{};struct D:A,B,virtual W{int m;};struct Unused:A{};struct E:A{E(const E&)=default;};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"indirect-copy-plan").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"D".as_slice(), b"V", b"W", b"Unused", b"E"]
            .map(|name| cursor.intern_name(name).unwrap());
        let syntax = (tree.expressions.mark(), tree.declarations.mark());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let [class, shared, direct, unused, defaulted] = names.map(|name| {
            analyzer
                .graph
                .binding(
                    analyzer
                        .graph
                        .own(global, name, super::super::lookup::Filter::Elaborated)
                        .unwrap(),
                )
                .entity
        });
        let binding = analyzer.graph.entity(defaulted).declared_constructors[0];
        let function = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity);
        let facts = analyzer
            .graph
            .copy_properties
            .get(function.copy_properties.unwrap());
        // Explicit defaulting selects the indirect base before E publishes
        // virtual facts, while its callable body remains undemanded.
        assert_eq!(facts.subobjects.len(), 2);
        assert_eq!(facts.subobjects[1].1, shared);
        assert!(matches!(facts.plan, ConstructorState::NotStarted));
        analyzer.prepare_copy_properties(class).unwrap();
        let id = analyzer
            .build_implicit_copy_plan(class, RefKind::Lvalue)
            .unwrap();
        assert_eq!(
            id,
            analyzer
                .build_implicit_copy_plan(class, RefKind::Lvalue)
                .unwrap()
        );
        let plan = analyzer.graph.constructors.get(id);
        let actions = plan.copy.as_ref().unwrap();
        assert_eq!(actions.len(), 5); // three direct bases, a member, one indirect virtual base
        assert!(matches!(actions[2].target, CopyTarget::Base(2)));
        assert!(matches!(actions[3].target, CopyTarget::Member(_)));
        assert!(matches!(actions[4].target, CopyTarget::VirtualBase(base) if base == shared));
        assert_eq!(
            actions
                .iter()
                .filter(|action| matches!(action.target, CopyTarget::VirtualBase(_)))
                .count(),
            1
        );
        assert_eq!(
            analyzer
                .graph
                .copy_target_type(class, actions[2].target)
                .unwrap(),
            analyzer.graph.entity(direct).ty.unwrap()
        );
        assert!(actions[4].constructor.is_some());
        assert!(!plan.nothrow && plan.nontrivial());
        assert!(analyzer.graph.entity(unused).implicit_copies.is_none());
        assert!(analyzer.graph.storage_layouts.is_empty());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(
            syntax,
            (
                analyzer.tree.expressions.mark(),
                analyzer.tree.declarations.mark()
            )
        );
    }

    #[test]
    fn defaulted_copy_keeps_written_clause_but_synthesizes_unnamed_parameters() {
        let source = SourceBuffer::new(
            b"struct M{M();M(M&&);};struct X{M member;X();X(X&& other)=default;};void f(){X a;X b(static_cast<X&&>(a));}".to_vec(),
        ).unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"synthesized-copy-parameters").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"X").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let class = graph
            .binding(
                graph
                    .own(graph.global, name, super::super::lookup::Filter::Elaborated)
                    .unwrap(),
            )
            .entity;
        let function = graph.entity(
            graph
                .binding(graph.entity(class).declared_constructors[1])
                .entity,
        );
        let parameters = function.function_parameters.unwrap();
        let written = tree.parameters.get(parameters.parameters)[0];
        assert!(
            tree.declarators
                .get(written.declarator.unwrap())
                .name
                .is_some()
        );
        assert_eq!(function.parameter_entities.len(), 1);
        assert!(graph.entity(function.parameter_entities[0]).name.is_none());
    }

    #[test]
    fn ranking_copy_declarations_reuses_identity_without_building_bodies() {
        let source = SourceBuffer::new(b"struct Leaf{Leaf(const Leaf&);};struct Box{Leaf member;};struct Unused{Leaf member;};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"copy-declarations").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"Box".as_slice(), b"Unused"].map(|name| cursor.intern_name(name).unwrap());
        let syntax_mark = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let [class, unused] = names.map(|name| {
            analyzer
                .graph
                .binding(
                    analyzer
                        .graph
                        .own(global, name, super::super::lookup::Filter::Elaborated)
                        .unwrap(),
                )
                .entity
        });
        let first = analyzer.implicit_copy_bindings(class).unwrap();
        let second = analyzer.implicit_copy_bindings(class).unwrap();
        assert_eq!(first, second);
        for binding in first.into_iter().flatten() {
            let function = analyzer
                .graph
                .entity(analyzer.graph.binding(binding).entity);
            assert!(function.constructor.is_none() && function.scope.is_none());
            let properties = analyzer
                .graph
                .copy_properties
                .get(function.copy_properties.unwrap());
            assert!(matches!(properties.plan, ConstructorState::NotStarted));
        }
        assert!(analyzer.graph.entity(unused).implicit_copies.is_none());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.graph.storage_layouts.is_empty());
        assert_eq!(syntax_mark, tree.expressions.mark());
    }

    #[test]
    fn repeated_copy_uses_share_plans_without_syntax_or_unused_body_demand() {
        let source = SourceBuffer::new(b"struct Leaf{int n;Leaf();Leaf(const Leaf&);};struct Box{int tag;Leaf a[2];};struct Unused{Leaf member;};void f(Box&a){Box b(a);Box c(a);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"copy-plan-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let box_name = cursor.intern_name(b"Box").unwrap();
        let unused_name = cursor.intern_name(b"Unused").unwrap();
        let expressions = tree.expressions.mark();
        let syntax_declarations = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(expressions, analyzer.tree.expressions.mark());
        assert_eq!(syntax_declarations, analyzer.tree.declarations.mark());
        let graph = &analyzer.graph;
        let class = graph
            .binding(
                graph
                    .own(global, box_name, super::super::lookup::Filter::Elaborated)
                    .unwrap(),
            )
            .entity;
        let unused = graph
            .binding(
                graph
                    .own(
                        global,
                        unused_name,
                        super::super::lookup::Filter::Elaborated,
                    )
                    .unwrap(),
            )
            .entity;
        assert!(
            graph.entity(unused).copy_preparation
                == super::super::copy_properties::Preparation::Pending
        );
        let copies = graph.entity(class).implicit_copies.unwrap();
        let properties = graph.copy_properties.get(copies.copy_properties.unwrap());
        let ConstructorState::Ready(plan) = properties.plan else {
            panic!("copy plan was demanded");
        };
        let constructor = graph.constructors.get(plan);
        assert!(constructor.nontrivial());
        assert_eq!(constructor.copy.as_ref().unwrap().len(), 2);
        let actions = constructor.copy.as_ref().unwrap();
        assert!(actions[0].constructor.is_none());
        let child = graph.constructors.get(actions[1].constructor.unwrap());
        assert!(child.copy.is_none()); // The leaf's user-provided body remains external.
        assert!(
            !graph
                .entity(graph.binding(child.binding.unwrap()).entity)
                .defined
        );
        assert!(matches!(
            graph
                .copy_properties
                .get(copies.move_properties.unwrap())
                .plan,
            ConstructorState::NotStarted
        ));
        let binding = constructor.binding.unwrap();
        assert!(graph.binding_access(binding) == super::super::graph::Access::Public);
        assert_eq!(analyzer.declared_copy_plan(class, binding).unwrap(), plan);
    }
}
