// Translated from sema/analyzer_construction.cpp::initialize_class/select_constructor
// and analyzer_copy.cpp::constructor_candidates. See NOTICE.
//! Initialization-use facts retain original arguments separately from callable plans.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    expression_facts::{ExprFacts, ValueCategory},
    graph::{BindingId, EntityId, ScopeId},
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{copy_initialization::CopyAction, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn base_initializers_retain_original_operands_and_share_the_selected_copy() {
        let source = SourceBuffer::new(
            br"
            struct A{int n;A(int);A(const A&);};
            struct B:A{B():A{A(7)}{}};
            struct C:A{C(const A&a):A(a){}};
            struct D:A{D(const A&a):A(a){}};
            struct Unused{A a;};
            void f(A&a){B b;C c(a);D d(a);}
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"base-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let declaration_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), declaration_count);
        let mut selected = None;
        for spelling in [b"B", b"C", b"D"] {
            let name = cursor.intern_name(spelling).unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            let binding = graph.entity(class).declared_constructors[0];
            let function = graph.binding(binding).entity;
            let constructor = graph.entity(function).constructor.unwrap();
            let bases = &graph.constructors.get(constructor).bases;
            assert_eq!(bases.len(), 1);
            let written = bases[0].initializer.unwrap();
            if spelling == b"B" {
                let ast::InitializerKind::Braced(list) = written.kind else {
                    panic!("base list");
                };
                let ast::ExprKind::Braced { elements, .. } = tree.expressions.get(list).kind else {
                    panic!("list syntax");
                };
                let operand = tree.expr_lists.get(elements)[0];
                let facts = graph.expression_facts.get(list).unwrap();
                assert!(matches!(facts.class_initialization,
                    Some(super::super::expression_facts::ClassInitialization::FromExpression(id)) if id == operand));
                assert_eq!(
                    graph.expression_facts.get(operand).unwrap().category,
                    ValueCategory::Prvalue
                );
                assert!(graph.lower_class_copy(operand).unwrap().is_none());
            } else {
                let ast::InitializerKind::Parenthesized(arguments) = written.kind else {
                    panic!("base arguments");
                };
                let operand = tree.expr_lists.get(arguments)[0];
                assert_eq!(
                    graph.expression_facts.get(operand).unwrap().category,
                    ValueCategory::Lvalue
                );
                let Some(CopyAction::Constructor { constructor, .. }) =
                    graph.lower_class_copy(operand).unwrap()
                else {
                    panic!("selected base copy");
                };
                assert_eq!(*selected.get_or_insert(constructor), constructor);
                let entity = graph.lower_constructor_entity(constructor).unwrap();
                assert!(graph.entity(entity).function_definition.is_none());
            }
        }
        let name = cursor.intern_name(b"Unused").unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        assert!(graph.entity(class).implicit_copies.is_none());
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ClassContext {
    Object,
    Base,
    AggregateMember,
    Delegation,
}
#[derive(Clone, Copy)]
pub(super) struct ClassInput {
    pub arguments: ConstructorArguments,
    pub list: Option<ast::ExprId>,
    pub converting: bool,
    pub copy_list: bool,
    pub context: ClassContext,
    pub selected: Option<BindingId>,
}
#[derive(Default)]
pub(super) struct InitializerConstructors {
    values: std::collections::HashMap<EntityId, Vec<ConstructorId>>,
    first: Option<ConstructorId>,
}
impl InitializerConstructors {
    pub(super) fn note(&mut self, entity: EntityId, constructor: ConstructorId) {
        if self.first.is_none() {
            self.first = Some(constructor);
        }
        let constructors = self.values.entry(entity).or_default();
        if constructors.last() != Some(&constructor) {
            constructors.push(constructor);
        }
    }
    pub(super) fn get(&self, entity: EntityId) -> &[ConstructorId] {
        self.values.get(&entity).map_or(&[], Vec::as_slice)
    }
    pub(super) fn absent_initializer(&self) -> Option<ConstructorId> {
        // The frozen PointerMap's null-key lookup observes the first chain
        // record once a chain exists. Retain its deterministic demand fact
        // explicitly, without using null keys or rebuilding semantic lookup.
        self.first
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
        self.initialize_class_object_context(
            target,
            initializer,
            scope,
            location,
            ClassContext::Object,
        )
    }
    pub(super) fn initialize_class_object_context(
        &mut self,
        target: TypeId,
        initializer: ast::Initializer,
        scope: ScopeId,
        location: SourceOffset,
        context: ClassContext,
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

        let list = match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id)
                if matches!(
                    self.tree.expressions.get(id).kind,
                    ast::ExprKind::Braced { .. }
                ) =>
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
                let ast::ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind
                else {
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
                context,
                selected: None,
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
            context,
            selected,
        } = input;

        let single;
        let args = match arguments {
            ConstructorArguments::Values(values) => self.tree.expr_lists.get(values),
            ConstructorArguments::Single(id) => {
                single = [id];
                &single
            }
            ConstructorArguments::Value => &[],
        };
        let argument_buffer = super::call_arguments::Arguments::new(args);
        let args = argument_buffer.as_slice();
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
        // Preserve the existing checked copy-elision path for same-class sources.
        if context != ClassContext::Delegation
            && (list.is_none() || context == ClassContext::Base)
            && let [id] = args
            && !matches!(
                self.tree.expressions.get(*id).kind,
                ast::ExprKind::Braced { .. }
            )
        {
            let facts = self.graph.expression_facts.get(*id)?;
            if self
                .graph
                .class_of(facts.ty)
                .is_some_and(|source| source == class || self.graph.is_base_of(class, source))
            {
                self.record_class_copy(
                    *id,
                    target,
                    scope,
                    super::copy_initialization::CopyUse::Object {
                        direct: !converting,
                    },
                    location,
                )?;
                if context == ClassContext::Base {
                    self.check_temporary_destruction(
                        facts,
                        scope,
                        self.tree.expressions.get(*id).location,
                    )?;
                    if let Some(list) = list {
                        self.publish_list(list, target, ValueCategory::Prvalue)?;
                        let mut facts = self.graph.expression_facts.get(list)?;
                        facts.class_initialization =
                            Some(super::expression_facts::ClassInitialization::FromExpression(*id));
                        self.graph.expression_facts.set(list, facts);
                    }
                }
                return Ok(None);
            }
        }
        if !converting
            && selected.is_none()
            && let [argument] = args
            && !matches!(
                self.tree.expressions.get(*argument).kind,
                ast::ExprKind::Braced { .. }
            )
            && self
                .graph
                .expression_facts
                .get(*argument)?
                .functions
                .is_none()
        {
            let constructor =
                self.initialize_single_constructor(class, *argument, input, scope, location)?;
            if let Some(list) = list {
                self.publish_constructor_list(list, target, constructor)?;
            }
            return Ok(Some((constructor, arguments)));
        }
        let conversion = if converting
            && let [id] = args
            && let Some(source) = self
                .graph
                .class_of(self.graph.expression_facts.get(*id)?.ty)
            && !self.visible_conversion_functions(source)?.is_empty()
        {
            let sequence = self.class_initializer_conversion(*id, target, context, location)?;
            match sequence.user_conversion {
                Some(super::procedural_calls::UserConversion::Constructor(_)) => Some(sequence),
                Some(super::procedural_calls::UserConversion::Function(_)) => {
                    self.apply_function_sequence_at(*id, sequence, scope, location)?;
                    return Ok(None);
                }
                _ => return Err(Error::Invariant("class user conversion selection")),
            }
        } else {
            None
        };
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
        let copies = !converting
            && matches!(args, [id]
            if !copy_list || !matches!(self.tree.expressions.get(*id).kind, ast::ExprKind::Braced { .. }));
        if !offered && !copies {
            return Err(Error::at(
                location,
                if context == ClassContext::AggregateMember {
                    "no conversion to the member's class"
                } else {
                    "no constructor for the initialization"
                },
            ));
        }
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let chosen = if let Some(sequence) = conversion {
            Self::prepare_converting_constructor(sequence, &mut scratch)
        } else if let Some(binding) = selected {
            self.prepare_selected_call(binding, args, &mut scratch)
        } else {
            self.resolve_call(
                location,
                if context == ClassContext::AggregateMember {
                    Candidates::ConvertingMember(class)
                } else {
                    Candidates::Constructors {
                        class,
                        converting,
                        copies,
                    }
                },
                args,
                None,
                &mut scratch,
            )
        };
        let chosen = chosen.and_then(|selected| {
            self.check_inherited_constructor_access(selected.binding, location)?;
            let function = self.graph.binding(selected.binding).entity;
            if copy_list && self.graph.entity(function).explicitness == Explicitness::Explicit {
                return Err(Error::at(
                    location,
                    "copy-list-initialization selects an explicit constructor",
                ));
            }
            let constructor = self.selected_constructor_plan(selected.binding, location)?;
            self.validate_constructor_use(constructor, scope, location)?;
            self.apply_call_arguments(selected, args, scope, &scratch)?;
            self.demand_constructor_use(constructor);
            Ok((constructor, function))
        });
        scratch.clear();
        self.call_scratch = scratch;
        let (constructor, function) = chosen?;
        self.check_constructor_parameters(function, args, scope, list.is_some())?;
        if converting {
            self.check_converting_final_copy(class, scope, location)?;
        }
        if let Some(id) = list {
            self.publish_constructor_list(id, target, constructor)?;
        }
        Ok(Some((constructor, arguments)))
    }

    pub(super) fn check_constructor_parameters(
        &mut self,
        function: EntityId,
        args: &[ast::ExprId],
        scope: ScopeId,
        list: bool,
    ) -> Result<(), Error> {
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
                self.tree.expressions.get(argument).location,
            )?;
            if !matches!(
                self.tree.expressions.get(argument).kind,
                ast::ExprKind::Braced { .. }
            ) {
                self.record_class_copy(
                    argument,
                    parameter,
                    scope,
                    super::copy_initialization::CopyUse::Value { in_place: false },
                    self.tree.expressions.get(argument).location,
                )?;
            }
            let mut facts = self.graph.expression_facts.get(argument)?;
            if let Some(conversion) = facts.conversion
                && self
                    .graph
                    .expression_facts
                    .user_conversion(argument)
                    .is_none()
            {
                facts.base_path = self.conversion_base_path(
                    facts,
                    conversion,
                    scope,
                    self.tree.expressions.get(argument).location,
                    true,
                )?;
                self.graph.expression_facts.set(argument, facts);
            }
            if list
                && !matches!(
                    self.graph.types.get(parameter).kind,
                    TypeKind::Reference { .. }
                )
            {
                self.check_narrowing(argument, parameter, scope)?;
            }
        }
        Ok(())
    }

    pub(super) fn class_has_default_constructor(&self, class: EntityId) -> Result<bool, Error> {
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
            self.check_special_use_access(
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
        if self.unevaluated != 0
            || self
                .graph
                .anonymous_member_storage(self.graph.constructors.get(id).class)
                .is_some()
        {
            return;
        }
        if self.body_context.is_none()
            && self.graph.constructors.get(id).nontrivial()
            && let Some(entity) = self.initializing
            && self.graph.entity(entity).data_member != Some(super::graph::DataMemberKind::Instance)
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
