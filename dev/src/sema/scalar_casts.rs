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
            let (TypeKind::Pointer(a), TypeKind::Pointer(b)) = (
                &self.graph.types.get(from).kind,
                &self.graph.types.get(to).kind,
            ) else {
                return false;
            };
            let ca = self.type_cv(*a).0;
            let cb = self.type_cv(*b).0;
            if ca & !cb != 0 || ca != cb && !const_so_far {
                return true;
            }
            const_so_far &= cb & 1 != 0;
            from = *a;
            to = *b;
        }
    }
    fn const_castable(&self, mut from: TypeId, mut to: TypeId) -> Result<bool, Error> {
        let mut levels = 0;
        loop {
            let a = &self.graph.types.get(from).kind;
            let b = &self.graph.types.get(to).kind;
            if let (TypeKind::Pointer(ea), TypeKind::Pointer(eb)) = (a, b) {
                if matches!(self.graph.types.get(*ea).kind, TypeKind::Function { .. })
                    || matches!(self.graph.types.get(*eb).kind, TypeKind::Function { .. })
                {
                    return Ok(levels != 0
                        && self.graph.types.canonical_view(from)
                            == self.graph.types.canonical_view(to));
                }
                from = *ea;
                to = *eb;
                levels += 1;
            } else {
                if matches!(a, TypeKind::Array { .. }) || matches!(b, TypeKind::Array { .. }) {
                    return Err(Error::Missing("array reference and qualification casts"));
                }
                return Ok(levels != 0
                    && !matches!(a, TypeKind::Pointer(_) | TypeKind::Function { .. })
                    && !matches!(b, TypeKind::Pointer(_))
                    && a == b);
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
        if source.functions.is_some() {
            let binding = self.matching_function(source, target)?.ok_or_else(|| {
                Error::at(
                    self.tree.expressions.get(id).location,
                    "no overloaded function matches the cast's type",
                )
            })?;
            self.select_function_designation(operand, binding)?;
        }
        let source = self.graph.expression_facts.get(operand)?;
        if matches!(self.graph.types.get(target).kind, TypeKind::Class(_))
            || matches!(self.graph.types.get(source.ty).kind, TypeKind::Class(_))
            || cast == Some(K::DynamicCast)
        {
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
            let mut allowed =
                glvalue && constness && self.const_castable(from_pointer, to_pointer)?;
            if !allowed && static_kind {
                let related = self.bare_type(element)? == self.bare_type(source.ty)?;
                allowed = glvalue && related && (keeps_const || constness);
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
            let mut facts = self.new_expression_facts(element, category)?;
            facts.reference_cast = Some(target);
            return Ok(facts);
        }
        let goal = self.bare_type(target)?;
        if matches!(
            self.graph.types.get(goal).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return self.new_expression_facts(goal, V::Prvalue);
        }
        let from = self.decayed_type(source.ty)?;
        if static_kind && let Some(mut conversion) = self.standard_conversion(source, goal, true)? {
            conversion.initialization = false;
            let mut facts = source;
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
        if !allowed && constness {
            allowed = self.const_castable(from, goal)?;
        }
        if !allowed && reinterpret {
            allowed = if matches!(fk, TypeKind::Pointer(_)) && matches!(gk, TypeKind::Pointer(_)) {
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
            TypeKind::Scalar(_) | TypeKind::Pointer(_) | TypeKind::Enum(_)
        ) {
            return Err(Error::Missing("nonscalar value initialization"));
        }
        let mut facts = self.new_expression_facts(ty, V::Prvalue)?;
        facts.folded = Some(0);
        Ok(facts)
    }
}
