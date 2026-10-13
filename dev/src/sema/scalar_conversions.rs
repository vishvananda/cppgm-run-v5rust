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
    pub(super) fn type_cv(&self, mut ty: TypeId) -> Cv {
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        self.graph.types.get(ty).cv
    }
    pub(super) fn bare_type(&mut self, mut ty: TypeId) -> Result<TypeId, Error> {
        let mut bounds = Vec::new();
        while let TypeKind::Array { element, bound } = self.graph.types.get(ty).kind {
            bounds.push(bound);
            ty = element;
        }
        if self.graph.types.get(ty).cv.0 != 0 {
            let kind = self.graph.types.get(ty).kind.clone();
            ty = self.graph.types.intern(kind, Cv::default())?;
        }
        for bound in bounds.into_iter().rev() {
            ty = self
                .graph
                .types
                .intern(TypeKind::Array { element: ty, bound }, Cv::default())?;
        }
        Ok(ty)
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
    pub(super) fn is_arithmetic_type(&self, ty: TypeId) -> bool {
        !matches!(self.graph.types.get(ty).kind, TypeKind::Enum(_))
            && self.is_arithmetic_operand(ty)
    }
    pub(super) fn standard_conversion(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<Conversion>, Error> {
        let target = self.graph.types.canonical_function(target)?;
        let source = self.decayed_type(facts.ty)?;
        let goal = self.bare_type(target)?;
        let first = match self.graph.types.get(facts.ty).kind {
            TypeKind::Array { .. } => L::ArrayToPointer,
            TypeKind::Function { .. } => L::FunctionToPointer,
            _ if facts.category != V::Prvalue => L::Read,
            _ => L::None,
        };
        let mut qualification = source != goal && self.pointer_qualification(source, goal)?;
        let promotion = self.is_integral_operand(source)
            && self.is_arithmetic_type(goal)
            && source != goal
            && (self.promoted_type(source)? == goal
                || matches!(self.graph.types.get(source).kind, TypeKind::Enum(entity)
                    if self.graph.entity(entity).enumeration.is_some_and(|info|
                        info.fixed && info.underlying == Some(goal))));
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
                || sk.pointer_level().is_some()
                || direct && matches!(sk, TypeKind::Scalar(Scalar::Nullptr))
            {
                if sk.pointer_level().is_some() || matches!(sk, TypeKind::Scalar(Scalar::Nullptr)) {
                    C::PointerBoolean
                } else {
                    C::Boolean
                }
            } else {
                return Ok(None);
            }
        } else if self.is_arithmetic_operand(source) && self.is_arithmetic_type(goal) {
            C::Arithmetic
        } else if (facts.null_literal || matches!(sk, TypeKind::Scalar(Scalar::Nullptr)))
            && (gk.pointer_level().is_some()
                || facts.null_literal && matches!(gk, TypeKind::Scalar(Scalar::Nullptr)))
        {
            C::NullPointer
        } else if let (TypeKind::Pointer(a), TypeKind::Pointer(b)) = (sk, gk) {
            if matches!(
                self.graph.types.get(*b).kind,
                TypeKind::Scalar(Scalar::Void)
            ) && !matches!(self.graph.types.get(*a).kind, TypeKind::Function { .. })
                && self.type_cv(*a).0 & !self.type_cv(*b).0 == 0
            {
                qualification = self.type_cv(*a) != self.type_cv(*b);
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
        loop {
            let (Some((oa, ea)), Some((ob, eb))) = (
                self.graph.types.get(a).kind.pointer_level(),
                self.graph.types.get(b).kind.pointer_level(),
            ) else {
                return Ok(false);
            };
            if oa != ob {
                return Ok(false);
            }
            let ca = self.type_cv(ea).0;
            let cb = self.type_cv(eb).0;
            if ca & !cb != 0 || ca != cb && !const_so_far {
                return Ok(false);
            }
            const_so_far &= cb & 1 != 0;
            a = ea;
            b = eb;
            let ak = &self.graph.types.get(a).kind;
            let bk = &self.graph.types.get(b).kind;
            if ak.pointer_level().is_none() || bk.pointer_level().is_none() {
                if matches!(ak, TypeKind::Array { .. }) || matches!(bk, TypeKind::Array { .. }) {
                    return Ok(self.same_array_shape(a, b));
                }
                return Ok(ak == bk);
            }
        }
    }
    pub(super) fn same_array_shape(&self, mut a: TypeId, mut b: TypeId) -> bool {
        while let (
            TypeKind::Array {
                element: ae,
                bound: ab,
            },
            TypeKind::Array {
                element: be,
                bound: bb,
            },
        ) = (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
        {
            if ab != bb {
                return false;
            }
            a = *ae;
            b = *be;
        }
        self.graph.types.get(a).kind == self.graph.types.get(b).kind
    }
    pub(super) fn scalar_sequence(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<Conversion>, Error> {
        if matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return Ok(None);
        }
        let target = self.graph.types.canonical_function(target)?;
        let conversion = if let TypeKind::Reference { element, kind } =
            self.graph.types.get(target).kind
        {
            let related = self.bare_type(element)? == self.bare_type(facts.ty)?;
            let compatible = related && self.type_cv(facts.ty).0 & !self.type_cv(element).0 == 0;
            let function = matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Function { .. }
            );
            let lvalue = kind == RefKind::Lvalue;
            let constant = self.type_cv(element) == Cv::CONST;
            let reference_qualification = if !related
                && lvalue
                && facts.category == V::Lvalue
                && self
                    .graph
                    .types
                    .get(facts.ty)
                    .kind
                    .pointer_level()
                    .is_some()
                && self.graph.types.get(element).kind.pointer_level().is_some()
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
                        || (!lvalue || constant) && (facts.category == V::Xvalue || function)
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
                    temporary: facts.category == V::Prvalue
                        && !function
                        && !matches!(self.graph.types.get(facts.ty).kind, TypeKind::Class(_)),
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
        Ok(conversion)
    }
    pub(super) fn convert_scalar(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        direct: bool,
    ) -> Result<(), Error> {
        let facts = self.graph.expression_facts.get(id)?;
        if facts.functions.is_some() {
            if let Some(binding) = self.matching_function(facts, target)? {
                self.select_function_designation(id, binding)?;
            } else {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "no overloaded function matches the target type",
                ));
            }
        }
        let facts = self.graph.expression_facts.get(id)?;
        let conversion = self.scalar_sequence(facts, target, direct)?;
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
}
