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
        use F::{Double, Float, LongDouble};
        for scalar in [LongDouble, Double, Float] {
            if [a, b].iter().any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Scalar(Scalar::Basic(f)) if f == scalar)) { return self.graph.types.scalar(Scalar::Basic(scalar)); }
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
        if matches!(
            self.graph.types.get(element).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return Err(self.operator_error(id, "arithmetic on a pointer to an incomplete type"));
        }
        self.type_size(element, self.tree.expressions.get(id).location)?;
        Ok(())
    }
    pub(super) fn scalar_unary(
        &mut self,
        id: ast::ExprId,
        op: P,
        operand: ast::ExprId,
        postfix: bool,
    ) -> Result<ExprFacts, Error> {
        let facts = self.graph.expression_facts.get(operand)?;
        if facts.functions.is_some() && matches!(op, P::BitAnd | P::Multiply) {
            let ty = if op == P::BitAnd {
                self.graph.types.pointer(facts.ty)?
            } else {
                facts.ty
            };
            let mut result = self.new_expression_facts(
                ty,
                if op == P::BitAnd {
                    V::Prvalue
                } else {
                    V::Lvalue
                },
            )?;
            result.functions = facts.functions;
            return Ok(result);
        }
        self.check_resolved_function(operand)?;
        let facts = self.graph.expression_facts.get(operand)?;
        if op == P::BitAnd {
            if facts.category != V::Lvalue
                && !matches!(
                    self.graph.types.get(facts.ty).kind,
                    TypeKind::Function { .. }
                )
            {
                return Err(self.operator_error(id, "taking the address of an rvalue"));
            }
            let ty = self.graph.types.pointer(facts.ty)?;
            return self.new_expression_facts(ty, V::Prvalue);
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
                || !(self.is_arithmetic_operand(facts.ty)
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
            return self.new_expression_facts(ty, if postfix { V::Prvalue } else { V::Lvalue });
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
            return self.new_expression_facts(element, V::Lvalue);
        }
        if op == P::LogicalNot {
            self.convert_bool(operand)?;
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            return self.new_expression_facts(ty, V::Prvalue);
        }
        let pointer_plus =
            op == P::Add && matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_));
        if !pointer_plus
            && (op == P::Complement && !self.is_integral_operand(ty)
                || matches!(op, P::Add | P::Subtract) && !self.is_arithmetic_operand(ty))
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
            self.promoted_type(ty)?
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
    ) -> Result<ExprFacts, Error> {
        self.check_resolved_function(left)?;
        self.check_resolved_function(right)?;
        let lf = self.graph.expression_facts.get(left)?;
        let rf = self.graph.expression_facts.get(right)?;
        if op == P::Comma {
            return self.new_expression_facts(rf.ty, rf.category);
        }
        if matches!(op, P::LogicalAnd | P::LogicalOr) {
            self.convert_bool(left)?;
            self.convert_bool(right)?;
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            return self.new_expression_facts(ty, V::Prvalue);
        }
        let a = self.decayed_type(lf.ty)?;
        let b = self.decayed_type(rf.ty)?;
        let compare = matches!(
            op,
            P::Equal | P::NotEqual | P::Less | P::Greater | P::LessEqual | P::GreaterEqual
        );
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
                    .composite_pointer(lf, rf)?
                    .ok_or_else(|| self.operator_error(id, "invalid operands to a comparison"))?;
                if !matches!(
                    self.graph.types.get(pointer).kind,
                    TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)
                ) {
                    return Err(self.operator_error(id, "invalid operands to a comparison"));
                }
                pointer
            };
            if !matches!(op, P::Equal | P::NotEqual)
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Scalar(Scalar::Nullptr)
                )
            {
                return Err(self.operator_error(id, "invalid operands to a comparison"));
            }
            self.convert_operand_scalar(left, ty, false)?;
            self.convert_operand_scalar(right, ty, false)?;
            let ty = self.graph.types.scalar(Scalar::Bool)?;
            return self.new_expression_facts(ty, V::Prvalue);
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
        if matches!(self.graph.types.get(a).kind, TypeKind::Scalar(Scalar::Void))
            || matches!(self.graph.types.get(b).kind, TypeKind::Scalar(Scalar::Void))
        {
            return Err(self.operator_error(id, "operand of type void"));
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
    ) -> Result<Option<TypeId>, Error> {
        let at = self.decayed_type(a.ty)?;
        let bt = self.decayed_type(b.ty)?;
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
            return Ok(matches!(self.graph.types.get(bt).kind, TypeKind::Pointer(_)).then_some(bt));
        }
        if b_null {
            return Ok(matches!(self.graph.types.get(at).kind, TypeKind::Pointer(_)).then_some(at));
        }
        if at == bt {
            return Ok(Some(at));
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
        if bare == self.bare_type(pb)? {
            let element = self.graph.types.qualify(bare, cv)?;
            return self.graph.types.pointer(element).map(Some);
        }
        if self.pointer_qualification(at, bt)? {
            return Ok(Some(bt));
        }
        if self.pointer_qualification(bt, at)? {
            return Ok(Some(at));
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
        while let (TypeKind::Pointer(ea), TypeKind::Pointer(eb)) = (
            self.graph.types.get(a).kind.clone(),
            self.graph.types.get(b).kind.clone(),
        ) {
            let ca = self.graph.types.get(ea).cv.0;
            let cb = self.graph.types.get(eb).cv.0;
            levels.push((ca | cb, ca != cb));
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
        for (cv, differs) in levels.into_iter().rev() {
            let cv = super::types::Cv(cv | u8::from(outer_const));
            let element = self.graph.types.qualify(result, cv)?;
            result = self.graph.types.pointer(element)?;
            outer_const |= differs;
        }
        Ok(Some(result))
    }
    pub(super) fn scalar_assignment(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: ast::ExprId,
        right: ast::ExprId,
    ) -> Result<ExprFacts, Error> {
        self.check_resolved_function(left)?;
        let lf = self.graph.expression_facts.get(left)?;
        if !self.modifiable(lf) {
            return Err(self.operator_error(
                id,
                "assignment to something that is not a modifiable lvalue",
            ));
        }
        let target = self.bare_type(lf.ty)?;
        if op == P::Assign {
            self.convert_scalar(right, target, false)?;
        } else {
            self.check_resolved_function(right)?;
            let rf = self.graph.expression_facts.get(right)?;
            let rt = self.decayed_type(rf.ty)?;
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
                if !self.is_arithmetic_operand(target)
                    || !self.is_arithmetic_operand(rt)
                    || integral_only
                        && (!self.is_integral_operand(target) || !self.is_integral_operand(rt))
                {
                    return Err(
                        self.operator_error(id, "invalid operands to a compound assignment")
                    );
                }
                let common = if matches!(op, P::ShiftLeftAssign | P::ShiftRightAssign) {
                    self.promoted_type(rt)?
                } else {
                    self.arithmetic_type(target, rt)?
                };
                self.convert_operand_scalar(right, common, false)?;
            }
        }
        self.new_expression_facts(lf.ty, V::Lvalue)
    }
    pub(super) fn scalar_conditional(
        &mut self,
        id: ast::ExprId,
        left: ast::ExprId,
        right: ast::ExprId,
        a: ExprFacts,
        b: ExprFacts,
    ) -> Result<ExprFacts, Error> {
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
            if va && vb {
                return self.new_expression_facts(at, V::Prvalue);
            }
            return Err(self.operator_error(id, "only one operand of ?: is void"));
        }
        if matches!(self.graph.types.get(at).kind, TypeKind::Class(_))
            || matches!(self.graph.types.get(bt).kind, TypeKind::Class(_))
        {
            return Err(Error::Missing("class conditional expressions"));
        }
        let ac = self.graph.types.get(a.ty).cv.0;
        let bc = self.graph.types.get(b.ty).cv.0;
        if a.category == b.category
            && a.category != V::Prvalue
            && at == bt
            && (ac & bc == ac.min(bc))
        {
            return self.new_expression_facts(if ac & !bc == 0 { b.ty } else { a.ty }, a.category);
        }
        let at = self.decayed_type(a.ty)?;
        let bt = self.decayed_type(b.ty)?;
        let ty = if at == bt {
            at
        } else if self.is_arithmetic_operand(at) && self.is_arithmetic_operand(bt) {
            self.arithmetic_type(at, bt)?
        } else {
            self.composite_pointer(a, b)?
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
        let a = self.graph.expression_facts.get(base)?;
        let b = self.graph.expression_facts.get(index)?;
        let at = self.decayed_type(a.ty)?;
        let bt = self.decayed_type(b.ty)?;
        let (pointer, integer, it, commuted) = if self.object_pointer(at).is_some() {
            (at, index, bt, false)
        } else {
            (bt, base, at, true)
        };
        if self.object_pointer(pointer).is_none() || !self.is_integral_operand(it) {
            return Err(self.operator_error(id, "invalid subscript operands"));
        }
        self.complete_pointer(pointer, id)?;
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
