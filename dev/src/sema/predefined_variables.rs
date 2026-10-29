// Translated from analyzer_bodies.cpp::unqualified and entity_records.h. See NOTICE.
//! Function-owned implicit arrays are materialized only by ordinary lookup.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::ValueKey,
    graph::{Binding, BindingKind, EntityId, ScopeId, ScopeKind, VariableProperties},
    lookup::{Filter, LookupResult},
    types::{Cv, Scalar, TypeKind},
};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    types::FundamentalType,
};

#[derive(Default)]
pub(super) struct Variables {
    texts: HashMap<EntityId, NameId>,
    placeholder_results: HashSet<EntityId>,
}
impl Variables {
    pub(super) fn mark_placeholder_result(&mut self, entity: EntityId) {
        self.placeholder_results.insert(entity);
    }
    pub(super) fn placeholder_result(&self, entity: EntityId) -> bool {
        self.placeholder_results.contains(&entity)
    }
    pub(super) fn text(&self, entity: EntityId) -> Option<NameId> {
        self.texts.get(&entity).copied()
    }
}
impl Analyzer<'_> {
    pub(super) fn predefined_lookup(
        &mut self,
        scope: ScopeId,
        key: ValueKey,
        filter: Filter,
        found: LookupResult,
    ) -> Result<LookupResult, Error> {
        let Some(name) = key
            .identifier()
            .filter(|name| self.predefined_names.contains(name))
        else {
            return Ok(found);
        };
        if filter != Filter::Ordinary {
            return Ok(found);
        }
        let mut function = Some(scope);
        while let Some(id) = function {
            let node = self.graph.scope(id);
            if node.kind == ScopeKind::Function && node.owner.is_some() {
                break;
            }
            function = node.parent;
        }
        if let Some(binding) = found.binding {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if self
                .graph
                .predefined_variables
                .text(self.graph.binding(binding).entity)
                .is_none()
                || entity.home == function
            {
                return Ok(found);
            }
        }
        let Some(function) = function else {
            return Ok(found);
        };
        let owner = self
            .graph
            .scope(function)
            .owner
            .ok_or(Error::Invariant("function without owner"))?;
        if self.graph.entity(owner).function.is_none() {
            return Ok(found);
        }
        let text = if name == self.predefined_names[2] {
            super::predefined_names::pretty_function_name(
                &self.graph,
                self.identifiers,
                owner,
                self.stack_budget,
                self.stack_base,
            )?
        } else {
            super::predefined_names::function_name(
                &self.graph,
                self.identifiers,
                super::predefined_names::SourceFunction::Function(owner),
                self.stack_budget,
                self.stack_base,
            )?
        };
        let bound = u64::try_from(text.len())
            .ok()
            .and_then(|length| length.checked_add(1))
            .ok_or(Error::Limit("predefined name length"))?;
        let text = self
            .identifiers
            .intern(&text)
            .map_err(|_| Error::Limit("predefined name text"))?;
        let element = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Char))?;
        let element = self.graph.types.qualify(element, Cv::CONST)?;
        let ty = self.graph.types.intern(
            TypeKind::Array {
                element,
                bound,
                operand: None,
            },
            Cv::default(),
        )?;
        let entity = self.graph.add_entity(Some(ty), Some(name), Keyword::Char)?;
        self.graph.entity_mut(entity).variable = Some(VariableProperties {
            is_extern: false,
            internal_linkage: false,
            language: super::linkage::Language::Cpp,
            automatic: false,
            thread_storage: super::specifiers::ThreadStorage::Ordinary,
            static_specifier: super::specifiers::StaticSpecifier::Absent,
        });
        self.graph.predefined_variables.texts.insert(entity, text);
        self.graph.add_binding(
            function,
            None,
            Binding {
                kind: BindingKind::Variable,
                name: Some(name),
                entity,
                ty: Some(ty),
                key: Keyword::Char,
            },
        )?;
        self.unqualified_result(scope, key, filter)
    }
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
    fn lookup_memoizes_function_arrays_and_nested_bodies_own_their_bindings() {
        let source = SourceBuffer::new(
            b"const char*f(){__func__;__func__;__FUNCTION__;\
              return [](){return __func__;}();}\
              const char*g(){const char*__func__=\"shadow\";return __func__;}\
              template<class T>const char*unused(){return __PRETTY_FUNCTION__;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"predefined-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let variables = &graph.predefined_variables.texts;
        assert_eq!(variables.len(), 4);
        let mut owners = HashSet::default();
        for (&entity, &text) in variables {
            let record = graph.entity(entity);
            assert!(!record.variable.unwrap().automatic);
            let owner = record.home.unwrap();
            assert_eq!(graph.scope(owner).kind, ScopeKind::Function);
            assert_eq!(graph.scope(owner).bindings.len(), 0);
            owners.insert(owner);
            let name = record.name.unwrap();
            let binding = graph.own(owner, name, Filter::Ordinary).unwrap();
            assert_eq!(graph.binding(binding).entity, entity);
            let TypeKind::Array {
                element,
                bound,
                operand,
            } = graph.types.get(record.ty.unwrap()).kind
            else {
                panic!("predefined variable must be an array")
            };
            assert_eq!(graph.types.get(element).cv, Cv::CONST);
            assert_eq!(bound, u64::try_from(cursor.name(text).len()).unwrap() + 1);
            assert_eq!(operand, None);
        }
        assert_eq!(owners.len(), 3);
    }
}
