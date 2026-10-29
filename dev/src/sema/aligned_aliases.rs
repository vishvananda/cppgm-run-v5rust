// Translated from sema/layout.cpp's alias alignment records. See NOTICE.
//! Declaration alignment is not canonical type identity. These sparse facts
//! belong to one unit and are published with the declaration/type-id they describe.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingId, BindingKind, EntityId},
    specifiers::Specifiers,
    types::TypeKind,
};
use crate::support::hash::HashMap;
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum AlignmentTarget {
    Object,
    Pointee,
}

#[derive(Default)]
pub(super) struct AliasAlignments {
    aliases: HashMap<BindingId, u32>,
    variables: HashMap<(EntityId, AlignmentTarget), u32>,
    type_operands: HashMap<ast::TypeId, u32>,
}
impl AliasAlignments {
    pub(super) fn alias(&self, binding: BindingId) -> u32 {
        self.aliases.get(&binding).copied().unwrap_or(0)
    }
    pub(super) fn object(&self, entity: EntityId, natural: u64) -> u64 {
        self.variables
            .get(&(entity, AlignmentTarget::Object))
            .map_or(natural, |&value| u64::from(value))
    }
    pub(super) fn record_type(&mut self, syntax: ast::TypeId, alignment: u32) {
        if alignment != 0 {
            self.type_operands.insert(syntax, alignment);
        }
    }
    pub(super) fn type_operand(&self, syntax: ast::TypeId) -> u32 {
        self.type_operands.get(&syntax).copied().unwrap_or(0)
    }
}
impl Analyzer<'_> {
    pub(super) fn alias_declaration_alignment(
        &mut self,
        binding: BindingId,
        specs: Specifiers,
        attributes: crate::syntax::storage::List<ast::Attribute>,
        scope: super::graph::ScopeId,
    ) -> Result<(), Error> {
        let own = self.layout_attributes(attributes, scope, false)?.alignment;
        self.record_alias_alignment(specs, binding, own)
    }

    pub(super) fn record_alias_alignment(
        &mut self,
        specs: Specifiers,
        declared: BindingId,
        own: u64,
    ) -> Result<(), Error> {
        if self.template_depth != 0 || own == 0 && specs.alias_alignment == 0 {
            return Ok(());
        }
        let binding = self.graph.binding(declared);
        let aligned = if own != 0 {
            super::layout_attributes::target_alignment(own)?
        } else {
            specs.alias_alignment
        };
        if binding.kind == BindingKind::Alias {
            if own != 0 || binding.ty == Some(specs.ty) {
                self.graph
                    .alias_alignments
                    .aliases
                    .insert(declared, aligned);
            }
            return Ok(());
        }
        if binding.kind != BindingKind::Variable {
            return Ok(());
        }
        let ty = binding
            .ty
            .ok_or(Error::Invariant("aligned declaration without type"))?;
        if matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(element) if element == specs.ty)
        {
            self.graph
                .alias_alignments
                .variables
                .insert((binding.entity, AlignmentTarget::Pointee), aligned);
        } else if ty == specs.ty {
            self.graph
                .alias_alignments
                .variables
                .insert((binding.entity, AlignmentTarget::Object), aligned);
            // Only a stronger alias alignment becomes a storage request.
            // A weaker one still controls a member's placement and __alignof__.
            if self.query_complete(ty)?
                && u64::from(aligned)
                    > self
                        .type_size(ty, crate::support::source::SourceOffset(0))?
                        .1
            {
                let mut record = self.graph.entity_mut(binding.entity);
                let attributes = &mut record.attributes_mut().storage_attributes;
                attributes.alignment = attributes.alignment.max(aligned);
            }
        }
        Ok(())
    }

    pub(super) fn expression_alias_alignment(
        &self,
        mut expression: ast::ExprId,
    ) -> Result<u32, Error> {
        let mut target = AlignmentTarget::Object;
        loop {
            let node = self.tree.expressions.get(expression);
            match node.kind {
                ast::ExprKind::Paren(inner) => expression = inner,
                ast::ExprKind::Unary { op, operand }
                    if op.token.as_punctuator() == Some(P::Multiply)
                        && target == AlignmentTarget::Object =>
                {
                    target = AlignmentTarget::Pointee;
                    expression = operand;
                }
                ast::ExprKind::Unary { op, operand }
                    if op.token.as_punctuator() == Some(P::BitAnd)
                        && target == AlignmentTarget::Pointee =>
                {
                    target = AlignmentTarget::Object;
                    expression = operand;
                }
                ast::ExprKind::Id(_) | ast::ExprKind::Member { .. } => {
                    let binding = match node.kind {
                        ast::ExprKind::Id(name) => Some(
                            self.graph
                                .resolved_names
                                .get(name.index())
                                .copied()
                                .flatten()
                                .ok_or(Error::Invariant("aligned query without resolved name"))?,
                        ),
                        _ => self
                            .graph
                            .expression_facts
                            .get(expression)?
                            .member(&self.graph.expression_facts),
                    };
                    return Ok(binding
                        .map(|binding| self.graph.binding(binding).entity)
                        .and_then(|entity| {
                            self.graph.alias_alignments.variables.get(&(entity, target))
                        })
                        .copied()
                        .unwrap_or(0));
                }
                _ => return Ok(0),
            }
        }
    }
}
