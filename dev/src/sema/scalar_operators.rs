// Translated from sema/analyzer_expressions.cpp and analyzer_conversions.cpp. See NOTICE.
//! Built-in scalar operators; no overload or whole-unit candidate search.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    types::{Scalar, TypeId, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast, types::FundamentalType as F};
impl Analyzer<'_> {
    fn operator_error(&self, id: ast::ExprId, message: &'static str) -> Error {
        Error::at(self.tree.expressions.get(id).location, message)
    }
    fn modifiable(&self, facts: ExprFacts) -> bool {
        facts.category == V::Lvalue
            && self.graph.types.get(facts.ty).cv.0 & 1 == 0
            && !matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Array { .. } | TypeKind::Function { .. }
            )
    }
    pub(super) fn arithmetic_type(&mut self, a: TypeId, b: TypeId) -> Result<TypeId, Error> {
        if self.complex_part_type(a).is_some() || self.complex_part_type(b).is_some() {
            let part = self.arithmetic_type(
                self.complex_part_type(a).unwrap_or(a),
                self.complex_part_type(b).unwrap_or(b),
            )?;
            return self
                .graph
                .types
                .intern(TypeKind::Complex(part), super::types::Cv::default());
        }
        let rank = |ty| match self.graph.types.get(ty).kind {
            TypeKind::Scalar(Scalar::Basic(f)) => f.floating_rank(),
            _ => 0,
        };
        let (ar, br) = (rank(a), rank(b));
        if ar != 0 || br != 0 {
            let standard = matches!(
                self.graph.types.get(a).kind,
                TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
            );
            return self.bare_type(if ar > br || ar == br && !standard {
                a
            } else {
                b
            });
        }
        self.common_integral_type(a, b)
    }
    fn object_pointer(&self, ty: TypeId) -> Option<TypeId> {
        if let TypeKind::Pointer(element) = self.graph.types.get(ty).kind
            && !matches!(
                self.graph.types.get(element).kind,
                TypeKind::Function { .. }
            )
        {
            Some(element)
        } else {
            None
        }
    }
    fn complete_pointer(&mut self, ty: TypeId, id: ast::ExprId) -> Result<(), Error> {
        let element = self
            .object_pointer(ty)
            .ok_or(Error::Invariant("nonobject pointer"))?;
        self.complete_pointee(element, self.tree.expressions.get(id).location)
    }
    pub(super) fn complete_pointee(
        &mut self,
        element: TypeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        if matches!(
            self.graph.types.get(element).kind,
            TypeKind::Scalar(Scalar::Void) | TypeKind::Array { bound: 0, .. }
        ) {
            return Err(Error::at(
                location,
                "arithmetic on a pointer to an incomplete type",
            ));
        }
        if let Some(class) = self.graph.class_of(element)
            && !self.complete_class(class)?
        {
            return Err(Error::at(
                location,
                "arithmetic on a pointer to an incomplete class",
            ));
        }
        Ok(())
    }
    pub(super) fn scalar_unary(
        &mut self,
        id: ast::ExprId,
        op: P,
        operand: ast::ExprId,
        postfix: bool,
    ) -> Result<ExprFacts, Error> {
        let facts = self.builtin_operand_facts(operand)?;
        if facts.object.is_some()
            && facts
                .functions
                .is_some_and(|set| self.graph.overloads.get(set).len() == 1)
        {
            self.check_bound_function(operand)?;
        }
        if facts.functions.is_some()
            && self.ordinary_function_operand(facts).is_none()
            && matches!(op, P::BitAnd | P::Multiply)
        {
            // Unresolved address/dereference syntax retains only the overload
            // set's source signature. Selection forms its actual pointer type.
            let mut result = self.new_expression_facts(
                facts.ty,
                if op == P::BitAnd {
                    V::Prvalue
                } else {
                    V::Lvalue
                },
            )?;
            result.functions = facts.functions;
            result.object = facts.object;
            return Ok(result);
        }
        self.check_resolved_function(operand)?;
        let facts = self.builtin_operand_facts(operand)?;
        if op == P::BitAnd {
            self.odr_use_constants(operand, None)?;
            if facts.is_bit_field() {
                return Err(self.operator_error(id, "taking the address of a bit-field"));
            }
            if facts.category != V::Lvalue
                && !matches!(
                    self.graph.types.get(facts.ty).kind,
                    TypeKind::Function { .. }
                )
            {
                return Err(self.operator_error(id, "taking the address of an rvalue"));
            }
            let ty = self.graph.types.pointer(facts.ty)?;
            let mut result = self.new_expression_facts(ty, V::Prvalue)?;
            result.function = facts.function;
            return Ok(result);
        }
        if matches!(op, P::Increment | P::Decrement) {
            if !self.modifiable(facts) {
                return Err(self.operator_error(
                    id,
                    "increment or decrement of something that is not a modifiable lvalue",
                ));
            }
            if matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Scalar(Scalar::Bool)
            ) && op == P::Decrement
                || !(self.is_arithmetic_type(facts.ty)
                    || self.complex_part_type(facts.ty).is_some()
                    || self.object_pointer(facts.ty).is_some())
            {
                return Err(self.operator_error(id, "invalid operand of increment or decrement"));
            }
            if self.object_pointer(facts.ty).is_some() {
                self.complete_pointer(facts.ty, id)?;
            }
            let ty = if postfix {
                self.bare_type(facts.ty)?
            } else {
                facts.ty
            };
            let mut result =
                self.new_expression_facts(ty, if postfix { V::Prvalue } else { V::Lvalue })?;
            result.set_bit_field(facts.is_bit_field() && !postfix);
            return Ok(result);
        }
        let ty = self.decayed_type(facts.ty)?;
        if op == P::Multiply {
            let TypeKind::Pointer(element) = self.graph.types.get(ty).kind else {
                return Err(self.operator_error(
                    id,
                    "indirection requires a pointer to an object or function type",
                ));
            };
            if matches!(
                self.graph.types.get(element).kind,
                TypeKind::Scalar(Scalar::Void)
            ) {
                return Err(self.operator_error(
                    id,
                    "indirection requires a pointer to an object or function type",
                ));
            }
            self.convert_operand_scalar(operand, ty, false)?;
            let mut result = self.new_expression_facts(element, V::Lvalue)?;
            result.function = facts.function;
            return Ok(result);
        }
        if op == P::LogicalNot {
            self.convert_bool(operand)?;
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            return self.new_expression_facts(ty, V::Prvalue);
        }
        if self.vector_shape(ty).is_some() {
            return self.vector_unary(id, op, operand, ty);
        }
        let pointer_plus =
            op == P::Add && matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_));
        if !pointer_plus
            && (op == P::Complement
                && !self.is_integral_operand(ty)
                && self.complex_part_type(ty).is_none()
                || matches!(op, P::Add | P::Subtract)
                    && !self.is_arithmetic_operand(ty)
                    && self.complex_part_type(ty).is_none())
        {
            return Err(self.operator_error(
                id,
                if op == P::Complement {
                    "~ requires an integral operand"
                } else {
                    "unary + or - requires an arithmetic operand"
                },
            ));
        }
        let result = if self.is_integral_operand(ty) {
            self.promoted_field_type(facts, ty)?
        } else {
            ty
        };
        self.convert_operand_scalar(operand, result, false)?;
        self.new_expression_facts(result, V::Prvalue)
    }
    pub(super) fn scalar_binary(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: ast::ExprId,
        right: ast::ExprId,
        scope: super::graph::ScopeId,
    ) -> Result<ExprFacts, Error> {
        if matches!(op, P::DotStar | P::ArrowStar) {
            return self.member_pointer_access(id, op, left, right, scope);
        }
        self.check_resolved_function(left)?;
        if matches!(op, P::LogicalAnd | P::LogicalOr) {
            self.convert_bool(left)?;
            self.check_resolved_function(right)?;
            self.convert_bool(right)?;
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            return self.new_expression_facts(ty, V::Prvalue);
        }
        self.check_resolved_function(right)?;
        let lf = self.builtin_operand_facts(left)?;
        let rf = self.builtin_operand_facts(right)?;
        if op == P::Comma {
            self.check_temporary_destruction(lf, scope, self.tree.expressions.get(left).location)?;
            let mut result = self.new_expression_facts(rf.ty, rf.category)?;
            result.set_bit_field(rf.is_bit_field());
            return Ok(result);
        }
        let a = self.decayed_type(lf.ty)?;
        let b = self.decayed_type(rf.ty)?;
        let a = self.field_type(lf, a)?;
        let b = self.field_type(rf, b)?;
        let compare = matches!(
            op,
            P::Equal | P::NotEqual | P::Less | P::Greater | P::LessEqual | P::GreaterEqual
        );
        if self.vector_shape(a).is_some() || self.vector_shape(b).is_some() {
            if !compare
                && (matches!(self.graph.types.get(a).kind, TypeKind::Scalar(Scalar::Void))
                    || matches!(self.graph.types.get(b).kind, TypeKind::Scalar(Scalar::Void)))
            {
                return Err(self.operator_error(id, "operand of type void"));
            }
            // The frozen arithmetic owner diagnoses pointer +/- before vectors;
            // vector checks precede complex arithmetic and comparison checks.
            if matches!(op, P::Add | P::Subtract)
                && (self.object_pointer(a).is_some() || self.object_pointer(b).is_some())
            {
                return Err(self.operator_error(id, "invalid operands to pointer arithmetic"));
            }
            return self.vector_binary(id, op, [left, right], [a, b], scope);
        }
        if self.complex_part_type(a).is_some() || self.complex_part_type(b).is_some() {
            return self.complex_binary(id, op, [left, right], [a, b]);
        }
        if compare {
            if (self.scoped_enum(a) || self.scoped_enum(b)) && a != b {
                return Err(
                    self.operator_error(id, "comparison of a scoped enumeration with another type")
                );
            }
            let ty = if self.is_arithmetic_operand(a) && self.is_arithmetic_operand(b) {
                self.arithmetic_type(a, b)?
            } else if self.scoped_enum(a) && a == b {
                a
            } else {
                let pointer = self
                    .composite_pointer(lf, rf, true)?
                    .ok_or_else(|| self.operator_error(id, "invalid operands to a comparison"))?;
                if !matches!(
                    self.graph.types.get(pointer).kind,
                    TypeKind::Pointer(_)
                        | TypeKind::MemberPointer { .. }
                        | TypeKind::Scalar(Scalar::Nullptr)
                ) {
                    return Err(self.operator_error(id, "invalid operands to a comparison"));
                }
                pointer
            };
            if !matches!(op, P::Equal | P::NotEqual)
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Scalar(Scalar::Nullptr) | TypeKind::MemberPointer { .. }
                )
            {
                return Err(self.operator_error(id, "invalid operands to a comparison"));
            }
            self.convert_operand_scalar(left, ty, false)?;
            self.convert_operand_scalar(right, ty, false)?;
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            return self.new_expression_facts(ty, V::Prvalue);
        }
        if matches!(self.graph.types.get(a).kind, TypeKind::Scalar(Scalar::Void))
            || matches!(self.graph.types.get(b).kind, TypeKind::Scalar(Scalar::Void))
        {
            return Err(self.operator_error(id, "operand of type void"));
        }
        let lp = self.object_pointer(a).is_some();
        let rp = self.object_pointer(b).is_some();
        if matches!(op, P::Add | P::Subtract) && (lp || rp) {
            if op == P::Subtract && lp && rp {
                let ea = self
                    .object_pointer(a)
                    .ok_or(Error::Invariant("missing pointee"))?;
                let eb = self
                    .object_pointer(b)
                    .ok_or(Error::Invariant("missing pointee"))?;
                if self.bare_type(ea)? != self.bare_type(eb)?
                    || matches!(
                        self.graph.types.get(ea).kind,
                        TypeKind::Scalar(Scalar::Void)
                    )
                {
                    return Err(
                        self.operator_error(id, "subtraction of pointers to different types")
                    );
                }
                self.convert_operand_scalar(left, a, false)?;
                self.convert_operand_scalar(right, b, false)?;
                let ty = self.graph.types.scalar(Scalar::Basic(F::LongInt))?;
                return self.new_expression_facts(ty, V::Prvalue);
            }
            let (pointer, index, index_ty) = if lp { (a, right, b) } else { (b, left, a) };
            if (lp || op == P::Subtract) && rp || !self.is_integral_operand(index_ty) {
                return Err(self.operator_error(id, "invalid operands to pointer arithmetic"));
            }
            self.complete_pointer(pointer, id)?;
            self.convert_operand_scalar(if lp { left } else { right }, pointer, false)?;
            let index_ty = self.promoted_type(index_ty)?;
            self.convert_operand_scalar(index, index_ty, false)?;
            return self.new_expression_facts(pointer, V::Prvalue);
        }
        let integral_only = matches!(
            op,
            P::Remainder | P::ShiftLeft | P::ShiftRight | P::BitAnd | P::BitOr | P::Xor
        );
        if !self.is_arithmetic_operand(a)
            || !self.is_arithmetic_operand(b)
            || integral_only && (!self.is_integral_operand(a) || !self.is_integral_operand(b))
        {
            return Err(self.operator_error(id, "invalid operands to a binary operator"));
        }
        let ty = if matches!(op, P::ShiftLeft | P::ShiftRight) {
            let ty = self.promoted_type(b)?;
            self.convert_operand_scalar(right, ty, false)?;
            self.promoted_type(a)?
        } else {
            let ty = self.arithmetic_type(a, b)?;
            self.convert_operand_scalar(right, ty, false)?;
            ty
        };
        self.convert_operand_scalar(left, ty, false)?;
        self.new_expression_facts(ty, V::Prvalue)
    }
    pub(super) fn composite_pointer(
        &mut self,
        a: ExprFacts,
        b: ExprFacts,
        comparison: bool,
    ) -> Result<Option<TypeId>, Error> {
        let mut at = self.decayed_type(a.ty)?;
        let mut bt = self.decayed_type(b.ty)?;
        let a_null = a.null_literal
            || matches!(
                self.graph.types.get(at).kind,
                TypeKind::Scalar(Scalar::Nullptr)
            );
        let b_null = b.null_literal
            || matches!(
                self.graph.types.get(bt).kind,
                TypeKind::Scalar(Scalar::Nullptr)
            );
        if a_null && b_null {
            return Ok(Some(
                if matches!(
                    self.graph.types.get(at).kind,
                    TypeKind::Scalar(Scalar::Nullptr)
                ) {
                    at
                } else {
                    bt
                },
            ));
        }
        if a_null {
            return Ok(self
                .graph
                .types
                .get(bt)
                .kind
                .pointer_level()
                .is_some()
                .then_some(bt));
        }
        if b_null {
            return Ok(self
                .graph
                .types
                .get(at)
                .kind
                .pointer_level()
                .is_some()
                .then_some(at));
        }
        if at == bt {
            return Ok(Some(at));
        }
        if matches!(
            self.graph.types.get(at).kind,
            TypeKind::MemberPointer { .. }
        ) && matches!(
            self.graph.types.get(bt).kind,
            TypeKind::MemberPointer { .. }
        ) {
            let TypeKind::MemberPointer {
                owner: a,
                element: ae,
            } = self.graph.types.get(at).kind
            else {
                return Err(Error::Invariant("member composite source"));
            };
            let TypeKind::MemberPointer {
                owner: b,
                element: be,
            } = self.graph.types.get(bt).kind
            else {
                return Err(Error::Invariant("member composite target"));
            };
            if a != b {
                if self.derived_class_conversion(a, b)? {
                    at = self.graph.types.member_pointer(b, ae)?;
                } else if self.derived_class_conversion(b, a)? {
                    bt = self.graph.types.member_pointer(a, be)?;
                } else {
                    return Ok(None);
                }
                if at == bt {
                    return Ok(Some(at));
                }
            }
            if self.pointer_qualification(at, bt)? {
                return Ok(Some(bt));
            }
            if self.pointer_qualification(bt, at)? {
                return Ok(Some(at));
            }
            return if comparison {
                self.combined_pointer_cv(at, bt)
            } else {
                Ok(None)
            };
        }
        let (TypeKind::Pointer(pa), TypeKind::Pointer(pb)) = (
            self.graph.types.get(at).kind.clone(),
            self.graph.types.get(bt).kind.clone(),
        ) else {
            return Ok(None);
        };
        let cv = super::types::Cv(self.graph.types.get(pa).cv.0 | self.graph.types.get(pb).cv.0);
        let va = matches!(
            self.graph.types.get(pa).kind,
            TypeKind::Scalar(Scalar::Void)
        );
        let vb = matches!(
            self.graph.types.get(pb).kind,
            TypeKind::Scalar(Scalar::Void)
        );
        if va || vb {
            if matches!(self.graph.types.get(pa).kind, TypeKind::Function { .. })
                || matches!(self.graph.types.get(pb).kind, TypeKind::Function { .. })
            {
                return Ok(None);
            }
            let void = self.graph.types.scalar(Scalar::Void)?;
            let element = self.graph.types.qualify(void, cv)?;
            return self.graph.types.pointer(element).map(Some);
        }
        let bare = self.bare_type(pa)?;
        let same_pointee = if matches!(self.graph.types.get(pa).kind, TypeKind::Array { .. }) {
            pa == pb
        } else {
            bare == self.bare_type(pb)?
        };
        if same_pointee {
            let element = self.graph.types.qualify(bare, cv)?;
            return self.graph.types.pointer(element).map(Some);
        }
        if self.pointer_qualification(at, bt)? {
            return Ok(Some(bt));
        }
        if self.pointer_qualification(bt, at)? {
            return Ok(Some(at));
        }
        if let (Some(a_class), Some(b_class)) = (self.graph.class_of(pa), self.graph.class_of(pb)) {
            let ancestor = if self.graph.is_base_of(a_class, b_class) {
                Some(pa)
            } else if self.graph.is_base_of(b_class, a_class) {
                Some(pb)
            } else {
                None
            };
            if let Some(ancestor) = ancestor {
                let ancestor = self.bare_type(ancestor)?;
                let element = self.graph.types.qualify(ancestor, cv)?;
                return self.graph.types.pointer(element).map(Some);
            }
        }
        self.combined_pointer_cv(at, bt)
    }
    fn combined_pointer_cv(
        &mut self,
        mut a: TypeId,
        mut b: TypeId,
    ) -> Result<Option<TypeId>, Error> {
        // CWG 1512: union each level's cv and add const above differing levels.
        let mut levels = Vec::new();
        while let (Some((oa, ea)), Some((ob, eb))) = (
            self.graph.types.get(a).kind.pointer_level(),
            self.graph.types.get(b).kind.pointer_level(),
        ) {
            if oa != ob {
                return Ok(None);
            }
            let ca = self.graph.types.get(ea).cv.0;
            let cb = self.graph.types.get(eb).cv.0;
            levels.push((oa, ca | cb, ca != cb));
            a = self.bare_type(ea)?;
            b = self.bare_type(eb)?;
        }
        if levels.is_empty()
            || a != b
            || matches!(self.graph.types.get(a).kind, TypeKind::Array { .. })
        {
            return Ok(None);
        }
        let mut result = a;
        let mut outer_const = false;
        for (owner, cv, differs) in levels.into_iter().rev() {
            let cv = super::types::Cv(cv | u8::from(outer_const));
            let element = self.graph.types.qualify(result, cv)?;
            result = if let Some(owner) = owner {
                self.graph.types.member_pointer(owner, element)?
            } else {
                self.graph.types.pointer(element)?
            };
            outer_const |= differs;
        }
        Ok(Some(result))
    }
    pub(super) fn scalar_list_assignment(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: ExprFacts,
        right: ast::ExprId,
        scope: super::graph::ScopeId,
    ) -> Result<ExprFacts, Error> {
        let target = self.bare_type(left.ty)?;
        let scalar = match self.graph.types.get(target).kind {
            TypeKind::Scalar(scalar) => scalar != Scalar::Void,
            TypeKind::Enum(_)
            | TypeKind::Pointer(_)
            | TypeKind::MemberPointer { .. }
            | TypeKind::Complex(_) => true,
            _ => false,
        };
        if op != P::Assign || !scalar {
            return Err(Error::at(
                self.tree.expressions.get(right).location,
                "assignment from a braced-init-list is not supported",
            ));
        }
        if !self.modifiable(left) {
            return Err(self.operator_error(
                id,
                "assignment to something that is not a modifiable lvalue",
            ));
        }
        self.list_initialize(right, target, scope)?;
        let mut result = self.new_expression_facts(left.ty, V::Lvalue)?;
        result.set_bit_field(left.is_bit_field());
        Ok(result)
    }

    pub(super) fn scalar_assignment(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: ast::ExprId,
        right: ast::ExprId,
    ) -> Result<ExprFacts, Error> {
        self.check_resolved_function(left)?;
        let lf = self.builtin_operand_facts(left)?;
        if !self.modifiable(lf) {
            return Err(self.operator_error(
                id,
                "assignment to something that is not a modifiable lvalue",
            ));
        }
        let target = self.bare_type(lf.ty)?;
        let mut operation = target;
        if op == P::Assign {
            self.convert_scalar(right, target, false)?;
        } else {
            self.check_resolved_function(right)?;
            let rf = self.builtin_operand_facts(right)?;
            let rt = self.decayed_type(rf.ty)?;
            if self.vector_shape(target).is_some() {
                self.vector_compound(id, op, right, target, rt)?;
                let right_type = self
                    .graph
                    .expression_facts
                    .get(right)?
                    .conversion
                    .ok_or(Error::Invariant("vector compound conversion"))?
                    .result;
                self.graph.record_arithmetic_types(id, target, right_type);
                return self.new_expression_facts(lf.ty, V::Lvalue);
            }
            if matches!(op, P::AddAssign | P::SubtractAssign)
                && self.object_pointer(target).is_some()
            {
                if !self.is_integral_operand(rt) {
                    return Err(
                        self.operator_error(id, "invalid operands to a compound assignment")
                    );
                }
                self.complete_pointer(target, id)?;
                let promoted = self.promoted_type(rt)?;
                self.convert_operand_scalar(right, promoted, false)?;
            } else if op == P::AddAssign
                && matches!(
                    self.graph.types.get(target).kind,
                    TypeKind::Scalar(Scalar::Bool)
                )
                && self.object_pointer(rt).is_some()
            {
                // The frozen owner's E1 = E1 + E2 rule admits bool plus an
                // object pointer, converting the pointer result back to bool.
                self.complete_pointer(rt, id)?;
                self.convert_operand_scalar(right, rt, false)?;
                operation = rt;
            } else {
                let integral_only = matches!(
                    op,
                    P::RemainderAssign
                        | P::ShiftLeftAssign
                        | P::ShiftRightAssign
                        | P::AndAssign
                        | P::OrAssign
                        | P::XorAssign
                );
                if !(self.is_arithmetic_type(target) || self.complex_part_type(target).is_some())
                    || !(self.is_arithmetic_operand(rt) || self.complex_part_type(rt).is_some())
                    || integral_only
                        && (!self.is_integral_operand(target) || !self.is_integral_operand(rt))
                {
                    return Err(
                        self.operator_error(id, "invalid operands to a compound assignment")
                    );
                }
                if self.complex_part_type(target).is_some() {
                    let common = self.arithmetic_type(target, rt)?;
                    let component = self
                        .complex_part_type(common)
                        .ok_or(Error::Invariant("complex compound type"))?;
                    self.convert_operand_scalar(
                        right,
                        if self.complex_part_type(rt).is_some() {
                            common
                        } else {
                            component
                        },
                        false,
                    )?;
                    let right_type = self
                        .graph
                        .expression_facts
                        .get(right)?
                        .conversion
                        .ok_or(Error::Invariant("complex compound conversion"))?
                        .result;
                    self.graph.record_arithmetic_types(id, common, right_type);
                    return self.new_expression_facts(lf.ty, V::Lvalue);
                }
                let promoted_left = self.field_type(lf, target)?;
                let common = if matches!(op, P::ShiftLeftAssign | P::ShiftRightAssign) {
                    self.promoted_type(rt)?
                } else {
                    let promoted_right = self.field_type(rf, rt)?;
                    self.arithmetic_type(promoted_left, promoted_right)?
                };
                self.convert_operand_scalar(right, common, false)?;
                operation = if matches!(op, P::ShiftLeftAssign | P::ShiftRightAssign) {
                    self.promoted_type(promoted_left)?
                } else {
                    common
                };
            }
        }
        if op != P::Assign {
            let right_type = self
                .graph
                .expression_facts
                .get(right)?
                .conversion
                .ok_or(Error::Invariant(
                    "compound assignment without right conversion",
                ))?
                .result;
            self.graph
                .record_arithmetic_types(id, operation, right_type);
        }
        let mut result = self.new_expression_facts(lf.ty, V::Lvalue)?;
        result.set_bit_field(lf.is_bit_field());
        Ok(result)
    }
    pub(super) fn scalar_conditional(
        &mut self,
        id: ast::ExprId,
        operands: [ast::ExprId; 2],
        facts: &[ExprFacts; 2],
        scope: super::graph::ScopeId,
    ) -> Result<ExprFacts, Error> {
        let [left, right] = operands;
        let [a, b] = *facts;
        let at = self.bare_type(a.ty)?;
        let bt = self.bare_type(b.ty)?;
        let va = matches!(
            self.graph.types.get(at).kind,
            TypeKind::Scalar(Scalar::Void)
        );
        let vb = matches!(
            self.graph.types.get(bt).kind,
            TypeKind::Scalar(Scalar::Void)
        );
        if va || vb {
            let throws = operands.map(|operand| {
                matches!(
                    self.tree
                        .expressions
                        .get(self.graph.expression_facts.root(operand))
                        .kind,
                    ast::ExprKind::Throw(_)
                )
            });
            if throws[0] != throws[1] && !(if throws[0] { vb } else { va }) {
                let other = usize::from(throws[0]);
                let operand = operands[other];
                let facts = facts[other];
                if facts.category != V::Prvalue
                    && !matches!(
                        self.graph.types.get(facts.ty).kind,
                        TypeKind::Array { .. } | TypeKind::Function { .. }
                    )
                {
                    let mut result = self.new_expression_facts(facts.ty, facts.category)?;
                    result.set_bit_field(facts.is_bit_field());
                    return Ok(result);
                }
                let ty = self.decayed_type(facts.ty)?;
                if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                    self.record_class_copy(
                        operand,
                        ty,
                        scope,
                        super::copy_initialization::CopyUse::Value { in_place: true },
                        self.tree.expressions.get(operand).location,
                    )?;
                } else {
                    self.convert_operand_scalar(operand, ty, false)?;
                }
                return self.new_expression_facts(ty, V::Prvalue);
            }
            if va && vb {
                return self.new_expression_facts(at, V::Prvalue);
            }
            return Err(self.operator_error(id, "only one operand of ?: is void"));
        }
        let ac = self.graph.types.get(a.ty).cv.0;
        let bc = self.graph.types.get(b.ty).cv.0;
        // The C++ conditional owner compares shallow unqualified identities:
        // unlike reference-relatedness, an array retains its element cv here.
        let same_glvalue_type = if matches!(self.graph.types.get(a.ty).kind, TypeKind::Array { .. })
        {
            a.ty == b.ty
        } else {
            at == bt
        };
        if a.category == b.category
            && a.category != V::Prvalue
            && same_glvalue_type
            && (ac & bc == ac.min(bc))
        {
            let mut result =
                self.new_expression_facts(if ac & !bc == 0 { b.ty } else { a.ty }, a.category)?;
            result.set_bit_field(a.is_bit_field() || b.is_bit_field());
            return Ok(result);
        }
        if matches!(self.graph.types.get(at).kind, TypeKind::Class(_))
            || matches!(self.graph.types.get(bt).kind, TypeKind::Class(_))
        {
            return self.class_conditional(id, operands, facts, scope);
        }
        let at = self.decayed_type(a.ty)?;
        let bt = self.decayed_type(b.ty)?;
        let ty = if at == bt {
            at
        } else if (self.is_arithmetic_operand(at) || self.complex_part_type(at).is_some())
            && (self.is_arithmetic_operand(bt) || self.complex_part_type(bt).is_some())
        {
            self.arithmetic_type(at, bt)?
        } else {
            self.composite_pointer(a, b, false)?
                .ok_or_else(|| self.operator_error(id, "incompatible operands of ?:"))?
        };
        self.convert_operand_scalar(left, ty, false)?;
        self.convert_operand_scalar(right, ty, false)?;
        self.new_expression_facts(ty, V::Prvalue)
    }
    pub(super) fn scalar_subscript(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        index: ast::ExprId,
    ) -> Result<ExprFacts, Error> {
        self.check_resolved_function(base)?;
        self.check_resolved_function(index)?;
        let a = self.builtin_operand_facts(base)?;
        let b = self.builtin_operand_facts(index)?;
        let at = self.decayed_type(a.ty)?;
        let bt = self.decayed_type(b.ty)?;
        if self.vector_shape(a.ty).is_some() && self.is_integral_operand(bt) {
            return self.vector_subscript(a, index, bt);
        }
        let (pointer, integer, it, commuted) = if self.object_pointer(at).is_some() {
            (at, index, bt, false)
        } else {
            (bt, base, at, true)
        };
        if self.object_pointer(pointer).is_none_or(|element| {
            matches!(
                self.graph.types.get(element).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        }) || !self.is_integral_operand(it)
        {
            return Err(self.operator_error(id, "invalid subscript operands"));
        }
        self.convert_operand_scalar(if commuted { index } else { base }, pointer, false)?;
        let promoted = self.promoted_type(it)?;
        self.convert_operand_scalar(integer, promoted, false)?;
        let element = self
            .object_pointer(pointer)
            .ok_or(Error::Invariant("subscript without pointee"))?;
        let mut facts = self.new_expression_facts(element, V::Lvalue)?;
        facts.commuted = commuted;
        Ok(facts)
    }
}
