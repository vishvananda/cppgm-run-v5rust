// Translated from sema/conversions.cpp and analyzer_conversions.cpp. See NOTICE.
//! Standard scalar conversions and reference bindings, recorded on operands.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, LvalueTransform as L, ReferenceSource,
        ValueCategory as V, ValueConversion as C,
    },
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::syntax::ast;
#[derive(Clone, Copy, PartialEq, Eq)]
enum SequenceContext {
    Ordinary,
    Arithmetic,
}

impl Analyzer<'_> {
    pub(super) fn record_conversion(&mut self, facts: &mut ExprFacts, conversion: Conversion) {
        self.graph.work.totals.conversions += 1;
        facts.conversion = Some(conversion);
    }

    pub(super) fn type_cv(&self, mut ty: TypeId) -> Cv {
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        self.graph.types.get(ty).cv
    }
    pub(super) fn bare_type(&mut self, mut ty: TypeId) -> Result<TypeId, Error> {
        let mut bounds = Vec::new();
        while let TypeKind::Array {
            element,
            bound,
            operand,
        } = self.graph.types.get(ty).kind
        {
            bounds.push((bound, operand));
            ty = element;
        }
        if self.graph.types.get(ty).cv.0 != 0 {
            let kind = self.graph.types.get(ty).kind.clone();
            ty = self.graph.types.intern(kind, Cv::default())?;
        }
        for (bound, operand) in bounds.into_iter().rev() {
            ty = self.graph.types.intern(
                TypeKind::Array {
                    element: ty,
                    bound,
                    operand,
                },
                Cv::default(),
            )?;
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
                TypeKind::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0
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
        self.standard_conversion_in(facts, target, direct, SequenceContext::Ordinary)
    }
    fn standard_conversion_in(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
        direct: bool,
        context: SequenceContext,
    ) -> Result<Option<Conversion>, Error> {
        let target = self.graph.types.canonical_function(target)?;
        let source = self.decayed_type(facts.ty)?;
        let goal = self.bare_type(target)?;
        // 13.3.3.1p6: a class value has an identity or derived-to-base
        // standard sequence. Ranking this sequence does not select or demand
        // a copy constructor; the chosen initialization does that separately.
        if self.graph.class_of(source).is_some() || self.graph.class_of(goal).is_some() {
            let second = if source == goal {
                C::None
            } else if self.derived_class_conversion(goal, source)? {
                C::DerivedToBase
            } else {
                return Ok(None);
            };
            return Ok(Some(Conversion {
                target,
                result: goal,
                first: L::None,
                second,
                rank: if second == C::None {
                    R::Exact
                } else {
                    R::Conversion
                },
                qualification: false,
                initialization: true,
                list_initialization: crate::sema::expression_facts::ListInitialization::Ordinary,
                temporary: false,
                reference_source: ReferenceSource::Other,
            }));
        }
        let first = match self.graph.types.get(facts.ty).kind {
            TypeKind::Array { .. } => L::ArrayToPointer,
            TypeKind::Function { .. } => L::FunctionToPointer,
            _ if facts.category != V::Prvalue => L::Read,
            _ => L::None,
        };
        let mut qualification = source != goal && self.pointer_qualification(source, goal)?;
        let pointer_pair = match (
            &self.graph.types.get(source).kind,
            &self.graph.types.get(goal).kind,
        ) {
            (TypeKind::Pointer(a), TypeKind::Pointer(b)) => Some((*a, *b)),
            _ => None,
        };
        let derived_pointer = if let Some((a, b)) = pointer_pair {
            self.derived_class_conversion(b, a)?
        } else {
            false
        };
        let promotion = self.is_integral_operand(source)
            && self.is_arithmetic_type(goal)
            && source != goal
            && (self.promoted_field_type(facts, source)? == goal
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
                || self.complex_part_type(source).is_some()
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
        } else if self.complex_part_type(goal).is_some()
            && (self.is_arithmetic_operand(source) || self.complex_part_type(source).is_some())
        {
            C::Complex
        } else if self.is_arithmetic_operand(source) && self.is_arithmetic_type(goal) {
            use crate::types::FundamentalType as F;
            if context == SequenceContext::Ordinary
                && matches!(
                    (sk, gk),
                    (
                        TypeKind::Scalar(Scalar::Basic(F::BFloat16)),
                        TypeKind::Scalar(Scalar::Basic(F::Float16))
                    ) | (
                        TypeKind::Scalar(Scalar::Basic(F::Float16)),
                        TypeKind::Scalar(Scalar::Basic(F::BFloat16))
                    )
                )
            {
                return Ok(None);
            }
            C::Arithmetic
        } else if (facts.null_literal() || matches!(sk, TypeKind::Scalar(Scalar::Nullptr)))
            && (gk.pointer_level().is_some()
                || facts.null_literal() && matches!(gk, TypeKind::Scalar(Scalar::Nullptr)))
        {
            C::NullPointer
        } else if let (
            TypeKind::MemberPointer {
                owner: a,
                element: ae,
            },
            TypeKind::MemberPointer {
                owner: b,
                element: _,
            },
        ) = (sk, gk)
        {
            let (a, b, ae) = (*a, *b, *ae);
            let adjusted = self.graph.types.member_pointer(b, ae)?;
            if self.derived_class_conversion(a, b)?
                && (adjusted == goal || self.pointer_qualification(adjusted, goal)?)
            {
                qualification = adjusted != goal;
                C::MemberToDerived
            } else {
                return Ok(None);
            }
        } else if let (TypeKind::Pointer(a), TypeKind::Pointer(b)) = (sk, gk) {
            if derived_pointer && self.type_cv(*a).0 & !self.type_cv(*b).0 == 0 {
                qualification = self.type_cv(*a) != self.type_cv(*b);
                C::DerivedToBase
            } else if matches!(
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
                C::Complex if self.complex_part_type(source).is_none() => R::ComplexReal,
                C::Complex if self.complex_promotion(source, goal)? => R::Promotion,
                _ => R::Conversion,
            },
            qualification,
            initialization: true,
            list_initialization: crate::sema::expression_facts::ListInitialization::Ordinary,
            temporary: false,
            reference_source: ReferenceSource::Other,
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
                ..
            },
            TypeKind::Array {
                element: be,
                bound: bb,
                ..
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
        self.scalar_sequence_in(facts, target, direct, SequenceContext::Ordinary)
    }
    fn scalar_sequence_in(
        &mut self,
        facts: ExprFacts,
        target: TypeId,
        direct: bool,
        context: SequenceContext,
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
            let derived = self.derived_class_conversion(element, facts.ty)?;
            let related = derived || self.bare_type(element)? == self.bare_type(facts.ty)?;
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
                    && (lvalue
                        && facts.category == V::Lvalue
                        && (!facts.is_bit_field() || !constant)
                        || (!lvalue || constant)
                            && (facts.category == V::Xvalue || function)
                            && !facts.is_bit_field()
                        || facts.category == V::Prvalue && (constant || !lvalue))
            {
                Some(Conversion {
                    target,
                    result: element,
                    first: L::None,
                    second: if derived { C::DerivedToBase } else { C::None },
                    rank: if derived { R::Conversion } else { R::Exact },
                    qualification: reference_qualification,
                    initialization: true,
                    list_initialization:
                        crate::sema::expression_facts::ListInitialization::Ordinary,
                    temporary: facts.category == V::Prvalue
                        && !function
                        && !matches!(self.graph.types.get(facts.ty).kind, TypeKind::Class(_)),
                    reference_source: if function && facts.category == V::Lvalue {
                        ReferenceSource::FunctionLvalue
                    } else {
                        ReferenceSource::Other
                    },
                })
            } else if lvalue && !constant
                || related
                    && (kind == RefKind::Rvalue && facts.category == V::Lvalue || !compatible)
            {
                None
            } else {
                let mut conversion = self.standard_conversion_in(facts, element, false, context)?;
                if let Some(c) = &mut conversion {
                    c.target = target;
                    c.result = if c.second == C::NullPointer && facts.null_literal() {
                        self.bare_type(element)?
                    } else {
                        element
                    };
                    c.temporary = true;
                    c.reference_source = if function && facts.category == V::Lvalue {
                        ReferenceSource::FunctionLvalue
                    } else {
                        ReferenceSource::Other
                    };
                }
                conversion
            }
        } else {
            self.standard_conversion_in(facts, target, direct, context)?
        };
        Ok(conversion)
    }
    pub(super) fn convert_scalar(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        direct: bool,
    ) -> Result<(), Error> {
        self.convert_scalar_in(id, target, direct, SequenceContext::Ordinary)
    }
    fn convert_scalar_in(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        direct: bool,
        context: SequenceContext,
    ) -> Result<(), Error> {
        let facts = self.builtin_operand_facts(id)?;
        if facts.functions(&self.graph.expression_facts).is_some() {
            let binding = if let Some(binding) = self.ordinary_function_operand(facts) {
                Some(binding)
            } else {
                self.matching_function(facts, target)?
            };
            if let Some(binding) = binding {
                self.select_function_designation(id, binding)?;
            } else {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "no overloaded function matches the target type",
                ));
            }
        }
        self.check_bound_function(id)?;
        let facts = self.builtin_operand_facts(id)?;
        let conversion = self.scalar_sequence_in(facts, target, direct, context)?;
        let Some(conversion) = conversion else {
            if let Some(sequence) = self.user_conversion_sequence(facts, target, direct)? {
                let scope = self.graph.expression_scopes
                    [self.graph.expression_facts.root(id).index()]
                .ok_or(Error::Invariant("user conversion without expression scope"))?;
                self.apply_user_sequence(
                    id,
                    sequence,
                    scope,
                    super::user_conversions::ConversionSite::Call,
                )?;
                return Ok(());
            }
            self.graph
                .analysis
                .substitution
                .emit(super::substitution_state::Failure {
                    location: self.tree.expressions.get(id).location,
                    message: "invalid implicit conversion",
                    transient: false,
                })?;
            return Ok(());
        };
        self.check_bit_field_binding(id, conversion)?;
        if let Some(mut step) = self.graph.expression_facts.function_conversion(id)
            && step.usage == super::conversion_functions::ConversionUse::Builtin
        {
            step.after = conversion;
            let scope = self.graph.expression_scopes[self.graph.expression_facts.root(id).index()]
                .ok_or(Error::Invariant(
                    "builtin conversion without expression scope",
                ))?;
            step.after_path = self.conversion_base_path(
                facts,
                conversion,
                scope,
                self.tree.expressions.get(id).location,
                true,
            )?;
            self.graph
                .expression_facts
                .set_function_conversion(id, step);
            let mut original = self.graph.expression_facts.get(id)?;
            self.record_conversion(&mut original, conversion);
            self.graph.expression_facts.set(id, original);
            return Ok(());
        }
        let mut facts = facts;
        if matches!(conversion.second, C::DerivedToBase | C::MemberToDerived) {
            let scope = self.graph.expression_scopes[self.graph.expression_facts.root(id).index()]
                .ok_or(Error::Invariant("base conversion without expression scope"))?;
            facts.set_base_path(
                self.conversion_base_path(
                    facts,
                    conversion,
                    scope,
                    self.tree.expressions.get(id).location,
                    true,
                )?,
                &mut self.graph.expression_facts,
            )?;
        }
        if matches!(
            self.graph.types.get(conversion.target).kind,
            TypeKind::Reference { .. }
        ) {
            let scope = self.graph.expression_scopes[self.graph.expression_facts.root(id).index()]
                .ok_or(Error::Invariant(
                    "reference conversion without expression scope",
                ))?;
            self.check_binding_temporary(
                facts,
                conversion,
                scope,
                self.tree.expressions.get(id).location,
            )?;
        }
        self.odr_use_constants(id, Some(conversion))?;
        self.record_conversion(&mut facts, conversion);
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }
    pub(super) fn convert_operand_scalar(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        direct: bool,
    ) -> Result<(), Error> {
        self.convert_scalar_in(id, target, direct, SequenceContext::Arithmetic)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        if facts.conversion.is_none()
            && self.graph.analysis.substitution.current().failure.is_some()
        {
            return Ok(());
        }
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
        let source = self.builtin_operand_facts(id)?;
        if self.graph.class_of(source.ty).is_some()
            && let Some(sequence) = self.conversion_function_sequence(source, target, true)?
        {
            let Some(super::procedural_calls::UserConversion::Function(binding)) =
                sequence.user_conversion
            else {
                return Err(Error::Invariant(
                    "bool conversion without selected function",
                ));
            };
            self.check_function_not_deleted(
                self.graph.binding(binding).entity,
                self.tree.expressions.get(id).location,
            )?;
            let scope = self.graph.expression_scopes[self.graph.expression_facts.root(id).index()]
                .ok_or(Error::Invariant("bool conversion without expression scope"))?;
            self.convert_class_operand(id, binding, scope)?;
        }
        self.convert_operand_scalar(id, target, true)
    }
}
