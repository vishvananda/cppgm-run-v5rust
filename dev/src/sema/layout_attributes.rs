// Translated from analyzer_classes.cpp::attribute_alignment and
// analyzer_declarations.cpp::request_alignment. See NOTICE.
//! Attribute operands are analyzed once; layout and storage consume typed requests.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, EntityId, ScopeId},
    types::TypeKind,
};
use crate::{
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};

#[derive(Clone, Copy, Default)]
pub(super) struct StorageAttributes {
    pub alignment: u32,
    pub packed: bool,
    pub standard_alignment: bool,
}

#[derive(Clone, Copy, Default)]
pub(super) struct LayoutAttributes {
    alignment: u64,
    packed: bool,
    standard: Option<SourceOffset>,
}
impl Analyzer<'_> {
    pub(super) fn layout_attributes(
        &mut self,
        list: List<ast::Attribute>,
        scope: ScopeId,
        class_site: bool,
    ) -> Result<LayoutAttributes, Error> {
        let mut result = LayoutAttributes::default();
        if self.template_depth != 0 {
            return Ok(result);
        }
        for attribute in self.tree.attributes.get(list) {
            if (attribute.site == ast::AttributeSite::Class) != class_site {
                continue;
            }
            match attribute.kind {
                ast::AttributeKind::Alignas => {
                    if attribute.site == ast::AttributeSite::Type
                        && attribute.form != ast::AttributeForm::Gnu
                    {
                        return Err(Error::at(
                            attribute.location,
                            "an alignment-specifier applied to a type",
                        ));
                    }
                    let alignment = if let Some(ty) = attribute.ty {
                        let ty = self.type_syntax(ty, scope)?;
                        self.type_size(ty, attribute.location)?.1
                    } else if let Some(expression) = attribute.expression {
                        if self.semantic() {
                            self.ensure_typed(expression, scope)?;
                            self.check_resolved_function(expression)?;
                        }
                        let constant = self
                            .constant_value(expression, scope, true)?
                            .ok_or(Error::Invariant("missing alignment constant"))?;
                        let value = constant.bits;
                        if value & value.wrapping_sub(1) != 0 {
                            return Err(Error::at(
                                attribute.location,
                                "an alignment that is not a power of two",
                            ));
                        }
                        value
                    } else {
                        16
                    };
                    result.alignment = result.alignment.max(alignment);
                    if attribute.form != ast::AttributeForm::Gnu && result.standard.is_none() {
                        result.standard = Some(attribute.location);
                    }
                }
                ast::AttributeKind::Packed => result.packed = true,
                _ => (),
            }
        }
        Ok(result)
    }
    pub(super) fn declarator_layout_attributes(
        &mut self,
        mut attributes: LayoutAttributes,
        declarator: ast::DeclaratorId,
        scope: ScopeId,
    ) -> Result<LayoutAttributes, Error> {
        let mut part = Some(declarator);
        while let Some(id) = part {
            let node = self.tree.declarators.get(id);
            let own = self.layout_attributes(node.attributes, scope, false)?;
            attributes.alignment = attributes.alignment.max(own.alignment);
            attributes.packed |= own.packed;
            attributes.standard = attributes.standard.or(own.standard);
            part = node.nested;
        }
        Ok(attributes)
    }
    pub(super) fn apply_storage_attributes(
        &mut self,
        declarator: ast::DeclaratorId,
        attributes: LayoutAttributes,
    ) -> Result<(), Error> {
        let binding = self
            .graph
            .declared_bindings
            .get(declarator.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("attributes without declaration"))?;
        let binding = self.graph.binding(binding);
        if binding.kind != BindingKind::Variable {
            if let Some(location) = attributes.standard {
                return Err(Error::at(
                    location,
                    "an alignment-specifier on a typedef or function declaration",
                ));
            }
            if binding.kind == BindingKind::Alias && attributes.alignment != 0 {
                return Err(Error::Missing("aligned typedef storage"));
            }
            return Ok(());
        }
        let ty = binding
            .ty
            .ok_or(Error::Invariant("aligned variable without type"))?;
        if let Some(location) = attributes.standard
            && attributes.alignment != 0
            && !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            && self.query_complete(ty)
            && attributes.alignment < self.type_size(ty, location)?.1
        {
            return Err(Error::at(
                location,
                "an alignment-specifier weaker than the alignment of the entity's type",
            ));
        }
        let old = &mut self.graph.entity_mut(binding.entity).storage_attributes;
        if attributes.alignment > u64::from(old.alignment) {
            old.alignment = target_alignment(attributes.alignment)?;
        }
        old.packed |= attributes.packed;
        Ok(())
    }
    pub(super) fn class_layout_attributes(
        &mut self,
        class: EntityId,
        list: List<ast::Attribute>,
        scope: ScopeId,
        class_site: bool,
    ) -> Result<(), Error> {
        for attribute in self.tree.attributes.get(list) {
            if (attribute.site == ast::AttributeSite::Class) == class_site
                && attribute.kind == ast::AttributeKind::NoReturn
                && attribute.form == ast::AttributeForm::Standard
            {
                return Err(Error::at(
                    attribute.location,
                    "noreturn attribute on a class",
                ));
            }
        }
        if class_site && self.graph.entity(class).defined {
            return Ok(());
        }
        let attributes = self.layout_attributes(list, scope, class_site)?;
        let old = &mut self.graph.entity_mut(class).storage_attributes;
        old.alignment = old.alignment.max(target_alignment(attributes.alignment)?);
        if !class_site {
            old.packed = attributes.packed;
            old.standard_alignment = attributes.standard.is_some();
        }
        Ok(())
    }
    pub(super) fn reject_bit_field_alignment(
        &self,
        leading: List<ast::Attribute>,
        fields: List<ast::BitField>,
    ) -> Result<(), Error> {
        let check = |list| {
            for attribute in self.tree.attributes.get(list) {
                if attribute.kind == ast::AttributeKind::Alignas {
                    return Err(Error::at(
                        attribute.location,
                        "an alignment-specifier on a bit-field",
                    ));
                }
            }
            Ok(())
        };
        check(leading)?;
        for field in self.tree.bit_fields.get(fields) {
            let mut part = field.declarator;
            while let Some(id) = part {
                let node = self.tree.declarators.get(id);
                check(node.attributes)?;
                part = node.nested;
            }
        }
        Ok(())
    }
}
fn target_alignment(value: u64) -> Result<u32, Error> {
    // The frozen semantic record stores a target uint32_t request.
    u32::try_from(value & u64::from(u32::MAX)).map_err(|_| Error::Invariant("alignment truncation"))
}
