// Translated from constant_expressions.cpp and constant_interpreter.cpp. See NOTICE.
//! Symbolic member identity and subobject adjustment survive constant evaluation.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constants::{Constant, ConstantState},
    expression_facts::ValueCategory,
    graph::{EntityId, Graph, ScopeId},
    types::{TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct MemberPointer {
    pub ty: TypeId,
    pub member: Option<EntityId>,
    pub adjustment: i64,
    provisional: bool,
}
impl MemberPointer {
    pub(super) fn provisional(self) -> bool {
        self.provisional
    }

    pub(super) fn null(ty: TypeId) -> Self {
        Self {
            ty,
            member: None,
            adjustment: 0,
            provisional: false,
        }
    }
}
impl Graph {
    pub(super) fn argument_member_pointer(&self, binding: EntityId) -> Option<MemberPointer> {
        let argument = self.constant_addresses.argument(binding)?;
        let TypeKind::Address { ty, entity } = self.types.get(argument).kind else {
            return None;
        };
        matches!(self.types.get(ty).kind, TypeKind::MemberPointer { .. }).then_some(MemberPointer {
            member: Some(entity),
            ..MemberPointer::null(ty)
        })
    }
}
impl Analyzer<'_> {
    pub(super) fn member_pointer_expression(&self, id: ast::ExprId) -> bool {
        self.graph.expression_facts.get(id).is_ok_and(|f| {
            matches!(
                self.graph.types.get(f.ty).kind,
                TypeKind::MemberPointer { .. }
            )
        })
    }
    pub(super) fn convert_member_scalar(
        value: Value,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<MemberPointer, Error> {
        match value {
            Value::MemberPointer(mut value) => {
                value.ty = target;
                Ok(value)
            }
            Value::Integer(Constant { bits: 0, .. })
            | Value::Pointer(Address {
                root: Root::Null, ..
            }) => Ok(MemberPointer::null(target)),
            _ => Err(nonconstant(
                location,
                "a conversion to a pointer to member outside constant evaluation",
            )),
        }
    }
    pub(super) fn adjusted_member(
        &mut self,
        mut value: MemberPointer,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<MemberPointer, Error> {
        let TypeKind::MemberPointer { owner: from, .. } = self.graph.types.get(value.ty).kind
        else {
            return Err(Error::Invariant("member constant without owner"));
        };
        let TypeKind::MemberPointer { owner: to, .. } = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("member conversion without target owner"));
        };
        if from != to
            && let Some(member) = value.member
        {
            let source = self
                .graph
                .class_of(from)
                .ok_or(Error::Invariant("member source class"))?;
            let destination = self
                .graph
                .class_of(to)
                .ok_or(Error::Invariant("member target class"))?;
            let inverse = self.graph.is_base_of(destination, source);
            let (derived, base) = if inverse {
                (source, destination)
            } else {
                (destination, source)
            };
            if !self.graph.is_base_of(base, derived) {
                return Err(nonconstant(
                    location,
                    "a pointer to member conversion between unrelated classes",
                ));
            }
            if inverse {
                let home = self.graph.member_holder(member, None)?;
                if home != destination
                    && !self.graph.is_base_of(home, destination)
                    && !self.graph.is_base_of(destination, home)
                {
                    return Err(nonconstant(
                        location,
                        "a pointer to member cast to a class unrelated to the member's",
                    ));
                }
            }
            let offset = if self.graph.entity(derived).complete {
                self.class_layout(derived, location)?;
                let path = self.graph.retain_base_path(derived, base)?;
                super::integral::signed(self.graph.lower_path_offset(path)?)
            } else {
                value.provisional = true;
                self.provisional_member_offset(derived, base, location)?
            };
            value.adjustment = value.adjustment.wrapping_add(if inverse {
                offset.wrapping_neg()
            } else {
                offset
            });
        }
        value.ty = self.bare_type(target)?;
        Ok(value)
    }
    fn provisional_member_offset(
        &mut self,
        derived: EntityId,
        base: EntityId,
        location: SourceOffset,
    ) -> Result<i64, Error> {
        for index in 0..self.graph.entity(derived).bases.len() {
            let edge = self.graph.entity(derived).bases[index];
            if edge.is_virtual || (edge.class != base && !self.graph.is_base_of(base, edge.class)) {
                continue;
            }
            let inner = if edge.class == base {
                0
            } else {
                self.class_layout(edge.class, location)?;
                let path = self.graph.retain_base_path(edge.class, base)?;
                self.graph.lower_path_offset(path)?
            };
            let ordinal =
                u64::try_from(index).map_err(|_| Error::Limit("provisional base ordinal"))?;
            return Ok(super::integral::signed(
                ordinal.wrapping_add(1).wrapping_shl(32).wrapping_add(inner),
            ));
        }
        Err(nonconstant(
            location,
            "a pointer to member conversion between unrelated classes",
        ))
    }
    pub(super) fn member_pointer_operand(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<MemberPointer, Error> {
        let value = self.scalar_value(id, scope)?;
        let location = self.tree.expressions.get(id).location;
        if let Value::MemberPointer(value) = value {
            self.adjusted_member(value, target, location)
        } else {
            let target = self.bare_type(target)?;
            Self::convert_member_scalar(value, target, location)
        }
    }
    pub(super) fn evaluate_member_pointer(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<MemberPointer, Error> {
        self.in_constant_evaluation(|a| a.member_pointer_value(id, scope))
    }
    fn member_pointer_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<MemberPointer, Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let node = *self.tree.expressions.get(id);
        self.constant_step(node.location)?;
        let facts = self.graph.expression_facts.get(id)?;
        let ty = self.bare_type(facts.ty)?;
        if let ExprKind::Id(name) = node.kind
            && let Some(binding) = self.graph.resolved_name(name)
            && let Some(value) = self
                .graph
                .argument_member_pointer(self.graph.binding(binding).entity)
        {
            return Ok(value);
        }
        if facts.category == ValueCategory::Prvalue && facts.folded == Some(0) {
            return Ok(MemberPointer::null(ty));
        }
        if facts.category != ValueCategory::Prvalue
            && !matches!(node.kind, ExprKind::Conditional { .. })
        {
            let place = self.constant_place(id, scope)?;
            return self.read_constant_member_pointer(place, node.location);
        }
        match node.kind {
            ExprKind::Unary { op, .. } if op.token.as_punctuator() == Some(P::BitAnd) => {
                let member = facts.function.or(facts.member).ok_or_else(|| {
                    nonconstant(node.location, "a pointer to member of no member")
                })?;
                Ok(MemberPointer {
                    ty,
                    member: Some(self.graph.binding(member).entity),
                    adjustment: 0,
                    provisional: false,
                })
            }
            ExprKind::Paren(inner) => self.evaluate_member_pointer(inner, scope),
            ExprKind::Cast {
                operand, keyword, ..
            } => {
                if matches!(
                    keyword,
                    Some(Keyword::ReinterpretCast | Keyword::DynamicCast)
                ) {
                    return Err(nonconstant(
                        node.location,
                        "a reinterpret_cast or dynamic_cast in a constant expression",
                    ));
                }
                let value = self.scalar_value(operand, scope)?;
                if let Value::MemberPointer(value) = value {
                    self.adjusted_member(value, ty, node.location)
                } else {
                    Self::convert_member_scalar(value, ty, node.location)
                }
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if let Some(braced) = braced {
                    return self.member_pointer_operand(braced, ty, scope);
                }
                self.member_pointer_arguments(arguments, ty, scope, node.location)
            }
            ExprKind::Call { arguments, .. } if facts.functional_call => {
                self.member_pointer_arguments(arguments, ty, scope, node.location)
            }
            ExprKind::Call { .. } => {
                let returned = self.constant_call_result(id, scope)?;
                let value = self.returned_scalar(returned, node.location)?;
                Self::convert_member_scalar(value, ty, node.location)
            }
            ExprKind::Braced { elements, .. } => {
                self.member_pointer_arguments(elements, ty, scope, node.location)
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
                self.member_pointer_operand(selected, ty, scope)
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment: false,
            } if op.token.as_punctuator() == Some(P::Comma) => {
                self.discard_constant(left, scope)?;
                self.member_pointer_operand(right, ty, scope)
            }
            _ => Err(nonconstant(
                node.location,
                "not an integral constant expression",
            )),
        }
    }
    fn member_pointer_arguments(
        &mut self,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<MemberPointer, Error> {
        match self.tree.expr_lists.get(arguments) {
            [] => Ok(MemberPointer::null(ty)),
            [id] => self.member_pointer_operand(*id, ty, scope),
            _ => Err(nonconstant(location, "not an integral constant expression")),
        }
    }
    pub(super) fn read_constant_member_pointer(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<MemberPointer, Error> {
        if self.type_cv(place.designated).0 & 2 != 0 {
            return Err(nonconstant(location, "a read of a volatile glvalue"));
        }
        if place.past_end {
            return Err(nonconstant(
                location,
                "an access one past the end of an array",
            ));
        }
        if let Some(value) = self.published_scalar(place, location)? {
            return match value {
                Value::MemberPointer(value) => Ok(value),
                _ => Err(Error::Invariant("published member-pointer scalar type")),
            };
        }
        let constant = self.type_cv(place.designated).0 == 1;
        place.designated = self.bare_type(place.designated)?;
        let value = match place.root {
            Root::Object(_) => self.constexpr_frames.read(place, location)?,
            Root::Variable(entity) => {
                self.instantiate_variable_value(entity)?;
                let declared = self.graph.lower_entity_type(entity)?;
                if self.type_cv(declared).0 & 2 != 0 {
                    return Err(nonconstant(location, "a read of a volatile object"));
                }
                if place.offset != 0 || self.bare_type(declared)? != place.designated {
                    return Err(nonconstant(
                        location,
                        "an access outside the designated object",
                    ));
                }
                match self.graph.entity(entity).constant {
                    ConstantState::MemberPointer(value) if self.graph.entity(entity).constexpr => {
                        return Ok(value);
                    }
                    ConstantState::Evaluating => {
                        return Err(nonconstant(
                            location,
                            "a read of an object that is not initialized",
                        ));
                    }
                    _ => {
                        return Err(nonconstant(
                            location,
                            "a variable is not usable in a constant expression",
                        ));
                    }
                }
            }
            Root::Ephemeral(id) => self
                .expression_temporary_value(id)
                .ok_or(Error::Invariant("member-pointer temporary without value"))?,
            Root::Temporary(entity) if constant => self
                .graph
                .constant_addresses
                .temporary_scalar(entity)
                .ok_or(Error::Invariant(
                    "member-pointer reference temporary without value",
                ))?,
            Root::Temporary(_) | Root::Published(_) => {
                return Err(nonconstant(
                    location,
                    "an access to a temporary that is not const, bound outside the evaluation",
                ));
            }
            Root::Null => {
                return Err(nonconstant(
                    location,
                    "an indirection through a null pointer",
                ));
            }
            Root::TypeInformation(_) => {
                return Err(nonconstant(
                    location,
                    "an access to a std::type_info object",
                ));
            }
            Root::Function(_) => return Err(nonconstant(location, "a function is not an object")),
            Root::Literal(_) => {
                return Err(nonconstant(
                    location,
                    "an access outside the designated object",
                ));
            }
        };
        match value {
            Value::MemberPointer(value) => Ok(value),
            _ => Err(Error::Invariant("member-pointer place holds other scalar")),
        }
    }
    pub(super) fn constant_member_pointer_place(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let (pointer, object) = self.constant_member_pointer_object(id, scope)?;
        let member = pointer
            .member
            .ok_or(Error::Invariant("applied null member constant"))?;
        let ty = self.graph.expression_facts.get(id)?.ty;
        self.constant_member_designation(object, member, ty, self.tree.expressions.get(id).location)
    }
    pub(super) fn constant_member_pointer_object(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(MemberPointer, Address), Error> {
        let mut id = self.graph.expression_facts.root(id);
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        let node = *self.tree.expressions.get(id);
        let ExprKind::Binary {
            op, left, right, ..
        } = node.kind
        else {
            return Err(nonconstant(
                node.location,
                "a bound member function call outside constant evaluation",
            ));
        };
        let mut object = if op.token.as_punctuator() == Some(P::ArrowStar) {
            let object = self.address_operand(left, scope)?;
            if object.root == Root::Null {
                return Err(nonconstant(node.location, "->* through a null pointer"));
            }
            object
        } else {
            self.constant_object_place(left, scope)?
        };
        let ty = self.graph.expression_facts.get(right)?.ty;
        let pointer = self.member_pointer_operand(right, ty, scope)?;
        let Some(member) = pointer.member else {
            return Err(nonconstant(
                node.location,
                "a null pointer to member applied to an object",
            ));
        };
        let TypeKind::MemberPointer { owner, .. } = self.graph.types.get(pointer.ty).kind else {
            return Err(Error::Invariant("applied member constant type"));
        };
        let class = self
            .graph
            .class_of(owner)
            .ok_or(Error::Invariant("applied member owner"))?;
        let from = self
            .graph
            .class_of(object.designated)
            .ok_or(Error::Invariant("member object type"))?;
        if class != from {
            let path = self.graph.retain_base_path(from, class)?;
            object = self.constant_project_path(object, Some(path), node.location)?;
        }
        let home = self.graph.member_holder(member, None)?;
        if pointer.adjustment != 0 || home != class {
            object.offset = object.offset.wrapping_add(pointer.adjustment);
            object.designated = self.graph.types.qualify(
                self.graph.lower_entity_type(home)?,
                self.type_cv(object.designated),
            )?;
            let root = self.address_root_type(object, node.location)?;
            if !self.constant_type_holds(root, object.offset, object.designated, node.location)? {
                return Err(nonconstant(
                    node.location,
                    "a pointer to member of a class the object does not contain",
                ));
            }
        }
        Ok((pointer, object))
    }
    pub(super) fn record_member_pointer_initializer(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.graph.entity_mut(entity).constant = ConstantState::Evaluating;
        let result = self.in_constant_evaluation(|a| {
            let ty = a.graph.lower_entity_type(entity)?;
            if let Some(expr) = a.single_initializer(initializer) {
                a.member_pointer_operand(expr, ty, scope)
            } else {
                Ok(MemberPointer::null(a.bare_type(ty)?))
            }
        });
        self.graph.entity_mut(entity).constant = ConstantState::None;
        match result {
            Ok(value) => {
                self.graph.entity_mut(entity).constant = ConstantState::MemberPointer(value);
                if value.provisional {
                    self.deferred
                        .push(super::analyzer::DeferredMember::Constant(
                            entity,
                            initializer,
                            scope,
                        ));
                }
            }
            Err(Error::NonConstant { location, message }) => {
                return Err(Error::at(location, message));
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }
    pub(super) fn consumes_member_pointer(&self, id: ast::ExprId) -> bool {
        match self.tree.expressions.get(id).kind {
            ExprKind::Binary {
                left, right, op, ..
            } => {
                matches!(op.token.as_punctuator(), Some(P::DotStar | P::ArrowStar))
                    || self.member_pointer_expression(left)
                    || self.member_pointer_expression(right)
            }
            _ => false,
        }
    }
    pub(super) fn member_integral_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Option<Constant>, Error> {
        let node = *self.tree.expressions.get(id);
        if let ExprKind::Binary { op, .. } = node.kind
            && matches!(op.token.as_punctuator(), Some(P::DotStar | P::ArrowStar))
        {
            if skipping {
                return Ok(Some(Constant {
                    bits: 0,
                    ty: self.graph.expression_facts.get(id)?.ty,
                }));
            }
            let place = self.constant_place(id, scope)?;
            return self.read_constant_integer(place, node.location).map(Some);
        }
        if let ExprKind::Binary {
            left, right, op, ..
        } = node.kind
            && matches!(op.token.as_punctuator(), Some(P::LogicalAnd | P::LogicalOr))
            && (self.member_pointer_expression(left) || self.member_pointer_expression(right))
        {
            let truth = if skipping {
                false
            } else {
                let first = self.constant_truth(left, scope, false)?;
                if op.token.as_punctuator() == Some(P::LogicalAnd) {
                    first && self.constant_truth(right, scope, false)?
                } else {
                    first || self.constant_truth(right, scope, false)?
                }
            };
            return Ok(Some(Constant {
                bits: u64::from(truth),
                ty: self.graph.expression_facts.get(id)?.ty,
            }));
        }
        let operands = match node.kind {
            ExprKind::Binary {
                left,
                right,
                op,
                assignment: false,
            } if matches!(op.token.as_punctuator(), Some(P::Equal | P::NotEqual))
                && (self.member_pointer_expression(left)
                    || self.member_pointer_expression(right)) =>
            {
                Some((
                    left,
                    Some((right, op.token.as_punctuator() == Some(P::Equal))),
                ))
            }
            ExprKind::Unary { op, operand }
                if op.token.as_punctuator() == Some(P::LogicalNot)
                    && self.member_pointer_expression(operand) =>
            {
                Some((operand, None))
            }
            ExprKind::Cast { operand, .. } if self.member_pointer_expression(operand) => {
                Some((operand, None))
            }
            _ => None,
        };
        let Some((left, right)) = operands else {
            return Ok(None);
        };
        let ty = self.bare_type(self.graph.expression_facts.get(id)?.ty)?;
        if skipping {
            return Ok(Some(Constant { bits: 0, ty }));
        }
        let bits = if let Some((right, equality)) = right {
            let common = self
                .graph
                .expression_facts
                .get(left)?
                .conversion
                .ok_or(Error::Invariant("member comparison without conversion"))?
                .result;
            let a = self.member_pointer_operand(left, common, scope)?;
            let b = self.member_pointer_operand(right, common, scope)?;
            if a.member.is_some()
                && b.member.is_some()
                && [a.member, b.member]
                    .into_iter()
                    .flatten()
                    .any(|m| self.graph.lower_virtual_slot(m).is_some())
            {
                return Err(nonconstant(
                    node.location,
                    "an unspecified comparison of pointers to virtual member functions",
                ));
            }
            u64::from(
                equality
                    == (a.member == b.member
                        && (a.member.is_none() || a.adjustment == b.adjustment)),
            )
        } else {
            let value = self.scalar_value(left, scope)?;
            if matches!(node.kind, ExprKind::Unary { .. }) {
                let Value::MemberPointer(value) = value else {
                    return Err(Error::Invariant("member truth type"));
                };
                u64::from(value.member.is_none())
            } else {
                let Value::Integer(value) =
                    self.convert_constant_scalar(value, ty, node.location)?
                else {
                    return Err(nonconstant(
                        node.location,
                        "not an integral constant expression",
                    ));
                };
                value.bits
            }
        };
        Ok(Some(Constant { bits, ty }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn member_constants_keep_declarations_and_release_provisional_offsets() {
        let source = SourceBuffer::new(
            br#"
            struct A { long padding; };
            struct B { int first, second; constexpr int get() const { return second; } };
            struct D : A, B {
                static constexpr int D::*p = &B::first;
                static constexpr int D::*q = &B::second;
                static_assert(p != q && p == p, "");
                int tail;
            };
            constexpr int D::*D::p;
            constexpr int D::*D::q;
            constexpr int B::*reverse = static_cast<int B::*>(D::q);
            constexpr int D::*null = nullptr;
            struct Holder { int D::*p; int (B::*f)() const; };
            constexpr Holder saved = { D::q, &B::get };
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-constant-storage").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"D".as_slice(), b"reverse", b"null", b"saved"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let syntax_mark = analyzer.tree.expressions.mark();
        analyzer.finish_static_initializers().unwrap();
        assert_eq!(syntax_mark, analyzer.tree.expressions.mark());
        let entities: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
                analyzer.graph.binding(binding).entity
            })
            .collect();
        let ConstantState::MemberPointer(reverse) = analyzer.graph.entity(entities[1]).constant
        else {
            panic!("reverse member value");
        };
        assert_eq!(reverse.adjustment, 0);
        assert!(!reverse.provisional);
        let ConstantState::MemberPointer(null) = analyzer.graph.entity(entities[2]).constant else {
            panic!("null member value");
        };
        assert!(null.member.is_none());
        assert_eq!(null.adjustment, 0);
        let class_scope = analyzer.graph.entity(entities[0]).scope.unwrap();
        let members: Vec<_> = analyzer.graph.entity(entities[0]).members.clone();
        assert_eq!(members.len(), 1);
        let saved = analyzer.graph.aggregate_constants.get(entities[3]).unwrap();
        let values: Vec<_> = saved
            .values()
            .filter_map(|(_, _, _, value)| match value {
                Some(Value::MemberPointer(value)) => Some(value),
                _ => None,
            })
            .collect();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].member, reverse.member);
        assert_eq!(values[0].adjustment, 8);
        assert!(!values[0].provisional);
        assert_ne!(values[1].member, reverse.member);
        assert_eq!(values[1].adjustment, 0);
        assert!(analyzer.graph.scope(class_scope).owner.is_some());
        assert!(analyzer.deferred.is_empty());
        assert!(analyzer.constant_evaluation.is_none());
        assert!(!analyzer.constexpr_frames.active());
    }
}
