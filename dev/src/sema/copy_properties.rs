// Translated from sema/analyzer_copy.cpp. See NOTICE.
//! Demand-owned constructor properties, computed deepest first without native recursion.
//! The selected subobject functions are semantic facts; no syntax is synthesized.
use super::{
    Error,
    analyzer::Analyzer,
    copy_initialization::{CopyOrigin, CopySelection, ImplicitCopies},
    exceptions::{ExceptionSet, ExceptionState},
    expression_facts::{ExprFacts, ValueCategory},
    function_names::{SpecialName, ValueKey},
    graph::{BindingId, EntityId, FunctionAvailability, Graph, Mutability},
    special_members::SpecialDefinition,
    types::{Cv, RefKind, TypeKind},
};
use crate::{
    lex::{
        pp_lexer::Punctuator as P,
        vocabulary::{Keyword, SimpleToken},
    },
    support::source::SourceOffset,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Preparation {
    #[default]
    Pending,
    Active,
    Ready,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CopyPropertiesId(u32);
#[derive(Clone, Copy)]
pub(super) struct CopySummary {
    pub deleted: bool,
    pub trivial: bool,
    pub nothrow: bool,
}
pub(super) struct CopyProperties {
    pub deleted: bool,
    pub trivial: bool,
    pub nothrow: bool,
    pub plan: super::construction::ConstructorState,
    pub declaration: Option<EntityId>,
    pub callable: Option<BindingId>,
    pub subobjects: Vec<(Option<EntityId>, EntityId, Option<CopySelection>)>,
}
#[derive(Default)]
pub(super) struct CopyPropertiesStore {
    records: Vec<CopyProperties>,
}
impl CopyPropertiesStore {
    fn add(&mut self, facts: CopyProperties) -> Result<CopyPropertiesId, Error> {
        let id = CopyPropertiesId(
            u32::try_from(self.records.len()).map_err(|_| Error::Limit("copy property limit"))?,
        );
        self.records.push(facts);
        Ok(id)
    }
    pub(super) fn get_mut(&mut self, id: CopyPropertiesId) -> &mut CopyProperties {
        &mut self.records[id.0 as usize]
    }
    pub(super) fn get(&self, id: CopyPropertiesId) -> &CopyProperties {
        &self.records[id.0 as usize]
    }
}
impl Graph {
    // References do not make an object dependency. Array cv lives on its element.
    pub(super) fn copy_child(
        &self,
        class: EntityId,
        index: usize,
    ) -> Result<Option<(EntityId, Option<EntityId>, Cv)>, Error> {
        let entity = self.entity(class);
        if let Some(base) = entity.class().bases.get(index) {
            return Ok(Some((base.class, None, Cv::default())));
        }
        let member = entity.class().members[index - entity.class().bases.len()];
        let mut ty = self
            .entity(member)
            .ty
            .ok_or(Error::Invariant("copy member without type"))?;
        while let TypeKind::Array { element, .. } = self.types.get(ty).kind {
            ty = element;
        }
        Ok(if let TypeKind::Class(child) = self.types.get(ty).kind {
            Some((child, Some(member), self.types.get(ty).cv))
        } else {
            None
        })
    }
    pub(super) fn copy_property_id(
        &self,
        class: EntityId,
        selected: CopySelection,
    ) -> Result<Option<CopyPropertiesId>, Error> {
        Ok(match selected.origin {
            CopyOrigin::Declared(binding) => {
                self.entity(self.binding(binding).entity)
                    .special()
                    .copy_properties
            }
            CopyOrigin::Implicit { kind, .. } => {
                let copies = self
                    .entity(class)
                    .special()
                    .implicit_copies
                    .ok_or(Error::Invariant("implicit copy properties absent"))?;
                Some(if kind == RefKind::Lvalue {
                    copies
                        .copy_properties
                        .ok_or(Error::Invariant("implicit copy properties absent"))?
                } else {
                    copies
                        .move_properties
                        .ok_or(Error::Invariant("implicit move properties absent"))?
                })
            }
        })
    }
}
impl Analyzer<'_> {
    pub(super) fn prepare_copy_children(&mut self, class: EntityId) -> Result<(), Error> {
        let count = self.graph.entity(class).class().bases.len()
            + self.graph.entity(class).class().members.len();
        for index in 0..count {
            if let Some((child, _, _)) = self.graph.copy_child(class, index)? {
                self.prepare_copy_properties(child)?;
            }
        }
        Ok(())
    }
    pub(super) fn prepare_copy_properties(&mut self, class: EntityId) -> Result<(), Error> {
        if self.graph.entity(class).special().copy_preparation == Preparation::Ready {
            return Ok(());
        }
        if !self.graph.entity(class).complete {
            return Err(Error::Invariant("copy demand for incomplete class"));
        }
        if self.graph.entity(class).special().copy_preparation == Preparation::Active {
            return Err(Error::Invariant("cyclic copy property demand"));
        }
        let policy = self.graph.analysis.arguments.copy_members();
        self.with_argument_policy(policy, |analyzer| {
            analyzer
                .graph
                .entity_mut(class)
                .special_mut()
                .copy_preparation = Preparation::Active;
            let mut work = vec![(class, 0usize)];
            while let Some((current, index)) = work.last_mut() {
                let count = analyzer.graph.entity(*current).class().bases.len()
                    + analyzer.graph.entity(*current).class().members.len();
                if *index < count {
                    let child = analyzer.graph.copy_child(*current, *index)?;
                    *index += 1;
                    let Some((child, _, _)) = child else {
                        continue;
                    };
                    match analyzer.graph.entity(child).special().copy_preparation {
                        Preparation::Pending => {
                            analyzer
                                .graph
                                .entity_mut(child)
                                .special_mut()
                                .copy_preparation = Preparation::Active;
                            work.push((child, 0));
                        }
                        Preparation::Active => {
                            return Err(Error::Invariant("cyclic subobject copy demand"));
                        }
                        Preparation::Ready => (),
                    }
                    continue;
                }
                let current = *current;
                work.pop();
                analyzer.complete_copy_properties(current)?;
                analyzer
                    .graph
                    .entity_mut(current)
                    .special_mut()
                    .copy_preparation = Preparation::Ready;
            }
            Ok(())
        })
    }
    fn accepts_const_copy(&self, class: EntityId) -> Result<bool, Error> {
        if let Some((parameter, _)) = self
            .graph
            .entity(class)
            .special()
            .implicit_copies
            .and_then(|c| c.copy)
        {
            let TypeKind::Reference { element, .. } = self.graph.types.get(parameter).kind else {
                return Err(Error::Invariant("implicit copy parameter"));
            };
            return Ok(self.graph.types.get(element).cv.0 & Cv::CONST.0 != 0);
        }
        for &binding in &self.graph.entity(class).class().declared_constructors {
            if self.declared_copy_kind(binding, class)? == Some(RefKind::Lvalue) {
                let ty = self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .ty
                    .ok_or(Error::Invariant("copy signature"))?;
                let parameter = self.graph.types.function_parameter(ty, 0)?;
                if let TypeKind::Reference { element, .. } = self.graph.types.get(parameter).kind
                    && self.graph.types.get(element).cv.0 & Cv::CONST.0 != 0
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    fn const_copy_source(&self, class: EntityId) -> Result<bool, Error> {
        let count = self.graph.entity(class).class().bases.len()
            + self.graph.entity(class).class().members.len();
        for index in 0..count {
            if let Some((child, _, _)) = self.graph.copy_child(class, index)?
                && !self.accepts_const_copy(child)?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn complete_copy_properties(&mut self, class: EntityId) -> Result<(), Error> {
        let mut has_copy = false;
        let mut has_move = false;
        for index in 0..self.graph.entity(class).class().declared_constructors.len() {
            let binding = self.graph.entity(class).class().declared_constructors[index];
            match self.declared_copy_kind(binding, class)? {
                Some(RefKind::Lvalue) => has_copy = true,
                Some(RefKind::Rvalue) => has_move = true,
                None => (),
            }
            self.prepare_declared_copy(binding, class)?;
        }
        let home = self
            .graph
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("copy class without scope"))?;
        let assignment = ValueKey::Special(SpecialName::Operator {
            token: SimpleToken::punctuator(P::Assign)
                .ok_or(Error::Invariant("assignment token"))?,
            array: false,
        });
        let mut copy_assignment = false;
        let mut move_assignment = false;
        for binding in self.graph.functions_at(home, assignment) {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.home != Some(home)
                || entity.context().template_head.is_some()
                || entity.callable().function.is_some_and(|p| p.is_static)
            {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("assignment without type"))?;
            if self.graph.types.function_parameter_count(ty)? != 1
                || matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Function { variadic: true, .. }
                )
            {
                continue;
            }
            let parameter = self.graph.types.function_parameter(ty, 0)?;
            let (element, moving) = match self.graph.types.get(parameter).kind {
                TypeKind::Reference { element, kind } => (element, kind == RefKind::Rvalue),
                _ => (parameter, false),
            };
            if matches!(self.graph.types.get(element).kind, TypeKind::Class(owner) if owner == class)
            {
                move_assignment |= moving;
                copy_assignment |= !moving;
            }
        }
        let destructor = self
            .graph
            .entity(class)
            .class()
            .declared_destructor
            .is_some();
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("copy class without type"))?;
        let mut copy = None;
        let mut copy_properties = None;
        if !has_copy {
            let cv = if self.const_copy_source(class)? {
                Cv::CONST
            } else {
                Cv::default()
            };
            let element = self.graph.types.qualify(ty, cv)?;
            let parameter = self.graph.types.reference(element, RefKind::Lvalue)?;
            // A user-declared move deletes the implicit copy before its
            // definition's subobject selections are demanded (12.8p7).
            let facts = if has_move || move_assignment {
                CopyProperties {
                    deleted: true,
                    trivial: false,
                    nothrow: false,
                    plan: super::construction::ConstructorState::NotStarted,
                    declaration: None,
                    callable: None,
                    subobjects: Vec::new(),
                }
            } else {
                self.defaulted_copy_properties(class, RefKind::Lvalue, cv)?
            };
            copy = Some((parameter, facts.deleted));
            let properties = self.graph.copy_properties.add(facts)?;
            self.declare_implicit_copy(class, RefKind::Lvalue, parameter, properties)?;
            copy_properties = Some(properties);
        }
        // analyzer_copy.cpp treats a closure's deleted copy assignment as
        // declared: its rvalue sources use the copy constructor, with no
        // implicit move member to select or demand.
        let closure = self.graph.closure_call(class).is_some();
        let moving =
            !(has_copy || has_move || copy_assignment || move_assignment || destructor || closure);
        let mut move_properties = None;
        let mut move_parameter = None;
        if moving {
            let facts = self.defaulted_copy_properties(class, RefKind::Rvalue, Cv::default())?;
            let deleted = facts.deleted;
            let properties = self.graph.copy_properties.add(facts)?;
            let parameter = self.graph.types.reference(ty, RefKind::Rvalue)?;
            self.declare_implicit_copy(class, RefKind::Rvalue, parameter, properties)?;
            if !deleted {
                move_properties = Some(properties);
                move_parameter = Some(parameter);
            }
        }
        self.graph.entity_mut(class).special_mut().implicit_copies = Some(ImplicitCopies {
            copy,
            moving: move_parameter,
            copy_properties,
            move_properties,
            passing: None,
        });
        self.declare_assignments(
            class,
            has_copy || has_move || destructor || closure,
            has_move || move_assignment || closure,
        )?;
        Ok(())
    }
    pub(super) fn prepare_declared_copy(
        &mut self,
        binding: BindingId,
        class: EntityId,
    ) -> Result<(), Error> {
        let function = self.graph.binding(binding).entity;
        let entity = self.graph.entity(function);
        let Some(kind) = entity.callable().copy_kind else {
            return Ok(());
        };
        if entity.special().copy_properties.is_some()
            || !matches!(
                entity.callable().special_definition,
                Some(SpecialDefinition::Defaulted { .. })
            )
        {
            return Ok(());
        }
        let signature = entity
            .ty
            .ok_or(Error::Invariant("defaulted copy signature"))?;
        let parameter = self.graph.types.function_parameter(signature, 0)?;
        let TypeKind::Reference { element, .. } = self.graph.types.get(parameter).kind else {
            return Err(Error::Invariant("defaulted copy parameter"));
        };
        let cv = self.graph.types.get(element).cv;
        let first = matches!(
            entity.callable().special_definition,
            Some(SpecialDefinition::Defaulted { first: true, .. })
        );
        let inferred = entity.callable().constructor_exceptions
            == super::construction::ConstructorExceptionSource::SubobjectPlan;
        let explicitly_deleted = entity.availability == FunctionAvailability::Deleted;
        let mut facts = self.defaulted_copy_properties(class, kind, cv)?;
        facts.declaration = Some(function);
        facts.deleted |= explicitly_deleted;
        facts.trivial &= first;
        if inferred {
            let mut entity = self.graph.entity_mut(function);
            entity.callable_mut().exceptions = ExceptionState::Value(if facts.nothrow {
                ExceptionSet::None
            } else {
                ExceptionSet::All
            });
            entity.callable_mut().constructor_exceptions =
                super::construction::ConstructorExceptionSource::Declared;
        } else {
            facts.nothrow = !self.function_may_throw(function)?;
        }
        if facts.deleted {
            self.graph.entity_mut(function).availability = FunctionAvailability::Deleted;
        }
        let id = self.graph.copy_properties.add(facts)?;
        self.graph
            .entity_mut(function)
            .special_mut()
            .copy_properties = Some(id);
        Ok(())
    }
    pub(super) fn selected_copy_properties(
        &mut self,
        class: EntityId,
        selected: CopySelection,
    ) -> Result<CopySummary, Error> {
        if let Some(id) = self.graph.copy_property_id(class, selected)? {
            let facts = self.graph.copy_properties.get(id);
            return Ok(CopySummary {
                deleted: facts.deleted,
                trivial: facts.trivial,
                nothrow: facts.nothrow,
            });
        }
        let CopyOrigin::Declared(binding) = selected.origin else {
            return Err(Error::Invariant("copy properties without origin"));
        };
        let function = self.graph.binding(binding).entity;
        Ok(CopySummary {
            deleted: self.graph.entity(function).availability == FunctionAvailability::Deleted,
            trivial: false,
            nothrow: !self.function_may_throw(function)?,
        })
    }
    fn defaulted_copy_properties(
        &mut self,
        class: EntityId,
        kind: RefKind,
        source_cv: Cv,
    ) -> Result<CopyProperties, Error> {
        let mut facts = CopyProperties {
            deleted: false,
            trivial: !self.graph.entity(class).class().polymorphism.is_dynamic()
                && !self
                    .graph
                    .entity(class)
                    .class()
                    .bases
                    .iter()
                    .any(|edge| edge.is_virtual),
            nothrow: true,
            plan: super::construction::ConstructorState::NotStarted,
            declaration: None,
            callable: None,
            subobjects: Vec::new(),
        };
        let scope = self
            .graph
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("copy class scope"))?;
        for target in self.graph.copy_targets(class)? {
            let Some((child, member, cv)) = self.graph.copy_target_child(class, target)? else {
                if let super::copy_construction::CopyTarget::Member(member) = target {
                    let ty = self
                        .graph
                        .entity(member)
                        .ty
                        .ok_or(Error::Invariant("copy member type"))?;
                    facts.deleted |= kind == RefKind::Lvalue
                        && matches!(
                            self.graph.types.get(ty).kind,
                            TypeKind::Reference {
                                kind: RefKind::Rvalue,
                                ..
                            }
                        );
                }
                continue;
            };
            let mutable =
                member.is_some_and(|m| self.graph.entity(m).mutability == Mutability::Mutable);
            let ty = self
                .graph
                .entity(child)
                .ty
                .ok_or(Error::Invariant("subobject copy type"))?;
            let ty = self
                .graph
                .types
                .qualify(ty, Cv(cv.0 | if mutable { 0 } else { source_cv.0 }))?;
            let source = ExprFacts::new(
                ty,
                if kind == RefKind::Lvalue {
                    ValueCategory::Lvalue
                } else {
                    ValueCategory::Xvalue
                },
            );
            let selected = match self.select_subobject_copy(child, source, true) {
                Ok(selected) => Some(selected),
                Err(Error::Diagnostic { .. }) => {
                    facts.deleted = true;
                    None
                }
                Err(error) => return Err(error),
            };
            if let Some(selected) = selected {
                if let CopyOrigin::Declared(binding) = selected.origin {
                    facts.deleted |= self
                        .check_special_member_access(
                            binding,
                            if member.is_some() { child } else { class },
                            scope,
                            SourceOffset(0),
                        )
                        .is_err();
                }
                let selected_facts = self.selected_copy_properties(child, selected)?;
                facts.deleted |= selected_facts.deleted;
                facts.trivial &= selected_facts.trivial;
                facts.nothrow &= selected_facts.nothrow;
                if let CopyOrigin::Declared(binding) = selected.origin {
                    let signature = self
                        .graph
                        .entity(self.graph.binding(binding).entity)
                        .ty
                        .ok_or(Error::Invariant("subobject copy signature"))?;
                    facts.nothrow &= self.graph.types.function_parameter_count(signature)? == 1;
                }
                facts.deleted |=
                    self.graph.entity(class).key == Keyword::Union && !selected_facts.trivial;
            }
            let anonymous = member.is_some_and(|m| {
                self.graph
                    .entity(child)
                    .class()
                    .anonymous_storage
                    .is_some_and(|b| self.graph.binding(b).entity == m)
            });
            if !anonymous {
                facts.deleted |= self
                    .graph
                    .entity(child)
                    .special()
                    .destruction
                    .is_some_and(|f| f.deleted);
                if let Some(binding) = self.graph.destructor_binding(child) {
                    facts.deleted |= self
                        .check_special_member_access(
                            binding,
                            if member.is_some() { child } else { class },
                            scope,
                            SourceOffset(0),
                        )
                        .is_err();
                }
            }
            facts.subobjects.push((member, child, selected));
        }
        Ok(facts)
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
    use std::fmt::Write as _;

    fn with_analyzer(source: &[u8], check: impl FnOnce(&mut Analyzer<'_>)) {
        let source = SourceBuffer::new(source.to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"recursive-copy-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        check(&mut analyzer);
        assert_eq!(tree.expressions.mark(), expressions);
    }
    fn class(analyzer: &Analyzer<'_>, index: usize) -> EntityId {
        // Written classes retain source binding order; the builtin alias is hidden.
        analyzer
            .graph
            .binding(analyzer.graph.scope(analyzer.graph.global).bindings[index])
            .entity
    }
    fn copy<'a>(analyzer: &'a mut Analyzer<'_>, class: EntityId) -> &'a CopyProperties {
        analyzer.prepare_copy_properties(class).unwrap();
        let id = analyzer
            .graph
            .entity(class)
            .special()
            .implicit_copies
            .unwrap()
            .copy_properties
            .unwrap();
        analyzer.graph.copy_properties.get(id)
    }
    #[test]
    fn closures_copy_rvalues_without_implicit_move_members() {
        with_analyzer(
            br"struct Item{Item();Item(const Item&)noexcept;Item(Item&&)=delete;};
            int main(){int n=1;Item item;auto empty=[]{};auto ref=[&]{return n;};
            auto value=[item]{};auto a=static_cast<decltype(empty)&&>(empty);
            auto b=static_cast<decltype(ref)&&>(ref);auto c=static_cast<decltype(value)&&>(value);}",
            |a| {
                let classes: Vec<_> = a.graph.closures.records.keys().copied().collect();
                assert_eq!(classes.len(), 3);
                for class in classes {
                    let facts = copy(a, class);
                    assert!(!facts.deleted && facts.nothrow);
                    let copies = a.graph.entity(class).special().implicit_copies.unwrap();
                    assert!(copies.copy.is_some());
                    assert!(copies.moving.is_none());
                    assert!(copies.move_properties.is_none());
                }
            },
        );
    }

    #[test]
    fn recursive_facts_keep_selected_edges_and_const_deletion_access_and_throw_properties() {
        with_analyzer(
            b"struct L{L(const L&)noexcept;};\
            struct A{L a[2];};\
            struct X{X(X&);};\
            struct N{X a;};\
            struct V{volatile L a;};\
            struct R{int&&r;};\
            struct B{protected:B(const B&)noexcept;};\
            struct D:B{};\
            struct H{B b;};\
            union U{L l;};\
            struct W{L l;W(const W&)=default;};\
            struct C{C(C&)noexcept;C(const C&);};\
            struct M{mutable C c;};\
            struct Z{L l;Z(const Z&)noexcept(false)=default;};\
            struct Q{L l;Q(Q&&);};\
            struct P{private:P(const P&);};\
            struct T{P p;};",
            |a| {
                let ids: Vec<_> = (0..17).map(|i| class(a, i)).collect();
                let facts = copy(a, ids[1]);
                assert!(!facts.deleted && !facts.trivial && facts.nothrow);
                assert_eq!(facts.subobjects.len(), 1);
                let (member, child, selected) = facts.subobjects[0];
                assert!(member.is_some());
                assert_eq!(child, ids[0]);
                assert!(matches!(selected.unwrap().origin, CopyOrigin::Declared(_)));
                assert!(!a.const_copy_source(ids[3]).unwrap());
                assert!(!copy(a, ids[3]).deleted);
                assert!(
                    a.graph
                        .entity(ids[3])
                        .special()
                        .implicit_copies
                        .unwrap()
                        .moving
                        .is_none()
                );
                assert!(copy(a, ids[4]).deleted);
                assert!(copy(a, ids[5]).deleted);
                assert!(
                    a.graph
                        .entity(ids[5])
                        .special()
                        .implicit_copies
                        .unwrap()
                        .moving
                        .is_some()
                );
                assert!(!copy(a, ids[7]).deleted);
                assert!(copy(a, ids[8]).deleted);
                assert!(copy(a, ids[9]).deleted);
                let binding = a.graph.entity(ids[10]).class().declared_constructors[0];
                let function = a.graph.entity(a.graph.binding(binding).entity);
                let id = function.special().copy_properties.unwrap();
                assert!(!a.graph.copy_properties.get(id).trivial);
                assert!(matches!(
                    function.callable().exceptions,
                    ExceptionState::Value(ExceptionSet::None)
                ));
                // Its written defaulted copy demanded children but not its own implicit members.
                assert!(a.graph.entity(ids[10]).special().implicit_copies.is_none());
                // Const-source qualification is removed only for mutable members;
                // its nonconst overload is nonthrowing, its const overload can throw.
                assert!(copy(a, ids[12]).nothrow);
                let binding = a.graph.entity(ids[13]).class().declared_constructors[0];
                let id = a
                    .graph
                    .entity(a.graph.binding(binding).entity)
                    .special()
                    .copy_properties
                    .unwrap();
                assert!(!a.graph.copy_properties.get(id).nothrow);
                let facts = copy(a, ids[14]);
                assert!(facts.deleted);
                assert!(facts.subobjects.is_empty());
                let facts = copy(a, ids[16]);
                assert!(facts.deleted);
                assert!(facts.subobjects[0].2.is_some());
            },
        );
    }
    #[test]
    fn deep_copy_dependencies_are_memoized_without_layout_or_body_demand() {
        let mut source = String::from("struct L0{int n;};");
        for i in 1..512 {
            write!(source, "struct L{i}{{L{} x;}};", i - 1).unwrap();
        }
        source.push_str("struct Unused{int n;};");
        with_analyzer(source.as_bytes(), |a| {
            let root = class(a, 511);
            assert!(copy(a, root).trivial);
            let count = a.graph.copy_properties.records.len();
            assert_eq!(count, 1024);
            a.prepare_copy_properties(root).unwrap();
            assert_eq!(a.graph.copy_properties.records.len(), count);
            for i in 0..512 {
                let id = class(a, i);
                assert!(matches!(
                    a.graph.entity(id).class().layout,
                    super::super::layout::LayoutState::NotStarted
                ));
                assert!(matches!(
                    a.graph.entity(id).special().default_construction,
                    super::super::construction::ConstructorState::NotStarted
                ));
            }
            let unused = class(a, 512);
            assert!(a.graph.entity(unused).special().copy_preparation == Preparation::Pending);
            assert!(a.graph.entity(unused).special().implicit_copies.is_none());
            assert!(super::super::function_demands::trailing(&a.graph).is_empty());
        });
    }
}
