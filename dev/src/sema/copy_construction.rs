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
}
#[derive(Clone, Copy)]
pub(crate) struct SubobjectCopy {
    pub target: CopyTarget,
    pub ty: TypeId,
    pub constructor: Option<ConstructorId>,
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
                let selected =
                    selected.ok_or(Error::Invariant("deleted subobject in copy body"))?;
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
        let count = self.graph.entity(class).bases.len() + self.graph.entity(class).members.len();
        let mut actions = Vec::with_capacity(count);
        let mut child_index = 0;
        for index in 0..count {
            let bases = self.graph.entity(class).bases.len();
            let target = if index < bases {
                CopyTarget::Base(u32::try_from(index).map_err(|_| Error::Limit("copy base index"))?)
            } else {
                CopyTarget::Member(self.graph.entity(class).members[index - bases])
            };
            let ty = match target {
                CopyTarget::Base(index) => {
                    self.graph
                        .entity(self.graph.entity(class).bases[index as usize].class)
                        .ty
                }
                CopyTarget::Member(member) => self.graph.entity(member).ty,
            }
            .ok_or(Error::Invariant("copy action without type"))?;
            let constructor = if let Some((child, _, _)) = self.graph.copy_child(class, index)? {
                let (_, planned_child, selected) =
                    self.graph.copy_properties.get(properties).subobjects[child_index];
                child_index += 1;
                if child != planned_child {
                    return Err(Error::Invariant("copy action child identity"));
                }
                let selected = selected.ok_or(Error::Invariant("deleted copy action"))?;
                self.ready_copy_plan(child, selected.origin, location)?
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
            subobjects: Vec::new(),
            delegation: None,
            copy: Some(actions),
            triviality: if facts.trivial {
                Triviality::Trivial
            } else {
                Triviality::Nontrivial
            },
        };
        let id = self.graph.constructors.add(record)?;
        self.graph
            .entity_mut(self.graph.binding(binding).entity)
            .constructor = Some(id);
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
            no_inline: false,
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
            let syntax = self
                .graph
                .entity(function)
                .function_parameters
                .and_then(|p| self.tree.parameters.get(p.parameters).get(index));
            let name = syntax
                .map(|p| self.declarator_name(p.declarator))
                .transpose()?
                .flatten();
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
