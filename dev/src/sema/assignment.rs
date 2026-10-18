// Translated from sema/analyzer_copy.cpp. See NOTICE.
//! Implicit assignment declarations and their selected subobject actions.
//! The constructor-property worklist prepares children before publishing these facts.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{Conversion, ExprFacts, ObjectArgument, ObjectSource, ValueCategory},
    function_names::ValueKey,
    function_templates::TemplateOrdering,
    graph::{BindingId, EntityId, FunctionAvailability, Graph, Mutability, ScopeId, ScopeKind},
    procedural_calls::ArgumentSequence,
    special_members::SpecialDefinition,
    types::{Cv, RefKind, TypeId, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator, support::source::SourceOffset};
use std::{cmp::Ordering, collections::HashSet};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Origin {
    #[default]
    Declared,
    Implicit,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ImplicitUse {
    #[default]
    Unused,
    Used,
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
    fn assignment_parameter_plans_follow_reached_selections_without_reparsing() {
        let source = SourceBuffer::new(b"struct M{int n;M(const M&);M&operator=(M);~M();};struct X{M m[9];};struct Unused{M m;};void f(X&a,const X&b){a=b;a=b;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"assignment-plans").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            cursor.intern_name(b"X").unwrap(),
            cursor.intern_name(b"Unused").unwrap(),
        ];
        let expressions = tree.expressions.mark();
        let declarations_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer
                    .graph
                    .own(global, name, super::super::lookup::Filter::Elaborated)
                    .unwrap();
                analyzer.graph.binding(binding).entity
            })
            .collect();
        let home = analyzer.graph.entity(classes[0]).scope.unwrap();
        let function = analyzer
            .graph
            .functions_at(home, key().unwrap())
            .map(|b| analyzer.graph.binding(b).entity)
            .find(|&f| analyzer.graph.entity(f).assignment_kind == Some(RefKind::Lvalue))
            .unwrap();
        let action = analyzer.graph.lower_assignment(function).unwrap().actions[0];
        assert!(action.parameter_copy.is_some());
        assert!(action.parameter_constructor.is_none());
        analyzer.finish_lowering_boundaries().unwrap();
        let action = analyzer.graph.lower_assignment(function).unwrap().actions[0];
        let constructor = action.parameter_constructor.unwrap();
        let plan = analyzer.graph.lower_constructor(constructor);
        let child = plan.class;
        assert_eq!(
            analyzer
                .copy_constructor_plan(child, action.parameter_copy.unwrap(), SourceOffset(0))
                .unwrap(),
            constructor
        );
        let selected = action.function.unwrap();
        let signature = analyzer.graph.lower_entity_type(selected).unwrap();
        let parameter = analyzer
            .graph
            .types
            .function_parameter(signature, 0)
            .unwrap();
        assert_eq!(analyzer.graph.class_of(parameter), Some(child));
        assert!(analyzer.graph.destructor_binding(child).is_some());
        assert!(
            analyzer.graph.entity(classes[1]).copy_preparation
                == super::super::copy_properties::Preparation::Pending
        );
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), declarations_count);
    }

    #[test]
    fn assignment_demand_keeps_syntax_and_reuses_deepest_first_selections() {
        let source = SourceBuffer::new(b"struct Leaf{int n;};struct Holder{Leaf a;};struct Unused{int n;};int f(Holder&a,const Holder&b){a=b;a=b;return 0;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"assignment-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            cursor.intern_name(b"Leaf").unwrap(),
            cursor.intern_name(b"Holder").unwrap(),
            cursor.intern_name(b"Unused").unwrap(),
        ];
        let expressions = tree.expressions.mark();
        let declarations_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(expressions, analyzer.tree.expressions.mark());
        assert_eq!(declarations_count, analyzer.tree.declarations.mark());
        assert!(analyzer.assignment_scratch.candidates.is_empty());
        assert!(analyzer.assignment_scratch.candidates.capacity() >= 2);
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer
                    .graph
                    .own(global, name, super::super::lookup::Filter::Elaborated)
                    .unwrap();
                analyzer.graph.binding(binding).entity
            })
            .collect();
        let graph = &analyzer.graph;
        assert!(
            graph.entity(classes[2]).copy_preparation
                == super::super::copy_properties::Preparation::Pending
        );
        for &class in &classes[..2] {
            assert!(
                graph.entity(class).copy_preparation
                    == super::super::copy_properties::Preparation::Ready
            );
            let home = graph.entity(class).scope.unwrap();
            let bindings: Vec<_> = graph.functions_at(home, key().unwrap()).collect();
            assert_eq!(bindings.len(), 2);
            for binding in bindings {
                let function = graph.binding(binding).entity;
                let facts = graph.lower_assignment(function).unwrap();
                assert!(facts.trivial && facts.nothrow && !facts.deleted);
                assert_eq!(facts.actions.len(), 1);
                if class == classes[1] {
                    let selected = facts.actions[0].function.unwrap();
                    assert_eq!(graph.entity(selected).home, graph.entity(classes[0]).scope);
                }
            }
        }
    }

    #[test]
    fn template_assignment_signature_is_cached_separately_from_body_demand() {
        let source = SourceBuffer::new(
            b"struct M{template<class T>void operator=(T const&)volatile noexcept(sizeof(T)>0){}};\
              struct X{volatile M m;};struct Unused{volatile M m;};\
              void f(X&a,const X&b){a=b;a=b;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(
            Preprocessor::with_name(&source, b"template-assignment-demand").unwrap(),
        );
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            cursor.intern_name(b"M").unwrap(),
            cursor.intern_name(b"X").unwrap(),
            cursor.intern_name(b"Unused").unwrap(),
        ];
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer
                    .graph
                    .own(global, name, super::super::lookup::Filter::Elaborated)
                    .unwrap();
                analyzer.graph.binding(binding).entity
            })
            .collect();
        let home = analyzer.graph.entity(classes[1]).scope.unwrap();
        let assignment = analyzer
            .graph
            .functions_at(home, key().unwrap())
            .map(|b| analyzer.graph.binding(b).entity)
            .find(|&f| analyzer.graph.entity(f).assignment_kind == Some(RefKind::Lvalue))
            .unwrap();
        let properties = analyzer.graph.lower_assignment(assignment).unwrap();
        assert!(!properties.trivial && !properties.deleted && properties.nothrow);
        let selected = properties.actions[0].function.unwrap();
        assert!(
            analyzer
                .graph
                .function_templates
                .primary(selected)
                .is_some()
        );
        assert!(
            !analyzer
                .graph
                .function_templates
                .specializations
                .contains(selected)
        );
        let ty = analyzer.graph.entity(classes[0]).ty.unwrap();
        let ty = analyzer
            .graph
            .types
            .qualify(ty, Cv(Cv::CONST.0 | 2))
            .unwrap();
        let source = ExprFacts::new(ty, ValueCategory::Lvalue);
        let expressions = analyzer.tree.expressions.mark();
        let declarations = analyzer.tree.declarations.mark();
        for _ in 0..2 {
            let binding = analyzer
                .subobject_assignment(classes[0], source, Cv(2))
                .unwrap()
                .unwrap();
            assert_eq!(analyzer.graph.binding(binding).entity, selected);
            assert!(analyzer.assignment_scratch.candidates.is_empty());
            assert!(analyzer.assignment_scratch.seen.is_empty());
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.declarations.mark(), declarations);
        let primary = analyzer.graph.function_templates.primary(selected).unwrap();
        let primary = analyzer.graph.binding(primary).entity;
        assert!(!analyzer.graph.function_templates.body_demanded(primary));
        analyzer.finish_lowering_boundaries().unwrap();
        assert!(analyzer.graph.function_templates.body_demanded(primary));
        analyzer.finish_lowering_boundaries().unwrap();
        assert!(!analyzer.graph.function_templates.bodies.started(selected));
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.declarations.mark(), declarations);
        assert!(
            analyzer.graph.entity(classes[2]).copy_preparation
                == super::super::copy_properties::Preparation::Pending
        );
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ExceptionSource {
    #[default]
    Explicit,
    Subobjects,
}
pub(super) fn key() -> Result<ValueKey, Error> {
    super::operator_candidates::operator_key(Punctuator::Assign)
}
#[derive(Default)]
pub(super) struct Scratch {
    candidates: Vec<Candidate>,
    seen: HashSet<EntityId>,
}
#[derive(Clone, Copy)]
struct Candidate {
    binding: BindingId,
    object: Conversion,
    source: ArgumentSequence,
}
pub(crate) struct Properties {
    pub deleted: bool,
    pub trivial: bool,
    pub nothrow: bool,
    pub actions: Vec<Action>,
}
#[derive(Clone, Copy)]
pub(crate) struct Action {
    pub member: Option<EntityId>,
    pub base: Option<(EntityId, u32)>,
    pub ty: TypeId,
    pub function: Option<EntityId>,
    parameter_copy: Option<super::copy_initialization::CopySelection>,
    pub parameter_constructor: Option<super::construction::ConstructorId>,
}
impl Graph {
    pub(crate) fn lower_implicit_assignment_demanded(&self, class: EntityId) -> bool {
        self.entity(class).implicit_assignment_use == ImplicitUse::Used
    }
    pub(crate) fn lower_assignment(&self, function: EntityId) -> Option<&Properties> {
        self.entity(function).assignment.as_ref()
    }
    pub(crate) fn lower_implicit_assignment(&self, function: EntityId) -> bool {
        self.entity(function).assignment_origin == Origin::Implicit
    }
    pub(super) fn ignored_assignment(&self, function: EntityId) -> bool {
        let entity = self.entity(function);
        entity.assignment_kind == Some(RefKind::Rvalue)
            && entity.assignment.as_ref().is_some_and(|p| p.deleted)
    }
}
impl Analyzer<'_> {
    /// Prepare only the selected edges of an assignment reached by lowering.
    pub(super) fn assignment_boundaries(
        &mut self,
        function: EntityId,
        work: &mut Vec<EntityId>,
        constructors: &mut Vec<super::construction::ConstructorId>,
    ) -> Result<(), Error> {
        let count = self
            .graph
            .entity(function)
            .assignment
            .as_ref()
            .map_or(0, |p| p.actions.len());
        for index in 0..count {
            let action = self
                .graph
                .entity(function)
                .assignment
                .as_ref()
                .ok_or(Error::Invariant("assignment boundary plan"))?
                .actions[index];
            if let Some(selected) = action.function {
                let binding = self.graph.function_origin(selected)?;
                self.demand_specialization_body(binding);
                work.push(selected);
                if let Some(copy) = action.parameter_copy {
                    let signature = self.graph.lower_entity_type(selected)?;
                    let parameter = self.graph.types.function_parameter(signature, 0)?;
                    let class = self
                        .graph
                        .class_of(parameter)
                        .ok_or(Error::Invariant("assignment parameter class"))?;
                    let constructor = self.copy_constructor_plan(class, copy, SourceOffset(0))?;
                    self.graph
                        .entity_mut(function)
                        .assignment
                        .as_mut()
                        .ok_or(Error::Invariant("assignment boundary plan"))?
                        .actions[index]
                        .parameter_constructor = Some(constructor);
                    constructors.push(constructor);
                }
            }
        }
        Ok(())
    }
    pub(super) fn prepare_assignment_lookup(
        &mut self,
        mut scope: ScopeId,
        name: ValueKey,
        lexical: bool,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 || name != key()? {
            return Ok(());
        }
        loop {
            if self.graph.scope(scope).kind == ScopeKind::Class
                && let Some(class) = self.graph.scope(scope).owner
                && self.graph.entity(class).complete
            {
                self.prepare_copy_properties(class)?;
            }
            if !lexical
                || self
                    .graph
                    .own_key(scope, name, super::lookup::Filter::Ordinary)
                    .is_some()
            {
                break;
            }
            let Some(parent) = self.graph.scope(scope).parent else {
                break;
            };
            scope = parent;
        }
        Ok(())
    }
    fn assignment_kind(
        &self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<Option<RefKind>, Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        if entity.assignment_origin == Origin::Implicit
            || entity.template_head.is_some()
            || entity.function.is_some_and(|p| p.is_static)
            || entity.home != self.graph.entity(class).scope
        {
            return Ok(None);
        }
        let ty = entity.ty.ok_or(Error::Invariant("assignment signature"))?;
        if self.graph.types.function_parameter_count(ty)? != 1
            || matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Function { variadic: true, .. }
            )
        {
            return Ok(None);
        }
        let parameter = self.graph.types.function_parameter(ty, 0)?;
        let (element, kind) = match self.graph.types.get(parameter).kind {
            TypeKind::Reference { element, kind } => (element, kind),
            _ => (parameter, RefKind::Lvalue),
        };
        Ok(
            matches!(self.graph.types.get(element).kind, TypeKind::Class(c) if c == class)
                .then_some(kind),
        )
    }
    pub(super) fn classify_assignments(&mut self, class: EntityId) -> Result<(), Error> {
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("assignment class scope"))?;
        let key = key()?;
        let mut next = 0;
        while let Some(binding) = self.graph.next_function(home, key, &mut next) {
            if self.graph.entity(self.graph.binding(binding).entity).home != Some(home) {
                continue;
            }
            let kind = self.assignment_kind(binding, class)?;
            self.graph
                .entity_mut(self.graph.binding(binding).entity)
                .assignment_kind = kind;
            if kind.is_some() {
                self.prepare_default_assignment(binding)?;
            }
        }
        Ok(())
    }
    pub(super) fn prepare_default_assignment(&mut self, binding: BindingId) -> Result<(), Error> {
        let function = self.graph.binding(binding).entity;
        let entity = self.graph.entity(function);
        if entity.assignment.is_some()
            || !matches!(
                entity.special_definition,
                Some(SpecialDefinition::Defaulted { .. })
            )
        {
            return Ok(());
        }
        let home = entity
            .home
            .ok_or(Error::Invariant("defaulted assignment home"))?;
        let class = self
            .graph
            .scope(home)
            .owner
            .ok_or(Error::Invariant("defaulted assignment class"))?;
        let kind = self
            .assignment_kind(binding, class)?
            .ok_or(Error::Invariant("defaulted assignment kind"))?;
        let first = matches!(
            entity.special_definition,
            Some(SpecialDefinition::Defaulted { first: true, .. })
        );
        let infer = entity.assignment_exceptions;
        let signature = entity
            .ty
            .ok_or(Error::Invariant("defaulted assignment signature"))?;
        let parameter = self.graph.types.function_parameter(signature, 0)?;
        let TypeKind::Reference { element, .. } = self.graph.types.get(parameter).kind else {
            return Err(Error::Invariant("defaulted assignment parameter"));
        };
        let cv = self.graph.types.get(element).cv;
        self.prepare_copy_children(class)?;
        let mut properties = self.assignment_properties(class, kind, cv)?;
        properties.trivial &= first;
        self.graph.entity_mut(function).assignment_kind = Some(kind);
        if infer == ExceptionSource::Subobjects {
            self.graph.entity_mut(function).exceptions =
                super::exceptions::ExceptionState::Value(if properties.nothrow {
                    super::exceptions::ExceptionSet::None
                } else {
                    super::exceptions::ExceptionSet::All
                });
        } else {
            properties.nothrow = !self.function_may_throw(function)?;
        }
        if properties.deleted {
            self.graph.entity_mut(function).availability = FunctionAvailability::Deleted;
        }
        self.graph.entity_mut(function).assignment = Some(properties);
        if self.graph.entity(function).scope.is_none() {
            let scope = self
                .graph
                .add_scope(ScopeKind::Function, home, None, None)?;
            self.graph.scope_mut(scope).owner = Some(function);
            self.graph.entity_mut(function).scope = Some(scope);
            // A defaulted assignment has a synthesized body, with an unnamed
            // parameter binding even when its declaration supplied a name.
            let name = None;
            let other = self.graph.add_entity(
                Some(parameter),
                name,
                crate::lex::vocabulary::Keyword::Int,
            )?;
            self.graph.add_binding(
                scope,
                Some(scope),
                super::graph::Binding {
                    kind: super::graph::BindingKind::Parameter,
                    name,
                    entity: other,
                    ty: Some(parameter),
                    key: crate::lex::vocabulary::Keyword::Int,
                },
            )?;
        }
        Ok(())
    }
    fn const_assignment_source(&self, class: EntityId) -> Result<bool, Error> {
        let count = self.graph.entity(class).bases.len() + self.graph.entity(class).members.len();
        for index in 0..count {
            let Some((child, _, _)) = self.graph.copy_child(class, index)? else {
                continue;
            };
            let home = self
                .graph
                .entity(child)
                .scope
                .ok_or(Error::Invariant("assignment child scope"))?;
            let mut accepts = false;
            for binding in self.graph.functions_at(home, key()?) {
                let entity = self.graph.entity(self.graph.binding(binding).entity);
                if entity.assignment_kind != Some(RefKind::Lvalue) || entity.home != Some(home) {
                    continue;
                }
                let ty = entity
                    .ty
                    .ok_or(Error::Invariant("assignment child signature"))?;
                let parameter = self.graph.types.function_parameter(ty, 0)?;
                accepts |= match self.graph.types.get(parameter).kind {
                    TypeKind::Reference { element, .. } => {
                        self.graph.types.get(element).cv.0 & Cv::CONST.0 != 0
                    }
                    _ => true,
                };
            }
            if !accepts {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn declare_assignments(
        &mut self,
        class: EntityId,
        suppressed: bool,
        moves: bool,
    ) -> Result<(), Error> {
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("assignment class scope"))?;
        let mut copy = false;
        let mut moving = false;
        let mut next = 0;
        while let Some(binding) = self.graph.next_function(home, key()?, &mut next) {
            match self.assignment_kind(binding, class)? {
                Some(RefKind::Lvalue) => copy = true,
                Some(RefKind::Rvalue) => moving = true,
                None => (),
            }
            self.prepare_default_assignment(binding)?;
        }
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("assignment class type"))?;
        let result = self.graph.types.reference(ty, RefKind::Lvalue)?;
        for kind in [RefKind::Lvalue, RefKind::Rvalue] {
            if kind == RefKind::Lvalue && copy
                || kind == RefKind::Rvalue && (suppressed || copy || moving)
            {
                continue;
            }
            let cv = if kind == RefKind::Lvalue && self.const_assignment_source(class)? {
                Cv::CONST
            } else {
                Cv::default()
            };
            let element = self.graph.types.qualify(ty, cv)?;
            let parameter = self.graph.types.reference(element, kind)?;
            let signature = self.graph.types.intern(
                TypeKind::Function {
                    result,
                    parameters: vec![parameter],
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )?;
            let properties = if kind == RefKind::Lvalue && moves {
                Properties {
                    deleted: true,
                    trivial: false,
                    nothrow: false,
                    actions: Vec::new(),
                }
            } else {
                self.assignment_properties(class, kind, cv)?
            };
            if kind == RefKind::Rvalue && properties.deleted {
                continue;
            }
            self.graph
                .add_implicit_assignment(class, signature, kind, properties)?;
        }
        Ok(())
    }
    fn assignment_properties(
        &mut self,
        class: EntityId,
        kind: RefKind,
        source_cv: Cv,
    ) -> Result<Properties, Error> {
        let mut properties = Properties {
            deleted: false,
            trivial: !self.graph.lower_polymorphic(class)
                && !self
                    .graph
                    .entity(class)
                    .bases
                    .iter()
                    .any(|edge| edge.is_virtual),
            nothrow: true,
            actions: Vec::new(),
        };
        let entity = self.graph.entity(class);
        let scope = entity
            .scope
            .ok_or(Error::Invariant("assignment class scope"))?;
        let count = entity.bases.len() + entity.members.len();
        for index in 0..count {
            let base_count = self.graph.entity(class).bases.len();
            let member = index
                .checked_sub(base_count)
                .and_then(|index| self.graph.entity(class).members.get(index))
                .copied();
            let ty = if let Some(member) = member {
                self.graph.entity(member).ty
            } else {
                self.graph
                    .entity(self.graph.entity(class).bases[index].class)
                    .ty
            }
            .ok_or(Error::Invariant("assignment subobject type"))?;
            let mut action = Action {
                member,
                base: if member.is_none() {
                    Some((
                        class,
                        u32::try_from(index).map_err(|_| Error::Limit("assignment base limit"))?,
                    ))
                } else {
                    None
                },
                ty,
                function: None,
                parameter_copy: None,
                parameter_constructor: None,
            };
            properties.deleted |=
                matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                    || self.type_cv(ty).0 & Cv::CONST.0 != 0;
            if let Some((child, _, object_cv)) = self.graph.copy_child(class, index)? {
                let mutable =
                    member.is_some_and(|m| self.graph.entity(m).mutability == Mutability::Mutable);
                let cv = Cv(object_cv.0 | if mutable { 0 } else { source_cv.0 });
                let child_ty = self
                    .graph
                    .entity(child)
                    .ty
                    .ok_or(Error::Invariant("assignment child type"))?;
                let source_ty = self.graph.types.qualify(child_ty, cv)?;
                let source = ExprFacts::new(
                    source_ty,
                    if kind == RefKind::Lvalue {
                        ValueCategory::Lvalue
                    } else {
                        ValueCategory::Xvalue
                    },
                );
                let acted = if member.is_some() { child } else { class };
                let selected = self.subobject_assignment(child, source, object_cv)?;
                if let Some(binding) = selected {
                    let function = self.graph.binding(binding).entity;
                    action.function = Some(function);
                    let entity = self.graph.entity(function);
                    properties.deleted |= entity.availability == FunctionAvailability::Deleted;
                    properties.trivial &= entity.assignment.as_ref().is_some_and(|p| p.trivial);
                    properties.nothrow &= !self.function_may_throw(function)?;
                    let signature = self
                        .graph
                        .entity(function)
                        .ty
                        .ok_or(Error::Invariant("assignment parameter signature"))?;
                    let parameter = self.graph.types.function_parameter(signature, 0)?;
                    if !matches!(
                        self.graph.types.get(parameter).kind,
                        TypeKind::Reference { .. }
                    ) && self.graph.class_of(parameter) == Some(child)
                    {
                        action.parameter_copy = self.assignment_parameter_copy(
                            child,
                            source,
                            acted,
                            scope,
                            &mut properties,
                        )?;
                    }
                    properties.deleted |=
                        !self.subobject_assignment_accessible(child, function, acted, scope)?;
                    properties.deleted |= self.graph.entity(class).key
                        == crate::lex::vocabulary::Keyword::Union
                        && !self
                            .graph
                            .entity(function)
                            .assignment
                            .as_ref()
                            .is_some_and(|p| p.trivial);
                } else {
                    properties.deleted = true;
                }
            }
            properties.actions.push(action);
        }
        Ok(properties)
    }
    fn subobject_assignment_accessible(
        &self,
        class: EntityId,
        selected: EntityId,
        acted: EntityId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        // subobject_copy checks access only through a matching named
        // declaration. Hidden deductions do not inherit it from their primary.
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("assignment child scope"))?;
        for declared in self.graph.functions_at(home, key()?) {
            if self.graph.binding(declared).entity == selected
                && self
                    .check_special_member_access(declared, acted, scope, SourceOffset(0))
                    .is_err()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn assignment_parameter_copy(
        &mut self,
        child: EntityId,
        source: ExprFacts,
        acted: EntityId,
        scope: ScopeId,
        properties: &mut Properties,
    ) -> Result<Option<super::copy_initialization::CopySelection>, Error> {
        let selected = match self.select_subobject_copy(child, source, false) {
            Ok(selected) => Some(selected),
            Err(Error::Diagnostic { .. }) => {
                properties.deleted = true;
                None
            }
            Err(error) => return Err(error),
        };
        if let Some(selected) = selected {
            let summary = self.selected_copy_properties(child, selected)?;
            properties.deleted |= summary.deleted;
            properties.nothrow &= summary.nothrow;
            if let super::copy_initialization::CopyOrigin::Declared(binding) = selected.origin {
                let signature = self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .ty
                    .ok_or(Error::Invariant("assignment parameter copy signature"))?;
                // analyzer_copy.cpp::NothrowCopy assumes that evaluating
                // any omitted constructor defaults can throw.
                properties.nothrow &= self.graph.types.function_parameter_count(signature)? == 1;
                properties.deleted |= self
                    .check_special_member_access(binding, acted, scope, SourceOffset(0))
                    .is_err();
            }
        }
        self.prepare_class_parameter(child)?;
        Ok(selected)
    }
    fn subobject_assignment(
        &mut self,
        class: EntityId,
        source: ExprFacts,
        cv: Cv,
    ) -> Result<Option<BindingId>, Error> {
        let home = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("assignment child scope"))?;
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("assignment child type"))?;
        let ty = self.graph.types.qualify(ty, cv)?;
        let object = ObjectArgument {
            ty,
            category: ValueCategory::Lvalue,
            source: ObjectSource::Implicit,
        };
        let mut scratch = std::mem::take(&mut self.assignment_scratch);
        let result = self.resolve_subobject_assignment(home, object, source, &mut scratch);
        scratch.candidates.clear();
        scratch.seen.clear();
        self.assignment_scratch = scratch;
        result
    }
    fn resolve_subobject_assignment(
        &mut self,
        home: ScopeId,
        object: ObjectArgument,
        source: ExprFacts,
        scratch: &mut Scratch,
    ) -> Result<Option<BindingId>, Error> {
        // analyzer_calls.cpp::resolve tries ordinary declarations first. Only
        // its last viable ordinary candidate decides whether deduction is moot.
        self.assignment_candidates(home, object, source, false, scratch)?;
        if !scratch.candidates.last().is_some_and(|candidate| {
            let sequence = ArgumentSequence {
                conversion: candidate.object,
                application: Some(candidate.object),
                function: None,
                user_conversion: None,
            };
            self.perfect_sequence(Some(sequence), object.ty, object.category, false)
                && self.perfect_sequence(Some(candidate.source), source.ty, source.category, false)
        }) {
            self.assignment_candidates(home, object, source, true, scratch)?;
        }
        let candidates = &scratch.candidates;
        let mut best = 0;
        for index in 1..candidates.len() {
            if self.compare_assignment_candidates(candidates[index], candidates[best])?
                == Ordering::Greater
            {
                best = index;
            }
        }
        let Some(&selected) = candidates.get(best) else {
            return Ok(None);
        };
        for (index, &other) in candidates.iter().enumerate() {
            if index != best
                && self.compare_assignment_candidates(selected, other)? != Ordering::Greater
            {
                return Ok(None);
            }
        }
        // subobject_copy retains no callable for a deleted selection. This is
        // decided after ranking, so a worse overload cannot replace it.
        if self
            .graph
            .entity(self.graph.binding(selected.binding).entity)
            .availability
            == FunctionAvailability::Deleted
        {
            return Ok(None);
        }
        Ok(Some(selected.binding))
    }
    fn assignment_candidates(
        &mut self,
        home: ScopeId,
        object: ObjectArgument,
        source: ExprFacts,
        templated: bool,
        scratch: &mut Scratch,
    ) -> Result<(), Error> {
        let mut next = 0;
        while let Some(binding) = self.graph.next_function(home, key()?, &mut next) {
            let function = self.graph.binding(binding).entity;
            let entity = self.graph.entity(function);
            if entity.function.is_some_and(|p| p.is_static)
                || self.graph.ignored_assignment(function)
                || entity.template_head.is_some() != templated
                || !scratch.seen.insert(function)
            {
                continue;
            }
            let Some(binding) = self.deduce_function_call(binding, &[], false, Some(source))?
            else {
                continue;
            };
            let ty = self
                .graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("assignment signature"))?;
            if self.graph.types.function_parameter_count(ty)? != 1 {
                continue;
            }
            let parameter = self.graph.types.function_parameter(ty, 0)?;
            let Some(object) = self.object_sequence(object, binding, true)? else {
                continue;
            };
            let sequence =
                if let Some(conversion) = self.scalar_sequence(source, parameter, false)? {
                    Some(ArgumentSequence {
                        conversion,
                        application: Some(conversion),
                        function: None,
                        user_conversion: None,
                    })
                } else {
                    self.user_conversion_sequence(source, parameter, false)?
                };
            let Some(source) = sequence else {
                continue;
            };
            scratch.candidates.push(Candidate {
                binding,
                object,
                source,
            });
        }
        Ok(())
    }
    fn compare_assignment_candidates(
        &mut self,
        a: Candidate,
        b: Candidate,
    ) -> Result<Ordering, Error> {
        let object = self.compare_sequences(a.object, b.object)?;
        let source = self.compare_argument_sequences(a.source, b.source)?;
        Ok(match (object, source) {
            (Ordering::Equal, Ordering::Equal) => {
                self.compare_function_templates(a.binding, b.binding, TemplateOrdering::Call(1))?
            }
            (Ordering::Greater, Ordering::Less) | (Ordering::Less, Ordering::Greater) => {
                Ordering::Equal
            }
            (Ordering::Greater, _) | (_, Ordering::Greater) => Ordering::Greater,
            _ => Ordering::Less,
        })
    }
}
