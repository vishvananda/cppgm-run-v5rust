// Translated from sema/analyzer_declarations.cpp's declare_variable,
// check_variable_specifiers and earlier_variable. See NOTICE.
//! Data member declaration facts separate object storage from print ownership.
use super::{
    Error,
    analyzer::Analyzer,
    constants::ConstantState,
    declarators::DeclaratorInfo,
    graph::{BindingId, BindingKind, DataMemberKind, Mutability, ScopeId, ScopeKind},
    lookup::Filter,
    specifiers::{DeclarationRole, Specifiers},
    types::TypeKind,
};
use crate::{support::source::SourceOffset, syntax::ast};

impl Analyzer<'_> {
    pub(super) fn typed_variable_initialization(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        home: ScopeId,
        initialized: bool,
    ) -> bool {
        self.semantic()
            && self.template_depth == 0
            && !self.reserved_context.pattern
            && (self.graph.scope(home).kind != ScopeKind::Class
                || info.home.is_some()
                || specs.is_static && initialized)
    }
    pub(super) fn inline_static_member(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        home: ScopeId,
    ) -> bool {
        self.hosted()
            && specs.inline == super::specifiers::InlinePreference::Inline
            && specs.is_static
            && self.graph.scope(home).kind == ScopeKind::Class
            && info.home.is_none()
    }
    pub(super) fn request_in_class_initializer(
        &mut self,
        entity: super::graph::EntityId,
    ) -> Result<(), Error> {
        if let Some(initializer) = self.graph.entity(entity).member_initializer {
            let home = self
                .graph
                .entity(entity)
                .home
                .ok_or(Error::Invariant("static member without class scope"))?;
            self.request_static_initializer(entity, Some(initializer), home, home)?;
        }
        Ok(())
    }
    pub(super) fn record_declared_constant(
        &mut self,
        entity: super::graph::EntityId,
        declaration: ast::InitDeclarator,
        info: &DeclaratorInfo,
    ) -> Result<(), Error> {
        if self.reserved_context.pattern || self.graph.reserved_deferrals.initializer(entity) {
            return Ok(());
        }

        let initializer = declaration
            .initializer
            .ok_or(Error::Invariant("declared constant without initializer"))?;
        self.record_constant_initializer(entity, initializer, info.lookup)?;
        let variable = self.graph.entity(entity);
        let ty = variable
            .ty
            .ok_or(Error::Invariant("static constant without type"))?;
        // C++ leaves ConstantState::Unknown for an ordinary nonconst object;
        // None diagnoses only a failed eligible constant-initialization query.
        // Reference values live in the address store rather than ConstantState.
        let failed_constant = if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
        {
            self.graph.constant_addresses.value(entity).is_none()
        } else {
            self.graph.integral_kind(ty).is_some()
                && self.graph.types.get(ty).cv.0 == 1
                && matches!(variable.constant, ConstantState::None)
        };
        if self.semantic()
            && self.template_depth == 0
            && info.home.is_none()
            && variable.data_member == Some(DataMemberKind::Static)
            && !variable.constexpr
            && failed_constant
        {
            return Err(Error::at(
                self.tree.declarators.get(declaration.declarator).location,
                "a static data member's in-class initializer is not a constant expression",
            ));
        }
        Ok(())
    }
    pub(super) fn static_member_constant(
        &self,
        facts: &mut super::expression_facts::ExprFacts,
        entity: super::graph::EntityId,
    ) {
        let specialization =
            self.template_depth == 0 && self.graph.variable_instances.templates.contains(entity);
        let entity = self.graph.entity(entity);
        if specialization && entity.ty.is_some_and(|ty| self.scoped_enum(ty)) {
            return;
        }
        if (entity.data_member != Some(DataMemberKind::Static) && !specialization)
            || entity.ty.is_some_and(|ty| {
                matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            })
        {
            return;
        }
        if let ConstantState::Value(value) = entity.constant {
            facts.folded = self.held_integral_constant(value).then_some(value.bits);
            facts.fold_style = super::expression_facts::FoldStyle::StaticMember;
        }
    }
    pub(super) fn check_mutable_declarator(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if specs.mutability == Mutability::Mutable
            && (specs.role == DeclarationRole::Typedef
                || matches!(
                    self.graph.types.get(info.ty).kind,
                    TypeKind::Function { .. }
                ))
        {
            return Err(Error::at(
                location,
                "mutable in a typedef or function declaration",
            ));
        }
        Ok(())
    }
    fn record_zero_length_member(
        &mut self,
        entity: super::graph::EntityId,
        ty: super::types::TypeId,
        declarator: ast::DeclaratorId,
    ) {
        if self.hosted()
            && self.graph.entity(entity).data_member == Some(super::graph::DataMemberKind::Instance)
            && matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Array { bound: 0, .. }
            )
        {
            // The suffix closest to the declarator-id owns this member array.
            let mut declarator = Some(declarator);
            let mut own = None;
            while let Some(id) = declarator {
                let node = self.tree.declarators.get(id);
                if let Some(suffix) = self.tree.suffixes.get(node.suffixes).first() {
                    own = Some(*suffix);
                }
                declarator = node.nested;
            }
            self.graph.entity_mut(entity).array_member =
                if matches!(own, Some(ast::Suffix::Array(Some(_)))) {
                    super::aggregate_initialization::ArrayMemberKind::ZeroLength
                } else {
                    super::aggregate_initialization::ArrayMemberKind::Ordinary
                };
        }
    }
    pub(super) fn record_data_member(
        &mut self,
        binding: BindingId,
        specs: Specifiers,
        info: &DeclaratorInfo,
        init: ast::InitDeclarator,
        home: ScopeId,
    ) -> Result<(), Error> {
        let entity = self.graph.binding(binding).entity;
        if self.graph.scope(home).kind == ScopeKind::Class && info.home.is_none() {
            self.graph.entity_mut(entity).member_access = self.graph.binding_access(binding);
            self.graph.entity_mut(entity).member_initializer = init.initializer;
            self.graph.entity_mut(entity).mutability = specs.mutability;
            self.graph.entity_mut(entity).data_member = Some(if specs.is_static {
                DataMemberKind::Static
            } else {
                DataMemberKind::Instance
            });
        }
        if self.graph.scope(home).kind == ScopeKind::Class
            && !specs.is_static
            && info.home.is_none()
        {
            let owner = self
                .graph
                .scope(home)
                .owner
                .ok_or(Error::Invariant("class scope without owner"))?;
            self.record_zero_length_member(entity, info.ty, init.declarator);
            self.graph.entity_mut(owner).members.push(entity);
            if init.initializer.is_some()
                || self.graph.binding_access(binding) != super::graph::Access::Public
                || matches!(
                    self.graph.types.get(info.ty).kind,
                    TypeKind::Reference { .. }
                )
            {
                self.graph.entity_mut(owner).layout_category =
                    super::layout::LayoutCategory::NonPod;
            }
            if init.initializer.is_some()
                || self.graph.binding_access(binding) != super::graph::Access::Public
            {
                self.graph.entity_mut(owner).aggregation =
                    super::aggregate_initialization::Aggregation::Ineligible;
            }
        }
        Ok(())
    }
    pub(super) fn check_mutable_variable(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let in_class = self.graph.scope(home).kind == ScopeKind::Class;
        if specs.mutability == Mutability::Mutable {
            let mut object = info.ty;
            while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
                object = element;
            }
            if !in_class
                || specs.is_static
                || self.graph.types.get(object).cv.0 & 1 != 0
                || matches!(
                    self.graph.types.get(info.ty).kind,
                    TypeKind::Reference { .. }
                )
            {
                return Err(Error::at(
                    location,
                    "mutable on a variable that is no non-const, non-reference data member",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn finish_class_initializers(
        &mut self,
        class: super::graph::EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.semantic() || self.template_depth != 0 {
            return Ok(());
        }
        // MemberInitialized counts an anonymous member object's initialized
        // children as one initialized direct member. Each child is already complete.
        let initialized = self
            .graph
            .entity(class)
            .members
            .iter()
            .filter(|&&member| {
                let member = self.graph.entity(member);
                member.member_initializer.is_some()
                    || member.name.is_none()
                        && member.ty.is_some_and(|ty| {
                            matches!(self.graph.types.get(ty).kind, TypeKind::Class(inner)
                        if self.graph.entity(inner).anonymous_class.is_some()
                            && self.graph.entity(inner).initialized_members != 0)
                        })
            })
            .count();
        self.graph.entity_mut(class).initialized_members = initialized;
        if self.graph.entity(class).key == crate::lex::vocabulary::Keyword::Union && initialized > 1
        {
            return Err(Error::at(
                location,
                "a union has several default member initializers",
            ));
        }
        Ok(())
    }
    pub(super) fn check_data_member_declaration(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        initializer: Option<ast::Initializer>,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<Option<BindingId>, Error> {
        let name = info
            .name
            .ok_or(Error::Invariant("data declaration without name"))?;
        let in_class = self.graph.scope(home).kind == ScopeKind::Class;
        if self.graph.scope(home).kind == ScopeKind::Block
            && info.home.is_none()
            && let Some(previous) = self.graph.own(home, name, Filter::Ordinary)
            && self.graph.binding(previous).kind == BindingKind::Variable
            && (!specs.is_extern
                || !self
                    .graph
                    .entity(self.graph.binding(previous).entity)
                    .variable
                    .is_some_and(|properties| properties.is_extern))
        {
            return Err(Error::at(
                location,
                "redeclaration of a name in the same scope",
            ));
        }
        if in_class {
            let owner = self
                .graph
                .scope(home)
                .owner
                .ok_or(Error::Invariant("class without owner"))?;
            if self.graph.entity(owner).name == Some(name)
                && (info.home.is_some()
                    || specs.is_static
                    || self.graph.entity(owner).user_constructor.is_some())
            {
                return Err(Error::at(location, "invalid variable name"));
            }
            if info.home.is_none() && specs.constexpr && !specs.is_static {
                return Err(Error::at(location, "a constexpr non-static data member"));
            }
            if info.home.is_none() && specs.constexpr && initializer.is_none() {
                return Err(Error::at(
                    location,
                    "a constexpr static data member declared without an initializer",
                ));
            }
        }
        if let Some(qualified) = info.home {
            let previous = self
                .qualified(qualified, name, Filter::Ordinary)?
                .ok_or_else(|| {
                    Error::at(
                        location,
                        "qualified definition does not match a declaration",
                    )
                })?;
            let binding = self.graph.binding(previous);
            if binding.kind != BindingKind::Variable {
                return Err(Error::at(
                    location,
                    "qualified definition does not match a declaration",
                ));
            }
            if in_class && specs.is_static {
                return Err(Error::at(
                    location,
                    "static on a static data member's definition outside its class",
                ));
            }
            let entity = self.graph.entity(binding.entity);
            if entity.data_member == Some(DataMemberKind::Instance) {
                return Err(Error::at(
                    location,
                    "an out-of-class definition names a non-static data member",
                ));
            }
            if entity.data_member == Some(DataMemberKind::Static)
                && (entity.constexpr || matches!(entity.constant, ConstantState::Value(_)))
                && initializer.is_some()
            {
                return Err(Error::at(
                    location,
                    "the definition of a static data member initialized in its class repeats the initializer",
                ));
            }
            return Ok(Some(previous));
        }
        Ok(None)
    }

    pub(super) fn check_static_member_initializer(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        initializer: Option<ast::Initializer>,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if specs.is_static
            && info.home.is_none()
            && initializer.is_some()
            && self.graph.scope(home).kind == ScopeKind::Class
            && !specs.constexpr
            && !self.inline_static_member(specs, info, home)
            && self.template_depth == 0
            && (self.graph.types.get(info.ty).cv.0 & 1 == 0
                || (self.graph.integral_kind(info.ty).is_none()
                    && !matches!(self.graph.types.get(info.ty).kind, TypeKind::Enum(_))))
        {
            return Err(Error::at(
                location,
                "a static data member initialized in its class must be const integral or constexpr",
            ));
        }
        Ok(())
    }
}
