// Translated from template_packs.cpp::DefinitionPacks/expression_packs and
// syntax/ast_clone.cpp's collect_packs view. See NOTICE.
//! Read parsed operands in their definition scope; nested expansions own their names.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, ScopeId},
    lookup::Filter,
};
use crate::syntax::{
    ParseError, ast,
    cloning::{Cloner, PackExpander},
};

impl Analyzer<'_> {
    pub(super) fn instantiation_dependent_operand(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let mut collector = DefinitionNames;
        let uses = Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .with_expander(&mut collector)
            .expression_name_uses(expression)
            .map_err(|error| super::template_bodies::copy_error(&error))?;
        // The collection performs every lookup before testing dependence.
        // Repeated occurrences and ambiguous results remain distinct queries.
        let mut bindings = Vec::with_capacity(uses.names.len());
        for name in uses.names {
            let found =
                self.unqualified_declaration_result(scope, name.into(), Filter::Ordinary)?;
            if let Some(binding) = found.binding {
                bindings.push((!found.ambiguous).then_some(binding));
            }
        }
        if uses.expansion || uses.pack_size || uses.this && self.dependent_class_scope(scope) {
            return Ok(true);
        }
        for binding in bindings {
            let Some(id) = binding else { return Ok(true) };
            let binding = self.graph.binding(id);
            let entity = self.graph.entity(binding.entity);
            if entity.context().template_parameter.is_some() {
                return Ok(true);
            }
            let owner = self.graph.binding_scope(id);
            let member = self.graph.scope(owner).kind == super::graph::ScopeKind::Class
                && self.dependent_class_scope(owner);
            let dependent = match binding.kind {
                BindingKind::Type | BindingKind::Alias | BindingKind::Parameter => binding
                    .ty
                    .is_none_or(|ty| self.exception_dependent_type(ty)),
                BindingKind::Variable => binding.ty.is_none_or(|ty| {
                    self.exception_dependent_type(ty)
                        || member
                            && (entity.object().data_member
                                == Some(super::graph::DataMemberKind::Instance)
                                || self.graph.types.get(ty).cv.0 & super::types::Cv::CONST.0 != 0)
                }),
                BindingKind::Namespace | BindingKind::NamespaceAlias => false,
                _ => member,
            };
            if dependent {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn operand_pack_names(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(Option<super::types::TypeId>, Option<super::types::TypeId>), Error> {
        let uses = self.definition_pack_partitions(expression, scope, DefinitionKind::Template)?;
        let outer = self.canonical_definition_packs(&uses.names, scope)?;
        let outer = if outer.is_empty() {
            None
        } else {
            Some(self.graph.types.intern(
                super::types::TypeKind::Pack(outer),
                super::types::Cv::default(),
            )?)
        };
        let nested = self.canonical_definition_packs(&uses.nested_names, scope)?;
        let mut nested = if nested.is_empty() {
            None
        } else {
            Some(self.graph.types.intern(
                super::types::TypeKind::Pack(nested),
                super::types::Cv::default(),
            )?)
        };
        if uses.nested_expansion && outer.is_some() {
            let parameters =
                self.definition_pack_partitions(expression, scope, DefinitionKind::Parameter)?;
            if !parameters.nested_names.is_empty() {
                let mut all = if let Some(nested) = nested {
                    self.graph.types.pack_elements(nested)?.to_vec()
                } else {
                    Vec::new()
                };
                for name in parameters.nested_names {
                    let found =
                        self.unqualified_declaration_result(scope, name.into(), Filter::Ordinary)?;
                    let binding = found
                        .binding
                        .ok_or(Error::Invariant("nested parameter pack without binding"))?;
                    let ty = self
                        .graph
                        .binding(binding)
                        .ty
                        .ok_or(Error::Invariant("nested parameter pack without type"))?;
                    let super::types::TypeKind::Expansion(pattern) = self.graph.types.get(ty).kind
                    else {
                        return Err(Error::Invariant("nested parameter pack without expansion"));
                    };
                    all.push(pattern);
                }
                nested = Some(self.graph.types.intern(
                    super::types::TypeKind::Pack(all),
                    super::types::Cv::default(),
                )?);
            }
        }
        Ok((outer, nested))
    }
    fn definition_pack_partitions(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        kind: DefinitionKind,
    ) -> Result<crate::syntax::cloning::NameUses, Error> {
        let view = super::lookup::DeclarationLookup::new(
            &self.graph,
            &self.dependent_uses,
            &self.value_parameter_uses,
        );
        let mut finder = DefinitionPacks {
            lookup: view,
            graph: &self.graph,
            scope,
            kind,
            error: std::cell::RefCell::new(None),
        };
        let result = Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .with_expander(&mut finder)
            .expression_pack_partitions(expression);
        if let Some(error) = finder.error.into_inner() {
            return Err(error);
        }
        result.map_err(|error| super::template_bodies::copy_error(&error))
    }
    fn canonical_definition_packs(
        &mut self,
        names: &[crate::lex::identifiers::NameId],
        scope: ScopeId,
    ) -> Result<Vec<super::types::TypeId>, Error> {
        let mut packs = Vec::with_capacity(names.len());
        for &name in names {
            let found =
                self.unqualified_declaration_result(scope, name.into(), Filter::Ordinary)?;
            let binding = found
                .binding
                .ok_or(Error::Invariant("definition pack without binding"))?;
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.context().template_parameter.is_none() {
                return Err(Error::Invariant("definition pack without parameter"));
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("definition pack without parameter type"))?;
            packs.push(self.graph.types.canonical_function(ty)?);
        }
        Ok(packs)
    }

    #[cfg(test)]
    pub(super) fn expression_names_pack(
        &mut self,
        root: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        self.operand_pack_names(root, scope)
            .map(|(outer, _)| outer.is_some())
    }
}

// Collection needs every free leading identifier; semantic lookup then decides
// whether it denotes a definition-owned pack. Lengths are never consumed and
// the cloner does not substitute names in this mode. Its lexical shadows and
// nested-expansion boundaries remain authoritative.
struct DefinitionNames;
impl PackExpander for DefinitionNames {
    fn mismatch(&mut self, _: crate::support::source::SourceOffset) -> Result<(), ParseError> {
        Ok(())
    }

    fn pack_length(&self, _: crate::lex::identifiers::NameId) -> Option<usize> {
        Some(0)
    }
    fn element_name(
        &mut self,
        _: crate::lex::identifiers::NameId,
        _: usize,
    ) -> Result<crate::lex::identifiers::NameId, ParseError> {
        Err(ParseError::Invariant(
            "definition pack collection attempted substitution",
        ))
    }
}

#[derive(Clone, Copy)]
enum DefinitionKind {
    Template,
    Parameter,
}
struct DefinitionPacks<'a> {
    lookup: super::lookup::DeclarationLookup<'a>,
    graph: &'a super::graph::Graph,
    scope: ScopeId,
    kind: DefinitionKind,
    error: std::cell::RefCell<Option<Error>>,
}
impl PackExpander for DefinitionPacks<'_> {
    fn pack_length(&self, name: crate::lex::identifiers::NameId) -> Option<usize> {
        if self.error.borrow().is_some() {
            return None;
        }
        let found = match self.lookup.unqualified_declaration_result(
            self.scope,
            name.into(),
            Filter::Ordinary,
        ) {
            Ok(found) => found,
            Err(error) => {
                *self.error.borrow_mut() = Some(error);
                return None;
            }
        };
        let binding = self
            .graph
            .binding(found.binding.filter(|_| !found.ambiguous)?);
        match self.kind {
            DefinitionKind::Template => self
                .graph
                .entity(binding.entity)
                .context()
                .template_parameter
                .is_some_and(super::templates::ParameterInfo::is_pack)
                .then_some(0),
            DefinitionKind::Parameter => (binding.kind == super::graph::BindingKind::Parameter
                && binding.ty.is_some_and(|ty| {
                    matches!(
                        self.graph.types.get(ty).kind,
                        super::types::TypeKind::Expansion(_)
                    )
                }))
            .then_some(0),
        }
    }
    fn element_name(
        &mut self,
        name: crate::lex::identifiers::NameId,
        _: usize,
    ) -> Result<crate::lex::identifiers::NameId, ParseError> {
        Ok(name)
    }
    fn mismatch(&mut self, _: crate::support::source::SourceOffset) -> Result<(), ParseError> {
        Ok(())
    }
}
