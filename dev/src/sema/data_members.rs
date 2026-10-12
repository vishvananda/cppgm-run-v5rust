// Translated from sema/analyzer_declarations.cpp's declare_variable,
// check_variable_specifiers and earlier_variable. See NOTICE.
//! Data member declaration facts separate object storage from print ownership.
use super::{
    Error,
    analyzer::Analyzer,
    constants::ConstantState,
    declarators::DeclaratorInfo,
    graph::{BindingKind, DataMemberKind, ScopeId, ScopeKind},
    lookup::Filter,
    specifiers::Specifiers,
    types::TypeKind,
};
use crate::{support::source::SourceOffset, syntax::ast};

impl Analyzer<'_> {
    pub(super) fn check_data_member_declaration(
        &self,
        specs: Specifiers,
        info: &DeclaratorInfo,
        initializer: Option<ast::Initializer>,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let name = info
            .name
            .ok_or(Error::Invariant("data declaration without name"))?;
        let in_class = self.graph.scope(home).kind == ScopeKind::Class;
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
                .graph
                .own(qualified, name, Filter::Ordinary)
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
        }
        Ok(())
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
