// Translated from sema/analyzer_virtual.cpp's declaration and override analysis. See NOTICE.
//! Indexed virtual identities and declaration-owned override facts. No body or layout demand.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    function_names::ValueKey,
    graph::{BindingId, EntityId, MemberKind, ScopeId, ScopeKind},
    specifiers::{Friendship, Specifiers, Virtuality},
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Default, Debug)]
pub(super) struct VirtualFunction {
    pub is_virtual: bool,
    pub is_final: bool,
    pub pure: bool,
    pub overridden: Option<EntityId>,
}

#[derive(Default)]
pub(super) struct Virtuals {
    functions: HashMap<EntityId, VirtualFunction>,
    declarations: HashMap<(EntityId, ValueKey, TypeId), EntityId>,
    signatures: HashSet<(ValueKey, TypeId)>,
    destructors: HashMap<EntityId, EntityId>,
    pure_counts: HashMap<EntityId, u32>,
}

impl Virtuals {
    pub(super) fn function(&self, entity: EntityId) -> VirtualFunction {
        self.functions.get(&entity).copied().unwrap_or_default()
    }
    pub(super) fn destructor(&self, class: EntityId) -> Option<EntityId> {
        self.destructors.get(&class).copied()
    }
}

impl Analyzer<'_> {
    pub(super) fn has_virt_specifier(&self, info: &DeclaratorInfo) -> bool {
        info.qualifiers.is_some_and(|list| {
            self.tree
                .qualifiers
                .get(list)
                .iter()
                .any(|qualifier| matches!(qualifier.kind, ast::QualifierKind::Virt(_)))
        })
    }

    pub(super) fn check_virtual_alias(
        &self,
        info: &DeclaratorInfo,
        specs: Specifiers,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if specs.virtuality == Virtuality::Virtual || self.has_virt_specifier(info) {
            return Err(Error::at(
                location,
                "a virt-specifier or virtual in an alias-declaration",
            ));
        }
        Ok(())
    }

    pub(super) fn virtual_declaration(
        &mut self,
        binding: BindingId,
        info: &DeclaratorInfo,
        specs: Specifiers,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 {
            return Ok(());
        }
        let mut is_override = false;
        let mut is_final = false;
        if let Some(list) = info.qualifiers {
            for qualifier in self.tree.qualifiers.get(list) {
                if let ast::QualifierKind::Virt(name) = qualifier.kind {
                    let seen = if name == self.override_name {
                        &mut is_override
                    } else {
                        &mut is_final
                    };
                    if *seen {
                        return Err(Error::at(qualifier.location, "a repeated virt-specifier"));
                    }
                    *seen = true;
                }
            }
        }
        let function = self.graph.binding(binding).entity;
        let home = self
            .graph
            .entity(function)
            .home
            .ok_or(Error::Invariant("function without home"))?;
        let first = self.graph.function_origin(function)? == binding;
        if !first
            || self.graph.scope(home).kind != ScopeKind::Class
            || specs.friendship == Friendship::Friend
        {
            if specs.virtuality == Virtuality::Virtual {
                return Err(Error::at(
                    location,
                    "virtual outside the initial declaration of a class member function",
                ));
            }
            if is_override || is_final {
                return Err(Error::at(
                    location,
                    "a virt-specifier outside the declaration of a class member function",
                ));
            }
            return self.check_abstract_function(info.ty, location);
        }
        let class = self
            .graph
            .scope(home)
            .owner
            .ok_or(Error::Invariant("member without class"))?;
        let special = self
            .graph
            .entity(function)
            .member_function
            .map(|key| key.kind);
        let is_static = self
            .graph
            .entity(function)
            .function
            .is_some_and(|props| props.is_static);
        let mut facts = VirtualFunction {
            is_virtual: specs.virtuality == Virtuality::Virtual,
            is_final,
            ..VirtualFunction::default()
        };
        if facts.is_virtual {
            let message = if is_static {
                Some("a static member function declared virtual")
            } else if special == Some(MemberKind::Constructor) {
                Some("a constructor declared virtual")
            } else if self.graph.entity(class).key == Keyword::Union {
                Some("a union with a virtual function")
            } else {
                None
            };
            if let Some(message) = message {
                return Err(Error::at(location, message));
            }
        }
        let key = if special == Some(MemberKind::Conversion) {
            ValueKey::Special(super::function_names::SpecialName::Conversion)
        } else {
            self.declaration_key(info)?
        };
        let signature = if special == Some(MemberKind::Conversion) {
            self.graph.types.canonical_function(info.ty)?
        } else {
            self.graph.types.declaration_signature(info.ty)?
        };
        if special != Some(MemberKind::Constructor)
            && self.graph.entity(function).template_head.is_none()
        {
            let destructor = special == Some(MemberKind::Destructor);
            if destructor || self.graph.virtuals.signatures.contains(&(key, signature)) {
                let mut work: Vec<_> = self
                    .graph
                    .entity(class)
                    .bases
                    .iter()
                    .rev()
                    .map(|edge| edge.class)
                    .collect();
                let mut seen = HashSet::new();
                while let Some(base) = work.pop() {
                    if !seen.insert(base) {
                        continue;
                    }
                    let found = if destructor {
                        self.graph.virtuals.destructor(base)
                    } else {
                        self.graph
                            .virtuals
                            .declarations
                            .get(&(base, key, signature))
                            .copied()
                    };
                    if let Some(found) = found {
                        self.check_virtual_override(function, found, class, location)?;
                        self.check_override_nonthrowing(
                            function,
                            found,
                            destructor.then_some(base),
                            location,
                        )?;
                        facts.is_virtual = true;
                        facts.overridden = facts.overridden.or(Some(found));
                        self.note_exception_override(
                            function,
                            found,
                            destructor.then_some(base),
                            location,
                        );
                    } else {
                        work.extend(
                            self.graph
                                .entity(base)
                                .bases
                                .iter()
                                .rev()
                                .map(|edge| edge.class),
                        );
                    }
                }
            }
        }
        if is_override && facts.overridden.is_none() {
            return Err(Error::at(
                location,
                "a function marked override overrides no base class member function",
            ));
        }
        if is_final && !facts.is_virtual {
            return Err(Error::at(
                location,
                "a function marked final is not virtual",
            ));
        }
        if facts.is_virtual && specs.constexpr {
            return Err(Error::at(location, "a constexpr virtual function"));
        }
        if facts.is_virtual {
            self.graph.virtuals.functions.insert(function, facts);
            if special == Some(MemberKind::Destructor) {
                self.graph.virtuals.destructors.insert(class, function);
            } else {
                self.graph.virtuals.signatures.insert((key, signature));
                self.graph
                    .virtuals
                    .declarations
                    .insert((class, key, signature), function);
            }
            self.graph.entity_mut(class).aggregation =
                super::aggregate_initialization::Aggregation::Ineligible;
            self.graph.entity_mut(class).layout_category = super::layout::LayoutCategory::NonPod;
        }
        self.check_abstract_function(info.ty, location)
    }

    pub(super) fn check_abstract_object(
        &self,
        mut ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 {
            return Ok(());
        }
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        if let TypeKind::Class(class) = self.graph.types.get(ty).kind
            && self.graph.entity(class).complete
            && self
                .graph
                .virtuals
                .pure_counts
                .get(&class)
                .copied()
                .unwrap_or(0)
                != 0
        {
            return Err(Error::at(location, "an object of an abstract class type"));
        }
        Ok(())
    }

    fn check_abstract_function(&self, ty: TypeId, location: SourceOffset) -> Result<(), Error> {
        let TypeKind::Function {
            result, parameters, ..
        } = &self.graph.types.get(ty).kind
        else {
            return Err(Error::Invariant(
                "abstract function check without function type",
            ));
        };
        self.check_abstract_object(*result, location)?;
        for &parameter in parameters {
            self.check_abstract_object(parameter, location)?;
        }
        Ok(())
    }

    pub(super) fn check_virtual_object_declaration(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        init: ast::InitDeclarator,
    ) -> Result<(), Error> {
        let function = specs.role != super::specifiers::DeclarationRole::Typedef
            && matches!(
                self.graph.types.get(info.ty).kind,
                TypeKind::Function { .. }
            );
        let location = self.declarator_location(init.declarator)?;
        if specs.virtuality == Virtuality::Virtual && !function {
            return Err(Error::at(
                location,
                "virtual in a declaration that declares no function",
            ));
        }
        Ok(())
    }

    pub(super) fn complete_virtual_members(&mut self, class: EntityId) -> Result<(), Error> {
        // The PA13 table has one primary chain. Counts are inherited once;
        // replacing a pure entry changes the count, without copying slot arrays.
        let mut pure = self
            .graph
            .entity(class)
            .bases
            .first()
            .and_then(|edge| self.graph.virtuals.pure_counts.get(&edge.class))
            .copied()
            .unwrap_or(0);
        let scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("virtual class without scope"))?;
        for &binding in &self.graph.scope(scope).bindings {
            let function = self.graph.binding(binding).entity;
            if self.graph.entity(function).home != Some(scope) {
                continue;
            }
            let facts = self.graph.virtuals.function(function);
            if !facts.is_virtual
                || self
                    .graph
                    .entity(function)
                    .member_function
                    .is_some_and(|key| key.kind == MemberKind::Destructor)
            {
                continue;
            }
            if facts
                .overridden
                .is_some_and(|base| self.graph.virtuals.function(base).pure)
            {
                pure = pure
                    .checked_sub(1)
                    .ok_or(Error::Invariant("virtual pure count underflow"))?;
            }
            pure = pure
                .checked_add(u32::from(facts.pure))
                .ok_or(Error::Limit("virtual pure count limit"))?;
        }
        let destructor_pure = |class| {
            self.graph
                .entity(class)
                .declared_destructor
                .is_some_and(|binding| {
                    self.graph
                        .virtuals
                        .function(self.graph.binding(binding).entity)
                        .pure
                })
        };
        if self
            .graph
            .entity(class)
            .bases
            .first()
            .is_some_and(|edge| destructor_pure(edge.class))
        {
            pure = pure
                .checked_sub(1)
                .ok_or(Error::Invariant("destructor pure count underflow"))?;
        }
        pure = pure
            .checked_add(u32::from(destructor_pure(class)))
            .ok_or(Error::Limit("virtual pure count limit"))?;
        if pure != 0 {
            self.graph.virtuals.pure_counts.insert(class, pure);
        }
        Ok(())
    }

    fn check_virtual_override(
        &mut self,
        function: EntityId,
        found: EntityId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self
            .graph
            .entity(function)
            .function
            .is_some_and(|props| props.is_static)
        {
            return Err(Error::at(
                location,
                "a static member function overrides a virtual function",
            ));
        }
        if self.graph.virtuals.function(found).is_final {
            return Err(Error::at(location, "overriding a final virtual function"));
        }
        if !self
            .graph
            .entity(function)
            .member_function
            .is_some_and(|key| key.kind == MemberKind::Destructor)
        {
            if self.graph.entity(function).availability != self.graph.entity(found).availability {
                return Err(Error::at(
                    location,
                    "a deleted function and a function not deleted override one another",
                ));
            }
            self.check_override_result(function, found, class, location)?;
        }
        Ok(())
    }

    fn check_override_result(
        &mut self,
        function: EntityId,
        found: EntityId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let result = |entity| -> Result<TypeId, Error> {
            let ty = self.graph.lower_entity_type(entity)?;
            if let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind {
                Ok(result)
            } else {
                Err(Error::Invariant("virtual without function type"))
            }
        };
        let mine = result(function)?;
        let theirs = result(found)?;
        if mine == theirs {
            return Ok(());
        }
        let m = self.graph.types.get(mine);
        let t = self.graph.types.get(theirs);
        let elements = match (&m.kind, &t.kind) {
            (TypeKind::Pointer(a), TypeKind::Pointer(b)) => Some((*a, *b)),
            (
                TypeKind::Reference {
                    element: a,
                    kind: ar,
                },
                TypeKind::Reference {
                    element: b,
                    kind: br,
                },
            ) if ar == br => Some((*a, *b)),
            _ => None,
        }
        .filter(|_| m.cv == t.cv);
        let classes = elements
            .and_then(|(a, b)| Some((a, b, self.graph.class_of(a)?, self.graph.class_of(b)?)));
        let Some((a, b, derived, base)) = classes else {
            return Err(Error::at(
                location,
                "an overrider's return type is neither the overridden function's nor covariant with it",
            ));
        };
        if self.graph.types.get(a).cv.0 & !self.graph.types.get(b).cv.0 != 0 {
            return Err(Error::at(
                location,
                "a covariant return type more cv-qualified than the overridden function's",
            ));
        }
        if derived == base {
            return Ok(());
        }
        if !self.graph.entity(derived).complete && derived != class {
            return Err(Error::at(
                location,
                "a covariant return type of an incomplete class",
            ));
        }
        if !self.graph.is_base_of(base, derived) {
            return Err(Error::at(
                location,
                "a covariant return type whose class is not derived from the overridden function's",
            ));
        }
        if self.graph.base_count(derived, base) > 1 {
            return Err(Error::at(
                location,
                "a covariant return type whose class has the overridden function's as an ambiguous base",
            ));
        }
        let path = self.graph.retain_base_path(derived, base)?;
        self.check_base_path_access(
            path,
            self.graph
                .entity(class)
                .scope
                .ok_or(Error::Invariant("class without scope"))?,
            location,
        )
    }

    pub(super) fn pure_specifier(
        &mut self,
        function: EntityId,
        initializer: Option<ast::Initializer>,
        info: &DeclaratorInfo,
        specs: Specifiers,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 {
            return Ok(());
        }
        let Some(ast::Initializer {
            kind: ast::InitializerKind::Assign(expr),
            ..
        }) = initializer
        else {
            return Ok(());
        };
        let zero = matches!(self.tree.expressions.get(expr).kind, ast::ExprKind::Literal(lit) if self.literal_bytes.get(lit.spelling) == b"0");
        if !zero
            || !self.graph.virtuals.function(function).is_virtual
            || self.graph.scope(self.declaration_scope(scope)).kind != ScopeKind::Class
            || info.home.is_some()
            || specs.friendship == Friendship::Friend
        {
            return Err(Error::at(
                location,
                "a pure-specifier on a declaration that is not of a virtual function",
            ));
        }
        self.graph
            .virtuals
            .functions
            .get_mut(&function)
            .ok_or(Error::Invariant("pure nonvirtual function"))?
            .pure = true;
        Ok(())
    }

    pub(super) fn complete_virtual_destructor(
        &mut self,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let implicit = self.graph.entity(class).declared_destructor.is_none();
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index].class;
            if let Some(found) = self.graph.virtuals.destructor(base) {
                if implicit && self.graph.virtuals.function(found).is_final {
                    return Err(Error::at(location, "overriding a final virtual function"));
                }
                if implicit {
                    self.graph.virtuals.destructors.insert(class, found);
                    self.note_implicit_destructor_override(class, base, location);
                }
                self.check_completed_destructor_override(class, base, location)?;
            }
        }
        if self.graph.virtuals.destructor(class).is_some() {
            self.graph
                .entity_mut(class)
                .destruction
                .as_mut()
                .ok_or(Error::Invariant("virtual destructor without properties"))?
                .trivial = false;
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
    fn override_facts_share_entities_without_demanding_layouts_or_bodies() {
        let source = SourceBuffer::new(b"struct A{virtual A*f()const &;virtual void g()=0;virtual ~A()=0;};struct B:A{B*f()const & override;void g()override;};struct Unused{virtual void h();};".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"virtual-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let names: Vec<_> = [b"A".as_slice(), b"B", b"Unused", b"f", b"g"]
            .iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let class = |name| {
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Elaborated)
                        .unwrap(),
                )
                .entity
        };
        let a = class(names[0]);
        let b = class(names[1]);
        let unused = class(names[2]);
        let member = |class, name| {
            let home = graph.entity(class).scope.unwrap();
            graph
                .binding(
                    graph
                        .own(home, name, super::super::lookup::Filter::Ordinary)
                        .unwrap(),
                )
                .entity
        };
        let af = member(a, names[3]);
        let bf = member(b, names[3]);
        let bg = member(b, names[4]);
        assert_eq!(graph.virtuals.function(bf).overridden, Some(af));
        assert!(graph.virtuals.function(bf).is_virtual);
        assert!(!graph.virtuals.function(bg).pure);
        assert_eq!(graph.virtuals.pure_counts.get(&a), Some(&2));
        assert!(!graph.virtuals.pure_counts.contains_key(&b));
        assert_eq!(graph.virtuals.functions.len(), 6);
        assert_eq!(graph.virtuals.destructor(b), graph.virtuals.destructor(a));
        assert!(graph.entity(b).implicit_destructor.is_none());
        assert!(graph.root_demands.as_slice().is_empty());
        for id in [a, b, unused] {
            assert!(!matches!(
                graph.entity(id).layout,
                super::super::layout::LayoutState::Value { .. }
            ));
        }
        assert_eq!(expressions, tree.expressions.mark());
    }
}
