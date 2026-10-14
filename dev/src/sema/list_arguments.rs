// Translated from sema/analyzer_initialization.cpp::braced_argument/list_sequence. See NOTICE.
//! Lists have typed elements during ranking; their object facts are published only after selection.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank, LvalueTransform, ReferenceSource, ValueConversion,
    },
    graph::ScopeId,
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::syntax::ast::{self, ExprKind};
impl Analyzer<'_> {
    pub(super) fn type_call_argument(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
            for &element in self.tree.expr_lists.get(elements) {
                self.type_call_argument(element, scope)?;
            }
        } else {
            self.typed_expression(id, scope)?;
        }
        Ok(())
    }
    pub(super) fn list_sequence(
        &mut self,
        id: ast::ExprId,
        parameter: TypeId,
    ) -> Result<Option<Conversion>, Error> {
        self.check_stack()?;
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("list sequence without list"));
        };
        let reference = match self.graph.types.get(parameter).kind {
            TypeKind::Reference { element, kind } => Some((element, kind)),
            _ => None,
        };
        let target = reference.map_or(parameter, |(element, _)| element);
        if reference.is_some_and(|(_, kind)| kind == RefKind::Lvalue)
            && self.type_cv(target) != Cv::CONST
        {
            return Ok(None);
        }
        let mut sequence = Conversion {
            target: parameter,
            result: target,
            first: LvalueTransform::None,
            second: ValueConversion::None,
            rank: ConversionRank::Exact,
            qualification: false,
            initialization: true,
            temporary: false,
            reference_source: ReferenceSource::Other,
        };
        let values = self.tree.expr_lists.get(elements);
        match self.graph.types.get(target).kind {
            TypeKind::Scalar(Scalar::Void) => return Ok(None),
            TypeKind::Array { element, bound } => {
                if bound == 0
                    || u64::try_from(values.len()).map_err(|_| Error::Limit("list size"))? > bound
                {
                    return Ok(None);
                }
                for &id in values {
                    let step =
                        if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. }) {
                            self.list_sequence(id, element)?
                        } else {
                            self.scalar_sequence(
                                self.graph.expression_facts.get(id)?,
                                element,
                                false,
                            )?
                        };
                    let Some(step) = step else {
                        return Ok(None);
                    };
                    if self.compare_sequences(step, sequence)? == std::cmp::Ordering::Less {
                        sequence = step;
                    }
                }
                sequence.result = target;
            }
            TypeKind::Class(_) => return Err(Error::Missing("class list argument ranking")),
            _ => {
                if values.len() > 1 {
                    return Ok(None);
                }
                if let Some(&single) = values.first() {
                    if matches!(
                        self.tree.expressions.get(single).kind,
                        ExprKind::Braced { .. }
                    ) {
                        return Ok(None);
                    }
                    let bare = self.bare_type(target)?;
                    let Some(step) = self.scalar_sequence(
                        self.graph.expression_facts.get(single)?,
                        bare,
                        false,
                    )?
                    else {
                        return Ok(None);
                    };
                    sequence = step;
                }
                sequence.result = self.bare_type(target)?;
            }
        }
        sequence.target = parameter;
        sequence.temporary = reference.is_some();
        if reference.is_some() {
            sequence.result = target;
        }
        Ok(Some(sequence))
    }
}
