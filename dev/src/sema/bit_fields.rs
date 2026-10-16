// Translated from sema/analyzer_declarations.cpp::bit_field_declaration. See NOTICE.
//! Declaration-owned bit-field identity; layout publishes storage separately on demand.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{DataMemberKind, EntityId, ScopeId, ScopeKind},
    types::TypeKind,
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::{ast, storage::List},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BitFieldId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Storage {
    pub bit: u64,
    pub offset: u64,
    pub bytes: u8,
    pub split: u8,
    pub shared: bool,
}
struct Field {
    width: u64,
    operation: super::types::TypeId,
    signed: bool,
    storage: Option<Storage>,
}
#[derive(Default)]
pub(super) struct BitFields {
    fields: Vec<Field>,
}
impl BitFields {
    pub(super) fn add(
        &mut self,
        width: u64,
        operation: super::types::TypeId,
        signed: bool,
    ) -> Result<BitFieldId, Error> {
        let id = BitFieldId(
            u32::try_from(self.fields.len()).map_err(|_| Error::Limit("too many bit-fields"))?,
        );
        self.fields.push(Field {
            width,
            operation,
            signed,
            storage: None,
        });
        Ok(id)
    }
    pub(super) fn operation(&self, id: BitFieldId) -> super::types::TypeId {
        self.fields[id.0 as usize].operation
    }
    pub(super) fn signed(&self, id: BitFieldId) -> bool {
        self.fields[id.0 as usize].signed
    }
    pub(super) fn width(&self, id: BitFieldId) -> u64 {
        self.fields[id.0 as usize].width
    }
    pub(super) fn storage(&self, id: BitFieldId) -> Result<Storage, Error> {
        self.fields[id.0 as usize]
            .storage
            .ok_or(Error::Invariant("bit-field without completed layout"))
    }
    pub(super) fn publish(&mut self, id: BitFieldId, storage: Storage) {
        self.fields[id.0 as usize].storage = Some(storage);
    }
}
impl Analyzer<'_> {
    pub(super) fn check_bit_field_binding(
        &self,
        expression: ast::ExprId,
        conversion: super::expression_facts::Conversion,
    ) -> Result<(), Error> {
        if self.graph.expression_facts.get(expression)?.is_bit_field()
            && !conversion.temporary
            && matches!(
                self.graph.types.get(conversion.target).kind,
                TypeKind::Reference {
                    kind: super::types::RefKind::Lvalue,
                    ..
                }
            )
        {
            return Err(Error::at(
                self.tree.expressions.get(expression).location,
                "a non-const reference bound to a bit-field",
            ));
        }
        Ok(())
    }
    pub(super) fn field_type(
        &mut self,
        facts: super::expression_facts::ExprFacts,
        ty: super::types::TypeId,
    ) -> Result<super::types::TypeId, Error> {
        if !facts.is_bit_field() || !matches!(self.graph.types.get(ty).kind, TypeKind::Scalar(_)) {
            return Ok(ty);
        }
        let Some(id) = facts.field_identity else {
            return Ok(ty);
        };
        let Some(kind) = self.graph.integral_kind(ty) else {
            return Ok(ty);
        };
        let width = self.graph.bit_fields.width(id).min(u64::from(kind.bits));
        if width < 32 || width == 32 && kind.signed {
            self.graph.types.scalar(super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
        } else if width == 32 {
            self.graph.types.scalar(super::types::Scalar::Basic(
                crate::types::FundamentalType::UnsignedInt,
            ))
        } else {
            Ok(ty)
        }
    }
    pub(super) fn promoted_field_type(
        &mut self,
        facts: super::expression_facts::ExprFacts,
        ty: super::types::TypeId,
    ) -> Result<super::types::TypeId, Error> {
        let ty = self.field_type(facts, ty)?;
        self.promoted_type(ty)
    }
    pub(super) fn bit_field_declaration(
        &mut self,
        declaration: ast::DeclId,
        specifiers: List<ast::Specifier>,
        fields: List<ast::BitField>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let node = *self.tree.declarations.get(declaration);
        if self.graph.scope(scope).kind != ScopeKind::Class {
            return Err(Error::at(node.location, "bit-field outside a class"));
        }
        let specs = self.specifiers(specifiers, scope, false)?;
        if matches!(self.graph.types.get(specs.ty).kind, TypeKind::Auto) {
            return Err(Error::at(node.location, "bit-field without a type"));
        }
        self.reject_bit_field_alignment(node.attributes, None)?;
        let owner = self
            .graph
            .scope(scope)
            .owner
            .ok_or(Error::Invariant("bit-field without class"))?;
        for syntax_index in 0..self.tree.bit_fields.get(fields).len() {
            let syntax_item = self.tree.bit_fields.get(fields)[syntax_index];
            let field = &syntax_item;
            self.reject_bit_field_alignment(List::default(), field.declarator)?;
            let info = field
                .declarator
                .map(|declarator| self.placeholder_declarator(Some(declarator), specs, scope))
                .transpose()?;
            let ty = info.as_ref().map_or(specs.ty, |info| info.ty);
            if self.graph.integral_kind(ty).is_none()
                && !matches!(self.graph.types.get(ty).kind, TypeKind::Enum(_))
                && !(self.template_depth != 0 && self.exception_dependent_type(ty))
            {
                return Err(Error::at(node.location, "bit-field of non-integral type"));
            }
            let width = if self.template_depth == 0 {
                if self.semantic() {
                    self.expression(field.width, scope)?;
                }
                let value = self
                    .constant_value(field.width, scope, true)?
                    .ok_or(Error::Invariant("required bit-field width missing"))?;
                if self.scoped_enum(value.ty)
                    || self
                        .integral_kind(value.ty)
                        .is_some_and(|kind| kind.signed && value.bits.cast_signed() < 0)
                {
                    return Err(Error::at(
                        self.tree.expressions.get(field.width).location,
                        "invalid bit-field width",
                    ));
                }
                value.bits
            } else {
                0
            };
            let entity = if let Some(declarator) = field.declarator {
                if width == 0 && self.template_depth == 0 {
                    return Err(Error::at(node.location, "named bit-field of width zero"));
                }
                self.declare_analyzed_init(
                    specs,
                    ast::InitDeclarator {
                        declarator,
                        initializer: None,
                        tokens: ast::TokenSpan::default(),
                    },
                    scope,
                    &mut None,
                    info.ok_or(Error::Invariant("named bit-field without declarator facts"))?,
                )?;
                let binding = self
                    .graph
                    .declared_bindings
                    .get(declarator.index())
                    .copied()
                    .flatten()
                    .ok_or(Error::Invariant("named bit-field without binding"))?;
                let entity = self.graph.binding(binding).entity;
                if self.graph.entity(owner).members.last() != Some(&entity) {
                    // Static/typedef declarations do not become physical fields.
                    continue;
                }
                let attributes = self.layout_attributes(node.attributes, scope, false)?;
                let attributes =
                    self.declarator_layout_attributes(attributes, declarator, scope)?;
                self.apply_storage_attributes(declarator, attributes)?;
                entity
            } else {
                let entity = self.graph.add_entity(Some(ty), None, Keyword::Int)?;
                self.graph.entity_mut(entity).data_member = Some(DataMemberKind::Instance);
                self.graph.entity_mut(entity).home = Some(scope);
                self.graph.entity_mut(owner).members.push(entity);
                entity
            };
            self.record_bit_field(entity, width)?;
        }
        Ok(())
    }
    fn record_bit_field(&mut self, entity: EntityId, width: u64) -> Result<(), Error> {
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("bit-field without type"))?;
        let kind = self
            .graph
            .integral_kind(ty)
            .ok_or(Error::Invariant("bit-field without integral kind"))?;
        let mut signed = kind.signed && kind.bits > 1;
        if let TypeKind::Enum(owner) = self.graph.types.get(ty).kind {
            let info = self
                .graph
                .entity(owner)
                .enumeration
                .ok_or(Error::Invariant("bit-field enum without facts"))?;
            if !info.fixed {
                signed = info.negative;
            }
        }
        let mut operation = self.bare_type(ty)?;
        if matches!(self.graph.types.get(ty).kind, TypeKind::Scalar(_)) && kind.bits > 1 {
            let width = width.min(u64::from(kind.bits));
            if width < 32 || width == 32 && signed {
                operation = self.graph.types.scalar(super::types::Scalar::Basic(
                    crate::types::FundamentalType::Int,
                ))?;
            } else if width == 32 {
                operation = self.graph.types.scalar(super::types::Scalar::Basic(
                    crate::types::FundamentalType::UnsignedInt,
                ))?;
            }
        }
        let id = self.graph.bit_fields.add(width, operation, signed)?;
        self.graph.entity_mut(entity).bit_field = Some(id);
        Ok(())
    }
}
impl super::graph::Graph {
    pub(super) fn unnamed_bit_field(&self, entity: EntityId) -> bool {
        let node = self.entity(entity);
        node.name.is_none() && node.bit_field.is_some()
    }
}
