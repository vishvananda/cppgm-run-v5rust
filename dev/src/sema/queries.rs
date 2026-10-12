// Translated from sema/constant_evaluator.cpp and layout.cpp's scalar queries. See NOTICE.
//! Unevaluated operands consume declared types; they never demand an object construction.
use super::{
    Error,
    analyzer::Analyzer,
    constants::Constant,
    graph::{BindingId, BindingKind, ScopeId},
    lookup::Filter,
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
    types::FundamentalType as F,
};
impl Analyzer<'_> {
    pub(super) fn binding_expression_type(&self, binding: BindingId) -> Result<TypeId, Error> {
        let binding = self.graph.binding(binding);
        let ty = if binding.kind == BindingKind::Variable {
            self.graph.entity(binding.entity).ty
        } else {
            binding.ty
        };
        let ty = ty.ok_or(Error::Invariant("expression binding without type"))?;
        Ok(
            if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
                element
            } else {
                ty
            },
        )
    }
    fn operand_type(&mut self, expr: ast::ExprId, scope: ScopeId) -> Result<TypeId, Error> {
        let mut inner = expr;
        while let ExprKind::Paren(id) = self.tree.expressions.get(inner).kind {
            inner = id;
        }
        if let ExprKind::Id(name) = self.tree.expressions.get(inner).kind {
            let binding = self.lookup(name, scope, Filter::Ordinary)?;
            if matches!(
                self.graph.binding(binding).kind,
                BindingKind::Type
                    | BindingKind::Alias
                    | BindingKind::Variable
                    | BindingKind::Parameter
                    | BindingKind::Function
            ) {
                return self.binding_expression_type(binding);
            }
        }
        self.evaluate_constant(expr, scope, true)
            .map(|value| value.ty)
    }
    pub(super) fn constant_query(
        &mut self,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
        alignment: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let ty = if let Some(ty) = ty {
            self.type_syntax(ty, scope)?
        } else {
            self.operand_type(
                operand.ok_or(Error::Invariant("query without operand"))?,
                scope,
            )?
        };
        let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        let (size, align) = self.type_size(ty, location)?;
        Ok(Constant {
            bits: if alignment { align } else { size },
            ty: self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?,
        })
    }
    fn type_size(&self, mut ty: TypeId, location: SourceOffset) -> Result<(u64, u64), Error> {
        let mut count = Some(1u64);
        while let TypeKind::Array { element, bound } = self.graph.types.get(ty).kind {
            if bound == 0 {
                return Err(Error::at(location, "size of an array of unknown bound"));
            }
            // C++ checks an element's size before multiplication overflow.
            // Remember overflow while descending; diagnose the element first.
            count = count.and_then(|count| count.checked_mul(bound));
            ty = element;
        }
        while let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
            ty = self
                .graph
                .entity(entity)
                .enumeration
                .and_then(|info| info.underlying)
                .ok_or_else(|| Error::at(location, "size of an incomplete enumeration"))?;
        }
        let size = match self.graph.types.get(ty).kind {
            TypeKind::Scalar(Scalar::Basic(scalar)) => {
                u64::try_from(scalar.size()).map_err(|_| Error::Limit("target scalar size"))?
            }
            TypeKind::Scalar(Scalar::Void) => return Err(Error::at(location, "size of void")),
            TypeKind::Scalar(Scalar::Bool | Scalar::SignedChar | Scalar::UnsignedChar) => 1,
            TypeKind::Scalar(Scalar::Short | Scalar::UnsignedShort) => 2,
            TypeKind::Scalar(Scalar::Nullptr)
            | TypeKind::Pointer(_)
            | TypeKind::Reference { .. } => 8,
            TypeKind::Function { .. } => {
                return Err(Error::at(location, "size of a function type"));
            }
            TypeKind::Class(entity) => {
                if !self.graph.entity(entity).complete {
                    return Err(Error::at(location, "size of an incomplete class"));
                }
                return Err(Error::Missing("class layout for size queries"));
            }
            TypeKind::Enum(_) => return Err(Error::Invariant("enum query not unwound")),
            TypeKind::Array { .. } => return Err(Error::Invariant("array query not unwound")),
        };
        Ok((
            count
                .and_then(|count| size.checked_mul(count))
                .ok_or_else(|| Error::at(location, "array too large"))?,
            size,
        ))
    }
    pub(super) fn decltype_type(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let mut expr = operand;
        let mut parenthesized = false;
        while let ExprKind::Paren(inner) = self.tree.expressions.get(expr).kind {
            expr = inner;
            parenthesized = true;
        }
        if let ExprKind::Id(name) = self.tree.expressions.get(expr).kind {
            let found = self.lookup(name, scope, Filter::Ordinary)?;
            let binding = self.graph.binding(found);
            let ty = match binding.kind {
                BindingKind::Variable => self.graph.entity(binding.entity).ty,
                BindingKind::Enumerator => {
                    let super::constants::ConstantState::Value(value) =
                        self.graph.entity(binding.entity).constant
                    else {
                        return Err(Error::Invariant("enumerator without value"));
                    };
                    return Ok(value.ty);
                }
                BindingKind::Parameter => binding.ty,
                BindingKind::Function => {
                    let ty = self
                        .graph
                        .entity(binding.entity)
                        .ty
                        .ok_or(Error::Invariant("function without type"))?;
                    Some(self.graph.types.canonical_function(ty)?)
                }
                _ => {
                    return Err(Error::at(
                        self.tree.expressions.get(expr).location,
                        "decltype operand is not an expression",
                    ));
                }
            }
            .ok_or(Error::Invariant("decltype binding without type"))?;
            return if parenthesized {
                self.graph.types.reference(ty, RefKind::Lvalue)
            } else {
                Ok(ty)
            };
        }
        match self.evaluate_constant(operand, scope, true) {
            Ok(value) => Ok(value.ty),
            Err(Error::NonConstant { .. }) => Err(Error::at(
                self.tree.expressions.get(operand).location,
                "unsupported decltype operand",
            )),
            Err(error) => Err(error),
        }
    }
}
