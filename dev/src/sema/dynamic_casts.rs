// Translated from sema/analyzer_rtti.cpp::dynamic_cast_expression. See NOTICE.
//! Complete-class cast checks retain runtime operands on the original expression.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{EntityId, Graph, ScopeId},
    types::{RefKind, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, syntax::ast};

#[derive(Clone, Copy, Debug)]
pub(crate) struct RuntimeCast {
    pub from: EntityId,
    pub to: EntityId,
    pub reference: bool,
}

impl Graph {
    pub(crate) fn lower_cast_bases(
        &self,
        class: EntityId,
    ) -> impl Iterator<Item = Result<(EntityId, bool, bool, u64), Error>> + '_ {
        self.entity(class).bases.iter().map(|base| {
            Ok((
                base.class,
                base.access == super::graph::Access::Public,
                base.is_virtual,
                base.offset
                    .ok_or(Error::Invariant("cast hint base without layout"))?,
            ))
        })
    }
    pub(crate) fn lower_runtime_cast(&self, id: ast::ExprId) -> Result<Option<RuntimeCast>, Error> {
        self.expression_facts.get(id)?;
        Ok(self.expression_facts.runtime_cast(id))
    }
}

impl Analyzer<'_> {
    pub(super) fn dynamic_class_cast(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        operand: ast::ExprId,
        source: ExprFacts,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let pointer = matches!(self.graph.types.get(target).kind, TypeKind::Pointer(_));
        let to = match self.graph.types.get(target).kind {
            TypeKind::Pointer(element) | TypeKind::Reference { element, .. } => Some(element),
            _ => None,
        };
        let from = if pointer {
            let value = self.decayed_type(source.ty)?;
            if let TypeKind::Pointer(element) = self.graph.types.get(value).kind {
                Some(element)
            } else {
                None
            }
        } else if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference { .. }
        ) {
            Some(source.ty)
        } else {
            None
        };
        let goal = to.and_then(|ty| self.graph.class_of(ty));
        let origin = from.and_then(|ty| self.graph.class_of(ty));
        for class in [goal, origin].into_iter().flatten() {
            if !self.complete_class(class)? {
                return Err(Error::at(
                    location,
                    "a dynamic_cast to or from an incomplete class",
                ));
            }
        }
        if pointer
            && to.is_some_and(|ty| {
                matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Scalar(super::types::Scalar::Void)
                )
            })
        {
            return Err(Error::Missing("dynamic_cast to void pointer"));
        }
        if goal.is_none()
            || origin.is_none()
            || origin == goal
            || goal
                .zip(origin)
                .is_some_and(|(base, derived)| self.graph.is_base_of(base, derived))
        {
            let facts = self.explicit_conversion_from(
                id,
                target,
                operand,
                source,
                Some(Keyword::DynamicCast),
                scope,
            )?;
            if origin.is_none() || goal.is_none() {
                return Err(Error::at(
                    location,
                    "a dynamic_cast not to or from a pointer or reference to a class",
                ));
            }
            return Ok(facts);
        }
        let (Some(from_class), Some(to_class), Some(from), Some(to)) = (origin, goal, from, to)
        else {
            return Err(Error::Invariant("runtime cast without class operands"));
        };
        if !self.graph.lower_polymorphic(from_class) {
            return Err(Error::at(
                location,
                "a dynamic_cast of an operand of no polymorphic class",
            ));
        }
        let from_pointer = self.graph.types.pointer(from)?;
        let to_pointer = self.graph.types.pointer(to)?;
        if self.cast_constness(from_pointer, to_pointer) {
            return Err(Error::at(location, "a dynamic_cast casting away constness"));
        }
        let lvalue = matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference {
                kind: RefKind::Lvalue,
                ..
            }
        );
        if lvalue && source.category != V::Lvalue {
            return Err(Error::at(
                location,
                "a dynamic_cast of an rvalue to an lvalue reference",
            ));
        }
        let facts = if pointer {
            let value = self.decayed_type(source.ty)?;
            self.convert_scalar(operand, value, false)?;
            let target = self.bare_type(target)?;
            self.new_expression_facts(target, V::Prvalue)?
        } else {
            let mut facts =
                self.new_expression_facts(to, if lvalue { V::Lvalue } else { V::Xvalue })?;
            facts.reference_cast = Some(target);
            facts
        };
        if self.unevaluated == 0 {
            let owner = self.body_context.as_ref().map(|body| body.entity);
            self.graph
                .class_information_uses
                .extend([(owner, from_class), (owner, to_class)]);
        }
        self.graph.expression_facts.set_runtime_cast(
            id,
            RuntimeCast {
                from: from_class,
                to: to_class,
                reference: !pointer,
            },
        );
        Ok(facts)
    }
}
