// Translated from sema/analyzer_construction.cpp::initialize_class/select_constructor
// and analyzer_copy.cpp::constructor_candidates. See NOTICE.
//! Initialization-use facts retain original arguments separately from callable plans.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    expression_facts::{ExprFacts, ValueCategory},
    graph::{EntityId, ScopeId},
    procedural_calls::Candidates,
    specifiers::Explicitness,
    types::{TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::{ast, storage::List},
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum ConstructorArguments {
    Values(List<ast::ExprId>),
    Single(ast::ExprId),
    Value,
}
#[derive(Clone, Copy)]
pub(super) struct ClassInput {
    pub arguments: ConstructorArguments,
    pub list: Option<ast::ExprId>,
    pub converting: bool,
    pub copy_list: bool,
}
#[derive(Default)]
pub(super) struct InitializerConstructors {
    values: std::collections::HashMap<EntityId, Vec<ConstructorId>>,
}
impl InitializerConstructors {
    pub(super) fn note(&mut self, entity: EntityId, constructor: ConstructorId) {
        let constructors = self.values.entry(entity).or_default();
        if constructors.last() != Some(&constructor) {
            constructors.push(constructor);
        }
    }
    pub(super) fn get(&self, entity: EntityId) -> &[ConstructorId] {
        self.values.get(&entity).map_or(&[], Vec::as_slice)
    }
}
impl Analyzer<'_> {
    pub(super) fn initialize_class_object(
        &mut self,
        target: TypeId,
        initializer: ast::Initializer,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Option<(ConstructorId, ConstructorArguments)>, Error> {
        let TypeKind::Class(class) = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("class initialization without class"));
        };
        if !self.graph.entity(class).complete || self.graph.entity(class).scope.is_none() {
            return Err(Error::at(
                location,
                "initialization of an incomplete class object",
            ));
        }
        let tree = self.tree;
        let list = match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id)
                if matches!(tree.expressions.get(id).kind, ast::ExprKind::Braced { .. }) =>
            {
                Some(id)
            }
            _ => None,
        };
        if let Some(id) = list
            && self.graph.aggregate_class(class)
        {
            self.class_list_initialize(id, target, scope)?;
            return Ok(None);
        }
        let (arguments, converting, copy_list) = match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id)
                if list.is_some() =>
            {
                let ast::ExprKind::Braced { elements, .. } = tree.expressions.get(id).kind else {
                    return Err(Error::Invariant("constructor list shape"));
                };
                (
                    ConstructorArguments::Values(elements),
                    false,
                    matches!(initializer.kind, ast::InitializerKind::Assign(_)),
                )
            }
            ast::InitializerKind::Parenthesized(values) => {
                (ConstructorArguments::Values(values), false, false)
            }
            ast::InitializerKind::Assign(id) => (ConstructorArguments::Single(id), true, false),
            _ => return Err(Error::Invariant("class initializer shape")),
        };
        self.initialize_class_arguments(
            target,
            ClassInput {
                arguments,
                list,
                converting,
                copy_list,
            },
            scope,
            location,
        )
    }
    pub(super) fn initialize_class_arguments(
        &mut self,
        target: TypeId,
        input: ClassInput,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Option<(ConstructorId, ConstructorArguments)>, Error> {
        let TypeKind::Class(class) = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("class arguments without class"));
        };
        let ClassInput {
            arguments,
            list,
            converting,
            copy_list,
        } = input;
        let tree = self.tree;
        let single;
        let args = match arguments {
            ConstructorArguments::Values(values) => tree.expr_lists.get(values),
            ConstructorArguments::Single(id) => {
                single = [id];
                &single
            }
            ConstructorArguments::Value => &[],
        };
        if args.is_empty() && self.class_has_default_constructor(class)? {
            let constructor = self.default_constructor(class, location)?;
            self.validate_constructor_use(constructor, scope, location)?;
            self.demand_constructor_use(constructor);
            if let Some(id) = list {
                self.publish_constructor_list(id, target, constructor)?;
            }
            return Ok(Some((constructor, ConstructorArguments::Value)));
        }
        for &argument in args {
            if self.graph.expression_facts.get(argument).is_err() {
                self.type_call_argument(argument, scope)?;
            }
        }
        if list.is_none()
            && let [id] = args
            && self.graph.aggregate_class(class)
            && matches!(tree.expressions.get(*id).kind, ast::ExprKind::Braced { .. })
        {
            self.class_list_initialize(*id, target, scope)?;
            self.elide_class_copy(*id, target, scope, !converting, false, location)?;
            return Ok(None);
        }
        // Preserve the existing checked copy-elision path for same-class sources.
        if list.is_none()
            && let [id] = args
            && !matches!(tree.expressions.get(*id).kind, ast::ExprKind::Braced { .. })
        {
            let facts = self.graph.expression_facts.get(*id)?;
            if self.graph.class_of(facts.ty) == Some(class) {
                self.elide_class_copy(*id, target, scope, !converting, false, location)?;
                return Ok(None);
            }
        }
        let offered = self
            .graph
            .entity(class)
            .declared_constructors
            .iter()
            .any(|&binding| {
                !converting
                    || self
                        .graph
                        .entity(self.graph.binding(binding).entity)
                        .explicitness
                        != Explicitness::Explicit
            });
        if !offered {
            if !converting
                && let [argument] = args
                && (!copy_list
                    || !matches!(
                        tree.expressions.get(*argument).kind,
                        ast::ExprKind::Braced { .. }
                    ))
            {
                // The frozen candidate set includes implicit copy/move members
                // for this single-argument form, even when none is viable.
                if matches!(
                    tree.expressions.get(*argument).kind,
                    ast::ExprKind::Braced { .. }
                ) {
                    return Err(Error::Missing(
                        "implicit copy constructor list argument ranking",
                    ));
                }
                let facts = self.graph.expression_facts.get(*argument)?;
                self.select_copy_initialization(class, facts, scope, location)?;
                return Err(Error::Missing("class list copy initialization"));
            }
            return Err(Error::at(location, "no constructor for the initialization"));
        }
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let chosen = self.resolve_call(
            location,
            Candidates::Constructors { class, converting },
            args,
            None,
            &mut scratch,
        );
        let chosen = chosen.and_then(|selected| {
            let function = self.graph.binding(selected.binding).entity;
            if copy_list && self.graph.entity(function).explicitness == Explicitness::Explicit {
                return Err(Error::at(
                    location,
                    "copy-list-initialization selects an explicit constructor",
                ));
            }
            let constructor = self.declared_constructor_plan(selected.binding)?;
            self.validate_constructor_use(constructor, scope, location)?;
            self.apply_call_arguments(selected, args, scope, &scratch)?;
            self.demand_constructor_use(constructor);
            Ok((constructor, function))
        });
        scratch.clear();
        self.call_scratch = scratch;
        let (constructor, function) = chosen?;
        let ty = self
            .graph
            .entity(function)
            .ty
            .ok_or(Error::Invariant("selected constructor without type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        for (index, &argument) in args.iter().enumerate().take(count) {
            let parameter = self.graph.types.function_parameter(ty, index)?;
            self.class_parameter_destruction(
                parameter,
                scope,
                tree.expressions.get(argument).location,
            )?;
            if !matches!(
                tree.expressions.get(argument).kind,
                ast::ExprKind::Braced { .. }
            ) {
                self.elide_class_copy(
                    argument,
                    parameter,
                    scope,
                    false,
                    false,
                    tree.expressions.get(argument).location,
                )?;
            }
            let mut facts = self.graph.expression_facts.get(argument)?;
            if let Some(conversion) = facts.conversion {
                facts.base_path = self.conversion_base_path(
                    facts,
                    conversion,
                    scope,
                    tree.expressions.get(argument).location,
                    true,
                )?;
                self.graph.expression_facts.set(argument, facts);
            }
            if list.is_some()
                && !matches!(
                    self.graph.types.get(parameter).kind,
                    TypeKind::Reference { .. }
                )
            {
                self.check_narrowing(argument, parameter, scope)?;
            }
        }
        if converting {
            self.check_converting_final_copy(class, scope, location)?;
        }
        if let Some(id) = list {
            self.publish_constructor_list(id, target, constructor)?;
        }
        Ok(Some((constructor, arguments)))
    }

    fn class_has_default_constructor(&self, class: EntityId) -> Result<bool, Error> {
        if self.graph.entity(class).user_constructor.is_none() {
            return Ok(true);
        }
        for &binding in &self.graph.entity(class).declared_constructors {
            let function = self.graph.entity(self.graph.binding(binding).entity);
            let ty = function
                .ty
                .ok_or(Error::Invariant("constructor without type"))?;
            let count = self.graph.types.function_parameter_count(ty)?;
            if (0..count)
                .all(|index| self.function_has_default(self.graph.binding(binding).entity, index))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn validate_constructor_use(
        &mut self,
        id: ConstructorId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.constructors.get(id).deleted {
            return Err(Error::at(location, "use of a deleted function"));
        }
        if let Some(binding) = self.graph.constructors.get(id).binding {
            self.check_special_member_access(
                binding,
                self.base_object
                    .unwrap_or(self.graph.constructors.get(id).class),
                scope,
                location,
            )?;
        }
        Ok(())
    }
    pub(super) fn demand_constructor_use(&mut self, id: ConstructorId) {
        if self.unevaluated != 0 {
            return;
        }
        if self.body_context.is_none()
            && self.graph.constructors.get(id).nontrivial()
            && let Some(entity) = self.initializing
        {
            self.graph.initializer_constructors.note(entity, id);
        }
        if let Some(binding) = self.graph.constructors.get(id).binding {
            self.demand_function(binding, true);
        } else {
            self.record_demand(super::function_demands::Demand::Constructor(id));
        }
    }
    fn publish_constructor_list(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        constructor: ConstructorId,
    ) -> Result<(), Error> {
        let target = self.bare_type(target)?;
        self.publish_list(id, target, ValueCategory::Prvalue)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        facts.constructor = Some(constructor);
        let ast::ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("constructor list arguments"));
        };
        facts.class_initialization = Some(
            super::expression_facts::ClassInitialization::Constructed(if elements.is_empty() {
                ConstructorArguments::Value
            } else {
                ConstructorArguments::Values(elements)
            }),
        );
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }
    pub(super) fn check_converting_final_copy(
        &mut self,
        class: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let target = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("copy class without type"))?;
        let facts = ExprFacts::new(target, ValueCategory::Xvalue);
        self.check_final_copy_selection(class, facts, scope, location)
    }
}
