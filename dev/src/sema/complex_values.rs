// Translated from analyzer_types.cpp, analyzer_traits.cpp and analyzer_builtins.cpp. See NOTICE.
//! GNU complex identities and component facts belong to the original source expressions.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::ScopeId,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    syntax::{ast, storage::List},
};
impl Analyzer<'_> {
    pub(super) fn complex_specifier(&self, specifier: ast::Specifier) -> bool {
        let ast::SpecifierKind::Named(name) = specifier.kind else {
            return false;
        };
        let node = self.tree.names.get(name);
        let parts = self.tree.name_lists.get(node.components);
        !node.global
            && matches!(parts,[part] if part.arguments.is_none()
            && matches!(part.kind,ast::NameKind::Identifier(name) if matches!(self.identifiers.bytes(name),b"_Complex"|b"__complex__"|b"__complex")))
    }
    pub(super) fn complex_part_type(&self, ty: TypeId) -> Option<TypeId> {
        if let TypeKind::Complex(part) = self.graph.types.get(ty).kind {
            Some(part)
        } else {
            None
        }
    }
    pub(super) fn complex_promotion(&mut self, from: TypeId, to: TypeId) -> Result<bool, Error> {
        let (Some(from), Some(to)) = (self.complex_part_type(from), self.complex_part_type(to))
        else {
            return Ok(false);
        };
        Ok(matches!(
            (
                self.graph.types.get(from).kind.clone(),
                self.graph.types.get(to).kind.clone()
            ),
            (
                TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Float)),
                TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Double))
            )
        ) || self.is_integral_operand(from)
            && !matches!(
                self.graph.types.get(from).kind,
                TypeKind::Scalar(Scalar::Bool)
            )
            && self.promoted_type(from)? == to)
    }
    pub(super) fn complex_component(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        imaginary: bool,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let facts = self.typed_expression(operand, scope)?;
        if let Some(part) = self.complex_part_type(facts.ty) {
            let ty = self
                .graph
                .types
                .qualify(part, self.graph.types.get(facts.ty).cv)?;
            return self.new_expression_facts(ty, facts.category);
        }
        if !self.is_arithmetic_type(facts.ty) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "__real__ or __imag__ of an operand that is no arithmetic or complex value",
            ));
        }
        self.new_expression_facts(
            facts.ty,
            if imaginary {
                V::Prvalue
            } else {
                facts.category
            },
        )
    }
    pub(super) fn make_complex_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let args = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let [real, imaginary] = *args.as_slice() else {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "wrong number of arguments to __builtin_complex",
            ));
        };
        let a = self.typed_expression(real, scope)?;
        let b = self.typed_expression(imaginary, scope)?;
        let part = self.bare_type(a.ty)?;
        if !matches!(self.graph.types.get(part).kind, TypeKind::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0)
            || self.bare_type(b.ty)? != part
        {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "__builtin_complex of operands that are no two values of one floating type",
            ));
        }
        self.convert_scalar(real, part, false)?;
        self.convert_scalar(imaginary, part, false)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Complex(part), Cv::default())?;
        self.graph.expression_facts.note_complex_construction(id);
        self.new_expression_facts(ty, V::Prvalue)
    }
    pub(super) fn complex_binary(
        &mut self,
        id: ast::ExprId,
        op: P,
        operands: [ast::ExprId; 2],
        types: [TypeId; 2],
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let comparison = matches!(
            op,
            P::Equal | P::NotEqual | P::Less | P::Greater | P::LessEqual | P::GreaterEqual
        );
        if comparison && !matches!(op, P::Equal | P::NotEqual) {
            return Err(Error::at(
                location,
                "an ordering comparison of complex values",
            ));
        }
        if !(comparison || matches!(op, P::Add | P::Subtract | P::Multiply | P::Divide))
            || !types
                .iter()
                .all(|&ty| self.is_arithmetic_operand(ty) || self.complex_part_type(ty).is_some())
        {
            return Err(Error::at(
                location,
                if comparison {
                    "invalid operands to a comparison"
                } else {
                    "invalid operands to a binary operator"
                },
            ));
        }
        let ty = self.arithmetic_type(types[0], types[1])?;
        let part = self
            .complex_part_type(ty)
            .ok_or(Error::Invariant("complex arithmetic without part type"))?;
        for (operand, original) in operands.into_iter().zip(types) {
            self.convert_operand_scalar(
                operand,
                if comparison || self.complex_part_type(original).is_some() {
                    ty
                } else {
                    part
                },
                false,
            )?;
        }
        let result = if comparison {
            self.graph.types.scalar(Scalar::Bool)?
        } else {
            ty
        };
        self.new_expression_facts(result, V::Prvalue)
    }
}
