// Translated from analyzer_expressions.cpp::explicit_conversion/functional_cast. See NOTICE.
//! Explicit scalar casts preserve the source operand and its conversion carrier.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, LvalueTransform as L, ValueCategory as V,
        ValueConversion as C,
    },
    graph::ScopeId,
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword as K,
    syntax::{ast, storage::List},
};
impl Analyzer<'_> {
    fn cast_constness(&self, mut from: TypeId, mut to: TypeId) -> bool {
        let mut const_so_far = true;
        loop {
            let (Some((_, a)), Some((_, b))) = (
                self.graph.types.get(from).kind.pointer_level(),
                self.graph.types.get(to).kind.pointer_level(),
            ) else {
                return false;
            };
            let ca = self.type_cv(a).0;
            let cb = self.type_cv(b).0;
            if ca & !cb != 0 || ca != cb && !const_so_far {
                return true;
            }
            const_so_far &= cb & 1 != 0;
            from = a;
            to = b;
        }
    }
    fn const_castable(&self, mut from: TypeId, mut to: TypeId) -> bool {
        let mut levels = 0;
        loop {
            let a = &self.graph.types.get(from).kind;
            let b = &self.graph.types.get(to).kind;
            if let (Some((oa, ea)), Some((ob, eb))) = (a.pointer_level(), b.pointer_level()) {
                if oa != ob {
                    return false;
                }
                if matches!(self.graph.types.get(ea).kind, TypeKind::Function { .. })
                    || matches!(self.graph.types.get(eb).kind, TypeKind::Function { .. })
                {
                    return levels != 0
                        && self.graph.types.canonical_view(from)
                            == self.graph.types.canonical_view(to);
                }
                from = ea;
                to = eb;
                levels += 1;
            } else {
                return levels != 0
                    && a.pointer_level().is_none()
                    && !matches!(a, TypeKind::Function { .. })
                    && b.pointer_level().is_none()
                    && self.same_array_shape(from, to);
            }
        }
    }
    pub(super) fn explicit_scalar_conversion(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        operand: ast::ExprId,
        cast: Option<K>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let target = self.graph.types.canonical_function(target)?;
        let source = self.typed_expression(operand, scope)?;
        self.resolve_cast_function(id, operand, source, target)?;
        let source = self.graph.expression_facts.get(operand)?;
        if cast == Some(K::DynamicCast) {
            return Err(Error::Missing("class explicit conversions"));
        }
        let c_style = cast.is_none();
        let reinterpret = c_style || cast == Some(K::ReinterpretCast);
        let constness = c_style || cast == Some(K::ConstCast);
        let static_kind = !matches!(cast, Some(K::ConstCast | K::ReinterpretCast));
        if let TypeKind::Reference { element, kind } = self.graph.types.get(target).kind {
            let rvalue = kind == RefKind::Rvalue;
            let glvalue = rvalue || source.category == V::Lvalue;
            let from_pointer = self.graph.types.pointer(source.ty)?;
            let to_pointer = self.graph.types.pointer(element)?;
            let keeps_const = !self.cast_constness(from_pointer, to_pointer);
            let mut allowed = glvalue && constness && self.const_castable(from_pointer, to_pointer);
            if !allowed && static_kind {
                let hierarchy = self
                    .graph
                    .class_of(source.ty)
                    .zip(self.graph.class_of(element));
                let related = self.bare_type(element)? == self.bare_type(source.ty)?
                    || hierarchy.is_some_and(|(from, to)| {
                        self.graph.is_base_of(from, to) || self.graph.is_base_of(to, from)
                    });
                if !related
                    && (matches!(self.graph.types.get(element).kind, TypeKind::Class(_))
                        || matches!(self.graph.types.get(source.ty).kind, TypeKind::Class(_)))
                {
                    return Err(Error::Missing(
                        "class hierarchy and user reference conversions",
                    ));
                }
                allowed = glvalue && related && (keeps_const || constness);
                if allowed
                    && let Some(facts) = self.reference_hierarchy_cast(
                        source,
                        target,
                        scope,
                        self.tree.expressions.get(id).location,
                        c_style,
                    )?
                {
                    return Ok(facts);
                }
                if !allowed {
                    match self.convert_scalar(operand, target, true) {
                        Ok(()) => allowed = true,
                        Err(Error::Diagnostic {
                            message: "invalid implicit conversion",
                            ..
                        }) => (),
                        Err(error) => return Err(error),
                    }
                }
            }
            if !allowed && reinterpret {
                allowed = glvalue && (keeps_const || constness);
            }
            if !allowed {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    if glvalue {
                        "invalid cast to a reference type"
                    } else {
                        "cast of an rvalue to an lvalue reference"
                    },
                ));
            }
            let category = if !rvalue
                || matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) {
                V::Lvalue
            } else {
                V::Xvalue
            };
            let mut converted = self.graph.expression_facts.get(operand)?;
            if let Some(conversion) = converted.conversion.as_mut() {
                conversion.initialization = false;
            }
            self.graph.expression_facts.set(operand, converted);
            let mut facts = self.new_expression_facts(element, category)?;
            facts.reference_cast = Some(target);
            return Ok(facts);
        }
        if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
            if !static_kind {
                return Err(Error::Missing("class explicit conversions"));
            }
            self.elide_class_copy(
                operand,
                target,
                scope,
                true,
                false,
                self.tree.expressions.get(id).location,
            )?;
            return self.new_expression_facts(target, V::Prvalue);
        }
        let goal = self.bare_type(target)?;
        if matches!(
            self.graph.types.get(goal).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return self.new_expression_facts(goal, V::Prvalue);
        }
        if matches!(self.graph.types.get(source.ty).kind, TypeKind::Class(_)) {
            return Err(Error::Missing("class explicit conversions"));
        }
        let from = self.decayed_type(source.ty)?;
        // analyzer_member_pointers.cpp shows a successful static member-pointer
        // cast as the value itself, retyped, rather than a cast-expression node.
        if static_kind
            && matches!(
                self.graph.types.get(goal).kind,
                TypeKind::MemberPointer { .. }
            )
        {
            if let Some(mut conversion) = self.standard_conversion(source, goal, true)? {
                conversion.initialization = false;
                let mut facts = source;
                facts.conversion = Some(conversion);
                self.graph.expression_facts.set(operand, facts);
                let mut result = self.new_expression_facts(goal, V::Prvalue)?;
                result.reference_cast = Some(goal);
                return Ok(result);
            }
            if !c_style
                || !matches!(
                    self.graph.types.get(from).kind,
                    TypeKind::MemberPointer { .. }
                )
            {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "invalid conversion to a pointer to member",
                ));
            }
        }
        if static_kind && let Some(mut conversion) = self.standard_conversion(source, goal, true)? {
            conversion.initialization = false;
            let mut facts = source;
            facts.base_path = self.conversion_base_path(
                source,
                conversion,
                scope,
                self.tree.expressions.get(id).location,
                !c_style,
            )?;
            facts.conversion = Some(conversion);
            self.graph.expression_facts.set(operand, facts);
            return self.new_expression_facts(goal, V::Prvalue);
        }
        let fk = &self.graph.types.get(from).kind;
        let gk = &self.graph.types.get(goal).kind;
        let mut allowed = static_kind
            && (matches!(gk, TypeKind::Enum(_))
                && (self.is_arithmetic_operand(from) || matches!(fk, TypeKind::Enum(_)))
                || self.scoped_enum(from) && self.is_arithmetic_operand(goal));
        if static_kind && let (TypeKind::Pointer(a), TypeKind::Pointer(b)) = (fk, gk) {
            allowed |= matches!(
                self.graph.types.get(*a).kind,
                TypeKind::Scalar(Scalar::Void)
            ) && !matches!(self.graph.types.get(*b).kind, TypeKind::Function { .. })
                && (constness || !self.cast_constness(from, goal));
        }
        if !allowed
            && static_kind
            && let Some(facts) =
                self.pointer_hierarchy_cast(id, operand, source, goal, scope, c_style)?
        {
            return Ok(facts);
        }
        if !allowed && constness {
            allowed = self.const_castable(from, goal);
        }
        let fk = &self.graph.types.get(from).kind;
        let gk = &self.graph.types.get(goal).kind;
        if !allowed && reinterpret {
            let member_pair = match (fk, gk) {
                (
                    TypeKind::MemberPointer { element: a, .. },
                    TypeKind::MemberPointer { element: b, .. },
                ) => {
                    matches!(self.graph.types.get(*a).kind, TypeKind::Function { .. })
                        == matches!(self.graph.types.get(*b).kind, TypeKind::Function { .. })
                }
                _ => false,
            };
            allowed = if matches!(fk, TypeKind::Pointer(_)) && matches!(gk, TypeKind::Pointer(_))
                || member_pair
            {
                constness || !self.cast_constness(from, goal)
            } else if matches!(fk, TypeKind::Pointer(_)) {
                self.integral_kind(goal).is_some_and(|kind| kind.bits >= 64)
            } else if matches!(fk, TypeKind::Scalar(Scalar::Nullptr)) {
                self.integral_kind(goal).is_some()
            } else {
                self.is_integral_operand(from) && matches!(gk, TypeKind::Pointer(_)) || from == goal
            };
        }
        if !allowed {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "invalid explicit conversion",
            ));
        }
        let mut facts = source;
        facts.conversion = Some(Conversion {
            target: goal,
            result: goal,
            first: L::None,
            second: C::None,
            rank: R::Conversion,
            qualification: false,
            initialization: false,
            temporary: false,
        });
        self.graph.expression_facts.set(operand, facts);
        self.new_expression_facts(goal, V::Prvalue)
    }
    fn resolve_cast_function(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        source: ExprFacts,
        target: TypeId,
    ) -> Result<(), Error> {
        if source.functions.is_none() {
            return Ok(());
        }
        let binding = if let Some(binding) = self.ordinary_function_operand(source) {
            Some(binding)
        } else {
            self.matching_function(source, target)?
        };
        let binding = binding.ok_or_else(|| {
            Error::at(
                self.tree.expressions.get(id).location,
                "no overloaded function matches the cast's type",
            )
        })?;
        self.select_function_designation(operand, binding)
    }
    fn reference_hierarchy_cast(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
        c_style: bool,
    ) -> Result<Option<ExprFacts>, Error> {
        let TypeKind::Reference { element, kind } = self.graph.types.get(target).kind else {
            return Err(Error::Invariant(
                "hierarchy reference cast without reference type",
            ));
        };
        let Some((from, to)) = self
            .graph
            .class_of(source.ty)
            .zip(self.graph.class_of(element))
        else {
            return Ok(None);
        };
        if from == to {
            return Ok(None);
        }
        let downcast = self.graph.is_base_of(from, to);
        let (derived, base) = if downcast { (to, from) } else { (from, to) };
        let path = self.checked_base_path(derived, base, scope, location, !c_style, downcast)?;
        let mut facts = self.new_expression_facts(
            element,
            if kind == RefKind::Rvalue {
                V::Xvalue
            } else {
                V::Lvalue
            },
        )?;
        facts.reference_cast = Some(target);
        facts.base_path = Some(path);
        Ok(Some(facts))
    }

    fn pointer_hierarchy_cast(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        source: ExprFacts,
        goal: TypeId,
        scope: ScopeId,
        c_style: bool,
    ) -> Result<Option<ExprFacts>, Error> {
        let from = self.decayed_type(source.ty)?;
        let (TypeKind::Pointer(a), TypeKind::Pointer(b)) = (
            &self.graph.types.get(from).kind,
            &self.graph.types.get(goal).kind,
        ) else {
            return Ok(None);
        };
        let (Some(from_class), Some(to_class)) = (self.graph.class_of(*a), self.graph.class_of(*b))
        else {
            return Ok(None);
        };
        if !c_style && self.cast_constness(from, goal) {
            return Ok(None);
        }
        let downcast = self.graph.is_base_of(from_class, to_class);
        if !(downcast || c_style && self.graph.is_base_of(to_class, from_class)) {
            return Ok(None);
        }
        let (derived, base) = if downcast {
            (to_class, from_class)
        } else {
            (from_class, to_class)
        };
        let path = self.checked_base_path(
            derived,
            base,
            scope,
            self.tree.expressions.get(id).location,
            !c_style,
            downcast,
        )?;
        let mut facts = source;
        facts.base_path = Some(path);
        facts.conversion = Some(Conversion {
            target: goal,
            result: goal,
            first: L::None,
            second: C::None,
            rank: R::Conversion,
            qualification: false,
            initialization: false,
            temporary: false,
        });
        self.graph.expression_facts.set(operand, facts);
        let mut facts = self.new_expression_facts(goal, V::Prvalue)?;
        facts.base_path = Some(path);
        Ok(Some(facts))
    }

    pub(super) fn scalar_functional_cast(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        arguments: List<ast::ExprId>,
        braced: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        if let Some(braced) = braced {
            if matches!(self.graph.types.get(target).kind, TypeKind::Array { .. }) {
                let ty = self.array_braced_list(braced, target, scope)?;
                return self.new_expression_facts(ty, V::Prvalue);
            }
            if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
                return Err(Error::Missing("class list initialization"));
            }
            let ast::ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
            else {
                return Err(Error::Invariant("functional cast without list"));
            };
            let elements = self.tree.expr_lists.get(elements);
            if elements.len() > 1 {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "list-initialization of a scalar from several values",
                ));
            }
            if let Some(&element) = elements.first() {
                self.initialize_element(element, target, scope, true)?;
                let ty = self.bare_type(target)?;
                return self.new_expression_facts(ty, V::Prvalue);
            }
        }
        let arguments = self.tree.expr_lists.get(arguments);
        if arguments.len() == 1 {
            return self.explicit_scalar_conversion(id, target, arguments[0], None, scope);
        }
        if arguments.len() > 1 {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "functional cast with several arguments is not supported",
            ));
        }
        let ty = self.bare_type(target)?;
        if !matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(_)
                | TypeKind::Pointer(_)
                | TypeKind::MemberPointer { .. }
                | TypeKind::Enum(_)
        ) {
            return Err(Error::Missing("nonscalar value initialization"));
        }
        let mut facts = self.new_expression_facts(ty, V::Prvalue)?;
        facts.folded = Some(0);
        Ok(facts)
    }
}
