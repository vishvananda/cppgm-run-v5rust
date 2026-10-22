// Translated from constant_interpreter.cpp and constant_expressions.cpp. See NOTICE.
//! Target byte-offset arithmetic, literal identity and integral uses of addresses.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, nonconstant},
    constants::Constant,
    expression_facts::ValueCategory,
    graph::{EntityId, ScopeId},
    integral,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
};

/// A completed class's immutable byte spans. The evaluation scratch belongs to
/// one analyzer/unit; only complete layouts enter the class-identity cache.
#[derive(Default)]
pub(super) struct MemberSpans(std::collections::HashMap<EntityId, Vec<MemberSpan>>);
#[derive(Clone, Copy)]
struct MemberSpan {
    member: EntityId,
    ty: TypeId,
    start: i64,
    size: i64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum OrderedPart {
    Base(usize),
    Member(EntityId),
}
#[derive(Clone, Copy)]
struct Subobject {
    part: OrderedPart,
    ty: TypeId,
    start: i64,
}

impl Analyzer<'_> {
    pub(super) fn constant_subscript_place(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let node = *self.tree.expressions.get(id);
        let ExprKind::Subscript { base, index } = node.kind else {
            return Err(Error::Invariant(
                "constant subscript without subscript syntax",
            ));
        };
        let facts = self.graph.expression_facts.get(id)?;
        let (base, index) = if facts.commuted {
            (index, base)
        } else {
            (base, index)
        };
        let array = self.graph.expression_facts.root(base);
        let base_facts = self.graph.expression_facts.get(array)?;
        let mut pointer = if let TypeKind::Array { element, .. } =
            self.graph.types.get(base_facts.ty).kind
            && base_facts.category == ValueCategory::Prvalue
        {
            // CWG 1213: the element belongs to the array prvalue's
            // temporary. Materialize it once before evaluating the index.
            let mut place = self.constant_object_place(array, scope)?;
            place.designated = element;
            place.pointer()
        } else {
            self.address_operand(base, scope)?
        };
        let index = self.evaluate_constant(index, scope, false)?;
        if pointer.root == Root::Null {
            return Err(nonconstant(node.location, "a subscript of a null pointer"));
        }
        self.advance_address(&mut pointer, integral::signed(index.bits), node.location)?;
        pointer.designated = facts.ty;
        Ok(pointer)
    }

    pub(super) fn constant_truth(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<bool, Error> {
        if self.semantic() && self.member_pointer_expression(id) {
            return if skipping {
                Ok(false)
            } else {
                Ok(self.evaluate_member_pointer(id, scope)?.member.is_some())
            };
        }
        if self.semantic() && self.floating_expression(id) {
            return if skipping {
                Ok(false)
            } else {
                Ok(self.evaluate_floating(id, scope)?.value.truth())
            };
        }
        if self.semantic() && self.is_address_expression(id) {
            if skipping {
                return Ok(false);
            }
            return Ok(self.evaluate_address(id, scope)?.root != Root::Null);
        }
        let value = self.evaluate_constant(id, scope, skipping)?;
        self.check_constant_bool(value, self.tree.expressions.get(id).location)?;
        Ok(value.truth())
    }
    pub(super) fn consumes_address(&self, id: ast::ExprId) -> bool {
        if self
            .graph
            .expression_facts
            .get(id)
            .is_ok_and(|facts| facts.call_form == super::expression_facts::CallForm::Operator)
        {
            return false;
        }
        if self.is_address_expression(id) {
            return false;
        }
        match self.tree.expressions.get(id).kind {
            ExprKind::Binary {
                left,
                right,
                op,
                assignment: false,
            } => {
                matches!(
                    op.token.as_punctuator(),
                    Some(
                        P::Equal
                            | P::NotEqual
                            | P::Less
                            | P::Greater
                            | P::LessEqual
                            | P::GreaterEqual
                            | P::LogicalAnd
                            | P::LogicalOr
                            | P::Subtract
                    )
                ) && (self.is_address_expression(left) || self.is_address_expression(right))
            }
            ExprKind::Unary { op, operand } => {
                matches!(op.token.as_punctuator(), Some(P::LogicalNot | P::Multiply))
                    && self.is_address_expression(operand)
            }
            ExprKind::Subscript { .. } => true,
            ExprKind::Cast { operand, .. } => {
                self.is_address_expression(operand)
                    || self
                        .graph
                        .expression_facts
                        .get(id)
                        .is_ok_and(|facts| facts.reference_cast.is_some())
            }
            ExprKind::FunctionalCast { arguments, .. } | ExprKind::Call { arguments, .. } => {
                if self
                    .graph
                    .expression_facts
                    .get(id)
                    .is_ok_and(|facts| facts.reference_cast.is_some())
                {
                    return true;
                }
                !self.is_address_expression(id)
                    && self
                        .tree
                        .expr_lists
                        .get(arguments)
                        .first()
                        .is_some_and(|&operand| self.is_address_expression(operand))
                    && self.graph.expression_facts.get(id).is_ok_and(|facts| {
                        !matches!(self.tree.expressions.get(id).kind, ExprKind::Call { .. })
                            || facts.functional_call
                    })
            }
            _ => false,
        }
    }
    pub(super) fn address_integral_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<Option<Constant>, Error> {
        if !self.consumes_address(id) {
            return Ok(None);
        }
        let facts = self.graph.expression_facts.get(id)?;
        if skipping {
            return Ok(Some(Constant {
                high: 0,
                bits: 0,
                ty: facts.ty,
            }));
        }
        let node = *self.tree.expressions.get(id);
        let bits = match node.kind {
            ExprKind::Binary {
                left, right, op, ..
            } => {
                let op = op
                    .token
                    .as_punctuator()
                    .ok_or(Error::Invariant("address operator without punctuator"))?;
                if matches!(op, P::LogicalAnd | P::LogicalOr) {
                    let left = self.constant_truth(left, scope, false)?;
                    u64::from(if op == P::LogicalAnd {
                        left && self.constant_truth(right, scope, false)?
                    } else {
                        left || self.constant_truth(right, scope, false)?
                    })
                } else {
                    let left = self.address_operand(left, scope)?;
                    let right = self.address_operand(right, scope)?;
                    if op == P::Subtract {
                        self.address_difference(left, right, node.location)?
                            .cast_unsigned()
                    } else {
                        u64::from(self.address_comparison(op, left, right, node.location)?)
                    }
                }
            }
            ExprKind::Unary { op, .. } if op.token.as_punctuator() == Some(P::Multiply) => {
                let place = self.constant_place(id, scope)?;
                return self.read_constant_integer(place, node.location).map(Some);
            }
            ExprKind::Subscript { .. } => {
                let place = self.constant_place(id, scope)?;
                return self.read_constant_integer(place, node.location).map(Some);
            }
            ExprKind::Unary { operand, .. } => {
                u64::from(!self.constant_truth(operand, scope, false)?)
            }
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
                if facts.reference_cast.is_some() {
                    let place = self.constant_place(id, scope)?;
                    return self.read_constant_integer(place, node.location).map(Some);
                }
                self.address_to_integer(operand, facts.ty, scope, node.location)?
            }
            ExprKind::FunctionalCast { arguments, .. } | ExprKind::Call { arguments, .. } => {
                if facts.reference_cast.is_some() {
                    let place = self.constant_place(id, scope)?;
                    return self.read_constant_integer(place, node.location).map(Some);
                }
                let operand = self
                    .tree
                    .expr_lists
                    .get(arguments)
                    .first()
                    .copied()
                    .ok_or(Error::Invariant("address conversion without operand"))?;
                self.address_to_integer(operand, facts.ty, scope, node.location)?
            }
            _ => {
                return Err(Error::Invariant(
                    "address integral query without owning expression",
                ));
            }
        };
        Ok(Some(Constant {
            high: 0,
            bits,
            ty: self.bare_type(facts.ty)?,
        }))
    }
    fn address_to_integer(
        &mut self,
        operand: ast::ExprId,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<u64, Error> {
        let address = self.evaluate_address(operand, scope)?;
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(Scalar::Bool)
        ) {
            return Ok(u64::from(address.root != Root::Null));
        }
        Err(nonconstant(
            location,
            "a pointer converted to an integer in a constant expression",
        ))
    }
    pub(super) fn address_arithmetic(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let node = *self.tree.expressions.get(id);
        let ExprKind::Binary {
            left, right, op, ..
        } = node.kind
        else {
            return Err(Error::Invariant("pointer arithmetic without binary syntax"));
        };
        // Rvalues are obtained in source order, even when the index precedes the pointer.
        let (mut pointer, count) = if self.is_address_expression(left) {
            let pointer = self.evaluate_address(left, scope)?;
            let count = self.evaluate_constant(right, scope, false)?;
            (pointer, count)
        } else {
            let count = self.evaluate_constant(left, scope, false)?;
            let pointer = self.evaluate_address(right, scope)?;
            (pointer, count)
        };
        let steps = integral::signed(count.bits);
        let steps = if op.token.as_punctuator() == Some(P::Subtract) {
            steps.wrapping_neg()
        } else {
            steps
        };
        self.advance_address(&mut pointer, steps, node.location)?;
        Ok(pointer)
    }
    pub(super) fn address_root_type(
        &self,
        address: Address,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        match address.root {
            Root::Object(id) => self.constexpr_frames.ty(id, location),
            Root::Variable(entity) => self
                .graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("address variable without type")),
            Root::Temporary(entity) => {
                let ty = self
                    .graph
                    .entity(entity)
                    .ty
                    .ok_or(Error::Invariant("extended temporary without owner type"))?;
                match self.graph.types.get(ty).kind {
                    TypeKind::Reference { element, .. } => Ok(element),
                    _ => Err(Error::Invariant("temporary owner is not a reference")),
                }
            }
            Root::Published(id) => Ok(self.graph.aggregate_constants.temporary(id).ty),
            Root::Ephemeral(expr) => Ok(self.graph.expression_facts.get(expr)?.ty),
            Root::Literal(id) => self.graph.lower_literal_storage(id),
            Root::TypeInformation(_) => Err(nonconstant(
                location,
                "an access to a std::type_info object",
            )),
            Root::Function(_) => Err(nonconstant(location, "a function is not an object")),
            Root::Null => Err(nonconstant(
                location,
                "an indirection through a null pointer",
            )),
        }
    }
    pub(super) fn same_address_root(&self, a: Address, b: Address) -> Result<bool, Error> {
        if a.root == b.root {
            return Ok(true);
        }
        let (Root::Literal(a), Root::Literal(b)) = (a.root, b.root) else {
            return Ok(false);
        };
        let (a_element, a_bytes, a_null) = self.constant_literal_units(a)?;
        let (b_element, b_bytes, b_null) = self.constant_literal_units(b)?;
        Ok(a_element == b_element
            && a_bytes
                .iter()
                .copied()
                .chain(a_null.then_some(0))
                .eq(b_bytes.iter().copied().chain(b_null.then_some(0))))
    }
    fn constant_literal_units(&self, id: ast::ExprId) -> Result<(Scalar, &[u8], bool), Error> {
        let ExprKind::Literal(literal) = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("literal address without literal syntax"));
        };
        let TypeKind::Array { element, .. } = self
            .graph
            .types
            .get(self.graph.lower_literal_storage(id)?)
            .kind
        else {
            return Err(Error::Invariant("literal address without array storage"));
        };
        let TypeKind::Scalar(element) = self.graph.types.get(element).kind else {
            return Err(Error::Invariant("literal address without scalar elements"));
        };
        if let Some(super::literal_calls::LiteralCall::Raw { digits, .. }) =
            self.graph.expression_facts.get(id)?.literal_call
        {
            let bytes = self
                .literal_bytes
                .get(literal.spelling)
                .get(..digits)
                .ok_or(Error::Invariant("raw literal digits outside spelling"))?;
            return Ok((element, bytes, true));
        }
        Ok((element, self.literal_bytes.get(literal.data), false))
    }
    fn address_comparison(
        &mut self,
        op: P,
        a: Address,
        b: Address,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let same = self.same_address_root(a, b)?;
        if matches!(op, P::Equal | P::NotEqual) {
            return Ok((op == P::Equal) == (same && a.offset == b.offset));
        }
        if !same || matches!(a.root, Root::Null | Root::Function(_)) {
            return Err(nonconstant(location, "an unspecified pointer comparison"));
        }
        let ty = self.address_root_type(a, location)?;
        if !self.ordered_constant_offsets(ty, a.offset, b.offset, location)? {
            return Err(nonconstant(location, "an unspecified pointer comparison"));
        }
        integral::comparison(op, true, a.offset.cast_unsigned(), b.offset.cast_unsigned()).ok_or(
            Error::Invariant("pointer comparison without comparison operator"),
        )
    }
    fn ordered_constant_offsets(
        &mut self,
        mut ty: TypeId,
        mut a: i64,
        mut b: i64,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        loop {
            if a == b {
                return Ok(true);
            }
            if let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                let size = self.address_element_size(element, location)?;
                if size == 0 || a / size != b / size {
                    return Ok(true);
                }
                ty = element;
                a %= size;
                b %= size;
                continue;
            }
            let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
                return Ok(true);
            };
            if self.graph.entity(class).key == Keyword::Union {
                return Ok(true);
            }
            self.class_layout(class, location)?;
            let left = self.ordered_constant_subobject(class, a, location)?;
            let right = self.ordered_constant_subobject(class, b, location)?;
            if left.is_some_and(|s| matches!(s.part, OrderedPart::Base(_)))
                || right.is_some_and(|s| matches!(s.part, OrderedPart::Base(_)))
            {
                if left.map(|s| s.part) != right.map(|s| s.part) {
                    return Ok(false);
                }
            } else if let (Some(left), Some(right)) = (left, right) {
                if left.part != right.part {
                    return Ok(self
                        .ordered_member_access(class, left.part)
                        .is_some_and(|a| {
                            Some(a) == self.ordered_member_access(class, right.part)
                        }));
                }
                if matches!(
                    self.graph.types.get(left.ty).kind,
                    TypeKind::Reference { .. }
                ) {
                    return Ok(true);
                }
            }
            let (Some(left), Some(right)) = (left, right) else {
                return Ok(true);
            };
            ty = left.ty;
            a = a.wrapping_sub(left.start);
            b = b.wrapping_sub(right.start);
        }
    }
    fn ordered_member_access(
        &self,
        class: EntityId,
        part: OrderedPart,
    ) -> Option<super::graph::Access> {
        let OrderedPart::Member(member) = part else {
            return None;
        };
        let name = self.graph.entity(member).name?;
        let scope = self.graph.entity(class).scope?;
        self.graph
            .own(scope, name, super::lookup::Filter::Ordinary)
            .map(|binding| self.graph.binding_access(binding))
    }
    fn ordered_constant_subobject(
        &mut self,
        class: EntityId,
        offset: i64,
        location: SourceOffset,
    ) -> Result<Option<Subobject>, Error> {
        for index in 0..self.graph.entity(class).bases.len() {
            let base = self.graph.entity(class).bases[index];
            if self.graph.lower_class_empty(base.class)? {
                continue;
            }
            let start = integral::signed(
                base.offset
                    .ok_or(Error::Invariant("ordered base without layout"))?,
            );
            let ty = self.graph.lower_entity_type(base.class)?;
            let size = self.address_element_size(ty, location)?;
            if offset >= start && offset < start.wrapping_add(size) {
                return Ok(Some(Subobject {
                    part: OrderedPart::Base(index),
                    ty,
                    start,
                }));
            }
        }
        let known = self
            .constant_evaluation
            .as_ref()
            .ok_or(Error::Invariant("ordering outside evaluation"))?
            .projections
            .as_ref()
            .is_some_and(|query| query.member_spans.0.contains_key(&class));
        if !known {
            let mut spans = Vec::with_capacity(self.graph.entity(class).members.len());
            for index in 0..self.graph.entity(class).members.len() {
                let member = self.graph.entity(class).members[index];
                if self.graph.entity(member).bit_field.is_some() {
                    continue;
                }
                let ty = self.graph.lower_entity_type(member)?;
                let start = integral::signed(
                    self.graph
                        .entity(member)
                        .member_offset
                        .ok_or(Error::Invariant("ordered member without layout"))?,
                );
                let size = if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                    8
                } else {
                    self.address_element_size(ty, location)?.max(1)
                };
                spans.push(MemberSpan {
                    member,
                    ty,
                    start,
                    size,
                });
            }
            spans.sort_unstable_by_key(|span| span.start);
            self.constant_evaluation
                .as_mut()
                .ok_or(Error::Invariant("ordering lost evaluation"))?
                .projections
                .get_or_insert_with(Box::default)
                .member_spans
                .0
                .insert(class, spans);
        }
        let spans = &self
            .constant_evaluation
            .as_ref()
            .ok_or(Error::Invariant("ordering outside evaluation"))?
            .projections
            .as_ref()
            .ok_or(Error::Invariant("ordering lost member spans"))?
            .member_spans
            .0
            .get(&class)
            .ok_or(Error::Invariant("completed member spans missing"))?;
        let found = spans
            .partition_point(|span| span.start <= offset)
            .checked_sub(1)
            .and_then(|index| spans.get(index))
            .filter(|span| offset < span.start.wrapping_add(span.size));
        Ok(found.map(|span| Subobject {
            part: OrderedPart::Member(span.member),
            ty: span.ty,
            start: span.start,
        }))
    }
    fn address_difference(
        &mut self,
        a: Address,
        b: Address,
        location: SourceOffset,
    ) -> Result<i64, Error> {
        if !self.same_address_root(a, b)? {
            return Err(nonconstant(
                location,
                "a difference of pointers into different objects",
            ));
        }
        let size = self.address_element_size(a.designated, location)?;
        if size == 0 {
            return Err(nonconstant(
                location,
                "a difference of pointers to an incomplete type",
            ));
        }
        if a.root != Root::Null
            && self.address_array_bounds(a, size, location)?
                != self.address_array_bounds(b, size, location)?
        {
            return Err(nonconstant(
                location,
                "a difference of pointers into different arrays",
            ));
        }
        Ok(a.offset.wrapping_sub(b.offset) / size)
    }
    fn address_element_size(&mut self, ty: TypeId, location: SourceOffset) -> Result<i64, Error> {
        match self.type_size(ty, location) {
            Ok((size, _)) => Ok(integral::signed(size)),
            Err(Error::Diagnostic {
                message:
                    "size of an incomplete class"
                    | "size of an array of unknown bound"
                    | "size of an incomplete enumeration",
                ..
            }) => Ok(0),
            Err(error) => Err(error),
        }
    }
    pub(super) fn advance_address(
        &mut self,
        pointer: &mut Address,
        count: i64,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if pointer.root == Root::Null {
            return if count == 0 {
                Ok(())
            } else {
                Err(nonconstant(location, "arithmetic on a null pointer"))
            };
        }
        if matches!(pointer.root, Root::Function(_)) {
            return Err(nonconstant(
                location,
                "arithmetic on a pointer that designates no object",
            ));
        }
        let size = self.address_element_size(pointer.designated, location)?;
        if count == 0 {
            return Ok(());
        }
        if size == 0 {
            return Err(nonconstant(
                location,
                "arithmetic on a pointer to an incomplete type",
            ));
        }
        let (first, bound) = self.address_array_bounds(*pointer, size, location)?;
        let moved = pointer.offset.wrapping_sub(first) / size;
        let moved = moved.wrapping_add(count);
        if moved < 0 || moved > bound {
            return Err(nonconstant(
                location,
                "pointer arithmetic beyond the array's bounds",
            ));
        }
        pointer.offset = first.wrapping_add(moved.wrapping_mul(size));
        pointer.past_end = moved == bound;
        Ok(())
    }
    fn address_array_bounds(
        &mut self,
        mut place: Address,
        size: i64,
        location: SourceOffset,
    ) -> Result<(i64, i64), Error> {
        if place.past_end {
            place.offset = place.offset.wrapping_sub(size);
        }
        let mut ty = self.address_root_type(place, location)?;
        let mut base = 0i64;
        let mut found = None;
        for _ in 0..256 {
            let offset = place.offset.wrapping_sub(base);
            if let TypeKind::Array { element, bound, .. } = self.graph.types.get(ty).kind {
                let bytes = self.address_element_size(element, location)?;
                let count = integral::signed(bound);
                if bytes == 0 || offset < 0 || offset >= bytes.wrapping_mul(count) {
                    break;
                }
                if self.bare_type(element)? == self.bare_type(place.designated)?
                    && offset % bytes == 0
                {
                    found = Some((base, count));
                }
                base = base.wrapping_add((offset / bytes).wrapping_mul(bytes));
                ty = element;
            } else if offset == 0
                && self.bare_type(ty)? == self.bare_type(place.designated)?
                && found.is_none()
            {
                return Ok((base, 1));
            } else if let TypeKind::Class(class) = self.graph.types.get(ty).kind {
                self.class_layout(class, location)?;
                let Some((member_type, start)) =
                    self.array_constant_subobject(class, offset, location)?
                else {
                    break;
                };
                base = base.wrapping_add(start);
                ty = member_type;
            } else {
                break;
            }
        }
        found.ok_or_else(|| nonconstant(location, "pointer arithmetic outside an array"))
    }

    fn array_constant_subobject(
        &mut self,
        class: EntityId,
        offset: i64,
        location: SourceOffset,
    ) -> Result<Option<(TypeId, i64)>, Error> {
        if self.graph.entity(class).key != Keyword::Union {
            // Reuse the completed class's indexed ordinary-member spans and
            // base traversal. Bit-fields cannot be array-addressable subobjects.
            let next = self.ordered_constant_subobject(class, offset, location)?;
            return Ok(next
                .filter(|next| {
                    !matches!(
                        self.graph.types.get(next.ty).kind,
                        TypeKind::Reference { .. }
                    )
                })
                .map(|next| (next.ty, next.start)));
        }
        // Union alternatives retain declaration order at their shared offset.
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            if self.graph.entity(member).bit_field.is_some() {
                continue;
            }
            let ty = self.graph.lower_entity_type(member)?;
            if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                continue;
            }
            let start = integral::signed(
                self.graph
                    .entity(member)
                    .member_offset
                    .ok_or(Error::Invariant("array bounds member without offset"))?,
            );
            let bytes = self.address_element_size(ty, location)?.max(1);
            if offset >= start && offset < start.wrapping_add(bytes) {
                return Ok(Some((ty, start)));
            }
        }
        Ok(None)
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
    fn ordered_member_spans_are_reused_only_for_their_completed_class() {
        let source = SourceBuffer::new(
            br#"
            struct First;
            struct Second { int b, a; } second;
            struct First { int a, b; } first;
            static_assert(&first.a < &first.b, "first");
            static_assert(&second.b < &second.a, "second");
            static_assert(&first.a < &first.b, "reuse");
            int main(){return 0;}
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"ordered-spans").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mark = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        assert_eq!(analyzer.constexpr_frames.depth(), 0);
        let spans = &analyzer
            .constant_scratch
            .projections
            .as_ref()
            .unwrap()
            .member_spans
            .0;
        assert_eq!(spans.len(), 2);
        let mut keys = spans.keys();
        let first = &spans[keys.next().unwrap()];
        let second = &spans[keys.next().unwrap()];
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
        assert_ne!(first[0].member, second[0].member);
        assert_eq!(
            first.iter().map(|s| s.start).collect::<Vec<_>>(),
            vec![0, 4]
        );
        assert_eq!(
            second.iter().map(|s| s.start).collect::<Vec<_>>(),
            vec![0, 4]
        );
    }
}
