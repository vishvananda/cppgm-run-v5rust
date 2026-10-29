// Translated from analyzer_types.cpp and class_templates.cpp. See NOTICE.
//! Vector formation retains a scalar identity and a deferred size operand, never attribute text.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::ScopeId,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};

impl Analyzer<'_> {
    pub(super) fn vector_reinterpretation(
        &mut self,
        from: TypeId,
        to: TypeId,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let from = self.bare_type(from)?;
        let to = self.bare_type(to)?;
        let from_vector = self.vector_shape(from).is_some();
        let to_vector = self.vector_shape(to).is_some();
        let integral = |ty| {
            matches!(self.graph.types.get(ty).kind, TypeKind::Scalar(_))
                && self.integral_kind(ty).is_some()
        };
        if !from_vector && !to_vector
            || matches!(
                self.graph.types.get(from).kind,
                TypeKind::Vector {
                    operand: Some(_),
                    ..
                }
            )
            || matches!(
                self.graph.types.get(to).kind,
                TypeKind::Vector {
                    operand: Some(_),
                    ..
                }
            )
            || !from_vector
                && (!integral(from)
                    || self.graph.types.get(from).kind == TypeKind::Scalar(Scalar::Bool))
            || !to_vector && !integral(to)
        {
            return Ok(false);
        }
        Ok(self.type_size(from, location)?.0 == self.type_size(to, location)?.0)
    }

    pub(super) fn vector_specifier_types(
        &mut self,
        mut ty: TypeId,
        list: List<ast::Specifier>,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        for index in 0..self.tree.specs.get(list).len() {
            let specifier = self.tree.specs.get(list)[index];
            if let ast::SpecifierKind::Vector { extended, operand } = specifier.kind {
                ty = self.vector_type(ty, operand, !extended, scope, specifier.location)?;
            }
        }
        Ok(ty)
    }

    pub(super) fn vector_declarator_type(
        &mut self,
        ty: TypeId,
        declarator: Option<ast::DeclaratorId>,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        if let Some(declarator) = declarator {
            self.vector_attributes(ty, self.tree.declarators.get(declarator).attributes, scope)
        } else {
            Ok(ty)
        }
    }
    pub(super) fn vector_attributes(
        &mut self,
        mut ty: TypeId,
        attributes: List<ast::Attribute>,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        for index in 0..self.tree.attributes.get(attributes).len() {
            let attribute = self.tree.attributes.get(attributes)[index];
            if matches!(
                attribute.kind,
                ast::AttributeKind::VectorSize | ast::AttributeKind::ExtVectorType
            ) {
                ty = self.vector_type(
                    ty,
                    attribute
                        .expression
                        .ok_or(Error::Invariant("vector attribute without size"))?,
                    attribute.kind == ast::AttributeKind::VectorSize,
                    scope,
                    attribute.location,
                )?;
            }
        }
        Ok(ty)
    }

    pub(super) fn vector_type(
        &mut self,
        ty: TypeId,
        size: ast::ExprId,
        bytes: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let element = self.bare_type(ty)?;
        let cv = self.graph.types.get(ty).cv;
        let value = if self.template_depth != 0 && self.graph.dependent_type(element) {
            None
        } else {
            self.constant_value(size, scope, false)?
        };
        if self.template_depth != 0 && value.is_none() {
            let operand = self.dependent_operand(size, scope, false)?;
            return self.graph.types.intern(
                TypeKind::Vector {
                    element,
                    bound: 0,
                    operand: Some(operand),
                    bytes,
                },
                cv,
            );
        }
        let value = value.ok_or_else(|| {
            Error::at(
                location,
                "a vector size that is no positive integral constant",
            )
        })?;
        if self.integral_kind(value.ty).is_none() {
            return Err(Error::at(
                location,
                "a vector size that is no positive integral constant",
            ));
        }
        let bound = self
            .checked_vector_bound(element, value.bits, bytes)
            .map_err(|message| Error::at(location, message))?;
        self.graph.types.intern(
            TypeKind::Vector {
                element,
                bound,
                operand: None,
                bytes: false,
            },
            cv,
        )
    }

    pub(super) fn checked_vector_bound(
        &self,
        element: TypeId,
        size: u64,
        bytes: bool,
    ) -> Result<u64, &'static str> {
        if !matches!(self.graph.types.get(element).kind, TypeKind::Scalar(scalar)
            if !(matches!(scalar, Scalar::Void | Scalar::Nullptr) || bytes && scalar == Scalar::Bool))
        {
            return Err("a vector of no integral or floating type");
        }
        // Every accepted fundamental has a fixed, nonzero target size.
        let lane_size = match self.graph.types.get(element).kind {
            TypeKind::Scalar(Scalar::Basic(f)) => {
                u64::try_from(f.size()).map_err(|_| "target scalar size")?
            }
            TypeKind::Scalar(Scalar::Short | Scalar::UnsignedShort) => 2,
            _ => 1,
        };
        let lanes = if bytes { size / lane_size } else { size };
        if lanes == 0
            || lanes > 1024
            || bytes && (!size.is_multiple_of(lane_size) || !lanes.is_power_of_two())
        {
            return Err(
                "a vector size that is no positive power-of-two multiple of its lane's size",
            );
        }
        Ok(lanes)
    }

    pub(super) fn substitute_vector_type(
        &mut self,
        ty: TypeId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let TypeKind::Vector { operand, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("vector substitution without vector"));
        };
        if operand.is_none() {
            return Ok(Some(ty));
        }
        self.substitute_bound_type(ty, environment)
    }
    pub(super) fn deduce_vector_type(
        &mut self,
        pattern: TypeId,
        actual: TypeId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        let (
            TypeKind::Vector {
                element: p,
                operand: Some(operand),
                bytes,
                ..
            },
            TypeKind::Vector {
                element: a,
                bound,
                operand: None,
                ..
            },
        ) = (
            self.graph.types.get(pattern).kind.clone(),
            self.graph.types.get(actual).kind.clone(),
        )
        else {
            return Ok(self.bare_type(pattern)? == self.bare_type(actual)?);
        };
        if let Some(index) = self.named_bound_parameter(operand, depth)
            && let Some(slot) = arguments.get_mut(index)
        {
            let count = if bytes {
                bound
                    .checked_mul(self.type_size(a, SourceOffset(0))?.0)
                    .ok_or(Error::Limit("target vector size"))?
            } else {
                bound
            };
            let ty = self.graph.types.scalar(Scalar::Basic(
                crate::types::FundamentalType::UnsignedLongInt,
            ))?;
            let value = self
                .graph
                .types
                .intern(TypeKind::Value { ty, bits: count }, Cv::default())?;
            if slot.is_some_and(|known| !self.same_template_value(known, value)) {
                return Ok(false);
            }
            slot.get_or_insert(value);
        }
        self.deduce_signature_type(p, a, depth, arguments, false)
    }
}
