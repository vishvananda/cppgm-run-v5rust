// Translated from class_templates.cpp::instantiate_default_argument and
// analyzer_calls.cpp::call. See NOTICE.
//! Selected-call default demand, separate from signature, body and emission.
use super::{
    Error,
    analyzer::Analyzer,
    default_arguments::DefaultArgument,
    graph::{BindingId, EntityId, ScopeId},
    templates::HeadId,
};
use crate::syntax::{ast, cloning::Cloner};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) struct Pattern {
    pub head: HeadId,
    pub scope: ScopeId,
    pub limit: usize,
}
#[derive(Clone, Copy)]
enum State {
    InProgress,
    Complete,
}
#[derive(Default)]
pub(super) struct Defaults {
    states: HashMap<(EntityId, usize), State>,
}
impl Analyzer<'_> {
    pub(super) fn instantiate_call_defaults(
        &mut self,
        binding: BindingId,
        supplied: usize,
    ) -> Result<(), Error> {
        let entity = self.graph.binding(binding).entity;
        let Some(primary) = self.graph.function_templates.primary(entity) else {
            return Ok(());
        };
        let primary = self.graph.binding(primary).entity;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("default without function type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        for index in supplied..count {
            let key = (entity, index);
            match self
                .graph
                .function_templates
                .argument_defaults
                .states
                .get(&key)
            {
                Some(State::Complete) => continue,
                Some(State::InProgress) => {
                    let default = self.graph.entity(primary).defaults[index]
                        .ok_or(Error::Invariant("recursive default without pattern"))?;
                    return Err(Error::at(
                        self.tree.expressions.get(default.expression).location,
                        "template instantiation nested too deeply",
                    ));
                }
                None => (),
            }
            let Some(default) = self
                .graph
                .entity(primary)
                .defaults
                .get(index)
                .copied()
                .flatten()
            else {
                continue;
            };
            let pattern = default
                .template
                .ok_or(Error::Invariant("template default without context"))?;
            self.graph
                .function_templates
                .argument_defaults
                .states
                .insert(key, State::InProgress);
            let result = self.instantiate_default(entity, ty, index, default, pattern);
            if let Err(error) = result {
                self.graph
                    .function_templates
                    .argument_defaults
                    .states
                    .remove(&key);
                return Err(error.required_instantiation());
            }
            self.graph
                .function_templates
                .argument_defaults
                .states
                .insert(key, State::Complete);
        }
        Ok(())
    }

    fn instantiate_default(
        &mut self,
        entity: EntityId,
        ty: super::types::TypeId,
        index: usize,
        default: DefaultArgument,
        pattern: Pattern,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let copy = Cloner::new(self.tree, self.stack_budget, self.stack_base)
            .expression(default.expression)
            .map_err(|error| super::template_bodies::copy_error(&error))?;
        let frame = self.template_body_frame(entity, pattern.head, pattern.scope)?;
        let target = self.graph.types.function_parameter(ty, index)?;
        // The default belongs to its declaring scope, with no caller locals or
        // implicit object. Instantiation is evaluated even inside sizeof/noexcept.
        let saved_body = self.body_context.take();
        let saved_limit = self.graph.lookup_limit.replace(pattern.limit);
        let saved_scope = self.default_argument_scope.replace(frame);
        let saved_unevaluated = std::mem::replace(&mut self.unevaluated, 0);
        let saved_depth = std::mem::replace(&mut self.template_depth, 0);
        let saved_class_depth = std::mem::replace(&mut self.class_depth, 0);
        let saved_head = self.template_head.take();
        let saved_initializing = self.initializing.take();
        let saved_base = self.base_object.take();
        let result = (|| {
            if matches!(
                self.tree.expressions.get(copy).kind,
                ast::ExprKind::Braced { .. }
            ) {
                self.initialize_list_argument(copy, target, frame, None)
            } else {
                self.typed_expression(copy, frame)?;
                self.convert_scalar(copy, target, false)?;
                self.record_class_copy(
                    copy,
                    target,
                    frame,
                    super::copy_initialization::CopyUse::Value { in_place: false },
                    self.tree.expressions.get(copy).location,
                )
            }
        })();
        self.body_context = saved_body;
        self.graph.lookup_limit = saved_limit;
        self.default_argument_scope = saved_scope;
        self.unevaluated = saved_unevaluated;
        self.template_depth = saved_depth;
        self.class_depth = saved_class_depth;
        self.template_head = saved_head;
        self.initializing = saved_initializing;
        self.base_object = saved_base;
        result?;
        let count = self.graph.types.function_parameter_count(ty)?;
        self.graph.entity_mut(entity).defaults.resize(count, None);
        self.graph.entity_mut(entity).defaults[index] = Some(DefaultArgument {
            expression: copy,
            template: None,
            ..default
        });
        Ok(())
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
    fn default_copies_belong_to_instances_and_only_demanded_parameters_get_facts() {
        let source = SourceBuffer::new(
            b"template<class T>int f(T,int x=T::missing,int y=sizeof(T)){return x+y;}\
              template<class T>int unused(T,int x=T::missing){return x;}\
              int main(){return f(1,2)+f(2,3)+f('a',2)+f(1,2,4);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let original_expressions = tree.expressions.mark();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let defaults = &graph.function_templates.argument_defaults;
        assert_eq!(defaults.states.len(), 2);
        let mut copies = std::collections::HashSet::new();
        let mut primaries = std::collections::HashSet::new();
        for (&(entity, index), state) in &defaults.states {
            assert_eq!(index, 2);
            assert!(matches!(state, State::Complete));
            let function = graph.entity(entity);
            assert!(function.defaults[0].is_none());
            assert!(function.defaults[1].is_none());
            let copy = function.defaults[2].unwrap().expression;
            assert!(copy.index() >= original_expressions);
            assert!(copies.insert(copy));
            assert!(graph.expression_facts.get(copy).is_ok());
            let primary = graph
                .binding(graph.function_templates.primary(entity).unwrap())
                .entity;
            primaries.insert(primary);
            for pattern in graph.entity(primary).defaults.iter().flatten() {
                assert!(pattern.template.is_some());
                assert!(graph.expression_facts.get(pattern.expression).is_err());
                assert_ne!(pattern.expression, copy);
            }
        }
        assert_eq!(primaries.len(), 1);
        assert!(graph.lookup_limit.is_none());
    }

    #[test]
    fn failed_default_restores_the_caller_context_and_publishes_no_completed_fact() {
        let source = SourceBuffer::new(
            b"template<class T>int f(T,int x=T::missing);int caller(){return f(1);}".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"default-error-context").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        let declaration = analyzer.tree.decl_lists.get(declarations)[0];
        analyzer.declaration(declaration, global).unwrap();
        let head = analyzer.template_head;
        let saved_limit = analyzer.graph.lookup_limit;
        let declaration = analyzer.tree.decl_lists.get(declarations)[1];
        assert!(matches!(
            analyzer.declaration(declaration, global),
            Err(Error::InstantiationDiagnostic { .. })
        ));
        assert_eq!(analyzer.graph.lookup_limit, saved_limit);
        assert_eq!(analyzer.template_head, head);
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.template_depth, 0);
        assert!(analyzer.default_argument_scope.is_none());
        assert!(analyzer.initializing.is_none());
        assert!(analyzer.base_object.is_none());
        assert!(
            analyzer
                .graph
                .function_templates
                .argument_defaults
                .states
                .is_empty()
        );
    }
}
