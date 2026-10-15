// Translated from sema/analyzer_operators.cpp::operator_candidates/builtin_operator. See NOTICE.
//! Indexed member, ordinary and associated lookup; enumeration builtin competition.
use super::{
    Error,
    analyzer::Analyzer,
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
            && self.graph.entity(class).complete
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
    // Only called when at least one operand is an enum and no operand is a
    // class. Class conversion-function builtin candidates have their own owner.
    pub(super) fn enum_builtin_parameters(
        &mut self,
        op: P,
        operands: &[ast::ExprId],
        postfix: bool,
        same_enum_list: bool,
    ) -> Result<Option<[TypeId; 2]>, Error> {
        let a = self.graph.expression_facts.get(operands[0])?;
        let boolean = self.graph.types.scalar(Scalar::Bool)?;
        if operands.len() == 1 {
            if postfix || matches!(op, P::Increment | P::Decrement) {
                return Ok(None);
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
        let b = self.graph.expression_facts.get(operands[1])?;
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
                if (p1 == p2
                    && matches!(self.graph.types.get(p1).kind, TypeKind::Enum(_))
                    && !same_enum_list)
                    || (matches!(self.graph.types.get(p1).kind, TypeKind::Pointer(_))
                        && b.null_literal)
                {
                    parameters.fill(p1);
                    true
                } else if matches!(self.graph.types.get(p2).kind, TypeKind::Pointer(_))
                    && a.null_literal
                {
                    parameters.fill(p2);
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
