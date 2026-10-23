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
    pub(super) alignment: u64,
    packed: bool,
    standard: Option<SourceOffset>,
    pub no_inline: bool,
    pub effects: super::graph::FunctionEffect,
    pub instantiation: super::explicit_instantiations::MemberPolicy,
}
impl Analyzer<'_> {
    pub(super) fn reject_no_return(
        &self,
        list: List<ast::Attribute>,
        message: &'static str,
    ) -> Result<(), Error> {
        if let Some(attribute) = self.tree.attributes.get(list).iter().find(|attribute| {
            attribute.kind == ast::AttributeKind::NoReturn
                && attribute.form == ast::AttributeForm::Standard
        }) {
            return Err(Error::at(attribute.location, message));
        }
        Ok(())
    }
    // analyzer_declarations.cpp::effect_attributes / StandardNoReturn.
    // The fact belongs to the callable entity, never its canonical function
    // type: calls through pointers must not inherit a declaration's guarantee.
    pub(super) fn declaration_return_attributes(
        &mut self,
        function: Option<EntityId>,
        leading: List<ast::Attribute>,
        declarator: ast::DeclaratorId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let mut requested = false;
        let mut apply = |list| {
            for attribute in self.tree.attributes.get(list) {
                if attribute.kind != ast::AttributeKind::NoReturn {
                    continue;
                }
                if attribute.form == ast::AttributeForm::Standard {
                    if function.is_none() {
                        return Err(Error::at(
                            location,
                            "noreturn attribute requires a function",
                        ));
                    }
                    if attribute.site != ast::AttributeSite::Entity {
                        return Err(Error::at(
                            attribute.location,
                            "noreturn attribute applied to a type",
                        ));
                    }
                }
                requested = true;
            }
            Ok(())
        };
        apply(leading)?;
        let mut part = Some(declarator);
        while let Some(id) = part {
            let node = self.tree.declarators.get(id);
            apply(node.attributes)?;
            part = node.nested;
        }
        if requested && let Some(entity) = function {
            self.graph
                .entity_mut(entity)
                .function
                .as_mut()
                .ok_or(Error::Invariant("return attribute owner without function"))?
                .returns = super::graph::FunctionReturn::NoReturn;
        }
        Ok(())
    }

    pub(super) fn declarator_effects(
        &self,
        mut effect: super::graph::FunctionEffect,
        declarator: ast::DeclaratorId,
    ) -> super::graph::FunctionEffect {
        let mut part = Some(declarator);
        while let Some(id) = part {
            let node = self.tree.declarators.get(id);
            for attribute in self.tree.attributes.get(node.attributes) {
                effect = effect.max(match attribute.kind {
                    ast::AttributeKind::Const => super::graph::FunctionEffect::ReadNone,
                    ast::AttributeKind::Pure => super::graph::FunctionEffect::ReadOnly,
                    _ => super::graph::FunctionEffect::Default,
                });
            }
            part = node.nested;
        }
        effect
    }
    pub(super) fn note_instantiation_policy(
        &mut self,
        entity: EntityId,
        declarator: ast::DeclaratorId,
        mut policy: super::explicit_instantiations::MemberPolicy,
    ) {
        let mut current = Some(declarator);
        while let Some(id) = current {
            let node = self.tree.declarators.get(id);
            for attribute in self.tree.attributes.get(node.attributes) {
                policy.excluded |=
                    attribute.kind == ast::AttributeKind::ExcludeFromExplicitInstantiation;
                policy.always_inline |= attribute.kind == ast::AttributeKind::AlwaysInline;
                policy.gnu_inline |= attribute.kind == ast::AttributeKind::GnuInline;
            }
            current = node.nested;
        }
        self.graph
            .function_templates
            .instantiations
            .note_policy(entity, policy);
    }
    pub(super) fn layout_attributes(
        &mut self,
        list: List<ast::Attribute>,
        scope: ScopeId,
        class_site: bool,
    ) -> Result<LayoutAttributes, Error> {
        let mut result = LayoutAttributes::default();
        for syntax_index in 0..self.tree.attributes.get(list).len() {
            let syntax_item = self.tree.attributes.get(list)[syntax_index];
            let attribute = &syntax_item;
            if (attribute.site == ast::AttributeSite::Class) != class_site {
                continue;
            }
            if self.template_depth != 0
                && !matches!(
                    attribute.kind,
                    ast::AttributeKind::NoInline
                        | ast::AttributeKind::Pure
                        | ast::AttributeKind::Const
                        | ast::AttributeKind::AlwaysInline
                        | ast::AttributeKind::GnuInline
                        | ast::AttributeKind::ExcludeFromExplicitInstantiation
                )
            {
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
                            self.integral_constant_operand(expression, scope, None)?;
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
                ast::AttributeKind::NoInline => result.no_inline = true,
                ast::AttributeKind::Pure => {
                    result.effects = result.effects.max(super::graph::FunctionEffect::ReadOnly);
                }
                ast::AttributeKind::Const => {
                    result.effects = super::graph::FunctionEffect::ReadNone;
                }
                ast::AttributeKind::AlwaysInline => result.instantiation.always_inline = true,
                ast::AttributeKind::GnuInline => result.instantiation.gnu_inline = true,
                ast::AttributeKind::ExcludeFromExplicitInstantiation => {
                    result.instantiation.excluded = true;
                }
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
            let syntax_item = *self.tree.declarators.get(id);
            let node = &syntax_item;
            let own = self.layout_attributes(node.attributes, scope, false)?;
            attributes.alignment = attributes.alignment.max(own.alignment);
            attributes.packed |= own.packed;
            attributes.no_inline |= own.no_inline;
            attributes.effects = attributes.effects.max(own.effects);
            attributes.instantiation.excluded |= own.instantiation.excluded;
            attributes.instantiation.always_inline |= own.instantiation.always_inline;
            attributes.instantiation.gnu_inline |= own.instantiation.gnu_inline;
            attributes.standard = attributes.standard.or(own.standard);
            part = node.nested;
        }
        Ok(attributes)
    }
    pub(super) fn apply_storage_attributes(
        &mut self,
        declarator: ast::DeclaratorId,
        specs: super::specifiers::Specifiers,
        attributes: LayoutAttributes,
    ) -> Result<(), Error> {
        let binding = self
            .graph
            .declared_bindings
            .get(declarator.index())
            .copied()
            .flatten();
        let Some(binding) = binding else {
            // C++ leaves init.declared null for a partial or a deferred
            // template-id outside its head's scope. There is no object here.
            if self
                .graph
                .variable_instances
                .nonobject_declaration(declarator)
            {
                return Ok(());
            }
            return Err(Error::Invariant("attributes without declaration"));
        };
        let declared = binding;
        let binding = self.graph.binding(binding);
        if binding.kind == BindingKind::Function {
            self.graph
                .note_effects(binding.entity, attributes.effects)?;
            self.graph
                .function_templates
                .instantiations
                .note_policy(binding.entity, attributes.instantiation);
            self.graph
                .note_no_inline(binding.entity, attributes.no_inline)?;
        }
        if binding.kind != BindingKind::Variable {
            if let Some(location) = attributes.standard {
                return Err(Error::at(
                    location,
                    "an alignment-specifier on a typedef or function declaration",
                ));
            }
            return self.record_alias_alignment(specs, declared, attributes.alignment);
        }
        self.record_alias_alignment(specs, declared, 0)?;
        let ty = binding
            .ty
            .ok_or(Error::Invariant("aligned variable without type"))?;
        if let Some(location) = attributes.standard
            && attributes.alignment != 0
            && !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            && self.query_complete(ty)?
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
        for syntax_index in 0..self.tree.attributes.get(list).len() {
            let syntax_item = self.tree.attributes.get(list)[syntax_index];
            let attribute = &syntax_item;
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
        declarator: Option<ast::DeclaratorId>,
    ) -> Result<(), Error> {
        let check = |list| {
            for syntax_index in 0..self.tree.attributes.get(list).len() {
                let syntax_item = self.tree.attributes.get(list)[syntax_index];
                let attribute = &syntax_item;
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
        let mut part = declarator;
        while let Some(id) = part {
            let syntax_item = *self.tree.declarators.get(id);
            let node = &syntax_item;
            check(node.attributes)?;
            part = node.nested;
        }
        Ok(())
    }
}
pub(super) fn target_alignment(value: u64) -> Result<u32, Error> {
    // The frozen semantic record stores a target uint32_t request.
    u32::try_from(value & u64::from(u32::MAX)).map_err(|_| Error::Invariant("alignment truncation"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn return_guarantees_belong_to_entities_and_specializations_keep_own_declarations() {
        use super::super::{graph::FunctionReturn as R, types::Scalar};
        let source = SourceBuffer::new(
            b"void stop(); [[noreturn]] void stop(); void plain();\
              void stop(){throw 1;}\
              template<class T> [[noreturn]] void raise(T x){throw x;}\
              template<> void raise<char>(char){}\
              void use(){raise(1);raise('x');}\
              template<class T>int value;\
              template<class T>[[noreturn]]int value<T*> = 1;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"return-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let stop = cursor.intern_name(b"stop").unwrap();
        let plain = cursor.intern_name(b"plain").unwrap();
        let raise = cursor.intern_name(b"raise").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let stop = graph
            .functions_at(graph.global, stop.into())
            .next()
            .unwrap();
        let plain = graph
            .functions_at(graph.global, plain.into())
            .next()
            .unwrap();
        assert_eq!(graph.binding(stop).ty, graph.binding(plain).ty);
        assert!(graph.lower_no_return(graph.binding(stop).entity));
        assert!(!graph.lower_no_return(graph.binding(plain).entity));
        let mut instances = 0;
        for (&entity, arguments) in &graph.function_templates.arguments {
            let function = graph.entity(entity);
            if function.name != Some(raise) {
                continue;
            }
            let expected = match graph.types.get(arguments[0]).kind {
                TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Char)) => {
                    R::MayReturn
                }
                TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Int)) => R::NoReturn,
                _ => panic!("unexpected specialization argument"),
            };
            assert!(function.function.unwrap().returns == expected);
            instances += 1;
        }
        assert_eq!(instances, 2);
    }
}
