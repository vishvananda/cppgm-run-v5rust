// Translated from lower/global_lowering.cpp::static_lvalue/static_value. See NOTICE.
//! Static relocations have their own query: a frozen reference cast may bind
//! storage even though it is forbidden in a core constant expression.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, nonconstant},
    graph::ScopeId,
    static_initialization::Value,
    types::TypeKind,
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    syntax::ast::{self, ExprKind},
};

impl Analyzer<'_> {
    pub(super) fn static_place(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.check_stack()?;
        match self.constant_place(id, scope) {
            Ok(place) => return Ok(place),
            Err(Error::NonConstant { .. } | Error::Missing(_)) => (),
            Err(error) => return Err(error),
        }
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        let facts = self.graph.expression_facts.get(id)?;
        let mut place = match node.kind {
            ExprKind::Cast { operand, .. } if facts.reference_cast.is_some() => {
                let place = self.static_place(operand, scope)?;
                self.project_static_cast(place, facts.base_cast, node.location)?
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
                let ty = self.graph.lower_entity_type(entity)?;
                let value = self
                    .graph
                    .entity(entity)
                    .static_initializer
                    .filter(|_| matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }))
                    .ok_or_else(|| {
                        nonconstant(node.location, "a reference without a static target")
                    })?;
                Self::static_image_place(value, facts.ty, node.location)?
            }
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::Multiply) => {
                self.static_address_operand(operand, scope)?
            }
            ExprKind::Subscript { base, index } => {
                let (base, index) = if facts.commuted {
                    (index, base)
                } else {
                    (base, index)
                };
                let mut place = self.static_address_operand(base, scope)?;
                let count = self.evaluate_constant(index, scope, false)?;
                let (size, _) = self.type_size(facts.ty, node.location)?;
                place.offset = place
                    .offset
                    .wrapping_add(super::integral::signed(count.bits.wrapping_mul(size)));
                place
            }
            ExprKind::Member { base, op, .. } => {
                let member = facts
                    .member
                    .ok_or(Error::Invariant("static member designation"))?;
                let place = if op.token.as_punctuator() == Some(P::Arrow) {
                    self.static_address_operand(base, scope)?
                } else {
                    self.static_place(base, scope)?
                };
                let place =
                    self.constant_project_path(place, facts.member_naming_path, node.location)?;
                let place = self.constant_project_path(place, facts.member_path, node.location)?;
                if matches!(
                    self.graph
                        .types
                        .get(
                            self.graph
                                .lower_entity_type(self.graph.binding(member).entity)?
                        )
                        .kind,
                    TypeKind::Reference { .. }
                ) {
                    return Err(nonconstant(
                        node.location,
                        "a reference member without a static target",
                    ));
                }
                return self.constant_member_designation(
                    place,
                    self.graph.binding(member).entity,
                    facts.ty,
                    node.location,
                );
            }
            _ => {
                return Err(nonconstant(
                    node.location,
                    "a glvalue without static storage",
                ));
            }
        };
        place.designated = facts.ty;
        Ok(place)
    }

    pub(super) fn static_address_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.check_stack()?;
        match self.address_operand(id, scope) {
            Ok(place) => return Ok(place),
            Err(Error::NonConstant { .. } | Error::Missing(_)) => (),
            Err(error) => return Err(error),
        }
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        let facts = self.graph.expression_facts.get(id)?;
        match node.kind {
            ExprKind::Unary { op, operand } if op.token.as_punctuator() == Some(P::BitAnd) => {
                self.static_place(operand, scope)
            }
            ExprKind::Cast { operand, .. } => {
                let place = self.static_address_operand(operand, scope)?;
                let mut place = self.project_static_cast(place, facts.base_cast, node.location)?;
                if let TypeKind::Pointer(element) = self.graph.types.get(facts.ty).kind {
                    place.designated = element;
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
                let mut place = self.static_address_operand(pointer, scope)?;
                let count = self.evaluate_constant(index, scope, false)?;
                let (size, _) = self.type_size(place.designated, node.location)?;
                let offset = super::integral::signed(count.bits.wrapping_mul(size));
                place.offset = if op.token.as_punctuator() == Some(P::Subtract) {
                    place.offset.wrapping_sub(offset)
                } else {
                    place.offset.wrapping_add(offset)
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
                    place.designated = element;
                }
                Ok(place)
            }
            _ => Err(nonconstant(
                node.location,
                "a pointer without a static target",
            )),
        }
    }
    fn project_static_cast(
        &mut self,
        mut place: Address,
        cast: Option<super::expression_facts::BaseCast>,
        location: crate::support::source::SourceOffset,
    ) -> Result<Address, Error> {
        if place.root != Root::Null
            && let Some(cast) = cast
        {
            let &(derived, _) = self
                .graph
                .base_paths
                .get(cast.path)
                .first()
                .ok_or(Error::Invariant("empty static cast route"))?;
            self.class_layout(derived, location)?;
            let offset = super::integral::signed(self.graph.lower_path_offset(cast.path)?);
            place.offset = if cast.downcast {
                place.offset.wrapping_sub(offset)
            } else {
                place.offset.wrapping_add(offset)
            };
        }
        Ok(place)
    }
    fn static_image_place(
        value: Value,
        ty: super::types::TypeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Address, Error> {
        let (root, offset) = match value {
            Value::Address(entity, offset) => (Root::Variable(entity), offset),
            Value::TypeInformation(ty, offset) => (Root::TypeInformation(ty), offset),
            Value::Literal(id, offset) => (Root::Literal(id), offset),
            Value::Temporary(id, offset) => (Root::Published(id), offset),
            Value::Integer(_) | Value::Floating(..) | Value::MemberPointer(_) => {
                return Err(nonconstant(location, "a static image without an address"));
            }
        };
        Ok(Address {
            offset,
            ..Address::new(root, ty)
        })
    }
}
