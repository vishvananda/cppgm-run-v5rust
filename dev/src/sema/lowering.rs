// Translated from sema/semantic_unit.h's lowering facts. See NOTICE.
//! Read-only views of analyzed facts; the syntax and semantic stores remain their owners.
use super::{
    Error,
    graph::{BindingKind, Graph},
    types::TypeKind,
};
pub(crate) use super::{
    graph::{EntityId, ScopeId},
    types::{Scalar, TypeId},
};
use crate::{lex::identifiers::NameId, syntax::ast};

pub(crate) enum TypeView<'a> {
    Scalar(Scalar),
    Pointer(TypeId),
    Reference,
    Enum(TypeId),
    Function {
        result: TypeId,
        parameters: &'a [TypeId],
        variadic: bool,
    },
    Unsupported,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Emission {
    Always,
    Demanded,
}
pub(crate) struct FunctionFacts {
    pub name: NameId,
    pub home: ScopeId,
    pub ty: TypeId,
    pub scope: Option<ScopeId>,
    pub internal: bool,
    pub c_linkage: bool,
    pub emission: Emission,
    pub nothrow: bool,
    pub definition: Option<ast::DeclId>,
}
#[derive(Clone, Copy)]
pub(crate) struct Expression {
    pub ty: TypeId,
    pub target: Option<TypeId>,
    pub folded: Option<u64>,
    pub query: Option<u64>,
    pub is_place: bool,
    pub null_pointer: bool,
}
impl Graph {
    pub(crate) fn lower_arithmetic_types(
        &self,
        id: ast::ExprId,
    ) -> Result<(TypeId, TypeId), Error> {
        self.arithmetic_types(id)
    }
    pub(crate) fn lower_case(&self, id: ast::StmtId) -> Result<u64, Error> {
        self.case_values
            .get(id.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("case without converted value"))
    }
    pub(crate) fn lower_condition_target(&self, id: ast::DeclaratorId) -> Result<TypeId, Error> {
        Ok(self.condition_conversion(id)?.result)
    }
    pub(crate) fn lower_automatic(&self, id: EntityId) -> bool {
        self.entity(id)
            .variable
            .is_some_and(|properties| properties.automatic)
    }
    pub(crate) fn lower_is_variable(&self, id: EntityId) -> bool {
        self.entity(id).variable.is_some()
    }
    pub(crate) fn lower_local_name(&self, id: EntityId) -> Option<NameId> {
        self.entity(id).name
    }

    pub(crate) fn lower_declared(&self, id: ast::DeclaratorId) -> Result<EntityId, Error> {
        self.declared_bindings
            .get(id.index())
            .copied()
            .flatten()
            .map(|binding| self.binding(binding).entity)
            .ok_or(Error::Invariant("lowering declarator without binding"))
    }
    pub(crate) fn lower_function(&self, id: EntityId) -> Result<FunctionFacts, Error> {
        let entity = self.entity(id);
        let function = entity
            .function
            .ok_or(Error::Invariant("lowering non-function"))?;
        Ok(FunctionFacts {
            name: entity
                .name
                .ok_or(Error::Missing("lowering special function names"))?,
            home: entity
                .home
                .ok_or(Error::Invariant("function without home"))?,
            ty: entity.ty.ok_or(Error::Invariant("function without type"))?,
            scope: entity.scope,
            internal: function.internal_linkage,
            c_linkage: function.language == super::linkage::Language::C,
            emission: if entity.definition_emission
                == super::function_demands::DefinitionEmission::Demanded
                || entity.inline == super::specifiers::InlinePreference::Inline
            {
                Emission::Demanded
            } else {
                Emission::Always
            },
            nothrow: matches!(
                &entity.exceptions,
                super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None)
            ),
            definition: entity.function_definition,
        })
    }
    pub(crate) fn lower_namespace(
        &self,
        id: ScopeId,
    ) -> Result<(Option<ScopeId>, Option<NameId>), Error> {
        let scope = self.scope(id);
        if scope.kind != super::graph::ScopeKind::Namespace {
            return Err(Error::Missing("lowering class and local scopes"));
        }
        Ok((scope.parent, scope.name))
    }
    pub(crate) fn lower_overloads(
        &self,
        id: EntityId,
    ) -> Result<impl Iterator<Item = EntityId> + '_, Error> {
        let entity = self.entity(id);
        let home = entity
            .home
            .ok_or(Error::Invariant("function without home"))?;
        let name = entity
            .name
            .ok_or(Error::Missing("special name overload family"))?;
        Ok(self
            .functions_at(home, name.into())
            .map(|binding| self.binding(binding).entity))
    }
    pub(crate) fn lower_parameters(
        &self,
        scope: Option<ScopeId>,
    ) -> impl Iterator<Item = (EntityId, Option<NameId>, TypeId)> + '_ {
        scope
            .into_iter()
            .flat_map(|id| self.scope(id).bindings.iter())
            .filter_map(|&id| {
                let binding = self.binding(id);
                (binding.kind == BindingKind::Parameter).then_some(binding)
            })
            .filter_map(|binding| binding.ty.map(|ty| (binding.entity, binding.name, ty)))
    }
    pub(crate) fn lower_type(&self, id: TypeId) -> Result<TypeView<'_>, Error> {
        Ok(match &self.types.get(id).kind {
            TypeKind::Scalar(scalar) => TypeView::Scalar(*scalar),
            TypeKind::Pointer(element) => TypeView::Pointer(*element),
            TypeKind::Reference { .. } => TypeView::Reference,
            TypeKind::Enum(entity) => TypeView::Enum(
                self.entity(*entity)
                    .enumeration
                    .and_then(|info| info.underlying)
                    .ok_or(Error::Invariant("enum without underlying type"))?,
            ),
            TypeKind::Function {
                result,
                parameters,
                variadic,
                ..
            } => TypeView::Function {
                result: *result,
                parameters,
                variadic: *variadic,
            },
            _ => TypeView::Unsupported,
        })
    }
    pub(crate) fn lower_cv(&self, id: TypeId) -> u8 {
        self.types.get(id).cv.0
    }
    pub(crate) fn lower_expression(&self, id: ast::ExprId) -> Result<Expression, Error> {
        let facts = self.expression_facts.get(id)?;
        Ok(Expression {
            ty: facts.ty,
            target: facts.conversion.map(|c| c.result),
            folded: facts.folded,
            query: facts.query_value,
            is_place: facts.category != super::expression_facts::ValueCategory::Prvalue,
            null_pointer: facts
                .conversion
                .is_some_and(|c| c.second == super::expression_facts::ValueConversion::NullPointer),
        })
    }
    pub(crate) fn lower_name(&self, name: ast::Name) -> Result<EntityId, Error> {
        self.resolved_name(name)
            .map(|id| self.binding(id).entity)
            .ok_or(Error::Invariant("lowering name without entity"))
    }
    pub(crate) fn lower_entity_type(&self, id: EntityId) -> Result<TypeId, Error> {
        self.entity(id)
            .ty
            .ok_or(Error::Invariant("lowering entity without type"))
    }
    pub(crate) fn lower_call(&self, id: ast::ExprId) -> Result<EntityId, Error> {
        if !matches!(
            self.expression_facts.get(id)?.call,
            Some(super::function_resolution::CallTarget::Direct(_))
        ) {
            return Err(Error::Missing("indirect call lowering"));
        }
        self.call_entity(id)
    }
    pub(crate) fn lower_default(&self, entity: EntityId, index: usize) -> Option<ast::ExprId> {
        self.function_default(entity, index)
    }
    pub(crate) fn lower_statement(&self, id: ast::StmtId) -> Result<ast::StmtId, Error> {
        self.statement_readings
            .get(id.index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("ambiguous statement without reading"))
    }
}
