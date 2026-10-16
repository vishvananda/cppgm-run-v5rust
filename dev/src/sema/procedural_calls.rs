// Translated from sema/analyzer_calls.cpp. See NOTICE.
//! Ordinary call resolution; candidate failures stay compact until the final diagnostic.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank as R, ExprFacts, LvalueTransform as L, ReferenceSource,
        ValueCategory as V, ValueConversion as C,
    },
    function_resolution::CallTarget,
    graph::{BindingId, ScopeId},
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    syntax::{ast, storage::List},
    types::FundamentalType as F,
};
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub(super) enum Candidates {
    Functions(super::function_resolution::OverloadId),
    ConvertingMember(super::graph::EntityId),
    Constructors {
        class: super::graph::EntityId,
        converting: bool,
        copies: bool,
    },
}

impl Candidates {
    fn bindings(self, graph: &super::graph::Graph) -> &[BindingId] {
        match self {
            Self::Functions(set) => graph.overloads.get(set),
            Self::Constructors { class, .. } | Self::ConvertingMember(class) => {
                &graph.entity(class).declared_constructors
            }
        }
    }
    fn includes(self, graph: &super::graph::Graph, binding: BindingId) -> bool {
        !matches!(
            self,
            Self::Constructors {
                converting: true,
                ..
            } | Self::ConvertingMember(_)
        ) || graph.entity(graph.binding(binding).entity).explicitness
            != super::specifiers::Explicitness::Explicit
    }
}

#[derive(Clone, Copy)]
pub(super) struct CallSelection {
    pub binding: BindingId,
    first: usize,
    object_count: usize,
}
#[derive(Default)]
pub(super) struct CallScratch {
    candidates: Vec<(BindingId, usize)>,
    sequences: Vec<Option<ArgumentSequence>>,
    pub(super) surrogates: super::callable_objects::SurrogateScratch,
}
impl CallScratch {
    pub(super) fn has_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }
    pub(super) fn clear(&mut self) {
        self.candidates.clear();
        self.sequences.clear();
        self.surrogates.clear();
    }
}
#[derive(Clone, Copy)]
enum ArgumentConversions {
    Standard,
    UserDefined,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ArgumentSequence {
    pub(super) conversion: Conversion,
    // An overload designation ranks as an exact target match, even when its
    // actual unary shape cannot bind that target. Diagnose only after selection.
    // User conversions keep the constructor's first or conversion function's
    // second standard sequence here, with its original rank.
    pub(super) application: Option<Conversion>,
    pub(super) function: Option<BindingId>,
    pub(super) user_conversion: Option<UserConversion>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UserConversion {
    Constructor(BindingId),
    ImplicitConstructor(super::graph::EntityId),
    Function(BindingId),
}
impl Analyzer<'_> {
    pub(super) fn procedural_call(
        &mut self,
        id: ast::ExprId,
        callee: ast::ExprId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let mut named = callee;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(named).kind {
            named = inner;
        }
        let facts = if let ast::ExprKind::Id(name) = self.tree.expressions.get(named).kind {
            let callee_uses = self.dependent_uses.get();
            let mut found = self.lookup_call_result(name, scope)?;
            if let Some(facts) = self.builtin_call(id, name, arguments, scope, &mut found)? {
                return Ok(facts);
            }
            let syntax_item = *self.tree.names.get(name);
            let syntax_name = &syntax_item;
            let qualified =
                syntax_name.global || self.tree.name_lists.get(syntax_name.components).len() > 1;
            if found.binding.is_none() && (qualified || callee != named) {
                return Err(Error::at(
                    self.tree.expressions.get(named).location,
                    self.missing_callee_message(name),
                ));
            }
            if let Some(binding) = found.binding {
                let binding = self.graph.binding(binding);
                if matches!(
                    binding.kind,
                    super::graph::BindingKind::Type | super::graph::BindingKind::Alias
                ) {
                    let target = binding
                        .ty
                        .ok_or(Error::Invariant("cast name without type"))?;
                    let mut facts =
                        self.scalar_functional_cast(id, target, arguments, None, scope)?;
                    facts.functional_call = true;
                    return Ok(facts);
                }
            }
            let named_value = if found.binding.is_some_and(|binding| {
                self.graph.binding(binding).kind != super::graph::BindingKind::Function
            }) {
                Some(self.named_expression(named, &found, scope)?)
            } else {
                None
            };
            let component = *self
                .tree
                .name_lists
                .get(syntax_name.components)
                .last()
                .ok_or(Error::Invariant("empty call name"))?;
            let explicit = component
                .arguments
                .map(|syntax| self.explicit_function_arguments(syntax, scope, component.location))
                .transpose()?;
            let uses = if component.arguments.is_some() {
                callee_uses
            } else {
                self.dependent_uses.get()
            };
            for syntax_index in 0..self.tree.expr_lists.get(arguments).len() {
                let argument = self.tree.expr_lists.get(arguments)[syntax_index];
                self.type_call_argument(argument, scope)?;
            }
            if callee == named
                && !qualified
                && !found.binding.is_some_and(|binding| {
                    self.graph
                        .entity(self.graph.binding(binding).entity)
                        .home
                        .is_some_and(|home| {
                            self.graph.scope(home).kind == super::graph::ScopeKind::Class
                        })
                })
            {
                let key = Self::expression_name(component.kind)?;
                let context = super::call_lookup::AssociationContext::from_dependence(
                    self.dependent_uses.get() != uses,
                );
                self.argument_dependent_lookup(key, arguments, &mut found, context)?;
            }
            if found.binding.is_none() {
                return Err(Error::at(
                    self.tree.expressions.get(callee).location,
                    if self.tree.names.get(name).global
                        || self
                            .tree
                            .name_lists
                            .get(self.tree.names.get(name).components)
                            .len()
                            > 1
                    {
                        "undeclared name"
                    } else {
                        "undeclared function name"
                    },
                ));
            }
            if let Some(given) = explicit {
                self.specialize_explicit_functions(component, &mut found, &given, true)?;
            }
            let facts = if let Some(facts) = named_value {
                facts
            } else {
                self.named_expression(named, &found, scope)?
            };
            self.publish_named_callee(callee, named, facts, scope)?;
            facts
        } else {
            let facts = self.typed_expression(callee, scope)?;
            for syntax_index in 0..self.tree.expr_lists.get(arguments).len() {
                let argument = self.tree.expr_lists.get(arguments)[syntax_index];
                self.type_call_argument(argument, scope)?;
            }
            facts
        };
        if let Some(target) = facts.destructor {
            let mut result = self.destructor_call(id, arguments, target)?;
            result.dispatch = facts.dispatch;
            return Ok(result);
        }
        if self.graph.class_of(facts.ty).is_some() {
            return self.callable_object_call(id, facts, scope);
        }
        let designation = if let Some(set) = facts.functions {
            Some(set)
        } else if let Some(binding) = facts.function {
            Some(self.graph.overloads.add(&[binding])?)
        } else {
            None
        };
        if let Some(set) = designation {
            let mut designator = callee;
            let mut levels = 0;
            loop {
                designator = self.graph.expression_facts.root(designator);
                let ast::ExprKind::Unary { operand, .. } =
                    self.tree.expressions.get(designator).kind
                else {
                    break;
                };
                levels += 1;
                designator = operand;
            }
            if levels > 1 {
                self.check_resolved_function(callee)?;
            }
            // Nested argument calls finish before this query borrows the reusable scratch.
            let mut scratch = std::mem::take(&mut self.call_scratch);
            let argument_buffer =
                super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
            let chosen = self.resolve_call(
                self.tree.expressions.get(id).location,
                Candidates::Functions(set),
                argument_buffer.as_slice(),
                facts.object,
                &mut scratch,
            );
            let result = chosen.and_then(|selected| {
                self.finish_selected_call(id, facts, scope, selected, &scratch)
            });
            scratch.clear();
            self.call_scratch = scratch;
            return result;
        }
        self.indirect_call(id, callee, arguments, facts.ty, scope)
    }
    pub(super) fn finish_selected_call(
        &mut self,
        id: ast::ExprId,
        facts: ExprFacts,
        scope: ScopeId,
        selected: CallSelection,
        scratch: &CallScratch,
    ) -> Result<ExprFacts, Error> {
        let ast::ExprKind::Call { callee, arguments } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("selected call without call syntax"));
        };
        let binding = selected.binding;
        let location = self.tree.expressions.get(id).location;
        if self
            .graph
            .entity(self.graph.binding(binding).entity)
            .availability
            == super::graph::FunctionAvailability::Deleted
        {
            return Err(Error::at(location, "use of a deleted function"));
        }
        self.record_call_receiver(callee, binding, facts, scope, location)?;
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        self.apply_call_arguments(selected, argument_buffer.as_slice(), scope, scratch)?;
        self.record_argument_base_paths(argument_buffer.as_slice(), scope)?;
        self.check_class_arguments(binding, arguments, scope)?;
        self.select_function_designation(callee, binding)?;
        self.demand_function(binding, true);
        let ty = self.graph.types.canonical_function(
            self.graph
                .binding(binding)
                .ty
                .ok_or(Error::Invariant("function without type"))?,
        )?;
        let mut facts = self.call_result(ty)?;
        facts.call = Some(CallTarget::Direct(
            self.graph
                .function_origin(self.graph.binding(binding).entity)?,
        ));
        if self
            .graph
            .virtuals
            .function(self.graph.binding(binding).entity)
            .is_virtual
            && !self.qualified_call_designator(callee)
        {
            facts.dispatch = super::expression_facts::Dispatch::Virtual;
        }
        self.graph.record_call_binding(id, binding);
        Ok(facts)
    }
    fn qualified_call_designator(&self, mut expression: ast::ExprId) -> bool {
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(expression).kind {
            expression = inner;
        }
        let (ast::ExprKind::Id(name) | ast::ExprKind::Member { member: name, .. }) =
            self.tree.expressions.get(expression).kind
        else {
            return false;
        };
        let node = self.tree.names.get(name);
        node.global || self.tree.name_lists.get(node.components).len() > 1
    }
    // Member access is checked through its naming declaration; the implicit
    // object conversion retains the physical route without rechecking that access.
    fn record_call_receiver(
        &mut self,
        callee: ast::ExprId,
        binding: BindingId,
        facts: ExprFacts,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        if let Some(object) = facts.object {
            let class = self
                .graph
                .class_of(object.ty)
                .ok_or(Error::Invariant("call object without class"))?;
            let naming = facts.member_naming.unwrap_or(class);
            self.check_named_member_access(binding, naming, class, scope, location)?;
            if let Some(home_type) = self.member_function_owner(binding) {
                let home = self
                    .graph
                    .class_of(home_type)
                    .ok_or(Error::Invariant("member function owner without class"))?;
                let access = self
                    .tree
                    .expressions
                    .get(self.graph.expression_facts.root(callee));
                let access_location = if matches!(access.kind, ast::ExprKind::Member { .. }) {
                    access.location
                } else {
                    location
                };
                if self.graph.base_count(class, naming) > 1
                    || self.graph.base_count(naming, home) > 1
                {
                    return Err(Error::at(
                        access_location,
                        "a non-static member of an ambiguous base class subobject",
                    ));
                }
                if class != naming {
                    self.checked_base_path(class, naming, scope, access_location, true, false)?;
                }
                let conversion = self
                    .object_sequence(object, binding, false)?
                    .ok_or(Error::Invariant("chosen object conversion missing"))?;
                let object_facts = ExprFacts::new(object.ty, object.category);
                let path =
                    self.conversion_base_path(object_facts, conversion, scope, location, false)?;
                let mut selected = self.graph.expression_facts.get(callee)?;
                selected.conversion = Some(conversion);
                selected.base_path = path;
                if naming != class && naming != home {
                    selected.member_naming_path = Some(self.graph.retain_base_path(class, naming)?);
                    selected.member_path = Some(self.graph.retain_base_path(naming, home)?);
                }
                self.graph.expression_facts.set(callee, selected);
            }
        } else {
            self.check_declaration_direct_access(binding, scope, location)?;
            if self.member_function_owner(binding).is_some() && self.unevaluated == 0 {
                return Err(Error::at(
                    location,
                    "non-static member function called without an object",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn object_sequence(
        &mut self,
        object: super::expression_facts::ObjectArgument,
        binding: BindingId,
        ranking: bool,
    ) -> Result<Option<Conversion>, Error> {
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("object candidate without type"))?;
        let TypeKind::Function { cv, reference, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("object candidate without function type"));
        };
        let owner = if ranking {
            self.bare_type(object.ty)?
        } else {
            self.member_function_owner(binding)
                .ok_or(Error::Invariant("object sequence for static function"))?
        };
        let owner = self.graph.types.qualify(owner, cv)?;
        let target = self
            .graph
            .types
            .reference(owner, reference.unwrap_or(RefKind::Lvalue))?;
        let facts = ExprFacts::new(
            object.ty,
            if reference.is_none() {
                V::Lvalue
            } else {
                object.category
            },
        );
        let mut conversion = self.scalar_sequence(facts, target, false)?;
        if reference.is_none()
            && let Some(conversion) = &mut conversion
        {
            conversion.reference_source = ReferenceSource::UnqualifiedObject;
        }
        Ok(conversion)
    }
    pub(super) fn indirect_call(
        &mut self,
        id: ast::ExprId,
        callee: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        source: TypeId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let target = self.decayed_type(source)?;
        let function = match self.graph.types.get(target).kind {
            TypeKind::Function { .. } => target,
            TypeKind::Pointer(element)
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) =>
            {
                self.convert_operand_scalar(callee, target, false)?;
                element
            }
            _ => {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "called object is not a function or pointer to function",
                ));
            }
        };
        let TypeKind::Function {
            parameters,
            variadic,
            ..
        } = &self.graph.types.get(function).kind
        else {
            return Err(Error::Invariant("indirect function type"));
        };
        let count = parameters.len();
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let args = argument_buffer.as_slice();
        if args.len() < count || args.len() > count && !variadic {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "wrong number of arguments in an indirect call",
            ));
        }
        for (index, &argument) in args.iter().enumerate() {
            if index < count {
                let target = self.graph.types.function_parameter(function, index)?;
                if matches!(
                    self.tree.expressions.get(argument).kind,
                    ast::ExprKind::Braced { .. }
                ) {
                    self.initialize_list_argument(argument, target, scope, None)?;
                    continue;
                }
                self.class_parameter_destruction(
                    target,
                    scope,
                    self.tree.expressions.get(argument).location,
                )?;
                self.convert_scalar(argument, target, false)?;
                self.record_class_copy(
                    argument,
                    target,
                    scope,
                    super::copy_initialization::CopyUse::Value { in_place: false },
                    self.tree.expressions.get(argument).location,
                )?;
            } else {
                self.ellipsis_argument(argument)?;
            }
        }
        if self.lowering() && self.unevaluated == 0 {
            self.graph.storage_layouts.push((
                self.body_context.as_ref().map(|body| body.entity),
                super::layout::StorageDemand::CallBoundary(function),
                self.tree.expressions.get(id).location,
            ));
        }
        let mut facts = self.call_result(function)?;
        facts.call = Some(CallTarget::Indirect(
            function,
            self.callee_target_entity(callee)?,
        ));
        Ok(facts)
    }
    fn publish_named_callee(
        &mut self,
        callee: ast::ExprId,
        named: ast::ExprId,
        facts: ExprFacts,
        scope: ScopeId,
    ) -> Result<(), Error> {
        super::record(&mut self.graph.expression_scopes, named.index(), scope);
        self.graph.expression_facts.set(named, facts);
        super::record(&mut self.graph.expression_types, named.index(), facts.ty);
        let mut outer = callee;
        while outer != named {
            super::record(&mut self.graph.expression_scopes, outer.index(), scope);
            self.graph.expression_facts.share(outer, named);
            let ast::ExprKind::Paren(inner) = self.tree.expressions.get(outer).kind else {
                return Err(Error::Invariant("named callee without parentheses"));
            };
            outer = inner;
        }
        Ok(())
    }
    pub(super) fn argument_sequence(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
    ) -> Result<Option<ArgumentSequence>, Error> {
        self.argument_sequence_with_conversions(id, target, ArgumentConversions::UserDefined)
    }
    fn argument_sequence_with_conversions(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        conversions: ArgumentConversions,
    ) -> Result<Option<ArgumentSequence>, Error> {
        if matches!(
            self.tree.expressions.get(id).kind,
            ast::ExprKind::Braced { .. }
        ) {
            return self.list_sequence(id, target);
        }
        let mut facts = self.graph.expression_facts.get(id)?;
        if facts.object.is_some()
            && (facts.functions.is_some() || facts.destructor.is_some())
            && self.ordinary_function_operand(facts).is_none()
        {
            // The frozen implicit_sequence accepts only a lone static
            // designation here; a bound overload set is not a value argument.
            return Ok(None);
        }
        let mut function = None;
        let overloaded =
            facts.functions.is_some() && self.ordinary_function_operand(facts).is_none();
        if facts.functions.is_some() {
            let binding = if let Some(binding) = self.ordinary_function_operand(facts) {
                Some(binding)
            } else {
                self.matching_function(facts, target)?
            };
            let Some(binding) = binding else {
                return Ok(None);
            };
            let ty = self.graph.types.canonical_function(
                self.graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("function argument without type"))?,
            )?;
            facts.ty = if facts.member_address_scope.is_some() {
                self.function_address_type(binding, ty)?
            } else if matches!(self.graph.types.get(facts.ty).kind, TypeKind::Pointer(_))
                || overloaded
                    && matches!(
                        self.tree
                            .expressions
                            .get(self.graph.expression_facts.root(id))
                            .kind,
                        ast::ExprKind::Unary { .. }
                    )
            {
                self.graph.types.pointer(ty)?
            } else {
                ty
            };
            facts.functions = None;
            function = Some(binding);
        }
        let application = self.scalar_sequence(facts, target, false)?;
        if overloaded {
            return Ok(Some(ArgumentSequence {
                conversion: Conversion {
                    target,
                    result: target,
                    first: if matches!(
                        self.graph.types.get(target).kind,
                        TypeKind::Reference { .. }
                    ) {
                        L::None
                    } else {
                        L::FunctionToPointer
                    },
                    second: C::None,
                    rank: R::Exact,
                    qualification: false,
                    initialization: true,
                    temporary: false,
                    reference_source: if matches!(
                        self.graph.types.get(target).kind,
                        TypeKind::Reference { .. }
                    ) {
                        ReferenceSource::FunctionLvalue
                    } else {
                        ReferenceSource::Other
                    },
                },
                application,
                function,
                user_conversion: None,
            }));
        }
        if application.is_none()
            && !overloaded
            && matches!(conversions, ArgumentConversions::UserDefined)
        {
            let mut sequence = self.user_conversion_sequence(facts, target, false)?;
            if let Some(sequence) = &mut sequence {
                sequence.function = function;
            }
            return Ok(sequence);
        }
        Ok(application.map(|conversion| ArgumentSequence {
            conversion,
            application: Some(conversion),
            function,
            user_conversion: None,
        }))
    }
    pub(super) fn record_argument_base_paths(
        &mut self,
        arguments: &[ast::ExprId],
        scope: ScopeId,
    ) -> Result<(), Error> {
        for &argument in arguments {
            let mut facts = self.graph.expression_facts.get(argument)?;
            if let Some(conversion) = facts.conversion
                && self
                    .graph
                    .expression_facts
                    .function_conversion(argument)
                    .is_none()
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
        }
        Ok(())
    }
    fn check_class_arguments(
        &mut self,
        binding: BindingId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        self.check_class_argument_slice(binding, argument_buffer.as_slice(), 0, scope)
    }
    pub(super) fn check_class_argument_slice(
        &mut self,
        binding: BindingId,
        arguments: &[ast::ExprId],
        offset: usize,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ty = self
            .graph
            .entity(self.graph.binding(binding).entity)
            .ty
            .ok_or(Error::Invariant("selected function without type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        for (index, &id) in arguments
            .iter()
            .enumerate()
            .take(count.saturating_sub(offset))
        {
            let target = self.graph.types.function_parameter(ty, index + offset)?;
            self.class_parameter_destruction(
                target,
                scope,
                self.tree.expressions.get(id).location,
            )?;
            self.record_class_copy(
                id,
                target,
                scope,
                super::copy_initialization::CopyUse::Value { in_place: false },
                self.tree.expressions.get(id).location,
            )?;
        }
        Ok(())
    }
    pub(super) fn resolve_call(
        &mut self,
        location: crate::support::source::SourceOffset,
        set: Candidates,
        args: &[ast::ExprId],
        object: Option<super::expression_facts::ObjectArgument>,
        scratch: &mut CallScratch,
    ) -> Result<CallSelection, Error> {
        self.rank_call(set, args, object, scratch)?;
        if matches!(set, Candidates::ConvertingMember(_)) {
            return self
                .unique_call(args.len(), usize::from(object.is_some()), scratch)?
                .ok_or_else(|| Error::at(location, "no conversion to the member's class"));
        }
        self.select_call(location, args.len(), usize::from(object.is_some()), scratch)
    }
    pub(super) fn rank_call(
        &mut self,
        set: Candidates,
        args: &[ast::ExprId],
        object: Option<super::expression_facts::ObjectArgument>,
        scratch: &mut CallScratch,
    ) -> Result<(), Error> {
        self.rank_prefixed_call(set, args, object, None, scratch)
    }
    pub(super) fn rank_prefixed_call(
        &mut self,
        set: Candidates,
        args: &[ast::ExprId],
        object: Option<super::expression_facts::ObjectArgument>,
        prefix: Option<ExprFacts>,
        scratch: &mut CallScratch,
    ) -> Result<(), Error> {
        scratch.candidates.clear();
        scratch.sequences.clear();
        let offset = usize::from(prefix.is_some());
        let written = args.len() + offset;
        let copies = if let Candidates::Constructors {
            class,
            copies: true,
            ..
        } = set
        {
            self.implicit_copy_bindings(class)?
        } else {
            [None; 2]
        };
        let declared = set.bindings(&self.graph).len();
        for index in 0..declared + copies.len() {
            let primary = if index < declared {
                set.bindings(&self.graph)[index]
            } else if let Some(binding) = copies[index - declared] {
                binding
            } else {
                continue;
            };
            if !set.includes(&self.graph, primary) {
                continue;
            }
            if prefix.is_some()
                && self
                    .graph
                    .entity(self.graph.binding(primary).entity)
                    .template_head
                    .is_some()
            {
                return Err(Error::Missing("allocation function template deduction"));
            }
            let Some(binding) = self.deduce_function_call(primary, args, false)? else {
                continue;
            };
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            let ty = entity.ty.ok_or(Error::Invariant("function without type"))?;
            let ty = self.graph.types.canonical_function(ty)?;
            let TypeKind::Function {
                parameters,
                variadic,
                ..
            } = &self.graph.types.get(ty).kind
            else {
                return Err(Error::Invariant("candidate without function type"));
            };
            let count = parameters.len();
            if written > count && !variadic
                || (written..count).any(|index| {
                    !self.function_has_default(self.graph.binding(binding).entity, index)
                })
            {
                continue;
            }
            let first = scratch.sequences.len();
            if let Some(object) = object {
                let sequence = if self.member_function_owner(binding).is_some() {
                    let Some(conversion) = self.object_sequence(object, binding, true)? else {
                        continue;
                    };
                    Some(ArgumentSequence {
                        conversion,
                        application: Some(conversion),
                        function: None,
                        user_conversion: None,
                    })
                } else {
                    None
                };
                scratch.sequences.push(sequence);
            }
            if let Some(facts) = prefix {
                if count == 0 {
                    continue;
                }
                let target = self.graph.types.function_parameter(ty, 0)?;
                let Some(conversion) = self.scalar_sequence(facts, target, false)? else {
                    continue;
                };
                scratch.sequences.push(Some(ArgumentSequence {
                    conversion,
                    application: Some(conversion),
                    function: None,
                    user_conversion: None,
                }));
            }
            let mut viable = true;
            for (index, &arg) in args.iter().enumerate() {
                let index = index + offset;
                let sequence = if index < count {
                    let target = self.graph.types.function_parameter(ty, index)?;
                    let conversions = if matches!(
                        set,
                        Candidates::Constructors {
                            converting: true,
                            ..
                        } | Candidates::ConvertingMember(_)
                    ) {
                        ArgumentConversions::Standard
                    } else {
                        ArgumentConversions::UserDefined
                    };
                    let Some(conversion) =
                        self.argument_sequence_with_conversions(arg, target, conversions)?
                    else {
                        viable = false;
                        break;
                    };
                    Some(conversion)
                } else {
                    if matches!(
                        self.tree.expressions.get(arg).kind,
                        ast::ExprKind::Braced { .. }
                    ) {
                        viable = false;
                        break;
                    }
                    let facts = self.graph.expression_facts.get(arg)?;
                    if facts.functions.is_some() && self.ordinary_function_operand(facts).is_none()
                        || matches!(
                            self.graph.types.get(facts.ty).kind,
                            TypeKind::Scalar(Scalar::Void)
                        )
                    {
                        viable = false;
                        break;
                    }
                    None
                };
                scratch.sequences.push(sequence);
            }
            if viable {
                scratch.candidates.push((binding, first));
            } else {
                scratch.sequences.truncate(first);
            }
        }
        Ok(())
    }
    pub(super) fn select_call(
        &mut self,
        location: crate::support::source::SourceOffset,
        count: usize,
        object_count: usize,
        scratch: &CallScratch,
    ) -> Result<CallSelection, Error> {
        if scratch.candidates.is_empty() {
            return Err(Error::at(location, "no viable function for the call"));
        }
        self.unique_call(count, object_count, scratch)?
            .ok_or_else(|| Error::at(location, "ambiguous call to overloaded function"))
    }
    pub(super) fn prepare_selected_call(
        &mut self,
        binding: BindingId,
        args: &[ast::ExprId],
        scratch: &mut CallScratch,
    ) -> Result<CallSelection, Error> {
        scratch.clear();
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("selected callable without type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        for (index, &arg) in args.iter().enumerate() {
            scratch.sequences.push(if index < count {
                let target = self.graph.types.function_parameter(ty, index)?;
                Some(
                    self.argument_sequence(arg, target)?
                        .ok_or(Error::Invariant("selected callable lost argument sequence"))?,
                )
            } else {
                None
            });
        }
        Ok(CallSelection {
            binding,
            first: 0,
            object_count: 0,
        })
    }
    pub(super) fn prepare_converting_constructor(
        sequence: ArgumentSequence,
        scratch: &mut CallScratch,
    ) -> Result<CallSelection, Error> {
        let Some(UserConversion::Constructor(binding)) = sequence.user_conversion else {
            return Err(Error::Invariant(
                "constructor selection without constructor",
            ));
        };
        let first = sequence
            .application
            .ok_or(Error::Invariant("constructor first sequence"))?;
        scratch.clear();
        scratch.sequences.push(Some(ArgumentSequence {
            conversion: first,
            application: Some(first),
            function: None,
            user_conversion: None,
        }));
        Ok(CallSelection {
            binding,
            first: 0,
            object_count: 0,
        })
    }
    pub(super) fn unique_call(
        &mut self,
        count: usize,
        object_count: usize,
        scratch: &CallScratch,
    ) -> Result<Option<CallSelection>, Error> {
        if scratch.candidates.is_empty() {
            return Ok(None);
        }
        let mut best = 0;
        for index in 1..scratch.candidates.len() {
            if self.compare_candidates(scratch, index, best, count, object_count)?
                == Ordering::Greater
            {
                best = index;
            }
        }
        for index in 0..scratch.candidates.len() {
            if index != best
                && self.compare_candidates(scratch, best, index, count, object_count)?
                    != Ordering::Greater
            {
                return Ok(None);
            }
        }
        let (binding, first) = scratch.candidates[best];
        Ok(Some(CallSelection {
            binding,
            first,
            object_count,
        }))
    }
    // Literal operands are already typed values, not synthetic syntax. They
    // use the same standard sequences and candidate ordering as ordinary calls.
    pub(super) fn resolve_value_call(
        &mut self,
        location: crate::support::source::SourceOffset,
        bindings: &[BindingId],
        args: &[ExprFacts],
    ) -> Result<BindingId, Error> {
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let result = self.resolve_value_candidates(location, bindings, args, &mut scratch);
        scratch.clear();
        self.call_scratch = scratch;
        result
    }
    fn resolve_value_candidates(
        &mut self,
        location: crate::support::source::SourceOffset,
        bindings: &[BindingId],
        args: &[ExprFacts],
        scratch: &mut CallScratch,
    ) -> Result<BindingId, Error> {
        for &binding in bindings {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("literal operator without type"))?;
            let ty = self.graph.types.canonical_function(ty)?;
            let TypeKind::Function {
                parameters,
                variadic,
                ..
            } = &self.graph.types.get(ty).kind
            else {
                return Err(Error::Invariant("literal operator without function type"));
            };
            let count = parameters.len();
            if args.len() > count && !variadic
                || (args.len()..count).any(|index| {
                    !self.function_has_default(self.graph.binding(binding).entity, index)
                })
            {
                continue;
            }
            let first = scratch.sequences.len();
            let mut viable = true;
            for (index, &facts) in args.iter().enumerate() {
                let sequence = if index < count {
                    let target = self.graph.types.function_parameter(ty, index)?;
                    let Some(conversion) = self.scalar_sequence(facts, target, false)? else {
                        viable = false;
                        break;
                    };
                    Some(ArgumentSequence {
                        conversion,
                        application: Some(conversion),
                        function: None,
                        user_conversion: None,
                    })
                } else {
                    None
                };
                scratch.sequences.push(sequence);
            }
            if viable {
                scratch.candidates.push((binding, first));
            } else {
                scratch.sequences.truncate(first);
            }
        }
        self.select_call(location, args.len(), 0, scratch)
            .map(|chosen| chosen.binding)
    }
    pub(super) fn apply_call_arguments(
        &mut self,
        selected: CallSelection,
        args: &[ast::ExprId],
        scope: ScopeId,
        scratch: &CallScratch,
    ) -> Result<(), Error> {
        let CallSelection {
            binding,
            first,
            object_count,
        } = selected;
        for (index, &arg) in args.iter().enumerate() {
            if let Some(sequence) = scratch.sequences[first + object_count + index] {
                if matches!(
                    self.tree.expressions.get(arg).kind,
                    ast::ExprKind::Braced { .. }
                ) {
                    self.class_parameter_destruction(
                        sequence.conversion.target,
                        scope,
                        self.tree.expressions.get(arg).location,
                    )?;
                    self.initialize_ranked_list_argument(arg, sequence, scope)?;
                    continue;
                }
                self.apply_argument_sequence(arg, sequence, scope)?;
            } else {
                self.ellipsis_argument(arg)?;
            }
        }
        self.instantiate_call_defaults(binding, args.len())
    }
    pub(super) fn apply_argument_sequence(
        &mut self,
        arg: ast::ExprId,
        sequence: ArgumentSequence,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if self.apply_user_sequence(
            arg,
            sequence,
            scope,
            super::user_conversions::ConversionSite::Call,
        )? {
            return Ok(());
        }
        let mut facts = self.graph.expression_facts.get(arg)?;
        if let Some(binding) = sequence.function {
            self.select_function_designation(arg, binding)?;
            facts = self.graph.expression_facts.get(arg)?;
        }
        facts.conversion = Some(sequence.application.ok_or(Error::at(
            self.tree.expressions.get(arg).location,
            "invalid implicit conversion",
        ))?);
        self.check_binding_temporary(
            facts,
            facts
                .conversion
                .ok_or(Error::Invariant("call argument without conversion"))?,
            scope,
            self.tree.expressions.get(arg).location,
        )?;
        self.odr_use_constants(arg, facts.conversion)?;
        self.graph.expression_facts.set(arg, facts);
        Ok(())
    }
    pub(super) fn apply_prefixed_arguments(
        &mut self,
        mut selected: CallSelection,
        args: &[ast::ExprId],
        scope: ScopeId,
        scratch: &CallScratch,
    ) -> Result<(), Error> {
        selected.object_count += 1;
        self.apply_call_arguments(selected, args, scope, scratch)
    }
    fn compare_candidates(
        &mut self,
        scratch: &CallScratch,
        a: usize,
        b: usize,
        count: usize,
        object_count: usize,
    ) -> Result<Ordering, Error> {
        let (_, af) = scratch.candidates[a];
        let (_, bf) = scratch.candidates[b];
        let mut better = false;
        let mut worse = false;
        for index in 0..count + object_count {
            if index == 0
                && object_count != 0
                && (scratch.sequences[af].is_none() || scratch.sequences[bf].is_none())
            {
                continue;
            }
            let order = match (scratch.sequences[af + index], scratch.sequences[bf + index]) {
                (Some(a), Some(b)) => self.compare_argument_sequences(a, b)?,
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            };
            better |= order == Ordering::Greater;
            worse |= order == Ordering::Less;
        }
        let order = match (better, worse) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => Ordering::Equal,
        };
        if !better && !worse {
            return self.compare_function_templates(
                scratch.candidates[a].0,
                scratch.candidates[b].0,
                Some(count),
            );
        }
        Ok(order)
    }
    pub(super) fn ellipsis_argument(&mut self, id: ast::ExprId) -> Result<(), Error> {
        if matches!(
            self.tree.expressions.get(id).kind,
            ast::ExprKind::Braced { .. }
        ) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "a braced-init-list argument matched by the ellipsis",
            ));
        }
        self.check_resolved_function(id)?;
        let mut facts = self.graph.expression_facts.get(id)?;
        if matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "argument of type void",
            ));
        }
        let mut target = if matches!(self.graph.types.get(facts.ty).kind, TypeKind::Class(_)) {
            facts.ty
        } else {
            self.decayed_type(facts.ty)?
        };
        if self.lowering()
            && self.unevaluated == 0
            && matches!(self.graph.types.get(target).kind, TypeKind::Class(class) if self.graph.entity(class).complete)
        {
            // The ellipsis has no declared parameter whose call boundary
            // would request this storage. Defer layout with the owning body.
            self.request_object_layout(target, crate::support::source::SourceOffset(0));
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Basic(F::Float))
        ) {
            target = self.graph.types.scalar(Scalar::Basic(F::Double))?;
        } else if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Nullptr)
        ) {
            let void = self.graph.types.scalar(Scalar::Void)?;
            target = self.graph.types.pointer(void)?;
        } else if self.is_integral_operand(target) {
            target = self.promoted_type(target)?;
        }
        // The frozen ellipsis path records a target even when no ordinary
        // implicit sequence exists; it does not select a copy constructor.
        let mut conversion = self
            .scalar_sequence(facts, target, false)?
            .unwrap_or(Conversion {
                target,
                result: target,
                first: L::None,
                second: C::None,
                rank: R::Exact,
                qualification: false,
                initialization: false,
                temporary: false,
                reference_source: ReferenceSource::Other,
            });
        conversion.rank = R::Ellipsis;
        facts.conversion = Some(conversion);
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }
    pub(super) fn call_result(&mut self, function: TypeId) -> Result<ExprFacts, Error> {
        let TypeKind::Function { result, .. } = self.graph.types.get(function).kind else {
            return Err(Error::Invariant("call without function type"));
        };
        if let TypeKind::Reference { element, kind } = self.graph.types.get(result).kind {
            let category = if kind == RefKind::Lvalue
                || matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) {
                V::Lvalue
            } else {
                V::Xvalue
            };
            self.new_expression_facts(element, category)
        } else {
            let result = if matches!(self.graph.types.get(result).kind, TypeKind::Class(_)) {
                result
            } else {
                self.bare_type(result)?
            };
            self.new_expression_facts(result, V::Prvalue)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{
            Analyzer,
            expression_facts::{ConversionRank, ValueCategory},
            layout::LayoutState,
            lookup::Filter,
            types::Cv,
        },
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn ellipsis_retains_class_operand_identity_without_selecting_a_copy() {
        let source = SourceBuffer::new(
            b"struct E{int n;};int take(...);int g(const E&e){return take(((e)));}".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"ellipsis-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax_count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(syntax_count, tree.expressions.mark());
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("function body");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound body");
        };
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return call");
        };
        let ast::ExprKind::Call { arguments, .. } = tree.expressions.get(call).kind else {
            panic!("ordinary call");
        };
        let argument = tree.expr_lists.get(arguments)[0];
        let root = graph.expression_facts.root(argument);
        assert_ne!(argument, root);
        let facts = graph.expression_facts.get(argument).unwrap();
        assert_eq!(facts.category, ValueCategory::Lvalue);
        assert_eq!(graph.types.get(facts.ty).cv, Cv::CONST);
        let conversion = facts.conversion.unwrap();
        assert_eq!(conversion.target, facts.ty);
        assert_eq!(graph.types.get(conversion.result).cv, Cv::default());
        assert_eq!(graph.class_of(conversion.result), graph.class_of(facts.ty));
        assert_eq!(conversion.rank, ConversionRank::Ellipsis);
        assert_eq!(graph.expression_facts.get(root).unwrap().ty, facts.ty);
        assert!(facts.class_copy.is_none());
        assert!(graph.lower_class_copy(argument).unwrap().is_none());
    }

    #[test]
    fn ellipsis_storage_layout_follows_emission_and_unevaluated_boundaries() {
        let source = SourceBuffer::new(
            b"int take(...);struct Used{int n;};struct Unused{int n;};struct Query{int n;};inline int unused(){return take(Unused{});}int g(){return take(Used{2})+sizeof(take(Query{}));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"ellipsis-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        for (name, needed) in [
            (b"Used".as_slice(), true),
            (b"Unused", false),
            (b"Query", false),
        ] {
            let name = cursor.intern_name(name).unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            assert_eq!(
                matches!(graph.entity(class).layout, LayoutState::Value { .. }),
                needed,
            );
            assert!(graph.entity(class).implicit_copies.is_none());
        }
    }
}
