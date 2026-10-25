// Translated from sema/analyzer_traits.cpp and analyzer_builtins.cpp. See NOTICE.
//! Mixed type/value builtins retain one parsed operand and canonical result type.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    runtime_intrinsics::RuntimeIntrinsic,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reduction {
    And,
    Or,
    Xor,
    Add,
    Multiply,
    Max,
    Min,
}

impl Analyzer<'_> {
    pub(super) fn builtin_value_expression(
        &mut self,
        id: ast::ExprId,
        builtin: ast::ValueBuiltin,
        operand: ast::ExprId,
        ty: ast::TypeId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let facts = self.typed_expression(operand, scope)?;
        self.check_resolved_function(operand)?;
        let ty = self.type_syntax(ty, scope)?;
        let from = self.bare_type(facts.ty)?;
        let to = self.bare_type(ty)?;
        if self.graph.dependent_type(from) || self.graph.dependent_type(to) {
            return self.new_expression_facts(to, ValueCategory::Prvalue);
        }
        let location = self.tree.expressions.get(id).location;
        match builtin {
            ast::ValueBuiltin::ConvertVector => {
                if !matches!((&self.graph.types.get(from).kind, &self.graph.types.get(to).kind),
                    (TypeKind::Vector { bound: a, .. }, TypeKind::Vector { bound: b, .. }) if a == b)
                {
                    return Err(Error::at(
                        location,
                        "__builtin_convertvector of no vector, or to a vector of other lanes",
                    ));
                }
            }
            ast::ValueBuiltin::BitCast => {
                let eligible = !matches!(
                    self.graph.types.get(to).kind,
                    TypeKind::Reference { .. } | TypeKind::Array { .. } | TypeKind::Function { .. }
                );
                let valid = if eligible {
                    let to_size = self.bit_cast_size(to, location)?;
                    let from_size = if to_size.is_some() {
                        self.bit_cast_size(from, location)?
                    } else {
                        None
                    };
                    to_size.is_some()
                        && to_size == from_size
                        && self.property_copyable(to)?
                        && self.property_copyable(from)?
                } else {
                    false
                };
                if !valid {
                    return Err(Error::at(
                        location,
                        "__builtin_bit_cast between types of different sizes or not trivially copyable",
                    ));
                }
            }
        }
        self.convert_scalar(operand, from, false)?;
        self.new_expression_facts(to, ValueCategory::Prvalue)
    }

    fn bit_cast_size(
        &mut self,
        mut ty: TypeId,
        location: SourceOffset,
    ) -> Result<Option<u64>, Error> {
        let original = ty;
        while let TypeKind::Array {
            element,
            bound,
            operand,
        } = self.graph.types.get(ty).kind
        {
            if bound == 0 || operand.is_some() {
                return Ok(None);
            }
            ty = element;
        }
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(Scalar::Void) | TypeKind::Function { .. }
        ) {
            return Ok(None);
        }
        if let Some(class) = self.graph.class_of(ty)
            && !self.complete_class(class)?
        {
            return Ok(None);
        }
        self.type_size(original, location)
            .map(|(size, _)| Some(size))
    }

    pub(super) fn vector_reduction(
        &mut self,
        id: ast::ExprId,
        arguments: &[ast::ExprId],
        kind: Reduction,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        if arguments.len() != 1 {
            return Err(Error::at(
                location,
                "wrong number of arguments to a vector reduction",
            ));
        }
        let operand = arguments[0];
        let facts = self.typed_expression(operand, scope)?;
        self.check_resolved_function(operand)?;
        let ty = self.bare_type(facts.ty)?;
        let TypeKind::Vector {
            element,
            operand: None,
            ..
        } = self.graph.types.get(ty).kind
        else {
            return Err(Error::at(
                location,
                "a vector reduction of no vector of suitable lanes",
            ));
        };
        let element = self.bare_type(element)?;
        let floating = matches!(self.graph.types.get(element).kind,
            TypeKind::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0);
        let boolean = matches!(
            self.graph.types.get(element).kind,
            TypeKind::Scalar(Scalar::Bool)
        );
        if floating && !matches!(kind, Reduction::Max | Reduction::Min)
            || boolean && !matches!(kind, Reduction::And | Reduction::Or | Reduction::Xor)
        {
            return Err(Error::at(
                location,
                "a vector reduction of no vector of suitable lanes",
            ));
        }
        self.convert_scalar(operand, ty, false)?;
        self.graph
            .expression_facts
            .set_runtime_intrinsic(id, RuntimeIntrinsic::VectorReduction(kind));
        self.new_expression_facts(element, ValueCategory::Prvalue)
    }
}
