// Translated from constant_interpreter.cpp and constant_expressions.cpp. See NOTICE.
//! Target byte-offset arithmetic, literal identity and integral uses of addresses.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, nonconstant},
    constants::Constant,
    graph::ScopeId,
    integral,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
};

impl Analyzer<'_> {
    pub(super) fn constant_truth(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        skipping: bool,
    ) -> Result<bool, Error> {
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
        Ok(value.bits != 0)
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
            Root::Ephemeral(expr) => Ok(self.graph.expression_facts.get(expr)?.ty),
            Root::Literal(id) => Ok(self.graph.expression_facts.get(id)?.ty),
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
        let (ExprKind::Literal(a), ExprKind::Literal(b)) = (
            self.tree.expressions.get(a).kind,
            self.tree.expressions.get(b).kind,
        ) else {
            return Err(Error::Invariant("literal address without literal syntax"));
        };
        Ok(a.kind == b.kind && self.literal_bytes.get(a.data) == self.literal_bytes.get(b.data))
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
        // Ordered scalar and array roots have one access-control domain. Class
        // member ordering is a distinct owner and must not be assumed from offsets.
        let mut ty = self.address_root_type(a, location)?;
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
            return Err(Error::Missing("constant class member pointer ordering"));
        }
        integral::comparison(op, true, a.offset.cast_unsigned(), b.offset.cast_unsigned()).ok_or(
            Error::Invariant("pointer comparison without comparison operator"),
        )
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
                let mut next = None;
                for index in 0..self.graph.entity(class).members.len() {
                    let member = self.graph.entity(class).members[index];
                    let member_type = self
                        .graph
                        .entity(member)
                        .ty
                        .ok_or(Error::Invariant("array bounds member without type"))?;
                    if matches!(
                        self.graph.types.get(member_type).kind,
                        TypeKind::Reference { .. }
                    ) {
                        continue;
                    }
                    let start = integral::signed(
                        self.graph
                            .entity(member)
                            .member_offset
                            .ok_or(Error::Invariant("array bounds member without offset"))?,
                    );
                    let bytes = self.address_element_size(member_type, location)?;
                    if offset >= start && offset < start.wrapping_add(bytes.max(1)) {
                        next = Some((member_type, start));
                        break;
                    }
                }
                let Some((member_type, start)) = next else {
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
}
