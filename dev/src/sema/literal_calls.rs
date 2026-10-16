// Translated from sema/analyzer_literals.cpp. See NOTICE.
//! Text literal calls retain the selected entity and decoded operand shape.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    function_names::{SpecialName, ValueKey},
    function_resolution::CallTarget,
    graph::{BindingKind, FunctionAvailability, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{lex::PostTokenKind, syntax::ast, types::FundamentalType};

#[derive(Clone, Copy, Debug)]
pub(crate) enum TextCall {
    Character { source: TypeId, bits: u64 },
    String { element: TypeId, units: u32 },
}

impl Analyzer<'_> {
    pub(super) fn literal_expression(
        &mut self,
        id: ast::ExprId,
        literal: ast::Literal,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        if literal.suffix_name.is_some() {
            return self.text_literal_call(id, literal, scope);
        }
        let (ty, category) = match literal.kind {
            PostTokenKind::Literal { ty, .. }
            | PostTokenKind::Character {
                ty,
                suffix_begin: None,
                ..
            } => (
                self.graph.types.scalar(Scalar::Basic(ty))?,
                ValueCategory::Prvalue,
            ),
            PostTokenKind::String { ty, elements, .. } => {
                let element = self.graph.types.scalar(Scalar::Basic(ty))?;
                let element = self.graph.types.qualify(element, Cv::CONST)?;
                (
                    self.graph.types.intern(
                        TypeKind::Array {
                            element,
                            bound: u64::from(elements),
                            operand: None,
                        },
                        Cv::default(),
                    )?,
                    ValueCategory::Lvalue,
                )
            }
            _ => return Err(Error::Missing("user-defined semantic literals")),
        };
        let mut facts = self.new_expression_facts(ty, category)?;
        if let PostTokenKind::Literal { ty, data, .. } = literal.kind {
            facts.null_literal = super::integral::IntegralKind::of(Scalar::Basic(ty)).is_some()
                && data.as_slice().iter().all(|&b| b == 0);
        }
        Ok(facts)
    }
    pub(super) fn text_literal_call(
        &mut self,
        id: ast::ExprId,
        literal: ast::Literal,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let suffix = literal
            .suffix_name
            .ok_or(Error::Invariant("literal call without suffix"))?;
        let (shape, first, second, missing) = match literal.kind {
            PostTokenKind::Character { ty, data, .. } => {
                let source = self.graph.types.scalar(Scalar::Basic(ty))?;
                let mut bytes = [0; 8];
                let data = data.as_slice();
                bytes[..data.len()].copy_from_slice(data);
                (
                    TextCall::Character {
                        source,
                        bits: u64::from_le_bytes(bytes),
                    },
                    ExprFacts::new(source, ValueCategory::Prvalue),
                    None,
                    "no literal operator for the user-defined character literal",
                )
            }
            PostTokenKind::String { ty, elements, .. } => {
                let element = self.graph.types.scalar(Scalar::Basic(ty))?;
                let unit = self.graph.types.qualify(element, Cv::CONST)?;
                let array = self.graph.types.intern(
                    TypeKind::Array {
                        element: unit,
                        bound: u64::from(elements),
                        operand: None,
                    },
                    Cv::default(),
                )?;
                let length = self
                    .graph
                    .types
                    .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?;
                (
                    TextCall::String {
                        element,
                        units: elements,
                    },
                    ExprFacts::new(array, ValueCategory::Lvalue),
                    Some(ExprFacts::new(length, ValueCategory::Prvalue)),
                    "no literal operator for the user-defined string literal",
                )
            }
            _ => return Err(Error::Missing("numeric literal operator calls")),
        };
        self.lookup_location = location;
        let found = self.unqualified_result(
            scope,
            ValueKey::Special(SpecialName::Literal(suffix)),
            Filter::Ordinary,
        )?;
        if !found
            .binding
            .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
        {
            return Err(Error::at(location, missing));
        }
        let operands = [first, second.unwrap_or(first)];
        let binding = self.resolve_value_call(
            location,
            found.functions.as_slice(),
            &operands[..=usize::from(second.is_some())],
        )?;
        let entity = self.graph.binding(binding).entity;
        if self.graph.entity(entity).availability == FunctionAvailability::Deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.demand_specialization_body(binding);
        self.demand_function(binding, true);
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("literal operator without type"))?;
        let ty = self.graph.types.canonical_function(ty)?;
        let mut facts = self.call_result(ty)?;
        facts.call = Some(CallTarget::Direct(self.graph.function_origin(entity)?));
        facts.literal_call = Some(shape);
        self.graph.record_call_binding(id, binding);
        Ok(facts)
    }
}
