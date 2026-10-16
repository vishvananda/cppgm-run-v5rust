// Translated from sema/analyzer_operators.cpp::operator_candidates/builtin_operator. See NOTICE.
//! Indexed member, ordinary and associated lookup; enumeration builtin competition.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::ExprFacts,
    function_names::{SpecialName, ValueKey},
    graph::{BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::{Filter, LookupResult},
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator as P, vocabulary::SimpleToken},
    syntax::ast,
    types::FundamentalType as F,
};
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct OperatorLookup {
    pub bindings: Vec<BindingId>,
    pub seen: HashSet<EntityId>,
}
impl OperatorLookup {
    pub(super) fn clear(&mut self) {
        self.bindings.clear();
        self.seen.clear();
    }
}

pub(super) fn operator_key(op: P) -> Result<ValueKey, Error> {
    Ok(ValueKey::Special(SpecialName::Operator {
        token: SimpleToken::punctuator(op)
            .ok_or(Error::Invariant("operator token without identity"))?,
        array: false,
    }))
}
impl Analyzer<'_> {
    fn class_operator(&self, binding: BindingId) -> bool {
        self.graph
            .entity(self.graph.binding(binding).entity)
            .home
            .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class)
    }
    pub(super) fn operator_candidates(
        &mut self,
        op: P,
        operands: &[ast::ExprId],
        scope: ScopeId,
        lookup: &mut OperatorLookup,
    ) -> Result<(), Error> {
        let key = operator_key(op)?;
        self.lookup_location = self.tree.expressions.get(operands[0]).location;
        let left = self.graph.expression_facts.get(operands[0])?;
        let OperatorLookup {
            bindings: set,
            seen,
        } = lookup;
        if let Some(class) = self.graph.class_of(left.ty)
            && self.complete_class(class)?
            && let Some(home) = self.graph.entity(class).scope
        {
            self.prepare_assignment_lookup(home, key, false)?;
            let found = self
                .qualified_result(home, key, Filter::Ordinary)
                .map_err(|error| match error {
                    Error::Diagnostic {
                        message: "ambiguous name",
                        ..
                    } => Error::at(self.lookup_location, "ambiguous member operator name"),
                    other => other,
                })?;
            for &binding in found.functions.as_slice() {
                if self.class_operator(binding) && seen.insert(self.graph.binding(binding).entity) {
                    set.push(binding);
                }
            }
        }
        if matches!(op, P::Assign | P::LeftBracket | P::Arrow) {
            return Ok(());
        }
        let mut from = Some(scope);
        while let Some(scope) = from {
            let found = self.unqualified_result(scope, key, Filter::Ordinary)?;
            let Some(binding) = found.binding else {
                break;
            };
            if self.graph.binding(binding).kind != BindingKind::Function {
                break;
            }
            let owner = self.graph.entity(self.graph.binding(binding).entity).home;
            if let Some(owner) =
                owner.filter(|&home| self.graph.scope(home).kind == ScopeKind::Class)
            {
                let home = self
                    .graph
                    .scope(owner)
                    .owner
                    .ok_or(Error::Invariant("operator class without owner"))?;
                let mut current = Some(scope);
                while let Some(scope) = current {
                    let node = self.graph.scope(scope);
                    if node.kind == ScopeKind::Class
                        && node.owner.is_some_and(|class| {
                            class == home || self.graph.is_base_of(home, class)
                        })
                    {
                        break;
                    }
                    current = node.parent;
                }
                from = current.and_then(|scope| self.graph.scope(scope).parent);
                continue;
            }
            for &binding in found.functions.as_slice() {
                if !self.class_operator(binding) && seen.insert(self.graph.binding(binding).entity)
                {
                    set.push(binding);
                }
            }
            break;
        }
        let mut adl = LookupResult::default();
        self.associated_functions(key, operands, &mut adl)?;
        for &binding in adl.functions.as_slice() {
            if !self.class_operator(binding) && seen.insert(self.graph.binding(binding).entity) {
                set.push(binding);
            }
        }
        Ok(())
    }

    fn operator_promoted_type(&mut self, ty: TypeId) -> Result<TypeId, Error> {
        if self.is_integral_operand(ty) {
            self.promoted_type(ty)
        } else {
            self.bare_type(ty)
        }
    }
    pub(super) fn builtin_parameters(
        &mut self,
        op: P,
        operands: &[ExprFacts],
        postfix: bool,
        same_enum_list: bool,
    ) -> Result<Option<[TypeId; 2]>, Error> {
        let a = operands[0];
        let boolean = self.graph.types.scalar(Scalar::Bool)?;
        if operands.len() == 1 {
            if matches!(op, P::Increment | P::Decrement) {
                let parameter = self.graph.types.reference(a.ty, RefKind::Lvalue)?;
                let counted = if self.is_arithmetic_type(a.ty) {
                    op == P::Increment || self.bare_type(a.ty)? != boolean
                } else {
                    matches!(self.graph.types.get(a.ty).kind, TypeKind::Pointer(element)
                        if !matches!(self.graph.types.get(element).kind,
                            TypeKind::Function { .. } | TypeKind::Scalar(Scalar::Void)))
                };
                return Ok(
                    (counted && self.type_cv(a.ty).0 & 1 == 0).then_some([parameter, parameter])
                );
            }
            if postfix {
                return Ok(None);
            }
            if op == P::Multiply
                || op == P::Add && matches!(self.graph.types.get(a.ty).kind, TypeKind::Pointer(_))
            {
                let parameter = self.bare_type(a.ty)?;
                return Ok(
                    matches!(self.graph.types.get(parameter).kind, TypeKind::Pointer(_))
                        .then_some([parameter, parameter]),
                );
            }
            let parameter = if op == P::LogicalNot {
                boolean
            } else {
                self.operator_promoted_type(a.ty)?
            };
            return Ok((op == P::LogicalNot
                || matches!(op, P::Add | P::Subtract) && self.is_arithmetic_operand(a.ty)
                || op == P::Complement && self.is_integral_operand(a.ty))
            .then_some([parameter, parameter]));
        }
        let b = operands[1];
        let p1 = self.decayed_type(a.ty)?;
        let p2 = self.decayed_type(b.ty)?;
        let difference = self.graph.types.scalar(Scalar::Basic(F::LongInt))?;
        let arithmetic = self.is_arithmetic_operand(a.ty) && self.is_arithmetic_operand(b.ty);
        let integral = self.is_integral_operand(a.ty) && self.is_integral_operand(b.ty);
        let mut parameters = [
            self.operator_promoted_type(a.ty)?,
            self.operator_promoted_type(b.ty)?,
        ];
        let pointer = |ty| matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(element) if !matches!(self.graph.types.get(element).kind, TypeKind::Function { .. }));
        let (ptr1, ptr2) = (pointer(p1), pointer(p2));
        let viable = match op {
            P::Equal | P::NotEqual | P::Less | P::Greater | P::LessEqual | P::GreaterEqual => {
                if p1 == p2
                    && matches!(self.graph.types.get(p1).kind, TypeKind::Enum(_))
                    && !same_enum_list
                {
                    parameters.fill(p1);
                    true
                } else if let Some(common) = self.builtin_comparison_pointer(a, b, p1, p2)? {
                    parameters.fill(common);
                    true
                } else {
                    arithmetic
                }
            }
            P::Multiply | P::Divide => arithmetic,
            P::Add | P::Subtract | P::LeftBracket => {
                if arithmetic && op != P::LeftBracket {
                    true
                } else if ptr1 && self.is_integral_operand(b.ty) {
                    parameters = [p1, difference];
                    true
                } else if op != P::Subtract && self.is_integral_operand(a.ty) && ptr2 {
                    parameters = [difference, p2];
                    true
                } else if op == P::Subtract && ptr1 && ptr2 {
                    let (TypeKind::Pointer(a), TypeKind::Pointer(b)) = (
                        self.graph.types.get(p1).kind.clone(),
                        self.graph.types.get(p2).kind.clone(),
                    ) else {
                        return Err(Error::Invariant("pointer candidate without pointees"));
                    };
                    if self.bare_type(a)? != self.bare_type(b)? {
                        return Ok(None);
                    }
                    parameters.fill(if self.type_cv(a).0 & !self.type_cv(b).0 != 0 {
                        p1
                    } else {
                        p2
                    });
                    true
                } else {
                    false
                }
            }
            P::Remainder | P::BitAnd | P::BitOr | P::Xor | P::ShiftLeft | P::ShiftRight => integral,
            P::LogicalAnd | P::LogicalOr => {
                parameters.fill(boolean);
                true
            }
            P::MultiplyAssign
            | P::DivideAssign
            | P::AddAssign
            | P::SubtractAssign
            | P::RemainderAssign
            | P::AndAssign
            | P::OrAssign
            | P::XorAssign
            | P::ShiftLeftAssign
            | P::ShiftRightAssign => {
                let additive = matches!(op, P::AddAssign | P::SubtractAssign);
                let multiplicative = additive || matches!(op, P::MultiplyAssign | P::DivideAssign);
                parameters[0] = self.graph.types.reference(a.ty, RefKind::Lvalue)?;
                if self.graph.types.get(a.ty).cv.0 & 1 != 0 {
                    false
                } else if multiplicative
                    && self.is_arithmetic_type(a.ty)
                    && self.is_arithmetic_operand(b.ty)
                    || !multiplicative
                        && matches!(self.graph.types.get(a.ty).kind, TypeKind::Scalar(scalar) if super::integral::IntegralKind::of(scalar).is_some())
                        && self.is_integral_operand(b.ty)
                {
                    true
                } else if additive && ptr1 && self.is_integral_operand(b.ty) {
                    parameters[1] = difference;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        Ok(viable.then_some(parameters))
    }

    fn builtin_comparison_pointer(
        &mut self,
        a: ExprFacts,
        b: ExprFacts,
        p1: TypeId,
        p2: TypeId,
    ) -> Result<Option<TypeId>, Error> {
        let pointer_like = |ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr)
            )
        };
        let ptr1 = matches!(self.graph.types.get(p1).kind, TypeKind::Pointer(_));
        let ptr2 = matches!(self.graph.types.get(p2).kind, TypeKind::Pointer(_));
        if !(pointer_like(p1) || a.null_literal && ptr2)
            || !(pointer_like(p2) || b.null_literal && ptr1)
            || !(ptr1 || ptr2)
        {
            return Ok(None);
        }
        if !ptr1 {
            return Ok(Some(p2));
        }
        if !ptr2 || p1 == p2 {
            return Ok(Some(p1));
        }
        let value = super::expression_facts::ValueCategory::Prvalue;
        if self
            .scalar_sequence(ExprFacts::new(p2, value), p1, false)?
            .is_some()
        {
            Ok(Some(p1))
        } else if self
            .scalar_sequence(ExprFacts::new(p1, value), p2, false)?
            .is_some()
        {
            Ok(Some(p2))
        } else {
            Ok(None)
        }
    }

    pub(super) fn check_operator_declaration(
        &self,
        binding: BindingId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        let Some(SpecialName::Operator { token, .. }) =
            entity.function.and_then(|props| props.special)
        else {
            return Ok(());
        };
        if entity
            .home
            .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class)
            || token.as_keyword().is_some()
        {
            return Ok(());
        }
        if matches!(
            token.as_punctuator(),
            Some(P::Assign | P::LeftParen | P::LeftBracket | P::Arrow)
        ) {
            return Err(Error::at(
                location,
                "this operator function must be a non-static member",
            ));
        }
        if entity.template_head.is_some() {
            return Ok(());
        }
        let ty = entity
            .ty
            .ok_or(Error::Invariant("operator declaration without type"))?;
        let TypeKind::Function { parameters, .. } = &self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("operator declaration without signature"));
        };
        for &parameter in parameters {
            let parameter = match self.graph.types.get(parameter).kind {
                TypeKind::Reference { element, .. } => element,
                _ => parameter,
            };
            if matches!(
                self.graph.types.get(parameter).kind,
                TypeKind::Class(_) | TypeKind::Enum(_)
            ) {
                return Ok(());
            }
        }
        Err(Error::at(
            location,
            "an operator function needs a parameter of class or enumeration type",
        ))
    }
}
