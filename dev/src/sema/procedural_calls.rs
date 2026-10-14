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
    Constructors {
        class: super::graph::EntityId,
        converting: bool,
    },
}
impl Candidates {
    fn bindings(self, graph: &super::graph::Graph) -> &[BindingId] {
        match self {
            Self::Functions(set) => graph.overloads.get(set),
            Self::Constructors { class, .. } => &graph.entity(class).declared_constructors,
        }
    }
    fn includes(self, graph: &super::graph::Graph, binding: BindingId) -> bool {
        !matches!(
            self,
            Self::Constructors {
                converting: true,
                ..
            }
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
}
impl CallScratch {
    pub(super) fn clear(&mut self) {
        self.candidates.clear();
        self.sequences.clear();
    }
}
#[derive(Clone, Copy)]
pub(super) struct ArgumentSequence {
    pub(super) conversion: Conversion,
    // An overload designation ranks as an exact target match, even when its
    // actual unary shape cannot bind that target. Diagnose only after selection.
    pub(super) application: Option<Conversion>,
    pub(super) function: Option<BindingId>,
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
            let mut found = self.lookup_call_result(name, scope)?;
            if let Some(facts) = self.builtin_call(id, name, arguments, scope, &mut found)? {
                return Ok(facts);
            }
            let syntax_name = self.tree.names.get(name);
            let qualified =
                syntax_name.global || self.tree.name_lists.get(syntax_name.components).len() > 1;
            if found.binding.is_none() && (qualified || callee != named) {
                return Err(Error::at(
                    self.tree.expressions.get(named).location,
                    if qualified {
                        "undeclared name"
                    } else {
                        "undeclared function name"
                    },
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
            for &argument in self.tree.expr_lists.get(arguments) {
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
                let component = self.tree.name_lists.get(syntax_name.components)[0];
                let key = Self::expression_name(component.kind)?;
                self.argument_dependent_lookup(key, arguments, &mut found)?;
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
            if let Some(component) = self.tree.name_lists.get(syntax_name.components).last()
                && component.arguments.is_some()
            {
                self.explicit_function_specializations(*component, &mut found, scope, true)?;
            }
            let facts = if let Some(facts) = named_value {
                facts
            } else {
                self.named_expression(named, &found, scope)?
            };
            self.publish_named_callee(callee, named, facts)?;
            facts
        } else {
            let facts = self.typed_expression(callee, scope)?;
            for &argument in self.tree.expr_lists.get(arguments) {
                self.type_call_argument(argument, scope)?;
            }
            facts
        };
        let facts = if let Some(class) = self.graph.class_of(facts.ty) {
            let location = self.tree.expressions.get(id).location;
            if !self.graph.entity(class).complete {
                return Err(Error::at(
                    location,
                    "call of an object of incomplete class type",
                ));
            }
            let home = self
                .graph
                .entity(class)
                .scope
                .ok_or(Error::Invariant("call class without scope"))?;
            self.lookup_location = location;
            let found = self.qualified_result(
                home,
                super::operator_candidates::operator_key(
                    crate::lex::pp_lexer::Punctuator::LeftParen,
                )?,
                super::lookup::Filter::Ordinary,
            )?;
            let binding = found
                .binding
                .filter(|&binding| {
                    self.graph.binding(binding).kind == super::graph::BindingKind::Function
                })
                .ok_or(Error::at(
                    location,
                    "called object is not a function or pointer to function",
                ))?;
            let object = super::expression_facts::ObjectArgument {
                source: super::expression_facts::ObjectSource::Explicit {
                    expression: callee,
                    arrow: false,
                },
                ty: facts.ty,
                category: facts.category,
            };
            let mut receiver = facts;
            receiver.object = Some(object);
            receiver.member_naming = Some(class);
            self.graph.expression_facts.set(callee, receiver);
            let mut designation = self.function_designation(binding, found.functions.as_slice())?;
            designation.object = Some(object);
            designation.member_naming = Some(class);
            designation
        } else {
            facts
        };
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
            let chosen = self.resolve_call(
                self.tree.expressions.get(id).location,
                Candidates::Functions(set),
                self.tree.expr_lists.get(arguments),
                facts.object,
                &mut scratch,
            );
            let chosen = chosen.and_then(|selected| {
                self.apply_call_arguments(
                    selected,
                    self.tree.expr_lists.get(arguments),
                    scope,
                    &scratch,
                )?;
                Ok(selected.binding)
            });
            scratch.clear();
            self.call_scratch = scratch;
            let binding = chosen?;
            for &argument in self.tree.expr_lists.get(arguments) {
                let mut facts = self.graph.expression_facts.get(argument)?;
                if let Some(conversion) = facts.conversion {
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
            self.check_class_arguments(binding, arguments, scope)?;
            let location = self.tree.expressions.get(id).location;
            self.record_call_receiver(callee, binding, facts, scope, location)?;
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
            self.graph.record_call_binding(id, binding);
            return Ok(facts);
        }
        self.indirect_call(id, callee, arguments, facts.ty, scope)
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
            self.check_direct_member_access(binding, scope, location)?;
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
        self.scalar_sequence(facts, target, false)
    }
    fn indirect_call(
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
        let args = self.tree.expr_lists.get(arguments);
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
                    self.initialize_list_argument(argument, target, scope)?;
                    continue;
                }
                self.class_parameter_destruction(
                    target,
                    scope,
                    self.tree.expressions.get(argument).location,
                )?;
                self.convert_scalar(argument, target, false)?;
                self.elide_class_copy(
                    argument,
                    target,
                    scope,
                    false,
                    false,
                    self.tree.expressions.get(argument).location,
                )?;
            } else {
                self.ellipsis_argument(argument)?;
            }
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
    ) -> Result<(), Error> {
        self.graph.expression_facts.set(named, facts);
        super::record(&mut self.graph.expression_types, named.index(), facts.ty);
        let mut outer = callee;
        while outer != named {
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
        if matches!(
            self.tree.expressions.get(id).kind,
            ast::ExprKind::Braced { .. }
        ) {
            return Ok(self
                .list_sequence(id, target)?
                .map(|conversion| ArgumentSequence {
                    conversion,
                    application: Some(conversion),
                    function: None,
                }));
        }
        let mut facts = self.graph.expression_facts.get(id)?;
        if facts.object.is_some()
            && facts.functions.is_some()
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
            }));
        }
        Ok(application.map(|conversion| ArgumentSequence {
            conversion,
            application: Some(conversion),
            function,
        }))
    }
    fn check_class_arguments(
        &mut self,
        binding: BindingId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ty = self
            .graph
            .entity(self.graph.binding(binding).entity)
            .ty
            .ok_or(Error::Invariant("selected function without type"))?;
        let count = self.graph.types.function_parameter_count(ty)?;
        for (index, &id) in self
            .tree
            .expr_lists
            .get(arguments)
            .iter()
            .enumerate()
            .take(count)
        {
            let target = self.graph.types.function_parameter(ty, index)?;
            self.class_parameter_destruction(
                target,
                scope,
                self.tree.expressions.get(id).location,
            )?;
            self.elide_class_copy(
                id,
                target,
                scope,
                false,
                false,
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
        scratch.candidates.clear();
        scratch.sequences.clear();
        for index in 0..set.bindings(&self.graph).len() {
            let primary = set.bindings(&self.graph)[index];
            if !set.includes(&self.graph, primary) {
                continue;
            }
            let Some(binding) = self.deduce_function_call(primary, args)? else {
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
            if args.len() > count && !variadic
                || (args.len()..count).any(|index| {
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
                    })
                } else {
                    None
                };
                scratch.sequences.push(sequence);
            }
            let mut viable = true;
            for (index, &arg) in args.iter().enumerate() {
                let sequence = if index < count {
                    let target = self.graph.types.function_parameter(ty, index)?;
                    let Some(conversion) = self.argument_sequence(arg, target)? else {
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
        if scratch.candidates.is_empty() {
            return Err(Error::at(location, "no viable function for the call"));
        }
        let object_count = usize::from(object.is_some());
        let mut best = 0;
        for index in 1..scratch.candidates.len() {
            if self.compare_candidates(scratch, index, best, args.len(), object_count)?
                == Ordering::Greater
            {
                best = index;
            }
        }
        for index in 0..scratch.candidates.len() {
            if index != best
                && self.compare_candidates(scratch, best, index, args.len(), object_count)?
                    != Ordering::Greater
            {
                return Err(Error::at(location, "ambiguous call to overloaded function"));
            }
        }
        let (binding, first) = scratch.candidates[best];
        Ok(CallSelection {
            binding,
            first,
            object_count,
        })
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
        self.check_template_call_defaults(binding, args.len())?;
        for (index, &arg) in args.iter().enumerate() {
            if let Some(sequence) = scratch.sequences[first + object_count + index] {
                if matches!(
                    self.tree.expressions.get(arg).kind,
                    ast::ExprKind::Braced { .. }
                ) {
                    self.initialize_list_argument(arg, sequence.conversion.target, scope)?;
                    continue;
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
                self.odr_use_constants(arg, facts.conversion)?;
                self.graph.expression_facts.set(arg, facts);
            } else {
                self.ellipsis_argument(arg)?;
            }
        }
        Ok(())
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
                (Some(a), Some(b)) => self.compare_sequences(a.conversion, b.conversion)?,
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
    fn ellipsis_argument(&mut self, id: ast::ExprId) -> Result<(), Error> {
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
        let mut target = self.decayed_type(facts.ty)?;
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
        let mut conversion = self
            .scalar_sequence(facts, target, false)?
            .ok_or(Error::Missing("class ellipsis argument"))?;
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
