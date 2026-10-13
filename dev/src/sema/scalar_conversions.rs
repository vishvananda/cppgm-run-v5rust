// Translated from sema/conversions.cpp and analyzer_conversions.cpp. See NOTICE.
//! Standard scalar conversions and reference bindings, recorded on operands.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, LvalueTransform as L, ValueCategory as V,
        ValueConversion as C,
    },
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::syntax::ast;
impl Analyzer<'_> {
    pub(super) fn bare_type(&mut self, ty: TypeId) -> Result<TypeId, Error> {
        let node = self.graph.types.get(ty);
        if node.cv.0 == 0 {
            return Ok(ty);
        }
        let kind = node.kind.clone();
        self.graph.types.intern(kind, Cv::default())
    }
    pub(super) fn decayed_type(&mut self, ty: TypeId) -> Result<TypeId, Error> {
        match self.graph.types.get(ty).kind {
            TypeKind::Array { element, .. } => self.graph.types.pointer(element),
            TypeKind::Function { .. } => self.graph.types.pointer(ty),
            _ => self.bare_type(ty),
        }
    }
    pub(super) fn is_integral_operand(&self, ty: TypeId) -> bool {
        self.integral_kind(ty).is_some() && !self.scoped_enum(ty)
    }
    pub(super) fn is_arithmetic_operand(&self, ty: TypeId) -> bool {
        self.is_integral_operand(ty)
            || matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Basic(
                    crate::types::FundamentalType::Float
                        | crate::types::FundamentalType::Double
                        | crate::types::FundamentalType::LongDouble
                ))
            )
    }
    pub(super) fn standard_conversion(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<Conversion>, Error> {
        let source = self.decayed_type(facts.ty)?;
        let goal = self.bare_type(target)?;
        let first = match self.graph.types.get(facts.ty).kind {
            TypeKind::Array { .. } => L::ArrayToPointer,
            TypeKind::Function { .. } => L::FunctionToPointer,
            _ if facts.category != V::Prvalue => L::Read,
            _ => L::None,
        };
        let qualification = self.pointer_qualification(source, goal)?;
        let promotion = self.is_integral_operand(source)
            && self.is_integral_operand(goal)
            && source != goal
            && self.promoted_type(source)? == goal;
        let sk = &self.graph.types.get(source).kind;
        let gk = &self.graph.types.get(goal).kind;
        let second = if source == goal || qualification {
            C::None
        } else if promotion {
            C::IntegralPromotion
        } else if matches!(
            sk,
            TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Float))
        ) && matches!(
            gk,
            TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Double))
        ) {
            C::FloatingPromotion
        } else if matches!(gk, TypeKind::Scalar(Scalar::Bool)) {
            if self.is_arithmetic_operand(source)
                || matches!(sk, TypeKind::Pointer(_))
                || direct && matches!(sk, TypeKind::Scalar(Scalar::Nullptr))
            {
                if matches!(sk, TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)) {
                    C::PointerBoolean
                } else {
                    C::Boolean
                }
            } else {
                return Ok(None);
            }
        } else if self.is_arithmetic_operand(source) && self.is_arithmetic_operand(goal) {
            C::Arithmetic
        } else if (facts.null_literal || matches!(sk, TypeKind::Scalar(Scalar::Nullptr)))
            && (matches!(gk, TypeKind::Pointer(_))
                || facts.null_literal && matches!(gk, TypeKind::Scalar(Scalar::Nullptr)))
        {
            C::NullPointer
        } else if let (TypeKind::Pointer(a), TypeKind::Pointer(b)) = (sk, gk) {
            if matches!(
                self.graph.types.get(*b).kind,
                TypeKind::Scalar(Scalar::Void)
            ) && !matches!(self.graph.types.get(*a).kind, TypeKind::Function { .. })
                && self.graph.types.get(*a).cv.0 & !self.graph.types.get(*b).cv.0 == 0
            {
                C::Pointer
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        };
        Ok(Some(Conversion {
            target,
            result: goal,
            first,
            second,
            rank: match second {
                C::None => R::Exact,
                C::IntegralPromotion | C::FloatingPromotion => R::Promotion,
                _ => R::Conversion,
            },
            qualification,
            initialization: true,
            temporary: false,
        }))
    }
    pub(super) fn pointer_qualification(
        &self,
        mut a: TypeId,
        mut b: TypeId,
    ) -> Result<bool, Error> {
        let mut const_so_far = true;
        let mut level = 0;
        loop {
            let (TypeKind::Pointer(ea), TypeKind::Pointer(eb)) =
                (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
            else {
                return Ok(false);
            };
            let ca = self.graph.types.get(*ea).cv.0;
            let cb = self.graph.types.get(*eb).cv.0;
            if ca & !cb != 0 || ca != cb && !const_so_far {
                return Ok(false);
            }
            const_so_far &= cb & 1 != 0;
            level += 1;
            a = *ea;
            b = *eb;
            let ak = &self.graph.types.get(a).kind;
            let bk = &self.graph.types.get(b).kind;
            if !matches!(ak, TypeKind::Pointer(_)) || !matches!(bk, TypeKind::Pointer(_)) {
                if matches!(ak, TypeKind::Array { .. }) || matches!(bk, TypeKind::Array { .. }) {
                    return Err(Error::Missing("pointer-to-array qualification conversions"));
                }
                return Ok(level != 0 && ak == bk);
            }
        }
    }
    pub(super) fn convert_scalar(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        direct: bool,
    ) -> Result<(), Error> {
        let facts = self.graph.expression_facts.get(id)?;
        if matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "invalid implicit conversion",
            ));
        }
        let conversion = if let TypeKind::Reference { element, kind } =
            self.graph.types.get(target).kind
        {
            let related = self.bare_type(element)? == self.bare_type(facts.ty)?;
            let compatible = related
                && self.graph.types.get(facts.ty).cv.0 & !self.graph.types.get(element).cv.0 == 0;
            let function = matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Function { .. }
            );
            let lvalue = kind == RefKind::Lvalue;
            let constant = self.graph.types.get(element).cv == Cv::CONST;
            let reference_qualification = if !related
                && lvalue
                && facts.category == V::Lvalue
                && matches!(self.graph.types.get(facts.ty).kind, TypeKind::Pointer(_))
                && matches!(self.graph.types.get(element).kind, TypeKind::Pointer(_))
            {
                let from = self.graph.types.pointer(facts.ty)?;
                let to = self.graph.types.pointer(element)?;
                self.pointer_qualification(from, to)?
            } else {
                false
            };
            if reference_qualification
                || compatible
                    && (lvalue && facts.category == V::Lvalue
                        || facts.category == V::Xvalue
                        || function
                        || facts.category == V::Prvalue && (constant || !lvalue))
            {
                Some(Conversion {
                    target,
                    result: element,
                    first: L::None,
                    second: C::None,
                    rank: R::Exact,
                    qualification: reference_qualification,
                    initialization: true,
                    temporary: facts.category == V::Prvalue && !function,
                })
            } else if lvalue && !constant
                || related
                    && (kind == RefKind::Rvalue && facts.category == V::Lvalue || !compatible)
            {
                None
            } else {
                let mut conversion = self.standard_conversion(facts, element, false)?;
                if let Some(c) = &mut conversion {
                    c.target = target;
                    c.result = if c.second == C::NullPointer && facts.null_literal {
                        self.bare_type(element)?
                    } else {
                        element
                    };
                    c.temporary = true;
                }
                conversion
            }
        } else {
            self.standard_conversion(facts, target, direct)?
        };
        let Some(conversion) = conversion else {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "invalid implicit conversion",
            ));
        };
        let mut facts = facts;
        facts.conversion = Some(conversion);
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }
    pub(super) fn convert_operand_scalar(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        direct: bool,
    ) -> Result<(), Error> {
        self.convert_scalar(id, target, direct)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        facts
            .conversion
            .as_mut()
            .ok_or(Error::Invariant("missing operand conversion"))?
            .initialization = false;
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }
    pub(super) fn convert_bool(&mut self, id: ast::ExprId) -> Result<(), Error> {
        let target = self.graph.types.scalar(Scalar::Bool)?;
        self.convert_operand_scalar(id, target, true)
    }
    pub(super) fn scalar_initializer(
        &mut self,
        initializer: ast::Initializer,
        scope: super::graph::ScopeId,
        target: TypeId,
    ) -> Result<(), Error> {
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Class(_) | TypeKind::Array { .. }
        ) {
            return Err(Error::Missing("class and array semantic initialization"));
        }
        let (id, direct) = match initializer.kind {
            ast::InitializerKind::Assign(id) => (id, false),
            ast::InitializerKind::Parenthesized(arguments) => {
                let arguments = self.tree.expr_lists.get(arguments);
                if arguments.len() != 1 {
                    return Err(Error::Missing("scalar value initialization"));
                }
                (arguments[0], true)
            }
            ast::InitializerKind::Braced(_) => {
                return Err(Error::Missing("scalar list initialization"));
            }
            ast::InitializerKind::Default | ast::InitializerKind::Delete => {
                return Err(Error::Invariant("nonobject initializer"));
            }
        };
        self.typed_expression(id, scope)?;
        self.convert_scalar(id, target, direct)
    }
}
