// Translated from lower/global_lowering.cpp::static_lvalue/static_value. See NOTICE.
//! Static relocations have their own query: a frozen reference cast may bind
//! storage even though it is forbidden in a core constant expression.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, nonconstant},
    graph::{EntityId, ScopeId},
    inheritance::BasePathId,
    static_initialization::Value,
    types::TypeKind,
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    syntax::ast::{self, ExprKind},
};

use std::collections::HashMap;

#[derive(Clone, Copy)]
struct CompleteObject {
    class: EntityId,
    stride: u64,
}
#[derive(Clone, Copy)]
struct StaticAddress {
    address: Address,
    complete: Option<CompleteObject>,
}
impl StaticAddress {
    fn evaluated(address: Address) -> Self {
        Self {
            address,
            complete: None,
        }
    }
}
/// Static binding provenance is distinct from a relocation's storage root.
/// An evaluated call/conditional can name storage without proving its layout.
#[derive(Default)]
pub(super) struct Queries {
    routes: super::base_projections::BaseRoutes,
    references: HashMap<EntityId, StaticAddress>,
}

impl Analyzer<'_> {
    fn static_place(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<StaticAddress, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        let facts = self.graph.expression_facts.get(id)?;
        let mut place = match node.kind {
            ExprKind::Cast { operand, .. } if facts.reference_cast.is_some() => {
                let place = self.static_place(operand, scope)?;
                self.project_static_cast(
                    place,
                    facts.base_cast,
                    self.is_member_expression(operand),
                    node.location,
                )?
            }
            ExprKind::Id(name) => {
                let entity = self
                    .graph
                    .binding(
                        self.graph
                            .resolved_name(name)
                            .ok_or(Error::Invariant("static reference binding"))?,
                    )
                    .entity;
                self.static_named_place(entity, id, scope)?
            }
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::Multiply) => {
                self.static_pointer(operand, scope)?
            }
            ExprKind::Subscript { base, index } => {
                let (base, index) = if facts.commuted {
                    (index, base)
                } else {
                    (base, index)
                };
                let mut place = self.static_pointer(base, scope)?;
                let count = self.evaluate_constant(index, scope, false)?;
                if count.indeterminate() {
                    return Err(super::constant_addresses::nonconstant(
                        node.location,
                        "not a constant scalar value",
                    ));
                }
                let (size, _) = self.type_size(facts.ty, node.location)?;
                place.address.offset = place
                    .address
                    .offset
                    .wrapping_add(super::integral::signed(count.bits.wrapping_mul(size)));
                place
            }
            ExprKind::Member { base, op, .. } => {
                let member = facts
                    .member
                    .ok_or(Error::Invariant("static member designation"))?;
                let entity = self.graph.binding(member).entity;
                if self.graph.entity(entity).data_member
                    != Some(super::graph::DataMemberKind::Instance)
                    || matches!(
                        self.graph
                            .types
                            .get(self.graph.lower_entity_type(entity)?)
                            .kind,
                        TypeKind::Reference { .. }
                    )
                {
                    // Static members and evaluated reference members use the
                    // core evaluator's stored value, not a field relocation.
                    return self.constant_place(id, scope).map(StaticAddress::evaluated);
                }
                let place = if op.token.as_punctuator() == Some(P::Arrow) {
                    self.static_pointer(base, scope)?
                } else {
                    self.static_place(base, scope)?
                };
                let member_object =
                    op.token.as_punctuator() != Some(P::Arrow) && self.is_member_expression(base);
                let place = self.static_project_path(
                    place,
                    facts.member_naming_path,
                    member_object,
                    node.location,
                )?;
                let place = self.static_project_path(
                    place,
                    facts.member_path,
                    member_object && facts.member_naming_path.is_none(),
                    node.location,
                )?;
                let address = self.constant_member_designation(
                    place.address,
                    self.graph.binding(member).entity,
                    facts.ty,
                    node.location,
                )?;
                return Ok(StaticAddress { address, ..place });
            }
            _ => StaticAddress::evaluated(self.constant_place(id, scope)?),
        };
        place.address.designated = facts.ty;
        Ok(place)
    }

    fn static_named_place(
        &mut self,
        entity: EntityId,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<StaticAddress, Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        if let Some(&place) = self.static_addresses.references.get(&entity) {
            return Ok(place);
        }
        if let Some(value) = self.graph.entity(entity).static_initializer
            && matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            && self.graph.entity(entity).reference_temporary.is_none()
        {
            return Self::static_image_place(
                value,
                self.graph.expression_facts.get(id)?.ty,
                self.tree.expressions.get(id).location,
            );
        }
        let address = self.constant_place(id, scope)?;
        let mut place = StaticAddress::evaluated(address);
        if !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            && self.graph.entity(entity).home.is_some_and(|home| {
                self.graph.scope(home).kind == super::graph::ScopeKind::Namespace
            })
        {
            let mut element = ty;
            while let TypeKind::Array { element: child, .. } = self.graph.types.get(element).kind {
                element = child;
            }
            if let Some(class) = self.graph.class_of(element) {
                let stride = if element == ty {
                    0
                } else {
                    self.type_size(element, self.tree.expressions.get(id).location)?
                        .0
                };
                place.complete = Some(CompleteObject { class, stride });
            }
        }
        Ok(place)
    }
    pub(super) fn static_address_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.static_pointer(id, scope).map(|place| place.address)
    }
    fn static_pointer(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<StaticAddress, Error> {
        self.check_stack()?;
        let place = self.static_pointer_value(id, scope)?;
        let facts = self.graph.expression_facts.get(id)?;
        if facts.conversion.is_some_and(|conversion| {
            conversion.second == super::expression_facts::ValueConversion::DerivedToBase
        }) {
            return self.static_project_path(
                place,
                facts.base_path,
                false,
                self.tree.expressions.get(id).location,
            );
        }
        Ok(place)
    }
    fn static_pointer_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<StaticAddress, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        let facts = self.graph.expression_facts.get(id)?;
        match node.kind {
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::BitAnd) => {
                self.static_place(operand, scope)
            }
            ExprKind::Cast { operand, .. } => {
                // The cast owns this base adjustment. Its operand can retain
                // the same conversion fact, which must not be applied twice.
                let place = if facts.base_cast.is_some() {
                    self.static_pointer_value(operand, scope)?
                } else {
                    self.static_pointer(operand, scope)?
                };
                let mut place =
                    self.project_static_cast(place, facts.base_cast, false, node.location)?;
                if let TypeKind::Pointer(element) = self.graph.types.get(facts.ty).kind {
                    place.address.designated = element;
                }
                Ok(place)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if matches!(op.token.as_punctuator(), Some(P::Add | P::Subtract)) => {
                let left_pointer = matches!(
                    self.graph
                        .types
                        .get(self.graph.expression_facts.get(left)?.ty)
                        .kind,
                    TypeKind::Pointer(_) | TypeKind::Array { .. }
                );
                let (pointer, index) = if left_pointer {
                    (left, right)
                } else {
                    (right, left)
                };
                let mut place = self.static_pointer(pointer, scope)?;
                let count = self.evaluate_constant(index, scope, false)?;
                if count.indeterminate() {
                    return Err(super::constant_addresses::nonconstant(
                        node.location,
                        "not a constant scalar value",
                    ));
                }
                let (size, _) = self.type_size(place.address.designated, node.location)?;
                let offset = super::integral::signed(count.bits.wrapping_mul(size));
                place.address.offset = if op.token.as_punctuator() == Some(P::Subtract) {
                    place.address.offset.wrapping_sub(offset)
                } else {
                    place.address.offset.wrapping_add(offset)
                };
                Ok(place)
            }
            _ if matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Array { .. } | TypeKind::Function { .. }
            ) =>
            {
                let mut place = self.static_place(id, scope)?;
                if let TypeKind::Array { element, .. } = self.graph.types.get(facts.ty).kind {
                    place.address.designated = element;
                }
                Ok(place)
            }
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::Add) => {
                self.static_pointer(operand, scope)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let selected = if self.constant_truth(condition, scope, false)? {
                    then_expr
                } else {
                    else_expr
                };
                self.static_pointer(selected, scope)
            }
            _ => self
                .evaluate_address(id, scope)
                .map(StaticAddress::evaluated),
        }
    }
    fn project_static_cast(
        &mut self,
        mut place: StaticAddress,
        cast: Option<super::expression_facts::BaseCast>,
        member: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<StaticAddress, Error> {
        if place.address.root != Root::Null
            && let Some(cast) = cast
        {
            let &(derived, _) = self
                .graph
                .base_paths
                .get(cast.path)
                .first()
                .ok_or(Error::Invariant("empty static cast route"))?;
            self.class_layout(derived, location)?;
            if !cast.downcast {
                return self.static_project_path(place, Some(cast.path), member, location);
            }
            let offset = super::integral::signed(self.graph.lower_path_offset(cast.path)?);
            place.address.offset = place.address.offset.wrapping_sub(offset);
        }
        Ok(place)
    }
    fn is_member_expression(&self, id: ast::ExprId) -> bool {
        matches!(
            self.tree
                .expressions
                .get(self.graph.expression_facts.root(id))
                .kind,
            ExprKind::Member { .. }
        )
    }
    pub(super) fn static_binding(
        &mut self,
        entity: EntityId,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        let recorded = self.graph.constant_addresses.value(entity);
        let direct = match self
            .tree
            .expressions
            .get(self.graph.expression_facts.root(id))
            .kind
        {
            ExprKind::Id(_)
            | ExprKind::Literal(_)
            | ExprKind::Member { .. }
            | ExprKind::Subscript { .. }
            | ExprKind::Cast { .. } => true,
            ExprKind::Unary { op, .. } => op.token.as_punctuator() == Some(P::Multiply),
            _ => false,
        };
        // define_reference first tries static_binding, then initialization_value.
        // Reuse a recorded core binding for forms static_lvalue cannot describe,
        // and for published temporaries; evaluating again would create a new
        // temporary and lose the original storage/demand identity.
        let evaluated = !direct
            || facts.category == super::expression_facts::ValueCategory::Prvalue
            || self
                .graph
                .expression_facts
                .function_conversion(id)
                .is_some()
            || recorded.is_some_and(|address| matches!(address.root, Root::Published(_)));
        let result = if evaluated && let Some(address) = recorded {
            Ok(StaticAddress::evaluated(address))
        } else {
            self.static_direct_binding(id, scope)
        };
        let place = match (result, recorded) {
            (Ok(place), _) => place,
            (Err(Error::NonConstant { .. } | Error::Missing(_)), Some(address)) => {
                StaticAddress::evaluated(address)
            }
            (Err(error), _) => return Err(error),
        };
        self.static_addresses.references.insert(entity, place);
        Ok(place.address)
    }
    fn static_direct_binding(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<StaticAddress, Error> {
        let mut place = self.static_place(id, scope)?;
        let facts = self.graph.expression_facts.get(id)?;
        if facts.conversion.is_some_and(|conversion| {
            conversion.second == super::expression_facts::ValueConversion::DerivedToBase
        }) {
            place = self.static_project_path(
                place,
                facts.base_path,
                self.is_member_expression(id),
                self.tree.expressions.get(id).location,
            )?;
        }
        Ok(place)
    }
    fn static_project_path(
        &mut self,
        mut place: StaticAddress,
        path: Option<BasePathId>,
        member: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<StaticAddress, Error> {
        let Some(path) = path else {
            return Ok(place);
        };
        let edges = self.graph.base_paths.get(path);
        let &(from, _) = edges
            .first()
            .ok_or(Error::Invariant("empty static base path"))?;
        let &(last, index) = edges
            .last()
            .ok_or(Error::Invariant("empty static base path"))?;
        let to = self.graph.entity(last).bases[index].class;
        self.class_layout(from, location)?;
        if place.address.root == Root::Null {
            return Ok(place);
        }
        let route = self.static_addresses.routes.get(&self.graph, from, to)?;
        if route.via.is_none() {
            place.address.offset = place
                .address
                .offset
                .wrapping_add(super::integral::signed(route.offset));
        } else {
            // The root remains the named variable through member access and
            // aliases. Only an immediately named member proves its own layout.
            let (owner, stride) = if member {
                (Some(from), 0)
            } else {
                (
                    place.complete.map(|complete| complete.class),
                    place.complete.map_or(0, |complete| complete.stride),
                )
            };
            let owner = owner
                .filter(|&owner| owner == from || self.graph.is_base_of(from, owner))
                .ok_or_else(|| {
                    nonconstant(location, "a virtual base without a known complete object")
                })?;
            self.class_layout(owner, location)?;
            let origin = if member {
                place.address.offset
            } else if stride != 0 {
                place.address.offset.wrapping_sub(
                    place
                        .address
                        .offset
                        .wrapping_rem(super::integral::signed(stride)),
                )
            } else {
                0
            };
            let placed = self.static_addresses.routes.get(&self.graph, owner, to)?;
            let offset = placed.offset.wrapping_add(if let Some(base) = placed.via {
                self.graph
                    .lower_virtual_base_offset(owner, base)
                    .ok_or(Error::Invariant("static projection before virtual layout"))?
            } else {
                0
            });
            place.address.offset = origin.wrapping_add(super::integral::signed(offset));
        }
        place.address.designated = self.graph.lower_entity_type(to)?;
        Ok(place)
    }
    fn static_image_place(
        value: Value,
        ty: super::types::TypeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<StaticAddress, Error> {
        let (root, offset) = match value {
            Value::Address(entity, offset) => (Root::Variable(entity), offset),
            Value::TypeInformation(ty, offset) => (Root::TypeInformation(ty), offset),
            Value::Literal(id, offset) => (Root::Literal(id), offset),
            Value::Temporary(id, offset) => (Root::Published(id), offset),
            Value::Integer(_)
            | Value::WideInteger(_)
            | Value::Floating(..)
            | Value::MemberPointer(_) => {
                return Err(nonconstant(location, "a static image without an address"));
            }
        };
        Ok(StaticAddress::evaluated(Address {
            offset,
            ..Address::new(root, ty)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{base_projections::BaseRoutes, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn static_virtual_projections_keep_complete_roots_and_core_evaluation_separate() {
        let source = SourceBuffer::new(
            br"
            struct V { int v; virtual ~V() {} };
            struct P { int p; virtual ~P() {} };
            struct B : P, virtual V { int b; };
            struct D : B { int d; };
            struct H { long prefix; D member; };
            D object; D array[3]; H holder;
            V &reference = object; V *pointer = &object;
            B &alias = object; V &alias_reference = alias;
            V &array_reference = array[2];
            V &member_reference = holder.member;
            V *member_pointer = &holder.member;
            B &member_alias = holder.member;
            V &dynamic_reference = member_alias;
            constexpr B &choose(D &object) { return object; }
            B &returned = choose(object); V &returned_base = returned;
            B &conditional = true ? static_cast<B&>(object) : static_cast<B&>(object);
            V *conditional_base = &conditional;
            int target; struct R { int &r; };
            R mutable_member{target}; constexpr R constant_member{target};
            int &mutable_reference = mutable_member.r;
            int &constant_reference = constant_member.r;
            struct Only : virtual V {};
            Only single; V &cast_reference = static_cast<V&>(single);
            struct Left { int l; }; struct Right { int r; };
            struct Both : Left, Right {}; Both both;
            constexpr Both *saved = &both;
            Right *implicit = saved; Right *explicit_cast = static_cast<Right*>(saved);
            template<class T> struct Dormant : T { void f() { T::missing(); } };
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"static-routes").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut entity = |name: &[u8]| {
            graph
                .binding(
                    graph
                        .own(
                            graph.global,
                            cursor.intern_name(name).unwrap(),
                            Filter::Ordinary,
                        )
                        .unwrap(),
                )
                .entity
        };
        let object = entity(b"object");
        let array = entity(b"array");
        let holder = entity(b"holder");
        for (name, root, offset) in [
            (&b"reference"[..], object, 24),
            (b"pointer", object, 24),
            (b"alias_reference", object, 24),
            (b"array_reference", array, 104),
            (b"member_reference", holder, 32),
        ] {
            assert!(matches!(graph.entity(entity(name)).static_initializer,
                Some(Value::Address(found, at)) if found == root && at == offset));
        }
        assert!(
            graph
                .entity(entity(b"member_pointer"))
                .static_initializer
                .is_none()
        );
        assert!(
            graph
                .entity(entity(b"dynamic_reference"))
                .static_initializer
                .is_none()
        );
        for name in [
            &b"returned_base"[..],
            b"conditional_base",
            b"mutable_reference",
        ] {
            assert!(graph.entity(entity(name)).static_initializer.is_none());
        }
        let target = entity(b"target");
        assert!(
            matches!(graph.entity(entity(b"constant_reference")).static_initializer,
            Some(Value::Address(root, 0)) if root == target)
        );
        let single = entity(b"single");
        let reference = entity(b"cast_reference");
        assert!(matches!(graph.entity(reference).static_initializer,
            Some(Value::Address(root, 8)) if root == single));
        assert!(graph.constant_addresses.value(reference).is_none());
        let both = entity(b"both");
        for name in [&b"implicit"[..], b"explicit_cast"] {
            assert!(matches!(graph.entity(entity(name)).static_initializer,
                Some(Value::Address(root, 4)) if root == both));
        }
        let from = graph
            .class_of(graph.lower_entity_type(object).unwrap())
            .unwrap();
        let to = graph.lower_virtual_bases(from)[0].0;
        let mut routes = BaseRoutes::default();
        let selected = routes.get(&graph, from, to).unwrap();
        assert_eq!(selected.via, Some(to));
        assert_eq!(selected.offset, 0);
        assert_eq!(routes.len(), 1);
        for _ in 0..3 {
            assert_eq!(routes.get(&graph, from, to).unwrap().via, Some(to));
        }
        assert_eq!(routes.len(), 1);
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
    }
}
